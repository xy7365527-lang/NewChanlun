"""newchan.codex — Codex 代码层异质审查工位模块化包。

公共接口：
- CodexChallenger: 核心类
- ReviewResult: 结果数据类
- review/diagnose/decide: 同步便捷函数

概念溯源: [新缠论] — 155号谱系：代码层异质审查（OpenAI Codex）
"""

from newchan.codex.modes import (
    CodexChallenger,
    ReviewResult,
    decide,
    diagnose,
    review,
)

__all__ = [
    "CodexChallenger",
    "ReviewResult",
    "review",
    "diagnose",
    "decide",
]
