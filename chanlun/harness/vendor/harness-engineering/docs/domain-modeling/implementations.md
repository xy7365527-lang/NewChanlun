# Architectures That Teach

## Ryan’s homelab: one semantic owner

Ryan’s homelab keeps current tool versions in canonical manifests. Boundary
parsers turn those files into domain values, and policy code checks durable
relationships such as exact pinning, cooling-off periods, and dependency trust
classes. A routine version change updates the manifest that owns the value. The
agent can infer the intended state from that owner without reconciling copied
constants, policy branches, and serialized fixture text.

The same pattern applies to commands and generated configuration: external
syntax enters through a typed boundary, and downstream operations use the
trusted model. [The homelab case study] develops the package boundaries, tools,
proof, and consequential operations built on top of that ownership model.

[The homelab case study]: homelab.md

## Artichoke: capabilities as context boundaries

Artichoke is a modular implementation of the Ruby programming language written
in Rust. Its original execution backend embeds mruby, a C implementation of a
Ruby virtual machine and parser. Artichoke's Rust frontend depends on
interpreter capabilities expressed as traits; the mruby backend supplies those
capabilities, while separate crates own Ruby types and standard-library
components.

Callers can reason from the stable capability contract without loading the
backend's C integration into active context. The same boundary lets the project
replace mruby behavior incrementally: new Rust implementations can take over one
capability at a time while the remaining behavior continues through the embedded
backend.

[Capability and migration seams] follows the architecture’s origin and Strangler
Fig migration. [The state-model evaluation] uses a later refactor to test the
harder question: whether a harness can help an agent preserve future attainment
while many independently merged changes move the repository toward the seam.

[Capability and migration seams]: ../lineage/#capability-and-migration-seams
[The state-model evaluation]: ../../evals/artichoke-state-modeling.md

## `rand_mt`: repository shape as operating context

[`artichoke/rand_mt`] is a small Rust crate implementing the reference MT19937
and MT19937-64 Mersenne Twister pseudorandom-number generators. Its
compatibility surface includes deterministic output for a seed, reproducibility
with Ruby's generator, a stable public API, `no_std` operation without
allocation, and an optional `rand_core` adapter that delegates to the same
implementation. These generators are not suitable for cryptographic use.

The repository keeps a short root route, a deliberately flat architecture with
those invariants, dependency policy, focused guardrails, and checked-in
automation runbooks. An agent can discover which document owns a decision and
receives a precise failure when a mechanically checked constraint is violated.
Documentation governs the remaining judgment. That combination makes the
repository’s preferred continuation easier to infer without placing every rule
in the initial prompt. The [source library] indexes the repository documents,
rollout history, and later adaptations.

[`artichoke/rand_mt`]: https://github.com/artichoke/rand_mt
[source library]: ../../sources/#projects-and-cases
