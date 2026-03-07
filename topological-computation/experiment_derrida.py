"""Jacques Derrida — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Derrida's thought —
from the early critiques of Husserl and structuralism through
deconstruction, differance, and the later ethico-political turn —
as an incremental topological growth process.

Covers ~5 major works + related conceptual clusters (1967-1993).
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
#   derrida_differance, derrida_trace, derrida_supplement,
#   derrida_presence, derrida_sign, derrida_logocentrism,
#   derrida_writing, derrida_speech, derrida_center,
#   derrida_play, derrida_structure, derrida_absence
# ---------------------------------------------------------------------------


def work_speech_and_phenomena():
    """1967: Speech and Phenomena (La voix et le phenomene).

    Derrida's most concentrated critique of Husserl's phenomenology.
    Attacks the privilege of self-present voice (phonocentrism) and the
    distinction between indication and expression. Shows that the sign
    always already contaminates presence.
    """
    vertices = [
        Vertex("derrida_presence", content="presence (Gegenwart) — metaphysical foundation Derrida deconstructs"),
        Vertex("derrida_voice", content="voice (phonè) — illusory self-presence of speaking subject"),
        Vertex("derrida_sign", content="sign — always deferred, never purely present"),
        Vertex("derrida_indication", content="indication (Anzeichen) — Husserl's exterior sign"),
        Vertex("derrida_expression", content="expression (Ausdruck) — Husserl's meaning-bearing sign"),
        Vertex("derrida_solitary_mental_life", content="solitary mental life — Husserl's ideal of pure expression"),
        Vertex("derrida_representation", content="representation (Vorstellung) — re-presentation as repetition"),
        Vertex("derrida_living_present", content="living present (lebendige Gegenwart) — temporal self-awareness"),
        Vertex("derrida_retention", content="retention — non-present element within the present"),
        Vertex("derrida_iterability_early", content="iterability — sign must be repeatable in absence of origin"),
    ]
    edges = [
        Edge("derrida_voice", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_sign", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_indication", "derrida_expression", EdgeType.NEGATION),
        Edge("derrida_expression", "derrida_solitary_mental_life", EdgeType.DEPENDENCY),
        Edge("derrida_solitary_mental_life", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_voice", "derrida_expression", EdgeType.DEPENDENCY),
        Edge("derrida_representation", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_living_present", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_retention", "derrida_living_present", EdgeType.NEGATION),
        Edge("derrida_retention", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_iterability_early", "derrida_sign", EdgeType.DEPENDENCY),
        Edge("derrida_iterability_early", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_indication", "derrida_sign", EdgeType.DEPENDENCY),
        Edge("derrida_expression", "derrida_sign", EdgeType.DEPENDENCY),
    ]
    return "1967: Speech and Phenomena", vertices, edges


def work_of_grammatology():
    """1967: Of Grammatology (De la grammatologie).

    Derrida's most systematic work. Introduces arche-writing, the logic
    of the supplement, and the critique of logocentrism. The reading of
    Rousseau demonstrates that writing is not derivative of speech but
    is the condition of possibility for all signification.
    """
    vertices = [
        Vertex("derrida_differance", content="differance — neither word nor concept, the movement of differing/deferring"),
        Vertex("derrida_trace", content="trace — mark of absent presence, origin without origin"),
        Vertex("derrida_supplement", content="supplement — that which adds to and replaces, logic of supplementarity"),
        Vertex("derrida_arche_writing", content="arche-writing (archi-ecriture) — originary trace prior to speech/writing"),
        Vertex("derrida_logocentrism", content="logocentrism — metaphysical privilege of logos, speech, presence"),
        Vertex("derrida_phonocentrism", content="phonocentrism — privilege of voice over writing"),
        Vertex("derrida_writing", content="writing (ecriture) — not secondary to speech, condition of all signification"),
        Vertex("derrida_speech", content="speech (parole) — falsely privileged over writing in Western metaphysics"),
        Vertex("derrida_saussure_critique", content="critique of Saussure — the signifier/signified distinction deconstructs itself"),
        Vertex("derrida_rousseau_reading", content="Rousseau reading — supplement as dangerous, nature/culture undone"),
        Vertex("derrida_nature_culture", content="nature/culture opposition — deconstructed by supplementarity"),
        Vertex("derrida_grammatology", content="grammatology — science of writing, never fully constituted"),
        Vertex("derrida_metaphysics_of_presence", content="metaphysics of presence — the dominant tradition Derrida deconstructs"),
        Vertex("derrida_signifier_signified", content="signifier/signified — the distinction is never pure"),
        Vertex("derrida_levi_strauss_critique", content="critique of Levi-Strauss — bricoleur and engineer, nature/culture"),
    ]
    edges = [
        Edge("derrida_differance", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_differance", "derrida_trace", EdgeType.DEPENDENCY),
        Edge("derrida_trace", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_trace", "derrida_arche_writing", EdgeType.DEPENDENCY),
        Edge("derrida_supplement", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_supplement", "derrida_nature_culture", EdgeType.NEGATION),
        Edge("derrida_arche_writing", "derrida_writing", EdgeType.DEPENDENCY),
        Edge("derrida_arche_writing", "derrida_speech", EdgeType.NEGATION),
        Edge("derrida_logocentrism", "derrida_metaphysics_of_presence", EdgeType.DEPENDENCY),
        Edge("derrida_logocentrism", "derrida_phonocentrism", EdgeType.DEPENDENCY),
        Edge("derrida_phonocentrism", "derrida_speech", EdgeType.DEPENDENCY),
        Edge("derrida_phonocentrism", "derrida_writing", EdgeType.NEGATION),
        Edge("derrida_writing", "derrida_speech", EdgeType.NEGATION),
        Edge("derrida_saussure_critique", "derrida_signifier_signified", EdgeType.NEGATION),
        Edge("derrida_saussure_critique", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_rousseau_reading", "derrida_supplement", EdgeType.DEPENDENCY),
        Edge("derrida_rousseau_reading", "derrida_nature_culture", EdgeType.DEPENDENCY),
        Edge("derrida_nature_culture", "derrida_logocentrism", EdgeType.DEPENDENCY),
        Edge("derrida_grammatology", "derrida_arche_writing", EdgeType.DEPENDENCY),
        Edge("derrida_grammatology", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_signifier_signified", "derrida_differance", EdgeType.NEGATION),
        Edge("derrida_metaphysics_of_presence", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_levi_strauss_critique", "derrida_nature_culture", EdgeType.NEGATION),
        Edge("derrida_levi_strauss_critique", "derrida_logocentrism", EdgeType.NEGATION),
        # Cross-work
        Edge("derrida_differance", "derrida_sign", EdgeType.DEPENDENCY),
        Edge("derrida_arche_writing", "derrida_voice", EdgeType.NEGATION),
        Edge("derrida_supplement", "derrida_representation", EdgeType.REFERENCE),
    ]
    return "1967: Of Grammatology", vertices, edges


def work_writing_and_difference():
    """1967: Writing and Difference (L'ecriture et la difference).

    Collection of essays including 'Structure, Sign and Play' (the
    Sorbonne lecture that inaugurated deconstruction), readings of
    Levinas, Foucault, Freud, Artaud, and Bataille. Introduces the
    concept of play as alternative to centered structure.
    """
    vertices = [
        Vertex("derrida_structure", content="structure — the concept of structure and its center"),
        Vertex("derrida_center", content="center — that which organizes structure while escaping structurality"),
        Vertex("derrida_play", content="play (jeu) — free play of signification without center"),
        Vertex("derrida_absence", content="absence — constitutive absence at the origin"),
        Vertex("derrida_freeplay", content="freeplay of the world — Nietzschean affirmation without nostalgia"),
        Vertex("derrida_nostalgia", content="nostalgia for origin — Rousseauist desire for lost presence"),
        Vertex("derrida_cogito_madness", content="cogito and madness — Foucault critique, reason excludes unreason"),
        Vertex("derrida_force_signification", content="force and signification — force exceeds structural form"),
        Vertex("derrida_freud_scene", content="Freud's mystic writing pad — trace and the psychic apparatus"),
        Vertex("derrida_levinas_violence", content="violence and metaphysics — Levinas, the Other, totality"),
        Vertex("derrida_artaud", content="theater of cruelty — Artaud, non-representational theater"),
        Vertex("derrida_bataille_economy", content="restricted vs general economy — Bataille, expenditure, sovereignty"),
    ]
    edges = [
        Edge("derrida_center", "derrida_structure", EdgeType.DEPENDENCY),
        Edge("derrida_center", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_play", "derrida_center", EdgeType.NEGATION),
        Edge("derrida_play", "derrida_structure", EdgeType.NEGATION),
        Edge("derrida_absence", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_absence", "derrida_center", EdgeType.NEGATION),
        Edge("derrida_freeplay", "derrida_play", EdgeType.DEPENDENCY),
        Edge("derrida_freeplay", "derrida_nostalgia", EdgeType.NEGATION),
        Edge("derrida_nostalgia", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_cogito_madness", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_force_signification", "derrida_structure", EdgeType.NEGATION),
        Edge("derrida_freud_scene", "derrida_trace", EdgeType.DEPENDENCY),
        Edge("derrida_freud_scene", "derrida_writing", EdgeType.DEPENDENCY),
        Edge("derrida_levinas_violence", "derrida_metaphysics_of_presence", EdgeType.NEGATION),
        Edge("derrida_levinas_violence", "derrida_absence", EdgeType.REFERENCE),
        Edge("derrida_artaud", "derrida_representation", EdgeType.NEGATION),
        Edge("derrida_bataille_economy", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_bataille_economy", "derrida_play", EdgeType.REFERENCE),
        # Cross-work
        Edge("derrida_play", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_structure", "derrida_signifier_signified", EdgeType.REFERENCE),
    ]
    return "1967: Writing and Difference", vertices, edges


def work_margins_of_philosophy():
    """1972: Margins of Philosophy (Marges de la philosophie).

    Includes 'Differance' (the definitive essay on the concept),
    'White Mythology' (on metaphor in philosophy), 'Signature Event
    Context' (iterability thesis), and essays on Hegel, Heidegger,
    Austin. Introduces sous rature, pharmakon, dissemination.
    """
    vertices = [
        Vertex("derrida_sous_rature", content="sous rature (under erasure) — using concepts while crossing them out"),
        Vertex("derrida_pharmakon", content="pharmakon — both remedy and poison, undecidable"),
        Vertex("derrida_dissemination", content="dissemination — meaning scatters, cannot be gathered"),
        Vertex("derrida_white_mythology", content="white mythology — metaphor in philosophy, concept as worn metaphor"),
        Vertex("derrida_iterability", content="iterability — structural repeatability in new contexts, breaks with origin"),
        Vertex("derrida_signature", content="signature — paradox: must be repeatable yet singular"),
        Vertex("derrida_event", content="event — rupture that is never pure, always iterable"),
        Vertex("derrida_context", content="context — never fully determinable, always open"),
        Vertex("derrida_austin_critique", content="critique of Austin — serious/parasitic distinction deconstructs itself"),
        Vertex("derrida_tympan", content="tympan — the limit of philosophy, the margin"),
        Vertex("derrida_hegel_aufhebung_critique", content="critique of Aufhebung — sublation as reappropriation of difference"),
        Vertex("derrida_catachresis", content="catachresis — forced metaphor without proper meaning"),
    ]
    edges = [
        Edge("derrida_sous_rature", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_sous_rature", "derrida_metaphysics_of_presence", EdgeType.NEGATION),
        Edge("derrida_pharmakon", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_pharmakon", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_dissemination", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_dissemination", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_dissemination", "derrida_signifier_signified", EdgeType.NEGATION),
        Edge("derrida_white_mythology", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_white_mythology", "derrida_catachresis", EdgeType.DEPENDENCY),
        Edge("derrida_iterability", "derrida_sign", EdgeType.DEPENDENCY),
        Edge("derrida_iterability", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_iterability", "derrida_iterability_early", EdgeType.DEPENDENCY),
        Edge("derrida_signature", "derrida_iterability", EdgeType.DEPENDENCY),
        Edge("derrida_signature", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_event", "derrida_iterability", EdgeType.NEGATION),
        Edge("derrida_event", "derrida_context", EdgeType.NEGATION),
        Edge("derrida_context", "derrida_iterability", EdgeType.DEPENDENCY),
        Edge("derrida_austin_critique", "derrida_iterability", EdgeType.DEPENDENCY),
        Edge("derrida_austin_critique", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_tympan", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_tympan", "derrida_differance", EdgeType.REFERENCE),
        Edge("derrida_hegel_aufhebung_critique", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_hegel_aufhebung_critique", "derrida_logocentrism", EdgeType.NEGATION),
        Edge("derrida_catachresis", "derrida_white_mythology", EdgeType.DEPENDENCY),
        Edge("derrida_catachresis", "derrida_signifier_signified", EdgeType.NEGATION),
    ]
    return "1972: Margins of Philosophy", vertices, edges


def work_specters_of_marx():
    """1993: Specters of Marx (Spectres de Marx).

    Derrida's engagement with Marx after the fall of communism.
    Introduces hauntology (ontology haunted by specters), the messianic
    without messianism, and a new thinking of justice as irreducible
    to law. A turn toward ethico-political responsibility.
    """
    vertices = [
        Vertex("derrida_hauntology", content="hauntology — being haunted by what is neither present nor absent"),
        Vertex("derrida_specter", content="specter — revenant that haunts, neither living nor dead"),
        Vertex("derrida_justice", content="justice — irreducible to law, always to-come"),
        Vertex("derrida_messianic", content="messianic without messianism — structural openness to the to-come"),
        Vertex("derrida_inheritance", content="inheritance — we are heirs, must choose what to inherit"),
        Vertex("derrida_conjuration", content="conjuration — attempt to exorcise the specter, doomed"),
        Vertex("derrida_new_international", content="new International — alliance beyond nation-state"),
        Vertex("derrida_end_of_history_critique", content="critique of end of history — Fukuyama, triumphalism"),
        Vertex("derrida_visor_effect", content="visor effect — the specter sees us, we cannot see it"),
        Vertex("derrida_to_come", content="to-come (a-venir) — the future that is not programmed"),
        Vertex("derrida_mourning", content="mourning — work of mourning, impossible mourning"),
        Vertex("derrida_gift", content="gift — impossible gift, beyond economy of exchange"),
    ]
    edges = [
        Edge("derrida_hauntology", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_hauntology", "derrida_trace", EdgeType.DEPENDENCY),
        Edge("derrida_hauntology", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_specter", "derrida_hauntology", EdgeType.DEPENDENCY),
        Edge("derrida_specter", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_justice", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_justice", "derrida_to_come", EdgeType.DEPENDENCY),
        Edge("derrida_messianic", "derrida_to_come", EdgeType.DEPENDENCY),
        Edge("derrida_messianic", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_messianic", "derrida_justice", EdgeType.DEPENDENCY),
        Edge("derrida_inheritance", "derrida_specter", EdgeType.DEPENDENCY),
        Edge("derrida_inheritance", "derrida_iterability", EdgeType.REFERENCE),
        Edge("derrida_conjuration", "derrida_specter", EdgeType.NEGATION),
        Edge("derrida_conjuration", "derrida_presence", EdgeType.DEPENDENCY),
        Edge("derrida_new_international", "derrida_justice", EdgeType.DEPENDENCY),
        Edge("derrida_end_of_history_critique", "derrida_hauntology", EdgeType.DEPENDENCY),
        Edge("derrida_end_of_history_critique", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_visor_effect", "derrida_specter", EdgeType.DEPENDENCY),
        Edge("derrida_visor_effect", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_to_come", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_to_come", "derrida_presence", EdgeType.NEGATION),
        Edge("derrida_mourning", "derrida_specter", EdgeType.DEPENDENCY),
        Edge("derrida_mourning", "derrida_absence", EdgeType.DEPENDENCY),
        Edge("derrida_gift", "derrida_differance", EdgeType.DEPENDENCY),
        Edge("derrida_gift", "derrida_presence", EdgeType.NEGATION),
    ]
    return "1993: Specters of Marx", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_speech_and_phenomena,
    work_of_grammatology,
    work_writing_and_difference,
    work_margins_of_philosophy,
    work_specters_of_marx,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("DERRIDA COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
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

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates, skip if endpoints missing)
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

        # 3. Create or update engine
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

        # 4. Digest: run until beta_1 stable for 50 consecutive steps
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

    # ---------------------------------------------------------------------------
    # Final summary
    # ---------------------------------------------------------------------------
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

    deltas = [(r["work"], r["delta_beta_1"], r["work_index"])
              for r in work_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by work:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    W{idx:2d} {name}: +{delta}")

    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1

    type_counts = {}
    for e in engine.k_active.active_edges():
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1

    neg_count = type_counts.get("negation", 0)
    neg_density = neg_count / final_e if final_e > 0 else 0

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "derrida_full_incremental",
        "source": "Derrida — 5 major works (1967-1993)",
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

    output_path = "experiment_derrida.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
