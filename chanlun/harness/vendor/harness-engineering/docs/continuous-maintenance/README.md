# Run Known Work as a Continuous Loop

A large class of software work already has an agreed direction. Localize the
product for a known user group. Fill instrumentation gaps. Keep dependencies
current. Remove obsolete compatibility paths. Repair drift from architectural
rules. Refresh generated data and documentation. Once the desired outcome,
proof, and authority are legible, an agent can keep this work moving without
waiting for a roadmap slot.

[Software Work Is No Longer Scheduled] develops this shift through work that
organizations routinely postpone: Japanese support for internal users,
instrumentation, dashboards, security hardening, performance work, and small
user-experience repairs. These changes compete poorly for a quarter’s planning
capacity even when everyone agrees they should happen. When their desired
state, interfaces, guardrails, and acceptance procedure are legible, agents can
execute them asynchronously and return evidence for review.

[Software Work Is No Longer Scheduled]:
  https://hyperbo.la/w/software-work-not-scheduled/#:~:text=work%20does%20not%20need%20to%20wait%20for%20a%20sprint%20slot

Continuous maintenance begins from an accepted direction. A viable loop can
answer five questions:

- What condition should remain true?
- What signal shows that the current system has departed from it?
- What evidence proves that a proposed change restores it?
- Which operations may proceed autonomously, and where is approval required?
- Which durable state records the result for the next iteration?

[Code Reds Need Maintenance Loops] treats an emergency as an adaptive episode
that makes the loop’s contract discoverable. During the response, people learn
which invariant matters, which signals predict drift, which interventions work,
and which judgment calls matter. A green metric marks recovery. Durable
completion also records the goal, sensors, authorized actions, escalation path,
and owner of the loop that will keep the invariant healthy.

[Code Reds Need Maintenance Loops]:
  https://hyperbo.la/w/code-reds-need-maintenance-loops/#:~:text=That%20understanding%20should%20become%20a%20maintenance%20loop%20the%20team%20leaves%20behind

That durable loop changes how the response is allowed to end. Agent execution
can make ongoing ownership cheap enough for some newly discovered invariants to
remain maintained without an indefinitely staffed team. [A goal, a harness, and
a review loop] carry the operating model after the tiger team disbands.

[A goal, a harness, and a review loop]:
  https://hyperbo.la/w/code-reds-need-maintenance-loops/#:~:text=The%20work%20that%20used%20to%20require%20an%20indefinitely%20staffed%20team%20can%20sometimes%20become%20a%20goal%2C%20a%20harness%2C%20and%20a%20review%20loop

That continuing responsibility is a goal rather than a task. The code-red
article uses [Codex’s `/goal` primitive] for a persistent objective that
discovers and spawns tasks across runs. “Fix this flaky test” can end with one
patch. “Keep p95 CI latency under N minutes” keeps observing the system,
generating and testing interventions, and requesting human authority when
necessary. Durable state connects those tasks. An explicit retirement condition
closes the loop when the standing mandate or maintained capability becomes
obsolete.

[Codex’s `/goal` primitive]:
  https://hyperbo.la/w/code-reds-need-maintenance-loops/#:~:text=A%20goal%20is%20a%20process%20for%20doing%20and%20discovering%20work%20which%20itself%20spawns%20tasks

A missing answer changes the immediate outcome to investigation, a proposal, or
an escalation. Product invention remains foreground work when the user need,
interface, architecture, or acceptance boundary is still unresolved. The
boundary follows how settled the desired outcome is. A six-month migration can
be known work; a three-line product change can still require product judgment.

The [OpenAI Symphony article] describes the same division at team scale: routine
implementation can flow through an always-on supervisor, while ambiguous work
and decisions requiring strong judgment remain active engineering work. The
[fixed-worker thesis] develops how a model or agent change reopens assumptions
about which outcomes can safely enter a loop.

[OpenAI Symphony article]:
  https://openai.com/index/open-source-codex-orchestration-symphony/#:~:text=Symphony%20can%20handle%20the%20bulk%20of%20routine%20implementation%20work
[fixed-worker thesis]: ../fixed-worker/

## Give the loop a repository-owned contract

Cron, event triggers, and tail calls—a completed run invoking or scheduling its
successor—wake the worker. A checked-in runbook supplies the job: intent,
current source of truth, candidate-selection rules, scope, proof, authority,
escalation, rollback, and state for the next run. Ryan Lopopolo calls these
trigger mechanisms [loop engineering]. The scheduler can stay small because the
repository carries the durable operating knowledge.

[loop engineering]: https://x.com/_lopopolo/status/2068882715399926026

A useful maintenance runbook tells the agent:

- which external and repository state to inspect;
- how to decide whether work is needed;
- which changes fall inside the standing mandate;
- which checks establish a safe candidate;
- which mutations require human approval;
- how to prove the real outcome;
- what to record when the run changes nothing;
- how to recover after partial progress; and
- when the runbook or the loop itself should be retired.

This arrangement makes maintenance policy reviewable, versioned, and available
to every future trajectory. Ryan’s [repository maintenance layout] pairs a
routing-oriented `AGENTS.md` with human onboarding, pinned toolchains,
dependency-update runbooks, and guardrails distilled from repository history.
The automation points into that knowledge base and executes the current
contract.

[repository maintenance layout]:
  https://x.com/_lopopolo/status/2060767456302428343

[Enabling Codex to Upgrade My Robot Vacuum] shows the arrangement under real
operational risk. Loki is a robot vacuum running the open-source Valetudo
firmware. A custom Tailscale daemon is its remote-access path. A weekly
automation inspects Tailscale releases and upstream build-tag behavior, compares
the current and candidate feature sets, builds a `linux/arm64` candidate, and
prepares a pull request. Its prompt is a short pointer into checked-in
documentation. The repository owns the complete assessment contract.

[Enabling Codex to Upgrade My Robot Vacuum]:
  https://hyperbo.la/w/robot-vacuum-canary-tailscale/#:~:text=The%20repo%20carries%20the%20durable%20instructions

The same contract narrows the automation’s authority. The scheduled run can
research, build, and propose. A separate deploy runbook stages the candidate on
the same vacuum under a second Tailscale identity and state directory. That
canary must boot, join the tailnet, and accept SSH before production changes.
Two human approvals sit at consequential boundaries: authenticating the new
identity, then authorizing replacement of the production access path after a
fresh backup.

## Carry a current world model across runs

A continuous loop acts across many agent trajectories. It needs durable state
for what the last run observed, which candidates it rejected and why, which
change is active, what proof completed, and which decision still needs a person.
That state may live in repository history, an issue tracker, an execution plan,
automation memory, or the external system that owns the live condition. Each
owner should remain clear.

[openai/symphony:SPEC.md] demonstrates the coordination layer. Symphony
continuously reads eligible work from an issue tracker, creates one workspace
per issue, respects dependencies, retries interrupted work, reconciles tracker
changes, and exposes operator-visible status. Repository-owned workflow policy
tells the coding agent how to perform the work; the tracker records which
outcomes are available, blocked, under review, or complete.

[openai/symphony:SPEC.md]: https://github.com/openai/symphony/blob/main/SPEC.md

The world model also has to notice change outside the repository. A request
asked whether [Canonical Multipass], a tool for launching local Ubuntu virtual
machines on macOS, Windows, and Linux, could become a backend in an agent
orchestrator and give coding agents isolated machines to work in. Ryan proposed
broadening that single suggestion into [research across the credible VM-runner
class]: ask Codex to find high-quality, well-maintained, trusted VM runners that
provide the same capability and add backends for suitable candidates.

[Canonical Multipass]: https://x.com/_lopopolo/status/2061613371883954430
[research across the credible VM-runner class]:
  https://x.com/_lopopolo/status/2062220470057898417

He then proposed a [persistent VM-runner agent] that would keep the class
current as its members change: search for new candidates, add useful backends,
and remove runners that have become obsolete or insecure. In this operating
model, the desired capability remains stable while its implementations change,
so there is no final runner list. Durable state records the latest
classification and evidence; retirement follows the capability or standing
mandate becoming obsolete.

[persistent VM-runner agent]: https://x.com/_lopopolo/status/2062222490483474772

A finite campaign carries a known transformation across many runs. Ryan set a
[daily relicensing loop] to move a Rust crate from the MIT license to the dual
MIT OR Apache-2.0 license and keep running toward repository-wide completion. An
API owner could likewise [migrate dependents] to a settled replacement contract,
including consumers across service and organization boundaries. The campaign
needs dependency-aware batches, an independently useful result from each run, a
ratchet against regression, a measurable completion condition, and retirement of
the temporary migration machinery.

[daily relicensing loop]: https://x.com/_lopopolo/status/2056958332464984429
[migrate dependents]: https://x.com/_lopopolo/status/2049591048759116267

When the target interface or architecture is still unresolved, the loop’s
deliverable is evidence, a proposal, or an escalation for the person choosing
the next step.

A settled discovery contract can govern findings whose importance remains
unresolved. A latent code red is a severe, deteriorating condition that would
justify emergency attention once recognized but has not yet been identified or
declared. A [process for discovering latent code reds] can inspect named
metrics, data sources, and weak signals, assemble a decision-ready candidate,
and record which false positives waste attention. This reduces dependence on who
happens to notice a problem, can narrate it convincingly, has standing to
escalate, and reaches the right executive. A person still decides whether the
candidate deserves a maintenance goal and which invariant the resulting loop
should own.

[process for discovering latent code reds]:
  https://hyperbo.la/w/code-reds-need-maintenance-loops/#:~:text=An%20agentic%20system%20reduces%20dependence%20on%20executive-attention%20topology%20from%20the%20discovery%20process

## Make every iteration prove and record its outcome

A maintenance loop should finish in one of a small number of durable states:
nothing needed, candidate proposed, change proved, approval requested, recovery
required, or policy obsolete. Quiet no-op runs are healthy. Repeated rediscovery
of the same facts signals missing state.

Proof has to match the maintained condition. A successful compile establishes a
candidate binary. A canary establishes that the binary can boot, join its
Tailscale private network, and accept SSH. Production logs and an SSH check
establish that the cutover preserved the access path. The robot-vacuum runbook
records those different claims separately and keeps the old binary available for
recovery.

The same discipline applies to lower-consequence maintenance. A runner-image
loop must preserve both properties in its policy: images stay [pinned and up to
date]. A documentation loop checks whether text still describes real behavior. A
refactoring loop scans for a named class of divergence, migrates every instance,
and leaves a ratchet that prevents the old form from returning.

[pinned and up to date]: https://x.com/_lopopolo/status/2077904710003228694

Retries, bounded concurrency, deduplication, stop conditions, and an audit trail
make persistence operable. Escalation should carry a decision-ready packet:
current state, attempted work, evidence, risk, and the smallest unresolved
choice. The person supplies judgment without reconstructing the trajectory.

## Maintain the harness with the same loop

The environment around the agent accumulates drift too. Documentation becomes
stale, code develops competing patterns, review comments expose missing
guardrails, and a tool that once helped may become unnecessary after a model or
agent upgrade.

OpenAI's [recurring background maintenance tasks] replace a weekly human cleanup
session. Those tasks scan for deviations from golden principles, update quality
grades, open focused refactoring pull requests, and garden stale documentation.
Small repairs land before a bad pattern can become prompt material for future
changes.

[recurring background maintenance tasks]:
  https://openai.com/index/harness-engineering/#:~:text=On%20a%20regular%20cadence%2C%20we%20have%20a%20set%20of%20background%20Codex%20tasks

Feedback collection can run continuously as well. The [AI Native DevCon talk]
proposes a nightly distillation loop over review comments, human interventions,
failed builds, and production exceptions. Agents inspect that evidence for
missing context, tools, and guardrails, then propose durable improvements to the
environment.

[AI Native DevCon talk]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=Every%20review%20comment,every%20night

The [daily session-log reflection] describes this at team scale: agent session
logs are collected each day, inspected for ways the team can work better, and
reflected back into repositories so one person’s experience benefits later runs.
Raw trajectories remain telemetry. Reviewed distillation promotes stable lessons
into the smallest durable owner: a runbook, type, tool, test, lint, architecture
rule, or routing document.

[daily session-log reflection]:
  https://www.latent.space/p/harness-eng#:~:text=Running%20agent%20loops%20over%20them%20every%20day

Automatic refactoring, garbage collection, and [distillation] form a maintenance
loop over the harness itself. Accepted changes sharpen future runs; obsolete
instructions and redundant controls are removed. The loop should reduce known
failures and retire repeated validation made redundant by a stronger owner.

[distillation]: https://x.com/_lopopolo/status/2064346868398874848

## Keep invention in the foreground

Continuous maintenance preserves an already chosen relationship between policy
and a changing world. Humans still decide which world is worth building. They
set product direction, resolve incompatible user needs, choose among
architectures whose tradeoffs remain unknown, and define new acceptance
boundaries.

A mature loop completes settled work, gathers evidence around uncertain work,
and escalates choices that need judgment. Human attention reaches invention
sooner because localization, dependency updates, instrumentation, drift repair,
documentation gardening, and long mechanical migrations no longer wait beside it
in the same queue.
