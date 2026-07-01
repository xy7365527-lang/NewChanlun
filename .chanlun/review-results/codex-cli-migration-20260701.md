# Codex 调用迁移：OpenAI API → codex CLI（2026-07-01）

## 结论

codex 异质审查调用链从 OpenAI Responses API 迁移到 codex CLI（`codex exec`），
走 ChatGPT 订阅认证，配额独立于 OpenAI API，解 `insufficient_quota` 429 死锁。

改动文件（全在 `src/newchan/codex/*` + 垫片）：

- `src/newchan/codex/engine.py`：删 `call_with_fallback`（OpenAI Responses API +
  双模型 fallback + `_extract_text`），换为 `call_codex_cli(prompt, system_prompt, model)`
  —— subprocess 调 `codex exec`，从 `--output-last-message` 文件读干净回答。
- `src/newchan/codex/modes.py`：删 `_create_client`/`_get_openai_ref`/`import openai`/
  `import os`；`CodexChallenger.__init__` 去掉 `api_key` 参数（CLI 无需 key）；
  `_run_mode` 改调 `call_codex_cli`。
- `src/newchan/codex_challenger.py`：删 `import openai` 与 `_PatchProxyModule`
  （openai mock 同步机制，已无消费者）。
- `src/newchan/codex/__main__.py`：删无用的 `ValueError`（key 缺失）try/except 与 `import sys`。
- `tests/test_codex_challenger.py`：mock 层从 `patch(codex.modes.openai)` + `responses.create`
  改为 `patch(codex.modes.call_codex_cli)`（modes 层）和 `patch(codex.engine.subprocess.run)`
  （engine 层，验证 CLI 参数组装 + `--output-last-message` 解析 + 失败/超时路径）；
  删 `TestFallback`（CLI 无双模型 fallback）；`TestCodexChallengerInit` 改为验证无 key 可构造。

### CLI 调用形态（已验证）

```
codex exec --skip-git-repo-check --sandbox read-only --ephemeral \
  --output-last-message <tmpfile> "<system_prompt>\n\n<prompt>"
```

- cwd = 临时目录（`tempfile.TemporaryDirectory`）→ 避免 auto-load `.codex/memories`/`.agents/skills` 污染。
- **不带** `--ignore-user-config`：该 flag 会丢 config.toml 的默认模型，
  回退到 `gpt-5.3-codex`——而 ChatGPT 订阅账户不支持该模型（实测报 400）。
  模型由 codex config.toml 决定，`ReviewResult.model` 标记为 `codex-cli`。
- `--output-last-message` 文件只含最终回答，绕过 stdout 的 MCP 认证噪音 + `hook: SessionStart` 噪音。

## 验证

- **CLI 端到端（不撞 429）**：`codex exec ... "reply with exactly: HELLO_CLEAN"` →
  文件内容 `HELLO_CLEAN`，exit 0。
- **`python -m newchan.codex diagnose` 冒烟**：`PYTHONPATH=src python -m newchan.codex diagnose ...`
  → `[diagnose] model=codex-cli`，返回真实回答，持久化成功，exit 0，无 429。
- **pytest**：`tests/test_codex_challenger.py` 30 passed。

## 边界条件

- codex CLI 首次跑若需登录/认证失败 → `call_codex_cli` 抛 `RuntimeError`（exit≠0）。
  当前订阅认证已就绪（冒烟通过）。
- 若 config.toml 默认模型被改成订阅不支持的模型 → CLI 报 400，`call_codex_cli` 抛 RuntimeError。
- 超时上限 `_TIMEOUT_SEC = 600`；超时抛 RuntimeError。
- 若 codex 二进制不在 PATH → `subprocess.run` 抛 `FileNotFoundError`（未额外包装，直接冒泡）。

## 影响声明

- 影响模块：`newchan.codex.engine` / `.modes` / `.__main__` / `newchan.codex_challenger`
  + `tests/test_codex_challenger.py`。
- **未碰** `econ_positive.rs` / classifier（parity-analysis + ledger-dump 在用）。
- 不涉及 bit-exact（审计工具链，非缠论核心）。
- agent 定义 `.claude/agents/codex-challenger.md` 无需改：入口 `python -m newchan.codex`
  内部已换 CLI subprocess，无 openai/OPENAI_API_KEY 引用。
- **行为变更**：`CodexChallenger` 不再接受 `api_key` 参数、不再读 `OPENAI_API_KEY`。
  下游若显式传 `api_key=` 会 TypeError（现无此调用点）。
