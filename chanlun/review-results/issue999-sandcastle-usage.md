# Sandcastle 官方用法调研报告（#999）

- 调研对象：`@ai-hero/sandcastle`（npm），上游仓 `mattpocock/sandcastle`（GitHub，公开，MIT，约 7.4k star，默认分支 `main`）
- 调研时间基准：上游 main 分支当前 HEAD；最新发布 0.12.0（与本仓已装版本一致）
- 调研方式：GitHub API / raw.githubusercontent.com 直抓（README、docs/、src/templates/、.sandcastle/ 狗食目录、src/ 核心源码、CHANGELOG、251 个 open+closed issue 标题与关键 issue 正文）；aihero.dev sitemap + 文章直抓
- 信源说明：**该仓未启用 GitHub Discussions**（`https://github.com/mattpocock/sandcastle/discussions` 返回 404），社区讨论载体是 Issues；aihero.dev 全站 sitemap（188 个 URL）**无 sandcastle 专文**，Matt 对 sandcastle 的定位陈述主要在 README 与其 RALPH 方法论系列文章中。会话期间 websearch（Serper）未配置 key，未能检索 Matt 的 YouTube/X 发布视频，相关定位标「未证实」。

来源缩写：`[README]` = https://github.com/mattpocock/sandcastle/blob/main/README.md ；`[CHANGELOG]` = https://github.com/mattpocock/sandcastle/blob/main/CHANGELOG.md ；issue 引用为 `https://github.com/mattpocock/sandcastle/issues/<N>`。

---

## 1. 官方设定的典型工作流形态

### 1.1 产品定位（官方原话）

README 开宗明义：

> "A TypeScript library for orchestrating AI coding agents in isolated sandboxes: 1. You invoke agents with a single `sandcastle.run()`. 2. Sandcastle handles sandboxing the agent with a configurable branch strategy. 3. The commits made on the branches get merged back. … Great for **parallelizing multiple AFK agents**, **creating review pipelines**, or even just orchestrating your own agents." — [README] "What Is Sandcastle?"

关键词：**AFK（away-from-keyboard）agent**、**review pipelines**。即官方自我定位是「无人值守编码 agent 的沙盒编排库」，不是框架、不带工作流主张：

> "Sandcastle uses a flexible prompt system. **You write the prompt, and the engine executes it — no opinions about workflow, task management, or context sources are imposed.**" — [README] "Prompts"

模板也只是演示编排形态而非方法论载体：

> "The built-in templates … are deliberately minimal, framework-agnostic starting points. They exist to **demonstrate the orchestration shapes Sandcastle supports, not to encode any particular external methodology**." — https://github.com/mattpocock/sandcastle/blob/main/.out-of-scope/bundled-workflow-templates.md

### 1.2 RALPH loop 血统与定位

Sandcastle 是 Matt Pocock 的 **RALPH 方法论的工业化产物**。RALPH = bash 循环反复喂同一个 prompt 文件给 Claude Code，每次迭代全新上下文，以 `<promise>COMPLETE</promise>` 作终止信号：

- 方法论文章：https://www.aihero.dev/getting-started-with-ralph 、https://www.aihero.dev/tips-for-ai-coding-with-ralph-wiggum
- AFK + 流式输出（sandcastle 的技术前身脚本）：https://www.aihero.dev/heres-how-to-stream-claude-code-with-afk-ralph —— 该文的 bash 脚本形态（docker 沙盒 + `--print` + 轮询 COMPLETE 信号）就是 sandcastle 的原型。
- 为什么每次迭代必须全新上下文（"smart zone" 理论，前 40% 上下文才聪明）：https://www.aihero.dev/why-the-anthropic-ralph-plugin-sucks
- 上游开发总流程观（grill → PRD → issues → 并行实现）：https://www.aihero.dev/my-7-phases-of-ai-development

模板 prompt 里 agent 直接自称 RALPH："You are RALPH — an autonomous coding agent working through issues one at a time."（https://github.com/mattpocock/sandcastle/blob/main/src/templates/simple-loop/prompt.md ）。**结论：simple-loop 就是 RALPH loop 的正装版**；issue 驱动是其默认工作源（见 §2、§6.2）。

### 1.3 官方推荐的三种编排形态

从 5 个模板 + 上游自己狗食的 `.sandcastle/run.ts` 可归纳官方认可的形态梯度：

| 形态 | 载体 | 定位 |
|---|---|---|
| 单发循环（RALPH） | `simple-loop` 模板 | issue 驱动，一次一个 issue，merge-to-head 回合 |
| 串行 implement+review | `sequential-reviewer` 模板 | 每 issue 一条独立分支，implementer 与 reviewer **共享同一沙盒同一分支** |
| 并行 plan→execute→merge | `parallel-planner` / `parallel-planner-with-review` 模板 + 上游自用 `.sandcastle/run.ts` | 三/四阶段外循环，opposite 模型分工（opus 规划、sonnet 执行） |

review 管线的官方推荐形态有两层：**(a) 模板级**——reviewer 与 implementer 在同一 `createSandbox()` 沙盒、同一分支上先后跑（`parallel-planner-with-review/main.mts`、`sequential-reviewer/main.mts`）；**(b) CI 级**——上游仓自己的 `.github/workflows/agent-{explore,implement,review,update-branch}.yml`，用 issue/PR **label 作为触发器**（`agent:explore` / `agent:implement` / `agent:review` / `agent:update-branch`），每个 label 事件触发一次 sandcastle 运行（https://github.com/mattpocock/sandcastle/tree/main/.github/workflows 、https://github.com/mattpocock/sandcastle/tree/main/.sandcastle/agent-workflows ）。

---

## 2. 模板各自适用场景与内部结构

`sandcastle init` 提供 **5 个模板**（比任务书假设的"三件套"多两个）——[README] "Templates"：

| 模板 | 一句话 | 内部结构 |
|---|---|---|
| `blank` | 裸脚手架，自带一切自己写 | 仅 `main.mts` + `prompt.md` + `template.json` |
| `simple-loop` | 逐个挑 open issue 并关掉 | `main.mts`：单次 `run()`，`maxIterations: 3`，`merge-to-head`，`copyToWorktree: ["node_modules"]`，hook `npm install`。`prompt.md`：RALPH 人格 + 优先级排序（bug > tracer bullet > polish > refactor）+ RGR（Red→Green→Repeat→Refactor）+ 一次迭代只做一个 issue + 完成/受阻即 `<promise>COMPLETE</promise>` |
| `sequential-reviewer` | 逐 issue 实现 + 每 issue 后过评审 | 外循环每轮 `createSandbox({branch: 时间戳分支})` → implementer（maxIterations 1）→ 有 commit 才 reviewer；共享沙盒共享分支；无 commit = backlog 尽，break |
| `parallel-planner` | 规划可并行的 issue，分支并行执行，最后合并 | 三阶段外循环（MAX_ITERATIONS 10）：① opus planner 出 `<plan>` JSON（`Output.object` + zod 校验，要求 maxIterations=1）② `Promise.allSettled` 每 issue 一个 sonnet implementer，各跑各的 `branch` 分支（maxIterations 100）③ 单个 sonnet merger 合并所有有 commit 的分支并关 issue |
| `parallel-planner-with-review` | 同上 + 每分支评审 | Phase 2 改为每 issue 一个 `createSandbox()`，implementer 跑完有 commit 则**同沙盒同分支**再跑 reviewer（1 迭代）；多个 issue 管线仍 `Promise.allSettled` 并发 |

来源：https://github.com/mattpocock/sandcastle/tree/main/src/templates （各模板 `main.mts` 与 `*-prompt.md` 已逐一核对全文）。

### Prompt 工程三件套（模板通用）

1. **动态上下文 `` !`command` ``**：prompt 文件里的 shell 表达式在**沙盒内**、hooks 完成之后并行执行，stdout 替换进 prompt。任一命令非零退出即整个 run fail-fast。用于注入 issue 列表（`gh issue list …`）、近期 commit 等新鲜状态——[README] "Dynamic context"。
2. **参数替换 `{{KEY}}`**：`promptArgs` 在 host 侧先于 shell 展开替换；未匹配的占位符是错误；**参数值内的 `` !`…` `` 按惰性文本处理不执行**（防注入设计——issue 标题等用户内容可安全走 promptArgs）。内置 `{{SOURCE_BRANCH}}` / `{{TARGET_BRANCH}}` 自动注入。注意：**inline `prompt:` 完全不做替换与展开**（字面透传，[README] "Prompt resolution"，ADR 0008）。
3. **完成信号 `<promise>COMPLETE</promise>`**：纯终止信号、无载荷；是 prompt 里写给 agent 的约定，引擎从不注入；可被 `completionSignal` 覆盖（支持数组，任一命中即停）——[README] "Early termination"。

### Hooks 与输出约束

- Hooks 按**执行位置**分两组：`host.onWorktreeReady`（顺序执行）/ `host.onSandboxReady` / `sandbox.onSandboxReady`（与 host 组并行）；sandbox hook 支持 `sudo: true`、`timeoutMs`；任一非零退出 fail-fast——[README] "Hooks"。
- 输出约束的正式机制是 **Structured Output**（见 §3.4），prompt 里明文要求 agent 把 JSON 包在指定 XML tag 里，`Output.object({tag, schema})` 解析校验；模板 planner 即此用法（`<plan>` tag）。

---

## 3. 编排能力全图

### 3.1 四个入口

| API | 语义 | 来源 |
|---|---|---|
| `run(options)` | 一次性：自动建 worktree+沙盒、跑 N 迭代、按 branch strategy 合回、清理 | [README] "API" |
| `interactive(options)` | 人机交互会话（TUI），默认 `noSandbox()`，可省略 prompt 直接进 TUI | [README] "Sandbox Providers" |
| `createSandbox(options)` | **长寿命热沙盒**：容器只起一次，`sandbox.run()` 可多次调用（implement→review→edit 复用同一容器与分支），`sandbox.exec(cmd)` 可在两次 run 之间跑验证命令（非零 exitCode 返回不抛）；`await using` 自动清理，脏 worktree 保留 | [README] "createSandbox()" |
| `createWorktree(options)` | worktree 作为一等公民：先交互后交给 AFK agent；`wt.run/interactive/createSandbox`；只接受 `branch`/`merge-to-head` 策略（`head` 是编译期类型错误）；split ownership——`wt.createSandbox()` 出的沙盒 close 只拆容器，worktree 归 `wt.close()` 管 | [README] "createWorktree()" |

### 3.2 Branch strategy 三型语义

[README] "How it works" / "Branch strategies"：

- **`head`**（bind-mount 默认）：agent 直写 host 工作目录，无 worktree 无分支间接层。仅供快速迭代；`copyToWorktree` 与之不兼容。
- **`merge-to-head`**（isolated 默认）：临时分支 + worktree，完成即合回 host 当前 HEAD，临时分支删除。自动化/CI 的安全默认。
- **`branch`**：落到显式命名分支；同名分支重跑**复用既有 worktree**，干净时先 `git fetch origin <branch>` + `git merge --ff-only` 防读到陈旧代码（ADR 0003，https://github.com/mattpocock/sandcastle/blob/main/docs/adr/0003-reuse-worktree-by-default.md ）。并发同名分支由 `.sandcastle/locks/<name>.lock` PID 文件锁防重（ADR 0007）。

### 3.3 Resume / fork / 会话迁移

[README] "Session capture / resume / fork"：

- 每次 resumable provider 迭代后，**会话 JSONL 自动从沙盒捕回 host**（Claude `~/.claude/projects/…`、Codex `~/.codex/sessions/…`、Pi `~/.pi/agent/sessions/…`），并把文件内 `cwd` 字段改写为 host 路径，使 provider 原生 resume 命令可用。`captureSessions: false` 可关。Claude Code 的 Agent/Workflow 子代理 transcript 也一并捕获（best-effort）。
- `resumeSession: "<id>"` 传入 `run()`/`sandbox.run()`，或 `result.resume?.(prompt)`；**每次 resume 恰好一个迭代**（ADR 0011：多迭代续接语义有二义性，故 API 形状上锁死单迭代，要多步就自己链式调用）。
- `result.fork?.(prompt)`：`claude --fork-session` / `codex exec fork`，父会话原封不动，子会话新 ID。**fork 只隔离会话 JSONL，不隔离分支/沙盒**——并发扇出必须给每个 fork 显式 `branchStrategy: { type: "branch", branch: … }`，`head`/`merge-to-head` 下并发 fork 不安全（ADR 0018，https://github.com/mattpocock/sandcastle/blob/main/docs/adr/0018-fork-is-session-only.md ）。
- 硬约束：resume 要求 provider 有**文件系统支撑的会话存储**；会话只在数据库里的 agent（OpenCode 的 SQLite）不支持 resume，`RunResult.resume` 类型为 `never`（ADR 0016）。cursor/opencode/copilot 均不可 resume。

### 3.4 Structured outputs

[README] "Structured output" + ADR 0010（https://github.com/mattpocock/sandcastle/blob/main/docs/adr/0010-structured-output.md ）：

- `Output.object({ tag, schema })`：从 agent stdout 抓指定 XML tag 内容，JSON 解析 + Standard Schema 校验（zod/valibot/arktype 皆可），结果落在 `result.output`（类型收窄）。`Output.string({tag})` 取纯文本。
- **要求 `maxIterations === 1`**，且解析后的 prompt 必须包含该 opening tag 字面量，否则入口处抛错。
- 失败抛 `StructuredOutputError`（携带 `tag/rawMatched/commits/branch/sessionId/sessionFilePath`）；`maxRetries` 可让 sandcastle 自动 resume 失败会话、反馈错误让 agent 重发 tag（需可 resume 的 provider）。
- 与 completion signal 是正交概念：信号只管终止不带载荷，output 带载荷不管终止。

### 3.5 日志与观测

[README] `logging` 选项 + [CHANGELOG] 0.10.0/0.12.0：

- 默认写文件到 `.sandcastle/logs/`（`{type:"file", path}`）或 `{type:"stdout"}`。
- `onAgentStreamEvent(event)` 回调把 agent 流（text/toolCall/raw，含 iteration、timestamp）转发给外部观测系统；回调抛错被吞掉不杀 run。
- `verbose: true` 把 agent 每行原始 stdout（含解析器会丢弃的行）并进日志，用于调试卡死 agent。
- 每次迭代 `IterationResult` 带 `sessionId`/`sessionFilePath`/token `usage`（input/cache create/cache read/output，0.6.2 起 Codex 也有）。
- 超时体系：`idleTimeoutSeconds`（默认 600，信号前）、`completionTimeoutSeconds`（默认 60，信号后挂起进程的宽限，ADR 0019）、hooks 60s/个可覆写、生命周期步骤超时 `timeouts.{copyToWorktreeMs,gitSetupMs,commitCollectionMs,mergeToHostMs}`（ADR 0001）。
- `signal: AbortSignal` 可取消 run/interactive，worktree 保留（ADR 0004）。

---

## 4. 多 agent 并发官方姿势

### 4.1 parallel-planner 怎么派

官方姿势（`src/templates/parallel-planner/main.mts` + 上游自用 `.sandcastle/run.ts`，https://github.com/mattpocock/sandcastle/blob/main/.sandcastle/run.ts ）：

1. **planner 建依赖图**：prompt 里给了 blocked-by 的三条判定（A 引入 B 需要的代码 / 改重叠文件易冲突 / B 依赖 A 定的 API 形状），只放行 unblocked issue；分支名强制确定性 `sandcastle/issue-{id}`（**0.6.1 修复**：原来带 slug 的格式每轮规划重新派生，导致同 issue 开重复分支丢进度——https://github.com/mattpocock/sandcastle/issues/618 ）。
2. **执行扇出**：`Promise.allSettled(issues.map(run))`，**每 issue 一条独立 branch 分支**（`branchStrategy: {type:"branch"}`），失败不拖垮同伴；上游自用版加了 MAX_PARALLEL=4 的信号量限流（模板版未限流，靠 planner 少放行）。
3. **只把有 commit 的分支交给 merger**（跑成功但没产出的分支不合并）。
4. **合并冲突处理**：没有算法层方案——**单起一个 merger agent**，prompt 要求逐分支 `git merge --no-edit`、读两边智能解冲突、解完跑 typecheck+test，全绿后一个汇总 commit 并按清单关 issue（`merge-prompt.md`）。上游自用版 merger 用 opus 且 `maxIterations: 10`。
5. 外循环最多 10 轮，让合并后新解锁的 issue 进入下一轮。

### 4.2 Issue 认领怎么防重

三个层面，各有缺口：

- **模板层**：靠 planner 的依赖图分析 + issue list 是唯一事实源（prompt 明令"不要自己再查未过滤列表"）+ 确定性分支名复用 worktree。**已知坑**：#918（已合并但 issue 未关的会被 planner 反复重选，open）、#598（GitHub 状态延迟 15-30s 导致 planner 选中刚关闭的 issue，closed——报告者自改用 `state == "OPEN" and closedAt == null` 的 jq 过滤缓解）、#841/#812（一个迭代内 agent 连关多个 issue / 累积式实现，open）。
- **库层**：同名分支并发访问由 `.sandcastle/locks/*.lock` PID 锁挡（ADR 0007）。
- **CI 层（上游自用）**：label 状态机防重——触发即 `remove-label agent:implement && add-label agent:in-progress`，再加 GitHub Actions `concurrency.group` 按 issue/PR 号串行化（https://github.com/mattpocock/sandcastle/blob/main/.github/workflows/agent-implement.yml ）。这是最完整的官方认领示范。

---

## 5. 凭据与安全模型

### 5.1 .env 注入面

- 配置目录 `.sandcastle/`（含 `Dockerfile`、`prompt.md`、`.env.example`、`.gitignore`——`.env` 与 `logs/` 被 ignore）——[README] "Configuration"。
- 环境变量自动从 `.sandcastle/.env` 与 `process.env` 解析；**agent provider 与 sandbox provider 各自可带 `env: Record<string,string>`**，合并规则：provider env 覆盖 `.env` 同名键；agent env 与 sandbox env **键不得重叠**（重叠即抛错）——[README] "Provider env"。
- 官方 `.env.example` 就两行：`ANTHROPIC_API_KEY=` / `GH_TOKEN=`（上游自用仓 https://github.com/mattpocock/sandcastle/blob/main/.sandcastle/.env.example ）；0.9.0 起脚手架默认改用 `CLAUDE_CODE_OAUTH_TOKEN`（`claude setup-token` 获取），`ANTHROPIC_API_KEY` 作为注释掉的备选（[CHANGELOG] 0.9.0）。

### 5.2 GH_TOKEN 权限

官方 envExample 写的是细粒度 PAT：**Issues (Read and write) + Metadata (Read)**（https://github.com/mattpocock/sandcastle/blob/main/src/InitService.ts ）。**已知低估**：要让 agent 推分支、开 PR，还需 **Contents (Read and write) + Pull requests (Read and write)**——open issue #937（https://github.com/mattpocock/sandcastle/issues/937 ），"失败发生在 run 的最后一步且报错不指向 token"。本仓若要 agent 开 PR 必须按四项配。

### 5.3 沙盒网络面与权限模式

- **网络不做默认隔离**：docker() 不指定 `network` 时用 Docker 默认 bridge（即可自由出网，装包、调 API、`gh` 都依赖此）；可经 `network: "my-network"` 收窄（https://github.com/mattpocock/sandcastle/blob/main/src/sandboxes/docker.ts ）。无内置 egress 白名单——**网络面收窄官方未提供开箱机制（未证实有路线图）**。
- **内核共享**：docker() 是普通容器、共享 host 内核；与 Docker Sandbox 的 microVM 相比隔离弱一档，open issue #682 明确记录了这个 trade-off（https://github.com/mattpocock/sandcastle/issues/682 ）。要更强隔离官方路径是换 isolated provider（Vercel Firecracker microVM）。
- **agent 权限**：AFK 默认 `--dangerously-skip-permissions`（沙盒内）。0.8.0 起提供 AI 中介审批替代全 bypass：`claudeCode({ permissionMode: "auto" })` / `codex({ approvalsReviewer: "auto_review" })`，官方明言定位是 `noSandbox()` + host 直跑场景（[CHANGELOG] 0.8.0）。
- **prompt 注入面**：promptArgs 值内的 `` !`…` `` 不执行（§2），但 **issue 评论注入是 open 安全洞**——#870：攻击者等 issue 被打上 agent 触发 label 后塞恶意评论，agent 读取全部评论时被注入（https://github.com/mattpocock/sandcastle/issues/870 ）。本仓流程若读取公开 issue 评论需注意。
- **provider 错误不重试**：rate limit/auth/quota 错误一律 fail-fast，重试属调用方责任（https://github.com/mattpocock/sandcastle/blob/main/.out-of-scope/provider-error-retry.md ）。

---

## 6. 与本仓适配点

### 6.1 自定义 AgentProvider 的官方预期用法

官方接口定义于 `src/AgentProvider.ts`，贡献者文档 https://github.com/mattpocock/sandcastle/blob/main/docs/agents/adding-an-agent-provider.md 。**比照内置 provider 的形态**，一个 provider = 工厂函数返回的 `AgentProvider` 对象：

```ts
interface AgentProvider {
  name: string;
  env: Record<string, string>;            // 注入沙盒，auth key 放这里
  captureSessions: boolean;               // 用户级开关，默认 true
  sessionStorage?: AgentSessionStorage;   // 可 resume 的 provider 必填
  buildPrintCommand(options): PrintCommand;      // 非交互运行命令；优先 stdin 喂 prompt
  buildInteractiveArgs?(options): string[];      // 可选，无 TUI 就省略
  parseStreamLine(line): ParsedStreamEvent[];    // 消费行分隔 JSON 流
  parseSessionUsage?(content): IterationUsage | undefined;
}
```

接入的硬门槛（官方"must-have"，达不到"该 agent 大概率无法被支持"）：

1. **非交互运行模式**（`--print`/`exec` 类 flag，跑完即退出）；
2. **prompt 走 stdin 优先**（argv 上限 ~128KB）；所有插值必须 `shellEscape`；
3. **自动批准/bypass flag**（沙盒内任何确认提示都会挂死）；
4. **env 变量认证**（不能要交互登录）；
5. **行分隔 JSON 流输出**（`parseStreamLine` 消费；事件类型 `text/result/tool_call/session_id/usage`；**发出 `session_id` 是硬性要求**，否则无法捕获会话）；
6. **resume 端到端**（ADR 0012 定为新 provider 硬要求）：resume-by-ID flag + **session-ID 往返稳定**（要求实证验证）+ **文件系统支撑的会话存储**（ADR 0016：SQLite 类库存储不做，`captureSessions:false` 且无 `sessionStorage`）；`sessionStorage` 六个方法：`captureToHost / resumeIntoSandbox / readHostSession / existsOnHost / hostSessionFilePath / findByIdOnHost`，cwd 改写可用官方纯函数 `transferClaudeSession/transferCodexSession`（`src/SessionStore.ts` 已导出）。

对 prime-agent provider 的直接含义：**若 prime-agent 的会话不在文件系统上按 session-id 可寻址，则官方预期形态是不带 `sessionStorage` 的不可 resume provider**（同 cursor/opencode/copilot 一档），代价是不能用 `resumeSession`/`.resume()`/`.fork()`/`Output maxRetries`。另注意：**内置 provider 名单不接受扩张请求**（https://github.com/mattpocock/sandcastle/blob/main/.out-of-scope/built-in-agent-providers.md ），自定义 provider 走 `run({ agent: … })` 注入即是官方为第三方预留的路径。

### 6.2 github-issues issue-tracker 模板假设的流程

关键认知：**issue tracker 集成只是脚手架模板，不是运行时组件**。官方原话：

> "Sandcastle does not embed any issue tracker itself. An issue-tracker entry is a **scaffold template**: … we substitute three CLI commands (`LIST_TASKS_COMMAND`, `VIEW_TASK_COMMAND`, `CLOSE_TASK_COMMAND`) into the generated prompt files … **The generated project then runs those commands itself — Sandcastle is not in the loop at runtime.**" — https://github.com/mattpocock/sandcastle/blob/main/docs/agents/adding-an-issue-tracker.md

github-issues 内建三条命令（`src/InitService.ts`）：
- LIST：`gh issue list --state open --label Sandcastle --limit 100 --json …`（**按 label 过滤出"agent 可接"的队列**；上游自用仓改用 `ready-for-agent` 标签）
- VIEW：`gh issue view <ID>`
- CLOSE：`gh issue close <ID> --comment "Completed by Sandcastle"`

假设的流程：**人（或 triage 流程）给 issue 打标签 → agent 循环消费已过滤队列 → 完成后关 issue**。追踪器 CLI 必须能非交互跑在 Debian 容器里；**MCP server 不能替代 CLI**（官方明示"not sufficient on its own"）。本仓 triage label 体系（`ready-for-agent` 等）与该假设同构，LIST 命令换 label 即可对齐。

### 6.3 官方已知坑/限制（与本仓适配直接相关者加 ★）

来自 issues（open 优先）与 CHANGELOG 修复史，全部附链接：

**并发/分支**
- ★ #849（open）：branch 策略并发 worktree 互删——容器内 git repair 改写共享 admin gitdir 回指路径，host 侧 `git worktree prune` 误杀在跑的兄弟 worktree。https://github.com/mattpocock/sandcastle/issues/849
- ★ #919/#917（open）：`noSandbox` 并发时 `git config --global` 抢 `~/.gitconfig.lock`。
- #846（open）：`safe.directory` 条目只加不删，全局 git config 膨胀。
- #931（open）：被保留的脏 worktree 永不回收，无关闭选项。
- ★ #918（open）：planner 反复重选已合并未关闭的 issue（§4.2）。
- #642（open）：worktree 冲突检查是 repo 全局而非 repoDir 级，并行调用者互踩。

**createSandbox 路径（本仓工作流核心，全部 open 于 0.12.0）**
- ★ #943：`createSandbox` 忽略 sandbox onSandboxReady hooks 的非零退出——setup 失败与成功不可分辨。https://github.com/mattpocock/sandcastle/issues/943
- ★ #907：`sandbox.onSandboxReady[].timeoutMs` 被 `createSandbox()` / `worktree.createSandbox()` 忽略。
- ★ #925/#900：`createSandbox` 丢 agent provider env（per-run sandbox.run 只继承容器 env）——**自定义 provider 若靠 `env` 传 auth key，此坑直接命中，需实测确认**。https://github.com/mattpocock/sandcastle/issues/925
- #916：create() 超时前未返回 handle 时 Docker 容器泄漏。

**会话/输出**
- #942（open）：0.12.0 的 `run().resume()/fork()` 把 promptArgs 泄漏进 inline-prompt 重跑，报 `PromptError`。
- #928（open）：claudeCode() 永远 stdin 喂 prompt，`/slash-command` prompt 永不展开。
- #898（open）：Codex "Context window" 报的是会话累计 token 而非窗口大小。
- #879（open）：想要迭代后 agent 摘要捕获——官方还没有。
- ★ #866（open）：**沙盒内拿不到全局 `~/.claude/CLAUDE.md` / AGENTS.md / hooks / 全局 MCP 配置**——对本仓尤其相关：本仓 `AGENTS.md` 纪律、缠论权威链等 agent 指引若在沙盒内不可见，行为会与 host 侧分叉。缓解只有 Dockerfile/镜像内置或 prompt 注入。https://github.com/mattpocock/sandcastle/issues/866

**模板层**
- #873（open）：init 模板默认把 host `node_modules` 拷进容器 worktree；#872（open）：hooks 硬编码 `npm install` 无视包管理器探测；#786（open）：verify 循环硬编码 `npm run typecheck/test`。
- #745（open）：macOS 上 `pnpm install` hook 会把 Linux-only native binaries 写进 host `node_modules`。
- #841（open）：simple-loop 一次调用可连关多个 issue，绕过 maxIterations 语义。

**安全/隔离**
- #870（open）：issue 评论 prompt 注入（§5.3）。
- #682（open）：容器共享内核 vs microVM（§5.3）。
- ★ #766（open）：noSandbox 拆除时不收割 agent 进程树（进程组 kill 缺失）。

**平台（本仓 macOS+colima 基本免疫，仅列名）**：Windows 系列坑密集（#924 worktree 超时硬编码、#940 noSandbox POSIX 引号被 cmd.exe 字面化、#904 reflink 假 CoW、#859/#855 等）；Docker 29.x 下 `resolveGitMounts` 挂死（#868 open）。

**官方明示不做（.out-of-scope 裁定）**：多 repo 单沙盒（https://github.com/mattpocock/sandcastle/blob/main/.out-of-scope/multi-repo-sandbox.md ）、`sandcastle` 命名前缀可配（#552 → configurable-namespace-prefix.md）、provider 错误重试、内置 provider 名单扩张、Dockerfile 组合抽象层（用户自持 Dockerfile 即是答案）。

---

## 7. 对「工作流全面适配 sandcastle」裁定的要点提炼

1. **sandcastle 是库不是框架**：工作流形态（RALPH 循环 / implement-review 管线 / plan-execute-merge 并行）全部落在用户自持的 `main.mts` + prompt 文件里，官方模板只是"演示编排形态"的最小样例。全面适配 = 把本仓 wayfinder/蜂群纪律写进自己的编排脚本与 prompt，而非等待官方功能。
2. **官方最成熟形态 = issue 驱动 + label 队列 + 确定性分支 `sandcastle/issue-{id}` + Promise.allSettled 扇出 + 单 merger agent 收口**，合并冲突交给 merger agent 而非算法；issue 防重靠"planner 依赖图 + 库层 worktree 锁 + CI 层 label 状态机"三层，每层都有已知缺口（#918/#598/#841）。
3. **自定义 AgentProvider 有明确的官方接口与硬门槛**，其中"文件系统会话存储"决定 resume/fork/structured-output-retry 一整组能力的可用性——这是 prime-agent provider 设计的第一决策点。
4. **安全面要本仓自己兜**：网络默认全通、内核共享、issue 评论注入未修、全局 AGENTS.md 不进沙盒——四条都需在适配 destination 里显式裁决。
5. **0.12.0 在 createSandbox 路径上有一簇 open bug（#943/#907/#925/#900）**，而 createSandbox 恰是 implement→review 共享热沙盒的推荐姿势；适配前建议对这四个坑逐一做实测验证。

## 附：本次未能证实项

- Matt Pocock 在 YouTube/X 上对 sandcastle 的发布视频与定位原话（websearch 未配置；#505 评论侧面证实存在官方演示视频："I watched your video"）。
- aihero.dev 无 sandcastle 专文（已对全站 sitemap 188 个 URL 逐一核对，0 命中）；官方文档站内容即仓内 `docs/content/docs/*.mdx`（Getting Started / Agents / Configuration 三页，信息量为 README 子集）。
- GitHub Discussions 未启用（404），无 discussions 信源可言。
