"""各模式的具体实现 — CodexChallenger 类 + 模块级便捷函数。

prompt 构建、CLI 调用入口。
调用走 codex CLI（engine.call_codex_cli），ChatGPT 订阅认证，配额独立。

概念溯源: [新缠论] — 155号谱系：代码层异质审查（Codex CLI）
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Literal

from newchan.codex.engine import _MODEL, call_codex_cli
from newchan.codex.registry import ModeKey, get_mode_config


@dataclass(frozen=True, slots=True)
class ReviewResult:
    """审查结果。"""

    mode: Literal["review", "diagnose", "decide"]
    subject: str
    response: str
    model: str
    prompt: str = ""
    context_file: str | None = None

    def to_markdown(self, timestamp: datetime | None = None) -> str:
        """返回格式化的 Markdown 字符串，用于持久化。

        完整交互（prompt + response）都被持久化，
        不只是结果，而是完整的对话记录。

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

        if self.prompt:
            lines.extend([
                "",
                "## Prompt",
                "",
                self.prompt,
            ])

        lines.extend([
            "",
            "## Response",
            "",
            self.response,
            "",
        ])
        return "\n".join(lines)


class CodexChallenger:
    """Codex 代码层异质审查工位的核心类。

    调用走 codex CLI（`codex exec`），ChatGPT 订阅认证，无需 API Key。

    Parameters
    ----------
    model : str
        ReviewResult.model 的标记。实际模型由 codex config.toml 决定，默认 codex-cli。
    """

    def __init__(self, model: str = _MODEL) -> None:
        self._model = model

    def _run_mode(
        self,
        mode: ModeKey,
        subject: str,
        context: str,
    ) -> ReviewResult:
        """通用同步模式执行。"""
        cfg = get_mode_config(mode)
        prompt = cfg.template.format(subject=subject, context=context)
        text, model_used = call_codex_cli(
            prompt, cfg.system_prompt, model=self._model,
        )
        return ReviewResult(
            mode=mode, subject=subject,
            response=text, model=model_used,
            prompt=prompt,
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
