# pstack 上游仓库调查

> 关联票：[#1113](https://github.com/xy7365527-lang/NewChanlun/issues/1113)  
> 调查时间：2026-08-19（UTC）  
> Marketplace 条目：[pstack | Cursor Plugins](https://cursor.com/marketplace/cursor/pstack)  
> 验证过的精确上游目录：[github.com/cursor/plugins/tree/main/pstack](https://github.com/cursor/plugins/tree/main/pstack)  
> 本次固定快照：[`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack)

## 0. 口径

本文用三种标签区分证据强度：

- **事实**：Marketplace、Cursor 官方文档、仓库 manifest/README/源码、Git 历史或本次可复现命令直接支持。
- **推断/评估**：由事实推出来的适用人群、成熟度或风险判断，不冒充上游承诺。
- **未验证报告**：第三方 PR/issue 中的说法，尚未被上游合并或确认。

## 1. 结论先行

### 1.1 它是什么

- **事实**：pstack 不是一个应用、编译器或 SDK，而是一个 **Cursor Plugin 形式的工程工作流/提示词包**。当前 manifest 版本是 **0.14.1**，作者字段是 **Lauren Tan**，MIT 许可；manifest 只注册了 `skills/` 和 `agents/` 两类组件。[plugin.json](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L1-L29)
- **事实**：Marketplace 页面把它描述为“少写、写更高质量的代码，并能有信心地并行化严谨的 agent 工作流”，页面显示 **44 个 Skills、2 个 Subagents**，发布者为 Cursor 且 “Verified by Cursor”。[Marketplace](https://cursor.com/marketplace/cursor/pstack)
- **事实**：Cursor 官方多插件清单把 `pstack` 映射到同仓 `pstack/` 子目录；Marketplace 当前页面也固定到仓库 commit `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，所以精确上游不是一个另名独立仓库，而是官方 monorepo 的该目录。[marketplace.json](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/.cursor-plugin/marketplace.json#L67-L74)
- **事实**：Marketplace UI 的“Created by Cursor”与包内 manifest 的“Lauren Tan”同时存在。前者是 Marketplace 发布者身份，后者是插件作者字段。[Marketplace](https://cursor.com/marketplace/cursor/pstack) · [manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L5-L10)
- **推断/评估**：最准确的定位是：**由 Lauren Tan/poteto 整理、经 Cursor 官方仓库与 Marketplace 发布审核的、强意见化的 Cursor Agent 工程方法栈**。不应把它误解为提供新的基础模型或独立执行沙箱。

### 1.2 一句话判断

- **推断/评估**：它适合已经在 Cursor 里做复杂代码工作、愿意用多模型/多子代理换取设计与验证深度的资深工程师或团队；不适合把“低 token、低延迟、每次外部写操作都要人工批准、行为必须确定可复现”放在首位的场景。
- **推断/评估**：成熟度应归为 **“早期、活跃、工程内容不少，但还不是稳定平台”**。理由是 0.x 版本、不到三个月的快速迭代、没有仓库 release/tag、维护面明显集中；正向信号则是 Cursor Marketplace 已审核、文档完整、核心脚本有单测且本次实跑通过。

## 2. 上游身份核验

| 项目 | 结论 | 一手证据 |
|---|---|---|
| GitHub 仓库 | `cursor/plugins` | [仓库](https://github.com/cursor/plugins) |
| 精确目录 | `pstack/` | [固定快照](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack) |
| Marketplace 解析路径 | `source: "pstack"` | [根 marketplace manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/.cursor-plugin/marketplace.json#L67-L74) |
| 插件名/版本 | `pstack` / `0.14.1` | [插件 manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L1-L5) |
| 作者字段 | Lauren Tan | [插件 manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L5-L10) |
| Marketplace 发布/验证 | Cursor / Verified by Cursor | [Marketplace](https://cursor.com/marketplace/cursor/pstack) |
| 许可 | MIT | [LICENSE](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/LICENSE) |

## 3. 它解决什么问题

### 3.1 上游明确陈述

- **事实**：作者认为 AI 容易追求代码吞吐量而产生“slop code”；pstack 的目标相反，是让 Agent 少写但写得更好，并以可验证的单 agent 深度支撑可信并行。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L3-L13)
- **事实**：推荐入口是 `/poteto-mode`。用户给目标和完成判据，mode 选择 playbook、把步骤复制进 todo，再按步骤调用其他技能。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L32-L38) · [路由机制](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L82-L91)
- **事实**：工作重心是“理解、设计、实现、验证、评审、PR/堆栈交付、长程自治”，而不是提供某一门语言的代码生成器。其公开指南也按这条路径组织。[Guide](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/README.md)

### 3.2 本文归纳

- **推断/评估**：pstack 实质上是在 Cursor Agent 上方增加一层 **可组合的 SOP + 工程原则 + 模型路由 + 多 agent 编排约定**。
- **推断/评估**：它的“产品价值”主要来自流程约束和审查深度，而不是运行时能力。绝大多数内容是 Markdown `SKILL.md`；真正可执行代码集中在少量 PR 监控、编排台账、worktree 审计和决策日志脚本中。[源码树](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills)

## 4. 工作机制

### 4.1 Cursor 如何加载它

- **事实**：Cursor Plugin 用 `.cursor-plugin/plugin.json` 声明组件；pstack 显式声明 `"skills": "./skills/"` 与 `"agents": "./agents/"`。它没有在 manifest 中声明 rules、commands、hooks 或 MCP server。[pstack manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L21-L29) · [Cursor Plugins Reference](https://cursor.com/docs/reference/plugins#cursor-plugin-manifest)
- **事实**：Cursor 启动时发现 skills，将名称和 description 暴露给 Agent；Agent 可按上下文自动选择，也可由用户输入 `/skill-name` 手动调用。`disable-model-invocation: true` 会关闭模型自动调用。[Cursor Skills 文档](https://cursor.com/docs/skills#how-skills-work) · [关闭自动调用](https://cursor.com/docs/skills#disabling-automatic-invocation)
- **事实**：`poteto-mode` 自身明确设置 `disable-model-invocation: true` 和 `mode: true`，因此主入口是显式 `/poteto-mode`；上游宣称进入后会在本会话跨 turn 保持，直到用户退出。[poteto-mode frontmatter](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md#L1-L8) · [README sticky-mode 说明](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L82-L93)
- **推断/评估**：安装本身不会常驻启动一个守护进程，也不会凭空授予新的 MCP 权限。实际行为仍取决于 Cursor Agent 是否调用技能以及现有工具权限。

### 4.2 `/poteto-mode` 怎样路由任务

- **事实**：每个多步骤任务先创建 todo 并读原则索引；随后依据任务类型匹配 playbook，逐字复制 playbook 步骤，按触发条件路由到 `how`、`architect`、`arena`、`swarm`、`interrogate`、`tdd`、`unslop` 等技能；跳过步骤必须显式写原因。[poteto-mode](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md#L13-L35) · [playbook 执行规则](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md#L112-L130)
- **事实**：README 列出 22 个面向用户的 playbook，覆盖调查、bug、性能、hillclimb、运行时/trace 取证、功能、重构、原型、视觉对齐、技能编写与 eval、PR babysit/shipping、autonomous/orchestrate/autopilot、会话接力/安全暂停、多阶段计划与 worktree 清理。[playbook 总表](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L50-L76)
- **事实**：`poteto-mode` 内嵌 21 条工程原则索引，包括最小变更、领域建模、边界与类型纪律、先复现根因、真实产物验证、可验证单元、上下文隔离、可逆工作不等人等。[原则索引](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L193-L224)
- **推断/评估**：这是“软约束”而非编译期/运行时硬门。除少量脚本外，是否严格复现、是否真的做独立验证，依赖模型遵循长指令和主控复核。

### 4.3 模型路由与子代理

- **事实**：首次 `/setup-pstack` 会探测当前可用的 Task 模型 slug，要求真实 slug 必须已确认可用，并把角色到模型的映射写到用户级 `~/.cursor/rules/pstack-models.mdc`，且 `alwaysApply: true`；配置在新会话生效。[setup-pstack](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L8-L30) · [写入规则](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L30-L61)
- **事实**：当前默认把不同角色分给 Grok 4.6、GPT-5.6 Sol、Claude Fable 5、Claude Opus 5；`how` critics、arena runners、architect runners、interrogate reviewers 默认各有四项面板。`auto`/`inherit-parent` 表示省略 model 字段、继承父会话模型。[默认映射](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L32-L56)
- **事实**：pstack 默认把子任务放到 background Task；通用 playbook 子任务走 `poteto-agent`，多模型工作流则使用各技能自己的 subagent 规格。主控必须审 diff，不能直接转述子代理结论。[poteto-mode 子代理规则](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md#L87-L93)
- **事实**：Cursor 官方 subagent 机制本身提供独立上下文、并行执行、模型选择和可选 worktree/云环境隔离。[Cursor Subagents](https://cursor.com/docs/subagents)
- **推断/评估**：pstack 的质量主张很大一部分依赖多模型差异和交叉复核；模型面板越大，token、云 agent 数、延迟与费用越高。好处是 `/setup-pstack` 允许缩短面板或全部继承父模型。

### 4.4 并行与验证

- **事实**：`/arena` 让多个候选对同一 brief 独立产出，再由异模型 judge 打分，主控选择一个 base 并把落选方案的优点重构式 graft 进去；候选写入独立 worktree 或临时目录。[arena](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/arena/SKILL.md)
- **事实**：`/swarm` 用于独立切片、覆盖矩阵或 race；worker 默认在 cloud background 运行，parent 汇总为一个结果，不回灌原始 dump。[swarm](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/swarm/SKILL.md)
- **事实**：`/interrogate` 将相同 diff、意图和 rubric 发给不同模型；两模型以上独立提出的同一问题被视为高信号，lead 仍需判断 act on/consider/noted/dismissed。[interrogate](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/interrogate/SKILL.md)
- **事实**：验证原则要求运行真实 CLI/UI/存储读取或性能基准，而不把“编译通过”当完成；插件还可生成项目内 `verify-<app>` skill 与 feature map。[验证指南](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/06-verify-and-ship.md) · [create-verification-skill](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/create-verification-skill/SKILL.md)

### 4.5 可执行辅助代码

- **事实**：主要代码工具位于 `poteto-mode/scripts/`。`orch` 管理多日编排的 plain-file units/ledger/inbox/gates/frontier；`watch-pr` 经 `git`/`gh` 读取 PR、review threads、CI 和 stack 状态；另有只读 worktree 审计脚本与 TSV 决策日志脚本。[scripts 目录](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts)
- **事实**：这些 TypeScript 工具使用 Bun；`package.json` 提供 `bun test orch watch-pr` 与严格 typecheck，运行时依赖 `commander@14.0.0`。[package.json](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts/package.json#L1-L15)
- **事实**：首次运行工具时，bootstrap 会执行 `bun install --frozen-lockfile`，验证 `commander` 已落盘，然后重启原命令。[bootstrap.ts](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts/bootstrap.ts#L17-L61)
- **推断/评估**：因此“插件主要是 Markdown、没有二进制”不等于“绝不执行代码/联网安装依赖”。会用到这些 playbook 的组织应审查脚本和 lockfile，并在受控环境预装 Bun 依赖。

## 5. 主要能力图谱

| 能力组 | 代表能力 | 说明/来源 |
|---|---|---|
| 理解与取证 | `/how`, `/why`, `/teach`, `/recall`, `/blast-radius` | 代码运行流、历史动机、跨 MCP 证据、个人会话恢复、改动影响验证。[skills 表](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L107-L134) |
| 设计探索 | `/architect`, `/arena`, `/swarm` | 先定调用方用法/类型/边界；同题多方案 bakeoff；独立切片并行覆盖。[设计指南](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/04-design.md) |
| 实施与调试 | bug/feature/refactor/perf/hillclimb/prototype/visual-parity playbooks, `/tdd` | 复现优先、数据形状优先、行为钉死、先 profile 后优化、指标循环。[构建指南](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/05-build-and-clean.md) |
| 评审与清理 | `/interrogate`, `/no-comments`, `/unslop`, TypeScript practices | 多模型对抗评审、独立评论审查、去 AI 腔、类型纪律。[skills 表](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L117-L134) |
| 验证与交付 | verification skill、Babysit、Shipping、Autopilot | 真实产物证据、PR blocker 处理、逐 PR 独立验证、连续安全区间落栈。[交付指南](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/06-verify-and-ship.md) |
| 长程自治 | `/show-me-your-work`, autonomous run, orchestrate, autopilot-full/stack | 决策 TSV、循环完成判据、多日 coordinator、owner/verifier 分离。[overnight 指南](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/07-overnight.md) |
| 个性化与学习 | `/setup-pstack`, `/automate-me`, `/reflect` | 模型路由、从近期 transcript 归纳个人 mode、把一次性经验固化成技能。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L240-L246) |
| 可选自动化 | Benny | Slack issue 分诊，随后复现/修复确认过的 bug；默认 dormant，不注册为 slash skill。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L248-L252) |

## 6. 安装与典型使用

### 6.1 Marketplace 安装

**方式 A：在 Cursor chat 中执行 README/Marketplace 给出的命令。**

```text
/add-plugin pstack
```

来源：[README 安装段](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L15-L28) · [Marketplace](https://cursor.com/marketplace/cursor/pstack)

**方式 B：图形界面。**

1. 打开 Cursor 侧栏的 **Customize**。
2. 搜索 pstack。
3. 选择 **Install**，再选择 project scope 或 user scope。

来源：[Cursor 官方 Installing plugins](https://cursor.com/docs/plugins#installing-plugins)

### 6.2 首次配置

```text
/setup-pstack
```

- 它枚举当前账户可用于 Task subagent 的模型。
- 逐角色确认模型；不想多模型时可用 `auto` 或 `inherit-parent`。
- 写入 `~/.cursor/rules/pstack-models.mdc`。
- 若项目没有真实 app 验证入口，会询问是否生成 `/create-verification-skill`。
- 上游指南要求设置后开新会话。

来源：[setup guide](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/01-setup.md) · [setup skill](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md)

### 6.3 日常入口

推荐只给目标、约束和完成判据，让路由器决定过程：

```text
/poteto-mode add a --json flag to this command. text output stays byte-identical. verify both.
```

bug 示例：

```text
/poteto-mode users get two notifications after a retry. repro first, then fix and verify.
```

只需要单能力时可直接调用：

```text
/how do we dedupe notifications? is there an n+1?
/interrogate the whole branch. don't change anything yet.
/swarm check every package against its check.sh. one worker per package. one report.
```

来源：[Guide 首页](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/README.md) · [recipes](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/10-recipes-and-pitfalls.md)

### 6.4 本地审计/开发加载

- **事实**：Cursor 官方允许把插件复制或 symlink 到 `~/.cursor/plugins/local/<name>`，随后重启或 `Developer: Reload Window`；这适合先固定并审计某个 commit 再试用。[Test plugins locally](https://cursor.com/docs/plugins#test-plugins-locally)
- **推断/评估**：高合规环境建议先采用 project scope 或本地固定 SHA，不要一开始就启用 Shipping、Autopilot 或 Benny。

## 7. 目标用户

### 7.1 明确事实

- **事实**：作者把它描述为自己在 Cursor 日常使用的高质量代码工程技能，并强调多模型和可信并行。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L3-L13)
- **事实**：上游指南假设用户会处理真实 repo、tests、GitHub PR、worktree、长任务，部分交付路径还使用 Graphite。[Guide](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/README.md) · [Shipping](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide/06-verify-and-ship.md#land-the-stack-with-shipping)
- **事实**：作者明说 `poteto-mode` 是其个人风格，用户未必完全想要，并提供 `/automate-me` 生成自己的 mode。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L240-L246)

### 7.2 推断出的适配画像

**较适合：**

- 使用 Cursor 做中大型代码库、跨文件/跨模块变更的资深开发者。
- 需要把调查、架构、实现、独立验证和 PR 交付串成一条可审计链的团队。
- 已有 GitHub/CI、可运行测试或 UI/CLI 驱动器，并愿意维护验证技能。
- 能接受多 subagent 和多模型带来的费用、延迟与环境复杂度。

**不太适合：**

- 只做一次性小改动，或主要目标是最低 token/最快响应。
- 企业策略要求所有 ticket、团队消息、PR、commit 都逐次人工批准，而又不准备改 pstack 的 Autonomy 规则。
- 只允许单模型、没有 cloud subagents，或无法提供 Bun/`gh`/Graphite 等特定 playbook 所需工具。
- 希望 workflow 是硬状态机、可形式化复现，而不是依靠 LLM 解释 Markdown 指令。

## 8. 成熟度、维护状态与采用度

### 8.1 版本与时间线

- **事实**：当前 manifest 为 **0.14.1**。[manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L1-L5)
- **事实**：`pstack/` 首次进入仓库的 commit 是作者时区 2026-05-22（UTC 为 05-23）的 [`24bd6eb`](https://github.com/cursor/plugins/commit/24bd6eb895d63a6e76fabfea2cb344384d177d79)；最近一次触及 pstack 的 commit 是 2026-08-13 的 [`63d938c`](https://github.com/cursor/plugins/commit/63d938c2e4a165a0fec1bd0f61a8e325f0cb751e)，把 Grok 默认从 4.5 更新到 4.6 并升到 0.14.1。
- **事实**：截至本次快照，`git log -- pstack` 计 **71 个 commit**。commit author 字段中 Lauren 两个邮箱身份合计 68，Cursor Agent 3。[路径提交历史](https://github.com/cursor/plugins/commits/main/pstack/)
- **推断/评估**：更新频率高，说明活跃；作者高度集中，说明 bus factor 偏低。commit author 计数不能排除 PR co-author 或 review 贡献。

### 8.2 发布与 issue/PR 状态

- **事实**：整个 `cursor/plugins` monorepo 当前没有 GitHub Releases，也没有 tags；pstack 以 manifest 版本和合并 commit 演进，而不是独立 release artifact。[Releases](https://github.com/cursor/plugins/releases) · [Tags](https://github.com/cursor/plugins/tags)
- **事实**：当前有一个题名明确针对 pstack 的开放 PR：[#159 `map default model slugs from xhigh to high`](https://github.com/cursor/plugins/pull/159)，创建于 2026-07-20，无 review/comment；主线后来仍继续更新 pstack。
- **未验证报告**：#159 声称部分 `-xhigh` 默认 slug 不在 Task allowlist、会导致 delegation 失败。该说法尚未被维护者接受，不能当成已确认 bug；但当前默认配置确实仍包含 `xhigh`/`max` 形式，因此新环境应依赖 `/setup-pstack` 的“探测并校验可用 slug”步骤，而不是盲信默认值。[当前默认映射](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L39-L56)
- **事实**：以 `is:issue pstack` 搜索当前 tracker 未发现独立 pstack issue；pstack 的公开讨论主要在 PR 中。[Issues 搜索](https://github.com/cursor/plugins/issues?q=is%3Aissue+pstack)

### 8.3 测试与 CI

- **事实**：仓库内有 4 个 Bun test 文件，覆盖 `orch` 与 `watch-pr`；package script 同时提供 strict TypeScript typecheck。[tests 目录](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts) · [package.json](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts/package.json#L5-L14)
- **本次验证**：在固定 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4` 的临时 clone 中，以 Bun 1.3.13 执行：

  ```text
  bun install --frozen-lockfile
  bun test orch watch-pr       # 52 pass, 0 fail, 206 expect() calls
  bun run typecheck            # exit 0
  ```

- **事实**：仓库现有 GitHub workflow 只在 marketplace manifest、plugin.json 或 schemas 变动时运行插件定义校验，没有运行 pstack 的 Bun tests/typecheck。[validate-plugins.yml](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/.github/workflows/validate-plugins.yml#L1-L24)
- **推断/评估**：有单测是正向信号，但自动 CI 对脚本和 44 个 prompt skill 的覆盖不足；本次通过只能证明当前脚本单测/typecheck，不证明长 playbook 在各种模型上都遵循一致。

### 8.4 Marketplace 审核与采用度

- **事实**：Cursor 声明 Marketplace 插件上架前人工审核，每次更新也人工审核；插件必须开源。官方同时明确说插件仍是第三方软件，安装风险由用户自行承担，建议读源码。[Marketplace security](https://cursor.com/help/security-and-privacy/marketplace-security)
- **事实**：Marketplace 页没有公开 pstack 的安装量/活跃用户数；GitHub stars/forks 属于整个 `cursor/plugins` monorepo，不能归因到 pstack；又没有 release 下载量。
- **推断/评估**：所以可以确认“官方 Marketplace 已批准且维护活跃”，但 **无法从一手公开数据证明采用规模或生产成功率**。不要用 monorepo stars 代替 pstack adoption。

## 9. 采用风险清单

| 风险 | 一手事实 | 评估 | 建议 |
|---|---|---|---|
| 权限与自治边界 | `poteto-mode` 规定可逆工作和团队消息、ticket 更新、启动 eval 可不询问；force-push、部署、删数据、客户消息才强制暂停。[Autonomy](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md#L77-L85) | **中到高**。它可能与组织的“所有外部写操作先审批”政策冲突。 | 先审并覆写 autonomy；项目 scope 试点；禁用团队消息/ticket/merge 写权限。 |
| Prompt 不是硬门 | 主体是 Markdown playbook/原则，manifest 没有 hooks。[manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L21-L29) | **中**。不同模型、长上下文和压缩后可能产生遵循漂移。 | 对关键门仍用 CI、branch protection、测试和人审；不要把 prompt 当 policy engine。 |
| 多模型成本/延迟 | 多个默认 panel 各含四个模型，arena 还另有 judge。[setup](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L39-L56) | **中到高**，尤其大仓和 autonomous/orchestrate。 | 初始缩到 1–2 个 reviewer；用 `inherit-parent`；只在昂贵决策启用 arena/interrogate。 |
| 模型可用性漂移 | 模型 slug 硬编码在 skill 中；#159 提过兼容性报告；setup 可探测/校验。[setup](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md#L12-L26) | **中**。账户 entitlement 或 Cursor slug 变化会破坏 delegation。 | 安装后必须跑 setup；升级模型后重跑；失败时先查 Task 返回的有效 slug。 |
| 外部工具依赖 | 完整流程引用 `cursor-team-kit` 的 deslop/control skills；Shipping 用 Graphite；脚本用 Bun、`gh`、`git`。[not shipped here](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L226-L234) | **中**。核心技能可用，但某些 playbook 会降级或卡住。 | 按 playbook 建依赖矩阵；不使用的路径不装；先验证 `gh`/Bun/Graphite。 |
| 供应链与本地执行 | bootstrap 首次执行 `bun install --frozen-lockfile`；插件可调用现有 shell/MCP。[bootstrap](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/scripts/bootstrap.ts#L25-L61) | **中**。无二进制不等于无执行面。 | 固定 commit、审 lockfile、离线/代理环境预装依赖、最小化 token 与 shell 权限。 |
| 本地/企业信息读取 | `/recall`、`/automate-me`、`show-me-your-work` 读取 Cursor transcript；`/why` 可查询 issue、chat、observability、analytics MCP。[skills 表](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L113-L123) | **中**。能力符合设计，但会扩大敏感上下文进入模型的范围。 | 对 repo、transcript 和 MCP 做最小权限；不要给不需要的 Slack/analytics 工具。 |
| 强意见化风格 | 作者明确说 `poteto-mode` 是个人风格，且有激进的 comment 删除、少问人、先做可逆工作等原则。[README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L240-L246) | **中**。可能与团队注释、兼容性、审批文化冲突。 | 先评审 21 principles；用 `/automate-me` 或 fork 改成团队版。 |
| 维护集中/0.x | 0.14.1、作者 commit 高度集中、无 release/tag。[manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L1-L5) | **中**。API/流程/模型默认仍可能快速变化。 | 固定 SHA，升级看 diff；把关键本地覆盖写成独立规则，不追 `main` 静默漂移。 |
| 采用度未知 | 无公开安装量、独立 star、release download。 | **未知**，不能由“Verified”推成“广泛验证”。 | 用自己仓库的 2–3 个真实任务做 A/B 试点，记录质量、成本、延迟、误操作。 |

## 10. 安全与治理细节

- **事实**：Cursor 称 Marketplace 插件“主要是 Markdown 和 supporting files，不分发二进制”，并沿用 MCP allowlist/blocklist；每个插件版本不会从源仓自动更新进入 Marketplace，而是逐次人工审核。[Marketplace security](https://cursor.com/help/security-and-privacy/marketplace-security)
- **事实**：pstack 本身不声明 MCP server、hook 或 secret variable；它会使用用户当前已经给 Cursor 的 GitHub、MCP、shell 等能力。[manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json#L21-L29)
- **推断/评估**：其首要风险不是“安装即驻留恶意进程”，而是 **高权限 Agent 按强自治 prompt 使用既有凭证与工具**。安全评审重点应放在 playbook 的外部写操作、Shipping/Autopilot、transcript/MCP 读取范围和脚本依赖，而不只看 manifest。
- **事实**：Benny 是 dormant 文件，未注册为 slash skill；只有用户指向 `FOR_AGENTS.md` 并把它复制/配置到目标仓库才会进入自动化路径。[Benny 说明](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md#L248-L252)

## 11. 建议的试点验收

以下是本文基于风险的建议，不是上游要求：

1. **固定来源**：以本次 Marketplace 的 SHA `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4` 或内部镜像安装，不直接跟随 `main`。
2. **最小配置**：项目 scope；`/setup-pstack` 将关键角色先设为 `inherit-parent`，review panel 只留 1–2 项。
3. **选三个任务**：一个只读 `/how`，一个小 bug（有真实 repro），一个 `/interrogate` 评审；暂不开 Shipping/Autopilot/Benny。
4. **记录四项指标**：一次通过率、漏/误报、token/agent 数、墙钟时间；和普通 Cursor Agent 同题对照。
5. **治理验证**：确认 branch protection、CI、MCP allowlist、shell/ticket/team-chat 写权限不会被 prompt 绕开。
6. **再决定扩面**：只有真实验证收益覆盖成本后，再加 arena、swarm、overnight 和 PR 自动化。

## 12. 最终判断

- **事实结论**：验证过的上游是 [`https://github.com/cursor/plugins/tree/main/pstack`](https://github.com/cursor/plugins/tree/main/pstack)，当前 Marketplace 快照为 `fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`，版本 0.14.1。
- **事实结论**：它是 44 skills + 2 subagents 的 Cursor 工程工作流插件，核心入口 `/poteto-mode` 用 22 类 playbook、21 条原则和按角色模型路由组织 Agent；另有少量 Bun/TS/Shell 辅助代码。
- **推断/评估**：它的长处是把“理解再改、并行探索、真实验证、独立评审、长程交付”写成一个完整且可复用的方法栈。主要风险是 prompt 软约束、强自治、多模型成本、外部依赖、模型 slug 漂移和维护集中。
- **采用建议**：值得给熟练 Cursor 工程团队做固定 SHA、最小权限、缩小面板的受控试点；现有一手证据不足以支持“不经试点直接全团队启用 autonomous/shipping”的决定。

## 13. 主要一手来源索引

1. [Cursor Marketplace: pstack](https://cursor.com/marketplace/cursor/pstack)
2. [上游固定快照](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack)
3. [pstack README](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/README.md)
4. [pstack plugin manifest](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/.cursor-plugin/plugin.json)
5. [poteto-mode SKILL.md](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/poteto-mode/SKILL.md)
6. [setup-pstack SKILL.md](https://github.com/cursor/plugins/blob/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/skills/setup-pstack/SKILL.md)
7. [pstack public guide](https://github.com/cursor/plugins/tree/fd6dd6f7276956a532bb78a748a8d2818b6eb5f4/pstack/docs/guide)
8. [Cursor Plugins 官方文档](https://cursor.com/docs/plugins)
9. [Cursor Skills 官方文档](https://cursor.com/docs/skills)
10. [Cursor Subagents 官方文档](https://cursor.com/docs/subagents)
11. [Cursor Plugins Reference](https://cursor.com/docs/reference/plugins)
12. [Cursor Marketplace Security](https://cursor.com/help/security-and-privacy/marketplace-security)
13. [pstack 路径提交历史](https://github.com/cursor/plugins/commits/main/pstack/)
14. [GitHub Releases](https://github.com/cursor/plugins/releases) / [Tags](https://github.com/cursor/plugins/tags)
15. [开放 PR #159](https://github.com/cursor/plugins/pull/159)
