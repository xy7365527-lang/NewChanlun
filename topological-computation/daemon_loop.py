"""Daemon 主循环 — CC session 的无状态调度器。

409号谱系：Daemon = DAG 中的 liveness 节点。

主循环逻辑：
    while True:
        events = drain(event_sources)      # 收集外部事件（channel/ACP）
        handoff = read_handoff()           # 读取上一轮状态
        scan = run_ceremony_scan()         # 扫描工位
        context = build_cc_context(...)    # 构造 CC 输入（含 events）
        result = invoke_cc_session(context)# CC 做认知判断
        new_handoff = parse_output(result) # 解析 CC 产出
        write_handoff(new_handoff)         # 持久化

关键原则：
- Daemon 不持有认知权威——所有概念判断委托给 CC session
- Daemon 是纯调度器——读状态→构造上下文→唤起CC→解析结果→持久化
- CC session 的短暂性是特性——每个 session 是一次性工作脉冲
- 唯一长期状态在 handoff schema 中（持久化到文件系统）
- Channel adapter/ACP bridge 是事件源，不是决策层（409号推论4/5）

用法：
    python daemon_loop.py                    # 单次执行（默认）
    python daemon_loop.py --continuous       # 持续循环
    python daemon_loop.py --dry-run          # 只构造上下文，不唤起 CC
    python daemon_loop.py --interval 60      # 循环间隔（秒）
    python daemon_loop.py --webhook 9800     # 启用 webhook adapter（端口 9800）
    python daemon_loop.py --acp              # 启用 ACP bridge（stdin/stdout）

纯 Python，零外部依赖。
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# 确保 topological-computation 和 scripts 在 sys.path 中
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
sys.path.insert(0, str(SCRIPT_DIR))
sys.path.insert(0, str(REPO_ROOT / "scripts"))

from handoff_schema import (
    HandoffRecord,
    SessionInput,
    SessionOutput,
    create_session_input,
    mark_session_complete,
    read_handoff,
    write_handoff,
    HANDOFF_PATH,
)
from channel_adapter import EventSourceManager


# ---------------------------------------------------------------------------
# 日志
# ---------------------------------------------------------------------------

LOG_PATH = SCRIPT_DIR / "daemon_loop.log"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    handlers=[
        logging.FileHandler(str(LOG_PATH), encoding="utf-8"),
        logging.StreamHandler(sys.stderr),
    ],
)
log = logging.getLogger("daemon_loop")


# ---------------------------------------------------------------------------
# ceremony_scan 调用
# ---------------------------------------------------------------------------

def run_ceremony_scan() -> dict | None:
    """执行 ceremony_scan.py，返回 JSON 输出。失败返回 None。"""
    scan_script = REPO_ROOT / "scripts" / "ceremony_scan.py"
    if not scan_script.exists():
        log.error("ceremony_scan.py 不存在: %s", scan_script)
        return None

    try:
        result = subprocess.run(
            [sys.executable, str(scan_script)],
            capture_output=True,
            text=True,
            timeout=120,
            cwd=str(REPO_ROOT),
        )
        if result.returncode != 0:
            log.error("ceremony_scan 失败 (rc=%d): %s", result.returncode, result.stderr[:500])
            return None

        return json.loads(result.stdout)
    except subprocess.TimeoutExpired:
        log.error("ceremony_scan 超时（120s）")
        return None
    except json.JSONDecodeError as exc:
        log.error("ceremony_scan 输出不是合法 JSON: %s", exc)
        return None


# ---------------------------------------------------------------------------
# 逢亮穿越状态获取
# ---------------------------------------------------------------------------

def get_traversal_status() -> dict:
    """尝试从逢亮 HTTP API 获取穿越状态。不可达时返回空 dict。"""
    import urllib.request
    import urllib.error

    try:
        req = urllib.request.Request("http://localhost:9765/status")
        with urllib.request.urlopen(req, timeout=5) as resp:
            if resp.status == 200:
                return json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, OSError, json.JSONDecodeError):
        pass

    return {}


# ---------------------------------------------------------------------------
# CC session 上下文构造
# ---------------------------------------------------------------------------

# CC session 输入的 prompt 模板
_CC_CONTEXT_TEMPLATE = """\
# Session Handoff — Epoch {epoch}

你是逢亮的 CC session 工作脉冲。以下是你的上下文。

## 上一轮结论
{last_conclusion}

## 纲目位置
{gangmu_position}

## 工位列表（ceremony_scan 输出）
{workstations_summary}

## 逢亮穿越状态
{traversal_summary}

## 待处理事件
{pending_events}

## 指令
1. 阅读工位列表，按局部依赖原则（275号）并行执行可执行工位
2. 执行完成后，产出 JSON 格式的 session 结果（见下方 schema）
3. 结果写入 stdout，daemon 会解析

## 输出 Schema（JSON，写到 stdout 的最后一行，以 `---HANDOFF---` 分隔）
```json
{{
  "dag_changes": [],
  "tasks_to_dispatch": [],
  "tuche_detected": false,
  "tuche_description": "",
  "needs_escalation": false,
  "escalation_reason": "",
  "session_summary": "本次工作脉冲的产出摘要",
  "settled_genealogy_ids": [],
  "commit_hash": ""
}}
```
"""

HANDOFF_DELIMITER = "---HANDOFF---"


def build_cc_context(handoff: HandoffRecord) -> str:
    """从 handoff 记录构造 CC session 的输入 prompt。"""
    inp = handoff.input

    # 工位摘要
    workstations = inp.ceremony_scan_output.get("workstations", [])
    if workstations:
        ws_lines = []
        for i, ws in enumerate(workstations, 1):
            name = ws.get("name", "?")
            priority = ws.get("priority", "?")
            ws_lines.append(f"  {i}. [{priority}] {name}")
        workstations_summary = "\n".join(ws_lines)
    else:
        workstations_summary = "（无工位——可能是干净终止条件）"

    # 穿越状态摘要
    ts = inp.traversal_status
    if ts:
        traversal_summary = (
            f"  步骤: {ts.get('total_steps', '?')}, "
            f"beta_1: {ts.get('beta_1', '?')}, "
            f"顶点: {ts.get('vertices_active', '?')}, "
            f"结晶次数: {ts.get('crystallization_count', '?')}"
        )
    else:
        traversal_summary = "（逢亮未在线或不可达）"

    # 待处理事件
    if inp.pending_events:
        event_lines = [f"  - {json.dumps(e, ensure_ascii=False)}" for e in inp.pending_events]
        pending_events = "\n".join(event_lines)
    else:
        pending_events = "（无待处理事件）"

    return _CC_CONTEXT_TEMPLATE.format(
        epoch=handoff.epoch,
        last_conclusion=inp.last_session_conclusion or "（首次 session，无前序结论）",
        gangmu_position=inp.gangmu_position or "（未设定）",
        workstations_summary=workstations_summary,
        traversal_summary=traversal_summary,
        pending_events=pending_events,
    )


# ---------------------------------------------------------------------------
# CC session 唤起
# ---------------------------------------------------------------------------

def invoke_cc_session(context: str, timeout: int = 600) -> str | None:
    """唤起 CC session（通过 claude --print 命令）。

    返回 CC 的 stdout 输出。失败返回 None。
    """
    try:
        result = subprocess.run(
            ["claude", "--print", "-p", context],
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(REPO_ROOT),
        )
        if result.returncode != 0:
            log.error("CC session 失败 (rc=%d): %s", result.returncode, result.stderr[:500])
            return None

        return result.stdout
    except FileNotFoundError:
        log.error("claude CLI 未安装或不在 PATH 中")
        return None
    except subprocess.TimeoutExpired:
        log.error("CC session 超时（%ds）", timeout)
        return None


# ---------------------------------------------------------------------------
# CC 输出解析
# ---------------------------------------------------------------------------

def parse_cc_output(raw_output: str) -> SessionOutput:
    """从 CC session 的 stdout 解析 SessionOutput。

    约定：CC session 在输出末尾以 `---HANDOFF---` 分隔符后面跟一行 JSON。
    如果没有分隔符，尝试从输出的最后一行解析 JSON。
    如果都失败，用整个输出作为 session_summary。
    """
    output = SessionOutput()

    # 尝试从分隔符后解析
    if HANDOFF_DELIMITER in raw_output:
        parts = raw_output.rsplit(HANDOFF_DELIMITER, 1)
        json_part = parts[1].strip()
        try:
            data = json.loads(json_part)
            return _dict_to_session_output(data)
        except json.JSONDecodeError:
            log.warning("分隔符后内容不是合法 JSON，回退到 session_summary")

    # 尝试最后一行
    lines = raw_output.strip().splitlines()
    for line in reversed(lines):
        line = line.strip()
        if line.startswith("{"):
            try:
                data = json.loads(line)
                if "session_summary" in data:
                    return _dict_to_session_output(data)
            except json.JSONDecodeError:
                continue

    # 全部失败：用整个输出作为摘要
    output.session_summary = raw_output[-2000:] if len(raw_output) > 2000 else raw_output
    return output


def _dict_to_session_output(data: dict) -> SessionOutput:
    """从 dict 构造 SessionOutput。"""
    return SessionOutput(
        dag_changes=data.get("dag_changes", []),
        tasks_to_dispatch=data.get("tasks_to_dispatch", []),
        tuche_detected=data.get("tuche_detected", False),
        tuche_description=data.get("tuche_description", ""),
        needs_escalation=data.get("needs_escalation", False),
        escalation_reason=data.get("escalation_reason", ""),
        session_summary=data.get("session_summary", ""),
        settled_genealogy_ids=data.get("settled_genealogy_ids", []),
        commit_hash=data.get("commit_hash", ""),
    )


# ---------------------------------------------------------------------------
# 主循环
# ---------------------------------------------------------------------------

def daemon_once(event_sources: EventSourceManager | None = None) -> HandoffRecord:
    """执行一次 daemon 循环：drain events → scan → context → invoke → parse → persist。

    Args:
        event_sources: 事件源管理器（channel adapter + ACP bridge）。
                       None 时不收集外部事件。

    返回完成后的 HandoffRecord。
    """
    # 0. 收集外部事件（409号推论4/5）
    pending_events: list[dict] = []
    if event_sources is not None:
        pending_events = event_sources.drain()
        if pending_events:
            log.info("收集到 %d 个外部事件", len(pending_events))

    # 1. 读取上一轮 handoff
    last_handoff = read_handoff()
    epoch = (last_handoff.epoch + 1) if last_handoff else 1
    log.info("=== Epoch %d 开始 ===", epoch)

    # 2. 执行 ceremony_scan
    log.info("执行 ceremony_scan...")
    scan_output = run_ceremony_scan()
    if scan_output is None:
        log.warning("ceremony_scan 失败，使用空输出")
        scan_output = {}

    # 检查干净终止条件
    if scan_output.get("clean_terminate", False):
        log.info("ceremony_scan 报告干净终止——无工位可执行")
        # 仍然写入 handoff 记录
        handoff = create_session_input(
            epoch=epoch,
            last_handoff=last_handoff,
            ceremony_scan_output=scan_output,
            pending_events=pending_events,
        )
        handoff = mark_session_complete(
            handoff,
            SessionOutput(session_summary="干净终止——无工位可执行"),
        )
        write_handoff(handoff)
        return handoff

    # 3. 获取逢亮穿越状态
    traversal_status = get_traversal_status()
    if traversal_status:
        log.info("逢亮穿越状态: steps=%s, beta_1=%s",
                 traversal_status.get("total_steps"),
                 traversal_status.get("beta_1"))
    else:
        log.info("逢亮不可达（离线或未启动）")

    # 4. 构造 handoff 记录
    handoff = create_session_input(
        epoch=epoch,
        last_handoff=last_handoff,
        ceremony_scan_output=scan_output,
        traversal_status=traversal_status,
        pending_events=pending_events,
    )
    write_handoff(handoff)  # 先持久化 pending 状态

    # 5. 构造 CC 上下文
    context = build_cc_context(handoff)
    log.info("CC 上下文已构造（%d 字符）", len(context))

    # 6. 唤起 CC session
    log.info("唤起 CC session...")
    raw_output = invoke_cc_session(context)
    if raw_output is None:
        log.error("CC session 唤起失败")
        output = SessionOutput(
            session_summary="CC session 唤起失败",
            needs_escalation=True,
            escalation_reason="CC session 唤起失败——检查 claude CLI",
        )
    else:
        log.info("CC session 完成（%d 字符输出）", len(raw_output))
        output = parse_cc_output(raw_output)

    # 7. 标记完成并持久化
    completed = mark_session_complete(handoff, output)
    write_handoff(completed)

    log.info("Epoch %d 完成: %s", epoch, output.session_summary[:100])

    if output.needs_escalation:
        log.warning("需要升级: %s", output.escalation_reason)

    return completed


def daemon_loop(
    interval: int = 60,
    max_epochs: int | None = None,
    event_sources: EventSourceManager | None = None,
) -> None:
    """持续执行 daemon 循环。

    Args:
        interval: 循环间隔（秒）
        max_epochs: 最大执行轮次（None = 无限）
        event_sources: 事件源管理器（channel adapter + ACP bridge）
    """
    log.info("Daemon 主循环启动（间隔=%ds, max_epochs=%s）", interval, max_epochs)
    epochs_run = 0

    while max_epochs is None or epochs_run < max_epochs:
        try:
            result = daemon_once(event_sources=event_sources)
            epochs_run += 1

            # 干净终止：ceremony_scan 报告无工位
            if result.output.session_summary == "干净终止——无工位可执行":
                log.info("干净终止条件达成——等待 %ds 后重试", interval)

            # 升级通知
            if result.output.needs_escalation:
                log.warning("升级标记已设——等待编排者处理后继续")
                # 不退出循环，下一轮会重新检查

        except KeyboardInterrupt:
            log.info("收到中断信号，退出")
            break
        except Exception as exc:
            log.exception("Daemon 循环异常: %s", exc)

        if max_epochs is not None and epochs_run >= max_epochs:
            break

        log.info("等待 %ds 后进入下一轮...", interval)
        time.sleep(interval)

    log.info("Daemon 主循环结束（共执行 %d 轮）", epochs_run)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Daemon 主循环 — CC session 的无状态调度器（409号谱系）",
    )
    parser.add_argument(
        "--continuous", action="store_true",
        help="持续循环模式（默认只执行一次）",
    )
    parser.add_argument(
        "--interval", type=int, default=60,
        help="循环间隔，秒（默认 60）",
    )
    parser.add_argument(
        "--max-epochs", type=int, default=None,
        help="最大执行轮次（默认无限）",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="只构造上下文并打印，不唤起 CC session",
    )
    parser.add_argument(
        "--webhook", type=int, default=None, metavar="PORT",
        help="启用 webhook adapter（指定端口，如 9800）",
    )
    parser.add_argument(
        "--acp", action="store_true",
        help="启用 ACP bridge（stdin/stdout NDJSON）",
    )
    args = parser.parse_args()

    # 初始化事件源管理器（409号推论4/5）
    event_sources = EventSourceManager()

    if args.webhook is not None:
        event_sources.create_webhook(port=args.webhook)
        log.info("已注册 WebhookAdapter（端口 %d）", args.webhook)

    if args.acp:
        from acp_bridge import ACPBridge
        acp = ACPBridge(event_sources.queue)
        event_sources.register(acp)
        log.info("已注册 ACPBridge")

    if args.dry_run:
        # Dry run：只构造上下文
        last_handoff = read_handoff()
        epoch = (last_handoff.epoch + 1) if last_handoff else 1
        scan_output = run_ceremony_scan()
        traversal_status = get_traversal_status()

        handoff = create_session_input(
            epoch=epoch,
            last_handoff=last_handoff,
            ceremony_scan_output=scan_output or {},
            traversal_status=traversal_status,
        )

        context = build_cc_context(handoff)
        print("=== CC Session Context (Dry Run) ===")
        print(context)
        print()
        print("=== Handoff Record ===")
        print(json.dumps(handoff.to_dict(), ensure_ascii=False, indent=2))
        return

    # 启动事件源
    event_sources.start_all()

    try:
        if args.continuous:
            daemon_loop(
                interval=args.interval,
                max_epochs=args.max_epochs,
                event_sources=event_sources,
            )
        else:
            daemon_once(event_sources=event_sources)
    finally:
        event_sources.stop_all()


if __name__ == "__main__":
    main()
