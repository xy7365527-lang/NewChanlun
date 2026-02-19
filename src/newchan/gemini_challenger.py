"""Gemini 异质质询工位

结构性工位：用 Gemini 3 Pro 对缠论形式化产出提供异质否定。
Gemini 的理解方式与 Claude 根本不同，因此可能产出 Claude 自身
永远产出不了的否定。

两种模式：
- 纯文本模式：challenge() / verify() — 传入文本，返回质询结果
- MCP 工具模式：challenge_with_tools() / verify_with_tools()
  — Gemini 通过 MCP 协议访问 Serena 语义工具（find_symbol、
    get_symbols_overview、find_referencing_symbols 等），
    自主导航代码库后再质询。google-genai SDK 的
    automatic_function_calling 自动处理 agentic loop。

操作规则（SKILL.md）：
1. Lead 或指定 agent 调用 challenge() / verify()
2. 返回结果由调用者判断否定是否成立
3. 成立 → 正常矛盾流程（写谱系、走质询循环）
4. 不成立 → 记录为外部工具误判，不进入谱系

本体论位置：030a（生成态）— 不属于成员/工具任何一个已有范畴。

概念溯源: [新缠论] — 异质模型质询
"""

from __future__ import annotations

import asyncio
import logging
import os
from dataclasses import dataclass, field
from typing import Literal

from google import genai
from google.genai import errors as genai_errors, types as genai_types

logger = logging.getLogger(__name__)

__all__ = [
    "GeminiChallenger",
    "ChallengeResult",
    "challenge",
    "verify",
    "achallenge",
    "averify",
]

_MODEL = "gemini-3-pro-preview"
_FALLBACK_MODEL = "gemini-2.5-pro"

_SYSTEM_PROMPT = """\
你是缠论形式化项目的异质质询者。你的任务是从不同角度审视概念定义、\
代码实现、和推理链条，找出可能的矛盾、遗漏、或逻辑漏洞。

关键原则：
- 你不需要认同项目的所有前提，但需要理解它们
- 你的价值在于提供 Claude 可能看不到的否定
- 如果你认为没有问题，明确说"无否定"
- 如果你发现问题，精确描述矛盾：什么跟什么冲突、为什么不可弥合
- 不要客套，不要模糊化，直击要害
"""

_SYSTEM_PROMPT_WITH_TOOLS = """\
你是缠论形式化项目的异质质询者，拥有代码库的语义级访问能力。

你可以使用工具来理解代码结构和关系。工作流程：
1. 先用工具理解相关代码的结构和关系（符号导航、引用追踪）
2. 基于实际代码（而非假设）进行质询
3. 引用具体的文件路径和符号名称

关键原则：
- 你不需要认同项目的所有前提，但需要理解它们
- 你的价值在于提供 Claude 可能看不到的否定
- 如果你认为没有问题，明确说"无否定"
- 如果你发现问题，精确描述矛盾：什么跟什么冲突、为什么不可弥合
- 不要客套，不要模糊化，直击要害
- 引用具体代码位置支撑你的判断
"""

_CHALLENGE_TEMPLATE = """\
## 质询目标

{subject}

## 上下文

{context}

## 质询要求

请从以下角度审视：
1. 定义内部一致性：是否自相矛盾？
2. 定义间一致性：是否与其他定义冲突？
3. 逻辑完备性：是否存在未覆盖的边界情况？
4. 实现忠实度：代码是否忠实反映了定义？

如果发现问题，请按以下格式输出：
- **矛盾点**：精确描述
- **冲突方**：A 说什么 vs B 说什么
- **严重性**：致命 / 重要 / 建议
- **建议**：如何解决（如果有）

如果没有发现问题，输出"无否定"并说明你检查了什么。
"""

_VERIFY_TEMPLATE = """\
## 验证目标

{subject}

## 上下文

{context}

## 验证要求

请验证以下断言是否成立：
1. 给定的推理链是否逻辑有效？
2. 前提是否充分支撑结论？
3. 是否存在隐藏假设？

输出格式：
- **结论**：成立 / 不成立 / 部分成立
- **依据**：为什么
- **隐藏假设**：如果有
"""


@dataclass(frozen=True, slots=True)
class ChallengeResult:
    """质询结果。"""

    mode: Literal["challenge", "verify"]
    subject: str
    response: str
    model: str
    tool_calls: tuple[str, ...] = ()  # MCP 工具调用历史摘要
    reasoning_chain: tuple[dict, ...] = ()  # 完整推理链（thought/tool_call/tool_result）


class GeminiChallenger:
    """Gemini 质询工位的核心类。

    Parameters
    ----------
    api_key : str | None
        Google API Key。None 时从 GOOGLE_API_KEY 环境变量读取。
    model : str
        模型名称，默认 gemini-3-pro-preview。
    """

    def __init__(
        self,
        api_key: str | None = None,
        model: str = _MODEL,
    ) -> None:
        key = api_key or os.environ.get("GOOGLE_API_KEY", "")
        if not key:
            raise ValueError(
                "GOOGLE_API_KEY 未设置。"
                "请在 .env 中设置或传入 api_key 参数。"
            )
        self._client = genai.Client(api_key=key)
        self._model = model

    def _call_with_fallback(
        self,
        prompt: str,
        temperature: float,
    ) -> tuple[str, str]:
        """调用 Gemini API，主模型 503 时自动降级到 fallback。

        Returns (response_text, actual_model_used)。
        """
        for model in (self._model, _FALLBACK_MODEL):
            try:
                response = self._client.models.generate_content(
                    model=model,
                    contents=prompt,
                    config=genai.types.GenerateContentConfig(
                        system_instruction=_SYSTEM_PROMPT,
                        temperature=temperature,
                    ),
                )
                return response.text or "", model
            except genai_errors.ServerError:
                if model == self._model and model != _FALLBACK_MODEL:
                    logger.warning(
                        "%s 不可用 (503)，降级到 %s",
                        model,
                        _FALLBACK_MODEL,
                    )
                    continue
                raise
        # unreachable, but satisfies type checker
        raise RuntimeError("所有模型均不可用")  # pragma: no cover

    def challenge(self, subject: str, context: str = "") -> ChallengeResult:
        """对给定主题发起质询。

        Parameters
        ----------
        subject : str
            质询目标（定义、代码、推理链等）。
        context : str
            相关上下文信息。

        Returns
        -------
        ChallengeResult
        """
        prompt = _CHALLENGE_TEMPLATE.format(subject=subject, context=context)
        text, model_used = self._call_with_fallback(prompt, temperature=0.3)
        return ChallengeResult(
            mode="challenge",
            subject=subject,
            response=text,
            model=model_used,
        )

    def verify(self, subject: str, context: str = "") -> ChallengeResult:
        """验证给定断言是否成立。

        Parameters
        ----------
        subject : str
            验证目标。
        context : str
            相关上下文信息。

        Returns
        -------
        ChallengeResult
        """
        prompt = _VERIFY_TEMPLATE.format(subject=subject, context=context)
        text, model_used = self._call_with_fallback(prompt, temperature=0.1)
        return ChallengeResult(
            mode="verify",
            subject=subject,
            response=text,
            model=model_used,
        )

    # ── MCP 工具模式（async） ──

    async def _call_with_tools_and_fallback(
        self,
        prompt: str,
        temperature: float,
        session: object,  # mcp.client.session.ClientSession
        max_tool_calls: int = 20,
    ) -> tuple[str, str, tuple[str, ...], tuple[dict, ...]]:
        """Gemini + MCP 自动 function calling 循环。

        google-genai SDK 原生支持 MCP ClientSession 作为 tool。
        SDK 自动处理：列出工具 → Gemini 发 function call →
        SDK 执行 → 结果回传 → 重复。

        Returns (response_text, actual_model, tool_call_summaries, reasoning_chain)。
        """
        for model in (self._model, _FALLBACK_MODEL):
            try:
                response = await self._client.aio.models.generate_content(
                    model=model,
                    contents=prompt,
                    config=genai_types.GenerateContentConfig(
                        system_instruction=_SYSTEM_PROMPT_WITH_TOOLS,
                        temperature=temperature,
                        tools=[session],
                        automatic_function_calling=genai_types.AutomaticFunctionCallingConfig(
                            maximum_remote_calls=max_tool_calls,
                        ),
                    ),
                )
                # 提取工具调用历史 + 完整推理链
                tool_calls: list[str] = []
                chain: list[dict] = []
                history = getattr(
                    response, "automatic_function_calling_history", None,
                )
                if history:
                    for entry in history:
                        for part in getattr(entry, "parts", []):
                            # Gemini 的文本推理
                            text = getattr(part, "text", None)
                            if text and text.strip():
                                chain.append({
                                    "type": "thought",
                                    "content": text.strip(),
                                })
                            # 工具调用请求
                            fc = getattr(part, "function_call", None)
                            if fc:
                                args = dict(fc.args or {})
                                summary = f"{fc.name}({', '.join(f'{k}={v!r}' for k, v in args.items())})"
                                tool_calls.append(summary)
                                chain.append({
                                    "type": "tool_call",
                                    "name": fc.name,
                                    "args": args,
                                })
                            # 工具返回结果
                            fr = getattr(part, "function_response", None)
                            if fr:
                                content = str(getattr(fr, "response", ""))
                                chain.append({
                                    "type": "tool_result",
                                    "name": getattr(fr, "name", ""),
                                    "content": content[:500] if len(content) > 500 else content,
                                })
                return (
                    response.text or "",
                    model,
                    tuple(tool_calls),
                    tuple(chain),
                )
            except genai_errors.ServerError:
                if model == self._model and model != _FALLBACK_MODEL:
                    logger.warning(
                        "%s 不可用 (503)，降级到 %s",
                        model,
                        _FALLBACK_MODEL,
                    )
                    continue
                raise
        raise RuntimeError("所有模型均不可用")  # pragma: no cover

    async def challenge_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强质询。Gemini 可自主导航代码库。

        Parameters
        ----------
        subject : str
            质询目标。
        context : str
            额外上下文。
        session : mcp ClientSession | None
            MCP 会话。None 时自动连接 Serena。
        max_tool_calls : int
            最大工具调用次数。
        """
        prompt = _CHALLENGE_TEMPLATE.format(subject=subject, context=context)
        if session is not None:
            text, model_used, calls, chain = await self._call_with_tools_and_fallback(
                prompt, 0.3, session, max_tool_calls,
            )
            return ChallengeResult(
                mode="challenge",
                subject=subject,
                response=text,
                model=model_used,
                tool_calls=calls,
                reasoning_chain=chain,
            )
        # 自动连接 Serena
        from newchan.mcp_bridge import SerenaConfig, mcp_session

        async with mcp_session(SerenaConfig()) as sess:
            text, model_used, calls, chain = await self._call_with_tools_and_fallback(
                prompt, 0.3, sess, max_tool_calls,
            )
        return ChallengeResult(
            mode="challenge",
            subject=subject,
            response=text,
            model=model_used,
            tool_calls=calls,
            reasoning_chain=chain,
        )

    async def verify_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强验证。Gemini 可自主导航代码库。

        Parameters
        ----------
        subject : str
            验证目标。
        context : str
            额外上下文。
        session : mcp ClientSession | None
            MCP 会话。None 时自动连接 Serena。
        max_tool_calls : int
            最大工具调用次数。
        """
        prompt = _VERIFY_TEMPLATE.format(subject=subject, context=context)
        if session is not None:
            text, model_used, calls, chain = await self._call_with_tools_and_fallback(
                prompt, 0.1, session, max_tool_calls,
            )
            return ChallengeResult(
                mode="verify",
                subject=subject,
                response=text,
                model=model_used,
                tool_calls=calls,
                reasoning_chain=chain,
            )
        from newchan.mcp_bridge import SerenaConfig, mcp_session

        async with mcp_session(SerenaConfig()) as sess:
            text, model_used, calls, chain = await self._call_with_tools_and_fallback(
                prompt, 0.1, sess, max_tool_calls,
            )
        return ChallengeResult(
            mode="verify",
            subject=subject,
            response=text,
            model=model_used,
            tool_calls=calls,
            reasoning_chain=chain,
        )


# ── 模块级便捷函数 ──

_default_challenger: GeminiChallenger | None = None


def _get_challenger() -> GeminiChallenger:
    global _default_challenger
    if _default_challenger is None:
        _default_challenger = GeminiChallenger()
    return _default_challenger


def challenge(subject: str, context: str = "") -> ChallengeResult:
    """模块级质询（纯文本模式）。"""
    return _get_challenger().challenge(subject, context)


def verify(subject: str, context: str = "") -> ChallengeResult:
    """模块级验证（纯文本模式）。"""
    return _get_challenger().verify(subject, context)


async def achallenge(
    subject: str,
    context: str = "",
    *,
    max_tool_calls: int = 20,
) -> ChallengeResult:
    """模块级 MCP 工具增强质询（async，自动连接 Serena）。"""
    return await _get_challenger().challenge_with_tools(
        subject, context, max_tool_calls=max_tool_calls,
    )


async def averify(
    subject: str,
    context: str = "",
    *,
    max_tool_calls: int = 20,
) -> ChallengeResult:
    """模块级 MCP 工具增强验证（async，自动连接 Serena）。"""
    return await _get_challenger().verify_with_tools(
        subject, context, max_tool_calls=max_tool_calls,
    )


# ── CLI 入口 ──

if __name__ == "__main__":
    import argparse
    import sys

    from dotenv import load_dotenv

    load_dotenv()

    parser = argparse.ArgumentParser(description="Gemini 质询工位 CLI")
    parser.add_argument(
        "mode",
        choices=["challenge", "verify"],
        help="质询模式",
    )
    parser.add_argument(
        "subject",
        help="质询/验证目标",
    )
    parser.add_argument(
        "--context",
        default="",
        help="上下文信息",
    )
    parser.add_argument(
        "--context-file",
        default=None,
        help="从文件读取上下文",
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
    args = parser.parse_args()

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
        # MCP 工具模式（async）
        async def _run() -> ChallengeResult:
            if args.mode == "challenge":
                return await challenger.challenge_with_tools(
                    args.subject, ctx, max_tool_calls=args.max_tool_calls,
                )
            return await challenger.verify_with_tools(
                args.subject, ctx, max_tool_calls=args.max_tool_calls,
            )

        result = asyncio.run(_run())
    else:
        # 纯文本模式（sync）
        if args.mode == "challenge":
            result = challenger.challenge(args.subject, ctx)
        else:
            result = challenger.verify(args.subject, ctx)

    print(f"[{result.mode}] model={result.model}")
    if result.tool_calls:
        print(f"tool_calls ({len(result.tool_calls)}):")
        for tc in result.tool_calls:
            print(f"  → {tc}")
    if args.verbose and result.reasoning_chain:
        print()
        print("=== Gemini 推理链 ===")
        for i, step in enumerate(result.reasoning_chain, 1):
            stype = step["type"]
            if stype == "thought":
                print(f"[{i}] 💭 {step['content']}")
            elif stype == "tool_call":
                args_str = ", ".join(
                    f"{k}={v!r}" for k, v in step.get("args", {}).items()
                )
                print(f"[{i}] 🔍 {step['name']}({args_str})")
            elif stype == "tool_result":
                content = step.get("content", "")
                preview = content[:200] + "..." if len(content) > 200 else content
                print(f"    → {step.get('name', '')}: {preview}")
        print()
    print("=" * 60)
    print(result.response)
