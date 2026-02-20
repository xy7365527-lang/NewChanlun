"""各模式的具体实现 — GeminiChallenger 类 + 模块级便捷函数。

prompt 构建、输出解析、sync/async 调用入口。
genai 在此模块级导入，测试通过 patch("newchan.gemini.modes.genai") 或
通过兼容垫片 patch("newchan.gemini_challenger.genai") 拦截。

概念溯源: [新缠论] — 异质模型质询 + 编排者代理 + 形式推导
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Literal

from google import genai
from google.genai import errors as genai_errors, types as genai_types

from newchan.gemini.engine import (
    _MODEL,
    call_with_fallback,
    call_with_tools_and_fallback,
)
from newchan.gemini.registry import ModeKey, get_mode_config


@dataclass(frozen=True, slots=True)
class ChallengeResult:
    """质询结果。"""

    mode: Literal["challenge", "verify", "decide", "derive"]
    subject: str
    response: str
    model: str
    tool_calls: tuple[str, ...] = ()
    reasoning_chain: tuple[dict, ...] = ()


def _create_client(api_key: str | None = None) -> genai.Client:
    """创建 Gemini 客户端，使用模块级 genai 引用（可被 mock 替换）。"""
    import newchan.gemini.modes as _self

    key = api_key or os.environ.get("GOOGLE_API_KEY", "")
    if not key:
        raise ValueError(
            "GOOGLE_API_KEY 未设置。"
            "请在 .env 中设置或传入 api_key 参数。"
        )
    return _self.genai.Client(api_key=key)


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
        self._client = _create_client(api_key)
        self._model = model

    def _get_genai_refs(self) -> tuple[object, object]:
        """获取当前模块级 genai 和 genai_errors 引用（支持 mock）。"""
        import newchan.gemini.modes as _self

        return _self.genai, _self.genai_errors

    # ── 纯文本模式（sync） ──

    def _run_mode(
        self,
        mode: ModeKey,
        subject: str,
        context: str,
        *,
        extra_template_kwargs: dict | None = None,
    ) -> ChallengeResult:
        """通用同步模式执行。"""
        cfg = get_mode_config(mode)
        fmt_kwargs = {"subject": subject, "context": context}
        if extra_template_kwargs:
            fmt_kwargs = {**fmt_kwargs, **extra_template_kwargs}
        prompt = cfg.template.format(**fmt_kwargs)
        genai_mod, genai_err = self._get_genai_refs()
        text, model_used = call_with_fallback(
            self._client, self._model, prompt,
            cfg.temperature, cfg.system_prompt,
            genai_mod, genai_err,
        )
        return ChallengeResult(
            mode=mode, subject=subject,
            response=text, model=model_used,
        )

    def challenge(self, subject: str, context: str = "") -> ChallengeResult:
        """对给定主题发起质询。"""
        return self._run_mode("challenge", subject, context)

    def verify(self, subject: str, context: str = "") -> ChallengeResult:
        """验证给定断言是否成立。"""
        return self._run_mode("verify", subject, context)

    def decide(self, subject: str, context: str = "") -> ChallengeResult:
        """编排者代理决策（纯文本模式）。"""
        return self._run_mode("decide", subject, context)

    def derive(
        self,
        subject: str,
        context: str = "",
        domain: str = "General Mathematics",
    ) -> ChallengeResult:
        """形式推导（纯文本模式）。"""
        return self._run_mode(
            "derive", subject, context,
            extra_template_kwargs={"domain": domain},
        )

    # ── MCP 工具模式（async） ──

    async def _run_mode_with_tools(
        self,
        mode: ModeKey,
        subject: str,
        context: str,
        session: object | None,
        max_tool_calls: int,
        *,
        extra_template_kwargs: dict | None = None,
    ) -> ChallengeResult:
        """通用异步工具模式执行。"""
        cfg = get_mode_config(mode)
        fmt_kwargs = {"subject": subject, "context": context}
        if extra_template_kwargs:
            fmt_kwargs = {**fmt_kwargs, **extra_template_kwargs}
        prompt = cfg.template.format(**fmt_kwargs)
        _, genai_err = self._get_genai_refs()

        async def _do_call(sess: object) -> ChallengeResult:
            text, model_used, calls, chain = (
                await call_with_tools_and_fallback(
                    self._client, self._model, prompt,
                    cfg.temperature, sess, max_tool_calls,
                    cfg.system_prompt_with_tools,
                    genai_types, genai_err,
                )
            )
            return ChallengeResult(
                mode=mode, subject=subject, response=text,
                model=model_used, tool_calls=calls,
                reasoning_chain=chain,
            )

        if session is not None:
            return await _do_call(session)

        from newchan.mcp_bridge import SerenaConfig, mcp_session

        async with mcp_session(SerenaConfig()) as sess:
            return await _do_call(sess)

    async def challenge_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强质询。"""
        return await self._run_mode_with_tools(
            "challenge", subject, context, session, max_tool_calls,
        )

    async def verify_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强验证。"""
        return await self._run_mode_with_tools(
            "verify", subject, context, session, max_tool_calls,
        )

    async def decide_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强编排者决策。"""
        return await self._run_mode_with_tools(
            "decide", subject, context, session, max_tool_calls,
        )

    async def derive_with_tools(
        self,
        subject: str,
        context: str = "",
        domain: str = "General Mathematics",
        *,
        session: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强形式推导。"""
        return await self._run_mode_with_tools(
            "derive", subject, context, session, max_tool_calls,
            extra_template_kwargs={"domain": domain},
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


def decide(subject: str, context: str = "") -> ChallengeResult:
    """模块级编排者代理决策（纯文本模式）。"""
    return _get_challenger().decide(subject, context)


def derive(
    subject: str, context: str = "", domain: str = "General Mathematics",
) -> ChallengeResult:
    """模块级形式推导（纯文本模式）。"""
    return _get_challenger().derive(subject, context, domain)


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


async def adecide(
    subject: str,
    context: str = "",
    *,
    max_tool_calls: int = 20,
) -> ChallengeResult:
    """模块级 MCP 工具增强决策（async，自动连接 Serena）。"""
    return await _get_challenger().decide_with_tools(
        subject, context, max_tool_calls=max_tool_calls,
    )


async def aderive(
    subject: str,
    context: str = "",
    domain: str = "General Mathematics",
    *,
    max_tool_calls: int = 20,
) -> ChallengeResult:
    """模块级 MCP 工具增强形式推导（async，自动连接 Serena）。"""
    return await _get_challenger().derive_with_tools(
        subject, context, domain, max_tool_calls=max_tool_calls,
    )
