# Harness Engineering

This guide is for agents consuming the corpus. Use the repository as shared
context for applying Ryan Lopopolo's harness-engineering work to another task,
system, or repository. When this repository is supplied as context for another
target, treat it as read-only. When a task explicitly targets this repository,
follow [`docs/AGENTS.md`](docs/AGENTS.md) for public prose and
[`sources/AGENTS.md`](sources/AGENTS.md) for source material. Follow
[`playbooks/AGENTS.md`](playbooks/AGENTS.md) when using or editing a playbook.

## Application routing

- Read [`playbooks/README.md`](playbooks/README.md) before selecting a
  repository-authored procedure.
- To learn or teach the practice, read [`README.md`](README.md) and the thesis
  index in [`docs/README.md`](docs/README.md).
- To improve the harness around one observed job, read
  [`playbooks/improve-harness.md`](playbooks/improve-harness.md) after the
  target's instructions and before collecting a baseline or making a change.
  Use thesis routing inside its gap-classification step.
- To assess a repository broadly, read
  [`playbooks/repository-review.md`](playbooks/repository-review.md) before
  beginning the review.
- To make a comparative, causal, or longitudinal claim, read
  [`evals/README.md`](evals/README.md) before running conditions.

For other target-specific work, do not preload application guides or the thesis
corpus; use the unresolved-decision routing below.

## Working loop

1. Read the target's available instructions and architecture first. For a
   repository, begin with its agent guide.
2. Inspect the target's actual domain model, manifests, tools, tests,
   permissions, operating history, and neighboring implementations. Find the
   local owner or precedent for the concern before retrieving general guidance.
3. Name the governing decision that local evidence leaves unresolved. Continue
   without a thesis when no unresolved decision remains. Otherwise, choose one
   primary route under [Context routing](#context-routing) and read that thesis
   `README.md`. Add another route only for a distinct unresolved concern. Follow
   a case or source link only when its evidence can change the decision.
4. Let target-local truth govern the implementation. The context bundle can
   sharpen a decision; it cannot override the target's contracts or authority.
5. Adapt the applicable ideas to the target without copying file layouts,
   policies, version pins, or fixtures.
6. Verify the result through the target system's native checks and observable
   user journeys.

## Context routing

- Re-evaluating capability, ambition, task-size priors, instruction following,
  or tool semantics after choosing or upgrading the worker:
  [`docs/fixed-worker/`](docs/fixed-worker/)
- Deciding which private, changing operating knowledge the worker needs, how it
  should retrieve that knowledge, or how internal code and tool use should
  produce a domain outcome for a user who never sees the code:
  [`docs/last-mile-deployment/`](docs/last-mile-deployment/)
- Defining the outcome boundary, ownership of the whole job, use of subagents,
  or lifecycle closure: [`docs/whole-job/`](docs/whole-job/)
- Deciding whether missing or overloaded context calls for progressive
  disclosure, a skill, a runbook, or a goal-specific context-curator sidecar
  with a trusted publication boundary:
  [`docs/just-in-time-context/`](docs/just-in-time-context/)
- Deciding whether the worker lacks a capability or whether an existing tool is
  hard to discover, invoke, or interpret:
  [`docs/tool-legibility/`](docs/tool-legibility/)
- Deciding where local nonfunctional requirements should live as context,
  examples, types, tests, or policy; where external data becomes canonical
  repository-owned state; or where repeated interpretation indicates missing
  semantic ownership: [`docs/domain-modeling/`](docs/domain-modeling/)
- Defining who may perform a consequential mutation, under which scope,
  approval, credential, audit, and recovery contract, including alternate
  mutation paths: [`docs/authority/`](docs/authority/)
- Defining the evidence boundary for a user or operational claim, including the
  identity of an artifact across build, test, and release:
  [`docs/proof/`](docs/proof/)
- Diagnosing an observed trajectory, recovering a golden principle, or
  garbage-collecting drift from the governed artifact and its harness:
  [`docs/feedback/`](docs/feedback/)
- Choosing for future coherence, dependency ownership, lifetime risk, migration
  completion, or proportionality:
  [`docs/durable-systems/`](docs/durable-systems/)
- Turning a recurring process into a durable loop with owned state,
  distillation, and gardening:
  [`docs/continuous-maintenance/`](docs/continuous-maintenance/)
- Evaluating human attention, outcome effectiveness, utilization, or economics:
  [`docs/effectiveness/`](docs/effectiveness/)

The unresolved decision selects the route. Target-local evidence may resolve it
without a thesis. When context is needed, start with one thesis and add a second
only while a separate decision remains open.

For public-post evidence, use
[`sources/twitter/README.md`](sources/twitter/README.md) as the human index and
query the adjacent JSON corpus for captured text and canonical links. Use
[`sources/sources.json`](sources/sources.json) for source provenance.

When the task needs Ryan's full argument or original language, follow the local
text links in the [source library](sources/README.md). The repository preserves
his hyperbo.la articles so an agent does not depend on web access to read them.

For attribution, chronology, or related-work questions, consult
[`docs/lineage/`](docs/lineage/) and the source manifest.
