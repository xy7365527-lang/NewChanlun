"""CLI 入口 — python -m newchan.codex

用法:
  python -m newchan.codex review "审查目标"
  python -m newchan.codex diagnose "诊断目标" --context "失败信息"
  python -m newchan.codex decide "技术选型问题"
"""

from __future__ import annotations

import argparse
from dataclasses import replace
from datetime import datetime, timezone
from pathlib import Path

from dotenv import load_dotenv

from newchan.codex.modes import CodexChallenger, ReviewResult

_RESULTS_DIR = Path(".chanlun/review-results")


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


def _save_result(result: ReviewResult, timestamp: datetime) -> Path:
    """将审查结果持久化到 .chanlun/review-results/。"""
    _RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    ts_tag = timestamp.strftime("%Y%m%d-%H%M")
    filename = f"codex-{result.mode}-{ts_tag}.md"
    path = _RESULTS_DIR / filename
    path.write_text(result.to_markdown(timestamp), encoding="utf-8")
    return path


def main() -> None:
    load_dotenv()
    args = _build_parser().parse_args()

    ctx = args.context
    if args.context_file:
        with open(args.context_file, encoding="utf-8") as f:
            ctx = f.read()

    challenger = CodexChallenger()

    if args.mode == "review":
        result = challenger.review(args.subject, ctx)
    elif args.mode == "diagnose":
        result = challenger.diagnose(args.subject, ctx)
    else:
        result = challenger.decide(args.subject, ctx)

    if args.context_file:
        result = replace(result, context_file=args.context_file)

    _print_result(result)

    now = datetime.now(tz=timezone.utc)
    saved = _save_result(result, now)
    print(f"\n[持久化] {saved}")


if __name__ == "__main__":
    main()
