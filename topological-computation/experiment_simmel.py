"""Georg Simmel — complete works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Simmel's sociological
and philosophical thought as an incremental topological growth process.
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
#   form, content, sociation, interaction, reciprocity,
#   money, value, exchange, culture, tragedy_of_culture,
#   individuality, freedom, objectification, alienation,
#   stranger, distance, proximity, bridge, door,
#   life, more_life, more_than_life, death,
#   metropolis, mental_life, blase_attitude,
#   fashion, imitation, differentiation,
#   secrecy, trust, conflict, domination, subordination,
#   adventure, ruin, sociability, play
# ---------------------------------------------------------------------------


def work_social_differentiation():
    """1890: On Social Differentiation."""
    vertices = [
        Vertex("differentiation", content="social differentiation"),
        Vertex("individuality", content="individuality"),
        Vertex("form", content="form (social)"),
        Vertex("content", content="content (social)"),
        Vertex("interaction", content="interaction"),
        Vertex("group_size", content="group size"),
        Vertex("web_of_affiliations_proto", content="web of affiliations (proto)"),
        Vertex("crossing_social_circles", content="crossing of social circles"),
        Vertex("division_of_labor", content="division of labor"),
        Vertex("organic_solidarity_proto", content="organic solidarity (proto-concept)"),
        Vertex("quantitative_determination", content="quantitative determination of groups"),
    ]
    edges = [
        Edge("differentiation", "individuality", EdgeType.DEPENDENCY),
        Edge("differentiation", "group_size", EdgeType.DEPENDENCY),
        Edge("form", "content", EdgeType.NEGATION),
        Edge("interaction", "form", EdgeType.DEPENDENCY),
        Edge("crossing_social_circles", "individuality", EdgeType.DEPENDENCY),
        Edge("crossing_social_circles", "differentiation", EdgeType.DEPENDENCY),
        Edge("web_of_affiliations_proto", "crossing_social_circles", EdgeType.DEPENDENCY),
        Edge("division_of_labor", "differentiation", EdgeType.DEPENDENCY),
        Edge("division_of_labor", "group_size", EdgeType.REFERENCE),
        Edge("organic_solidarity_proto", "division_of_labor", EdgeType.DEPENDENCY),
        Edge("quantitative_determination", "group_size", EdgeType.DEPENDENCY),
        Edge("quantitative_determination", "differentiation", EdgeType.REFERENCE),
    ]
    return "1890: On Social Differentiation", vertices, edges


def work_philosophy_of_history_1():
    """1892: The Problems of the Philosophy of History (1st ed)."""
    vertices = [
        Vertex("historical_understanding", content="historical understanding"),
        Vertex("a_priori_forms_history", content="a priori forms of historical knowledge"),
        Vertex("historical_laws", content="historical laws"),
        Vertex("historical_individual", content="historical individual"),
        Vertex("re_experiencing", content="re-experiencing (Nacherleben)"),
        Vertex("objectivity_history", content="objectivity in history"),
        Vertex("fragmentary_character", content="fragmentary character of knowledge"),
        Vertex("historical_causation", content="historical causation"),
        Vertex("freedom_necessity_history", content="freedom vs necessity in history"),
    ]
    edges = [
        Edge("historical_understanding", "a_priori_forms_history", EdgeType.DEPENDENCY),
        Edge("historical_understanding", "re_experiencing", EdgeType.DEPENDENCY),
        Edge("a_priori_forms_history", "form", EdgeType.REFERENCE),
        Edge("historical_laws", "historical_causation", EdgeType.DEPENDENCY),
        Edge("historical_individual", "historical_understanding", EdgeType.DEPENDENCY),
        Edge("re_experiencing", "interaction", EdgeType.REFERENCE),
        Edge("objectivity_history", "a_priori_forms_history", EdgeType.DEPENDENCY),
        Edge("objectivity_history", "fragmentary_character", EdgeType.NEGATION),
        Edge("fragmentary_character", "historical_understanding", EdgeType.DEPENDENCY),
        Edge("freedom_necessity_history", "historical_laws", EdgeType.NEGATION),
        Edge("historical_causation", "historical_individual", EdgeType.DEPENDENCY),
    ]
    return "1892: Problems of the Philosophy of History (1st ed)", vertices, edges


def work_problem_of_sociology():
    """1894: The Problem of Sociology."""
    vertices = [
        Vertex("sociation", content="sociation (Vergesellschaftung)"),
        Vertex("reciprocity", content="reciprocity (Wechselwirkung)"),
        Vertex("sociology_as_method", content="sociology as method, not substance"),
        Vertex("social_geometry", content="social geometry"),
        Vertex("formal_sociology", content="formal sociology"),
        Vertex("dyad", content="dyad"),
        Vertex("triad", content="triad"),
    ]
    edges = [
        Edge("sociation", "interaction", EdgeType.DEPENDENCY),
        Edge("sociation", "form", EdgeType.DEPENDENCY),
        Edge("sociation", "reciprocity", EdgeType.DEPENDENCY),
        Edge("reciprocity", "interaction", EdgeType.SUBLATION),
        Edge("formal_sociology", "form", EdgeType.DEPENDENCY),
        Edge("formal_sociology", "content", EdgeType.NEGATION),
        Edge("formal_sociology", "sociology_as_method", EdgeType.DEPENDENCY),
        Edge("social_geometry", "form", EdgeType.REFERENCE),
        Edge("social_geometry", "quantitative_determination", EdgeType.REFERENCE),
        Edge("dyad", "sociation", EdgeType.DEPENDENCY),
        Edge("triad", "sociation", EdgeType.DEPENDENCY),
        Edge("triad", "dyad", EdgeType.NEGATION),
    ]
    return "1894: The Problem of Sociology", vertices, edges


def work_sociological_aesthetics():
    """1896: Sociological Aesthetics."""
    vertices = [
        Vertex("aesthetic_distance", content="aesthetic distance"),
        Vertex("symmetry", content="symmetry"),
        Vertex("aesthetic_form", content="aesthetic form"),
        Vertex("style", content="style"),
        Vertex("beauty_social", content="beauty as social phenomenon"),
        Vertex("aesthetic_play", content="aesthetic play"),
    ]
    edges = [
        Edge("aesthetic_distance", "distance", EdgeType.REFERENCE),
        Edge("aesthetic_form", "form", EdgeType.DEPENDENCY),
        Edge("aesthetic_form", "content", EdgeType.NEGATION),
        Edge("style", "aesthetic_form", EdgeType.DEPENDENCY),
        Edge("style", "individuality", EdgeType.NEGATION),
        Edge("beauty_social", "sociation", EdgeType.REFERENCE),
        Edge("beauty_social", "aesthetic_form", EdgeType.DEPENDENCY),
        Edge("symmetry", "reciprocity", EdgeType.REFERENCE),
        Edge("symmetry", "aesthetic_form", EdgeType.DEPENDENCY),
        Edge("aesthetic_play", "aesthetic_form", EdgeType.DEPENDENCY),
        Edge("aesthetic_play", "reciprocity", EdgeType.REFERENCE),
    ]
    return "1896: Sociological Aesthetics", vertices, edges


def work_persistence_social_groups():
    """1897: The Persistence of Social Groups."""
    vertices = [
        Vertex("group_persistence", content="persistence of social groups"),
        Vertex("group_boundary", content="group boundary"),
        Vertex("membership_change", content="membership change"),
        Vertex("group_identity", content="group identity across time"),
        Vertex("institutional_continuity", content="institutional continuity"),
        Vertex("objective_culture_proto", content="objective culture (proto)"),
    ]
    edges = [
        Edge("group_persistence", "sociation", EdgeType.DEPENDENCY),
        Edge("group_persistence", "group_identity", EdgeType.DEPENDENCY),
        Edge("group_boundary", "differentiation", EdgeType.REFERENCE),
        Edge("group_boundary", "group_persistence", EdgeType.DEPENDENCY),
        Edge("membership_change", "group_persistence", EdgeType.NEGATION),
        Edge("group_identity", "membership_change", EdgeType.NEGATION),
        Edge("institutional_continuity", "group_persistence", EdgeType.DEPENDENCY),
        Edge("institutional_continuity", "group_identity", EdgeType.DEPENDENCY),
        Edge("objective_culture_proto", "institutional_continuity", EdgeType.DEPENDENCY),
        Edge("objective_culture_proto", "form", EdgeType.REFERENCE),
    ]
    return "1897: The Persistence of Social Groups", vertices, edges


def work_philosophy_of_value():
    """1898: A Chapter in the Philosophy of Value."""
    vertices = [
        Vertex("value", content="value"),
        Vertex("desire", content="desire"),
        Vertex("distance_value", content="distance as condition of value"),
        Vertex("sacrifice", content="sacrifice"),
        Vertex("exchange", content="exchange"),
        Vertex("objective_value", content="objective value"),
        Vertex("subjective_value", content="subjective value"),
    ]
    edges = [
        Edge("value", "desire", EdgeType.DEPENDENCY),
        Edge("value", "distance_value", EdgeType.DEPENDENCY),
        Edge("distance_value", "desire", EdgeType.DEPENDENCY),
        Edge("sacrifice", "value", EdgeType.DEPENDENCY),
        Edge("sacrifice", "exchange", EdgeType.DEPENDENCY),
        Edge("exchange", "reciprocity", EdgeType.DEPENDENCY),
        Edge("exchange", "value", EdgeType.DEPENDENCY),
        Edge("objective_value", "subjective_value", EdgeType.NEGATION),
        Edge("objective_value", "exchange", EdgeType.DEPENDENCY),
        Edge("subjective_value", "desire", EdgeType.DEPENDENCY),
        Edge("subjective_value", "value", EdgeType.DEPENDENCY),
    ]
    return "1898: A Chapter in the Philosophy of Value", vertices, edges


def work_philosophy_of_money():
    """1900: The Philosophy of Money."""
    vertices = [
        Vertex("money", content="money"),
        Vertex("money_pure_means", content="money as pure means becoming end"),
        Vertex("money_leveler", content="money as leveler"),
        Vertex("objectification", content="objectification"),
        Vertex("culture", content="culture"),
        Vertex("alienation", content="alienation"),
        Vertex("freedom", content="freedom"),
        Vertex("personal_value_vs_money", content="personal value vs money value"),
        Vertex("cynicism", content="cynicism"),
        Vertex("blase_attitude", content="blasé attitude"),
        Vertex("trust", content="trust"),
        Vertex("abstract_exchange", content="abstract exchange"),
        Vertex("means_ends_reversal", content="means-ends reversal"),
    ]
    edges = [
        Edge("money", "exchange", EdgeType.DEPENDENCY),
        Edge("money", "value", EdgeType.DEPENDENCY),
        Edge("money", "trust", EdgeType.DEPENDENCY),
        Edge("money_pure_means", "money", EdgeType.DEPENDENCY),
        Edge("money_pure_means", "means_ends_reversal", EdgeType.DEPENDENCY),
        Edge("means_ends_reversal", "money", EdgeType.SUBLATION),
        Edge("money_leveler", "money", EdgeType.DEPENDENCY),
        Edge("money_leveler", "differentiation", EdgeType.NEGATION),
        Edge("objectification", "culture", EdgeType.DEPENDENCY),
        Edge("objectification", "money", EdgeType.DEPENDENCY),
        Edge("alienation", "objectification", EdgeType.DEPENDENCY),
        Edge("alienation", "freedom", EdgeType.NEGATION),
        Edge("freedom", "money", EdgeType.DEPENDENCY),
        Edge("freedom", "individuality", EdgeType.DEPENDENCY),
        Edge("personal_value_vs_money", "money_leveler", EdgeType.NEGATION),
        Edge("personal_value_vs_money", "individuality", EdgeType.DEPENDENCY),
        Edge("cynicism", "money_leveler", EdgeType.DEPENDENCY),
        Edge("cynicism", "value", EdgeType.NEGATION),
        Edge("blase_attitude", "cynicism", EdgeType.DEPENDENCY),
        Edge("blase_attitude", "money_leveler", EdgeType.DEPENDENCY),
        Edge("abstract_exchange", "money", EdgeType.DEPENDENCY),
        Edge("abstract_exchange", "reciprocity", EdgeType.REFERENCE),
    ]
    return "1900: The Philosophy of Money", vertices, edges


def work_metropolis():
    """1903: The Metropolis and Mental Life."""
    vertices = [
        Vertex("metropolis", content="metropolis"),
        Vertex("mental_life", content="mental life"),
        Vertex("intensification_nervous_stimulation", content="intensification of nervous stimulation"),
        Vertex("intellectualism", content="intellectualism (metropolitan)"),
        Vertex("reserve", content="reserve (Reserviertheit)"),
        Vertex("distance", content="distance"),
        Vertex("proximity", content="proximity"),
        Vertex("money_economy_city", content="money economy in the city"),
        Vertex("punctuality", content="punctuality"),
        Vertex("calculability", content="calculability"),
        Vertex("atrophy_individual_culture", content="atrophy of individual culture"),
    ]
    edges = [
        Edge("metropolis", "mental_life", EdgeType.DEPENDENCY),
        Edge("mental_life", "intensification_nervous_stimulation", EdgeType.DEPENDENCY),
        Edge("intensification_nervous_stimulation", "blase_attitude", EdgeType.DEPENDENCY),
        Edge("intellectualism", "intensification_nervous_stimulation", EdgeType.DEPENDENCY),
        Edge("intellectualism", "blase_attitude", EdgeType.REFERENCE),
        Edge("reserve", "blase_attitude", EdgeType.DEPENDENCY),
        Edge("reserve", "distance", EdgeType.DEPENDENCY),
        Edge("distance", "proximity", EdgeType.NEGATION),
        Edge("proximity", "distance", EdgeType.NEGATION),
        Edge("money_economy_city", "money", EdgeType.DEPENDENCY),
        Edge("money_economy_city", "metropolis", EdgeType.DEPENDENCY),
        Edge("money_economy_city", "calculability", EdgeType.DEPENDENCY),
        Edge("punctuality", "money_economy_city", EdgeType.DEPENDENCY),
        Edge("calculability", "intellectualism", EdgeType.DEPENDENCY),
        Edge("atrophy_individual_culture", "objectification", EdgeType.DEPENDENCY),
        Edge("atrophy_individual_culture", "metropolis", EdgeType.DEPENDENCY),
        Edge("atrophy_individual_culture", "individuality", EdgeType.NEGATION),
        Edge("freedom", "metropolis", EdgeType.REFERENCE),
    ]
    return "1903: The Metropolis and Mental Life", vertices, edges


def work_fashion():
    """1904: Fashion."""
    vertices = [
        Vertex("fashion", content="fashion"),
        Vertex("imitation", content="imitation"),
        Vertex("fashion_cycle", content="fashion cycle"),
        Vertex("class_distinction", content="class distinction"),
        Vertex("trickle_down", content="trickle-down"),
        Vertex("conformity", content="conformity"),
        Vertex("fashion_as_form", content="fashion as social form"),
    ]
    edges = [
        Edge("fashion", "imitation", EdgeType.DEPENDENCY),
        Edge("fashion", "differentiation", EdgeType.DEPENDENCY),
        Edge("fashion", "conformity", EdgeType.DEPENDENCY),
        Edge("imitation", "conformity", EdgeType.DEPENDENCY),
        Edge("imitation", "differentiation", EdgeType.NEGATION),
        Edge("differentiation", "imitation", EdgeType.NEGATION),
        Edge("fashion_cycle", "fashion", EdgeType.DEPENDENCY),
        Edge("fashion_cycle", "trickle_down", EdgeType.DEPENDENCY),
        Edge("class_distinction", "differentiation", EdgeType.DEPENDENCY),
        Edge("class_distinction", "fashion", EdgeType.REFERENCE),
        Edge("trickle_down", "class_distinction", EdgeType.DEPENDENCY),
        Edge("trickle_down", "imitation", EdgeType.DEPENDENCY),
        Edge("conformity", "individuality", EdgeType.NEGATION),
        Edge("fashion_as_form", "form", EdgeType.DEPENDENCY),
        Edge("fashion_as_form", "fashion", EdgeType.DEPENDENCY),
    ]
    return "1904: Fashion", vertices, edges


def work_sociology_of_religion():
    """1905: A Contribution to the Sociology of Religion."""
    vertices = [
        Vertex("religious_form", content="religious form"),
        Vertex("faith_social", content="faith as social relation"),
        Vertex("piety", content="piety"),
        Vertex("unity_religious", content="religious unity"),
        Vertex("sacred_profane", content="sacred vs profane"),
    ]
    edges = [
        Edge("religious_form", "form", EdgeType.DEPENDENCY),
        Edge("religious_form", "sociation", EdgeType.REFERENCE),
        Edge("faith_social", "trust", EdgeType.DEPENDENCY),
        Edge("faith_social", "reciprocity", EdgeType.REFERENCE),
        Edge("piety", "faith_social", EdgeType.DEPENDENCY),
        Edge("unity_religious", "sociation", EdgeType.DEPENDENCY),
        Edge("unity_religious", "piety", EdgeType.DEPENDENCY),
        Edge("sacred_profane", "form", EdgeType.REFERENCE),
        Edge("sacred_profane", "content", EdgeType.NEGATION),
    ]
    return "1905: A Contribution to the Sociology of Religion", vertices, edges


def work_sociology_of_secrecy():
    """1906: The Sociology of Secrecy and of Secret Societies."""
    vertices = [
        Vertex("secrecy", content="secrecy"),
        Vertex("secret_society", content="secret society"),
        Vertex("concealment", content="concealment"),
        Vertex("revelation", content="revelation"),
        Vertex("lying", content="lying"),
        Vertex("confidence", content="confidence"),
        Vertex("adornment", content="adornment"),
        Vertex("written_communication", content="written communication"),
        Vertex("discretion", content="discretion"),
    ]
    edges = [
        Edge("secrecy", "trust", EdgeType.NEGATION),
        Edge("secrecy", "sociation", EdgeType.DEPENDENCY),
        Edge("secrecy", "concealment", EdgeType.DEPENDENCY),
        Edge("concealment", "revelation", EdgeType.NEGATION),
        Edge("revelation", "concealment", EdgeType.NEGATION),
        Edge("secret_society", "secrecy", EdgeType.DEPENDENCY),
        Edge("secret_society", "group_boundary", EdgeType.DEPENDENCY),
        Edge("lying", "secrecy", EdgeType.DEPENDENCY),
        Edge("lying", "trust", EdgeType.NEGATION),
        Edge("confidence", "trust", EdgeType.DEPENDENCY),
        Edge("confidence", "secrecy", EdgeType.NEGATION),
        Edge("adornment", "distance", EdgeType.DEPENDENCY),
        Edge("adornment", "form", EdgeType.REFERENCE),
        Edge("written_communication", "distance", EdgeType.DEPENDENCY),
        Edge("written_communication", "secrecy", EdgeType.REFERENCE),
        Edge("discretion", "distance", EdgeType.DEPENDENCY),
        Edge("discretion", "secrecy", EdgeType.REFERENCE),
    ]
    return "1906: The Sociology of Secrecy and of Secret Societies", vertices, edges


def work_philosophy_of_money_2nd():
    """1907: The Philosophy of Money (2nd ed, expanded)."""
    vertices = [
        Vertex("money_symbolism", content="money as symbol"),
        Vertex("credit", content="credit"),
        Vertex("money_and_freedom_expanded", content="money-freedom relation expanded"),
        Vertex("impersonal_relations", content="impersonal relations"),
        Vertex("style_of_life", content="style of life"),
    ]
    edges = [
        Edge("money_symbolism", "money", EdgeType.DEPENDENCY),
        Edge("money_symbolism", "abstract_exchange", EdgeType.DEPENDENCY),
        Edge("credit", "money", EdgeType.DEPENDENCY),
        Edge("credit", "trust", EdgeType.DEPENDENCY),
        Edge("money_and_freedom_expanded", "freedom", EdgeType.DEPENDENCY),
        Edge("money_and_freedom_expanded", "money", EdgeType.DEPENDENCY),
        Edge("money_and_freedom_expanded", "alienation", EdgeType.NEGATION),
        Edge("impersonal_relations", "money_leveler", EdgeType.DEPENDENCY),
        Edge("impersonal_relations", "reserve", EdgeType.REFERENCE),
        Edge("style_of_life", "money", EdgeType.DEPENDENCY),
        Edge("style_of_life", "culture", EdgeType.DEPENDENCY),
        Edge("style_of_life", "fashion", EdgeType.REFERENCE),
    ]
    return "1907: The Philosophy of Money (2nd ed, expanded)", vertices, edges


def work_sociology_main():
    """1908: Sociology: Investigations on the Forms of Sociation (main work)."""
    vertices = [
        Vertex("superordination", content="superordination"),
        Vertex("self_preservation_group", content="self-preservation of the group"),
        Vertex("number_social", content="quantitative aspects of the group"),
        Vertex("spatial_sociology", content="sociology of space"),
        Vertex("boundary_spatial", content="spatial boundary"),
        Vertex("fixity_spatial", content="spatial fixity"),
        Vertex("social_types", content="social types"),
        Vertex("intersection_circles", content="intersection of social circles"),
        Vertex("expansion_group", content="expansion of the group"),
        Vertex("nobility", content="nobility (social type)"),
    ]
    edges = [
        Edge("superordination", "domination", EdgeType.DEPENDENCY),
        Edge("superordination", "subordination", EdgeType.NEGATION),
        Edge("self_preservation_group", "group_persistence", EdgeType.DEPENDENCY),
        Edge("self_preservation_group", "sociation", EdgeType.DEPENDENCY),
        Edge("number_social", "dyad", EdgeType.REFERENCE),
        Edge("number_social", "triad", EdgeType.REFERENCE),
        Edge("number_social", "group_size", EdgeType.DEPENDENCY),
        Edge("spatial_sociology", "form", EdgeType.DEPENDENCY),
        Edge("spatial_sociology", "boundary_spatial", EdgeType.DEPENDENCY),
        Edge("boundary_spatial", "group_boundary", EdgeType.REFERENCE),
        Edge("fixity_spatial", "spatial_sociology", EdgeType.DEPENDENCY),
        Edge("social_types", "form", EdgeType.DEPENDENCY),
        Edge("social_types", "sociation", EdgeType.DEPENDENCY),
        Edge("intersection_circles", "crossing_social_circles", EdgeType.SUBLATION),
        Edge("intersection_circles", "individuality", EdgeType.DEPENDENCY),
        Edge("expansion_group", "differentiation", EdgeType.DEPENDENCY),
        Edge("expansion_group", "individuality", EdgeType.DEPENDENCY),
        Edge("nobility", "social_types", EdgeType.DEPENDENCY),
        Edge("nobility", "class_distinction", EdgeType.REFERENCE),
    ]
    return "1908: Sociology — Forms of Sociation", vertices, edges


def work_the_stranger():
    """1908: The Stranger (excursus from Sociology)."""
    vertices = [
        Vertex("stranger", content="the stranger"),
        Vertex("wanderer", content="the wanderer"),
        Vertex("trader", content="the trader (stranger as)"),
        Vertex("objectivity_stranger", content="objectivity of the stranger"),
        Vertex("nearness_remoteness", content="nearness and remoteness (unity)"),
    ]
    edges = [
        Edge("stranger", "distance", EdgeType.DEPENDENCY),
        Edge("stranger", "proximity", EdgeType.DEPENDENCY),
        Edge("stranger", "nearness_remoteness", EdgeType.DEPENDENCY),
        Edge("nearness_remoteness", "distance", EdgeType.SUBLATION),
        Edge("nearness_remoteness", "proximity", EdgeType.SUBLATION),
        Edge("wanderer", "stranger", EdgeType.DEPENDENCY),
        Edge("trader", "stranger", EdgeType.DEPENDENCY),
        Edge("trader", "money", EdgeType.REFERENCE),
        Edge("objectivity_stranger", "stranger", EdgeType.DEPENDENCY),
        Edge("objectivity_stranger", "freedom", EdgeType.REFERENCE),
        Edge("stranger", "sociation", EdgeType.DEPENDENCY),
    ]
    return "1908: The Stranger", vertices, edges


def work_the_poor():
    """1908: The Poor (excursus from Sociology)."""
    vertices = [
        Vertex("the_poor", content="the poor"),
        Vertex("assistance", content="assistance (social obligation)"),
        Vertex("right_to_assistance", content="right to assistance"),
        Vertex("poverty_relation", content="poverty as social relation"),
    ]
    edges = [
        Edge("the_poor", "sociation", EdgeType.DEPENDENCY),
        Edge("the_poor", "subordination", EdgeType.REFERENCE),
        Edge("assistance", "the_poor", EdgeType.DEPENDENCY),
        Edge("assistance", "reciprocity", EdgeType.NEGATION),
        Edge("right_to_assistance", "assistance", EdgeType.DEPENDENCY),
        Edge("right_to_assistance", "the_poor", EdgeType.DEPENDENCY),
        Edge("poverty_relation", "the_poor", EdgeType.DEPENDENCY),
        Edge("poverty_relation", "sociation", EdgeType.DEPENDENCY),
        Edge("poverty_relation", "money_leveler", EdgeType.REFERENCE),
    ]
    return "1908: The Poor", vertices, edges


def work_conflict():
    """1908: Conflict (from Sociology)."""
    vertices = [
        Vertex("conflict", content="conflict (Streit)"),
        Vertex("conflict_as_sociation", content="conflict as form of sociation"),
        Vertex("competition", content="competition"),
        Vertex("jealousy", content="jealousy"),
        Vertex("unity_of_opposites_conflict", content="unity produced by conflict"),
        Vertex("peace", content="peace"),
    ]
    edges = [
        Edge("conflict", "sociation", EdgeType.SUBLATION),
        Edge("conflict_as_sociation", "conflict", EdgeType.DEPENDENCY),
        Edge("conflict_as_sociation", "sociation", EdgeType.DEPENDENCY),
        Edge("conflict_as_sociation", "reciprocity", EdgeType.DEPENDENCY),
        Edge("competition", "conflict", EdgeType.DEPENDENCY),
        Edge("competition", "differentiation", EdgeType.DEPENDENCY),
        Edge("jealousy", "conflict", EdgeType.DEPENDENCY),
        Edge("unity_of_opposites_conflict", "conflict", EdgeType.SUBLATION),
        Edge("peace", "conflict", EdgeType.NEGATION),
        Edge("conflict", "peace", EdgeType.NEGATION),
    ]
    return "1908: Conflict", vertices, edges


def work_domination():
    """1908: Domination and Subordination (from Sociology)."""
    vertices = [
        Vertex("domination", content="domination (Herrschaft)"),
        Vertex("subordination", content="subordination"),
        Vertex("authority", content="authority"),
        Vertex("legitimacy", content="legitimacy"),
        Vertex("domination_by_plurality", content="domination by a plurality"),
        Vertex("leveling", content="leveling"),
        Vertex("primus_inter_pares", content="primus inter pares"),
    ]
    edges = [
        Edge("domination", "subordination", EdgeType.NEGATION),
        Edge("subordination", "domination", EdgeType.NEGATION),
        Edge("domination", "sociation", EdgeType.DEPENDENCY),
        Edge("domination", "reciprocity", EdgeType.DEPENDENCY),
        Edge("authority", "domination", EdgeType.DEPENDENCY),
        Edge("authority", "legitimacy", EdgeType.DEPENDENCY),
        Edge("legitimacy", "trust", EdgeType.DEPENDENCY),
        Edge("domination_by_plurality", "domination", EdgeType.DEPENDENCY),
        Edge("domination_by_plurality", "triad", EdgeType.REFERENCE),
        Edge("leveling", "domination", EdgeType.NEGATION),
        Edge("leveling", "money_leveler", EdgeType.REFERENCE),
        Edge("primus_inter_pares", "domination", EdgeType.DEPENDENCY),
        Edge("primus_inter_pares", "superordination", EdgeType.REFERENCE),
    ]
    return "1908: Domination and Subordination", vertices, edges


def work_web_of_affiliations():
    """1908: The Web of Group Affiliations (from Sociology)."""
    vertices = [
        Vertex("web_of_affiliations", content="web of group affiliations"),
        Vertex("multiple_group_membership", content="multiple group membership"),
        Vertex("unique_individual", content="unique individual (intersection point)"),
        Vertex("organic_membership", content="organic vs rational membership"),
        Vertex("concentric_circles", content="concentric circles"),
    ]
    edges = [
        Edge("web_of_affiliations", "crossing_social_circles", EdgeType.SUBLATION),
        Edge("web_of_affiliations", "intersection_circles", EdgeType.DEPENDENCY),
        Edge("web_of_affiliations", "individuality", EdgeType.DEPENDENCY),
        Edge("multiple_group_membership", "web_of_affiliations", EdgeType.DEPENDENCY),
        Edge("multiple_group_membership", "differentiation", EdgeType.DEPENDENCY),
        Edge("unique_individual", "multiple_group_membership", EdgeType.DEPENDENCY),
        Edge("unique_individual", "individuality", EdgeType.SUBLATION),
        Edge("organic_membership", "web_of_affiliations", EdgeType.DEPENDENCY),
        Edge("concentric_circles", "web_of_affiliations", EdgeType.DEPENDENCY),
        Edge("concentric_circles", "group_boundary", EdgeType.REFERENCE),
    ]
    return "1908: The Web of Group Affiliations", vertices, edges


def work_bridge_and_door():
    """1909: Bridge and Door."""
    vertices = [
        Vertex("bridge", content="bridge"),
        Vertex("door", content="door"),
        Vertex("separation_connection", content="separation and connection"),
        Vertex("frame", content="frame (Rahmen)"),
        Vertex("threshold", content="threshold"),
    ]
    edges = [
        Edge("bridge", "separation_connection", EdgeType.SUBLATION),
        Edge("bridge", "proximity", EdgeType.DEPENDENCY),
        Edge("bridge", "distance", EdgeType.NEGATION),
        Edge("door", "separation_connection", EdgeType.SUBLATION),
        Edge("door", "distance", EdgeType.DEPENDENCY),
        Edge("door", "proximity", EdgeType.DEPENDENCY),
        Edge("separation_connection", "form", EdgeType.DEPENDENCY),
        Edge("frame", "form", EdgeType.DEPENDENCY),
        Edge("frame", "boundary_spatial", EdgeType.REFERENCE),
        Edge("threshold", "door", EdgeType.DEPENDENCY),
        Edge("threshold", "bridge", EdgeType.REFERENCE),
        Edge("bridge", "door", EdgeType.NEGATION),
        Edge("door", "bridge", EdgeType.NEGATION),
    ]
    return "1909: Bridge and Door", vertices, edges


def work_how_is_society_possible():
    """1910: How Is Society Possible?"""
    vertices = [
        Vertex("a_priori_sociation", content="a priori of sociation"),
        Vertex("generalization_individual", content="generalization of the individual"),
        Vertex("social_role", content="social role"),
        Vertex("vocation", content="vocation (Beruf)"),
        Vertex("incompleteness_knowledge", content="incompleteness of knowledge of other"),
        Vertex("society_possible", content="how is society possible"),
    ]
    edges = [
        Edge("a_priori_sociation", "sociation", EdgeType.DEPENDENCY),
        Edge("a_priori_sociation", "a_priori_forms_history", EdgeType.REFERENCE),
        Edge("generalization_individual", "individuality", EdgeType.NEGATION),
        Edge("generalization_individual", "social_types", EdgeType.DEPENDENCY),
        Edge("social_role", "form", EdgeType.DEPENDENCY),
        Edge("social_role", "sociation", EdgeType.DEPENDENCY),
        Edge("vocation", "social_role", EdgeType.DEPENDENCY),
        Edge("vocation", "unique_individual", EdgeType.DEPENDENCY),
        Edge("incompleteness_knowledge", "fragmentary_character", EdgeType.REFERENCE),
        Edge("incompleteness_knowledge", "trust", EdgeType.DEPENDENCY),
        Edge("society_possible", "a_priori_sociation", EdgeType.DEPENDENCY),
        Edge("society_possible", "incompleteness_knowledge", EdgeType.DEPENDENCY),
    ]
    return "1910: How Is Society Possible?", vertices, edges


def work_sociability():
    """1910: Sociability (Die Geselligkeit)."""
    vertices = [
        Vertex("sociability", content="sociability (Geselligkeit)"),
        Vertex("play", content="play (Spiel)"),
        Vertex("play_form", content="play-form of sociation"),
        Vertex("coquetry", content="coquetry"),
        Vertex("conversation", content="conversation (as play)"),
        Vertex("tact", content="tact"),
    ]
    edges = [
        Edge("sociability", "sociation", EdgeType.DEPENDENCY),
        Edge("sociability", "form", EdgeType.DEPENDENCY),
        Edge("sociability", "content", EdgeType.NEGATION),
        Edge("play", "sociability", EdgeType.DEPENDENCY),
        Edge("play", "aesthetic_play", EdgeType.REFERENCE),
        Edge("play_form", "sociability", EdgeType.DEPENDENCY),
        Edge("play_form", "form", EdgeType.SUBLATION),
        Edge("coquetry", "play_form", EdgeType.DEPENDENCY),
        Edge("coquetry", "distance", EdgeType.DEPENDENCY),
        Edge("coquetry", "proximity", EdgeType.DEPENDENCY),
        Edge("conversation", "sociability", EdgeType.DEPENDENCY),
        Edge("conversation", "reciprocity", EdgeType.DEPENDENCY),
        Edge("tact", "sociability", EdgeType.DEPENDENCY),
        Edge("tact", "discretion", EdgeType.REFERENCE),
    ]
    return "1910: Sociability", vertices, edges


def work_the_adventurer():
    """1911: The Adventurer."""
    vertices = [
        Vertex("adventure", content="the adventure"),
        Vertex("risk", content="risk"),
        Vertex("adventure_form", content="adventure as form"),
        Vertex("present_intensity", content="intensity of the present moment"),
        Vertex("gambler", content="the gambler"),
        Vertex("lover", content="the lover (adventurer)"),
    ]
    edges = [
        Edge("adventure", "form", EdgeType.DEPENDENCY),
        Edge("adventure", "life", EdgeType.DEPENDENCY),
        Edge("adventure", "risk", EdgeType.DEPENDENCY),
        Edge("adventure_form", "adventure", EdgeType.DEPENDENCY),
        Edge("adventure_form", "play_form", EdgeType.REFERENCE),
        Edge("present_intensity", "adventure", EdgeType.DEPENDENCY),
        Edge("present_intensity", "life", EdgeType.REFERENCE),
        Edge("gambler", "adventure", EdgeType.DEPENDENCY),
        Edge("gambler", "risk", EdgeType.DEPENDENCY),
        Edge("lover", "adventure", EdgeType.DEPENDENCY),
        Edge("lover", "coquetry", EdgeType.REFERENCE),
        Edge("risk", "trust", EdgeType.NEGATION),
    ]
    return "1911: The Adventurer", vertices, edges


def work_the_ruin():
    """1911: The Ruin."""
    vertices = [
        Vertex("ruin", content="the ruin"),
        Vertex("nature_spirit_ruin", content="nature vs spirit in ruin"),
        Vertex("past_present_ruin", content="past and present in ruin"),
        Vertex("decay_as_form", content="decay as aesthetic form"),
    ]
    edges = [
        Edge("ruin", "form", EdgeType.DEPENDENCY),
        Edge("ruin", "culture", EdgeType.REFERENCE),
        Edge("nature_spirit_ruin", "ruin", EdgeType.DEPENDENCY),
        Edge("nature_spirit_ruin", "objectification", EdgeType.NEGATION),
        Edge("past_present_ruin", "ruin", EdgeType.DEPENDENCY),
        Edge("past_present_ruin", "group_persistence", EdgeType.REFERENCE),
        Edge("decay_as_form", "ruin", EdgeType.DEPENDENCY),
        Edge("decay_as_form", "aesthetic_form", EdgeType.DEPENDENCY),
        Edge("decay_as_form", "life", EdgeType.NEGATION),
    ]
    return "1911: The Ruin", vertices, edges


def work_philosophical_culture():
    """1911: Philosophical Culture."""
    vertices = [
        Vertex("philosophical_culture", content="philosophical culture"),
        Vertex("subjective_culture", content="subjective culture"),
        Vertex("objective_culture", content="objective culture"),
        Vertex("female_culture", content="female culture"),
        Vertex("cultural_forms", content="cultural forms"),
    ]
    edges = [
        Edge("philosophical_culture", "culture", EdgeType.DEPENDENCY),
        Edge("subjective_culture", "individuality", EdgeType.DEPENDENCY),
        Edge("subjective_culture", "culture", EdgeType.DEPENDENCY),
        Edge("objective_culture", "objectification", EdgeType.DEPENDENCY),
        Edge("objective_culture", "culture", EdgeType.DEPENDENCY),
        Edge("objective_culture", "objective_culture_proto", EdgeType.SUBLATION),
        Edge("subjective_culture", "objective_culture", EdgeType.NEGATION),
        Edge("objective_culture", "subjective_culture", EdgeType.NEGATION),
        Edge("female_culture", "subjective_culture", EdgeType.DEPENDENCY),
        Edge("female_culture", "objective_culture", EdgeType.NEGATION),
        Edge("cultural_forms", "form", EdgeType.DEPENDENCY),
        Edge("cultural_forms", "culture", EdgeType.DEPENDENCY),
    ]
    return "1911: Philosophical Culture", vertices, edges


def work_tragedy_of_culture():
    """1912: The Concept and Tragedy of Culture."""
    vertices = [
        Vertex("tragedy_of_culture", content="the tragedy of culture"),
        Vertex("life", content="life (Leben)"),
        Vertex("more_life", content="more-life (Mehr-Leben)"),
        Vertex("more_than_life", content="more-than-life (Mehr-als-Leben)"),
        Vertex("cultural_lag", content="cultural lag (objective outpaces subjective)"),
        Vertex("spirit_objectified", content="objectified spirit"),
        Vertex("means_ends_culture", content="means-ends inversion in culture"),
    ]
    edges = [
        Edge("tragedy_of_culture", "culture", EdgeType.DEPENDENCY),
        Edge("tragedy_of_culture", "objective_culture", EdgeType.DEPENDENCY),
        Edge("tragedy_of_culture", "subjective_culture", EdgeType.NEGATION),
        Edge("life", "more_life", EdgeType.DEPENDENCY),
        Edge("more_life", "more_than_life", EdgeType.DEPENDENCY),
        Edge("more_than_life", "life", EdgeType.SUBLATION),
        Edge("more_than_life", "culture", EdgeType.DEPENDENCY),
        Edge("cultural_lag", "objective_culture", EdgeType.DEPENDENCY),
        Edge("cultural_lag", "subjective_culture", EdgeType.NEGATION),
        Edge("cultural_lag", "tragedy_of_culture", EdgeType.DEPENDENCY),
        Edge("spirit_objectified", "objectification", EdgeType.DEPENDENCY),
        Edge("spirit_objectified", "culture", EdgeType.DEPENDENCY),
        Edge("means_ends_culture", "means_ends_reversal", EdgeType.REFERENCE),
        Edge("means_ends_culture", "tragedy_of_culture", EdgeType.DEPENDENCY),
        Edge("alienation", "tragedy_of_culture", EdgeType.REFERENCE),
    ]
    return "1912: The Concept and Tragedy of Culture", vertices, edges


def work_goethe():
    """1913: Goethe."""
    vertices = [
        Vertex("goethe_unity", content="Goethe as unity of life and form"),
        Vertex("personality_totality", content="personality as totality"),
        Vertex("life_form_unity", content="unity of life and form"),
        Vertex("genius", content="genius"),
        Vertex("naivete", content="naiveté (Goethean)"),
    ]
    edges = [
        Edge("goethe_unity", "life", EdgeType.DEPENDENCY),
        Edge("goethe_unity", "form", EdgeType.DEPENDENCY),
        Edge("goethe_unity", "life_form_unity", EdgeType.DEPENDENCY),
        Edge("personality_totality", "individuality", EdgeType.SUBLATION),
        Edge("personality_totality", "goethe_unity", EdgeType.DEPENDENCY),
        Edge("life_form_unity", "life", EdgeType.SUBLATION),
        Edge("life_form_unity", "form", EdgeType.SUBLATION),
        Edge("life_form_unity", "tragedy_of_culture", EdgeType.NEGATION),
        Edge("genius", "personality_totality", EdgeType.DEPENDENCY),
        Edge("genius", "unique_individual", EdgeType.REFERENCE),
        Edge("naivete", "goethe_unity", EdgeType.DEPENDENCY),
    ]
    return "1913: Goethe", vertices, edges


def work_rembrandt():
    """1916: Rembrandt: An Essay in the Philosophy of Art."""
    vertices = [
        Vertex("rembrandt_individuality", content="Rembrandt's individuality (vs Renaissance type)"),
        Vertex("portrait_individuality", content="portrait as individuality"),
        Vertex("movement_art", content="movement in art"),
        Vertex("death", content="death"),
        Vertex("religious_art", content="religious art (inner life)"),
        Vertex("landscape_mood", content="landscape as mood"),
        Vertex("life_unity_art", content="life-unity in art"),
    ]
    edges = [
        Edge("rembrandt_individuality", "individuality", EdgeType.DEPENDENCY),
        Edge("rembrandt_individuality", "personality_totality", EdgeType.REFERENCE),
        Edge("portrait_individuality", "rembrandt_individuality", EdgeType.DEPENDENCY),
        Edge("portrait_individuality", "form", EdgeType.DEPENDENCY),
        Edge("movement_art", "life", EdgeType.DEPENDENCY),
        Edge("movement_art", "rembrandt_individuality", EdgeType.DEPENDENCY),
        Edge("death", "life", EdgeType.NEGATION),
        Edge("death", "more_than_life", EdgeType.REFERENCE),
        Edge("religious_art", "rembrandt_individuality", EdgeType.DEPENDENCY),
        Edge("religious_art", "subjective_culture", EdgeType.REFERENCE),
        Edge("landscape_mood", "rembrandt_individuality", EdgeType.DEPENDENCY),
        Edge("landscape_mood", "aesthetic_form", EdgeType.REFERENCE),
        Edge("life_unity_art", "life_form_unity", EdgeType.REFERENCE),
        Edge("life_unity_art", "rembrandt_individuality", EdgeType.DEPENDENCY),
    ]
    return "1916: Rembrandt", vertices, edges


def work_fundamental_questions():
    """1917: Fundamental Questions of Sociology."""
    vertices = [
        Vertex("individual_society_problem", content="the individual and society problem"),
        Vertex("sociological_a_priori", content="the three sociological a priori"),
        Vertex("social_type_generalized", content="social type (generalized)"),
        Vertex("vocation_generalized", content="vocation as a priori"),
        Vertex("society_as_more", content="society as more than sum of individuals"),
    ]
    edges = [
        Edge("individual_society_problem", "individuality", EdgeType.DEPENDENCY),
        Edge("individual_society_problem", "sociation", EdgeType.DEPENDENCY),
        Edge("sociological_a_priori", "a_priori_sociation", EdgeType.SUBLATION),
        Edge("sociological_a_priori", "society_possible", EdgeType.DEPENDENCY),
        Edge("social_type_generalized", "social_types", EdgeType.SUBLATION),
        Edge("social_type_generalized", "generalization_individual", EdgeType.DEPENDENCY),
        Edge("vocation_generalized", "vocation", EdgeType.SUBLATION),
        Edge("vocation_generalized", "sociological_a_priori", EdgeType.DEPENDENCY),
        Edge("society_as_more", "sociation", EdgeType.DEPENDENCY),
        Edge("society_as_more", "reciprocity", EdgeType.DEPENDENCY),
        Edge("society_as_more", "individual_society_problem", EdgeType.SUBLATION),
    ]
    return "1917: Fundamental Questions of Sociology", vertices, edges


def work_conflict_modern_culture():
    """1918: The Conflict in Modern Culture."""
    vertices = [
        Vertex("conflict_life_form", content="conflict between life and form"),
        Vertex("life_against_form", content="life against the principle of form"),
        Vertex("perpetual_revolution", content="perpetual revolution of life"),
        Vertex("formless_life", content="formless life (modern tendency)"),
        Vertex("expressionism", content="expressionism"),
        Vertex("pragmatism", content="pragmatism"),
    ]
    edges = [
        Edge("conflict_life_form", "life", EdgeType.DEPENDENCY),
        Edge("conflict_life_form", "form", EdgeType.DEPENDENCY),
        Edge("conflict_life_form", "tragedy_of_culture", EdgeType.SUBLATION),
        Edge("life_against_form", "conflict_life_form", EdgeType.DEPENDENCY),
        Edge("life_against_form", "form", EdgeType.NEGATION),
        Edge("perpetual_revolution", "conflict_life_form", EdgeType.DEPENDENCY),
        Edge("perpetual_revolution", "more_life", EdgeType.REFERENCE),
        Edge("formless_life", "life_against_form", EdgeType.DEPENDENCY),
        Edge("formless_life", "form", EdgeType.NEGATION),
        Edge("expressionism", "formless_life", EdgeType.DEPENDENCY),
        Edge("expressionism", "subjective_culture", EdgeType.REFERENCE),
        Edge("pragmatism", "formless_life", EdgeType.DEPENDENCY),
        Edge("pragmatism", "objective_value", EdgeType.NEGATION),
    ]
    return "1918: The Conflict in Modern Culture", vertices, edges


def work_lebensanschauung():
    """1918: Lebensanschauung (The View of Life)."""
    vertices = [
        Vertex("life_transcendence", content="life as self-transcendence"),
        Vertex("boundary_consciousness", content="consciousness of boundaries"),
        Vertex("individual_law", content="individual law (individuelles Gesetz)"),
        Vertex("turning_to_idea", content="the turning to the idea"),
        Vertex("death_boundary", content="death as life's own boundary"),
        Vertex("absolute_immanence", content="absolute and immanence"),
        Vertex("ought", content="the Ought (Sollen)"),
    ]
    edges = [
        Edge("life_transcendence", "life", EdgeType.SUBLATION),
        Edge("life_transcendence", "more_than_life", EdgeType.DEPENDENCY),
        Edge("life_transcendence", "form", EdgeType.NEGATION),
        Edge("boundary_consciousness", "life_transcendence", EdgeType.DEPENDENCY),
        Edge("boundary_consciousness", "distance", EdgeType.REFERENCE),
        Edge("individual_law", "individuality", EdgeType.DEPENDENCY),
        Edge("individual_law", "form", EdgeType.SUBLATION),
        Edge("individual_law", "ought", EdgeType.DEPENDENCY),
        Edge("turning_to_idea", "life_transcendence", EdgeType.DEPENDENCY),
        Edge("turning_to_idea", "more_than_life", EdgeType.REFERENCE),
        Edge("death_boundary", "death", EdgeType.DEPENDENCY),
        Edge("death_boundary", "life_transcendence", EdgeType.DEPENDENCY),
        Edge("death_boundary", "boundary_consciousness", EdgeType.DEPENDENCY),
        Edge("absolute_immanence", "life_transcendence", EdgeType.SUBLATION),
        Edge("absolute_immanence", "conflict_life_form", EdgeType.REFERENCE),
        Edge("ought", "individual_law", EdgeType.DEPENDENCY),
        Edge("ought", "form", EdgeType.REFERENCE),
    ]
    return "1918: Lebensanschauung (The View of Life)", vertices, edges


# ---------------------------------------------------------------------------
# Full works list in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_social_differentiation,            # 1890
    work_philosophy_of_history_1,           # 1892
    work_problem_of_sociology,              # 1894
    work_sociological_aesthetics,           # 1896
    work_persistence_social_groups,         # 1897
    work_philosophy_of_value,               # 1898
    work_philosophy_of_money,               # 1900
    work_metropolis,                        # 1903
    work_fashion,                           # 1904
    work_sociology_of_religion,             # 1905
    work_sociology_of_secrecy,              # 1906
    work_philosophy_of_money_2nd,           # 1907
    work_sociology_main,                    # 1908 main
    work_the_stranger,                      # 1908
    work_the_poor,                          # 1908
    work_conflict,                          # 1908
    work_domination,                        # 1908
    work_web_of_affiliations,               # 1908
    work_bridge_and_door,                   # 1909
    work_how_is_society_possible,           # 1910
    work_sociability,                       # 1910
    work_the_adventurer,                    # 1911
    work_the_ruin,                          # 1911
    work_philosophical_culture,             # 1911
    work_tragedy_of_culture,                # 1912
    work_goethe,                            # 1913
    work_rembrandt,                         # 1916
    work_fundamental_questions,             # 1917
    work_conflict_modern_culture,           # 1918
    work_lebensanschauung,                  # 1918
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("GEORG SIMMEL — COMPLETE WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "simmel_complete_works_incremental",
        "source": "Georg Simmel — complete works, 30 entries (1890-1918)",
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

    output_path = "experiment_simmel.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
