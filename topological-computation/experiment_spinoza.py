"""Spinoza Collected Works — incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Spinoza's thought
from early treatises through the mature Ethics and political writings,
as an incremental topological growth process.

Covers 12 works/sections spanning ~1660-1677.
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
#   substance, attribute, mode, God_Nature, extension, thought,
#   conatus, affect, joy, sadness, desire, adequate_idea,
#   imagination, reason, intuitive_knowledge, freedom, necessity,
#   eternity, mind, body, power, essence, existence,
#   infinite, finite, cause_of_itself, natura_naturans,
#   natura_naturata, perfection, virtue, bondage, blessedness,
#   common_notions, truth, falsity, duration, affects_active,
#   affects_passive, knowledge_first, knowledge_second, knowledge_third
# ---------------------------------------------------------------------------


def w_short_treatise():
    """~1660: Short Treatise on God, Man and His Well-Being (Korte Verhandeling)."""
    vertices = [
        Vertex("God_Nature", content="God or Nature — the sole substance"),
        Vertex("substance", content="substance — that which is in itself"),
        Vertex("attribute", content="attribute — what intellect perceives as essence of substance"),
        Vertex("thought", content="thought — attribute of substance"),
        Vertex("extension", content="extension — attribute of substance"),
        Vertex("mode", content="mode — affection of substance"),
        Vertex("infinite", content="infinite — that which is unlimited in its kind"),
        Vertex("finite", content="finite — limited by another of the same nature"),
        Vertex("knowledge_opinion", content="opinion (imaginatio) — first kind of knowledge"),
        Vertex("knowledge_true_belief", content="true belief — second kind of knowledge"),
        Vertex("knowledge_clear", content="clear knowledge (intuition) — third kind"),
        Vertex("love_of_God_early", content="intellectual love of God (early)"),
        Vertex("good_evil_early", content="good and evil as relative to us"),
        Vertex("will_intellect_unity", content="will and intellect are one"),
    ]
    edges = [
        Edge("God_Nature", "substance", EdgeType.DEPENDENCY),
        Edge("attribute", "substance", EdgeType.DEPENDENCY),
        Edge("thought", "attribute", EdgeType.DEPENDENCY),
        Edge("extension", "attribute", EdgeType.DEPENDENCY),
        Edge("thought", "extension", EdgeType.NEGATION),
        Edge("mode", "substance", EdgeType.DEPENDENCY),
        Edge("mode", "attribute", EdgeType.DEPENDENCY),
        Edge("infinite", "substance", EdgeType.DEPENDENCY),
        Edge("finite", "infinite", EdgeType.NEGATION),
        Edge("finite", "mode", EdgeType.DEPENDENCY),
        Edge("knowledge_true_belief", "knowledge_opinion", EdgeType.SUBLATION),
        Edge("knowledge_clear", "knowledge_true_belief", EdgeType.SUBLATION),
        Edge("love_of_God_early", "knowledge_clear", EdgeType.DEPENDENCY),
        Edge("love_of_God_early", "God_Nature", EdgeType.DEPENDENCY),
        Edge("good_evil_early", "mode", EdgeType.DEPENDENCY),
        Edge("will_intellect_unity", "thought", EdgeType.DEPENDENCY),
    ]
    return "~1660: Short Treatise on God, Man and His Well-Being", vertices, edges


def w_emendation_intellect():
    """~1661: Treatise on the Emendation of the Intellect (TdIE)."""
    vertices = [
        Vertex("true_good", content="true good — continuous supreme joy"),
        Vertex("ordinary_goods", content="ordinary goods (wealth, honor, pleasure)"),
        Vertex("adequate_idea", content="adequate idea — clear and distinct in itself"),
        Vertex("truth", content="truth — idea agreeing with its ideatum"),
        Vertex("falsity", content="falsity — mutilated and confused ideas"),
        Vertex("method", content="method — reflexive knowledge of the idea of the true"),
        Vertex("imagination", content="imagination — confused, first kind of knowledge"),
        Vertex("intellect", content="intellect — true ideas, foundation of method"),
        Vertex("definition", content="definition — expresses intimate essence of thing"),
        Vertex("fictitious_idea", content="fictitious idea — joining unconnected ideas"),
        Vertex("false_idea", content="false idea — imagination without correction"),
        Vertex("doubt", content="doubt — suspension due to inadequate ideas"),
    ]
    edges = [
        Edge("true_good", "ordinary_goods", EdgeType.NEGATION),
        Edge("true_good", "love_of_God_early", EdgeType.DEPENDENCY),
        Edge("adequate_idea", "truth", EdgeType.DEPENDENCY),
        Edge("adequate_idea", "falsity", EdgeType.NEGATION),
        Edge("method", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("method", "intellect", EdgeType.DEPENDENCY),
        Edge("imagination", "intellect", EdgeType.NEGATION),
        Edge("imagination", "knowledge_opinion", EdgeType.REFERENCE),
        Edge("intellect", "thought", EdgeType.DEPENDENCY),
        Edge("definition", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("definition", "method", EdgeType.DEPENDENCY),
        Edge("fictitious_idea", "imagination", EdgeType.DEPENDENCY),
        Edge("false_idea", "imagination", EdgeType.DEPENDENCY),
        Edge("false_idea", "adequate_idea", EdgeType.NEGATION),
        Edge("doubt", "falsity", EdgeType.DEPENDENCY),
        Edge("doubt", "adequate_idea", EdgeType.NEGATION),
        Edge("truth", "falsity", EdgeType.NEGATION),
    ]
    return "~1661: Treatise on the Emendation of the Intellect", vertices, edges


def w_principles_cartesian():
    """1663: Principles of Cartesian Philosophy + Metaphysical Thoughts."""
    vertices = [
        Vertex("descartes_substance", content="Descartes' substance (res cogitans/extensa)"),
        Vertex("causa_sui", content="causa sui — cause of itself"),
        Vertex("existence", content="existence — not distinct from essence in God"),
        Vertex("essence", content="essence — what constitutes a thing"),
        Vertex("duration", content="duration — indefinite continuation of existing"),
        Vertex("eternity", content="eternity — existence conceived under infinite attribute"),
        Vertex("creation_continuous", content="continuous creation (conservatio)"),
        Vertex("perfection", content="perfection — degree of reality/being"),
        Vertex("evil_privation", content="evil as privation, not positive being"),
        Vertex("will_intellect_distinction", content="Cartesian distinction of will and intellect (Spinoza rejects)"),
    ]
    edges = [
        Edge("descartes_substance", "substance", EdgeType.NEGATION),
        Edge("causa_sui", "God_Nature", EdgeType.DEPENDENCY),
        Edge("causa_sui", "substance", EdgeType.DEPENDENCY),
        Edge("existence", "essence", EdgeType.DEPENDENCY),
        Edge("existence", "causa_sui", EdgeType.DEPENDENCY),
        Edge("essence", "substance", EdgeType.DEPENDENCY),
        Edge("duration", "eternity", EdgeType.NEGATION),
        Edge("duration", "finite", EdgeType.DEPENDENCY),
        Edge("eternity", "substance", EdgeType.DEPENDENCY),
        Edge("eternity", "infinite", EdgeType.DEPENDENCY),
        Edge("creation_continuous", "causa_sui", EdgeType.DEPENDENCY),
        Edge("perfection", "existence", EdgeType.DEPENDENCY),
        Edge("perfection", "essence", EdgeType.DEPENDENCY),
        Edge("evil_privation", "good_evil_early", EdgeType.REFERENCE),
        Edge("evil_privation", "perfection", EdgeType.NEGATION),
        Edge("will_intellect_distinction", "will_intellect_unity", EdgeType.NEGATION),
        Edge("will_intellect_distinction", "descartes_substance", EdgeType.DEPENDENCY),
    ]
    return "1663: Principles of Cartesian Philosophy", vertices, edges


def w_ethics_part1():
    """Ethics Part I: Concerning God (De Deo)."""
    vertices = [
        Vertex("substance_unique", content="there can be only one substance"),
        Vertex("attribute_infinite", content="substance has infinite attributes"),
        Vertex("God_proof", content="God necessarily exists (ontological proof)"),
        Vertex("natura_naturans", content="natura naturans — God as free cause"),
        Vertex("natura_naturata", content="natura naturata — modes following from God"),
        Vertex("necessity", content="all things follow from God's nature necessarily"),
        Vertex("freedom_as_necessity", content="freedom = acting from necessity of own nature"),
        Vertex("contingency_denied", content="nothing is contingent — only from ignorance"),
        Vertex("will_not_free_cause", content="will cannot be called a free cause"),
        Vertex("determinism", content="every thing determined by another to exist and act"),
        Vertex("infinite_modes", content="infinite modes — immediate and mediate"),
        Vertex("finite_mode_chain", content="finite modes determined by other finite modes"),
    ]
    edges = [
        Edge("substance_unique", "substance", EdgeType.SUBLATION),
        Edge("substance_unique", "descartes_substance", EdgeType.NEGATION),
        Edge("attribute_infinite", "attribute", EdgeType.SUBLATION),
        Edge("attribute_infinite", "substance_unique", EdgeType.DEPENDENCY),
        Edge("God_proof", "causa_sui", EdgeType.DEPENDENCY),
        Edge("God_proof", "substance_unique", EdgeType.DEPENDENCY),
        Edge("God_proof", "existence", EdgeType.DEPENDENCY),
        Edge("natura_naturans", "God_Nature", EdgeType.SUBLATION),
        Edge("natura_naturans", "substance_unique", EdgeType.DEPENDENCY),
        Edge("natura_naturata", "natura_naturans", EdgeType.DEPENDENCY),
        Edge("natura_naturata", "mode", EdgeType.SUBLATION),
        Edge("necessity", "God_proof", EdgeType.DEPENDENCY),
        Edge("necessity", "contingency_denied", EdgeType.DEPENDENCY),
        Edge("freedom_as_necessity", "necessity", EdgeType.DEPENDENCY),
        Edge("freedom_as_necessity", "causa_sui", EdgeType.DEPENDENCY),
        Edge("contingency_denied", "finite", EdgeType.DEPENDENCY),
        Edge("contingency_denied", "imagination", EdgeType.REFERENCE),
        Edge("will_not_free_cause", "will_intellect_unity", EdgeType.SUBLATION),
        Edge("will_not_free_cause", "will_intellect_distinction", EdgeType.NEGATION),
        Edge("will_not_free_cause", "determinism", EdgeType.DEPENDENCY),
        Edge("determinism", "necessity", EdgeType.DEPENDENCY),
        Edge("determinism", "natura_naturata", EdgeType.DEPENDENCY),
        Edge("infinite_modes", "natura_naturans", EdgeType.DEPENDENCY),
        Edge("infinite_modes", "attribute_infinite", EdgeType.DEPENDENCY),
        Edge("finite_mode_chain", "determinism", EdgeType.DEPENDENCY),
        Edge("finite_mode_chain", "finite", EdgeType.SUBLATION),
    ]
    return "Ethics I: Concerning God", vertices, edges


def w_ethics_part2():
    """Ethics Part II: On the Nature and Origin of the Mind (De Mente)."""
    vertices = [
        Vertex("mind", content="mind — idea of the body"),
        Vertex("body", content="body — mode of extension"),
        Vertex("parallelism", content="order of ideas = order of things"),
        Vertex("mind_body_unity", content="mind and body are one and the same thing"),
        Vertex("common_notions", content="common notions — basis of reason"),
        Vertex("knowledge_first", content="first kind: imagination/opinion"),
        Vertex("knowledge_second", content="second kind: reason (from common notions)"),
        Vertex("knowledge_third", content="third kind: intuitive knowledge (scientia intuitiva)"),
        Vertex("idea_of_idea", content="idea of idea — reflexive self-knowledge"),
        Vertex("sensation", content="sensation — ideas of body's affections"),
        Vertex("memory", content="memory — association of bodily images"),
        Vertex("universals_abstractions", content="universals as confused abstractions"),
        Vertex("adequate_knowledge_body", content="adequate knowledge of the body is limited"),
        Vertex("error_source", content="error arises from inadequate ideas"),
    ]
    edges = [
        Edge("mind", "thought", EdgeType.DEPENDENCY),
        Edge("mind", "mode", EdgeType.DEPENDENCY),
        Edge("body", "extension", EdgeType.DEPENDENCY),
        Edge("body", "mode", EdgeType.DEPENDENCY),
        Edge("parallelism", "thought", EdgeType.DEPENDENCY),
        Edge("parallelism", "extension", EdgeType.DEPENDENCY),
        Edge("parallelism", "attribute_infinite", EdgeType.DEPENDENCY),
        Edge("mind_body_unity", "parallelism", EdgeType.DEPENDENCY),
        Edge("mind_body_unity", "mind", EdgeType.DEPENDENCY),
        Edge("mind_body_unity", "body", EdgeType.DEPENDENCY),
        Edge("mind_body_unity", "descartes_substance", EdgeType.NEGATION),
        Edge("common_notions", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("common_notions", "body", EdgeType.DEPENDENCY),
        Edge("knowledge_first", "imagination", EdgeType.SUBLATION),
        Edge("knowledge_first", "sensation", EdgeType.DEPENDENCY),
        Edge("knowledge_second", "common_notions", EdgeType.DEPENDENCY),
        Edge("knowledge_second", "knowledge_first", EdgeType.SUBLATION),
        Edge("knowledge_second", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("knowledge_third", "knowledge_second", EdgeType.SUBLATION),
        Edge("knowledge_third", "knowledge_clear", EdgeType.SUBLATION),
        Edge("knowledge_third", "essence", EdgeType.DEPENDENCY),
        Edge("idea_of_idea", "mind", EdgeType.DEPENDENCY),
        Edge("idea_of_idea", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("sensation", "body", EdgeType.DEPENDENCY),
        Edge("sensation", "mind", EdgeType.DEPENDENCY),
        Edge("memory", "sensation", EdgeType.DEPENDENCY),
        Edge("memory", "imagination", EdgeType.DEPENDENCY),
        Edge("universals_abstractions", "imagination", EdgeType.DEPENDENCY),
        Edge("universals_abstractions", "knowledge_first", EdgeType.DEPENDENCY),
        Edge("adequate_knowledge_body", "common_notions", EdgeType.DEPENDENCY),
        Edge("adequate_knowledge_body", "mind_body_unity", EdgeType.DEPENDENCY),
        Edge("error_source", "falsity", EdgeType.SUBLATION),
        Edge("error_source", "knowledge_first", EdgeType.DEPENDENCY),
        Edge("error_source", "adequate_idea", EdgeType.NEGATION),
    ]
    return "Ethics II: On the Nature and Origin of the Mind", vertices, edges


def w_ethics_part3():
    """Ethics Part III: On the Origin and Nature of the Affects (De Affectibus)."""
    vertices = [
        Vertex("conatus", content="conatus — striving to persevere in being"),
        Vertex("desire", content="desire (cupiditas) — conatus with consciousness"),
        Vertex("joy", content="joy (laetitia) — passage to greater perfection"),
        Vertex("sadness", content="sadness (tristitia) — passage to lesser perfection"),
        Vertex("affects_active", content="active affects — from adequate ideas"),
        Vertex("affects_passive", content="passive affects (passiones) — from inadequate ideas"),
        Vertex("love", content="love — joy with idea of external cause"),
        Vertex("hate", content="hate — sadness with idea of external cause"),
        Vertex("hope", content="hope — inconstant joy from uncertain future"),
        Vertex("fear", content="fear — inconstant sadness from uncertain future"),
        Vertex("imitation_affects", content="imitation of affects — affective mimesis"),
        Vertex("fluctuatio_animi", content="fluctuatio animi — vacillation between contrary affects"),
        Vertex("power", content="power (potentia) — capacity to act and be affected"),
    ]
    edges = [
        Edge("conatus", "essence", EdgeType.DEPENDENCY),
        Edge("conatus", "natura_naturata", EdgeType.DEPENDENCY),
        Edge("conatus", "determinism", EdgeType.DEPENDENCY),
        Edge("desire", "conatus", EdgeType.DEPENDENCY),
        Edge("desire", "mind", EdgeType.DEPENDENCY),
        Edge("joy", "conatus", EdgeType.DEPENDENCY),
        Edge("joy", "perfection", EdgeType.DEPENDENCY),
        Edge("sadness", "conatus", EdgeType.DEPENDENCY),
        Edge("sadness", "joy", EdgeType.NEGATION),
        Edge("affects_active", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("affects_active", "conatus", EdgeType.DEPENDENCY),
        Edge("affects_passive", "imagination", EdgeType.DEPENDENCY),
        Edge("affects_passive", "affects_active", EdgeType.NEGATION),
        Edge("love", "joy", EdgeType.DEPENDENCY),
        Edge("hate", "sadness", EdgeType.DEPENDENCY),
        Edge("hate", "love", EdgeType.NEGATION),
        Edge("hope", "joy", EdgeType.DEPENDENCY),
        Edge("hope", "imagination", EdgeType.DEPENDENCY),
        Edge("fear", "sadness", EdgeType.DEPENDENCY),
        Edge("fear", "hope", EdgeType.NEGATION),
        Edge("imitation_affects", "affects_passive", EdgeType.DEPENDENCY),
        Edge("imitation_affects", "imagination", EdgeType.DEPENDENCY),
        Edge("fluctuatio_animi", "affects_passive", EdgeType.DEPENDENCY),
        Edge("fluctuatio_animi", "hope", EdgeType.DEPENDENCY),
        Edge("fluctuatio_animi", "fear", EdgeType.DEPENDENCY),
        Edge("power", "conatus", EdgeType.DEPENDENCY),
        Edge("power", "affects_active", EdgeType.DEPENDENCY),
    ]
    return "Ethics III: On the Origin and Nature of the Affects", vertices, edges


def w_ethics_part4():
    """Ethics Part IV: Of Human Bondage, or the Power of the Affects."""
    vertices = [
        Vertex("bondage", content="human bondage — subjection to passive affects"),
        Vertex("virtue", content="virtue — power of acting from adequate ideas"),
        Vertex("self_preservation", content="self-preservation = foundation of virtue"),
        Vertex("reason_dictates", content="dictates of reason — what reason prescribes"),
        Vertex("good_evil", content="good and evil as relative to human model"),
        Vertex("strength_of_mind", content="fortitudo — strength of mind"),
        Vertex("generosity", content="generositas — rational desire to aid others"),
        Vertex("free_man", content="the free man — one guided by reason"),
        Vertex("society_rational", content="rational society — agreement by nature"),
        Vertex("superstition_bondage", content="superstition as form of bondage"),
        Vertex("temporal_affect", content="present affect stronger than future — temporal gradient"),
    ]
    edges = [
        Edge("bondage", "affects_passive", EdgeType.DEPENDENCY),
        Edge("bondage", "imagination", EdgeType.DEPENDENCY),
        Edge("bondage", "freedom_as_necessity", EdgeType.NEGATION),
        Edge("virtue", "power", EdgeType.DEPENDENCY),
        Edge("virtue", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("virtue", "conatus", EdgeType.DEPENDENCY),
        Edge("self_preservation", "conatus", EdgeType.SUBLATION),
        Edge("self_preservation", "virtue", EdgeType.DEPENDENCY),
        Edge("reason_dictates", "knowledge_second", EdgeType.DEPENDENCY),
        Edge("reason_dictates", "common_notions", EdgeType.DEPENDENCY),
        Edge("reason_dictates", "virtue", EdgeType.DEPENDENCY),
        Edge("good_evil", "good_evil_early", EdgeType.SUBLATION),
        Edge("good_evil", "perfection", EdgeType.DEPENDENCY),
        Edge("good_evil", "joy", EdgeType.DEPENDENCY),
        Edge("good_evil", "sadness", EdgeType.DEPENDENCY),
        Edge("strength_of_mind", "virtue", EdgeType.DEPENDENCY),
        Edge("strength_of_mind", "affects_active", EdgeType.DEPENDENCY),
        Edge("generosity", "strength_of_mind", EdgeType.DEPENDENCY),
        Edge("generosity", "reason_dictates", EdgeType.DEPENDENCY),
        Edge("free_man", "reason_dictates", EdgeType.DEPENDENCY),
        Edge("free_man", "bondage", EdgeType.NEGATION),
        Edge("free_man", "virtue", EdgeType.DEPENDENCY),
        Edge("society_rational", "free_man", EdgeType.DEPENDENCY),
        Edge("society_rational", "generosity", EdgeType.DEPENDENCY),
        Edge("superstition_bondage", "bondage", EdgeType.DEPENDENCY),
        Edge("superstition_bondage", "imagination", EdgeType.DEPENDENCY),
        Edge("superstition_bondage", "fear", EdgeType.DEPENDENCY),
        Edge("temporal_affect", "affects_passive", EdgeType.DEPENDENCY),
        Edge("temporal_affect", "imagination", EdgeType.DEPENDENCY),
        Edge("temporal_affect", "duration", EdgeType.DEPENDENCY),
    ]
    return "Ethics IV: Of Human Bondage", vertices, edges


def w_ethics_part5():
    """Ethics Part V: Of the Power of the Intellect, or On Human Freedom."""
    vertices = [
        Vertex("remedy_affects", content="remedies against the affects"),
        Vertex("affect_as_idea", content="affect ceases to be passion when we form clear idea"),
        Vertex("love_of_God", content="amor Dei intellectualis — intellectual love of God"),
        Vertex("blessedness", content="blessedness (beatitudo) — not reward of virtue, but virtue itself"),
        Vertex("eternity_of_mind", content="something of the mind is eternal"),
        Vertex("intuitive_knowledge", content="scientia intuitiva — knowledge of things sub specie aeternitatis"),
        Vertex("sub_specie_aeternitatis", content="under the aspect of eternity"),
        Vertex("freedom", content="freedom — self-determination through adequate ideas"),
        Vertex("acquiescentia_in_se", content="acquiescentia in se ipso — self-contentment from reason"),
        Vertex("joy_from_knowledge", content="highest joy from third kind of knowledge"),
    ]
    edges = [
        Edge("remedy_affects", "affects_passive", EdgeType.NEGATION),
        Edge("remedy_affects", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("remedy_affects", "knowledge_second", EdgeType.DEPENDENCY),
        Edge("affect_as_idea", "remedy_affects", EdgeType.DEPENDENCY),
        Edge("affect_as_idea", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("affect_as_idea", "affects_passive", EdgeType.NEGATION),
        Edge("love_of_God", "love_of_God_early", EdgeType.SUBLATION),
        Edge("love_of_God", "God_Nature", EdgeType.DEPENDENCY),
        Edge("love_of_God", "intuitive_knowledge", EdgeType.DEPENDENCY),
        Edge("love_of_God", "joy", EdgeType.DEPENDENCY),
        Edge("blessedness", "love_of_God", EdgeType.DEPENDENCY),
        Edge("blessedness", "virtue", EdgeType.SUBLATION),
        Edge("blessedness", "freedom", EdgeType.DEPENDENCY),
        Edge("eternity_of_mind", "mind", EdgeType.DEPENDENCY),
        Edge("eternity_of_mind", "eternity", EdgeType.DEPENDENCY),
        Edge("eternity_of_mind", "knowledge_third", EdgeType.DEPENDENCY),
        Edge("intuitive_knowledge", "knowledge_third", EdgeType.SUBLATION),
        Edge("intuitive_knowledge", "God_Nature", EdgeType.DEPENDENCY),
        Edge("intuitive_knowledge", "essence", EdgeType.DEPENDENCY),
        Edge("sub_specie_aeternitatis", "eternity", EdgeType.DEPENDENCY),
        Edge("sub_specie_aeternitatis", "intuitive_knowledge", EdgeType.DEPENDENCY),
        Edge("sub_specie_aeternitatis", "duration", EdgeType.NEGATION),
        Edge("freedom", "freedom_as_necessity", EdgeType.SUBLATION),
        Edge("freedom", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("freedom", "bondage", EdgeType.NEGATION),
        Edge("acquiescentia_in_se", "virtue", EdgeType.DEPENDENCY),
        Edge("acquiescentia_in_se", "joy", EdgeType.DEPENDENCY),
        Edge("acquiescentia_in_se", "knowledge_second", EdgeType.DEPENDENCY),
        Edge("joy_from_knowledge", "knowledge_third", EdgeType.DEPENDENCY),
        Edge("joy_from_knowledge", "love_of_God", EdgeType.DEPENDENCY),
        Edge("joy_from_knowledge", "joy", EdgeType.SUBLATION),
    ]
    return "Ethics V: Of the Power of the Intellect", vertices, edges


def w_theological_political():
    """1670: Theological-Political Treatise (TTP)."""
    vertices = [
        Vertex("scripture_interpretation", content="scripture must be interpreted from scripture alone"),
        Vertex("prophecy", content="prophecy — vivid imagination, not intellectual knowledge"),
        Vertex("miracles_denied", content="miracles = ignorance of natural causes"),
        Vertex("freedom_of_thought", content="freedom of thought and expression"),
        Vertex("state_purpose", content="purpose of state = freedom, not obedience"),
        Vertex("democracy_best", content="democracy as most natural form of government"),
        Vertex("theology_philosophy_separation", content="theology and philosophy have separate domains"),
        Vertex("obedience_piety", content="theology teaches obedience and piety, not truth"),
        Vertex("natural_right_ttp", content="natural right = power"),
        Vertex("social_contract_ttp", content="social contract — transfer of right to sovereign"),
        Vertex("superstition_origin", content="superstition arises from fear and ignorance"),
        Vertex("hebrew_state", content="the Hebrew theocracy — historical model"),
    ]
    edges = [
        Edge("scripture_interpretation", "method", EdgeType.REFERENCE),
        Edge("scripture_interpretation", "adequate_idea", EdgeType.REFERENCE),
        Edge("prophecy", "imagination", EdgeType.DEPENDENCY),
        Edge("prophecy", "knowledge_first", EdgeType.DEPENDENCY),
        Edge("prophecy", "intuitive_knowledge", EdgeType.NEGATION),
        Edge("miracles_denied", "necessity", EdgeType.DEPENDENCY),
        Edge("miracles_denied", "God_Nature", EdgeType.DEPENDENCY),
        Edge("miracles_denied", "contingency_denied", EdgeType.DEPENDENCY),
        Edge("freedom_of_thought", "reason_dictates", EdgeType.DEPENDENCY),
        Edge("freedom_of_thought", "free_man", EdgeType.DEPENDENCY),
        Edge("state_purpose", "freedom_of_thought", EdgeType.DEPENDENCY),
        Edge("state_purpose", "freedom", EdgeType.DEPENDENCY),
        Edge("democracy_best", "state_purpose", EdgeType.DEPENDENCY),
        Edge("democracy_best", "society_rational", EdgeType.DEPENDENCY),
        Edge("theology_philosophy_separation", "imagination", EdgeType.DEPENDENCY),
        Edge("theology_philosophy_separation", "knowledge_second", EdgeType.NEGATION),
        Edge("obedience_piety", "theology_philosophy_separation", EdgeType.DEPENDENCY),
        Edge("obedience_piety", "truth", EdgeType.NEGATION),
        Edge("natural_right_ttp", "power", EdgeType.DEPENDENCY),
        Edge("natural_right_ttp", "conatus", EdgeType.DEPENDENCY),
        Edge("social_contract_ttp", "natural_right_ttp", EdgeType.DEPENDENCY),
        Edge("social_contract_ttp", "society_rational", EdgeType.DEPENDENCY),
        Edge("superstition_origin", "superstition_bondage", EdgeType.SUBLATION),
        Edge("superstition_origin", "fear", EdgeType.DEPENDENCY),
        Edge("superstition_origin", "imagination", EdgeType.DEPENDENCY),
        Edge("hebrew_state", "social_contract_ttp", EdgeType.REFERENCE),
        Edge("hebrew_state", "democracy_best", EdgeType.REFERENCE),
    ]
    return "1670: Theological-Political Treatise", vertices, edges


def w_political_treatise():
    """1677: Political Treatise (Tractatus Politicus, unfinished)."""
    vertices = [
        Vertex("political_science", content="political science — from affects as they are, not as they should be"),
        Vertex("natural_right_power", content="natural right = actual power (jus sive potentia)"),
        Vertex("sovereignty", content="sovereignty — power of the multitude"),
        Vertex("multitude", content="the multitude (multitudo) — collective political subject"),
        Vertex("monarchy", content="monarchy — one holds sovereign power"),
        Vertex("aristocracy", content="aristocracy — council holds sovereign power"),
        Vertex("democracy_tp", content="democracy — all citizens hold sovereign power"),
        Vertex("indignation", content="indignatio — political affect of revolt"),
        Vertex("common_affect", content="communis affectus — shared political affect"),
        Vertex("right_of_state", content="right of the state = its power"),
    ]
    edges = [
        Edge("political_science", "affects_passive", EdgeType.DEPENDENCY),
        Edge("political_science", "conatus", EdgeType.DEPENDENCY),
        Edge("political_science", "reason_dictates", EdgeType.NEGATION),
        Edge("natural_right_power", "natural_right_ttp", EdgeType.SUBLATION),
        Edge("natural_right_power", "power", EdgeType.DEPENDENCY),
        Edge("natural_right_power", "conatus", EdgeType.DEPENDENCY),
        Edge("sovereignty", "multitude", EdgeType.DEPENDENCY),
        Edge("sovereignty", "natural_right_power", EdgeType.DEPENDENCY),
        Edge("multitude", "society_rational", EdgeType.SUBLATION),
        Edge("multitude", "social_contract_ttp", EdgeType.SUBLATION),
        Edge("monarchy", "sovereignty", EdgeType.DEPENDENCY),
        Edge("aristocracy", "sovereignty", EdgeType.DEPENDENCY),
        Edge("aristocracy", "monarchy", EdgeType.SUBLATION),
        Edge("democracy_tp", "sovereignty", EdgeType.DEPENDENCY),
        Edge("democracy_tp", "democracy_best", EdgeType.SUBLATION),
        Edge("democracy_tp", "aristocracy", EdgeType.SUBLATION),
        Edge("indignation", "sadness", EdgeType.DEPENDENCY),
        Edge("indignation", "affects_passive", EdgeType.DEPENDENCY),
        Edge("indignation", "imitation_affects", EdgeType.DEPENDENCY),
        Edge("common_affect", "imitation_affects", EdgeType.SUBLATION),
        Edge("common_affect", "multitude", EdgeType.DEPENDENCY),
        Edge("right_of_state", "sovereignty", EdgeType.DEPENDENCY),
        Edge("right_of_state", "natural_right_power", EdgeType.DEPENDENCY),
    ]
    return "1677: Political Treatise", vertices, edges


def w_selected_letters():
    """1661-76: Selected Correspondence (key letters)."""
    vertices = [
        Vertex("infinite_letter12", content="Letter 12 on the infinite — three kinds of infinity"),
        Vertex("substance_monism_defense", content="defense of substance monism against objections"),
        Vertex("evil_as_privation_letter", content="evil is privation — letter to Blyenbergh"),
        Vertex("determinism_defense", content="defense of determinism — we are determined but free through understanding"),
        Vertex("worm_in_blood", content="worm in blood analogy — part/whole knowledge"),
        Vertex("chinese_political", content="political remarks in letters"),
    ]
    edges = [
        Edge("infinite_letter12", "infinite", EdgeType.SUBLATION),
        Edge("infinite_letter12", "duration", EdgeType.DEPENDENCY),
        Edge("infinite_letter12", "eternity", EdgeType.DEPENDENCY),
        Edge("substance_monism_defense", "substance_unique", EdgeType.REFERENCE),
        Edge("substance_monism_defense", "God_proof", EdgeType.REFERENCE),
        Edge("evil_as_privation_letter", "evil_privation", EdgeType.SUBLATION),
        Edge("evil_as_privation_letter", "good_evil", EdgeType.DEPENDENCY),
        Edge("evil_as_privation_letter", "perfection", EdgeType.DEPENDENCY),
        Edge("determinism_defense", "determinism", EdgeType.REFERENCE),
        Edge("determinism_defense", "freedom", EdgeType.DEPENDENCY),
        Edge("determinism_defense", "adequate_idea", EdgeType.DEPENDENCY),
        Edge("worm_in_blood", "adequate_knowledge_body", EdgeType.REFERENCE),
        Edge("worm_in_blood", "knowledge_first", EdgeType.DEPENDENCY),
        Edge("worm_in_blood", "parallelism", EdgeType.REFERENCE),
        Edge("chinese_political", "democracy_tp", EdgeType.REFERENCE),
        Edge("chinese_political", "sovereignty", EdgeType.REFERENCE),
    ]
    return "1661-76: Selected Correspondence", vertices, edges


# ---------------------------------------------------------------------------
# All works in chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    w_short_treatise,
    w_emendation_intellect,
    w_principles_cartesian,
    w_ethics_part1,
    w_ethics_part2,
    w_ethics_part3,
    w_ethics_part4,
    w_ethics_part5,
    w_theological_political,
    w_political_treatise,
    w_selected_letters,
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("SPINOZA COLLECTED WORKS — INCREMENTAL TRAVERSAL")
    print(f"{len(ALL_WORKS)} works/sections, incremental injection, "
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
        "experiment": "spinoza_collected_works_incremental",
        "source": "Spinoza Collected Works — 11 works/sections (~1660-1677)",
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

    output_path = "experiment_spinoza.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
