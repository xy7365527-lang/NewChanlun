"""Michel Foucault — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Foucault's thought —
from the early archaeology of madness through the genealogy
of power/knowledge to the late ethics of self-care —
as an incremental topological growth process.

Covers 5 major works (1961-1976).
"""

from __future__ import annotations

import json
import sys
import time

sys.path.insert(0, ".")

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Shared / recurring vertex IDs across works:
#   foucault_power, foucault_knowledge, foucault_discourse,
#   foucault_discipline, foucault_biopower, foucault_episteme,
#   foucault_subject, foucault_surveillance, foucault_panopticon,
#   foucault_archaeology, foucault_genealogy
# ---------------------------------------------------------------------------


def work_madness_and_civilization():
    """1961: Madness and Civilization (Folie et Deraison).

    Foucault's first major work. Traces the historical exclusion of
    madness by reason. The 'great confinement' of the classical age.
    The ship of fools, the birth of the asylum, the constitution
    of madness as mental illness.
    """
    vertices = [
        Vertex("foucault_reason_unreason", content="reason/unreason — the great division, reason constitutes itself by excluding unreason"),
        Vertex("foucault_madness", content="madness — not a natural fact but a cultural construct, silenced experience"),
        Vertex("foucault_great_confinement", content="great confinement — classical age locks up the mad, poor, deviant together"),
        Vertex("foucault_ship_of_fools", content="ship of fools (Narrenschiff) — Renaissance, madness circulates freely"),
        Vertex("foucault_asylum", content="asylum — moral treatment replaces chains with guilt and observation"),
        Vertex("foucault_mental_illness", content="mental illness — modern medical category, madness captured by psychiatry"),
        Vertex("foucault_classical_age", content="classical age — age of representation, confinement as social control"),
        Vertex("foucault_moral_treatment", content="moral treatment — Tuke and Pinel, liberation that is new subjection"),
        Vertex("foucault_experience_limit", content="limit-experience — madness as experience at the edge of reason"),
        Vertex("foucault_silence_madness", content="silence of madness — reason speaks about madness, madness cannot speak"),
    ]
    edges = [
        Edge("foucault_reason_unreason", "foucault_madness", EdgeType.NEGATION),
        Edge("foucault_madness", "foucault_reason_unreason", EdgeType.NEGATION),
        Edge("foucault_great_confinement", "foucault_classical_age", EdgeType.DEPENDENCY),
        Edge("foucault_great_confinement", "foucault_madness", EdgeType.DEPENDENCY),
        Edge("foucault_ship_of_fools", "foucault_great_confinement", EdgeType.NEGATION),
        Edge("foucault_asylum", "foucault_great_confinement", EdgeType.NEGATION),
        Edge("foucault_asylum", "foucault_moral_treatment", EdgeType.DEPENDENCY),
        Edge("foucault_mental_illness", "foucault_madness", EdgeType.NEGATION),
        Edge("foucault_mental_illness", "foucault_asylum", EdgeType.DEPENDENCY),
        Edge("foucault_moral_treatment", "foucault_asylum", EdgeType.DEPENDENCY),
        Edge("foucault_moral_treatment", "foucault_reason_unreason", EdgeType.DEPENDENCY),
        Edge("foucault_experience_limit", "foucault_madness", EdgeType.DEPENDENCY),
        Edge("foucault_experience_limit", "foucault_reason_unreason", EdgeType.NEGATION),
        Edge("foucault_silence_madness", "foucault_madness", EdgeType.DEPENDENCY),
        Edge("foucault_silence_madness", "foucault_reason_unreason", EdgeType.DEPENDENCY),
        Edge("foucault_classical_age", "foucault_ship_of_fools", EdgeType.NEGATION),
    ]
    return "1961: Madness and Civilization", vertices, edges


def work_order_of_things():
    """1966: The Order of Things (Les Mots et les Choses).

    Foucault's archaeology of the human sciences. Introduces the
    concept of episteme — the deep structure of knowledge in each
    period. The classical episteme (representation), the modern
    episteme (man), and the announcement of man's dissolution.
    """
    vertices = [
        Vertex("foucault_episteme", content="episteme — historical a priori, conditions of possibility for knowledge"),
        Vertex("foucault_archaeology", content="archaeology — method: uncover deep structures of discourse"),
        Vertex("foucault_man_invention", content="man as recent invention — the figure of 'man' emerged around 1800"),
        Vertex("foucault_death_of_man", content="death of man — man will be erased, like a face drawn in sand"),
        Vertex("foucault_classical_episteme", content="classical episteme — representation, taxonomy, mathesis"),
        Vertex("foucault_modern_episteme", content="modern episteme — man as both subject and object of knowledge"),
        Vertex("foucault_renaissance_episteme", content="Renaissance episteme — resemblance, similitude, signatures"),
        Vertex("foucault_representation", content="representation — classical age organizes knowledge through representation"),
        Vertex("foucault_language_being", content="language returns to being — literature, linguistics, formalization"),
        Vertex("foucault_human_sciences", content="human sciences — psychology, sociology, literary studies: secondary formation"),
        Vertex("foucault_empirico_transcendental", content="empirico-transcendental doublet — man as both knowing subject and known object"),
        Vertex("foucault_finitude", content="finitude — modern man defined by labor, life, language as finite"),
        Vertex("foucault_analytic_finitude", content="analytic of finitude — Kant's question: conditions of knowledge = conditions of experience"),
    ]
    edges = [
        Edge("foucault_episteme", "foucault_archaeology", EdgeType.DEPENDENCY),
        Edge("foucault_man_invention", "foucault_modern_episteme", EdgeType.DEPENDENCY),
        Edge("foucault_death_of_man", "foucault_man_invention", EdgeType.NEGATION),
        Edge("foucault_death_of_man", "foucault_language_being", EdgeType.DEPENDENCY),
        Edge("foucault_classical_episteme", "foucault_renaissance_episteme", EdgeType.NEGATION),
        Edge("foucault_classical_episteme", "foucault_representation", EdgeType.DEPENDENCY),
        Edge("foucault_modern_episteme", "foucault_classical_episteme", EdgeType.NEGATION),
        Edge("foucault_modern_episteme", "foucault_man_invention", EdgeType.DEPENDENCY),
        Edge("foucault_modern_episteme", "foucault_finitude", EdgeType.DEPENDENCY),
        Edge("foucault_renaissance_episteme", "foucault_classical_episteme", EdgeType.NEGATION),
        Edge("foucault_representation", "foucault_classical_episteme", EdgeType.DEPENDENCY),
        Edge("foucault_language_being", "foucault_modern_episteme", EdgeType.NEGATION),
        Edge("foucault_language_being", "foucault_representation", EdgeType.NEGATION),
        Edge("foucault_human_sciences", "foucault_modern_episteme", EdgeType.DEPENDENCY),
        Edge("foucault_human_sciences", "foucault_empirico_transcendental", EdgeType.DEPENDENCY),
        Edge("foucault_empirico_transcendental", "foucault_man_invention", EdgeType.DEPENDENCY),
        Edge("foucault_finitude", "foucault_analytic_finitude", EdgeType.DEPENDENCY),
        Edge("foucault_analytic_finitude", "foucault_empirico_transcendental", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("foucault_archaeology", "foucault_reason_unreason", EdgeType.REFERENCE),
        Edge("foucault_episteme", "foucault_classical_age", EdgeType.REFERENCE),
    ]
    return "1966: The Order of Things", vertices, edges


def work_archaeology_of_knowledge():
    """1969: The Archaeology of Knowledge (L'Archeologie du savoir).

    Foucault's methodological treatise. Defines the key concepts:
    discourse, statement (enonce), archive, discursive formation,
    discursive practice. Replaces the history of ideas with
    archaeological analysis.
    """
    vertices = [
        Vertex("foucault_discourse", content="discourse — regulated system of statements, not just speech"),
        Vertex("foucault_statement", content="statement (enonce) — atomic function of discourse, not sentence"),
        Vertex("foucault_archive", content="archive — the law of what can be said, system of discursive functioning"),
        Vertex("foucault_discursive_formation", content="discursive formation — system of regularity among statements"),
        Vertex("foucault_discursive_practice", content="discursive practice — rules that produce statements"),
        Vertex("foucault_positivity", content="positivity — conditions of existence of discourse"),
        Vertex("foucault_historical_a_priori", content="historical a priori — conditions of discourse, historically variable"),
        Vertex("foucault_enunciative_function", content="enunciative function — what makes a statement a statement"),
        Vertex("foucault_regularity", content="regularity — patterns in discursive formation, not hidden meaning"),
        Vertex("foucault_discontinuity", content="discontinuity — ruptures between discursive formations, not smooth progress"),
    ]
    edges = [
        Edge("foucault_discourse", "foucault_statement", EdgeType.DEPENDENCY),
        Edge("foucault_discourse", "foucault_discursive_formation", EdgeType.DEPENDENCY),
        Edge("foucault_statement", "foucault_enunciative_function", EdgeType.DEPENDENCY),
        Edge("foucault_archive", "foucault_discourse", EdgeType.DEPENDENCY),
        Edge("foucault_archive", "foucault_historical_a_priori", EdgeType.DEPENDENCY),
        Edge("foucault_discursive_formation", "foucault_regularity", EdgeType.DEPENDENCY),
        Edge("foucault_discursive_formation", "foucault_discursive_practice", EdgeType.DEPENDENCY),
        Edge("foucault_discursive_practice", "foucault_statement", EdgeType.DEPENDENCY),
        Edge("foucault_positivity", "foucault_discourse", EdgeType.DEPENDENCY),
        Edge("foucault_positivity", "foucault_archive", EdgeType.DEPENDENCY),
        Edge("foucault_historical_a_priori", "foucault_episteme", EdgeType.DEPENDENCY),
        Edge("foucault_historical_a_priori", "foucault_archive", EdgeType.DEPENDENCY),
        Edge("foucault_regularity", "foucault_discursive_practice", EdgeType.DEPENDENCY),
        Edge("foucault_discontinuity", "foucault_regularity", EdgeType.NEGATION),
        Edge("foucault_discontinuity", "foucault_discursive_formation", EdgeType.NEGATION),
        # Cross-work
        Edge("foucault_discourse", "foucault_episteme", EdgeType.DEPENDENCY),
        Edge("foucault_discontinuity", "foucault_classical_episteme", EdgeType.REFERENCE),
        Edge("foucault_archive", "foucault_silence_madness", EdgeType.REFERENCE),
    ]
    return "1969: The Archaeology of Knowledge", vertices, edges


def work_discipline_and_punish():
    """1975: Discipline and Punish (Surveiller et Punir).

    Foucault's genealogy of the modern disciplinary society. From
    sovereign punishment (spectacle of the scaffold) to discipline
    (prison, school, factory, hospital). The Panopticon as model
    of disciplinary power. Power produces subjects, not just represses.
    """
    vertices = [
        Vertex("foucault_discipline", content="discipline — microphysics of power, techniques for training bodies"),
        Vertex("foucault_panopticon", content="Panopticon — Bentham's design, perfect surveillance, internalized gaze"),
        Vertex("foucault_power_knowledge", content="power/knowledge — power produces knowledge, knowledge enables power"),
        Vertex("foucault_surveillance", content="surveillance — hierarchical observation, normalizing judgment"),
        Vertex("foucault_docile_body", content="docile body — body subjected to discipline, made useful and obedient"),
        Vertex("foucault_sovereign_power", content="sovereign power — spectacle of punishment, the scaffold"),
        Vertex("foucault_prison", content="prison — the carceral archipelago, discipline as norm"),
        Vertex("foucault_norm", content="norm — normalizing judgment, the examination"),
        Vertex("foucault_examination", content="examination — combines hierarchical observation + normalizing judgment"),
        Vertex("foucault_soul", content="soul — the prison of the body, produced by disciplinary power"),
        Vertex("foucault_genealogy", content="genealogy — method: power relations, not hidden truth, Nietzschean"),
        Vertex("foucault_carceral", content="carceral continuum — discipline extends from prison to school to hospital"),
        Vertex("foucault_delinquent", content="delinquent — category produced by prison system, not pre-existing"),
        Vertex("foucault_power", content="power — relational, productive, not possessed, not purely repressive"),
        Vertex("foucault_subject", content="subject — produced by power relations, subjection and subjectivation"),
    ]
    edges = [
        Edge("foucault_discipline", "foucault_sovereign_power", EdgeType.NEGATION),
        Edge("foucault_discipline", "foucault_docile_body", EdgeType.DEPENDENCY),
        Edge("foucault_discipline", "foucault_norm", EdgeType.DEPENDENCY),
        Edge("foucault_panopticon", "foucault_surveillance", EdgeType.DEPENDENCY),
        Edge("foucault_panopticon", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_power_knowledge", "foucault_power", EdgeType.DEPENDENCY),
        Edge("foucault_power_knowledge", "foucault_discourse", EdgeType.DEPENDENCY),
        Edge("foucault_surveillance", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_surveillance", "foucault_norm", EdgeType.DEPENDENCY),
        Edge("foucault_docile_body", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_docile_body", "foucault_subject", EdgeType.DEPENDENCY),
        Edge("foucault_sovereign_power", "foucault_discipline", EdgeType.NEGATION),
        Edge("foucault_prison", "foucault_panopticon", EdgeType.DEPENDENCY),
        Edge("foucault_prison", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_prison", "foucault_delinquent", EdgeType.DEPENDENCY),
        Edge("foucault_norm", "foucault_examination", EdgeType.DEPENDENCY),
        Edge("foucault_examination", "foucault_surveillance", EdgeType.DEPENDENCY),
        Edge("foucault_examination", "foucault_norm", EdgeType.DEPENDENCY),
        Edge("foucault_soul", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_soul", "foucault_docile_body", EdgeType.NEGATION),
        Edge("foucault_genealogy", "foucault_archaeology", EdgeType.NEGATION),
        Edge("foucault_genealogy", "foucault_power", EdgeType.DEPENDENCY),
        Edge("foucault_carceral", "foucault_prison", EdgeType.DEPENDENCY),
        Edge("foucault_carceral", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_delinquent", "foucault_prison", EdgeType.DEPENDENCY),
        Edge("foucault_delinquent", "foucault_norm", EdgeType.NEGATION),
        Edge("foucault_power", "foucault_subject", EdgeType.DEPENDENCY),
        Edge("foucault_power", "foucault_power_knowledge", EdgeType.DEPENDENCY),
        Edge("foucault_subject", "foucault_power", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("foucault_discipline", "foucault_great_confinement", EdgeType.REFERENCE),
        Edge("foucault_genealogy", "foucault_episteme", EdgeType.REFERENCE),
        Edge("foucault_panopticon", "foucault_asylum", EdgeType.REFERENCE),
    ]
    return "1975: Discipline and Punish", vertices, edges


def work_history_of_sexuality():
    """1976: The History of Sexuality Vol. 1: The Will to Knowledge.

    Foucault's analysis of sexuality as a discourse-effect, not
    repressed nature. Introduces biopower — the regulation of
    populations. The repressive hypothesis is overturned: modern
    society produces sexuality through discourse, not represses it.
    """
    vertices = [
        Vertex("foucault_biopower", content="biopower — power over life, regulation of populations"),
        Vertex("foucault_sexuality_discourse", content="sexuality as discourse — not repressed nature but produced object"),
        Vertex("foucault_repressive_hypothesis", content="repressive hypothesis — the idea that sex has been silenced (Foucault refutes this)"),
        Vertex("foucault_confession", content="confession — technique for extracting truth about the self, sexuality"),
        Vertex("foucault_scientia_sexualis", content="scientia sexualis — Western science of sexuality, tied to confession"),
        Vertex("foucault_ars_erotica", content="ars erotica — Eastern tradition, truth from pleasure itself"),
        Vertex("foucault_biopolitics", content="biopolitics — politics of life: birth rate, health, hygiene, population"),
        Vertex("foucault_anatomopolitics", content="anatomo-politics — discipline of individual body (connects to D&P)"),
        Vertex("foucault_deployment_sexuality", content="deployment of sexuality — network of power/knowledge about sex"),
        Vertex("foucault_deployment_alliance", content="deployment of alliance — family, kinship, inheritance (prior system)"),
        Vertex("foucault_right_of_death", content="sovereign right of death — 'let live or make die'"),
        Vertex("foucault_power_over_life", content="power over life — 'make live or let die', biopolitical inversion"),
        Vertex("foucault_resistance", content="resistance — where there is power, there is resistance"),
    ]
    edges = [
        Edge("foucault_biopower", "foucault_power", EdgeType.DEPENDENCY),
        Edge("foucault_biopower", "foucault_sovereign_power", EdgeType.NEGATION),
        Edge("foucault_biopower", "foucault_biopolitics", EdgeType.DEPENDENCY),
        Edge("foucault_biopower", "foucault_anatomopolitics", EdgeType.DEPENDENCY),
        Edge("foucault_sexuality_discourse", "foucault_discourse", EdgeType.DEPENDENCY),
        Edge("foucault_sexuality_discourse", "foucault_repressive_hypothesis", EdgeType.NEGATION),
        Edge("foucault_repressive_hypothesis", "foucault_sexuality_discourse", EdgeType.NEGATION),
        Edge("foucault_confession", "foucault_sexuality_discourse", EdgeType.DEPENDENCY),
        Edge("foucault_confession", "foucault_power_knowledge", EdgeType.DEPENDENCY),
        Edge("foucault_scientia_sexualis", "foucault_confession", EdgeType.DEPENDENCY),
        Edge("foucault_scientia_sexualis", "foucault_ars_erotica", EdgeType.NEGATION),
        Edge("foucault_ars_erotica", "foucault_scientia_sexualis", EdgeType.NEGATION),
        Edge("foucault_biopolitics", "foucault_biopower", EdgeType.DEPENDENCY),
        Edge("foucault_biopolitics", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_anatomopolitics", "foucault_discipline", EdgeType.DEPENDENCY),
        Edge("foucault_anatomopolitics", "foucault_docile_body", EdgeType.DEPENDENCY),
        Edge("foucault_deployment_sexuality", "foucault_deployment_alliance", EdgeType.NEGATION),
        Edge("foucault_deployment_sexuality", "foucault_power_knowledge", EdgeType.DEPENDENCY),
        Edge("foucault_right_of_death", "foucault_power_over_life", EdgeType.NEGATION),
        Edge("foucault_power_over_life", "foucault_biopower", EdgeType.DEPENDENCY),
        Edge("foucault_power_over_life", "foucault_right_of_death", EdgeType.NEGATION),
        Edge("foucault_resistance", "foucault_power", EdgeType.DEPENDENCY),
        Edge("foucault_resistance", "foucault_power", EdgeType.NEGATION),
        # Cross-work
        Edge("foucault_biopower", "foucault_panopticon", EdgeType.REFERENCE),
        Edge("foucault_confession", "foucault_examination", EdgeType.REFERENCE),
        Edge("foucault_biopolitics", "foucault_norm", EdgeType.REFERENCE),
    ]
    return "1976: History of Sexuality Vol. 1", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_madness_and_civilization,
    work_order_of_things,
    work_archaeology_of_knowledge,
    work_discipline_and_punish,
    work_history_of_sexuality,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("FOUCAULT COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
    print(f"{len(ALL_WORKS)} works, incremental injection, "
          "digestion until beta_1 stable for 50 steps")
    print("=" * 70)

    graph = Graph()
    engine = None
    work_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for w_idx, w_fn in enumerate(ALL_WORKS):
        work_name, w_vertices, w_edges = w_fn()
        print(f"\n{'─' * 60}")
        print(f"Work {w_idx + 1}/{len(ALL_WORKS)}: {work_name}")
        print(f"{'─' * 60}")

        # Collect new vertices
        new_vertices = []
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                new_vertices.append(v)
                existing_vids.add(v.id)
        new_v_count = len(new_vertices)

        # Collect new edges (skip duplicates, check endpoints in existing + new vids)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_edges = []
        for e in w_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in existing_vids and e.target in existing_vids:
                    new_edges.append(e)
                    existing_edges.add(key)
        new_e_count = len(new_edges)

        if new_vertices or new_edges:
            graph = graph.add_vertices_and_edges_batch(new_vertices, new_edges)

        beta_before = compute_beta_1(graph)
        print(f"  Injected: {new_v_count} vertices, {new_e_count} edges")
        print(f"  Complex now: {len(graph.active_vertex_ids())} V, "
              f"{len(graph.active_edges())} E")
        print(f"  beta_1 after injection: {beta_before}")

        if engine is None:
            start = graph.active_vertex_ids()[0]
            engine = TraversalEngine(
                graph, start=start, settlement_threshold=10, seed=42,
            )
        else:
            engine.k_active = graph
            engine.k_full = graph
            engine.terrain = compute_terrain(engine.k_active)
            if engine.position not in graph.vertices:
                engine.position = graph.active_vertex_ids()[0]

        stable_count = 0
        last_beta = compute_beta_1(engine.k_active)
        steps_this_work = 0
        max_steps = 2000

        while stable_count < 50 and steps_this_work < max_steps:
            engine.run_step()
            steps_this_work += 1
            cumulative_steps += 1
            current_beta = compute_beta_1(engine.k_active)
            if current_beta == last_beta:
                stable_count += 1
            else:
                stable_count = 0
                last_beta = current_beta

        beta_after = compute_beta_1(engine.k_active)
        converged = stable_count >= 50

        settled_count = len(engine.settlement.settled_cycles)
        blocked_count = len(engine.settlement.blocked_log)

        print(f"  Steps to digest: {steps_this_work} "
              f"{'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} "
              f"(delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        work_logs = engine.logs[-steps_this_work:] if steps_this_work > 0 else []
        op_counts = {}
        for log in work_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        result = {
            "work": work_name,
            "work_index": w_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_work": blocked_count,
            "steps_to_digest": steps_this_work,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        work_results.append(result)

        beta_1_curve.append({
            "work_index": w_idx + 1,
            "work_name": work_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        graph = engine.k_active

    elapsed = time.time() - t0

    print("\n" + "=" * 70)
    print("FINAL SUMMARY")
    print("=" * 70)

    final_beta = compute_beta_1(engine.k_active)
    final_v = len(engine.k_active.active_vertex_ids())
    final_e = len(engine.k_active.active_edges())
    final_settled = len(engine.settlement.settled_cycles)

    print(f"\n  Total steps: {cumulative_steps}")
    print(f"  Final beta_1: {final_beta}")
    print(f"  Final complex: {final_v} V, {final_e} E")
    print(f"  Settled cycles: {final_settled}")
    print(f"  Elapsed: {elapsed:.1f}s")

    print(f"\n  beta_1 growth curve (by work):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    W{entry['work_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['work_name']}")

    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1

    type_counts = {}
    for e in engine.k_active.active_edges():
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1

    neg_count = type_counts.get("negation", 0)
    neg_density = neg_count / final_e if final_e > 0 else 0

    output = {
        "experiment": "foucault_full_incremental",
        "source": "Foucault — 5 major works (1961-1976)",
        "methodology": "manual conceptual complex per work, incremental injection, "
                        "digestion until 50-step beta_1 stability",
        "work_results": work_results,
        "beta_1_curve": beta_1_curve,
        "final_complex": {
            "vertices_active": final_v,
            "edges_active": final_e,
            "beta_1": final_beta,
            "settled_cycles": final_settled,
            "edge_type_distribution": type_counts,
            "terrain_distribution": ft_counts,
            "negation_density": neg_density,
        },
        "total_steps": cumulative_steps,
        "elapsed_seconds": round(elapsed, 2),
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_foucault.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
