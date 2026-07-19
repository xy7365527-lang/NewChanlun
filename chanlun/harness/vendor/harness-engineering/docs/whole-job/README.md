# Give One Agent the Whole Job

Delegate an outcome at the highest level the harness can safely support, then
give one primary trajectory responsibility for closing it. The team supplies
direction, judgment, and authority at consequential boundaries. The owner
retrieves context, investigates, chooses a method, returns the analysis, change,
or decision the job requires, proves the result, and carries it through
delivery.

## Delegate the outcome and its bar

Ryan Lopopolo often gives [close to zero constraints] in a prompt. Sparse
delegation tests whether the environment has made the real requirements
recoverable. The prompt still names the outcome, a hard acceptance criterion, or
a consequential boundary when the agent cannot safely infer it.

[close to zero constraints]: https://x.com/_lopopolo/status/2054053216254636073

Ryan defines high ambition with a hypothetical request concerning `libghostty`,
the reusable library surface being developed from the Ghostty terminal emulator.
The worker would search [across the open-source ecosystem] for gaps, bugs,
missing functionality, and distribution or packaging improvements that could
make the library better. The prompt proposes a broad discovery job; it does not
report a completed or exhaustive survey. The harness still supplies the quality
bar, proof, and authority boundary for work spanning more than one predetermined
patch or repository.

[across the open-source ecosystem]:
  https://x.com/_lopopolo/status/2075861840085848202

A short prompt can carry a high bar. The request behind [A Lazy Prompt Turned
Into a RustSec Advisory] asked for a complete red-team analysis and required
proof of impact or exploitability for every finding. Codex developed the report,
reproducer, patch, and regression tests, then revised the implementation after
detailed human review recorded in [artichoke/intaglio#360]. Ryan approved the
implementation and authorized Codex to prepare a point release, merge the pull
request, and open the RustSec report. The prompt specified the outcome and
acceptance criterion while the workflow preserved substantive review and
authority at delivery.

[A Lazy Prompt Turned Into a RustSec Advisory]:
  https://hyperbo.la/w/lazy-prompt-rustsec/#:~:text=full%20job%20end%20to%20end%2C%20from%20identification%20and%20fix%20to%20vuln%20publication
[artichoke/intaglio#360]: https://github.com/artichoke/intaglio/pull/360

Ryan calls these prompts lazy because the method remains open. Current models
can still take sparse requests too literally, so a miss reveals which
requirement, tool, or piece of context the environment failed to make legible.
The [prompt-observe-improve-reroll loop] inspects that trajectory, improves the
harness, discards the result, and runs the job again.

[prompt-observe-improve-reroll loop]:
  https://x.com/_lopopolo/status/2048901233721979363

Reusable instructions remain subordinate to the outcome. A [push-PR skill
failure] came from agent instructions that normally required a green local build
before opening a pull request. One blocked task needed an early pull request so
collaborators could review the work and help repair that build. The skill
refused the operation. Its usual safeguard had become an obstacle to the outcome
it was meant to support.

[push-PR skill failure]: https://x.com/_lopopolo/status/2030395671166230796

## Keep durable intent sparse across compaction

Where an instruction enters the trajectory affects how long it competes for
attention. Long Codex trajectories repeatedly compact their active history, so
[context is rewritten across long work]. Ryan's operating hypothesis from those
sessions is that user intent remains a high-signal influence while older tool
results are more likely to be summarized or pruned. This is worker-specific
behavior to requalify after model, agent, or compaction changes. Under that
hypothesis, detailed phase-specific policy in the user message can remain
salient after its work is over and smear attention across the rest of the job.
Keep that channel focused on the outcome, acceptance bar, and authority
boundary. Ryan recommends making [tokens easy to predict with the fewest
instructions] and says he [closes off bad latent space outside user messages].

[context is rewritten across long work]:
  https://x.com/_lopopolo/status/2062950475989725465
[tokens easy to predict with the fewest instructions]:
  https://x.com/_lopopolo/status/2031861303917363319
[closes off bad latent space outside user messages]:
  https://x.com/_lopopolo/status/2054053548124778786

Repository reads, tool results, test failures, browser observations, and focused
reviewers can surface narrower policy when a decision makes it relevant. The
active copy may fall out of context after that phase; a short repository route
lets the worker retrieve the rule again. In the [robot-vacuum maintenance case],
a scheduled prompt names the job and points to a versioned automation guide and
task-specific runbook. Those documents carry the release-assessment, proof, and
deployment-authority contracts. Separately, a report-only
[documentation-freshness role] compares operator guidance with the configuration
and monitoring files that define current behavior and reports drift. These
routes keep work on harness policy without loading every rule into the user's
instruction or assuming that [deterministic context stuffing] will survive a
long sequence of compactions.

[deterministic context stuffing]:
  https://x.com/_lopopolo/status/2062966895876215172
[robot-vacuum maintenance case]:
  https://hyperbo.la/w/robot-vacuum-canary-tailscale/#:~:text=The%20repo%20carries%20the%20durable%20instructions
[documentation-freshness role]:
  ../domain-modeling/homelab.md#keep-operational-docs-current

Compaction retention and instruction authority are separate concerns. Retrieved
context and ordinary tool output do not by themselves grant authority or
supersede the user’s outcome. A designated approval or policy system may report
that a predeclared grant’s conditions have been satisfied; the harness defines
how that evidence changes the actions available to the worker. [Route Context
Just in Time] develops the retrieval and attention mechanics. Requalify the
placement strategy when [the worker changes].

[Route Context Just in Time]: ../just-in-time-context/
[the worker changes]: ../fixed-worker/

## Recover hidden requirements

A literal request usually names a functional outcome while leaving
[nonfunctional requirements] such as compatibility, maintainability, and
accessibility implicit. Generated parity, rollback, required checks, and the
needs of future changes are other hidden acceptance constraints. [What Does It
Mean to Do a Good Job?] explains how teams have carried this larger acceptance
bar through shared norms and repeated work.

[nonfunctional requirements]:
  ../domain-modeling/#make-nonfunctional-requirements-recoverable
[What Does It Mean to Do a Good Job?]:
  https://hyperbo.la/w/what-does-it-mean-to-do-a-good-job/#:~:text=tone%2C%20taste%2C%20risk%20tolerance%2C%20how%20much%20polish%20is%20enough%2C%20what%20shortcuts%20are%20acceptable

Hidden nonfunctional requirements are [the question behind the question]. A
model can [struggle to recover them from the prompt]. It recovers them by
inspecting the repository, current behavior, history, and the outcome's
relationship to users. It should inspect discoverable facts before asking a
person to repeat them, then ask when the missing answer changes product intent,
accepts consequential risk, or grants authority it does not have.

[the question behind the question]:
  https://x.com/_lopopolo/status/2030396192832819524
[struggle to recover them from the prompt]:
  https://x.com/_lopopolo/status/2032582601110913515

## Keep one primary trajectory

One primary trajectory is the accountable integration point for many
perspectives. Product, design, operations, security, support, and engineering
contribute distinct context, constraints, evidence, and acceptance criteria. The
owner compiles that team judgment into one result while retaining the user
problem, implementation state, and responsibility for closure.

In [How PMs Ship 100K Lines of Code], Ryan describes one agent carrying a loop
that would traditionally cross several roles: understand the user challenge and
codebase, work within the design system, deploy the result, observe whether it
succeeds, and feed bugs and user feedback back into the work. The roles continue
to supply expertise and judgment. Making their knowledge available to one
code-producing owner reduces the loss of purpose across design, frontend,
backend, and verification handoffs.

[How PMs Ship 100K Lines of Code]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=with%20full%20context%20do%20the%20full%20job

With strong compaction, [one trajectory is the default]. A separate context
window earns its cost when it buys independent evidence, especially [parallel
discovery] and adversarial review. The primary trajectory integrates those
results. It can [invoke a person or subagent] for distinct judgment, authority,
or context while retaining ownership of the result.

[one trajectory is the default]:
  https://x.com/_lopopolo/status/2051892678254854359
[parallel discovery]: https://x.com/_lopopolo/status/2051893022208708717
[invoke a person or subagent]:
  https://x.com/_lopopolo/status/2075863856275226709

## Carry purpose across the organization and across trajectories

Ownership attaches to the outcome. Ryan describes [an organization-scale OODA
loop]—Observe, Orient, Decide, Act—with horizontal awareness of relevant
organizational activity and vertical awareness across delegated trajectories. A
parent responsible for a migration might delegate a compatibility investigation,
incorporate the child's evidence into its plan, and then decide which batch can
move safely. Tracker state, plans, decision logs, and proof carry that purpose
across compaction, retries, and agent sessions. Unrelated outcomes can proceed
in parallel without dividing responsibility for this one.

[an organization-scale OODA loop]:
  https://x.com/_lopopolo/status/2055110057726263552

[Symphony] gives this topology a concrete form. The issue tracker holds the
deliverable; each independently owned issue receives a dedicated workspace and
agent; dependencies prevent blocked work from starting; and the supervisor
restarts stalled execution. One issue may produce several pull requests or
finish as investigation and analysis without touching code. Sessions and pull
requests remain execution artifacts beneath the tracked result.

[Symphony]:
  https://openai.com/index/open-source-codex-orchestration-symphony/#:~:text=each%20open%20Linear%20issue%20maps%20to%20a%20dedicated%20agent%20workspace

Large outcomes and tight tasks are compatible. Decompose work into
dependency-aware, independently provable pieces while retaining one owner for
the complete result. In the [OpenAI harness-engineering essay], a plan becomes
durable executable context: reviewed separately, updated as evidence changes,
and carried across a long trajectory.

[OpenAI harness-engineering essay]:
  https://openai.com/index/harness-engineering/#:~:text=Plans%20are%20treated%20as%20first-class%20artifacts

## Close the lifecycle

Observation dominates the long middle of many jobs: opening dashboards,
inspecting traces, reproducing behavior, comparing releases, finding hidden
assumptions, and assembling proof. Patch generation is one station in that
lifecycle. [Harness Engineering: How to Build Software When Humans Steer, Agents
Execute] extends the addressable job through QA, user-feedback triage, privacy
and log review, operational runbooks, and product repair.

[Harness Engineering: How to Build Software When Humans Steer, Agents Execute]:
  https://www.youtube.com/watch?v=am_oeAoUhew&t=2615s

For ordinary software work, task closure can include:

1. retrieve relevant context;
2. reproduce or inspect current behavior;
3. implement the root correction;
4. update generated artifacts and public contracts;
5. run focused and system-level proof;
6. adversarially review the result;
7. respond to material review and CI failures;
8. obtain required approval;
9. merge, release, or deploy through the normal protected path; and
10. verify the user-visible or operational result.

Delivery is active work. The agent opens or updates the pull request, waits for
review and CI, fixes material failures and conflicts, and enters the protected
merge path. When the outcome includes a release or deployment, it follows the
proved artifact through approval and cutover, observes the running result, and
stays with failures or user feedback until the promised boundary is verified.

The [literal admin-merge failure] shows why that protected path belongs to the
outcome. Asked to branch, commit, push, and merge, the worker tried to satisfy
the final verb with an administrative bypass even though the intended job
included waiting for tests.

[literal admin-merge failure]:
  https://x.com/_lopopolo/status/2032582327336083831

## Let the outcome choose the artifact

When users reported performance problems in an internal application, the team
collected an exported trace tarball. An on-call engineer spent an afternoon with
Codex building a polished local Next.js trace viewer. The team then realized
Codex could inspect the tarball directly and return the requested analysis in
about five minutes, as recounted in [Extreme Harness Engineering for Token
Billionaires].

[Extreme Harness Engineering for Token Billionaires]:
  https://www.latent.space/p/harness-eng#:~:text=you%20could%20just%20spin%20up%20codex%20and%20give%20it%20the%20tar%20ball

Give the worker the evidence and desired outcome. Its trajectory can decide
whether it needs an observability stack, a parser, a one-off visualization, or
direct analysis. The same interview describes optional skills and scripts that
let Codex boot a local observability stack when traces, logs, or metrics require
it.

Whole-job reasoning can also answer a product question without building the
imagined backend. The first implementation of the Codex app's
scheduled-automations interface coupled a new cron backend to the agent core and
even placed scheduling logic in frontend JavaScript. The team reverted that
tangled implementation. In the [painted-door experiment], they kept the complete
interface over a no-op backend and instrumented the production surface to learn
whether people understood and wanted scheduled automations. That evidence could
determine whether the backend deserved priority.

[painted-door experiment]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=ended%20up%20with%20what%20we%E2%80%99re%20calling%20a%20painted%20door%20style

A task may finish as analysis, an instrumented experiment, a changed operating
procedure, or a decision to leave an imagined system unbuilt. The trace needed
diagnosis, so direct inspection closed the loop. The scheduler question needed
demand evidence, so the painted door closed the loop.

## Spend human attention on ambiguity and authority

Autonomy clears known work and creates room for human judgment in zero-to-one
definition, difficult interfaces, and consequential tradeoffs. Whole-job
delegation applies when the intended outcome, authority boundary, and proof are
legible enough to close. A production service warrants stronger evidence and
tighter authority than an investigative script.

[Software Work Is No Longer Scheduled] draws this operating boundary around
work with a clear desired state, understood interfaces, and verifiable results.
Zero-to-one products, difficult interface refactors, and domains where the
interfaces are unknown still require sustained human judgment. The agent can
investigate and assemble evidence, while the unresolved judgment remains visible
instead of being smuggled into a routine work queue.

[Software Work Is No Longer Scheduled]:
  https://hyperbo.la/w/software-work-not-scheduled/#:~:text=Building%20a%20product%20from%20zero,all%20require%20sustained%20human%20judgment
