# AGENTS.md（跨 harness 桥接）

本文件面向所有非 Claude Code 的 agent harness（GitHub Copilot CLI / coding agent、Codex 等）。规则本体不在这里。

1. **语言**：一律用简体中文回复与产出文档。
2. **基因组**：先读根目录 `CLAUDE.md`——本项目的蜂群方法论、原则 0-17、skill 索引全部在那里。skills 在 `.claude/skills/`，agents 在 `.claude/agents/`。
3. **缠论权威链**：博文（`docs/chanlun/text/blog/INDEX.md`）> 编纂版（`docs/chanlun/text/chan99/INDEX.md`）> 思维导图。速查：`缠论知识库.md`。
4. **蜂群循环入口**：`python scripts/ceremony_state.py write 1 initial` → `python scripts/ceremony_scan.py`。goal 协议见 `.claude/commands/goal.md` 与 `.chanlun/goals/events.jsonl`。
5. **090 号严格纪律**：禁简化实装、禁补丁方案、禁模糊地带。声明必须与实际能力一致。
6. **平台差异**：Copilot 下的运行手册与降级说明见 `docs/copilot-runbook.md`。

## 记忆（高信号持久事实，continual-learning 维护）

- **偏好**：编排者粘贴 shell 命令时期望逐字执行；破坏性 git 操作前先备份 + 只读预检，且不打印任何密钥值。
- **工作线**：真实工作线是长寿分支 `gap3-rework-codex9-fix`（领先 `main` 300+ commit）；compact/重启后 HEAD 可能被错切到 `main`，信任 checkout 前先用 `git reflog` 核对。
- **Stop-Guard**：其注入内容可能反映过期状态（已结算谱系仍列为 pending）；重复执行前先核实际状态。
- **goal 事件**：正式 GOAL_SET/SUPERSEDE 由 Lead 直接 append 到 `.chanlun/goals/events.jsonl`（`scripts/goal_events.py` 只出草稿），随后用 reducer/scan 验证。
- **裁定文档**：编排者钦定的裁定 PDF 先归档到 `docs/formal-chain/` 再登记为 goal 权威。
