"""Codex 代码层异质审查工位 — 向后兼容垫片。

此文件将所有公共 API 委托给 newchan.codex 包。
调用走 codex CLI（ChatGPT 订阅认证），见 newchan.codex.engine。

python -m newchan.codex_challenger 仍可用（见底部 __main__ 块）。

概念溯源: [新缠论] — 155号谱系：代码层异质审查（Codex CLI）
"""

from __future__ import annotations

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
        import sys

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


if __name__ == "__main__":
    from newchan.codex.__main__ import main

    main()
