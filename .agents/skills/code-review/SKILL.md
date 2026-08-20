---
name: code-review
description: Review the changes since a fixed point (commit, branch, tag, or merge-base) along two axes — Standards (does the code follow this repo's documented coding standards?) and Spec (does the code match what the originating issue/PRD asked for?). Runs both reviews in parallel sub-agents and reports them side by side. Use when the user wants to review a branch, a PR, work-in-progress changes, or asks to "review since X".
---

Two-axis review of the diff between `HEAD` and a fixed point the user supplies:

- **Standards** — does the code conform to this repo's documented coding standards?
- **Spec** — does the code faithfully implement the originating issue / PRD / spec?

Both axes run as **parallel sub-agents** so they don't pollute each other's context, then this skill aggregates their findings.

The issue tracker should have been provided to you — run `/setup-matt-pocock-skills` if `docs/agents/issue-tracker.md` is missing.

## 两种模式与试点边界

本 skill 有两种模式：

- **标准评审（默认，轻量）**：下方 `## Process` 的双轴流程，输出固定为 `## Standards` 与
  `## Spec` 两节，此输出契约不变。
- **对抗评审（pilot，explicit-only）**：仅在用户通过 `/skill:code-review` 显式调用本 skill，
  并明确要求使用 adversarial 子模式时尝试。流程见文末 `## 对抗评审模式`，细节见
  [ADVERSARIAL-MODE.md](ADVERSARIAL-MODE.md)。普通语义路由不自动进入此子模式。

高风险、有争议或非小型 diff 若没有上述显式入口，仍执行标准 Standards/Spec 双轴；可以提示
`Adversarial status: explicit-only`，但不得自动扇出 reviewer，也不得把标准双轴说成多模型
对抗评审。显式进入后仍必须通过模型多样性门：门不足报告 `adversarial unavailable`，启动或
真实回流失败报告 `adversarial degraded`，随后回退标准双轴。低风险小 diff 始终走标准路径。

重新启用自动升级前，`code-review-adversarial` 3+3 夹具必须在至少两个模型上重复稳定触发，
并且 3-selector/2-owner 行为门可真实运行；在此之前 frontmatter 不承诺高风险或关键词自动升级。

## Process

### 1. Pin the fixed point

Whatever the user said is the fixed point — a commit SHA, branch name, tag, `main`, `HEAD~5`, etc. If they didn't specify one, ask for it.

Capture the diff command once: `git diff <fixed-point>...HEAD` (three-dot, so the comparison is against the merge-base). Also note the list of commits via `git log <fixed-point>..HEAD --oneline`.

Before going further, confirm the fixed point resolves (`git rev-parse <fixed-point>`) and the diff is non-empty. A bad ref or empty diff should fail here — not inside two parallel sub-agents.

### 2. Identify the spec source

Look for the originating spec, in this order:

1. Issue references in the commit messages (`#123`, `Closes #45`, GitLab `!67`, etc.) — fetch via the workflow in `docs/agents/issue-tracker.md`.
2. A path the user passed as an argument.
3. A PRD/spec file under `docs/`, `specs/`, or `.scratch/` matching the branch name or feature.
4. If nothing is found, ask the user where the spec is. If they say there isn't one, the **Spec** sub-agent will skip and report "no spec available".

### 3. Identify the standards sources

Anything in the repo that documents how code should be written, such as `CODING_STANDARDS.md` or `CONTRIBUTING.md`.

On top of whatever the repo documents, the Standards axis always carries the **smell baseline** below — a fixed set of Fowler code smells (_Refactoring_, ch.3) that applies even when a repo documents nothing. Two rules bind it:

- **The repo overrides.** A documented repo standard always wins; where it endorses something the baseline would flag, suppress the smell.
- **Always a judgement call.** Each smell is a labelled heuristic ("possible Feature Envy"), never a hard violation — and, like any standard here, skip anything tooling already enforces.

Each smell reads *what it is* → *how to fix*; match it against the diff:

- **Mysterious Name** — a function, variable, or type whose name doesn't reveal what it does or holds. → rename it; if no honest name comes, the design's murky.
- **Duplicated Code** — the same logic shape appears in more than one hunk or file in the change. → extract the shared shape, call it from both.
- **Feature Envy** — a method that reaches into another object's data more than its own. → move the method onto the data it envies.
- **Data Clumps** — the same few fields or params keep travelling together (a type wanting to be born). → bundle them into one type, pass that.
- **Primitive Obsession** — a primitive or string standing in for a domain concept that deserves its own type. → give the concept its own small type.
- **Repeated Switches** — the same `switch`/`if`-cascade on the same type recurs across the change. → replace with polymorphism, or one map both sites share.
- **Shotgun Surgery** — one logical change forces scattered edits across many files in the diff. → gather what changes together into one module.
- **Divergent Change** — one file or module is edited for several unrelated reasons. → split so each module changes for one reason.
- **Speculative Generality** — abstraction, parameters, or hooks added for needs the spec doesn't have. → delete it; inline back until a real need shows.
- **Message Chains** — long `a.b().c().d()` navigation the caller shouldn't depend on. → hide the walk behind one method on the first object.
- **Middle Man** — a class or function that mostly just delegates onward. → cut it, call the real target direct.
- **Refused Bequest** — a subclass or implementer that ignores or overrides most of what it inherits. → drop the inheritance, use composition.

### 4. Spawn both sub-agents in parallel

Send a single message with two `Agent` tool calls. Use the `general-purpose` subagent for both.

**Standards sub-agent prompt** — include:

- The full diff command and commit list.
- The list of standards-source files you found in step 3, **plus the smell baseline from step 3** pasted in full — the sub-agent has no other access to it.
- The brief: "Report — per file/hunk where relevant — (a) every place the diff violates a documented standard: cite the standard (file + the rule); and (b) any baseline smell you spot: name it and quote the hunk. Distinguish hard violations from judgement calls — documented-standard breaches can be hard, but baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces. Under 400 words."

**Spec sub-agent prompt** — include:

- The diff command and commit list.
- The path or fetched contents of the spec.
- The brief: "Report: (a) requirements the spec asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec line for each finding. Under 400 words."

If the spec is missing, skip the Spec sub-agent and note this in the final report.

### 5. Aggregate

Present the two reports under `## Standards` and `## Spec` headings, verbatim or lightly cleaned. Do **not** merge or rerank findings — the two axes are deliberately separate (see _Why two axes_).

End with a one-line summary: total findings per axis, and the worst issue _within each axis_ (if any). Don't pick a single winner across axes — that's the reranking the separation exists to prevent.

## Why two axes

A change can pass one axis and fail the other:

- Code that follows every standard but implements the wrong thing → **Standards pass, Spec fail.**
- Code that does exactly what the issue asked but breaks the project's conventions → **Spec pass, Standards fail.**

Reporting them separately stops one axis from masking the other.

## 对抗评审模式（explicit-only pilot）

试点边界见文首「两种模式与试点边界」。只有 `/skill:code-review` 显式调用并明确要求
adversarial 子模式时才尝试进入。此模式把 pstack `interrogate` 的独立 reviewer、统一 rubric、
agreement map 与 lead judgment 并入本 skill，并按 Prime RLM 异步子代理语义改写。完整流程与
输出格式见 [ADVERSARIAL-MODE.md](ADVERSARIAL-MODE.md)，要点：

1. **确认显式入口并写明意图**：一段话说明这段改动想达成什么；reviewer 只挑战
   「实现是否达成意图」，不挑战意图本身。
2. **模型多样性门 + 独立 reviewer**：显式进入后先调用 `await rlm.find_models(limit=8)`；
   只有至少 3 个可启动 selector 且覆盖至少 2 个 vendor/family 才准拉起 3 名 reviewer。
   任一跨厂 selector 启动失败即停止对抗流程并回退标准双轴，不得用同厂模型补足。每个
   reviewer 用 `await rlm('task', name='adversarial-reviewer-<A/B/C>', model=<selector>)`
   独立拉起，拿到相同的意图 + diff + rubric，**绝不把其他 reviewer 的结论转给它**。
   结果经 `agent_message`、约定的 session-dir 文件或最终 JSONL 回流；根代理必须读取真实
   child 产物，admission handle 不是结果。门不足报告 `adversarial unavailable`，启动或
   真实结果回流失败报告 `adversarial degraded`，随后仍输出标准 `Standards`/`Spec` 双轴。
3. **统一 rubric**：每个 reviewer 都用 [ADVERSARIAL-RUBRIC.md](ADVERSARIAL-RUBRIC.md)
   （正确性 / 根因 vs 症状 / 结构完整 / 验证 / 复杂度预算 / 安全）与
   [ADVERSARIAL-CODE-QUALITY.md](ADVERSARIAL-CODE-QUALITY.md)（结构简化 / 1k 行门槛 /
   意面增长等）。reviewer prompt 模板见
   [ADVERSARIAL-REVIEWER-PROMPT.md](ADVERSARIAL-REVIEWER-PROMPT.md)。
4. **汇总 + agreement map**：解析所有 finding，识别共识（≥2 模型独立发现）、单模型发现、
   去重、分歧。
5. **主控裁决（lead judgment）**：你是主控，不是聚合器。按
   [ADVERSARIAL-LEAD-JUDGMENT.md](ADVERSARIAL-LEAD-JUDGMENT.md) 把每条 finding 归入
   **Act on（接受）/ Consider（考虑）/ Noted（记录）/ Dismissed（驳回）**，各写一句理由。
   **重复发现不替代你的技术判断**。

输出结构：`### Intent` / `### Reviewers` / `### Act On` / `### Consider` / `### Noted` /
`### Dismissed` / `### Agreement Map`。**不要自动应用改动**。

## 来源（interrogate 移植记录）

- **上游仓库**：`cursor/plugins`（GitHub，`pstack/` 目录）
- **固定 SHA**：`fd6dd6f7276956a532bb78a748a8d2818b6eb5f4`（不自动跟随上游 `main`）
- **原始 Skill**：`pstack/skills/interrogate/SKILL.md` 及其
  `references/{reviewer-prompt,rubric,code-quality-review,lead-judgment}.md`
- **许可证**：MIT（Copyright (c) 2026 Lauren Tan），原文见
  `docs/agents/pstack-lite/LICENSE.MIT`。
- **借用片段与本地改写**：逐条见 [ADVERSARIAL-MODE.md](ADVERSARIAL-MODE.md)
  「来源与本地改写」节（含各上游文件 SHA-256）。
- 本 skill **不创建 `pstack-interrogate`**：对抗评审是 `code-review` 的可选模式，普通
  评审的轻量路径与双轴输出保持不变。
