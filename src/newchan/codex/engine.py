"""核心调用引擎 — codex CLI subprocess。

走 codex-cli(codex exec)+ ChatGPT 订阅认证，配额独立于 OpenAI API。
用 --output-last-message 拿干净的最终回答（不含 MCP/hook 噪音）。

概念溯源: [新缠论] — 155号谱系：代码层异质审查
迁移: OpenAI Responses API → codex CLI（解 insufficient_quota 429 死锁）
"""

from __future__ import annotations

import logging
import subprocess
import tempfile
from pathlib import Path

logger = logging.getLogger(__name__)

# config.toml 默认模型（ChatGPT 订阅支持的）负责实际选型。
# _MODEL 仅作为 ReviewResult.model 的标记，不传给 CLI（--ignore-user-config
# 会丢失订阅支持的默认模型，回退到订阅不支持的 gpt-5.3-codex）。
_MODEL = "codex-cli"
_TIMEOUT_SEC = 600


def call_codex_cli(
    prompt: str,
    system_prompt: str,
    *,
    model: str = _MODEL,
) -> tuple[str, str]:
    """调 `codex exec` 非交互跑，返回 (response_text, model_used)。

    Parameters
    ----------
    prompt : str
        用户 prompt（review/diagnose/decide 模板渲染后的文本）。
    system_prompt : str
        系统指令，作为 prompt 的前缀合并（codex exec 无独立 system 通道）。
    model : str
        仅用于回填 ReviewResult.model 的标记。实际模型由 config.toml 决定。

    在干净临时目录跑（cwd=tmpdir）避免 codex auto-load
    .codex/memories + .agents/skills 污染审计。
    --output-last-message 写最终回答到文件，绕过 stdout 的 MCP/hook 噪音。
    """
    full_prompt = f"{system_prompt}\n\n{prompt}" if system_prompt else prompt

    with tempfile.TemporaryDirectory(prefix="codex-cli-") as workdir:
        out_file = Path(workdir) / "last-message.txt"
        cmd = [
            "codex", "exec",
            "--skip-git-repo-check",
            "--sandbox", "read-only",
            "--ephemeral",
            "--output-last-message", str(out_file),
            full_prompt,
        ]
        try:
            proc = subprocess.run(
                cmd,
                cwd=workdir,
                capture_output=True,
                text=True,
                timeout=_TIMEOUT_SEC,
            )
        except subprocess.TimeoutExpired as e:
            raise RuntimeError(
                f"codex CLI 超时（>{_TIMEOUT_SEC}s）",
            ) from e

        if proc.returncode != 0:
            tail = (proc.stderr or proc.stdout or "").strip()[-800:]
            raise RuntimeError(
                f"codex CLI 失败（exit={proc.returncode}）: {tail}",
            )

        if not out_file.exists():
            raise RuntimeError(
                "codex CLI 未写入 --output-last-message 文件",
            )
        text = out_file.read_text(encoding="utf-8").strip()

    return text, model
