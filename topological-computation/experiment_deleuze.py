"""Gilles Deleuze — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Deleuze's thought —
from the early monographs (Nietzsche, Bergson, Spinoza) through
the great works with Guattari (Anti-Oedipus, A Thousand Plateaus)
to the final synthesis (What is Philosophy?) —
as an incremental topological growth process.

Covers 5 major works (1968-1991).
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
#   deleuze_difference, deleuze_repetition, deleuze_virtual,
#   deleuze_actual, deleuze_intensity, deleuze_rhizome,
#   deleuze_assemblage, deleuze_bwo, deleuze_deterritorialization,
#   deleuze_plane_of_immanence, deleuze_becoming, deleuze_concept
# ---------------------------------------------------------------------------


def work_difference_and_repetition():
    """1968: Difference and Repetition (Difference et Repetition).

    Deleuze's magnum opus. Critiques the subordination of difference
    to identity in the history of philosophy. Develops a philosophy of
    difference-in-itself and repetition as productive, not as copying.
    The virtual/actual distinction replaces the possible/real.
    """
    vertices = [
        Vertex("deleuze_difference", content="difference-in-itself — not subordinated to identity or analogy"),
        Vertex("deleuze_repetition", content="repetition — productive, creates the new, not copying"),
        Vertex("deleuze_virtual", content="virtual — fully real but not actual, differential field"),
        Vertex("deleuze_actual", content="actual — actualization of the virtual, differentiation"),
        Vertex("deleuze_intensity", content="intensity — difference in itself, prior to quality and extension"),
        Vertex("deleuze_representation", content="representation — the four shackles: identity, analogy, opposition, resemblance"),
        Vertex("deleuze_identity_critique", content="critique of identity — identity is secondary effect of difference"),
        Vertex("deleuze_simulacrum", content="simulacrum — copy without original, overturns Platonism"),
        Vertex("deleuze_eternal_return", content="eternal return — return of difference, not the same"),
        Vertex("deleuze_problematic", content="problematic — problems are not reducible to solutions"),
        Vertex("deleuze_idea", content="Idea — differential structure, multiplicity"),
        Vertex("deleuze_individuation", content="individuation — process, not pre-given individual"),
        Vertex("deleuze_dramatization", content="dramatization — spatio-temporal dynamisms beneath concepts"),
        Vertex("deleuze_multiplicity", content="multiplicity — neither one nor many, continuous manifold"),
    ]
    edges = [
        Edge("deleuze_difference", "deleuze_identity_critique", EdgeType.DEPENDENCY),
        Edge("deleuze_difference", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_repetition", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_repetition", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_virtual", "deleuze_actual", EdgeType.NEGATION),
        Edge("deleuze_virtual", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_actual", "deleuze_virtual", EdgeType.DEPENDENCY),
        Edge("deleuze_intensity", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_intensity", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_identity_critique", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_simulacrum", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_simulacrum", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_eternal_return", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_eternal_return", "deleuze_repetition", EdgeType.DEPENDENCY),
        Edge("deleuze_problematic", "deleuze_idea", EdgeType.DEPENDENCY),
        Edge("deleuze_problematic", "deleuze_virtual", EdgeType.DEPENDENCY),
        Edge("deleuze_idea", "deleuze_multiplicity", EdgeType.DEPENDENCY),
        Edge("deleuze_idea", "deleuze_virtual", EdgeType.DEPENDENCY),
        Edge("deleuze_individuation", "deleuze_intensity", EdgeType.DEPENDENCY),
        Edge("deleuze_individuation", "deleuze_virtual", EdgeType.DEPENDENCY),
        Edge("deleuze_dramatization", "deleuze_individuation", EdgeType.DEPENDENCY),
        Edge("deleuze_dramatization", "deleuze_idea", EdgeType.DEPENDENCY),
        Edge("deleuze_multiplicity", "deleuze_difference", EdgeType.DEPENDENCY),
        Edge("deleuze_multiplicity", "deleuze_identity_critique", EdgeType.DEPENDENCY),
    ]
    return "1968: Difference and Repetition", vertices, edges


def work_logic_of_sense():
    """1969: The Logic of Sense (Logique du sens).

    Develops a theory of sense as a surface event, distinct from both
    bodies (depths) and propositions. Uses Carroll and the Stoics.
    The paradoxes of sense (regress, proliferation, neutrality).
    """
    vertices = [
        Vertex("deleuze_sense", content="sense — surface effect, neither body nor proposition"),
        Vertex("deleuze_event", content="event — incorporeal transformation, pure becoming"),
        Vertex("deleuze_series", content="series — heterogeneous series converge/diverge"),
        Vertex("deleuze_paradox", content="paradox — essential to sense, not deficiency"),
        Vertex("deleuze_surface", content="surface — where sense is produced, between depth and height"),
        Vertex("deleuze_phantasm", content="phantasm — simulacrum on the surface, desexualized energy"),
        Vertex("deleuze_nonsense", content="nonsense — condition of sense, not its opposite"),
        Vertex("deleuze_aion", content="Aion — pure empty time of the event, versus Chronos"),
        Vertex("deleuze_chronos", content="Chronos — time of bodies, living present"),
        Vertex("deleuze_quasi_cause", content="quasi-cause — events cause each other on the surface"),
        Vertex("deleuze_humor", content="humor — Stoic descent to surfaces, versus irony's ascent"),
    ]
    edges = [
        Edge("deleuze_sense", "deleuze_event", EdgeType.DEPENDENCY),
        Edge("deleuze_sense", "deleuze_surface", EdgeType.DEPENDENCY),
        Edge("deleuze_event", "deleuze_aion", EdgeType.DEPENDENCY),
        Edge("deleuze_event", "deleuze_chronos", EdgeType.NEGATION),
        Edge("deleuze_series", "deleuze_sense", EdgeType.DEPENDENCY),
        Edge("deleuze_series", "deleuze_paradox", EdgeType.DEPENDENCY),
        Edge("deleuze_paradox", "deleuze_sense", EdgeType.DEPENDENCY),
        Edge("deleuze_paradox", "deleuze_nonsense", EdgeType.DEPENDENCY),
        Edge("deleuze_surface", "deleuze_event", EdgeType.DEPENDENCY),
        Edge("deleuze_phantasm", "deleuze_surface", EdgeType.DEPENDENCY),
        Edge("deleuze_phantasm", "deleuze_simulacrum", EdgeType.REFERENCE),
        Edge("deleuze_nonsense", "deleuze_sense", EdgeType.NEGATION),
        Edge("deleuze_aion", "deleuze_chronos", EdgeType.NEGATION),
        Edge("deleuze_quasi_cause", "deleuze_event", EdgeType.DEPENDENCY),
        Edge("deleuze_quasi_cause", "deleuze_surface", EdgeType.DEPENDENCY),
        Edge("deleuze_humor", "deleuze_surface", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("deleuze_event", "deleuze_difference", EdgeType.REFERENCE),
        Edge("deleuze_sense", "deleuze_virtual", EdgeType.REFERENCE),
    ]
    return "1969: The Logic of Sense", vertices, edges


def work_anti_oedipus():
    """1972: Anti-Oedipus (L'Anti-Oedipe, with Felix Guattari).

    First volume of Capitalism and Schizophrenia. Critiques
    psychoanalysis and the Oedipal triangle. Introduces desiring-machines,
    the body without organs, schizoanalysis. Desire is productive,
    not lacking.
    """
    vertices = [
        Vertex("deleuze_desiring_machine", content="desiring-machine — desire as production, not lack"),
        Vertex("deleuze_bwo", content="body without organs (BwO) — limit of desiring-production, surface of recording"),
        Vertex("deleuze_deterritorialization", content="deterritorialization — movement away from coded territory"),
        Vertex("deleuze_reterritorialization", content="reterritorialization — recapture of flows onto new territory"),
        Vertex("deleuze_schizoanalysis", content="schizoanalysis — alternative to psychoanalysis, follows desire's lines"),
        Vertex("deleuze_oedipus_critique", content="critique of Oedipus — the Oedipal triangle as repressive apparatus"),
        Vertex("deleuze_desire_production", content="desire is production — not lack, not representation"),
        Vertex("deleuze_socius", content="socius — the social body that codes flows"),
        Vertex("deleuze_capitalism_flows", content="capitalism — decodes flows, axiomatizes, never enough deterritorialization"),
        Vertex("deleuze_schizo", content="the schizo — revolutionary process, not clinical entity"),
        Vertex("deleuze_paranoid_pole", content="paranoid-fascist pole — molar, reterritorializing"),
        Vertex("deleuze_schizo_pole", content="schizoid-revolutionary pole — molecular, deterritorializing"),
        Vertex("deleuze_coding", content="coding — inscribing flows on the socius"),
        Vertex("deleuze_overcoding", content="overcoding — despotic appropriation of codes"),
    ]
    edges = [
        Edge("deleuze_desiring_machine", "deleuze_desire_production", EdgeType.DEPENDENCY),
        Edge("deleuze_desiring_machine", "deleuze_oedipus_critique", EdgeType.NEGATION),
        Edge("deleuze_bwo", "deleuze_desiring_machine", EdgeType.NEGATION),
        Edge("deleuze_bwo", "deleuze_desire_production", EdgeType.DEPENDENCY),
        Edge("deleuze_deterritorialization", "deleuze_reterritorialization", EdgeType.NEGATION),
        Edge("deleuze_deterritorialization", "deleuze_coding", EdgeType.NEGATION),
        Edge("deleuze_reterritorialization", "deleuze_deterritorialization", EdgeType.NEGATION),
        Edge("deleuze_schizoanalysis", "deleuze_oedipus_critique", EdgeType.DEPENDENCY),
        Edge("deleuze_schizoanalysis", "deleuze_desiring_machine", EdgeType.DEPENDENCY),
        Edge("deleuze_oedipus_critique", "deleuze_desire_production", EdgeType.DEPENDENCY),
        Edge("deleuze_desire_production", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_socius", "deleuze_coding", EdgeType.DEPENDENCY),
        Edge("deleuze_capitalism_flows", "deleuze_deterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_capitalism_flows", "deleuze_reterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_capitalism_flows", "deleuze_coding", EdgeType.NEGATION),
        Edge("deleuze_schizo", "deleuze_deterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_schizo", "deleuze_oedipus_critique", EdgeType.DEPENDENCY),
        Edge("deleuze_paranoid_pole", "deleuze_reterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_schizo_pole", "deleuze_deterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_paranoid_pole", "deleuze_schizo_pole", EdgeType.NEGATION),
        Edge("deleuze_coding", "deleuze_socius", EdgeType.DEPENDENCY),
        Edge("deleuze_overcoding", "deleuze_coding", EdgeType.DEPENDENCY),
        Edge("deleuze_overcoding", "deleuze_deterritorialization", EdgeType.NEGATION),
        # Cross-work
        Edge("deleuze_desire_production", "deleuze_difference", EdgeType.REFERENCE),
        Edge("deleuze_bwo", "deleuze_intensity", EdgeType.REFERENCE),
    ]
    return "1972: Anti-Oedipus", vertices, edges


def work_thousand_plateaus():
    """1980: A Thousand Plateaus (Mille Plateaux, with Felix Guattari).

    Second volume of Capitalism and Schizophrenia. Introduces the
    rhizome, smooth/striated space, becomings, assemblage, abstract
    machine, war machine/State apparatus, faciality, refrain.
    Each plateau is a concept-cluster.
    """
    vertices = [
        Vertex("deleuze_rhizome", content="rhizome — non-hierarchical connection, any point to any point"),
        Vertex("deleuze_assemblage", content="assemblage (agencement) — heterogeneous arrangement of bodies and enunciations"),
        Vertex("deleuze_smooth_space", content="smooth space — nomadic, intensive, haptic"),
        Vertex("deleuze_striated_space", content="striated space — State, extensive, optical, gridded"),
        Vertex("deleuze_becoming", content="becoming — becoming-animal, -woman, -imperceptible; not imitation"),
        Vertex("deleuze_plane_of_immanence", content="plane of immanence — pre-philosophical field, no transcendence"),
        Vertex("deleuze_war_machine", content="war machine — exterior to the State, nomadic invention"),
        Vertex("deleuze_state_apparatus", content="State apparatus — capture, striation, overcoding"),
        Vertex("deleuze_abstract_machine", content="abstract machine — diagrammatic, operates before forms/substances"),
        Vertex("deleuze_molar", content="molar — large-scale aggregates, segmented, organized"),
        Vertex("deleuze_molecular", content="molecular — micro-processes, flows, becomings"),
        Vertex("deleuze_line_of_flight", content="line of flight — absolute deterritorialization, creative escape"),
        Vertex("deleuze_refrain", content="refrain (ritournelle) — territorial motif, organizing rhythm"),
        Vertex("deleuze_faciality", content="faciality — white wall/black hole system, overcoding of body"),
        Vertex("deleuze_body_politic", content="body politic — the three bodies: earth, despot, capital"),
        Vertex("deleuze_nomadology", content="nomadology — treatise on the war machine, anti-State thought"),
        Vertex("deleuze_plateau", content="plateau — zone of continuous intensity, not climax"),
    ]
    edges = [
        Edge("deleuze_rhizome", "deleuze_assemblage", EdgeType.DEPENDENCY),
        Edge("deleuze_rhizome", "deleuze_representation", EdgeType.NEGATION),
        Edge("deleuze_rhizome", "deleuze_multiplicity", EdgeType.DEPENDENCY),
        Edge("deleuze_assemblage", "deleuze_abstract_machine", EdgeType.DEPENDENCY),
        Edge("deleuze_smooth_space", "deleuze_striated_space", EdgeType.NEGATION),
        Edge("deleuze_striated_space", "deleuze_smooth_space", EdgeType.NEGATION),
        Edge("deleuze_becoming", "deleuze_identity_critique", EdgeType.DEPENDENCY),
        Edge("deleuze_becoming", "deleuze_molecular", EdgeType.DEPENDENCY),
        Edge("deleuze_becoming", "deleuze_molar", EdgeType.NEGATION),
        Edge("deleuze_plane_of_immanence", "deleuze_virtual", EdgeType.DEPENDENCY),
        Edge("deleuze_plane_of_immanence", "deleuze_difference", EdgeType.REFERENCE),
        Edge("deleuze_war_machine", "deleuze_state_apparatus", EdgeType.NEGATION),
        Edge("deleuze_war_machine", "deleuze_smooth_space", EdgeType.DEPENDENCY),
        Edge("deleuze_war_machine", "deleuze_line_of_flight", EdgeType.DEPENDENCY),
        Edge("deleuze_state_apparatus", "deleuze_striated_space", EdgeType.DEPENDENCY),
        Edge("deleuze_state_apparatus", "deleuze_overcoding", EdgeType.DEPENDENCY),
        Edge("deleuze_abstract_machine", "deleuze_assemblage", EdgeType.DEPENDENCY),
        Edge("deleuze_abstract_machine", "deleuze_virtual", EdgeType.REFERENCE),
        Edge("deleuze_molar", "deleuze_molecular", EdgeType.NEGATION),
        Edge("deleuze_molecular", "deleuze_molar", EdgeType.NEGATION),
        Edge("deleuze_line_of_flight", "deleuze_deterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_line_of_flight", "deleuze_reterritorialization", EdgeType.NEGATION),
        Edge("deleuze_refrain", "deleuze_deterritorialization", EdgeType.NEGATION),
        Edge("deleuze_refrain", "deleuze_assemblage", EdgeType.DEPENDENCY),
        Edge("deleuze_faciality", "deleuze_overcoding", EdgeType.DEPENDENCY),
        Edge("deleuze_faciality", "deleuze_becoming", EdgeType.NEGATION),
        Edge("deleuze_nomadology", "deleuze_war_machine", EdgeType.DEPENDENCY),
        Edge("deleuze_nomadology", "deleuze_state_apparatus", EdgeType.NEGATION),
        Edge("deleuze_plateau", "deleuze_rhizome", EdgeType.DEPENDENCY),
        Edge("deleuze_plateau", "deleuze_intensity", EdgeType.REFERENCE),
        Edge("deleuze_body_politic", "deleuze_socius", EdgeType.REFERENCE),
        # Cross-work
        Edge("deleuze_smooth_space", "deleuze_bwo", EdgeType.REFERENCE),
        Edge("deleuze_assemblage", "deleuze_desiring_machine", EdgeType.REFERENCE),
    ]
    return "1980: A Thousand Plateaus", vertices, edges


def work_what_is_philosophy():
    """1991: What is Philosophy? (Qu'est-ce que la philosophie?, with Guattari).

    Final collaborative work. Distinguishes philosophy (concepts on plane
    of immanence), science (functions on plane of reference), and art
    (percepts/affects on plane of composition). The concept has a history
    but is not reducible to it.
    """
    vertices = [
        Vertex("deleuze_concept", content="concept — philosophical creation on the plane of immanence"),
        Vertex("deleuze_conceptual_personae", content="conceptual personae — the thinker's persona, pre-philosophical image"),
        Vertex("deleuze_plane_of_reference", content="plane of reference — science, functions, propositions"),
        Vertex("deleuze_plane_of_composition", content="plane of composition — art, percepts, affects"),
        Vertex("deleuze_percept", content="percept — non-human perception, wrested from perceptions"),
        Vertex("deleuze_affect", content="affect — non-human becoming, wrested from affections"),
        Vertex("deleuze_functive", content="functive — element of science, slows infinite speed to limit"),
        Vertex("deleuze_geo_philosophy", content="geo-philosophy — philosophy needs a milieu, Greek contingency"),
        Vertex("deleuze_opinion", content="opinion (doxa) — what philosophy, science, art fight against"),
        Vertex("deleuze_chaos", content="chaos — infinite speed where nothing takes form"),
        Vertex("deleuze_brain", content="brain — junction of planes, not metaphor"),
    ]
    edges = [
        Edge("deleuze_concept", "deleuze_plane_of_immanence", EdgeType.DEPENDENCY),
        Edge("deleuze_concept", "deleuze_opinion", EdgeType.NEGATION),
        Edge("deleuze_concept", "deleuze_conceptual_personae", EdgeType.DEPENDENCY),
        Edge("deleuze_conceptual_personae", "deleuze_plane_of_immanence", EdgeType.DEPENDENCY),
        Edge("deleuze_plane_of_reference", "deleuze_plane_of_immanence", EdgeType.NEGATION),
        Edge("deleuze_plane_of_composition", "deleuze_plane_of_immanence", EdgeType.NEGATION),
        Edge("deleuze_plane_of_composition", "deleuze_plane_of_reference", EdgeType.NEGATION),
        Edge("deleuze_percept", "deleuze_plane_of_composition", EdgeType.DEPENDENCY),
        Edge("deleuze_affect", "deleuze_plane_of_composition", EdgeType.DEPENDENCY),
        Edge("deleuze_affect", "deleuze_becoming", EdgeType.DEPENDENCY),
        Edge("deleuze_functive", "deleuze_plane_of_reference", EdgeType.DEPENDENCY),
        Edge("deleuze_functive", "deleuze_concept", EdgeType.NEGATION),
        Edge("deleuze_geo_philosophy", "deleuze_plane_of_immanence", EdgeType.DEPENDENCY),
        Edge("deleuze_geo_philosophy", "deleuze_deterritorialization", EdgeType.DEPENDENCY),
        Edge("deleuze_opinion", "deleuze_chaos", EdgeType.NEGATION),
        Edge("deleuze_chaos", "deleuze_plane_of_immanence", EdgeType.DEPENDENCY),
        Edge("deleuze_chaos", "deleuze_plane_of_reference", EdgeType.DEPENDENCY),
        Edge("deleuze_chaos", "deleuze_plane_of_composition", EdgeType.DEPENDENCY),
        Edge("deleuze_brain", "deleuze_concept", EdgeType.REFERENCE),
        Edge("deleuze_brain", "deleuze_percept", EdgeType.REFERENCE),
        Edge("deleuze_brain", "deleuze_functive", EdgeType.REFERENCE),
        # Cross-work
        Edge("deleuze_concept", "deleuze_difference", EdgeType.REFERENCE),
        Edge("deleuze_plane_of_immanence", "deleuze_rhizome", EdgeType.REFERENCE),
        Edge("deleuze_affect", "deleuze_intensity", EdgeType.REFERENCE),
    ]
    return "1991: What is Philosophy?", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_difference_and_repetition,
    work_logic_of_sense,
    work_anti_oedipus,
    work_thousand_plateaus,
    work_what_is_philosophy,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("DELEUZE COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
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
        "experiment": "deleuze_full_incremental",
        "source": "Deleuze — 5 major works (1968-1991)",
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

    output_path = "experiment_deleuze.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
