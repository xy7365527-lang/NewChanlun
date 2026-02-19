"""Gemini 异质质询工位

结构性工位：用 Gemini 3 Pro 对缠论形式化产出提供异质否定。
Gemini 的理解方式与 Claude 根本不同，因此可能产出 Claude 自身
永远产出不了的否定。

操作规则（SKILL.md）：
1. Lead 或指定 agent 调用 challenge() / verify()
2. 返回结果由调用者判断否定是否成立
3. 成立 → 正常矛盾流程（写谱系、走质询循环）
4. 不成立 → 记录为外部工具误判，不进入谱系

本体论位置：030a（生成态）— 不属于成员/工具任何一个已有范畴。

概念溯源: [新缠论] — 异质模型质询
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from typing import Literal

from google import genai

__all__ = [
    "GeminiChallenger",
    "ChallengeResult",
    "challenge",
    "verify",
]

_MODEL = "gemini-3-pro-preview"

_SYSTEM_PROMPT = """\
你是缠论形式化项目的异质质询者。你的任务是从不同角度审视概念定义、\
代码实现、和推理链条，找出可能的矛盾、遗漏、或逻辑漏洞。

关键原则：
- 你不需要认同项目的所有前提，但需要理解它们
- 你的价值在于提供 Claude 可能看不到的否定
- 如果你认为没有问题，明确说"无否定"
- 如果你发现问题，精确描述矛盾：什么跟什么冲突、为什么不可弥合
- 不要客套，不要模糊化，直击要害
"""

_CHALLENGE_TEMPLATE = """\
## 质询目标

{subject}

## 上下文

{context}

## 质询要求

请从以下角度审视：
1. 定义内部一致性：是否自相矛盾？
2. 定义间一致性：是否与其他定义冲突？
3. 逻辑完备性：是否存在未覆盖的边界情况？
4. 实现忠实度：代码是否忠实反映了定义？

如果发现问题，请按以下格式输出：
- **矛盾点**：精确描述
- **冲突方**：A 说什么 vs B 说什么
- **严重性**：致命 / 重要 / 建议
- **建议**：如何解决（如果有）

如果没有发现问题，输出"无否定"并说明你检查了什么。
"""

_VERIFY_TEMPLATE = """\
## 验证目标

{subject}

## 上下文

{context}

## 验证要求

请验证以下断言是否成立：
1. 给定的推理链是否逻辑有效？
2. 前提是否充分支撑结论？
3. 是否存在隐藏假设？

输出格式：
- **结论**：成立 / 不成立 / 部分成立
- **依据**：为什么
- **隐藏假设**：如果有
"""


@dataclass(frozen=True, slots=True)
class ChallengeResult:
    """质询结果。"""

    mode: Literal["challenge", "verify"]
    subject: str
    response: str
    model: str


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
        key = api_key or os.environ.get("GOOGLE_API_KEY", "")
        if not key:
            raise ValueError(
                "GOOGLE_API_KEY 未设置。"
                "请在 .env 中设置或传入 api_key 参数。"
            )
        self._client = genai.Client(api_key=key)
        self._model = model

    def challenge(self, subject: str, context: str = "") -> ChallengeResult:
        """对给定主题发起质询。

        Parameters
        ----------
        subject : str
            质询目标（定义、代码、推理链等）。
        context : str
            相关上下文信息。

        Returns
        -------
        ChallengeResult
        """
        prompt = _CHALLENGE_TEMPLATE.format(subject=subject, context=context)
        response = self._client.models.generate_content(
            model=self._model,
            contents=prompt,
            config=genai.types.GenerateContentConfig(
                system_instruction=_SYSTEM_PROMPT,
                temperature=0.3,
            ),
        )
        return ChallengeResult(
            mode="challenge",
            subject=subject,
            response=response.text or "",
            model=self._model,
        )

    def verify(self, subject: str, context: str = "") -> ChallengeResult:
        """验证给定断言是否成立。

        Parameters
        ----------
        subject : str
            验证目标。
        context : str
            相关上下文信息。

        Returns
        -------
        ChallengeResult
        """
        prompt = _VERIFY_TEMPLATE.format(subject=subject, context=context)
        response = self._client.models.generate_content(
            model=self._model,
            contents=prompt,
            config=genai.types.GenerateContentConfig(
                system_instruction=_SYSTEM_PROMPT,
                temperature=0.1,
            ),
        )
        return ChallengeResult(
            mode="verify",
            subject=subject,
            response=response.text or "",
            model=self._model,
        )


# ── 模块级便捷函数 ──

_default_challenger: GeminiChallenger | None = None


def _get_challenger() -> GeminiChallenger:
    global _default_challenger
    if _default_challenger is None:
        _default_challenger = GeminiChallenger()
    return _default_challenger


def challenge(subject: str, context: str = "") -> ChallengeResult:
    """模块级质询（自动初始化）。"""
    return _get_challenger().challenge(subject, context)


def verify(subject: str, context: str = "") -> ChallengeResult:
    """模块级验证（自动初始化）。"""
    return _get_challenger().verify(subject, context)


# ── CLI 入口 ──

if __name__ == "__main__":
    import argparse
    import sys

    parser = argparse.ArgumentParser(description="Gemini 质询工位 CLI")
    parser.add_argument(
        "mode",
        choices=["challenge", "verify"],
        help="质询模式",
    )
    parser.add_argument(
        "subject",
        help="质询/验证目标",
    )
    parser.add_argument(
        "--context",
        default="",
        help="上下文信息",
    )
    parser.add_argument(
        "--context-file",
        default=None,
        help="从文件读取上下文",
    )
    args = parser.parse_args()

    ctx = args.context
    if args.context_file:
        with open(args.context_file, encoding="utf-8") as f:
            ctx = f.read()

    try:
        challenger = GeminiChallenger()
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    if args.mode == "challenge":
        result = challenger.challenge(args.subject, ctx)
    else:
        result = challenger.verify(args.subject, ctx)

    print(f"[{result.mode}] model={result.model}")
    print("=" * 60)
    print(result.response)
