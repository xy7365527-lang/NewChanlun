"""核心调用引擎 — Anthropic API 调用、重试、结果解析。

纯逻辑模块：anthropic 客户端通过参数或环境变量配置。

概念溯源: [新缠论] — 异质审计引擎
"""

from __future__ import annotations

import json
import logging
import os
import time
from dataclasses import dataclass, field
from typing import Any

import anthropic

from newchan.claude_audit.modes import AuditType, get_mode_config

logger = logging.getLogger(__name__)

_DEFAULT_MODEL = "claude-sonnet-4-6-20250514"
_MAX_RETRIES = 3
_RETRY_BASE_DELAY = 2  # 秒，指数退避基数


@dataclass(frozen=True, slots=True)
class AuditResult:
    """审计结果。"""

    audit_type: str
    task: str
    response_text: str
    model: str
    parsed: dict[str, Any] = field(default_factory=dict)


def _create_client(api_key: str | None = None) -> anthropic.Anthropic:
    """创建 Anthropic 客户端。

    Raises
    ------
    ValueError
        ANTHROPIC_API_KEY 未设置且未传入 api_key。
    """
    key = api_key or os.environ.get("ANTHROPIC_API_KEY", "")
    if not key:
        raise ValueError(
            "ANTHROPIC_API_KEY 未设置。"
            "请设置环境变量或传入 api_key 参数。"
        )
    return anthropic.Anthropic(api_key=key)


def _try_parse_json(text: str) -> dict[str, Any]:
    """尝试从响应中解析 JSON。

    支持 Markdown JSON 代码块包裹的情况。
    解析失败时返回 {"raw": text}。
    """
    stripped = text.strip()
    # 去除 markdown ```json ... ``` 包裹
    if stripped.startswith("```"):
        lines = stripped.split("\n")
        # 去除首行 ```json 和末行 ```
        if len(lines) >= 3:
            inner = "\n".join(lines[1:-1]).strip()
            try:
                return json.loads(inner)
            except (json.JSONDecodeError, ValueError):
                pass
    try:
        return json.loads(stripped)
    except (json.JSONDecodeError, ValueError):
        return {"raw": text}


class ClaudeAuditor:
    """Claude 异质审计引擎。

    Parameters
    ----------
    api_key : str | None
        Anthropic API key。None 时从 ANTHROPIC_API_KEY 环境变量读取。
    model : str
        模型名称，默认 claude-sonnet-4-6-20250514。
    """

    def __init__(
        self,
        api_key: str | None = None,
        model: str = _DEFAULT_MODEL,
    ) -> None:
        self._client = _create_client(api_key)
        self._model = model

    def audit(
        self,
        task: str,
        context: str,
        audit_type: AuditType,
    ) -> AuditResult:
        """执行审计。

        Parameters
        ----------
        task : str
            审计任务描述。
        context : str
            需要审计的代码/文本。
        audit_type : AuditType
            审计类型。

        Returns
        -------
        AuditResult
            审计结果。
        """
        cfg = get_mode_config(audit_type)
        user_message = f"## Audit Task\n{task}\n\n## Context\n{context}"

        last_err: Exception | None = None
        for attempt in range(_MAX_RETRIES):
            try:
                response = self._client.messages.create(
                    model=self._model,
                    max_tokens=cfg.max_tokens,
                    temperature=cfg.temperature,
                    system=cfg.system_prompt,
                    messages=[{"role": "user", "content": user_message}],
                )
                text = response.content[0].text if response.content else ""
                parsed = _try_parse_json(text)
                return AuditResult(
                    audit_type=audit_type,
                    task=task,
                    response_text=text,
                    model=self._model,
                    parsed=parsed,
                )
            except anthropic.APIStatusError as e:
                last_err = e
                if e.status_code in (429, 500, 502, 503, 529):
                    if attempt < _MAX_RETRIES - 1:
                        delay = _RETRY_BASE_DELAY * (2 ** attempt)
                        logger.warning(
                            "Claude API %d (attempt %d/%d), %d秒后重试",
                            e.status_code, attempt + 1, _MAX_RETRIES, delay,
                        )
                        time.sleep(delay)
                        continue
                raise
            except anthropic.APIConnectionError as e:
                last_err = e
                if attempt < _MAX_RETRIES - 1:
                    delay = _RETRY_BASE_DELAY * (2 ** attempt)
                    logger.warning(
                        "Claude API 连接失败 (attempt %d/%d), %d秒后重试",
                        attempt + 1, _MAX_RETRIES, delay,
                    )
                    time.sleep(delay)
                    continue
                raise

        if last_err is not None:
            raise last_err
        raise RuntimeError("Unreachable")  # pragma: no cover
