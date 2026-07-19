# Turn Feedback Into Infrastructure

Feedback compounds when a team converts recurring corrections into the
environment that shapes the next run. Reviews, interventions, failures,
incidents, user reactions, and accepted or discarded work show where agents
struggle and where people still relay context, evidence, or authority.

Across repeated work, later trajectories inherit constraints and positive
examples distilled from earlier outcomes. Over years, that accumulated judgment
can [make coherence cumulative].

[make coherence cumulative]: ../durable-systems/#make-coherence-cumulative

## Collect observable signals

The useful record is the observable trajectory and its outcome. Preserve the
prompt, tool calls and output, diff, checks, review, runtime evidence, human
interventions, and accepted or rejected result. This evidence supports
corroboration without requiring hidden model reasoning.

Different sensors expose different parts of the system:

| Sensor                          | What it can reveal                                            |
| ------------------------------- | ------------------------------------------------------------- |
| human steering or relay         | missing context, tools, authority, or outcome understanding   |
| review comments                 | divergence from team judgment or an unexpressed requirement   |
| tests, lints, and failed builds | violation of an already executable invariant                  |
| logs, traces, and incidents     | end-to-end consequences absent from local proof               |
| user feedback and analytics     | whether the product outcome, demand, or explanation was wrong |
| MLD self-reports                | friction and missing surfaces the worker noticed              |
| accepted and failed artifacts   | dense examples of good decisions and known failure paths      |

The [feedback-capture inventory] asks teams to capture review comments,
interrupted runs, agent interventions, failed builds, and production exceptions,
then analyze them together for missing guardrails and better ways of working.
OpenAI's [feedback encoded as documentation and tooling] describes the same flow
from review, refactoring, and user-facing bugs into durable environment changes.

[feedback-capture inventory]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=Every%20review%20comment,every%20night
[feedback encoded as documentation and tooling]:
  https://openai.com/index/harness-engineering/#:~:text=Review%20comments%2C%20refactoring%20pull%20requests%2C%20and%20user-facing%20bugs%20are%20captured%20as%20documentation%20updates

A signal is a lead rather than a diagnosis. External failures, stochastic
variation, bad task framing, and overfit controls remain possible. Compare the
worker's explanation with the rest of the trajectory before changing the
harness.

Many complaints labeled “slop” point to a missing or inaccessible nonfunctional
requirement. [Slop can reflect missing requirement context and merge gates]: the
worker lacked the local quality bar, while tests and other blocking checks did
not constrain the accepted result. Recovering the governing requirement turns a
vague rejection into a search for the context, example, review policy, or
executable boundary that should shape later work.

[Slop can reflect missing requirement context and merge gates]:
  https://x.com/_lopopolo/status/2036967120345743776

## Recover the governing failure class

A correction to one named line can leave every sibling defect intact. Ryan
Lopopolo's [sentinel-error example] gave an agent a concrete Go API
correction—return a sentinel error instead of a success boolean—and the agent
changed only the named function. The instruction carried a broader principle
about API design, but the repair did not search for the rest of the class.

[sentinel-error example]: https://x.com/_lopopolo/status/2054027190967349685

Treat recurring steering as a systems question:

1. Reconstruct the promised outcome and the observed failure.
2. Locate the earliest boundary that could have prevented or exposed it.
3. Search for other instances governed by the same principle.
4. Separate a harness gap from worker variance, external failure, and a bad
   premise.
5. Choose an intervention that covers the class without forbidding legitimate
   exceptions.

When the same correction recurs, give its underlying principle a durable owner.
[High-signal repeated steering] deserves a document, tool, check, or
architectural change, and the [principle implicit in the prompt] should guide
the search for sibling cases. This reduces repeated steering while leaving room
for novel and stochastic failures.

[High-signal repeated steering]:
  https://x.com/_lopopolo/status/2054011131572990266
[principle implicit in the prompt]:
  https://x.com/_lopopolo/status/2054021564862177517

The AI Native DevCon talk uses [React and Suspense] as an illustrative
onboarding scenario. Suppose a team uses Suspense in a particular family of
screens and components to meet its frontend-performance goals. A person can
remember that convention after one review comment; a fresh agent run cannot. The
team must decide whether the missing context belongs in documentation, a lint, a
test, or a reviewer applied to every pull request. That routing choice preserves
the recurring correction at the earliest boundary that can shape later work.

[React and Suspense]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=make%20these%20mistakes%20statically%20impossible%20going%20forward

## Promote the lesson into the smallest durable owner

Match the intervention to the maturity and consequence of the lesson:

| Intervention                   | Use when                                                        |
| ------------------------------ | --------------------------------------------------------------- |
| prompt adjustment and reroll   | the task framing or desired outcome is still being discovered   |
| routing doc, runbook, or skill | stable knowledge must appear at a particular decision point     |
| reviewer or evaluation         | judgment is qualitative, cross-cutting, or still gaining nuance |
| type, API, or domain tool      | the system can make correct use natural and misuse difficult    |
| lint, test, or policy check    | a deterministic invariant should block recurrence               |
| architecture or migration      | repeated defects expose the wrong owner or dependency direction |

The [steering-to-validator progression] begins with steering, writes down
recurring guidance, uses review for judgment that remains contextual, and moves
settled invariants into executable controls. Stable lessons can continue
upstream into types and APIs; repeated ownership defects may justify an
architectural migration. [Nuance after overfit] adds the corresponding
maintenance rule: add nuance when a coarse instruction overfits.

[steering-to-validator progression]:
  https://tessl.io/podcast/109/#:~:text=the%20cheapest%20thing%20I%20can%20do%20is%20literally%20produce%20a%20change%20to%20my%20prompt,making%20deterministic%20tests%20that%20suss%20out%20this%20behavior
[Nuance after overfit]:
  https://hyperbo.la/w/what-does-it-mean-to-do-a-good-job/#:~:text=add%20nuance%20only%20when%20the%20coarse%20instruction%20starts%20to%20overfit

Remove downstream defenses when a better upstream owner makes them redundant.
Layering another validator over a missing domain model preserves the incoherence
and increases carrying cost.

## Learn golden principles and garbage-collect drift

Review feedback, failed rollouts, incidents, and repeated corrections can expose
a candidate principle. Recover the governing failure class and test it against
accepted work, legitimate exceptions, and later outcomes. Give a stable result
an owner and a reviewable statement of where it applies. When it can be stated
as an opinionated, mechanical rule, encode it as a scoped [golden principle].

[golden principle]:
  https://openai.com/index/harness-engineering/#:~:text=Instead%2C%20we%20started%20encoding,open%20targeted%20refactoring%20pull%20requests

Learn from the outcome after review. [Accepted work], [failed attempts],
reviewer corrections, and documented exceptions can supply different evidence
about the principle and its implementation. The [nightly feedback-distillation
proposal] collects review comments, agent interventions, failed builds, and
production exceptions so stable lessons can improve later context and
guardrails. Preserve the rationale and provenance of those outcomes before
promoting the lesson.

[Accepted work]: https://x.com/_lopopolo/status/2054037448297201696
[failed attempts]: https://x.com/_lopopolo/status/2063229530840690892
[nightly feedback-distillation proposal]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=Every%20review%20comment,every%20night

Give learned guidance a retrieval route: describe when it matters and link to
bounded supporting examples. Keep changing target corpora and underlying
trajectories with their authoritative owners. Bounded citation snapshots may
follow the source policy, while later work retrieves relevant slices without
creating a parallel semantic owner for target truth. [Route Context Just in
Time] develops that retrieval boundary.

[Route Context Just in Time]: ../just-in-time-context/

Agents use the repository’s code and prose as precedent. Ryan’s observation that
[code is prompts for future prompts] means a weak pattern left in the tree can
shape later trajectories after its immediate defect has been fixed. A durable
principle therefore needs two paths: a forward path that places the invariant at
its earliest semantic owner, and a backward path that finds and migrates the
existing population. Ryan describes [the no-await-in-loop migration]: enabling
the ESLint rule, repairing roughly 600 existing violations, and adding
exhaustive coverage in one pull request. The article states [the durable rule]
separately: an invariant that matters belongs in a repository-owned verifier,
which can [return descriptive feedback] when the model produces a later
violation.

[code is prompts for future prompts]:
  https://x.com/_lopopolo/status/2030460794765451300
[the no-await-in-loop migration]:
  https://hyperbo.la/w/production-function-changed/#:~:text=I%20turned%20on%20no-await-in-loop%20from%20ESLint,exhaustively%20add%20test%20coverage
[the durable rule]:
  https://hyperbo.la/w/production-function-changed/#:~:text=If%20it%20matters%2C%20it%20belongs%20in%20a%20verifier%20owned%20by%20the%20repo.
[return descriptive feedback]:
  https://hyperbo.la/w/production-function-changed/#:~:text=surface%20a%20descriptive%20failure%20to%20the%20model%20every%20time%20it%20writes%20another%20fetch%20call

Garbage collection keeps that relationship current. Recurring scans find drift,
update quality grades, and propose narrow repairs, shortening the time
counterexamples remain available as precedent. The collector also maintains the
harness: stronger types, APIs, or architecture can make downstream checks
redundant, while changed requirements or worker behavior can make instructions
and exemptions stale. Ryan joins automatic refactoring and garbage collection
with [automatic distillation]; [continuous maintenance] owns the cadence,
authority, proof, and retirement contract for the loop.

[automatic distillation]: https://x.com/_lopopolo/status/2064346868398874848
[continuous maintenance]: ../continuous-maintenance/

The collector produces candidates for judgment. Keep a suspected violation
separate from the claim that its proposed repair is safe and complete. The
[authority boundary] governs who may apply a repair, and the [proof boundary]
governs the evidence needed to accept it. Uncertain, consequential, and
cross-cutting changes need appropriate review; narrow repairs with
outcome-matched proof may proceed automatically. Reviewed results can teach
later passes without turning the collector’s own unverified output into policy.

[authority boundary]: ../authority/
[proof boundary]: ../proof/

Enforced knowledge should accumulate while validator count consolidates.
Principles that remain qualitative stay in context, examples, and review.
Settled invariants can move into executable owners. Complete migrations remove
contradictory precedent, and each collection pass can consolidate or retire the
machinery made unnecessary by a stronger owner.

Ryan’s instruction to [replace “codebase” with “artifact”] generalizes the loop
beyond software repositories. Applying it to documentation, datasets, reports,
interfaces, and operational workflows means defining domain-appropriate
principles, acceptance evidence, and maintenance.

[replace “codebase” with “artifact”]:
  https://x.com/_lopopolo/status/2076878736507736390

## Accrue domain expertise through the work

Review feedback often reveals a reasonable decision made from incomplete local
context. In [Build Hour: API & Codex], an internal OpenAI product team had
worked with security to choose an approved cryptography implementation, but the
decision lived in an old Slack thread rather than the repository. Codex, working
for a new engineer, added a different npm cryptography dependency. The team
recovered the decision, encoded a repository rule requiring the
security-approved implementation and forbidding alternate npm cryptography
packages, and reran the change under that constraint. The repository now owns
the scoped decision and its enforcement boundary.

[Build Hour: API & Codex]:
  https://rewiz.app/channels/%40openai/build-hour-api-codex#:~:text=I%20partnered%20with%20our%20security%20engineering%20team%20to%20upgrade%20our%20app%20to%20the%20blessed%20cryptography%20library

That move changes the next worker's decision before review. It also gives the
team one versioned place to revise the choice when the approved library or risk
posture changes.

In the 2026 [un]prompted talk [Code Is Free], security engineer Paul McMillan
and Ryan Lopopolo follow the longer loop from a precise question to durable
security judgment. When a code change invalidates a risk-accepted assumption in
the threat model, the worker identifies the responsible expert, explains why
their judgment is needed, and presents the ambiguity that the repository cannot
resolve. The conversation preserves provenance and scope while concentrating
scarce expert attention on the decision that requires it.

[Code Is Free]: https://www.youtube.com/watch?v=U2O14Jd3MBU&t=395s

The supply-chain case begins with a pull request to the public `openai/codex`
repository that [updates pnpm to repair vulnerabilities and forbids post-install
scripts]. Ryan recovers the principles behind the patch: package managers are a
high-risk boundary, dependencies should stay current, installation should not
execute arbitrary code in CI, and exceptions require human review.

The talk then presents a sanitized, hypothetical Electron product with roughly
1,500 packages in its lockfile. In a fifteen-minute conversation, security
engineer Paul McMillan asks Ryan to [risk-profile the direct and transitive
dependencies], gather evidence about upstream activity and author reputation,
and identify packages to upgrade, remove, or replace in-house. Four hours of
agent work turns the point-in-time analysis into a daily scanner run by sixteen
agents. It examines the dependency graph and upstream repositories, then
publishes machine-readable security findings in the Static Analysis Results
Interchange Format (SARIF) through GitHub Advanced Security. Ryan described the
result as [making the harness an expert security engineer]. The reports can also
feed downstream agents that take those actions or migrate away from frameworks
carrying problematic dependencies.

[updates pnpm to repair vulnerabilities and forbids post-install scripts]:
  https://www.youtube.com/watch?v=U2O14Jd3MBU&t=912s
[risk-profile the direct and transitive dependencies]:
  https://www.youtube.com/watch?v=U2O14Jd3MBU&t=1030s
[making the harness an expert security engineer]:
  https://x.com/_lopopolo/status/2022853771517100166

Later Slack feedback showed that the scanner had captured point instances more
readily than Paul's [general reference frame]. The team added the missing
guardrails and investigative signals to `SECURITY.md`; those principles then fed
the daily scanner and new dependency-depth and license scanners. This is how
expertise accrues: retain the source and accepted scope, recover the governing
principle, encode stable judgment in the earliest owner that can shape future
work, and route materially changed assumptions back to the expert. Later scans
then apply Paul's revised principles across the dependency graph instead of
rediscovering them one package at a time.

[general reference frame]: https://www.youtube.com/watch?v=U2O14Jd3MBU&t=1176s

## Keep review convergent

Feedback systems need an acceptance policy. Early reviewer agents in Ryan's team
could repeatedly bully the implementation agent into widening the change. The
team instructed reviewers to bias toward merge and surface material findings,
while authors could fix, defer with rationale, or push back. The [reviewer
convergence account] develops that evolution.

[reviewer convergence account]:
  https://www.latent.space/p/harness-eng#:~:text=gave%20it%20the%20flexibility%20to%20either%20defer%20or%20push%20back%20against%20review%20feedback

Treat reviewer output as evidence. Severity thresholds, explicit response
choices, and a stopping rule keep the review loop convergent while bounding its
cost in agent and human attention.

## Let product feedback change the job

Product learning can change the artifact the job should produce. When the team
first built the Codex app’s scheduled-automations UI, the implementation coupled
the interface to a cron backend and the agent core; part of the scheduler even
ended up in frontend JavaScript. The team reverted that tangled implementation.
The [painted-door experiment] kept the complete scheduling interface over a
no-op backend and instrumented the product surface. That artifact could test
whether people understood and wanted scheduled automations before the team
decided whether the backend deserved priority.

[painted-door experiment]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=ended%20up%20with%20what%20we%E2%80%99re%20calling%20a%20painted%20door%20style

Consequential operations can improve the harness while the original job
continues. Loki is a robot vacuum running the open-source Valetudo firmware. A
custom Tailscale daemon is its only remote-access path. Upgrading that daemon
from v1.90.8 to v1.96.4 could therefore destroy the route needed to recover a
failed deployment. The [robot-vacuum upgrade] added a second Tailscale identity
and separate state on the same machine; the candidate had to boot, join the
tailnet, and accept SSH before it could replace production.

The first canary run taught the deployment tool about the real boundary. OpenSSH
`scp` attempted SFTP and could not complete cleanly, so the tool learned to
stream the binary over SSH. It added Tailscale’s `--accept-risk=lose-ssh` flag
and isolated known-hosts state for the canary identity. Later, `prod backup`
exposed that sending compound remote scripts as `ssh ... sh -c <script>` broke
shell constructs such as `if ... then ... fi`. The tool switched to sending
scripts over standard input to `sh -s`, added focused regression tests, reran
validation, and continued the deployment.

[robot-vacuum upgrade]:
  https://hyperbo.la/w/robot-vacuum-canary-tailscale/#:~:text=The%20deploy%20also%20improved%20the%20tool

## Keep successful and failed work as evidence

Accepted and failed work can preserve decisions and boundary conditions that a
short instruction cannot enumerate.

The [Symphony distillation loop] extracts a specification from a successful
implementation, tests that specification through an independent
reimplementation, and uses the differences to refine it. This works because
[blessed work is information dense]. It contains many decisions that a short
instruction cannot enumerate. [Failed work as evidence] becomes useful when kept
with the trajectory and outcome that explain what failed.

[Symphony distillation loop]: https://x.com/_lopopolo/status/2048906262109413801
[blessed work is information dense]:
  https://x.com/_lopopolo/status/2054037448297201696
[Failed work as evidence]: https://x.com/_lopopolo/status/2063229530840690892

Keep these artifacts with their outcome and provenance. Promote the recurring
principle into ordinary context or tooling; do not turn a raw archive into a
permanent prompt.

## Treat MLD as one sensor

The [MLD instruction] asks the agent to record Mistakes, Learnings, and Desires
while it works. Ryan later named this the MLD framework and said the files are
[for the agent builder]. Treat the report as telemetry: the harness builder
corroborates it against the trajectory and promotes only recurring,
well-supported gaps.

[MLD instruction]: https://x.com/_lopopolo/status/2037307669464363433
[for the agent builder]: https://x.com/_lopopolo/status/2052095157336703410

An agent can miss its most important error, misidentify a cause, or request the
wrong tool. Raw MLD remains attached to the run that produced it. [MLD:
Telemetry for the Harness Builder] develops collection, corroboration,
promotion, expiry, and privacy boundaries.

[MLD: Telemetry for the Harness Builder]: mld.md

## Close the loop on later runs

The intervention earns its carrying cost when later outcomes improve: fewer
human relays, less repeated steering, shorter repair loops, stronger proof,
lower operational risk, or broader work completed under the same quality bar.
Rerun comparable tasks with the worker condition held constant, and remove
context or tooling whose effect does not survive ablation.

[Evaluate the Harness] isolates one intervention under a fixed worker condition,
keeps truth with the target, and grades the contract at the outcome boundary.
[Optimize for Measured Effectiveness] accounts for the human attention, latency,
risk, compute, and maintenance cost of the resulting improvement.

[Evaluate the Harness]: ../../evals/
[Optimize for Measured Effectiveness]: ../effectiveness/
