# Hold the Worker Constant

Harness engineering, the practice of improving agent output by shaping the
environment around it, holds a chosen model and coding agent constant while
changing context and tools. The constant lasts for one evaluation or deployment
epoch. A change to the model, coding-agent runtime, or native action interface
opens a new epoch and requires the environment to be qualified again.

The [coding-agent boundary] and [game-engine distinction] define the black box.
Model and coding-agent engineering produce workers. Harness engineering deploys
a selected worker into a particular body of work.

[coding-agent boundary]: https://x.com/_lopopolo/status/2051039234346438723
[game-engine distinction]: https://x.com/_lopopolo/status/2051058498235195856

Within that boundary, context gives the worker intent, local ontology, current
state, constraints, examples, and definitions of good work. Tools let it search,
inspect, transform, run, observe, and deliver in the systems where that context
matters. The environment curates both through repository topology, architecture,
permissions, tests, logs, graders, and canaries. [Route Context Just in Time]
and [Make Capabilities Legible and Operable] develop the two levers.

[Route Context Just in Time]: ../just-in-time-context/
[Make Capabilities Legible and Operable]: ../tool-legibility/

## Work in adoption epochs

Holding the worker constant makes failures actionable. A capable worker that
cannot complete a job may lack the relevant institutional context, an operable
capability, a useful feedback loop, sufficient authority, or evidence at the
outcome boundary. Those are properties of the deployment environment that a team
can inspect and improve.

Record the worker configuration for each epoch: the model, coding-agent host and
version, native tools, and runtime capabilities such as compaction, background
processes, and computer use. Qualify it with representative whole-job journeys.
Repeated runs matter because a fixed worker is still stochastic; the boundary
identifies the condition being evaluated while repeated trials account for
stochastic variation.

The internal product described in the [OpenAI harness-engineering essay] grew
from an empty repository to roughly one million lines of code over five months;
the [Latent Space interview] reports about 1,500 merged pull requests. Codex
wrote all of its code; people did not contribute code manually. The engineers
improved the repository, browser, observability, review, and delivery surfaces
until Codex could launch the application, observe its behavior, respond to
review, and deliver a verified change. That constraint made missing environment
capability concrete.

[OpenAI harness-engineering essay]:
  https://openai.com/index/harness-engineering/#:~:text=Throughout%20the%20development%20process%2C%20humans%20never%20directly%20contributed%20any%20code
[Latent Space interview]:
  https://www.latent.space/p/harness-eng#:~:text=You%20kicked%20off%20as%20a%20team%20of%20three%20people.%20You%E2%80%99re%20putting%20out%20a%20million%20line%20of%20code%2C%20like%201500%20prs

## Requalify every worker upgrade

Human priors are part of the adoption state. Earlier workers teach a team how
small a task should be, how much orchestration it needs, how long an inner loop
may take, and where a person must hover. Carrying those habits into a new epoch
can leave a [capability overhang] unused. Assuming every point release improves
every dimension creates the opposite error.

[capability overhang]: https://x.com/_lopopolo/status/2031810853801247190

Re-run complete jobs and [reset those priors regularly]. Adapt prompts, skills,
documentation, tools, and proof to the new worker's instruction following,
reasoning, compaction, tool reliability, and failure modes. Ryan Lopopolo
describes this as [manicuring prompts, skills, and docs] for each release in a
model family.

[reset those priors regularly]:
  https://x.com/_lopopolo/status/2062192014561603850
[manicuring prompts, skills, and docs]:
  https://x.com/_lopopolo/status/2055157677496443281

The [AI Native DevCon talk] treats point releases as deployment changes: the
attainable work can move enough that the surrounding operating environment needs
to be reconsidered. [GPT-5.2 shipped while Ryan Lopopolo was on winter break].
When he returned, the team was producing one or two additional pull requests per
engineer each day without additional harness investment. A later, distinct
Symphony harness intervention reported a tenfold gain in pull requests per
engineer per week. The qualification loop determines whether the environment
should remain stable, adapt, or simplify.

[AI Native DevCon talk]:
  https://tessl.io/registry/ainativedev/aidevcon-2026-ldn/0.100.8/files/talk-lopopolo-harness-engineering/transcript.md#:~:text=retooling%20your%20stack%20in%20the%20way%20you%20work%20with%20every%20point%20release
[GPT-5.2 shipped while Ryan Lopopolo was on winter break]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=when%20I%20came%20back%20without%20any%20additional%20investment

Recalibrate ambition in the same cycle. Probe progressively larger outcomes
until proof begins to fail. One [proposed high-ambition prompt] asks a worker to
search the open-source ecosystem for changes that would improve `libghostty`.
The proposal probes whether the worker can discover and close over
cross-repository work. In a separate operator observation, Ryan reports that
GPT-5.3 was [less willing than GPT-5.2 to attempt difficult work]. The
evaluation should discover the new outcome boundary instead of preserving an old
one by habit.

[proposed high-ambition prompt]:
  https://x.com/_lopopolo/status/2075861840085848202
[less willing than GPT-5.2 to attempt difficult work]:
  https://x.com/_lopopolo/status/2030441273900097933

## Treat inner-loop latency as part of qualification

A worker reasons from the feedback it can obtain before its trajectory loses
focus or patience. Build, test, browser, and observability latency therefore
shape usable capability. The GPT-5.2 Codex harness lacked background shells, so
the worker would wait for blocking build scripts. GPT-5.3 added background
shells and was less willing to block on the same loop. Over one week, the team
moved the repository from a bespoke Makefile through Bazel, Turbo, and Nx. They
stopped when a complete build finished in under one minute. The team adopted the
[sub-minute build loop] as a round upper bound. A breach told them to stop the
current work and decompose the build graph until the loop fit the bound again.

[sub-minute build loop]:
  https://www.latent.space/p/harness-eng#:~:text=retool%20the%20entire%20build%20system%20to%20complete%20in%20under%20a%20minute

Retest that threshold for every epoch. Fast focused checks support exploration;
broader proof can run later without blocking the worker's next useful action.
Measure accepted outcomes as well as speed so a shorter loop does not merely
produce incorrect work faster.

## Retire scaffolding the worker has absorbed

Prefer interventions that remain visible through the worker's native loop:
architecture, types, tests, tool contracts, observable state, and acceptance
criteria. Bespoke mediation can be valuable during one epoch and obsolete in the
next. In the [native computer-use example], native computer use let the team
discard a virtual display and video-capture stack while preserving end-to-end
application validation.

[native computer-use example]:
  https://www.aakashg.com/how-pms-ship-100k-lines-of-code/#:~:text=get%20to%20throw%20away%20all%20that%20bespoke%20code

Requalification includes subtraction. Remove scaffolding that no longer improves
complete journeys, keep durable constraints that still express the job, and let
newly native capabilities simplify the environment. A coding agent's action
language is part of the worker configuration; [Model-Native Semantics Across
Agent Hosts] develops the migration surface between adoption epochs.

[Model-Native Semantics Across Agent Hosts]: model-native-semantics.md
