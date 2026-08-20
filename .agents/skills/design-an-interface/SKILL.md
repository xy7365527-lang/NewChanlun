---
name: design-an-interface
description: Generate multiple radically different interface designs for a module using parallel sub-agents, each written usage-first (caller's usage before the type signature) and screened for design red flags before comparing on depth, locality, and seam placement. Use when user wants to design an API, explore interface options, compare module shapes, or mentions "design it twice". Not for a small, obvious change (rename, reformat, comment fix) — skip design entirely for those.
---

# Design an Interface

Based on "Design It Twice" from "A Philosophy of Software Design": your first idea is unlikely to be the best. Generate multiple radically different designs, then compare.

**Scope: multi-scheme interface exploration.** For **single-scheme deep-module design** (one module's interface, deepening vocabulary, caller-first sketch, design red flags, and the design-to-implementation loop), use `codebase-design`. Pick one per question — don't fan out a multi-candidate bakeoff when the question is really "make this one module deeper," and don't use this skill for a small, obvious change (rename, reformat, comment fix).

## Prime 异步契约

本工作流使用 Prime RLM，而不是同步任务运行器。

- 根代理负责扇出。admission 前，使用运行时提供的根 `session-dir`，按稳定候选名为每个候选分配一个唯一的**绝对结果文件路径**，并写入对应 child 的 prompt；不得在 admission 后猜测 child 目录。
- 至少分别调用三次 `await rlm('sub-task', name='<stable-name>')`，生成三个独立 child，并保留每个返回的 handle。handle 只是 admission metadata，**不是设计结果**；admission 完成后结束当前 turn，让回复在后续 turn 到达。
- 每个 child 都须收到明确上限。每个候选默认最多读取 **8 files**、使用 **10 tool calls**、花费 **6 minutes**，输出不超过 **1,200 words**。最后 **2 tool calls** 专供返回完整结果。约在 **70% context** 时停止探索；宁可如实列出 open questions，也不得耗尽预算。
- child 先用 `await agent_message.send(message, receiver_role='parent')` 发送完整结果。若 `agent_message` 不可用、未导入或发送失败，才把**同一份完整结果**写入已分配的结果文件；这是 child 唯一允许的写操作。无论消息或文件回流是否成功，child 的最终 assistant response 仍须保留同一份完整结果。
- 根代理按顺序检查三个渠道：完整的 parent message、已分配的结果文件、保留 handle 对应的最终会话 JSONL。第三路从 `handle.session_dir` 枚举 `*.jsonl`；只有恰好一个日志文件时才逐行解析 JSON，并从末尾向前查找 `message.role = "assistant"` 且 `message.content` 含 `type = "text"` 的记录。忽略 `thinking`、`toolCall`、`toolResult` 和其他非文本内容；rollout preview、source-tool tail event 与 admission handle 都不是结果。
- 任一渠道的文本只有在模板的六个标题 `Caller's usage`、`Interface signature`、`What it hides`、`Trade-offs`、`Red-flag screen`、`Open questions` 全部存在且各自正文非空时，才算完整候选。读取 JSONL 时继续向前查找，直到找到满足此判据的 assistant 文本或日志耗尽。
- 只有**每个已 admission 的 child** 都取得完整结果后，才可比较和综合。若某结果缺失或不完整，只能对原 child 做一次 bounded follow-up：调用 `await agent_message.send(message=<recovery request>, receiver_role='child', receiver_name=handle.name)`，然后再检查一次三个渠道。仍缺失时，报告缺失候选并在比较前停止；不得虚构候选、由根代理代写或比较不完整集合。

## Workflow

### 1. Gather Requirements

Before designing, understand:

- [ ] What problem does this module solve?
- [ ] Who are the callers? (other modules, external users, tests)
- [ ] What are the key operations?
- [ ] Any constraints? (performance, compatibility, existing patterns)
- [ ] What should be hidden inside vs exposed?
- [ ] The caller's usage, written first: the README-style usage plus two or three realistic call sites (what they import, what they call, what they get back). The usage is the spec — the type sketches below are derived from it, not the reverse.

Ask: "What does this module need to do? Who will use it? What does the caller's code look like?"

### 2. Generate Designs (Parallel Sub-Agents)

根代理按 Prime 异步契约创建**至少三个彼此独立的 RLM admission**，每个候选一个。为每个 child 指定真正不同的设计压力；只改名称或方法数量、却保留同一条 seam，不算不同设计。

为每个 child 填写并发送 [`references/design-candidate-prompt.md`](references/design-candidate-prompt.md)。admission 前须替换所有占位符，包括唯一绝对路径 `{RESULT_FILE}` 以及明确的读取、工具、时间和输出预算。典型的独立压力包括：

- 候选 1：尽量减少方法数量（目标为 1–3 个）。
- 候选 2：尽量覆盖已知用例的灵活性。
- 候选 3：优化主调用路径，并让误用变得困难。
- 可选候选 4：选择实质不同的 seam 或范式。

每个 child 必须保留既有 usage-first 输出契约：调用方用法、由用法推导的接口签名、隐藏的复杂度与权衡。根代理不得把这次扇出再委托给其他编排器，也不得对同一接口问题调用 `pstack-arena`。

### 3. Present Designs

先应用 Prime 异步契约中的完成门。从消息、指定文件或最终 JSONL 恢复每个 child 的完整结果；不得用 admission handle 或根代理代写的候选替代。

Show each design with:

1. **Usage examples** - how callers actually use it in practice
2. **Interface signature** - types, methods, params
3. **What it hides** - complexity kept internal

Present designs sequentially so user can absorb each approach before comparison.

### 4. Compare Designs

Screen each candidate against the design red flags first — shallow module, information leakage, temporal decomposition, pass-through method (see `codebase-design`, "Design red flags"). Reject or revise any candidate that trips one; only viable shapes reach the comparison.

After showing all designs, compare them on:

- **Interface simplicity**: fewer methods, simpler params
- **General-purpose vs specialized**: flexibility vs focus
- **Implementation efficiency**: does shape allow efficient internals?
- **Depth**: small interface hiding significant complexity (good) vs large interface with thin implementation (bad)
- **Ease of correct use** vs **ease of misuse**

Discuss trade-offs in prose, not tables. Highlight where designs diverge most.

### 5. Synthesize

Often the best design combines insights from multiple options. Ask:

- "Which design best fits your primary use case?"
- "Any elements from other designs worth incorporating?"

## Evaluation Criteria

From "A Philosophy of Software Design":

**Interface simplicity**: Fewer methods, simpler params = easier to learn and use correctly.

**General-purpose**: Can handle future use cases without changes. But beware over-generalization.

**Implementation efficiency**: Does interface shape allow efficient implementation? Or force awkward internals?

**Depth**: Small interface hiding significant complexity = deep module (good). Large interface with thin implementation = shallow module (avoid).

## Anti-Patterns

- Don't let sub-agents produce similar designs - enforce radical difference
- Don't skip comparison - the value is in contrast
- Don't implement - this is purely about interface shape
- Don't evaluate based on implementation effort

## Source

The caller-first and design-red-flag checks above are adapted from pstack `architect`
(`cursor/plugins@fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`, `pstack/skills/architect/`,
MIT — Copyright (c) 2026 Lauren Tan). See [ARCHITECT-MERGE.md](ARCHITECT-MERGE.md) for
borrowed fragments and local Prime rewrites. 异步 RLM 结果契约及其确定性夹具是 #1136 的本地 Prime 修复，并非借用上游内容。
