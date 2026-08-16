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

## 文件地图

| 文件 | 作用 |
|---|---|
| `main.mts` | 管线入口。两段循环：frontier 取票+claim → 起沙盒 → Phase 1 实装 → Phase 2 评审；模型面常量（实装/评审模型、provider、迭代数）集中在文件顶部，升档改这里重跑。 |
| `prime-agent-provider.ts` | prime-agent 的自定义 AgentProvider。无头模式 `prime-agent -p --mode json` 输出 NDJSON 事件流，prompt 走 stdin（避开 Linux 128 KB argv 上限），事件映射到 sandcastle 的 stream 协议。 |
| `implement-prompt.md` | Phase 1 实装工蜂的 prompt 模板（`{{ISSUE_NUMBER}}` 注入）：Explore→Plan→Execute→Verify→Commit 流程 + 硬性规则（不关票不合 main 不 push、被卡留评论收工）。 |
| `review-prompt.md` | Phase 2 评审工蜂的 prompt 模板（`{{BRANCH}}`/`{{ISSUE_NUMBER}}` 注入）：看分支 diff 与原始 issue，保持功能语义前提下直接在分支上修正。 |
| `CODING_STANDARDS.md` | 本仓编码标准，两个工蜂都经 `@.sandcastle/CODING_STANDARDS.md` 加载（#1008 AC-3）：Rust 面 fmt/clippy/check 闸、commit 编号声明、引用纪律、架构口径；引用 AGENTS.md 与纪律文档，不复制全文。 |
| `Dockerfile` | 沙盒镜像定义（`sandcastle:newchanlun`，base `node:22-bookworm`）：git/gh/python3 等系统依赖 + Rust 工具链 + prime-agent 本体（#1010 扩容）。 |
| `image/` | 打入镜像的静态资产：`skills/`（serena / codebase-memory / cli-hub-mcp 三件套）、`prime-agent-settings.json`（prime-agent 镜像内配置）、`cli-hub-mcp/server.py`。 |
| `prompt.md.smoke` / `prompt.md.ralph.md` | 改造前的遗留 prompt：冒烟首跑（建 `SANDCASTLE_SMOKE.md`）与 RALPH 自主轮询版；两段式改造（#1008）后不再被 `main.mts` 引用，留档备查。 |

`.gitignore` 排除 `.env` / `logs/` / `worktrees/`。

## 相关票

- #996：sandcastle × prime-agent 首跑（provider 由来）；
- #1001/#1002/#1003：两段式形态裁定；
- #1008：simple-loop → sequential-reviewer 改造（本目录现状的来源）；
- #1010：镜像工具链扩容（Rust + 三 skill + provider 凭据）。
