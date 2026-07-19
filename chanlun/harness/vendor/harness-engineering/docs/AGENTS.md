# Synthesis Guide

Documents under `docs/` are organized by thesis. A folder `README.md` develops
the argument; supporting files develop cases, applications, caveats, or
evaluation.

## Document ownership

[`ARCHITECTURE.md`](../ARCHITECTURE.md) owns the document and routing boundaries
for the root, agent guide, thesis index, playbooks, evaluation method, and
sources. Follow that ownership map and link to an owner instead of recreating
its section in another front door.

## Writing

- Open with the claim. Do not justify why the document exists.
- Before landing a substantive public-facing synthesis change, obtain separate
  adversarial reviews for claim integrity and external-reader signal-to-noise.
- Write for an external audience. Remove research status, source counts,
  visibility classifications, attribution deliberation, and editorial dialogue.
- Prefer concrete nouns and verbs over labels such as “framework,” “pattern,” or
  “lineage” when the relationship can be stated directly.
- Use named reference links with human-readable text such as
  `artichoke/artichoke:ARCHITECTURE.md` or `artichoke/artichoke@fbb43b6`. Put
  each `[name]: URL` definition immediately after the paragraph or list that
  uses it. Do not use inline link destinations in prose.
- When a long article or transcript supports a specific claim, add a Chrome Text
  Fragment (`#:~:text=`) that selects a short, distinctive excerpt. Keep
  bibliographic and whole-work links canonical, and omit a fragment when the
  target does not expose stable matching text.
- Let sources shape the argument. Cite long-form work for developed arguments
  and public posts for the precise claims or examples they contribute. Weave
  each source into the paragraph it supports; do not end a thesis or section
  with a bag of loosely related links. Leave unused evidence in the source index
  instead of attaching it after the synthesis.
- State the positive claim directly. Avoid manufacturing emphasis with “not X,
  but Y,” “the point is not,” “rather than,” and similar contrastive templates.
  Use negation only when the excluded alternative is materially necessary to the
  argument.
- Use Mermaid for diagrams. Do not use ASCII art.
- Do not repeat a thesis at length in a supporting note. Link to its owner and
  develop only what is new.

## Claims

- Distinguish Ryan’s individual writing, work by his team, other implementation
  evidence, earlier influences, and later alternate framings through accurate
  prose—not reader-facing classification labels.
- The OpenAI harness-engineering essay was published on 2026-02-11. Böckeler’s
  Fowler-site article appeared on 2026-04-02 and is not prior art for it.
- Harness engineering holds a chosen model and coding agent fixed as a black box
  while changing context, tools, and environment. “Fixed” is an evaluation and
  deployment boundary, not permanent allegiance to a lab product.
- Polytoken’s reuse of Codex `apply_patch` semantics and capability-preserving
  ACP interoperability are aligned examples. Least-common-denominator flattening
  is the caveat.
- MLD is telemetry for the harness builder, not raw context automatically fed to
  future agents.

## Cases

Sanitize private implementation evidence into substantive design and tradeoff
analysis. Do not mention authorization mechanics, private paths, hostnames,
credentials, or inaccessible links. State what the system does, why the design
matters, what proof exists, and which maintenance or residual risk remains.
