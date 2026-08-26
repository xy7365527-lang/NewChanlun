# 迁移 Runbook：Claude Code 数据面 → XFS on LVM-VDO + Desktop VM 边界钉死（#1238）

> 本文件是 #1238 的迁移执行手册。执行前先读 #1227（总图）、#1236（边界与容量）、
> #1244（dedup/容量/配额/告警裁决）、#1241（canary 建盘）、#1239（基准与恢复证据）。
> 配套脚本：`claude-migrate.sh`（同目录），沙盒对拍：`claude-migrate.test.sh`。
> 复制原语的语义（停写/基线/增量/回滚）与 #1235 的 `.sandcastle/migrate-data-surface/migrate-surface.sh`
> 一致；但本票的「切换」**不是 symlink**，而是官方环境变量 `CLAUDE_CONFIG_DIR`（见 §3），
> 因此 §6 用 `claude-migrate.sh` + 原生 rsync，不调用 migrate-surface.sh 的 symlink switch。

## 1. 适用范围与出界

| 项 | 归属 |
|---|---|
| Claude Code（`claude` CLI）Linux 可迁移执行面 | **本票**（`/srv/agents/claude`） |
| Claude Code 可安全重定位的数据（`~/.claude/` + `.claude.json` 非凭据字段） | **本票** |
| Claude Desktop 外壳、厂商 local-agent VM 镜像、GUI/browser/computer-use、Keychain | **保留 macOS 控制面，不迁**（#1236 已裁，本票钉死归属，见 §4.2） |
| Prime/Sandcastle（#1235）、Codex CLI/Desktop（#1240） | 出界，另票 |
| 分批切换/监控/回滚演练/旧路径退役（#1237） | 出界，本票完成后接续 |

## 2. 已裁口径（写码/执行前复核这些数字）

来自 #1244（编排者拍板，2026-08-26）：

| 项 | 裁决 |
|---|---|
| dedup | **维持 off** |
| 生产容量 | **700 GiB 确认**（不调整） |
| 配额 | **500 GiB 单一总量红线**，不设独立硬配额；**必须保留四组分项计量** |
| 告警 | **warning 400 GiB / critical 450 GiB**（VDO 物理占用口径，同步监控宿主 APFS 剩余） |
| 执行约束 | critical 立即停扩张型任务并清理/扩容；claude 组计量 = `du -s /srv/agents/claude` |

来自 #1236：

- macOS 控制面保留：**Claude Desktop 外壳、Keychain、应用签名包、厂商管理的 Claude VM**。
- 可写数据面与 Linux 兼容执行面进**单一 Linux VM**；禁止多 VM 同挂同一 XFS；SQLite/WAL、socket、lockfile 不放 NFS/SMB/FUSE。
- Mach-O CLI/Darwin addon 不复制进 guest；**在 Linux 中重新安装对应构建**。
- 宿主外置 APFS（`/Volumes/AgentStorage`）至少保留约 200 GiB 余量；生产迁移前启用 ownership 或 guest 内受控加密并**重新登录**，不复制现有 auth/Keychain。

来自 #1239（已知风险）：默认 VDO 性能门槛 FAIL（SQLite WAL/FULL 慢 81%、`git add` 慢 17.2%）。
`~/.claude/projects/` 是大量 JSONL 小文件（会话转录），落 XFS-on-VDO 前先按 §8 做一次读写 p95 抽样，异常即停。

## 3. 官方配置入口盘点（本票的核心发现，全部已实证）

Claude Code 的官方重定位杠杆**只有一个**：环境变量 `CLAUDE_CONFIG_DIR`。

**实证结论（沙盒内 `claude` 2.1.246，2026-08-26，用 `claude mcp add` 落盘验证）：**

1. 默认布局：`$HOME/.claude/`（配置目录）与 `$HOME/.claude.json`（全局状态）**平级**。
2. 设 `CLAUDE_CONFIG_DIR=/path` 后，**整棵数据面都改写到该目录下**：
   - `$CLAUDE_CONFIG_DIR/.claude.json`（注意：`.claude.json` 进到配置目录**内部**，不再在 `$HOME` 平级）；
   - `$CLAUDE_CONFIG_DIR/backups/`（`.claude.json` 历史副本）；
   - settings、session history、plugins 亦在此（官方 env-vars 文档原话：*All settings, session history, and plugins are stored under this path, as are credentials on Linux and Windows; on macOS, credentials are in the system Keychain*）。
3. **macOS 凭据在系统 Keychain**（`CLAUDE_CONFIG_DIR` 覆盖不到、也绝不可复制）；**Linux 凭据在 `$CLAUDE_CONFIG_DIR` 下**（重登录后自然落在新卷）。
4. 因此：**迁移 = 在 Linux VM 里设 `CLAUDE_CONFIG_DIR=/srv/agents/claude` 并重登录**，不需要也不允许用 symlink 强搬 `~/.claude`，更不得碰厂商 VM bundle。

官方文档入口（code.claude.com/docs，2026-08-26 抓取核对）：`settings`、`claude-directory`、`env-vars`、`mcp`、`setup`、`cli-reference`。关键字段引用见 §11。

### 3.1 Claude Code 数据面清单（`~/.claude/`，官方 `claude-directory` 页逐项）

| 内容 | 路径（默认） | 特性 |
|---|---|---|
| 用户配置 | `~/.claude/settings.json`、`CLAUDE.md` | 个人权限/hooks/env/模型默认、全局记忆 |
| 定制文件 | `skills/`、`commands/`、`agents/`、`workflows/`、`rules/`、`output-styles/`、`agent-memory/`、`keybindings.json`、`themes/` | 个人全局（不提交）；随 `CLAUDE_CONFIG_DIR` 走 |
| 插件 | `~/.claude/plugins/` | `claude plugin` 管理；marketplaces/版本/插件数据 |
| 会话转录/自动记忆 | `~/.claude/projects/<project>/<session>.jsonl` + `memory/` | `--resume`/`--continue`/`/resume` 依赖；**明文**（官方明文存储告警，见 §11） |
| checkpoint 快照 | `~/.claude/file-history/<session>/` | 恢复编辑历史 |
| 运行中会话检测 | `~/.claude/sessions/` | 每会话一小文件，退出清理；**禁复制** |
| 缓存/易重生 | `plans/`、`debug/`、`paste-cache/`、`image-cache/`、`uploads/`、`session-env/`、`tasks/`、`shell-snapshots/`、`usage-data/`、`feedback-bundles/`、`cache/`、`stats-cache.json` | 自动清理或启动重抓 |
| 旧版本遗留 | `todos/`、`statsig/`、`logs/` | 现版本不再写入 |
| `.claude.json` 历史副本 | `~/.claude/backups/` | **可能含旧 OAuth 会话，禁复制** |
| 提示历史 | `~/.claude/history.jsonl` | 上箭头召回；**可能含粘贴的明文凭据，默认禁复制** |
| 组织缓存 | `remote-settings.json`、`policy-limits.json` | 启动重取，禁复制 |

### 3.2 `.claude.json`（全局状态文件，官方 `claude-directory` 页）

位置：默认 `$HOME/.claude.json`；设 `CLAUDE_CONFIG_DIR` 后 = `$CLAUDE_CONFIG_DIR/.claude.json`（实证）。

内容（fresh 落盘实测顶层键）：`mcpServers`（user 档 MCP）、`projects`（每项目 trust/MCP/审批状态）、
`machineID`、`userID`、`firstStartTime` 等 UI/迁移标记；**登录后另含 OAuth 会话字段**。

- **凭据字段（OAuth 会话等）禁止复制**；`machineID`/`userID`/`projects` 属本机/本项目状态，目标机重登录、首次运行自动重建，不复制。
- **唯一默认复制的是 `mcpServers`（user 档）**——非凭据、且重配成本高；由 `claude-migrate.sh config-json` 用 `jq` 白名单只保留该键。

### 3.3 MCP 三档落盘（官方 `mcp` 页 + 沙盒实证）

| 档 | 落盘 | 迁移处置 |
|---|---|---|
| user | `~/.claude.json#mcpServers` | 随 `.claude.json` 白名单迁移 |
| local | `~/.claude.json#projects["<cwd>"].mcpServers` | 不迁（项目路径变了，重加） |
| project | 仓库根 `.mcp.json` | 随 git checkout，不迁 |

> 本仓 `.mcp.json`（`claude-audit`）随仓库；`~/.claude/.mcp.json`（tradingview-mcp，见 AGENTS.md）在
> `CLAUDE_CONFIG_DIR` 覆盖范围内随迁。**若该文件内嵌明文 token（而非 `${VAR}` 引用），迁移前按 secrets
> 规则改 `${VAR}` 引用或目标侧重配，不得把明文凭据带进 XFS。** `claude mcp add-from-claude-desktop`
> （Mac/WSL only）可在迁移时把 Desktop 侧 MCP 显式导入 CLI，不搬 VM。

## 4. 归属决策表（每个大目录一行，证据支持）

### 4.1 Claude Code（Linux 可迁移面）

| 旧路径（macOS） | 内容 | 官方入口 | 处置 |
|---|---|---|---|
| `~/.claude/`（settings/CLAUDE.md/skills/commands/agents/workflows/rules/output-styles/agent-memory/keybindings/themes） | 个人配置与定制 | `CLAUDE_CONFIG_DIR` | **迁** `/srv/agents/claude/` |
| `~/.claude/projects/`、`file-history/`、`plans/`、`tasks/`、`session-env/` | 会话/记忆/checkpoint | `CLAUDE_CONFIG_DIR` | **迁** |
| `~/.claude/plugins/` | 插件 | `CLAUDE_CONFIG_DIR` + `claude plugin` | **迁** |
| `~/.claude/stats-cache.json`、`cache/`、`usage-data/`、`feedback-bundles/` | 小缓存/报告 | `CLAUDE_CONFIG_DIR` | 迁（或丢弃重生成） |
| `~/.claude/{sessions,shell-snapshots,backups,debug,image-cache,paste-cache,uploads,todos,statsig,logs}/`、`history.jsonl`、`remote-settings.json`、`policy-limits.json` | 运行时/易重生/可能含凭据 | — | **禁复制**（§6 excludes） |
| `~/.claude.json`（mcpServers 键） | user 档 MCP 配置 | `CLAUDE_CONFIG_DIR` | **迁**（`jq` 白名单） |
| `~/.claude.json`（OAuth/凭据、machineID/userID/projects 键） | 登录态/本机态 | 无 | **禁复制**，目标重登录重建 |
| macOS 系统 Keychain | 登录凭据 | 无（系统） | **禁复制**（#1236），重登录 |
| `~/.local/bin/claude` → `~/.local/share/claude/versions/` | Mach-O 二进制 + launcher | native installer / npm | **不复制**；guest 用 `curl -fsSL https://claude.ai/install.sh | bash` 重装 Linux 构建 |
| 仓库 `.claude/`、`.mcp.json`、`CLAUDE.md` | 项目级配置 | git | 随 checkout，不迁 |

### 4.2 Claude Desktop（macOS 控制面保留，不迁）

> #1236 已裁「Claude Desktop 外壳、Keychain、应用签名包、厂商管理的 Claude VM → 保留 macOS 控制面」。
> 本票落地该裁定的每一项目录归属；路径为 macOS 标准约定，**执行前宿主侧 `du -sh`/`ls` 现场核实**。

| 路径 | 内容 | 处置 | 证据/理由 |
|---|---|---|---|
| `/Applications/Claude.app`（安装位置宿主核实） | 应用签名包 | **保留 macOS** | 签名包不可受支持重定位（#1236） |
| `~/Library/Application Support/Claude/` | 厂商 local-agent **VM 镜像** + session 数据 | **保留 macOS** | 厂商管理，无官方重定位入口；**不得 symlink/copy 强搬**（issue 执行要求①） |
| `~/Library/Preferences/com.anthropic.claudefordesktop.plist` | GUI 偏好 | 保留 macOS | preference domain（官方 desktop 页：macOS 经 `com.anthropic.claudefordesktop` 域下发管理设置） |
| `~/Library/Caches/com.anthropic.claudefordesktop/` | GUI 缓存 | 保留 macOS（可清） | 缓存易重生 |
| `~/Library/Logs/Claude/` | GUI 日志 | 保留 macOS | 日志 |
| Keychain（`com.anthropic.claudefordesktop` 等条目） | 登录凭据 | **禁止复制** | macOS 系统 Keychain（#1236） |
| Desktop 侧 browser/GUI/computer-use 能力 | GUI 协作 | **保留 macOS** | Linux CLI 无此能力；不属数据面 |

**Desktop local-agent 镜像归属结论（issue 点名要求）：不可受支持地重定位 → 明确记为 macOS 控制面保留项，
不算作「已迁」。** Linux VM 里只有 `claude` CLI（重装 + `CLAUDE_CONFIG_DIR` + 重登录）。

## 5. 目标拓扑、环境变量与启动方式（可复现）

```text
macOS APFS（外置 AgentStorage，保留 ≥200 GiB）
  └─ Lima 生产 VM（真实 raw virtio 块盘，≤700 GiB，见 #1244）
       └─ LVM-VDO（LZ4，dedup off）→ XFS（mkfs.xfs -K，周期 fstrim）
            └─ /srv/agents/claude/          ← Claude Code 数据面（本票）
```

环境变量（写进 guest shell profile，例 `~/.bashrc` 或受管 env）：

```bash
export CLAUDE_CONFIG_DIR="/srv/agents/claude"
# 可选：固定 projects/ 子目录名（v2.1.234+，与 CLAUDE_CONFIG_DIR 配套）：
# export CLAUDE_CODE_PROJECT_DIR_NAME="claude"
```

启动方式（Linux 重装，不复制 macOS 二进制）：

```bash
curl -fsSL https://claude.ai/install.sh | bash    # launcher: ~/.local/bin/claude -> ~/.local/share/claude/versions/<ver>
export PATH="$HOME/.local/bin:$PATH"
claude --version                                  # 期望 >= 2.1.246
claude auth login          # 或 claude setup-token（长活 token）；macOS Keychain 不复制
```

回滚 = **`unset CLAUDE_CONFIG_DIR`**（回到 guest 默认 `~/.claude`），macOS 旧路径自始未动（§7）。

## 6. 迁移程序（两机、三阶段）

> 复制用 `rsync`（macOS/Linux 均有；`-A`/`-X` 保留 ACL/xattr）。跨机走 `rsync -e ssh`（一次性复制，
> 不是 NFS/SMB 常驻挂载——#1236 禁的是把 socket/lock/SQLite 放到网络文件系统，一次性 rsync 不在此列）。
> `~/.claude.json` 的**凭据剔除必须在 macOS 源侧完成**（`config-json`），原始文件不得跨机。

### 6.0 停写点与 writer 清单

Claude Code 无常驻 daemon；「停写」= 确认没有任何 writer 再写 `~/.claude/`：

- 退出所有 `claude` 交互/后台会话（`claude agents` 列出后台会话并停掉；`pgrep -f claude` 应无存活 CLI）；
- 退出 Claude Desktop 应用、VS Code 的 Claude Code 扩展、JetBrains 插件（官方 setup 页明写这三者**都写 `~/.claude/`**）；
- 停写后 macOS 侧不再启动任何 Claude 会话，直到目标机验收通过。

### 6.1 阶段 0：源侧预检 + 凭据剔除（macOS）

> 以下命令在仓库 `.sandcastle/migrate-claude-data-surface/` 目录下执行（`C=./claude-migrate.sh`）。

```bash
C=./claude-migrate.sh
$C preflight --src "$HOME/.claude" --dst "/srv/agents/claude" --config-json "$HOME/.claude.json"
$C config-json --config-json "$HOME/.claude.json" --out "$HOME/.claude.json.redacted"
# 人工抽查：redacted 文件只剩 mcpServers 键（cat/jq 目视），无任何 auth/oauth/token 字段。
```

`config-json` fail-closed：`jq` 缺失或源 JSON 非法即拒绝；默认**只保留 `mcpServers`**，其余全弃。

### 6.2 阶段 1：全量复制（macOS → VM，服务在线）

```bash
rsync -aAX --numeric-ids --delete --partial \
  --exclude 'sessions/' --exclude 'shell-snapshots/' --exclude 'backups/' \
  --exclude 'debug/' --exclude 'image-cache/' --exclude 'paste-cache/' \
  --exclude 'uploads/' --exclude 'todos/' --exclude 'statsig/' --exclude 'logs/' \
  --exclude 'history.jsonl' --exclude 'remote-settings.json' --exclude 'policy-limits.json' \
  --exclude '.claude.json' \
  -e ssh "$HOME/.claude/" <vmuser>@<vm>:/srv/agents/claude/

# redacted 全局配置落到目标机的 CLAUDE_CONFIG_DIR 内部（实证布局）：
scp "$HOME/.claude.json.redacted" <vmuser>@<vm>:/srv/agents/claude/.claude.json
```

> 同一组 excludes 也内置于 `claude-migrate.sh`（`baseline`/`final-copy` 子命令，`RSYNC` 可覆盖以在沙盒对拍）。
> `--exclude '.claude.json'` 必不可少：该文件只经 §6.1 `config-json` 白名单 + 下方 `scp` 落位；
> 若 rsync 不排除它，`--delete` 会把目标机刚就位的 redacted `.claude.json` 当作「源侧不存在」删掉。
> 需要提示历史时显式 `--include-history`（默认禁复制，因 history.jsonl 可能含粘贴的明文凭据）。

### 6.3 阶段 2：停写后最终增量（macOS）

```bash
# 先执行 §6.0 停写清单，再跑增量收尾（同参数；excludes 必须含 '.claude.json'，否则 --delete 删掉 redacted 配置）
rsync -aAX --numeric-ids --delete --partial \
  --exclude 'sessions/' ...（同上全组 excludes，含 '.claude.json'）\
  -e ssh "$HOME/.claude/" <vmuser>@<vm>:/srv/agents/claude/
```

### 6.4 阶段 3：目标机切换 + 校验 + 重登录（VM）

```bash
C=./claude-migrate.sh
$C emit-env --dst "/srv/agents/claude"           # 打印 export 片段，抄进 guest shell profile
$C verify    --dst "/srv/agents/claude"           # 非空 / .claude.json 合法且无凭据键 / 无 sessions/
export CLAUDE_CONFIG_DIR="/srv/agents/claude"
claude doctor                                      # 平台/安装/设置健康（读设置、不 trust prompt）
claude auth login                                  # 重登录；凭据落在 /srv/agents/claude 下（Linux 语义）
claude mcp list                                    # 核对 user 档 MCP（probe 应列出）
claude plugin list                                    # 核对已装插件
```

## 7. 回滚步骤（可复现）

| 情形 | 动作 |
|---|---|
| 目标机验收失败（env 切换后） | `unset CLAUDE_CONFIG_DIR`（并从 profile 移除）；旧 macOS `~/.claude/` 一直未动、继续可用 |
| macOS 源侧未停写（阶段 1 已复制） | 无切换发生；直接重跑阶段 1（rsync 幂等增量） |
| 目标 `.claude.json` 混入凭据 | 立即 `rm /srv/agents/claude/.claude.json`，重跑 §6.1 `config-json` 再 `scp` |
| Desktop VM 被误动 | 已按 §4.2 保留 macOS；不得回滚到「复制过 VM」——本票程序从未复制它 |

旧路径退役（`~/.claude/` 只读/删除）不在本票，归 #1237 单独批准的退役步骤。

## 8. 验收清单（对应 issue 验收三条 + agent brief）

**沙盒内已核（本票交付时完成）：**

- [x] 官方配置入口盘点（§3，`CLAUDE_CONFIG_DIR` 单一杠杆 + 各目录表，实证）。
- [x] Desktop/VM 每个大目录有归属决定（§4.2 决策表 + 证据）。
- [x] 迁移程序/回滚步骤写成可复现命令（§5–§7），`claude-migrate.sh` 沙盒对拍通过。

**宿主侧执行（有真机/真 VM/真凭据后逐条打勾，本票不虚报为已完成）：**

- [ ] guest 内 `CLAUDE_CONFIG_DIR=/srv/agents/claude` 生效，`claude doctor` 无设置错误。
- [ ] 登录通过：`claude auth status` 显示已登录（重登录，非复制 Keychain）。
- [ ] MCP：`claude mcp list` 列出 user 档（含本仓 `claude-audit` 项目档与 TradingView MCP 的迁移态）。
- [ ] skills/plugins：`~/.claude/skills/`、`plugins/` 在新卷可发现；`claude plugin list` 与源侧一致。
- [ ] session 恢复：用一个迁移前真实 session ID 跑 `claude --resume <id>` / `claude --continue` 成功续上。
- [ ] 代表性 agent 任务：在新卷 + 本仓 worktree 里跑一个真实实现/评审任务，产出正确。
- [ ] browser/GUI 协作：确认属 Desktop 控制面保留项（macOS 侧 Desktop 正常，Linux CLI 不承担 GUI）。
- [ ] 旧路径无双写分叉：停写后 macOS 侧无 `claude` 存活进程、无新 `.jsonl` 落盘。
- [ ] `du -s /srv/agents/claude` 记入 claude 组计量；VDO 物理占用/宿主剩余符合 #1244 告警线。

## 9. 待执行时确认项（诚实清单，宿主侧执行前逐条钉死）

1. **生产 VM 尚未建**：canary 是 64 GiB 数据盘；生产 VM 按 #1244 建 ≤700 GiB raw 块盘 + VDO/XFS（沿用 #1241）。
   本票不建盘。
2. **`~/.claude/projects/` 小文件 p95**：#1239 显示 SQLite/小文件在 VDO 上明显变慢；迁移前在 canary 卷
   抽样读写该目录，超过阈值即按 #1239 风险口径向 map owner 上报，不硬迁。
3. **仓库项目级 hooks 的绝对路径**：本仓 `.claude/settings.json` 的 `UserPromptSubmit` hook 硬编码
   `bash /Users/silencehan/Projects/NewChanlun/scripts/wayfinder_session_brief.sh`。若 Claude Code 在 VM 内
   跑该仓库，此 macOS 绝对路径不存在——需二选一：仓库路径在 VM 内一致挂载/同步，或该 hook 做
   host 条件化（`uname` 分支）。本票只标记，改仓库 hook 另票。
4. **执行位置**：默认 #1236 ①「CLI 在 Linux VM 内运行」。若保留 macOS 侧 CLI 而仅把数据面放 XFS，
   则需单 VM 受控导出且 socket/lock 不放网络文件系统——与本票「数据面进 XFS」目标冲突，不推荐。
5. **`.claude.json` 白名单的充分性**：只保留 `mcpServers` 是保守默认；若需要保留 trust/审批态，
   在源侧人工核对 `projects` 键后由 operator 显式决定（本脚本不自动放行任何额外键）。

## 10. 失败处理

- `claude-migrate.sh` 任一子命令失败：按 §6/§7 对应「失败时」列处理，不硬闯。
- `config-json` 无法产出干净文件：**不得**把原始 `.claude.json` 跨机；留 #1238 票面卡点（复现证据 +
  解除条件 + 续跑命令）。
- 回滚也失败：立即停止一切切换动作，在 #1238 票面留卡点，收工。

## 11. 来源与核实记录

- 官方文档（code.claude.com/docs，2026-08-26 抓取）：`settings`（settings 层级 + `.claude.json` 内容）、
  `claude-directory`（`~/.claude/` 全目录表 + 明文存储告警 + `claude project purge`）、`env-vars`
  （`CLAUDE_CONFIG_DIR`/`CLAUDE_CODE_PROJECT_DIR_NAME`/`ANTHROPIC_API_KEY` 语义）、`mcp`（三档落盘 +
  `add-from-claude-desktop`）、`setup`（native installer 布局 + Desktop/IDE 也写 `~/.claude/`）、
  `cli-reference`（`--resume`/`--continue`/`-r`）。
- 沙盒实证（`claude` 2.1.246 native 安装，未登录、未发 API 请求）：`CLAUDE_CONFIG_DIR` 把 `.claude.json`+
  `backups/` 改写到配置目录内部；`mcp add --scope user|local|project` 三档落盘位置；fresh `.claude.json`
  顶层键实测。
- 本仓事实：`.claude/settings.json`（hook 绝对路径）、`.claude/settings.local.json`、`.mcp.json`
  （`claude-audit`）、AGENTS.md（`~/.claude/.mcp.json` TradingView MCP、skills 正本口径）。
- 裁决：`#1244`（dedup off/700GiB/500GiB 红线/400+450GiB 告警）、`#1236`（边界与容量来源）、`#1239`（性能风险）。
