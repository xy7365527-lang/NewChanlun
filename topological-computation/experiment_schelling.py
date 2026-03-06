"""F.W.J. Schelling — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Schelling's philosophy
from early Fichtean idealism through Naturphilosophie, identity philosophy,
the freedom essay, Ages of the World, to the late positive philosophy
of mythology and revelation — as an incremental topological growth process.
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
#   nature, spirit, absolute, identity, freedom, evil, ground, existence,
#   potency, mythology, revelation, unconscious, art, intellectual_intuition,
#   subject, object, self_consciousness, ego, not_ego, unconditioned,
#   productive_intuition, world_soul, organism, polarity, magnetism,
#   electricity, chemical_process, life, gravity, light,
#   indifference_point, quantitative_difference, real, ideal,
#   will, love, dark_ground, longing, understanding,
#   necessity, contingency, god, creation, fall,
#   positive_philosophy, negative_philosophy, reason, experience,
#   tautegory, symbol, potency_A, potency_B, potency_AB,
#   theogony, cosmogony, monotheism, polytheism, trinity
# ---------------------------------------------------------------------------


def work_uber_die_moglichkeit():
    """1794: On the Possibility of a Form of Philosophy in General."""
    vertices = [
        Vertex("unconditioned", content="the unconditioned (das Unbedingte)"),
        Vertex("ego", content="ego (Ich) — absolute subject"),
        Vertex("not_ego", content="not-ego (Nicht-Ich)"),
        Vertex("subject", content="subject"),
        Vertex("object", content="object"),
        Vertex("form_philosophy", content="form of philosophy as such"),
        Vertex("first_principle", content="first principle (Grundsatz)"),
        Vertex("conditioned", content="the conditioned"),
        Vertex("identity_principle", content="A = A, principle of identity"),
    ]
    edges = [
        Edge("unconditioned", "ego", EdgeType.DEPENDENCY),
        Edge("unconditioned", "conditioned", EdgeType.NEGATION),
        Edge("ego", "not_ego", EdgeType.NEGATION),
        Edge("subject", "object", EdgeType.NEGATION),
        Edge("first_principle", "unconditioned", EdgeType.DEPENDENCY),
        Edge("form_philosophy", "first_principle", EdgeType.DEPENDENCY),
        Edge("identity_principle", "ego", EdgeType.DEPENDENCY),
        Edge("identity_principle", "unconditioned", EdgeType.REFERENCE),
        Edge("conditioned", "first_principle", EdgeType.DEPENDENCY),
    ]
    return "1794: On the Possibility of a Form of Philosophy in General", vertices, edges


def work_vom_ich():
    """1795: Of the Ego as Principle of Philosophy."""
    vertices = [
        Vertex("absolute_ego", content="absolute ego — beyond subject/object"),
        Vertex("intellectual_intuition", content="intellectual intuition (intellektuelle Anschauung)"),
        Vertex("self_consciousness", content="self-consciousness"),
        Vertex("thing_in_itself", content="thing-in-itself (rejected)"),
        Vertex("spinoza_substance", content="Spinoza's substance (reinterpreted)"),
        Vertex("dogmatism", content="dogmatism"),
        Vertex("criticism", content="criticism (Kantian/Fichtean)"),
        Vertex("finite_ego", content="finite ego"),
    ]
    edges = [
        Edge("absolute_ego", "ego", EdgeType.SUBLATION),
        Edge("absolute_ego", "unconditioned", EdgeType.DEPENDENCY),
        Edge("intellectual_intuition", "absolute_ego", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "absolute_ego", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "finite_ego", EdgeType.REFERENCE),
        Edge("thing_in_itself", "absolute_ego", EdgeType.NEGATION),
        Edge("spinoza_substance", "absolute_ego", EdgeType.REFERENCE),
        Edge("dogmatism", "thing_in_itself", EdgeType.DEPENDENCY),
        Edge("dogmatism", "criticism", EdgeType.NEGATION),
        Edge("criticism", "ego", EdgeType.DEPENDENCY),
        Edge("finite_ego", "absolute_ego", EdgeType.DEPENDENCY),
        Edge("finite_ego", "not_ego", EdgeType.NEGATION),
    ]
    return "1795: Of the Ego as Principle of Philosophy", vertices, edges


def work_philosophical_letters():
    """1795: Philosophical Letters on Dogmatism and Criticism."""
    vertices = [
        Vertex("practical_decision", content="practical decision between systems"),
        Vertex("tragedy_philosophy", content="tragedy in philosophy"),
        Vertex("moral_freedom_early", content="moral freedom (early)"),
        Vertex("infinite_striving", content="infinite striving (Fichtean)"),
        Vertex("aesthetic_resolution", content="aesthetic resolution of dogmatism/criticism"),
    ]
    edges = [
        Edge("practical_decision", "dogmatism", EdgeType.REFERENCE),
        Edge("practical_decision", "criticism", EdgeType.REFERENCE),
        Edge("tragedy_philosophy", "practical_decision", EdgeType.DEPENDENCY),
        Edge("moral_freedom_early", "infinite_striving", EdgeType.DEPENDENCY),
        Edge("infinite_striving", "absolute_ego", EdgeType.DEPENDENCY),
        Edge("aesthetic_resolution", "dogmatism", EdgeType.SUBLATION),
        Edge("aesthetic_resolution", "criticism", EdgeType.SUBLATION),
        Edge("aesthetic_resolution", "intellectual_intuition", EdgeType.REFERENCE),
        Edge("practical_decision", "moral_freedom_early", EdgeType.DEPENDENCY),
    ]
    return "1795: Philosophical Letters on Dogmatism and Criticism", vertices, edges


def work_ideas_nature():
    """1797: Ideas for a Philosophy of Nature."""
    vertices = [
        Vertex("nature", content="nature as visible spirit"),
        Vertex("spirit", content="spirit as invisible nature"),
        Vertex("Naturphilosophie", content="philosophy of nature (Naturphilosophie)"),
        Vertex("productive_intuition", content="productive intuition of nature"),
        Vertex("polarity", content="polarity (Polarität)"),
        Vertex("magnetism", content="magnetism — first potency of nature"),
        Vertex("electricity", content="electricity — second potency"),
        Vertex("chemical_process", content="chemical process — third potency"),
        Vertex("organism", content="organism (living nature)"),
        Vertex("matter_force", content="matter as product of opposing forces"),
        Vertex("dynamic_process", content="dynamic process"),
        Vertex("nature_as_subject", content="nature as subject, self-producing"),
    ]
    edges = [
        Edge("nature", "spirit", EdgeType.NEGATION),
        Edge("Naturphilosophie", "nature", EdgeType.DEPENDENCY),
        Edge("Naturphilosophie", "criticism", EdgeType.SUBLATION),
        Edge("productive_intuition", "intellectual_intuition", EdgeType.REFERENCE),
        Edge("productive_intuition", "nature", EdgeType.DEPENDENCY),
        Edge("polarity", "nature", EdgeType.DEPENDENCY),
        Edge("magnetism", "polarity", EdgeType.DEPENDENCY),
        Edge("electricity", "polarity", EdgeType.DEPENDENCY),
        Edge("chemical_process", "polarity", EdgeType.DEPENDENCY),
        Edge("magnetism", "electricity", EdgeType.REFERENCE),
        Edge("electricity", "chemical_process", EdgeType.REFERENCE),
        Edge("organism", "chemical_process", EdgeType.SUBLATION),
        Edge("organism", "nature_as_subject", EdgeType.DEPENDENCY),
        Edge("matter_force", "polarity", EdgeType.DEPENDENCY),
        Edge("dynamic_process", "matter_force", EdgeType.DEPENDENCY),
        Edge("nature_as_subject", "nature", EdgeType.DEPENDENCY),
        Edge("nature_as_subject", "subject", EdgeType.REFERENCE),
    ]
    return "1797: Ideas for a Philosophy of Nature", vertices, edges


def work_world_soul():
    """1798: On the World Soul."""
    vertices = [
        Vertex("world_soul", content="world soul (Weltseele)"),
        Vertex("universal_organism", content="universal organism"),
        Vertex("duality_forces", content="duality of forces in nature"),
        Vertex("gravity", content="gravity — universal attraction"),
        Vertex("light", content="light — universal expansion"),
        Vertex("life", content="life as process"),
        Vertex("organic_inorganic", content="continuum organic-inorganic"),
        Vertex("universal_medium", content="universal medium (aether)"),
    ]
    edges = [
        Edge("world_soul", "nature", EdgeType.DEPENDENCY),
        Edge("world_soul", "organism", EdgeType.SUBLATION),
        Edge("world_soul", "universal_organism", EdgeType.DEPENDENCY),
        Edge("universal_organism", "organism", EdgeType.DEPENDENCY),
        Edge("duality_forces", "polarity", EdgeType.DEPENDENCY),
        Edge("gravity", "light", EdgeType.NEGATION),
        Edge("gravity", "duality_forces", EdgeType.DEPENDENCY),
        Edge("light", "duality_forces", EdgeType.DEPENDENCY),
        Edge("life", "organism", EdgeType.DEPENDENCY),
        Edge("life", "duality_forces", EdgeType.DEPENDENCY),
        Edge("organic_inorganic", "organism", EdgeType.REFERENCE),
        Edge("organic_inorganic", "matter_force", EdgeType.REFERENCE),
        Edge("universal_medium", "world_soul", EdgeType.DEPENDENCY),
        Edge("universal_medium", "light", EdgeType.REFERENCE),
    ]
    return "1798: On the World Soul", vertices, edges


def work_first_outline():
    """1799: First Outline of a System of the Philosophy of Nature."""
    vertices = [
        Vertex("productivity", content="productivity (Produktivität) — nature's infinite activity"),
        Vertex("product", content="product (Produkt) — arrested productivity"),
        Vertex("inhibition", content="inhibition (Hemmung) — what arrests productivity"),
        Vertex("potency_levels", content="potencies (Potenzen) of nature"),
        Vertex("construction", content="construction of matter"),
        Vertex("infinite_activity_nature", content="infinite activity of nature"),
        Vertex("natura_naturans", content="natura naturans"),
        Vertex("natura_naturata", content="natura naturata"),
        Vertex("reproduction", content="reproduction — highest organic potency"),
        Vertex("sensibility", content="sensibility"),
        Vertex("irritability", content="irritability"),
    ]
    edges = [
        Edge("productivity", "product", EdgeType.NEGATION),
        Edge("inhibition", "productivity", EdgeType.NEGATION),
        Edge("product", "inhibition", EdgeType.DEPENDENCY),
        Edge("potency_levels", "polarity", EdgeType.SUBLATION),
        Edge("potency_levels", "magnetism", EdgeType.REFERENCE),
        Edge("potency_levels", "electricity", EdgeType.REFERENCE),
        Edge("potency_levels", "chemical_process", EdgeType.REFERENCE),
        Edge("construction", "matter_force", EdgeType.DEPENDENCY),
        Edge("construction", "potency_levels", EdgeType.DEPENDENCY),
        Edge("infinite_activity_nature", "productivity", EdgeType.DEPENDENCY),
        Edge("natura_naturans", "productivity", EdgeType.DEPENDENCY),
        Edge("natura_naturata", "product", EdgeType.DEPENDENCY),
        Edge("natura_naturans", "natura_naturata", EdgeType.NEGATION),
        Edge("reproduction", "organism", EdgeType.DEPENDENCY),
        Edge("sensibility", "irritability", EdgeType.NEGATION),
        Edge("reproduction", "sensibility", EdgeType.SUBLATION),
        Edge("infinite_activity_nature", "nature_as_subject", EdgeType.DEPENDENCY),
    ]
    return "1799: First Outline of a System of the Philosophy of Nature", vertices, edges


def work_system_transcendental_idealism():
    """1800: System of Transcendental Idealism."""
    vertices = [
        Vertex("transcendental_idealism", content="system of transcendental idealism"),
        Vertex("theoretical_philosophy", content="theoretical philosophy — necessity"),
        Vertex("practical_philosophy", content="practical philosophy — freedom"),
        Vertex("art", content="art as organon of philosophy"),
        Vertex("aesthetic_intuition", content="aesthetic intuition — objective intellectual intuition"),
        Vertex("unconscious", content="unconscious productivity"),
        Vertex("history", content="history as progressive revelation"),
        Vertex("teleology", content="teleology — purposiveness in nature"),
        Vertex("parallel_nature_spirit", content="parallel construction: nature/spirit"),
        Vertex("absolute_identity_early", content="absolute identity of subject/object (early)"),
        Vertex("genius", content="genius — conscious + unconscious"),
        Vertex("mythology_early", content="new mythology (early call)"),
    ]
    edges = [
        Edge("transcendental_idealism", "Naturphilosophie", EdgeType.REFERENCE),
        Edge("transcendental_idealism", "intellectual_intuition", EdgeType.DEPENDENCY),
        Edge("theoretical_philosophy", "practical_philosophy", EdgeType.NEGATION),
        Edge("theoretical_philosophy", "nature", EdgeType.REFERENCE),
        Edge("practical_philosophy", "spirit", EdgeType.REFERENCE),
        Edge("art", "aesthetic_intuition", EdgeType.DEPENDENCY),
        Edge("art", "transcendental_idealism", EdgeType.SUBLATION),
        Edge("aesthetic_intuition", "intellectual_intuition", EdgeType.SUBLATION),
        Edge("unconscious", "productivity", EdgeType.REFERENCE),
        Edge("unconscious", "nature", EdgeType.REFERENCE),
        Edge("genius", "unconscious", EdgeType.DEPENDENCY),
        Edge("genius", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("history", "practical_philosophy", EdgeType.DEPENDENCY),
        Edge("history", "unconscious", EdgeType.DEPENDENCY),
        Edge("teleology", "organism", EdgeType.REFERENCE),
        Edge("teleology", "nature_as_subject", EdgeType.DEPENDENCY),
        Edge("parallel_nature_spirit", "nature", EdgeType.REFERENCE),
        Edge("parallel_nature_spirit", "spirit", EdgeType.REFERENCE),
        Edge("parallel_nature_spirit", "transcendental_idealism", EdgeType.DEPENDENCY),
        Edge("absolute_identity_early", "subject", EdgeType.SUBLATION),
        Edge("absolute_identity_early", "object", EdgeType.SUBLATION),
        Edge("mythology_early", "art", EdgeType.DEPENDENCY),
        Edge("mythology_early", "history", EdgeType.REFERENCE),
    ]
    return "1800: System of Transcendental Idealism", vertices, edges


def work_bruno():
    """1802: Bruno, or On the Natural and Divine Principle of Things."""
    vertices = [
        Vertex("absolute", content="the absolute — total indifference of real/ideal"),
        Vertex("identity", content="identity of identity and difference"),
        Vertex("indifference_point", content="point of indifference (Indifferenzpunkt)"),
        Vertex("real", content="the real (nature pole)"),
        Vertex("ideal", content="the ideal (spirit pole)"),
        Vertex("quantitative_difference", content="quantitative difference within identity"),
        Vertex("eternal_forms", content="eternal forms / ideas in the absolute"),
        Vertex("finitude", content="finitude as falling from identity"),
        Vertex("copula", content="copula — bond of real and ideal"),
    ]
    edges = [
        Edge("absolute", "absolute_identity_early", EdgeType.SUBLATION),
        Edge("absolute", "indifference_point", EdgeType.DEPENDENCY),
        Edge("identity", "absolute", EdgeType.DEPENDENCY),
        Edge("identity", "subject", EdgeType.SUBLATION),
        Edge("identity", "object", EdgeType.SUBLATION),
        Edge("indifference_point", "real", EdgeType.DEPENDENCY),
        Edge("indifference_point", "ideal", EdgeType.DEPENDENCY),
        Edge("real", "ideal", EdgeType.NEGATION),
        Edge("real", "nature", EdgeType.REFERENCE),
        Edge("ideal", "spirit", EdgeType.REFERENCE),
        Edge("quantitative_difference", "identity", EdgeType.DEPENDENCY),
        Edge("quantitative_difference", "real", EdgeType.REFERENCE),
        Edge("quantitative_difference", "ideal", EdgeType.REFERENCE),
        Edge("eternal_forms", "absolute", EdgeType.DEPENDENCY),
        Edge("finitude", "absolute", EdgeType.NEGATION),
        Edge("finitude", "quantitative_difference", EdgeType.DEPENDENCY),
        Edge("copula", "identity", EdgeType.DEPENDENCY),
        Edge("copula", "real", EdgeType.REFERENCE),
        Edge("copula", "ideal", EdgeType.REFERENCE),
    ]
    return "1802: Bruno, or On the Natural and Divine Principle of Things", vertices, edges


def work_further_presentations():
    """1802: Further Presentations from the System of Philosophy."""
    vertices = [
        Vertex("Identitaetsphilosophie", content="identity philosophy (Identitätsphilosophie)"),
        Vertex("reason_as_absolute", content="reason as absolute — totale Indifferenz"),
        Vertex("potency_A", content="potency A — real/nature"),
        Vertex("potency_B", content="potency B — ideal/spirit"),
        Vertex("potency_AB", content="potency A/B — reason/absolute"),
        Vertex("intellectual_intuition_obj", content="intellectual intuition as objective (identity form)"),
        Vertex("absolute_knowing", content="absolute knowing"),
    ]
    edges = [
        Edge("Identitaetsphilosophie", "absolute", EdgeType.DEPENDENCY),
        Edge("Identitaetsphilosophie", "identity", EdgeType.DEPENDENCY),
        Edge("reason_as_absolute", "absolute", EdgeType.DEPENDENCY),
        Edge("reason_as_absolute", "intellectual_intuition", EdgeType.REFERENCE),
        Edge("potency_A", "real", EdgeType.DEPENDENCY),
        Edge("potency_A", "nature", EdgeType.REFERENCE),
        Edge("potency_B", "ideal", EdgeType.DEPENDENCY),
        Edge("potency_B", "spirit", EdgeType.REFERENCE),
        Edge("potency_AB", "potency_A", EdgeType.SUBLATION),
        Edge("potency_AB", "potency_B", EdgeType.SUBLATION),
        Edge("potency_AB", "absolute", EdgeType.DEPENDENCY),
        Edge("intellectual_intuition_obj", "intellectual_intuition", EdgeType.SUBLATION),
        Edge("intellectual_intuition_obj", "identity", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "reason_as_absolute", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "intellectual_intuition_obj", EdgeType.DEPENDENCY),
    ]
    return "1802: Further Presentations from the System of Philosophy", vertices, edges


def work_philosophy_of_art():
    """1802-03: Philosophy of Art (lectures, pub. 1859)."""
    vertices = [
        Vertex("philosophy_of_art", content="philosophy of art — absolute in art-form"),
        Vertex("mythology_as_material", content="mythology as material of art"),
        Vertex("symbol", content="symbol — universal in particular"),
        Vertex("allegory", content="allegory — particular for universal"),
        Vertex("schema", content="schema — universal for particular"),
        Vertex("plastic_arts", content="plastic/visual arts — real series"),
        Vertex("verbal_arts", content="verbal arts — ideal series"),
        Vertex("music", content="music — rhythm, first potency of verbal"),
        Vertex("poetry", content="poetry — highest potency of verbal arts"),
        Vertex("greek_art_ideal", content="Greek art as ideal of indifference"),
    ]
    edges = [
        Edge("philosophy_of_art", "art", EdgeType.SUBLATION),
        Edge("philosophy_of_art", "Identitaetsphilosophie", EdgeType.DEPENDENCY),
        Edge("mythology_as_material", "mythology_early", EdgeType.SUBLATION),
        Edge("mythology_as_material", "philosophy_of_art", EdgeType.DEPENDENCY),
        Edge("symbol", "allegory", EdgeType.NEGATION),
        Edge("symbol", "schema", EdgeType.NEGATION),
        Edge("symbol", "identity", EdgeType.REFERENCE),
        Edge("allegory", "schema", EdgeType.NEGATION),
        Edge("plastic_arts", "verbal_arts", EdgeType.NEGATION),
        Edge("plastic_arts", "real", EdgeType.REFERENCE),
        Edge("verbal_arts", "ideal", EdgeType.REFERENCE),
        Edge("music", "verbal_arts", EdgeType.DEPENDENCY),
        Edge("poetry", "verbal_arts", EdgeType.DEPENDENCY),
        Edge("poetry", "music", EdgeType.SUBLATION),
        Edge("greek_art_ideal", "indifference_point", EdgeType.REFERENCE),
        Edge("greek_art_ideal", "mythology_as_material", EdgeType.DEPENDENCY),
        Edge("philosophy_of_art", "potency_A", EdgeType.REFERENCE),
        Edge("philosophy_of_art", "potency_B", EdgeType.REFERENCE),
    ]
    return "1802-03: Philosophy of Art", vertices, edges


def work_method_academic_study():
    """1803: On the Method of Academic Study (Lectures)."""
    vertices = [
        Vertex("method_academic", content="method of academic study"),
        Vertex("organic_totality_knowledge", content="organic totality of knowledge"),
        Vertex("living_knowledge", content="living knowledge vs dead learning"),
        Vertex("theology_philosophy_unity", content="unity of theology and philosophy"),
        Vertex("historical_construction", content="historical construction of knowledge"),
    ]
    edges = [
        Edge("method_academic", "Identitaetsphilosophie", EdgeType.DEPENDENCY),
        Edge("organic_totality_knowledge", "absolute", EdgeType.REFERENCE),
        Edge("organic_totality_knowledge", "method_academic", EdgeType.DEPENDENCY),
        Edge("living_knowledge", "intellectual_intuition", EdgeType.REFERENCE),
        Edge("living_knowledge", "organic_totality_knowledge", EdgeType.DEPENDENCY),
        Edge("theology_philosophy_unity", "absolute", EdgeType.DEPENDENCY),
        Edge("theology_philosophy_unity", "identity", EdgeType.REFERENCE),
        Edge("historical_construction", "history", EdgeType.REFERENCE),
        Edge("historical_construction", "method_academic", EdgeType.DEPENDENCY),
    ]
    return "1803: On the Method of Academic Study", vertices, edges


def work_philosophy_and_religion():
    """1804: Philosophy and Religion."""
    vertices = [
        Vertex("fall_from_absolute", content="fall (Abfall) from the absolute"),
        Vertex("finite_world_origin", content="origin of the finite world"),
        Vertex("return_to_absolute", content="return (Rückkehr) to the absolute"),
        Vertex("leap_irrational", content="irrational leap — finitude inexplicable"),
        Vertex("immortality_soul", content="immortality of the soul"),
        Vertex("freedom_early", content="freedom (early — tied to the fall)"),
    ]
    edges = [
        Edge("fall_from_absolute", "absolute", EdgeType.NEGATION),
        Edge("fall_from_absolute", "finitude", EdgeType.DEPENDENCY),
        Edge("finite_world_origin", "fall_from_absolute", EdgeType.DEPENDENCY),
        Edge("return_to_absolute", "fall_from_absolute", EdgeType.NEGATION),
        Edge("return_to_absolute", "absolute", EdgeType.DEPENDENCY),
        Edge("leap_irrational", "fall_from_absolute", EdgeType.DEPENDENCY),
        Edge("leap_irrational", "reason_as_absolute", EdgeType.NEGATION),
        Edge("immortality_soul", "return_to_absolute", EdgeType.DEPENDENCY),
        Edge("freedom_early", "fall_from_absolute", EdgeType.DEPENDENCY),
        Edge("freedom_early", "practical_philosophy", EdgeType.REFERENCE),
    ]
    return "1804: Philosophy and Religion", vertices, edges


def work_stuttgart_seminars():
    """1810: Stuttgart Seminars (Stuttgarter Privatvorlesungen)."""
    vertices = [
        Vertex("god_becoming", content="God as becoming — not static absolute"),
        Vertex("three_potencies_god", content="three potencies in God (A¹ A² A³)"),
        Vertex("dark_principle_god", content="dark principle in God (gravity, contraction)"),
        Vertex("light_principle_god", content="light principle in God (expansion, love)"),
        Vertex("creation_as_separation", content="creation as separation from God"),
        Vertex("human_as_bond", content="human as bond (Band) of nature and spirit"),
        Vertex("illness_as_autonomy", content="illness as false autonomy of a principle"),
        Vertex("evil_preview", content="evil (preview — wrong potency dominance)"),
    ]
    edges = [
        Edge("god_becoming", "absolute", EdgeType.SUBLATION),
        Edge("god_becoming", "productivity", EdgeType.REFERENCE),
        Edge("three_potencies_god", "god_becoming", EdgeType.DEPENDENCY),
        Edge("three_potencies_god", "potency_A", EdgeType.REFERENCE),
        Edge("three_potencies_god", "potency_B", EdgeType.REFERENCE),
        Edge("three_potencies_god", "potency_AB", EdgeType.REFERENCE),
        Edge("dark_principle_god", "three_potencies_god", EdgeType.DEPENDENCY),
        Edge("dark_principle_god", "gravity", EdgeType.REFERENCE),
        Edge("light_principle_god", "three_potencies_god", EdgeType.DEPENDENCY),
        Edge("light_principle_god", "light", EdgeType.REFERENCE),
        Edge("dark_principle_god", "light_principle_god", EdgeType.NEGATION),
        Edge("creation_as_separation", "fall_from_absolute", EdgeType.SUBLATION),
        Edge("creation_as_separation", "god_becoming", EdgeType.DEPENDENCY),
        Edge("human_as_bond", "nature", EdgeType.REFERENCE),
        Edge("human_as_bond", "spirit", EdgeType.REFERENCE),
        Edge("human_as_bond", "creation_as_separation", EdgeType.DEPENDENCY),
        Edge("illness_as_autonomy", "organism", EdgeType.REFERENCE),
        Edge("illness_as_autonomy", "evil_preview", EdgeType.REFERENCE),
        Edge("evil_preview", "dark_principle_god", EdgeType.DEPENDENCY),
        Edge("evil_preview", "freedom_early", EdgeType.REFERENCE),
    ]
    return "1810: Stuttgart Seminars", vertices, edges


def work_freedom_essay():
    """1809: Of Human Freedom (Freiheitsschrift)."""
    vertices = [
        Vertex("freedom", content="human freedom — capacity for good and evil"),
        Vertex("evil", content="evil as real positive force"),
        Vertex("ground", content="ground of existence (Grund) — dark yearning"),
        Vertex("existence", content="existence (Existenz) — what exists from ground"),
        Vertex("longing", content="longing (Sehnsucht) — ground's yearning for light"),
        Vertex("understanding", content="understanding (Verstand) — word spoken into darkness"),
        Vertex("will", content="will — primordial being (Ursein ist Wollen)"),
        Vertex("love", content="love — bond uniting ground and existence"),
        Vertex("dark_ground", content="dark ground in God — that in God which is not God"),
        Vertex("pantheism_defense", content="defense of pantheism — system of freedom"),
        Vertex("personality_god", content="personality of God — living will"),
        Vertex("inversion_potencies", content="inversion of potencies — evil as reversal"),
    ]
    edges = [
        Edge("freedom", "evil", EdgeType.DEPENDENCY),
        Edge("freedom", "will", EdgeType.DEPENDENCY),
        Edge("evil", "ground", EdgeType.DEPENDENCY),
        Edge("evil", "inversion_potencies", EdgeType.DEPENDENCY),
        Edge("ground", "existence", EdgeType.NEGATION),
        Edge("ground", "dark_ground", EdgeType.DEPENDENCY),
        Edge("longing", "ground", EdgeType.DEPENDENCY),
        Edge("longing", "light", EdgeType.REFERENCE),
        Edge("understanding", "longing", EdgeType.NEGATION),
        Edge("understanding", "ground", EdgeType.DEPENDENCY),
        Edge("will", "ground", EdgeType.DEPENDENCY),
        Edge("will", "unconditioned", EdgeType.SUBLATION),
        Edge("love", "ground", EdgeType.REFERENCE),
        Edge("love", "existence", EdgeType.REFERENCE),
        Edge("love", "understanding", EdgeType.DEPENDENCY),
        Edge("dark_ground", "god_becoming", EdgeType.DEPENDENCY),
        Edge("dark_ground", "dark_principle_god", EdgeType.SUBLATION),
        Edge("pantheism_defense", "identity", EdgeType.REFERENCE),
        Edge("pantheism_defense", "freedom", EdgeType.DEPENDENCY),
        Edge("personality_god", "god_becoming", EdgeType.DEPENDENCY),
        Edge("personality_god", "will", EdgeType.DEPENDENCY),
        Edge("inversion_potencies", "potency_levels", EdgeType.REFERENCE),
        Edge("inversion_potencies", "ground", EdgeType.DEPENDENCY),
    ]
    return "1809: Of Human Freedom", vertices, edges


def work_ages_of_the_world():
    """1811-15: Ages of the World (Die Weltalter — drafts)."""
    vertices = [
        Vertex("ages_of_world", content="ages of the world (Weltalter)"),
        Vertex("past", content="the past — contraction, ground"),
        Vertex("present", content="the present — expansion, existence"),
        Vertex("future", content="the future — resolution, love"),
        Vertex("eternal_beginning", content="eternal beginning — before time"),
        Vertex("rotary_drive", content="rotary drive (rotatorische Bewegung)"),
        Vertex("decision", content="decision (Entscheidung) — cutting ground/existence"),
        Vertex("narrative_philosophy", content="narrative philosophy — told not deduced"),
        Vertex("god_before_creation", content="God before creation — contracted in self"),
        Vertex("time_origin", content="origin of time from eternity"),
    ]
    edges = [
        Edge("ages_of_world", "god_becoming", EdgeType.DEPENDENCY),
        Edge("ages_of_world", "ground", EdgeType.REFERENCE),
        Edge("past", "ground", EdgeType.DEPENDENCY),
        Edge("past", "dark_ground", EdgeType.REFERENCE),
        Edge("present", "existence", EdgeType.DEPENDENCY),
        Edge("present", "past", EdgeType.NEGATION),
        Edge("future", "love", EdgeType.DEPENDENCY),
        Edge("future", "past", EdgeType.SUBLATION),
        Edge("future", "present", EdgeType.SUBLATION),
        Edge("eternal_beginning", "will", EdgeType.DEPENDENCY),
        Edge("eternal_beginning", "ages_of_world", EdgeType.DEPENDENCY),
        Edge("rotary_drive", "longing", EdgeType.SUBLATION),
        Edge("rotary_drive", "eternal_beginning", EdgeType.DEPENDENCY),
        Edge("decision", "freedom", EdgeType.DEPENDENCY),
        Edge("decision", "rotary_drive", EdgeType.NEGATION),
        Edge("decision", "time_origin", EdgeType.DEPENDENCY),
        Edge("narrative_philosophy", "ages_of_world", EdgeType.DEPENDENCY),
        Edge("narrative_philosophy", "history", EdgeType.REFERENCE),
        Edge("god_before_creation", "dark_ground", EdgeType.DEPENDENCY),
        Edge("god_before_creation", "rotary_drive", EdgeType.DEPENDENCY),
        Edge("time_origin", "eternal_beginning", EdgeType.NEGATION),
    ]
    return "1811-15: Ages of the World", vertices, edges


def work_on_the_deities_of_samothrace():
    """1815: On the Deities of Samothrace."""
    vertices = [
        Vertex("samothrace_mysteries", content="Samothracian mysteries — Cabiri"),
        Vertex("theogonic_process_proto", content="theogonic process (proto)"),
        Vertex("cabiri", content="Cabiri — potency sequence in myth"),
        Vertex("mythological_consciousness_proto", content="mythological consciousness (proto)"),
    ]
    edges = [
        Edge("samothrace_mysteries", "mythology_as_material", EdgeType.SUBLATION),
        Edge("theogonic_process_proto", "three_potencies_god", EdgeType.REFERENCE),
        Edge("theogonic_process_proto", "samothrace_mysteries", EdgeType.DEPENDENCY),
        Edge("cabiri", "samothrace_mysteries", EdgeType.DEPENDENCY),
        Edge("cabiri", "potency_A", EdgeType.REFERENCE),
        Edge("cabiri", "potency_B", EdgeType.REFERENCE),
        Edge("mythological_consciousness_proto", "mythology_as_material", EdgeType.SUBLATION),
        Edge("mythological_consciousness_proto", "unconscious", EdgeType.REFERENCE),
    ]
    return "1815: On the Deities of Samothrace", vertices, edges


def work_erlangen_lectures():
    """1820-21: Erlangen Lectures (Initia Philosophiae Universae)."""
    vertices = [
        Vertex("negative_philosophy", content="negative philosophy — what things are (essence)"),
        Vertex("positive_philosophy", content="positive philosophy — that things are (existence)"),
        Vertex("potency_doctrine_late", content="potency doctrine (late — A, B, A/B)"),
        Vertex("ecstatic_reason", content="ecstatic reason — reason going beyond itself"),
        Vertex("prius", content="prius — that which is before being"),
        Vertex("lord_of_being", content="lord of being (Herr des Seyns)"),
    ]
    edges = [
        Edge("negative_philosophy", "Identitaetsphilosophie", EdgeType.SUBLATION),
        Edge("negative_philosophy", "reason_as_absolute", EdgeType.REFERENCE),
        Edge("positive_philosophy", "negative_philosophy", EdgeType.NEGATION),
        Edge("positive_philosophy", "existence", EdgeType.DEPENDENCY),
        Edge("positive_philosophy", "will", EdgeType.REFERENCE),
        Edge("potency_doctrine_late", "potency_A", EdgeType.SUBLATION),
        Edge("potency_doctrine_late", "potency_B", EdgeType.SUBLATION),
        Edge("potency_doctrine_late", "potency_AB", EdgeType.SUBLATION),
        Edge("ecstatic_reason", "intellectual_intuition", EdgeType.SUBLATION),
        Edge("ecstatic_reason", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("prius", "absolute", EdgeType.SUBLATION),
        Edge("prius", "will", EdgeType.DEPENDENCY),
        Edge("lord_of_being", "prius", EdgeType.DEPENDENCY),
        Edge("lord_of_being", "personality_god", EdgeType.REFERENCE),
    ]
    return "1820-21: Erlangen Lectures", vertices, edges


def work_philosophy_of_mythology():
    """1842 (lectures 1837+): Philosophy of Mythology."""
    vertices = [
        Vertex("mythology", content="mythology as real theogonic process"),
        Vertex("theogony", content="theogony — birth of gods in consciousness"),
        Vertex("mythological_consciousness", content="mythological consciousness — not allegorical"),
        Vertex("tautegory", content="tautegory — myth means what it says"),
        Vertex("polytheism", content="polytheism as necessary stage"),
        Vertex("monotheism", content="relative monotheism (Urmonotheismus)"),
        Vertex("potency_sequence_myth", content="potency sequence in mythological process"),
        Vertex("cosmogony", content="cosmogonic process"),
        Vertex("dionysus", content="Dionysus — liberating god, final mythological potency"),
        Vertex("necessity_myth", content="necessity of mythological process — not invention"),
    ]
    edges = [
        Edge("mythology", "mythology_as_material", EdgeType.SUBLATION),
        Edge("mythology", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("theogony", "mythology", EdgeType.DEPENDENCY),
        Edge("theogony", "theogonic_process_proto", EdgeType.SUBLATION),
        Edge("mythological_consciousness", "mythological_consciousness_proto", EdgeType.SUBLATION),
        Edge("mythological_consciousness", "unconscious", EdgeType.REFERENCE),
        Edge("tautegory", "symbol", EdgeType.SUBLATION),
        Edge("tautegory", "allegory", EdgeType.NEGATION),
        Edge("tautegory", "mythological_consciousness", EdgeType.DEPENDENCY),
        Edge("polytheism", "monotheism", EdgeType.NEGATION),
        Edge("polytheism", "mythology", EdgeType.DEPENDENCY),
        Edge("monotheism", "mythology", EdgeType.DEPENDENCY),
        Edge("potency_sequence_myth", "potency_doctrine_late", EdgeType.DEPENDENCY),
        Edge("potency_sequence_myth", "theogony", EdgeType.DEPENDENCY),
        Edge("cosmogony", "theogony", EdgeType.REFERENCE),
        Edge("cosmogony", "nature", EdgeType.REFERENCE),
        Edge("dionysus", "theogony", EdgeType.DEPENDENCY),
        Edge("dionysus", "freedom", EdgeType.REFERENCE),
        Edge("necessity_myth", "mythological_consciousness", EdgeType.DEPENDENCY),
        Edge("necessity_myth", "ground", EdgeType.REFERENCE),
    ]
    return "1842: Philosophy of Mythology", vertices, edges


def work_philosophy_of_revelation():
    """1842-43 (lectures 1841+): Philosophy of Revelation."""
    vertices = [
        Vertex("revelation", content="revelation as free divine act"),
        Vertex("christianity_philosophy", content="Christianity as philosophical fact"),
        Vertex("trinity", content="trinity — three potencies of God realized"),
        Vertex("incarnation", content="incarnation — second potency enters history"),
        Vertex("creation_free_act", content="creation as free act (not necessary emanation)"),
        Vertex("redemption", content="redemption — restoration of potency order"),
        Vertex("church_ages", content="ages of the church — Petrine, Pauline, Johannine"),
        Vertex("factual_philosophy", content="factual philosophy — philosophy of facts, not concepts"),
        Vertex("god_as_lord_history", content="God as lord of history — personal will"),
    ]
    edges = [
        Edge("revelation", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("revelation", "mythology", EdgeType.SUBLATION),
        Edge("revelation", "freedom", EdgeType.DEPENDENCY),
        Edge("christianity_philosophy", "revelation", EdgeType.DEPENDENCY),
        Edge("christianity_philosophy", "mythology", EdgeType.NEGATION),
        Edge("trinity", "three_potencies_god", EdgeType.SUBLATION),
        Edge("trinity", "revelation", EdgeType.DEPENDENCY),
        Edge("incarnation", "trinity", EdgeType.DEPENDENCY),
        Edge("incarnation", "potency_B", EdgeType.REFERENCE),
        Edge("incarnation", "history", EdgeType.REFERENCE),
        Edge("creation_free_act", "creation_as_separation", EdgeType.SUBLATION),
        Edge("creation_free_act", "will", EdgeType.DEPENDENCY),
        Edge("creation_free_act", "revelation", EdgeType.REFERENCE),
        Edge("redemption", "evil", EdgeType.NEGATION),
        Edge("redemption", "revelation", EdgeType.DEPENDENCY),
        Edge("redemption", "love", EdgeType.REFERENCE),
        Edge("church_ages", "revelation", EdgeType.DEPENDENCY),
        Edge("church_ages", "ages_of_world", EdgeType.REFERENCE),
        Edge("factual_philosophy", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("factual_philosophy", "negative_philosophy", EdgeType.NEGATION),
        Edge("god_as_lord_history", "lord_of_being", EdgeType.SUBLATION),
        Edge("god_as_lord_history", "personality_god", EdgeType.SUBLATION),
        Edge("god_as_lord_history", "history", EdgeType.DEPENDENCY),
    ]
    return "1842-43: Philosophy of Revelation", vertices, edges


def work_grounding_positive_philosophy():
    """1842-43: Grounding of Positive Philosophy (Berlin lectures intro)."""
    vertices = [
        Vertex("hegel_critique", content="critique of Hegel — purely negative/logical"),
        Vertex("existence_beyond_reason", content="existence irreducible to reason"),
        Vertex("unvordenkliches_sein", content="unvordenkliches Sein — immemorial being"),
        Vertex("experience_philosophy", content="experience (Erfahrung) in positive philosophy"),
        Vertex("contingency", content="contingency — what need not be"),
        Vertex("a_posteriori_philosophy", content="a posteriori philosophy"),
    ]
    edges = [
        Edge("hegel_critique", "negative_philosophy", EdgeType.REFERENCE),
        Edge("hegel_critique", "reason_as_absolute", EdgeType.NEGATION),
        Edge("existence_beyond_reason", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("existence_beyond_reason", "existence", EdgeType.DEPENDENCY),
        Edge("existence_beyond_reason", "hegel_critique", EdgeType.DEPENDENCY),
        Edge("unvordenkliches_sein", "prius", EdgeType.SUBLATION),
        Edge("unvordenkliches_sein", "ground", EdgeType.REFERENCE),
        Edge("experience_philosophy", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("experience_philosophy", "factual_philosophy", EdgeType.REFERENCE),
        Edge("contingency", "necessity_myth", EdgeType.NEGATION),
        Edge("contingency", "freedom", EdgeType.REFERENCE),
        Edge("a_posteriori_philosophy", "positive_philosophy", EdgeType.DEPENDENCY),
        Edge("a_posteriori_philosophy", "experience_philosophy", EdgeType.DEPENDENCY),
        Edge("a_posteriori_philosophy", "negative_philosophy", EdgeType.NEGATION),
    ]
    return "1842-43: Grounding of Positive Philosophy", vertices, edges


# ---------------------------------------------------------------------------
# Work list — chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_uber_die_moglichkeit,             # 1794
    work_vom_ich,                          # 1795
    work_philosophical_letters,            # 1795
    work_ideas_nature,                     # 1797
    work_world_soul,                       # 1798
    work_first_outline,                    # 1799
    work_system_transcendental_idealism,   # 1800
    work_bruno,                            # 1802
    work_further_presentations,            # 1802
    work_philosophy_of_art,               # 1802-03
    work_method_academic_study,           # 1803
    work_philosophy_and_religion,         # 1804
    work_stuttgart_seminars,              # 1810
    work_freedom_essay,                   # 1809
    work_ages_of_the_world,              # 1811-15
    work_on_the_deities_of_samothrace,   # 1815
    work_erlangen_lectures,              # 1820-21
    work_philosophy_of_mythology,        # 1842
    work_philosophy_of_revelation,       # 1842-43
    work_grounding_positive_philosophy,  # 1842-43
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("F.W.J. SCHELLING — COMPLETE WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "schelling_complete_works_incremental",
        "source": "F.W.J. Schelling — complete works, 20 entries (1794-1843)",
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

    output_path = "experiment_schelling.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
