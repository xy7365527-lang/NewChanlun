# Release Integrity

A release is trustworthy when the deployed artifact is the artifact the
repository proved, the operation uses only the authority it needs, and the
running result is verified.

## Build once

Source formatting, generation, and static checks apply to the exact source state
selected for release. CI then produces an immutable artifact from pinned,
reviewable inputs and attaches provenance that identifies its source and build.
Artifact-level tests, security checks, and browser proof exercise the produced
artifact wherever the claim depends on compilation or packaging.

The deploy job should consume the artifact in a minimal environment without a
source checkout, development toolchains, or package registries. A privileged
rebuild creates a second unproved artifact and broadens the critical supply
chain.

## Bound release authority

Use the smallest workflow permissions and credential lifetime that can complete
the operation. Separate assessment and preparation from production cutover. A
worker should request a named operation against a named resource without
receiving the credential that authorizes it. A trusted broker can resolve the
secret from secure storage at execution time, keep it in broker memory, inject
or apply it only to that operation, and return the operation's receipt and
observed postcondition. Ryan's [`gh` sidecar example] routes GitHub CLI traffic
through a proxy that holds the key material, so `gh` remains an ordinary tool to
the agent while the token stays outside model-readable context and the
repository.

[`gh` sidecar example]: https://x.com/_lopopolo/status/2061150109232939288

Consequential changes should expose explicit stages:

1. inspect current state;
2. prepare the candidate artifact and rollback material;
3. deploy to an isolated canary;
4. verify real health and access paths;
5. obtain required approval;
6. cut over;
7. verify production; and
8. retain or exercise rollback.

The agent can own every reversible stage while a person retains the authority to
approve the consequential transition.

The [authority thesis] develops identity, secret isolation, least privilege,
approval, and revocation across all agent operations.

[authority thesis]: ../authority/

## Prove the deployed boundary

An upload or successful workflow is not the user outcome. Check the hosted base
path, service health, critical browser or API journey, and any access path the
operation could destroy. Record concise evidence and enough state to diagnose or
reverse a failure.

Canaries are useful when they exercise the same binary, identity shape, network,
and health checks as production while limiting impact. A canary that tests only
installation but not reachability does not prove a remote upgrade is safe.
