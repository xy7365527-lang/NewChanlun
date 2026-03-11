#!/usr/bin/env python
"""Escalate 分级路由系统——判定 /escalate 产出的级别并格式化输出包。

三级判定：
  L1（agent 闭环）：推论冲突、数据不足、coverage 判定 → self_resolve
  L2（路由到对话）：跨谱系判断但不改架构前提 → route_to_conversation
  L3（主权决策）：架构边界变更 → route_to_conversation_sovereign

用法:
  python scripts/escalate_router.py grade "矛盾描述文本"
  python scripts/escalate_router.py format --genealogy 410 --question "..."
  python scripts/escalate_router.py send --genealogy 410 --block xxx --question "..." --agents "a:posA,b:posB"
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

# ---------------------------------------------------------------------------
# 关键词集合（不可变 frozenset）
# ---------------------------------------------------------------------------

L3_KEYWORDS: frozenset[str] = frozenset({
    "定义冲突",
    "架构",
    "settled",
    "穿越引擎操作语义",
    "核心公式删除",
    "重定义",
    "架构边界变更",
    "删除核心公式",
    "操作语义",
})

L2_KEYWORDS: frozenset[str] = frozenset({
    "谱系",
    "cross-gang",
    "否定",
    "分裂",
    "跨谱系",
    "概念分离",
    "分歧",
})

# ---------------------------------------------------------------------------
# 级别枚举与路由结果
# ---------------------------------------------------------------------------

LEVEL_LABELS: dict[int, str] = {
    1: "self_resolve",
    2: "route_to_conversation",
    3: "route_to_conversation_sovereign",
}


@dataclass(frozen=True)
class GradeResult:
    """分级判定的不可变结果。"""
    level: int
    route: str
    matched_keywords: tuple[str, ...]
    description: str


@dataclass(frozen=True)
class EscalatePackage:
    """格式化后的 escalate 包（不可变）。"""
    level: int
    genealogy_id: str
    block_hash: str
    divergences: tuple[tuple[str, str], ...]  # ((agent, position), ...)
    collision_records: tuple[str, ...]
    question: str

    def render(self) -> str:
        """渲染为标准 escalate 包文本。"""
        lines = [
            f"/escalate L{self.level}",
            f"谱系: {self.genealogy_id}",
            f"block: {self.block_hash}",
        ]

        if self.divergences:
            div_parts = [
                f"{agent} 认为 {position}"
                for agent, position in self.divergences
            ]
            lines.append(f"分歧: [{', '.join(div_parts)}]")
        else:
            lines.append("分歧: []")

        if self.collision_records:
            lines.append(f"碰撞记录: [{', '.join(self.collision_records)}]")
        else:
            lines.append("碰撞记录: []")

        lines.append(f"问题: [{self.question}]")
        return "\n".join(lines)


# ---------------------------------------------------------------------------
# 分级逻辑
# ---------------------------------------------------------------------------

def _find_matching_keywords(
    text: str,
    keyword_set: frozenset[str],
) -> tuple[str, ...]:
    """在文本中查找匹配的关键词，返回不可变元组。"""
    return tuple(kw for kw in keyword_set if kw in text)


def grade_escalation(description: str) -> GradeResult:
    """判定 escalate 描述的级别。

    判级规则（按优先级）：
    1. 包含 L3 关键词 → L3（主权决策）
    2. 包含 L2 关键词但不触发 L3 → L2（路由到对话）
    3. 其余 → L1（agent 闭环）
    """
    if not description or not description.strip():
        return GradeResult(
            level=1,
            route=LEVEL_LABELS[1],
            matched_keywords=(),
            description="空描述，默认 L1",
        )

    l3_matches = _find_matching_keywords(description, L3_KEYWORDS)
    if l3_matches:
        return GradeResult(
            level=3,
            route=LEVEL_LABELS[3],
            matched_keywords=l3_matches,
            description="触发 L3 关键词——架构边界变更/主权决策",
        )

    l2_matches = _find_matching_keywords(description, L2_KEYWORDS)
    if l2_matches:
        return GradeResult(
            level=2,
            route=LEVEL_LABELS[2],
            matched_keywords=l2_matches,
            description="触发 L2 关键词——跨谱系判断",
        )

    return GradeResult(
        level=1,
        route=LEVEL_LABELS[1],
        matched_keywords=(),
        description="无 L2/L3 关键词——agent 闭环处理",
    )


# ---------------------------------------------------------------------------
# 格式化
# ---------------------------------------------------------------------------

def _parse_agents_string(agents_str: str) -> tuple[tuple[str, str], ...]:
    """解析 "agent_a:position_a,agent_b:position_b" 格式。

    返回不可变的 ((agent, position), ...) 元组。
    """
    if not agents_str or not agents_str.strip():
        return ()

    pairs = []
    for pair in agents_str.split(","):
        pair = pair.strip()
        if ":" not in pair:
            raise ValueError(
                f"agents 格式错误：'{pair}' 应为 'agent:position' 格式"
            )
        agent, position = pair.split(":", 1)
        pairs.append((agent.strip(), position.strip()))
    return tuple(pairs)


def format_escalate_package(
    *,
    genealogy_id: str,
    block_hash: str = "",
    question: str,
    agents_str: str = "",
    collision_summary: str = "",
    description: str = "",
) -> EscalatePackage:
    """构建格式化的 escalate 包。

    先对 description（或 question）执行分级，然后组装完整包。
    """
    grade_text = description if description else question
    grade_result = grade_escalation(grade_text)

    divergences = _parse_agents_string(agents_str)

    collision_records: tuple[str, ...]
    if collision_summary and collision_summary.strip():
        collision_records = tuple(
            s.strip()
            for s in collision_summary.split(";")
            if s.strip()
        )
    else:
        collision_records = ()

    return EscalatePackage(
        level=grade_result.level,
        genealogy_id=genealogy_id,
        block_hash=block_hash,
        divergences=divergences,
        collision_records=collision_records,
        question=question,
    )


# ---------------------------------------------------------------------------
# 发送（委托给 escalate_send.py）
# ---------------------------------------------------------------------------

def send_escalate_package(package: EscalatePackage) -> str:
    """调用 escalate_send.py 发送 escalate 包并返回响应。

    仅 L2/L3 级别才发送。L1 返回 self_resolve 提示。
    """
    if package.level == 1:
        return (
            f"L1 级别无需路由——agent 闭环处理。\n"
            f"问题: {package.question}"
        )

    scripts_dir = Path(__file__).resolve().parent
    send_script = scripts_dir / "escalate_send.py"

    if not send_script.is_file():
        raise FileNotFoundError(
            f"发送脚本不存在: {send_script}\n"
            f"请确认 escalate_send.py 已创建"
        )

    message = package.render()

    result = subprocess.run(
        [sys.executable, str(send_script), "--message", message],
        capture_output=True,
        text=True,
        timeout=360,
    )

    if result.returncode != 0:
        raise RuntimeError(
            f"escalate_send.py 执行失败 (code {result.returncode}):\n"
            f"stderr: {result.stderr}\n"
            f"stdout: {result.stdout}"
        )

    return result.stdout.strip()


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Escalate 分级路由系统",
        prog="escalate_router.py",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # grade 子命令
    grade_p = sub.add_parser("grade", help="判定 escalate 级别")
    grade_p.add_argument("description", help="矛盾描述文本")

    # format 子命令
    fmt_p = sub.add_parser("format", help="格式化 escalate 包（不发送）")
    fmt_p.add_argument("--genealogy", required=True, help="谱系编号")
    fmt_p.add_argument("--block", default="", help="区块 hash")
    fmt_p.add_argument("--question", required=True, help="需要决断的问题")
    fmt_p.add_argument("--agents", default="", help="分歧方 agent:position,...")
    fmt_p.add_argument("--collision", default="", help="碰撞记录摘要（分号分隔）")
    fmt_p.add_argument("--description", default="", help="完整矛盾描述（用于分级）")

    # send 子命令
    send_p = sub.add_parser("send", help="格式化并发送 escalate 包")
    send_p.add_argument("--genealogy", required=True, help="谱系编号")
    send_p.add_argument("--block", default="", help="区块 hash")
    send_p.add_argument("--question", required=True, help="需要决断的问题")
    send_p.add_argument("--agents", default="", help="分歧方 agent:position,...")
    send_p.add_argument("--collision", default="", help="碰撞记录摘要（分号分隔）")
    send_p.add_argument("--description", default="", help="完整矛盾描述（用于分级）")

    return parser


def main() -> None:
    parser = _build_parser()
    args = parser.parse_args()

    if args.command == "grade":
        result = grade_escalation(args.description)
        output = {
            "level": result.level,
            "route": result.route,
            "matched_keywords": list(result.matched_keywords),
            "description": result.description,
        }
        print(json.dumps(output, ensure_ascii=False, indent=2))

    elif args.command == "format":
        package = format_escalate_package(
            genealogy_id=args.genealogy,
            block_hash=args.block,
            question=args.question,
            agents_str=args.agents,
            collision_summary=args.collision,
            description=args.description,
        )
        print(package.render())

    elif args.command == "send":
        package = format_escalate_package(
            genealogy_id=args.genealogy,
            block_hash=args.block,
            question=args.question,
            agents_str=args.agents,
            collision_summary=args.collision,
            description=args.description,
        )
        print(f"--- 包内容 ---")
        print(package.render())
        print(f"--- 发送中 ---")
        response = send_escalate_package(package)
        print(f"--- 响应 ---")
        print(response)


if __name__ == "__main__":
    main()
