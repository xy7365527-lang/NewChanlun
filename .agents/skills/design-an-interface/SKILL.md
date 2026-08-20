---
name: design-an-interface
description: Generate multiple radically different interface designs for a module using parallel sub-agents, each written usage-first (caller's usage before the type signature) and screened for design red flags before comparing on depth, locality, and seam placement. Use when user wants to design an API, explore interface options, compare module shapes, or mentions "design it twice". Not for a small, obvious change (rename, reformat, comment fix) — skip design entirely for those.
---

# Design an Interface

Based on "Design It Twice" from "A Philosophy of Software Design": your first idea is unlikely to be the best. Generate multiple radically different designs, then compare.

**Scope: multi-scheme interface exploration.** For **single-scheme deep-module design** (one module's interface, deepening vocabulary, caller-first sketch, design red flags, and the design-to-implementation loop), use `codebase-design`. Pick one per question — don't fan out a multi-candidate bakeoff when the question is really "make this one module deeper," and don't use this skill for a small, obvious change (rename, reformat, comment fix).

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

Spawn 3+ sub-agents simultaneously using Task tool. Each must produce a **radically different** approach.

```
Prompt template for each sub-agent:

Design an interface for: [module description]

Requirements: [gathered requirements]

Constraints for this design: [assign a different constraint to each agent]
- Agent 1: "Minimize method count - aim for 1-3 methods max"
- Agent 2: "Maximize flexibility - support many use cases"
- Agent 3: "Optimize for the most common case"
- Agent 4: "Take inspiration from [specific paradigm/library]"

Output format (usage first — see codebase-design's caller-first sketch):
1. Caller's usage: the README-style usage plus two or three real call sites (what they import, call, and get back)
2. Interface signature (types/methods), derived from that usage
3. What this design hides internally
4. Trade-offs of this approach
```

### 3. Present Designs

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
borrowed fragments and local Prime rewrites.
