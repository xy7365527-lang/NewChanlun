"""Mao Zedong Full Corpus — 65-work incremental traversal experiment.

Extended from the 35-work version. Covers:
- Mao Zedong Selected Works (Xuanji) vols 1-5
- Mao Zedong Collected Works (Wenji) vols 1-8
- Important uncollected texts, speeches, and letters

Each work is injected as a dialectical complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

Works are ordered chronologically (1919-1976), modeling the evolution
of Mao's conceptual apparatus across nearly six decades.

Cross-work shared concepts (contradiction, practice, mass_line, etc.)
connect across works through shared vertex IDs.
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
# ---------------------------------------------------------------------------


def work_01_great_union():
    """1919: The Great Union of the Popular Masses —
    earliest political essay, mass mobilization, anti-imperialism."""
    vertices = [
        Vertex("mass_mobilization", content="mass mobilization — union of the popular masses"),
        Vertex("imperialism", content="imperialism — foreign domination"),
        Vertex("popular_masses", content="popular masses — workers, peasants, students, women"),
        Vertex("resistance", content="resistance — organized struggle against oppression"),
    ]
    edges = [
        Edge("mass_mobilization", "popular_masses", EdgeType.DEPENDENCY),
        Edge("mass_mobilization", "resistance", EdgeType.DEPENDENCY),
        Edge("resistance", "imperialism", EdgeType.NEGATION),
        Edge("popular_masses", "imperialism", EdgeType.NEGATION),
    ]
    return ("1919: The Great Union of the Popular Masses", vertices, edges)


def work_02_classes_in_chinese_society():
    """1925: Analysis of the Classes in Chinese Society —
    foundational class analysis, who are our enemies, who are our friends."""
    vertices = [
        Vertex("class_analysis", content="class analysis — who are our enemies, who are our friends"),
        Vertex("landlord_class", content="landlord class — feudal exploiters"),
        Vertex("comprador_bourgeoisie", content="comprador bourgeoisie — imperialist agents"),
        Vertex("national_bourgeoisie", content="national bourgeoisie — vacillating middle"),
        Vertex("petty_bourgeoisie", content="petty bourgeoisie — small proprietors"),
        Vertex("semi_proletariat", content="semi-proletariat — poor peasants"),
        Vertex("proletariat", content="proletariat — industrial workers"),
        Vertex("lumpenproletariat", content="lumpenproletariat — destructive if unled"),
        Vertex("revolution", content="revolution — overthrow of oppressor classes"),
        Vertex("enemy_friend", content="fundamental question: who are enemies, who are friends"),
    ]
    edges = [
        Edge("class_analysis", "enemy_friend", EdgeType.DEPENDENCY),
        Edge("landlord_class", "imperialism", EdgeType.DEPENDENCY),
        Edge("comprador_bourgeoisie", "imperialism", EdgeType.DEPENDENCY),
        Edge("national_bourgeoisie", "proletariat", EdgeType.NEGATION),
        Edge("national_bourgeoisie", "imperialism", EdgeType.NEGATION),
        Edge("proletariat", "landlord_class", EdgeType.NEGATION),
        Edge("proletariat", "comprador_bourgeoisie", EdgeType.NEGATION),
        Edge("semi_proletariat", "proletariat", EdgeType.REFERENCE),
        Edge("revolution", "class_analysis", EdgeType.DEPENDENCY),
        Edge("revolution", "proletariat", EdgeType.DEPENDENCY),
        Edge("lumpenproletariat", "revolution", EdgeType.REFERENCE),
        Edge("popular_masses", "proletariat", EdgeType.REFERENCE),
    ]
    return ("1925: Analysis of the Classes in Chinese Society", vertices, edges)


def work_03_national_revolution_peasant():
    """1926: The National Revolution and the Peasant Movement —
    peasantry as main force of revolution."""
    vertices = [
        Vertex("peasantry", content="peasantry — main force of Chinese revolution"),
        Vertex("national_revolution", content="national revolution against imperialism and feudalism"),
        Vertex("agrarian_revolution", content="agrarian revolution — land to the tiller"),
        Vertex("feudalism", content="feudalism — landlord exploitation of peasants"),
    ]
    edges = [
        Edge("peasantry", "revolution", EdgeType.DEPENDENCY),
        Edge("national_revolution", "imperialism", EdgeType.NEGATION),
        Edge("national_revolution", "feudalism", EdgeType.NEGATION),
        Edge("agrarian_revolution", "feudalism", EdgeType.NEGATION),
        Edge("agrarian_revolution", "peasantry", EdgeType.DEPENDENCY),
        Edge("peasantry", "proletariat", EdgeType.REFERENCE),
        Edge("peasantry", "semi_proletariat", EdgeType.REFERENCE),
    ]
    return ("1926: The National Revolution and the Peasant Movement", vertices, edges)


def work_04_hunan_report():
    """1927: Report on Investigation of Peasant Movement in Hunan —
    investigation and research, mass initiative, revolutionary violence."""
    vertices = [
        Vertex("investigation_research", content="investigation and research — no right to speak without it"),
        Vertex("mass_initiative", content="mass initiative — spontaneous revolutionary energy"),
        Vertex("revolutionary_violence", content="revolutionary violence — not a dinner party"),
        Vertex("peasant_association", content="peasant association — organizational form"),
        Vertex("rural_revolution", content="rural revolution — overthrowing local tyrants"),
    ]
    edges = [
        Edge("investigation_research", "class_analysis", EdgeType.DEPENDENCY),
        Edge("mass_initiative", "peasantry", EdgeType.DEPENDENCY),
        Edge("revolutionary_violence", "revolution", EdgeType.DEPENDENCY),
        Edge("revolutionary_violence", "landlord_class", EdgeType.NEGATION),
        Edge("peasant_association", "peasantry", EdgeType.DEPENDENCY),
        Edge("rural_revolution", "agrarian_revolution", EdgeType.DEPENDENCY),
        Edge("rural_revolution", "feudalism", EdgeType.NEGATION),
        Edge("mass_initiative", "revolution", EdgeType.REFERENCE),
    ]
    return ("1927: Report on Investigation of Peasant Movement in Hunan", vertices, edges)


def work_05_red_political_power():
    """1928: Why Is It That Red Political Power Can Exist in China? —
    base areas, encirclement from countryside, uneven development."""
    vertices = [
        Vertex("base_areas", content="revolutionary base areas — red political power in countryside"),
        Vertex("uneven_development", content="uneven development of Chinese society"),
        Vertex("encircle_cities", content="encircle the cities from the countryside"),
        Vertex("red_army", content="Red Army — armed force of revolution"),
        Vertex("white_regime", content="white regime — KMT/warlord reaction"),
    ]
    edges = [
        Edge("base_areas", "peasantry", EdgeType.DEPENDENCY),
        Edge("base_areas", "red_army", EdgeType.DEPENDENCY),
        Edge("uneven_development", "base_areas", EdgeType.DEPENDENCY),
        Edge("encircle_cities", "base_areas", EdgeType.DEPENDENCY),
        Edge("encircle_cities", "rural_revolution", EdgeType.REFERENCE),
        Edge("red_army", "revolution", EdgeType.DEPENDENCY),
        Edge("white_regime", "base_areas", EdgeType.NEGATION),
        Edge("white_regime", "imperialism", EdgeType.DEPENDENCY),
    ]
    return ("1928: Why Is It That Red Political Power Can Exist in China?", vertices, edges)


def work_06_struggle_jinggang():
    """1928: The Struggle in the Chingkang Mountains —
    land policy, military principles, base building."""
    vertices = [
        Vertex("land_policy_jinggang", content="land policy — confiscate and distribute landlord land"),
        Vertex("military_principles_early", content="early military principles — enemy advances we retreat"),
        Vertex("base_building", content="base area construction — political + military + economic"),
    ]
    edges = [
        Edge("land_policy_jinggang", "agrarian_revolution", EdgeType.DEPENDENCY),
        Edge("land_policy_jinggang", "peasantry", EdgeType.DEPENDENCY),
        Edge("military_principles_early", "red_army", EdgeType.DEPENDENCY),
        Edge("military_principles_early", "white_regime", EdgeType.NEGATION),
        Edge("base_building", "base_areas", EdgeType.DEPENDENCY),
        Edge("base_building", "mass_line", EdgeType.REFERENCE),
    ]
    return ("1928: The Struggle in the Chingkang Mountains", vertices, edges)


def work_07_correcting_mistaken_ideas():
    """1929: On Correcting Mistaken Ideas in the Party —
    rectification, party discipline, combating wrong tendencies."""
    vertices = [
        Vertex("rectification", content="rectification — correcting wrong ideas through criticism"),
        Vertex("criticism_self_criticism", content="criticism and self-criticism — party method"),
        Vertex("ultra_democracy", content="ultra-democracy — anarchist tendency"),
        Vertex("absolute_equalitarianism", content="absolute equalitarianism — petty-bourgeois tendency"),
        Vertex("purely_military_viewpoint", content="purely military viewpoint — army vs politics"),
        Vertex("party_discipline", content="party discipline — democratic centralism"),
    ]
    edges = [
        Edge("rectification", "criticism_self_criticism", EdgeType.DEPENDENCY),
        Edge("rectification", "ultra_democracy", EdgeType.NEGATION),
        Edge("rectification", "absolute_equalitarianism", EdgeType.NEGATION),
        Edge("rectification", "purely_military_viewpoint", EdgeType.NEGATION),
        Edge("party_discipline", "rectification", EdgeType.DEPENDENCY),
        Edge("ultra_democracy", "party_discipline", EdgeType.NEGATION),
        Edge("criticism_self_criticism", "class_analysis", EdgeType.REFERENCE),
    ]
    return ("1929: On Correcting Mistaken Ideas in the Party", vertices, edges)


def work_08_oppose_book_worship():
    """1930: Oppose Book Worship — no investigation no right to speak,
    against dogmatism, linking theory to practice."""
    vertices = [
        Vertex("oppose_bookism", content="oppose book worship — dogmatism negated"),
        Vertex("dogmatism", content="dogmatism — applying theory mechanically"),
        Vertex("theory_practice_link", content="linking theory to practice — unity of knowing and doing"),
        Vertex("social_investigation", content="social investigation — systematic inquiry into conditions"),
    ]
    edges = [
        Edge("oppose_bookism", "dogmatism", EdgeType.NEGATION),
        Edge("theory_practice_link", "dogmatism", EdgeType.NEGATION),
        Edge("theory_practice_link", "investigation_research", EdgeType.DEPENDENCY),
        Edge("social_investigation", "investigation_research", EdgeType.DEPENDENCY),
        Edge("social_investigation", "class_analysis", EdgeType.REFERENCE),
        Edge("oppose_bookism", "investigation_research", EdgeType.REFERENCE),
    ]
    return ("1930: Oppose Book Worship", vertices, edges)


def work_09_single_spark():
    """1930: A Single Spark Can Start a Prairie Fire —
    revolutionary optimism, dialectics of small and large, subjective forces."""
    vertices = [
        Vertex("single_spark", content="a single spark can start a prairie fire"),
        Vertex("revolutionary_optimism", content="revolutionary optimism — seeing the future in the present"),
        Vertex("subjective_forces", content="subjective forces — role of consciousness and will"),
    ]
    edges = [
        Edge("single_spark", "base_areas", EdgeType.DEPENDENCY),
        Edge("single_spark", "revolutionary_optimism", EdgeType.DEPENDENCY),
        Edge("revolutionary_optimism", "revolution", EdgeType.REFERENCE),
        Edge("subjective_forces", "revolution", EdgeType.DEPENDENCY),
        Edge("single_spark", "uneven_development", EdgeType.REFERENCE),
    ]
    return ("1930: A Single Spark Can Start a Prairie Fire", vertices, edges)


def work_10_xunwu_investigation():
    """1930: Xunwu Investigation —
    concrete social investigation, class composition of rural China."""
    vertices = [
        Vertex("xunwu_investigation", content="Xunwu investigation — exemplary social investigation"),
        Vertex("concrete_analysis", content="concrete analysis of concrete conditions"),
    ]
    edges = [
        Edge("xunwu_investigation", "social_investigation", EdgeType.DEPENDENCY),
        Edge("xunwu_investigation", "class_analysis", EdgeType.DEPENDENCY),
        Edge("concrete_analysis", "investigation_research", EdgeType.DEPENDENCY),
        Edge("concrete_analysis", "dogmatism", EdgeType.NEGATION),
    ]
    return ("1930: Xunwu Investigation", vertices, edges)


def work_11_counter_encirclement():
    """1931: How to Differentiate the Classes in the Rural Areas +
    strategic principles against encirclement campaigns."""
    vertices = [
        Vertex("rural_class_differentiation", content="differentiate rural classes — rich/middle/poor peasant, landlord"),
        Vertex("counter_encirclement", content="counter-encirclement campaigns — active defense"),
        Vertex("active_defense", content="active defense — offensive within strategic defensive"),
    ]
    edges = [
        Edge("rural_class_differentiation", "class_analysis", EdgeType.DEPENDENCY),
        Edge("rural_class_differentiation", "peasantry", EdgeType.DEPENDENCY),
        Edge("counter_encirclement", "red_army", EdgeType.DEPENDENCY),
        Edge("counter_encirclement", "white_regime", EdgeType.NEGATION),
        Edge("active_defense", "counter_encirclement", EdgeType.DEPENDENCY),
        Edge("active_defense", "military_principles_early", EdgeType.SUBLATION),
    ]
    return ("1931: Rural Class Differentiation + Counter-Encirclement", vertices, edges)


def work_12_well_being_of_masses():
    """1934: Be Concerned with the Well-Being of the Masses —
    mass line emerges, serving the people, connecting to masses."""
    vertices = [
        Vertex("mass_line", content="mass line — from the masses, to the masses"),
        Vertex("serve_the_people", content="serve the people — party's purpose"),
        Vertex("connecting_to_masses", content="connecting to masses — bureaucracy negated"),
        Vertex("bureaucratism", content="bureaucratism — separation from masses"),
    ]
    edges = [
        Edge("mass_line", "peasantry", EdgeType.DEPENDENCY),
        Edge("mass_line", "proletariat", EdgeType.DEPENDENCY),
        Edge("serve_the_people", "mass_line", EdgeType.DEPENDENCY),
        Edge("connecting_to_masses", "mass_line", EdgeType.DEPENDENCY),
        Edge("connecting_to_masses", "bureaucratism", EdgeType.NEGATION),
        Edge("bureaucratism", "mass_line", EdgeType.NEGATION),
        Edge("mass_line", "investigation_research", EdgeType.REFERENCE),
    ]
    return ("1934: Be Concerned with the Well-Being of the Masses", vertices, edges)


def work_13_tactics_against_japan():
    """1935: On Tactics Against Japanese Imperialism —
    united front, principal contradiction shifts, national contradiction."""
    vertices = [
        Vertex("united_front", content="united front — alliance of all anti-Japanese forces"),
        Vertex("principal_contradiction", content="principal contradiction — determines the main enemy"),
        Vertex("national_contradiction", content="national contradiction — China vs Japan"),
        Vertex("class_contradiction", content="class contradiction — internal class struggle"),
    ]
    edges = [
        Edge("united_front", "principal_contradiction", EdgeType.DEPENDENCY),
        Edge("principal_contradiction", "national_contradiction", EdgeType.DEPENDENCY),
        Edge("national_contradiction", "imperialism", EdgeType.DEPENDENCY),
        Edge("national_contradiction", "class_contradiction", EdgeType.NEGATION),
        Edge("class_contradiction", "class_analysis", EdgeType.REFERENCE),
        Edge("united_front", "national_bourgeoisie", EdgeType.REFERENCE),
        Edge("principal_contradiction", "class_analysis", EdgeType.REFERENCE),
    ]
    return ("1935: On Tactics Against Japanese Imperialism", vertices, edges)


def work_14_strategy_revolutionary_war():
    """1936: Problems of Strategy in China's Revolutionary War —
    strategic retreat, protracted war concepts emerge, annihilation vs attrition."""
    vertices = [
        Vertex("protracted_war", content="protracted war — strategic concept"),
        Vertex("strategic_retreat", content="strategic retreat — lure deep, concentrate forces"),
        Vertex("annihilation_strategy", content="annihilation — destroying enemy forces completely"),
        Vertex("attrition_strategy", content="attrition — wearing down enemy gradually"),
        Vertex("concentration_forces", content="concentrate a superior force to destroy enemy piecemeal"),
        Vertex("initiative_flexibility", content="initiative and flexibility in war"),
    ]
    edges = [
        Edge("protracted_war", "strategic_retreat", EdgeType.DEPENDENCY),
        Edge("protracted_war", "uneven_development", EdgeType.REFERENCE),
        Edge("strategic_retreat", "concentration_forces", EdgeType.DEPENDENCY),
        Edge("annihilation_strategy", "attrition_strategy", EdgeType.NEGATION),
        Edge("annihilation_strategy", "concentration_forces", EdgeType.DEPENDENCY),
        Edge("initiative_flexibility", "protracted_war", EdgeType.DEPENDENCY),
        Edge("protracted_war", "red_army", EdgeType.REFERENCE),
    ]
    return ("1936: Problems of Strategy in China's Revolutionary War", vertices, edges)


def work_15_on_practice():
    """1937: On Practice — epistemology, stages of cognition,
    practice as criterion of truth, unity of theory and practice."""
    vertices = [
        Vertex("practice", content="practice — social practice as basis and criterion of knowledge"),
        Vertex("perceptual_knowledge", content="perceptual knowledge — stage of sensation/impression"),
        Vertex("rational_knowledge", content="rational knowledge — stage of concepts/judgments/inferences"),
        Vertex("leap_to_rational", content="leap from perceptual to rational knowledge"),
        Vertex("leap_to_practice", content="second leap — from rational knowledge back to practice"),
        Vertex("criterion_of_truth", content="practice is the sole criterion of truth"),
        Vertex("dialectical_materialism", content="dialectical materialism — Marxist epistemology"),
    ]
    edges = [
        Edge("practice", "criterion_of_truth", EdgeType.DEPENDENCY),
        Edge("perceptual_knowledge", "practice", EdgeType.DEPENDENCY),
        Edge("rational_knowledge", "perceptual_knowledge", EdgeType.SUBLATION),
        Edge("leap_to_rational", "perceptual_knowledge", EdgeType.NEGATION),
        Edge("leap_to_rational", "rational_knowledge", EdgeType.DEPENDENCY),
        Edge("leap_to_practice", "rational_knowledge", EdgeType.NEGATION),
        Edge("leap_to_practice", "practice", EdgeType.DEPENDENCY),
        Edge("criterion_of_truth", "dogmatism", EdgeType.NEGATION),
        Edge("dialectical_materialism", "practice", EdgeType.DEPENDENCY),
        Edge("dialectical_materialism", "theory_practice_link", EdgeType.REFERENCE),
        Edge("practice", "investigation_research", EdgeType.REFERENCE),
    ]
    return ("1937: On Practice", vertices, edges)


def work_16_on_contradiction():
    """1937: On Contradiction — universality and particularity of contradiction,
    principal and secondary contradictions, identity and struggle of opposites."""
    vertices = [
        Vertex("contradiction", content="contradiction — unity of opposites, law of dialectics"),
        Vertex("universality_of_contradiction", content="universality — contradiction exists in all things"),
        Vertex("particularity_of_contradiction", content="particularity — each contradiction has specific character"),
        Vertex("principal_and_secondary", content="principal and secondary contradictions"),
        Vertex("principal_aspect", content="principal aspect of a contradiction — determines its nature"),
        Vertex("identity_of_opposites", content="identity of opposites — mutual dependence and transformation"),
        Vertex("struggle_of_opposites", content="struggle of opposites — absolute, driving change"),
        Vertex("antagonistic_contradiction", content="antagonistic contradiction — resolved by revolution"),
        Vertex("non_antagonistic_contradiction", content="non-antagonistic contradiction — resolved by criticism"),
        Vertex("transformation_of_opposites", content="transformation of opposites — conditions for reversal"),
    ]
    edges = [
        Edge("contradiction", "universality_of_contradiction", EdgeType.DEPENDENCY),
        Edge("contradiction", "particularity_of_contradiction", EdgeType.DEPENDENCY),
        Edge("universality_of_contradiction", "particularity_of_contradiction", EdgeType.NEGATION),
        Edge("principal_and_secondary", "contradiction", EdgeType.DEPENDENCY),
        Edge("principal_and_secondary", "principal_contradiction", EdgeType.REFERENCE),
        Edge("principal_aspect", "contradiction", EdgeType.DEPENDENCY),
        Edge("identity_of_opposites", "struggle_of_opposites", EdgeType.NEGATION),
        Edge("identity_of_opposites", "transformation_of_opposites", EdgeType.DEPENDENCY),
        Edge("struggle_of_opposites", "identity_of_opposites", EdgeType.REFERENCE),
        Edge("antagonistic_contradiction", "non_antagonistic_contradiction", EdgeType.NEGATION),
        Edge("antagonistic_contradiction", "revolution", EdgeType.REFERENCE),
        Edge("non_antagonistic_contradiction", "criticism_self_criticism", EdgeType.REFERENCE),
        Edge("contradiction", "dialectical_materialism", EdgeType.DEPENDENCY),
        Edge("transformation_of_opposites", "practice", EdgeType.REFERENCE),
    ]
    return ("1937: On Contradiction", vertices, edges)


def work_17_combat_liberalism():
    """1937: Combat Liberalism — eleven manifestations of liberalism,
    principled struggle vs unprincipled peace."""
    vertices = [
        Vertex("liberalism", content="liberalism — unprincipled peace, avoiding struggle"),
        Vertex("principled_struggle", content="principled struggle — ideological combat"),
    ]
    edges = [
        Edge("liberalism", "principled_struggle", EdgeType.NEGATION),
        Edge("principled_struggle", "criticism_self_criticism", EdgeType.DEPENDENCY),
        Edge("liberalism", "party_discipline", EdgeType.NEGATION),
        Edge("principled_struggle", "rectification", EdgeType.REFERENCE),
    ]
    return ("1937: Combat Liberalism", vertices, edges)


def work_18_national_self_defense_war():
    """1937: Urgent Tasks Following the Establishment of KMT-CPC Cooperation —
    war of national self-defense, independence within united front."""
    vertices = [
        Vertex("national_self_defense", content="war of national self-defense against Japan"),
        Vertex("independence_in_uf", content="independence and initiative within the united front"),
    ]
    edges = [
        Edge("national_self_defense", "united_front", EdgeType.DEPENDENCY),
        Edge("national_self_defense", "national_contradiction", EdgeType.DEPENDENCY),
        Edge("independence_in_uf", "united_front", EdgeType.DEPENDENCY),
        Edge("independence_in_uf", "party_leadership", EdgeType.REFERENCE),
    ]
    return ("1937: Urgent Tasks of KMT-CPC Cooperation", vertices, edges)


def work_19_on_protracted_war():
    """1938: On Protracted War — three stages, strategic defensive/stalemate/offensive,
    man over weapons, people's war."""
    vertices = [
        Vertex("three_stages", content="three stages of protracted war"),
        Vertex("strategic_defensive", content="strategic defensive — first stage, retreat and preserve"),
        Vertex("strategic_stalemate", content="strategic stalemate — second stage, guerrilla war expands"),
        Vertex("strategic_offensive", content="strategic offensive — third stage, counterattack"),
        Vertex("peoples_war", content="people's war — mobilize the whole nation"),
        Vertex("man_over_weapons", content="man is the decisive factor, not weapons"),
        Vertex("guerrilla_war", content="guerrilla warfare — flexible, mobile operations"),
    ]
    edges = [
        Edge("three_stages", "protracted_war", EdgeType.DEPENDENCY),
        Edge("strategic_defensive", "strategic_stalemate", EdgeType.DEPENDENCY),
        Edge("strategic_stalemate", "strategic_offensive", EdgeType.DEPENDENCY),
        Edge("strategic_defensive", "strategic_offensive", EdgeType.NEGATION),
        Edge("peoples_war", "mass_line", EdgeType.DEPENDENCY),
        Edge("peoples_war", "peasantry", EdgeType.DEPENDENCY),
        Edge("man_over_weapons", "subjective_forces", EdgeType.DEPENDENCY),
        Edge("guerrilla_war", "base_areas", EdgeType.DEPENDENCY),
        Edge("guerrilla_war", "peoples_war", EdgeType.REFERENCE),
        Edge("three_stages", "transformation_of_opposites", EdgeType.REFERENCE),
    ]
    return ("1938: On Protracted War", vertices, edges)


def work_20_guerrilla_strategy():
    """1938: Problems of Strategy in Guerrilla War —
    six problems of strategy, guerrilla vs regular operations."""
    vertices = [
        Vertex("guerrilla_strategy", content="guerrilla strategy — six strategic problems"),
        Vertex("defense_within_offense", content="strategic defensive contains tactical offensive"),
        Vertex("interior_exterior_lines", content="interior and exterior lines of operation"),
    ]
    edges = [
        Edge("guerrilla_strategy", "guerrilla_war", EdgeType.DEPENDENCY),
        Edge("guerrilla_strategy", "protracted_war", EdgeType.REFERENCE),
        Edge("defense_within_offense", "strategic_defensive", EdgeType.DEPENDENCY),
        Edge("defense_within_offense", "strategic_offensive", EdgeType.REFERENCE),
        Edge("interior_exterior_lines", "concentration_forces", EdgeType.REFERENCE),
        Edge("guerrilla_strategy", "initiative_flexibility", EdgeType.REFERENCE),
    ]
    return ("1938: Problems of Strategy in Guerrilla War", vertices, edges)


def work_21_role_of_ccp():
    """1938: The Role of the Chinese Communist Party in the National War —
    party leadership, cadre policy, three great styles of work."""
    vertices = [
        Vertex("party_leadership", content="CCP leadership — vanguard role in national war"),
        Vertex("three_styles_of_work", content="three great styles: theory-practice unity, mass connection, self-criticism"),
        Vertex("cadre_policy", content="cadre policy — select, train, promote cadres"),
        Vertex("self_reliance", content="self-reliance — rely mainly on own efforts"),
    ]
    edges = [
        Edge("party_leadership", "united_front", EdgeType.DEPENDENCY),
        Edge("three_styles_of_work", "theory_practice_link", EdgeType.DEPENDENCY),
        Edge("three_styles_of_work", "mass_line", EdgeType.DEPENDENCY),
        Edge("three_styles_of_work", "criticism_self_criticism", EdgeType.DEPENDENCY),
        Edge("cadre_policy", "party_leadership", EdgeType.DEPENDENCY),
        Edge("self_reliance", "subjective_forces", EdgeType.DEPENDENCY),
        Edge("party_leadership", "proletariat", EdgeType.REFERENCE),
    ]
    return ("1938: The Role of the CCP in the National War", vertices, edges)


def work_22_war_and_strategy():
    """1938: Problems of War and Strategy —
    political power grows out of the barrel of a gun."""
    vertices = [
        Vertex("power_from_gun", content="political power grows out of the barrel of a gun"),
        Vertex("party_commands_gun", content="the party commands the gun, the gun must never command the party"),
    ]
    edges = [
        Edge("power_from_gun", "revolution", EdgeType.DEPENDENCY),
        Edge("power_from_gun", "red_army", EdgeType.DEPENDENCY),
        Edge("party_commands_gun", "party_leadership", EdgeType.DEPENDENCY),
        Edge("party_commands_gun", "purely_military_viewpoint", EdgeType.NEGATION),
        Edge("party_commands_gun", "power_from_gun", EdgeType.DEPENDENCY),
    ]
    return ("1938: Problems of War and Strategy", vertices, edges)


def work_23_chinese_revolution_and_ccp():
    """1939: The Chinese Revolution and the Chinese Communist Party —
    two-stage revolution, semi-colonial semi-feudal analysis."""
    vertices = [
        Vertex("two_stage_revolution", content="two-stage revolution: new-democratic -> socialist"),
        Vertex("semi_colonial_semi_feudal", content="semi-colonial semi-feudal society — China's character"),
        Vertex("new_democratic_revolution", content="new-democratic revolution — bourgeois-democratic under proletarian leadership"),
    ]
    edges = [
        Edge("two_stage_revolution", "new_democratic_revolution", EdgeType.DEPENDENCY),
        Edge("semi_colonial_semi_feudal", "imperialism", EdgeType.DEPENDENCY),
        Edge("semi_colonial_semi_feudal", "feudalism", EdgeType.DEPENDENCY),
        Edge("new_democratic_revolution", "semi_colonial_semi_feudal", EdgeType.NEGATION),
        Edge("new_democratic_revolution", "proletariat", EdgeType.DEPENDENCY),
        Edge("new_democratic_revolution", "peasantry", EdgeType.REFERENCE),
        Edge("two_stage_revolution", "revolution", EdgeType.REFERENCE),
    ]
    return ("1939: The Chinese Revolution and the Chinese Communist Party", vertices, edges)


def work_24_introducing_the_communist():
    """1939: Introducing The Communist —
    three magic weapons: united front, armed struggle, party building."""
    vertices = [
        Vertex("three_magic_weapons", content="three magic weapons: united front, armed struggle, party building"),
        Vertex("armed_struggle", content="armed struggle — central form of revolution"),
        Vertex("party_building", content="party building — organizational guarantee"),
    ]
    edges = [
        Edge("three_magic_weapons", "united_front", EdgeType.DEPENDENCY),
        Edge("three_magic_weapons", "armed_struggle", EdgeType.DEPENDENCY),
        Edge("three_magic_weapons", "party_building", EdgeType.DEPENDENCY),
        Edge("armed_struggle", "power_from_gun", EdgeType.DEPENDENCY),
        Edge("party_building", "party_leadership", EdgeType.DEPENDENCY),
        Edge("party_building", "rectification", EdgeType.REFERENCE),
    ]
    return ("1939: Introducing The Communist", vertices, edges)


def work_25_on_new_democracy():
    """1940: On New Democracy — new-democratic politics/economy/culture,
    joint dictatorship, national form."""
    vertices = [
        Vertex("new_democracy", content="new democracy — joint dictatorship of revolutionary classes"),
        Vertex("new_democratic_politics", content="new-democratic politics — democratic centralism"),
        Vertex("new_democratic_economy", content="new-democratic economy — state/cooperative/private"),
        Vertex("new_democratic_culture", content="new-democratic culture — national, scientific, mass"),
        Vertex("joint_dictatorship", content="joint dictatorship of several revolutionary classes"),
        Vertex("old_democracy", content="old bourgeois democracy — superseded"),
    ]
    edges = [
        Edge("new_democracy", "new_democratic_revolution", EdgeType.DEPENDENCY),
        Edge("new_democracy", "old_democracy", EdgeType.SUBLATION),
        Edge("new_democratic_politics", "new_democracy", EdgeType.DEPENDENCY),
        Edge("new_democratic_economy", "new_democracy", EdgeType.DEPENDENCY),
        Edge("new_democratic_culture", "new_democracy", EdgeType.DEPENDENCY),
        Edge("joint_dictatorship", "united_front", EdgeType.DEPENDENCY),
        Edge("joint_dictatorship", "proletariat", EdgeType.DEPENDENCY),
        Edge("joint_dictatorship", "peasantry", EdgeType.DEPENDENCY),
        Edge("old_democracy", "new_democracy", EdgeType.NEGATION),
        Edge("new_democratic_culture", "mass_line", EdgeType.REFERENCE),
    ]
    return ("1940: On New Democracy", vertices, edges)


def work_26_current_situation_and_tasks():
    """1940: Current Problems of Tactics in the Anti-Japanese United Front —
    distinction between die-hards, middle forces, progressives."""
    vertices = [
        Vertex("three_forces", content="three political forces: die-hards, middle, progressives"),
        Vertex("struggle_with_diehards", content="struggle with die-hards — on just grounds, to advantage, with restraint"),
    ]
    edges = [
        Edge("three_forces", "united_front", EdgeType.DEPENDENCY),
        Edge("three_forces", "class_analysis", EdgeType.REFERENCE),
        Edge("struggle_with_diehards", "three_forces", EdgeType.DEPENDENCY),
        Edge("struggle_with_diehards", "principled_struggle", EdgeType.REFERENCE),
    ]
    return ("1940: Tactics in the Anti-Japanese United Front", vertices, edges)


def work_27_preface_rural_surveys():
    """1941: Preface and Postscript to Rural Surveys —
    investigation method refined, combating subjectivism."""
    vertices = [
        Vertex("rural_surveys", content="rural surveys — systematic investigation of countryside"),
    ]
    edges = [
        Edge("rural_surveys", "social_investigation", EdgeType.DEPENDENCY),
        Edge("rural_surveys", "xunwu_investigation", EdgeType.REFERENCE),
        Edge("rural_surveys", "concrete_analysis", EdgeType.REFERENCE),
    ]
    return ("1941: Preface and Postscript to Rural Surveys", vertices, edges)


def work_28_reform_our_study():
    """1941: Reform Our Study — subjectivism negated, Marxism sinified,
    seeking truth from facts."""
    vertices = [
        Vertex("subjectivism", content="subjectivism — divorced from practice"),
        Vertex("seeking_truth_from_facts", content="seeking truth from facts — shi shi qiu shi"),
        Vertex("sinification_of_marxism", content="sinification of Marxism — apply to Chinese conditions"),
        Vertex("empiricism", content="empiricism — narrow experience worship"),
    ]
    edges = [
        Edge("subjectivism", "dogmatism", EdgeType.REFERENCE),
        Edge("subjectivism", "empiricism", EdgeType.REFERENCE),
        Edge("seeking_truth_from_facts", "subjectivism", EdgeType.NEGATION),
        Edge("seeking_truth_from_facts", "practice", EdgeType.DEPENDENCY),
        Edge("seeking_truth_from_facts", "investigation_research", EdgeType.DEPENDENCY),
        Edge("sinification_of_marxism", "seeking_truth_from_facts", EdgeType.DEPENDENCY),
        Edge("sinification_of_marxism", "dogmatism", EdgeType.NEGATION),
        Edge("empiricism", "dogmatism", EdgeType.NEGATION),
    ]
    return ("1941: Reform Our Study", vertices, edges)


def work_29_rectify_party_style():
    """1942: Rectify the Party's Style of Work — three winds rectified:
    subjectivism in study, sectarianism in party relations, formalism in propaganda."""
    vertices = [
        Vertex("rectification_movement", content="rectification movement (Zhengfeng) — Yan'an 1942"),
        Vertex("sectarianism", content="sectarianism — faction above party"),
        Vertex("formalism_party_jargon", content="party formalism — stereotyped writing"),
        Vertex("unity_through_struggle", content="unity — criticism — unity formula"),
    ]
    edges = [
        Edge("rectification_movement", "rectification", EdgeType.DEPENDENCY),
        Edge("rectification_movement", "subjectivism", EdgeType.NEGATION),
        Edge("rectification_movement", "sectarianism", EdgeType.NEGATION),
        Edge("rectification_movement", "formalism_party_jargon", EdgeType.NEGATION),
        Edge("unity_through_struggle", "criticism_self_criticism", EdgeType.DEPENDENCY),
        Edge("unity_through_struggle", "non_antagonistic_contradiction", EdgeType.REFERENCE),
        Edge("rectification_movement", "sinification_of_marxism", EdgeType.REFERENCE),
    ]
    return ("1942: Rectify the Party's Style of Work", vertices, edges)


def work_30_oppose_stereotypes():
    """1942: Oppose Stereotyped Party Writing —
    eight charges against party jargon, lively style demanded."""
    vertices = [
        Vertex("oppose_stereotypes", content="oppose stereotyped party writing — eight indictments"),
        Vertex("lively_style", content="lively, fresh writing style — learn from the people's language"),
    ]
    edges = [
        Edge("oppose_stereotypes", "formalism_party_jargon", EdgeType.NEGATION),
        Edge("oppose_stereotypes", "rectification_movement", EdgeType.DEPENDENCY),
        Edge("lively_style", "mass_line", EdgeType.REFERENCE),
        Edge("lively_style", "oppose_stereotypes", EdgeType.DEPENDENCY),
    ]
    return ("1942: Oppose Stereotyped Party Writing", vertices, edges)


def work_31_yanan_forum():
    """1942: Talks at the Yan'an Forum on Literature and Art —
    art serves politics, workers-peasants-soldiers audience, popularization vs raising."""
    vertices = [
        Vertex("art_serves_politics", content="art and literature must serve the revolution"),
        Vertex("workers_peasants_soldiers", content="workers, peasants, soldiers — primary audience"),
        Vertex("popularization_vs_raising", content="popularization vs raising standards — dialectic"),
        Vertex("art_for_art", content="art for art's sake — bourgeois position negated"),
    ]
    edges = [
        Edge("art_serves_politics", "revolution", EdgeType.DEPENDENCY),
        Edge("art_serves_politics", "art_for_art", EdgeType.NEGATION),
        Edge("workers_peasants_soldiers", "mass_line", EdgeType.DEPENDENCY),
        Edge("popularization_vs_raising", "workers_peasants_soldiers", EdgeType.DEPENDENCY),
        Edge("popularization_vs_raising", "identity_of_opposites", EdgeType.REFERENCE),
        Edge("art_serves_politics", "class_analysis", EdgeType.REFERENCE),
    ]
    return ("1942: Talks at the Yan'an Forum on Literature and Art", vertices, edges)


def work_32_economic_financial():
    """1942: Economic and Financial Problems in the Anti-Japanese War —
    production campaigns, self-sufficiency, Nanniwan."""
    vertices = [
        Vertex("production_campaign", content="production campaign — army and people produce to survive blockade"),
        Vertex("self_sufficiency_economy", content="self-sufficiency economy — overcome enemy blockade"),
    ]
    edges = [
        Edge("production_campaign", "self_reliance", EdgeType.DEPENDENCY),
        Edge("production_campaign", "base_areas", EdgeType.DEPENDENCY),
        Edge("self_sufficiency_economy", "production_campaign", EdgeType.DEPENDENCY),
        Edge("self_sufficiency_economy", "peoples_war", EdgeType.REFERENCE),
    ]
    return ("1942: Economic and Financial Problems in the Anti-Japanese War", vertices, edges)


def work_33_methods_of_leadership():
    """1943: Some Questions Concerning Methods of Leadership —
    mass line formalized: from masses to masses, general and particular."""
    vertices = [
        Vertex("from_masses_to_masses", content="from the masses, to the masses — leadership method"),
        Vertex("general_particular_combine", content="combine the general call with particular guidance"),
        Vertex("line_struggle", content="two-line struggle — correct vs incorrect line"),
    ]
    edges = [
        Edge("from_masses_to_masses", "mass_line", EdgeType.DEPENDENCY),
        Edge("from_masses_to_masses", "investigation_research", EdgeType.REFERENCE),
        Edge("general_particular_combine", "universality_of_contradiction", EdgeType.REFERENCE),
        Edge("general_particular_combine", "particularity_of_contradiction", EdgeType.REFERENCE),
        Edge("general_particular_combine", "mass_line", EdgeType.DEPENDENCY),
        Edge("line_struggle", "principled_struggle", EdgeType.DEPENDENCY),
        Edge("line_struggle", "rectification", EdgeType.REFERENCE),
    ]
    return ("1943: Some Questions Concerning Methods of Leadership", vertices, edges)


def work_34_resolution_on_history():
    """1945: Resolution on Certain Questions in the History of Our Party —
    summary of line struggles, Marxism-Leninism + Chinese practice."""
    vertices = [
        Vertex("resolution_history", content="resolution on history — systematic summary of line struggles"),
        Vertex("left_deviation", content="left deviation — adventurism, closed-doorism"),
        Vertex("right_deviation", content="right deviation — capitulationism, tailism"),
    ]
    edges = [
        Edge("resolution_history", "line_struggle", EdgeType.DEPENDENCY),
        Edge("resolution_history", "sinification_of_marxism", EdgeType.DEPENDENCY),
        Edge("left_deviation", "right_deviation", EdgeType.NEGATION),
        Edge("resolution_history", "left_deviation", EdgeType.NEGATION),
        Edge("resolution_history", "right_deviation", EdgeType.NEGATION),
        Edge("resolution_history", "seeking_truth_from_facts", EdgeType.REFERENCE),
    ]
    return ("1945: Resolution on Certain Questions in the History of Our Party", vertices, edges)


def work_35_on_coalition_government():
    """1945: On Coalition Government — post-war program, democratic coalition,
    two destinies for China."""
    vertices = [
        Vertex("coalition_government", content="coalition government — democratic post-war program"),
        Vertex("two_destinies", content="two destinies — democratic China vs feudal-fascist China"),
        Vertex("peoples_army", content="people's army — an army that serves the people"),
    ]
    edges = [
        Edge("coalition_government", "new_democracy", EdgeType.DEPENDENCY),
        Edge("coalition_government", "united_front", EdgeType.DEPENDENCY),
        Edge("two_destinies", "principal_contradiction", EdgeType.REFERENCE),
        Edge("two_destinies", "coalition_government", EdgeType.DEPENDENCY),
        Edge("peoples_army", "red_army", EdgeType.SUBLATION),
        Edge("peoples_army", "peoples_war", EdgeType.DEPENDENCY),
        Edge("peoples_army", "serve_the_people", EdgeType.REFERENCE),
    ]
    return ("1945: On Coalition Government", vertices, edges)


def work_36_foolish_old_man():
    """1945: The Foolish Old Man Who Removed the Mountains —
    perseverance, moving the masses, two mountains (imperialism + feudalism)."""
    vertices = [
        Vertex("foolish_old_man", content="Yu Gong — perseverance of the masses"),
        Vertex("two_mountains", content="two mountains: imperialism and feudalism"),
        Vertex("perseverance", content="perseverance — sustained struggle through difficulty"),
    ]
    edges = [
        Edge("foolish_old_man", "serve_the_people", EdgeType.REFERENCE),
        Edge("two_mountains", "imperialism", EdgeType.NEGATION),
        Edge("two_mountains", "feudalism", EdgeType.NEGATION),
        Edge("perseverance", "subjective_forces", EdgeType.DEPENDENCY),
        Edge("perseverance", "mass_line", EdgeType.REFERENCE),
        Edge("foolish_old_man", "revolutionary_optimism", EdgeType.REFERENCE),
    ]
    return ("1945: The Foolish Old Man Who Removed the Mountains", vertices, edges)


def work_37_situation_after_victory():
    """1945: The Situation and Our Policy After Victory in the War of Resistance —
    Chiang Kai-shek's two-faced tactics, prepare for civil war."""
    vertices = [
        Vertex("chiangs_two_faces", content="Chiang's two-faced tactics — peace talk + war preparation"),
        Vertex("prepare_civil_war", content="prepare for civil war — self-defense strategy"),
    ]
    edges = [
        Edge("chiangs_two_faces", "united_front", EdgeType.NEGATION),
        Edge("prepare_civil_war", "self_reliance", EdgeType.DEPENDENCY),
        Edge("prepare_civil_war", "peoples_army", EdgeType.DEPENDENCY),
        Edge("chiangs_two_faces", "class_analysis", EdgeType.REFERENCE),
    ]
    return ("1945: The Situation After Victory in the War of Resistance", vertices, edges)


def work_38_chungking_negotiations():
    """1945: On the Chungking Negotiations —
    tit-for-tat struggle, fighting while negotiating."""
    vertices = [
        Vertex("tit_for_tat", content="tit-for-tat — fight on just grounds, to advantage, with restraint"),
        Vertex("fight_while_negotiate", content="fighting while negotiating — dual tactics"),
    ]
    edges = [
        Edge("tit_for_tat", "principled_struggle", EdgeType.DEPENDENCY),
        Edge("fight_while_negotiate", "tit_for_tat", EdgeType.DEPENDENCY),
        Edge("fight_while_negotiate", "peoples_army", EdgeType.REFERENCE),
        Edge("tit_for_tat", "transformation_of_opposites", EdgeType.REFERENCE),
    ]
    return ("1945: On the Chungking Negotiations", vertices, edges)


def work_39_paper_tiger():
    """1946: Talk with Anna Louise Strong —
    paper tiger thesis, strategically despise, tactically respect."""
    vertices = [
        Vertex("paper_tiger", content="all reactionaries are paper tigers"),
        Vertex("strategic_despise", content="strategically despise the enemy, tactically take seriously"),
    ]
    edges = [
        Edge("paper_tiger", "revolutionary_optimism", EdgeType.DEPENDENCY),
        Edge("paper_tiger", "imperialism", EdgeType.NEGATION),
        Edge("strategic_despise", "paper_tiger", EdgeType.DEPENDENCY),
        Edge("strategic_despise", "transformation_of_opposites", EdgeType.REFERENCE),
    ]
    return ("1946: Talk with Anna Louise Strong — Paper Tiger", vertices, edges)


def work_40_present_situation_tasks():
    """1947: The Present Situation and Our Tasks —
    ten military principles, new-democratic economy program."""
    vertices = [
        Vertex("ten_military_principles", content="ten principles of operation — concentrate superior forces"),
        Vertex("confiscate_bureaucratic_capital", content="confiscate bureaucrat-capital"),
    ]
    edges = [
        Edge("ten_military_principles", "concentration_forces", EdgeType.DEPENDENCY),
        Edge("ten_military_principles", "annihilation_strategy", EdgeType.DEPENDENCY),
        Edge("confiscate_bureaucratic_capital", "new_democratic_economy", EdgeType.DEPENDENCY),
        Edge("confiscate_bureaucratic_capital", "comprador_bourgeoisie", EdgeType.NEGATION),
    ]
    return ("1947: The Present Situation and Our Tasks", vertices, edges)


def work_41_revolutionary_war_turning_point():
    """1947: Manifesto of the Chinese People's Liberation Army —
    war at turning point, land reform + military offensive."""
    vertices = [
        Vertex("pla_manifesto", content="PLA manifesto — overthrow Chiang, land to the tiller"),
        Vertex("land_reform", content="land reform — confiscate landlord land, distribute to peasants"),
    ]
    edges = [
        Edge("pla_manifesto", "peoples_army", EdgeType.DEPENDENCY),
        Edge("pla_manifesto", "agrarian_revolution", EdgeType.REFERENCE),
        Edge("land_reform", "agrarian_revolution", EdgeType.SUBLATION),
        Edge("land_reform", "landlord_class", EdgeType.NEGATION),
        Edge("land_reform", "peasantry", EdgeType.DEPENDENCY),
    ]
    return ("1947: PLA Manifesto + Land Reform", vertices, edges)


def work_42_report_jinsuiyu():
    """1948: A Talk on the Problems of the Jinsuiyu Region —
    policy in new liberated areas, winning the middle peasant."""
    vertices = [
        Vertex("new_liberated_areas_policy", content="policy in new liberated areas — go slow on land reform"),
        Vertex("winning_middle_peasant", content="win the middle peasant — do not encroach on middle peasants"),
    ]
    edges = [
        Edge("new_liberated_areas_policy", "land_reform", EdgeType.DEPENDENCY),
        Edge("new_liberated_areas_policy", "concrete_analysis", EdgeType.REFERENCE),
        Edge("winning_middle_peasant", "peasantry", EdgeType.DEPENDENCY),
        Edge("winning_middle_peasant", "united_front", EdgeType.REFERENCE),
    ]
    return ("1948: Problems of the Jinsuiyu Region", vertices, edges)


def work_43_peoples_democratic_dictatorship():
    """1949: On the People's Democratic Dictatorship —
    state theory, democracy for the people, dictatorship over reactionaries."""
    vertices = [
        Vertex("peoples_democratic_dictatorship", content="people's democratic dictatorship — state form"),
        Vertex("democracy_for_people", content="democracy for the people — who are 'the people'"),
        Vertex("dictatorship_over_reactionaries", content="dictatorship over reactionaries — suppress resistance"),
        Vertex("lean_to_one_side", content="lean to one side — ally with socialist camp"),
    ]
    edges = [
        Edge("peoples_democratic_dictatorship", "joint_dictatorship", EdgeType.SUBLATION),
        Edge("peoples_democratic_dictatorship", "new_democracy", EdgeType.DEPENDENCY),
        Edge("democracy_for_people", "peoples_democratic_dictatorship", EdgeType.DEPENDENCY),
        Edge("dictatorship_over_reactionaries", "peoples_democratic_dictatorship", EdgeType.DEPENDENCY),
        Edge("democracy_for_people", "dictatorship_over_reactionaries", EdgeType.NEGATION),
        Edge("lean_to_one_side", "imperialism", EdgeType.NEGATION),
        Edge("peoples_democratic_dictatorship", "class_analysis", EdgeType.REFERENCE),
    ]
    return ("1949: On the People's Democratic Dictatorship", vertices, edges)


def work_44_dont_hit_all_directions():
    """1950: Don't Hit Out in All Directions —
    isolate the main enemy, distinguish contradictions, consolidate majority."""
    vertices = [
        Vertex("isolate_main_enemy", content="isolate the main enemy — focus attack"),
        Vertex("consolidate_majority", content="consolidate the majority — win over middle forces"),
    ]
    edges = [
        Edge("isolate_main_enemy", "principal_contradiction", EdgeType.DEPENDENCY),
        Edge("isolate_main_enemy", "united_front", EdgeType.REFERENCE),
        Edge("consolidate_majority", "mass_line", EdgeType.DEPENDENCY),
        Edge("consolidate_majority", "national_bourgeoisie", EdgeType.REFERENCE),
        Edge("isolate_main_enemy", "enemy_friend", EdgeType.REFERENCE),
    ]
    return ("1950: Don't Hit Out in All Directions", vertices, edges)


def work_45_oppose_us_aggression():
    """1950: Resist US Aggression and Aid Korea —
    internationalism, fighting imperialism abroad."""
    vertices = [
        Vertex("resist_us_aid_korea", content="resist US aggression and aid Korea — just war"),
        Vertex("proletarian_internationalism", content="proletarian internationalism — support revolutionary peoples"),
    ]
    edges = [
        Edge("resist_us_aid_korea", "imperialism", EdgeType.NEGATION),
        Edge("resist_us_aid_korea", "peoples_war", EdgeType.REFERENCE),
        Edge("proletarian_internationalism", "resist_us_aid_korea", EdgeType.DEPENDENCY),
        Edge("proletarian_internationalism", "lean_to_one_side", EdgeType.REFERENCE),
    ]
    return ("1950: Resist US Aggression and Aid Korea", vertices, edges)


def work_46_handling_contradictions_draft():
    """1953: On the Correct Handling of Contradictions Among the People (draft) —
    early formulation, transitional."""
    vertices = [
        Vertex("contradictions_among_people_early", content="contradictions among the people — early formulation 1953"),
    ]
    edges = [
        Edge("contradictions_among_people_early", "non_antagonistic_contradiction", EdgeType.DEPENDENCY),
        Edge("contradictions_among_people_early", "peoples_democratic_dictatorship", EdgeType.REFERENCE),
        Edge("contradictions_among_people_early", "antagonistic_contradiction", EdgeType.NEGATION),
    ]
    return ("1953: Contradictions Among the People (draft)", vertices, edges)


def work_47_socialist_transformation():
    """1953-55: On the Cooperative Transformation of Agriculture —
    collectivization, socialist transformation of agriculture."""
    vertices = [
        Vertex("cooperative_agriculture", content="cooperative transformation of agriculture — mutual aid to higher cooperatives"),
        Vertex("socialist_transformation", content="socialist transformation — private to public ownership"),
    ]
    edges = [
        Edge("cooperative_agriculture", "peasantry", EdgeType.DEPENDENCY),
        Edge("cooperative_agriculture", "land_reform", EdgeType.SUBLATION),
        Edge("socialist_transformation", "new_democratic_economy", EdgeType.SUBLATION),
        Edge("socialist_transformation", "cooperative_agriculture", EdgeType.DEPENDENCY),
    ]
    return ("1953-55: Cooperative Transformation of Agriculture", vertices, edges)


def work_48_socialist_upsurge():
    """1955: Editor's Notes from Socialist Upsurge in China's Countryside —
    accelerate collectivization, criticize right conservatism."""
    vertices = [
        Vertex("socialist_upsurge", content="socialist upsurge — mass enthusiasm for cooperatives"),
        Vertex("right_conservatism", content="right conservatism — slow pace of collectivization criticized"),
    ]
    edges = [
        Edge("socialist_upsurge", "cooperative_agriculture", EdgeType.DEPENDENCY),
        Edge("socialist_upsurge", "mass_initiative", EdgeType.REFERENCE),
        Edge("right_conservatism", "socialist_upsurge", EdgeType.NEGATION),
        Edge("right_conservatism", "right_deviation", EdgeType.REFERENCE),
    ]
    return ("1955: Socialist Upsurge in China's Countryside", vertices, edges)


def work_49_ten_major_relationships():
    """1956: On the Ten Major Relationships —
    dialectics of socialist construction, balanced development, learning from USSR mistakes."""
    vertices = [
        Vertex("ten_major_relationships", content="ten major relationships — dialectics of construction"),
        Vertex("heavy_light_industry", content="heavy vs light industry + agriculture balance"),
        Vertex("coastal_inland", content="coastal vs inland development"),
        Vertex("center_local", content="center vs local — give localities more power"),
        Vertex("china_foreign", content="China and foreign countries — learn, don't copy"),
        Vertex("learn_from_ussr_mistakes", content="learn from USSR mistakes — avoid their path"),
    ]
    edges = [
        Edge("ten_major_relationships", "contradiction", EdgeType.DEPENDENCY),
        Edge("ten_major_relationships", "seeking_truth_from_facts", EdgeType.REFERENCE),
        Edge("heavy_light_industry", "ten_major_relationships", EdgeType.DEPENDENCY),
        Edge("coastal_inland", "ten_major_relationships", EdgeType.DEPENDENCY),
        Edge("center_local", "ten_major_relationships", EdgeType.DEPENDENCY),
        Edge("china_foreign", "sinification_of_marxism", EdgeType.REFERENCE),
        Edge("learn_from_ussr_mistakes", "dogmatism", EdgeType.NEGATION),
        Edge("learn_from_ussr_mistakes", "china_foreign", EdgeType.DEPENDENCY),
    ]
    return ("1956: On the Ten Major Relationships", vertices, edges)


def work_50_handling_contradictions():
    """1957: On the Correct Handling of Contradictions Among the People —
    two types of contradictions, six criteria, hundred flowers."""
    vertices = [
        Vertex("contradictions_among_people", content="contradictions among the people — non-antagonistic, handled by discussion"),
        Vertex("contradictions_with_enemy", content="contradictions between us and the enemy — antagonistic"),
        Vertex("two_types_contradictions", content="two types of social contradictions"),
        Vertex("six_criteria", content="six criteria for distinguishing fragrant flowers from poisonous weeds"),
        Vertex("hundred_flowers", content="let a hundred flowers bloom, let a hundred schools contend"),
    ]
    edges = [
        Edge("two_types_contradictions", "antagonistic_contradiction", EdgeType.DEPENDENCY),
        Edge("two_types_contradictions", "non_antagonistic_contradiction", EdgeType.DEPENDENCY),
        Edge("contradictions_among_people", "non_antagonistic_contradiction", EdgeType.DEPENDENCY),
        Edge("contradictions_among_people", "contradictions_among_people_early", EdgeType.SUBLATION),
        Edge("contradictions_with_enemy", "antagonistic_contradiction", EdgeType.DEPENDENCY),
        Edge("contradictions_among_people", "contradictions_with_enemy", EdgeType.NEGATION),
        Edge("six_criteria", "contradictions_among_people", EdgeType.DEPENDENCY),
        Edge("hundred_flowers", "contradictions_among_people", EdgeType.DEPENDENCY),
        Edge("hundred_flowers", "unity_through_struggle", EdgeType.REFERENCE),
        Edge("contradictions_among_people", "peoples_democratic_dictatorship", EdgeType.REFERENCE),
    ]
    return ("1957: On the Correct Handling of Contradictions Among the People", vertices, edges)


def work_51_propaganda_work():
    """1957: Speech at the CPC National Conference on Propaganda Work —
    intellectuals, ideological remolding, serving the people through propaganda."""
    vertices = [
        Vertex("ideological_remolding", content="ideological remolding of intellectuals"),
        Vertex("propaganda_work", content="propaganda work — spread correct ideas"),
    ]
    edges = [
        Edge("ideological_remolding", "rectification", EdgeType.REFERENCE),
        Edge("ideological_remolding", "contradictions_among_people", EdgeType.DEPENDENCY),
        Edge("propaganda_work", "mass_line", EdgeType.DEPENDENCY),
        Edge("propaganda_work", "art_serves_politics", EdgeType.REFERENCE),
        Edge("ideological_remolding", "criticism_self_criticism", EdgeType.REFERENCE),
    ]
    return ("1957: Speech on Propaganda Work", vertices, edges)


def work_52_sixty_points_working_methods():
    """1958: Sixty Points on Working Methods —
    red and expert, politics in command, great leap forward ideology."""
    vertices = [
        Vertex("red_and_expert", content="red and expert — politics and technique united"),
        Vertex("politics_in_command", content="politics in command — political consciousness leads"),
        Vertex("great_leap_ideology", content="Great Leap ideology — mass mobilization for rapid development"),
    ]
    edges = [
        Edge("red_and_expert", "party_leadership", EdgeType.DEPENDENCY),
        Edge("red_and_expert", "practice", EdgeType.REFERENCE),
        Edge("politics_in_command", "party_commands_gun", EdgeType.REFERENCE),
        Edge("politics_in_command", "red_and_expert", EdgeType.DEPENDENCY),
        Edge("great_leap_ideology", "mass_mobilization", EdgeType.DEPENDENCY),
        Edge("great_leap_ideology", "revolutionary_optimism", EdgeType.REFERENCE),
    ]
    return ("1958: Sixty Points on Working Methods", vertices, edges)


def work_53_reading_notes_soviet_textbook():
    """1958-60: Reading Notes on the Soviet Textbook of Political Economy —
    critique of Soviet model, transition period, commodity production under socialism."""
    vertices = [
        Vertex("critique_soviet_model", content="critique of Soviet economic model — mechanistic, divorced from masses"),
        Vertex("commodity_under_socialism", content="commodity production under socialism — transitional"),
        Vertex("socialist_transition", content="socialist transition — protracted, full of contradictions"),
        Vertex("bourgeois_right", content="bourgeois right — remnant inequality under socialism"),
    ]
    edges = [
        Edge("critique_soviet_model", "learn_from_ussr_mistakes", EdgeType.DEPENDENCY),
        Edge("critique_soviet_model", "dogmatism", EdgeType.NEGATION),
        Edge("commodity_under_socialism", "socialist_transition", EdgeType.DEPENDENCY),
        Edge("socialist_transition", "contradiction", EdgeType.DEPENDENCY),
        Edge("bourgeois_right", "socialist_transition", EdgeType.DEPENDENCY),
        Edge("bourgeois_right", "class_analysis", EdgeType.REFERENCE),
        Edge("critique_soviet_model", "sinification_of_marxism", EdgeType.REFERENCE),
    ]
    return ("1958-60: Reading Notes on the Soviet Textbook of Political Economy", vertices, edges)


def work_54_lessons_great_leap():
    """1959: Letter to Production Team Leaders (six articles) —
    self-correction after Great Leap excesses, truth from facts."""
    vertices = [
        Vertex("correct_great_leap", content="correct Great Leap excesses — exaggeration, commandism"),
        Vertex("commandism", content="commandism — coercing the masses instead of persuading"),
    ]
    edges = [
        Edge("correct_great_leap", "seeking_truth_from_facts", EdgeType.DEPENDENCY),
        Edge("correct_great_leap", "great_leap_ideology", EdgeType.NEGATION),
        Edge("commandism", "mass_line", EdgeType.NEGATION),
        Edge("correct_great_leap", "commandism", EdgeType.NEGATION),
    ]
    return ("1959: Correcting Great Leap Excesses", vertices, edges)


def work_55_where_correct_ideas():
    """1963: Where Do Correct Ideas Come From? —
    three stages of cognition, social practice, matter to consciousness and back."""
    vertices = [
        Vertex("correct_ideas_from_practice", content="correct ideas come from social practice — three sources"),
        Vertex("three_practices", content="three forms of practice: production, class struggle, scientific experiment"),
        Vertex("matter_to_consciousness", content="matter determines consciousness, consciousness reacts on matter"),
    ]
    edges = [
        Edge("correct_ideas_from_practice", "practice", EdgeType.DEPENDENCY),
        Edge("correct_ideas_from_practice", "criterion_of_truth", EdgeType.REFERENCE),
        Edge("three_practices", "correct_ideas_from_practice", EdgeType.DEPENDENCY),
        Edge("matter_to_consciousness", "dialectical_materialism", EdgeType.DEPENDENCY),
        Edge("matter_to_consciousness", "leap_to_rational", EdgeType.REFERENCE),
        Edge("matter_to_consciousness", "leap_to_practice", EdgeType.REFERENCE),
    ]
    return ("1963: Where Do Correct Ideas Come From?", vertices, edges)


def work_56_learn_from_dazhai():
    """1963-64: Learn from Dazhai / Learn from the PLA —
    model emulation, self-reliance in agriculture and military."""
    vertices = [
        Vertex("learn_from_models", content="learn from Dazhai/PLA — model emulation campaigns"),
        Vertex("socialist_education", content="socialist education movement — four cleanups"),
    ]
    edges = [
        Edge("learn_from_models", "self_reliance", EdgeType.DEPENDENCY),
        Edge("learn_from_models", "mass_line", EdgeType.REFERENCE),
        Edge("socialist_education", "class_analysis", EdgeType.DEPENDENCY),
        Edge("socialist_education", "rectification", EdgeType.REFERENCE),
    ]
    return ("1963-64: Learn from Dazhai / Learn from the PLA", vertices, edges)


def work_57_khrushchev_phoney_communism():
    """1964: On Khrushchev's Phoney Communism —
    anti-revisionism, preventing capitalist restoration, continuing revolution."""
    vertices = [
        Vertex("anti_revisionism", content="anti-revisionism — Khrushchev betrays Marxism-Leninism"),
        Vertex("capitalist_restoration", content="danger of capitalist restoration under socialism"),
        Vertex("continuing_revolution", content="continuing the revolution under the dictatorship of the proletariat"),
        Vertex("revisionism", content="revisionism — negation of class struggle under socialism"),
    ]
    edges = [
        Edge("anti_revisionism", "revisionism", EdgeType.NEGATION),
        Edge("capitalist_restoration", "bourgeois_right", EdgeType.DEPENDENCY),
        Edge("capitalist_restoration", "socialist_transition", EdgeType.REFERENCE),
        Edge("continuing_revolution", "capitalist_restoration", EdgeType.NEGATION),
        Edge("continuing_revolution", "class_analysis", EdgeType.DEPENDENCY),
        Edge("revisionism", "critique_soviet_model", EdgeType.REFERENCE),
        Edge("anti_revisionism", "line_struggle", EdgeType.REFERENCE),
        Edge("continuing_revolution", "peoples_democratic_dictatorship", EdgeType.REFERENCE),
    ]
    return ("1964: On Khrushchev's Phoney Communism", vertices, edges)


def work_58_twenty_three_articles():
    """1965: Twenty-Three Articles —
    target of socialist education = capitalist roaders in the party."""
    vertices = [
        Vertex("capitalist_roaders", content="capitalist roaders — party members taking capitalist road"),
        Vertex("target_in_party", content="target of movement = those in authority taking capitalist road"),
    ]
    edges = [
        Edge("capitalist_roaders", "revisionism", EdgeType.REFERENCE),
        Edge("capitalist_roaders", "class_analysis", EdgeType.REFERENCE),
        Edge("target_in_party", "capitalist_roaders", EdgeType.DEPENDENCY),
        Edge("target_in_party", "socialist_education", EdgeType.DEPENDENCY),
        Edge("target_in_party", "continuing_revolution", EdgeType.REFERENCE),
    ]
    return ("1965: Twenty-Three Articles", vertices, edges)


def work_59_may_16_circular():
    """1966: May 16 Circular —
    launch of the Cultural Revolution, critique of bourgeois headquarters."""
    vertices = [
        Vertex("cultural_revolution", content="Great Proletarian Cultural Revolution"),
        Vertex("may_16_circular", content="May 16 circular — programmatic document of Cultural Revolution"),
    ]
    edges = [
        Edge("cultural_revolution", "continuing_revolution", EdgeType.DEPENDENCY),
        Edge("cultural_revolution", "capitalist_restoration", EdgeType.NEGATION),
        Edge("may_16_circular", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("may_16_circular", "target_in_party", EdgeType.REFERENCE),
    ]
    return ("1966: May 16 Circular", vertices, edges)


def work_60_letter_to_jiang_qing():
    """1966: Letter to Jiang Qing —
    self-doubt, being used by rightists, implicit dialectic of leader and movement."""
    vertices = [
        Vertex("leader_movement_dialectic", content="dialectic of leader and mass movement — leader can be surpassed"),
        Vertex("being_used_by_right", content="danger of being used by the right"),
    ]
    edges = [
        Edge("leader_movement_dialectic", "mass_line", EdgeType.REFERENCE),
        Edge("leader_movement_dialectic", "transformation_of_opposites", EdgeType.REFERENCE),
        Edge("being_used_by_right", "line_struggle", EdgeType.DEPENDENCY),
        Edge("being_used_by_right", "continuing_revolution", EdgeType.REFERENCE),
    ]
    return ("1966: Letter to Jiang Qing", vertices, edges)


def work_61_bombard_headquarters():
    """1966: My First Big-Character Poster — Bombard the Headquarters —
    attack party bureaucracy directly from above."""
    vertices = [
        Vertex("bombard_headquarters", content="bombard the headquarters — attack party bureaucracy"),
        Vertex("mass_democracy_cr", content="mass democracy — big-character posters, great debates"),
    ]
    edges = [
        Edge("bombard_headquarters", "bureaucratism", EdgeType.NEGATION),
        Edge("bombard_headquarters", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("mass_democracy_cr", "mass_line", EdgeType.DEPENDENCY),
        Edge("mass_democracy_cr", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("bombard_headquarters", "capitalist_roaders", EdgeType.REFERENCE),
    ]
    return ("1966: Bombard the Headquarters", vertices, edges)


def work_62_sixteen_points():
    """1966: Decision Concerning the Great Proletarian Cultural Revolution (Sixteen Points) —
    guidelines for conducting the Cultural Revolution."""
    vertices = [
        Vertex("sixteen_points", content="sixteen points — guidelines for Cultural Revolution conduct"),
        Vertex("trust_the_masses", content="trust the masses, rely on the masses — CR principle"),
    ]
    edges = [
        Edge("sixteen_points", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("sixteen_points", "mass_democracy_cr", EdgeType.DEPENDENCY),
        Edge("trust_the_masses", "mass_line", EdgeType.DEPENDENCY),
        Edge("trust_the_masses", "mass_initiative", EdgeType.REFERENCE),
    ]
    return ("1966: Sixteen Points", vertices, edges)


def work_63_great_disorder_great_order():
    """1967: On the great disorder —
    chaos as method, dialectic of disorder and order."""
    vertices = [
        Vertex("chaos_as_method", content="great disorder under heaven leads to great order"),
        Vertex("revolutionary_committees", content="revolutionary committees — three-in-one combination"),
    ]
    edges = [
        Edge("chaos_as_method", "transformation_of_opposites", EdgeType.DEPENDENCY),
        Edge("chaos_as_method", "contradiction", EdgeType.REFERENCE),
        Edge("chaos_as_method", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("revolutionary_committees", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("revolutionary_committees", "mass_democracy_cr", EdgeType.REFERENCE),
    ]
    return ("1967: Great Disorder to Great Order", vertices, edges)


def work_64_talk_to_edgar_snow():
    """1970: Talk with Edgar Snow —
    personality cult dialectic, need for worship now, later to dismantle."""
    vertices = [
        Vertex("personality_cult_dialectic", content="personality cult — needed to combat bureaucracy, but eventually must end"),
        Vertex("worship_to_overcome", content="cult of personality as weapon against entrenched bureaucracy"),
    ]
    edges = [
        Edge("personality_cult_dialectic", "leader_movement_dialectic", EdgeType.DEPENDENCY),
        Edge("personality_cult_dialectic", "transformation_of_opposites", EdgeType.REFERENCE),
        Edge("worship_to_overcome", "bureaucratism", EdgeType.NEGATION),
        Edge("worship_to_overcome", "cultural_revolution", EdgeType.REFERENCE),
    ]
    return ("1970: Talk with Edgar Snow", vertices, edges)


def work_65_critique_of_lin_biao():
    """1971-76: Critique of Lin Biao and Confucius / Late reflections —
    anti-Confucius campaign, self-assessment of revolution's limits."""
    vertices = [
        Vertex("critique_lin_confucius", content="criticize Lin Biao and Confucius — anti-revisionism + anti-restorationism"),
        Vertex("revolution_limits", content="revolution's limits — three great differences persist"),
        Vertex("continuous_revolution", content="continuous revolution under proletarian dictatorship — final formulation"),
        Vertex("three_worlds_theory", content="three worlds theory — first/second/third world geopolitics"),
    ]
    edges = [
        Edge("critique_lin_confucius", "line_struggle", EdgeType.DEPENDENCY),
        Edge("critique_lin_confucius", "cultural_revolution", EdgeType.REFERENCE),
        Edge("revolution_limits", "bourgeois_right", EdgeType.DEPENDENCY),
        Edge("revolution_limits", "socialist_transition", EdgeType.REFERENCE),
        Edge("continuous_revolution", "continuing_revolution", EdgeType.SUBLATION),
        Edge("continuous_revolution", "contradiction", EdgeType.DEPENDENCY),
        Edge("three_worlds_theory", "imperialism", EdgeType.NEGATION),
        Edge("three_worlds_theory", "proletarian_internationalism", EdgeType.REFERENCE),
    ]
    return ("1971-76: Critique of Lin Biao and Confucius / Late Reflections", vertices, edges)


# ---------------------------------------------------------------------------
# Work sequence
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_01_great_union,
    work_02_classes_in_chinese_society,
    work_03_national_revolution_peasant,
    work_04_hunan_report,
    work_05_red_political_power,
    work_06_struggle_jinggang,
    work_07_correcting_mistaken_ideas,
    work_08_oppose_book_worship,
    work_09_single_spark,
    work_10_xunwu_investigation,
    work_11_counter_encirclement,
    work_12_well_being_of_masses,
    work_13_tactics_against_japan,
    work_14_strategy_revolutionary_war,
    work_15_on_practice,
    work_16_on_contradiction,
    work_17_combat_liberalism,
    work_18_national_self_defense_war,
    work_19_on_protracted_war,
    work_20_guerrilla_strategy,
    work_21_role_of_ccp,
    work_22_war_and_strategy,
    work_23_chinese_revolution_and_ccp,
    work_24_introducing_the_communist,
    work_25_on_new_democracy,
    work_26_current_situation_and_tasks,
    work_27_preface_rural_surveys,
    work_28_reform_our_study,
    work_29_rectify_party_style,
    work_30_oppose_stereotypes,
    work_31_yanan_forum,
    work_32_economic_financial,
    work_33_methods_of_leadership,
    work_34_resolution_on_history,
    work_35_on_coalition_government,
    work_36_foolish_old_man,
    work_37_situation_after_victory,
    work_38_chungking_negotiations,
    work_39_paper_tiger,
    work_40_present_situation_tasks,
    work_41_revolutionary_war_turning_point,
    work_42_report_jinsuiyu,
    work_43_peoples_democratic_dictatorship,
    work_44_dont_hit_all_directions,
    work_45_oppose_us_aggression,
    work_46_handling_contradictions_draft,
    work_47_socialist_transformation,
    work_48_socialist_upsurge,
    work_49_ten_major_relationships,
    work_50_handling_contradictions,
    work_51_propaganda_work,
    work_52_sixty_points_working_methods,
    work_53_reading_notes_soviet_textbook,
    work_54_lessons_great_leap,
    work_55_where_correct_ideas,
    work_56_learn_from_dazhai,
    work_57_khrushchev_phoney_communism,
    work_58_twenty_three_articles,
    work_59_may_16_circular,
    work_60_letter_to_jiang_qing,
    work_61_bombard_headquarters,
    work_62_sixteen_points,
    work_63_great_disorder_great_order,
    work_64_talk_to_edgar_snow,
    work_65_critique_of_lin_biao,
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    n_works = len(ALL_WORKS)
    print("=" * 70)
    print("MAO ZEDONG — FULL CORPUS INCREMENTAL TRAVERSAL")
    print(f"{n_works} works, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    work_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for w_idx, w_fn in enumerate(ALL_WORKS):
        work_name, w_vertices, w_edges = w_fn()
        print(f"\n{'~' * 60}")
        print(f"Work {w_idx + 1}/{n_works}: {work_name}")
        print(f"{'~' * 60}")

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

        print(f"  Steps to digest: {steps_this_work} {'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} (delta from injection: {beta_after - beta_before})")
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

        # Update graph reference
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
        "experiment": "mao_zedong_full_incremental",
        "source": f"Mao Zedong — {n_works} works chronologically ordered (1919-1976)",
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
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_mao_full.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
