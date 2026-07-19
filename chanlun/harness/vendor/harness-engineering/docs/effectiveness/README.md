# Measure Effectiveness at the Outcome Boundary

Measure effectiveness through valuable, proven outcomes. Place those outcomes
beside the full cost and risk of producing and maintaining them. Human
attention, elapsed time, compute, rework, consequence, and future carrying cost
describe different constraints. Keep them visible as separate dimensions; one
blended score would conceal the tradeoffs a team needs to manage.

## Measure the accepted outcome

Start where a user, operator, dependent system, or decision-maker experiences
the result. Depending on the job, the accepted outcome may be shipped user
benefit, a proven security correction and advisory, a safely completed upgrade,
a validated decision to avoid a premature build, or a durable capability that
changes later work.

[Prove a Security Claim] follows a report through impact analysis, correction,
release, and advisory. [The robot-vacuum upgrade] closes only after the upgraded
machine remains reachable and leaves a tested path for the next operation. The
[painted-door experiment] began when a designer explored the scheduled-task
interaction that later appeared in the Codex app. The first implementation
coupled a new cron backend to the agent core and at one point put scheduling
logic in frontend JavaScript. The team reverted that implementation, retained
the complete user interface over a no-op backend, and instrumented the surface
to learn whether users understood and wanted the capability. The accepted
outcome was evidence for the product decision about whether to prioritize the
backend.

[Prove a Security Claim]: ../proof/rustsec.md
[The robot-vacuum upgrade]:
  https://hyperbo.la/w/robot-vacuum-canary-tailscale/#:~:text=After%20those%20gates%2C%20Codex%20ran%20the%20runbook%20end%20to%20end
[painted-door experiment]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=ended%20up%20with%20what%20we%E2%80%99re%20calling%20a%20painted%20door%20style

A pull request, line count, plan, token total, or generated artifact can
contribute to an outcome. None establishes value on its own. Ask what the work
[ladders up to], who it serves, what failure would cost, and how it changes the
team's future velocity. The [whole-job boundary] and its proof should be
declared before proxy measures begin to look like success.

[ladders up to]: https://x.com/_lopopolo/status/2055111894181335279
[whole-job boundary]: ../whole-job/

## Count fully loaded human attention

Implementation cost can fall while the costs of direction, relay, review,
coordination, verification, recovery, and maintenance remain. Count prompting
and steering, copy-and-paste relay, context switching, CI and merge babysitting,
smoke testing, synchronous meetings, escalation, and incident recovery across
the whole job. Time removed from implementation may otherwise reappear as review
burden or future cleanup.

[The Production Function Changed] describes cheaper repository-wide
implementation shifting human attention toward choosing the invariant and
demanding proof. [Software Work Is No Longer Scheduled] applies the same
economics to bounded work with a clear desired state, understood interfaces,
and verifiable results while reserving attention for new products, difficult
interfaces, and other unresolved shapes.

[The Production Function Changed]:
  https://hyperbo.la/w/production-function-changed/#:~:text=The%20scarce%20work%20is%20choosing%20the%20invariant%20and%20deciding%20whether%20the%20proof%20is%20sufficient.
[Software Work Is No Longer Scheduled]:
  https://hyperbo.la/w/software-work-not-scheduled/#:~:text=Once%20the%20decision%20has%20been%20made%20and%20success%20can%20be%20specified%2C%20I%20would%20rather%20hand%20execution%20to%20agents%20than%20make%20the%20change%20compete%20for%20a%20place%20on%20the%20roadmap.

Ryan Lopopolo's [2023 ChatGPT pairing account] records the earlier loop
directly: code copied between an editor and a browser, tests manually relayed
back into the chat, sessions with hundreds of prompts, five-minute compile
repairs, and work that became mentally taxing after an hour. In a later [60-hour
refactor account], he describes one of the hardest refactors of his career,
which he estimated would have taken him three weeks by hand. Codex ran for 60
hours across three or four days, produced one pull request, and needed only the
initial prompt plus two follow-ups. The [run summary] and later interview put
its cost at roughly 330–350 million tokens and $2,800. The wall-clock duration
grew while the synchronous interface with the human became dramatically smaller.

[2023 ChatGPT pairing account]:
  https://hyperbo.la/w/chatgpt-4000/#:~:text=I%20interacted%20with%20ChatGPT%20by%20copying%20code%20in%20vim
[60-hour refactor account]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=this%20is%20a%20PR%20that%20probably%20would%20have%20taken%20me%20three%20weeks
[run summary]: https://x.com/_lopopolo/status/2031207017919209690

The runs differ in model, task, tools, and environment, so they do not establish
a trend. They do show why wall time and human attention need separate clocks.
Track where people spend time, then use the worker and its environment to
[remove recurring human work] without displacing it into another stage.

[remove recurring human work]:
  https://x.com/_lopopolo/status/2051445312049553642

## Keep four clocks

“Latency” describes several different loops:

| Clock                       | What it measures                                       | Why it matters                                           |
| --------------------------- | ------------------------------------------------------ | -------------------------------------------------------- |
| worker feedback latency     | action to useful test, build, trace, or review signal  | governs iteration speed                                  |
| worker wall-clock duration  | start to completed run                                 | occupies execution capacity and may delay dependent work |
| synchronous human attention | minutes a person must direct, relay, review, or rescue | determines how much work the organization can supervise  |
| time to accepted outcome    | request to proved, accepted, or deployed result        | captures queues, reruns, review, and delivery together   |

Fast local feedback and long runs can coexist. Under GPT-5.2, the Codex harness
lacked background shells, so the team could rely on blocking build scripts.
After GPT-5.3 added background shells, Ryan found the worker less willing to
wait for a blocking build. Over one week, the team moved from a bespoke Makefile
through Bazel and Turbo to Nx, stopping once the [build loop completed in under
roughly one minute]. That minute was a round upper bound: a slower build
triggered another decomposition of the build graph. The 60-hour refactor above
shows why this feedback loop and total worker duration need separate clocks.

[build loop completed in under roughly one minute]:
  https://www.latent.space/p/harness-eng#:~:text=over%20the%20course%20of%20a%20week%2C%20we%20went%20from%20a%20bespoke%20make%20file%20build%20to%20Basil%2C%20to%20turbo%20to%20nx

When the first dozen users of an internal application reported performance
problems, the team asked them to export a trace tarball. An on-call engineer
spent an afternoon working with Codex to build a polished local Next.js viewer
for the traces. The team then gave [the same tarball directly to Codex], which
returned the requested analysis in about five minutes. Direct inspection closed
this job; a specialized viewer would have been warranted only if Codex needed
one to obtain the answer.

[the same tarball directly to Codex]:
  https://www.latent.space/p/harness-eng#:~:text=you%20could%20just%20spin%20up%20codex%20and%20give%20it%20the%20tar%20ball

Small harness changes can alter a clock independently. When long-running data
queries returned a pending response, Codex spent time reasoning before each
poll. An instruction to sleep on pending responses produced a reported
[threefold wall-clock reduction] without changing the underlying query.

[threefold wall-clock reduction]:
  https://x.com/_lopopolo/status/2052066301821255929

## Separate addressability, activity, and productive utilization

A worker can create value only across parts of the job it can reach. Repository
access, local applications, tests, browsers, logs, traces, deployment, and proof
increase **addressability**. Tokens and busy time measure **activity**.
**Productive utilization** is activity that reaches an accepted outcome or
produces reusable evidence. Preventable discard, review, and repair reduce it.

[Agent Utilization Is the New Performance Ceiling] uses high utilization as a
forcing function for exposing inaccessible parts of the lifecycle. Idle time may
reveal missing context, tools, permissions, observability, review, or delivery.
Busy workers may also generate rejected patches or maintenance burden at high
speed. In the later [Tessl interview], the billion-token framing acquires an
outcome gate: the work must reach main and improve the user experience.

[Agent Utilization Is the New Performance Ceiling]:
  https://hyperbo.la/w/agents-agents-agents/#:~:text=A%20billion%20tokens%20per%20engineer%20per%20day%20is%20a%20utilization%20target
[Tessl interview]:
  https://tessl.io/podcast/109/#:~:text=that%20make%20it%20to%20main%20and%20improve%20the%20end%20experience

[Token leaderboards] can reward a signal that rises while value stands still.
[Spend unanchored from value] says little about ROI; accepted outcomes make
[effectiveness the objective].

[Token leaderboards]: https://x.com/_lopopolo/status/2035101303140167745
[Spend unanchored from value]:
  https://x.com/_lopopolo/status/2035102053979271654
[effectiveness the objective]:
  https://x.com/_lopopolo/status/2035132736621748332

[Please Go Brr, on Token Mandates] treats broad token budgets as an
organizational search strategy. At the evaluation boundary, token volume records
the search effort. Record which situated jobs, practitioners, missing
capabilities, and reusable deployment patterns the search discovers. Accepted
outcomes and durable deployment knowledge determine which discoveries deserve
continued investment.

[Please Go Brr, on Token Mandates]:
  https://hyperbo.la/w/token-mandates/#:~:text=organizational%20discovery%20mechanism%20for%20learning%20how%20to%20deploy%20reasoning%20compute

## Distinguish exploration, rework, and waste

Discard can be productive when an attempt resolves an uncertainty that matters
to later work. The scheduled-task implementation above exposed an architectural
failure before the team had evidence that users wanted the backend. Reverting it
while preserving the instrumented interface separated product discovery from the
discarded implementation. Other failed attempts can expose a missing constraint,
tool, or grader more densely than a successful patch.

When a high-ambiguity refactor's final interface shape remains unknown, Ryan
describes a [large disposable-probe pattern]. He lets the agent produce an
exploratory diff that can reach roughly 50,000 lines, pushes the pull request so
the work can be inspected, and then discards it. The attempt reveals where the
worker is likely to fail and which interface seams the eventual cutover needs.
He turns that map into roughly fifteen preparation and staging pull requests
before returning to the cutover. The exploratory implementation is temporary;
the failure map and the preparation and staging changes are what he carries into
the later cutover.

[large disposable-probe pattern]:
  https://tessl.io/podcast/109/#:~:text=50%2C000%20line%20of%20diff%20PR%20and%20throw%20it%20away

Use [an explicit rollback point] when exploratory work may be thrown away. A
failed attempt becomes [information dense] when the team preserves the
constraint, seam, or grader it exposed. The value appears when the attempt
changes a decision, improves the environment, or makes the next success easier.
A discarded diff compounds only when later work can recover what it revealed.

[an explicit rollback point]: https://x.com/_lopopolo/status/2029085149749756028
[information dense]: https://x.com/_lopopolo/status/2063229530840690892

Count attempts, CI cycles, reviewer cycles, reversions, and repeated failure
classes. Repeating a known failure without new evidence is waste. A harness that
keeps eliciting the same high-signal steering has a longitudinal defect even
when each individual correction is cheap.

Repeated failure classes belong in the environment. Promote them into context,
tools, tests, reviewers, or architecture through [Turn Feedback Into
Infrastructure]. [Evaluate the Harness] prices the trajectory as well as the
accepted result.

[Turn Feedback Into Infrastructure]: ../feedback/
[Evaluate the Harness]: ../../evals/

## Price the lifetime

The ownership ledger begins with model compute and human attention, then
continues through proof, recovery, operation, and maintenance. Operational
exposure and consequence remain visible beside incurred cost. Cheap generation
lowers exploration and implementation cost while the repository retains the
obligations created by the change.

Dependency internalization makes the tradeoff concrete. Removing an outside
package can reduce supply-chain surface while transferring compatibility,
security, performance, and standards maintenance into the repository. [Ryan's
homelab Markdown implementation] accepts that transfer with an owned parser,
AST, renderer, and sanitizer plus corpus, golden, security, cancellation, error,
and coverage proof. A high-quality parser or codec can be the lower-risk owner
when the repository cannot sustain those obligations.

[Ryan's homelab Markdown implementation]: ../domain-modeling/homelab.md

Track dependency and tool count, upgrade work, policy complexity, standards
drift, incident exposure, and future change cost. Compute belongs in the same
accounting: tokens, inference cost, CPU or GPU time, CI minutes, and storage are
real inputs even when model budgets are intentionally generous.

## Measure compounding longitudinally

A harness earns its carrying cost when comparable jobs improve enough to cover
the cost of building and maintaining it. Record the harness revision and compare
the dashboard across like jobs within one worker epoch.

An OpenAI team began [an internal software product] from an empty repository and
developed it for five months as a beta used inside the company. Codex generated
all of its roughly one million lines of code; the team reported about 1,500
merged pull requests and no manually written code. [Early progress was slow]
while the repository lacked the tools, abstractions, and structure the agent
needed. [Throughput rose as the team and harness grew], so later work inherited
capabilities and judgment encoded by earlier work. The robot-vacuum case leaves
the next upgrade a documented and tested standing path. These longitudinal
claims ask whether comparable later jobs inherit enough leverage to justify the
harness's carrying cost.

[an internal software product]:
  https://openai.com/index/harness-engineering/#:~:text=Five%20months%20later%2C%20the%20repository%20contains%20on%20the%20order%20of%20a%20million%20lines%20of%20code
[Early progress was slow]:
  https://openai.com/index/harness-engineering/#:~:text=Early%20progress%20was%20slower%20than%20we%20expected
[Throughput rose as the team and harness grew]:
  https://openai.com/index/harness-engineering/#:~:text=surprisingly%20the%20throughput%20has%20increased%20as%20the%20team%20has%20grown

Start a new worker epoch when the model or coding agent changes materially, and
requalify context, tools, ambition, and operating priors. Compare like jobs. A
stronger worker or easier task mix can otherwise masquerade as a harness
improvement. [Hold the Worker Constant] defines the worker boundary.

[Hold the Worker Constant]: ../fixed-worker/

## Keep the dimensions visible

For every accepted outcome, record:

| Dimension   | Question                                                   | Representative evidence                                      |
| ----------- | ---------------------------------------------------------- | ------------------------------------------------------------ |
| outcome     | What changed for a user, operation, business, or decision? | task-specific acceptance and deployed behavior               |
| attention   | What human time did the whole job consume?                 | steering, relay, review, QA, coordination, recovery minutes  |
| flow        | Which clock constrained completion?                        | feedback, worker-duration, human, and acceptance percentiles |
| rework      | What was discarded, repeated, or learned?                  | attempts, reversions, CI and reviewer cycles                 |
| risk        | What could fail, and how was confidence established?       | claim-matched proof, incidents, rollback, recovery           |
| lifetime    | What future obligations were added or removed?             | dependency, tooling, policy, upgrade, and change burden      |
| compute     | What did the accepted outcome consume?                     | tokens, inference cost, CPU/GPU time, CI minutes, storage    |
| compounding | Did comparable work improve within the worker epoch?       | trends by harness revision and matched task class            |
