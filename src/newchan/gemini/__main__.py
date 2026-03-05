"""CLI 入口 — python -m newchan.gemini

保持与 python -m newchan.gemini_challenger 完全相同的行为。
"""

from __future__ import annotations

import argparse
import asyncio
import sys

from dotenv import load_dotenv

from newchan.gemini.modes import ChallengeResult, GeminiChallenger


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Gemini 质询工位 CLI")
    parser.add_argument(
        "mode",
        choices=["challenge", "verify", "decide", "derive"],
        help="质询模式",
    )
    parser.add_argument("subject", help="质询/验证目标")
    parser.add_argument("--context", default="", help="上下文信息")
    parser.add_argument(
        "--context-file", default=None, help="从文件读取上下文",
    )
    parser.add_argument(
        "--domain",
        default="General Mathematics",
        help="推导所在的公理域（仅 derive 模式，默认 General Mathematics）",
    )
    parser.add_argument(
        "--tools",
        action="store_true",
        help="启用 MCP 工具模式（Gemini 可访问 Serena 语义工具）",
    )
    parser.add_argument(
        "--max-tool-calls",
        type=int,
        default=20,
        help="MCP 模式下最大工具调用次数（默认 20）",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="输出完整推理链（Gemini 的每步思考 + 工具调用 + 工具返回）",
    )
    return parser


def _print_result(result: ChallengeResult, *, verbose: bool) -> None:
    print(f"[{result.mode}] model={result.model}")
    if result.tool_calls:
        print(f"tool_calls ({len(result.tool_calls)}):")
        for tc in result.tool_calls:
            print(f"  → {tc}")
    if verbose and result.reasoning_chain:
        print()
        print("=== Gemini 推理链 ===")
        for i, step in enumerate(result.reasoning_chain, 1):
            stype = step["type"]
            if stype == "thought":
                print(f"[{i}] 💭 {step['content']}")
            elif stype == "tool_call":
                args_str = ", ".join(
                    f"{k}={v!r}"
                    for k, v in step.get("args", {}).items()
                )
                print(f"[{i}] 🔍 {step['name']}({args_str})")
            elif stype == "tool_result":
                content = step.get("content", "")
                preview = (
                    content[:200] + "..."
                    if len(content) > 200
                    else content
                )
                print(f"    → {step.get('name', '')}: {preview}")
        print()
    print("=" * 60)
    print(result.response)


def main() -> None:
    load_dotenv(override=True)
    args = _build_parser().parse_args()

    ctx = args.context
    if args.context_file:
        with open(args.context_file, encoding="utf-8") as f:
            ctx = f.read()

    try:
        challenger = GeminiChallenger()
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if args.tools:
        async def _run() -> ChallengeResult:
            if args.mode == "challenge":
                return await challenger.challenge_with_tools(
                    args.subject, ctx,
                    max_tool_calls=args.max_tool_calls,
                )
            if args.mode == "decide":
                return await challenger.decide_with_tools(
                    args.subject, ctx,
                    max_tool_calls=args.max_tool_calls,
                )
            if args.mode == "derive":
                return await challenger.derive_with_tools(
                    args.subject, ctx, args.domain,
                    max_tool_calls=args.max_tool_calls,
                )
            return await challenger.verify_with_tools(
                args.subject, ctx,
                max_tool_calls=args.max_tool_calls,
            )

        result = asyncio.run(_run())
    else:
        if args.mode == "challenge":
            result = challenger.challenge(args.subject, ctx)
        elif args.mode == "decide":
            result = challenger.decide(args.subject, ctx)
        elif args.mode == "derive":
            result = challenger.derive(args.subject, ctx, args.domain)
        else:
            result = challenger.verify(args.subject, ctx)

    _print_result(result, verbose=args.verbose)


if __name__ == "__main__":
    main()
