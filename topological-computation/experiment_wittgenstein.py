"""Wittgenstein Complete Works — incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Wittgenstein's thought
from early logical atomism through the later philosophy of language games,
rule-following, and certainty — as an incremental topological growth process.

Covers ~22 works spanning 1913-1951 (including posthumous publications).
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
#   proposition, picture, logical_form, world, fact, state_of_affairs,
#   object, name, sense, nonsense, showing, saying, tautology,
#   language, meaning, use, language_game, form_of_life,
#   rule_following, private_language, family_resemblance,
#   grammar, certainty, doubt, knowledge, belief, hinge_proposition,
#   beetle_in_box, pain, sensation, inner_outer,
#   aspect_seeing, duck_rabbit, colour, mathematics,
#   ethics, aesthetics, religion, culture, value,
#   philosophy, therapy, clarity, logical_space
# ---------------------------------------------------------------------------


def w_notes_on_logic():
    """1913: Notes on Logic."""
    vertices = [
        Vertex("proposition", content="proposition — what can be true or false"),
        Vertex("logical_form", content="logical form — the structure shared by proposition and fact"),
        Vertex("fact", content="fact — what is the case"),
        Vertex("sense", content="sense — the way a proposition determines reality"),
        Vertex("nonsense", content="nonsense — pseudo-propositions without sense"),
        Vertex("showing", content="showing — what cannot be said but shows itself"),
        Vertex("saying", content="saying — what propositions express"),
        Vertex("logical_constant", content="logical constants do not represent"),
        Vertex("ab_notation", content="ab-notation — bipolarity of propositions"),
        Vertex("molecular_prop", content="molecular propositions — truth-functions"),
    ]
    edges = [
        Edge("proposition", "logical_form", EdgeType.DEPENDENCY),
        Edge("proposition", "sense", EdgeType.DEPENDENCY),
        Edge("proposition", "fact", EdgeType.REFERENCE),
        Edge("showing", "saying", EdgeType.NEGATION),
        Edge("nonsense", "sense", EdgeType.NEGATION),
        Edge("logical_constant", "proposition", EdgeType.REFERENCE),
        Edge("ab_notation", "proposition", EdgeType.DEPENDENCY),
        Edge("molecular_prop", "proposition", EdgeType.FOLD),
        Edge("logical_form", "showing", EdgeType.DEPENDENCY),
        Edge("saying", "sense", EdgeType.DEPENDENCY),
    ]
    return "Notes on Logic (1913)", vertices, edges


def w_notes_dictated_to_moore():
    """1914: Notes Dictated to G. E. Moore in Norway."""
    vertices = [
        Vertex("tautology", content="tautology — says nothing, shows logical form"),
        Vertex("contradiction", content="contradiction — no possible state of affairs"),
        Vertex("truth_function", content="truth-function — proposition as function of elementary propositions"),
        Vertex("bipolarity", content="bipolarity — every proposition can be true or false"),
        Vertex("type_theory", content="type theory is superfluous if symbols used correctly"),
    ]
    edges = [
        Edge("tautology", "proposition", EdgeType.REFERENCE),
        Edge("tautology", "sense", EdgeType.NEGATION),
        Edge("tautology", "showing", EdgeType.DEPENDENCY),
        Edge("contradiction", "tautology", EdgeType.NEGATION),
        Edge("truth_function", "proposition", EdgeType.FOLD),
        Edge("truth_function", "molecular_prop", EdgeType.DEPENDENCY),
        Edge("bipolarity", "proposition", EdgeType.DEPENDENCY),
        Edge("bipolarity", "ab_notation", EdgeType.REFERENCE),
        Edge("type_theory", "nonsense", EdgeType.REFERENCE),
        Edge("type_theory", "logical_form", EdgeType.NEGATION),
    ]
    return "Notes Dictated to Moore (1914)", vertices, edges


def w_notebooks_1914_1916():
    """1914-1916: Notebooks 1914-1916."""
    vertices = [
        Vertex("object", content="object — simple, the substance of the world"),
        Vertex("name", content="name — the simplest sign, stands for an object"),
        Vertex("state_of_affairs", content="state of affairs — combination of objects"),
        Vertex("world", content="world — totality of facts, not things"),
        Vertex("logical_space", content="logical space — the space of all possible states of affairs"),
        Vertex("subject", content="the subject — limit of the world, not in it"),
        Vertex("ethics", content="ethics — transcendental, cannot be put into words"),
        Vertex("will", content="the will — attitude of the subject to the world"),
        Vertex("solipsism", content="solipsism — what it means coincides with realism"),
        Vertex("simplicity", content="the demand for simples — analysis must come to an end"),
    ]
    edges = [
        Edge("object", "name", EdgeType.DEPENDENCY),
        Edge("state_of_affairs", "object", EdgeType.FOLD),
        Edge("world", "fact", EdgeType.FOLD),
        Edge("world", "state_of_affairs", EdgeType.DEPENDENCY),
        Edge("logical_space", "state_of_affairs", EdgeType.FOLD),
        Edge("logical_space", "proposition", EdgeType.REFERENCE),
        Edge("name", "proposition", EdgeType.DEPENDENCY),
        Edge("subject", "world", EdgeType.NEGATION),
        Edge("ethics", "saying", EdgeType.NEGATION),
        Edge("ethics", "showing", EdgeType.DEPENDENCY),
        Edge("will", "subject", EdgeType.DEPENDENCY),
        Edge("will", "ethics", EdgeType.REFERENCE),
        Edge("solipsism", "subject", EdgeType.DEPENDENCY),
        Edge("solipsism", "world", EdgeType.REFERENCE),
        Edge("simplicity", "object", EdgeType.DEPENDENCY),
        Edge("simplicity", "name", EdgeType.REFERENCE),
    ]
    return "Notebooks 1914-1916 (1914-1916)", vertices, edges


def w_tractatus():
    """1921: Tractatus Logico-Philosophicus."""
    vertices = [
        Vertex("picture", content="picture — a model of reality, shares logical form with fact"),
        Vertex("thought", content="thought — a logical picture of facts"),
        Vertex("elementary_prop", content="elementary proposition — the simplest assertion about a state of affairs"),
        Vertex("truth_table", content="truth-table — method of representing truth-functions"),
        Vertex("general_form_prop", content="general form of proposition — the one logical constant"),
        Vertex("identity", content="identity — not a relation, expressed by sameness of sign"),
        Vertex("number", content="number — exponent of an operation"),
        Vertex("philosophy", content="philosophy — not a doctrine but an activity of clarification"),
        Vertex("mystical", content="the mystical — that the world exists, not how"),
        Vertex("silence", content="whereof one cannot speak, thereof one must be silent"),
        Vertex("ladder", content="the ladder — propositions of the Tractatus themselves nonsensical"),
    ]
    edges = [
        Edge("picture", "fact", EdgeType.DEPENDENCY),
        Edge("picture", "logical_form", EdgeType.DEPENDENCY),
        Edge("picture", "proposition", EdgeType.REFERENCE),
        Edge("thought", "picture", EdgeType.DEPENDENCY),
        Edge("thought", "proposition", EdgeType.REFERENCE),
        Edge("elementary_prop", "state_of_affairs", EdgeType.DEPENDENCY),
        Edge("elementary_prop", "name", EdgeType.FOLD),
        Edge("truth_table", "truth_function", EdgeType.DEPENDENCY),
        Edge("truth_table", "elementary_prop", EdgeType.FOLD),
        Edge("general_form_prop", "truth_function", EdgeType.FOLD),
        Edge("general_form_prop", "proposition", EdgeType.DEPENDENCY),
        Edge("identity", "name", EdgeType.REFERENCE),
        Edge("identity", "nonsense", EdgeType.REFERENCE),
        Edge("number", "general_form_prop", EdgeType.REFERENCE),
        Edge("philosophy", "nonsense", EdgeType.NEGATION),
        Edge("philosophy", "showing", EdgeType.DEPENDENCY),
        Edge("mystical", "ethics", EdgeType.DEPENDENCY),
        Edge("mystical", "showing", EdgeType.DEPENDENCY),
        Edge("mystical", "saying", EdgeType.NEGATION),
        Edge("silence", "nonsense", EdgeType.DEPENDENCY),
        Edge("silence", "mystical", EdgeType.REFERENCE),
        Edge("ladder", "nonsense", EdgeType.SUBLATION),
        Edge("ladder", "proposition", EdgeType.NEGATION),
        Edge("ladder", "showing", EdgeType.DEPENDENCY),
    ]
    return "Tractatus Logico-Philosophicus (1921)", vertices, edges


def w_some_remarks_on_logical_form():
    """1929: Some Remarks on Logical Form."""
    vertices = [
        Vertex("colour_exclusion", content="colour exclusion problem — two colours cannot be at same point"),
        Vertex("logical_independence", content="logical independence of elementary propositions — challenged"),
        Vertex("number_in_logic", content="numbers enter logic at the level of atomic propositions"),
        Vertex("phenomenological_language", content="phenomenological language — direct description of phenomena"),
    ]
    edges = [
        Edge("colour_exclusion", "elementary_prop", EdgeType.NEGATION),
        Edge("colour_exclusion", "logical_independence", EdgeType.NEGATION),
        Edge("logical_independence", "elementary_prop", EdgeType.DEPENDENCY),
        Edge("logical_independence", "truth_function", EdgeType.REFERENCE),
        Edge("number_in_logic", "number", EdgeType.REFERENCE),
        Edge("number_in_logic", "logical_form", EdgeType.DEPENDENCY),
        Edge("phenomenological_language", "picture", EdgeType.REFERENCE),
        Edge("phenomenological_language", "logical_form", EdgeType.NEGATION),
    ]
    return "Some Remarks on Logical Form (1929)", vertices, edges


def w_philosophical_remarks():
    """1930: Philosophical Remarks."""
    vertices = [
        Vertex("verification", content="verification — understanding a proposition = knowing how to verify it"),
        Vertex("grammar", content="grammar — rules for the use of signs, autonomous"),
        Vertex("expectation", content="expectation — internally related to its fulfilment"),
        Vertex("visual_space", content="visual space — the space of immediate experience"),
        Vertex("infinity", content="infinity — not a number, a rule for generating numbers"),
        Vertex("hypothesis", content="hypothesis — a law for forming propositions, not itself verifiable"),
        Vertex("intentionality", content="intentionality — the internal relation of thought and object"),
    ]
    edges = [
        Edge("verification", "sense", EdgeType.DEPENDENCY),
        Edge("verification", "proposition", EdgeType.REFERENCE),
        Edge("verification", "picture", EdgeType.NEGATION),
        Edge("grammar", "logical_form", EdgeType.SUBLATION),
        Edge("grammar", "nonsense", EdgeType.REFERENCE),
        Edge("expectation", "intentionality", EdgeType.DEPENDENCY),
        Edge("expectation", "proposition", EdgeType.REFERENCE),
        Edge("visual_space", "logical_space", EdgeType.NEGATION),
        Edge("visual_space", "phenomenological_language", EdgeType.DEPENDENCY),
        Edge("infinity", "number", EdgeType.NEGATION),
        Edge("infinity", "general_form_prop", EdgeType.REFERENCE),
        Edge("hypothesis", "verification", EdgeType.NEGATION),
        Edge("hypothesis", "proposition", EdgeType.REFERENCE),
        Edge("intentionality", "thought", EdgeType.DEPENDENCY),
        Edge("intentionality", "picture", EdgeType.REFERENCE),
    ]
    return "Philosophical Remarks (1930)", vertices, edges


def w_philosophical_grammar():
    """1933: Philosophical Grammar."""
    vertices = [
        Vertex("meaning", content="meaning — not an object correlated with a word, but its use"),
        Vertex("use", content="use — the life of the sign"),
        Vertex("calculus", content="calculus model of language — operating with signs according to rules"),
        Vertex("understanding", content="understanding — mastery of a technique, not a mental process"),
        Vertex("language", content="language — not a calculus, a family of practices"),
        Vertex("rule", content="rule — standard of correctness for use of signs"),
    ]
    edges = [
        Edge("meaning", "use", EdgeType.DEPENDENCY),
        Edge("meaning", "name", EdgeType.NEGATION),
        Edge("meaning", "picture", EdgeType.NEGATION),
        Edge("use", "grammar", EdgeType.DEPENDENCY),
        Edge("use", "language", EdgeType.DEPENDENCY),
        Edge("calculus", "grammar", EdgeType.REFERENCE),
        Edge("calculus", "truth_function", EdgeType.NEGATION),
        Edge("understanding", "meaning", EdgeType.DEPENDENCY),
        Edge("understanding", "verification", EdgeType.NEGATION),
        Edge("understanding", "use", EdgeType.REFERENCE),
        Edge("language", "calculus", EdgeType.SUBLATION),
        Edge("language", "proposition", EdgeType.REFERENCE),
        Edge("rule", "grammar", EdgeType.DEPENDENCY),
        Edge("rule", "calculus", EdgeType.REFERENCE),
    ]
    return "Philosophical Grammar (1933)", vertices, edges


def w_blue_book():
    """1933-1934: The Blue Book."""
    vertices = [
        Vertex("language_game", content="language game — simple practices with words as objects of comparison"),
        Vertex("family_resemblance", content="family resemblance — overlapping similarities, no common essence"),
        Vertex("craving_generality", content="craving for generality — the philosophical disease"),
        Vertex("ostensive_definition", content="ostensive definition — pointing alone does not fix meaning"),
        Vertex("mental_process", content="mental process — understanding/meaning are not inner processes"),
    ]
    edges = [
        Edge("language_game", "language", EdgeType.DEPENDENCY),
        Edge("language_game", "use", EdgeType.DEPENDENCY),
        Edge("language_game", "calculus", EdgeType.NEGATION),
        Edge("family_resemblance", "craving_generality", EdgeType.NEGATION),
        Edge("family_resemblance", "object", EdgeType.NEGATION),
        Edge("craving_generality", "philosophy", EdgeType.REFERENCE),
        Edge("ostensive_definition", "name", EdgeType.NEGATION),
        Edge("ostensive_definition", "meaning", EdgeType.REFERENCE),
        Edge("ostensive_definition", "language_game", EdgeType.DEPENDENCY),
        Edge("mental_process", "understanding", EdgeType.NEGATION),
        Edge("mental_process", "intentionality", EdgeType.NEGATION),
    ]
    return "The Blue Book (1933-1934)", vertices, edges


def w_brown_book():
    """1934-1935: The Brown Book."""
    vertices = [
        Vertex("primitive_lg", content="primitive language games — simplified models of language use"),
        Vertex("builder_game", content="builder's game — Slab! Pillar! Block!"),
        Vertex("training", content="training — how one is brought into a practice"),
        Vertex("aspect_seeing", content="aspect-seeing — seeing-as, noticing an aspect"),
    ]
    edges = [
        Edge("primitive_lg", "language_game", EdgeType.DEPENDENCY),
        Edge("primitive_lg", "language", EdgeType.REFERENCE),
        Edge("builder_game", "primitive_lg", EdgeType.DEPENDENCY),
        Edge("builder_game", "ostensive_definition", EdgeType.REFERENCE),
        Edge("training", "understanding", EdgeType.DEPENDENCY),
        Edge("training", "rule", EdgeType.REFERENCE),
        Edge("training", "language_game", EdgeType.REFERENCE),
        Edge("aspect_seeing", "visual_space", EdgeType.REFERENCE),
        Edge("aspect_seeing", "meaning", EdgeType.DEPENDENCY),
        Edge("aspect_seeing", "picture", EdgeType.NEGATION),
    ]
    return "The Brown Book (1934-1935)", vertices, edges


def w_remarks_foundations_math_i():
    """1937-1938: Remarks on the Foundations of Mathematics, Parts I-II."""
    vertices = [
        Vertex("mathematics", content="mathematics — a motley of techniques, not discovery of truths"),
        Vertex("proof", content="proof — produces a new concept, not discovery of a fact"),
        Vertex("surveyability", content="surveyability — a proof must be surveyable"),
        Vertex("following_a_rule_math", content="following a rule in mathematics — normative, not descriptive"),
        Vertex("logical_compulsion", content="logical compulsion — the hardness of the logical must"),
    ]
    edges = [
        Edge("mathematics", "grammar", EdgeType.DEPENDENCY),
        Edge("mathematics", "language_game", EdgeType.REFERENCE),
        Edge("mathematics", "tautology", EdgeType.NEGATION),
        Edge("proof", "mathematics", EdgeType.DEPENDENCY),
        Edge("proof", "showing", EdgeType.REFERENCE),
        Edge("proof", "proposition", EdgeType.NEGATION),
        Edge("surveyability", "proof", EdgeType.DEPENDENCY),
        Edge("surveyability", "understanding", EdgeType.REFERENCE),
        Edge("following_a_rule_math", "rule", EdgeType.DEPENDENCY),
        Edge("following_a_rule_math", "training", EdgeType.REFERENCE),
        Edge("logical_compulsion", "following_a_rule_math", EdgeType.DEPENDENCY),
        Edge("logical_compulsion", "grammar", EdgeType.REFERENCE),
    ]
    return "Remarks on Foundations of Mathematics I-II (1937-1938)", vertices, edges


def w_lectures_on_aesthetics():
    """1938: Lectures on Aesthetics, Psychology, and Religious Belief."""
    vertices = [
        Vertex("aesthetics", content="aesthetics — not a matter of liking, but of the right word/gesture"),
        Vertex("religion", content="religious belief — a passionate commitment, not a hypothesis"),
        Vertex("psychology_confusion", content="conceptual confusion in psychology"),
        Vertex("intransitive_use", content="intransitive use — 'I know what I mean' without criteria"),
    ]
    edges = [
        Edge("aesthetics", "language_game", EdgeType.DEPENDENCY),
        Edge("aesthetics", "family_resemblance", EdgeType.REFERENCE),
        Edge("aesthetics", "ethics", EdgeType.REFERENCE),
        Edge("religion", "language_game", EdgeType.DEPENDENCY),
        Edge("religion", "hypothesis", EdgeType.NEGATION),
        Edge("religion", "form_of_life", EdgeType.REFERENCE),
        Edge("psychology_confusion", "grammar", EdgeType.DEPENDENCY),
        Edge("psychology_confusion", "mental_process", EdgeType.REFERENCE),
        Edge("intransitive_use", "meaning", EdgeType.REFERENCE),
        Edge("intransitive_use", "use", EdgeType.NEGATION),
    ]
    return "Lectures on Aesthetics (1938)", vertices, edges


def w_philosophical_investigations():
    """1945-1949: Philosophical Investigations (Parts I & II)."""
    vertices = [
        Vertex("form_of_life", content="form of life — the given, agreement in actions and judgments"),
        Vertex("rule_following", content="rule-following — no interpretation determines meaning, it is a practice"),
        Vertex("private_language", content="private language argument — no language can be essentially private"),
        Vertex("beetle_in_box", content="beetle in a box — private object drops out of language game"),
        Vertex("pain", content="pain — not identified by inner observation, expressed"),
        Vertex("sensation", content="sensation — the grammar of sensation-words"),
        Vertex("inner_outer", content="inner/outer — the misleading picture of inside and outside"),
        Vertex("criterion", content="criterion — grammatical relation between expression and what is expressed"),
        Vertex("therapy", content="philosophy as therapy — dissolving not solving problems"),
        Vertex("clarity", content="clarity — the philosophical goal, not theory"),
        Vertex("duck_rabbit", content="duck-rabbit — aspect perception, seeing-as"),
        Vertex("secondary_sense", content="secondary sense — 'fat' Wednesday, metaphorical use"),
        Vertex("certainty_PI", content="agreement in judgments — groundless ground of language games"),
    ]
    edges = [
        Edge("form_of_life", "language_game", EdgeType.DEPENDENCY),
        Edge("form_of_life", "training", EdgeType.DEPENDENCY),
        Edge("form_of_life", "world", EdgeType.NEGATION),
        Edge("rule_following", "rule", EdgeType.SUBLATION),
        Edge("rule_following", "meaning", EdgeType.DEPENDENCY),
        Edge("rule_following", "understanding", EdgeType.DEPENDENCY),
        Edge("rule_following", "form_of_life", EdgeType.DEPENDENCY),
        Edge("private_language", "language_game", EdgeType.DEPENDENCY),
        Edge("private_language", "rule_following", EdgeType.DEPENDENCY),
        Edge("private_language", "sensation", EdgeType.REFERENCE),
        Edge("beetle_in_box", "private_language", EdgeType.DEPENDENCY),
        Edge("beetle_in_box", "language_game", EdgeType.REFERENCE),
        Edge("beetle_in_box", "object", EdgeType.NEGATION),
        Edge("pain", "sensation", EdgeType.DEPENDENCY),
        Edge("pain", "private_language", EdgeType.REFERENCE),
        Edge("pain", "criterion", EdgeType.DEPENDENCY),
        Edge("pain", "inner_outer", EdgeType.NEGATION),
        Edge("sensation", "grammar", EdgeType.DEPENDENCY),
        Edge("sensation", "mental_process", EdgeType.NEGATION),
        Edge("inner_outer", "picture", EdgeType.NEGATION),
        Edge("inner_outer", "private_language", EdgeType.REFERENCE),
        Edge("criterion", "grammar", EdgeType.DEPENDENCY),
        Edge("criterion", "verification", EdgeType.SUBLATION),
        Edge("therapy", "philosophy", EdgeType.SUBLATION),
        Edge("therapy", "nonsense", EdgeType.REFERENCE),
        Edge("therapy", "clarity", EdgeType.DEPENDENCY),
        Edge("clarity", "philosophy", EdgeType.DEPENDENCY),
        Edge("clarity", "showing", EdgeType.REFERENCE),
        Edge("duck_rabbit", "aspect_seeing", EdgeType.DEPENDENCY),
        Edge("duck_rabbit", "meaning", EdgeType.REFERENCE),
        Edge("secondary_sense", "meaning", EdgeType.DEPENDENCY),
        Edge("secondary_sense", "language_game", EdgeType.REFERENCE),
        Edge("certainty_PI", "form_of_life", EdgeType.DEPENDENCY),
        Edge("certainty_PI", "rule_following", EdgeType.REFERENCE),
    ]
    return "Philosophical Investigations (1945-1949)", vertices, edges


def w_zettel():
    """1945-1948: Zettel (compiled posthumously)."""
    vertices = [
        Vertex("intention", content="intention — not an inner act, embedded in situation and custom"),
        Vertex("concept_formation", content="concept formation — concepts shaped by natural facts"),
        Vertex("thinking", content="thinking — operating with signs, not an accompaniment"),
        Vertex("imagination", content="imagination — not a faded picture, a capacity"),
        Vertex("disposition", content="disposition — not an inner state, a pattern of behaviour"),
    ]
    edges = [
        Edge("intention", "mental_process", EdgeType.NEGATION),
        Edge("intention", "language_game", EdgeType.DEPENDENCY),
        Edge("intention", "form_of_life", EdgeType.REFERENCE),
        Edge("concept_formation", "grammar", EdgeType.DEPENDENCY),
        Edge("concept_formation", "form_of_life", EdgeType.DEPENDENCY),
        Edge("concept_formation", "family_resemblance", EdgeType.REFERENCE),
        Edge("thinking", "mental_process", EdgeType.NEGATION),
        Edge("thinking", "language", EdgeType.DEPENDENCY),
        Edge("thinking", "use", EdgeType.REFERENCE),
        Edge("imagination", "picture", EdgeType.NEGATION),
        Edge("imagination", "aspect_seeing", EdgeType.REFERENCE),
        Edge("disposition", "inner_outer", EdgeType.NEGATION),
        Edge("disposition", "criterion", EdgeType.DEPENDENCY),
        Edge("disposition", "language_game", EdgeType.REFERENCE),
    ]
    return "Zettel (1945-1948)", vertices, edges


def w_remarks_philosophy_psychology_i():
    """1946-1947: Remarks on the Philosophy of Psychology, Vol. I."""
    vertices = [
        Vertex("expression", content="expression — pain behaviour is not a report of inner states"),
        Vertex("attitude", content="attitude toward a soul — not belief in a hidden object"),
        Vertex("seeing_as", content="seeing-as — categorical, not an interpretation added to seeing"),
        Vertex("experience_of_meaning", content="experience of meaning — meaning is not an experience alongside the word"),
    ]
    edges = [
        Edge("expression", "pain", EdgeType.DEPENDENCY),
        Edge("expression", "inner_outer", EdgeType.NEGATION),
        Edge("expression", "criterion", EdgeType.REFERENCE),
        Edge("attitude", "form_of_life", EdgeType.DEPENDENCY),
        Edge("attitude", "private_language", EdgeType.REFERENCE),
        Edge("attitude", "beetle_in_box", EdgeType.REFERENCE),
        Edge("seeing_as", "aspect_seeing", EdgeType.DEPENDENCY),
        Edge("seeing_as", "duck_rabbit", EdgeType.REFERENCE),
        Edge("seeing_as", "understanding", EdgeType.REFERENCE),
        Edge("experience_of_meaning", "meaning", EdgeType.DEPENDENCY),
        Edge("experience_of_meaning", "mental_process", EdgeType.NEGATION),
        Edge("experience_of_meaning", "use", EdgeType.REFERENCE),
    ]
    return "Remarks on Philosophy of Psychology I (1946-1947)", vertices, edges


def w_remarks_philosophy_psychology_ii():
    """1947-1948: Remarks on the Philosophy of Psychology, Vol. II."""
    vertices = [
        Vertex("imponderable_evidence", content="imponderable evidence — subtle cues in understanding others"),
        Vertex("fine_shades_meaning", content="fine shades of meaning — nuances that resist analysis"),
    ]
    edges = [
        Edge("imponderable_evidence", "criterion", EdgeType.REFERENCE),
        Edge("imponderable_evidence", "expression", EdgeType.DEPENDENCY),
        Edge("imponderable_evidence", "form_of_life", EdgeType.REFERENCE),
        Edge("fine_shades_meaning", "meaning", EdgeType.DEPENDENCY),
        Edge("fine_shades_meaning", "experience_of_meaning", EdgeType.REFERENCE),
        Edge("fine_shades_meaning", "secondary_sense", EdgeType.DEPENDENCY),
    ]
    return "Remarks on Philosophy of Psychology II (1947-1948)", vertices, edges


def w_last_writings_i():
    """1948-1949: Last Writings on the Philosophy of Psychology, Vol. I."""
    vertices = [
        Vertex("inner_certainty", content="inner certainty — not based on evidence, part of the language game"),
        Vertex("pretending", content="pretending — not merely behaviour, requires a context"),
        Vertex("understanding_others", content="understanding others — not inference from behaviour to inner states"),
    ]
    edges = [
        Edge("inner_certainty", "certainty_PI", EdgeType.DEPENDENCY),
        Edge("inner_certainty", "private_language", EdgeType.NEGATION),
        Edge("inner_certainty", "criterion", EdgeType.REFERENCE),
        Edge("pretending", "expression", EdgeType.DEPENDENCY),
        Edge("pretending", "language_game", EdgeType.REFERENCE),
        Edge("pretending", "inner_outer", EdgeType.REFERENCE),
        Edge("understanding_others", "attitude", EdgeType.DEPENDENCY),
        Edge("understanding_others", "imponderable_evidence", EdgeType.REFERENCE),
        Edge("understanding_others", "form_of_life", EdgeType.DEPENDENCY),
    ]
    return "Last Writings on Phil. of Psychology I (1948-1949)", vertices, edges


def w_last_writings_ii():
    """1949-1951: Last Writings on the Philosophy of Psychology, Vol. II."""
    vertices = [
        Vertex("inner_outer_final", content="inner/outer revisited — the picture's grip loosens"),
        Vertex("concept_experience", content="experience of a concept — not sensation, not nothing"),
    ]
    edges = [
        Edge("inner_outer_final", "inner_outer", EdgeType.SUBLATION),
        Edge("inner_outer_final", "expression", EdgeType.DEPENDENCY),
        Edge("inner_outer_final", "form_of_life", EdgeType.REFERENCE),
        Edge("concept_experience", "experience_of_meaning", EdgeType.DEPENDENCY),
        Edge("concept_experience", "aspect_seeing", EdgeType.REFERENCE),
        Edge("concept_experience", "sensation", EdgeType.NEGATION),
    ]
    return "Last Writings on Phil. of Psychology II (1949-1951)", vertices, edges


def w_remarks_foundations_math_iii_vii():
    """1941-1944: Remarks on the Foundations of Mathematics, Parts III-VII."""
    vertices = [
        Vertex("consistency", content="consistency — not a mathematical discovery, a grammatical decision"),
        Vertex("godel", content="Gödel's theorem — prose interpretation misleading"),
        Vertex("contradiction_math", content="contradiction in mathematics — not catastrophic, just unusable"),
        Vertex("concept_proof", content="proof changes the concept — new proof, new concept"),
    ]
    edges = [
        Edge("consistency", "mathematics", EdgeType.DEPENDENCY),
        Edge("consistency", "grammar", EdgeType.REFERENCE),
        Edge("consistency", "logical_compulsion", EdgeType.NEGATION),
        Edge("godel", "proof", EdgeType.REFERENCE),
        Edge("godel", "nonsense", EdgeType.REFERENCE),
        Edge("godel", "saying", EdgeType.NEGATION),
        Edge("contradiction_math", "contradiction", EdgeType.REFERENCE),
        Edge("contradiction_math", "language_game", EdgeType.DEPENDENCY),
        Edge("contradiction_math", "mathematics", EdgeType.REFERENCE),
        Edge("concept_proof", "proof", EdgeType.DEPENDENCY),
        Edge("concept_proof", "concept_formation", EdgeType.REFERENCE),
        Edge("concept_proof", "grammar", EdgeType.DEPENDENCY),
    ]
    return "Remarks on Foundations of Mathematics III-VII (1941-1944)", vertices, edges


def w_remarks_on_colour():
    """1950-1951: Remarks on Colour."""
    vertices = [
        Vertex("colour", content="colour — grammar of colour words, not physics or optics"),
        Vertex("colour_geometry", content="colour geometry — the logical structure of colour space"),
        Vertex("transparency", content="transparency — white cannot be transparent (grammatical)"),
        Vertex("reddish_green", content="reddish green — no such thing; grammatical, not empirical"),
    ]
    edges = [
        Edge("colour", "grammar", EdgeType.DEPENDENCY),
        Edge("colour", "colour_exclusion", EdgeType.SUBLATION),
        Edge("colour", "language_game", EdgeType.REFERENCE),
        Edge("colour_geometry", "colour", EdgeType.DEPENDENCY),
        Edge("colour_geometry", "logical_space", EdgeType.REFERENCE),
        Edge("colour_geometry", "visual_space", EdgeType.REFERENCE),
        Edge("transparency", "colour", EdgeType.DEPENDENCY),
        Edge("transparency", "grammar", EdgeType.REFERENCE),
        Edge("reddish_green", "nonsense", EdgeType.DEPENDENCY),
        Edge("reddish_green", "colour_geometry", EdgeType.REFERENCE),
        Edge("reddish_green", "grammar", EdgeType.REFERENCE),
    ]
    return "Remarks on Colour (1950-1951)", vertices, edges


def w_on_certainty():
    """1950-1951: On Certainty."""
    vertices = [
        Vertex("certainty", content="certainty — not justified belief, the bedrock of action"),
        Vertex("doubt", content="doubt — requires grounds; universal doubt is senseless"),
        Vertex("knowledge", content="knowledge — I know = I am certain, but certainty is not knowledge"),
        Vertex("hinge_proposition", content="hinge proposition — what stands fast, the riverbed"),
        Vertex("world_picture", content="world-picture — the inherited background of judging"),
        Vertex("belief", content="belief — acting, not a mental state"),
        Vertex("groundlessness", content="groundlessness — at the bottom justification comes to an end"),
        Vertex("mistake_vs_doubt", content="mistake vs. doubt — a mistake presupposes the framework, doubt questions it"),
    ]
    edges = [
        Edge("certainty", "doubt", EdgeType.NEGATION),
        Edge("certainty", "knowledge", EdgeType.NEGATION),
        Edge("certainty", "form_of_life", EdgeType.DEPENDENCY),
        Edge("certainty", "certainty_PI", EdgeType.DEPENDENCY),
        Edge("doubt", "language_game", EdgeType.DEPENDENCY),
        Edge("doubt", "grammar", EdgeType.REFERENCE),
        Edge("knowledge", "certainty", EdgeType.REFERENCE),
        Edge("knowledge", "proposition", EdgeType.REFERENCE),
        Edge("hinge_proposition", "certainty", EdgeType.DEPENDENCY),
        Edge("hinge_proposition", "world_picture", EdgeType.DEPENDENCY),
        Edge("hinge_proposition", "rule_following", EdgeType.REFERENCE),
        Edge("world_picture", "form_of_life", EdgeType.DEPENDENCY),
        Edge("world_picture", "grammar", EdgeType.REFERENCE),
        Edge("world_picture", "language_game", EdgeType.REFERENCE),
        Edge("belief", "certainty", EdgeType.DEPENDENCY),
        Edge("belief", "form_of_life", EdgeType.REFERENCE),
        Edge("belief", "mental_process", EdgeType.NEGATION),
        Edge("groundlessness", "hinge_proposition", EdgeType.DEPENDENCY),
        Edge("groundlessness", "rule_following", EdgeType.REFERENCE),
        Edge("groundlessness", "justification", EdgeType.NEGATION),
        Edge("mistake_vs_doubt", "doubt", EdgeType.DEPENDENCY),
        Edge("mistake_vs_doubt", "hinge_proposition", EdgeType.REFERENCE),
    ]
    return "On Certainty (1950-1951)", vertices, edges


def w_culture_and_value():
    """1914-1951: Culture and Value (selections spanning entire career)."""
    vertices = [
        Vertex("culture", content="culture — not civilization; depth vs. surface"),
        Vertex("value", content="value — cannot be stated, only shown in how one lives"),
        Vertex("genius", content="genius — courage in one's talent"),
        Vertex("honesty", content="honesty — the philosophical virtue"),
        Vertex("music", content="music — understanding music as a model for understanding language"),
        Vertex("justification", content="justification — comes to an end somewhere"),
    ]
    edges = [
        Edge("culture", "form_of_life", EdgeType.REFERENCE),
        Edge("culture", "value", EdgeType.DEPENDENCY),
        Edge("value", "ethics", EdgeType.DEPENDENCY),
        Edge("value", "saying", EdgeType.NEGATION),
        Edge("value", "showing", EdgeType.DEPENDENCY),
        Edge("genius", "honesty", EdgeType.DEPENDENCY),
        Edge("genius", "culture", EdgeType.REFERENCE),
        Edge("honesty", "philosophy", EdgeType.DEPENDENCY),
        Edge("honesty", "clarity", EdgeType.REFERENCE),
        Edge("music", "language_game", EdgeType.REFERENCE),
        Edge("music", "understanding", EdgeType.REFERENCE),
        Edge("music", "aspect_seeing", EdgeType.REFERENCE),
        Edge("justification", "groundlessness", EdgeType.REFERENCE),
        Edge("justification", "rule_following", EdgeType.REFERENCE),
        Edge("justification", "certainty", EdgeType.REFERENCE),
    ]
    return "Culture and Value (1914-1951)", vertices, edges


def w_lectures_foundations_math():
    """1939: Wittgenstein's Lectures on the Foundations of Mathematics."""
    vertices = [
        Vertex("math_invention", content="mathematical invention, not discovery"),
        Vertex("rule_in_math", content="rule in mathematics — we lay down rails, not discover them"),
        Vertex("contradiction_toleration", content="contradiction — may be tolerated if it has no practical effect"),
    ]
    edges = [
        Edge("math_invention", "mathematics", EdgeType.DEPENDENCY),
        Edge("math_invention", "proof", EdgeType.REFERENCE),
        Edge("math_invention", "grammar", EdgeType.REFERENCE),
        Edge("rule_in_math", "following_a_rule_math", EdgeType.DEPENDENCY),
        Edge("rule_in_math", "rule_following", EdgeType.REFERENCE),
        Edge("rule_in_math", "grammar", EdgeType.REFERENCE),
        Edge("contradiction_toleration", "contradiction_math", EdgeType.REFERENCE),
        Edge("contradiction_toleration", "language_game", EdgeType.DEPENDENCY),
        Edge("contradiction_toleration", "consistency", EdgeType.NEGATION),
    ]
    return "Lectures on Foundations of Mathematics (1939)", vertices, edges


# ---------------------------------------------------------------------------
# ALL_WORKS in approximate chronological order
# ---------------------------------------------------------------------------

ALL_WORKS = [
    w_notes_on_logic,                    # 1913
    w_notes_dictated_to_moore,           # 1914
    w_notebooks_1914_1916,               # 1914-1916
    w_tractatus,                         # 1921
    w_some_remarks_on_logical_form,      # 1929
    w_philosophical_remarks,             # 1930
    w_philosophical_grammar,             # 1933
    w_blue_book,                         # 1933-1934
    w_brown_book,                        # 1934-1935
    w_remarks_foundations_math_i,        # 1937-1938
    w_lectures_on_aesthetics,            # 1938
    w_lectures_foundations_math,         # 1939
    w_remarks_foundations_math_iii_vii,  # 1941-1944
    w_philosophical_investigations,      # 1945-1949
    w_zettel,                            # 1945-1948
    w_remarks_philosophy_psychology_i,   # 1946-1947
    w_remarks_philosophy_psychology_ii,  # 1947-1948
    w_last_writings_i,                   # 1948-1949
    w_last_writings_ii,                  # 1949-1951
    w_remarks_on_colour,                 # 1950-1951
    w_on_certainty,                      # 1950-1951
    w_culture_and_value,                 # 1914-1951 (spans entire career)
]


# ---------------------------------------------------------------------------
# Runner
# ---------------------------------------------------------------------------


def run_experiment():
    t0 = time.time()

    print("=" * 70)
    print("WITTGENSTEIN COMPLETE WORKS — INCREMENTAL TRAVERSAL")
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

    # Sublation density
    sub_count = type_counts.get("sublation", 0)
    sub_density = sub_count / final_e if final_e > 0 else 0
    print(f"  Sublation density: {sub_density:.4f} ({sub_count}/{final_e})")

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "wittgenstein_complete_works_incremental",
        "source": "Wittgenstein Complete Works — 22 works (1913-1951)",
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

    output_path = "experiment_wittgenstein.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
