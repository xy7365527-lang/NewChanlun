"""Lenin Collected Works — incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Lenin's thought
across 32 works (1893-1923) as an incremental topological
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
# Work encodings: each returns (work_name, vertices, edges)
# Cross-work edges connect to vertices already introduced in prior works.
# ---------------------------------------------------------------------------


def work_01_friends_of_people():
    """1893: What the Friends of the People Are.

    Polemic against Narodniks, defense of Marxist method,
    historical materialism vs subjective sociology.
    """
    vertices = [
        Vertex("historical_materialism", content="historical materialism — social being determines consciousness"),
        Vertex("class_struggle", content="class struggle — motor of history"),
        Vertex("proletariat", content="proletariat — revolutionary class"),
        Vertex("bourgeoisie", content="bourgeoisie — ruling class under capitalism"),
        Vertex("narodism", content="Narodism — petty-bourgeois socialism"),
        Vertex("subjective_sociology", content="subjective sociology — Mikhailovsky's method"),
        Vertex("dialectics", content="dialectics — Marxist method"),
        Vertex("materialism", content="materialism — matter primary, consciousness secondary"),
        Vertex("capitalism", content="capitalism — commodity production generalized"),
        Vertex("peasantry", content="peasantry — differentiation under capitalism"),
    ]
    edges = [
        Edge("historical_materialism", "materialism", EdgeType.DEPENDENCY),
        Edge("historical_materialism", "dialectics", EdgeType.DEPENDENCY),
        Edge("class_struggle", "proletariat", EdgeType.DEPENDENCY),
        Edge("class_struggle", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("proletariat", "bourgeoisie", EdgeType.NEGATION),
        Edge("narodism", "historical_materialism", EdgeType.NEGATION),
        Edge("subjective_sociology", "historical_materialism", EdgeType.NEGATION),
        Edge("narodism", "subjective_sociology", EdgeType.DEPENDENCY),
        Edge("capitalism", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("capitalism", "proletariat", EdgeType.DEPENDENCY),
        Edge("peasantry", "capitalism", EdgeType.DEPENDENCY),
        Edge("dialectics", "materialism", EdgeType.REFERENCE),
    ]
    return ("1893: What the Friends of the People Are", vertices, edges)


def work_02_economic_content_narodism():
    """1895: The Economic Content of Narodism.

    Critique of Struve's legal Marxism, objectivism vs materialism,
    development of capitalism in Russia.
    """
    vertices = [
        Vertex("legal_marxism", content="legal Marxism — Struve's objectivism"),
        Vertex("objectivism", content="objectivism — bourgeois impartiality disguised as science"),
        Vertex("partisanship", content="partisanship — Marxist class standpoint"),
        Vertex("commodity_production", content="commodity production — basis of capitalism"),
        Vertex("market", content="market — internal market created by capitalism"),
    ]
    edges = [
        Edge("legal_marxism", "objectivism", EdgeType.DEPENDENCY),
        Edge("objectivism", "partisanship", EdgeType.NEGATION),
        Edge("partisanship", "historical_materialism", EdgeType.DEPENDENCY),
        Edge("partisanship", "proletariat", EdgeType.DEPENDENCY),
        Edge("legal_marxism", "narodism", EdgeType.NEGATION),
        Edge("commodity_production", "capitalism", EdgeType.DEPENDENCY),
        Edge("market", "commodity_production", EdgeType.DEPENDENCY),
        Edge("market", "capitalism", EdgeType.REFERENCE),
        Edge("legal_marxism", "capitalism", EdgeType.REFERENCE),
    ]
    return ("1895: The Economic Content of Narodism", vertices, edges)


def work_03_development_capitalism():
    """1897-1899: The Development of Capitalism in Russia.

    Massive empirical study: differentiation of peasantry,
    growth of internal market, industrial capitalism in Russia.
    """
    vertices = [
        Vertex("differentiation_peasantry", content="differentiation of peasantry — rural bourgeoisie and proletariat"),
        Vertex("internal_market", content="internal market — created by capitalism itself"),
        Vertex("industrial_capitalism", content="industrial capitalism — large-scale machine industry"),
        Vertex("rural_bourgeoisie", content="rural bourgeoisie — kulaks"),
        Vertex("rural_proletariat", content="rural proletariat — landless laborers"),
        Vertex("primitive_accumulation", content="primitive accumulation — expropriation of producers"),
    ]
    edges = [
        Edge("differentiation_peasantry", "peasantry", EdgeType.DEPENDENCY),
        Edge("differentiation_peasantry", "capitalism", EdgeType.DEPENDENCY),
        Edge("differentiation_peasantry", "rural_bourgeoisie", EdgeType.DEPENDENCY),
        Edge("differentiation_peasantry", "rural_proletariat", EdgeType.DEPENDENCY),
        Edge("rural_bourgeoisie", "rural_proletariat", EdgeType.NEGATION),
        Edge("internal_market", "commodity_production", EdgeType.DEPENDENCY),
        Edge("internal_market", "differentiation_peasantry", EdgeType.DEPENDENCY),
        Edge("industrial_capitalism", "capitalism", EdgeType.DEPENDENCY),
        Edge("industrial_capitalism", "proletariat", EdgeType.DEPENDENCY),
        Edge("primitive_accumulation", "capitalism", EdgeType.REFERENCE),
        Edge("primitive_accumulation", "peasantry", EdgeType.NEGATION),
    ]
    return ("1897: The Development of Capitalism in Russia", vertices, edges)


def work_04_protest():
    """1899: A Protest by Russian Social-Democrats.

    Against Bernstein's revisionism, defense of revolutionary program.
    """
    vertices = [
        Vertex("revisionism", content="revisionism — Bernstein's rejection of revolution"),
        Vertex("reformism", content="reformism — gradual improvement within capitalism"),
        Vertex("revolutionary_program", content="revolutionary program — overthrow of capitalism"),
    ]
    edges = [
        Edge("revisionism", "revolutionary_program", EdgeType.NEGATION),
        Edge("revisionism", "reformism", EdgeType.DEPENDENCY),
        Edge("reformism", "revolutionary_program", EdgeType.NEGATION),
        Edge("revolutionary_program", "proletariat", EdgeType.DEPENDENCY),
        Edge("revolutionary_program", "class_struggle", EdgeType.DEPENDENCY),
        Edge("revisionism", "legal_marxism", EdgeType.REFERENCE),
    ]
    return ("1899: A Protest by Russian Social-Democrats", vertices, edges)


def work_05_what_is_to_be_done():
    """1902: What Is To Be Done?

    The foundational text: vanguard party, consciousness from without,
    spontaneity vs consciousness, professional revolutionaries.
    """
    vertices = [
        Vertex("vanguard", content="vanguard party — organized detachment of the class"),
        Vertex("spontaneity", content="spontaneity — trade-unionist consciousness"),
        Vertex("consciousness", content="socialist consciousness — brought from without"),
        Vertex("trade_unionism", content="trade unionism — bourgeois ideology among workers"),
        Vertex("professional_revolutionaries", content="professional revolutionaries — core of the party"),
        Vertex("party", content="party — organization of revolutionaries"),
        Vertex("economism", content="economism — tail-ending the spontaneous movement"),
        Vertex("newspaper", content="all-Russia newspaper — collective organizer"),
    ]
    edges = [
        Edge("spontaneity", "consciousness", EdgeType.NEGATION),
        Edge("consciousness", "vanguard", EdgeType.DEPENDENCY),
        Edge("vanguard", "party", EdgeType.DEPENDENCY),
        Edge("party", "professional_revolutionaries", EdgeType.DEPENDENCY),
        Edge("trade_unionism", "spontaneity", EdgeType.DEPENDENCY),
        Edge("trade_unionism", "bourgeoisie", EdgeType.REFERENCE),
        Edge("economism", "spontaneity", EdgeType.DEPENDENCY),
        Edge("economism", "revisionism", EdgeType.REFERENCE),
        Edge("consciousness", "historical_materialism", EdgeType.REFERENCE),
        Edge("vanguard", "proletariat", EdgeType.DEPENDENCY),
        Edge("newspaper", "party", EdgeType.DEPENDENCY),
        Edge("spontaneity", "revolutionary_program", EdgeType.NEGATION),
    ]
    return ("1902: What Is To Be Done?", vertices, edges)


def work_06_one_step_forward():
    """1904: One Step Forward, Two Steps Back.

    Party organizational principles, Bolsheviks vs Mensheviks,
    democratic centralism in embryo.
    """
    vertices = [
        Vertex("democratic_centralism", content="democratic centralism — unity of will and action"),
        Vertex("bolsheviks", content="Bolsheviks — majority, hard line on party membership"),
        Vertex("mensheviks", content="Mensheviks — minority, loose party organization"),
        Vertex("party_discipline", content="party discipline — subordination of minority to majority"),
        Vertex("opportunism_organizational", content="organizational opportunism — loose, amorphous party"),
    ]
    edges = [
        Edge("democratic_centralism", "party", EdgeType.DEPENDENCY),
        Edge("democratic_centralism", "party_discipline", EdgeType.DEPENDENCY),
        Edge("bolsheviks", "mensheviks", EdgeType.NEGATION),
        Edge("bolsheviks", "democratic_centralism", EdgeType.DEPENDENCY),
        Edge("mensheviks", "opportunism_organizational", EdgeType.DEPENDENCY),
        Edge("opportunism_organizational", "democratic_centralism", EdgeType.NEGATION),
        Edge("party_discipline", "vanguard", EdgeType.DEPENDENCY),
        Edge("bolsheviks", "party", EdgeType.REFERENCE),
        Edge("mensheviks", "party", EdgeType.REFERENCE),
    ]
    return ("1904: One Step Forward, Two Steps Back", vertices, edges)


def work_07_two_tactics():
    """1905: Two Tactics of Social-Democracy in the Democratic Revolution.

    Bourgeois-democratic revolution, hegemony of proletariat,
    alliance with peasantry, revolutionary-democratic dictatorship.
    """
    vertices = [
        Vertex("bourgeois_democratic_revolution", content="bourgeois-democratic revolution — first stage"),
        Vertex("hegemony", content="hegemony of proletariat in democratic revolution"),
        Vertex("worker_peasant_alliance", content="worker-peasant alliance — democratic dictatorship"),
        Vertex("revolutionary_democratic_dictatorship", content="revolutionary-democratic dictatorship of proletariat and peasantry"),
        Vertex("liberal_bourgeoisie", content="liberal bourgeoisie — inconsistent democrat, fears revolution"),
    ]
    edges = [
        Edge("bourgeois_democratic_revolution", "capitalism", EdgeType.DEPENDENCY),
        Edge("hegemony", "proletariat", EdgeType.DEPENDENCY),
        Edge("hegemony", "bourgeois_democratic_revolution", EdgeType.DEPENDENCY),
        Edge("worker_peasant_alliance", "proletariat", EdgeType.DEPENDENCY),
        Edge("worker_peasant_alliance", "peasantry", EdgeType.DEPENDENCY),
        Edge("revolutionary_democratic_dictatorship", "worker_peasant_alliance", EdgeType.DEPENDENCY),
        Edge("revolutionary_democratic_dictatorship", "hegemony", EdgeType.DEPENDENCY),
        Edge("liberal_bourgeoisie", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("liberal_bourgeoisie", "bourgeois_democratic_revolution", EdgeType.REFERENCE),
        Edge("liberal_bourgeoisie", "revolutionary_democratic_dictatorship", EdgeType.NEGATION),
        Edge("mensheviks", "liberal_bourgeoisie", EdgeType.REFERENCE),
    ]
    return ("1905: Two Tactics of Social-Democracy", vertices, edges)


def work_08_materialism_empiriocriticism():
    """1908: Materialism and Empirio-Criticism.

    Philosophical defense of materialism against Machism,
    reflection theory, matter and consciousness, partisanship in philosophy.
    """
    vertices = [
        Vertex("reflection_theory", content="reflection theory — consciousness reflects objective reality"),
        Vertex("machism", content="Machism — empirio-criticism, subjective idealism"),
        Vertex("idealism", content="idealism — consciousness primary, matter secondary"),
        Vertex("objective_reality", content="objective reality — exists independently of consciousness"),
        Vertex("thing_in_itself", content="thing-in-itself — knowable through practice"),
        Vertex("partisanship_philosophy", content="partisanship in philosophy — materialism vs idealism, no third line"),
    ]
    edges = [
        Edge("reflection_theory", "materialism", EdgeType.DEPENDENCY),
        Edge("reflection_theory", "objective_reality", EdgeType.DEPENDENCY),
        Edge("machism", "materialism", EdgeType.NEGATION),
        Edge("machism", "idealism", EdgeType.DEPENDENCY),
        Edge("idealism", "materialism", EdgeType.NEGATION),
        Edge("objective_reality", "materialism", EdgeType.DEPENDENCY),
        Edge("thing_in_itself", "objective_reality", EdgeType.DEPENDENCY),
        Edge("thing_in_itself", "reflection_theory", EdgeType.REFERENCE),
        Edge("partisanship_philosophy", "materialism", EdgeType.DEPENDENCY),
        Edge("partisanship_philosophy", "idealism", EdgeType.NEGATION),
        Edge("partisanship_philosophy", "partisanship", EdgeType.REFERENCE),
    ]
    return ("1908: Materialism and Empirio-Criticism", vertices, edges)


def work_09_three_sources():
    """1913: The Three Sources and Three Component Parts of Marxism.

    Summary: German philosophy (dialectics), English political economy,
    French socialism → unified in Marxism.
    """
    vertices = [
        Vertex("german_philosophy", content="German philosophy — Hegel's dialectics, materialized"),
        Vertex("english_political_economy", content="English political economy — labor theory of value"),
        Vertex("french_socialism", content="French socialism — utopian, class struggle tradition"),
        Vertex("surplus_value", content="surplus value — exploitation of labor by capital"),
        Vertex("utopian_socialism", content="utopian socialism — socialism without class struggle"),
    ]
    edges = [
        Edge("german_philosophy", "dialectics", EdgeType.DEPENDENCY),
        Edge("german_philosophy", "materialism", EdgeType.REFERENCE),
        Edge("english_political_economy", "capitalism", EdgeType.DEPENDENCY),
        Edge("english_political_economy", "surplus_value", EdgeType.DEPENDENCY),
        Edge("surplus_value", "proletariat", EdgeType.DEPENDENCY),
        Edge("surplus_value", "bourgeoisie", EdgeType.REFERENCE),
        Edge("french_socialism", "class_struggle", EdgeType.DEPENDENCY),
        Edge("utopian_socialism", "french_socialism", EdgeType.DEPENDENCY),
        Edge("utopian_socialism", "class_struggle", EdgeType.NEGATION),
        Edge("french_socialism", "revolutionary_program", EdgeType.REFERENCE),
    ]
    return ("1913: Three Sources and Three Component Parts of Marxism", vertices, edges)


def work_10_self_determination():
    """1914: The Right of Nations to Self-Determination.

    National question, self-determination as democratic right,
    against Rosa Luxemburg's position.
    """
    vertices = [
        Vertex("self_determination", content="self-determination of nations — right to secession"),
        Vertex("national_question", content="national question — oppressor vs oppressed nations"),
        Vertex("oppressed_nations", content="oppressed nations — colonial and semi-colonial peoples"),
        Vertex("great_power_chauvinism", content="great-power chauvinism — denial of self-determination"),
        Vertex("national_liberation", content="national liberation — democratic content of national movements"),
    ]
    edges = [
        Edge("self_determination", "national_question", EdgeType.DEPENDENCY),
        Edge("national_question", "oppressed_nations", EdgeType.DEPENDENCY),
        Edge("self_determination", "bourgeois_democratic_revolution", EdgeType.REFERENCE),
        Edge("great_power_chauvinism", "self_determination", EdgeType.NEGATION),
        Edge("national_liberation", "self_determination", EdgeType.DEPENDENCY),
        Edge("national_liberation", "class_struggle", EdgeType.REFERENCE),
        Edge("oppressed_nations", "capitalism", EdgeType.DEPENDENCY),
    ]
    return ("1914: The Right of Nations to Self-Determination", vertices, edges)


def work_11_philosophical_notebooks():
    """1914-16: Philosophical Notebooks (Hegel notes).

    Study of Hegel's Logic, unity of opposites as kernel of dialectics,
    leap, negation of negation, practice as criterion of truth.
    """
    vertices = [
        Vertex("unity_of_opposites", content="unity and struggle of opposites — kernel of dialectics"),
        Vertex("negation_of_negation", content="negation of negation — spiral development"),
        Vertex("leap", content="leap — quantitative to qualitative transformation"),
        Vertex("practice", content="practice — criterion of truth, basis of knowledge"),
        Vertex("concrete_universal", content="concrete universal — unity of abstract and concrete"),
        Vertex("contradiction", content="contradiction — internal source of development"),
    ]
    edges = [
        Edge("unity_of_opposites", "dialectics", EdgeType.DEPENDENCY),
        Edge("unity_of_opposites", "contradiction", EdgeType.DEPENDENCY),
        Edge("negation_of_negation", "dialectics", EdgeType.DEPENDENCY),
        Edge("negation_of_negation", "contradiction", EdgeType.REFERENCE),
        Edge("leap", "dialectics", EdgeType.DEPENDENCY),
        Edge("leap", "contradiction", EdgeType.REFERENCE),
        Edge("practice", "materialism", EdgeType.DEPENDENCY),
        Edge("practice", "reflection_theory", EdgeType.REFERENCE),
        Edge("practice", "thing_in_itself", EdgeType.REFERENCE),
        Edge("concrete_universal", "dialectics", EdgeType.DEPENDENCY),
        Edge("contradiction", "unity_of_opposites", EdgeType.DEPENDENCY),
        Edge("contradiction", "class_struggle", EdgeType.REFERENCE),
    ]
    return ("1914-16: Philosophical Notebooks", vertices, edges)


def work_12_collapse_second_international():
    """1915: The Collapse of the Second International.

    Betrayal of social-democracy, social-chauvinism,
    opportunism as class phenomenon.
    """
    vertices = [
        Vertex("social_chauvinism", content="social-chauvinism — socialism in words, chauvinism in deeds"),
        Vertex("opportunism", content="opportunism — adaptation to bourgeoisie within workers' movement"),
        Vertex("labor_aristocracy", content="labor aristocracy — bribed upper stratum of working class"),
        Vertex("second_international", content="Second International — collapse into social-chauvinism"),
        Vertex("civil_peace", content="civil peace — class collaboration during war"),
    ]
    edges = [
        Edge("social_chauvinism", "opportunism", EdgeType.DEPENDENCY),
        Edge("social_chauvinism", "great_power_chauvinism", EdgeType.REFERENCE),
        Edge("opportunism", "labor_aristocracy", EdgeType.DEPENDENCY),
        Edge("labor_aristocracy", "proletariat", EdgeType.REFERENCE),
        Edge("labor_aristocracy", "bourgeoisie", EdgeType.REFERENCE),
        Edge("second_international", "social_chauvinism", EdgeType.DEPENDENCY),
        Edge("second_international", "revolutionary_program", EdgeType.NEGATION),
        Edge("civil_peace", "class_struggle", EdgeType.NEGATION),
        Edge("civil_peace", "social_chauvinism", EdgeType.DEPENDENCY),
        Edge("opportunism", "revisionism", EdgeType.REFERENCE),
    ]
    return ("1915: The Collapse of the Second International", vertices, edges)


def work_13_imperialism():
    """1916: Imperialism, the Highest Stage of Capitalism.

    Five features of imperialism, finance capital, division of world,
    parasitism, moribund capitalism.
    """
    vertices = [
        Vertex("imperialism", content="imperialism — monopoly stage of capitalism"),
        Vertex("finance_capital", content="finance capital — merger of bank and industrial capital"),
        Vertex("monopoly", content="monopoly — concentration and centralization of production"),
        Vertex("export_of_capital", content="export of capital — characteristic of imperialism"),
        Vertex("division_of_world", content="division of world among great powers"),
        Vertex("parasitism", content="parasitism — rentier state, coupon-clipping"),
        Vertex("uneven_development", content="uneven development — law of capitalism/imperialism"),
        Vertex("moribund_capitalism", content="moribund capitalism — eve of socialist revolution"),
    ]
    edges = [
        Edge("imperialism", "capitalism", EdgeType.DEPENDENCY),
        Edge("imperialism", "monopoly", EdgeType.DEPENDENCY),
        Edge("finance_capital", "monopoly", EdgeType.DEPENDENCY),
        Edge("finance_capital", "capitalism", EdgeType.REFERENCE),
        Edge("export_of_capital", "finance_capital", EdgeType.DEPENDENCY),
        Edge("division_of_world", "imperialism", EdgeType.DEPENDENCY),
        Edge("division_of_world", "great_power_chauvinism", EdgeType.REFERENCE),
        Edge("parasitism", "finance_capital", EdgeType.DEPENDENCY),
        Edge("parasitism", "labor_aristocracy", EdgeType.DEPENDENCY),
        Edge("uneven_development", "imperialism", EdgeType.DEPENDENCY),
        Edge("moribund_capitalism", "imperialism", EdgeType.DEPENDENCY),
        Edge("moribund_capitalism", "revolutionary_program", EdgeType.REFERENCE),
        Edge("imperialism", "oppressed_nations", EdgeType.DEPENDENCY),
    ]
    return ("1916: Imperialism, the Highest Stage of Capitalism", vertices, edges)


def work_14_state_and_revolution():
    """1917: The State and Revolution.

    State as instrument of class rule, smashing the state,
    dictatorship of the proletariat, withering away, commune-state.
    """
    vertices = [
        Vertex("state", content="state — instrument of class domination"),
        Vertex("dictatorship_proletariat", content="dictatorship of the proletariat — workers' state"),
        Vertex("smashing_state", content="smashing the bourgeois state machine"),
        Vertex("withering_away", content="withering away of the state — under communism"),
        Vertex("commune_state", content="commune-state — Paris Commune model"),
        Vertex("bourgeois_state", content="bourgeois state — standing army, bureaucracy, police"),
        Vertex("soviets", content="soviets — organs of workers' power"),
        Vertex("communism", content="communism — classless society"),
    ]
    edges = [
        Edge("state", "class_struggle", EdgeType.DEPENDENCY),
        Edge("state", "bourgeoisie", EdgeType.REFERENCE),
        Edge("dictatorship_proletariat", "state", EdgeType.DEPENDENCY),
        Edge("dictatorship_proletariat", "proletariat", EdgeType.DEPENDENCY),
        Edge("smashing_state", "bourgeois_state", EdgeType.NEGATION),
        Edge("smashing_state", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("bourgeois_state", "state", EdgeType.DEPENDENCY),
        Edge("bourgeois_state", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("withering_away", "state", EdgeType.NEGATION),
        Edge("withering_away", "communism", EdgeType.DEPENDENCY),
        Edge("commune_state", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("commune_state", "soviets", EdgeType.REFERENCE),
        Edge("soviets", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("soviets", "bourgeois_state", EdgeType.NEGATION),
        Edge("communism", "class_struggle", EdgeType.NEGATION),
        Edge("opportunism", "bourgeois_state", EdgeType.REFERENCE),
    ]
    return ("1917: The State and Revolution", vertices, edges)


def work_15_april_theses():
    """1917: April Theses.

    No support for provisional government, all power to soviets,
    transition from bourgeois-democratic to socialist revolution.
    """
    vertices = [
        Vertex("provisional_government", content="provisional government — bourgeois power after February"),
        Vertex("dual_power", content="dual power — soviets vs provisional government"),
        Vertex("all_power_to_soviets", content="all power to the soviets — transfer of state power"),
        Vertex("transition_to_socialist", content="transition to socialist revolution — growing over"),
    ]
    edges = [
        Edge("provisional_government", "bourgeois_state", EdgeType.REFERENCE),
        Edge("dual_power", "provisional_government", EdgeType.DEPENDENCY),
        Edge("dual_power", "soviets", EdgeType.DEPENDENCY),
        Edge("all_power_to_soviets", "soviets", EdgeType.DEPENDENCY),
        Edge("all_power_to_soviets", "provisional_government", EdgeType.NEGATION),
        Edge("transition_to_socialist", "bourgeois_democratic_revolution", EdgeType.DEPENDENCY),
        Edge("transition_to_socialist", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("transition_to_socialist", "all_power_to_soviets", EdgeType.REFERENCE),
    ]
    return ("1917: April Theses", vertices, edges)


def work_16_letters_from_afar():
    """1917: Letters from Afar.

    Analysis of February revolution, tasks of proletariat,
    arming of the people, workers' militia.
    """
    vertices = [
        Vertex("february_revolution", content="February revolution — overthrow of tsarism"),
        Vertex("armed_people", content="armed people — universal arming as democratic demand"),
        Vertex("tsarism", content="tsarism — autocratic monarchy"),
    ]
    edges = [
        Edge("february_revolution", "tsarism", EdgeType.NEGATION),
        Edge("february_revolution", "bourgeois_democratic_revolution", EdgeType.REFERENCE),
        Edge("february_revolution", "dual_power", EdgeType.DEPENDENCY),
        Edge("armed_people", "bourgeois_state", EdgeType.NEGATION),
        Edge("armed_people", "soviets", EdgeType.REFERENCE),
        Edge("tsarism", "state", EdgeType.DEPENDENCY),
    ]
    return ("1917: Letters from Afar", vertices, edges)


def work_17_can_bolsheviks_retain():
    """1917: Can the Bolsheviks Retain State Power?

    Practical questions of workers' state, accounting and control,
    soviet apparatus, grain monopoly.
    """
    vertices = [
        Vertex("accounting_control", content="accounting and control — key task of workers' state"),
        Vertex("workers_control", content="workers' control over production — transition measure"),
        Vertex("soviet_apparatus", content="soviet apparatus — new type of state machine"),
    ]
    edges = [
        Edge("accounting_control", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("workers_control", "accounting_control", EdgeType.DEPENDENCY),
        Edge("workers_control", "proletariat", EdgeType.DEPENDENCY),
        Edge("soviet_apparatus", "soviets", EdgeType.DEPENDENCY),
        Edge("soviet_apparatus", "commune_state", EdgeType.REFERENCE),
        Edge("soviet_apparatus", "bourgeois_state", EdgeType.NEGATION),
    ]
    return ("1917: Can the Bolsheviks Retain State Power?", vertices, edges)


def work_18_immediate_tasks():
    """1918: The Immediate Tasks of the Soviet Government.

    From smashing to building, labor discipline, one-man management,
    state capitalism under workers' state.
    """
    vertices = [
        Vertex("labor_discipline", content="labor discipline — new proletarian discipline"),
        Vertex("one_man_management", content="one-man management — combining democracy with iron discipline"),
        Vertex("state_capitalism_transitional", content="state capitalism under dictatorship of proletariat — transitional form"),
        Vertex("socialist_construction", content="socialist construction — building new economic base"),
    ]
    edges = [
        Edge("labor_discipline", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("labor_discipline", "proletariat", EdgeType.REFERENCE),
        Edge("one_man_management", "democratic_centralism", EdgeType.REFERENCE),
        Edge("one_man_management", "labor_discipline", EdgeType.DEPENDENCY),
        Edge("state_capitalism_transitional", "capitalism", EdgeType.REFERENCE),
        Edge("state_capitalism_transitional", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("state_capitalism_transitional", "accounting_control", EdgeType.DEPENDENCY),
        Edge("socialist_construction", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("socialist_construction", "smashing_state", EdgeType.DEPENDENCY),
    ]
    return ("1918: The Immediate Tasks of the Soviet Government", vertices, edges)


def work_19_renegade_kautsky():
    """1918: The Proletarian Revolution and the Renegade Kautsky.

    Critique of Kautsky, dictatorship vs democracy,
    bourgeois democracy vs proletarian democracy.
    """
    vertices = [
        Vertex("bourgeois_democracy", content="bourgeois democracy — democracy for the exploiters"),
        Vertex("proletarian_democracy", content="proletarian democracy — democracy for the exploited"),
        Vertex("kautsky_renegade", content="Kautsky as renegade — pure democracy covers class content"),
        Vertex("pure_democracy", content="pure democracy — classless democracy is illusion"),
    ]
    edges = [
        Edge("bourgeois_democracy", "proletarian_democracy", EdgeType.NEGATION),
        Edge("bourgeois_democracy", "bourgeois_state", EdgeType.DEPENDENCY),
        Edge("proletarian_democracy", "soviets", EdgeType.DEPENDENCY),
        Edge("proletarian_democracy", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("kautsky_renegade", "opportunism", EdgeType.DEPENDENCY),
        Edge("kautsky_renegade", "pure_democracy", EdgeType.DEPENDENCY),
        Edge("pure_democracy", "bourgeois_democracy", EdgeType.REFERENCE),
        Edge("pure_democracy", "class_struggle", EdgeType.NEGATION),
        Edge("kautsky_renegade", "second_international", EdgeType.REFERENCE),
    ]
    return ("1918: The Proletarian Revolution and the Renegade Kautsky", vertices, edges)


def work_20_economics_politics_dictatorship():
    """1919: Economics and Politics in the Era of the Dictatorship of the Proletariat.

    Transitional economy, three forms: socialism, small commodity, capitalism.
    Class struggle continues under dictatorship.
    """
    vertices = [
        Vertex("transitional_economy", content="transitional economy — mixed forms under dictatorship"),
        Vertex("small_commodity_production", content="small commodity production — generates capitalism spontaneously"),
        Vertex("class_struggle_under_dictatorship", content="class struggle continues under dictatorship of proletariat"),
    ]
    edges = [
        Edge("transitional_economy", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("transitional_economy", "socialism", EdgeType.REFERENCE) if False else
        Edge("transitional_economy", "socialist_construction", EdgeType.REFERENCE),
        Edge("small_commodity_production", "commodity_production", EdgeType.DEPENDENCY),
        Edge("small_commodity_production", "capitalism", EdgeType.REFERENCE),
        Edge("small_commodity_production", "peasantry", EdgeType.DEPENDENCY),
        Edge("class_struggle_under_dictatorship", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("class_struggle_under_dictatorship", "class_struggle", EdgeType.DEPENDENCY),
        Edge("class_struggle_under_dictatorship", "bourgeoisie", EdgeType.REFERENCE),
    ]
    return ("1919: Economics and Politics in the Era of the Dictatorship", vertices, edges)


def work_21_left_wing_communism():
    """1920: Left-Wing Communism: An Infantile Disorder.

    Against ultra-leftism, flexibility of tactics, compromises,
    participation in parliament and reactionary trade unions.
    """
    vertices = [
        Vertex("ultra_leftism", content="ultra-leftism — infantile disorder, rejection of compromises"),
        Vertex("tactical_flexibility", content="tactical flexibility — all forms of struggle"),
        Vertex("compromise", content="compromise — permissible when serving revolution"),
        Vertex("parliamentarism", content="revolutionary parliamentarism — using bourgeois parliament"),
        Vertex("mass_line", content="connection with the masses — party must not lose contact"),
    ]
    edges = [
        Edge("ultra_leftism", "revolutionary_program", EdgeType.REFERENCE),
        Edge("ultra_leftism", "tactical_flexibility", EdgeType.NEGATION),
        Edge("tactical_flexibility", "vanguard", EdgeType.DEPENDENCY),
        Edge("tactical_flexibility", "democratic_centralism", EdgeType.REFERENCE),
        Edge("compromise", "tactical_flexibility", EdgeType.DEPENDENCY),
        Edge("compromise", "opportunism", EdgeType.NEGATION),
        Edge("parliamentarism", "bourgeois_democracy", EdgeType.REFERENCE),
        Edge("parliamentarism", "tactical_flexibility", EdgeType.DEPENDENCY),
        Edge("mass_line", "vanguard", EdgeType.DEPENDENCY),
        Edge("mass_line", "proletariat", EdgeType.DEPENDENCY),
        Edge("ultra_leftism", "opportunism", EdgeType.NEGATION),
    ]
    return ("1920: Left-Wing Communism: An Infantile Disorder", vertices, edges)


def work_22_comintern_report():
    """1920: Report on the International Situation (Second Congress of the Comintern).

    World revolution, colonial question, national-colonial thesis.
    """
    vertices = [
        Vertex("world_revolution", content="world revolution — international character of proletarian revolution"),
        Vertex("comintern", content="Communist International — world party of revolution"),
        Vertex("colonial_question", content="colonial question — alliance with national liberation movements"),
    ]
    edges = [
        Edge("world_revolution", "imperialism", EdgeType.DEPENDENCY),
        Edge("world_revolution", "dictatorship_proletariat", EdgeType.REFERENCE),
        Edge("comintern", "party", EdgeType.DEPENDENCY),
        Edge("comintern", "world_revolution", EdgeType.DEPENDENCY),
        Edge("comintern", "second_international", EdgeType.NEGATION),
        Edge("colonial_question", "national_liberation", EdgeType.DEPENDENCY),
        Edge("colonial_question", "oppressed_nations", EdgeType.DEPENDENCY),
        Edge("colonial_question", "imperialism", EdgeType.REFERENCE),
    ]
    return ("1920: Report on International Situation (Comintern)", vertices, edges)


def work_23_tax_in_kind():
    """1921: On the Tax in Kind (New Economic Policy).

    NEP as strategic retreat, state capitalism, small peasant economy,
    transitional measures.
    """
    vertices = [
        Vertex("NEP", content="New Economic Policy — strategic retreat, market relations"),
        Vertex("tax_in_kind", content="tax in kind — replacing surplus appropriation"),
        Vertex("war_communism", content="war communism — direct transition attempt, failed"),
        Vertex("free_trade", content="free trade — limited, under state supervision"),
        Vertex("strategic_retreat", content="strategic retreat — orderly withdrawal to defensible positions"),
    ]
    edges = [
        Edge("NEP", "tax_in_kind", EdgeType.DEPENDENCY),
        Edge("NEP", "war_communism", EdgeType.NEGATION),
        Edge("NEP", "state_capitalism_transitional", EdgeType.DEPENDENCY),
        Edge("NEP", "strategic_retreat", EdgeType.DEPENDENCY),
        Edge("tax_in_kind", "peasantry", EdgeType.DEPENDENCY),
        Edge("war_communism", "socialist_construction", EdgeType.REFERENCE),
        Edge("free_trade", "NEP", EdgeType.DEPENDENCY),
        Edge("free_trade", "small_commodity_production", EdgeType.DEPENDENCY),
        Edge("strategic_retreat", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("NEP", "worker_peasant_alliance", EdgeType.REFERENCE),
    ]
    return ("1921: On the Tax in Kind (NEP)", vertices, edges)


def work_24_trade_unions():
    """1921: Once Again on the Trade Unions.

    Dialectics applied to trade union debate, Trotsky vs Lenin,
    trade unions as school of communism.
    """
    vertices = [
        Vertex("trade_unions_school", content="trade unions as school of communism — transmission belt"),
        Vertex("transmission_belt", content="transmission belt — party-class-masses link"),
        Vertex("bureaucratism", content="bureaucratism — deformation of workers' state"),
    ]
    edges = [
        Edge("trade_unions_school", "proletariat", EdgeType.DEPENDENCY),
        Edge("trade_unions_school", "trade_unionism", EdgeType.REFERENCE),
        Edge("trade_unions_school", "dictatorship_proletariat", EdgeType.REFERENCE),
        Edge("transmission_belt", "party", EdgeType.DEPENDENCY),
        Edge("transmission_belt", "mass_line", EdgeType.DEPENDENCY),
        Edge("bureaucratism", "state", EdgeType.DEPENDENCY),
        Edge("bureaucratism", "dictatorship_proletariat", EdgeType.REFERENCE),
        Edge("bureaucratism", "commune_state", EdgeType.NEGATION),
    ]
    return ("1921: Once Again on the Trade Unions", vertices, edges)


def work_25_militant_materialism():
    """1922: On the Significance of Militant Materialism.

    Alliance of materialist philosophers and natural scientists,
    study of Hegel's dialectics, militant atheism.
    """
    vertices = [
        Vertex("militant_materialism", content="militant materialism — systematic materialist propaganda"),
        Vertex("militant_atheism", content="militant atheism — struggle against religion"),
        Vertex("natural_science_alliance", content="alliance with natural scientists — materialist epistemology"),
    ]
    edges = [
        Edge("militant_materialism", "materialism", EdgeType.DEPENDENCY),
        Edge("militant_materialism", "dialectics", EdgeType.DEPENDENCY),
        Edge("militant_materialism", "partisanship_philosophy", EdgeType.REFERENCE),
        Edge("militant_atheism", "materialism", EdgeType.DEPENDENCY),
        Edge("militant_atheism", "idealism", EdgeType.NEGATION),
        Edge("natural_science_alliance", "materialism", EdgeType.DEPENDENCY),
        Edge("natural_science_alliance", "practice", EdgeType.REFERENCE),
    ]
    return ("1922: On the Significance of Militant Materialism", vertices, edges)


def work_26_testament():
    """1922: Letter to the Congress (Testament).

    Assessment of leadership, danger of split, Stalin's rudeness,
    enlarging Central Committee.
    """
    vertices = [
        Vertex("party_unity", content="party unity — danger of split between Stalin and Trotsky"),
        Vertex("collective_leadership", content="collective leadership — enlarge Central Committee with workers"),
        Vertex("bureaucratic_deformation", content="bureaucratic deformation — state apparatus infected by old officials"),
    ]
    edges = [
        Edge("party_unity", "party", EdgeType.DEPENDENCY),
        Edge("party_unity", "democratic_centralism", EdgeType.REFERENCE),
        Edge("collective_leadership", "party", EdgeType.DEPENDENCY),
        Edge("collective_leadership", "proletariat", EdgeType.REFERENCE),
        Edge("bureaucratic_deformation", "bureaucratism", EdgeType.DEPENDENCY),
        Edge("bureaucratic_deformation", "bourgeois_state", EdgeType.REFERENCE),
        Edge("bureaucratic_deformation", "commune_state", EdgeType.NEGATION),
    ]
    return ("1922: Letter to the Congress (Testament)", vertices, edges)


def work_27_on_cooperation():
    """1923: On Cooperation.

    Cooperatives as path to socialism for peasantry,
    cultural revolution, civilizing the peasant.
    """
    vertices = [
        Vertex("cooperation", content="cooperation — path to socialism for peasants under dictatorship"),
        Vertex("cultural_revolution", content="cultural revolution — civilize, educate, raise culture"),
        Vertex("cooperative_plan", content="cooperative plan — voluntary, gradual, state-aided"),
    ]
    edges = [
        Edge("cooperation", "NEP", EdgeType.DEPENDENCY),
        Edge("cooperation", "peasantry", EdgeType.DEPENDENCY),
        Edge("cooperation", "socialist_construction", EdgeType.DEPENDENCY),
        Edge("cultural_revolution", "cooperation", EdgeType.DEPENDENCY),
        Edge("cultural_revolution", "consciousness", EdgeType.REFERENCE),
        Edge("cooperative_plan", "cooperation", EdgeType.DEPENDENCY),
        Edge("cooperative_plan", "worker_peasant_alliance", EdgeType.REFERENCE),
    ]
    return ("1923: On Cooperation", vertices, edges)


def work_28_better_fewer_but_better():
    """1923: Better Fewer, But Better.

    Quality of state apparatus, Workers' and Peasants' Inspection,
    struggle against bureaucracy.
    """
    vertices = [
        Vertex("state_apparatus_quality", content="state apparatus quality — fewer but better officials"),
        Vertex("workers_peasants_inspection", content="Workers' and Peasants' Inspection — Rabkrin reform"),
    ]
    edges = [
        Edge("state_apparatus_quality", "bureaucratic_deformation", EdgeType.NEGATION),
        Edge("state_apparatus_quality", "soviet_apparatus", EdgeType.DEPENDENCY),
        Edge("workers_peasants_inspection", "state_apparatus_quality", EdgeType.DEPENDENCY),
        Edge("workers_peasants_inspection", "workers_control", EdgeType.REFERENCE),
        Edge("state_apparatus_quality", "cultural_revolution", EdgeType.REFERENCE),
    ]
    return ("1923: Better Fewer, But Better", vertices, edges)


def work_29_our_revolution():
    """1923: Our Revolution (on Sukhanov's Notes).

    Defense of October, revolution first in backward country,
    dialectics of history vs mechanical prerequisite-ism.
    """
    vertices = [
        Vertex("revolution_in_backward_country", content="revolution possible first in backward country"),
        Vertex("mechanical_determinism", content="mechanical determinism — revolution only after full capitalist development"),
        Vertex("dialectical_development", content="dialectical development — altered order of historical steps"),
    ]
    edges = [
        Edge("revolution_in_backward_country", "uneven_development", EdgeType.DEPENDENCY),
        Edge("revolution_in_backward_country", "dialectics", EdgeType.REFERENCE),
        Edge("mechanical_determinism", "revolution_in_backward_country", EdgeType.NEGATION),
        Edge("mechanical_determinism", "revisionism", EdgeType.REFERENCE),
        Edge("dialectical_development", "dialectics", EdgeType.DEPENDENCY),
        Edge("dialectical_development", "mechanical_determinism", EdgeType.NEGATION),
        Edge("dialectical_development", "leap", EdgeType.REFERENCE),
    ]
    return ("1923: Our Revolution", vertices, edges)


def work_30_pages_from_diary():
    """1923: Pages from a Diary.

    State of literacy, cultural tasks, patience of construction.
    """
    vertices = [
        Vertex("literacy", content="literacy — basic cultural task of revolution"),
        Vertex("cultural_backwardness", content="cultural backwardness — obstacle to socialist construction"),
    ]
    edges = [
        Edge("literacy", "cultural_revolution", EdgeType.DEPENDENCY),
        Edge("cultural_backwardness", "socialist_construction", EdgeType.NEGATION),
        Edge("cultural_backwardness", "peasantry", EdgeType.REFERENCE),
        Edge("literacy", "cultural_backwardness", EdgeType.NEGATION),
    ]
    return ("1923: Pages from a Diary", vertices, edges)


# ---------------------------------------------------------------------------
# Work sequence
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_01_friends_of_people,
    work_02_economic_content_narodism,
    work_03_development_capitalism,
    work_04_protest,
    work_05_what_is_to_be_done,
    work_06_one_step_forward,
    work_07_two_tactics,
    work_08_materialism_empiriocriticism,
    work_09_three_sources,
    work_10_self_determination,
    work_11_philosophical_notebooks,
    work_12_collapse_second_international,
    work_13_imperialism,
    work_14_state_and_revolution,
    work_15_april_theses,
    work_16_letters_from_afar,
    work_17_can_bolsheviks_retain,
    work_18_immediate_tasks,
    work_19_renegade_kautsky,
    work_20_economics_politics_dictatorship,
    work_21_left_wing_communism,
    work_22_comintern_report,
    work_23_tax_in_kind,
    work_24_trade_unions,
    work_25_militant_materialism,
    work_26_testament,
    work_27_on_cooperation,
    work_28_better_fewer_but_better,
    work_29_our_revolution,
    work_30_pages_from_diary,
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    n_works = len(ALL_WORKS)
    print("=" * 70)
    print("LENIN COLLECTED WORKS — FULL INCREMENTAL TRAVERSAL")
    print(f"{n_works} works, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    work_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for w_idx, w_fn in enumerate(ALL_WORKS):
        work_name, w_vertices, w_edges = w_fn()
        print(f"\n{'─' * 60}")
        print(f"Work {w_idx + 1}/{n_works}: {work_name}")
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
            # If position was lost, reset
            if engine.position not in graph.vertices:
                engine.position = graph.active_vertex_ids()[0]

        # 4. Digest: run until beta_1 stable for 50 consecutive steps
        stable_count = 0
        last_beta = compute_beta_1(engine.k_active)
        steps_this_work = 0
        max_steps = 2000  # safety cap per work

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
        "experiment": "lenin_collected_works_incremental",
        "source": "Lenin, Collected Works — 30 works by publication date (1893-1923)",
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

    output_path = "experiment_lenin.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
