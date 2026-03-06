"""Theodor W. Adorno — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Adorno's thought —
from early aesthetics through the dialectic of enlightenment to
negative dialectics — as an incremental topological growth process.
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
#   negative_dialectics, non_identity, identity_thinking,
#   culture_industry, enlightenment, domination, mimesis_adorno,
#   constellation_adorno, damaged_life, autonomy, commodity_fetishism,
#   reification, totality, particular, universal, mediation,
#   natural_history, myth_enlightenment, reason, instrumental_reason,
#   suffering, reconciliation, semblance, truth_content,
#   form, content_adorno, material, expression_adorno,
#   atonality, dissonance, new_music, twelve_tone
# ---------------------------------------------------------------------------


def work_kierkegaard():
    """1933: Kierkegaard: Construction of the Aesthetic."""
    vertices = [
        Vertex("interiority", content="interiority — bourgeois retreat"),
        Vertex("semblance", content="semblance (Schein) — aesthetic illusion"),
        Vertex("melancholy_adorno", content="melancholy — Kierkegaard's situation"),
        Vertex("construction", content="construction — philosophical method"),
        Vertex("natural_history", content="natural history (Naturgeschichte)"),
        Vertex("mythical_nature", content="mythical nature — in bourgeois interiority"),
    ]
    edges = [
        Edge("interiority", "semblance", EdgeType.DEPENDENCY),
        Edge("interiority", "mythical_nature", EdgeType.DEPENDENCY),
        Edge("semblance", "construction", EdgeType.DEPENDENCY),
        Edge("melancholy_adorno", "interiority", EdgeType.DEPENDENCY),
        Edge("natural_history", "mythical_nature", EdgeType.NEGATION),
        Edge("natural_history", "construction", EdgeType.DEPENDENCY),
        Edge("mythical_nature", "interiority", EdgeType.DEPENDENCY),
    ]
    return "1933: Kierkegaard: Construction of the Aesthetic", vertices, edges


def work_dialectic_of_enlightenment():
    """1944/1947: Dialectic of Enlightenment (with Horkheimer)."""
    vertices = [
        Vertex("enlightenment", content="enlightenment — reverts to mythology"),
        Vertex("myth_enlightenment", content="myth is already enlightenment"),
        Vertex("domination", content="domination — of nature, then of humans"),
        Vertex("instrumental_reason", content="instrumental reason — self-preserving"),
        Vertex("culture_industry", content="culture industry — mass deception"),
        Vertex("identity_thinking", content="identity thinking — subsumption under concept"),
        Vertex("mimesis_adorno", content="mimesis — suppressed by enlightenment"),
        Vertex("odysseus", content="Odysseus — proto-bourgeois cunning"),
        Vertex("sacrifice", content="sacrifice — exchange with mythic powers"),
        Vertex("self_preservation", content="self-preservation — becomes self-destruction"),
        Vertex("reason", content="reason — entangled with unreason"),
        Vertex("antisemitism", content="antisemitism — return of suppressed mimesis"),
    ]
    edges = [
        Edge("enlightenment", "myth_enlightenment", EdgeType.NEGATION),
        Edge("enlightenment", "domination", EdgeType.DEPENDENCY),
        Edge("enlightenment", "instrumental_reason", EdgeType.DEPENDENCY),
        Edge("myth_enlightenment", "enlightenment", EdgeType.NEGATION),
        Edge("domination", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("domination", "mimesis_adorno", EdgeType.NEGATION),
        Edge("instrumental_reason", "self_preservation", EdgeType.DEPENDENCY),
        Edge("instrumental_reason", "reason", EdgeType.NEGATION),
        Edge("culture_industry", "enlightenment", EdgeType.DEPENDENCY),
        Edge("culture_industry", "domination", EdgeType.DEPENDENCY),
        Edge("identity_thinking", "mimesis_adorno", EdgeType.NEGATION),
        Edge("odysseus", "self_preservation", EdgeType.DEPENDENCY),
        Edge("odysseus", "sacrifice", EdgeType.NEGATION),
        Edge("sacrifice", "domination", EdgeType.DEPENDENCY),
        Edge("self_preservation", "domination", EdgeType.DEPENDENCY),
        Edge("reason", "instrumental_reason", EdgeType.NEGATION),
        Edge("antisemitism", "mimesis_adorno", EdgeType.DEPENDENCY),
        Edge("antisemitism", "identity_thinking", EdgeType.DEPENDENCY),
    ]
    return "1944/1947: Dialectic of Enlightenment", vertices, edges


def work_philosophy_of_new_music():
    """1949: Philosophy of New Music."""
    vertices = [
        Vertex("new_music", content="new music — Schoenberg's emancipation of dissonance"),
        Vertex("twelve_tone", content="twelve-tone technique — total rationalization"),
        Vertex("dissonance", content="dissonance — truth of expression"),
        Vertex("atonality", content="atonality — liberation from tonality"),
        Vertex("stravinsky_critique", content="Stravinsky — regression to archaic"),
        Vertex("expression_adorno", content="expression — subjective against convention"),
        Vertex("material", content="musical material — historically sedimented"),
        Vertex("form", content="form — dialectic with content"),
        Vertex("content_adorno", content="content — concrete, not imposed"),
    ]
    edges = [
        Edge("new_music", "atonality", EdgeType.DEPENDENCY),
        Edge("new_music", "dissonance", EdgeType.DEPENDENCY),
        Edge("twelve_tone", "atonality", EdgeType.SUBLATION),
        Edge("twelve_tone", "domination", EdgeType.DEPENDENCY),
        Edge("dissonance", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("atonality", "material", EdgeType.DEPENDENCY),
        Edge("stravinsky_critique", "mimesis_adorno", EdgeType.NEGATION),
        Edge("stravinsky_critique", "myth_enlightenment", EdgeType.REFERENCE),
        Edge("expression_adorno", "mimesis_adorno", EdgeType.REFERENCE),
        Edge("expression_adorno", "identity_thinking", EdgeType.NEGATION),
        Edge("material", "natural_history", EdgeType.REFERENCE),
        Edge("form", "content_adorno", EdgeType.NEGATION),
        Edge("form", "material", EdgeType.DEPENDENCY),
        Edge("content_adorno", "expression_adorno", EdgeType.DEPENDENCY),
    ]
    return "1949: Philosophy of New Music", vertices, edges


def work_minima_moralia():
    """1951: Minima Moralia: Reflections from Damaged Life."""
    vertices = [
        Vertex("damaged_life", content="damaged life — wrong life cannot be lived rightly"),
        Vertex("totality", content="totality — the false"),
        Vertex("particular", content="particular — refuge of truth"),
        Vertex("individual", content="individual — liquidated by late capitalism"),
        Vertex("aphorism", content="aphorism — form of damaged thought"),
        Vertex("love", content="love — promise of happiness"),
        Vertex("gift", content="gift — impossible in exchange society"),
        Vertex("dwelling", content="dwelling — impossible after Auschwitz"),
        Vertex("utopia", content="utopia — standpoint of redemption"),
    ]
    edges = [
        Edge("damaged_life", "totality", EdgeType.DEPENDENCY),
        Edge("damaged_life", "individual", EdgeType.DEPENDENCY),
        Edge("totality", "particular", EdgeType.NEGATION),
        Edge("totality", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("particular", "totality", EdgeType.NEGATION),
        Edge("individual", "damaged_life", EdgeType.DEPENDENCY),
        Edge("individual", "culture_industry", EdgeType.DEPENDENCY),
        Edge("aphorism", "particular", EdgeType.DEPENDENCY),
        Edge("aphorism", "totality", EdgeType.NEGATION),
        Edge("love", "damaged_life", EdgeType.NEGATION),
        Edge("love", "utopia", EdgeType.REFERENCE),
        Edge("gift", "domination", EdgeType.NEGATION),
        Edge("gift", "love", EdgeType.DEPENDENCY),
        Edge("dwelling", "damaged_life", EdgeType.DEPENDENCY),
        Edge("utopia", "damaged_life", EdgeType.NEGATION),
        Edge("utopia", "particular", EdgeType.DEPENDENCY),
    ]
    return "1951: Minima Moralia", vertices, edges


def work_authoritarian_personality():
    """1950: The Authoritarian Personality (with collaborators)."""
    vertices = [
        Vertex("f_scale", content="F-scale — measuring fascist potential"),
        Vertex("authoritarian_character", content="authoritarian character — conventional, submissive, aggressive"),
        Vertex("prejudice", content="prejudice — structured by psyche"),
        Vertex("projection", content="projection — paranoid, onto outgroup"),
    ]
    edges = [
        Edge("f_scale", "authoritarian_character", EdgeType.DEPENDENCY),
        Edge("authoritarian_character", "domination", EdgeType.DEPENDENCY),
        Edge("authoritarian_character", "identity_thinking", EdgeType.REFERENCE),
        Edge("prejudice", "authoritarian_character", EdgeType.DEPENDENCY),
        Edge("prejudice", "antisemitism", EdgeType.REFERENCE),
        Edge("projection", "mimesis_adorno", EdgeType.NEGATION),
        Edge("projection", "prejudice", EdgeType.DEPENDENCY),
    ]
    return "1950: The Authoritarian Personality", vertices, edges


def work_prisms():
    """1955: Prisms (collected essays)."""
    vertices = [
        Vertex("cultural_criticism", content="cultural criticism — and society"),
        Vertex("immanent_critique", content="immanent critique — vs transcendent critique"),
        Vertex("poetry_after_auschwitz", content="poetry after Auschwitz — barbaric"),
        Vertex("reification", content="reification — second nature"),
    ]
    edges = [
        Edge("cultural_criticism", "culture_industry", EdgeType.DEPENDENCY),
        Edge("cultural_criticism", "immanent_critique", EdgeType.DEPENDENCY),
        Edge("immanent_critique", "totality", EdgeType.DEPENDENCY),
        Edge("immanent_critique", "particular", EdgeType.DEPENDENCY),
        Edge("poetry_after_auschwitz", "damaged_life", EdgeType.DEPENDENCY),
        Edge("poetry_after_auschwitz", "expression_adorno", EdgeType.NEGATION),
        Edge("reification", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("reification", "natural_history", EdgeType.REFERENCE),
    ]
    return "1955: Prisms", vertices, edges


def work_against_epistemology():
    """1956: Against Epistemology: A Metacritique (Metacritique of Husserl)."""
    vertices = [
        Vertex("metacritique", content="metacritique — of epistemology"),
        Vertex("prima_philosophia", content="prima philosophia — impossible foundationalism"),
        Vertex("givenness", content="givenness — myth of the given"),
        Vertex("mediation", content="mediation — nothing immediate"),
    ]
    edges = [
        Edge("metacritique", "identity_thinking", EdgeType.NEGATION),
        Edge("metacritique", "construction", EdgeType.REFERENCE),
        Edge("prima_philosophia", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("prima_philosophia", "metacritique", EdgeType.NEGATION),
        Edge("givenness", "prima_philosophia", EdgeType.DEPENDENCY),
        Edge("givenness", "mediation", EdgeType.NEGATION),
        Edge("mediation", "givenness", EdgeType.NEGATION),
        Edge("mediation", "totality", EdgeType.DEPENDENCY),
    ]
    return "1956: Against Epistemology", vertices, edges


def work_jargon_of_authenticity():
    """1964: Jargon of Authenticity (Jargon der Eigentlichkeit)."""
    vertices = [
        Vertex("jargon", content="jargon — ideological pseudo-concreteness"),
        Vertex("authenticity_critique", content="critique of authenticity — Heidegger"),
        Vertex("concreteness_false", content="false concreteness — ontologizing the ontic"),
    ]
    edges = [
        Edge("jargon", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("jargon", "culture_industry", EdgeType.REFERENCE),
        Edge("authenticity_critique", "jargon", EdgeType.DEPENDENCY),
        Edge("authenticity_critique", "reification", EdgeType.REFERENCE),
        Edge("concreteness_false", "jargon", EdgeType.DEPENDENCY),
        Edge("concreteness_false", "mediation", EdgeType.NEGATION),
    ]
    return "1964: Jargon of Authenticity", vertices, edges


def work_notes_to_literature():
    """1958-1974: Notes to Literature (Noten zur Literatur)."""
    vertices = [
        Vertex("essay_form", content="essay as form — anti-systematic"),
        Vertex("lyric_and_society", content="lyric and society — social content in form"),
        Vertex("committed_art", content="committed vs autonomous art"),
        Vertex("parataxis", content="parataxis — Holderlin's late style"),
    ]
    edges = [
        Edge("essay_form", "aphorism", EdgeType.REFERENCE),
        Edge("essay_form", "particular", EdgeType.DEPENDENCY),
        Edge("essay_form", "totality", EdgeType.NEGATION),
        Edge("lyric_and_society", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("lyric_and_society", "form", EdgeType.DEPENDENCY),
        Edge("committed_art", "culture_industry", EdgeType.NEGATION),
        Edge("committed_art", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("parataxis", "dissonance", EdgeType.REFERENCE),
        Edge("parataxis", "identity_thinking", EdgeType.NEGATION),
    ]
    return "1958-1974: Notes to Literature", vertices, edges


def work_introduction_sociology_of_music():
    """1962: Introduction to the Sociology of Music."""
    vertices = [
        Vertex("adequate_listening", content="adequate listening — structural hearing"),
        Vertex("regressive_listening", content="regressive listening — fetishistic"),
        Vertex("music_commodity", content="music as commodity"),
        Vertex("musical_types", content="typology of musical behavior"),
    ]
    edges = [
        Edge("adequate_listening", "new_music", EdgeType.DEPENDENCY),
        Edge("adequate_listening", "regressive_listening", EdgeType.NEGATION),
        Edge("regressive_listening", "culture_industry", EdgeType.DEPENDENCY),
        Edge("regressive_listening", "mimesis_adorno", EdgeType.NEGATION),
        Edge("music_commodity", "culture_industry", EdgeType.DEPENDENCY),
        Edge("music_commodity", "reification", EdgeType.DEPENDENCY),
        Edge("musical_types", "adequate_listening", EdgeType.DEPENDENCY),
        Edge("musical_types", "regressive_listening", EdgeType.DEPENDENCY),
    ]
    return "1962: Introduction to the Sociology of Music", vertices, edges


def work_negative_dialectics():
    """1966: Negative Dialectics (Negative Dialektik)."""
    vertices = [
        Vertex("negative_dialectics", content="negative dialectics — without synthesis"),
        Vertex("non_identity", content="non-identity — preponderance of the object"),
        Vertex("constellation_adorno", content="constellation — non-violent concept"),
        Vertex("preponderance_object", content="preponderance of the object (Vorrang des Objekts)"),
        Vertex("suffering", content="suffering — somatic moment of knowledge"),
        Vertex("reconciliation", content="reconciliation — utopian, not actual"),
        Vertex("concept", content="concept — necessary but insufficient"),
        Vertex("universal", content="universal — coercive generality"),
        Vertex("freedom_adorno", content="freedom — possible only in unfreedom"),
        Vertex("metaphysics_after_auschwitz", content="metaphysics after Auschwitz"),
    ]
    edges = [
        Edge("negative_dialectics", "identity_thinking", EdgeType.NEGATION),
        Edge("negative_dialectics", "non_identity", EdgeType.DEPENDENCY),
        Edge("negative_dialectics", "constellation_adorno", EdgeType.DEPENDENCY),
        Edge("non_identity", "identity_thinking", EdgeType.NEGATION),
        Edge("non_identity", "preponderance_object", EdgeType.DEPENDENCY),
        Edge("constellation_adorno", "concept", EdgeType.SUBLATION),
        Edge("constellation_adorno", "particular", EdgeType.DEPENDENCY),
        Edge("preponderance_object", "mediation", EdgeType.DEPENDENCY),
        Edge("preponderance_object", "concept", EdgeType.NEGATION),
        Edge("suffering", "non_identity", EdgeType.DEPENDENCY),
        Edge("suffering", "damaged_life", EdgeType.DEPENDENCY),
        Edge("reconciliation", "non_identity", EdgeType.DEPENDENCY),
        Edge("reconciliation", "utopia", EdgeType.DEPENDENCY),
        Edge("concept", "identity_thinking", EdgeType.DEPENDENCY),
        Edge("concept", "universal", EdgeType.DEPENDENCY),
        Edge("universal", "particular", EdgeType.NEGATION),
        Edge("freedom_adorno", "domination", EdgeType.NEGATION),
        Edge("freedom_adorno", "reconciliation", EdgeType.REFERENCE),
        Edge("metaphysics_after_auschwitz", "poetry_after_auschwitz", EdgeType.SUBLATION),
        Edge("metaphysics_after_auschwitz", "suffering", EdgeType.DEPENDENCY),
    ]
    return "1966: Negative Dialectics", vertices, edges


def work_aesthetic_theory():
    """1970 (posthumous): Aesthetic Theory (Asthetische Theorie)."""
    vertices = [
        Vertex("autonomy", content="autonomy of art — fait social"),
        Vertex("truth_content", content="truth content (Wahrheitsgehalt) — non-intentional"),
        Vertex("enigma_character", content="enigma character (Ratselcharakter) of art"),
        Vertex("natural_beauty", content="natural beauty — non-identical in nature"),
        Vertex("sublime_adorno", content="sublime — shudder before overwhelming"),
        Vertex("modernism", content="modernism — immanent critique of tradition"),
        Vertex("late_style", content="late style — disintegration, not serenity"),
        Vertex("art_and_society", content="art and society — mediated autonomy"),
    ]
    edges = [
        Edge("autonomy", "culture_industry", EdgeType.NEGATION),
        Edge("autonomy", "art_and_society", EdgeType.DEPENDENCY),
        Edge("truth_content", "autonomy", EdgeType.DEPENDENCY),
        Edge("truth_content", "non_identity", EdgeType.DEPENDENCY),
        Edge("truth_content", "semblance", EdgeType.NEGATION),
        Edge("enigma_character", "truth_content", EdgeType.DEPENDENCY),
        Edge("enigma_character", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("natural_beauty", "non_identity", EdgeType.DEPENDENCY),
        Edge("natural_beauty", "mimesis_adorno", EdgeType.DEPENDENCY),
        Edge("sublime_adorno", "natural_beauty", EdgeType.SUBLATION),
        Edge("sublime_adorno", "suffering", EdgeType.REFERENCE),
        Edge("modernism", "dissonance", EdgeType.DEPENDENCY),
        Edge("modernism", "material", EdgeType.DEPENDENCY),
        Edge("modernism", "autonomy", EdgeType.DEPENDENCY),
        Edge("late_style", "form", EdgeType.NEGATION),
        Edge("late_style", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("art_and_society", "mediation", EdgeType.DEPENDENCY),
        Edge("art_and_society", "totality", EdgeType.DEPENDENCY),
    ]
    return "1970 (posth.): Aesthetic Theory", vertices, edges


def work_critical_models():
    """1963-1969: Critical Models (Stichworte / Eingriffe)."""
    vertices = [
        Vertex("education_after_auschwitz", content="education after Auschwitz — never again"),
        Vertex("resignation_critique", content="critique of resignation — thinking is doing"),
        Vertex("opinion_delusion", content="opinion, delusion, society"),
        Vertex("theory_and_practice", content="theory and practice — not identical"),
    ]
    edges = [
        Edge("education_after_auschwitz", "authoritarian_character", EdgeType.DEPENDENCY),
        Edge("education_after_auschwitz", "poetry_after_auschwitz", EdgeType.REFERENCE),
        Edge("education_after_auschwitz", "enlightenment", EdgeType.DEPENDENCY),
        Edge("resignation_critique", "negative_dialectics", EdgeType.DEPENDENCY),
        Edge("resignation_critique", "damaged_life", EdgeType.NEGATION),
        Edge("opinion_delusion", "culture_industry", EdgeType.DEPENDENCY),
        Edge("opinion_delusion", "identity_thinking", EdgeType.REFERENCE),
        Edge("theory_and_practice", "negative_dialectics", EdgeType.DEPENDENCY),
        Edge("theory_and_practice", "reconciliation", EdgeType.REFERENCE),
    ]
    return "1963-1969: Critical Models", vertices, edges


def work_positivism_dispute():
    """1961/1969: The Positivist Dispute in German Sociology."""
    vertices = [
        Vertex("positivism_critique", content="critique of positivism — value-free illusion"),
        Vertex("totalitarian_positivism", content="positivism — latent totalitarianism"),
        Vertex("immanent_analysis", content="immanent analysis — society not sum of facts"),
    ]
    edges = [
        Edge("positivism_critique", "identity_thinking", EdgeType.NEGATION),
        Edge("positivism_critique", "instrumental_reason", EdgeType.DEPENDENCY),
        Edge("totalitarian_positivism", "positivism_critique", EdgeType.DEPENDENCY),
        Edge("totalitarian_positivism", "domination", EdgeType.REFERENCE),
        Edge("immanent_analysis", "totality", EdgeType.DEPENDENCY),
        Edge("immanent_analysis", "mediation", EdgeType.DEPENDENCY),
    ]
    return "1961/1969: The Positivist Dispute", vertices, edges


def work_mahler():
    """1960: Mahler: A Musical Physiognomy."""
    vertices = [
        Vertex("mahler_breakthrough", content="Mahler — breakthrough, not breakdown"),
        Vertex("musical_physiognomy", content="musical physiognomy — reading expression"),
        Vertex("banality", content="banality — truth in vulgar material"),
        Vertex("variant_technique", content="variant technique — developing variation"),
    ]
    edges = [
        Edge("mahler_breakthrough", "new_music", EdgeType.REFERENCE),
        Edge("mahler_breakthrough", "dissonance", EdgeType.DEPENDENCY),
        Edge("musical_physiognomy", "expression_adorno", EdgeType.DEPENDENCY),
        Edge("musical_physiognomy", "mimesis_adorno", EdgeType.DEPENDENCY),
        Edge("banality", "culture_industry", EdgeType.NEGATION),
        Edge("banality", "material", EdgeType.DEPENDENCY),
        Edge("variant_technique", "twelve_tone", EdgeType.REFERENCE),
        Edge("variant_technique", "form", EdgeType.DEPENDENCY),
    ]
    return "1960: Mahler: A Musical Physiognomy", vertices, edges


# ---------------------------------------------------------------------------
# Full works list in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_kierkegaard,                        # 1933
    work_dialectic_of_enlightenment,         # 1944/1947
    work_philosophy_of_new_music,            # 1949
    work_authoritarian_personality,          # 1950
    work_minima_moralia,                     # 1951
    work_prisms,                             # 1955
    work_against_epistemology,               # 1956
    work_notes_to_literature,                # 1958-1974
    work_mahler,                             # 1960
    work_positivism_dispute,                 # 1961/1969
    work_introduction_sociology_of_music,    # 1962
    work_jargon_of_authenticity,             # 1964
    work_negative_dialectics,                # 1966
    work_critical_models,                    # 1963-1969
    work_aesthetic_theory,                   # 1970
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("THEODOR W. ADORNO — COMPLETE WORKS INCREMENTAL TRAVERSAL")
    print(f"{total_works} works, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    work_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for w_idx, w_fn in enumerate(ALL_WORKS):
        work_name, w_vertices, w_edges = w_fn()
        print(f"\n{'─' * 60}")
        print(f"Work {w_idx + 1}/{total_works}: {work_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
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
        print(f"  Complex now: {len(graph.active_vertex_ids())} V, {len(graph.active_edges())} E")
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

        # Count operations in this work's digestion
        work_logs = engine.logs[-steps_this_work:] if steps_this_work > 0 else []
        op_counts = {}
        for log in work_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record
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

        # Update graph reference for next work injection
        graph = engine.k_active

    # ---------------------------------------------------------------------------
    # Final summary
    # ---------------------------------------------------------------------------
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

    # Beta_1 growth curve
    print(f"\n  beta_1 growth curve (by work):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    W{entry['work_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['work_name']}")

    # Biggest jumps
    deltas = [(r["work"], r["delta_beta_1"], r["work_index"]) for r in work_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by work:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    W{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by work:")
    for r in work_results:
        print(f"    W{r['work_index']:2d} {r['work']}: "
              f"{r['steps_to_digest']} steps {'OK' if r['converged'] else 'MAX'}")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({final_settled}):")
        for i, sc in enumerate(engine.settlement.settled_cycles[:20]):
            print(f"    [{i+1}] settled@step={sc.settled_at_step}: "
                  f"{sorted(sc.edges)[:3]}...")

    # Final terrain
    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = (ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"])
                  if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0)
    print(f"\n  Final terrain: {ft_counts}")
    print(f"  Critical edge ratio: {crit_ratio:.4f}")

    # Edge type distribution
    type_counts = {}
    for e in engine.k_active.active_edges():
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    print(f"\n  Edge type distribution:")
    for et, count in sorted(type_counts.items()):
        print(f"    {et}: {count}")

    # Negation density
    neg_count = type_counts.get("negation", 0)
    neg_density = neg_count / final_e if final_e > 0 else 0
    print(f"\n  Negation density: {neg_density:.4f} ({neg_count}/{final_e})")

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "adorno_complete_works_incremental",
        "source": "Theodor W. Adorno — complete works, 15 entries (1933-1970)",
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
            "critical_edge_ratio": crit_ratio,
            "negation_density": neg_density,
        },
        "total_steps": cumulative_steps,
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_adorno.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
