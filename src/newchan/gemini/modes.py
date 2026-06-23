"""各模式的具体实现 — GeminiChallenger 类 + 模块级便捷函数。

底层模型 = OpenAI GPT-5.5（"gemini" 是异质质询工位的角色名，非模型名）。
prompt 构建、输出解析、sync/async 调用入口。
openai 在此模块级导入，测试通过 patch("newchan.gemini.modes.openai") 或
通过兼容垫片 patch("newchan.gemini_challenger.openai") 拦截。

概念溯源: [新缠论] — 异质模型质询（OpenAI GPT-5.5）+ 编排者代理 + 形式推导
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Literal

import openai

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


def _resolve_key(api_key: str | None = None) -> str:
    """解析 OpenAI API Key。缺失时 fail-loud。"""
    key = api_key or os.environ.get("OPENAI_API_KEY", "")
    if not key:
        raise ValueError(
            "OPENAI_API_KEY 未设置。"
            "请在 .env 中设置或传入 api_key 参数。"
        )
    return key


class GeminiChallenger:
    """异质质询工位的核心类（底层模型 = OpenAI GPT-5.5）。

    Parameters
    ----------
    api_key : str | None
        OpenAI API Key。None 时从 OPENAI_API_KEY 环境变量读取。
    model : str
        模型名称，默认 gpt-5.5-pro（pro 不可用时引擎降级到 gpt-5.5）。
    """

    def __init__(
        self,
        api_key: str | None = None,
        model: str = _MODEL,
    ) -> None:
        import newchan.gemini.modes as _self

        self._api_key = _resolve_key(api_key)
        self._client = _self.openai.OpenAI(api_key=self._api_key)
        self._model = model

    def _get_openai_ref(self) -> object:
        """获取当前模块级 openai 引用（支持 mock）。"""
        import newchan.gemini.modes as _self

        return _self.openai

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
        openai_mod = self._get_openai_ref()
        text, model_used = call_with_fallback(
            self._client, self._model, prompt,
            cfg.reasoning_effort, cfg.system_prompt,
            openai_mod,
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
        bridge: object | None,
        max_tool_calls: int,
        *,
        extra_template_kwargs: dict | None = None,
    ) -> ChallengeResult:
        """通用异步工具模式执行（OpenAI + Serena MCP 手动 dispatch）。"""
        cfg = get_mode_config(mode)
        fmt_kwargs = {"subject": subject, "context": context}
        if extra_template_kwargs:
            fmt_kwargs = {**fmt_kwargs, **extra_template_kwargs}
        prompt = cfg.template.format(**fmt_kwargs)
        openai_mod = self._get_openai_ref()
        async_client = openai_mod.AsyncOpenAI(api_key=self._api_key)

        async def _do_call(br: object) -> ChallengeResult:
            text, model_used, calls, chain = (
                await call_with_tools_and_fallback(
                    async_client, self._model, prompt,
                    cfg.reasoning_effort, br, max_tool_calls,
                    cfg.system_prompt_with_tools, openai_mod,
                )
            )
            return ChallengeResult(
                mode=mode, subject=subject, response=text,
                model=model_used, tool_calls=calls,
                reasoning_chain=chain,
            )

        if bridge is not None:
            return await _do_call(bridge)

        from newchan.mcp_bridge import McpBridge, SerenaConfig

        async with McpBridge(SerenaConfig()) as br:
            return await _do_call(br)

    async def challenge_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        bridge: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强质询。"""
        return await self._run_mode_with_tools(
            "challenge", subject, context, bridge, max_tool_calls,
        )

    async def verify_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        bridge: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强验证。"""
        return await self._run_mode_with_tools(
            "verify", subject, context, bridge, max_tool_calls,
        )

    async def decide_with_tools(
        self,
        subject: str,
        context: str = "",
        *,
        bridge: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强编排者决策。"""
        return await self._run_mode_with_tools(
            "decide", subject, context, bridge, max_tool_calls,
        )

    async def derive_with_tools(
        self,
        subject: str,
        context: str = "",
        domain: str = "General Mathematics",
        *,
        bridge: object | None = None,
        max_tool_calls: int = 20,
    ) -> ChallengeResult:
        """MCP 工具增强形式推导。"""
        return await self._run_mode_with_tools(
            "derive", subject, context, bridge, max_tool_calls,
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
