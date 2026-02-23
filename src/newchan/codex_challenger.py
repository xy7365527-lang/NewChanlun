"""Codex 代码层异质审查工位 — 向后兼容垫片。

此文件将所有公共 API 委托给 newchan.codex 包。
保留 `openai` 模块级属性以兼容 `patch("newchan.codex_challenger.openai")`。
当 openai 被 mock 替换时，自动同步到 newchan.codex.modes.openai。

python -m newchan.codex_challenger 仍可用（见底部 __main__ 块）。

概念溯源: [新缠论] — 155号谱系：代码层异质审查（OpenAI Codex）
"""

from __future__ import annotations

import sys

# 保留 openai 在模块级，使 patch("newchan.codex_challenger.openai") 生效。
import openai  # noqa: F401

from newchan.codex.modes import (  # noqa: F401
    CodexChallenger,
    ReviewResult,
)

__all__ = [
    "CodexChallenger",
    "ReviewResult",
    "review",
    "diagnose",
    "decide",
]

# ── 模块级便捷函数（本地实现，支持 patch.object 测试模式） ──

_default_challenger: CodexChallenger | None = None


def _get_challenger() -> CodexChallenger:
    global _default_challenger
    if _default_challenger is None:
        _this = sys.modules[__name__]
        _default_challenger = _this.CodexChallenger()
    return _default_challenger


def review(subject: str, context: str = "") -> ReviewResult:
    """模块级代码审查。"""
    return _get_challenger().review(subject, context)


def diagnose(subject: str, context: str = "") -> ReviewResult:
    """模块级根因诊断。"""
    return _get_challenger().diagnose(subject, context)


def decide(subject: str, context: str = "") -> ReviewResult:
    """模块级技术选型决策。"""
    return _get_challenger().decide(subject, context)


# ── 使 patch("newchan.codex_challenger.openai") 同步到 modes 模块 ──

_this = sys.modules[__name__]
_OrigModuleType = type(_this)


class _PatchProxyModule(_OrigModuleType):
    """当 openai 属性被替换时，同步到 newchan.codex.modes。"""

    def __setattr__(self, name: str, value: object) -> None:
        super().__setattr__(name, value)
        if name == "openai":
            import newchan.codex.modes as _modes

            _modes.openai = value  # type: ignore[attr-defined]


_this.__class__ = _PatchProxyModule


if __name__ == "__main__":
    from newchan.codex.__main__ import main

    main()
