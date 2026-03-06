"""Martin Heidegger — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Heidegger's thought
as an incremental topological growth process — from early
phenomenological hermeneutics through fundamental ontology to
the later topology of Being (Ereignis, Gestell, Geviert).
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
#   being, beings, dasein, world, being_in_the_world,
#   care, thrownness, projection, falling, authenticity, inauthenticity,
#   das_man, anxiety, death, being_toward_death,
#   temporality, time, ecstasis, understanding, interpretation,
#   mood, attunement, discourse, language,
#   truth, aletheia, unconcealment, concealment,
#   ereignis, gestell, enframing, techne, poiesis,
#   fourfold, earth, sky, mortals, divinities,
#   dwelling, building, thing, lichtung, clearing,
#   nothing, ground, abyss, freedom, transcendence,
#   ontological_difference, presence, absence,
#   history, historicity, destiny, sending,
#   logos, physis, noein, legein
# ---------------------------------------------------------------------------


def work_history_concept_of_time():
    """1925: History of the Concept of Time — Prolegomena."""
    vertices = [
        Vertex("being", content="Being (Sein)"),
        Vertex("beings", content="beings (Seiendes)"),
        Vertex("ontological_difference", content="ontological difference"),
        Vertex("dasein", content="Dasein"),
        Vertex("intentionality", content="intentionality"),
        Vertex("categorial_intuition", content="categorial intuition"),
        Vertex("a_priori", content="the a priori"),
        Vertex("phenomenology_method", content="phenomenological method"),
        Vertex("time_concept_history", content="history of the concept of time"),
        Vertex("natural_attitude", content="natural attitude"),
        Vertex("world", content="world"),
        Vertex("being_in_the_world", content="being-in-the-world"),
    ]
    edges = [
        Edge("ontological_difference", "being", EdgeType.DEPENDENCY),
        Edge("ontological_difference", "beings", EdgeType.DEPENDENCY),
        Edge("being", "beings", EdgeType.NEGATION),
        Edge("dasein", "being_in_the_world", EdgeType.DEPENDENCY),
        Edge("being_in_the_world", "world", EdgeType.DEPENDENCY),
        Edge("intentionality", "dasein", EdgeType.REFERENCE),
        Edge("categorial_intuition", "intentionality", EdgeType.SUBLATION),
        Edge("categorial_intuition", "a_priori", EdgeType.DEPENDENCY),
        Edge("phenomenology_method", "categorial_intuition", EdgeType.DEPENDENCY),
        Edge("time_concept_history", "being", EdgeType.REFERENCE),
        Edge("natural_attitude", "phenomenology_method", EdgeType.NEGATION),
        Edge("dasein", "ontological_difference", EdgeType.DEPENDENCY),
    ]
    return "1925: History of the Concept of Time", vertices, edges


def work_basic_problems_phenomenology():
    """1927a: Basic Problems of Phenomenology."""
    vertices = [
        Vertex("presence", content="presence (Anwesenheit)"),
        Vertex("absence", content="absence"),
        Vertex("temporality", content="temporality (Zeitlichkeit)"),
        Vertex("temporal_interpretation", content="temporal interpretation of Being"),
        Vertex("ontological_reduction", content="ontological/phenomenological reduction"),
        Vertex("ontological_construction", content="ontological construction"),
        Vertex("ontological_destruction", content="ontological destruction"),
        Vertex("res_cogitans", content="res cogitans (Descartes thesis)"),
        Vertex("medieval_ontology", content="medieval ontology"),
    ]
    edges = [
        Edge("presence", "being", EdgeType.DEPENDENCY),
        Edge("absence", "presence", EdgeType.NEGATION),
        Edge("temporality", "being", EdgeType.DEPENDENCY),
        Edge("temporality", "dasein", EdgeType.DEPENDENCY),
        Edge("temporal_interpretation", "temporality", EdgeType.DEPENDENCY),
        Edge("temporal_interpretation", "ontological_difference", EdgeType.DEPENDENCY),
        Edge("ontological_reduction", "phenomenology_method", EdgeType.DEPENDENCY),
        Edge("ontological_construction", "ontological_reduction", EdgeType.DEPENDENCY),
        Edge("ontological_destruction", "ontological_construction", EdgeType.DEPENDENCY),
        Edge("ontological_destruction", "medieval_ontology", EdgeType.NEGATION),
        Edge("res_cogitans", "dasein", EdgeType.NEGATION),
        Edge("presence", "absence", EdgeType.NEGATION),
    ]
    return "1927a: Basic Problems of Phenomenology", vertices, edges


def work_being_and_time():
    """1927b: Being and Time (Sein und Zeit)."""
    vertices = [
        Vertex("care", content="care (Sorge)"),
        Vertex("thrownness", content="thrownness (Geworfenheit)"),
        Vertex("projection", content="projection (Entwurf)"),
        Vertex("falling", content="falling (Verfallen)"),
        Vertex("authenticity", content="authenticity (Eigentlichkeit)"),
        Vertex("inauthenticity", content="inauthenticity (Uneigentlichkeit)"),
        Vertex("das_man", content="the They (das Man)"),
        Vertex("anxiety", content="anxiety (Angst)"),
        Vertex("death", content="death"),
        Vertex("being_toward_death", content="being-toward-death"),
        Vertex("understanding", content="understanding (Verstehen)"),
        Vertex("interpretation", content="interpretation (Auslegung)"),
        Vertex("mood", content="mood/attunement (Befindlichkeit)"),
        Vertex("discourse", content="discourse (Rede)"),
        Vertex("truth", content="truth (Wahrheit)"),
        Vertex("readiness_to_hand", content="readiness-to-hand (Zuhandenheit)"),
        Vertex("present_at_hand", content="present-at-hand (Vorhandenheit)"),
        Vertex("equipment", content="equipment (Zeug)"),
        Vertex("resoluteness", content="resoluteness (Entschlossenheit)"),
        Vertex("historicity", content="historicity (Geschichtlichkeit)"),
        Vertex("conscience", content="conscience (Gewissen)"),
        Vertex("guilt", content="guilt (Schuld)"),
        Vertex("ecstasis", content="ecstatic temporality"),
    ]
    edges = [
        Edge("care", "thrownness", EdgeType.DEPENDENCY),
        Edge("care", "projection", EdgeType.DEPENDENCY),
        Edge("care", "falling", EdgeType.DEPENDENCY),
        Edge("care", "dasein", EdgeType.DEPENDENCY),
        Edge("thrownness", "mood", EdgeType.DEPENDENCY),
        Edge("projection", "understanding", EdgeType.DEPENDENCY),
        Edge("falling", "das_man", EdgeType.DEPENDENCY),
        Edge("authenticity", "inauthenticity", EdgeType.NEGATION),
        Edge("inauthenticity", "das_man", EdgeType.DEPENDENCY),
        Edge("anxiety", "das_man", EdgeType.NEGATION),
        Edge("anxiety", "being_in_the_world", EdgeType.DEPENDENCY),
        Edge("death", "dasein", EdgeType.DEPENDENCY),
        Edge("being_toward_death", "death", EdgeType.DEPENDENCY),
        Edge("being_toward_death", "authenticity", EdgeType.DEPENDENCY),
        Edge("being_toward_death", "anxiety", EdgeType.DEPENDENCY),
        Edge("understanding", "interpretation", EdgeType.DEPENDENCY),
        Edge("understanding", "projection", EdgeType.REFERENCE),
        Edge("mood", "thrownness", EdgeType.REFERENCE),
        Edge("discourse", "understanding", EdgeType.DEPENDENCY),
        Edge("discourse", "mood", EdgeType.DEPENDENCY),
        Edge("truth", "being", EdgeType.DEPENDENCY),
        Edge("truth", "dasein", EdgeType.DEPENDENCY),
        Edge("readiness_to_hand", "present_at_hand", EdgeType.NEGATION),
        Edge("readiness_to_hand", "equipment", EdgeType.DEPENDENCY),
        Edge("equipment", "world", EdgeType.DEPENDENCY),
        Edge("present_at_hand", "readiness_to_hand", EdgeType.SUBLATION),
        Edge("resoluteness", "being_toward_death", EdgeType.DEPENDENCY),
        Edge("resoluteness", "conscience", EdgeType.DEPENDENCY),
        Edge("conscience", "guilt", EdgeType.DEPENDENCY),
        Edge("guilt", "care", EdgeType.DEPENDENCY),
        Edge("historicity", "temporality", EdgeType.DEPENDENCY),
        Edge("historicity", "dasein", EdgeType.DEPENDENCY),
        Edge("ecstasis", "temporality", EdgeType.DEPENDENCY),
        Edge("ecstasis", "care", EdgeType.DEPENDENCY),
        Edge("temporality", "ecstasis", EdgeType.REFERENCE),
    ]
    return "1927b: Being and Time", vertices, edges


def work_kant_problem_metaphysics():
    """1929a: Kant and the Problem of Metaphysics."""
    vertices = [
        Vertex("finitude", content="finitude of Dasein"),
        Vertex("transcendence", content="transcendence"),
        Vertex("imagination", content="transcendental imagination"),
        Vertex("ground_metaphysics", content="ground of metaphysics"),
        Vertex("ontological_knowledge", content="ontological knowledge"),
        Vertex("receptivity", content="receptivity (finite intuition)"),
        Vertex("spontaneity", content="spontaneity (understanding)"),
    ]
    edges = [
        Edge("finitude", "dasein", EdgeType.DEPENDENCY),
        Edge("finitude", "transcendence", EdgeType.DEPENDENCY),
        Edge("transcendence", "being_in_the_world", EdgeType.REFERENCE),
        Edge("imagination", "receptivity", EdgeType.SUBLATION),
        Edge("imagination", "spontaneity", EdgeType.SUBLATION),
        Edge("imagination", "temporality", EdgeType.DEPENDENCY),
        Edge("ground_metaphysics", "finitude", EdgeType.DEPENDENCY),
        Edge("ground_metaphysics", "ontological_difference", EdgeType.DEPENDENCY),
        Edge("ontological_knowledge", "imagination", EdgeType.DEPENDENCY),
        Edge("ontological_knowledge", "a_priori", EdgeType.REFERENCE),
        Edge("receptivity", "spontaneity", EdgeType.NEGATION),
    ]
    return "1929a: Kant and the Problem of Metaphysics", vertices, edges


def work_what_is_metaphysics():
    """1929b: What Is Metaphysics?"""
    vertices = [
        Vertex("nothing", content="the Nothing (das Nichts)"),
        Vertex("negation_logic", content="logical negation"),
        Vertex("ground", content="ground (Grund)"),
        Vertex("abyss", content="abyss (Ab-grund)"),
    ]
    edges = [
        Edge("nothing", "being", EdgeType.NEGATION),
        Edge("nothing", "anxiety", EdgeType.DEPENDENCY),
        Edge("nothing", "beings", EdgeType.NEGATION),
        Edge("negation_logic", "nothing", EdgeType.NEGATION),
        Edge("ground", "nothing", EdgeType.DEPENDENCY),
        Edge("abyss", "ground", EdgeType.NEGATION),
        Edge("abyss", "nothing", EdgeType.DEPENDENCY),
        Edge("dasein", "nothing", EdgeType.DEPENDENCY),
    ]
    return "1929b: What Is Metaphysics?", vertices, edges


def work_essence_of_ground():
    """1929c: On the Essence of Ground."""
    vertices = [
        Vertex("freedom", content="freedom (Freiheit)"),
        Vertex("world_forming", content="world-forming"),
        Vertex("transcendence_ground", content="transcendence as ground"),
    ]
    edges = [
        Edge("freedom", "transcendence", EdgeType.DEPENDENCY),
        Edge("freedom", "ground", EdgeType.DEPENDENCY),
        Edge("world_forming", "world", EdgeType.DEPENDENCY),
        Edge("world_forming", "transcendence", EdgeType.DEPENDENCY),
        Edge("transcendence_ground", "transcendence", EdgeType.DEPENDENCY),
        Edge("transcendence_ground", "ground", EdgeType.DEPENDENCY),
        Edge("transcendence_ground", "dasein", EdgeType.DEPENDENCY),
        Edge("freedom", "nothing", EdgeType.REFERENCE),
    ]
    return "1929c: On the Essence of Ground", vertices, edges


def work_essence_of_truth():
    """1930/1943: On the Essence of Truth."""
    vertices = [
        Vertex("aletheia", content="unconcealment (aletheia)"),
        Vertex("concealment", content="concealment (Verbergung)"),
        Vertex("errancy", content="errancy (Irre)"),
        Vertex("letting_be", content="letting-be (Seinlassen)"),
        Vertex("openness", content="openness (Offenheit)"),
    ]
    edges = [
        Edge("aletheia", "concealment", EdgeType.NEGATION),
        Edge("aletheia", "truth", EdgeType.SUBLATION),
        Edge("concealment", "truth", EdgeType.DEPENDENCY),
        Edge("errancy", "concealment", EdgeType.DEPENDENCY),
        Edge("errancy", "freedom", EdgeType.DEPENDENCY),
        Edge("letting_be", "freedom", EdgeType.DEPENDENCY),
        Edge("letting_be", "openness", EdgeType.DEPENDENCY),
        Edge("openness", "aletheia", EdgeType.DEPENDENCY),
        Edge("openness", "being", EdgeType.DEPENDENCY),
        Edge("truth", "aletheia", EdgeType.REFERENCE),
    ]
    return "1930/43: On the Essence of Truth", vertices, edges


def work_introduction_metaphysics():
    """1935: Introduction to Metaphysics."""
    vertices = [
        Vertex("physis", content="physis (emerging/arising)"),
        Vertex("logos", content="logos (gathering)"),
        Vertex("noein", content="noein (apprehending)"),
        Vertex("dike", content="dike (fittingness/order)"),
        Vertex("techne", content="techne (knowing/craft)"),
        Vertex("polis", content="polis"),
        Vertex("violence", content="violence (Gewalt-tatigkeit)"),
        Vertex("uncanny", content="the uncanny (Unheimliche)"),
    ]
    edges = [
        Edge("physis", "being", EdgeType.DEPENDENCY),
        Edge("physis", "aletheia", EdgeType.DEPENDENCY),
        Edge("logos", "physis", EdgeType.DEPENDENCY),
        Edge("logos", "noein", EdgeType.DEPENDENCY),
        Edge("noein", "being", EdgeType.DEPENDENCY),
        Edge("dike", "physis", EdgeType.DEPENDENCY),
        Edge("techne", "physis", EdgeType.NEGATION),
        Edge("techne", "dike", EdgeType.NEGATION),
        Edge("polis", "dasein", EdgeType.DEPENDENCY),
        Edge("polis", "dike", EdgeType.REFERENCE),
        Edge("violence", "uncanny", EdgeType.DEPENDENCY),
        Edge("uncanny", "dasein", EdgeType.DEPENDENCY),
        Edge("uncanny", "being_in_the_world", EdgeType.REFERENCE),
    ]
    return "1935: Introduction to Metaphysics", vertices, edges


def work_origin_work_of_art():
    """1935/36: The Origin of the Work of Art."""
    vertices = [
        Vertex("earth", content="earth (Erde)"),
        Vertex("art_work", content="work of art"),
        Vertex("strife", content="strife (Streit) between earth and world"),
        Vertex("setting_up_world", content="setting up a world"),
        Vertex("setting_forth_earth", content="setting forth the earth"),
        Vertex("rift", content="rift (Riss)"),
        Vertex("figure", content="figure/shape (Gestalt)"),
        Vertex("poiesis", content="poiesis (bringing-forth)"),
    ]
    edges = [
        Edge("earth", "world", EdgeType.NEGATION),
        Edge("strife", "earth", EdgeType.DEPENDENCY),
        Edge("strife", "world", EdgeType.DEPENDENCY),
        Edge("art_work", "strife", EdgeType.DEPENDENCY),
        Edge("art_work", "truth", EdgeType.DEPENDENCY),
        Edge("setting_up_world", "world", EdgeType.DEPENDENCY),
        Edge("setting_forth_earth", "earth", EdgeType.DEPENDENCY),
        Edge("setting_up_world", "setting_forth_earth", EdgeType.NEGATION),
        Edge("rift", "strife", EdgeType.DEPENDENCY),
        Edge("figure", "rift", EdgeType.DEPENDENCY),
        Edge("poiesis", "aletheia", EdgeType.DEPENDENCY),
        Edge("poiesis", "techne", EdgeType.SUBLATION),
        Edge("art_work", "poiesis", EdgeType.DEPENDENCY),
    ]
    return "1935/36: The Origin of the Work of Art", vertices, edges


def work_contributions_philosophy():
    """1936-38: Contributions to Philosophy (Beitrage zur Philosophie)."""
    vertices = [
        Vertex("ereignis", content="Ereignis (event of appropriation)"),
        Vertex("last_god", content="the last god (der letzte Gott)"),
        Vertex("grounding", content="grounding (Grundung)"),
        Vertex("leap", content="the leap (der Sprung)"),
        Vertex("interplay", content="interplay (Zuspiel)"),
        Vertex("echo", content="echo (Anklang)"),
        Vertex("future_ones", content="the future ones (die Zukunftigen)"),
        Vertex("abandonment_of_being", content="abandonment of being (Seinsverlassenheit)"),
        Vertex("machination", content="machination (Machenschaft)"),
        Vertex("lived_experience", content="lived experience (Erlebnis)"),
        Vertex("beyng", content="Beyng (Seyn)"),
        Vertex("abyss_ground", content="ab-ground (the abyss as ground)"),
        Vertex("time_space", content="time-space (Zeit-Raum)"),
    ]
    edges = [
        Edge("ereignis", "beyng", EdgeType.DEPENDENCY),
        Edge("ereignis", "dasein", EdgeType.DEPENDENCY),
        Edge("ereignis", "truth", EdgeType.SUBLATION),
        Edge("beyng", "being", EdgeType.SUBLATION),
        Edge("last_god", "ereignis", EdgeType.DEPENDENCY),
        Edge("last_god", "future_ones", EdgeType.DEPENDENCY),
        Edge("grounding", "abyss_ground", EdgeType.DEPENDENCY),
        Edge("grounding", "dasein", EdgeType.DEPENDENCY),
        Edge("grounding", "truth", EdgeType.DEPENDENCY),
        Edge("leap", "grounding", EdgeType.DEPENDENCY),
        Edge("leap", "echo", EdgeType.DEPENDENCY),
        Edge("interplay", "echo", EdgeType.DEPENDENCY),
        Edge("interplay", "being", EdgeType.REFERENCE),
        Edge("echo", "abandonment_of_being", EdgeType.DEPENDENCY),
        Edge("abandonment_of_being", "machination", EdgeType.DEPENDENCY),
        Edge("machination", "lived_experience", EdgeType.DEPENDENCY),
        Edge("machination", "techne", EdgeType.REFERENCE),
        Edge("lived_experience", "inauthenticity", EdgeType.REFERENCE),
        Edge("abyss_ground", "abyss", EdgeType.SUBLATION),
        Edge("abyss_ground", "ground", EdgeType.SUBLATION),
        Edge("future_ones", "dasein", EdgeType.DEPENDENCY),
        Edge("time_space", "temporality", EdgeType.SUBLATION),
        Edge("time_space", "ereignis", EdgeType.DEPENDENCY),
    ]
    return "1936-38: Contributions to Philosophy (Beitrage)", vertices, edges


def work_nietzsche_will_to_power():
    """1936-37: Nietzsche I — Will to Power as Art."""
    vertices = [
        Vertex("will_to_power", content="will to power"),
        Vertex("eternal_return", content="eternal return of the same"),
        Vertex("nihilism", content="nihilism"),
        Vertex("overman", content="overman (Ubermensch)"),
        Vertex("metaphysics_completion", content="completion of metaphysics"),
        Vertex("value_positing", content="value-positing"),
    ]
    edges = [
        Edge("will_to_power", "being", EdgeType.REFERENCE),
        Edge("will_to_power", "value_positing", EdgeType.DEPENDENCY),
        Edge("eternal_return", "will_to_power", EdgeType.DEPENDENCY),
        Edge("eternal_return", "temporality", EdgeType.NEGATION),
        Edge("nihilism", "being", EdgeType.NEGATION),
        Edge("nihilism", "nothing", EdgeType.DEPENDENCY),
        Edge("overman", "will_to_power", EdgeType.DEPENDENCY),
        Edge("overman", "das_man", EdgeType.NEGATION),
        Edge("metaphysics_completion", "nihilism", EdgeType.DEPENDENCY),
        Edge("metaphysics_completion", "will_to_power", EdgeType.DEPENDENCY),
        Edge("value_positing", "machination", EdgeType.REFERENCE),
    ]
    return "1936-37: Nietzsche I (Will to Power as Art)", vertices, edges


def work_nietzsche_eternal_return():
    """1937: Nietzsche II — Eternal Return of the Same."""
    vertices = [
        Vertex("moment", content="the moment (Augenblick)"),
        Vertex("overcoming_metaphysics", content="overcoming metaphysics (Uberwindung)"),
    ]
    edges = [
        Edge("moment", "eternal_return", EdgeType.DEPENDENCY),
        Edge("moment", "ecstasis", EdgeType.REFERENCE),
        Edge("moment", "resoluteness", EdgeType.REFERENCE),
        Edge("overcoming_metaphysics", "metaphysics_completion", EdgeType.DEPENDENCY),
        Edge("overcoming_metaphysics", "ereignis", EdgeType.DEPENDENCY),
        Edge("overcoming_metaphysics", "nihilism", EdgeType.SUBLATION),
    ]
    return "1937: Nietzsche II (Eternal Return of the Same)", vertices, edges


def work_nietzsche_metaphysics():
    """1940-41: Nietzsche III-IV — Nihilism and Metaphysics as History of Being."""
    vertices = [
        Vertex("history_of_being", content="history of Being (Seinsgeschichte)"),
        Vertex("sending", content="sending/dispensation (Geschick)"),
        Vertex("oblivion_of_being", content="oblivion of Being (Seinsvergessenheit)"),
        Vertex("destiny", content="destiny (Geschick)"),
    ]
    edges = [
        Edge("history_of_being", "being", EdgeType.DEPENDENCY),
        Edge("history_of_being", "sending", EdgeType.DEPENDENCY),
        Edge("history_of_being", "metaphysics_completion", EdgeType.DEPENDENCY),
        Edge("sending", "ereignis", EdgeType.DEPENDENCY),
        Edge("sending", "destiny", EdgeType.DEPENDENCY),
        Edge("oblivion_of_being", "ontological_difference", EdgeType.NEGATION),
        Edge("oblivion_of_being", "nihilism", EdgeType.DEPENDENCY),
        Edge("oblivion_of_being", "history_of_being", EdgeType.DEPENDENCY),
        Edge("destiny", "historicity", EdgeType.SUBLATION),
    ]
    return "1940-41: Nietzsche III-IV (Nihilism / History of Being)", vertices, edges


def work_letter_on_humanism():
    """1946: Letter on Humanism."""
    vertices = [
        Vertex("ek_sistence", content="ek-sistence (Ek-sistenz)"),
        Vertex("house_of_being", content="language as house of Being"),
        Vertex("thinking", content="thinking (Denken)"),
        Vertex("language", content="language (Sprache)"),
        Vertex("homelessness", content="homelessness"),
        Vertex("humanism_critique", content="critique of humanism"),
        Vertex("clearing", content="clearing (Lichtung)"),
    ]
    edges = [
        Edge("ek_sistence", "dasein", EdgeType.SUBLATION),
        Edge("ek_sistence", "being", EdgeType.DEPENDENCY),
        Edge("house_of_being", "language", EdgeType.DEPENDENCY),
        Edge("house_of_being", "being", EdgeType.DEPENDENCY),
        Edge("thinking", "being", EdgeType.DEPENDENCY),
        Edge("thinking", "language", EdgeType.DEPENDENCY),
        Edge("language", "discourse", EdgeType.SUBLATION),
        Edge("homelessness", "being_in_the_world", EdgeType.NEGATION),
        Edge("homelessness", "oblivion_of_being", EdgeType.DEPENDENCY),
        Edge("humanism_critique", "ek_sistence", EdgeType.DEPENDENCY),
        Edge("humanism_critique", "das_man", EdgeType.NEGATION),
        Edge("clearing", "aletheia", EdgeType.DEPENDENCY),
        Edge("clearing", "being", EdgeType.DEPENDENCY),
        Edge("clearing", "openness", EdgeType.SUBLATION),
    ]
    return "1946: Letter on Humanism", vertices, edges


def work_question_concerning_technology():
    """1949/53: The Question Concerning Technology."""
    vertices = [
        Vertex("gestell", content="Gestell (enframing)"),
        Vertex("enframing", content="enframing (challenging-forth)"),
        Vertex("standing_reserve", content="standing-reserve (Bestand)"),
        Vertex("challenging", content="challenging (Herausfordern)"),
        Vertex("revealing", content="revealing (Entbergen)"),
        Vertex("saving_power", content="the saving power"),
        Vertex("danger", content="the danger (die Gefahr)"),
    ]
    edges = [
        Edge("gestell", "enframing", EdgeType.DEPENDENCY),
        Edge("gestell", "being", EdgeType.DEPENDENCY),
        Edge("gestell", "machination", EdgeType.SUBLATION),
        Edge("enframing", "challenging", EdgeType.DEPENDENCY),
        Edge("enframing", "revealing", EdgeType.DEPENDENCY),
        Edge("standing_reserve", "enframing", EdgeType.DEPENDENCY),
        Edge("standing_reserve", "present_at_hand", EdgeType.SUBLATION),
        Edge("challenging", "poiesis", EdgeType.NEGATION),
        Edge("revealing", "aletheia", EdgeType.DEPENDENCY),
        Edge("revealing", "poiesis", EdgeType.REFERENCE),
        Edge("saving_power", "danger", EdgeType.DEPENDENCY),
        Edge("saving_power", "poiesis", EdgeType.DEPENDENCY),
        Edge("danger", "gestell", EdgeType.DEPENDENCY),
        Edge("danger", "oblivion_of_being", EdgeType.DEPENDENCY),
    ]
    return "1949/53: The Question Concerning Technology", vertices, edges


def work_building_dwelling_thinking():
    """1951: Building Dwelling Thinking."""
    vertices = [
        Vertex("dwelling", content="dwelling (Wohnen)"),
        Vertex("building", content="building (Bauen)"),
        Vertex("fourfold", content="the fourfold (Geviert)"),
        Vertex("sky", content="sky"),
        Vertex("mortals", content="mortals"),
        Vertex("divinities", content="divinities (die Gottlichen)"),
        Vertex("sparing", content="sparing (Schonen)"),
        Vertex("thing", content="the thing (das Ding)"),
        Vertex("location", content="location (Ort)"),
        Vertex("space_dwelling", content="space as grounded in dwelling"),
    ]
    edges = [
        Edge("dwelling", "being_in_the_world", EdgeType.SUBLATION),
        Edge("dwelling", "fourfold", EdgeType.DEPENDENCY),
        Edge("dwelling", "sparing", EdgeType.DEPENDENCY),
        Edge("building", "dwelling", EdgeType.DEPENDENCY),
        Edge("fourfold", "earth", EdgeType.DEPENDENCY),
        Edge("fourfold", "sky", EdgeType.DEPENDENCY),
        Edge("fourfold", "mortals", EdgeType.DEPENDENCY),
        Edge("fourfold", "divinities", EdgeType.DEPENDENCY),
        Edge("mortals", "being_toward_death", EdgeType.REFERENCE),
        Edge("divinities", "last_god", EdgeType.REFERENCE),
        Edge("sparing", "letting_be", EdgeType.REFERENCE),
        Edge("thing", "fourfold", EdgeType.DEPENDENCY),
        Edge("location", "thing", EdgeType.DEPENDENCY),
        Edge("location", "dwelling", EdgeType.DEPENDENCY),
        Edge("space_dwelling", "dwelling", EdgeType.DEPENDENCY),
        Edge("space_dwelling", "being_in_the_world", EdgeType.NEGATION),
    ]
    return "1951: Building Dwelling Thinking", vertices, edges


def work_what_is_called_thinking():
    """1951-52: What Is Called Thinking?"""
    vertices = [
        Vertex("thinking_calls", content="what calls for thinking"),
        Vertex("withdrawal", content="withdrawal (Entzug)"),
        Vertex("memory", content="memory/thinking (Andenken/Gedachtnis)"),
        Vertex("thanking", content="thanking (Danken)"),
        Vertex("most_thought_provoking", content="the most thought-provoking"),
    ]
    edges = [
        Edge("thinking_calls", "thinking", EdgeType.DEPENDENCY),
        Edge("thinking_calls", "being", EdgeType.DEPENDENCY),
        Edge("withdrawal", "being", EdgeType.DEPENDENCY),
        Edge("withdrawal", "concealment", EdgeType.REFERENCE),
        Edge("withdrawal", "most_thought_provoking", EdgeType.DEPENDENCY),
        Edge("memory", "thinking", EdgeType.DEPENDENCY),
        Edge("memory", "thanking", EdgeType.DEPENDENCY),
        Edge("thanking", "thinking", EdgeType.REFERENCE),
        Edge("most_thought_provoking", "oblivion_of_being", EdgeType.DEPENDENCY),
        Edge("most_thought_provoking", "gestell", EdgeType.REFERENCE),
    ]
    return "1951-52: What Is Called Thinking?", vertices, edges


def work_thing():
    """1950: The Thing (Das Ding)."""
    vertices = [
        Vertex("gathering", content="gathering (Versammlung)"),
        Vertex("nearness", content="nearness (Nahe)"),
        Vertex("distancelessness", content="distancelessness (Abstandlosigkeit)"),
        Vertex("mirroring", content="mirror-play of the fourfold"),
    ]
    edges = [
        Edge("gathering", "thing", EdgeType.DEPENDENCY),
        Edge("gathering", "fourfold", EdgeType.DEPENDENCY),
        Edge("nearness", "thing", EdgeType.DEPENDENCY),
        Edge("nearness", "distancelessness", EdgeType.NEGATION),
        Edge("distancelessness", "gestell", EdgeType.DEPENDENCY),
        Edge("mirroring", "fourfold", EdgeType.DEPENDENCY),
        Edge("mirroring", "ereignis", EdgeType.REFERENCE),
        Edge("thing", "gathering", EdgeType.REFERENCE),
    ]
    return "1950: The Thing", vertices, edges


def work_identity_difference():
    """1957: Identity and Difference."""
    vertices = [
        Vertex("identity", content="identity (Identitat)"),
        Vertex("difference", content="difference (Differenz)"),
        Vertex("belonging_together", content="belonging-together (Zusammengehoren)"),
        Vertex("onto_theo_logic", content="onto-theo-logical constitution of metaphysics"),
        Vertex("step_back", content="the step back (Schritt zuruck)"),
    ]
    edges = [
        Edge("identity", "difference", EdgeType.NEGATION),
        Edge("identity", "belonging_together", EdgeType.DEPENDENCY),
        Edge("belonging_together", "ereignis", EdgeType.DEPENDENCY),
        Edge("belonging_together", "being", EdgeType.DEPENDENCY),
        Edge("belonging_together", "dasein", EdgeType.DEPENDENCY),
        Edge("difference", "ontological_difference", EdgeType.SUBLATION),
        Edge("onto_theo_logic", "metaphysics_completion", EdgeType.DEPENDENCY),
        Edge("onto_theo_logic", "ground", EdgeType.DEPENDENCY),
        Edge("step_back", "onto_theo_logic", EdgeType.NEGATION),
        Edge("step_back", "overcoming_metaphysics", EdgeType.DEPENDENCY),
        Edge("step_back", "ereignis", EdgeType.DEPENDENCY),
    ]
    return "1957: Identity and Difference", vertices, edges


def work_principle_of_reason():
    """1955-56: The Principle of Reason (Der Satz vom Grund)."""
    vertices = [
        Vertex("principle_reason", content="the principle of reason (Satz vom Grund)"),
        Vertex("being_and_ground", content="being and ground (Sein und Grund)"),
        Vertex("groundlessness", content="groundlessness of Being"),
        Vertex("play", content="play (Spiel)"),
    ]
    edges = [
        Edge("principle_reason", "ground", EdgeType.DEPENDENCY),
        Edge("principle_reason", "being", EdgeType.DEPENDENCY),
        Edge("being_and_ground", "being", EdgeType.DEPENDENCY),
        Edge("being_and_ground", "ground", EdgeType.DEPENDENCY),
        Edge("being_and_ground", "ontological_difference", EdgeType.REFERENCE),
        Edge("groundlessness", "abyss", EdgeType.DEPENDENCY),
        Edge("groundlessness", "being_and_ground", EdgeType.NEGATION),
        Edge("play", "groundlessness", EdgeType.DEPENDENCY),
        Edge("play", "ereignis", EdgeType.REFERENCE),
    ]
    return "1955-56: The Principle of Reason", vertices, edges


def work_on_the_way_to_language():
    """1950-59: On the Way to Language."""
    vertices = [
        Vertex("saying", content="Saying (Sage)"),
        Vertex("showing", content="showing (Zeigen)"),
        Vertex("language_speaks", content="language speaks (die Sprache spricht)"),
        Vertex("peal_of_stillness", content="the peal of stillness (Gelaut der Stille)"),
        Vertex("way_to_language", content="the way to language"),
        Vertex("hermeneutic_circle", content="hermeneutic circle"),
    ]
    edges = [
        Edge("saying", "language", EdgeType.DEPENDENCY),
        Edge("saying", "showing", EdgeType.DEPENDENCY),
        Edge("saying", "ereignis", EdgeType.DEPENDENCY),
        Edge("showing", "aletheia", EdgeType.REFERENCE),
        Edge("language_speaks", "language", EdgeType.DEPENDENCY),
        Edge("language_speaks", "dasein", EdgeType.NEGATION),
        Edge("peal_of_stillness", "language_speaks", EdgeType.DEPENDENCY),
        Edge("peal_of_stillness", "saying", EdgeType.DEPENDENCY),
        Edge("way_to_language", "saying", EdgeType.DEPENDENCY),
        Edge("way_to_language", "thinking", EdgeType.DEPENDENCY),
        Edge("hermeneutic_circle", "understanding", EdgeType.DEPENDENCY),
        Edge("hermeneutic_circle", "interpretation", EdgeType.DEPENDENCY),
    ]
    return "1950-59: On the Way to Language", vertices, edges


def work_time_and_being():
    """1962: Time and Being (Zeit und Sein)."""
    vertices = [
        Vertex("es_gibt", content="It gives / there is (Es gibt)"),
        Vertex("giving", content="giving (Geben)"),
        Vertex("extending", content="extending (Reichen)"),
        Vertex("presence_absence_time", content="presence-absence in time"),
        Vertex("propriation", content="propriation (Ereignis as proper)"),
    ]
    edges = [
        Edge("es_gibt", "being", EdgeType.DEPENDENCY),
        Edge("es_gibt", "temporality", EdgeType.DEPENDENCY),
        Edge("es_gibt", "ereignis", EdgeType.DEPENDENCY),
        Edge("giving", "es_gibt", EdgeType.DEPENDENCY),
        Edge("giving", "sending", EdgeType.DEPENDENCY),
        Edge("extending", "temporality", EdgeType.DEPENDENCY),
        Edge("extending", "giving", EdgeType.DEPENDENCY),
        Edge("presence_absence_time", "presence", EdgeType.DEPENDENCY),
        Edge("presence_absence_time", "absence", EdgeType.DEPENDENCY),
        Edge("presence_absence_time", "ecstasis", EdgeType.REFERENCE),
        Edge("propriation", "ereignis", EdgeType.DEPENDENCY),
        Edge("propriation", "belonging_together", EdgeType.DEPENDENCY),
    ]
    return "1962: Time and Being", vertices, edges


def work_end_of_philosophy():
    """1964: The End of Philosophy and the Task of Thinking."""
    vertices = [
        Vertex("end_philosophy", content="the end of philosophy"),
        Vertex("task_thinking", content="the task of thinking"),
        Vertex("lichtung", content="Lichtung (clearing/lighting)"),
        Vertex("sciences_dissolution", content="dissolution of philosophy into sciences"),
    ]
    edges = [
        Edge("end_philosophy", "metaphysics_completion", EdgeType.DEPENDENCY),
        Edge("end_philosophy", "sciences_dissolution", EdgeType.DEPENDENCY),
        Edge("task_thinking", "end_philosophy", EdgeType.DEPENDENCY),
        Edge("task_thinking", "lichtung", EdgeType.DEPENDENCY),
        Edge("task_thinking", "thinking", EdgeType.DEPENDENCY),
        Edge("lichtung", "clearing", EdgeType.SUBLATION),
        Edge("lichtung", "aletheia", EdgeType.DEPENDENCY),
        Edge("lichtung", "concealment", EdgeType.DEPENDENCY),
        Edge("sciences_dissolution", "gestell", EdgeType.DEPENDENCY),
        Edge("sciences_dissolution", "overcoming_metaphysics", EdgeType.NEGATION),
    ]
    return "1964: The End of Philosophy and the Task of Thinking", vertices, edges


def work_zollikon_seminars():
    """1959-69: Zollikon Seminars."""
    vertices = [
        Vertex("bodily_being", content="bodily being (Leiblichkeit)"),
        Vertex("dasein_analysis_med", content="Daseinsanalysis (medical)"),
        Vertex("illness", content="illness as mode of Dasein"),
        Vertex("spatiality_body", content="spatiality of the lived body"),
        Vertex("psychosomatic", content="psychosomatic unity"),
    ]
    edges = [
        Edge("bodily_being", "dasein", EdgeType.DEPENDENCY),
        Edge("bodily_being", "being_in_the_world", EdgeType.DEPENDENCY),
        Edge("dasein_analysis_med", "dasein", EdgeType.DEPENDENCY),
        Edge("dasein_analysis_med", "care", EdgeType.REFERENCE),
        Edge("illness", "dasein", EdgeType.DEPENDENCY),
        Edge("illness", "being_toward_death", EdgeType.REFERENCE),
        Edge("spatiality_body", "bodily_being", EdgeType.DEPENDENCY),
        Edge("spatiality_body", "being_in_the_world", EdgeType.REFERENCE),
        Edge("psychosomatic", "bodily_being", EdgeType.DEPENDENCY),
        Edge("psychosomatic", "res_cogitans", EdgeType.NEGATION),
    ]
    return "1959-69: Zollikon Seminars", vertices, edges


def work_discourse_on_thinking():
    """1944-45: Discourse on Thinking (Gelassenheit)."""
    vertices = [
        Vertex("gelassenheit", content="releasement (Gelassenheit)"),
        Vertex("meditative_thinking", content="meditative thinking"),
        Vertex("calculative_thinking", content="calculative thinking"),
        Vertex("openness_mystery", content="openness to the mystery"),
        Vertex("region", content="the region (die Gegend/Gegnet)"),
    ]
    edges = [
        Edge("gelassenheit", "letting_be", EdgeType.SUBLATION),
        Edge("gelassenheit", "calculative_thinking", EdgeType.NEGATION),
        Edge("meditative_thinking", "gelassenheit", EdgeType.DEPENDENCY),
        Edge("meditative_thinking", "thinking", EdgeType.DEPENDENCY),
        Edge("calculative_thinking", "gestell", EdgeType.DEPENDENCY),
        Edge("calculative_thinking", "machination", EdgeType.REFERENCE),
        Edge("openness_mystery", "gelassenheit", EdgeType.DEPENDENCY),
        Edge("openness_mystery", "concealment", EdgeType.DEPENDENCY),
        Edge("region", "openness", EdgeType.SUBLATION),
        Edge("region", "gelassenheit", EdgeType.DEPENDENCY),
    ]
    return "1944-45: Discourse on Thinking (Gelassenheit)", vertices, edges


def work_parmenides():
    """1942-43: Parmenides (lecture course)."""
    vertices = [
        Vertex("aletheia_goddess", content="aletheia as goddess"),
        Vertex("lethe", content="Lethe (oblivion)"),
        Vertex("mythos", content="mythos"),
        Vertex("roman_verum", content="Roman verum/veritas"),
        Vertex("greek_aletheia_orig", content="original Greek aletheia"),
    ]
    edges = [
        Edge("aletheia_goddess", "aletheia", EdgeType.DEPENDENCY),
        Edge("aletheia_goddess", "truth", EdgeType.DEPENDENCY),
        Edge("lethe", "concealment", EdgeType.DEPENDENCY),
        Edge("lethe", "oblivion_of_being", EdgeType.DEPENDENCY),
        Edge("mythos", "logos", EdgeType.NEGATION),
        Edge("mythos", "aletheia_goddess", EdgeType.DEPENDENCY),
        Edge("roman_verum", "aletheia", EdgeType.NEGATION),
        Edge("roman_verum", "truth", EdgeType.REFERENCE),
        Edge("greek_aletheia_orig", "physis", EdgeType.DEPENDENCY),
        Edge("greek_aletheia_orig", "aletheia", EdgeType.DEPENDENCY),
    ]
    return "1942-43: Parmenides", vertices, edges


def work_four_seminars():
    """1966-73: Four Seminars (Le Thor, Zahringen)."""
    vertices = [
        Vertex("topology_of_being", content="topology of Being (Topologie des Seyns)"),
        Vertex("place_of_being", content="place of Being (Ort des Seins)"),
        Vertex("phenomenology_inconspicuous", content="phenomenology of the inconspicuous"),
    ]
    edges = [
        Edge("topology_of_being", "lichtung", EdgeType.DEPENDENCY),
        Edge("topology_of_being", "being", EdgeType.DEPENDENCY),
        Edge("topology_of_being", "ereignis", EdgeType.DEPENDENCY),
        Edge("place_of_being", "topology_of_being", EdgeType.DEPENDENCY),
        Edge("place_of_being", "location", EdgeType.REFERENCE),
        Edge("phenomenology_inconspicuous", "phenomenology_method", EdgeType.SUBLATION),
        Edge("phenomenology_inconspicuous", "lichtung", EdgeType.DEPENDENCY),
        Edge("phenomenology_inconspicuous", "gelassenheit", EdgeType.REFERENCE),
    ]
    return "1966-73: Four Seminars (Le Thor, Zahringen)", vertices, edges


# ---------------------------------------------------------------------------
# ALL_WORKS in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_history_concept_of_time,          # 1925
    work_basic_problems_phenomenology,     # 1927a
    work_being_and_time,                   # 1927b
    work_kant_problem_metaphysics,         # 1929a
    work_what_is_metaphysics,              # 1929b
    work_essence_of_ground,               # 1929c
    work_essence_of_truth,                 # 1930/43
    work_introduction_metaphysics,         # 1935
    work_origin_work_of_art,               # 1935/36
    work_contributions_philosophy,         # 1936-38
    work_nietzsche_will_to_power,          # 1936-37
    work_nietzsche_eternal_return,         # 1937
    work_nietzsche_metaphysics,            # 1940-41
    work_parmenides,                       # 1942-43
    work_discourse_on_thinking,            # 1944-45
    work_letter_on_humanism,               # 1946
    work_question_concerning_technology,   # 1949/53
    work_thing,                            # 1950
    work_building_dwelling_thinking,       # 1951
    work_what_is_called_thinking,          # 1951-52
    work_principle_of_reason,              # 1955-56
    work_identity_difference,              # 1957
    work_on_the_way_to_language,           # 1950-59
    work_zollikon_seminars,                # 1959-69
    work_time_and_being,                   # 1962
    work_end_of_philosophy,                # 1964
    work_four_seminars,                    # 1966-73
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("MARTIN HEIDEGGER — COMPLETE WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "heidegger_complete_works_incremental",
        "source": "Martin Heidegger — complete works, 27 entries (1925-1973)",
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

    output_path = "experiment_heidegger.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
