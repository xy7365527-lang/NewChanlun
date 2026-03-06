"""Marx Collected Works — incremental traversal experiment.

Each work is injected as a dialectical complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Marx's thought
from early Hegelian philosophy through mature political economy,
as an incremental topological growth process.

Covers ~32 works spanning 1841-1894 (including posthumous volumes).
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


def w_doctoral_dissertation():
    """1841: Doctoral Dissertation — Difference between Democritus and Epicurus."""
    vertices = [
        Vertex("epicurus", content="Epicurus — freedom of self-consciousness"),
        Vertex("democritus", content="Democritus — mechanical determinism"),
        Vertex("self_consciousness", content="self-consciousness"),
        Vertex("atom_swerve", content="clinamen — the swerve of the atom"),
        Vertex("necessity", content="necessity / determinism"),
        Vertex("contingency", content="contingency / freedom"),
        Vertex("abstract_individual", content="abstract individual"),
        Vertex("philosophy", content="philosophy as activity of self-consciousness"),
        Vertex("religion_critique_early", content="critique of religion (early)"),
        Vertex("prometheus", content="Prometheus — first saint of philosophy"),
    ]
    edges = [
        Edge("epicurus", "democritus", EdgeType.NEGATION),
        Edge("atom_swerve", "necessity", EdgeType.NEGATION),
        Edge("contingency", "atom_swerve", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "contingency", EdgeType.DEPENDENCY),
        Edge("self_consciousness", "necessity", EdgeType.NEGATION),
        Edge("abstract_individual", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("philosophy", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("religion_critique_early", "philosophy", EdgeType.DEPENDENCY),
        Edge("prometheus", "philosophy", EdgeType.REFERENCE),
        Edge("epicurus", "self_consciousness", EdgeType.DEPENDENCY),
        Edge("democritus", "necessity", EdgeType.DEPENDENCY),
    ]
    return "1841: Doctoral Dissertation", vertices, edges


def w_critique_hegel_right():
    """1843: Critique of Hegel's Philosophy of Right."""
    vertices = [
        Vertex("hegel_state", content="Hegel's rational state"),
        Vertex("civil_society", content="civil society (bürgerliche Gesellschaft)"),
        Vertex("state", content="the state"),
        Vertex("bureaucracy", content="bureaucracy"),
        Vertex("democracy", content="democracy as true resolution"),
        Vertex("species_being", content="Gattungswesen — species-being"),
        Vertex("inversion_method", content="inversion of subject and predicate"),
        Vertex("political_emancipation", content="political emancipation"),
        Vertex("real_democracy", content="real democracy vs formal state"),
        Vertex("alienation", content="alienation (Entfremdung)"),
    ]
    edges = [
        Edge("inversion_method", "hegel_state", EdgeType.NEGATION),
        Edge("civil_society", "state", EdgeType.NEGATION),
        Edge("state", "civil_society", EdgeType.DEPENDENCY),
        Edge("bureaucracy", "state", EdgeType.DEPENDENCY),
        Edge("democracy", "bureaucracy", EdgeType.NEGATION),
        Edge("real_democracy", "democracy", EdgeType.DEPENDENCY),
        Edge("real_democracy", "hegel_state", EdgeType.NEGATION),
        Edge("species_being", "self_consciousness", EdgeType.SUBLATION),
        Edge("alienation", "species_being", EdgeType.NEGATION),
        Edge("political_emancipation", "alienation", EdgeType.DEPENDENCY),
        Edge("political_emancipation", "state", EdgeType.DEPENDENCY),
        Edge("inversion_method", "philosophy", EdgeType.DEPENDENCY),
    ]
    return "1843: Critique of Hegel's Philosophy of Right", vertices, edges


def w_jewish_question():
    """1843: On the Jewish Question."""
    vertices = [
        Vertex("human_emancipation", content="human emancipation"),
        Vertex("rights_of_man", content="rights of man (bourgeois rights)"),
        Vertex("citoyen", content="citoyen — political citizen"),
        Vertex("bourgeois_individual", content="bourgeois — private individual"),
        Vertex("religion_private", content="religion as private matter"),
        Vertex("money_power", content="money as worldly god"),
        Vertex("abstract_citizen", content="abstract political citizen"),
    ]
    edges = [
        Edge("human_emancipation", "political_emancipation", EdgeType.SUBLATION),
        Edge("human_emancipation", "rights_of_man", EdgeType.NEGATION),
        Edge("citoyen", "bourgeois_individual", EdgeType.NEGATION),
        Edge("abstract_citizen", "citoyen", EdgeType.DEPENDENCY),
        Edge("bourgeois_individual", "civil_society", EdgeType.DEPENDENCY),
        Edge("rights_of_man", "bourgeois_individual", EdgeType.DEPENDENCY),
        Edge("religion_private", "political_emancipation", EdgeType.DEPENDENCY),
        Edge("money_power", "alienation", EdgeType.DEPENDENCY),
        Edge("human_emancipation", "alienation", EdgeType.NEGATION),
    ]
    return "1843: On the Jewish Question", vertices, edges


def w_paris_manuscripts():
    """1844: Economic and Philosophic Manuscripts (Paris Manuscripts)."""
    vertices = [
        Vertex("estranged_labor", content="estranged/alienated labor"),
        Vertex("labor", content="labor as life-activity"),
        Vertex("private_property", content="private property"),
        Vertex("objectification", content="objectification (Vergegenständlichung)"),
        Vertex("communism_early", content="communism as positive supersession"),
        Vertex("humanism", content="real humanism = naturalism"),
        Vertex("political_economy_critique", content="critique of political economy"),
        Vertex("wage_labor_early", content="wage labor"),
        Vertex("capital_early", content="capital (early)"),
        Vertex("rent_early", content="rent"),
        Vertex("sensuous_activity", content="sensuous human activity"),
        Vertex("nature", content="nature as man's inorganic body"),
    ]
    edges = [
        Edge("estranged_labor", "labor", EdgeType.NEGATION),
        Edge("estranged_labor", "alienation", EdgeType.DEPENDENCY),
        Edge("private_property", "estranged_labor", EdgeType.DEPENDENCY),
        Edge("objectification", "labor", EdgeType.DEPENDENCY),
        Edge("estranged_labor", "objectification", EdgeType.NEGATION),
        Edge("communism_early", "private_property", EdgeType.NEGATION),
        Edge("communism_early", "estranged_labor", EdgeType.NEGATION),
        Edge("communism_early", "alienation", EdgeType.NEGATION),
        Edge("humanism", "communism_early", EdgeType.DEPENDENCY),
        Edge("humanism", "species_being", EdgeType.DEPENDENCY),
        Edge("political_economy_critique", "wage_labor_early", EdgeType.DEPENDENCY),
        Edge("political_economy_critique", "capital_early", EdgeType.DEPENDENCY),
        Edge("political_economy_critique", "rent_early", EdgeType.DEPENDENCY),
        Edge("sensuous_activity", "self_consciousness", EdgeType.SUBLATION),
        Edge("nature", "sensuous_activity", EdgeType.DEPENDENCY),
        Edge("labor", "nature", EdgeType.DEPENDENCY),
    ]
    return "1844: Economic and Philosophic Manuscripts", vertices, edges


def w_critical_notes_king_prussia():
    """1844: Critical Notes on 'The King of Prussia'."""
    vertices = [
        Vertex("social_revolution", content="social revolution vs political revolution"),
        Vertex("political_revolution", content="political revolution (partial)"),
        Vertex("pauperism", content="pauperism"),
        Vertex("proletariat", content="proletariat"),
        Vertex("state_impotence", content="impotence of the state"),
    ]
    edges = [
        Edge("social_revolution", "political_revolution", EdgeType.NEGATION),
        Edge("political_revolution", "state", EdgeType.DEPENDENCY),
        Edge("social_revolution", "proletariat", EdgeType.DEPENDENCY),
        Edge("pauperism", "private_property", EdgeType.DEPENDENCY),
        Edge("state_impotence", "pauperism", EdgeType.DEPENDENCY),
        Edge("state_impotence", "state", EdgeType.NEGATION),
        Edge("proletariat", "estranged_labor", EdgeType.DEPENDENCY),
    ]
    return "1844: Critical Notes on King of Prussia", vertices, edges


def w_theses_feuerbach():
    """1845: Theses on Feuerbach."""
    vertices = [
        Vertex("praxis", content="revolutionary practical-critical activity"),
        Vertex("feuerbach_materialism", content="Feuerbach's contemplative materialism"),
        Vertex("idealism_active", content="idealism — active side"),
        Vertex("social_relations", content="ensemble of social relations"),
        Vertex("interpret_world", content="philosophers interpret the world"),
        Vertex("change_world", content="the point is to change it"),
        Vertex("new_materialism", content="new materialism (standpoint of social humanity)"),
    ]
    edges = [
        Edge("praxis", "feuerbach_materialism", EdgeType.NEGATION),
        Edge("praxis", "idealism_active", EdgeType.SUBLATION),
        Edge("new_materialism", "feuerbach_materialism", EdgeType.SUBLATION),
        Edge("new_materialism", "praxis", EdgeType.DEPENDENCY),
        Edge("social_relations", "species_being", EdgeType.SUBLATION),
        Edge("change_world", "interpret_world", EdgeType.NEGATION),
        Edge("change_world", "praxis", EdgeType.DEPENDENCY),
        Edge("sensuous_activity", "praxis", EdgeType.DEPENDENCY),
    ]
    return "1845: Theses on Feuerbach", vertices, edges


def w_holy_family():
    """1845: The Holy Family (with Engels)."""
    vertices = [
        Vertex("critical_criticism", content="critical criticism (Bauer)"),
        Vertex("mass", content="the mass / real movement"),
        Vertex("spirit_matter_split", content="spirit vs matter (Young Hegelian)"),
        Vertex("real_humanism_hf", content="real humanism (Holy Family)"),
        Vertex("proletariat_mission", content="proletariat's historical mission"),
    ]
    edges = [
        Edge("mass", "critical_criticism", EdgeType.NEGATION),
        Edge("real_humanism_hf", "critical_criticism", EdgeType.NEGATION),
        Edge("real_humanism_hf", "humanism", EdgeType.REFERENCE),
        Edge("spirit_matter_split", "critical_criticism", EdgeType.DEPENDENCY),
        Edge("mass", "proletariat", EdgeType.DEPENDENCY),
        Edge("proletariat_mission", "proletariat", EdgeType.DEPENDENCY),
        Edge("proletariat_mission", "private_property", EdgeType.NEGATION),
    ]
    return "1845: The Holy Family", vertices, edges


def w_german_ideology():
    """1846: The German Ideology (with Engels)."""
    vertices = [
        Vertex("historical_materialism", content="materialist conception of history"),
        Vertex("mode_of_production", content="mode of production (Produktionsweise)"),
        Vertex("forces_of_production", content="productive forces"),
        Vertex("relations_of_production", content="relations of production"),
        Vertex("division_of_labor", content="division of labor"),
        Vertex("ideology", content="ideology (camera obscura)"),
        Vertex("consciousness", content="consciousness determined by life"),
        Vertex("ruling_ideas", content="ruling ideas = ideas of ruling class"),
        Vertex("material_conditions", content="material conditions of life"),
        Vertex("class", content="class"),
        Vertex("class_struggle", content="class struggle"),
        Vertex("communism_gi", content="communism as real movement"),
    ]
    edges = [
        Edge("historical_materialism", "feuerbach_materialism", EdgeType.SUBLATION),
        Edge("historical_materialism", "praxis", EdgeType.DEPENDENCY),
        Edge("mode_of_production", "forces_of_production", EdgeType.DEPENDENCY),
        Edge("mode_of_production", "relations_of_production", EdgeType.DEPENDENCY),
        Edge("forces_of_production", "relations_of_production", EdgeType.NEGATION),
        Edge("division_of_labor", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("ideology", "consciousness", EdgeType.NEGATION),
        Edge("consciousness", "material_conditions", EdgeType.DEPENDENCY),
        Edge("ruling_ideas", "ideology", EdgeType.DEPENDENCY),
        Edge("ruling_ideas", "class", EdgeType.DEPENDENCY),
        Edge("class_struggle", "class", EdgeType.DEPENDENCY),
        Edge("class_struggle", "division_of_labor", EdgeType.DEPENDENCY),
        Edge("communism_gi", "communism_early", EdgeType.SUBLATION),
        Edge("communism_gi", "class_struggle", EdgeType.DEPENDENCY),
        Edge("communism_gi", "division_of_labor", EdgeType.NEGATION),
        Edge("material_conditions", "nature", EdgeType.DEPENDENCY),
        Edge("social_relations", "mode_of_production", EdgeType.DEPENDENCY),
    ]
    return "1846: The German Ideology", vertices, edges


def w_poverty_of_philosophy():
    """1847: The Poverty of Philosophy."""
    vertices = [
        Vertex("proudhon", content="Proudhon's petty-bourgeois socialism"),
        Vertex("use_value", content="use-value"),
        Vertex("exchange_value", content="exchange-value"),
        Vertex("value", content="value"),
        Vertex("constituted_value", content="Proudhon's constituted value"),
        Vertex("historical_categories", content="economic categories are historical"),
        Vertex("social_character_production", content="social character of production"),
    ]
    edges = [
        Edge("constituted_value", "use_value", EdgeType.DEPENDENCY),
        Edge("constituted_value", "exchange_value", EdgeType.DEPENDENCY),
        Edge("value", "constituted_value", EdgeType.NEGATION),
        Edge("value", "use_value", EdgeType.DEPENDENCY),
        Edge("value", "exchange_value", EdgeType.DEPENDENCY),
        Edge("historical_categories", "proudhon", EdgeType.NEGATION),
        Edge("historical_categories", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("social_character_production", "relations_of_production", EdgeType.DEPENDENCY),
        Edge("proudhon", "political_economy_critique", EdgeType.NEGATION),
    ]
    return "1847: The Poverty of Philosophy", vertices, edges


def w_wage_labour_and_capital():
    """1847: Wage Labour and Capital."""
    vertices = [
        Vertex("wage", content="wage"),
        Vertex("labor_power_early", content="labor as commodity (early)"),
        Vertex("nominal_vs_real_wage", content="nominal vs real wage"),
        Vertex("relative_wage", content="relative wage"),
        Vertex("capital_relation", content="capital as social relation"),
    ]
    edges = [
        Edge("wage", "labor_power_early", EdgeType.DEPENDENCY),
        Edge("wage", "capital_early", EdgeType.DEPENDENCY),
        Edge("nominal_vs_real_wage", "wage", EdgeType.DEPENDENCY),
        Edge("relative_wage", "wage", EdgeType.DEPENDENCY),
        Edge("relative_wage", "proletariat", EdgeType.DEPENDENCY),
        Edge("capital_relation", "capital_early", EdgeType.SUBLATION),
        Edge("capital_relation", "social_relations", EdgeType.DEPENDENCY),
        Edge("labor_power_early", "estranged_labor", EdgeType.REFERENCE),
    ]
    return "1847: Wage Labour and Capital", vertices, edges


def w_communist_manifesto():
    """1848: Communist Manifesto (with Engels)."""
    vertices = [
        Vertex("bourgeoisie", content="bourgeoisie"),
        Vertex("bourgeois_epoch", content="bourgeois epoch — constant revolution"),
        Vertex("world_market", content="world market"),
        Vertex("communist_party", content="communist party"),
        Vertex("abolition_property", content="abolition of bourgeois property"),
        Vertex("ten_measures", content="ten transitional measures"),
        Vertex("withering_state", content="public power loses political character"),
        Vertex("free_development", content="free development of each = condition for all"),
        Vertex("utopian_socialism", content="utopian socialism"),
        Vertex("history_class_struggle", content="all history is history of class struggles"),
        Vertex("spectre", content="spectre of communism"),
    ]
    edges = [
        Edge("history_class_struggle", "class_struggle", EdgeType.DEPENDENCY),
        Edge("bourgeoisie", "class", EdgeType.DEPENDENCY),
        Edge("proletariat", "bourgeoisie", EdgeType.NEGATION),
        Edge("bourgeois_epoch", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("bourgeois_epoch", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("world_market", "bourgeois_epoch", EdgeType.DEPENDENCY),
        Edge("communist_party", "proletariat", EdgeType.DEPENDENCY),
        Edge("abolition_property", "private_property", EdgeType.NEGATION),
        Edge("abolition_property", "communist_party", EdgeType.DEPENDENCY),
        Edge("ten_measures", "abolition_property", EdgeType.DEPENDENCY),
        Edge("withering_state", "state", EdgeType.NEGATION),
        Edge("withering_state", "class_struggle", EdgeType.DEPENDENCY),
        Edge("free_development", "communism_gi", EdgeType.DEPENDENCY),
        Edge("free_development", "proletariat_mission", EdgeType.DEPENDENCY),
        Edge("utopian_socialism", "communism_gi", EdgeType.NEGATION),
        Edge("spectre", "bourgeoisie", EdgeType.NEGATION),
    ]
    return "1848: Communist Manifesto", vertices, edges


def w_class_struggles_france():
    """1850: The Class Struggles in France."""
    vertices = [
        Vertex("finance_aristocracy", content="finance aristocracy"),
        Vertex("industrial_bourgeoisie", content="industrial bourgeoisie"),
        Vertex("petty_bourgeoisie", content="petty bourgeoisie"),
        Vertex("lumpenproletariat", content="lumpenproletariat"),
        Vertex("peasantry", content="peasantry"),
        Vertex("revolution_1848", content="revolution of 1848"),
        Vertex("class_fraction", content="class fractions"),
        Vertex("permanent_revolution_early", content="permanent revolution (early)"),
    ]
    edges = [
        Edge("finance_aristocracy", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("industrial_bourgeoisie", "bourgeoisie", EdgeType.DEPENDENCY),
        Edge("petty_bourgeoisie", "class", EdgeType.DEPENDENCY),
        Edge("lumpenproletariat", "proletariat", EdgeType.NEGATION),
        Edge("peasantry", "class", EdgeType.DEPENDENCY),
        Edge("revolution_1848", "class_struggle", EdgeType.DEPENDENCY),
        Edge("class_fraction", "class", EdgeType.DEPENDENCY),
        Edge("permanent_revolution_early", "social_revolution", EdgeType.DEPENDENCY),
        Edge("permanent_revolution_early", "revolution_1848", EdgeType.DEPENDENCY),
    ]
    return "1850: The Class Struggles in France", vertices, edges


def w_eighteenth_brumaire():
    """1852: The Eighteenth Brumaire of Louis Bonaparte."""
    vertices = [
        Vertex("bonapartism", content="Bonapartism — state autonomy"),
        Vertex("farce_tragedy", content="first time tragedy, second time farce"),
        Vertex("men_make_history", content="men make their own history, but not as they please"),
        Vertex("state_machine", content="enormous state machine / parasitic body"),
        Vertex("smallholding_peasant", content="smallholding peasantry as sack of potatoes"),
        Vertex("class_representation", content="political representation of class"),
        Vertex("tradition_dead_generations", content="tradition of dead generations"),
    ]
    edges = [
        Edge("bonapartism", "state", EdgeType.SUBLATION),
        Edge("bonapartism", "bourgeoisie", EdgeType.NEGATION),
        Edge("bonapartism", "proletariat", EdgeType.NEGATION),
        Edge("farce_tragedy", "revolution_1848", EdgeType.REFERENCE),
        Edge("men_make_history", "historical_materialism", EdgeType.DEPENDENCY),
        Edge("men_make_history", "material_conditions", EdgeType.DEPENDENCY),
        Edge("state_machine", "bureaucracy", EdgeType.DEPENDENCY),
        Edge("state_machine", "state", EdgeType.DEPENDENCY),
        Edge("smallholding_peasant", "peasantry", EdgeType.DEPENDENCY),
        Edge("class_representation", "class", EdgeType.DEPENDENCY),
        Edge("class_representation", "ideology", EdgeType.DEPENDENCY),
        Edge("tradition_dead_generations", "consciousness", EdgeType.DEPENDENCY),
    ]
    return "1852: The Eighteenth Brumaire", vertices, edges


def w_articles_india():
    """1853: Articles on India (New York Tribune)."""
    vertices = [
        Vertex("oriental_despotism", content="oriental despotism / Asiatic mode"),
        Vertex("village_community", content="self-sufficient village community"),
        Vertex("british_colonialism", content="British colonialism in India"),
        Vertex("double_mission", content="double mission of colonialism"),
        Vertex("asiatic_production", content="Asiatic mode of production"),
    ]
    edges = [
        Edge("oriental_despotism", "state", EdgeType.DEPENDENCY),
        Edge("village_community", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("british_colonialism", "village_community", EdgeType.NEGATION),
        Edge("double_mission", "british_colonialism", EdgeType.DEPENDENCY),
        Edge("double_mission", "world_market", EdgeType.DEPENDENCY),
        Edge("asiatic_production", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("asiatic_production", "oriental_despotism", EdgeType.DEPENDENCY),
    ]
    return "1853: Articles on India", vertices, edges


def w_grundrisse_intro():
    """1857: Introduction to Grundrisse."""
    vertices = [
        Vertex("method_pe", content="method of political economy"),
        Vertex("concrete_abstract", content="rise from abstract to concrete"),
        Vertex("production_general", content="production in general"),
        Vertex("production_distribution", content="production-distribution-exchange-consumption"),
        Vertex("historically_specific", content="historically specific categories"),
        Vertex("robinson_crusoe", content="Robinson Crusoe critique"),
    ]
    edges = [
        Edge("method_pe", "historical_materialism", EdgeType.DEPENDENCY),
        Edge("concrete_abstract", "method_pe", EdgeType.DEPENDENCY),
        Edge("concrete_abstract", "inversion_method", EdgeType.SUBLATION),
        Edge("production_general", "production_distribution", EdgeType.DEPENDENCY),
        Edge("historically_specific", "historical_categories", EdgeType.DEPENDENCY),
        Edge("historically_specific", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("robinson_crusoe", "abstract_individual", EdgeType.NEGATION),
    ]
    return "1857: Introduction to Grundrisse", vertices, edges


def w_grundrisse():
    """1857-58: Grundrisse."""
    vertices = [
        Vertex("money_form", content="money form"),
        Vertex("commodity", content="commodity"),
        Vertex("abstract_labor", content="abstract labor"),
        Vertex("concrete_labor", content="concrete/useful labor"),
        Vertex("surplus_value", content="surplus value (Mehrwert)"),
        Vertex("necessary_labor_time", content="necessary labor time"),
        Vertex("surplus_labor_time", content="surplus labor time"),
        Vertex("circulation", content="circulation process"),
        Vertex("capital_general", content="capital in general"),
        Vertex("fixed_capital", content="fixed capital"),
        Vertex("variable_capital", content="variable capital"),
        Vertex("general_intellect", content="general intellect / social knowledge"),
        Vertex("machine_fragment", content="fragment on machines"),
        Vertex("precapitalist_forms", content="pre-capitalist economic formations"),
        Vertex("formal_subsumption", content="formal subsumption of labor"),
        Vertex("real_subsumption", content="real subsumption of labor"),
    ]
    edges = [
        Edge("commodity", "use_value", EdgeType.DEPENDENCY),
        Edge("commodity", "exchange_value", EdgeType.DEPENDENCY),
        Edge("money_form", "commodity", EdgeType.DEPENDENCY),
        Edge("money_form", "money_power", EdgeType.SUBLATION),
        Edge("abstract_labor", "labor", EdgeType.DEPENDENCY),
        Edge("concrete_labor", "labor", EdgeType.DEPENDENCY),
        Edge("abstract_labor", "concrete_labor", EdgeType.NEGATION),
        Edge("value", "abstract_labor", EdgeType.DEPENDENCY),
        Edge("surplus_value", "surplus_labor_time", EdgeType.DEPENDENCY),
        Edge("surplus_value", "necessary_labor_time", EdgeType.NEGATION),
        Edge("surplus_labor_time", "necessary_labor_time", EdgeType.NEGATION),
        Edge("circulation", "commodity", EdgeType.DEPENDENCY),
        Edge("circulation", "money_form", EdgeType.DEPENDENCY),
        Edge("capital_general", "surplus_value", EdgeType.DEPENDENCY),
        Edge("capital_general", "capital_relation", EdgeType.SUBLATION),
        Edge("fixed_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("variable_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("fixed_capital", "variable_capital", EdgeType.NEGATION),
        Edge("general_intellect", "fixed_capital", EdgeType.DEPENDENCY),
        Edge("machine_fragment", "general_intellect", EdgeType.DEPENDENCY),
        Edge("machine_fragment", "real_subsumption", EdgeType.DEPENDENCY),
        Edge("real_subsumption", "formal_subsumption", EdgeType.SUBLATION),
        Edge("formal_subsumption", "estranged_labor", EdgeType.DEPENDENCY),
        Edge("precapitalist_forms", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("precapitalist_forms", "historical_materialism", EdgeType.DEPENDENCY),
    ]
    return "1857-58: Grundrisse", vertices, edges


def w_contribution_critique_pe():
    """1859: A Contribution to the Critique of Political Economy."""
    vertices = [
        Vertex("base_superstructure", content="base and superstructure"),
        Vertex("social_being", content="social being determines consciousness"),
        Vertex("epoch_revolution", content="epoch of social revolution"),
        Vertex("commodity_analysis", content="commodity as cell-form"),
        Vertex("simple_form_value", content="simple form of value"),
        Vertex("expanded_form_value", content="expanded form of value"),
        Vertex("general_form_value", content="general form of value"),
    ]
    edges = [
        Edge("base_superstructure", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("base_superstructure", "ideology", EdgeType.DEPENDENCY),
        Edge("social_being", "consciousness", EdgeType.NEGATION),
        Edge("social_being", "material_conditions", EdgeType.DEPENDENCY),
        Edge("epoch_revolution", "forces_of_production", EdgeType.DEPENDENCY),
        Edge("epoch_revolution", "relations_of_production", EdgeType.NEGATION),
        Edge("commodity_analysis", "commodity", EdgeType.DEPENDENCY),
        Edge("simple_form_value", "value", EdgeType.DEPENDENCY),
        Edge("expanded_form_value", "simple_form_value", EdgeType.SUBLATION),
        Edge("general_form_value", "expanded_form_value", EdgeType.SUBLATION),
        Edge("general_form_value", "money_form", EdgeType.DEPENDENCY),
    ]
    return "1859: Contribution to Critique of Political Economy", vertices, edges


def w_theories_surplus_value():
    """1861-63: Theories of Surplus Value."""
    vertices = [
        Vertex("physiocrats", content="Physiocrats — surplus in agriculture"),
        Vertex("adam_smith", content="Adam Smith — labor theory of value"),
        Vertex("ricardo", content="Ricardo — value and profit"),
        Vertex("productive_labor", content="productive vs unproductive labor"),
        Vertex("rate_of_profit_tendency", content="tendency of rate of profit to fall"),
        Vertex("absolute_sv", content="absolute surplus value"),
        Vertex("relative_sv", content="relative surplus value"),
        Vertex("vulgar_economy", content="vulgar economy vs classical"),
    ]
    edges = [
        Edge("adam_smith", "physiocrats", EdgeType.SUBLATION),
        Edge("ricardo", "adam_smith", EdgeType.SUBLATION),
        Edge("surplus_value", "ricardo", EdgeType.SUBLATION),
        Edge("productive_labor", "labor", EdgeType.DEPENDENCY),
        Edge("productive_labor", "surplus_value", EdgeType.DEPENDENCY),
        Edge("rate_of_profit_tendency", "surplus_value", EdgeType.DEPENDENCY),
        Edge("rate_of_profit_tendency", "capital_general", EdgeType.DEPENDENCY),
        Edge("absolute_sv", "surplus_value", EdgeType.DEPENDENCY),
        Edge("relative_sv", "surplus_value", EdgeType.DEPENDENCY),
        Edge("relative_sv", "absolute_sv", EdgeType.SUBLATION),
        Edge("vulgar_economy", "ricardo", EdgeType.NEGATION),
        Edge("vulgar_economy", "ideology", EdgeType.DEPENDENCY),
    ]
    return "1861-63: Theories of Surplus Value", vertices, edges


def w_results_immediate():
    """1863: Results of the Immediate Process of Production."""
    vertices = [
        Vertex("commodity_capital", content="commodity as product of capital"),
        Vertex("total_social_capital", content="total social capital"),
        Vertex("capitalist_production_process", content="capitalist production process as whole"),
        Vertex("mystification", content="mystification of capital relation"),
    ]
    edges = [
        Edge("commodity_capital", "commodity", EdgeType.SUBLATION),
        Edge("commodity_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("total_social_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("capitalist_production_process", "real_subsumption", EdgeType.DEPENDENCY),
        Edge("capitalist_production_process", "surplus_value", EdgeType.DEPENDENCY),
        Edge("mystification", "capitalist_production_process", EdgeType.DEPENDENCY),
        Edge("mystification", "ideology", EdgeType.DEPENDENCY),
    ]
    return "1863: Results of Immediate Process of Production", vertices, edges


def w_inaugural_address():
    """1864: Inaugural Address of the International."""
    vertices = [
        Vertex("international_wma", content="International Workingmen's Association"),
        Vertex("ten_hours_bill", content="Ten Hours Bill — legislative victory"),
        Vertex("cooperative_movement", content="cooperative movement"),
        Vertex("political_power_proletariat", content="political power of working class"),
    ]
    edges = [
        Edge("international_wma", "proletariat", EdgeType.DEPENDENCY),
        Edge("international_wma", "communist_party", EdgeType.SUBLATION),
        Edge("ten_hours_bill", "class_struggle", EdgeType.DEPENDENCY),
        Edge("cooperative_movement", "social_character_production", EdgeType.DEPENDENCY),
        Edge("political_power_proletariat", "international_wma", EdgeType.DEPENDENCY),
        Edge("political_power_proletariat", "class_struggle", EdgeType.DEPENDENCY),
    ]
    return "1864: Inaugural Address of the International", vertices, edges


def w_value_price_profit():
    """1865: Value, Price and Profit."""
    vertices = [
        Vertex("labor_power", content="labor-power as commodity"),
        Vertex("value_labor_power", content="value of labor-power"),
        Vertex("price_labor", content="price of labor"),
        Vertex("exploitation", content="exploitation (rate of surplus value)"),
        Vertex("profit", content="profit"),
        Vertex("wages_struggles", content="trade union wage struggles"),
    ]
    edges = [
        Edge("labor_power", "labor_power_early", EdgeType.SUBLATION),
        Edge("labor_power", "commodity", EdgeType.DEPENDENCY),
        Edge("value_labor_power", "labor_power", EdgeType.DEPENDENCY),
        Edge("value_labor_power", "necessary_labor_time", EdgeType.DEPENDENCY),
        Edge("price_labor", "value_labor_power", EdgeType.DEPENDENCY),
        Edge("price_labor", "wage", EdgeType.SUBLATION),
        Edge("exploitation", "surplus_value", EdgeType.DEPENDENCY),
        Edge("exploitation", "labor_power", EdgeType.DEPENDENCY),
        Edge("profit", "surplus_value", EdgeType.DEPENDENCY),
        Edge("wages_struggles", "class_struggle", EdgeType.DEPENDENCY),
        Edge("wages_struggles", "exploitation", EdgeType.DEPENDENCY),
    ]
    return "1865: Value, Price and Profit", vertices, edges


def w_capital_vol1():
    """1867: Capital Volume I."""
    vertices = [
        Vertex("fetishism", content="commodity fetishism"),
        Vertex("money_capital_form", content="money form of capital (M-C-M')"),
        Vertex("accumulation", content="accumulation of capital"),
        Vertex("primitive_accumulation", content="so-called primitive accumulation"),
        Vertex("reserve_army", content="industrial reserve army"),
        Vertex("working_day", content="struggle over the working day"),
        Vertex("machinery", content="machinery and large-scale industry"),
        Vertex("cooperation", content="cooperation"),
        Vertex("manufacture", content="manufacture"),
        Vertex("concentration", content="concentration and centralization of capital"),
        Vertex("expropriation", content="expropriation of expropriators"),
        Vertex("historical_tendency", content="historical tendency of capitalist accumulation"),
        Vertex("reproduction_simple", content="simple reproduction"),
        Vertex("reproduction_expanded", content="expanded reproduction"),
        Vertex("general_law_accumulation", content="general law of capitalist accumulation"),
    ]
    edges = [
        Edge("fetishism", "commodity", EdgeType.DEPENDENCY),
        Edge("fetishism", "mystification", EdgeType.DEPENDENCY),
        Edge("fetishism", "social_relations", EdgeType.NEGATION),
        Edge("money_capital_form", "money_form", EdgeType.SUBLATION),
        Edge("money_capital_form", "surplus_value", EdgeType.DEPENDENCY),
        Edge("accumulation", "surplus_value", EdgeType.DEPENDENCY),
        Edge("accumulation", "capital_general", EdgeType.DEPENDENCY),
        Edge("primitive_accumulation", "accumulation", EdgeType.NEGATION),
        Edge("primitive_accumulation", "precapitalist_forms", EdgeType.DEPENDENCY),
        Edge("reserve_army", "accumulation", EdgeType.DEPENDENCY),
        Edge("reserve_army", "proletariat", EdgeType.DEPENDENCY),
        Edge("working_day", "absolute_sv", EdgeType.DEPENDENCY),
        Edge("working_day", "class_struggle", EdgeType.DEPENDENCY),
        Edge("machinery", "relative_sv", EdgeType.DEPENDENCY),
        Edge("machinery", "real_subsumption", EdgeType.DEPENDENCY),
        Edge("cooperation", "labor", EdgeType.DEPENDENCY),
        Edge("manufacture", "cooperation", EdgeType.SUBLATION),
        Edge("manufacture", "division_of_labor", EdgeType.DEPENDENCY),
        Edge("machinery", "manufacture", EdgeType.SUBLATION),
        Edge("concentration", "accumulation", EdgeType.DEPENDENCY),
        Edge("expropriation", "primitive_accumulation", EdgeType.NEGATION),
        Edge("expropriation", "concentration", EdgeType.DEPENDENCY),
        Edge("historical_tendency", "expropriation", EdgeType.DEPENDENCY),
        Edge("historical_tendency", "communism_gi", EdgeType.REFERENCE),
        Edge("reproduction_simple", "capital_general", EdgeType.DEPENDENCY),
        Edge("reproduction_expanded", "reproduction_simple", EdgeType.SUBLATION),
        Edge("reproduction_expanded", "accumulation", EdgeType.DEPENDENCY),
        Edge("general_law_accumulation", "reserve_army", EdgeType.DEPENDENCY),
        Edge("general_law_accumulation", "accumulation", EdgeType.DEPENDENCY),
    ]
    return "1867: Capital Volume I", vertices, edges


def w_civil_war_france():
    """1871: The Civil War in France."""
    vertices = [
        Vertex("paris_commune", content="Paris Commune"),
        Vertex("commune_state_form", content="Commune as political form"),
        Vertex("smash_state_machine", content="smashing the state machine"),
        Vertex("social_republic", content="social republic"),
        Vertex("working_class_government", content="working class government"),
    ]
    edges = [
        Edge("paris_commune", "proletariat", EdgeType.DEPENDENCY),
        Edge("paris_commune", "class_struggle", EdgeType.DEPENDENCY),
        Edge("commune_state_form", "paris_commune", EdgeType.DEPENDENCY),
        Edge("commune_state_form", "withering_state", EdgeType.DEPENDENCY),
        Edge("smash_state_machine", "state_machine", EdgeType.NEGATION),
        Edge("smash_state_machine", "paris_commune", EdgeType.DEPENDENCY),
        Edge("social_republic", "commune_state_form", EdgeType.DEPENDENCY),
        Edge("social_republic", "political_revolution", EdgeType.SUBLATION),
        Edge("working_class_government", "commune_state_form", EdgeType.DEPENDENCY),
        Edge("working_class_government", "political_power_proletariat", EdgeType.DEPENDENCY),
    ]
    return "1871: The Civil War in France", vertices, edges


def w_conspectus_bakunin():
    """1874: Conspectus of Bakunin's Statism and Anarchy."""
    vertices = [
        Vertex("bakunin", content="Bakunin — abolish state immediately"),
        Vertex("transitional_state", content="transitional proletarian state"),
        Vertex("dictatorship_proletariat", content="dictatorship of the proletariat"),
    ]
    edges = [
        Edge("transitional_state", "bakunin", EdgeType.NEGATION),
        Edge("dictatorship_proletariat", "transitional_state", EdgeType.DEPENDENCY),
        Edge("dictatorship_proletariat", "commune_state_form", EdgeType.DEPENDENCY),
        Edge("dictatorship_proletariat", "withering_state", EdgeType.DEPENDENCY),
        Edge("bakunin", "smash_state_machine", EdgeType.REFERENCE),
    ]
    return "1874: Conspectus of Bakunin", vertices, edges


def w_critique_gotha():
    """1875: Critique of the Gotha Programme."""
    vertices = [
        Vertex("lower_phase", content="lower phase of communism (socialism)"),
        Vertex("higher_phase", content="higher phase of communism"),
        Vertex("labor_vouchers", content="labor vouchers (not money)"),
        Vertex("bourgeois_right", content="bourgeois right in lower phase"),
        Vertex("from_each_to_each", content="from each according to ability, to each according to needs"),
        Vertex("lassalle_critique", content="critique of Lassalle"),
        Vertex("iron_law_wages", content="iron law of wages (rejected)"),
        Vertex("undiminished_proceeds", content="deductions from total social product"),
    ]
    edges = [
        Edge("lower_phase", "dictatorship_proletariat", EdgeType.DEPENDENCY),
        Edge("lower_phase", "communism_gi", EdgeType.DEPENDENCY),
        Edge("higher_phase", "lower_phase", EdgeType.SUBLATION),
        Edge("labor_vouchers", "money_form", EdgeType.NEGATION),
        Edge("labor_vouchers", "lower_phase", EdgeType.DEPENDENCY),
        Edge("bourgeois_right", "lower_phase", EdgeType.DEPENDENCY),
        Edge("bourgeois_right", "rights_of_man", EdgeType.REFERENCE),
        Edge("from_each_to_each", "higher_phase", EdgeType.DEPENDENCY),
        Edge("from_each_to_each", "free_development", EdgeType.DEPENDENCY),
        Edge("lassalle_critique", "iron_law_wages", EdgeType.NEGATION),
        Edge("iron_law_wages", "wage", EdgeType.DEPENDENCY),
        Edge("undiminished_proceeds", "total_social_capital", EdgeType.DEPENDENCY),
    ]
    return "1875: Critique of the Gotha Programme", vertices, edges


def w_notes_wagner():
    """1880: Notes on Wagner (Randglossen zu Wagner)."""
    vertices = [
        Vertex("wagner_critique", content="critique of Wagner's economics"),
        Vertex("value_not_general", content="value is not supra-historical"),
        Vertex("method_analysis", content="method: from concrete to abstract and back"),
    ]
    edges = [
        Edge("wagner_critique", "vulgar_economy", EdgeType.DEPENDENCY),
        Edge("value_not_general", "value", EdgeType.DEPENDENCY),
        Edge("value_not_general", "historically_specific", EdgeType.DEPENDENCY),
        Edge("method_analysis", "concrete_abstract", EdgeType.DEPENDENCY),
        Edge("method_analysis", "method_pe", EdgeType.DEPENDENCY),
    ]
    return "1880: Notes on Wagner", vertices, edges


def w_letter_zasulich():
    """1881: Letter to Vera Zasulich."""
    vertices = [
        Vertex("russian_commune", content="Russian rural commune (obshchina)"),
        Vertex("skip_capitalism", content="possibility of skipping capitalist stage"),
        Vertex("multilinear_history", content="multilinear historical development"),
    ]
    edges = [
        Edge("russian_commune", "village_community", EdgeType.REFERENCE),
        Edge("russian_commune", "communism_gi", EdgeType.REFERENCE),
        Edge("skip_capitalism", "russian_commune", EdgeType.DEPENDENCY),
        Edge("skip_capitalism", "primitive_accumulation", EdgeType.NEGATION),
        Edge("multilinear_history", "historical_materialism", EdgeType.DEPENDENCY),
        Edge("multilinear_history", "skip_capitalism", EdgeType.DEPENDENCY),
    ]
    return "1881: Letter to Vera Zasulich", vertices, edges


def w_capital_vol2():
    """1885: Capital Volume II (edited by Engels)."""
    vertices = [
        Vertex("circuit_capital", content="circuit of capital (M-C...P...C'-M')"),
        Vertex("turnover_capital", content="turnover of capital"),
        Vertex("reproduction_schemas", content="reproduction schemas (Dept I and II)"),
        Vertex("constant_capital", content="constant capital"),
        Vertex("dept_I", content="Department I — means of production"),
        Vertex("dept_II", content="Department II — means of consumption"),
        Vertex("realization_problem", content="problem of realization"),
    ]
    edges = [
        Edge("circuit_capital", "money_capital_form", EdgeType.SUBLATION),
        Edge("circuit_capital", "circulation", EdgeType.DEPENDENCY),
        Edge("turnover_capital", "circuit_capital", EdgeType.DEPENDENCY),
        Edge("turnover_capital", "fixed_capital", EdgeType.DEPENDENCY),
        Edge("reproduction_schemas", "reproduction_expanded", EdgeType.SUBLATION),
        Edge("reproduction_schemas", "total_social_capital", EdgeType.DEPENDENCY),
        Edge("constant_capital", "fixed_capital", EdgeType.DEPENDENCY),
        Edge("constant_capital", "variable_capital", EdgeType.NEGATION),
        Edge("dept_I", "reproduction_schemas", EdgeType.DEPENDENCY),
        Edge("dept_II", "reproduction_schemas", EdgeType.DEPENDENCY),
        Edge("dept_I", "dept_II", EdgeType.NEGATION),
        Edge("realization_problem", "reproduction_schemas", EdgeType.DEPENDENCY),
        Edge("realization_problem", "circulation", EdgeType.DEPENDENCY),
    ]
    return "1885: Capital Volume II", vertices, edges


def w_capital_vol3():
    """1894: Capital Volume III (edited by Engels)."""
    vertices = [
        Vertex("rate_of_profit", content="rate of profit"),
        Vertex("average_rate_profit", content="average rate of profit"),
        Vertex("prices_production", content="prices of production"),
        Vertex("transformation_problem", content="transformation of values into prices"),
        Vertex("tendency_fall_rop", content="tendency of rate of profit to fall (law)"),
        Vertex("counteracting_causes", content="counteracting causes"),
        Vertex("commercial_capital", content="commercial capital"),
        Vertex("interest_bearing_capital", content="interest-bearing capital"),
        Vertex("interest", content="interest"),
        Vertex("rent", content="ground rent"),
        Vertex("absolute_rent", content="absolute rent"),
        Vertex("differential_rent", content="differential rent"),
        Vertex("trinity_formula", content="trinity formula (capital-profit, land-rent, labor-wages)"),
        Vertex("revenue_sources", content="revenue and its sources"),
        Vertex("fictitious_capital", content="fictitious capital"),
    ]
    edges = [
        Edge("rate_of_profit", "surplus_value", EdgeType.DEPENDENCY),
        Edge("rate_of_profit", "constant_capital", EdgeType.DEPENDENCY),
        Edge("rate_of_profit", "variable_capital", EdgeType.DEPENDENCY),
        Edge("average_rate_profit", "rate_of_profit", EdgeType.DEPENDENCY),
        Edge("average_rate_profit", "concentration", EdgeType.DEPENDENCY),
        Edge("prices_production", "average_rate_profit", EdgeType.DEPENDENCY),
        Edge("prices_production", "value", EdgeType.NEGATION),
        Edge("transformation_problem", "prices_production", EdgeType.DEPENDENCY),
        Edge("transformation_problem", "value", EdgeType.DEPENDENCY),
        Edge("tendency_fall_rop", "rate_of_profit_tendency", EdgeType.SUBLATION),
        Edge("tendency_fall_rop", "rate_of_profit", EdgeType.DEPENDENCY),
        Edge("counteracting_causes", "tendency_fall_rop", EdgeType.NEGATION),
        Edge("commercial_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("commercial_capital", "circulation", EdgeType.DEPENDENCY),
        Edge("interest_bearing_capital", "capital_general", EdgeType.DEPENDENCY),
        Edge("interest_bearing_capital", "money_capital_form", EdgeType.DEPENDENCY),
        Edge("interest", "interest_bearing_capital", EdgeType.DEPENDENCY),
        Edge("interest", "profit", EdgeType.NEGATION),
        Edge("rent", "rent_early", EdgeType.SUBLATION),
        Edge("absolute_rent", "rent", EdgeType.DEPENDENCY),
        Edge("differential_rent", "rent", EdgeType.DEPENDENCY),
        Edge("differential_rent", "absolute_rent", EdgeType.NEGATION),
        Edge("trinity_formula", "fetishism", EdgeType.DEPENDENCY),
        Edge("trinity_formula", "mystification", EdgeType.DEPENDENCY),
        Edge("trinity_formula", "profit", EdgeType.DEPENDENCY),
        Edge("trinity_formula", "rent", EdgeType.DEPENDENCY),
        Edge("trinity_formula", "wage", EdgeType.DEPENDENCY),
        Edge("revenue_sources", "trinity_formula", EdgeType.DEPENDENCY),
        Edge("fictitious_capital", "interest_bearing_capital", EdgeType.DEPENDENCY),
        Edge("fictitious_capital", "money_capital_form", EdgeType.NEGATION),
    ]
    return "1894: Capital Volume III", vertices, edges


def w_selected_correspondence():
    """1843-83: Selected Correspondence (key letters)."""
    vertices = [
        Vertex("method_letters", content="method explained in letters"),
        Vertex("totality", content="totality of social relations"),
        Vertex("determination_last_instance", content="determination in the last instance"),
        Vertex("relative_autonomy_superstructure", content="relative autonomy of superstructure"),
    ]
    edges = [
        Edge("method_letters", "historical_materialism", EdgeType.REFERENCE),
        Edge("method_letters", "method_pe", EdgeType.REFERENCE),
        Edge("totality", "social_relations", EdgeType.SUBLATION),
        Edge("totality", "mode_of_production", EdgeType.DEPENDENCY),
        Edge("determination_last_instance", "base_superstructure", EdgeType.DEPENDENCY),
        Edge("determination_last_instance", "material_conditions", EdgeType.DEPENDENCY),
        Edge("relative_autonomy_superstructure", "base_superstructure", EdgeType.DEPENDENCY),
        Edge("relative_autonomy_superstructure", "determination_last_instance", EdgeType.NEGATION),
        Edge("relative_autonomy_superstructure", "ideology", EdgeType.DEPENDENCY),
    ]
    return "1843-83: Selected Correspondence", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    w_doctoral_dissertation,
    w_critique_hegel_right,
    w_jewish_question,
    w_paris_manuscripts,
    w_critical_notes_king_prussia,
    w_theses_feuerbach,
    w_holy_family,
    w_german_ideology,
    w_poverty_of_philosophy,
    w_wage_labour_and_capital,
    w_communist_manifesto,
    w_class_struggles_france,
    w_eighteenth_brumaire,
    w_articles_india,
    w_grundrisse_intro,
    w_grundrisse,
    w_contribution_critique_pe,
    w_theories_surplus_value,
    w_results_immediate,
    w_inaugural_address,
    w_value_price_profit,
    w_capital_vol1,
    w_civil_war_france,
    w_conspectus_bakunin,
    w_critique_gotha,
    w_notes_wagner,
    w_letter_zasulich,
    w_capital_vol2,
    w_capital_vol3,
    w_selected_correspondence,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("MARX COLLECTED WORKS — INCREMENTAL TRAVERSAL")
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

        # Count operations in this work's digestion
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
    crit_ratio = (ft_counts["critical"]
                  / (ft_counts["tree"] + ft_counts["critical"])) \
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

    # Sublation count
    sub_count = type_counts.get("sublation", 0)
    sub_density = sub_count / final_e if final_e > 0 else 0
    print(f"  Sublation density: {sub_density:.4f} ({sub_count}/{final_e})")

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "marx_collected_works_incremental",
        "source": "Marx Collected Works — 30 works (1841-1894)",
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
            "sublation_density": sub_density,
        },
        "total_steps": cumulative_steps,
        "elapsed_seconds": elapsed,
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_marx.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
