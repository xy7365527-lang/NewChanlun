# Improve One Harnessed Job

See [About the playbooks] for this procedure's origin and current status.

Use this playbook to improve one bounded, representative job under a fixed
worker. It closes an operational loop:

> baseline → earliest gap → smallest owning intervention → native verification
> → fresh rerun → retain, revise, or remove

This playbook turns the [Harness Engineering anthology] into one bounded
operational loop. A single pass can show that a job closed under the observed
conditions and can reveal the next missing boundary. Use the [Repository Review
Playbook] for broad assessment and [Evaluate the Harness] for comparative,
causal, or longitudinal claims.

[Harness Engineering anthology]: ../README.md
[Repository Review Playbook]: repository-review.md
[Evaluate the Harness]: ../evals/
[About the playbooks]: README.md

## Establish scope and authority

The context bundle is supplemental and read-only when applied to another
target. The user's requested outcome and granted authority govern; the target's
instructions, owners, contracts, and current state supply local truth.
Authorized inspection permits only the reads within its scope. It does not
authorize broader data access, egress, or mutation. Change the target only when
both the task and target authority contract grant that operation.

Do not widen permissions, redefine acceptance, weaken a grader to make a run
pass, or promote uncorroborated self-report into policy. When implementation is
outside the task's authority, stop after recording the evidence, proposed
owner, intervention, proof plan, and expected effect.

Choose one real job small enough to rerun and important enough to exercise the
suspected gap. A broad repository audit, a multi-quarter architecture program,
and an unbounded mandate to “make agents better” need narrower claims before
this loop can begin.

## Record the job contract

Write down the conditions before changing the harness:

```text
Target and revision:
Relevant external state:
Fixed model and coding-agent configuration:
Representative job:
Accepted outcome:
Evidence that proves the outcome:
Authority envelope and approval boundary:
Budget and stop conditions:
Suspected harness gap:
```

Keep target truth with its authoritative owner. The playbook may record how to
reach that truth; it should not create a parallel production corpus or private
fixture that can agree with itself while the target drifts.

## 1. Observe the baseline

Run the job through a fresh trajectory when it is safe and authorized. When a
new run would be consequential, inspect a recent collaboration whose target
state, worker, and outcome are known. Record observable evidence rather than a
summary alone:

- whether the requested outcome was accepted;
- which proof the worker produced;
- which context was available, retrieved, and relevant;
- which capabilities were discovered, invoked, and interpreted correctly;
- where a person relayed facts, tool output, decomposition, or recovery;
- retries, latency, abandoned paths, and avoidable review cycles;
- authority friction or unintended access; and
- risks and carrying costs already present in the harness.

Separate a missing capability from poor discovery, a missing fact from poor
routing, and a worker limitation from an environmental gap. The baseline is a
trajectory through a particular system, not a verdict on the model in general.

## 2. Locate the earliest failed handoff

Find the first point where the trajectory lacked what the job required. Trace
the symptom upstream until one authoritative owner can shape future runs.
Classify the gap to select the relevant route:

- **Context:** the information was absent, stale, overloaded, or never
  retrieved.
- **Capability:** the operation was unavailable or failed in discovery,
  selection, invocation, interpretation, repair, or real-system verification.
- **Domain ownership:** several plausible representations competed, or no type,
  API, state machine, or canonical source owned the invariant.
- **Authority:** capability and permission were conflated, excessive, or missing
  an approval, audit, or recovery boundary.
- **Proof:** checks established an internal proxy while the user or operational
  claim remained untested.
- **Feedback or delivery:** an accepted lesson, artifact identity, release step,
  or maintenance condition failed to survive the trajectory.
- **Worker limitation:** the necessary behavior remains unreliable even when
  the environment makes the job legible and operable.

One failed trajectory cannot establish a worker limitation. Leave it as an open
candidate until comparable repeated evidence separates it from stochastic
variance and unresolved environmental gaps.

Use [`AGENTS.md`] to retrieve one thesis only when target-local evidence leaves
the owning decision unresolved. Do not import another repository's layout,
fixtures, policies, or version pins as a substitute for local ownership.

[`AGENTS.md`]: ../AGENTS.md

## 3. State one intervention hypothesis

Name the smallest reversible change at the earliest owner. State its expected
behavior before implementation:

```text
If <intervention> is added at <authoritative owner>, then the fixed worker will
<observable behavioral change> on <representative job>, because <mechanism>.

Evidence that would support this:
Evidence that would weaken this:
Expected carrying cost and owner:
```

Prefer an intervention that removes human relay or makes an invariant legible
at its source. Examples include a shorter root route, a canonical example, a
typed boundary, an actionable diagnostic, a domain tool, an authority gate, a
real-system test, or a runbook with explicit state and recovery. New machinery
should earn its maintenance cost against the observed failure.

## 4. Implement and verify at the claim boundary

Confirm mutation authority, then make the intervention through the target's
normal workflow. Keep the diff narrow enough that the behavioral hypothesis
remains intelligible. Verify both layers:

1. Run target-native checks that protect internal contracts.
2. Exercise the user or operational journey that establishes the accepted
   outcome.

Collect the evidence the worker should produce during ordinary operation. An
evaluator-only check can diagnose the result, but it cannot substitute for
proof the worker was expected to attach. Remove a downstream control only when
the new upstream owner makes it redundant and the target's evidence supports
the removal.

## 5. Run a fresh trajectory

Rerun the same class of job with the same model, coding-agent configuration,
authority envelope, and materially equivalent external state. Use a fresh
session and an isolated, equivalent starting state that preserves unrelated
work, so the rerun does not inherit hidden help from the implementation
conversation.

Confirm that the intervention was available, retrieved or invoked, and relevant
to the rerun. A successful outcome supplies no evidence about an instruction or
tool the trajectory never used.

Compare the rerun with the baseline across separate dimensions:

- accepted outcome and claim-matched proof;
- human relay, steering, and review convergence;
- elapsed time and retries;
- authority and recovery behavior;
- new failure modes or displaced complexity; and
- ongoing maintenance and latency cost.

## 6. Retain, revise, or remove

Retain the intervention provisionally when the fresh run closes the bounded
job, the intervention was actually retrieved or invoked, the evidence is
consistent with the hypothesized mechanism, and the apparent gain justifies its
carrying cost. Revise it when the gap was correctly located but the interface
remains hard to retrieve or use. When target-local evidence may already supply
the behavior, temporarily withhold the intervention and rerun before deciding.
Remove it when it adds noise, duplicates a better owner, or repeatedly fails to
improve the job.

Record the decision where the target keeps architecture or operational history.
Name the owner, the evidence, any follow-up case, and the condition under which
the intervention should be reconsidered or retired. The durable result is the
improved environment and its evidence, not the existence of another harness
component.

## Preserve a compact result record

```text
Job and accepted outcome:
Target revision and external state:
Fixed worker and authority envelope:

Baseline evidence:
Earliest failed handoff and owner:
Intervention and expected mechanism:
Verification performed:
Fresh-rerun evidence:

Outcome comparison:
Proof comparison:
Human-relay and latency comparison:
Risk and carrying-cost comparison:
Test-without evidence, if run:

Decision: retain | revise | remove
Durable owner and recorded lesson:
Follow-up or retirement condition:
Known limits:
```

This record is evidence about one operational loop. It supports a bounded claim
about the job and conditions observed. One before-and-after run does not
isolate the intervention's mechanism or estimate a general treatment effect or
stochastic variance.

Use the [Repository Review Playbook] when several journeys or ownership
boundaries need diagnosis. Use [Evaluate the Harness] when the decision requires
repeated conditions, environment parity, condition-blind grading, or evidence
that the intervention generalizes.
