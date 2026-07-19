# Repository Review Playbook

See [About the playbooks] for this procedure's origin and current status.

Review a repository by asking whether a capable coding agent can recover the
real job, make a coherent change, prove the outcome, and operate safely without
using a person as a relay. Representative whole journeys supply evidence for
the jobs claimed.

This playbook is a broad diagnostic. Use [Improve One Harnessed Job] to close a
single authorized intervention loop and [Evaluate the Harness] when making a
comparative or longitudinal claim. Authorized inspection permits only the reads
within its scope; it does not itself authorize broader access, egress, or
changes to the target.

[Improve One Harnessed Job]: improve-harness.md
[Evaluate the Harness]: ../evals/
[About the playbooks]: README.md

Hold the chosen model-and-coding-agent configuration constant during the review.
Assess the context, tools, repository structure, execution environment,
permissions, and feedback loops around that worker. Follow representative
journeys to determine whether the selected worker can complete the job through
those surfaces.

## Start with the outcome

The review should answer:

- Which complete jobs can the agent perform today?
- Where does it need hidden human knowledge, tool relay, or authority?
- Which checks establish the claimed behavior, and which merely establish
  internal consistency?
- Where has defensive machinery accumulated around a missing domain model?
- Which dependency and release decisions carry unowned risk?
- What is the smallest change that would improve the next many trajectories?

Keep these answers discrete. A repository with many controls can be harder to
maintain and less safe than one with a few well-owned boundaries. A single
release, credential, compatibility, or data-loss defect can dominate an
otherwise strong system.

## Define the review contract

Record the target revision and relevant external state, the fixed model and
coding-agent configuration, the jobs being claimed, their accepted outcomes,
the proof required at each boundary, the review's authority and access, and its
known exclusions. Name the stop conditions for consequential operations.

The review may diagnose a missing owner or propose a change outside its mutation
authority. Keep that finding explicit rather than treating inspection access as
permission to implement it.

## Inspect the work as a trajectory

Choose representative tasks and follow them from request to outcome:

1. How does the agent classify the task?
2. Which root guidance routes it to the relevant domain context?
3. Can it find the existing owner of the concept it needs to change?
4. Can it reproduce and observe the behavior without a human intermediary?
5. Does the implementation preserve one domain model and one source of truth?
6. Does the proof exercise the claim at the right boundary?
7. Can the agent handle review, CI, conflicts, and delivery?
8. Does it know which operation requires human judgment or new authority?
9. Does the result leave a durable improvement for the next run?

Static inspection is necessary but insufficient. Run at least one real journey
through the configured worker when the review claims end-to-end operability.
[Give One Agent the Whole Job] develops ownership of decomposition, execution,
integration, proof, review, delivery, and lifecycle closure.

[Give One Agent the Whole Job]: ../docs/whole-job/

## Recover evidence from prior collaborations

Review locally available session logs from work the user and agent performed
together. They preserve evidence that a repository snapshot cannot: which
requests required repeated steering, which formulations the user rejected, what
the agent misunderstood, where a person acted as a tool, which proof changed the
user's confidence, and which lessons never reached a durable owner.

Treat those logs as trajectory evidence rather than repository policy. Connect
each observation to the resulting diff, review, test, deployment, or user
outcome before drawing a conclusion. Look for recurring corrections and for
cases where the repository still permits a failure the user already taught an
agent to avoid. A durable repair moves the lesson into the smallest owner that
can shape future work: a type, API, test, lint, runbook, architecture rule,
example, or routing document.

Local logs may contain private source material, credentials, personal data,
unpublished decisions, or context intended only for one collaboration. Read only
logs within the review's authority. Keep raw logs in their existing storage,
record any access limitation, and publish only the bounded synthesis and
evidence the destination audience is allowed to receive. Do not embed the
session corpus in the reviewed repository.

## Review the ownership boundaries

### Context and routing

Look for a short root map that says what the repository is, why its
compatibility surfaces matter, which operating loop applies, and where each task
class should retrieve deeper guidance. Check that important knowledge is
versioned beside the work rather than trapped in chat, private memory, or one
automation instance. Test the retrieval hierarchy described in [Route Context
Just in Time]: can the agent find the relevant architecture, dependency posture,
generation contract, or runbook without preloading the whole corpus?

[Route Context Just in Time]: ../docs/just-in-time-context/

### Capabilities and tools

Follow each required capability through discovery, selection, invocation,
interpretation, repair, and real-system verification. A tool that exists but is
hard to find or whose response does not guide the next action leaves the person
as a relay. Check whether the agent can recover from malformed inputs, partial
results, timeouts, and unavailable operations without guessing at hidden state.

[Make Capabilities Legible and Operable] develops this complete tool loop.

[Make Capabilities Legible and Operable]: ../docs/tool-legibility/

### Domain and architecture

Use the canonical ownership and typed boundaries in [Make the Repository Teach
the Agent] to trace concepts that appear in several layers: versions, workflow
manifests, identifiers, generated records, UI states, release artifacts, and
dependency policy. Each should have one authoritative representation and an
obvious owner.

[Make the Repository Teach the Agent]: ../docs/domain-modeling/

Treat these as warning signs:

- competing semantic owners for one language or concept;
- one fact copied across manifests, policy, fixtures, and tests;
- fixtures coupled to incidental representation instead of domain conditions;
- untrusted values flowing beyond the boundary that decodes them;
- state transitions represented by overlapping defensive checks; and
- adapters that reimplement domain behavior instead of exposing its owner.

Repeated defensive checks can look rigorous while making the system less
coherent. Ask what type, boundary, state machine, or source of truth would make
the invalid condition unrepresentable.

### Execution and proof

Apply the claim-matched evidence in [Prove the Outcome in the Real Environment].
Verify that the agent can launch and inspect the real system. Depending on the
claim, useful evidence may include browser state, logs, metrics, traces,
screenshots, generated-corpus diffs, fuzz results, compatibility vectors, canary
health, or rollback verification.

[Prove the Outcome in the Real Environment]: ../docs/proof/

Map each green test to its assertion, then collect separate evidence at the user
boundary. Distinguish:

- generated freshness from source-to-output parity;
- unit behavior from a working browser journey;
- an uploaded artifact from a healthy deployment;
- coverage percentage from standards compatibility;
- a vulnerability-shaped pattern from a proven impact; and
- policy self-consistency from policy being sourced correctly.

### Feedback and guardrails

Use [Turn Feedback Into Infrastructure] to inspect whether review comments,
failed trajectories, incidents, and build failures improve the environment.
Stable lessons may become domain types, preferred APIs, lints, tests, runbooks,
or routing docs. The control should explain the violated principle and point
toward repair.

[Turn Feedback Into Infrastructure]: ../docs/feedback/

Evaluate enforcement by the failure class it prevents and the carrying cost it
adds. Look for duplicated validation, rules that encode taste before it
stabilizes, reviewers without a convergence policy, and prose that repeats
executable truth. Remove a control when a better upstream model makes it
redundant.

### Dependencies and supply chain

Review dependencies as the capability and lifetime-ownership decisions developed
in [Own Dependency Risk]. For each material dependency or repo-owned
replacement, ask:

[Own Dependency Risk]: ../docs/durable-systems/dependency-ownership.md

- What capability does it provide?
- Why is this source trusted?
- What transitive and update risk does it add?
- If it was removed, which compatibility and security obligations moved into the
  repository?
- What corpus, fuzzing, conformance, provenance, and maintenance plan rebuild
  confidence?

Evaluate each dependency against the target repository's documented trust
posture.

### Delivery and authority

Follow the exact artifact from validation to release. Prefer deploying the
immutable artifact CI proved rather than rebuilding it in a more privileged job.
Inspect workflow permissions, credential persistence, third-party action pins,
network execution, canary boundaries, approval gates, rollback, and post-deploy
checks.

Use [Maximize Autonomy Inside Explicit Authority] to separate capability from
authority. An agent may prepare a consequential operation completely while a
person retains approval for cutover. Secrets the worker does not need should not
enter its readable environment.

[Maximize Autonomy Inside Explicit Authority]: ../docs/authority/

### Proportionality

Use [Optimize for Measured Effectiveness] to account for each harness
component's carrying cost. Ask which observed failure or risk it addresses,
whether another control already owns that invariant, how long the normal loop
takes, and who will maintain the component. Prefer the smallest system that
covers the repository’s actual failure modes.

[Optimize for Measured Effectiveness]: ../docs/effectiveness/

## Findings

Lead with actionable findings ordered by consequence. Each finding should name:

- the behavior or invariant at risk;
- concrete evidence and the owning boundary;
- why existing proof did not catch it;
- the root correction at the owning boundary; and
- any redundant machinery that should be removed after the correction.

Then record strong foundations worth preserving, the journeys actually run, and
known limits of the review.

## After the review

When the task scope and target authority contract authorize changes, choose one
consequence-ordered finding and enter [Improve One Harnessed Job]. Preserve its
baseline evidence, expected behavioral effect, smallest owning intervention,
proof plan, carrying cost, and fresh-rerun criterion. Without mutation
authority, deliver the findings and stop at the proposed intervention.

A successful repair to one journey establishes an operational result under the
observed conditions. Route claims of general effect to [Evaluate the Harness].

## Readiness blockers

Treat the repository as not ready for the claimed job when evidence shows:

- silent data loss or semantic migration regression;
- a release path that can bypass the validated artifact;
- credentials or untrusted execution with material unintended authority;
- a consequential operation without an appropriate gate or recovery path;
- an owned compatibility or security surface without a credible confidence
  model;
- documentation that materially overstates enforced guarantees; or
- no way to prove the user-visible or operational outcome being claimed.

Scale the harness to the claim. Each blocker requires narrowing the claim or
building the missing boundary.
