"""核心调用引擎 — OpenAI Responses API + fallback 循环、推理链提取。

底层模型 = OpenAI GPT-5.5（"gemini" 是异质质询工位的角色名，非模型名）。
异质源从 Google Gemini 迁移到 OpenAI GPT-5.5（编排者 2026-06-23 指令：
Gemini API 429 RESOURCE_EXHAUSTED 不可用 → 换装 gpt-5.5 最高级别最高推理）。

纯逻辑模块：不直接导入 openai，所有外部依赖通过参数传入。

概念溯源: [新缠论] — 异质模型质询（OpenAI GPT-5.5）
"""

from __future__ import annotations

import json
import logging

logger = logging.getLogger(__name__)

# 最高级别（pro 层）+ 最高推理（xhigh），编排者 2026-06-23 指令。
# pro 不可用/超时时降级到标准 gpt-5.5（同样 xhigh）。
_MODEL = "gpt-5.5-pro"
_FALLBACK_MODEL = "gpt-5.5"


def call_with_fallback(
    client: object,
    model: str,
    prompt: str,
    reasoning_effort: str,
    system_prompt: str,
    openai_module: object,
) -> tuple[str, str]:
    """调用 OpenAI Responses API，主模型 API 错误时自动降级到 fallback。

    Parameters
    ----------
    client : openai.OpenAI
    model : str
    prompt : str
    reasoning_effort : str
        "none", "low", "medium", "high", or "xhigh"
    system_prompt : str
    openai_module : openai (用于 openai.APIError)

    Returns (response_text, actual_model_used)。
    """
    api_error_cls = getattr(openai_module, "APIError", Exception)

    for m in (model, _FALLBACK_MODEL):
        try:
            response = client.responses.create(
                model=m,
                instructions=system_prompt,
                input=prompt,
                reasoning={"effort": reasoning_effort},
            )
            return _extract_text(response), m
        except api_error_cls:
            if m == model and m != _FALLBACK_MODEL:
                logger.warning(
                    "%s 不可用，降级到 %s", m, _FALLBACK_MODEL,
                )
                continue
            raise
    raise RuntimeError("所有模型均不可用")  # pragma: no cover


async def call_with_tools_and_fallback(
    async_client: object,
    model: str,
    prompt: str,
    reasoning_effort: str,
    bridge: object,
    max_tool_calls: int,
    system_prompt: str,
    openai_module: object,
) -> tuple[str, str, tuple[str, ...], tuple[dict, ...]]:
    """OpenAI Responses API + MCP 手动 function calling 循环。

    google-genai 可直接接收 MCP ClientSession 做自动 function calling；
    OpenAI Responses API 对本地 stdio MCP 不支持直接传 session，
    因此走 mcp_bridge 的手动 dispatch 路径（get_tools / call_tool）。

    主模型 API 错误时降级到 fallback。

    Parameters
    ----------
    async_client : openai.AsyncOpenAI
    bridge : McpBridge（已连接），提供 get_tools() 与 call_tool()

    Returns (response_text, actual_model, tool_call_summaries, reasoning_chain)。
    """
    api_error_cls = getattr(openai_module, "APIError", Exception)

    tool_defs = await bridge.get_tools()
    openai_tools = [_to_openai_tool(t) for t in tool_defs]

    for m in (model, _FALLBACK_MODEL):
        try:
            return await _run_tool_loop(
                async_client, m, prompt, reasoning_effort,
                system_prompt, openai_tools, bridge, max_tool_calls,
            )
        except api_error_cls:
            if m == model and m != _FALLBACK_MODEL:
                logger.warning(
                    "%s 不可用，降级到 %s", m, _FALLBACK_MODEL,
                )
                continue
            raise
    raise RuntimeError("所有模型均不可用")  # pragma: no cover


def _to_openai_tool(tool_def: object) -> dict:
    """将 mcp_bridge.ToolDefinition 转换为 OpenAI function tool schema。"""
    return {
        "type": "function",
        "name": getattr(tool_def, "name", ""),
        "description": getattr(tool_def, "description", ""),
        "parameters": getattr(tool_def, "parameters", None) or {
            "type": "object",
            "properties": {},
        },
    }


async def _run_tool_loop(
    client: object,
    model: str,
    prompt: str,
    reasoning_effort: str,
    system_prompt: str,
    tools: list[dict],
    bridge: object,
    max_tool_calls: int,
) -> tuple[str, str, tuple[str, ...], tuple[dict, ...]]:
    """单模型 function-calling 循环：调用 → 执行工具 → 回填 → 直到无工具调用。"""
    input_items: list[dict] = [{"role": "user", "content": prompt}]
    chain: list[dict] = []
    tool_calls: list[str] = []
    calls_made = 0
    text = ""
    prev_id: str | None = None

    while True:
        response = await client.responses.create(
            model=model,
            instructions=system_prompt,
            input=input_items,
            reasoning={"effort": reasoning_effort, "summary": "auto"},
            tools=tools,
            previous_response_id=prev_id,
        )
        prev_id = getattr(response, "id", None)

        function_calls = []
        for item in getattr(response, "output", []) or []:
            itype = getattr(item, "type", "")
            if itype == "reasoning":
                for summ in getattr(item, "summary", []) or []:
                    stext = getattr(summ, "text", "")
                    if stext and stext.strip():
                        chain.append({
                            "type": "thought",
                            "content": stext.strip(),
                        })
            elif itype == "function_call":
                function_calls.append(item)

        if not function_calls:
            text = _extract_text(response)
            break

        # 用 previous_response_id 续接：下一轮只发送新的工具输出。
        input_items = []
        for fc in function_calls:
            if calls_made >= max_tool_calls:
                break
            calls_made += 1
            raw_args = getattr(fc, "arguments", "") or "{}"
            try:
                args = json.loads(raw_args)
            except (ValueError, TypeError):
                args = {}
            name = getattr(fc, "name", "")
            summary = (
                f"{name}("
                f"{', '.join(f'{k}={v!r}' for k, v in args.items())})"
            )
            tool_calls.append(summary)
            chain.append({"type": "tool_call", "name": name, "args": args})

            result = await bridge.call_tool(name, args)
            content = result.content if hasattr(result, "content") else str(result)
            chain.append({
                "type": "tool_result",
                "name": name,
                "content": content[:500] if len(content) > 500 else content,
            })
            input_items.append({
                "type": "function_call_output",
                "call_id": getattr(fc, "call_id", ""),
                "output": content,
            })

        if calls_made >= max_tool_calls:
            # 工具预算耗尽：再做一次无工具调用，逼模型给出最终文本结论。
            final = await client.responses.create(
                model=model,
                instructions=system_prompt,
                input=input_items,
                reasoning={"effort": reasoning_effort},
                previous_response_id=prev_id,
            )
            text = _extract_text(final)
            break

    return text, model, tuple(tool_calls), tuple(chain)


def _extract_text(response: object) -> str:
    """从 Responses API 返回对象中提取文本。"""
    # SDK 提供 output_text 便捷属性时优先用。
    output_text = getattr(response, "output_text", None)
    if isinstance(output_text, str) and output_text:
        return output_text

    output = getattr(response, "output", None)
    if not output:
        return ""
    parts: list[str] = []
    for item in output:
        if getattr(item, "type", "") == "message":
            for block in getattr(item, "content", []) or []:
                if getattr(block, "type", "") == "output_text":
                    parts.append(getattr(block, "text", ""))
    return "\n".join(parts) if parts else ""
