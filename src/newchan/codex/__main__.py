"""CLI 入口 — python -m newchan.codex

用法:
  python -m newchan.codex review "审查目标"
  python -m newchan.codex diagnose "诊断目标" --context "失败信息"
  python -m newchan.codex decide "技术选型问题"
"""

from __future__ import annotations

import argparse
import sys

from dotenv import load_dotenv

from newchan.codex.modes import CodexChallenger, ReviewResult


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Codex 代码层异质审查 CLI")
    parser.add_argument(
        "mode",
        choices=["review", "diagnose", "decide"],
        help="审查模式",
    )
    parser.add_argument("subject", help="审查/诊断/选型目标")
    parser.add_argument("--context", default="", help="上下文信息")
    parser.add_argument(
        "--context-file", default=None, help="从文件读取上下文",
    )
    return parser


def _print_result(result: ReviewResult) -> None:
    print(f"[{result.mode}] model={result.model}")
    print("=" * 60)
    print(result.response)


def main() -> None:
    load_dotenv()
    args = _build_parser().parse_args()

    ctx = args.context
    if args.context_file:
        with open(args.context_file, encoding="utf-8") as f:
            ctx = f.read()

    try:
        challenger = CodexChallenger()
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if args.mode == "review":
        result = challenger.review(args.subject, ctx)
    elif args.mode == "diagnose":
        result = challenger.diagnose(args.subject, ctx)
    else:
        result = challenger.decide(args.subject, ctx)

    _print_result(result)


if __name__ == "__main__":
    main()
