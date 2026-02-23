"""各模式的具体实现 — CodexChallenger 类 + 模块级便捷函数。

prompt 构建、输出解析、sync 调用入口。
openai 在此模块级导入，测试通过 patch("newchan.codex.modes.openai") 拦截。

概念溯源: [新缠论] — 155号谱系：代码层异质审查（OpenAI Codex）
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Literal

import openai

from newchan.codex.engine import _MODEL, call_with_fallback
from newchan.codex.registry import ModeKey, get_mode_config


@dataclass(frozen=True, slots=True)
class ReviewResult:
    """审查结果。"""

    mode: Literal["review", "diagnose", "decide"]
    subject: str
    response: str
    model: str
    context_file: str | None = None

    def to_markdown(self, timestamp: datetime | None = None) -> str:
        """返回格式化的 Markdown 字符串，用于持久化。

        Parameters
        ----------
        timestamp : datetime | None
            时间戳。None 时使用当前 UTC 时间。
        """
        ts = timestamp or datetime.now(tz=timezone.utc)
        ts_str = ts.strftime("%Y-%m-%d %H:%M:%S UTC")

        lines = [
            f"# Codex {self.mode} — {ts_str}",
            "",
            "## 元数据",
            "",
            f"- **mode**: {self.mode}",
            f"- **subject**: {self.subject}",
            f"- **model**: {self.model}",
            f"- **timestamp**: {ts_str}",
        ]
        if self.context_file:
            lines.append(f"- **context-file**: {self.context_file}")

        lines.extend([
            "",
            "## Response",
            "",
            self.response,
            "",
        ])
        return "\n".join(lines)


def _create_client(api_key: str | None = None) -> openai.OpenAI:
    """创建 OpenAI 客户端，使用模块级 openai 引用（可被 mock 替换）。"""
    import newchan.codex.modes as _self

    key = api_key or os.environ.get("OPENAI_API_KEY", "")
    if not key:
        raise ValueError(
            "OPENAI_API_KEY 未设置。"
            "请在 .env 中设置或传入 api_key 参数。"
        )
    return _self.openai.OpenAI(api_key=key)


class CodexChallenger:
    """Codex 代码层异质审查工位的核心类。

    Parameters
    ----------
    api_key : str | None
        OpenAI API Key。None 时从 OPENAI_API_KEY 环境变量读取。
    model : str
        模型名称，默认 codex-5.3。
    """

    def __init__(
        self,
        api_key: str | None = None,
        model: str = _MODEL,
    ) -> None:
        self._client = _create_client(api_key)
        self._model = model

    def _get_openai_ref(self) -> object:
        """获取当前模块级 openai 引用（支持 mock）。"""
        import newchan.codex.modes as _self

        return _self.openai

    def _run_mode(
        self,
        mode: ModeKey,
        subject: str,
        context: str,
    ) -> ReviewResult:
        """通用同步模式执行。"""
        cfg = get_mode_config(mode)
        prompt = cfg.template.format(subject=subject, context=context)
        openai_mod = self._get_openai_ref()
        text, model_used = call_with_fallback(
            self._client, self._model, prompt,
            cfg.system_prompt, cfg.reasoning_effort,
            openai_mod,
        )
        return ReviewResult(
            mode=mode, subject=subject,
            response=text, model=model_used,
        )

    def review(self, subject: str, context: str = "") -> ReviewResult:
        """对给定代码发起审查。"""
        return self._run_mode("review", subject, context)

    def diagnose(self, subject: str, context: str = "") -> ReviewResult:
        """对测试失败/代码错误进行根因诊断。"""
        return self._run_mode("diagnose", subject, context)

    def decide(self, subject: str, context: str = "") -> ReviewResult:
        """代码层技术选型决策。"""
        return self._run_mode("decide", subject, context)


# ── 模块级便捷函数 ──

_default_challenger: CodexChallenger | None = None


def _get_challenger() -> CodexChallenger:
    global _default_challenger
    if _default_challenger is None:
        _default_challenger = CodexChallenger()
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
