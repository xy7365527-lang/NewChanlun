"""核心调用引擎 — OpenAI Responses API + fallback 循环。

纯逻辑模块：不直接导入 openai，所有外部依赖通过参数传入。

概念溯源: [新缠论] — 155号谱系：代码层异质审查
"""

from __future__ import annotations

import logging

logger = logging.getLogger(__name__)

_MODEL = "gpt-5.3-codex"
_FALLBACK_MODEL = "gpt-5.2-codex"


def call_with_fallback(
    client: object,
    model: str,
    prompt: str,
    system_prompt: str,
    reasoning_effort: str,
    openai_module: object,
) -> tuple[str, str]:
    """调用 OpenAI Responses API，主模型 5xx 时自动降级到 fallback。

    Parameters
    ----------
    client : openai.OpenAI
    model : str
    prompt : str
    system_prompt : str
    reasoning_effort : str
        "none", "minimal", "low", "medium", "high", or "xhigh"
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
            text = _extract_text(response)
            return text, m
        except api_error_cls:
            if m == model and m != _FALLBACK_MODEL:
                logger.warning(
                    "%s 不可用，降级到 %s", m, _FALLBACK_MODEL,
                )
                continue
            raise
    raise RuntimeError("所有模型均不可用")  # pragma: no cover


def _extract_text(response: object) -> str:
    """从 Responses API 返回对象中提取文本。"""
    output = getattr(response, "output", None)
    if not output:
        return ""
    parts: list[str] = []
    for item in output:
        item_type = getattr(item, "type", "")
        if item_type == "message":
            content = getattr(item, "content", [])
            for block in content:
                block_type = getattr(block, "type", "")
                if block_type == "output_text":
                    parts.append(getattr(block, "text", ""))
    return "\n".join(parts) if parts else ""
