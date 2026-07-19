# Ryan Lopopolo’s Twitter Corpus

This index groups Ryan Lopopolo’s captured public posts by recurring working
principle. The [thesis pages] develop the arguments with long-form writing,
talks, interviews, cases, and this post evidence.

[thesis pages]: ../../docs/

The adjacent [machine-readable capture] preserves 764 posts with date, URL,
reply relation, and public provenance. It includes recovered text for 754 posts.
Ten quote-only records preserve resolved quote relationships and, where present,
media URLs; they do not copy or license third-party quoted text. Use the corpus
to trace a claim, search Ryan’s original language, or extend this readable
index.

[machine-readable capture]: lopopolo-public-x-2026.json

## Operating principles

### 1. Delegate outcomes with sparse prompts

Begin with the outcome and its nonfunctional requirements. Infer the question
behind the question, accept sparse prompts, and choose an ambitious scope the
agent can close over without step-by-step orchestration. Skill instructions
remain subordinate to the outcome.

- [“We’ve gone too heavy on skill instruction following”]
- [“Understanding the question behind the question”]
- [Hidden nonfunctional requirements are the question behind the question]
- [A literal merge prompt led to an attempted admin merge]
- [`/goal` is for processes and outcomes, not one PR]
- [“I put very close to zero constraints in my prompts”]
- [“Lazy prompts are the point”]
- [Prompt lazily, observe, improve, discard, and reroll]
- [Ask for ecosystem-wide gaps that would make a library better]

[“We’ve gone too heavy on skill instruction following”]:
  https://x.com/_lopopolo/status/2030395671166230796
[“Understanding the question behind the question”]:
  https://x.com/_lopopolo/status/2030396192832819524
[Hidden nonfunctional requirements are the question behind the question]:
  https://x.com/_lopopolo/status/2032582601110913515
[A literal merge prompt led to an attempted admin merge]:
  https://x.com/_lopopolo/status/2032582327336083831
[`/goal` is for processes and outcomes, not one PR]:
  https://x.com/_lopopolo/status/2051758630312280085
[“I put very close to zero constraints in my prompts”]:
  https://x.com/_lopopolo/status/2054053216254636073
[“Lazy prompts are the point”]:
  https://x.com/_lopopolo/status/2054055856594161709
[Prompt lazily, observe, improve, discard, and reroll]:
  https://x.com/_lopopolo/status/2048901233721979363
[Ask for ecosystem-wide gaps that would make a library better]:
  https://x.com/_lopopolo/status/2075861840085848202

### 2. Give one agent ownership of the whole lifecycle

Give one agent responsibility for discovery, implementation, verification,
deployment, maintenance, and coordination with code owners. Fork unrelated work
and use subagents for bounded parallel discovery. The owner carries purpose and
system guarantees across trajectories and may invoke a human as a tool when
needed.

- [“Get the machine to do our whole job”]
- [“Do the reasoning. Don’t do the tedious thing”]
- [Whole-job closure includes code-owner approvals]
- [Benchmarks create machines that produce code, not software engineers]
- [A one-rollout benchmark attempt is not representative of real work]
- [“Producing a codebase in one shot is fundamentally uninteresting”]
- [Scale the agent’s OODA loop across trajectories and the organization]
- [“You almost never want subagents” when compaction is strong]
- [Parallel discovery is a legitimate exception]
- [Fork unrelated tasks into another session]
- [Prefer one agent when orchestration cannot guarantee convergence]
- [Let the owner invoke a human or subagent as a bounded tool]
- [Post-merge review at agent-team scale]

[“Get the machine to do our whole job”]:
  https://x.com/_lopopolo/status/2030432109631004964
[“Do the reasoning. Don’t do the tedious thing”]:
  https://x.com/_lopopolo/status/2048801220014608849
[Whole-job closure includes code-owner approvals]:
  https://x.com/_lopopolo/status/2050329854680453375
[Benchmarks create machines that produce code, not software engineers]:
  https://x.com/_lopopolo/status/2054960622140326312
[A one-rollout benchmark attempt is not representative of real work]:
  https://x.com/_lopopolo/status/2054969017140752758
[“Producing a codebase in one shot is fundamentally uninteresting”]:
  https://x.com/_lopopolo/status/2054979794429956514
[Scale the agent’s OODA loop across trajectories and the organization]:
  https://x.com/_lopopolo/status/2055110057726263552
[“You almost never want subagents” when compaction is strong]:
  https://x.com/_lopopolo/status/2051892678254854359
[Parallel discovery is a legitimate exception]:
  https://x.com/_lopopolo/status/2051893022208708717
[Fork unrelated tasks into another session]:
  https://x.com/_lopopolo/status/2052024476645409006
[Prefer one agent when orchestration cannot guarantee convergence]:
  https://x.com/_lopopolo/status/2064372718095487137
[Let the owner invoke a human or subagent as a bounded tool]:
  https://x.com/_lopopolo/status/2075863856275226709
[Post-merge review at agent-team scale]:
  https://x.com/_lopopolo/status/2037291250072932493

### 3. Route context just in time

Treat disk as the durable knowledge store and attention as scarce. Repository
maps explain what matters and where to look; search, CLI help, quiet tool
output, and compact diagrams page in the relevant context. Compaction extends
the work horizon while the agent continues to retrieve context as needed.

- [The repository is an embedded knowledge base]
- [Make human-oriented tools quiet for agents]
- [“Disk is an infinite context sink”]
- [“The other scarce resource is attention”]
- [Autocompaction enables headless, long-horizon work]
- [Mermaid beats walls of text for humans and agents]
- [Context limits may be solved while attention remains scarce]
- [Put nonfunctional requirements in retrievable context and reduce context
  demand]
- [Make tokens predictable with the fewest instructions]
- [Lints and tests page context in as required]
- [Specify how agents should seek context and use their tools]
- [Deterministic context stuffing fails across many compactions]
- [Tell the agent what, why, and where; otherwise let it cook]

[The repository is an embedded knowledge base]:
  https://x.com/_lopopolo/status/2026812047581941898
[Make human-oriented tools quiet for agents]:
  https://x.com/_lopopolo/status/2030357237731037528
[“Disk is an infinite context sink”]:
  https://x.com/_lopopolo/status/2042702983893229946
[“The other scarce resource is attention”]:
  https://x.com/_lopopolo/status/2037315374467727667
[Autocompaction enables headless, long-horizon work]:
  https://x.com/_lopopolo/status/2046606006470533299
[Mermaid beats walls of text for humans and agents]:
  https://x.com/_lopopolo/status/2048803314683576527
[Context limits may be solved while attention remains scarce]:
  https://x.com/_lopopolo/status/2052026275326513224
[Put nonfunctional requirements in retrievable context and reduce context demand]:
  https://x.com/_lopopolo/status/2030356581792293212
[Make tokens predictable with the fewest instructions]:
  https://x.com/_lopopolo/status/2031861303917363319
[Lints and tests page context in as required]:
  https://x.com/_lopopolo/status/2030356864639410639
[Specify how agents should seek context and use their tools]:
  https://x.com/_lopopolo/status/2062950475989725465
[Deterministic context stuffing fails across many compactions]:
  https://x.com/_lopopolo/status/2062966895876215172
[Tell the agent what, why, and where; otherwise let it cook]:
  https://x.com/_lopopolo/status/2064742429438198056

### 4. Store operating knowledge in the repository

Keep repeatable operations in checked-in, versioned runbooks. Use skills to
teach the model an approach and decision frame, with descriptions that explain
their purpose and just-in-time links from relevant work. Automations delegate to
runbooks, and repeated runs commit what they learn back into the repository.

- [Automations delegate to versioned, repo-owned runbooks]
- [“Think of it as guided runbook synthesis”]
- [Do not oversteer a model through a skill it may not need]
- [Use skills for an approach and runbooks for repeatable work]
- [Skill descriptions preserve what the skill is and why it matters]
- [Dynamically link task-relevant skills just in time]
- [“The skills are for the model, not for humans”]
- [“Just Codex and a Git repo full of Markdown”]

[Automations delegate to versioned, repo-owned runbooks]:
  https://x.com/_lopopolo/status/2045245935228510213
[“Think of it as guided runbook synthesis”]:
  https://x.com/_lopopolo/status/2047080866636325275
[Do not oversteer a model through a skill it may not need]:
  https://x.com/_lopopolo/status/2063146423743328327
[Use skills for an approach and runbooks for repeatable work]:
  https://x.com/_lopopolo/status/2063148351885918623
[Skill descriptions preserve what the skill is and why it matters]:
  https://x.com/_lopopolo/status/2077149347528261712
[Dynamically link task-relevant skills just in time]:
  https://x.com/_lopopolo/status/2077306653540749667
[“The skills are for the model, not for humans”]:
  https://x.com/_lopopolo/status/2077629953366282318
[“Just Codex and a Git repo full of Markdown”]:
  https://x.com/_lopopolo/status/2063119284084023805

### 5. Encode constraints at owned boundaries

Systematize the nonfunctional requirements that determine whether work belongs
in the system. Put stable constraints into APIs, types, lints, tests, and
architecture. Parse hostile input at an owned boundary, convert it once into a
trusted domain model, and carry identifiers, units, and invariants in domain
types. Make correct use natural and broad escape hatches unrepresentable.

- [Close off bad latent space, but not through user messages]
- [Give models space to cook and be creative]
- [Go high on architecture, decomposition, and opinions]
- [“Just make all the code the same”]
- [Repository code is prompts for future prompts]
- [Opaque subsystems need documented, reliable guarantees]
- [The nonfunctional-requirement universe in code]
- [Systematize writing down nonfunctional requirements]
- [Taste is coherent principles for tacit nonfunctional requirements]
- [Blank-slate SDK design leaves nonfunctional choices unpruned]
- [Make correct use natural and unsafe use hard to express]
- [Ban `unknown` and `isRecord` with code]
- [“Force your agents to use branded strings for ID types”]
- [Prefer a duration type to a bare number with units in the name]
- [Ban decoding untyped values anywhere except the boundary]
- [The program should not still have an `unknown` value]
- [“This is how you structurally address AI slop”]

[Close off bad latent space, but not through user messages]:
  https://x.com/_lopopolo/status/2054053548124778786
[Give models space to cook and be creative]:
  https://x.com/_lopopolo/status/2054948084400988231
[Go high on architecture, decomposition, and opinions]:
  https://x.com/_lopopolo/status/2030324932761288915
[“Just make all the code the same”]:
  https://x.com/_lopopolo/status/2030325081868837216
[Repository code is prompts for future prompts]:
  https://x.com/_lopopolo/status/2030460794765451300
[Opaque subsystems need documented, reliable guarantees]:
  https://x.com/_lopopolo/status/2030325312471683075
[The nonfunctional-requirement universe in code]:
  https://x.com/_lopopolo/status/2028982729145237775
[Systematize writing down nonfunctional requirements]:
  https://x.com/_lopopolo/status/2048914423914614882
[Taste is coherent principles for tacit nonfunctional requirements]:
  https://x.com/_lopopolo/status/2043798404221022508
[Blank-slate SDK design leaves nonfunctional choices unpruned]:
  https://x.com/_lopopolo/status/2051411695873216842
[Make correct use natural and unsafe use hard to express]:
  https://x.com/_lopopolo/status/2054258642745172309
[Ban `unknown` and `isRecord` with code]:
  https://x.com/_lopopolo/status/2074020365991612690
[“Force your agents to use branded strings for ID types”]:
  https://x.com/_lopopolo/status/2076191023500611843
[Prefer a duration type to a bare number with units in the name]:
  https://x.com/_lopopolo/status/2076192816477425981
[Ban decoding untyped values anywhere except the boundary]:
  https://x.com/_lopopolo/status/2076200856681357613
[The program should not still have an `unknown` value]:
  https://x.com/_lopopolo/status/2032626632654275060
[“This is how you structurally address AI slop”]:
  https://x.com/_lopopolo/status/2032626773280927878

### 6. Convert feedback and artifacts into system properties

Treat each correction or failed rollout as evidence about the harness. Recover
the governing principle and whole failure class, encode it in a guardrail, test,
type, tool, or runbook, and rerun in a fresh session. Session logs, blessed
implementations, and failed attempts are dense evidence for distillation,
ablation, reimplementation, and evaluation.

- [Write a regression test when optional parameters cause trouble]
- [“If it does a thing you don’t like, have it write tests”]
- [“Slop” reflects missing nonfunctional-requirement context and gates]
- [“I do often throw work away”]
- [Audit session logs to improve how the operator uses Codex]
- [Curate a knowledge base, test it in a fresh session, and improve it]
- [Ablate what may not matter; distill what does]
- [Self-improvement can happen without touching model weights]
- [Onboarding, progression, and review are feedback loops]
- [“Refine your environment so I never give the same feedback twice”]
- [Apply the principles behind a prompt to all of the work]
- [Do not apply an API correction to only the named line]
- [“Anticipate everything I will give you feedback on and fix it”]
- [Distill Symphony’s spec from the blessed implementation]
- [“Blessed work is incredibly information dense”]
- [A labeled-good work artifact is denser than a prompt or spec]
- [“Love to see a failed attempt. They are really information dense”]
- [Implementation → spec → reimplementation → evaluation]

[Write a regression test when optional parameters cause trouble]:
  https://x.com/_lopopolo/status/2029468543453151253
[“If it does a thing you don’t like, have it write tests”]:
  https://x.com/_lopopolo/status/2029468769631064169
[“Slop” reflects missing nonfunctional-requirement context and gates]:
  https://x.com/_lopopolo/status/2036967120345743776
[“I do often throw work away”]:
  https://x.com/_lopopolo/status/2029085149749756028
[Audit session logs to improve how the operator uses Codex]:
  https://x.com/_lopopolo/status/2047728449574650183
[Curate a knowledge base, test it in a fresh session, and improve it]:
  https://x.com/_lopopolo/status/2048944585486012775
[Ablate what may not matter; distill what does]:
  https://x.com/_lopopolo/status/2049145174790725654
[Self-improvement can happen without touching model weights]:
  https://x.com/_lopopolo/status/2047066780498350342
[Onboarding, progression, and review are feedback loops]:
  https://x.com/_lopopolo/status/2047326250944119285
[“Refine your environment so I never give the same feedback twice”]:
  https://x.com/_lopopolo/status/2054011131572990266
[Apply the principles behind a prompt to all of the work]:
  https://x.com/_lopopolo/status/2054021564862177517
[Do not apply an API correction to only the named line]:
  https://x.com/_lopopolo/status/2054027190967349685
[“Anticipate everything I will give you feedback on and fix it”]:
  https://x.com/_lopopolo/status/2054223594180563268
[Distill Symphony’s spec from the blessed implementation]:
  https://x.com/_lopopolo/status/2051410617937125508
[“Blessed work is incredibly information dense”]:
  https://x.com/_lopopolo/status/2054037448297201696
[A labeled-good work artifact is denser than a prompt or spec]:
  https://x.com/_lopopolo/status/2054065911716585676
[“Love to see a failed attempt. They are really information dense”]:
  https://x.com/_lopopolo/status/2063229530840690892
[Implementation → spec → reimplementation → evaluation]:
  https://x.com/_lopopolo/status/2048906262109413801

### 7. Make acceptance criteria executable and prove the outcome

Express acceptance criteria as executable feedback: objective standards, tests,
custom lints, model graders, canaries, telemetry, and critical journeys. Judge
results by effectiveness and business value. Plans guide execution; real-system
evidence and shipped artifacts establish completion.

- [Give a deployment a canary identity and test before cutover]
- [Plant CTF-style flags and let Codex improve its own skills]
- [Use Miri as a ground-truth grader]
- [“Make your agents show you proof of work”]
- [Token-spend leaderboards are vulnerable to Goodhart’s law]
- [Token spend is unanchored from business value and ROI]
- [“No one should care about tokens … care about effectiveness”]
- [Every task has acceptance criteria]
- [Accessibility standards provide objective graders]
- [Ground truth requires an “unhackable grader”]
- [Identify slop patterns and write a fully tested lint]
- [Burn tokens in CI to enforce documentation quality]
- [Static analysis and blocking review remove plan theater]

[Give a deployment a canary identity and test before cutover]:
  https://x.com/_lopopolo/status/2045318364537790777
[Plant CTF-style flags and let Codex improve its own skills]:
  https://x.com/_lopopolo/status/2047052450604212728
[Use Miri as a ground-truth grader]:
  https://x.com/_lopopolo/status/2047102222530728035
[“Make your agents show you proof of work”]:
  https://x.com/_lopopolo/status/2048945121815867857
[Token-spend leaderboards are vulnerable to Goodhart’s law]:
  https://x.com/_lopopolo/status/2035101303140167745
[Token spend is unanchored from business value and ROI]:
  https://x.com/_lopopolo/status/2035102053979271654
[“No one should care about tokens … care about effectiveness”]:
  https://x.com/_lopopolo/status/2035132736621748332
[Every task has acceptance criteria]:
  https://x.com/_lopopolo/status/2054087262527422706
[Accessibility standards provide objective graders]:
  https://x.com/_lopopolo/status/2054987836731150606
[Ground truth requires an “unhackable grader”]:
  https://x.com/_lopopolo/status/2056991142093508831
[Identify slop patterns and write a fully tested lint]:
  https://x.com/_lopopolo/status/2062259570756505848
[Burn tokens in CI to enforce documentation quality]:
  https://x.com/_lopopolo/status/2074005336244322706
[Static analysis and blocking review remove plan theater]:
  https://x.com/_lopopolo/status/2048909909685846388

### 8. Grant autonomy inside explicit authority

Give agents broad execution freedom inside narrow, revocable authority. Keep
secrets outside context, provide ambient scoped access, and let agents launch
their own development and observability environments. Permit inexpensive,
recoverable mistakes while deterministic guards protect consequential effects.

- [“How do I enable my team rather than restrict my team?”]
- [Tokens should never enter context or the repository]
- [Endpoint-scoped access creates an independent revocation boundary]
- [“The first … read-only identities for my agents get all my business”]
- [Scope agent access tightly for financial data]
- [Automate while retaining sole custody of private data]
- [“Make no mistakes” is empty without context]
- [Most mistakes are not consequential and help teams become robust]
- [For settled deterministic policy, “just fail the build”]
- [Start with five bought-in systems thinkers]
- [Human long-horizon work is not deterministic either]
- [A harness is disqualified if model-visible credentials can be recovered]
- [Let `gh` work ambiently while a sidecar holds credentials]
- [The agent should launch and instrument its own dev server]
- [“The model is able to work some magic when it has Jaeger”]

[“How do I enable my team rather than restrict my team?”]:
  https://x.com/_lopopolo/status/2044956783735918789
[Tokens should never enter context or the repository]:
  https://x.com/_lopopolo/status/2046021111440458027
[Endpoint-scoped access creates an independent revocation boundary]:
  https://x.com/_lopopolo/status/2046026270132387870
[“The first … read-only identities for my agents get all my business”]:
  https://x.com/_lopopolo/status/2046656417508294829
[Scope agent access tightly for financial data]:
  https://x.com/_lopopolo/status/2046660497836245362
[Automate while retaining sole custody of private data]:
  https://x.com/_lopopolo/status/2046661045750747559
[“Make no mistakes” is empty without context]:
  https://x.com/_lopopolo/status/2071843335749489068
[Most mistakes are not consequential and help teams become robust]:
  https://x.com/_lopopolo/status/2072102011546591295
[For settled deterministic policy, “just fail the build”]:
  https://x.com/_lopopolo/status/2077153783654891635
[Start with five bought-in systems thinkers]:
  https://x.com/_lopopolo/status/2046608251987656840
[Human long-horizon work is not deterministic either]:
  https://x.com/_lopopolo/status/2046623033595773396
[A harness is disqualified if model-visible credentials can be recovered]:
  https://x.com/_lopopolo/status/2060873286465024427
[Let `gh` work ambiently while a sidecar holds credentials]:
  https://x.com/_lopopolo/status/2061150109232939288
[The agent should launch and instrument its own dev server]:
  https://x.com/_lopopolo/status/2074206094772322471
[“The model is able to work some magic when it has Jaeger”]:
  https://x.com/_lopopolo/status/2074206262250856780

### 9. Preserve coherence across thousands of changes

Make the next 50 or 5,000 changes and their effect on future velocity part of
today’s acceptance criteria. Agents can replace an architecture that no longer
serves the system, and each change should leave a coherent base for the work
that follows.

- [Models do not understand that work must scale to 5,000 PRs]
- [Long-term coherence of agent-produced artifacts]
- [Work is an iterative game]
- [Sometimes the architecture is wrong and must be blown up]
- [Ask what the work ladders up to and how it affects future velocity]
- [Agents do not fear writing 50 more PRs atop their work]
- [Use a language boundary to keep brownfield code from dominating the prompt]

[Models do not understand that work must scale to 5,000 PRs]:
  https://x.com/_lopopolo/status/2052834431996666267
[Long-term coherence of agent-produced artifacts]:
  https://x.com/_lopopolo/status/2052834649152618908
[Work is an iterative game]: https://x.com/_lopopolo/status/2052858891835465813
[Sometimes the architecture is wrong and must be blown up]:
  https://x.com/_lopopolo/status/2052837107400679555
[Ask what the work ladders up to and how it affects future velocity]:
  https://x.com/_lopopolo/status/2055111894181335279
[Agents do not fear writing 50 more PRs atop their work]:
  https://x.com/_lopopolo/status/2064221034547753105
[Use a language boundary to keep brownfield code from dominating the prompt]:
  https://x.com/_lopopolo/status/2054955223609680303

### 10. Own dependency risk over the repository’s lifetime

Evaluate dependencies as lifetime ownership choices. Prefer one toolchain, a
small reviewed dependency set, pins from one source of truth, and risk-assessed
cooldown runbooks. Internalize code where repository ownership lowers
maintenance and supply-chain risk.

- [Minimize maintenance burden and supply-chain risk when code is cheap]
- [Pin toolchains and advance them through risk-assessed runbooks]
- [Ask why not contribute to or fork `gitoxide`]
- [Keep GitHub Actions images pinned and automatically up to date]

[Minimize maintenance burden and supply-chain risk when code is cheap]:
  https://x.com/_lopopolo/status/2059023029666214126
[Pin toolchains and advance them through risk-assessed runbooks]:
  https://x.com/_lopopolo/status/2060767456302428343
[Ask why not contribute to or fork `gitoxide`]:
  https://x.com/_lopopolo/status/2064388932473622953
[Keep GitHub Actions images pinned and automatically up to date]:
  https://x.com/_lopopolo/status/2077904710003228694

### 11. Run maintenance as a continuous loop

Use cron, event triggers, tail calls, automated refactoring, garbage collection,
and distillation to maintain the world model continuously. Research the whole
class of a requested change, add useful capabilities, remove obsolete ones, and
migrate dependents when an interface evolves.

- [Research and add the whole class, not one suggested backend]
- [Continuously add good backends and remove obsolete ones]
- [Add automatic distillation to refactoring and garbage-collection loops]
- [Loop engineering uses cron, events, and tail calls]
- [Agents may migrate API dependents across organizations]

[Research and add the whole class, not one suggested backend]:
  https://x.com/_lopopolo/status/2062220470057898417
[Continuously add good backends and remove obsolete ones]:
  https://x.com/_lopopolo/status/2062222490483474772
[Add automatic distillation to refactoring and garbage-collection loops]:
  https://x.com/_lopopolo/status/2064346868398874848
[Loop engineering uses cron, events, and tail calls]:
  https://x.com/_lopopolo/status/2068882715399926026
[Agents may migrate API dependents across organizations]:
  https://x.com/_lopopolo/status/2049591048759116267

### 12. Deploy expert process through tools and context

Use tools and context to translate fixed general capability into expert service
over private ontology, process, and institutional knowledge. Code can serve as
the agent’s internal action language while the user receives a bespoke domain
product or service. The same approach applies to every kind of expert artifact.

- [Code is how an agent uses a computer]
- [Not everyone needs to know about or see the code]
- [Encode domain expertise for point-to-point service]
- [An agent can build a bespoke assistive screen reader]
- [Point agents at the full public body of work]
- [The model does not have your data; harness engineering is about that]
- [Build a cloud that adapts to customer knowledge and workflows]
- [Replace “codebase” with “artifact” to generalize agent work]
- [“Deployment remains (one of) the hard part”]

[Code is how an agent uses a computer]:
  https://x.com/_lopopolo/status/2043495733375230026
[Not everyone needs to know about or see the code]:
  https://x.com/_lopopolo/status/2047037215864418486
[Encode domain expertise for point-to-point service]:
  https://x.com/_lopopolo/status/2049939343901606332
[An agent can build a bespoke assistive screen reader]:
  https://x.com/_lopopolo/status/2045945850736832769
[Point agents at the full public body of work]:
  https://x.com/_lopopolo/status/2050698864542482709
[The model does not have your data; harness engineering is about that]:
  https://x.com/_lopopolo/status/2051086282236068178
[Build a cloud that adapts to customer knowledge and workflows]:
  https://x.com/_lopopolo/status/2074977994356212026
[Replace “codebase” with “artifact” to generalize agent work]:
  https://x.com/_lopopolo/status/2076878736507736390
[“Deployment remains (one of) the hard part”]:
  https://x.com/_lopopolo/status/2076889128952856870

## Capture coverage

The capture includes publicly retrievable posts through July 17, 2026. Public
discovery is incomplete and unstable, so archival completeness requires an
official X account export. The machine-readable corpus records the known
coverage limits and provenance for independently captured material.
