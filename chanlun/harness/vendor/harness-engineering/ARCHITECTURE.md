# Architecture

This repository is a retrieval-optimized context bundle. The root introduces
the practice and routes readers, the agent guide routes a target-specific
decision, the thesis index names the developed arguments, playbooks turn them
into proposed procedures, and the source library preserves evidence. Each layer
points to the next depth without repeating its job.

## Retrieval boundary

An agent applying the corpus begins with [`AGENTS.md`], reads the target
system’s local instructions and current state, and loads only the theses
relevant to the task. Cases and primary sources remain available for deeper
reasoning, claim tracing, or attribution. The raw corpus is not part of the
default working set.

[`AGENTS.md`]: AGENTS.md

Editors follow the nested guides in `docs/`, `playbooks/`, and `sources/`.
Those instructions govern synthesis, repository-authored procedures, provenance,
redistribution, and source refreshes without entering an ordinary consuming
trajectory.

## Ownership

| Path                             | Owns                                                                  |
| -------------------------------- | --------------------------------------------------------------------- |
| `README.md`                      | Public definition, promise, and entry points                          |
| `AGENTS.md`                      | Application and unresolved-decision routing for consuming agents      |
| `ARCHITECTURE.md`                | Document ownership and routing invariants                             |
| `docs/README.md`                 | The canonical thesis index                                            |
| `docs/<thesis>/`                 | Developed arguments, cases, and applications                          |
| `evals/`                         | Comparative evaluation method and longitudinal public cases           |
| `playbooks/README.md`            | Playbook selection, origin, and evidence status                        |
| `playbooks/improve-harness.md`   | One bounded baseline-to-rerun intervention loop                        |
| `playbooks/repository-review.md` | Repository-scale assessment and readiness findings                    |
| `playbooks/AGENTS.md`            | Instructions for using and editing authored procedures                 |
| `docs/AGENTS.md`                 | Instructions for writing and reviewing synthesis                      |
| `assets/`                        | Repository presentation assets                                        |
| `sources/`                       | Bibliography, source-specific indexes, snapshots, and provenance      |
| `sources/scripts/`               | Typed source boundaries, validation, tests, and retrieval helpers     |
| `sources/AGENTS.md`              | Source capture, refresh, and redistribution rules                     |
| `COPYING.md`                     | License scope, attribution, and rights held by other contributors     |

## Invariants

1. **Keep one navigation contract.** The root introduces and routes,
   `docs/README.md` indexes, thesis pages synthesize, `evals/` and `playbooks/`
   house applications, and `sources/` preserves evidence.
2. **Route just in time.** A normal task receives the smallest relevant thesis
   set. Cases and source corpora load only when they can change a decision.
3. **Separate argument from evidence.** A source or premise may support several
   theses. Full arguments live in thesis pages; the source library routes to or
   preserves original language and provenance.
4. **Preserve rights at the boundary.** A local snapshot requires an affirmative
   redistribution basis. Otherwise the source library keeps canonical and
   archive links, metadata, bounded quotation, and source notes.
5. **Test behavioral effect.** The bundle succeeds when it changes engineering
   decisions, not when an agent repeats its terminology.
6. **Keep the bundle proportional.** This documentation corpus does not need a
   build system or dependency graph merely to validate its own links and data.
