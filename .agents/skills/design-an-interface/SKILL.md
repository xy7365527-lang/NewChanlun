---
name: design-an-interface
description: Generate multiple radically different interface designs for a module using parallel sub-agents, each written usage-first (caller's usage before the type signature) and screened for design red flags before comparing on depth, locality, and seam placement. Use when user wants to design an API, explore interface options, compare module shapes, or mentions "design it twice". Not for a small, obvious change (rename, reformat, comment fix) — skip design entirely for those.
---

# Design an Interface

Based on "Design It Twice" from "A Philosophy of Software Design": your first idea is unlikely to be the best. Generate multiple radically different designs, then compare.

**Scope: multi-scheme interface exploration.** For **single-scheme deep-module design** (one module's interface, deepening vocabulary, caller-first sketch, design red flags, and the design-to-implementation loop), use `codebase-design`. Pick one per question — don't fan out a multi-candidate bakeoff when the question is really "make this one module deeper," and don't use this skill for a small, obvious change (rename, reformat, comment fix).

## Prime async contract

This workflow uses Prime RLM, not a synchronous task runner.

- The root agent owns the fan-out. Before admission, allocate one unique **absolute result-file path in the root session directory** for each candidate and put it in that child's prompt. Do not guess a child directory after admission.
- Spawn at least three independent children with separate `await rlm('sub-task', name='<stable-name>')` calls. Keep every returned handle. A handle is admission metadata, **not a design result**; end the turn after the admissions so replies can arrive on later turns.
- Give every child explicit limits. Default per candidate: read at most **8 files**, use at most **10 tool calls**, spend at most **6 minutes**, and return at most **1,200 words**. Reserve the final **2 tool calls** for returning the complete result. Stop exploring at about **70% context** and return honest open questions instead of exhausting the budget.
- A child first sends its complete result with `await agent_message.send(message, receiver_role='parent')`. If `agent_message` is unavailable, not imported, or the send fails, it writes the **same complete result** to its assigned result file. That fallback is the child's only allowed write.
- The root accepts a candidate only from a complete parent message, the assigned result file, or the child's complete final response in its final session JSONL located through the retained handle/session directory. Check those channels in that order; the final JSONL prevents a missing message bridge from discarding an already-produced design. Rollout previews, source-tool tail events, and admission handles are not results.
- Compare and synthesize only after **every admitted child** has a complete result through one of those channels. If a result remains missing after one bounded follow-up/recovery check, report the missing candidate and stop before comparison; do not invent it, replace it with root-authored work, or compare an incomplete set.

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

The root creates **at least three separate RLM admissions**, one per candidate, as specified by the Prime async contract. Assign a genuinely different design pressure to each child; changing names or method counts while preserving the same seam does not count as a different design.

Fill and send [`references/design-candidate-prompt.md`](references/design-candidate-prompt.md) for every child. Replace all placeholders before admission, including the unique absolute `{RESULT_FILE}` and explicit read/tool/time/output budgets. Typical independent pressures are:

- Candidate 1: minimize method count (aim for 1–3 methods).
- Candidate 2: maximize flexibility across known use cases.
- Candidate 3: optimize the dominant call path and make misuse difficult.
- Optional candidate 4: choose a materially different seam or paradigm.

Each child must return the existing usage-first output contract: caller usage, derived interface signature, hidden complexity, and trade-offs. The root must not delegate this fan-out to another orchestrator and must not invoke `pstack-arena` for the same interface question.

### 3. Present Designs

First apply the completion gate in the Prime async contract. Recover each child's complete result from its message, assigned file, or final JSONL; never substitute the admission handle or a root-written candidate.

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
borrowed fragments and local Prime rewrites. The async RLM result contract and its deterministic fixture are a local Prime repair for #1136; they are not borrowed from upstream.
