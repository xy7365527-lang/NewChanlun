# .sandcastle —— 两段式工蜂管线（sandcastle × prime-agent）

## 这是什么

本目录是 NewChanlun 的自动化领票干活管线：**sandcastle**（`@ai-hero/sandcastle`）负责沙盒编排（每票一个 Docker 沙盒、一条确定性分支），**prime-agent** 负责在沙盒里实际干活的 agent。

形态是 sandcastle 官方的 **sequential-reviewer 两段式**（#1001 裁定）：

1. **Phase 1 · 实装工蜂（implementer）**——host 侧先按专属拾取闸 label `sandcastle` 取一张未认领的 open issue 并 claim（摘 label + assign @me），然后 `createSandbox({ branch: sandcastle/issue-<号>, baseBranch: main })` 起沙盒，实装工蜂在分支上干活、commit；
2. **Phase 2 · 独立评审工蜂（reviewer）**——同一沙盒里换一个独立 invocation 的评审工蜂，对照分支 diff 与原始 issue 评审，**直接在分支上修正**（评审面 == 关票面）。

两段结束后**不合 main**——验收与合入是编排层人工闸（#1003 裁 3）。实装无 commit 即跳评审、留票待查（视为 backlog 空/被卡）。

## 怎么跑

前置：

- **colima 在跑**——Docker 沙盒依赖本机 Docker daemon（macOS 上经 colima 提供）；
- **`.sandcastle/.env` 凭据齐**——模型 provider（kimi-coding 等）与镜像内工具所需凭据；该文件已 gitignore，密钥永不入仓。

跑法（仓库根）：

```bash
npx tsx .sandcastle/main.mts
```

跑之前确保有可领的票：open + 带 `sandcastle` label + 未 assign。镜像变更后用 sandcastle 的 build-image 重建 `sandcastle:newchanlun`。

## wayfinder_engine 控制面（#1084）

`scripts/wayfinder_engine.mts` 是图清自动汇入主干的常驻控制面（tracker 即账本，stateless）：
每 4 分钟一轮，跑三件事——

1. **DAG 走查**：图清 → 起草 spec 草案开票（@编排者批【闸一】）；spec 批准 → 拆实装票（blocking 边 + 逐条挂裁定票）；实装全关 → 起草图关 comment + close map。
2. **队列交棒**：只给合资格实装票（已批准 spec/DAG + 未 assign + 无 open blocker）挂 `sandcastle`，不把全部历史 `ready-for-agent` 无差别放行；每轮五桶分明（ready-for-agent / sandcastle 拾取权 / blocked / claimed / running）。
3. **状态页**：写 `.sandcastle/logs/wayfinder-status.md`，不再把 `sandcastle` 空误写成 `ready-for-agent` 全空。

gh 查询有限重试 + 指数退避 + fail-loud；无 runnable 票 sleep 后重查、不退出。人工闸三处不动（spec 批准 / 不预授权决策票 / 合入 main）。

**部署成受管常驻服务**（launchd KeepAlive，默认真实执行 `--live`）：

```bash
bash .sandcastle/install-wayfinder-engine.sh install   # 安装并加载
bash .sandcastle/install-wayfinder-engine.sh status    # 查状态
bash .sandcastle/install-wayfinder-engine.sh uninstall # 卸载
```

手工调试（仓库根，不写 tracker）：`npx tsx scripts/wayfinder_engine.mts --once --dry-run`。

## sandcastle 自动拾取 claim 循环（#1182 方向 A）

`claim-loop.mts` 是补上「工蜂拾取」半链的常驻 claim 宿主（#1182 实测：引擎放行后无常驻进程认领，
队列积压靠人工拉 main.mts）。每 60 秒一轮：若 frontier（`sandcastle` 标签 + 未 assign + 无 open blocker）
非空且无 main.mts running 实例 → spawn `npx tsx .sandcastle/main.mts` 并等它收尾，下一轮再查。
防重复认领靠「无 running 实例」guard（`pgrep` 判活，TROUBLESHOOTING #9 同款判据），单线程 spawn+等待
天然不并发多实例；常驻不退出（launchd KeepAlive 兜底）。frontier 查询复用 main.mts 现有逻辑。

**部署成受管常驻服务**（launchd KeepAlive，默认真实执行 `--live`；宿主侧安装由编排者执行）：

```bash
bash .sandcastle/install-sandcastle-claimer.sh install   # 安装并加载
bash .sandcastle/install-sandcastle-claimer.sh status    # 查状态
bash .sandcastle/install-sandcastle-claimer.sh uninstall # 卸载
```

手工调试（仓库根，不写 tracker）：`npx tsx .sandcastle/claim-loop.mts --once --dry-run`。

## 文件地图

| 文件 | 作用 |
|---|---|
| `main.mts` | 管线入口。两段循环：frontier 取票+claim → 起沙盒 → Phase 1 实装 → Phase 2 评审；模型面常量（实装/评审模型、provider、迭代数）集中在文件顶部，升档改这里重跑。sandcastle 专属队列空时分开报全仓 `ready-for-agent` 数（#1084 追加，不把专属空误写成全空）。#1259 追加：claim 成功后、implementer started 之前的失败自动回滚认领（WorktreeTimeoutError 先重试一次，间隔 ≥5s）。 |
| `run-wayfinder-engine.sh` | wayfinder_engine 控制面常驻壳（#1084 追加）：前台 exec `npx tsx scripts/wayfinder_engine.mts`，launchd KeepAlive 调用。 |
| `install-wayfinder-engine.sh` | 控制面 launchd 任务的安装/卸载/查状态（#1084 追加）。 |
| `launchd/com.newchanlun.wayfinder-engine.plist` | 控制面 launchd 任务定义：KeepAlive + RunAtLoad + `--live`（#1084 追加）。 |
| `claim-loop.mts` | 自动拾取 claim 循环常驻宿主（#1182 方向 A）：60s 轮询 frontier（sandcastle 标签 + 未 assign + 无 open blocker）+ 无 running 实例 guard → spawn `npx tsx .sandcastle/main.mts`；纯函数 `isRunnable`/`hasRunnableIssue` 可离线单测。#1259 追加：main.mts 收尾失败（exit≠0/被信号杀）与每轮看门狗（claim 后 >15min 无后续事件）按 workers.jsonl 判定回滚 stale-claimed 票（纯函数 `needsRollback`/`findStaleClaims`）。 |
| `run-claim-loop.sh` | claim 循环常驻壳（#1182 追加）：前台 exec `npx tsx .sandcastle/claim-loop.mts`，launchd KeepAlive 调用。 |
| `install-sandcastle-claimer.sh` | claim 循环 launchd 任务的安装/卸载/查状态（#1182 追加，对齐 install-wayfinder-engine.sh 模式）。 |
| `launchd/com.newchanlun.sandcastle-claimer.plist` | claim 循环 launchd 任务定义：KeepAlive + RunAtLoad + `--live`（#1182 追加）。 |
| `prime-agent-provider.ts` | prime-agent 的自定义 AgentProvider。无头模式 `prime-agent -p --mode json` 输出 NDJSON 事件流，prompt 走 stdin（避开 Linux 128 KB argv 上限），事件映射到 sandcastle 的 stream 协议。 |
| `implement-prompt.md` | Phase 1 实装工蜂的 prompt 模板（`{{ISSUE_NUMBER}}` 注入）：Explore→Plan→Execute→Verify→Commit 流程 + 硬性规则（不关票不合 main 不 push、被卡留评论收工）。 |
| `review-prompt.md` | Phase 2 评审工蜂的 prompt 模板（`{{BRANCH}}`/`{{ISSUE_NUMBER}}` 注入）：看分支 diff 与原始 issue，保持功能语义前提下直接在分支上修正。 |
| `CODING_STANDARDS.md` | 本仓编码标准，两个工蜂都经 `@.sandcastle/CODING_STANDARDS.md` 加载（#1008 AC-3）：Rust 面 fmt/clippy/check 闸、commit 编号声明、引用纪律、架构口径；引用 AGENTS.md 与纪律文档，不复制全文。 |
| `Dockerfile` | 沙盒镜像定义（`sandcastle:newchanlun`，base `node:22-bookworm`）：git/gh/python3 等系统依赖 + Rust 工具链 + prime-agent 本体（#1010 扩容）。 |
| `image/` | 打入镜像的静态资产：`skills/`（serena / codebase-memory / cli-hub-mcp 三件套）、`prime-agent-settings.json`（prime-agent 镜像内配置）、`cli-hub-mcp/server.py`。 |
| `prompt.md.smoke` / `prompt.md.ralph.md` | 改造前的遗留 prompt：冒烟首跑（建 `SANDCASTLE_SMOKE.md`）与 RALPH 自主轮询版；两段式改造（#1008）后不再被 `main.mts` 引用，留档备查。 |
| `migrate-claude-data-surface/` | #1238 Claude Code 数据面迁移手册与助手：`RUNBOOK.md`（官方配置入口 `CLAUDE_CONFIG_DIR` 盘点 + CLI/Desktop 归属决策表 + 两机三阶段迁移/回滚命令 + 验收清单）、`claude-migrate.sh`（preflight/baseline/config-json 凭据白名单/emit-env/verify/rollback，env 切换非 symlink）、`claude-migrate.test.sh`（沙盒对拍 23 例，fake-rsync 模拟镜像语义）。 |

`.gitignore` 排除 `.env` / `logs/` / `worktrees/`。

## 相关票

- #996：sandcastle × prime-agent 首跑（provider 由来）；
- #1001/#1002/#1003：两段式形态裁定；
- #1008：simple-loop → sequential-reviewer 改造（本目录现状的来源）；
- #1010：镜像工具链扩容（Rust + 三 skill + provider 凭据）。
