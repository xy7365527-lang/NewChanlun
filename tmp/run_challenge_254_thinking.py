"""
临时脚本：用 gemini-2.5-pro + thinking mode 对 254 号执行异质质询。
"""
import asyncio
import sys
sys.path.insert(0, "src")

from dotenv import load_dotenv
load_dotenv()

from newchan.gemini.modes import GeminiChallenger

async def main():
    challenger = GeminiChallenger(model="gemini-2.5-pro")

    ctx_file = "C:/Users/hanju/NewChanlun/tmp/challenge-254-ontology-ctx.md"
    with open(ctx_file, encoding="utf-8") as f:
        ctx = f.read()

    subject = (
        "254号多经济体资本流转本体论的三个审计点："
        "(1)三态逆序递归顺序是否从ker(D)结构推出还是直觉包装；"
        "(2)黄金折叠后C-$边退化的拓扑后果是否影响六边分类；"
        "(3)E-$_USD移出ker(D)的可检测性"
    )

    result = await challenger.challenge_with_tools(
        subject,
        ctx,
        max_tool_calls=20,
    )

    print(f"[{result.mode}] model={result.model}")
    if result.tool_calls:
        print(f"tool_calls ({len(result.tool_calls)}):")
        for tc in result.tool_calls:
            print(f"  -> {tc}")
    print()
    print("=== 推理链 ===")
    for i, step in enumerate(result.reasoning_chain, 1):
        stype = step["type"]
        if stype == "thought":
            print(f"[{i}] THOUGHT: {step['content'][:300]}")
        elif stype == "tool_call":
            args_str = ", ".join(f"{k}={v!r}" for k, v in step.get("args", {}).items())
            print(f"[{i}] TOOL: {step['name']}({args_str[:200]})")
        elif stype == "tool_result":
            content = step.get("content", "")
            print(f"    -> {step.get('name','')}: {content[:200]}")
    print()
    print("=" * 60)
    print(result.response)

asyncio.run(main())
