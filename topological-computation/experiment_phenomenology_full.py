"""Phenomenology of Spirit — full 15-chapter incremental traversal experiment.

Each chapter is injected as a dialectical complex (vertices + edges).
After injection, the traversal engine "digests" the chapter
(runs until beta_1 is stable for 50 consecutive steps).
Then the next chapter is injected.

This models the progressive development of Hegel's argument
as an incremental topological growth process.
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
# Chapter encodings: each returns (chapter_name, vertices, edges)
# Cross-chapter edges connect to vertices already introduced in prior chapters.
# ---------------------------------------------------------------------------


def ch_preface():
    """Preface: truth as whole, substance-as-subject, dialectical method."""
    vertices = [
        Vertex("truth_whole", content="the truth is the whole"),
        Vertex("substance", content="substance"),
        Vertex("subject", content="subject (self-developing)"),
        Vertex("bud", content="the bud"),
        Vertex("blossom", content="the blossom"),
        Vertex("fruit", content="the fruit"),
        Vertex("mutual_necessity", content="mutual necessity"),
        Vertex("bacchanalian_revel", content="Bacchanalian revel"),
        Vertex("simple_repose", content="transparent simple repose"),
        Vertex("spirit", content="spirit"),
        Vertex("science", content="science"),
        Vertex("dogmatism", content="dogmatism"),
        Vertex("speculative_proposition", content="speculative proposition"),
        Vertex("dialectic_method", content="dialectic as method"),
        Vertex("fixed_result", content="fixed result/proposition"),
        Vertex("false_moment", content="the false as moment of truth"),
        Vertex("appearance_truth", content="appearance as actuality"),
    ]
    edges = [
        Edge("subject", "substance", EdgeType.NEGATION),
        Edge("substance", "subject", EdgeType.DEPENDENCY),
        Edge("truth_whole", "subject", EdgeType.DEPENDENCY),
        Edge("truth_whole", "substance", EdgeType.REFERENCE),
        Edge("blossom", "bud", EdgeType.NEGATION),
        Edge("fruit", "blossom", EdgeType.NEGATION),
        Edge("mutual_necessity", "bud", EdgeType.DEPENDENCY),
        Edge("mutual_necessity", "blossom", EdgeType.DEPENDENCY),
        Edge("mutual_necessity", "fruit", EdgeType.DEPENDENCY),
        Edge("bacchanalian_revel", "simple_repose", EdgeType.NEGATION),
        Edge("simple_repose", "bacchanalian_revel", EdgeType.NEGATION),
        Edge("science", "spirit", EdgeType.DEPENDENCY),
        Edge("spirit", "truth_whole", EdgeType.DEPENDENCY),
        Edge("dogmatism", "fixed_result", EdgeType.DEPENDENCY),
        Edge("speculative_proposition", "dogmatism", EdgeType.NEGATION),
        Edge("speculative_proposition", "dialectic_method", EdgeType.DEPENDENCY),
        Edge("false_moment", "truth_whole", EdgeType.DEPENDENCY),
        Edge("fixed_result", "false_moment", EdgeType.NEGATION),
        Edge("appearance_truth", "truth_whole", EdgeType.REFERENCE),
    ]
    return "Preface", vertices, edges


def ch_introduction():
    """Introduction: experience, natural consciousness, fear of error/truth."""
    vertices = [
        Vertex("cognition_instrument", content="cognition as instrument"),
        Vertex("cognition_medium", content="cognition as medium"),
        Vertex("absolute_being", content="the Absolute"),
        Vertex("fear_of_error", content="fear of error"),
        Vertex("fear_of_truth", content="fear of truth"),
        Vertex("experience", content="experience (dialectical movement)"),
        Vertex("natural_consciousness", content="natural consciousness"),
        Vertex("real_knowledge", content="real knowledge"),
        Vertex("concept_of_knowledge", content="concept of knowledge"),
        Vertex("negative_procedure", content="negative procedure"),
        Vertex("hidden_tendency", content="hidden tendency toward truth"),
    ]
    edges = [
        Edge("cognition_instrument", "absolute_being", EdgeType.DEPENDENCY),
        Edge("cognition_medium", "absolute_being", EdgeType.DEPENDENCY),
        Edge("cognition_instrument", "cognition_medium", EdgeType.NEGATION),
        Edge("fear_of_error", "fear_of_truth", EdgeType.NEGATION),
        Edge("fear_of_truth", "fear_of_error", EdgeType.NEGATION),
        Edge("experience", "natural_consciousness", EdgeType.DEPENDENCY),
        Edge("natural_consciousness", "concept_of_knowledge", EdgeType.DEPENDENCY),
        Edge("natural_consciousness", "real_knowledge", EdgeType.NEGATION),
        Edge("concept_of_knowledge", "real_knowledge", EdgeType.NEGATION),
        Edge("negative_procedure", "natural_consciousness", EdgeType.REFERENCE),
        Edge("hidden_tendency", "experience", EdgeType.DEPENDENCY),
        Edge("hidden_tendency", "real_knowledge", EdgeType.REFERENCE),
        # Cross-chapter: Preface -> Introduction
        Edge("dialectic_method", "experience", EdgeType.REFERENCE),
        Edge("false_moment", "negative_procedure", EdgeType.REFERENCE),
        Edge("truth_whole", "absolute_being", EdgeType.REFERENCE),
    ]
    return "Introduction", vertices, edges


def ch_sense_certainty():
    """A.I. Sense-Certainty: the dialectic of Now, This, Here, I."""
    vertices = [
        Vertex("sense_certainty", content="sense-certainty"),
        Vertex("immediate_knowledge", content="immediate knowledge"),
        Vertex("the_now", content="the Now"),
        Vertex("now_night", content="Now is Night"),
        Vertex("now_day", content="Now is Day"),
        Vertex("now_universal", content="Now as universal"),
        Vertex("the_this", content="the This"),
        Vertex("the_here", content="the Here"),
        Vertex("this_not_this", content="This as not-This (superseded)"),
        Vertex("determinate_nothing", content="determinate Nothing"),
        Vertex("mediated_simplicity", content="mediated simplicity"),
        Vertex("the_I", content="the I"),
        Vertex("singular_I", content="singular I"),
        Vertex("universal_I", content="universal I"),
        Vertex("language_sc", content="language (expresses universal)"),
        Vertex("what_is_meant", content="what is meant (das Meinen)"),
        Vertex("pure_being", content="pure being"),
        Vertex("universal", content="the universal"),
        Vertex("particular", content="the particular"),
        Vertex("immediacy", content="immediacy"),
        Vertex("mediation", content="mediation"),
    ]
    edges = [
        Edge("sense_certainty", "immediate_knowledge", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "the_this", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "the_I", EdgeType.DEPENDENCY),
        Edge("immediate_knowledge", "immediacy", EdgeType.DEPENDENCY),
        Edge("the_now", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("now_night", "the_now", EdgeType.DEPENDENCY),
        Edge("now_day", "the_now", EdgeType.DEPENDENCY),
        Edge("now_night", "now_day", EdgeType.NEGATION),
        Edge("now_day", "now_night", EdgeType.NEGATION),
        Edge("now_universal", "now_night", EdgeType.SUBLATION),
        Edge("now_universal", "now_day", EdgeType.SUBLATION),
        Edge("now_universal", "universal", EdgeType.DEPENDENCY),
        Edge("the_this", "particular", EdgeType.DEPENDENCY),
        Edge("this_not_this", "the_this", EdgeType.NEGATION),
        Edge("determinate_nothing", "this_not_this", EdgeType.DEPENDENCY),
        Edge("mediated_simplicity", "this_not_this", EdgeType.SUBLATION),
        Edge("mediated_simplicity", "the_this", EdgeType.SUBLATION),
        Edge("mediated_simplicity", "universal", EdgeType.DEPENDENCY),
        Edge("the_here", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("the_here", "particular", EdgeType.DEPENDENCY),
        Edge("the_I", "singular_I", EdgeType.DEPENDENCY),
        Edge("singular_I", "universal_I", EdgeType.NEGATION),
        Edge("universal_I", "singular_I", EdgeType.NEGATION),
        Edge("universal_I", "universal", EdgeType.DEPENDENCY),
        Edge("language_sc", "universal", EdgeType.DEPENDENCY),
        Edge("what_is_meant", "particular", EdgeType.DEPENDENCY),
        Edge("language_sc", "what_is_meant", EdgeType.NEGATION),
        Edge("particular", "universal", EdgeType.NEGATION),
        Edge("universal", "particular", EdgeType.NEGATION),
        Edge("immediacy", "mediation", EdgeType.NEGATION),
        Edge("mediation", "immediacy", EdgeType.NEGATION),
        Edge("pure_being", "universal", EdgeType.DEPENDENCY),
        Edge("pure_being", "sense_certainty", EdgeType.REFERENCE),
        # Cross-chapter
        Edge("natural_consciousness", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("experience", "sense_certainty", EdgeType.REFERENCE),
        Edge("real_knowledge", "immediate_knowledge", EdgeType.NEGATION),
        Edge("mediation", "experience", EdgeType.REFERENCE),
        Edge("mediation", "dialectic_method", EdgeType.REFERENCE),
        Edge("determinate_nothing", "negative_procedure", EdgeType.REFERENCE),
    ]
    return "A.I. Sense-Certainty", vertices, edges


def ch_perception():
    """A.II. Perception: thing, properties, One vs Also."""
    vertices = [
        Vertex("perception", content="perception"),
        Vertex("thing", content="the thing"),
        Vertex("many_properties", content="many properties"),
        Vertex("thinghood", content="thinghood (pure essence)"),
        Vertex("the_one", content="the One (excluding unity)"),
        Vertex("the_also", content="the Also (general medium)"),
        Vertex("salt_example", content="salt (white, pungent, cubical)"),
        Vertex("one_excludes", content="One excludes"),
        Vertex("also_includes", content="Also includes"),
        Vertex("contradictory_thing", content="contradictory nature of thing"),
        Vertex("free_matter", content="free matter/property"),
        Vertex("surface_enclosure", content="surface enclosure"),
        Vertex("white_vs_black", content="white opposed to black"),
        Vertex("property_contradiction", content="property as self-contradictory"),
        Vertex("unconditioned_universal", content="unconditioned universal"),
        Vertex("concrete_universal", content="concrete universal"),
        Vertex("sound_common_sense", content="sound common sense"),
        Vertex("insofar_as", content="insofar-as qualification"),
    ]
    edges = [
        Edge("perception", "universal", EdgeType.DEPENDENCY),
        Edge("perception", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("perception", "mediated_simplicity", EdgeType.DEPENDENCY),
        Edge("thing", "many_properties", EdgeType.DEPENDENCY),
        Edge("thing", "thinghood", EdgeType.DEPENDENCY),
        Edge("thing", "universal", EdgeType.DEPENDENCY),
        Edge("the_one", "thing", EdgeType.DEPENDENCY),
        Edge("the_also", "thing", EdgeType.DEPENDENCY),
        Edge("the_one", "the_also", EdgeType.NEGATION),
        Edge("the_also", "the_one", EdgeType.NEGATION),
        Edge("one_excludes", "the_one", EdgeType.DEPENDENCY),
        Edge("also_includes", "the_also", EdgeType.DEPENDENCY),
        Edge("one_excludes", "also_includes", EdgeType.NEGATION),
        Edge("salt_example", "many_properties", EdgeType.REFERENCE),
        Edge("salt_example", "the_also", EdgeType.REFERENCE),
        Edge("contradictory_thing", "the_one", EdgeType.DEPENDENCY),
        Edge("contradictory_thing", "the_also", EdgeType.DEPENDENCY),
        Edge("free_matter", "many_properties", EdgeType.DEPENDENCY),
        Edge("surface_enclosure", "free_matter", EdgeType.NEGATION),
        Edge("white_vs_black", "property_contradiction", EdgeType.REFERENCE),
        Edge("property_contradiction", "free_matter", EdgeType.DEPENDENCY),
        Edge("property_contradiction", "the_one", EdgeType.REFERENCE),
        Edge("unconditioned_universal", "contradictory_thing", EdgeType.SUBLATION),
        Edge("unconditioned_universal", "property_contradiction", EdgeType.SUBLATION),
        Edge("concrete_universal", "unconditioned_universal", EdgeType.DEPENDENCY),
        Edge("concrete_universal", "universal", EdgeType.DEPENDENCY),
        Edge("concrete_universal", "particular", EdgeType.DEPENDENCY),
        Edge("sound_common_sense", "insofar_as", EdgeType.DEPENDENCY),
        Edge("insofar_as", "contradictory_thing", EdgeType.REFERENCE),
        # Cross-chapter
        Edge("many_properties", "particular", EdgeType.REFERENCE),
    ]
    return "A.II. Perception", vertices, edges


def ch_force_understanding():
    """A.III. Force and Understanding: force, law, inverted world, infinity."""
    vertices = [
        Vertex("inner_being", content="inner being of thing"),
        Vertex("force", content="force"),
        Vertex("force_expression", content="expression of force"),
        Vertex("force_proper", content="force proper"),
        Vertex("play_of_forces", content="play of forces"),
        Vertex("inciting", content="inciting force"),
        Vertex("incited", content="incited force"),
        Vertex("law", content="law (stable image)"),
        Vertex("appearance_flux", content="appearance (flux)"),
        Vertex("explanation_tautology", content="explanation as tautology"),
        Vertex("supersensible_world", content="supersensible world"),
        Vertex("kingdom_of_laws", content="tranquil kingdom of laws"),
        Vertex("inverted_world", content="inverted world"),
        Vertex("sweet_sour", content="sweet becomes sour"),
        Vertex("crime_punishment", content="crime becomes punishment"),
        Vertex("internal_distinction", content="internal distinction"),
        Vertex("simple_infinity", content="simple infinity"),
        Vertex("self_consciousness_emerges", content="self-consciousness emerges"),
        Vertex("curtain", content="the curtain"),
        Vertex("nothing_behind", content="nothing behind unless we go there"),
        Vertex("self_related_negation", content="self-related negation"),
    ]
    edges = [
        Edge("inner_being", "thing", EdgeType.DEPENDENCY),
        Edge("inner_being", "unconditioned_universal", EdgeType.REFERENCE),
        Edge("inner_being", "concrete_universal", EdgeType.DEPENDENCY),
        Edge("force", "force_expression", EdgeType.NEGATION),
        Edge("force_expression", "force", EdgeType.NEGATION),
        Edge("force_proper", "force", EdgeType.DEPENDENCY),
        Edge("force_expression", "force_proper", EdgeType.NEGATION),
        Edge("play_of_forces", "inciting", EdgeType.DEPENDENCY),
        Edge("play_of_forces", "incited", EdgeType.DEPENDENCY),
        Edge("inciting", "incited", EdgeType.NEGATION),
        Edge("incited", "inciting", EdgeType.NEGATION),
        Edge("law", "play_of_forces", EdgeType.DEPENDENCY),
        Edge("law", "appearance_flux", EdgeType.DEPENDENCY),
        Edge("appearance_flux", "play_of_forces", EdgeType.REFERENCE),
        Edge("explanation_tautology", "law", EdgeType.NEGATION),
        Edge("supersensible_world", "law", EdgeType.DEPENDENCY),
        Edge("kingdom_of_laws", "supersensible_world", EdgeType.DEPENDENCY),
        Edge("inverted_world", "supersensible_world", EdgeType.NEGATION),
        Edge("sweet_sour", "inverted_world", EdgeType.REFERENCE),
        Edge("crime_punishment", "inverted_world", EdgeType.REFERENCE),
        Edge("internal_distinction", "inverted_world", EdgeType.SUBLATION),
        Edge("internal_distinction", "supersensible_world", EdgeType.SUBLATION),
        Edge("simple_infinity", "internal_distinction", EdgeType.DEPENDENCY),
        Edge("simple_infinity", "self_related_negation", EdgeType.DEPENDENCY),
        Edge("self_related_negation", "simple_infinity", EdgeType.REFERENCE),
        Edge("self_consciousness_emerges", "simple_infinity", EdgeType.DEPENDENCY),
        Edge("curtain", "supersensible_world", EdgeType.REFERENCE),
        Edge("nothing_behind", "curtain", EdgeType.NEGATION),
        Edge("self_consciousness_emerges", "curtain", EdgeType.SUBLATION),
        Edge("self_consciousness_emerges", "nothing_behind", EdgeType.DEPENDENCY),
        # Cross-chapter
        Edge("force", "unconditioned_universal", EdgeType.DEPENDENCY),
        Edge("law", "universal", EdgeType.REFERENCE),
        Edge("play_of_forces", "mediation", EdgeType.REFERENCE),
        Edge("self_related_negation", "particular", EdgeType.REFERENCE),
        Edge("self_consciousness_emerges", "spirit", EdgeType.REFERENCE),
        Edge("self_consciousness_emerges", "subject", EdgeType.REFERENCE),
    ]
    return "A.III. Force and Understanding", vertices, edges


def ch_lordship_bondage():
    """B.IV.A. Independence and Dependence of Self-Consciousness (Lordship and Bondage)."""
    vertices = [
        Vertex("self_consciousness", content="self-consciousness"),
        Vertex("desire", content="desire"),
        Vertex("life", content="life"),
        Vertex("recognition", content="recognition (Anerkennung)"),
        Vertex("lord", content="the lord (master)"),
        Vertex("bondsman", content="the bondsman (slave)"),
        Vertex("independence", content="independence"),
        Vertex("dependence", content="dependence"),
        Vertex("fear_of_death", content="fear of death"),
        Vertex("work", content="work (formative activity)"),
        Vertex("thinghood_sc", content="thinghood (object of desire)"),
        Vertex("enjoyment", content="enjoyment"),
        Vertex("struggle_life_death", content="life-and-death struggle"),
        Vertex("lord_reversal", content="lord's truth in bondsman"),
        Vertex("bondsman_freedom", content="bondsman's freedom through work"),
    ]
    edges = [
        Edge("self_consciousness", "desire", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "life", EdgeType.DEPENDENCY),
        Edge("desire", "thinghood_sc", EdgeType.DEPENDENCY),
        Edge("desire", "life", EdgeType.NEGATION),
        Edge("recognition", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("struggle_life_death", "recognition", EdgeType.DEPENDENCY),
        Edge("struggle_life_death", "fear_of_death", EdgeType.DEPENDENCY),
        Edge("lord", "recognition", EdgeType.DEPENDENCY),
        Edge("bondsman", "recognition", EdgeType.DEPENDENCY),
        Edge("lord", "bondsman", EdgeType.NEGATION),
        Edge("bondsman", "lord", EdgeType.NEGATION),
        Edge("lord", "independence", EdgeType.DEPENDENCY),
        Edge("bondsman", "dependence", EdgeType.DEPENDENCY),
        Edge("independence", "dependence", EdgeType.NEGATION),
        Edge("dependence", "independence", EdgeType.NEGATION),
        Edge("lord", "enjoyment", EdgeType.DEPENDENCY),
        Edge("bondsman", "work", EdgeType.DEPENDENCY),
        Edge("enjoyment", "thinghood_sc", EdgeType.DEPENDENCY),
        Edge("work", "thinghood_sc", EdgeType.DEPENDENCY),
        Edge("enjoyment", "work", EdgeType.NEGATION),
        Edge("lord_reversal", "lord", EdgeType.NEGATION),
        Edge("lord_reversal", "bondsman", EdgeType.DEPENDENCY),
        Edge("bondsman_freedom", "work", EdgeType.DEPENDENCY),
        Edge("bondsman_freedom", "fear_of_death", EdgeType.DEPENDENCY),
        Edge("bondsman_freedom", "independence", EdgeType.SUBLATION),
        Edge("bondsman_freedom", "dependence", EdgeType.SUBLATION),
        # Cross-chapter
        Edge("self_consciousness", "self_consciousness_emerges", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "simple_infinity", EdgeType.REFERENCE),
        Edge("life", "self_related_negation", EdgeType.REFERENCE),
    ]
    return "B.IV.A. Lordship and Bondage", vertices, edges


def ch_freedom_unhappy():
    """B.IV.B. Freedom of Self-Consciousness (Stoicism, Scepticism, Unhappy Consciousness)."""
    vertices = [
        Vertex("stoicism", content="Stoicism"),
        Vertex("scepticism", content="Scepticism"),
        Vertex("unhappy_consciousness", content="unhappy consciousness"),
        Vertex("changeable", content="the changeable (finite self)"),
        Vertex("unchangeable", content="the unchangeable (infinite beyond)"),
        Vertex("devotion", content="devotion"),
        Vertex("crusades", content="crusades (here and now of beyond)"),
        Vertex("asceticism", content="asceticism (self-mortification)"),
        Vertex("abstract_freedom", content="abstract freedom of thought"),
        Vertex("self_contradictory_consciousness", content="self-contradictory consciousness"),
        Vertex("mediation_priest", content="mediating consciousness (priest)"),
        Vertex("reconciliation_beyond", content="reconciliation projected beyond"),
    ]
    edges = [
        Edge("stoicism", "abstract_freedom", EdgeType.DEPENDENCY),
        Edge("stoicism", "bondsman_freedom", EdgeType.DEPENDENCY),
        Edge("scepticism", "stoicism", EdgeType.NEGATION),
        Edge("scepticism", "self_contradictory_consciousness", EdgeType.DEPENDENCY),
        Edge("self_contradictory_consciousness", "abstract_freedom", EdgeType.NEGATION),
        Edge("unhappy_consciousness", "scepticism", EdgeType.DEPENDENCY),
        Edge("unhappy_consciousness", "changeable", EdgeType.DEPENDENCY),
        Edge("unhappy_consciousness", "unchangeable", EdgeType.DEPENDENCY),
        Edge("changeable", "unchangeable", EdgeType.NEGATION),
        Edge("unchangeable", "changeable", EdgeType.NEGATION),
        Edge("devotion", "unchangeable", EdgeType.DEPENDENCY),
        Edge("crusades", "unchangeable", EdgeType.NEGATION),
        Edge("crusades", "devotion", EdgeType.NEGATION),
        Edge("asceticism", "changeable", EdgeType.NEGATION),
        Edge("asceticism", "unhappy_consciousness", EdgeType.DEPENDENCY),
        Edge("mediation_priest", "changeable", EdgeType.DEPENDENCY),
        Edge("mediation_priest", "unchangeable", EdgeType.DEPENDENCY),
        Edge("reconciliation_beyond", "unhappy_consciousness", EdgeType.DEPENDENCY),
        Edge("reconciliation_beyond", "mediation_priest", EdgeType.REFERENCE),
        # Cross-chapter
        Edge("stoicism", "lord", EdgeType.REFERENCE),
        Edge("stoicism", "bondsman", EdgeType.REFERENCE),
        Edge("abstract_freedom", "independence", EdgeType.REFERENCE),
    ]
    return "B.IV.B. Freedom of Self-Consciousness", vertices, edges


def ch_observing_reason():
    """C.V.A. Observing Reason."""
    vertices = [
        Vertex("reason", content="reason"),
        Vertex("observing_reason", content="observing reason"),
        Vertex("observation_nature", content="observation of nature"),
        Vertex("organic_nature", content="organic nature"),
        Vertex("self_conscious_individuality", content="self-conscious individuality"),
        Vertex("logical_laws", content="logical and psychological laws"),
        Vertex("psychological_laws", content="psychological laws"),
        Vertex("physiognomy", content="physiognomy"),
        Vertex("phrenology", content="phrenology"),
        Vertex("skull_bone", content="the skull-bone (being of spirit = bone)"),
        Vertex("reason_as_certainty", content="reason as certainty of being all reality"),
        Vertex("category", content="the category (unity of being and thinking)"),
        Vertex("inner_outer", content="inner and outer"),
    ]
    edges = [
        Edge("reason", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("reason", "reason_as_certainty", EdgeType.DEPENDENCY),
        Edge("reason_as_certainty", "category", EdgeType.DEPENDENCY),
        Edge("category", "universal", EdgeType.REFERENCE),
        Edge("observing_reason", "reason", EdgeType.DEPENDENCY),
        Edge("observing_reason", "observation_nature", EdgeType.DEPENDENCY),
        Edge("observation_nature", "organic_nature", EdgeType.DEPENDENCY),
        Edge("observation_nature", "logical_laws", EdgeType.DEPENDENCY),
        Edge("logical_laws", "psychological_laws", EdgeType.NEGATION),
        Edge("physiognomy", "inner_outer", EdgeType.DEPENDENCY),
        Edge("phrenology", "physiognomy", EdgeType.DEPENDENCY),
        Edge("skull_bone", "phrenology", EdgeType.DEPENDENCY),
        Edge("skull_bone", "spirit", EdgeType.NEGATION),
        Edge("inner_outer", "self_conscious_individuality", EdgeType.DEPENDENCY),
        Edge("self_conscious_individuality", "reason", EdgeType.DEPENDENCY),
        # Cross-chapter
        Edge("reason", "unhappy_consciousness", EdgeType.DEPENDENCY),
        Edge("reason", "reconciliation_beyond", EdgeType.NEGATION),
        Edge("category", "subject", EdgeType.REFERENCE),
    ]
    return "C.V.A. Observing Reason", vertices, edges


def ch_actualization_reason():
    """C.V.B. Actualization of Rational Self-Consciousness."""
    vertices = [
        Vertex("pleasure", content="pleasure and necessity"),
        Vertex("law_of_heart", content="law of the heart"),
        Vertex("frenzy_self_conceit", content="frenzy of self-conceit"),
        Vertex("virtue", content="virtue"),
        Vertex("way_of_world", content="the way of the world"),
        Vertex("virtue_vs_world", content="virtue vs way of the world"),
        Vertex("individuality_real", content="individuality as real"),
        Vertex("necessity_fate", content="necessity (fate)"),
        Vertex("heart_rebellion", content="heart's rebellion against order"),
    ]
    edges = [
        Edge("pleasure", "desire", EdgeType.DEPENDENCY),
        Edge("pleasure", "necessity_fate", EdgeType.NEGATION),
        Edge("necessity_fate", "pleasure", EdgeType.NEGATION),
        Edge("law_of_heart", "pleasure", EdgeType.DEPENDENCY),
        Edge("law_of_heart", "heart_rebellion", EdgeType.DEPENDENCY),
        Edge("frenzy_self_conceit", "law_of_heart", EdgeType.NEGATION),
        Edge("heart_rebellion", "frenzy_self_conceit", EdgeType.DEPENDENCY),
        Edge("virtue", "way_of_world", EdgeType.NEGATION),
        Edge("way_of_world", "virtue", EdgeType.NEGATION),
        Edge("virtue_vs_world", "virtue", EdgeType.DEPENDENCY),
        Edge("virtue_vs_world", "way_of_world", EdgeType.DEPENDENCY),
        Edge("individuality_real", "virtue_vs_world", EdgeType.SUBLATION),
        # Cross-chapter
        Edge("pleasure", "self_consciousness", EdgeType.REFERENCE),
        Edge("individuality_real", "reason", EdgeType.DEPENDENCY),
    ]
    return "C.V.B. Actualization of Rational Self-Consciousness", vertices, edges


def ch_individuality_real():
    """C.V.C. Individuality Real In and For Itself."""
    vertices = [
        Vertex("spiritual_animal_kingdom", content="spiritual animal kingdom"),
        Vertex("die_sache_selbst", content="die Sache selbst (the matter at hand)"),
        Vertex("honest_consciousness", content="honest consciousness"),
        Vertex("deception", content="deception/fraud"),
        Vertex("legislative_reason", content="legislative reason"),
        Vertex("law_testing_reason", content="law-testing reason"),
        Vertex("ethical_substance_intro", content="ethical substance (emerging)"),
        Vertex("work_product", content="work as product/expression"),
        Vertex("individual_universal_unity", content="unity of individual and universal"),
    ]
    edges = [
        Edge("spiritual_animal_kingdom", "individuality_real", EdgeType.DEPENDENCY),
        Edge("spiritual_animal_kingdom", "work_product", EdgeType.DEPENDENCY),
        Edge("die_sache_selbst", "spiritual_animal_kingdom", EdgeType.DEPENDENCY),
        Edge("die_sache_selbst", "honest_consciousness", EdgeType.NEGATION),
        Edge("deception", "honest_consciousness", EdgeType.NEGATION),
        Edge("deception", "die_sache_selbst", EdgeType.DEPENDENCY),
        Edge("legislative_reason", "die_sache_selbst", EdgeType.DEPENDENCY),
        Edge("law_testing_reason", "legislative_reason", EdgeType.NEGATION),
        Edge("ethical_substance_intro", "law_testing_reason", EdgeType.DEPENDENCY),
        Edge("ethical_substance_intro", "individual_universal_unity", EdgeType.DEPENDENCY),
        Edge("individual_universal_unity", "die_sache_selbst", EdgeType.SUBLATION),
        # Cross-chapter
        Edge("spiritual_animal_kingdom", "self_conscious_individuality", EdgeType.REFERENCE),
        Edge("work_product", "work", EdgeType.REFERENCE),
    ]
    return "C.V.C. Individuality Real In and For Itself", vertices, edges


def ch_ethical_order():
    """C.VI.A. The Ethical Order (Sittlichkeit)."""
    vertices = [
        Vertex("ethical_substance", content="ethical substance (Sittlichkeit)"),
        Vertex("divine_law", content="divine law (family)"),
        Vertex("human_law", content="human law (state/polis)"),
        Vertex("family", content="the family"),
        Vertex("state_polis", content="the state (polis)"),
        Vertex("antigone", content="Antigone"),
        Vertex("creon", content="Creon"),
        Vertex("guilt", content="guilt (ethical action = guilt)"),
        Vertex("woman_man", content="woman and man (ethical shapes)"),
        Vertex("brother_sister", content="brother-sister relation (pure recognition)"),
        Vertex("war", content="war (negation of ethical order)"),
        Vertex("ethical_destruction", content="destruction of ethical life"),
    ]
    edges = [
        Edge("ethical_substance", "ethical_substance_intro", EdgeType.DEPENDENCY),
        Edge("ethical_substance", "spirit", EdgeType.DEPENDENCY),
        Edge("divine_law", "ethical_substance", EdgeType.DEPENDENCY),
        Edge("human_law", "ethical_substance", EdgeType.DEPENDENCY),
        Edge("divine_law", "human_law", EdgeType.NEGATION),
        Edge("human_law", "divine_law", EdgeType.NEGATION),
        Edge("family", "divine_law", EdgeType.DEPENDENCY),
        Edge("state_polis", "human_law", EdgeType.DEPENDENCY),
        Edge("antigone", "divine_law", EdgeType.DEPENDENCY),
        Edge("creon", "human_law", EdgeType.DEPENDENCY),
        Edge("antigone", "creon", EdgeType.NEGATION),
        Edge("creon", "antigone", EdgeType.NEGATION),
        Edge("guilt", "antigone", EdgeType.DEPENDENCY),
        Edge("guilt", "creon", EdgeType.DEPENDENCY),
        Edge("woman_man", "family", EdgeType.DEPENDENCY),
        Edge("woman_man", "state_polis", EdgeType.DEPENDENCY),
        Edge("brother_sister", "family", EdgeType.DEPENDENCY),
        Edge("brother_sister", "recognition", EdgeType.REFERENCE),
        Edge("war", "state_polis", EdgeType.NEGATION),
        Edge("ethical_destruction", "divine_law", EdgeType.NEGATION),
        Edge("ethical_destruction", "human_law", EdgeType.NEGATION),
        Edge("ethical_destruction", "war", EdgeType.DEPENDENCY),
        Edge("ethical_destruction", "guilt", EdgeType.DEPENDENCY),
        # Cross-chapter
        Edge("ethical_substance", "individual_universal_unity", EdgeType.REFERENCE),
    ]
    return "C.VI.A. The Ethical Order", vertices, edges


def ch_culture():
    """C.VI.B. Culture (Bildung) — Spirit alienated from itself."""
    vertices = [
        Vertex("bildung", content="culture/education (Bildung)"),
        Vertex("noble_consciousness", content="noble consciousness"),
        Vertex("base_consciousness", content="base/ignoble consciousness"),
        Vertex("wealth", content="wealth"),
        Vertex("state_power", content="state power"),
        Vertex("language_flattery", content="language of flattery"),
        Vertex("language_disruption", content="language of disruption/wit"),
        Vertex("enlightenment", content="the Enlightenment"),
        Vertex("faith", content="faith"),
        Vertex("utility", content="utility (pure insight's object)"),
        Vertex("absolute_freedom", content="absolute freedom"),
        Vertex("terror", content="the Terror"),
        Vertex("self_alienation", content="self-alienation of spirit"),
        Vertex("pure_insight", content="pure insight"),
    ]
    edges = [
        Edge("bildung", "self_alienation", EdgeType.DEPENDENCY),
        Edge("self_alienation", "ethical_destruction", EdgeType.DEPENDENCY),
        Edge("noble_consciousness", "state_power", EdgeType.DEPENDENCY),
        Edge("base_consciousness", "wealth", EdgeType.DEPENDENCY),
        Edge("noble_consciousness", "base_consciousness", EdgeType.NEGATION),
        Edge("base_consciousness", "noble_consciousness", EdgeType.NEGATION),
        Edge("state_power", "wealth", EdgeType.NEGATION),
        Edge("wealth", "state_power", EdgeType.NEGATION),
        Edge("language_flattery", "noble_consciousness", EdgeType.DEPENDENCY),
        Edge("language_disruption", "language_flattery", EdgeType.NEGATION),
        Edge("language_disruption", "base_consciousness", EdgeType.DEPENDENCY),
        Edge("pure_insight", "language_disruption", EdgeType.DEPENDENCY),
        Edge("enlightenment", "pure_insight", EdgeType.DEPENDENCY),
        Edge("enlightenment", "faith", EdgeType.NEGATION),
        Edge("faith", "enlightenment", EdgeType.NEGATION),
        Edge("utility", "enlightenment", EdgeType.DEPENDENCY),
        Edge("absolute_freedom", "utility", EdgeType.DEPENDENCY),
        Edge("absolute_freedom", "universal", EdgeType.REFERENCE),
        Edge("terror", "absolute_freedom", EdgeType.NEGATION),
        Edge("terror", "absolute_freedom", EdgeType.DEPENDENCY),
        # Cross-chapter
        Edge("bildung", "ethical_substance", EdgeType.DEPENDENCY),
        Edge("bildung", "work", EdgeType.REFERENCE),
        Edge("faith", "unhappy_consciousness", EdgeType.REFERENCE),
    ]
    return "C.VI.B. Culture (Bildung)", vertices, edges


def ch_morality():
    """C.VI.C. Morality — from Kantian duty to conscience."""
    vertices = [
        Vertex("moral_worldview", content="moral world-view"),
        Vertex("duty", content="duty (Pflicht)"),
        Vertex("nature_morality", content="nature (amoral world)"),
        Vertex("postulates", content="postulates (God, immortality)"),
        Vertex("dissemblance", content="dissemblance (Verstellung)"),
        Vertex("conscience", content="conscience (Gewissen)"),
        Vertex("conviction", content="conviction"),
        Vertex("beautiful_soul", content="the beautiful soul"),
        Vertex("evil", content="evil (acting conscience)"),
        Vertex("judging_consciousness", content="judging consciousness"),
        Vertex("forgiveness", content="forgiveness"),
        Vertex("confession", content="confession"),
        Vertex("hard_heart", content="the hard heart"),
        Vertex("reconciliation_moral", content="reconciliation (moral)"),
    ]
    edges = [
        Edge("moral_worldview", "duty", EdgeType.DEPENDENCY),
        Edge("moral_worldview", "nature_morality", EdgeType.DEPENDENCY),
        Edge("duty", "nature_morality", EdgeType.NEGATION),
        Edge("nature_morality", "duty", EdgeType.NEGATION),
        Edge("postulates", "moral_worldview", EdgeType.DEPENDENCY),
        Edge("postulates", "duty", EdgeType.REFERENCE),
        Edge("dissemblance", "moral_worldview", EdgeType.NEGATION),
        Edge("dissemblance", "postulates", EdgeType.NEGATION),
        Edge("conscience", "duty", EdgeType.DEPENDENCY),
        Edge("conscience", "conviction", EdgeType.DEPENDENCY),
        Edge("conscience", "moral_worldview", EdgeType.NEGATION),
        Edge("beautiful_soul", "conscience", EdgeType.DEPENDENCY),
        Edge("beautiful_soul", "evil", EdgeType.NEGATION),
        Edge("evil", "beautiful_soul", EdgeType.NEGATION),
        Edge("evil", "conscience", EdgeType.DEPENDENCY),
        Edge("judging_consciousness", "beautiful_soul", EdgeType.DEPENDENCY),
        Edge("judging_consciousness", "evil", EdgeType.NEGATION),
        Edge("confession", "evil", EdgeType.DEPENDENCY),
        Edge("hard_heart", "judging_consciousness", EdgeType.DEPENDENCY),
        Edge("hard_heart", "confession", EdgeType.NEGATION),
        Edge("forgiveness", "hard_heart", EdgeType.NEGATION),
        Edge("forgiveness", "confession", EdgeType.DEPENDENCY),
        Edge("reconciliation_moral", "forgiveness", EdgeType.DEPENDENCY),
        Edge("reconciliation_moral", "confession", EdgeType.DEPENDENCY),
        Edge("reconciliation_moral", "evil", EdgeType.SUBLATION),
        Edge("reconciliation_moral", "judging_consciousness", EdgeType.SUBLATION),
        # Cross-chapter
        Edge("moral_worldview", "terror", EdgeType.DEPENDENCY),
        Edge("moral_worldview", "absolute_freedom", EdgeType.REFERENCE),
        Edge("conscience", "self_consciousness", EdgeType.REFERENCE),
        Edge("reconciliation_moral", "recognition", EdgeType.REFERENCE),
    ]
    return "C.VI.C. Morality", vertices, edges


def ch_religion():
    """C.VII. Religion — natural, art, revealed."""
    vertices = [
        Vertex("religion", content="religion"),
        Vertex("natural_religion", content="natural religion"),
        Vertex("light_being", content="light-being (Zoroastrian)"),
        Vertex("plant_animal_worship", content="plant and animal worship"),
        Vertex("artificer", content="the artificer (Egyptian)"),
        Vertex("art_religion", content="religion of art (Greek)"),
        Vertex("epic", content="epic (Homer)"),
        Vertex("tragedy", content="tragedy"),
        Vertex("comedy", content="comedy"),
        Vertex("revealed_religion", content="revealed religion (Christianity)"),
        Vertex("incarnation", content="incarnation"),
        Vertex("death_of_god", content="death of God"),
        Vertex("spiritual_community", content="spiritual community"),
        Vertex("trinity", content="Trinity (immanent dialectic)"),
        Vertex("picture_thinking", content="picture-thinking (Vorstellung)"),
    ]
    edges = [
        Edge("religion", "spirit", EdgeType.DEPENDENCY),
        Edge("religion", "ethical_substance", EdgeType.REFERENCE),
        Edge("natural_religion", "religion", EdgeType.DEPENDENCY),
        Edge("light_being", "natural_religion", EdgeType.DEPENDENCY),
        Edge("plant_animal_worship", "light_being", EdgeType.NEGATION),
        Edge("artificer", "plant_animal_worship", EdgeType.NEGATION),
        Edge("art_religion", "natural_religion", EdgeType.NEGATION),
        Edge("art_religion", "artificer", EdgeType.DEPENDENCY),
        Edge("epic", "art_religion", EdgeType.DEPENDENCY),
        Edge("tragedy", "epic", EdgeType.NEGATION),
        Edge("comedy", "tragedy", EdgeType.NEGATION),
        Edge("revealed_religion", "art_religion", EdgeType.NEGATION),
        Edge("revealed_religion", "religion", EdgeType.DEPENDENCY),
        Edge("incarnation", "revealed_religion", EdgeType.DEPENDENCY),
        Edge("death_of_god", "incarnation", EdgeType.NEGATION),
        Edge("spiritual_community", "death_of_god", EdgeType.DEPENDENCY),
        Edge("trinity", "revealed_religion", EdgeType.DEPENDENCY),
        Edge("picture_thinking", "revealed_religion", EdgeType.DEPENDENCY),
        Edge("picture_thinking", "science", EdgeType.NEGATION),
        # Cross-chapter
        Edge("revealed_religion", "reconciliation_moral", EdgeType.REFERENCE),
        Edge("revealed_religion", "unhappy_consciousness", EdgeType.REFERENCE),
        Edge("comedy", "ethical_destruction", EdgeType.REFERENCE),
        Edge("tragedy", "guilt", EdgeType.REFERENCE),
        Edge("incarnation", "self_consciousness", EdgeType.REFERENCE),
        Edge("death_of_god", "terror", EdgeType.REFERENCE),
    ]
    return "C.VII. Religion", vertices, edges


def ch_absolute_knowing():
    """C.VIII. Absolute Knowing — the culmination."""
    vertices = [
        Vertex("absolute_knowing", content="absolute knowing"),
        Vertex("concept", content="the Concept (Begriff)"),
        Vertex("time_history", content="time as history"),
        Vertex("science_of_experience", content="Science of the experience of consciousness"),
        Vertex("recollection", content="recollection (Er-Innerung)"),
        Vertex("calvary", content="Calvary of absolute Spirit"),
        Vertex("comprehended_history", content="comprehended history"),
        Vertex("substance_is_subject", content="substance IS subject (final)"),
        Vertex("spirit_knowing_itself", content="spirit knowing itself as spirit"),
    ]
    edges = [
        Edge("absolute_knowing", "concept", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "spirit_knowing_itself", EdgeType.DEPENDENCY),
        Edge("concept", "time_history", EdgeType.DEPENDENCY),
        Edge("science_of_experience", "absolute_knowing", EdgeType.DEPENDENCY),
        Edge("science_of_experience", "experience", EdgeType.DEPENDENCY),
        Edge("recollection", "absolute_knowing", EdgeType.DEPENDENCY),
        Edge("recollection", "time_history", EdgeType.DEPENDENCY),
        Edge("calvary", "absolute_knowing", EdgeType.DEPENDENCY),
        Edge("calvary", "death_of_god", EdgeType.REFERENCE),
        Edge("comprehended_history", "recollection", EdgeType.DEPENDENCY),
        Edge("comprehended_history", "time_history", EdgeType.DEPENDENCY),
        Edge("substance_is_subject", "substance", EdgeType.SUBLATION),
        Edge("substance_is_subject", "subject", EdgeType.SUBLATION),
        Edge("substance_is_subject", "absolute_knowing", EdgeType.DEPENDENCY),
        Edge("spirit_knowing_itself", "spirit", EdgeType.DEPENDENCY),
        Edge("spirit_knowing_itself", "science", EdgeType.DEPENDENCY),
        # Cross-chapter
        Edge("absolute_knowing", "revealed_religion", EdgeType.DEPENDENCY),
        Edge("absolute_knowing", "reconciliation_moral", EdgeType.REFERENCE),
        Edge("concept", "truth_whole", EdgeType.REFERENCE),
        Edge("science_of_experience", "natural_consciousness", EdgeType.REFERENCE),
        Edge("recollection", "sense_certainty", EdgeType.REFERENCE),
        Edge("absolute_knowing", "picture_thinking", EdgeType.NEGATION),
        Edge("spirit_knowing_itself", "self_consciousness", EdgeType.REFERENCE),
    ]
    return "C.VIII. Absolute Knowing", vertices, edges


# ---------------------------------------------------------------------------
# Full chapter list in order
# ---------------------------------------------------------------------------

ALL_CHAPTERS = [
    ch_preface,
    ch_introduction,
    ch_sense_certainty,
    ch_perception,
    ch_force_understanding,
    ch_lordship_bondage,
    ch_freedom_unhappy,
    ch_observing_reason,
    ch_actualization_reason,
    ch_individuality_real,
    ch_ethical_order,
    ch_culture,
    ch_morality,
    ch_religion,
    ch_absolute_knowing,
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    print("=" * 70)
    print("PHENOMENOLOGY OF SPIRIT — FULL INCREMENTAL TRAVERSAL")
    print("15 chapters, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    chapter_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for ch_idx, ch_fn in enumerate(ALL_CHAPTERS):
        chapter_name, ch_vertices, ch_edges = ch_fn()
        print(f"\n{'─' * 60}")
        print(f"Chapter {ch_idx + 1}/15: {chapter_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in ch_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                # Only add edge if both endpoints exist
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
            # Inject updated graph, preserving traversal state
            engine.k_active = graph
            engine.k_full = graph
            engine.terrain = compute_terrain(engine.k_active)
            # If position was lost (shouldn't happen with incremental add), reset
            if engine.position not in graph.vertices:
                engine.position = graph.active_vertex_ids()[0]

        # 4. Digest: run until beta_1 stable for 50 consecutive steps
        stable_count = 0
        last_beta = compute_beta_1(engine.k_active)
        steps_this_chapter = 0
        max_steps = 2000  # safety cap per chapter

        while stable_count < 50 and steps_this_chapter < max_steps:
            engine.run_step()
            steps_this_chapter += 1
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

        print(f"  Steps to digest: {steps_this_chapter} {'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} (delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        # Count operations in this chapter's digestion
        chapter_logs = engine.logs[-steps_this_chapter:] if steps_this_chapter > 0 else []
        op_counts = {}
        for log in chapter_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record
        result = {
            "chapter": chapter_name,
            "chapter_index": ch_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_chapter": blocked_count,
            "steps_to_digest": steps_this_chapter,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        chapter_results.append(result)

        beta_1_curve.append({
            "chapter_index": ch_idx + 1,
            "chapter_name": chapter_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        # Update graph reference for next chapter injection
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
    print(f"\n  beta_1 growth curve (by chapter):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    Ch{entry['chapter_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['chapter_name']}")

    # Biggest jumps
    deltas = [(r["chapter"], r["delta_beta_1"], r["chapter_index"]) for r in chapter_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by chapter:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    Ch{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by chapter:")
    for r in chapter_results:
        print(f"    Ch{r['chapter_index']:2d} {r['chapter']}: "
              f"{r['steps_to_digest']} steps {'OK' if r['converged'] else 'MAX'}")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({final_settled}):")
        for i, sc in enumerate(engine.settlement.settled_cycles[:20]):
            print(f"    [{i+1}] settled@step={sc.settled_at_step}: {sorted(sc.edges)[:3]}...")

    # Final terrain
    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"]) \
        if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0
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
        "experiment": "phenomenology_full_incremental",
        "source": "Hegel, Phenomenology of Spirit — all 15 chapters",
        "methodology": "manual dialectical complex per chapter, incremental injection, "
                        "digestion until 50-step beta_1 stability",
        "chapter_results": chapter_results,
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

    output_path = "experiment_phenomenology_full.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
