"""Friedrich Nietzsche — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Nietzsche's thought —
from the early Apollonian/Dionysian duality through the genealogy
of morality to the final transvaluation of all values —
as an incremental topological growth process.

Covers 5 major works (1872-1889).
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
#   nietzsche_will_to_power, nietzsche_eternal_recurrence,
#   nietzsche_ubermensch, nietzsche_slave_morality,
#   nietzsche_master_morality, nietzsche_ressentiment,
#   nietzsche_dionysian, nietzsche_apollonian,
#   nietzsche_perspectivism, nietzsche_nihilism
# ---------------------------------------------------------------------------


def work_birth_of_tragedy():
    """1872: The Birth of Tragedy (Die Geburt der Tragodie).

    Nietzsche's first book. Introduces the Apollonian/Dionysian duality
    as the two art-impulses of nature. Greek tragedy was their synthesis.
    Socrates destroyed tragedy through theoretical optimism.
    """
    vertices = [
        Vertex("nietzsche_apollonian", content="Apollonian — form, individuation, dream, plastic art"),
        Vertex("nietzsche_dionysian", content="Dionysian — intoxication, dissolution of individuality, music"),
        Vertex("nietzsche_tragedy", content="tragedy — synthesis of Apollonian and Dionysian, highest art"),
        Vertex("nietzsche_socratic_optimism", content="Socratic optimism — knowledge can cure existence, death of tragedy"),
        Vertex("nietzsche_chorus", content="tragic chorus — Dionysian ground from which Apollonian vision arises"),
        Vertex("nietzsche_primal_unity", content="primal unity (Ur-Eine) — Dionysian oneness behind individuation"),
        Vertex("nietzsche_aesthetic_justification", content="aesthetic justification of existence — life justified as aesthetic phenomenon"),
        Vertex("nietzsche_myth", content="myth — tragic myth as Dionysian wisdom in Apollonian form"),
        Vertex("nietzsche_wagner_early", content="Wagner — vessel for rebirth of tragedy (early hope)"),
        Vertex("nietzsche_socrates", content="Socrates — the non-mystic, theoretical man, destroyer of instinct"),
        Vertex("nietzsche_individuation", content="principium individuationis — Apollonian principle, veil of Maya"),
    ]
    edges = [
        Edge("nietzsche_apollonian", "nietzsche_dionysian", EdgeType.NEGATION),
        Edge("nietzsche_dionysian", "nietzsche_apollonian", EdgeType.NEGATION),
        Edge("nietzsche_tragedy", "nietzsche_apollonian", EdgeType.DEPENDENCY),
        Edge("nietzsche_tragedy", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_socratic_optimism", "nietzsche_tragedy", EdgeType.NEGATION),
        Edge("nietzsche_socratic_optimism", "nietzsche_dionysian", EdgeType.NEGATION),
        Edge("nietzsche_chorus", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_chorus", "nietzsche_tragedy", EdgeType.DEPENDENCY),
        Edge("nietzsche_primal_unity", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_primal_unity", "nietzsche_individuation", EdgeType.NEGATION),
        Edge("nietzsche_aesthetic_justification", "nietzsche_tragedy", EdgeType.DEPENDENCY),
        Edge("nietzsche_aesthetic_justification", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_myth", "nietzsche_tragedy", EdgeType.DEPENDENCY),
        Edge("nietzsche_myth", "nietzsche_apollonian", EdgeType.DEPENDENCY),
        Edge("nietzsche_myth", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_wagner_early", "nietzsche_tragedy", EdgeType.REFERENCE),
        Edge("nietzsche_socrates", "nietzsche_socratic_optimism", EdgeType.DEPENDENCY),
        Edge("nietzsche_individuation", "nietzsche_apollonian", EdgeType.DEPENDENCY),
    ]
    return "1872: The Birth of Tragedy", vertices, edges


def work_beyond_good_and_evil():
    """1886: Beyond Good and Evil (Jenseits von Gut und Bose).

    Critique of philosophers' dogmatism. Introduces master/slave morality,
    perspectivism, the will to truth, the concept of a 'free spirit'.
    Announces the need for a philosophy of the future.
    """
    vertices = [
        Vertex("nietzsche_master_morality", content="master morality — noble self-affirmation, good/bad"),
        Vertex("nietzsche_slave_morality", content="slave morality — reactive, born of ressentiment, good/evil"),
        Vertex("nietzsche_perspectivism", content="perspectivism — no facts, only interpretations"),
        Vertex("nietzsche_will_to_truth", content="will to truth — a refined form of will to power"),
        Vertex("nietzsche_free_spirit", content="free spirit — beyond conventional morality, experimenter"),
        Vertex("nietzsche_will_to_power", content="will to power — fundamental drive, self-overcoming"),
        Vertex("nietzsche_prejudices_philosophers", content="prejudices of philosophers — will to truth as unquestioned value"),
        Vertex("nietzsche_beyond_good_evil", content="beyond good and evil — beyond slave morality's categories"),
        Vertex("nietzsche_natural_order_rank", content="order of rank — natural hierarchy, not equality"),
        Vertex("nietzsche_herd_instinct", content="herd instinct — democratic-egalitarian morality as decadence"),
        Vertex("nietzsche_dangerous_perhaps", content="dangerous perhaps — the philosopher's first gesture"),
        Vertex("nietzsche_nobility", content="nobility — pathos of distance, self-legislation"),
    ]
    edges = [
        Edge("nietzsche_master_morality", "nietzsche_slave_morality", EdgeType.NEGATION),
        Edge("nietzsche_slave_morality", "nietzsche_master_morality", EdgeType.NEGATION),
        Edge("nietzsche_perspectivism", "nietzsche_will_to_truth", EdgeType.NEGATION),
        Edge("nietzsche_perspectivism", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_will_to_truth", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_will_to_truth", "nietzsche_prejudices_philosophers", EdgeType.DEPENDENCY),
        Edge("nietzsche_free_spirit", "nietzsche_beyond_good_evil", EdgeType.DEPENDENCY),
        Edge("nietzsche_free_spirit", "nietzsche_herd_instinct", EdgeType.NEGATION),
        Edge("nietzsche_will_to_power", "nietzsche_master_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_beyond_good_evil", "nietzsche_slave_morality", EdgeType.NEGATION),
        Edge("nietzsche_beyond_good_evil", "nietzsche_master_morality", EdgeType.NEGATION),
        Edge("nietzsche_natural_order_rank", "nietzsche_nobility", EdgeType.DEPENDENCY),
        Edge("nietzsche_natural_order_rank", "nietzsche_herd_instinct", EdgeType.NEGATION),
        Edge("nietzsche_herd_instinct", "nietzsche_slave_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_dangerous_perhaps", "nietzsche_will_to_truth", EdgeType.NEGATION),
        Edge("nietzsche_dangerous_perhaps", "nietzsche_perspectivism", EdgeType.DEPENDENCY),
        Edge("nietzsche_nobility", "nietzsche_master_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_prejudices_philosophers", "nietzsche_perspectivism", EdgeType.NEGATION),
        # Cross-work
        Edge("nietzsche_will_to_power", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_beyond_good_evil", "nietzsche_socratic_optimism", EdgeType.NEGATION),
    ]
    return "1886: Beyond Good and Evil", vertices, edges


def work_genealogy_of_morals():
    """1887: On the Genealogy of Morals (Zur Genealogie der Moral).

    Three essays tracing the historical origins of moral values.
    First: master/slave morality and ressentiment.
    Second: guilt, bad conscience, and the sovereign individual.
    Third: the ascetic ideal and its meaning.
    """
    vertices = [
        Vertex("nietzsche_ressentiment", content="ressentiment — creative force of slave morality, reactive valuation"),
        Vertex("nietzsche_bad_conscience", content="bad conscience — internalization of cruelty, guilt, debt"),
        Vertex("nietzsche_ascetic_ideal", content="ascetic ideal — will to nothingness rather than no will"),
        Vertex("nietzsche_noble_morality", content="noble morality — self-affirming, creates values spontaneously"),
        Vertex("nietzsche_priestly_caste", content="priestly caste — spiritual revenge, transvaluation from weakness"),
        Vertex("nietzsche_sovereign_individual", content="sovereign individual — can make promises, autonomous, free"),
        Vertex("nietzsche_guilt_debt", content="guilt (Schuld) = debt — moral concepts from economic exchange"),
        Vertex("nietzsche_punishment", content="punishment — multiple meanings, no single essence"),
        Vertex("nietzsche_cruelty", content="cruelty — pleasure in suffering, festival of cruelty"),
        Vertex("nietzsche_will_to_nothingness", content="will to nothingness — the ascetic's will, nihilism"),
        Vertex("nietzsche_truthfulness", content="truthfulness — ascetic ideal's own self-destruction"),
        Vertex("nietzsche_slave_revolt", content="slave revolt in morality — ressentiment becomes creative"),
    ]
    edges = [
        Edge("nietzsche_ressentiment", "nietzsche_slave_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_ressentiment", "nietzsche_master_morality", EdgeType.NEGATION),
        Edge("nietzsche_bad_conscience", "nietzsche_cruelty", EdgeType.DEPENDENCY),
        Edge("nietzsche_bad_conscience", "nietzsche_guilt_debt", EdgeType.DEPENDENCY),
        Edge("nietzsche_ascetic_ideal", "nietzsche_will_to_nothingness", EdgeType.DEPENDENCY),
        Edge("nietzsche_ascetic_ideal", "nietzsche_will_to_power", EdgeType.NEGATION),
        Edge("nietzsche_noble_morality", "nietzsche_master_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_noble_morality", "nietzsche_ressentiment", EdgeType.NEGATION),
        Edge("nietzsche_priestly_caste", "nietzsche_ressentiment", EdgeType.DEPENDENCY),
        Edge("nietzsche_priestly_caste", "nietzsche_slave_revolt", EdgeType.DEPENDENCY),
        Edge("nietzsche_sovereign_individual", "nietzsche_bad_conscience", EdgeType.NEGATION),
        Edge("nietzsche_sovereign_individual", "nietzsche_free_spirit", EdgeType.DEPENDENCY),
        Edge("nietzsche_guilt_debt", "nietzsche_punishment", EdgeType.DEPENDENCY),
        Edge("nietzsche_punishment", "nietzsche_cruelty", EdgeType.DEPENDENCY),
        Edge("nietzsche_will_to_nothingness", "nietzsche_ascetic_ideal", EdgeType.DEPENDENCY),
        Edge("nietzsche_will_to_nothingness", "nietzsche_will_to_power", EdgeType.NEGATION),
        Edge("nietzsche_truthfulness", "nietzsche_ascetic_ideal", EdgeType.NEGATION),
        Edge("nietzsche_truthfulness", "nietzsche_will_to_truth", EdgeType.DEPENDENCY),
        Edge("nietzsche_slave_revolt", "nietzsche_ressentiment", EdgeType.DEPENDENCY),
        Edge("nietzsche_slave_revolt", "nietzsche_noble_morality", EdgeType.NEGATION),
        Edge("nietzsche_cruelty", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("nietzsche_ascetic_ideal", "nietzsche_socratic_optimism", EdgeType.REFERENCE),
        Edge("nietzsche_noble_morality", "nietzsche_nobility", EdgeType.DEPENDENCY),
    ]
    return "1887: On the Genealogy of Morals", vertices, edges


def work_thus_spoke_zarathustra():
    """1883-85: Thus Spoke Zarathustra (Also sprach Zarathustra).

    Nietzsche's philosophical novel. Introduces the Ubermensch, eternal
    recurrence, and the death of God. Zarathustra's three metamorphoses
    (camel, lion, child). The last man as the antithesis of the Ubermensch.
    """
    vertices = [
        Vertex("nietzsche_ubermensch", content="Ubermensch — beyond-man, the meaning of the earth"),
        Vertex("nietzsche_eternal_recurrence", content="eternal recurrence — the heaviest weight, amor fati"),
        Vertex("nietzsche_death_of_god", content="death of God — God is dead, we have killed him"),
        Vertex("nietzsche_last_man", content="last man — comfortable mediocrity, no aspiration"),
        Vertex("nietzsche_three_metamorphoses", content="three metamorphoses — camel (burden), lion (freedom), child (creation)"),
        Vertex("nietzsche_zarathustra", content="Zarathustra — teacher of eternal recurrence and Ubermensch"),
        Vertex("nietzsche_self_overcoming", content="self-overcoming — life as that which must always overcome itself"),
        Vertex("nietzsche_amor_fati", content="amor fati — love of fate, affirmation of all existence"),
        Vertex("nietzsche_nihilism", content="nihilism — consequence of death of God, devaluation of highest values"),
        Vertex("nietzsche_great_noon", content="great noon — moment of highest self-awareness and decision"),
        Vertex("nietzsche_spirit_of_gravity", content="spirit of gravity — the dwarf, the antagonist of lightness"),
        Vertex("nietzsche_gift_giving_virtue", content="gift-giving virtue — overflow of power, not sacrifice"),
        Vertex("nietzsche_convalescent", content="convalescent — Zarathustra's recovery from the abyss of recurrence"),
    ]
    edges = [
        Edge("nietzsche_ubermensch", "nietzsche_last_man", EdgeType.NEGATION),
        Edge("nietzsche_ubermensch", "nietzsche_self_overcoming", EdgeType.DEPENDENCY),
        Edge("nietzsche_ubermensch", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_eternal_recurrence", "nietzsche_nihilism", EdgeType.NEGATION),
        Edge("nietzsche_eternal_recurrence", "nietzsche_amor_fati", EdgeType.DEPENDENCY),
        Edge("nietzsche_death_of_god", "nietzsche_nihilism", EdgeType.DEPENDENCY),
        Edge("nietzsche_death_of_god", "nietzsche_ubermensch", EdgeType.DEPENDENCY),
        Edge("nietzsche_last_man", "nietzsche_ubermensch", EdgeType.NEGATION),
        Edge("nietzsche_last_man", "nietzsche_herd_instinct", EdgeType.DEPENDENCY),
        Edge("nietzsche_three_metamorphoses", "nietzsche_self_overcoming", EdgeType.DEPENDENCY),
        Edge("nietzsche_zarathustra", "nietzsche_ubermensch", EdgeType.DEPENDENCY),
        Edge("nietzsche_zarathustra", "nietzsche_eternal_recurrence", EdgeType.DEPENDENCY),
        Edge("nietzsche_self_overcoming", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_amor_fati", "nietzsche_eternal_recurrence", EdgeType.DEPENDENCY),
        Edge("nietzsche_amor_fati", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_nihilism", "nietzsche_ascetic_ideal", EdgeType.DEPENDENCY),
        Edge("nietzsche_great_noon", "nietzsche_eternal_recurrence", EdgeType.DEPENDENCY),
        Edge("nietzsche_great_noon", "nietzsche_ubermensch", EdgeType.DEPENDENCY),
        Edge("nietzsche_spirit_of_gravity", "nietzsche_eternal_recurrence", EdgeType.NEGATION),
        Edge("nietzsche_gift_giving_virtue", "nietzsche_ubermensch", EdgeType.DEPENDENCY),
        Edge("nietzsche_gift_giving_virtue", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_convalescent", "nietzsche_eternal_recurrence", EdgeType.DEPENDENCY),
        Edge("nietzsche_convalescent", "nietzsche_zarathustra", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("nietzsche_ubermensch", "nietzsche_beyond_good_evil", EdgeType.DEPENDENCY),
        Edge("nietzsche_death_of_god", "nietzsche_dionysian", EdgeType.REFERENCE),
        Edge("nietzsche_eternal_recurrence", "nietzsche_dionysian", EdgeType.DEPENDENCY),
    ]
    return "1883-85: Thus Spoke Zarathustra", vertices, edges


def work_twilight_of_idols():
    """1889: Twilight of the Idols (Gotzen-Dammerung).

    Nietzsche's last original book. 'Sounding out idols' —
    the transvaluation of all values in condensed form. Critique
    of Western philosophy as symptom of decadence. The 'true world'
    becomes a fable.
    """
    vertices = [
        Vertex("nietzsche_transvaluation", content="transvaluation of all values — inverting the moral hierarchy"),
        Vertex("nietzsche_idols", content="idols — the eternal idols here sounded out with a hammer"),
        Vertex("nietzsche_hammer_philosophy", content="philosophy with a hammer — sounding out hollow idols"),
        Vertex("nietzsche_true_world_fable", content="how the true world became a fable — history of an error"),
        Vertex("nietzsche_decadence", content="decadence — decline of vital instincts, symptom reading"),
        Vertex("nietzsche_anti_nature", content="morality as anti-nature — morality against the instincts"),
        Vertex("nietzsche_four_errors", content="four great errors — false causality, free will, imaginary causes, purpose"),
        Vertex("nietzsche_healthy_instinct", content="healthy instinct — affirmative, life-enhancing"),
        Vertex("nietzsche_socrates_problem", content="the problem of Socrates — Socrates as decadent, dialectic as revenge"),
        Vertex("nietzsche_german_culture_critique", content="what the Germans lack — critique of German Bildung"),
        Vertex("nietzsche_realism", content="realism — not idealism, not pessimism, but Dionysian affirmation"),
    ]
    edges = [
        Edge("nietzsche_transvaluation", "nietzsche_slave_morality", EdgeType.NEGATION),
        Edge("nietzsche_transvaluation", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_transvaluation", "nietzsche_beyond_good_evil", EdgeType.DEPENDENCY),
        Edge("nietzsche_idols", "nietzsche_hammer_philosophy", EdgeType.DEPENDENCY),
        Edge("nietzsche_hammer_philosophy", "nietzsche_perspectivism", EdgeType.DEPENDENCY),
        Edge("nietzsche_true_world_fable", "nietzsche_perspectivism", EdgeType.DEPENDENCY),
        Edge("nietzsche_true_world_fable", "nietzsche_nihilism", EdgeType.DEPENDENCY),
        Edge("nietzsche_decadence", "nietzsche_ascetic_ideal", EdgeType.DEPENDENCY),
        Edge("nietzsche_decadence", "nietzsche_slave_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_anti_nature", "nietzsche_slave_morality", EdgeType.DEPENDENCY),
        Edge("nietzsche_anti_nature", "nietzsche_healthy_instinct", EdgeType.NEGATION),
        Edge("nietzsche_four_errors", "nietzsche_perspectivism", EdgeType.DEPENDENCY),
        Edge("nietzsche_four_errors", "nietzsche_will_to_truth", EdgeType.NEGATION),
        Edge("nietzsche_healthy_instinct", "nietzsche_will_to_power", EdgeType.DEPENDENCY),
        Edge("nietzsche_healthy_instinct", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_socrates_problem", "nietzsche_socratic_optimism", EdgeType.DEPENDENCY),
        Edge("nietzsche_socrates_problem", "nietzsche_decadence", EdgeType.DEPENDENCY),
        Edge("nietzsche_socrates_problem", "nietzsche_socrates", EdgeType.DEPENDENCY),
        Edge("nietzsche_german_culture_critique", "nietzsche_decadence", EdgeType.DEPENDENCY),
        Edge("nietzsche_realism", "nietzsche_dionysian", EdgeType.DEPENDENCY),
        Edge("nietzsche_realism", "nietzsche_true_world_fable", EdgeType.DEPENDENCY),
        Edge("nietzsche_realism", "nietzsche_amor_fati", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("nietzsche_transvaluation", "nietzsche_ubermensch", EdgeType.REFERENCE),
        Edge("nietzsche_decadence", "nietzsche_ressentiment", EdgeType.DEPENDENCY),
    ]
    return "1889: Twilight of the Idols", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_birth_of_tragedy,
    work_beyond_good_and_evil,
    work_genealogy_of_morals,
    work_thus_spoke_zarathustra,
    work_twilight_of_idols,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("NIETZSCHE COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
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

        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in w_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)
                    new_e_count += 1

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
        "experiment": "nietzsche_full_incremental",
        "source": "Nietzsche — 5 major works (1872-1889)",
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

    output_path = "experiment_nietzsche.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
