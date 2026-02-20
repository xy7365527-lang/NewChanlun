"""newchan.gemini — Gemini 异质质询工位模块化包。

公共接口：
- GeminiChallenger: 核心类
- ChallengeResult: 结果数据类
- challenge/verify/decide/derive: 同步便捷函数
- achallenge/averify/adecide/aderive: 异步便捷函数

概念溯源: [新缠论] — 异质模型质询 + 编排者代理 + 形式推导
"""

from newchan.gemini.modes import (
    ChallengeResult,
    GeminiChallenger,
    achallenge,
    adecide,
    aderive,
    averify,
    challenge,
    decide,
    derive,
    verify,
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
