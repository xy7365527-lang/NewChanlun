# GitHub Copilot 运行手册——在 Copilot 里跑本仓库的蜂群/模型

> 基因 073a/274号 交付物。调研时点：2026-07-04，来源以 GitHub Docs / GitHub Changelog / VS Code Docs 当日版本为准。标注"未实测"的条目为文档结论，未在本机验证。

## 0. 结论摘要：三种入口的支持度

| 能力 | Copilot CLI | VS Code agent mode | Copilot coding agent（云端） |
|------|-------------|--------------------|------------------------------|
| `.github/copilot-instructions.md` | ✅ 自动加载 | ✅ 自动加载 | ✅ 自动加载 |
| 根目录 `AGENTS.md` | ✅ 作为 primary instructions | ✅（`chat.useAgentsMdFile`，默认开） | ✅（`**/AGENTS.md`） |
| 根目录 `CLAUDE.md` 直读 | ❌ 文档未列（未实测） | ✅（`chat.useClaudeMdFile`；含 `.claude/CLAUDE.md`） | ✅（`/CLAUDE.md` 官方支持） |
| `.claude/skills/` 自动发现 | ✅ 官方支持 | ✅ 官方支持 | ✅ 官方支持 |
| `.claude/agents/` 自动发现 | ❌（Copilot 只认 `.github/agents/`） | ❌ 同左 | ❌ 同左 |
| Claude 系模型 | ✅ `/model` 或 `--model` | ✅ 模型选择器；另有 Claude Agent SDK 会话 | ✅ 任务发起时选择 |
| hooks / slash 命令 / 蜂群 Task 团队 | ❌ 无对应机制（有单发 `task` 子代理） | ❌ | ❌ |

桥接文件（本次已落库）：`.github/copilot-instructions.md` + 根目录 `AGENTS.md`，两者都只做指路——指回 `CLAUDE.md` 基因组与 `.claude/skills/`，不复制内容。

## 1. 前置：安装与登录

**Copilot CLI**（需 Copilot 订阅）：

```bash
npm install -g @github/copilot   # 或 brew install copilot-cli
copilot                           # 首次运行按提示 /login，走 GitHub 设备码授权
```

**VS Code**：安装 GitHub Copilot 扩展（内置 Chat），登录 GitHub 账号。确认设置里 `chat.useAgentsMdFile`、`chat.useClaudeMdFile` 均为开启（默认开启；后者让 VS Code 直接读根目录 `CLAUDE.md`）。

**云端 coding agent**：仓库需推到 GitHub 且订阅计划启用 coding agent。适合"派发一个 issue 让它出 PR"的场景，不适合跑持续蜂群循环（见 §5）。

## 2. 路线 A：Copilot CLI

```bash
cd /Users/silencehan/Projects/NewChanlun
copilot
```

进入交互会话后：

1. `/model` 选 Claude 系模型（见 §4）。
2. CLI 自动加载 `.github/copilot-instructions.md` 与根 `AGENTS.md`（两者同时存在时都会被使用，AGENTS.md 为 primary）。`.claude/skills/` 下的 60+ 个 skill 会被自动发现、按 description 相关性懒加载；`/skills` 可查看清单。
3. 发出第一条指令（触发 ceremony）：

```
先读根目录 CLAUDE.md 作为基因组，然后执行：
python scripts/ceremony_state.py write 1 initial
python scripts/ceremony_scan.py
把 scan 输出的 ready 工位报给我，之后按 .claude/commands/goal.md 的运行协议循环推进。
```

工具面差异参考（本机 `~/.claude/skills/superpowers/skills/using-superpowers/references/copilot-tools.md`）：Claude Code 的 `Task` 对应 CLI 的 `task`（agent_type: general-purpose/explore），`TodoWrite` 对应内置 `sql` todos 表，无 `WebSearch`（用 `web_fetch` 代替），无 plan mode。CLI 独有：异步 bash 会话（`bash async` + `read_bash`/`write_bash`）。

## 3. 路线 B：VS Code agent mode

1. 打开本仓库文件夹，Chat 面板（`⌃⌘I`）切到 **Agent** 模式。
2. 模型选择器选 Claude 系（Sonnet/Opus，视订阅）。
3. VS Code 会自动注入 `.github/copilot-instructions.md`、`AGENTS.md`、以及 `CLAUDE.md`（`chat.useClaudeMdFile`），并自动发现 `.claude/skills/`。
4. 第一条指令同 §2 第 3 步。

**加强路径（推荐尝试）**：VS Code 的第三方 agent 会话支持 **Claude Agent SDK**（设置 `github.copilot.chat.claudeAgent.enabled`，会话类型下拉选 "Claude"）。这是真正的 Claude harness 在本地跑，对 `CLAUDE.md`/skills 的语义还原度最高，是 Copilot 生态里最接近 Claude Code 行为的入口（细节未实测，以 VS Code 文档为准）。

## 4. 模型选择建议

- **优先 Claude 系**：本仓库全部指令/skill/谱系为 Claude 语境写成，Claude 系模型对 CLAUDE.md 先验的遵循度最好。CLI 用 `/model`，VS Code 用模型选择器。2026-06-29 起 Claude Opus 4.8（fast mode）在 CLI/VS Code/云端 agent 均可选（Pro+/Business/Enterprise，需管理员开策略）。
- 云端 coding agent 可选：Auto / Claude Sonnet 4.5 / Claude Opus 4.7 / Claude Haiku 4.5 / Gemini / GPT 系（2026-07 时点文档口径）。
- CLI 的 Auto 模式（2026-07-01 上线）按任务自动路由，省 credits 但不保证 Claude——跑蜂群协议时**不建议 Auto**，显式锁定 Claude。

## 5. 能力差异与降级说明（重要）

Copilot 三入口都**没有** Claude Code 的以下机制，对应协议降级如下：

| Claude Code 机制 | Copilot 现状 | 降级方式 |
|------------------|--------------|----------|
| hooks（session-start 注入 ceremony、Stop-Guard 推动 goal 循环） | 无 hooks 机制 | **人工触发**：每个新会话手动发 §2 第 3 步的启动指令；goal 循环靠人盯或让模型自述"完成一轮后自动继续" |
| slash 命令（`/ceremony` `/goal` `/inquire` `/ritual` 等，`.claude/commands/*.md`） | 不识别该目录 | 人工让模型"读 `.claude/commands/<x>.md` 并按其执行"；CLI 可把常用命令固化为 `.github/skills/` 或 `~/.copilot/skills/` 下的 skill（未实测） |
| Task 蜂群（并行 teammate、工位真封、子蜂群 ceremony） | CLI 有单发 `task` 子代理（general-purpose/explore + `.github/agents/` 自定义 agent）；VS Code/云端无等价物 | 蜂群并行度降为"Lead 单工位 + 少量子代理"；`.claude/agents/` 下的 22 个 agent 定义**不会被自动发现**，如需要须手工镜像为 `.github/agents/*.agent.md`（frontmatter 支持 `model`/`tools`/`skills` 预载） |
| CLAUDE.md 自动先验（CLI 场景） | CLI 文档未列 CLAUDE.md | 已由 `AGENTS.md` + `.github/copilot-instructions.md` 桥接指回 |
| MCP（tradingview-mcp 经 `~/.claude/.mcp.json` 注册） | Copilot 有自己的 MCP 配置（CLI：`/mcp add`；VS Code：`mcp.json`） | 需在 Copilot 侧重新注册 tradingview-mcp，**未实测** |
| 070 阻断等待 / 020 基因组保护等 hook 强制协议 | 无强制层 | 纯靠模型自律 + 人工审查，风险见 §6 |

## 6. 已知坑

1. **指令长度敏感**：Copilot 对超长 instructions 遵循度下降，故桥接文件刻意精炼——不要往 `copilot-instructions.md`/`AGENTS.md` 里堆内容，细节留在 CLAUDE.md 与 skills。
2. **云端 agent 有网络防火墙**：默认 allowlist 之外的域名被拦（如 fengmr.com 博文回溯会失败），且跑在 GitHub Actions 里无本地 TradingView。云端入口只适合纯代码任务。
3. **中文回复不保证**：Copilot 默认英文倾向强，桥接文件第零条已声明简体中文，但个别模型仍可能溜回英文——发现即纠正。
4. **严格纪律无强制层**：Claude Code 里由 hooks/谱系机制兜底的 090 号严格纪律，在 Copilot 里只剩指令声明。审查 Copilot 产出时按 090 标准把关（禁简化实装/补丁方案）。
5. **skills 懒加载靠 description 命中**：`.claude/skills/` 里部分 skill 的 frontmatter description 是 Claude 语境短语，Copilot 可能判不中相关性——关键协议（如 core-principles）建议在指令里显式点名让它读。

## 7. 来源

- GitHub Docs：Copilot CLI custom instructions / About agent skills / coding agent best practices / cloud agent model 选择（检索于 2026-07-04）
- GitHub Changelog：AGENTS.md 支持（2025-08-28）、Agent Skills 上线（2025-12-18，明确 `.claude/skills` 自动兼容）、CLI Auto 模型路由（2026-07-01）、Claude Opus 4.8 fast mode（2026-06-29）
- VS Code Docs：custom instructions（`chat.useAgentsMdFile`/`chat.useClaudeMdFile`）、third-party agents（Claude Agent SDK 会话）
- 本机参考：`~/.claude/skills/superpowers/skills/using-superpowers/references/copilot-tools.md`（Copilot CLI 工具映射表）
