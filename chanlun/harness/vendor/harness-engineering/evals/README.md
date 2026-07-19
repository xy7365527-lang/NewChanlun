# Evaluate the Harness

A harness earns its carrying cost when the same worker produces better accepted
outcomes in the environment where the work matters. Evaluate the complete
system: a selected model and coding agent operating through context, tools,
authority, feedback, proof, and delivery. Ryan argues that code-only benchmarks
produce [machines that produce code], while [one rollout is not representative]
of long-lived work.

[machines that produce code]: https://x.com/_lopopolo/status/2054960622140326312
[one rollout is not representative]:
  https://x.com/_lopopolo/status/2054969017140752758

Use [Improve One Harnessed Job] for a bounded operational before-and-after. This
guide owns comparisons intended to estimate an intervention's effect across
conditions or over time.

[Improve One Harnessed Job]: ../playbooks/improve-harness.md

## Start with the decision

Name the harness change and the outcome it is expected to improve. A useful
evaluation isolates a decision such as:

- whether a context route helps the worker recover a local invariant;
- whether a domain tool makes a previously inaccessible job possible;
- whether a runbook reduces human relay at an authority boundary;
- whether a type, API, test, or lint prevents a known failure class; or
- whether domain-level proof lets a user accept an artifact without reviewing
  its intermediate code.

Choose a task that genuinely exercises the intervention. A worker that resolves
the job from target-local evidence has not failed to use supplemental context. A
tool cannot improve a job that never needs its capability. Record the governing
decision before the run so the evaluation does not reward terminology or file
access detached from the outcome.

Before collecting results, write down:

- the accepted outcome and the evidence that establishes it;
- the target revision and external state;
- the fixed model, coding-agent host, and native action interface;
- the baseline instructions and tool surface;
- the authority envelope, budget, and stop conditions;
- the exact context or tool intervention that differs between conditions;
- the expected behavioral effect; and
- the result that would weaken the hypothesis.

## Hold the comparison steady

Give baseline and treatment the same target, task, authority, external state,
and budget. Hold every instruction and tool outside the named intervention
steady. Use fresh sessions and target states, randomize run order, and repeat
each condition enough to expose variance. Start a new evaluation epoch when the
model or coding agent changes materially.

[Hold the Worker Constant] develops why a worker change reopens assumptions
about ambition, instruction following, context use, and tool semantics.

[Hold the Worker Constant]: ../docs/fixed-worker/

Record whether the intervention was available, retrieved, invoked, and relevant
as separate facts. An offered context bundle estimates the effect of making
context available. It estimates the effect of read context only when the worker
actually retrieves it. A callable tool likewise supplies no evidence about tool
quality when the trajectory never discovers or invokes it.

When target-local examples resolve both conditions, the comparison estimates no
added value on that task. It cannot support a broader claim that the
intervention never helps.

Environment parity needs direct evidence. Record effective write access,
available executables, credentials, network reachability, and consequential
permissions for every run. A comparison cannot attribute outcome differences to
the harness when one condition could not edit the target or execute its checks.

## Keep truth with the target

Target-owned corpora, current records, and operational systems remain with their
authoritative owner and are equally accessible in every condition. Exercise
source-to-output parity in the target repository. The evaluator carries no
parallel production corpus.

Bounded fixtures may represent a named edge condition. Their scope should be
small enough that a reviewer can see which behavior they test. Copying the full
external corpus into the evaluator creates a shadow authority that can drift
while still agreeing with its own expected output.

Held-out tasks, expected results, and grader code stay outside the
worker-visible surface. Keep those materials independent of the target data the
worker legitimately needs to do and prove the job. A hidden evaluator result can
grade an outcome; it cannot substitute for proof the worker was expected to
produce.

## Grade the contract

Each evaluation owns an outcome boundary. Prefer a grader that observes ground
truth at that boundary and accepts equivalent implementations.

| Result                     | Question                                                                 |
| -------------------------- | ------------------------------------------------------------------------ |
| Outcome                    | Does the delivered artifact satisfy the case-specific behavioral claim?  |
| Proof                      | Does the worker’s evidence establish that claim in the real environment? |
| Architecture and ownership | Does the change preserve the governing invariant and maintenance model?  |
| Trajectory diagnostics     | What latency, retries, tools, tokens, and human attention did it use?    |

Outcome, proof, and architecture determine acceptance. Trajectory diagnostics
explain cost and failure. A fluent explanation cannot repair an incorrect
artifact, and evaluator-only success cannot repair missing worker-produced
proof.

Exact filenames, helper names, statement order, serialized whitespace, and
private fixture layouts are valid requirements only when the product contract
makes them observable. Use condition-blind review for qualitative architecture
and maintenance judgment. Preserve the rubric and every run, including failures.

## Measure whether judgment compounds

The central longitudinal question is whether later work inherits useful judgment
from earlier work. After promoting a lesson into context, an example, a type, a
tool, or an executable control, run fresh tasks from the same failure class.
Measure recurrence, human relay, repair cost, proof quality, and the carrying
cost of the intervention. Ablate context or tooling whose effect does not
survive comparable runs.

Long migrations need checkpoints that ask whether today’s accepted work makes
the eventual destination easier and safer to attain. [Future Regret in
Artichoke’s State Refactor] follows a failed core state-model change, the
preparatory pull requests it produced, later integration attempts, and lifetime
failures. That public history supports continuation tasks and maintenance
questions with a horizon appropriate to claims about years of coherence.

[Future Regret in Artichoke’s State Refactor]: artichoke-state-modeling.md

The public target and its history make this a retrospective calibration case.
Its known route cannot support a general capability claim from a worker that can
read this bundle. Capability measurement needs a fresh private target or a
materially transformed fixture whose route is absent from worker-visible
context.

The evaluation horizon should match the claim. A deterministic rule can be
tested against fresh examples immediately. Architectural coherence, future
regret, and maintenance ownership require repeated work across changing
repository states. These remain empirical outcomes even when individual
invariants have strong executable enforcement.

## Recognize an invalid result

An evaluation cannot support its intended claim when:

- the claim concerns consumed context or exercised capability, but the treatment
  was never retrieved or invoked;
- an instruction, tool, authority, external state, or write-access difference
  appears outside the predeclared intervention;
- one stochastic rollout is treated as representative;
- the grader requires an undisclosed reference implementation;
- evaluator-only checks are reported as worker-produced proof;
- a copied corpus lets the evaluator agree with its own shadow authority;
- token count, line count, or activity stands in for an accepted outcome;
- the worker changes inside a comparison; or
- a public report names opaque targets and revisions that readers cannot inspect
  or reproduce.

An internal trial may still guide local iteration after one of these failures.
Its public claim should stop at the evidence it actually preserves. A sharable
report needs an inspectable target, a runnable fixture generator, or enough
artifacts and method for another reader to audit the result.

## Feed results back into the harness

Cluster failures by missing context, poor routing, unavailable capability,
unclear authority, weak proof, domain-modeling error, or worker limitation.
Improve the smallest owning artifact, then rerun a fresh comparison. Add a
held-out case for a repeated failure class and retire controls whose effect or
value no longer covers their maintenance cost.

[Turn Feedback Into Infrastructure] describes how observed trajectories become
durable improvements without turning raw session archives into permanent
context.

[Turn Feedback Into Infrastructure]: ../docs/feedback/
