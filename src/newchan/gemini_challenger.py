"""Gemini 异质质询工位 — 向后兼容垫片。

此文件将所有公共 API 委托给 newchan.gemini 包。
保留 `genai` 模块级属性以兼容 `patch("newchan.gemini_challenger.genai")`。
当 genai 被 mock 替换时，自动同步到 newchan.gemini.modes.genai。

保留 _default_challenger / _get_challenger / 便捷函数 以兼容
`patch.object(mod, "GeminiChallenger")` + `mod._default_challenger = None` 测试模式。

python -m newchan.gemini_challenger 仍可用（见底部 __main__ 块）。
"""

from __future__ import annotations

import sys

# 保留 genai 在模块级，使 patch("newchan.gemini_challenger.genai") 生效。
from google import genai  # noqa: F401

from newchan.gemini.modes import (  # noqa: F401
    ChallengeResult,
    GeminiChallenger,
    achallenge,
    adecide,
    aderive,
    averify,
)

__all__ = [
    "GeminiChallenger",
    "ChallengeResult",
    "challenge",
    "verify",
    "decide",
    "derive",
    "achallenge",
    "averify",
    "adecide",
    "aderive",
]

# ── 模块级便捷函数（本地实现，支持 patch.object 测试模式） ──

_default_challenger: GeminiChallenger | None = None


def _get_challenger() -> GeminiChallenger:
    global _default_challenger
    if _default_challenger is None:
        _this = sys.modules[__name__]
        _default_challenger = _this.GeminiChallenger()
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


# ── 使 patch("newchan.gemini_challenger.genai") 同步到 modes 模块 ──

_this = sys.modules[__name__]
_OrigModuleType = type(_this)


class _PatchProxyModule(_OrigModuleType):
    """当 genai 属性被替换时，同步到 newchan.gemini.modes。"""

    def __setattr__(self, name: str, value: object) -> None:
        super().__setattr__(name, value)
        if name == "genai":
            import newchan.gemini.modes as _modes

            _modes.genai = value  # type: ignore[attr-defined]


_this.__class__ = _PatchProxyModule


if __name__ == "__main__":
    from newchan.gemini.__main__ import main

    main()
