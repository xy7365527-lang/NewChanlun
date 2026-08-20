---
name: pstack-how
description: "Use for \"how does X work\" questions, code walkthroughs before changing something, and placement / ownership / layering questions (\"where should this live\", \"which package owns this\", \"is this the right layer\"). Explains subsystem architecture, runtime flow, call chains, and onboarding mental models with traceable code evidence, fanning out async RLM explorer subagents for complex subsystems. Can critique architecture when asked. The deliverable is a code-evidence-backed explanation of how an existing system's current runtime mechanism, call chain, data flow, and module ownership work. 中文触发（本 Skill 是首选）：凡问「谁调用了 X」「把调用链画出来」「运行机制是什么」「数据怎么流」「这段代码属于哪个模块 / 层次」「模块归属」等调用链、运行机制、模块归属、层次位置、数据流类的可审计架构解释任务，一律优先用本 Skill。交付物是对现有系统当前运行机制、运行流、调用链、数据流、模块归属、层次位置的代码证据化解释。"
license: MIT
metadata:
  upstream-repo: cursor/plugins
  upstream-sha: fd6dd6f7276956a532bb78a748a8d2818b6eb5f4
  upstream-path: pstack/skills/how/SKILL.md
  license-file: LICENSE.MIT
---

# pstack-how（如何运行）

探索代码库回答「X 是怎么运行的？」这类问题，产出能让一名资深工程师快速建立心智模型的架构解释。目标是建立可工作的心智模型，不是给源码做注释。

两种模式：

1. **Explain（解释，默认）**：探索代码库，产出清晰解释。
2. **Critique（批判）**：先解释，再并行起多个模型独立找架构问题。

## 触发边界（何时用 / 何时不用）

**用本 Skill**，当问题在问：

- 运行机制：「X 是怎么运行的」「从入口到输出串一遍」；
- 调用链：「谁调用了 F」「把调用链画出来」；
- 模块归属 / 层次位置：「这段代码属于哪个模块 / 层次」「在架构里处于什么位置」；
- 改代码前的走查：动手改之前先理解一个子系统。

**不用本 Skill**（交给现役能力）：

- 只要原始图查询结果、不要解释 → `codebase-memory`；
- 接口 / 模块怎么改的设计建议 → `codebase-design` 或 `design-an-interface`；
- 普通事实查询（「README 讲了什么」）→ 直接回答，无需本流程。

本 Skill 只做解释与架构批判，**不修改代码、不创建 map/ticket、不接管 Wayfinder**——Wayfinder 仍是唯一上层规划工作流。

## Prime 异步执行契约（重要，先读）

本 Skill 在 Prime 里没有 Cursor 的同步 `Task` 返回与硬只读。执行时遵守：

- **根代理持有扇出**：需要并行探索时，由根代理（你）自己 spawn 子代理并汇总，不要委托给另一个编排层。
- `await rlm('sub-task', name='<稳定名>')` **只返回 admission handle**（`rlm_child_id` / `name` / `session_dir` / `model`），**不等待、不返回子代理答案**。**永远不要把 admission handle 当成研究结果。**
- 子代理结果**只通过两种方式回流**：`await agent_message.send(message, receiver_role='parent')` 回复，或子代理写文件后你读文件。没有第三种「同步返回值」。
- 相互独立的子代理要**在各自单独的 `rlm()` 调用里 spawn，然后结束本回合**；回复会在后续回合以普通 agent message 到达。用 `await rlm.list_subagents()` 找回子代理句柄，用 `agent_observe` 查看其 rollout。
- 子代理继承你的模型；如需不同模型，用 `await rlm.find_models(...)` 取精确 selector 再传 `model=`。**没有 Cursor model slug**（如 `grok-*`、`claude-*` 之类），一律从 Prime 实际模型目录里选。
- Prime 没有硬只读约束：**行为上保持只读**——探索和解释不修改任何代码。

## Explain 模式

### 第 1 步：理解问题并评估复杂度

解析用户问的是什么：

- 「限流器是怎么工作的？」→ 子系统；
- 「按需用量的计费怎么处理？」→ 功能流；
- 「认证服务是怎么组织的？」→ 架构总览；
- 「用户提交表单后会发生什么？」→ 运行时追踪。

先圈定范围；若含糊，先说明你对其意图的最优理解再探索，**不要反复追问**，用户会纠正你。

**按复杂度选路径：**

- **简单**（单个模块 / 小工具，或「函数 F 怎么工作」这类窄问题）：跳过 explorer 子代理，你在单次遍历里自己探索并解释，直接到第 2b 步。
- **复杂**（跨多文件 / 服务的子系统、横切特性、完整架构总览）：先并行 spawn explorer 子代理，再汇总解释，走第 2a 步。

拿不准就往「简单」靠；卡住了随时可以补 spawn explorer。

### 第 2a 步：探索（仅复杂问题）

把问题拆成 2–4 个并行探索角度，每个角度是子系统的一个互不重复的切片。例：「限流器怎么工作？」拆成：

- Explorer 1：数据模型与状态管理；
- Explorer 2：请求路径与执行；
- Explorer 3：配置与指标基础设施。

每个 explorer 拿到 `references/explorer-prompt.md` 的同一份底稿，外加指明其切片的角度。每个 explorer 要：

- 先广撒网：glob 相关目录、grep 关键类型 / 接口 / 类名（可用 `serena`、`codebase_memory` 做符号与调用图检索，也可直接 `%%bash` grep/read）；
- 沿线索追：从入口点追调用链（调用方、被调方、数据流、类型定义）；
- 读真实代码，不靠文件名猜；
- 追到能从头到尾说清「输入到输出（或触发到效果）」为止，中间任何一步都不含糊；
- 记下反直觉、不显然、新人会理解错的地方。

spawn 方式（Prime）：每个 explorer 一个独立的 `rlm('sub-task', name='<角度>-explorer')` 调用，任务 prompt = `references/explorer-prompt.md` 填好问题与该角度，并要求其以 `agent_message.send(..., receiver_role='parent')` 回传结构化发现。**在各自单独的调用里 spawn，然后结束回合，不要 await 完成。**

每个 explorer 返回结构化发现：找到的组件、追到的流、读过的文件、不显然之处。explorer 之间有重叠没关系，汇总时由你调和。

### 第 2b 步：直接解释（简单问题）

不 spawn explorer。你自己做探索（glob / grep / read，或 `serena` / `codebase_memory`），按 `references/explainer-prompt.md` 的输出格式与沟通风格直接写出解释。同样的结构，只是没有 explorer 发现作为输入。

### 第 3 步：汇总（仅复杂问题）

等所有 explorer 的 `agent_message` 回复到齐后，由你（根代理）把发现汇总成一份连贯解释。按 `references/explainer-prompt.md` 的完整格式写：调和重叠发现、用重读代码解决矛盾、把各切片织成统一图景。**汇总由根代理完成**——不再 spawn 第二个「explainer」子代理（这是相对上游的 Prime 改写）。

### 第 4 步：呈现

把解释呈现给用户。可轻度编辑以求清晰、或补充对话上下文，但不要大幅重写。解释本身才是产品。

### 输出格式

按问题适配以下结构，不必每节都写：

**Overview（概览）**。1–2 段：这是什么、做什么、为什么存在。够读者决定要不要继续读。

**Key Concepts（关键概念）**。重要的类型 / 服务 / 抽象，各一句定义；只列理解下文所需，不穷举。

**How It Works（怎么运行）**。解释的核心，最长一节。走一遍流程：什么触发它、逐步发生什么、数据去哪、决策点在哪。用散文不用伪代码，引用具体文件与函数让读者能自己去看，除非某段代码真的必要否则不贴大段代码。多组件交互或数据多阶段变换时，画一张 mermaid 流程图 / 时序图或 ASCII 图辅助说明（图要澄清、不要装饰）。

**Where Things Live（代码在哪）**。相关文件 / 目录的简要地图；只列开始在这里干活需要的那些。

**Gotchas（坑）**。不显然、令人意外、历史成因、已知锋利边缘；没有值得写的就省略。

## Critique 模式

当用户问的是架构问题 / 缺陷 / 改进，而不只是理解时触发。

### 第 1 步：先解释

完整跑一遍上面的 Explain 流程（第 1–4 步）。必须先理解架构才能批判它。

### 第 2 步：spawn 批评者

解释完成后，spawn 架构批评者子代理。默认每个模型一个批评者；子代理继承你的模型。若架构确实值得多视角，用 `await rlm.find_models(...)` 取多个模型的精确 selector，在各自 `rlm('sub-task', model=...)` 调用里 spawn（在一条消息里把全部批评者 spawn 出去）。

每个批评者拿到：

1. 第 1 步的解释（这样他们不必重新探索）；
2. 相关文件路径（好读真实代码）；
3. `references/critique-rubric.md` 的架构批判量规。

任务 prompt 按 `references/critic-prompt.md` 填，要求其以 `agent_message.send(..., receiver_role='parent')` 回传发现。

### 第 3 步：主审判断

与 interrogate 合并进 code-review 的同款框架：你是务实的 lead，不是聚合器。

给发现分级：

- **Act on（现在改）**：值得现在修的架构问题；
- **Consider（考虑）**：真问题，但成本 / 收益不明；
- **Noted（记录）**：有效观察，低优先级；
- **Dismissed（驳回）**：错的、缺上下文、或纯风格偏好。

先呈现第 1 步的解释，再在下方给批判结论。解释应能独立成立——只想理解系统的人不必趟批判的部分。

## 来源

- **上游仓库**：[cursor/plugins](https://github.com/cursor/plugins)（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/how/SKILL.md`（连同 `references/` 下 `explorer-prompt.md`、`explainer-prompt.md`、`critic-prompt.md`、`critique-rubric.md`）

## 本地 Prime 改写

- **名字**：`how` → `pstack-how`（`pstack-` 前缀，避免与未来用户级 Skill 静默 shadow）。
- **调用契约**：`both`（模型可见可自动调用，保留 `/skill:pstack-how` 显式入口）。
- **删除 Cursor 模型 slug**：上游 `how-explorer`（默认 `grok-4.6-fast-xhigh`）、`how-explainer`（默认 `claude-fable-5-thinking-max`）、`how-critics` 四档 slug 全部删除，改为「子代理继承当前模型；需不同模型用 `rlm.find_models(...)` 取精确 selector」。
- **删除同步 `Task` 返回**：上游 `Task` 子代理的同步返回改为 Prime 异步契约——根代理 `rlm('sub-task')` 扇出、`agent_message` / 文件回流、admission handle 不算结果（见「Prime 异步执行契约」节）。
- **删除 `readonly` 假设**：Prime 无硬只读，改为行为性只读（不改代码）。
- **删除 `subagent_type: generalPurpose`**：Prime 无此字段。
- **explainer 汇总收敛到根代理**：上游「复杂问题再 spawn 一个 explainer 子代理」改为根代理自己汇总，减少一轮异步编排。
- **简单问题 inline**：上游「简单问题 spawn 单个 explainer 子代理」改为根代理单次遍历内自己探索并解释。
- **删除「Use why for motivation」**：`why` 推迟到第二批，本批不引用。
- **工具提示本地化**：上游 Glob / Grep / Read 泛化为 Prime 的 `%%bash` / `serena` / `codebase_memory`。
- **触发边界**：新增「不用本 Skill」边界（原始图查询 → codebase-memory；设计建议 → codebase-design；普通事实查询直接答），避免近似反例误触发。
- **`references/` 四个文件**：`critique-rubric.md` 原样保留；其余三个 prompt 模板按上述 Prime 契约改写（子代理回传方式、只读表述、工具名）。

## 许可证

MIT（Copyright (c) 2026 Lauren Tan），见同目录 `LICENSE.MIT`。
