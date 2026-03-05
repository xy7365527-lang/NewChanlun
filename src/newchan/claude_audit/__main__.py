"""CLI 入口 — python -m newchan.claude_audit

两种模式：
  python -m newchan.claude_audit audit <task> --context <text> --type code_review
  python -m newchan.claude_audit serve   (启动 MCP server)

概念溯源: [新缠论] — 异质审计 CLI
"""

from __future__ import annotations

import argparse
import json
import sys

from dotenv import load_dotenv

from newchan.claude_audit.engine import ClaudeAuditor
from newchan.claude_audit.modes import _VALID_AUDIT_TYPES


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Claude 异质审计 CLI")
    sub = parser.add_subparsers(dest="command", required=True)

    # audit 子命令
    audit_p = sub.add_parser("audit", help="执行审计")
    audit_p.add_argument("task", help="审计任务描述")
    audit_p.add_argument("--context", default="", help="审计上下文")
    audit_p.add_argument(
        "--context-file", default=None, help="从文件读取上下文",
    )
    audit_p.add_argument(
        "--type", dest="audit_type", default="code_review",
        choices=sorted(_VALID_AUDIT_TYPES), help="审计类型",
    )
    audit_p.add_argument(
        "--model", default=None, help="Claude 模型",
    )

    # serve 子命令
    sub.add_parser("serve", help="启动 MCP server")

    return parser


def main() -> None:
    load_dotenv()
    args = _build_parser().parse_args()

    if args.command == "serve":
        from newchan.claude_audit.server import main as serve_main

        serve_main()
        return

    # audit 命令
    ctx = args.context
    if args.context_file:
        with open(args.context_file, encoding="utf-8") as f:
            ctx = f.read()

    try:
        auditor = (
            ClaudeAuditor(model=args.model)
            if args.model
            else ClaudeAuditor()
        )
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    result = auditor.audit(args.task, ctx, args.audit_type)
    output = {
        "audit_type": result.audit_type,
        "model": result.model,
        "result": result.parsed,
    }
    print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
