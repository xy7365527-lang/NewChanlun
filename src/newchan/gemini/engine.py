"""核心调用引擎 — API fallback 循环、推理链提取。

纯逻辑模块：不直接导入 genai，所有外部依赖通过参数传入。

概念溯源: [新缠论] — 异质模型质询
"""

from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

_MODEL = "gemini-3-pro-preview"
_FALLBACK_MODEL = "gemini-2.5-pro"


def call_with_fallback(
    client: object,
    model: str,
    prompt: str,
    temperature: float,
    system_prompt: str,
    genai_module: object,
    genai_errors_module: object,
) -> tuple[str, str]:
    """调用 Gemini API，主模型 503 时自动降级到 fallback。

    Parameters
    ----------
    client : genai.Client
    model : str
    prompt : str
    temperature : float
    system_prompt : str
    genai_module : google.genai (用于 types.GenerateContentConfig)
    genai_errors_module : google.genai.errors

    Returns (response_text, actual_model_used)。
    """
    for m in (model, _FALLBACK_MODEL):
        try:
            response = client.models.generate_content(
                model=m,
                contents=prompt,
                config=genai_module.types.GenerateContentConfig(
                    system_instruction=system_prompt,
                    temperature=temperature,
                ),
            )
            return response.text or "", m
        except (
            genai_errors_module.ServerError,
            genai_errors_module.ClientError,
        ):
            if m == model and m != _FALLBACK_MODEL:
                logger.warning(
                    "%s 不可用，降级到 %s", m, _FALLBACK_MODEL,
                )
                continue
            raise
    raise RuntimeError("所有模型均不可用")  # pragma: no cover


async def call_with_tools_and_fallback(
    client: object,
    model: str,
    prompt: str,
    temperature: float,
    session: object,
    max_tool_calls: int,
    system_prompt: str,
    genai_types_module: object,
    genai_errors_module: object,
) -> tuple[str, str, tuple[str, ...], tuple[dict, ...]]:
    """Gemini + MCP 自动 function calling 循环。

    Returns (response_text, actual_model, tool_call_summaries, reasoning_chain)。
    """
    for m in (model, _FALLBACK_MODEL):
        try:
            response = await client.aio.models.generate_content(
                model=m,
                contents=prompt,
                config=genai_types_module.GenerateContentConfig(
                    system_instruction=system_prompt,
                    temperature=temperature,
                    tools=[session],
                    automatic_function_calling=genai_types_module.AutomaticFunctionCallingConfig(
                        maximum_remote_calls=max_tool_calls,
                    ),
                ),
            )
            tool_calls, chain = extract_reasoning_chain(response)
            return (
                response.text or "",
                m,
                tuple(tool_calls),
                tuple(chain),
            )
        except (
            genai_errors_module.ServerError,
            genai_errors_module.ClientError,
        ):
            if m == model and m != _FALLBACK_MODEL:
                logger.warning(
                    "%s 不可用，降级到 %s", m, _FALLBACK_MODEL,
                )
                continue
            raise
    raise RuntimeError("所有模型均不可用")  # pragma: no cover


def extract_reasoning_chain(
    response: object,
) -> tuple[list[str], list[dict]]:
    """从 Gemini 响应中提取工具调用历史和推理链。"""
    tool_calls: list[str] = []
    chain: list[dict] = []
    history = getattr(
        response, "automatic_function_calling_history", None,
    )
    if not history:
        return tool_calls, chain

    for entry in history:
        for part in getattr(entry, "parts", []):
            text = getattr(part, "text", None)
            if text and text.strip():
                chain.append({
                    "type": "thought",
                    "content": text.strip(),
                })
            fc = getattr(part, "function_call", None)
            if fc:
                args = dict(fc.args or {})
                summary = (
                    f"{fc.name}("
                    f"{', '.join(f'{k}={v!r}' for k, v in args.items())})"
                )
                tool_calls.append(summary)
                chain.append({
                    "type": "tool_call",
                    "name": fc.name,
                    "args": args,
                })
            fr = getattr(part, "function_response", None)
            if fr:
                content = str(getattr(fr, "response", ""))
                chain.append({
                    "type": "tool_result",
                    "name": getattr(fr, "name", ""),
                    "content": (
                        content[:500] if len(content) > 500 else content
                    ),
                })
    return tool_calls, chain
