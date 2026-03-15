"""Walter Benjamin — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Benjamin's thought —
from early language philosophy through the dialectical image and
messianic materialism — as an incremental topological growth process.
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
#   aura, mechanical_reproduction, dialectical_image, messianic_time,
#   allegory, ruin, commodity, flaneur, shock, storytelling,
#   experience, translation, origin, monad, constellation,
#   now_time, catastrophe, progress, redemption, memory,
#   language, name, mimesis, semblance, beauty,
#   melancholy, baroque, fate, myth, violence_law,
#   divine_violence, collector, trace, phantasmagoria
# ---------------------------------------------------------------------------


def work_on_language():
    """1916: On Language as Such and on the Language of Man."""
    vertices = [
        Vertex("language", content="language — communicates communicability"),
        Vertex("name", content="name — Adamic naming, non-instrumental"),
        Vertex("word_of_god", content="word of God — creative word"),
        Vertex("translation_proto", content="translation — between languages of things"),
        Vertex("bourgeois_language", content="bourgeois conception of language — instrumental"),
        Vertex("mental_being", content="mental being — communicated in, not through language"),
        Vertex("overnaming", content="overnaming — human language after Fall"),
        Vertex("mute_nature", content="mute nature — sadness of unredeemed naming"),
    ]
    edges = [
        Edge("language", "name", EdgeType.DEPENDENCY),
        Edge("language", "mental_being", EdgeType.DEPENDENCY),
        Edge("name", "word_of_god", EdgeType.DEPENDENCY),
        Edge("name", "bourgeois_language", EdgeType.NEGATION),
        Edge("translation_proto", "language", EdgeType.DEPENDENCY),
        Edge("translation_proto", "name", EdgeType.REFERENCE),
        Edge("overnaming", "name", EdgeType.NEGATION),
        Edge("overnaming", "bourgeois_language", EdgeType.DEPENDENCY),
        Edge("mute_nature", "name", EdgeType.DEPENDENCY),
        Edge("mute_nature", "overnaming", EdgeType.NEGATION),
    ]
    return "1916: On Language as Such and on the Language of Man", vertices, edges


def work_critique_of_violence():
    """1921: Critique of Violence (Zur Kritik der Gewalt)."""
    vertices = [
        Vertex("violence_law", content="violence — relation to law and justice"),
        Vertex("law_making_violence", content="law-making violence (rechtsetzende Gewalt)"),
        Vertex("law_preserving_violence", content="law-preserving violence (rechtserhaltende Gewalt)"),
        Vertex("divine_violence", content="divine violence — annihilating, beyond law"),
        Vertex("general_strike", content="general strike — revolutionary, not political"),
        Vertex("fate", content="fate — mythical order"),
        Vertex("myth", content="myth — entanglement with guilt/law"),
        Vertex("justice", content="justice — beyond law, unrepresentable"),
    ]
    edges = [
        Edge("violence_law", "law_making_violence", EdgeType.DEPENDENCY),
        Edge("violence_law", "law_preserving_violence", EdgeType.DEPENDENCY),
        Edge("law_making_violence", "law_preserving_violence", EdgeType.NEGATION),
        Edge("divine_violence", "law_making_violence", EdgeType.NEGATION),
        Edge("divine_violence", "law_preserving_violence", EdgeType.NEGATION),
        Edge("divine_violence", "justice", EdgeType.DEPENDENCY),
        Edge("general_strike", "law_preserving_violence", EdgeType.NEGATION),
        Edge("fate", "myth", EdgeType.DEPENDENCY),
        Edge("fate", "violence_law", EdgeType.DEPENDENCY),
        Edge("myth", "divine_violence", EdgeType.NEGATION),
        Edge("justice", "fate", EdgeType.NEGATION),
    ]
    return "1921: Critique of Violence", vertices, edges


def work_task_of_translator():
    """1921: The Task of the Translator (Die Aufgabe des Ubersetzers)."""
    vertices = [
        Vertex("translation", content="translation — afterlife of the work"),
        Vertex("pure_language", content="pure language — reine Sprache, messianic horizon"),
        Vertex("original", content="original — calls for translation"),
        Vertex("kinship_languages", content="kinship of languages — converging toward pure language"),
        Vertex("literalness", content="literalness — word-for-word as ideal"),
        Vertex("translatability", content="translatability — essential quality of original"),
    ]
    edges = [
        Edge("translation", "translation_proto", EdgeType.SUBLATION),
        Edge("translation", "pure_language", EdgeType.DEPENDENCY),
        Edge("translation", "original", EdgeType.DEPENDENCY),
        Edge("pure_language", "language", EdgeType.SUBLATION),
        Edge("pure_language", "name", EdgeType.REFERENCE),
        Edge("kinship_languages", "pure_language", EdgeType.DEPENDENCY),
        Edge("kinship_languages", "translation", EdgeType.DEPENDENCY),
        Edge("literalness", "translation", EdgeType.DEPENDENCY),
        Edge("literalness", "bourgeois_language", EdgeType.NEGATION),
        Edge("translatability", "original", EdgeType.DEPENDENCY),
        Edge("translatability", "pure_language", EdgeType.REFERENCE),
    ]
    return "1921: The Task of the Translator", vertices, edges


def work_one_way_street():
    """1928: One-Way Street (Einbahnstrasse)."""
    vertices = [
        Vertex("montage", content="montage — literary technique"),
        Vertex("thought_image", content="thought-image (Denkbild)"),
        Vertex("urban_experience", content="urban experience"),
        Vertex("commodity_world", content="commodity world — everyday objects"),
        Vertex("distraction", content="distraction — mode of reception"),
        Vertex("imperialism_images", content="imperialism of images"),
    ]
    edges = [
        Edge("montage", "language", EdgeType.DEPENDENCY),
        Edge("montage", "bourgeois_language", EdgeType.NEGATION),
        Edge("thought_image", "montage", EdgeType.DEPENDENCY),
        Edge("thought_image", "name", EdgeType.REFERENCE),
        Edge("urban_experience", "commodity_world", EdgeType.DEPENDENCY),
        Edge("urban_experience", "distraction", EdgeType.DEPENDENCY),
        Edge("commodity_world", "fate", EdgeType.REFERENCE),
        Edge("distraction", "urban_experience", EdgeType.DEPENDENCY),
        Edge("imperialism_images", "commodity_world", EdgeType.DEPENDENCY),
    ]
    return "1928: One-Way Street", vertices, edges


def work_origin_german_trauerspiel():
    """1928: Origin of German Tragic Drama (Ursprung des deutschen Trauerspiels)."""
    vertices = [
        Vertex("origin", content="origin (Ursprung) — not genesis but eddy in the stream"),
        Vertex("allegory", content="allegory — not symbol, facies hippocratica of history"),
        Vertex("ruin", content="ruin — nature-history, transience"),
        Vertex("melancholy", content="melancholy — Saturnine contemplation"),
        Vertex("baroque", content="baroque — world as stage"),
        Vertex("monad", content="monad — idea contains total image"),
        Vertex("constellation", content="constellation — ideas as stars"),
        Vertex("natural_history", content="natural history — nature as history, history as nature"),
        Vertex("symbol_critique", content="critique of the Romantic symbol"),
        Vertex("sovereign", content="sovereign — creature, not ruler"),
    ]
    edges = [
        Edge("origin", "monad", EdgeType.DEPENDENCY),
        Edge("origin", "constellation", EdgeType.DEPENDENCY),
        Edge("allegory", "symbol_critique", EdgeType.NEGATION),
        Edge("allegory", "ruin", EdgeType.DEPENDENCY),
        Edge("allegory", "melancholy", EdgeType.DEPENDENCY),
        Edge("ruin", "natural_history", EdgeType.DEPENDENCY),
        Edge("ruin", "allegory", EdgeType.DEPENDENCY),
        Edge("melancholy", "baroque", EdgeType.DEPENDENCY),
        Edge("melancholy", "fate", EdgeType.REFERENCE),
        Edge("baroque", "allegory", EdgeType.DEPENDENCY),
        Edge("monad", "origin", EdgeType.DEPENDENCY),
        Edge("constellation", "monad", EdgeType.DEPENDENCY),
        Edge("natural_history", "myth", EdgeType.NEGATION),
        Edge("sovereign", "melancholy", EdgeType.DEPENDENCY),
        Edge("sovereign", "baroque", EdgeType.DEPENDENCY),
    ]
    return "1928: Origin of German Tragic Drama", vertices, edges


def work_surrealism():
    """1929: Surrealism: The Last Snapshot of the European Intelligentsia."""
    vertices = [
        Vertex("profane_illumination", content="profane illumination — materialist inspiration"),
        Vertex("intoxication", content="intoxication — threshold experience"),
        Vertex("body_space", content="body space (Leibraum) — revolutionary"),
        Vertex("image_space", content="image space (Bildraum) — political"),
    ]
    edges = [
        Edge("profane_illumination", "intoxication", EdgeType.SUBLATION),
        Edge("profane_illumination", "distraction", EdgeType.REFERENCE),
        Edge("body_space", "profane_illumination", EdgeType.DEPENDENCY),
        Edge("image_space", "profane_illumination", EdgeType.DEPENDENCY),
        Edge("image_space", "montage", EdgeType.REFERENCE),
        Edge("intoxication", "urban_experience", EdgeType.REFERENCE),
    ]
    return "1929: Surrealism", vertices, edges


def work_little_history_photography():
    """1931: Little History of Photography (Kleine Geschichte der Photographie)."""
    vertices = [
        Vertex("aura_proto", content="aura (proto) — strange weave of space and time"),
        Vertex("optical_unconscious", content="optical unconscious"),
        Vertex("decay_aura", content="decay of the aura"),
        Vertex("portrait", content="portrait — cult value persists"),
    ]
    edges = [
        Edge("aura_proto", "urban_experience", EdgeType.DEPENDENCY),
        Edge("aura_proto", "origin", EdgeType.REFERENCE),
        Edge("optical_unconscious", "aura_proto", EdgeType.DEPENDENCY),
        Edge("optical_unconscious", "distraction", EdgeType.REFERENCE),
        Edge("decay_aura", "aura_proto", EdgeType.NEGATION),
        Edge("decay_aura", "commodity_world", EdgeType.DEPENDENCY),
        Edge("portrait", "aura_proto", EdgeType.DEPENDENCY),
    ]
    return "1931: Little History of Photography", vertices, edges


def work_karl_kraus():
    """1931: Karl Kraus."""
    vertices = [
        Vertex("destructive_character", content="destructive character — makes room"),
        Vertex("quotation", content="quotation — tearing from context"),
        Vertex("kraus_demon", content="Kraus — demon of language"),
    ]
    edges = [
        Edge("destructive_character", "ruin", EdgeType.REFERENCE),
        Edge("destructive_character", "divine_violence", EdgeType.REFERENCE),
        Edge("quotation", "montage", EdgeType.DEPENDENCY),
        Edge("quotation", "language", EdgeType.DEPENDENCY),
        Edge("kraus_demon", "name", EdgeType.REFERENCE),
        Edge("kraus_demon", "quotation", EdgeType.DEPENDENCY),
    ]
    return "1931: Karl Kraus", vertices, edges


def work_berlin_childhood():
    """1932-1938: A Berlin Childhood around 1900 (Berliner Kindheit um 1900)."""
    vertices = [
        Vertex("childhood_memory", content="childhood memory — involuntary, spatial"),
        Vertex("threshold", content="threshold (Schwelle) — passage, not boundary"),
        Vertex("mimesis", content="mimesis — child's faculty of seeing resemblances"),
        Vertex("trace", content="trace — imprint of things"),
        Vertex("loggias", content="loggias — threshold between inside and outside"),
        Vertex("hiding", content="hiding — child becomes thing-like"),
    ]
    edges = [
        Edge("childhood_memory", "urban_experience", EdgeType.DEPENDENCY),
        Edge("childhood_memory", "thought_image", EdgeType.REFERENCE),
        Edge("threshold", "childhood_memory", EdgeType.DEPENDENCY),
        Edge("threshold", "profane_illumination", EdgeType.REFERENCE),
        Edge("mimesis", "name", EdgeType.DEPENDENCY),
        Edge("mimesis", "language", EdgeType.DEPENDENCY),
        Edge("trace", "aura_proto", EdgeType.DEPENDENCY),
        Edge("trace", "childhood_memory", EdgeType.DEPENDENCY),
        Edge("loggias", "threshold", EdgeType.DEPENDENCY),
        Edge("hiding", "mimesis", EdgeType.DEPENDENCY),
        Edge("hiding", "childhood_memory", EdgeType.DEPENDENCY),
    ]
    return "1932-1938: A Berlin Childhood around 1900", vertices, edges


def work_work_of_art():
    """1935-1939: The Work of Art in the Age of Mechanical Reproduction."""
    vertices = [
        Vertex("aura", content="aura — unique appearance of distance"),
        Vertex("mechanical_reproduction", content="mechanical reproduction — shatters aura"),
        Vertex("cult_value", content="cult value — embedded in ritual"),
        Vertex("exhibition_value", content="exhibition value — replaces cult value"),
        Vertex("film", content="film — art of distracted masses"),
        Vertex("shock_effect", content="shock effect — tactile reception"),
        Vertex("politicization_art", content="politicization of art — vs aestheticization of politics"),
        Vertex("aestheticization_politics", content="aestheticization of politics — fascism"),
    ]
    edges = [
        Edge("aura", "aura_proto", EdgeType.SUBLATION),
        Edge("aura", "cult_value", EdgeType.DEPENDENCY),
        Edge("mechanical_reproduction", "aura", EdgeType.NEGATION),
        Edge("mechanical_reproduction", "exhibition_value", EdgeType.DEPENDENCY),
        Edge("cult_value", "exhibition_value", EdgeType.NEGATION),
        Edge("exhibition_value", "mechanical_reproduction", EdgeType.DEPENDENCY),
        Edge("film", "mechanical_reproduction", EdgeType.DEPENDENCY),
        Edge("film", "distraction", EdgeType.DEPENDENCY),
        Edge("shock_effect", "film", EdgeType.DEPENDENCY),
        Edge("shock_effect", "distraction", EdgeType.SUBLATION),
        Edge("politicization_art", "aestheticization_politics", EdgeType.NEGATION),
        Edge("politicization_art", "mechanical_reproduction", EdgeType.DEPENDENCY),
        Edge("aestheticization_politics", "aura", EdgeType.DEPENDENCY),
        Edge("aestheticization_politics", "myth", EdgeType.REFERENCE),
    ]
    return "1935-1939: The Work of Art in the Age of Mechanical Reproduction", vertices, edges


def work_storyteller():
    """1936: The Storyteller (Der Erzahler)."""
    vertices = [
        Vertex("storytelling", content="storytelling — communicability of experience"),
        Vertex("experience_erfahrung", content="Erfahrung — transmissible experience"),
        Vertex("experience_erlebnis", content="Erlebnis — isolated lived experience"),
        Vertex("counsel", content="counsel — wisdom woven into storytelling"),
        Vertex("novel", content="novel — birth of the lonely individual"),
        Vertex("information", content="information — enemy of storytelling"),
        Vertex("death", content="death — authority of the storyteller"),
    ]
    edges = [
        Edge("storytelling", "experience_erfahrung", EdgeType.DEPENDENCY),
        Edge("storytelling", "information", EdgeType.NEGATION),
        Edge("experience_erfahrung", "experience_erlebnis", EdgeType.NEGATION),
        Edge("experience_erfahrung", "language", EdgeType.DEPENDENCY),
        Edge("counsel", "storytelling", EdgeType.DEPENDENCY),
        Edge("counsel", "experience_erfahrung", EdgeType.DEPENDENCY),
        Edge("novel", "storytelling", EdgeType.NEGATION),
        Edge("novel", "experience_erlebnis", EdgeType.DEPENDENCY),
        Edge("information", "mechanical_reproduction", EdgeType.REFERENCE),
        Edge("death", "storytelling", EdgeType.DEPENDENCY),
        Edge("death", "experience_erfahrung", EdgeType.DEPENDENCY),
    ]
    return "1936: The Storyteller", vertices, edges


def work_baudelaire():
    """1938-1939: Charles Baudelaire: A Lyric Poet in the Era of High Capitalism."""
    vertices = [
        Vertex("flaneur", content="flaneur — stroller in the crowd"),
        Vertex("crowd", content="crowd — shock experience"),
        Vertex("shock", content="shock — Baudelaire's parrying"),
        Vertex("commodity", content="commodity — fetish character"),
        Vertex("phantasmagoria", content="phantasmagoria — commodity as dreamworld"),
        Vertex("spleen", content="spleen — experience of time emptied"),
        Vertex("correspondances", content="correspondances — involuntary memory"),
        Vertex("heroism_modern", content="heroism of modern life — ragpicker"),
    ]
    edges = [
        Edge("flaneur", "crowd", EdgeType.DEPENDENCY),
        Edge("flaneur", "urban_experience", EdgeType.SUBLATION),
        Edge("crowd", "shock", EdgeType.DEPENDENCY),
        Edge("shock", "experience_erlebnis", EdgeType.DEPENDENCY),
        Edge("shock", "experience_erfahrung", EdgeType.NEGATION),
        Edge("commodity", "phantasmagoria", EdgeType.DEPENDENCY),
        Edge("commodity", "commodity_world", EdgeType.SUBLATION),
        Edge("phantasmagoria", "aura", EdgeType.NEGATION),
        Edge("phantasmagoria", "myth", EdgeType.DEPENDENCY),
        Edge("spleen", "shock", EdgeType.DEPENDENCY),
        Edge("spleen", "melancholy", EdgeType.REFERENCE),
        Edge("correspondances", "experience_erfahrung", EdgeType.REFERENCE),
        Edge("correspondances", "mimesis", EdgeType.REFERENCE),
        Edge("heroism_modern", "flaneur", EdgeType.DEPENDENCY),
        Edge("heroism_modern", "allegory", EdgeType.REFERENCE),
    ]
    return "1938-1939: Charles Baudelaire", vertices, edges


def work_theses_on_history():
    """1940: Theses on the Philosophy of History (On the Concept of History)."""
    vertices = [
        Vertex("messianic_time", content="messianic time (Jetztzeit) — now-time"),
        Vertex("angel_of_history", content="angel of history — Angelus Novus"),
        Vertex("progress", content="progress — storm from paradise"),
        Vertex("catastrophe", content="catastrophe — not exception but rule"),
        Vertex("redemption", content="redemption — weak messianic power"),
        Vertex("historical_materialism", content="historical materialism — tiger's leap"),
        Vertex("tradition_oppressed", content="tradition of the oppressed"),
        Vertex("dialectical_image_proto", content="dialectical image (proto) — flash of recognizability"),
        Vertex("homogeneous_empty_time", content="homogeneous empty time — historicism"),
    ]
    edges = [
        Edge("messianic_time", "homogeneous_empty_time", EdgeType.NEGATION),
        Edge("messianic_time", "redemption", EdgeType.DEPENDENCY),
        Edge("angel_of_history", "catastrophe", EdgeType.DEPENDENCY),
        Edge("angel_of_history", "progress", EdgeType.NEGATION),
        Edge("progress", "homogeneous_empty_time", EdgeType.DEPENDENCY),
        Edge("progress", "catastrophe", EdgeType.NEGATION),
        Edge("catastrophe", "ruin", EdgeType.DEPENDENCY),
        Edge("catastrophe", "tradition_oppressed", EdgeType.DEPENDENCY),
        Edge("redemption", "tradition_oppressed", EdgeType.DEPENDENCY),
        Edge("redemption", "messianic_time", EdgeType.DEPENDENCY),
        Edge("historical_materialism", "messianic_time", EdgeType.DEPENDENCY),
        Edge("historical_materialism", "dialectical_image_proto", EdgeType.DEPENDENCY),
        Edge("tradition_oppressed", "storytelling", EdgeType.REFERENCE),
        Edge("dialectical_image_proto", "messianic_time", EdgeType.DEPENDENCY),
        Edge("dialectical_image_proto", "constellation", EdgeType.REFERENCE),
        Edge("homogeneous_empty_time", "progress", EdgeType.DEPENDENCY),
    ]
    return "1940: Theses on the Philosophy of History", vertices, edges


def work_arcades_project():
    """1927-1940: The Arcades Project (Das Passagen-Werk)."""
    vertices = [
        Vertex("dialectical_image", content="dialectical image — now of recognizability"),
        Vertex("arcade", content="arcade (Passage) — dream house of the collective"),
        Vertex("collector", content="collector — rescues things from commodity"),
        Vertex("wish_image", content="wish image — utopian impulse in commodity"),
        Vertex("awakening", content="awakening — dialectics of dreaming"),
        Vertex("fashion", content="fashion — tiger's leap into the past"),
        Vertex("boredom", content="boredom — threshold to great deeds"),
        Vertex("prostitution", content="prostitution — seller and commodity in one"),
        Vertex("iron_construction", content="iron and glass — new materials, old forms"),
        Vertex("dream_collective", content="collective dream — mythic consciousness"),
    ]
    edges = [
        Edge("dialectical_image", "dialectical_image_proto", EdgeType.SUBLATION),
        Edge("dialectical_image", "constellation", EdgeType.DEPENDENCY),
        Edge("dialectical_image", "messianic_time", EdgeType.DEPENDENCY),
        Edge("arcade", "phantasmagoria", EdgeType.DEPENDENCY),
        Edge("arcade", "urban_experience", EdgeType.DEPENDENCY),
        Edge("collector", "trace", EdgeType.DEPENDENCY),
        Edge("collector", "commodity", EdgeType.NEGATION),
        Edge("wish_image", "commodity", EdgeType.DEPENDENCY),
        Edge("wish_image", "redemption", EdgeType.REFERENCE),
        Edge("awakening", "dream_collective", EdgeType.NEGATION),
        Edge("awakening", "dialectical_image", EdgeType.DEPENDENCY),
        Edge("fashion", "commodity", EdgeType.DEPENDENCY),
        Edge("fashion", "messianic_time", EdgeType.REFERENCE),
        Edge("boredom", "spleen", EdgeType.REFERENCE),
        Edge("boredom", "threshold", EdgeType.REFERENCE),
        Edge("prostitution", "commodity", EdgeType.DEPENDENCY),
        Edge("prostitution", "flaneur", EdgeType.REFERENCE),
        Edge("iron_construction", "arcade", EdgeType.DEPENDENCY),
        Edge("iron_construction", "wish_image", EdgeType.DEPENDENCY),
        Edge("dream_collective", "phantasmagoria", EdgeType.DEPENDENCY),
        Edge("dream_collective", "myth", EdgeType.DEPENDENCY),
    ]
    return "1927-1940: The Arcades Project", vertices, edges


def work_doctrine_of_similar():
    """1933: Doctrine of the Similar / On the Mimetic Faculty."""
    vertices = [
        Vertex("mimetic_faculty", content="mimetic faculty — producing similarities"),
        Vertex("nonsensuous_similarity", content="non-sensuous similarity — in language"),
        Vertex("astrology_mimesis", content="astrological correspondence — archaic mimesis"),
        Vertex("reading", content="reading — most recent form of mimesis"),
    ]
    edges = [
        Edge("mimetic_faculty", "mimesis", EdgeType.SUBLATION),
        Edge("mimetic_faculty", "name", EdgeType.REFERENCE),
        Edge("nonsensuous_similarity", "mimetic_faculty", EdgeType.SUBLATION),
        Edge("nonsensuous_similarity", "language", EdgeType.DEPENDENCY),
        Edge("astrology_mimesis", "mimetic_faculty", EdgeType.DEPENDENCY),
        Edge("astrology_mimesis", "correspondances", EdgeType.REFERENCE),
        Edge("reading", "nonsensuous_similarity", EdgeType.DEPENDENCY),
        Edge("reading", "language", EdgeType.REFERENCE),
    ]
    return "1933: Doctrine of the Similar", vertices, edges


def work_kafka():
    """1934: Franz Kafka: On the Tenth Anniversary of His Death."""
    vertices = [
        Vertex("gesture_kafka", content="gesture — Kafka's clouded parable"),
        Vertex("distortion", content="distortion (Entstellung) — truth as disfigured"),
        Vertex("forgotten", content="the forgotten — prehistoric forces"),
        Vertex("court_kafka", content="court — bureaucratic fate"),
    ]
    edges = [
        Edge("gesture_kafka", "storytelling", EdgeType.DEPENDENCY),
        Edge("gesture_kafka", "allegory", EdgeType.REFERENCE),
        Edge("distortion", "truth_content_proto", EdgeType.DEPENDENCY),
        Edge("distortion", "myth", EdgeType.DEPENDENCY),
        Edge("forgotten", "experience_erfahrung", EdgeType.NEGATION),
        Edge("forgotten", "tradition_oppressed", EdgeType.REFERENCE),
        Edge("court_kafka", "fate", EdgeType.DEPENDENCY),
        Edge("court_kafka", "law_preserving_violence", EdgeType.REFERENCE),
    ]
    # Need truth_content_proto
    vertices.append(Vertex("truth_content_proto", content="truth content (proto) — non-intentional"))
    return "1934: Franz Kafka", vertices, edges


def work_author_as_producer():
    """1934: The Author as Producer."""
    vertices = [
        Vertex("literary_technique", content="literary technique — progressive form"),
        Vertex("author_producer", content="author as producer — in production relations"),
        Vertex("apparatus", content="apparatus — transformation, not supply"),
    ]
    edges = [
        Edge("literary_technique", "montage", EdgeType.DEPENDENCY),
        Edge("literary_technique", "mechanical_reproduction", EdgeType.REFERENCE),
        Edge("author_producer", "literary_technique", EdgeType.DEPENDENCY),
        Edge("author_producer", "politicization_art", EdgeType.REFERENCE),
        Edge("apparatus", "author_producer", EdgeType.DEPENDENCY),
        Edge("apparatus", "commodity", EdgeType.NEGATION),
    ]
    return "1934: The Author as Producer", vertices, edges


# ---------------------------------------------------------------------------
# Full works list in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_on_language,                        # 1916
    work_critique_of_violence,               # 1921
    work_task_of_translator,                 # 1921
    work_one_way_street,                     # 1928
    work_origin_german_trauerspiel,          # 1928
    work_surrealism,                         # 1929
    work_little_history_photography,         # 1931
    work_karl_kraus,                         # 1931
    work_berlin_childhood,                   # 1932-1938
    work_doctrine_of_similar,                # 1933
    work_kafka,                              # 1934
    work_author_as_producer,                 # 1934
    work_work_of_art,                        # 1935-1939
    work_storyteller,                        # 1936
    work_baudelaire,                         # 1938-1939
    work_theses_on_history,                  # 1940
    work_arcades_project,                    # 1927-1940
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("WALTER BENJAMIN — COMPLETE WORKS INCREMENTAL TRAVERSAL")
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

        # 1. Collect new vertices
        new_vertices = []
        existing_vids = set(graph.vertices.keys())
        for v in w_vertices:
            if v.id not in existing_vids:
                new_vertices.append(v)
                existing_vids.add(v.id)
        new_v_count = len(new_vertices)

        # 2. Collect new edges (skip duplicates, check endpoints in existing + new vids)
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
        "experiment": "benjamin_complete_works_incremental",
        "source": "Walter Benjamin — complete works, 17 entries (1916-1940)",
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

    output_path = "experiment_benjamin.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
