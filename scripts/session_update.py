#!/usr/bin/env python3
"""session_update.py — session 文件的确定性更新

用法:
  python scripts/session_update.py                           # 基于 ceremony_scan 输出创建/更新 session
  python scripts/session_update.py --append "工位名: 产出摘要"  # 增量追加工位产出
  python scripts/session_update.py --finalize                # 收尾：标记完成状态

确定性操作，不需要 LLM。ceremony 收尾流程调用。
"""

import argparse
import json
import os
import subprocess
import sys
from datetime import datetime
from pathlib import Path


def get_root() -> Path:
    return Path(__file__).resolve().parent.parent


def get_latest_session(root: Path) -> Path | None:
    """找到最新的 session 文件。"""
    sessions_dir = root / ".chanlun" / "sessions"
    if not sessions_dir.exists():
        return None
    files = sorted(sessions_dir.glob("*.md"), reverse=True)
    return files[0] if files else None


def get_head() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            stderr=subprocess.DEVNULL,
        ).decode().strip()
    except Exception:
        return "unknown"


def get_scan_data(root: Path) -> dict | None:
    """运行 ceremony_scan.py 获取当前状态。"""
    script = root / "scripts" / "ceremony_scan.py"
    if not script.exists():
        return None
    try:
        result = subprocess.run(
            [sys.executable, str(script)],
            capture_output=True, text=True, cwd=str(root),
        )
        return json.loads(result.stdout)
    except Exception:
        return None


def create_session(root: Path, scan: dict) -> Path:
    """创建新 session 文件。"""
    sessions_dir = root / ".chanlun" / "sessions"
    sessions_dir.mkdir(parents=True, exist_ok=True)

    now = datetime.now()
    filename = now.strftime("%Y-%m-%d-%H%M-session.md")
    filepath = sessions_dir / filename

    head = get_head()
    mode = scan.get("mode", "unknown")
    definitions = scan.get("definitions", "?")
    settled = scan.get("settled", "?")
    pending = scan.get("pending", "?")
    workstations = scan.get("workstations", [])

    lines = [
        "# Session\n",
        f"\n**时间**: {now.strftime('%Y-%m-%d-%H%M')}",
        f"\n**分支**: main",
        f"\n**最新提交**: {head}",
        f"\n**模式**: {mode}",
        f"\n\n## 定义基底",
        f"\n定义数: {definitions}",
        f"\n\n## 谱系状态",
        f"\n- 已结算: {settled} 个",
        f"\n- 生成态: {pending} 个",
        f"\n\n## 工位",
        "",
    ]

    for ws in workstations:
        name = ws.get("name", "unknown")
        status = ws.get("status", "")
        priority = ws.get("priority", "")
        lines.append(f"\n### {name}")
        lines.append(f"\n- 优先级: {priority}")
        lines.append(f"\n- 状态: pending")
        lines.append(f"\n- 来源状态: {status}")

    lines.append("\n\n## 产出记录\n")
    lines.append("\n（工位完成后增量追加）\n")

    filepath.write_text("\n".join(lines), encoding="utf-8")
    return filepath


def append_output(session_path: Path, output_line: str):
    """增量追加工位产出到 session 文件。"""
    content = session_path.read_text(encoding="utf-8")
    marker = "## 产出记录"
    if marker in content:
        idx = content.index(marker) + len(marker)
        # 找到 marker 后的换行
        next_nl = content.index("\n", idx)
        content = content[:next_nl] + f"\n\n- {output_line}" + content[next_nl:]
    else:
        content += f"\n\n## 产出记录\n\n- {output_line}\n"
    session_path.write_text(content, encoding="utf-8")


def finalize_session(session_path: Path):
    """收尾：更新 HEAD，标记完成。"""
    content = session_path.read_text(encoding="utf-8")
    head = get_head()

    # 更新 HEAD
    import re
    content = re.sub(
        r"\*\*最新提交\*\*: \S+",
        f"**最新提交**: {head}",
        content,
    )

    # 追加收尾标记
    if "## 收尾" not in content:
        content += f"\n\n## 收尾\n\n完成时间: {datetime.now().strftime('%Y-%m-%d %H:%M')}\nHEAD: {head}\n"

    session_path.write_text(content, encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description="Session file manager")
    parser.add_argument("--append", type=str, help="Append workstation output")
    parser.add_argument("--finalize", action="store_true", help="Finalize session")
    args = parser.parse_args()

    root = get_root()

    if args.finalize:
        session = get_latest_session(root)
        if session:
            finalize_session(session)
            print(f"[session] 收尾: {session.name}")
        else:
            print("[session] 无 session 文件可收尾", file=sys.stderr)
            sys.exit(1)
        return

    if args.append:
        session = get_latest_session(root)
        if session:
            append_output(session, args.append)
            print(f"[session] 追加到 {session.name}: {args.append[:60]}")
        else:
            print("[session] 无 session 文件可追加", file=sys.stderr)
            sys.exit(1)
        return

    # 默认：创建新 session
    scan = get_scan_data(root)
    if not scan:
        print("[session] ceremony_scan.py 执行失败", file=sys.stderr)
        sys.exit(1)

    session = create_session(root, scan)
    print(f"[session] 创建: {session.name}")


if __name__ == "__main__":
    main()
