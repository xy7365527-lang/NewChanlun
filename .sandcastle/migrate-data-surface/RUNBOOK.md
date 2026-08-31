# 迁移 Runbook：Sandcastle / Wayfinder / Prime Agent 数据面 → XFS on LVM-VDO（#1235）

> 本文件是 #1235 的迁移执行手册。执行前先读 #1227（总图）、#1236（边界与容量）、
> #1244（dedup/容量/配额/告警裁决）、#1241（canary 建盘参考）、#1239（基准与恢复证据）。
> 配套原语：`migrate-surface.sh`（同目录），沙盒对拍：`migrate-surface.test.sh`。

## 1. 适用范围与出界

| 项 | 归属 |
|---|---|
| Sandcastle / Wayfinder 可写数据面 | **本票**（`/srv/agents/sandcastle`） |
| Prime Agent 可写数据面 | **本票**（`/srv/agents/prime`） |
| Claude Code / Desktop（#1238）、Codex CLI / Desktop（#1240） | 出界，另票 |
| 分批切换/监控/回滚演练/旧路径退役（#1237） | 出界，本票完成后接续 |
| macOS GUI、Keychain、应用签名包、厂商 VM（#1236 裁定） | 保留 macOS 控制面，不迁 |

## 2. 已裁口径（写码/执行前复核这些数字）

来自 #1244（编排者拍板，2026-08-26）：

| 项 | 裁决 |
|---|---|
| dedup | **维持 off** |
| 生产容量 | **700 GiB 确认**（不调整） |
| 配额 | **500 GiB 单一总量红线**，不设独立硬配额；**必须保留四组分项计量** |
| 告警 | **warning 400 GiB / critical 450 GiB**（可用预算 80%/90%） |
| 执行约束 | 达到 critical 立即停扩张型任务并清理/扩容；单数据面挤占风险靠四组分项计量暴露 |

来自 #1236：

- 可写数据面与 Linux 兼容执行面进**单一 Linux VM**；**禁止多 VM 同时块级挂载同一 XFS**。
- SQLite/WAL、Unix socket、lockfile **不放 NFS/SMB/FUSE**——数据面由该 VM 本地挂载 XFS。
- 运行位置二选一：① 服务在 Linux VM 内运行；② 单 VM 挂载后受控导出。**本票默认 ①**（见 §7 确认项）。
- 宿主外置 APFS（`/Volumes/AgentStorage`）**至少保留约 200 GiB 余量**；canary 不放凭据。
- 生产迁移前：外置卷启用 ownership，或 guest 内受控加密并重新登录——**不复制现有 auth/Keychain**。

来自 #1239（已知风险）：默认 VDO 性能门槛 **FAIL**（`git add` 慢 17.2%、SQLite WAL/FULL 慢 81%）。
#1244 已关且未再落调优项；**迁移每阶段后核对代表流程 p95 不超阈值**（本票验收 3 条），异常即停。

## 3. 目标拓扑

```text
macOS APFS（外置 AgentStorage，保留 ≥200 GiB）
  └─ Lima 生产 VM（真实 raw virtio 块盘，非 loop；≤700 GiB，见 #1244）
       ├─ LVM PV/VG → LVM-VDO（LZ4，dedup off）→ XFS（mkfs.xfs -K，周期 fstrim）
       └─ /srv/agents/{sandcastle,prime,claude,codex}   ← 本票只动 sandcastle/prime
```

canary 建盘参考 #1241：`agent-vdo-canary`、AlmaLinux 9.8 aarch64、VDO 48G phys/56G logical、
`/srv/agents` 按 UUID systemd 挂载、`fstrim.timer` enabled。

## 4. 数据面路径清单

### 4.1 Prime Agent（可配置目录 + 全局协调状态，已按 prime-agent 源码核实）

| 内容 | 旧路径（macOS 默认） | 配置入口 | 处置 |
|---|---|---|---|
| agent 目录 | `~/.prime/agent/` | `PRIME_AGENT_CODING_AGENT_DIR`（prefix 由 `piConfig.name=prime-agent` 而来） | 迁 `/srv/agents/prime/` |
| sessions | `<agent>/sessions/` | `PRIME_AGENT_SESSION_DIR`（旧名 `PRIME_AGENT_CODING_AGENT_SESSION_DIR`） | 迁 |
| session-artifacts | `<dirname(sessions)>/session-artifacts/<id>/`（sessions 兄弟目录） | 随 sessions 位置联动 | 迁 |
| kernel-venv | `~/.prime/agent/kernel-venv` | `PRIME_AGENT_KERNEL_VENV` | 迁（或 guest 重建） |
| harness（全局） | `<agent>/harness/` | 随 agent 目录（无独立 env；`RLM_GLOBAL_HARNESS_STATE_DIR` 是子进程输出 env，非输入） | 迁 |
| logs | `~/.prime/agent/logs/` | 随 agent 目录 | 迁 |
| auth/settings/models/cron | `<agent>/auth.json` 等 | 随 agent 目录 | **auth 含凭据：不复制进普通卷**（§8） |
| **supervisor socket** | `$TMPDIR/prime-agent-<uid>/daemon.sock` | `PRIME_AGENT_INTERNAL_DAEMON_SUPERVISOR_SOCKET` | **全局协调状态，禁迁/禁 symlink/禁双写** |
| **supervisor owner registry** | `$TMPDIR/prime-agent-<uid>/supervisor-owners/` | `PRIME_AGENT_INTERNAL_DAEMON_SUPERVISOR_REGISTRY_DIR` | **同上，禁迁** |

> 关键：socket 与 owner registry 默认在 `$TMPDIR`（`/var/folders/.../T` 或 `/tmp`），**不在**
> `~/.prime/agent/` 内；迁 `~/.prime/agent/` 天然不碰它们。若曾用 env 把它们改到 agent 目录内，
> 迁移时必须 `--exclude '*.sock' --exclude 'supervisor-owners'` 并在停写前 `prime-agent shutdown`。

### 4.2 Sandcastle / Wayfinder（仓库脚本/plist 硬编码，见 `.sandcastle/`）

| 内容 | 旧路径 | 写者 |
|---|---|---|
| 主 worktree | `/Users/silencehan/Projects/NewChanlun` | wayfinder-engine、watcher、harvest |
| sandcastle host worktree | `/Users/silencehan/Projects/NewChanlun-sandcastle-host` | claim-loop、arch-hotzone |
| 状态/日志 | `<repo>/.sandcastle/logs/`（workers.jsonl、wayfinder-status.md、STATUS.md、stdout/stderr） | 各自服务 |
| 每票 worktree | `<repo>/.sandcastle/worktrees/`（gitignore 运行产物） | main.mts |
| Docker 数据 | Colima ext4/overlayfs | Docker daemon |
| launchd 定义 | `~/Library/LaunchAgents/com.newchanlun.{wayfinder-engine,sandcastle-claimer,arch-hotzone}.plist` | launchd |

## 5. 迁移顺序（issue 执行要求，必须按此序）

1. **canary agent**：一个无生产权威状态的临时 prime-agent session（fresh 目录，不接 live 会话），
   用本原语完整走一遍并回滚演练——验证程序本身，不碰生产。
2. **runner cache / workspace**：Sandcastle 每票 worktree、日志/状态、Docker cache。
3. **session / artifact**：Prime sessions、session-artifacts、harness、kernel-venv。
4. 每类独立观察期后再下一类；**live claimer/engine 在迁移前不得停止**（停写点见 §6.3）。

## 6. 通用迁移程序（每数据面一遍）

用 `migrate-surface.sh`（POSIX bash + rsync；macOS/Linux 均可）。手工流程：

> **执行前字节与环境自检（#1235 人工闸退回，2026-08-29）**：`migrate-surface.sh` 与
> `migrate-surface.test.sh` 均为**纯 ASCII**（无中文等非 ASCII 字节），从根上消除「传送/编码
> 坏字节打断 bash 3.2 解析」这一类故障（宿主 macOS 默认 bash 3.2，沙盒 bash 5 会容忍坏字节）。
> 已对拍 GNU bash 3.2.57（macOS 同主次版本）与 bash 5.x，场景 A–G 全过。执行前核对：
>
> ```bash
> LC_ALL=C grep -n '[^ -~]' migrate-surface.sh migrate-surface.test.sh   # 应为空（纯 ASCII）
> ```
>
> **rsync 默认参数为 `-a --numeric-ids --delete --partial`**（macOS 自带 rsync 2.6.9 无 `-A/-X`）；
> Linux rsync 3.x 需要保留 ACL/xattr 时追加 `--rsync-opts '-A -X'`，macOS 需要扩展属性时用
> `--rsync-opts '-E'`。

```bash
M=./migrate-surface.sh
$M preflight  --src <旧> --dst <新> --name <面> [--exclude ...] [--lock-dir ...]
$M baseline   --src <旧> --dst <新> --name <面> [--exclude ...] [--rsync-opts ...]      # 服务在线，全量
$M stop-write --src <旧> --dst <新> --name <面> --stop-hook '<停写命令>'                  # ← 停写点
$M final-copy --src <旧> --dst <新> --name <面> [--exclude ...]                          # 停写后增量
$M switch     --src <旧> --dst <新> --name <面> [--pre-switch-check-hook ...] [--backup-dir ...]   # 切换前确认旧写者已停；mv 旧→备份 + ln -s 新
$M start      --src <旧> --dst <新> --name <面> --start-hook '<启动命令>'
$M healthcheck --src <旧> --dst <新> --name <面> --health-hook '<健康检查命令>'            # 非零即失败
$M verify     --src <旧> --dst <新> --name <面>
$M readonly-old --src <旧> --dst <新> --name <面>                                        # 旧路径 a-w
$M done       --src <旧> --dst <新> --name <面>                                          # 释放锁
```

或一条龙（失败自动回滚）：

```bash
$M full --src <旧> --dst <新> --name <面> \
  --stop-hook '<停写命令>' --start-hook '<启动命令>' --health-hook '<健康检查命令>' \
  [--pre-switch-check-hook '<确认无活跃 writer 命令>']
```

### 6.1 各步语义

| 步 | 语义 | 失败时 |
|---|---|---|
| preflight | 源/目标存在性、SRC∉DST、空间预检（目标可用 < 源占用即拒）、取锁 | 直接失败，无副作用 |
| baseline | rsync 全量镜像（服务在线，可重跑） | 可重跑 |
| stop-write | 执行停写 hook，**自此旧路径停写**；记阶段 `stop-write` | 回滚分支重启旧服务 |
| final-copy | rsync 增量收尾（停写后，秒级） | 回滚分支重启旧服务 |
| switch | `mv 旧 → 旧.migrate-backup.<ts>` + `ln -s 新 旧`（同父目录两次 rename，原子）；写 manifest | symlink 失败自动 mv 回 |
| start | 执行启动 hook（新路径） | 回滚分支拆 symlink 恢复旧路径 |
| healthcheck | 执行健康检查 hook | 回滚 |
| verify | 结构校验：旧路径是→新的 symlink、manifest 在、目标非空 | 回滚 |
| readonly-old | 备份 `chmod -R a-w` 并实测不可写 | 失败（仍可写）即报 |
| rollback | 按阶段分派：未停写→无动作；已停写未切换→重启旧服务；已切换→拆 symlink+mv 回+重启 | 需人工介入 |
| done | 释放迁移在途锁 | — |

### 6.2 原子切换说明

`mv`（同目录 rename）与 `ln -s` 都是原子元数据操作；中间不存在「旧路径既不是目录也不是 symlink」
的窗口之外状态。切换后旧数据完整留在 `<旧>.migrate-backup.<ts>`，只读保留到 #1237 退役步骤批准。

### 6.3 停写点与无双写 writer

- **停写点 = `stop-write` 完成那一刻**。之后到 `start` 之间，旧路径与新路径都不该有写者。
- 切换前用 `--pre-switch-check-hook` 二次确认旧写者已停（例：`! pgrep -f 'wayfinder_engine.mts'`）。
- 锁目录（默认 `$HOME/.migrate-surface-locks/<name>`）保证同名数据面同时只有一个迁移在跑。
- **Prime 专属**：迁移前 `prime-agent shutdown`（停 supervisor + 全部 worker）；**不得**在两个位置
  各起一个共享同一 supervisor socket / owner registry 的 daemon——socket/registry 只保留一份
  （默认 `$TMPDIR` 那份），新位置若需独立 daemon 须用独立 `PRIME_AGENT_INTERNAL_DAEMON_SUPERVISOR_SOCKET`。

## 7. 待执行时确认项（诚实清单，宿主侧执行前逐条钉死）

1. **生产 VM 尚未建**：canary 是 64 GiB 数据盘；生产 VM 需按 #1244 建 ≤700 GiB raw 块盘 + VDO/XFS
   （沿用 #1241 步骤）。本票不建盘，建盘是执行第一步。
2. **执行位置**：默认按 #1236 ①「服务在 Linux VM 内运行」；若保留 launchd 在 macOS，须由该单 VM
   受控导出且**不得**用 NFS/SMB/FUSE 承载 socket/lock/SQLite。hook 命令据此二选一（§8 两套）。
3. **auth 处置**：外置卷 ownership 启用 or guest 内受控加密重新登录——二选一，不复制 auth.json。
4. **RTO 目标**：建议 ≤15 分钟（回滚=拆 symlink+mv 回+重启），须在 #1237 回滚演练实测确认。

## 8. 各数据面具体命令（hook 示例）

### 8.1 canary agent（阶段 1，无生产权威状态）

```bash
# 新起一个独立 agent 目录（不接 live 会话、不 share supervisor socket）
export PRIME_AGENT_CODING_AGENT_DIR="$HOME/.prime-canary/agent"
export PRIME_AGENT_INTERNAL_DAEMON_SUPERVISOR_SOCKET="/tmp/prime-agent-canary/daemon.sock"
# 以无头模式起一条一次性 worker（`prime-agent -p --mode json`，见 .sandcastle/prime-agent-provider.ts），
# 产生 session/artifact 后再停：prime-agent shutdown

M=.sandcastle/migrate-data-surface/migrate-surface.sh
$M full \
  --src "$HOME/.prime-canary/agent" --dst "/srv/agents/prime/canary/agent" --name prime-canary \
  --exclude '*.sock' --exclude 'supervisor-owners' \
  --stop-hook 'true' --start-hook 'true' \
  --health-hook 'test -f /srv/agents/prime/canary/agent/settings.json'
# 验证 + 回滚演练 + 删除 canary 目录（见 §10）
```

### 8.2 runner cache / workspace（阶段 2，Sandcastle）

macOS launchd 版本（服务保留 macOS 时）：

```bash
$M full --src "/Users/silencehan/Projects/NewChanlun-sandcastle-host/.sandcastle/worktrees" \
        --dst "/srv/agents/sandcastle/worktrees" --name sandcastle-worktrees \
        --stop-hook 'launchctl unload ~/Library/LaunchAgents/com.newchanlun.sandcastle-claimer.plist' \
        --start-hook 'launchctl load ~/Library/LaunchAgents/com.newchanlun.sandcastle-claimer.plist' \
        --health-hook 'launchctl list | grep -q com.newchanlun.sandcastle-claimer' \
        --pre-switch-check-hook '! pgrep -f "claim-loop.mts"'

$M full --src "/Users/silencehan/Projects/NewChanlun/.sandcastle/logs" \
        --dst "/srv/agents/sandcastle/logs" --name sandcastle-logs \
        --stop-hook 'launchctl unload ~/Library/LaunchAgents/com.newchanlun.wayfinder-engine.plist' \
        --start-hook 'launchctl load ~/Library/LaunchAgents/com.newchanlun.wayfinder-engine.plist' \
        --health-hook 'launchctl list | grep -q com.newchanlun.wayfinder-engine'
```

guest systemd 版本（服务迁入 VM，默认）：

```bash
$M full --src "<host-mount>/worktrees" --dst "/srv/agents/sandcastle/worktrees" --name sandcastle-worktrees \
        --stop-hook 'systemctl stop sandcastle-claimer' \
        --start-hook 'systemctl start sandcastle-claimer' \
        --health-hook 'systemctl is-active --quiet sandcastle-claimer'
```

> Docker/Colima 数据面（ext4/overlayfs）迁移 = 在目标 VM 重建 Colima/Docker 实例并把镜像/缓存迁入
> `/srv/agents/sandcastle/docker`；具体按执行位置确认项决定（§7.2），本票不预设 unit 名。

### 8.3 session / artifact（阶段 3，Prime）

```bash
prime-agent shutdown   # 停写点前置：停 supervisor + workers

$M full --src "$HOME/.prime/agent/sessions" --dst "/srv/agents/prime/sessions" --name prime-sessions \
        --stop-hook 'true' --start-hook 'true' \
        --health-hook 'test -d /srv/agents/prime/sessions'

$M full --src "$HOME/.prime/agent/session-artifacts" --dst "/srv/agents/prime/session-artifacts" --name prime-artifacts \
        --stop-hook 'true' --start-hook 'true' \
        --health-hook 'test -d /srv/agents/prime/session-artifacts'

$M full --src "$HOME/.prime/agent/harness" --dst "/srv/agents/prime/harness" --name prime-harness \
        --stop-hook 'true' --start-hook 'true' \
        --health-hook 'test -d /srv/agents/prime/harness'
```

> kernel-venv（`~/.prime/agent/kernel-venv`）**不 rsync**：Python venv 含绝对路径、跨机/跨路径不可靠。
> 在 guest 删除后由 prime-agent 首次启动自动 bootstrap 重建（缺 `kernel-venv` 会 rebuild，见
> `PRIME_AGENT_KERNEL_VENV` 与 bootstrap 逻辑）。

迁移后用 env 重定向并重启（`auth.json` 不复制，重新登录）：

```bash
export PRIME_AGENT_CODING_AGENT_DIR="/srv/agents/prime"      # agent 目录：harness/logs/models 随之而来
export PRIME_AGENT_SESSION_DIR="/srv/agents/prime/sessions"  # sessions 覆盖（session-artifacts 随之联动）
# 全局 harness 目录 = $PRIME_AGENT_CODING_AGENT_DIR/harness，无需单独 env
# 重启 daemon，起一个 canary session 验证 session continuation + artifact 无回归（§10）
```

## 9. 监控 / 配额 / 告警（#1244 数值）

- 四组分项计量（prime/sandcastle/claude/codex）必须保留；单一总量红线 500 GiB。
- warning 400 GiB / critical 450 GiB（VDO **物理**占用口径，同时监控宿主 APFS 真实剩余）。
- 达到 critical：立即停扩张型任务（停 claimer 拾取新票）并清理/扩容——fail-closed。
- `fstrim.timer` 保持 enabled；`xfs_repair -n` 作为周期健康项。

## 10. 验收清单（对应 issue 验收三条 + agent brief）

- [ ] Prime implement/review、Sandcastle claim loop、Wayfinder engine 代表流程在新卷跑通。
- [ ] session continuation、artifact、worktree、socket、supervisor 所有权行为无回归（新旧各一次对拍）。
- [ ] 旧路径只读、无双写分叉；`rollback` 在约定 RTO 内恢复服务（实测计时）。
- [ ] claimer/wayfinder-engine 常驻正常（launchctl 或 systemd 在跑）；自动拾取链路 smoke 一次。
- [ ] 每阶段后核对代表流程 p95（#1239 性能 FAIL 风险）；异常即停写并上报。

## 11. 失败处理

- `migrate-surface.sh` 任一步失败：按 §6.1「失败时」列处理；`full` 自动回滚。
- 回滚也失败：**立即停止一切切换动作**，锁目录（`$HOME/.migrate-surface-locks/<name>`）保持，
  在 #1235 票面留卡点（复现证据 + 解除条件 + 续跑命令），不硬闯。
