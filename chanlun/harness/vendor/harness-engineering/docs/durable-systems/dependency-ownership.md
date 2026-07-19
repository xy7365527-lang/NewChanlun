# Dependency Ownership

A dependency decision chooses who owns a capability and which risks the
repository accepts. Count is a weak proxy.

## Record the capability

For each material dependency or internal replacement, record:

- the capability required and the compatibility surface it affects;
- the trust basis for the source or namespace;
- direct and transitive code, build, and network exposure;
- pinning, cooldown, vulnerability, provenance, and update policy;
- the cost of replacing or forking it; and
- the evidence that will detect a bad update or local regression.

Trust can be categorical and explicit. For example, a repository may approve
`golang.org/x/` as a high-trust namespace while requiring individual review for
other modules. Policy should preserve those distinct trust categories across
third-party code.

## Keep specialist dependencies when they reduce total risk

Cryptography, protocol stacks, Unicode, standards-heavy parsers, and other
evolving specialist domains often benefit from a concentrated upstream
community. A local rewrite can shrink a graph while increasing compatibility and
security risk.

Prefer a dependency when its capability is materially broader or more dynamic
than the repository can own well, its provenance and maintenance are credible,
and focused adapters can keep it behind a narrow seam.

## Own narrow behavior deliberately

A first-party implementation can be the lower-risk choice when the needed
behavior is small, stable, transparent, and central to the repository’s domain.
Define the supported dialect or contract before replacing the dependency.

Rebuild confidence with the evidence appropriate to the transferred burden:

- real-corpus and compatibility fixtures;
- negative and boundary cases;
- fuzz or property tests;
- coverage as a gap detector, not a compatibility claim;
- provenance and license review;
- clear package ownership; and
- an update plan for new inputs or standards.

An intentional repository-owned Markdown parser is sound when its constrained
dialect, corpus, failure behavior, and maintenance responsibility are explicit.

## Keep versions authoritative

Toolchain and package versions should have one source of truth. Policy code may
encode rules such as approved major lines, minimum patch age, or trust classes;
it should not repeat the current pin when it can parse the authoritative
manifest.

Tests should build versioned manifests from domain values. A routine bump that
requires editing many constants and hand-built strings is evidence that version
policy has not been modeled.
