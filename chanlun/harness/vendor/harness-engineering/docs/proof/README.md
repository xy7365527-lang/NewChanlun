# Prove the Outcome in the Real Environment

Verification belongs in the job. A change is complete when the evidence shows
the promised result in the environment where a user, operator, or dependent
system will rely on it.

## Define success where it will be experienced

Start by naming the product features and the journeys through them. Those
journeys give implementation, review, and release a shared target that an agent
can execute and observe.

In the [Aakash Gupta interview], a new engineer who had previously worked as a
product manager noticed that the repository did not document the product's
features. Codex crawled the codebase and drafted a feature inventory; people on
the team reviewed it against their product knowledge. The reviewed inventory
made the critical user journeys explicit, and QA agents could then boot the app,
exercise those journeys, and assert that they still worked. Deployment required
less manual smoke testing because one teammate's observation had become a
repeatable acceptance suite.

[Aakash Gupta interview]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=we%20had%20no%20documentation%20in%20the%20codebase%20for%20what%20product%20features%20we%20had

Define the acceptance boundary before implementation:

- who or what experiences the outcome;
- which starting state and inputs matter;
- which visible behavior and side effects must occur;
- which invariants must remain true;
- how the observer will distinguish success from a plausible imitation; and
- what remains outside the claim.

This specification can stay small. Its job is to identify the evidence the
worker must be able to produce.

## Match evidence to the claim

Map each green check to the assertion it actually makes. Then gather evidence at
the affected boundary.

| Claim                       | Evidence at the claim boundary                                                  |
| --------------------------- | ------------------------------------------------------------------------------- |
| browser behavior            | a real browser journey with semantic and rendered state                         |
| generated content           | complete source-to-output comparison and freshness                              |
| compatibility migration     | target-corpus parity, edge cases, and explicit accepted differences             |
| security impact             | a reproducer, bounded exploitability, and regression coverage                   |
| deployment                  | the validated artifact running with post-deploy health                          |
| consequential remote change | staged canary, cutover, recovery, and post-cutover checks                       |
| spreadsheet calculation     | formulas, units, source cells, and rendered workbook state                      |
| dataset transformation      | schema, provenance, and source-to-output reconciliation                         |
| document or presentation    | required structure and rendered pages; citations or accessibility when promised |
| analytical conclusion       | reproducible transformation and conclusions supported by evidence               |
| business-state mutation     | required approval, receipt, and observed postcondition                          |

Unit tests, type checks, lints, and builds establish important internal
properties. Standards compatibility needs conformance evidence. Semantic
completeness needs source-to-output comparison. A successful upload needs a
healthy deployment before it supports a release claim.

When intermediate code stays inside an agent trajectory, the proof packet speaks
in the user’s domain. A spreadsheet formula, rendered slide, cited finding, or
completed state transition carries the claim. Passing tests for the hidden
script establish only internal properties. The result is established at [the
user’s domain boundary].

[the user’s domain boundary]:
  ../last-mile-deployment/#serve-users-who-never-see-code

## Give the agent access to the real system

The agent should be able to launch the application, drive the browser or UI,
read logs, query metrics and traces, inspect persistent side effects, compare
generated corpora, and observe CI and deployment. A person should not have to
relay evidence the agent can inspect directly.

The [canonical harness-engineering essay] describes per-worktree application
instances and ephemeral observability stacks that let Codex reproduce bugs,
query logs and metrics, drive the UI, and validate its own changes. [The
Production Function Changed] carries the same requirement into review: attach
test results, a QA plan, staging logs, screenshots, or video that demonstrate
the result before asking for review.

[canonical harness-engineering essay]:
  https://openai.com/index/harness-engineering/#:~:text=we%20made%20the%20app%20bootable%20per%20git%20worktree
[The Production Function Changed]:
  https://hyperbo.la/w/production-function-changed/#:~:text=The%20agent%20should%20attach%20those%20validation%20artifacts%20to%20the%20PR%20before%20asking%20for%20review.

Use semantic and rendered evidence together. DOM snapshots and structured state
are efficient for roles, values, and relationships. Screenshots, video, or
computer use expose layout, clipping, focus, rendering, and interaction defects
that the semantic view omits. Logs and traces establish runtime behavior that a
visual demonstration cannot.

Ryan Lopopolo's OpenAI team exposed one such surface inside its Electron
application. The [Electron component workbench] was a launchable development
window containing the application's native rendering canvas and full
design-system component library. Codex could compose new screens from the
production components, render them in the surface the team would ship, and
return screenshots to a designer for review.

[Electron component workbench]:
  https://tessl.io/podcast/109/#:~:text=One%20neat%20way%20that%20we%20made%20that%20idea%20real,give%20them%20to%20our%20designer

The team also let Codex start a [production-like observability stack] for local
metrics and logs. Its [local browser shell] mounted the UI with Chrome DevTools
already connected, giving the worker the box model, console, and other
browser-native observations. Ryan described built-in computer use as the
approach he would use today to inspect the application, click through a journey,
and observe whether the intended side effects occur.

[production-like observability stack]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=What%20we%E2%80%99ve%20done%20instead%20is%20permit%20codeex%20to%20spin%20up,diagnose%20issues%20in%20local%20dev
[local browser shell]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=We%20mount%20the%20UI%20into%20a%20local%20browser%20shell,that%20it%20works%20end%20to%20end

Another complementary technique pairs [semantic and rasterized views]. A
semantic representation helps the model identify interface objects and their
roles. Rasterizing the interface exposes spatial relationships such as layout,
clipping, and occupied space. Together they give the worker both the named
objects and the rendered geometry it is manipulating.

[semantic and rasterized views]:
  https://www.latent.space/p/harness-eng#:~:text=We%20want%20the%20agent%20to%20be%20able%20to%20see%20the%20UI,object%20it%E2%80%99s%20manipulating

Direct access also lets the agent choose the smallest useful inspection tool.
The agent may query a trace archive directly when a human-facing dashboard would
only add an intermediate artifact and another relay. The [Latent Space
interview] recounts a teammate spending an afternoon on a polished trace viewer
before discovering that Codex could answer the question from the trace tarball
itself in about five minutes.

[Latent Space interview]:
  https://www.latent.space/p/harness-eng#:~:text=you%20could%20just%20spin%20up%20codex%20and%20give%20it%20the%20tar%20ball

## Compress the trajectory for review

Reviewers need enough evidence to assess the outcome without replaying an entire
session. The same interview describes PR video as a compression of the work: a
teammate presents the evidence needed to make the merge decision instead of
asking the reviewer to shoulder-surf the implementation.

A useful proof packet contains:

- the intended outcome and affected boundary;
- material design and risk decisions;
- the exact tests and journeys that ran;
- screenshots, video, logs, traces, diffs, or reproducers that carry the claim;
- known limits and unproved behavior; and
- the identity of the artifact proposed for delivery.

Plans may coordinate the trajectory. Because [plans are not shipped], the proof
packet covers the behavior, side effects, and exact artifact promised to the
user.

[Plans are not shipped]: https://x.com/_lopopolo/status/2048945121815867857

The packet should be concise because reviewer attention is scarce. More evidence
helps only when it changes confidence in the claim.

## Preserve artifact identity through delivery

Release proof follows one immutable artifact from validation through cutover.
Rebuilding in a more privileged job breaks that chain: the deployed bytes are no
longer the bytes the earlier evidence covered.

[Release Integrity] develops the complete path: build once, record provenance,
promote the same artifact, verify the running system, and preserve rollback. A
post-deploy check belongs to the proof because successful delivery is an
operational claim about the running boundary.

[Release Integrity]: release-integrity.md

The [homelab canary] applies this principle to a remote-access upgrade. A
successful build could not establish that the upgraded machine would remain
reachable. The proof proceeds through an isolated canary identity, an
independently verified access path, a backed-up production cutover, and
post-cutover access and log checks.

[homelab canary]: ../domain-modeling/homelab.md

## Give the agent ground truth it can test against

Objective graders make iteration cheap when the domain supplies them. [Miri] is
an interpreter for Rust's mid-level intermediate representation that can run a
binary or test suite and report classes of undefined behavior, including
out-of-bounds access, use-after-free, invalid use of uninitialized data, and
some aliasing violations. In his [Codex-and-Miri work], Ryan has used Miri as
executable ground truth while Codex searches popular Rust crates for unsound
executions. A clean Miri run is limited to the executions it explored; a
reported violation gives the worker a concrete failure to reproduce and repair.

[Miri]: https://github.com/rust-lang/miri
[Codex-and-Miri work]: https://x.com/_lopopolo/status/2047102222530728035

[Accessibility standards and assistive technology] provide concrete quality
checks. In a [CTF-style UI exercise]—“CTF” means capture the flag—the team
plants markers behind real application workflows. A skill succeeds by driving
the interface and capturing the marker, giving repeated improvement runs an
executable win condition. Compatibility vectors, canary health, and complete
corpus comparisons can likewise turn vague goals into observable outcomes.

[Accessibility standards and assistive technology]:
  https://x.com/_lopopolo/status/2054987836731150606
[CTF-style UI exercise]: https://x.com/_lopopolo/status/2047052450604212728

The hard part is building a validator that rewards the real outcome and resists
gaming. Ryan describes this as the need for an [unhackable grader].
Practitioners also carry [hard cases from their own work]. Encoding those cases
as repeatable local trials complements broad public benchmarks with the team's
own artifacts and risks.

[unhackable grader]: https://x.com/_lopopolo/status/2056991142093508831
[hard cases from their own work]:
  https://x.com/_lopopolo/status/2059378830939402614

Nonfunctional requirements become proof obligations at the boundary they govern.
Representative evidence can include compatibility against a relevant corpus,
version matrix, or specification; performance against a named workload and
threshold; accessibility conformance checks plus assistive-technology or user
journeys; and security controls evaluated against a threat model. Qualitative
requirements such as tone, taste, risk tolerance, polish, and acceptable
shortcuts become more reliable when a team names its judgment and gives
reviewers a convergence policy. [What Does It Mean to Do a Good Job?] develops
that part of verification.

[What Does It Mean to Do a Good Job?]:
  https://hyperbo.la/w/what-does-it-mean-to-do-a-good-job/#:~:text=tone%2C%20taste%2C%20risk%20tolerance%2C%20how%20much%20polish%20is%20enough%2C%20what%20shortcuts%20are%20acceptable

[Evaluate the Harness] separates outcome, proof, architecture, and trajectory
cost so a convenient proxy cannot silently replace the job being measured.

[Evaluate the Harness]: ../../evals/

## Say what the evidence did not establish

Security evidence should state the conditions under which a reproducer works and
the conclusions it supports. In [Prove a Security Claim], corruption required a
custom hasher and recovery after panic; the reproducer did not establish memory
unsafety on the default path. Codex developed the report, reproducer, patch,
tests, and responses to substantive implementation review. Ryan reviewed the
implementation, approved it, and separately authorized the merge, point release,
and RustSec report. The operative instruction was to [prove impact or
exploitability] for every reported issue; speculative security-shaped output did
not satisfy the job.

[Prove a Security Claim]: rustsec.md
[prove impact or exploitability]:
  https://x.com/_lopopolo/status/2038739679542215136

The proof burden scales with consequence. A typo, a parser migration, a remote
upgrade, and a production release need different evidence and authority. The
[Repository Review Playbook] follows those claims through representative
repository journeys and records any boundary the available evidence does not
reach.

[Repository Review Playbook]: ../../playbooks/repository-review.md
