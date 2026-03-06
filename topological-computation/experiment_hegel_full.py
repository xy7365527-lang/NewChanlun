"""Hegel Collected Works — full incremental traversal experiment.

Each work is injected as a dialectical complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Hegel's thought
from early theological writings through the mature system
(Phenomenology, Science of Logic, Encyclopedia, Philosophy of Right,
Lectures on Aesthetics, Religion, and History of Philosophy),
as an incremental topological growth process.

Covers ~28 works spanning 1795-1831+ (including posthumous lectures).
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
# Work encodings: each returns (work_name, vertices, edges)
# Cross-work edges connect to vertices already introduced in prior works.
# Ordered by writing / publication date.
# ---------------------------------------------------------------------------


def w_life_of_jesus():
    """1795: The Life of Jesus — Jesus as Kantian moral teacher."""
    vertices = [
        Vertex("jesus_moral_teacher", content="Jesus as moral teacher"),
        Vertex("pure_reason_religion", content="religion within pure reason"),
        Vertex("moral_law", content="moral law"),
        Vertex("positive_religion", content="positive religion (statutory)"),
        Vertex("folk_religion", content="folk religion (Volksreligion)"),
        Vertex("subjective_religion", content="subjective religion"),
        Vertex("objective_religion", content="objective religion"),
        Vertex("kantian_ethics_early", content="Kantian ethics (early)"),
        Vertex("love_early", content="love (early theological)"),
    ]
    edges = [
        Edge("jesus_moral_teacher", "moral_law", EdgeType.DEPENDENCY),
        Edge("jesus_moral_teacher", "pure_reason_religion", EdgeType.DEPENDENCY),
        Edge("positive_religion", "moral_law", EdgeType.NEGATION),
        Edge("folk_religion", "positive_religion", EdgeType.NEGATION),
        Edge("folk_religion", "subjective_religion", EdgeType.DEPENDENCY),
        Edge("subjective_religion", "objective_religion", EdgeType.NEGATION),
        Edge("objective_religion", "subjective_religion", EdgeType.NEGATION),
        Edge("kantian_ethics_early", "moral_law", EdgeType.DEPENDENCY),
        Edge("love_early", "moral_law", EdgeType.REFERENCE),
        Edge("love_early", "kantian_ethics_early", EdgeType.REFERENCE),
    ]
    return "1795: Life of Jesus", vertices, edges


def w_positivity_of_christianity():
    """1795-96: The Positivity of the Christian Religion."""
    vertices = [
        Vertex("positivity", content="positivity (dead letter of law)"),
        Vertex("living_religion", content="living religion"),
        Vertex("authority_religion", content="authority in religion"),
        Vertex("reason_freedom", content="reason and freedom"),
        Vertex("fetishism", content="fetishism (reification of spirit)"),
        Vertex("historical_contingency", content="historical contingency of doctrine"),
        Vertex("corruption_original", content="corruption of original teaching"),
    ]
    edges = [
        Edge("positivity", "living_religion", EdgeType.NEGATION),
        Edge("authority_religion", "reason_freedom", EdgeType.NEGATION),
        Edge("positivity", "authority_religion", EdgeType.DEPENDENCY),
        Edge("fetishism", "positivity", EdgeType.DEPENDENCY),
        Edge("fetishism", "living_religion", EdgeType.NEGATION),
        Edge("historical_contingency", "positivity", EdgeType.DEPENDENCY),
        Edge("corruption_original", "positivity", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("positivity", "positive_religion", EdgeType.DEPENDENCY),
        Edge("living_religion", "folk_religion", EdgeType.REFERENCE),
        Edge("reason_freedom", "moral_law", EdgeType.REFERENCE),
    ]
    return "1795-96: Positivity of the Christian Religion", vertices, edges


def w_spirit_of_christianity():
    """1798-99: The Spirit of Christianity and its Fate."""
    vertices = [
        Vertex("love_principle", content="love as unifying principle"),
        Vertex("fate", content="fate (Schicksal)"),
        Vertex("jewish_law", content="Jewish law (abstract universality)"),
        Vertex("abraham", content="Abraham — separation from nature"),
        Vertex("beauty_of_soul", content="beauty of soul"),
        Vertex("life_divine", content="divine life"),
        Vertex("punishment_fate", content="punishment as fate, not justice"),
        Vertex("reconciliation_early", content="reconciliation (early)"),
        Vertex("kantian_opposition", content="Kantian opposition duty/inclination"),
        Vertex("beyond_morality", content="beyond morality — love"),
        Vertex("pleroma", content="pleroma — fullness of life"),
    ]
    edges = [
        Edge("love_principle", "jewish_law", EdgeType.NEGATION),
        Edge("love_principle", "kantian_opposition", EdgeType.NEGATION),
        Edge("fate", "punishment_fate", EdgeType.DEPENDENCY),
        Edge("fate", "jewish_law", EdgeType.NEGATION),
        Edge("abraham", "jewish_law", EdgeType.DEPENDENCY),
        Edge("beauty_of_soul", "love_principle", EdgeType.DEPENDENCY),
        Edge("life_divine", "love_principle", EdgeType.DEPENDENCY),
        Edge("life_divine", "pleroma", EdgeType.DEPENDENCY),
        Edge("reconciliation_early", "fate", EdgeType.DEPENDENCY),
        Edge("reconciliation_early", "love_principle", EdgeType.DEPENDENCY),
        Edge("beyond_morality", "kantian_opposition", EdgeType.NEGATION),
        Edge("beyond_morality", "love_principle", EdgeType.DEPENDENCY),
        Edge("pleroma", "jewish_law", EdgeType.NEGATION),
        # Cross-work
        Edge("kantian_opposition", "kantian_ethics_early", EdgeType.DEPENDENCY),
        Edge("love_principle", "love_early", EdgeType.DEPENDENCY),
        Edge("reconciliation_early", "living_religion", EdgeType.REFERENCE),
    ]
    return "1798-99: Spirit of Christianity and its Fate", vertices, edges


def w_fragment_on_love():
    """1797-98: Fragment on Love."""
    vertices = [
        Vertex("love_unity", content="love as unity of subject and object"),
        Vertex("separation_reflective", content="separation by reflection"),
        Vertex("living_bond", content="living bond"),
        Vertex("property_love", content="property negated by love"),
    ]
    edges = [
        Edge("love_unity", "separation_reflective", EdgeType.NEGATION),
        Edge("living_bond", "love_unity", EdgeType.DEPENDENCY),
        Edge("property_love", "love_unity", EdgeType.NEGATION),
        # Cross-work
        Edge("love_unity", "love_principle", EdgeType.DEPENDENCY),
        Edge("separation_reflective", "kantian_opposition", EdgeType.REFERENCE),
    ]
    return "1797-98: Fragment on Love", vertices, edges


def w_system_fragment():
    """1800: Fragment of a System (Systemfragment)."""
    vertices = [
        Vertex("life_infinite", content="infinite life"),
        Vertex("finite_life", content="finite life"),
        Vertex("union_opposition", content="union of union and non-union"),
        Vertex("religion_elevation", content="religion as elevation above finite"),
        Vertex("philosophy_reflection", content="philosophy = reflection (limitation)"),
        Vertex("manifold_life", content="manifold of life"),
    ]
    edges = [
        Edge("life_infinite", "finite_life", EdgeType.NEGATION),
        Edge("union_opposition", "life_infinite", EdgeType.DEPENDENCY),
        Edge("union_opposition", "finite_life", EdgeType.SUBLATION),
        Edge("religion_elevation", "finite_life", EdgeType.NEGATION),
        Edge("religion_elevation", "life_infinite", EdgeType.DEPENDENCY),
        Edge("philosophy_reflection", "life_infinite", EdgeType.NEGATION),
        Edge("philosophy_reflection", "religion_elevation", EdgeType.NEGATION),
        Edge("manifold_life", "life_infinite", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("life_infinite", "life_divine", EdgeType.DEPENDENCY),
        Edge("union_opposition", "love_unity", EdgeType.REFERENCE),
        Edge("religion_elevation", "reconciliation_early", EdgeType.REFERENCE),
    ]
    return "1800: Fragment of a System", vertices, edges


def w_difference_essay():
    """1801: Differenzschrift — Difference between Fichte's and Schelling's System."""
    vertices = [
        Vertex("absolute_identity", content="absolute identity (Schelling)"),
        Vertex("fichte_subjectivity", content="Fichte's subjective idealism"),
        Vertex("schelling_objectivity", content="Schelling's objective identity"),
        Vertex("dichotomy", content="dichotomy (Entzweiung)"),
        Vertex("need_for_philosophy", content="need for philosophy"),
        Vertex("speculation", content="speculation vs reflection"),
        Vertex("intellectual_intuition", content="intellectual intuition"),
        Vertex("common_sense", content="common sense (gesunder Menschenverstand)"),
        Vertex("antinomy_reflection", content="antinomy of reflection"),
        Vertex("transcendental_intuition", content="transcendental intuition"),
    ]
    edges = [
        Edge("fichte_subjectivity", "schelling_objectivity", EdgeType.NEGATION),
        Edge("absolute_identity", "fichte_subjectivity", EdgeType.NEGATION),
        Edge("absolute_identity", "schelling_objectivity", EdgeType.DEPENDENCY),
        Edge("dichotomy", "need_for_philosophy", EdgeType.DEPENDENCY),
        Edge("need_for_philosophy", "dichotomy", EdgeType.NEGATION),
        Edge("speculation", "antinomy_reflection", EdgeType.NEGATION),
        Edge("intellectual_intuition", "speculation", EdgeType.DEPENDENCY),
        Edge("common_sense", "speculation", EdgeType.NEGATION),
        Edge("transcendental_intuition", "intellectual_intuition", EdgeType.DEPENDENCY),
        Edge("antinomy_reflection", "fichte_subjectivity", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("dichotomy", "separation_reflective", EdgeType.REFERENCE),
        Edge("absolute_identity", "union_opposition", EdgeType.REFERENCE),
        Edge("speculation", "philosophy_reflection", EdgeType.NEGATION),
    ]
    return "1801: Difference Essay (Differenzschrift)", vertices, edges


def w_faith_and_knowledge():
    """1802: Faith and Knowledge (Glauben und Wissen)."""
    vertices = [
        Vertex("faith_knowledge_opposition", content="faith vs knowledge opposition"),
        Vertex("kant_critique", content="Kant's critical philosophy (limit)"),
        Vertex("jacobi_immediacy", content="Jacobi's immediate faith"),
        Vertex("fichte_striving", content="Fichte's infinite striving"),
        Vertex("speculative_good_friday", content="speculative Good Friday"),
        Vertex("death_of_god_speculative", content="death of God (speculative)"),
        Vertex("highest_totality", content="highest totality"),
        Vertex("reflection_philosophy", content="philosophy of reflection"),
        Vertex("infinite_grief", content="infinite grief / pain"),
    ]
    edges = [
        Edge("faith_knowledge_opposition", "kant_critique", EdgeType.DEPENDENCY),
        Edge("faith_knowledge_opposition", "jacobi_immediacy", EdgeType.DEPENDENCY),
        Edge("faith_knowledge_opposition", "fichte_striving", EdgeType.DEPENDENCY),
        Edge("kant_critique", "jacobi_immediacy", EdgeType.NEGATION),
        Edge("jacobi_immediacy", "fichte_striving", EdgeType.NEGATION),
        Edge("speculative_good_friday", "death_of_god_speculative", EdgeType.DEPENDENCY),
        Edge("speculative_good_friday", "infinite_grief", EdgeType.DEPENDENCY),
        Edge("speculative_good_friday", "highest_totality", EdgeType.DEPENDENCY),
        Edge("death_of_god_speculative", "reflection_philosophy", EdgeType.NEGATION),
        Edge("highest_totality", "reflection_philosophy", EdgeType.NEGATION),
        Edge("reflection_philosophy", "kant_critique", EdgeType.DEPENDENCY),
        Edge("reflection_philosophy", "jacobi_immediacy", EdgeType.DEPENDENCY),
        Edge("reflection_philosophy", "fichte_striving", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("reflection_philosophy", "philosophy_reflection", EdgeType.DEPENDENCY),
        Edge("death_of_god_speculative", "religion_elevation", EdgeType.REFERENCE),
        Edge("speculative_good_friday", "reconciliation_early", EdgeType.REFERENCE),
        Edge("highest_totality", "absolute_identity", EdgeType.REFERENCE),
    ]
    return "1802: Faith and Knowledge", vertices, edges


def w_natural_law():
    """1802-03: On the Scientific Ways of Treating Natural Law."""
    vertices = [
        Vertex("empirical_natural_law", content="empirical treatment of natural law"),
        Vertex("formal_natural_law", content="formal treatment of natural law (Kant/Fichte)"),
        Vertex("ethical_totality", content="ethical totality (Sittlichkeit)"),
        Vertex("tragedy_ethical", content="tragedy in the ethical"),
        Vertex("absolute_ethical_life", content="absolute ethical life"),
        Vertex("bourgeois_society_early", content="bourgeois society (early)"),
        Vertex("crime_punishment_natural", content="crime and punishment (natural law)"),
    ]
    edges = [
        Edge("empirical_natural_law", "formal_natural_law", EdgeType.NEGATION),
        Edge("formal_natural_law", "empirical_natural_law", EdgeType.NEGATION),
        Edge("ethical_totality", "empirical_natural_law", EdgeType.NEGATION),
        Edge("ethical_totality", "formal_natural_law", EdgeType.NEGATION),
        Edge("tragedy_ethical", "ethical_totality", EdgeType.DEPENDENCY),
        Edge("absolute_ethical_life", "ethical_totality", EdgeType.DEPENDENCY),
        Edge("bourgeois_society_early", "ethical_totality", EdgeType.DEPENDENCY),
        Edge("crime_punishment_natural", "tragedy_ethical", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("ethical_totality", "speculation", EdgeType.REFERENCE),
        Edge("formal_natural_law", "kant_critique", EdgeType.REFERENCE),
        Edge("tragedy_ethical", "fate", EdgeType.REFERENCE),
    ]
    return "1802-03: Natural Law Essay", vertices, edges


def w_system_of_ethical_life():
    """1802-03: System of Ethical Life (System der Sittlichkeit)."""
    vertices = [
        Vertex("natural_ethical", content="natural ethical life"),
        Vertex("feeling_intuition", content="feeling / intuition (immediate)"),
        Vertex("formal_concept_ethical", content="formal concept of ethical life"),
        Vertex("absolute_sittlichkeit", content="absolute Sittlichkeit"),
        Vertex("potency_one", content="first potency (need/labour)"),
        Vertex("potency_two", content="second potency (tool/speech)"),
        Vertex("potency_three", content="third potency (family/people)"),
        Vertex("recognition_early", content="recognition (early Jena)"),
        Vertex("labour_tool", content="labour and tool"),
        Vertex("people_state", content="people (Volk) and state"),
    ]
    edges = [
        Edge("natural_ethical", "feeling_intuition", EdgeType.DEPENDENCY),
        Edge("formal_concept_ethical", "natural_ethical", EdgeType.NEGATION),
        Edge("absolute_sittlichkeit", "formal_concept_ethical", EdgeType.NEGATION),
        Edge("absolute_sittlichkeit", "natural_ethical", EdgeType.SUBLATION),
        Edge("potency_one", "natural_ethical", EdgeType.DEPENDENCY),
        Edge("potency_two", "potency_one", EdgeType.NEGATION),
        Edge("potency_three", "potency_two", EdgeType.NEGATION),
        Edge("recognition_early", "potency_two", EdgeType.DEPENDENCY),
        Edge("labour_tool", "potency_one", EdgeType.DEPENDENCY),
        Edge("people_state", "potency_three", EdgeType.DEPENDENCY),
        Edge("people_state", "absolute_sittlichkeit", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("absolute_sittlichkeit", "ethical_totality", EdgeType.DEPENDENCY),
        Edge("recognition_early", "bourgeois_society_early", EdgeType.REFERENCE),
        Edge("labour_tool", "potency_one", EdgeType.REFERENCE),
    ]
    return "1802-03: System of Ethical Life", vertices, edges


def w_jena_realphilosophie():
    """1805-06: Jena Realphilosophie (Lectures on Philosophy of Spirit)."""
    vertices = [
        Vertex("spirit_jena", content="spirit (Jena lectures)"),
        Vertex("language_jena", content="language (naming)"),
        Vertex("memory_jena", content="memory (mechanical)"),
        Vertex("will_jena", content="will"),
        Vertex("contract_jena", content="contract"),
        Vertex("crime_jena", content="crime"),
        Vertex("punishment_jena", content="punishment (restoring right)"),
        Vertex("recognition_struggle", content="struggle for recognition"),
        Vertex("state_jena", content="state (constitution)"),
        Vertex("world_history_jena", content="world history (Jena)"),
        Vertex("tool_cunning", content="cunning of reason (tool)"),
    ]
    edges = [
        Edge("spirit_jena", "language_jena", EdgeType.DEPENDENCY),
        Edge("spirit_jena", "memory_jena", EdgeType.DEPENDENCY),
        Edge("spirit_jena", "will_jena", EdgeType.DEPENDENCY),
        Edge("will_jena", "contract_jena", EdgeType.DEPENDENCY),
        Edge("crime_jena", "contract_jena", EdgeType.NEGATION),
        Edge("punishment_jena", "crime_jena", EdgeType.NEGATION),
        Edge("recognition_struggle", "will_jena", EdgeType.DEPENDENCY),
        Edge("state_jena", "contract_jena", EdgeType.DEPENDENCY),
        Edge("state_jena", "recognition_struggle", EdgeType.DEPENDENCY),
        Edge("world_history_jena", "state_jena", EdgeType.DEPENDENCY),
        Edge("tool_cunning", "labour_tool", EdgeType.DEPENDENCY),
        Edge("tool_cunning", "world_history_jena", EdgeType.REFERENCE),
        # Cross-work
        Edge("recognition_struggle", "recognition_early", EdgeType.DEPENDENCY),
        Edge("spirit_jena", "absolute_sittlichkeit", EdgeType.REFERENCE),
        Edge("language_jena", "potency_two", EdgeType.REFERENCE),
    ]
    return "1805-06: Jena Realphilosophie", vertices, edges


def w_phenomenology():
    """1807: Phenomenology of Spirit — consciousness to absolute knowing."""
    vertices = [
        Vertex("sense_certainty", content="sense-certainty"),
        Vertex("perception", content="perception (Wahrnehmung)"),
        Vertex("understanding", content="understanding (Verstand)"),
        Vertex("self_consciousness", content="self-consciousness"),
        Vertex("lord_bondsman", content="lordship and bondage"),
        Vertex("stoicism", content="stoicism"),
        Vertex("scepticism", content="scepticism"),
        Vertex("unhappy_consciousness", content="unhappy consciousness"),
        Vertex("reason_observing", content="observing reason"),
        Vertex("ethical_order", content="ethical order (Sittlichkeit)"),
        Vertex("culture_bildung", content="culture / Bildung"),
        Vertex("absolute_freedom", content="absolute freedom and terror"),
        Vertex("morality_phenom", content="morality (conscience/forgiveness)"),
        Vertex("religion_phenom", content="religion (natural/art/revealed)"),
        Vertex("absolute_knowing", content="absolute knowing"),
        Vertex("experience_consciousness", content="experience of consciousness"),
        Vertex("spirit_phenom", content="spirit (Geist)"),
        Vertex("substance_subject", content="substance is subject"),
        Vertex("dialectical_movement", content="dialectical movement"),
        Vertex("negation_determinate", content="determinate negation"),
    ]
    edges = [
        Edge("sense_certainty", "perception", EdgeType.NEGATION),
        Edge("perception", "understanding", EdgeType.NEGATION),
        Edge("understanding", "self_consciousness", EdgeType.NEGATION),
        Edge("self_consciousness", "lord_bondsman", EdgeType.DEPENDENCY),
        Edge("lord_bondsman", "stoicism", EdgeType.NEGATION),
        Edge("stoicism", "scepticism", EdgeType.NEGATION),
        Edge("scepticism", "unhappy_consciousness", EdgeType.NEGATION),
        Edge("unhappy_consciousness", "reason_observing", EdgeType.NEGATION),
        Edge("reason_observing", "ethical_order", EdgeType.NEGATION),
        Edge("ethical_order", "culture_bildung", EdgeType.NEGATION),
        Edge("culture_bildung", "absolute_freedom", EdgeType.NEGATION),
        Edge("absolute_freedom", "morality_phenom", EdgeType.NEGATION),
        Edge("morality_phenom", "religion_phenom", EdgeType.NEGATION),
        Edge("religion_phenom", "absolute_knowing", EdgeType.NEGATION),
        Edge("experience_consciousness", "dialectical_movement", EdgeType.DEPENDENCY),
        Edge("spirit_phenom", "ethical_order", EdgeType.DEPENDENCY),
        Edge("spirit_phenom", "culture_bildung", EdgeType.DEPENDENCY),
        Edge("spirit_phenom", "morality_phenom", EdgeType.DEPENDENCY),
        Edge("substance_subject", "spirit_phenom", EdgeType.DEPENDENCY),
        Edge("negation_determinate", "dialectical_movement", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "substance_subject", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "negation_determinate", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("lord_bondsman", "recognition_struggle", EdgeType.DEPENDENCY),
        Edge("ethical_order", "absolute_sittlichkeit", EdgeType.REFERENCE),
        Edge("substance_subject", "absolute_identity", EdgeType.NEGATION),
        Edge("absolute_knowing", "speculation", EdgeType.REFERENCE),
        Edge("unhappy_consciousness", "reconciliation_early", EdgeType.REFERENCE),
        Edge("absolute_freedom", "bourgeois_society_early", EdgeType.REFERENCE),
        Edge("religion_phenom", "death_of_god_speculative", EdgeType.REFERENCE),
    ]
    return "1807: Phenomenology of Spirit", vertices, edges


def w_logic_being():
    """1812: Science of Logic, Vol. 1 — The Doctrine of Being."""
    vertices = [
        Vertex("being", content="being (Sein)"),
        Vertex("nothing", content="nothing (Nichts)"),
        Vertex("becoming", content="becoming (Werden)"),
        Vertex("determinate_being", content="determinate being (Dasein)"),
        Vertex("quality", content="quality"),
        Vertex("quantity", content="quantity"),
        Vertex("measure", content="measure (Maß)"),
        Vertex("being_for_self", content="being-for-self (Fürsichsein)"),
        Vertex("one_many", content="the one and the many"),
        Vertex("something_other", content="something and other"),
        Vertex("finitude", content="finitude"),
        Vertex("infinity_true", content="true infinity"),
        Vertex("infinity_bad", content="bad infinity (Schlechte Unendlichkeit)"),
        Vertex("limit", content="limit / boundary"),
        Vertex("degree", content="degree"),
        Vertex("quantum", content="quantum"),
        Vertex("nodal_line", content="nodal line of measures"),
        Vertex("measureless", content="the measureless"),
    ]
    edges = [
        Edge("being", "nothing", EdgeType.NEGATION),
        Edge("nothing", "being", EdgeType.NEGATION),
        Edge("becoming", "being", EdgeType.SUBLATION),
        Edge("becoming", "nothing", EdgeType.SUBLATION),
        Edge("determinate_being", "becoming", EdgeType.DEPENDENCY),
        Edge("quality", "determinate_being", EdgeType.DEPENDENCY),
        Edge("something_other", "determinate_being", EdgeType.DEPENDENCY),
        Edge("finitude", "something_other", EdgeType.DEPENDENCY),
        Edge("limit", "finitude", EdgeType.DEPENDENCY),
        Edge("infinity_bad", "finitude", EdgeType.NEGATION),
        Edge("infinity_true", "infinity_bad", EdgeType.NEGATION),
        Edge("infinity_true", "finitude", EdgeType.SUBLATION),
        Edge("being_for_self", "infinity_true", EdgeType.DEPENDENCY),
        Edge("one_many", "being_for_self", EdgeType.DEPENDENCY),
        Edge("quantity", "quality", EdgeType.NEGATION),
        Edge("quantum", "quantity", EdgeType.DEPENDENCY),
        Edge("degree", "quantum", EdgeType.DEPENDENCY),
        Edge("measure", "quality", EdgeType.SUBLATION),
        Edge("measure", "quantity", EdgeType.SUBLATION),
        Edge("nodal_line", "measure", EdgeType.DEPENDENCY),
        Edge("measureless", "measure", EdgeType.NEGATION),
        # Cross-work
        Edge("being", "absolute_knowing", EdgeType.DEPENDENCY),
        Edge("becoming", "dialectical_movement", EdgeType.REFERENCE),
        Edge("negation_determinate", "nothing", EdgeType.REFERENCE),
        Edge("infinity_true", "life_infinite", EdgeType.REFERENCE),
    ]
    return "1812: Science of Logic — Doctrine of Being", vertices, edges


def w_logic_essence():
    """1813: Science of Logic, Vol. 2 — The Doctrine of Essence."""
    vertices = [
        Vertex("essence", content="essence (Wesen)"),
        Vertex("appearance", content="appearance (Erscheinung)"),
        Vertex("actuality", content="actuality (Wirklichkeit)"),
        Vertex("reflection", content="reflection (positing/external/determining)"),
        Vertex("identity_essence", content="identity (A=A)"),
        Vertex("difference_essence", content="difference"),
        Vertex("contradiction", content="contradiction"),
        Vertex("ground", content="ground (Grund)"),
        Vertex("existence_essence", content="existence (Existenz)"),
        Vertex("thing_essence", content="the thing (Ding)"),
        Vertex("law_of_appearance", content="law of appearance"),
        Vertex("essential_relation", content="essential relation (whole/parts, force, inner/outer)"),
        Vertex("substance_essence", content="substance (Spinoza)"),
        Vertex("causality", content="causality"),
        Vertex("reciprocity", content="reciprocity (Wechselwirkung)"),
        Vertex("necessity", content="necessity"),
        Vertex("contingency_essence", content="contingency"),
        Vertex("possibility_actuality", content="possibility / actuality"),
    ]
    edges = [
        Edge("essence", "being", EdgeType.NEGATION),
        Edge("essence", "appearance", EdgeType.DEPENDENCY),
        Edge("appearance", "actuality", EdgeType.NEGATION),
        Edge("reflection", "essence", EdgeType.DEPENDENCY),
        Edge("identity_essence", "reflection", EdgeType.DEPENDENCY),
        Edge("difference_essence", "identity_essence", EdgeType.NEGATION),
        Edge("contradiction", "difference_essence", EdgeType.DEPENDENCY),
        Edge("contradiction", "identity_essence", EdgeType.NEGATION),
        Edge("ground", "contradiction", EdgeType.DEPENDENCY),
        Edge("existence_essence", "ground", EdgeType.DEPENDENCY),
        Edge("thing_essence", "existence_essence", EdgeType.DEPENDENCY),
        Edge("law_of_appearance", "appearance", EdgeType.DEPENDENCY),
        Edge("essential_relation", "appearance", EdgeType.DEPENDENCY),
        Edge("substance_essence", "actuality", EdgeType.DEPENDENCY),
        Edge("causality", "substance_essence", EdgeType.DEPENDENCY),
        Edge("reciprocity", "causality", EdgeType.NEGATION),
        Edge("necessity", "actuality", EdgeType.DEPENDENCY),
        Edge("contingency_essence", "necessity", EdgeType.NEGATION),
        Edge("possibility_actuality", "actuality", EdgeType.DEPENDENCY),
        Edge("actuality", "essence", EdgeType.SUBLATION),
        Edge("actuality", "appearance", EdgeType.SUBLATION),
        # Cross-work
        Edge("essence", "measureless", EdgeType.DEPENDENCY),
        Edge("contradiction", "negation_determinate", EdgeType.REFERENCE),
        Edge("substance_essence", "substance_subject", EdgeType.REFERENCE),
        Edge("reciprocity", "dialectical_movement", EdgeType.REFERENCE),
    ]
    return "1813: Science of Logic — Doctrine of Essence", vertices, edges


def w_logic_concept():
    """1816: Science of Logic, Vol. 3 — The Doctrine of the Concept."""
    vertices = [
        Vertex("concept", content="the concept (Begriff)"),
        Vertex("universal", content="universality"),
        Vertex("particular", content="particularity"),
        Vertex("individual", content="individuality (Einzelheit)"),
        Vertex("judgment", content="judgment (Urteil)"),
        Vertex("syllogism", content="syllogism (Schluss)"),
        Vertex("mechanism", content="mechanism"),
        Vertex("chemism", content="chemism"),
        Vertex("teleology", content="teleology (purpose)"),
        Vertex("life_logic", content="life (logical)"),
        Vertex("cognition_logic", content="cognition (logical)"),
        Vertex("idea", content="the idea (Idee)"),
        Vertex("absolute_idea", content="the absolute idea"),
        Vertex("method_dialectical", content="the dialectical method"),
        Vertex("objectivity_logic", content="objectivity (logical)"),
        Vertex("subjectivity_logic", content="subjectivity (logical)"),
    ]
    edges = [
        Edge("concept", "reciprocity", EdgeType.DEPENDENCY),
        Edge("universal", "concept", EdgeType.DEPENDENCY),
        Edge("particular", "universal", EdgeType.NEGATION),
        Edge("individual", "particular", EdgeType.NEGATION),
        Edge("individual", "universal", EdgeType.SUBLATION),
        Edge("judgment", "concept", EdgeType.DEPENDENCY),
        Edge("syllogism", "judgment", EdgeType.DEPENDENCY),
        Edge("subjectivity_logic", "concept", EdgeType.DEPENDENCY),
        Edge("subjectivity_logic", "judgment", EdgeType.DEPENDENCY),
        Edge("subjectivity_logic", "syllogism", EdgeType.DEPENDENCY),
        Edge("objectivity_logic", "subjectivity_logic", EdgeType.NEGATION),
        Edge("mechanism", "objectivity_logic", EdgeType.DEPENDENCY),
        Edge("chemism", "mechanism", EdgeType.NEGATION),
        Edge("teleology", "chemism", EdgeType.NEGATION),
        Edge("life_logic", "teleology", EdgeType.DEPENDENCY),
        Edge("cognition_logic", "life_logic", EdgeType.DEPENDENCY),
        Edge("idea", "cognition_logic", EdgeType.DEPENDENCY),
        Edge("idea", "life_logic", EdgeType.SUBLATION),
        Edge("absolute_idea", "idea", EdgeType.DEPENDENCY),
        Edge("absolute_idea", "method_dialectical", EdgeType.DEPENDENCY),
        Edge("method_dialectical", "negation_determinate", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("concept", "actuality", EdgeType.DEPENDENCY),
        Edge("absolute_idea", "absolute_knowing", EdgeType.REFERENCE),
        Edge("life_logic", "life_infinite", EdgeType.REFERENCE),
        Edge("idea", "speculation", EdgeType.REFERENCE),
    ]
    return "1816: Science of Logic — Doctrine of the Concept", vertices, edges


def w_encyclopedia_logic():
    """1817/1827/1830: Encyclopedia Logic (Smaller Logic, Enzyklopädie §1-244)."""
    vertices = [
        Vertex("preliminary_concept", content="preliminary concept of logic"),
        Vertex("empiricism_enc", content="empiricism"),
        Vertex("critical_philosophy_enc", content="critical philosophy (Kant)"),
        Vertex("immediate_knowing_enc", content="immediate knowing (Jacobi)"),
        Vertex("three_sides_logical", content="three sides of the logical"),
        Vertex("abstract_understanding", content="abstract / understanding moment"),
        Vertex("dialectical_negative", content="dialectical / negative-rational moment"),
        Vertex("speculative_positive", content="speculative / positive-rational moment"),
        Vertex("beginning_logic", content="with what must logic begin"),
        Vertex("transition_being_essence", content="transition from being to essence"),
        Vertex("transition_essence_concept", content="transition from essence to concept"),
    ]
    edges = [
        Edge("preliminary_concept", "empiricism_enc", EdgeType.DEPENDENCY),
        Edge("preliminary_concept", "critical_philosophy_enc", EdgeType.DEPENDENCY),
        Edge("preliminary_concept", "immediate_knowing_enc", EdgeType.DEPENDENCY),
        Edge("empiricism_enc", "critical_philosophy_enc", EdgeType.NEGATION),
        Edge("critical_philosophy_enc", "immediate_knowing_enc", EdgeType.NEGATION),
        Edge("three_sides_logical", "abstract_understanding", EdgeType.DEPENDENCY),
        Edge("three_sides_logical", "dialectical_negative", EdgeType.DEPENDENCY),
        Edge("three_sides_logical", "speculative_positive", EdgeType.DEPENDENCY),
        Edge("abstract_understanding", "dialectical_negative", EdgeType.NEGATION),
        Edge("dialectical_negative", "speculative_positive", EdgeType.NEGATION),
        Edge("speculative_positive", "abstract_understanding", EdgeType.SUBLATION),
        Edge("speculative_positive", "dialectical_negative", EdgeType.SUBLATION),
        Edge("beginning_logic", "being", EdgeType.DEPENDENCY),
        Edge("transition_being_essence", "measureless", EdgeType.DEPENDENCY),
        Edge("transition_being_essence", "essence", EdgeType.DEPENDENCY),
        Edge("transition_essence_concept", "reciprocity", EdgeType.DEPENDENCY),
        Edge("transition_essence_concept", "concept", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("three_sides_logical", "method_dialectical", EdgeType.REFERENCE),
        Edge("critical_philosophy_enc", "kant_critique", EdgeType.REFERENCE),
        Edge("immediate_knowing_enc", "jacobi_immediacy", EdgeType.REFERENCE),
        Edge("speculative_positive", "absolute_idea", EdgeType.REFERENCE),
    ]
    return "1817/1830: Encyclopedia — Logic", vertices, edges


def w_encyclopedia_nature():
    """1817/1827/1830: Encyclopedia Philosophy of Nature (§245-376)."""
    vertices = [
        Vertex("nature_externality", content="nature as externality of the idea"),
        Vertex("space", content="space"),
        Vertex("time_nature", content="time"),
        Vertex("matter_motion", content="matter and motion"),
        Vertex("mechanics", content="mechanics"),
        Vertex("physics", content="physics (particularity of matter)"),
        Vertex("organics", content="organics (life in nature)"),
        Vertex("geological_organism", content="geological organism"),
        Vertex("vegetal_organism", content="vegetal organism"),
        Vertex("animal_organism", content="animal organism"),
        Vertex("death_natural", content="death of the natural individual"),
        Vertex("genus_process", content="genus process (Gattungsprozeß)"),
        Vertex("idea_returns_nature", content="idea returns from nature to spirit"),
    ]
    edges = [
        Edge("nature_externality", "absolute_idea", EdgeType.NEGATION),
        Edge("space", "nature_externality", EdgeType.DEPENDENCY),
        Edge("time_nature", "space", EdgeType.NEGATION),
        Edge("matter_motion", "space", EdgeType.DEPENDENCY),
        Edge("matter_motion", "time_nature", EdgeType.DEPENDENCY),
        Edge("mechanics", "matter_motion", EdgeType.DEPENDENCY),
        Edge("physics", "mechanics", EdgeType.NEGATION),
        Edge("organics", "physics", EdgeType.NEGATION),
        Edge("geological_organism", "organics", EdgeType.DEPENDENCY),
        Edge("vegetal_organism", "geological_organism", EdgeType.NEGATION),
        Edge("animal_organism", "vegetal_organism", EdgeType.NEGATION),
        Edge("death_natural", "animal_organism", EdgeType.NEGATION),
        Edge("genus_process", "animal_organism", EdgeType.DEPENDENCY),
        Edge("genus_process", "death_natural", EdgeType.DEPENDENCY),
        Edge("idea_returns_nature", "genus_process", EdgeType.DEPENDENCY),
        Edge("idea_returns_nature", "nature_externality", EdgeType.NEGATION),
        # Cross-work
        Edge("nature_externality", "absolute_idea", EdgeType.DEPENDENCY),
        Edge("organics", "life_logic", EdgeType.REFERENCE),
        Edge("idea_returns_nature", "spirit_phenom", EdgeType.REFERENCE),
    ]
    return "1817/1830: Encyclopedia — Philosophy of Nature", vertices, edges


def w_encyclopedia_spirit():
    """1817/1827/1830: Encyclopedia Philosophy of Spirit (§377-577)."""
    vertices = [
        Vertex("subjective_spirit", content="subjective spirit"),
        Vertex("anthropology_enc", content="anthropology (soul)"),
        Vertex("phenomenology_enc", content="phenomenology (consciousness)"),
        Vertex("psychology_enc", content="psychology (mind)"),
        Vertex("objective_spirit", content="objective spirit"),
        Vertex("abstract_right", content="abstract right"),
        Vertex("morality_enc", content="morality (Moralität)"),
        Vertex("ethical_life_enc", content="ethical life (Sittlichkeit)"),
        Vertex("absolute_spirit", content="absolute spirit"),
        Vertex("art_enc", content="art (absolute spirit)"),
        Vertex("revealed_religion_enc", content="revealed religion (absolute spirit)"),
        Vertex("philosophy_enc", content="philosophy (absolute spirit)"),
        Vertex("free_spirit", content="free spirit"),
        Vertex("habit", content="habit (second nature)"),
        Vertex("theoretical_spirit", content="theoretical spirit"),
        Vertex("practical_spirit", content="practical spirit"),
    ]
    edges = [
        Edge("subjective_spirit", "idea_returns_nature", EdgeType.DEPENDENCY),
        Edge("anthropology_enc", "subjective_spirit", EdgeType.DEPENDENCY),
        Edge("phenomenology_enc", "anthropology_enc", EdgeType.NEGATION),
        Edge("psychology_enc", "phenomenology_enc", EdgeType.NEGATION),
        Edge("theoretical_spirit", "psychology_enc", EdgeType.DEPENDENCY),
        Edge("practical_spirit", "theoretical_spirit", EdgeType.NEGATION),
        Edge("free_spirit", "practical_spirit", EdgeType.DEPENDENCY),
        Edge("free_spirit", "theoretical_spirit", EdgeType.SUBLATION),
        Edge("habit", "anthropology_enc", EdgeType.DEPENDENCY),
        Edge("objective_spirit", "free_spirit", EdgeType.DEPENDENCY),
        Edge("abstract_right", "objective_spirit", EdgeType.DEPENDENCY),
        Edge("morality_enc", "abstract_right", EdgeType.NEGATION),
        Edge("ethical_life_enc", "morality_enc", EdgeType.NEGATION),
        Edge("ethical_life_enc", "abstract_right", EdgeType.SUBLATION),
        Edge("absolute_spirit", "objective_spirit", EdgeType.NEGATION),
        Edge("art_enc", "absolute_spirit", EdgeType.DEPENDENCY),
        Edge("revealed_religion_enc", "art_enc", EdgeType.NEGATION),
        Edge("philosophy_enc", "revealed_religion_enc", EdgeType.NEGATION),
        Edge("philosophy_enc", "absolute_idea", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("phenomenology_enc", "experience_consciousness", EdgeType.REFERENCE),
        Edge("ethical_life_enc", "ethical_order", EdgeType.REFERENCE),
        Edge("absolute_spirit", "absolute_knowing", EdgeType.REFERENCE),
        Edge("art_enc", "religion_phenom", EdgeType.REFERENCE),
        Edge("objective_spirit", "spirit_phenom", EdgeType.REFERENCE),
    ]
    return "1817/1830: Encyclopedia — Philosophy of Spirit", vertices, edges


def w_philosophy_of_right():
    """1820: Elements of the Philosophy of Right."""
    vertices = [
        Vertex("right_abstract", content="abstract right (property, contract, wrong)"),
        Vertex("property", content="property (Eigentum)"),
        Vertex("contract", content="contract (Vertrag)"),
        Vertex("wrong_crime", content="wrong / crime (Unrecht)"),
        Vertex("morality_right", content="morality (Moralität)"),
        Vertex("purpose_responsibility", content="purpose and responsibility"),
        Vertex("intention_welfare", content="intention and welfare"),
        Vertex("good_conscience", content="the good and conscience"),
        Vertex("ethical_life_right", content="ethical life (Sittlichkeit)"),
        Vertex("family", content="family"),
        Vertex("civil_society", content="civil society (bürgerliche Gesellschaft)"),
        Vertex("state_right", content="the state"),
        Vertex("system_of_needs", content="system of needs"),
        Vertex("police_corporation", content="police and corporation"),
        Vertex("administration_of_justice", content="administration of justice"),
        Vertex("constitutional_law", content="constitutional law (internal)"),
        Vertex("international_law", content="international law (external)"),
        Vertex("world_history_right", content="world history"),
        Vertex("monarch", content="the monarch"),
        Vertex("executive_power", content="executive power"),
        Vertex("legislative_power", content="legislative power"),
        Vertex("estates", content="estates (Stände)"),
        Vertex("rational_actual", content="what is rational is actual"),
        Vertex("freedom_will", content="free will"),
        Vertex("rabble", content="the rabble (Pöbel)"),
    ]
    edges = [
        Edge("right_abstract", "freedom_will", EdgeType.DEPENDENCY),
        Edge("property", "right_abstract", EdgeType.DEPENDENCY),
        Edge("contract", "property", EdgeType.DEPENDENCY),
        Edge("wrong_crime", "contract", EdgeType.NEGATION),
        Edge("morality_right", "right_abstract", EdgeType.NEGATION),
        Edge("purpose_responsibility", "morality_right", EdgeType.DEPENDENCY),
        Edge("intention_welfare", "purpose_responsibility", EdgeType.DEPENDENCY),
        Edge("good_conscience", "intention_welfare", EdgeType.DEPENDENCY),
        Edge("ethical_life_right", "morality_right", EdgeType.NEGATION),
        Edge("ethical_life_right", "right_abstract", EdgeType.SUBLATION),
        Edge("family", "ethical_life_right", EdgeType.DEPENDENCY),
        Edge("civil_society", "family", EdgeType.NEGATION),
        Edge("state_right", "civil_society", EdgeType.NEGATION),
        Edge("state_right", "family", EdgeType.SUBLATION),
        Edge("system_of_needs", "civil_society", EdgeType.DEPENDENCY),
        Edge("administration_of_justice", "civil_society", EdgeType.DEPENDENCY),
        Edge("police_corporation", "civil_society", EdgeType.DEPENDENCY),
        Edge("constitutional_law", "state_right", EdgeType.DEPENDENCY),
        Edge("international_law", "state_right", EdgeType.DEPENDENCY),
        Edge("world_history_right", "international_law", EdgeType.DEPENDENCY),
        Edge("monarch", "constitutional_law", EdgeType.DEPENDENCY),
        Edge("executive_power", "monarch", EdgeType.DEPENDENCY),
        Edge("legislative_power", "executive_power", EdgeType.DEPENDENCY),
        Edge("estates", "legislative_power", EdgeType.DEPENDENCY),
        Edge("rational_actual", "ethical_life_right", EdgeType.DEPENDENCY),
        Edge("freedom_will", "concept", EdgeType.DEPENDENCY),
        Edge("rabble", "system_of_needs", EdgeType.DEPENDENCY),
        Edge("rabble", "civil_society", EdgeType.NEGATION),
        # Cross-work
        Edge("right_abstract", "abstract_right", EdgeType.DEPENDENCY),
        Edge("morality_right", "morality_enc", EdgeType.REFERENCE),
        Edge("ethical_life_right", "ethical_life_enc", EdgeType.DEPENDENCY),
        Edge("civil_society", "bourgeois_society_early", EdgeType.REFERENCE),
        Edge("world_history_right", "world_history_jena", EdgeType.REFERENCE),
        Edge("rational_actual", "actuality", EdgeType.REFERENCE),
        Edge("freedom_will", "free_spirit", EdgeType.REFERENCE),
    ]
    return "1820: Philosophy of Right", vertices, edges


def w_lectures_aesthetics():
    """1820s: Lectures on Aesthetics (Vorlesungen über die Ästhetik)."""
    vertices = [
        Vertex("beauty_idea", content="the beautiful is the sensuous shining of the idea"),
        Vertex("ideal", content="the ideal"),
        Vertex("symbolic_art", content="symbolic art form"),
        Vertex("classical_art", content="classical art form"),
        Vertex("romantic_art", content="romantic art form"),
        Vertex("end_of_art", content="end of art (dissolution)"),
        Vertex("architecture", content="architecture (symbolic)"),
        Vertex("sculpture", content="sculpture (classical)"),
        Vertex("painting", content="painting (romantic)"),
        Vertex("music", content="music (romantic)"),
        Vertex("poetry", content="poetry (romantic — universal art)"),
        Vertex("content_form_art", content="unity of content and form"),
        Vertex("natural_beauty", content="natural beauty (deficient)"),
        Vertex("artistic_beauty", content="artistic beauty (spiritual)"),
        Vertex("sublime", content="the sublime"),
    ]
    edges = [
        Edge("beauty_idea", "idea", EdgeType.DEPENDENCY),
        Edge("ideal", "beauty_idea", EdgeType.DEPENDENCY),
        Edge("symbolic_art", "ideal", EdgeType.DEPENDENCY),
        Edge("classical_art", "symbolic_art", EdgeType.NEGATION),
        Edge("romantic_art", "classical_art", EdgeType.NEGATION),
        Edge("end_of_art", "romantic_art", EdgeType.NEGATION),
        Edge("architecture", "symbolic_art", EdgeType.DEPENDENCY),
        Edge("sculpture", "classical_art", EdgeType.DEPENDENCY),
        Edge("painting", "romantic_art", EdgeType.DEPENDENCY),
        Edge("music", "romantic_art", EdgeType.DEPENDENCY),
        Edge("poetry", "romantic_art", EdgeType.DEPENDENCY),
        Edge("poetry", "music", EdgeType.NEGATION),
        Edge("content_form_art", "classical_art", EdgeType.DEPENDENCY),
        Edge("content_form_art", "symbolic_art", EdgeType.SUBLATION),
        Edge("natural_beauty", "beauty_idea", EdgeType.DEPENDENCY),
        Edge("artistic_beauty", "natural_beauty", EdgeType.NEGATION),
        Edge("sublime", "symbolic_art", EdgeType.DEPENDENCY),
        Edge("sublime", "classical_art", EdgeType.NEGATION),
        # Cross-work
        Edge("beauty_idea", "art_enc", EdgeType.DEPENDENCY),
        Edge("end_of_art", "philosophy_enc", EdgeType.REFERENCE),
        Edge("romantic_art", "revealed_religion_enc", EdgeType.REFERENCE),
        Edge("content_form_art", "absolute_idea", EdgeType.REFERENCE),
    ]
    return "1820s: Lectures on Aesthetics", vertices, edges


def w_lectures_religion():
    """1821-31: Lectures on the Philosophy of Religion."""
    vertices = [
        Vertex("concept_of_religion", content="the concept of religion"),
        Vertex("religious_consciousness", content="religious consciousness"),
        Vertex("feeling_religion", content="feeling (Gefühl) in religion"),
        Vertex("representation_religion", content="representation (Vorstellung)"),
        Vertex("thought_religion", content="thought in religion"),
        Vertex("determinate_religion", content="determinate religion"),
        Vertex("nature_religion", content="nature religion (immediate)"),
        Vertex("spiritual_individuality_religion", content="religion of spiritual individuality"),
        Vertex("religion_sublimity", content="religion of sublimity (Jewish)"),
        Vertex("religion_beauty", content="religion of beauty (Greek)"),
        Vertex("religion_purposiveness", content="religion of purposiveness (Roman)"),
        Vertex("consummate_religion", content="consummate/absolute religion (Christianity)"),
        Vertex("trinity_religion", content="Trinity (immanent life of God)"),
        Vertex("creation_fall", content="creation, fall, evil"),
        Vertex("reconciliation_religion", content="reconciliation (Versöhnung)"),
        Vertex("community_religion", content="community / church"),
        Vertex("cult", content="cult (devotion, sacrament)"),
    ]
    edges = [
        Edge("concept_of_religion", "absolute_spirit", EdgeType.DEPENDENCY),
        Edge("religious_consciousness", "concept_of_religion", EdgeType.DEPENDENCY),
        Edge("feeling_religion", "religious_consciousness", EdgeType.DEPENDENCY),
        Edge("representation_religion", "feeling_religion", EdgeType.NEGATION),
        Edge("thought_religion", "representation_religion", EdgeType.NEGATION),
        Edge("determinate_religion", "concept_of_religion", EdgeType.DEPENDENCY),
        Edge("nature_religion", "determinate_religion", EdgeType.DEPENDENCY),
        Edge("spiritual_individuality_religion", "nature_religion", EdgeType.NEGATION),
        Edge("religion_sublimity", "spiritual_individuality_religion", EdgeType.DEPENDENCY),
        Edge("religion_beauty", "religion_sublimity", EdgeType.NEGATION),
        Edge("religion_purposiveness", "religion_beauty", EdgeType.NEGATION),
        Edge("consummate_religion", "determinate_religion", EdgeType.NEGATION),
        Edge("consummate_religion", "religion_purposiveness", EdgeType.DEPENDENCY),
        Edge("trinity_religion", "consummate_religion", EdgeType.DEPENDENCY),
        Edge("creation_fall", "trinity_religion", EdgeType.DEPENDENCY),
        Edge("reconciliation_religion", "creation_fall", EdgeType.NEGATION),
        Edge("community_religion", "reconciliation_religion", EdgeType.DEPENDENCY),
        Edge("cult", "concept_of_religion", EdgeType.DEPENDENCY),
        Edge("cult", "community_religion", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("consummate_religion", "revealed_religion_enc", EdgeType.DEPENDENCY),
        Edge("representation_religion", "religion_phenom", EdgeType.REFERENCE),
        Edge("reconciliation_religion", "reconciliation_early", EdgeType.REFERENCE),
        Edge("thought_religion", "philosophy_enc", EdgeType.REFERENCE),
        Edge("trinity_religion", "absolute_idea", EdgeType.REFERENCE),
        Edge("religion_sublimity", "jewish_law", EdgeType.REFERENCE),
    ]
    return "1821-31: Lectures on Philosophy of Religion", vertices, edges


def w_lectures_history_philosophy():
    """1819-31: Lectures on the History of Philosophy."""
    vertices = [
        Vertex("history_of_philosophy", content="history of philosophy"),
        Vertex("oriental_philosophy", content="Oriental philosophy (beginning)"),
        Vertex("greek_philosophy", content="Greek philosophy"),
        Vertex("thales_to_anaxagoras", content="Thales to Anaxagoras"),
        Vertex("sophists_socrates", content="Sophists and Socrates"),
        Vertex("plato_hp", content="Plato (history of philosophy)"),
        Vertex("aristotle_hp", content="Aristotle (history of philosophy)"),
        Vertex("stoic_sceptic_epicurean", content="Stoics, Sceptics, Epicureans"),
        Vertex("neoplatonism", content="Neoplatonism"),
        Vertex("medieval_philosophy", content="medieval philosophy (Scholasticism)"),
        Vertex("modern_philosophy", content="modern philosophy"),
        Vertex("descartes_spinoza", content="Descartes and Spinoza"),
        Vertex("leibniz_wolff", content="Leibniz and Wolff"),
        Vertex("british_empiricism", content="British empiricism"),
        Vertex("french_enlightenment", content="French Enlightenment"),
        Vertex("kant_hp", content="Kant (history of philosophy)"),
        Vertex("fichte_hp", content="Fichte (history of philosophy)"),
        Vertex("schelling_hp", content="Schelling (history of philosophy)"),
        Vertex("philosophy_self_comprehension", content="philosophy comprehends itself"),
    ]
    edges = [
        Edge("history_of_philosophy", "philosophy_enc", EdgeType.DEPENDENCY),
        Edge("oriental_philosophy", "history_of_philosophy", EdgeType.DEPENDENCY),
        Edge("greek_philosophy", "oriental_philosophy", EdgeType.NEGATION),
        Edge("thales_to_anaxagoras", "greek_philosophy", EdgeType.DEPENDENCY),
        Edge("sophists_socrates", "thales_to_anaxagoras", EdgeType.NEGATION),
        Edge("plato_hp", "sophists_socrates", EdgeType.NEGATION),
        Edge("aristotle_hp", "plato_hp", EdgeType.NEGATION),
        Edge("stoic_sceptic_epicurean", "aristotle_hp", EdgeType.NEGATION),
        Edge("neoplatonism", "stoic_sceptic_epicurean", EdgeType.NEGATION),
        Edge("medieval_philosophy", "neoplatonism", EdgeType.NEGATION),
        Edge("modern_philosophy", "medieval_philosophy", EdgeType.NEGATION),
        Edge("descartes_spinoza", "modern_philosophy", EdgeType.DEPENDENCY),
        Edge("leibniz_wolff", "descartes_spinoza", EdgeType.NEGATION),
        Edge("british_empiricism", "leibniz_wolff", EdgeType.NEGATION),
        Edge("french_enlightenment", "british_empiricism", EdgeType.NEGATION),
        Edge("kant_hp", "french_enlightenment", EdgeType.NEGATION),
        Edge("fichte_hp", "kant_hp", EdgeType.NEGATION),
        Edge("schelling_hp", "fichte_hp", EdgeType.NEGATION),
        Edge("philosophy_self_comprehension", "schelling_hp", EdgeType.NEGATION),
        Edge("philosophy_self_comprehension", "history_of_philosophy", EdgeType.DEPENDENCY),
        # Cross-work
        Edge("philosophy_self_comprehension", "absolute_knowing", EdgeType.REFERENCE),
        Edge("kant_hp", "critical_philosophy_enc", EdgeType.REFERENCE),
        Edge("descartes_spinoza", "substance_essence", EdgeType.REFERENCE),
        Edge("schelling_hp", "absolute_identity", EdgeType.REFERENCE),
        Edge("fichte_hp", "fichte_subjectivity", EdgeType.REFERENCE),
        Edge("neoplatonism", "speculative_good_friday", EdgeType.REFERENCE),
    ]
    return "1819-31: Lectures on History of Philosophy", vertices, edges


def w_lectures_world_history():
    """1822-31: Lectures on the Philosophy of World History."""
    vertices = [
        Vertex("world_spirit", content="world spirit (Weltgeist)"),
        Vertex("cunning_of_reason", content="cunning of reason (List der Vernunft)"),
        Vertex("world_historical_individual", content="world-historical individual"),
        Vertex("oriental_world", content="Oriental world (one is free)"),
        Vertex("greek_world", content="Greek world (some are free)"),
        Vertex("roman_world", content="Roman world (abstract universality)"),
        Vertex("germanic_world", content="Germanic world (all are free)"),
        Vertex("freedom_consciousness", content="consciousness of freedom"),
        Vertex("progress_freedom", content="progress in consciousness of freedom"),
        Vertex("state_realization_freedom", content="state as realization of freedom"),
        Vertex("passions_individuals", content="passions of individuals (means)"),
        Vertex("geographical_basis", content="geographical basis of history"),
        Vertex("reformation", content="the Reformation"),
        Vertex("french_revolution_wh", content="French Revolution"),
    ]
    edges = [
        Edge("world_spirit", "absolute_spirit", EdgeType.DEPENDENCY),
        Edge("cunning_of_reason", "world_spirit", EdgeType.DEPENDENCY),
        Edge("cunning_of_reason", "passions_individuals", EdgeType.DEPENDENCY),
        Edge("world_historical_individual", "cunning_of_reason", EdgeType.DEPENDENCY),
        Edge("oriental_world", "freedom_consciousness", EdgeType.DEPENDENCY),
        Edge("greek_world", "oriental_world", EdgeType.NEGATION),
        Edge("roman_world", "greek_world", EdgeType.NEGATION),
        Edge("germanic_world", "roman_world", EdgeType.NEGATION),
        Edge("freedom_consciousness", "world_spirit", EdgeType.DEPENDENCY),
        Edge("progress_freedom", "freedom_consciousness", EdgeType.DEPENDENCY),
        Edge("progress_freedom", "germanic_world", EdgeType.DEPENDENCY),
        Edge("state_realization_freedom", "freedom_consciousness", EdgeType.DEPENDENCY),
        Edge("state_realization_freedom", "state_right", EdgeType.REFERENCE),
        Edge("passions_individuals", "world_historical_individual", EdgeType.DEPENDENCY),
        Edge("geographical_basis", "nature_externality", EdgeType.REFERENCE),
        Edge("reformation", "germanic_world", EdgeType.DEPENDENCY),
        Edge("french_revolution_wh", "reformation", EdgeType.DEPENDENCY),
        Edge("french_revolution_wh", "absolute_freedom", EdgeType.REFERENCE),
        # Cross-work
        Edge("world_spirit", "spirit_phenom", EdgeType.REFERENCE),
        Edge("cunning_of_reason", "tool_cunning", EdgeType.DEPENDENCY),
        Edge("world_historical_individual", "world_history_right", EdgeType.REFERENCE),
        Edge("germanic_world", "ethical_life_right", EdgeType.REFERENCE),
    ]
    return "1822-31: Lectures on Philosophy of World History", vertices, edges


def w_berlin_writings():
    """1826-31: Berlin Writings — Reviews, Prefaces, and Late Essays."""
    vertices = [
        Vertex("review_hamann", content="review of Hamann's writings"),
        Vertex("review_goeschel", content="review of Göschel (aphorisms)"),
        Vertex("review_humboldt", content="review of Humboldt (Bhagavad-Gita)"),
        Vertex("english_reform_bill", content="on the English Reform Bill"),
        Vertex("sovereignty_people", content="sovereignty of the people (critique)"),
        Vertex("philosophy_system_completion", content="philosophy as system, completion"),
        Vertex("actuality_philosophy_late", content="actuality of philosophy (late)"),
    ]
    edges = [
        Edge("review_hamann", "feeling_religion", EdgeType.REFERENCE),
        Edge("review_hamann", "immediate_knowing_enc", EdgeType.REFERENCE),
        Edge("review_goeschel", "thought_religion", EdgeType.REFERENCE),
        Edge("review_humboldt", "oriental_philosophy", EdgeType.REFERENCE),
        Edge("english_reform_bill", "state_right", EdgeType.DEPENDENCY),
        Edge("english_reform_bill", "estates", EdgeType.REFERENCE),
        Edge("sovereignty_people", "state_right", EdgeType.DEPENDENCY),
        Edge("sovereignty_people", "monarch", EdgeType.REFERENCE),
        Edge("philosophy_system_completion", "absolute_idea", EdgeType.DEPENDENCY),
        Edge("philosophy_system_completion", "philosophy_self_comprehension", EdgeType.DEPENDENCY),
        Edge("actuality_philosophy_late", "rational_actual", EdgeType.DEPENDENCY),
        Edge("actuality_philosophy_late", "philosophy_enc", EdgeType.DEPENDENCY),
    ]
    return "1826-31: Berlin Writings", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    w_life_of_jesus,
    w_positivity_of_christianity,
    w_spirit_of_christianity,
    w_fragment_on_love,
    w_system_fragment,
    w_difference_essay,
    w_faith_and_knowledge,
    w_natural_law,
    w_system_of_ethical_life,
    w_jena_realphilosophie,
    w_phenomenology,
    w_logic_being,
    w_logic_essence,
    w_logic_concept,
    w_encyclopedia_logic,
    w_encyclopedia_nature,
    w_encyclopedia_spirit,
    w_philosophy_of_right,
    w_lectures_aesthetics,
    w_lectures_religion,
    w_lectures_history_philosophy,
    w_lectures_world_history,
    w_berlin_writings,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("HEGEL COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
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

        # Count operations
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

    # Beta_1 growth curve
    print(f"\n  beta_1 growth curve (by work):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    W{entry['work_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['work_name']}")

    # Biggest jumps
    deltas = [(r["work"], r["delta_beta_1"], r["work_index"])
              for r in work_results]
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
    crit_ratio = (ft_counts["critical"]
                  / (ft_counts["tree"] + ft_counts["critical"])
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
        "experiment": "hegel_full_incremental",
        "source": "Hegel Collected Works — 23 works (1795-1831)",
        "methodology": "manual dialectical complex per work, incremental injection, "
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
        "elapsed_seconds": round(elapsed, 2),
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_hegel_full.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
