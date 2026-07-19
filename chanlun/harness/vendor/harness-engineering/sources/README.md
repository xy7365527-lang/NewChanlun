# Sources

- [Twitter corpus](twitter/)
- [Machine-readable source manifest](sources.json)

The manifest preserves each source's own kind and classification while its
`evidence.type` records how the repository can inspect it: by link, immutable
snapshot, living first-party note, or private review. Run
`uv run --script sources/scripts/validate_manifest.py` and
`uv run --script sources/scripts/test_manifest.py` after changing the manifest
or a local source artifact.

`reviewed_at` records when this corpus examined a source. Immutable snapshots
also retain their byte-retrieval date and their own rights basis. Individually
catalogued X posts select their evidence from the captured public-post corpus by
status ID. That row owns timestamps, capture and engagement data, availability,
and screenshot provenance; the manifest retains the bibliographic date and
links. Private sources use private-review evidence with an access label. The
other variants declare that their evidence is intended for public inspection;
they do not guarantee that an upstream host remains accessible.

The public-post capture is not an exhaustive account export. An official X
export can extend it with posts missed by public discovery or deleted without an
independent capture.

## Ryan Lopopolo’s writing

| Date       | Work                                                                                                                                                                                     | Contribution                                                                                                  |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 2023-06-13 | [I Wrote 4,000 Lines of Code with ChatGPT in a Weekend](https://hyperbo.la/w/chatgpt-4000/) · [local text](raw/hyperbola/chatgpt-4000.mdx)                                               | Regular modules improved first-pass generation and anticipated agents grounded in local code and history.     |
| 2025-08-10 | [MCP Solves Tool Discovery for LLMs](https://hyperbo.la/w/tool-discovery/) · [local text](raw/hyperbola/tool-discovery.mdx)                                                              | Capabilities must be discoverable and legible to the model.                                                   |
| 2026-02-11 | [Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/)                                                                           | The seminal statement of the practice.                                                                        |
| 2026-02-17 | [Harness Engineering the Blog Build (Again)](https://hyperbo.la/w/harness-engineering-the-blog-build/) · [local text](raw/hyperbola/harness-engineering-the-blog-build.mdx)              | A repository-local build, policy, architecture, and verification harness.                                     |
| 2026-03-13 | [The Production Function Changed](https://hyperbo.la/w/production-function-changed/) · [local text](raw/hyperbola/production-function-changed.mdx)                                       | Cheap implementation moves the bottleneck toward judgment and proof.                                          |
| 2026-03-13 | [Software Work Is No Longer Scheduled](https://hyperbo.la/w/software-work-not-scheduled/) · [local text](raw/hyperbola/software-work-not-scheduled.mdx)                                  | Bounded work with clear intent, understood interfaces, guardrails, and verifiable results can run asynchronously. |
| 2026-03-13 | [Stop Treating Code as the Artifact](https://hyperbo.la/w/code-is-not-the-artifact/) · [local text](raw/hyperbola/code-is-not-the-artifact.mdx)                                          | Specifications, boundaries, and guardrails outlast generated source.                                          |
| 2026-03-13 | [Agent Utilization Is the New Performance Ceiling](https://hyperbo.la/w/agents-agents-agents/) · [local text](raw/hyperbola/agents-agents-agents.mdx)                                    | Agent utilization depends on access across the software lifecycle.                                            |
| 2026-03-30 | [A Lazy Prompt Turned Into a RustSec Advisory](https://hyperbo.la/w/lazy-prompt-rustsec/) · [local text](raw/hyperbola/lazy-prompt-rustsec.mdx)                                          | Security work closes the loop from reproducer through release and advisory.                                   |
| 2026-04-09 | [Coding Agents for Technical Non-Engineers](https://hyperbo.la/w/coding-agents-for-technical-non-engineers/) · [local text](raw/hyperbola/coding-agents-for-technical-non-engineers.mdx) | Paved environments let domain experts encode their own work.                                                  |
| 2026-04-10 | [What Does It Mean to Do a Good Job?](https://hyperbo.la/w/what-does-it-mean-to-do-a-good-job/) · [local text](raw/hyperbola/what-does-it-mean-to-do-a-good-job.mdx)                     | Tacit requirements, proof, and focused reviewer convergence define quality.                                   |
| 2026-04-19 | [Enabling Codex to Upgrade My Robot Vacuum](https://hyperbo.la/w/robot-vacuum-canary-tailscale/) · [local text](raw/hyperbola/robot-vacuum-canary-tailscale.mdx)                         | Narrow authority, canaries, approval, access verification, and rollback make a dangerous operation tractable. |
| 2026-07-17 | [Please Go Brr, on Token Mandates](https://hyperbo.la/w/token-mandates/) · [local text](raw/hyperbola/token-mandates.mdx)                                                                | Distributed exploration discovers practitioners, business problems, and deployment patterns worth funding.    |
| 2026-07-17 | [Code Reds Need Maintenance Loops](https://hyperbo.la/w/code-reds-need-maintenance-loops/) · [local text](raw/hyperbola/code-reds-need-maintenance-loops.mdx)                            | Argues that adaptive episodes should leave durable maintenance loops for newly discovered invariants.         |

Agents that receive an empty response or a Cloudflare challenge from the
canonical OpenAI essay can use the [`uv` fetch helper].

[`uv` fetch helper]: scripts/fetch_openai.py

## Talks and interviews

| Date       | Work                                                                                                                                                                                                              | Preserved access                                               |
| ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| 2026-03-03 | [Code Is Free: Securing Software in the Agentic Future] · [official transcript] · [slides]                                                                                                                        | [Wayback conference page] · [Wayback video page]               |
| 2026-03-10 | [Build Hour: API & Codex](https://www.youtube.com/watch?v=rhsSqr0jdFw) · [Rewiz transcript]                                                                                                                       | [Wayback OpenAI Build Hour video page]                         |
| 2026-04-07 | [Extreme Harness Engineering for Token Billionaires](https://www.latent.space/p/harness-eng) · [video](https://www.youtube.com/watch?v=CeOXx-XTYek)                                                               | [Wayback transcript page] · [Wayback Latent Space video page]  |
| 2026-04-17 | [Harness Engineering: How to Build Software When Humans Steer, Agents Execute](https://www.youtube.com/watch?v=am_oeAoUhew)                                                                                       | [Wayback AI Engineer video page]                               |
| 2026-04-28 | [Harness Engineering—Practical Patterns for Agent-First Software Development](https://vimeo.com/1189320346) · [attendee notes](https://orenbochman.github.io/blog/posts/2026/04-28-ODSC-AI-2026-Day-1/talk7.html) | [Wayback ODSC attendee notes]                                  |
| 2026-05-25 | [How PMs Ship 100K Lines of Code](https://www.aakashg.com/how-pms-ship-100k-lines-of-code/) · [video](https://www.youtube.com/watch?v=8suwvrF0Lv0)                                                                | [Wayback Aakash interview] · [Wayback Aakash video page]       |
| 2026-06-01 | [Harness Engineering at AI Native DevCon](https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md)                                                  | [Wayback AI Native DevCon transcript]                          |
| 2026-06-04 | [Harness Engineering at Craft Conference](https://craft-conf.com/2026/talk/ryan-lopopolos-talk)                                                                                                                   | [Wayback Craft talk page]                                      |
| 2026-06-09 | [OpenAI’s Framework for Shipping Code at 70 PRs/Week](https://tessl.io/podcast/109/) · [video](https://www.youtube.com/watch?v=MFQIKbr1IEo)                                                                       | [Wayback Tessl interview] · [Wayback AI Native Dev video page] |

[Wayback OpenAI Build Hour video page]:
  https://web.archive.org/web/20260310195018id_/https://www.youtube.com/watch?v=rhsSqr0jdFw
[Code Is Free: Securing Software in the Agentic Future]:
  https://www.youtube.com/watch?v=U2O14Jd3MBU
[official transcript]:
  https://drive.google.com/file/d/1_frxpWyqd5n2lqBmiPzcBX7oUP1dm78f/view?usp=sharing
[slides]:
  https://drive.google.com/file/d/1auBbLrfTXr4nHKpXjojhVPZZcFjwBHBt/view?usp=sharing
[Wayback conference page]:
  https://web.archive.org/web/20260428044423id_/https://unpromptedcon.org/abstract-march2026/
[Wayback video page]:
  https://web.archive.org/web/20260327172831id_/https://www.youtube.com/watch?v=U2O14Jd3MBU
[Rewiz transcript]: https://rewiz.app/channels/%40openai/build-hour-api-codex
[Wayback transcript page]:
  https://web.archive.org/web/20260422112722id_/https://www.latent.space/p/harness-eng
[Wayback Latent Space video page]:
  https://web.archive.org/web/20260421021709id_/https://www.youtube.com/watch?v=CeOXx-XTYek
[Wayback AI Engineer video page]:
  https://web.archive.org/web/20260515140139id_/https://www.youtube.com/watch?v=am_oeAoUhew
[Wayback ODSC attendee notes]:
  https://web.archive.org/web/20260718075105id_/https://orenbochman.github.io/blog/posts/2026/04-28-ODSC-AI-2026-Day-1/talk7.html
[Wayback Aakash interview]:
  https://web.archive.org/web/20260718074920id_/https://www.aakashg.com/how-pms-ship-100k-lines-of-code/
[Wayback Aakash video page]:
  https://web.archive.org/web/20260606125303id_/https://www.youtube.com/watch?v=8suwvrF0Lv0&feature=youtu.be
[Wayback AI Native DevCon transcript]:
  https://web.archive.org/web/20260718014014id_/https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md
[Wayback Craft talk page]:
  https://web.archive.org/web/20260718075132id_/https://craft-conf.com/2026/talk/ryan-lopopolos-talk
[Wayback Tessl interview]:
  https://web.archive.org/web/20260718075246id_/https://tessl.io/podcast/109/
[Wayback AI Native Dev video page]:
  https://web.archive.org/web/20260613220003id_/https://www.youtube.com/watch?v=MFQIKbr1IEo

The video-page captures preserve the watch page, title or video identifier, and
metadata. They are not treated as archived copies of the audiovisual streams.
Vimeo’s robots policy prevented preservation of the ODSC video page. The Rewiz
transcript page returned an upstream TLS error during preservation. Their
canonical links and archive status remain in [`sources.json`](sources.json).

## Public posts

The [Twitter source index] groups captured posts into twelve recurring working
principles and keeps the direct evidence beside each claim.

[Twitter source index]: twitter/

## Influences and alternate framings

- [matklad: `ARCHITECTURE.md`](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html)
- [artichoke/artichoke: `ARCHITECTURE.md`](https://github.com/artichoke/artichoke/blob/trunk/ARCHITECTURE.md)
- [artichoke/artichoke@fbb43b6](https://github.com/artichoke/artichoke/commit/fbb43b6ce18556412a489a88e5cc24dea9b587a8)
- [artichoke/artichoke@1dd8978](https://github.com/artichoke/artichoke/commit/1dd8978fe4edf1971928fd00e30edbd124fd0aaa)
- [artichoke/artichoke@fa5b2eb](https://github.com/artichoke/artichoke/commit/fa5b2eb907d8746efd7cf51f07cd84e7e1f9b49f)
- [Alexis King: “Parse, don’t validate”](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [Martin Fowler: Strangler Fig](https://martinfowler.com/bliki/StranglerFigApplication.html)
- [George Zhang: “Harness Engineering Is Cybernetics”](https://x.com/i/article/2030414577213820928)
- [Birgitta Böckeler: initial harness-engineering memo](https://martinfowler.com/articles/exploring-gen-ai/harness-engineering-memo.html)
- [Birgitta Böckeler: “Harness engineering for coding agent users”](https://martinfowler.com/articles/harness-engineering.html)

[Influences and Alternate Framings](../docs/lineage/) develops these
relationships and their chronology.

## Related research

OpenAI's [confessions publication] and [research paper] study a separately
rewarded model self-report under deliberately induced failures. The work
sharpens the evidentiary limits of MLD: ordinary agent reflection remains
low-trust telemetry until traces, tests, review, and outcomes corroborate it.

[confessions publication]:
  https://openai.com/index/how-confessions-can-keep-language-models-honest/
[research paper]: https://arxiv.org/abs/2512.08093

## Agent interoperability

- [Polytoken introduction](https://docs.polytoken.dev/introduction/)
- [Polytoken tool reference](https://docs.polytoken.dev/reference/tools/)
- [Ryan Lopopolo's Polytoken implementation observation](ryan-notes.md#polytoken-and-codex-apply_patch)
- [Codex `apply_patch` grammar and instructions](https://github.com/openai/codex/blob/main/codex-rs/core/prompt_with_apply_patch_instructions.md)
- [Agent Client Protocol introduction](https://agentclientprotocol.com/get-started/introduction)
  · [local text](raw/acp/introduction.mdx)
- [Agent Client Protocol architecture](https://agentclientprotocol.com/get-started/architecture)
  · [local text](raw/acp/architecture.mdx)
- [Agent Client Protocol tool calls](https://agentclientprotocol.com/protocol/v1/tool-calls)
- [Agent Client Protocol extensibility](https://agentclientprotocol.com/protocol/v1/extensibility)

## Projects and cases

- [Future Regret in Artichoke's State Refactor](../evals/artichoke-state-modeling.md):
  [failed prototype #442](https://github.com/artichoke/artichoke/pull/442),
  [50-PR preparation ledger](artichoke-state-refactor-ledger.md),
  [second integration attempt #661](https://github.com/artichoke/artichoke/pull/661),
  [merged state change #670](https://github.com/artichoke/artichoke/pull/670),
  [shutdown regression #674](https://github.com/artichoke/artichoke/pull/674),
  [FFI-boundary follow-through #723](https://github.com/artichoke/artichoke/pull/723),
  and [Rust symbol table #730](https://github.com/artichoke/artichoke/pull/730).
- [Infrastructure as a Typed Control Plane](../docs/domain-modeling/homelab.md)
- [hyperbo.la](../docs/domain-modeling/hyperbola.md)
- [RustSec](../docs/proof/rustsec.md)
- [artichoke/rand_mt](https://github.com/artichoke/rand_mt):
  [agent guide](https://github.com/artichoke/rand_mt/blob/trunk/AGENTS.md),
  [architecture](https://github.com/artichoke/rand_mt/blob/trunk/ARCHITECTURE.md),
  [dependencies](https://github.com/artichoke/rand_mt/blob/trunk/docs/dependencies.md),
  [guardrails](https://github.com/artichoke/rand_mt/tree/trunk/docs/guardrails),
  and
  [automations](https://github.com/artichoke/rand_mt/tree/trunk/docs/automations)
- `rand_mt` rollout:
  [automation runbooks (#327)](https://github.com/artichoke/rand_mt/pull/327),
  [supply-chain posture (#331)](https://github.com/artichoke/rand_mt/pull/331),
  [agent routing and guardrails (#332)](https://github.com/artichoke/rand_mt/pull/332),
  [flat architecture and invariants (#334)](https://github.com/artichoke/rand_mt/pull/334),
  and
  [authoritative upstream diff review (#337)](https://github.com/artichoke/rand_mt/pull/337)
- The same repository-maintenance approach was adapted in
  [intaglio (#385)](https://github.com/artichoke/intaglio/pull/385),
  [sysdir-rs (#141)](https://github.com/artichoke/sysdir-rs/pull/141), and
  [known-folders-rs (#139)](https://github.com/artichoke/known-folders-rs/pull/139).
- [openai/symphony](https://github.com/openai/symphony) and
  [An open-source spec for Codex orchestration: Symphony](https://openai.com/index/open-source-codex-orchestration-symphony/)

## Source snapshots

- [Ryan’s hyperbo.la articles](raw/hyperbola/)
- [Agent Client Protocol documentation](raw/acp/)

See [`COPYING.md`](../COPYING.md) for license scope and attribution.
