"""Merleau-Ponty — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Merleau-Ponty's
phenomenological thought — from the structure of behavior through
the chiasmic ontology of flesh — as an incremental topological
growth process.
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
#   body, perception, consciousness, behavior, structure, form,
#   expression, meaning, sense, world, subject, object,
#   flesh, chiasm, visible, invisible, reversibility,
#   motility, habit, schema_corporel, phenomenal_field,
#   intersubjectivity, institution, passivity, nature,
#   language, speech, silence, painting, style,
#   freedom, situation, temporality, existence, ambiguity
# ---------------------------------------------------------------------------


def work_structure_of_behavior():
    """1942: The Structure of Behavior (La structure du comportement)."""
    vertices = [
        Vertex("behavior", content="behavior as form/structure"),
        Vertex("structure", content="structure (Gestalt)"),
        Vertex("form", content="form — organizing principle"),
        Vertex("reflex", content="reflex arc"),
        Vertex("stimulus_response", content="stimulus-response model"),
        Vertex("consciousness", content="consciousness"),
        Vertex("organism", content="organism as totality"),
        Vertex("physical_order", content="physical order"),
        Vertex("vital_order", content="vital order"),
        Vertex("human_order", content="human order"),
        Vertex("integration", content="integration of orders"),
        Vertex("dialectic_nature_consciousness", content="dialectic of nature and consciousness"),
    ]
    edges = [
        Edge("behavior", "structure", EdgeType.DEPENDENCY),
        Edge("behavior", "form", EdgeType.DEPENDENCY),
        Edge("structure", "reflex", EdgeType.NEGATION),
        Edge("structure", "stimulus_response", EdgeType.NEGATION),
        Edge("organism", "structure", EdgeType.DEPENDENCY),
        Edge("organism", "form", EdgeType.DEPENDENCY),
        Edge("physical_order", "vital_order", EdgeType.DEPENDENCY),
        Edge("vital_order", "human_order", EdgeType.DEPENDENCY),
        Edge("human_order", "consciousness", EdgeType.DEPENDENCY),
        Edge("integration", "physical_order", EdgeType.DEPENDENCY),
        Edge("integration", "vital_order", EdgeType.DEPENDENCY),
        Edge("integration", "human_order", EdgeType.DEPENDENCY),
        Edge("dialectic_nature_consciousness", "consciousness", EdgeType.DEPENDENCY),
        Edge("dialectic_nature_consciousness", "organism", EdgeType.NEGATION),
    ]
    return "1942: The Structure of Behavior", vertices, edges


def work_phenomenology_of_perception():
    """1945: Phenomenology of Perception (Phenomenologie de la perception)."""
    vertices = [
        Vertex("perception", content="perception — primordial contact with the world"),
        Vertex("body", content="lived body (corps propre / Leib)"),
        Vertex("phenomenal_field", content="phenomenal field"),
        Vertex("motility", content="motility — bodily movement as intentional"),
        Vertex("habit", content="habit — motor significance"),
        Vertex("schema_corporel", content="body schema (schema corporel)"),
        Vertex("sensation", content="sensation — not atomic impression"),
        Vertex("subject", content="embodied subject"),
        Vertex("world", content="world — not objective sum"),
        Vertex("intentionality", content="operative intentionality (fungierende Intentionalitat)"),
        Vertex("spatiality", content="bodily spatiality — situation not position"),
        Vertex("temporality", content="temporality — living present"),
        Vertex("freedom", content="situated freedom"),
        Vertex("existence", content="existence — being-in-the-world"),
        Vertex("ambiguity", content="ambiguity — constitutive, not defect"),
        Vertex("intersubjectivity", content="intersubjectivity — intercorporeality"),
        Vertex("sexuality", content="sexuality as existential dimension"),
        Vertex("speech", content="speech — bodily gesture of meaning"),
    ]
    edges = [
        Edge("perception", "body", EdgeType.DEPENDENCY),
        Edge("perception", "phenomenal_field", EdgeType.DEPENDENCY),
        Edge("perception", "sensation", EdgeType.NEGATION),
        Edge("body", "schema_corporel", EdgeType.DEPENDENCY),
        Edge("body", "motility", EdgeType.DEPENDENCY),
        Edge("body", "habit", EdgeType.DEPENDENCY),
        Edge("body", "consciousness", EdgeType.NEGATION),
        Edge("subject", "body", EdgeType.DEPENDENCY),
        Edge("subject", "world", EdgeType.DEPENDENCY),
        Edge("phenomenal_field", "world", EdgeType.DEPENDENCY),
        Edge("phenomenal_field", "structure", EdgeType.REFERENCE),
        Edge("intentionality", "perception", EdgeType.DEPENDENCY),
        Edge("intentionality", "motility", EdgeType.DEPENDENCY),
        Edge("spatiality", "body", EdgeType.DEPENDENCY),
        Edge("spatiality", "schema_corporel", EdgeType.DEPENDENCY),
        Edge("temporality", "perception", EdgeType.DEPENDENCY),
        Edge("temporality", "existence", EdgeType.DEPENDENCY),
        Edge("existence", "subject", EdgeType.DEPENDENCY),
        Edge("existence", "world", EdgeType.DEPENDENCY),
        Edge("freedom", "existence", EdgeType.DEPENDENCY),
        Edge("freedom", "ambiguity", EdgeType.DEPENDENCY),
        Edge("ambiguity", "perception", EdgeType.DEPENDENCY),
        Edge("ambiguity", "existence", EdgeType.DEPENDENCY),
        Edge("intersubjectivity", "body", EdgeType.DEPENDENCY),
        Edge("intersubjectivity", "perception", EdgeType.REFERENCE),
        Edge("sexuality", "body", EdgeType.DEPENDENCY),
        Edge("sexuality", "existence", EdgeType.REFERENCE),
        Edge("speech", "body", EdgeType.DEPENDENCY),
        Edge("speech", "motility", EdgeType.REFERENCE),
    ]
    return "1945: Phenomenology of Perception", vertices, edges


def work_humanism_and_terror():
    """1947: Humanism and Terror (Humanisme et terreur)."""
    vertices = [
        Vertex("violence", content="violence — inescapable in history"),
        Vertex("history", content="history — contingent, not teleological"),
        Vertex("humanism", content="humanism — not abstract ideal"),
        Vertex("terror", content="terror — revolutionary violence"),
        Vertex("proletariat", content="proletariat — historical subject"),
        Vertex("contingency", content="contingency of historical outcome"),
        Vertex("political_action", content="political action — risk and ambiguity"),
        Vertex("marxism_open", content="Marxism as open theory"),
    ]
    edges = [
        Edge("violence", "history", EdgeType.DEPENDENCY),
        Edge("violence", "political_action", EdgeType.DEPENDENCY),
        Edge("humanism", "terror", EdgeType.NEGATION),
        Edge("humanism", "freedom", EdgeType.REFERENCE),
        Edge("terror", "violence", EdgeType.DEPENDENCY),
        Edge("proletariat", "history", EdgeType.DEPENDENCY),
        Edge("contingency", "history", EdgeType.DEPENDENCY),
        Edge("contingency", "ambiguity", EdgeType.REFERENCE),
        Edge("political_action", "existence", EdgeType.REFERENCE),
        Edge("political_action", "contingency", EdgeType.DEPENDENCY),
        Edge("marxism_open", "proletariat", EdgeType.DEPENDENCY),
        Edge("marxism_open", "contingency", EdgeType.DEPENDENCY),
    ]
    return "1947: Humanism and Terror", vertices, edges


def work_sense_and_nonsense():
    """1948: Sense and Non-Sense (Sens et non-sens)."""
    vertices = [
        Vertex("sense", content="sense — embodied meaning"),
        Vertex("nonsense", content="non-sense — not absence but other of sense"),
        Vertex("painting", content="painting — Cezanne's doubt"),
        Vertex("style", content="style — bodily expression"),
        Vertex("engagement", content="engagement — situated thought"),
        Vertex("metaphysics_concrete", content="metaphysics — concrete, not abstract"),
    ]
    edges = [
        Edge("sense", "perception", EdgeType.DEPENDENCY),
        Edge("sense", "body", EdgeType.DEPENDENCY),
        Edge("nonsense", "sense", EdgeType.NEGATION),
        Edge("nonsense", "ambiguity", EdgeType.REFERENCE),
        Edge("painting", "perception", EdgeType.DEPENDENCY),
        Edge("painting", "style", EdgeType.DEPENDENCY),
        Edge("style", "body", EdgeType.DEPENDENCY),
        Edge("style", "expression_proto", EdgeType.DEPENDENCY),
        Edge("engagement", "existence", EdgeType.DEPENDENCY),
        Edge("engagement", "political_action", EdgeType.REFERENCE),
        Edge("metaphysics_concrete", "perception", EdgeType.DEPENDENCY),
        Edge("metaphysics_concrete", "existence", EdgeType.REFERENCE),
    ]
    # Need the expression_proto vertex
    vertices.append(Vertex("expression_proto", content="expression (proto — bodily gesture)"))
    return "1948: Sense and Non-Sense", vertices, edges


def work_adventures_of_dialectic():
    """1955: Adventures of the Dialectic (Les aventures de la dialectique)."""
    vertices = [
        Vertex("dialectic", content="dialectic — lived, not mechanical"),
        Vertex("weber_dialectic", content="Weber — rationalization as fate"),
        Vertex("lukacs_dialectic", content="Lukacs — totality and reification"),
        Vertex("sartre_critique", content="critique of Sartre's ultra-bolshevism"),
        Vertex("institution_political", content="political institution vs revolution"),
        Vertex("new_liberalism", content="new liberalism — beyond left/right"),
        Vertex("praxis", content="praxis — open dialectic"),
        Vertex("western_marxism", content="Western Marxism — reflection on practice"),
    ]
    edges = [
        Edge("dialectic", "history", EdgeType.DEPENDENCY),
        Edge("dialectic", "ambiguity", EdgeType.DEPENDENCY),
        Edge("weber_dialectic", "dialectic", EdgeType.DEPENDENCY),
        Edge("lukacs_dialectic", "dialectic", EdgeType.DEPENDENCY),
        Edge("lukacs_dialectic", "proletariat", EdgeType.REFERENCE),
        Edge("sartre_critique", "freedom", EdgeType.NEGATION),
        Edge("sartre_critique", "dialectic", EdgeType.DEPENDENCY),
        Edge("institution_political", "political_action", EdgeType.DEPENDENCY),
        Edge("institution_political", "violence", EdgeType.NEGATION),
        Edge("new_liberalism", "institution_political", EdgeType.DEPENDENCY),
        Edge("praxis", "dialectic", EdgeType.DEPENDENCY),
        Edge("praxis", "contingency", EdgeType.REFERENCE),
        Edge("western_marxism", "marxism_open", EdgeType.SUBLATION),
        Edge("western_marxism", "dialectic", EdgeType.DEPENDENCY),
    ]
    return "1955: Adventures of the Dialectic", vertices, edges


def work_prose_of_world():
    """1952-1959 (posthumous 1969): The Prose of the World (La prose du monde)."""
    vertices = [
        Vertex("expression", content="expression — creative, not representational"),
        Vertex("language", content="language — system of differences"),
        Vertex("indirect_language", content="indirect language — voices of silence"),
        Vertex("literary_expression", content="literary expression"),
        Vertex("sedimentation", content="sedimentation of meaning"),
        Vertex("institution_linguistic", content="linguistic institution"),
        Vertex("creative_expression", content="creative expression — surpassing sedimented"),
        Vertex("algorithm_language", content="algorithm of language — Saussure"),
    ]
    edges = [
        Edge("expression", "body", EdgeType.DEPENDENCY),
        Edge("expression", "speech", EdgeType.SUBLATION),
        Edge("expression", "expression_proto", EdgeType.SUBLATION),
        Edge("language", "expression", EdgeType.DEPENDENCY),
        Edge("language", "sedimentation", EdgeType.DEPENDENCY),
        Edge("indirect_language", "language", EdgeType.DEPENDENCY),
        Edge("indirect_language", "painting", EdgeType.REFERENCE),
        Edge("literary_expression", "expression", EdgeType.DEPENDENCY),
        Edge("literary_expression", "creative_expression", EdgeType.DEPENDENCY),
        Edge("sedimentation", "habit", EdgeType.REFERENCE),
        Edge("institution_linguistic", "language", EdgeType.DEPENDENCY),
        Edge("institution_linguistic", "sedimentation", EdgeType.DEPENDENCY),
        Edge("creative_expression", "sedimentation", EdgeType.NEGATION),
        Edge("algorithm_language", "language", EdgeType.DEPENDENCY),
        Edge("algorithm_language", "structure", EdgeType.REFERENCE),
    ]
    return "1952-1959 (posth. 1969): The Prose of the World", vertices, edges


def work_signs():
    """1960: Signs (Signes)."""
    vertices = [
        Vertex("indirect_ontology", content="indirect ontology — through signs"),
        Vertex("philosophy_and_sociology", content="philosopher and sociology"),
        Vertex("eye_and_mind_proto", content="eye and mind (proto-version in Signs)"),
        Vertex("sign", content="sign — diacritical, not representational"),
        Vertex("interrogation_proto", content="interrogation (proto — philosophical method)"),
    ]
    edges = [
        Edge("indirect_ontology", "perception", EdgeType.SUBLATION),
        Edge("indirect_ontology", "expression", EdgeType.DEPENDENCY),
        Edge("philosophy_and_sociology", "intersubjectivity", EdgeType.DEPENDENCY),
        Edge("philosophy_and_sociology", "dialectic", EdgeType.REFERENCE),
        Edge("eye_and_mind_proto", "painting", EdgeType.DEPENDENCY),
        Edge("eye_and_mind_proto", "body", EdgeType.DEPENDENCY),
        Edge("sign", "language", EdgeType.DEPENDENCY),
        Edge("sign", "sense", EdgeType.DEPENDENCY),
        Edge("interrogation_proto", "ambiguity", EdgeType.DEPENDENCY),
        Edge("interrogation_proto", "indirect_ontology", EdgeType.DEPENDENCY),
    ]
    return "1960: Signs", vertices, edges


def work_eye_and_mind():
    """1961: Eye and Mind (L'oeil et l'esprit)."""
    vertices = [
        Vertex("eye", content="eye — not instrument but of the body"),
        Vertex("vision", content="vision — flesh seeing itself"),
        Vertex("depth", content="depth — not third dimension but dimensionality"),
        Vertex("color", content="color — modulation of the visible"),
        Vertex("mirror", content="mirror — seeing oneself seeing"),
        Vertex("operative_body", content="operative body — I can, not I think"),
    ]
    edges = [
        Edge("eye", "body", EdgeType.DEPENDENCY),
        Edge("eye", "perception", EdgeType.DEPENDENCY),
        Edge("vision", "eye", EdgeType.DEPENDENCY),
        Edge("vision", "painting", EdgeType.DEPENDENCY),
        Edge("depth", "spatiality", EdgeType.SUBLATION),
        Edge("depth", "vision", EdgeType.DEPENDENCY),
        Edge("color", "vision", EdgeType.DEPENDENCY),
        Edge("color", "phenomenal_field", EdgeType.REFERENCE),
        Edge("mirror", "vision", EdgeType.DEPENDENCY),
        Edge("mirror", "intersubjectivity", EdgeType.REFERENCE),
        Edge("operative_body", "body", EdgeType.SUBLATION),
        Edge("operative_body", "motility", EdgeType.DEPENDENCY),
        Edge("operative_body", "eye", EdgeType.DEPENDENCY),
    ]
    return "1961: Eye and Mind", vertices, edges


def work_institution_and_passivity():
    """1954-1955 lectures (posthumous 2003): Institution and Passivity."""
    vertices = [
        Vertex("institution", content="institution (Stiftung) — not constitution"),
        Vertex("passivity", content="passivity — not inactivity but receptivity"),
        Vertex("Stiftung", content="Stiftung — founding that calls for continuation"),
        Vertex("sleep", content="sleep — exemplary passivity"),
        Vertex("unconscious_mp", content="unconscious — institutional sedimentation"),
        Vertex("event", content="event — instituting moment"),
        Vertex("continuation", content="continuation — picking up and transforming"),
    ]
    edges = [
        Edge("institution", "sedimentation", EdgeType.SUBLATION),
        Edge("institution", "habit", EdgeType.SUBLATION),
        Edge("institution", "Stiftung", EdgeType.DEPENDENCY),
        Edge("passivity", "perception", EdgeType.DEPENDENCY),
        Edge("passivity", "institution", EdgeType.NEGATION),
        Edge("Stiftung", "expression", EdgeType.DEPENDENCY),
        Edge("Stiftung", "temporality", EdgeType.DEPENDENCY),
        Edge("sleep", "passivity", EdgeType.DEPENDENCY),
        Edge("sleep", "body", EdgeType.REFERENCE),
        Edge("unconscious_mp", "passivity", EdgeType.DEPENDENCY),
        Edge("unconscious_mp", "institution", EdgeType.DEPENDENCY),
        Edge("event", "institution", EdgeType.DEPENDENCY),
        Edge("event", "contingency", EdgeType.REFERENCE),
        Edge("continuation", "institution", EdgeType.DEPENDENCY),
        Edge("continuation", "creative_expression", EdgeType.REFERENCE),
    ]
    return "1954-1955 (posth. 2003): Institution and Passivity", vertices, edges


def work_nature_lectures():
    """1956-1960 lectures (posthumous 1995): Nature (La Nature)."""
    vertices = [
        Vertex("nature", content="nature — not object but flesh of the world"),
        Vertex("animality", content="animality — behavior as Gestalt"),
        Vertex("human_body_nature", content="human body as natural being"),
        Vertex("nature_perception", content="nature as perceived, not objective"),
        Vertex("umwelt", content="Umwelt — animal world"),
        Vertex("lateral_universal", content="lateral universal — not from above"),
        Vertex("embryology", content="embryology — self-differentiation"),
    ]
    edges = [
        Edge("nature", "world", EdgeType.SUBLATION),
        Edge("nature", "body", EdgeType.DEPENDENCY),
        Edge("animality", "behavior", EdgeType.REFERENCE),
        Edge("animality", "organism", EdgeType.DEPENDENCY),
        Edge("animality", "nature", EdgeType.DEPENDENCY),
        Edge("human_body_nature", "body", EdgeType.DEPENDENCY),
        Edge("human_body_nature", "nature", EdgeType.DEPENDENCY),
        Edge("nature_perception", "perception", EdgeType.DEPENDENCY),
        Edge("nature_perception", "nature", EdgeType.DEPENDENCY),
        Edge("umwelt", "animality", EdgeType.DEPENDENCY),
        Edge("umwelt", "phenomenal_field", EdgeType.REFERENCE),
        Edge("lateral_universal", "intersubjectivity", EdgeType.SUBLATION),
        Edge("lateral_universal", "nature", EdgeType.DEPENDENCY),
        Edge("embryology", "organism", EdgeType.REFERENCE),
        Edge("embryology", "nature", EdgeType.DEPENDENCY),
    ]
    return "1956-1960 (posth. 1995): Nature lectures", vertices, edges


def work_visible_and_invisible():
    """1959-1961 (posthumous 1964): The Visible and the Invisible."""
    vertices = [
        Vertex("flesh", content="flesh (chair) — element, not substance"),
        Vertex("chiasm", content="chiasm (chiasme) — intertwining"),
        Vertex("visible", content="the visible"),
        Vertex("invisible", content="the invisible — not hidden but lining of the visible"),
        Vertex("reversibility", content="reversibility — touching-touched"),
        Vertex("interrogation", content="interrogation — philosophy as radical questioning"),
        Vertex("wild_being", content="wild being (etre sauvage) — pre-reflective"),
        Vertex("dehiscence", content="dehiscence — self-fission of the sensible"),
        Vertex("ecart", content="ecart — divergence, spread"),
        Vertex("fold", content="fold (pli) — self-folding of Being"),
        Vertex("hyperdialectic", content="hyper-dialectic — without synthesis"),
    ]
    edges = [
        Edge("flesh", "body", EdgeType.SUBLATION),
        Edge("flesh", "world", EdgeType.SUBLATION),
        Edge("flesh", "perception", EdgeType.SUBLATION),
        Edge("chiasm", "flesh", EdgeType.DEPENDENCY),
        Edge("chiasm", "reversibility", EdgeType.DEPENDENCY),
        Edge("visible", "perception", EdgeType.SUBLATION),
        Edge("visible", "flesh", EdgeType.DEPENDENCY),
        Edge("invisible", "visible", EdgeType.NEGATION),
        Edge("invisible", "ambiguity", EdgeType.SUBLATION),
        Edge("reversibility", "body", EdgeType.DEPENDENCY),
        Edge("reversibility", "intersubjectivity", EdgeType.SUBLATION),
        Edge("interrogation", "interrogation_proto", EdgeType.SUBLATION),
        Edge("interrogation", "hyperdialectic", EdgeType.DEPENDENCY),
        Edge("wild_being", "nature", EdgeType.DEPENDENCY),
        Edge("wild_being", "flesh", EdgeType.DEPENDENCY),
        Edge("dehiscence", "flesh", EdgeType.DEPENDENCY),
        Edge("dehiscence", "ecart", EdgeType.DEPENDENCY),
        Edge("ecart", "chiasm", EdgeType.DEPENDENCY),
        Edge("ecart", "ambiguity", EdgeType.REFERENCE),
        Edge("fold", "flesh", EdgeType.DEPENDENCY),
        Edge("fold", "reversibility", EdgeType.DEPENDENCY),
        Edge("hyperdialectic", "dialectic", EdgeType.SUBLATION),
        Edge("hyperdialectic", "ambiguity", EdgeType.DEPENDENCY),
    ]
    return "1959-1961 (posth. 1964): The Visible and the Invisible", vertices, edges


def work_child_psychology():
    """1949-1952 lectures (posthumous 2001): Child Psychology and Pedagogy (Sorbonne lectures)."""
    vertices = [
        Vertex("child_perception", content="child's perception — not deficient adult"),
        Vertex("child_body", content="child's body — motor intentionality developing"),
        Vertex("child_language", content="child's language acquisition"),
        Vertex("child_drawing", content="child's drawing — expression not copy"),
        Vertex("developmental_form", content="developmental form — Gestalt in growth"),
    ]
    edges = [
        Edge("child_perception", "perception", EdgeType.DEPENDENCY),
        Edge("child_perception", "phenomenal_field", EdgeType.REFERENCE),
        Edge("child_body", "body", EdgeType.DEPENDENCY),
        Edge("child_body", "schema_corporel", EdgeType.REFERENCE),
        Edge("child_language", "speech", EdgeType.DEPENDENCY),
        Edge("child_language", "child_body", EdgeType.DEPENDENCY),
        Edge("child_drawing", "expression_proto", EdgeType.REFERENCE),
        Edge("child_drawing", "child_perception", EdgeType.DEPENDENCY),
        Edge("developmental_form", "structure", EdgeType.REFERENCE),
        Edge("developmental_form", "child_body", EdgeType.DEPENDENCY),
    ]
    return "1949-1952 (posth. 2001): Child Psychology lectures", vertices, edges


def work_primacy_of_perception():
    """1946: The Primacy of Perception (address to Societe francaise de philosophie)."""
    vertices = [
        Vertex("primacy", content="primacy of perception — thesis"),
        Vertex("perceived_world", content="perceived world — primary reality"),
        Vertex("science_founded", content="science — founded on perceived world"),
        Vertex("classical_prejudices", content="classical prejudices — constancy hypothesis"),
    ]
    edges = [
        Edge("primacy", "perception", EdgeType.DEPENDENCY),
        Edge("primacy", "perceived_world", EdgeType.DEPENDENCY),
        Edge("perceived_world", "phenomenal_field", EdgeType.DEPENDENCY),
        Edge("perceived_world", "world", EdgeType.DEPENDENCY),
        Edge("science_founded", "perceived_world", EdgeType.DEPENDENCY),
        Edge("science_founded", "perception", EdgeType.REFERENCE),
        Edge("classical_prejudices", "sensation", EdgeType.DEPENDENCY),
        Edge("classical_prejudices", "perception", EdgeType.NEGATION),
    ]
    return "1946: The Primacy of Perception", vertices, edges


def work_in_praise_of_philosophy():
    """1953: In Praise of Philosophy (Eloge de la philosophie)."""
    vertices = [
        Vertex("philosophical_irony", content="philosophical irony — Socratic"),
        Vertex("non_philosophy", content="non-philosophy — philosophy's other"),
        Vertex("philosopher_and_shadow", content="philosopher and his shadow"),
        Vertex("heroic_philosophy", content="heroic philosophy — facing contingency"),
    ]
    edges = [
        Edge("philosophical_irony", "ambiguity", EdgeType.DEPENDENCY),
        Edge("philosophical_irony", "interrogation_proto", EdgeType.REFERENCE),
        Edge("non_philosophy", "philosophical_irony", EdgeType.NEGATION),
        Edge("non_philosophy", "existence", EdgeType.REFERENCE),
        Edge("philosopher_and_shadow", "indirect_ontology", EdgeType.REFERENCE),
        Edge("philosopher_and_shadow", "philosophical_irony", EdgeType.DEPENDENCY),
        Edge("heroic_philosophy", "contingency", EdgeType.DEPENDENCY),
        Edge("heroic_philosophy", "freedom", EdgeType.REFERENCE),
    ]
    return "1953: In Praise of Philosophy", vertices, edges


def work_phenomenology_and_sciences_of_man():
    """1951: Phenomenology and the Sciences of Man (lecture)."""
    vertices = [
        Vertex("eidetic_psychology", content="eidetic psychology — between empiricism and intellectualism"),
        Vertex("human_science", content="human science — not natural science model"),
        Vertex("phenomenological_method", content="phenomenological method — return to things"),
    ]
    edges = [
        Edge("eidetic_psychology", "perception", EdgeType.DEPENDENCY),
        Edge("eidetic_psychology", "structure", EdgeType.REFERENCE),
        Edge("human_science", "intersubjectivity", EdgeType.DEPENDENCY),
        Edge("human_science", "eidetic_psychology", EdgeType.DEPENDENCY),
        Edge("phenomenological_method", "perception", EdgeType.DEPENDENCY),
        Edge("phenomenological_method", "existence", EdgeType.REFERENCE),
    ]
    return "1951: Phenomenology and the Sciences of Man", vertices, edges


# ---------------------------------------------------------------------------
# Full works list in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_structure_of_behavior,              # 1942
    work_phenomenology_of_perception,        # 1945
    work_primacy_of_perception,              # 1946
    work_humanism_and_terror,                # 1947
    work_sense_and_nonsense,                 # 1948
    work_child_psychology,                   # 1949-1952
    work_phenomenology_and_sciences_of_man,  # 1951
    work_in_praise_of_philosophy,            # 1953
    work_institution_and_passivity,          # 1954-1955
    work_adventures_of_dialectic,            # 1955
    work_nature_lectures,                    # 1956-1960
    work_prose_of_world,                     # 1952-1959 (posth.)
    work_signs,                              # 1960
    work_eye_and_mind,                       # 1961
    work_visible_and_invisible,              # 1959-1961 (posth.)
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("MERLEAU-PONTY — COMPLETE WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "merleau_ponty_complete_works_incremental",
        "source": "Merleau-Ponty — complete works, 15 entries (1942-1961)",
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

    output_path = "experiment_merleau_ponty.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
