# 官方 GitHub Actions scaffold 来源与逐文件映射（#1110 AC-1）

## upstream 锁定

| 项 | 值 |
|---|---|
| 上游仓库 | `mattpocock/sandcastle`（`@ai-hero/sandcastle` 的官方源） |
| upstream commit | `e99f832f26dc9d245c019a9ddd19fa5dee792427`（2026-06-29，Merge PR #832 changeset-release/main，即 tag `v0.12.0`） |
| 本仓锁定 npm 版本 | `@ai-hero/sandcastle@0.12.0`（`package.json` / `package-lock.json`，与 tag 同一提交） |

本票未在本仓现有 `.sandcastle/` 上跑 `init` 覆盖：事件驱动 workflow 与 `agent-workflows/`
是上游 git 源码（tag `v0.12.0`）逐文件迁入，走 `@ai-hero/sandcastle@0.12.0` 已发布的
`dist/`（devDependency），不编译 sandcastle 库本体。

## 逐文件来源

### workflows（`.github/workflows/`）

| 本仓文件 | 上游源 | 改动 |
|---|---|---|
| `agent-implement.yml` | 同名 | ① issue shape 适配（见下）；② `npm run build`→`npm run typecheck`；③ 删除上游「Refuse sub-issue」步（leaf sub-issue 允许）；④ 新增「Refuse blocked issue」步。 |
| `agent-review.yml` | 同名 | `npm run build`→`npm run typecheck`；顶部注释补 pull_request_target/fork 口径。 |
| `agent-implement-pr.yml` | 同名 | `npm run build`→`npm run typecheck`；顶部注释同上。 |
| `agent-explore.yml` | 同名 | `npm run build`→`npm run typecheck`。 |
| `agent-update-branch.yml` | 同名 | `npm run build`→`npm run typecheck`；顶部注释同上。 |

### agent-workflows（`.sandcastle/agent-workflows/`）

| 本仓文件 | 上游源 | 改动 |
|---|---|---|
| `shared/common.ts` | 同名 | 移除 `claudeAgent()`（迁到 `shared/agent.ts`）与 sandcastle import。 |
| `shared/agent.ts` | **新文件**（自 `common.ts` 的 `claudeAgent` 拆出） | **AC-4 第二处适配**：模型常量集中 + `claudeCode()` + 缺 secret fail-loud。 |
| `shared/diff-lines.ts` | 同名 | 无（逐字）。 |
| `shared/review-context.ts` | 同名 | 相对 import 加 `.ts` 后缀（本仓 tsconfig `moduleResolution: nodenext`）。 |
| `shared/review-output.ts` | 同名 | 同上。 |
| `shared/run-with-extraction.ts` | 同名 | 无（仅 import 包）。 |
| `shared/detect-issue-shape.sh` | **新文件**（自 `agent-implement.yml` 的 Detect issue shape 内联 shell 提取） | **AC-4 第一处适配**：shape 判定独立成脚本，便于机械验证/对拍。 |
| `shared/shared.test.ts` | **新文件**（靶向测试） | `diff-lines`/`review-output`/`common` 纯逻辑对拍。 |
| `implement/implement.ts` | 同名 | `claudeAgent("implement")`；相对 import 加 `.ts`。 |
| `implement/prompt.md` | 同名 | 无（逐字）。 |
| `review/review.ts` | 同名 | `claudeAgent("review")`；相对 import 加 `.ts`。 |
| `review/prompt.md` / `review/extraction.md` | 同名 | 无（逐字）。 |
| `implement-pr/implement-pr.ts` | 同名 | `claudeAgent("implement-pr")`；相对 import 加 `.ts`。 |
| `implement-pr/prompt.md` / `implement-pr/extraction.md` | 同名 | 无（逐字）。 |
| `explore/explore.ts` | 同名 | `claudeAgent("explore")`；相对 import 加 `.ts`。 |
| `explore/prompt.md` / `explore/extraction.md` | 同名 | 无（逐字）。 |
| `update-branch/update-branch.ts` | 同名 | `claudeAgent("update-branch")`；相对 import 加 `.ts`。 |
| `update-branch/prompt.md` | 同名 | `npm run test` 改为 `cargo check` / `pytest`（本仓无 npm test）。 |
| `update-branch/extraction.md` | 同名 | 无（逐字）。 |

## 两处且仅两处必要适配（AC-4）

1. **issue shape**（`agent-implement.yml` + `detect-issue-shape.sh`）：
   - 有子票的 map/PRD（`sub_issues_summary.total > 0`）→ 拒绝；
   - **leaf sub-issue（有 parent、无子票、无 open blocker）→ 允许**（上游拒绝一切 sub-issue，此处改判）；
   - 原生 open blocker（`issue_dependencies_summary.blocked_by > 0`）→ 拒绝并标 `agent:blocked`。
2. **agent auth/model**（`shared/agent.ts`）：
   - Prime 是模型无关控制面，不进入 workflow；GitHub-hosted runner 用 Sandcastle 内置
     `claudeCode()` + GitHub Secret `CLAUDE_CODE_OAUTH_TOKEN`；
   - 模型常量集中：实装 Sonnet（`claude-sonnet-4-6`）、评审 Opus（`claude-opus-4-8`）；
   - 缺 secret fail-loud（写 `failure_reason.txt` + 非零退出 → workflow 回写 blocked + 评论）。

## 本仓必要接线（非语义适配）

- `package.json`：加 `typecheck` script + `@standard-schema/spec` devDependency。
- `tsconfig.sandcastle.json`：include 增 `agent-workflows/**/*.ts`。
- `.sandcastle/agent-workflows/package.json`：`{"type":"module"}`——上游在仓库根声明
  `type: module`，本仓 Rust/Python 为主、根 package.json 无 type 字段，故把 ESM 作用域
  收窄到本目录（tsgo `moduleResolution: nodenext` 依赖它识别 ESM）。
- `shared/common.ts` 的 `fail()` 保留上游 `process.exit(1)` 语义（见 `agent.ts` 内
  `|| fail(...)` 注释：tsgo 不把 never 调用当控制流终止点）。

## 验证（AC-6/AC-7）

| 验证 | 命令 | 结论 |
|---|---|---|
| actionlint | `actionlint .github/workflows/agent-*.yml` | 全过 |
| YAML/结构/shape 对拍/禁入扫描 | `python3 .sandcastle/verify-workflows.py`（需 PyYAML，与仓内 31 个 yaml 脚本同环境） | 全过 |
| TypeScript | `npm run typecheck` | 全过 |
| 脚本靶向测试 | `node --import tsx --test .sandcastle/agent-workflows/shared/shared.test.ts` | 7 全过 |

不 push、不激活真实标签、不运行真实付费 agent；凭据与专用 smoke issue 另票后再实装（本票边界）。
