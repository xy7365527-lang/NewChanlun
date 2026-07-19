# Make the Repository Teach the Agent

Every file an agent reads influences what it predicts next. A coherent
repository teaches one local answer for each recurring concept: one
authoritative owner, one canonical pattern, and mechanically checked
projections. The agent can infer what good looks like from an example, reuse
that inference elsewhere, and spend less attention deciding which local idiom is
current.

Source code has two roles in this loop. It is implementation output downstream
of specifications, boundaries, and guardrails. It is also input to the next
trajectory. [Stop Treating Code as the Artifact] describes the first
relationship; [code as prompts for future prompts] names the second. Durable
contracts shape today’s code, and today’s code becomes tomorrow’s example.

[Stop Treating Code as the Artifact]:
  https://hyperbo.la/w/code-is-not-the-artifact/#:~:text=The%20source%20tree%20still%20matters%20because%20future%20agents%20use%20it%20as%20a%20prompt%2C%20but%20it%20is%20one%20replaceable%20realization%20of%20the%20system.
[code as prompts for future prompts]:
  https://x.com/_lopopolo/status/2030460794765451300

## Code is part of the prompt

An agent reads source to discover business logic, adapters, state-management
choices, test structure, and the way a team expresses familiar operations. In
the [Aakash Gupta interview], Ryan describes this directly: “the code in the
repository is prompts.” Uniform structure lets context learned in one file
transfer across the codebase. The agent can “more effectively cargo cult” the
way a human newcomer searches for a precedent, copies it, and adapts it.

[Aakash Gupta interview]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=actually%20the%20code%20in%20the%20repository%20is%20prompts

A concrete antecedent appears years before Ryan’s 2026 harness-engineering
essay. In [I Wrote 4,000 Lines of Code with ChatGPT in a Weekend], ChatGPT
continued to mix unrelated directives when Ryan fed a combined `lib.rs`
implementation of Ruby’s `String#unpack` into one prompt. He split the crate
into small modules with highly regular structure and documentation. Those
repeated examples let the model implement additional state-machine operations
correctly on its first attempt. The same article imagined agents grounded in the
local codebase, commit history, issues, and pull requests. Ryan later [revisited
the article] and said that although the models had improved, the collaboration
loop of context, constraints, iteration, and feedback felt the same.

[I Wrote 4,000 Lines of Code with ChatGPT in a Weekend]:
  https://hyperbo.la/w/chatgpt-4000/#:~:text=implement%20these%20parts%20of%20the%20state%20machine%20without%20issue%20on%20the%20first%20attempt
[revisited the article]: https://x.com/_lopopolo/status/2032882451538977204

The model already contains many plausible ways to build software. The repository
[prunes that latent space] by selecting the team’s intended choices.

[prunes that latent space]: https://x.com/_lopopolo/status/2051411695873216842

## Make nonfunctional requirements recoverable

Nonfunctional requirements are quality attributes, constraints, and acceptance
bars that help determine whether an outcome belongs in a system: compatibility,
maintainability, accessibility, security, reliability, performance, and polish.
Teams make often-unstated choices about how to interpret, prioritize, trade off,
and satisfy them, including risk tolerance, acceptable shortcuts, and the
organization's definition of done. They have historically carried [what doing a
good job means] through organizational structure, social norms, hiring,
onboarding, and repeated exposure to coworkers who already knew the local
answer.

[what doing a good job means]:
  https://hyperbo.la/w/what-does-it-mean-to-do-a-good-job/#:~:text=tone%2C%20taste%2C%20risk%20tolerance%2C%20how%20much%20polish%20is%20enough%2C%20what%20shortcuts%20are%20acceptable

A general model has seen many internally coherent answers to each of these
choices. An [expert persona leaves the choice unresolved] because expert
developers can make different, locally valid decisions about the same
nonfunctional requirements. A blank repository creates the same ambiguity for
[SDK design]: without local precedent, nothing prunes the many plausible
language, interface, compatibility, and long-term-maintenance choices. The model
already knows many good options; the organization must select the ones that
belong in this system.

[expert persona leaves the choice unresolved]:
  https://x.com/_lopopolo/status/2071843335749489068
[SDK design]: https://x.com/_lopopolo/status/2051411695873216842

Ryan adopted a [systems-level framing] from the \[un\]prompted conference in
which harness engineering gets the whole universe of nonfunctional requirements
into code. The [two-part context economy] places those requirements where they
can be retrieved into context and reduces how much context the worker needs to
produce high-quality work. Making them legible and retrievable becomes [a
net-new function of the team], with relevant requirements delivered just in
time. An early scheduler failure followed from unspecified requirements for
scalable, maintainable code; [writing and injecting that missing specification]
is the harness's purpose.

[systems-level framing]: https://x.com/_lopopolo/status/2028982729145237775
[two-part context economy]: https://x.com/_lopopolo/status/2030356581792293212
[a net-new function of the team]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopolo-harness-engineering/transcript.md#:~:text=make%20all%20these%20nonfunctional%20requirements%20of%20writing%20good%20software%20legible%20to%20the%20agent
[writing and injecting that missing specification]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=engineering%20had%20not%20yet%20fully%20specified%20all%20these%20non-functional%20requirements

The appropriate owner depends on the requirement's maturity and scope. A blessed
implementation carries dense choices about structure and taste. A disliked
result can be [translated into its governing nonfunctional requirement]. The
team can document its chosen interpretation, apply contextual judgment through
reviewer policy, and enforce settled invariants with tests or lints.
Architecture, types, APIs, tools, and merge gates can represent and enforce
stable requirements. Examples and reviewer policy can express qualitative
judgment. Versioned guidance and runbooks record changing operational
requirements and the current choices for satisfying them. Each representation
should point back to one semantic owner for the requirement and its local
decisions, so the agent can retrieve the relevant choice without loading the
entire quality bar into every prompt.

[translated into its governing nonfunctional requirement]:
  https://rewiz.app/channels/%40openai/build-hour-api-codex#:~:text=work%20toward%20encoding%20those%20non-functional%20requirements%20in%20your%20codebase

## Consistency compresses context

Ryan’s instruction to [make all the code the same] covers structure, patterns,
language, build, documentation, and skills. Consistency lowers context and
instruction demand. One observability stack, command shape, and package pattern
lets the agent transfer context across the repository. Each variation that
expresses the same concept forces another round of discovery: which example is
canonical, whether a migration is complete, and which surrounding rules must
also enter attention.

[make all the code the same]: https://x.com/_lopopolo/status/2030325081868837216

The [AI Native DevCon transcript] develops the attention mechanism with a
concrete contrast. One observability pattern lets context from one part of the
repository apply elsewhere; six stacks require the agent to research which one
belongs at each call site. The [AI Engineer Europe talk] connects that reduced
attention demand to completed migrations: agents can standardize the whole
repository instead of leaving two eras of code in active use.

[AI Native DevCon transcript]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=if%20I%20have%20six%20of%20observability%20stacks%20in%20the%20code%20base
[AI Engineer Europe talk]: https://www.youtube.com/watch?v=am_oeAoUhew&t=466s

This is productive cargo-culting. An [accepted implementation] embodies
thousands of small decisions that would be difficult to specify. Reusing its
good parts gives a new rollout a [nucleation point]. Canonical repetition
reduces the competing instructions, migration history, and compatibility paths
the agent must inspect and hold in attention. Humans receive the same benefit:
the repository makes its preferred continuation obvious to a new contributor.

[nucleation point]: https://x.com/_lopopolo/status/2051416715406483526
[accepted implementation]: https://x.com/_lopopolo/status/2051410617937125508

Examples can propagate defects as readily as they propagate good judgment. The
[OpenAI harness-engineering essay] observes that Codex reproduces uneven or
suboptimal local patterns. A half-finished migration therefore creates competing
prompts, and a bad helper can spread through every package that needs similar
behavior. Golden principles, shared primitives, and recurring cleanup make the
example set trustworthy again.

[OpenAI harness-engineering essay]:
  https://openai.com/index/harness-engineering/#:~:text=Codex%20replicates%20patterns%20that%20already%20exist%20in%20the%20repository

## One concept, one authoritative owner

Before creating a parser, manifest type, version constant, state representation,
fixture builder, policy check, or command, find the existing owner of the
concept. Extend that capability seam when necessary.

One concept can have several physical renderings while retaining one semantic
owner. A version may appear in source, generated documentation, a pull request,
and automation state. Generate those projections from the owner, parse them from
the canonical source, or check their relationship mechanically. Stable
policy—exact pinning, a cooling-off interval, or an approved dependency
class—remains code even as the current version changes in a manifest.

The repository owns desired configuration and durable policy. A running service,
external control plane, hardware device, or deployment system owns its live
state. Tools and checks reconcile those views without copying volatile state
into a second supposed source of truth.

Warning signs include:

- a second parser for a language the repository already models;
- exact tool versions copied into policy code and fixtures;
- JSON or YAML assembled from string fragments;
- commands that reimplement domain behavior;
- untyped external data flowing into internal code; and
- several defensive checks protecting the same invariant.

Each duplicate owner creates another locally plausible continuation. An agent
can follow every nearby precedent faithfully and still make the repository less
coherent.

## Finish migrations and install the ratchet

Consistency becomes more attainable when implementation is abundant. [The
Production Function Changed] describes enabling ESLint's `no-await-in-loop`
rule, repairing roughly 600 violations, and adding exhaustive test coverage in
one pull request. Completing the migration in one coherent change removed
contradictory precedent.

[The Production Function Changed]:
  https://hyperbo.la/w/production-function-changed/#:~:text=I%20turned%20on%20no-await-in-loop%20from%20ESLint,exhaustively%20add%20test%20coverage

Completing the migration matters as much as choosing the target. Leaving old and
new patterns side by side makes both available as prompt material. A finished
migration removes that ambiguity, and a ratchet turns the new pattern into
durable context. Agents also make upkeep cheaper: recurring gardening can find
deviations, migrate them, and feed accepted corrections back into the repository
before drift compounds.

A brownfield migration can deliberately prevent the old system from supplying
the easiest continuation. Ryan has described an exploratory pattern of
decomposing Python packages into smaller Rust packages behind PyO3 boundaries.
The language and package boundary blocks casual dependencies on the existing
ball of mud, while the Rust type system makes more of the target structure
enforceable. Ryan presents [the brownfield pattern] as exploratory evidence
about deliberate discontinuity; its value depends on the migration and the
boundary it creates.

[the brownfield pattern]: https://x.com/_lopopolo/status/2054955223609680303

The [hyperbo.la build case] applies this move across a real repository: one
toolchain and workspace graph, explicit package owners, semantic URL and
frontmatter types, one rendering path, custom structural checks, and a one-sweep
content migration. The repository becomes easier for the next agent to infer
because the migration leaves no competing build or content path.

[hyperbo.la build case]: hyperbola.md

## Parse uncertainty at the boundary

Accept uncertainty where data enters from a real external system. Parse it once
into a trusted domain type, then let internal code rely on canonical states.
Repeated record checks, normalization in every function, readiness checks in
every event handler, and several stale-response guards usually mean no boundary
owns validity.

[Parse, don’t validate] supplies the earlier type-design principle: parsing
should preserve the evidence that a value satisfies the program’s needs. Ryan’s
later harness work turns that principle into model-facing structure with
[branded identifiers], [unit-bearing duration types], [boundary-only decoding],
and AST-aware lints for escape hatches the type system cannot prevent.

[Parse, don’t validate]:
  https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/#:~:text=discharging%20checks%20on%20input%20up-front%2C%20right%20on%20the%20boundary
[branded identifiers]: https://x.com/_lopopolo/status/2076191023500611843
[unit-bearing duration types]:
  https://x.com/_lopopolo/status/2076192816477425981
[boundary-only decoding]: https://x.com/_lopopolo/status/2076200856681357613

## Encode ownership mechanically

Commands adapt arguments, environment, and exit behavior to the typed API that
owns the operation. Architecture documents name package ownership and dependency
direction. Structural tests enforce those relationships. Custom diagnostics
explain the governing model and point the agent toward the proper repair, so a
failed check supplies the missing context at the moment it is useful.

Architecture also bounds the context needed for a change. When a subsystem has a
documented, reliable interface, the agent can reason from that contract and
leave most implementation detail outside active attention. [Capability-shaped
boundaries] make the same move executable: callers depend on the capability,
while the implementation behind it can change without making every consumer load
the migration into context.

[Capability-shaped boundaries]: ../lineage/#capability-and-migration-seams

Fixtures should express domain conditions through typed builders and serialize
only at the format boundary. Generated artifacts should have one explicit
source-to-output model. Their proof may compare bytes, semantics, or a complete
corpus depending on the contract; the [proof thesis] develops that distinction.

[proof thesis]: ../proof/

The [architecture cases] develop these mechanisms in Ryan’s homelab, Artichoke,
and `rand_mt`.

[architecture cases]: implementations.md

## Preserve meaningful variation

Consistency applies when several places express the same recurring concept.
Prototype and production work have [different nonfunctional requirements], so
their patterns need not match. They are not competing answers inside one
operating contract. Canonicality is local to the relevant domain and scope.
Within that scope, one semantic owner can deliberately produce several
representations.

[different nonfunctional requirements]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopolo-harness-engineering/transcript.md#:~:text=what%20it%20means%20for%20something%20to%20be%20a%20prototype%20versus%20production%20feature

Repository-specific rules should earn their carrying cost through a concrete
failure class or a valuable invariant. Broad architecture establishes stable
seams; local implementation remains open to the agent’s reasoning. This keeps
the example set coherent without turning every design choice into centralized
policy.

The result is a reinforcing loop. Canonical examples help agents make aligned
changes with less context. Agents make complete migrations and continuous
gardening affordable. Those changes improve the examples available to every
future trajectory.
