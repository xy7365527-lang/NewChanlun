# Influences and Alternate Framings

The through-line is an interest in making systems legible, preserving knowledge
at boundaries, exposing useful feedback, and improving a working environment
incrementally. Ryan Lopopolo's applied-AI framing holds the model and coding
agent constant, then uses context and tools to connect that black box to private
source, operating history, product judgment, current state, access boundaries,
and tacit process. Organizations cannot presume that this private, changing
material will be present in general model weights or that agents will reliably
infer which parts govern their work.

## A short, stable codemap

[matklad's `ARCHITECTURE.md` essay] argues for a compact physical map of a
repository rather than a comprehensive manual. Its codemap should answer
“where's the thing that does X?” while naming stable modules, boundaries,
dependency direction, invariants, and cross-cutting concerns.

[matklad's `ARCHITECTURE.md` essay]:
  https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html#:~:text=Describe%20coarse-grained%20modules%20and%20how%20they%20relate%20to%20each%20other

In a repository harness, that codemap can act as a context-routing layer. A root
guide can remain short because architecture and domain documents give the agent
a mental map, then route it toward details only when a task needs them. The
stability constraint matters as much as the map. Architecture guidance should
describe durable ownership and relationships. Canonical manifests should own
version pins and transient implementation facts. Ryan's [homelab
canonical-manifest enforcement] makes every consumer parse the canonical file
into domain values, while repository tests and lints reject parallel literals
and cross-file disagreement. Parsing and rejection make the manifest the
operative source of truth.

[homelab canonical-manifest enforcement]:
  ../domain-modeling/homelab.md#model-facts-and-parse-canonical-files

## Capability and migration seams

Artichoke Ruby is a modular implementation of the Ruby programming language
written in Rust. Its command-line frontend runs on an interpreter backend which,
at the time, embedded mruby, a small Ruby implementation written in C that
supplied the virtual machine and parser. Rust traits named interpreter
capabilities independently of mruby. The backend implemented those traits and
localized calls into mruby's C API, while separately owned Rust crates
implemented Ruby data structures and standard-library APIs. Frontend callers
could depend on stable capabilities while backend implementations changed.

On 2021-02-07, Ryan added Artichoke's first architecture document and diagram in
[artichoke/artichoke@fbb43b6]. The document mapped the system from the frontend
through the backend, core interpreter traits, data-structure crates, utilities,
and release surfaces. Its commit message explicitly cites matklad's essay.

[artichoke/artichoke@fbb43b6]:
  https://github.com/artichoke/artichoke/commit/fbb43b6ce18556412a489a88e5cc24dea9b587a8

The capability boundary provided a proposed seam for incremental migration. A
replacement could implement one trait-backed behavior without requiring stable
callers to invoke mruby directly. On 2021-02-08, [artichoke/artichoke@1dd8978]
made the Strangler Fig strategy explicit: mruby functions would be disabled and
replaced incrementally while compatible boundaries kept the remainder working.
On 2022-08-24, [artichoke/artichoke@fa5b2eb] removed a literal mruby version so
the architecture document would remain true across dependency updates. Together
the commits record an intended migration seam: Artichoke continued using mruby
while compatible behaviors could be replaced incrementally, and the architecture
description stayed stable across backend version changes.

[artichoke/artichoke@1dd8978]:
  https://github.com/artichoke/artichoke/commit/1dd8978fe4edf1971928fd00e30edbd124fd0aaa
[artichoke/artichoke@fa5b2eb]:
  https://github.com/artichoke/artichoke/commit/fa5b2eb907d8746efd7cf51f07cd84e7e1f9b49f

The sequence illustrates three harness-relevant habits:

- architecture maps durable ownership and relationships;
- behavior is organized behind capability-shaped boundaries; and
- large migrations proceed through explicit seams that preserve a working
  system.

## Parse at the boundary

Alexis King's [“Parse, don't validate”] supplies the typed boundary discipline.
A parser converts less-structured input into a more precise representation that
preserves what was learned. Once the boundary has produced a valid domain value,
downstream code does not need to repeat ad hoc checks or retain impossible
states.

[“Parse, don't validate”]:
  https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/#:~:text=consumes%20less-structured%20input%20and%20produces%20more-structured%20output

In a repository harness, manifests, CLI arguments, workflow files, remote
targets, and generated configuration are external syntax that should be parsed
once into semantic values. Policy then asks domain questions of those values:
which capability is present, which version is canonical, whether a target is
local or remote, which operation is authorized, or whether two manifests agree.
This prevents version policy from degenerating into duplicated constants and
serialized fixture text.

Typed boundary parsing also improves feedback. A sensor that understands the
domain can report the violated relationship and the intended repair. A string
comparison can usually report only that bytes differ.

## Close feedback loops around judgment

Ryan's 2026-02-11 [seminal harness-engineering essay] described the practice and
left architectural coherence over years as an open question. Birgitta Böckeler's
2026-02-17 [initial memo] responded through context, deterministic constraints,
LLM review, and recurring feedback.

[seminal harness-engineering essay]:
  https://openai.com/index/harness-engineering/#:~:text=What%20we%20don%E2%80%99t%20yet%20know%20is%20how%20architectural%20coherence%20evolves%20over%20years
[initial memo]:
  https://martinfowler.com/articles/exploring-gen-ai/harness-engineering-memo.html#:~:text=mix%20deterministic%20and%20LLM-based%20approaches%20across%203%20categories

George Zhang's 2026-03-07 [“Harness Engineering Is Cybernetics”] reads the
OpenAI experiment as a change in where a feedback loop can close. Compilers,
tests, and linters already detect mechanically observable deviations. A capable
coding agent can also inspect and repair architecture and design.
Repository-specific context and controls calibrate that sensor-actuator loop
against the system's desired state.

[“Harness Engineering Is Cybernetics”]:
  https://x.com/i/article/2030414577213820928

Böckeler's 2026-04-02 article develops a complementary operational vocabulary.
[Guides provide feedforward context] before the agent acts; [sensors observe
results] and enable self-correction afterward. Either may be computational, such
as a type checker or structural test, or inferential, such as a focused reviewer
agent. The article describes the harness as a [cybernetic governor] and uses
Ashby's Law of Requisite Variety to explain why [constrained service topologies]
can be easier to regulate. The law says that a regulator needs enough possible
responses to match the variety of states the governed system can enter.
Standardizing on fewer service topologies reduces that state space, allowing a
finite collection of guides and sensors to cover more of the behavior that can
occur.

[Guides provide feedforward context]:
  https://martinfowler.com/articles/harness-engineering.html#:~:text=Guides%20(feedforward%20controls)%20-%20anticipate%20the%20agent%27s%20behaviour
[sensors observe results]:
  https://martinfowler.com/articles/harness-engineering.html#:~:text=Sensors%20(feedback%20controls)%20-%20observe%20after%20the%20agent%20acts%20and%20help%20it%20self-correct
[cybernetic governor]:
  https://martinfowler.com/articles/harness-engineering.html#:~:text=The%20agent%20harness%20acts%20like%20a%20cybernetic%20governor
[constrained service topologies]:
  https://martinfowler.com/articles/harness-engineering.html#:~:text=committing%20to%20a%20topology%20narrows%20that%20space%2C%20making%20a%20comprehensive%20harness%20more%20achievable

Both are later interpretations of Ryan's essay. Zhang makes the higher-level
control loop and its calibration explicit; Böckeler separates controls by
direction, execution type, lifecycle timing, and the quality they regulate.

The vocabulary makes a closed loop concrete. An architecture rule guides; a lint
that rejects violations senses. A runbook guides a consequential operation; a
canary, access check, and rollback verification sense whether it remains safe.
Anticipated constraints shape the attempt, observed results correct it, and
repeated failures improve the next guide or sensor.

The worker held constant for a harness intervention is the selected model plus
the coding agent that runs its tool loop. Model-native semantics include the
tool names and arguments the model sees, the results and errors returned to it,
approval behavior, and session lifecycle. The harness intervention changes the
repository context, tools, access, validators, and surrounding environment while
keeping that worker configuration stable long enough to attribute an outcome.

The [Agent Client Protocol] (ACP) addresses the client-agent boundary. It is a
JSON-RPC protocol through which an editor can connect to a coding agent, receive
session and tool-call updates, and answer permission requests while the agent
retains responsibility for its model and internal tool loop. ACP can carry a
worker through a different client surface; an adapter still has to preserve any
model-native semantics that matter to that worker.

[Agent Client Protocol]:
  https://agentclientprotocol.com/get-started/architecture#:~:text=ACP%20makes%20heavy%20use%20of%20JSON%2DRPC%20notifications

## Incremental adoption

Martin Fowler's [“Strangler Fig”] describes a “gradual process of
modernization”: introduce seams, move behavior in manageable pieces, and keep
delivering while the old and new systems coexist.

[“Strangler Fig”]:
  https://martinfowler.com/bliki/StranglerFigApplication.html#:~:text=The%20alternative%20that%20my%20colleagues%20and%20I%20prefer%2C%20is%20to%20do%20a%20gradual%20process%20of%20modernization

Harness engineering can follow the same adoption pattern. A team can begin with
its most expensive human relay or most repeated failure: add a codemap, expose
one domain command, parse one canonical manifest, encode one review rule, make
one runtime surface observable, or separate one dangerous cutover from
autonomous preparation. Each step should leave the repository more legible and
the normal path more constrained than before.

Ryan's [private homelab infrastructure repository] records this sequence in
operational controls. After remote-access upgrade work exposed ways to lose the
only repair path into a robot vacuum, its runbook gained an isolated canary: a
second Tailscale identity and separate state prove that the replacement daemon
boots, joins the network, and permits SSH before it replaces production. A
separate report-only documentation-freshness role compares operator pages with
the configuration and monitoring sources that own the described behavior. It
names contradictory evidence and proposes an edit, then waits for human
authorization before changing the repository.

[private homelab infrastructure repository]: ../domain-modeling/homelab.md

Artichoke's capability seams and the homelab's runbooks arose around concrete
migrations and operations. Review and incident lessons then moved into types,
documentation, tests, lints, and state machines. Each addition left a working
system in place while making the next change easier to route and verify.

## Evolution across Ryan’s work

The implementation strategy changes as worker capability and deployment surfaces
improve. The underlying problem—situating a general model in local work—remains
stable.

### Manual pairing becomes whole-job autonomy

In [“I Wrote 4,000 Lines of Code with ChatGPT in a Weekend”], Ryan copied code
and test output between an editor and ChatGPT and personally relayed every
action. By 2026, the [RustSec case] and [robot-vacuum case] gave the agent
direct access to reproduce behavior, implement a change, run tests, prepare
delivery, and assemble proof. The [artichoke/intaglio#360 review record]
describes substantial implementation review: the reviewer requested a dedicated
module, explicit state transitions, a consuming terminal method, clearer names,
and focused tests, and Codex revised the patch. Ryan's [artichoke/intaglio#360
human approval] then authorized the merge, point release, and RustSec report.
The evolution reduced manual relay while preserving implementation judgment and
release authority.

[“I Wrote 4,000 Lines of Code with ChatGPT in a Weekend”]:
  https://hyperbo.la/w/chatgpt-4000/#:~:text=I%20interacted%20with%20ChatGPT%20by%20copying%20code%20in%20vim
[RustSec case]: ../proof/rustsec.md
[robot-vacuum case]: ../domain-modeling/homelab.md
[artichoke/intaglio#360 review record]:
  https://github.com/artichoke/intaglio/pull/360#issuecomment-4158222124
[artichoke/intaglio#360 human approval]:
  https://github.com/artichoke/intaglio/pull/360#issuecomment-4158273357

### Proposed specialists become a fixed worker with retrieval

In the 2023 article “I Wrote 4,000 Lines of Code with ChatGPT in a Weekend,”
Ryan [proposed off-the-shelf specialists]: one trained on Rust documentation and
release material, one for every dependency crate, one for Ruby Core and Standard
Library APIs, one for Ruby Spec, and one for the local codebase and its history.
The later harness practice selects a capable general model-and-agent worker and
supplies the relevant code, history, process data, and tools just in time. The
goal of situated expertise remains. The 2023 proposal placed that expertise in
separately trained agents; the later route curates the environment around a
fixed worker.

[proposed off-the-shelf specialists]:
  https://hyperbo.la/w/chatgpt-4000/#:~:text=Dreaming%20big%2C%20I%E2%80%99d%20love%20to%20see%20a%20workflow

### Tool discovery gains progressive disclosure

The 2025 essay [“MCP Solves Tool Discovery for LLMs”] argues that the Model
Context Protocol (MCP) makes capabilities visible by placing tool names,
descriptions, input schemas, and examples into the model's context. [Ryan's 2026
note on progressive disclosure] revisits the context cost of that mechanism:
loading an entire tool catalog resembles loading every manual page, while a
familiar command-line tool can expose detail progressively through `--help` only
when the worker asks for it. Both mechanisms address the same requirement: the
worker must receive enough information to discover an action and invoke it
correctly without consuming attention on every unavailable or irrelevant
operation.

[“MCP Solves Tool Discovery for LLMs”]:
  https://hyperbo.la/w/tool-discovery/#:~:text=names%2C%20descriptions%2C%20input%20schemas%2C%20and%20example%20calls
[Ryan's 2026 note on progressive disclosure]:
  https://x.com/_lopopolo/status/2026798214297571352

### “Code is free” acquires ownership and proportionality

Early 2026 writing, including the [hyperbo.la build], uses cheap implementation
to justify exhaustive migrations, local lints, and 100 percent coverage. Later
work draws a firmer boundary around what abundance changes. [Software Work Is No
Longer Scheduled] assigns work with a clear desired state, understood
interfaces, and verifiable results to asynchronous agent execution while
preserving sustained human judgment for zero-to-one products, difficult
interface refactors, and domains where the interfaces are unknown. The [Latent
Space interview] likewise describes difficult deep refactors as an open
problem. Product fit, compatibility, security, proof, and maintenance remain
expensive even when another implementation attempt is cheap.

[hyperbo.la build]:
  https://hyperbo.la/w/harness-engineering-the-blog-build/#:~:text=codify%20repo%20contracts%20as%20tools%20and%20lints%20instead%20of%20tribal%20memory
[Software Work Is No Longer Scheduled]:
  https://hyperbo.la/w/software-work-not-scheduled/#:~:text=Building%20a%20product%20from%20zero,all%20require%20sustained%20human%20judgment
[Latent Space interview]:
  https://www.latent.space/p/harness-eng#:~:text=where%20you%20don%E2%80%99t%20know%20what%20the%20proper%20shape%20of%20the%20interfaces%20are

The operating environment carries cost too. Ryan’s [ablation framing] removes
context or tools and measures whether performance regresses. This makes each
instruction and control justify the attention and maintenance it consumes
instead of treating accumulation as progress.

[ablation framing]: https://x.com/_lopopolo/status/2049145174790725654

### Utilization becomes a diagnostic within effectiveness

[Agents, Agents, Agents] defines a billion tokens per engineer per day as a
utilization target: a probe for how much of the software lifecycle agents are
allowed to observe and operate. That measures access, addressability, and agent
activity. Later posts make the outcome measure explicit: [token spend is
unanchored in business value or return on investment], and [effectiveness is
what matters]. Utilization can reveal a missing tool, inaccessible system, or
human relay that leaves addressable work idle. Effectiveness asks whether the
completed work creates sufficient value for its cost.

[Agents, Agents, Agents]:
  https://hyperbo.la/w/agents-agents-agents/#:~:text=A%20billion%20tokens%20per%20engineer%20per%20day%20is%20a%20utilization%20target
[token spend is unanchored in business value or return on investment]:
  https://x.com/_lopopolo/status/2035102053979271654
[effectiveness is what matters]:
  https://x.com/_lopopolo/status/2035132736621748332
