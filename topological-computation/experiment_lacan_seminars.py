"""Lacan Seminars I-XXIII — incremental traversal experiment.

Each seminar is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the seminar
(runs until beta_1 is stable for 50 consecutive steps).
Then the next seminar is injected.

This models the progressive development of Lacan's thought
across the 23 seminars (1953-1976) as an incremental topological
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
# Seminar encodings: each returns (seminar_name, vertices, edges)
# Cross-seminar edges connect to vertices already introduced in prior seminars.
# ---------------------------------------------------------------------------


def sem_I():
    """Seminar I: Freud's Papers on Technique (1953-54).

    Early Lacan: return to Freud, critique of ego psychology,
    symbolic vs imaginary, speech and language.
    """
    vertices = [
        Vertex("transference", content="transference"),
        Vertex("resistance", content="resistance"),
        Vertex("ego", content="ego (moi)"),
        Vertex("ideal_ego", content="ideal ego (moi ideal)"),
        Vertex("ego_ideal", content="ego-ideal (ideal du moi)"),
        Vertex("symbolic_exchange", content="symbolic exchange"),
        Vertex("word", content="word (parole)"),
        Vertex("empty_speech", content="empty speech (parole vide)"),
        Vertex("full_speech", content="full speech (parole pleine)"),
        Vertex("imaginary", content="the imaginary"),
        Vertex("symbolic", content="the symbolic"),
        Vertex("real", content="the real"),
        Vertex("intersubjectivity", content="intersubjectivity"),
    ]
    edges = [
        Edge("transference", "resistance", EdgeType.NEGATION),
        Edge("resistance", "ego", EdgeType.DEPENDENCY),
        Edge("ideal_ego", "ego", EdgeType.DEPENDENCY),
        Edge("ego_ideal", "ideal_ego", EdgeType.NEGATION),
        Edge("ego_ideal", "symbolic", EdgeType.DEPENDENCY),
        Edge("ideal_ego", "imaginary", EdgeType.DEPENDENCY),
        Edge("symbolic_exchange", "word", EdgeType.DEPENDENCY),
        Edge("empty_speech", "full_speech", EdgeType.NEGATION),
        Edge("full_speech", "symbolic_exchange", EdgeType.DEPENDENCY),
        Edge("empty_speech", "imaginary", EdgeType.DEPENDENCY),
        Edge("full_speech", "symbolic", EdgeType.DEPENDENCY),
        Edge("transference", "intersubjectivity", EdgeType.DEPENDENCY),
        Edge("intersubjectivity", "symbolic_exchange", EdgeType.DEPENDENCY),
        Edge("imaginary", "real", EdgeType.NEGATION),
        Edge("symbolic", "imaginary", EdgeType.NEGATION),
        Edge("real", "symbolic", EdgeType.NEGATION),
    ]
    return "I: Freud's Papers on Technique (1953-54)", vertices, edges


def sem_II():
    """Seminar II: The Ego in Freud's Theory (1954-55).

    Critique of ego psychology, cybernetics, the subject decentered.
    """
    vertices = [
        Vertex("ego_as_function", content="ego as imaginary function"),
        Vertex("imaginary_captation", content="imaginary captation"),
        Vertex("symbolic_determination", content="symbolic determination"),
        Vertex("cybernetics", content="cybernetics"),
        Vertex("repetition_automatism", content="repetition automatism"),
        Vertex("machine", content="machine (symbolic chain)"),
        Vertex("subject_decentered", content="decentered subject"),
        Vertex("other_scene", content="the other scene (ein anderer Schauplatz)"),
        Vertex("pleasure_principle", content="pleasure principle"),
        Vertex("beyond_pleasure", content="beyond the pleasure principle"),
        Vertex("death_drive_sem2", content="death drive (Sem II)"),
    ]
    edges = [
        Edge("ego_as_function", "ego", EdgeType.DEPENDENCY),
        Edge("ego_as_function", "imaginary_captation", EdgeType.DEPENDENCY),
        Edge("imaginary_captation", "imaginary", EdgeType.DEPENDENCY),
        Edge("symbolic_determination", "symbolic", EdgeType.DEPENDENCY),
        Edge("symbolic_determination", "ego_as_function", EdgeType.NEGATION),
        Edge("cybernetics", "machine", EdgeType.DEPENDENCY),
        Edge("repetition_automatism", "machine", EdgeType.DEPENDENCY),
        Edge("repetition_automatism", "symbolic_determination", EdgeType.DEPENDENCY),
        Edge("subject_decentered", "ego", EdgeType.NEGATION),
        Edge("subject_decentered", "other_scene", EdgeType.DEPENDENCY),
        Edge("other_scene", "symbolic", EdgeType.DEPENDENCY),
        Edge("beyond_pleasure", "pleasure_principle", EdgeType.NEGATION),
        Edge("death_drive_sem2", "beyond_pleasure", EdgeType.DEPENDENCY),
        Edge("death_drive_sem2", "repetition_automatism", EdgeType.DEPENDENCY),
    ]
    return "II: The Ego in Freud's Theory (1954-55)", vertices, edges


def sem_III():
    """Seminar III: The Psychoses (1955-56).

    Foreclosure, the Name-of-the-Father, psychotic structure.
    """
    vertices = [
        Vertex("foreclosure", content="foreclosure (Verwerfung)"),
        Vertex("Name_of_Father", content="Name-of-the-Father"),
        Vertex("psychosis", content="psychosis"),
        Vertex("signifier", content="signifier"),
        Vertex("signified", content="signified"),
        Vertex("metaphor_failure", content="metaphoric failure"),
        Vertex("delusional_metaphor", content="delusional metaphor"),
        Vertex("neurosis", content="neurosis"),
        Vertex("repression", content="repression (Verdrangung)"),
        Vertex("point_de_capiton", content="quilting point (point de capiton)"),
        Vertex("Schreber", content="Schreber case"),
    ]
    edges = [
        Edge("foreclosure", "repression", EdgeType.NEGATION),
        Edge("foreclosure", "Name_of_Father", EdgeType.DEPENDENCY),
        Edge("psychosis", "foreclosure", EdgeType.DEPENDENCY),
        Edge("neurosis", "repression", EdgeType.DEPENDENCY),
        Edge("signifier", "signified", EdgeType.NEGATION),
        Edge("signifier", "symbolic", EdgeType.DEPENDENCY),
        Edge("metaphor_failure", "Name_of_Father", EdgeType.DEPENDENCY),
        Edge("metaphor_failure", "signifier", EdgeType.DEPENDENCY),
        Edge("psychosis", "metaphor_failure", EdgeType.DEPENDENCY),
        Edge("delusional_metaphor", "metaphor_failure", EdgeType.NEGATION),
        Edge("delusional_metaphor", "psychosis", EdgeType.DEPENDENCY),
        Edge("point_de_capiton", "signifier", EdgeType.DEPENDENCY),
        Edge("point_de_capiton", "signified", EdgeType.DEPENDENCY),
        Edge("Schreber", "psychosis", EdgeType.REFERENCE),
        Edge("Schreber", "delusional_metaphor", EdgeType.REFERENCE),
        Edge("Name_of_Father", "symbolic", EdgeType.DEPENDENCY),
    ]
    return "III: The Psychoses (1955-56)", vertices, edges


def sem_IV():
    """Seminar IV: The Object Relation (1956-57).

    Critique of object relations theory. Three forms of lack.
    """
    vertices = [
        Vertex("lack_of_object", content="lack of object"),
        Vertex("frustration", content="frustration (Versagung)"),
        Vertex("privation", content="privation"),
        Vertex("castration", content="castration"),
        Vertex("phobic_object", content="phobic object"),
        Vertex("imaginary_phallus", content="imaginary phallus (-phi)"),
        Vertex("mother_desire", content="desire of the mother"),
        Vertex("little_Hans", content="Little Hans case"),
        Vertex("fetish", content="fetish"),
    ]
    edges = [
        Edge("lack_of_object", "frustration", EdgeType.DEPENDENCY),
        Edge("lack_of_object", "privation", EdgeType.DEPENDENCY),
        Edge("lack_of_object", "castration", EdgeType.DEPENDENCY),
        Edge("frustration", "imaginary", EdgeType.DEPENDENCY),
        Edge("privation", "real", EdgeType.DEPENDENCY),
        Edge("castration", "symbolic", EdgeType.DEPENDENCY),
        Edge("phobic_object", "castration", EdgeType.DEPENDENCY),
        Edge("phobic_object", "Name_of_Father", EdgeType.REFERENCE),
        Edge("imaginary_phallus", "lack_of_object", EdgeType.DEPENDENCY),
        Edge("imaginary_phallus", "mother_desire", EdgeType.DEPENDENCY),
        Edge("mother_desire", "castration", EdgeType.DEPENDENCY),
        Edge("little_Hans", "phobic_object", EdgeType.REFERENCE),
        Edge("fetish", "privation", EdgeType.DEPENDENCY),
        Edge("fetish", "castration", EdgeType.NEGATION),
    ]
    return "IV: The Object Relation (1956-57)", vertices, edges


def sem_V():
    """Seminar V: Formations of the Unconscious (1957-58).

    Graph of desire, paternal metaphor, the joke.
    """
    vertices = [
        Vertex("formations_unconscious", content="formations of the unconscious"),
        Vertex("joke", content="joke (Witz)"),
        Vertex("graph_of_desire", content="graph of desire"),
        Vertex("paternal_metaphor", content="paternal metaphor"),
        Vertex("demand", content="demand"),
        Vertex("desire", content="desire"),
        Vertex("need", content="need (besoin)"),
        Vertex("symbolic_phallus", content="symbolic phallus (Phi)"),
        Vertex("Other_A", content="the big Other (A)"),
        Vertex("message", content="message"),
        Vertex("code", content="code"),
    ]
    edges = [
        Edge("formations_unconscious", "signifier", EdgeType.DEPENDENCY),
        Edge("joke", "formations_unconscious", EdgeType.DEPENDENCY),
        Edge("joke", "Other_A", EdgeType.DEPENDENCY),
        Edge("graph_of_desire", "desire", EdgeType.DEPENDENCY),
        Edge("graph_of_desire", "demand", EdgeType.DEPENDENCY),
        Edge("graph_of_desire", "signifier", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "Name_of_Father", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "mother_desire", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "symbolic_phallus", EdgeType.DEPENDENCY),
        Edge("desire", "demand", EdgeType.NEGATION),
        Edge("demand", "need", EdgeType.NEGATION),
        Edge("desire", "need", EdgeType.DEPENDENCY),
        Edge("symbolic_phallus", "imaginary_phallus", EdgeType.SUBLATION),
        Edge("symbolic_phallus", "castration", EdgeType.DEPENDENCY),
        Edge("Other_A", "symbolic", EdgeType.DEPENDENCY),
        Edge("Other_A", "code", EdgeType.DEPENDENCY),
        Edge("message", "code", EdgeType.NEGATION),
        Edge("message", "Other_A", EdgeType.DEPENDENCY),
    ]
    return "V: Formations of the Unconscious (1957-58)", vertices, edges


def sem_VI():
    """Seminar VI: Desire and Its Interpretation (1958-59).

    Hamlet, fantasy, the object of desire.
    """
    vertices = [
        Vertex("desire_interpretation", content="desire and its interpretation"),
        Vertex("hamlet", content="Hamlet"),
        Vertex("fantasy", content="fantasy ($<>a)"),
        Vertex("object_of_desire", content="object of desire"),
        Vertex("phallus_signifier", content="phallus as signifier of desire"),
        Vertex("desire_of_Other", content="desire of the Other"),
        Vertex("mourning", content="mourning"),
        Vertex("acting_out", content="acting out"),
        Vertex("passage_to_act", content="passage to the act"),
    ]
    edges = [
        Edge("desire_interpretation", "desire", EdgeType.DEPENDENCY),
        Edge("desire_interpretation", "graph_of_desire", EdgeType.DEPENDENCY),
        Edge("hamlet", "desire_of_Other", EdgeType.REFERENCE),
        Edge("hamlet", "mourning", EdgeType.REFERENCE),
        Edge("fantasy", "desire", EdgeType.DEPENDENCY),
        Edge("fantasy", "object_of_desire", EdgeType.DEPENDENCY),
        Edge("object_of_desire", "lack_of_object", EdgeType.DEPENDENCY),
        Edge("phallus_signifier", "symbolic_phallus", EdgeType.DEPENDENCY),
        Edge("phallus_signifier", "desire", EdgeType.DEPENDENCY),
        Edge("desire_of_Other", "Other_A", EdgeType.DEPENDENCY),
        Edge("desire_of_Other", "desire", EdgeType.DEPENDENCY),
        Edge("mourning", "lack_of_object", EdgeType.DEPENDENCY),
        Edge("acting_out", "passage_to_act", EdgeType.NEGATION),
        Edge("acting_out", "fantasy", EdgeType.DEPENDENCY),
        Edge("passage_to_act", "fantasy", EdgeType.NEGATION),
    ]
    return "VI: Desire and Its Interpretation (1958-59)", vertices, edges


def sem_VII():
    """Seminar VII: The Ethics of Psychoanalysis (1959-60).

    das Ding, jouissance, sublimation, the Thing, Antigone.
    """
    vertices = [
        Vertex("das_Ding", content="das Ding (the Thing)"),
        Vertex("jouissance", content="jouissance"),
        Vertex("categorical_imperative", content="Kantian categorical imperative"),
        Vertex("sublimation", content="sublimation"),
        Vertex("Antigone", content="Antigone"),
        Vertex("between_two_deaths", content="between two deaths"),
        Vertex("Sade", content="Sade"),
        Vertex("ethics_psychoanalysis", content="ethics of psychoanalysis"),
        Vertex("beautiful", content="the beautiful (barrier to jouissance)"),
        Vertex("creation_ex_nihilo", content="creation ex nihilo"),
        Vertex("good_supreme", content="the sovereign good"),
    ]
    edges = [
        Edge("das_Ding", "real", EdgeType.DEPENDENCY),
        Edge("das_Ding", "pleasure_principle", EdgeType.NEGATION),
        Edge("jouissance", "das_Ding", EdgeType.DEPENDENCY),
        Edge("jouissance", "pleasure_principle", EdgeType.NEGATION),
        Edge("categorical_imperative", "jouissance", EdgeType.REFERENCE),
        Edge("Sade", "categorical_imperative", EdgeType.NEGATION),
        Edge("Sade", "jouissance", EdgeType.DEPENDENCY),
        Edge("sublimation", "das_Ding", EdgeType.DEPENDENCY),
        Edge("sublimation", "creation_ex_nihilo", EdgeType.DEPENDENCY),
        Edge("Antigone", "between_two_deaths", EdgeType.DEPENDENCY),
        Edge("Antigone", "desire", EdgeType.REFERENCE),
        Edge("between_two_deaths", "death_drive_sem2", EdgeType.DEPENDENCY),
        Edge("ethics_psychoanalysis", "desire", EdgeType.DEPENDENCY),
        Edge("ethics_psychoanalysis", "good_supreme", EdgeType.NEGATION),
        Edge("beautiful", "jouissance", EdgeType.NEGATION),
        Edge("beautiful", "sublimation", EdgeType.DEPENDENCY),
        Edge("good_supreme", "jouissance", EdgeType.NEGATION),
    ]
    return "VII: The Ethics of Psychoanalysis (1959-60)", vertices, edges


def sem_VIII():
    """Seminar VIII: Transference (1960-61).

    Transference as love, agalma, Socrates-Alcibiades.
    """
    vertices = [
        Vertex("transference_love", content="transference love"),
        Vertex("agalma", content="agalma (hidden treasure)"),
        Vertex("Socrates", content="Socrates"),
        Vertex("Alcibiades", content="Alcibiades"),
        Vertex("desire_analyst", content="desire of the analyst"),
        Vertex("eromenos", content="eromenos (beloved)"),
        Vertex("erastes", content="erastes (lover)"),
        Vertex("substitution_love", content="metaphor of love"),
    ]
    edges = [
        Edge("transference_love", "transference", EdgeType.SUBLATION),
        Edge("transference_love", "desire", EdgeType.DEPENDENCY),
        Edge("agalma", "object_of_desire", EdgeType.DEPENDENCY),
        Edge("agalma", "Socrates", EdgeType.REFERENCE),
        Edge("Alcibiades", "Socrates", EdgeType.DEPENDENCY),
        Edge("Alcibiades", "agalma", EdgeType.DEPENDENCY),
        Edge("desire_analyst", "transference_love", EdgeType.DEPENDENCY),
        Edge("desire_analyst", "desire_of_Other", EdgeType.DEPENDENCY),
        Edge("eromenos", "erastes", EdgeType.NEGATION),
        Edge("substitution_love", "eromenos", EdgeType.DEPENDENCY),
        Edge("substitution_love", "erastes", EdgeType.DEPENDENCY),
        Edge("substitution_love", "transference_love", EdgeType.DEPENDENCY),
    ]
    return "VIII: Transference (1960-61)", vertices, edges


def sem_IX():
    """Seminar IX: Identification (1961-62).

    Trait unaire, topology of identification, the torus.
    """
    vertices = [
        Vertex("identification", content="identification"),
        Vertex("trait_unaire", content="unary trait (trait unaire)"),
        Vertex("topology_identification", content="topology of identification"),
        Vertex("torus", content="torus"),
        Vertex("demand_circle", content="circle of demand"),
        Vertex("desire_circle", content="circle of desire"),
        Vertex("regression_identification", content="regressive identification"),
        Vertex("hysterical_identification", content="hysterical identification"),
    ]
    edges = [
        Edge("identification", "signifier", EdgeType.DEPENDENCY),
        Edge("trait_unaire", "signifier", EdgeType.DEPENDENCY),
        Edge("trait_unaire", "identification", EdgeType.DEPENDENCY),
        Edge("trait_unaire", "ego_ideal", EdgeType.SUBLATION),
        Edge("topology_identification", "torus", EdgeType.DEPENDENCY),
        Edge("topology_identification", "identification", EdgeType.DEPENDENCY),
        Edge("torus", "demand_circle", EdgeType.DEPENDENCY),
        Edge("torus", "desire_circle", EdgeType.DEPENDENCY),
        Edge("demand_circle", "demand", EdgeType.DEPENDENCY),
        Edge("desire_circle", "desire", EdgeType.DEPENDENCY),
        Edge("regression_identification", "identification", EdgeType.DEPENDENCY),
        Edge("hysterical_identification", "identification", EdgeType.DEPENDENCY),
        Edge("hysterical_identification", "desire_of_Other", EdgeType.DEPENDENCY),
    ]
    return "IX: Identification (1961-62)", vertices, edges


def sem_X():
    """Seminar X: Anxiety (1962-63).

    Anxiety as signal of the real, objet a formalized.
    """
    vertices = [
        Vertex("anxiety", content="anxiety (angoisse)"),
        Vertex("objet_a", content="objet a"),
        Vertex("anxiety_signal", content="anxiety as signal"),
        Vertex("remainder", content="remainder (reste)"),
        Vertex("oral_object", content="oral object (breast)"),
        Vertex("anal_object", content="anal object"),
        Vertex("gaze_obj", content="the gaze (objet a)"),
        Vertex("voice_obj", content="the voice (objet a)"),
        Vertex("lack_of_lack", content="lack of lack"),
        Vertex("uncanny", content="the uncanny (Unheimlich)"),
    ]
    edges = [
        Edge("anxiety", "real", EdgeType.DEPENDENCY),
        Edge("anxiety", "lack_of_lack", EdgeType.DEPENDENCY),
        Edge("objet_a", "remainder", EdgeType.DEPENDENCY),
        Edge("objet_a", "object_of_desire", EdgeType.SUBLATION),
        Edge("objet_a", "fantasy", EdgeType.DEPENDENCY),
        Edge("anxiety_signal", "anxiety", EdgeType.DEPENDENCY),
        Edge("anxiety_signal", "objet_a", EdgeType.DEPENDENCY),
        Edge("remainder", "das_Ding", EdgeType.REFERENCE),
        Edge("oral_object", "objet_a", EdgeType.DEPENDENCY),
        Edge("anal_object", "objet_a", EdgeType.DEPENDENCY),
        Edge("gaze_obj", "objet_a", EdgeType.DEPENDENCY),
        Edge("voice_obj", "objet_a", EdgeType.DEPENDENCY),
        Edge("lack_of_lack", "castration", EdgeType.NEGATION),
        Edge("uncanny", "anxiety", EdgeType.DEPENDENCY),
        Edge("uncanny", "imaginary", EdgeType.DEPENDENCY),
    ]
    return "X: Anxiety (1962-63)", vertices, edges


def sem_XI():
    """Seminar XI: The Four Fundamental Concepts of Psychoanalysis (1964).

    The unconscious, repetition, transference, drive. Alienation/separation.
    """
    vertices = [
        Vertex("four_concepts", content="four fundamental concepts"),
        Vertex("unconscious_concept", content="the unconscious (concept)"),
        Vertex("repetition_concept", content="repetition (concept)"),
        Vertex("transference_concept", content="transference (concept)"),
        Vertex("drive_concept", content="the drive (Trieb)"),
        Vertex("alienation", content="alienation"),
        Vertex("separation", content="separation"),
        Vertex("vel_alienation", content="vel of alienation (or)"),
        Vertex("tuche", content="tuche (encounter with the real)"),
        Vertex("automaton", content="automaton (symbolic network)"),
        Vertex("lamella", content="lamella (myth of the drive)"),
        Vertex("subject_supposed_know", content="subject supposed to know"),
    ]
    edges = [
        Edge("four_concepts", "unconscious_concept", EdgeType.DEPENDENCY),
        Edge("four_concepts", "repetition_concept", EdgeType.DEPENDENCY),
        Edge("four_concepts", "transference_concept", EdgeType.DEPENDENCY),
        Edge("four_concepts", "drive_concept", EdgeType.DEPENDENCY),
        Edge("unconscious_concept", "signifier", EdgeType.DEPENDENCY),
        Edge("unconscious_concept", "Other_A", EdgeType.DEPENDENCY),
        Edge("repetition_concept", "repetition_automatism", EdgeType.SUBLATION),
        Edge("repetition_concept", "tuche", EdgeType.DEPENDENCY),
        Edge("repetition_concept", "automaton", EdgeType.DEPENDENCY),
        Edge("tuche", "real", EdgeType.DEPENDENCY),
        Edge("automaton", "symbolic", EdgeType.DEPENDENCY),
        Edge("tuche", "automaton", EdgeType.NEGATION),
        Edge("transference_concept", "transference_love", EdgeType.SUBLATION),
        Edge("transference_concept", "subject_supposed_know", EdgeType.DEPENDENCY),
        Edge("subject_supposed_know", "Other_A", EdgeType.DEPENDENCY),
        Edge("drive_concept", "objet_a", EdgeType.DEPENDENCY),
        Edge("drive_concept", "lamella", EdgeType.REFERENCE),
        Edge("alienation", "vel_alienation", EdgeType.DEPENDENCY),
        Edge("alienation", "Other_A", EdgeType.DEPENDENCY),
        Edge("separation", "alienation", EdgeType.NEGATION),
        Edge("separation", "objet_a", EdgeType.DEPENDENCY),
        Edge("lamella", "drive_concept", EdgeType.DEPENDENCY),
    ]
    return "XI: The Four Fundamental Concepts (1964)", vertices, edges


def sem_XII():
    """Seminar XII: Crucial Problems for Psychoanalysis (1964-65).

    Subject of science, Descartes, the cogito revisited.
    """
    vertices = [
        Vertex("crucial_problems", content="crucial problems for psychoanalysis"),
        Vertex("subject_of_science", content="subject of science"),
        Vertex("cogito", content="Cartesian cogito"),
        Vertex("subject_unconscious", content="subject of the unconscious"),
        Vertex("truth_subject", content="truth as cause for the subject"),
    ]
    edges = [
        Edge("crucial_problems", "four_concepts", EdgeType.DEPENDENCY),
        Edge("subject_of_science", "cogito", EdgeType.DEPENDENCY),
        Edge("subject_of_science", "subject_unconscious", EdgeType.NEGATION),
        Edge("subject_unconscious", "unconscious_concept", EdgeType.DEPENDENCY),
        Edge("subject_unconscious", "signifier", EdgeType.DEPENDENCY),
        Edge("cogito", "subject_decentered", EdgeType.NEGATION),
        Edge("truth_subject", "subject_unconscious", EdgeType.DEPENDENCY),
        Edge("truth_subject", "desire", EdgeType.REFERENCE),
    ]
    return "XII: Crucial Problems for Psychoanalysis (1964-65)", vertices, edges


def sem_XIII():
    """Seminar XIII: The Object of Psychoanalysis (1965-66).

    Gaze and voice as objet a, the Velazquez analysis.
    """
    vertices = [
        Vertex("object_psychoanalysis", content="object of psychoanalysis"),
        Vertex("gaze_concept", content="the gaze (concept)"),
        Vertex("voice_concept", content="the voice (concept)"),
        Vertex("Velazquez", content="Las Meninas (Velazquez)"),
        Vertex("anamorphosis", content="anamorphosis"),
        Vertex("perspective_subject", content="subject of perspective"),
    ]
    edges = [
        Edge("object_psychoanalysis", "objet_a", EdgeType.DEPENDENCY),
        Edge("gaze_concept", "gaze_obj", EdgeType.SUBLATION),
        Edge("gaze_concept", "objet_a", EdgeType.DEPENDENCY),
        Edge("voice_concept", "voice_obj", EdgeType.SUBLATION),
        Edge("voice_concept", "objet_a", EdgeType.DEPENDENCY),
        Edge("Velazquez", "gaze_concept", EdgeType.REFERENCE),
        Edge("anamorphosis", "gaze_concept", EdgeType.DEPENDENCY),
        Edge("anamorphosis", "real", EdgeType.REFERENCE),
        Edge("perspective_subject", "subject_of_science", EdgeType.REFERENCE),
        Edge("perspective_subject", "gaze_concept", EdgeType.DEPENDENCY),
    ]
    return "XIII: The Object of Psychoanalysis (1965-66)", vertices, edges


def sem_XIV():
    """Seminar XIV: The Logic of Phantasy (1966-67).

    Formalization of fantasy, the sexual act, repetition.
    """
    vertices = [
        Vertex("logic_phantasy", content="logic of phantasy"),
        Vertex("sexual_act", content="there is no sexual relation (anticipated)"),
        Vertex("fantasy_formula", content="fantasy formula ($<>a)"),
        Vertex("alienation_repetition", content="alienation-repetition structure"),
        Vertex("act_concept", content="the act (concept)"),
    ]
    edges = [
        Edge("logic_phantasy", "fantasy", EdgeType.SUBLATION),
        Edge("logic_phantasy", "objet_a", EdgeType.DEPENDENCY),
        Edge("sexual_act", "jouissance", EdgeType.DEPENDENCY),
        Edge("sexual_act", "castration", EdgeType.DEPENDENCY),
        Edge("fantasy_formula", "fantasy", EdgeType.DEPENDENCY),
        Edge("fantasy_formula", "objet_a", EdgeType.DEPENDENCY),
        Edge("alienation_repetition", "alienation", EdgeType.DEPENDENCY),
        Edge("alienation_repetition", "repetition_concept", EdgeType.DEPENDENCY),
        Edge("act_concept", "sexual_act", EdgeType.DEPENDENCY),
        Edge("act_concept", "desire", EdgeType.NEGATION),
    ]
    return "XIV: The Logic of Phantasy (1966-67)", vertices, edges


def sem_XV():
    """Seminar XV: The Psychoanalytic Act (1967-68).

    The analytic act, the pass, destitution of the subject.
    """
    vertices = [
        Vertex("psychoanalytic_act", content="the psychoanalytic act"),
        Vertex("pass_procedure", content="the pass (la passe)"),
        Vertex("destitution", content="subjective destitution"),
        Vertex("analyst_position", content="position of the analyst"),
        Vertex("analysand_to_analyst", content="passage from analysand to analyst"),
    ]
    edges = [
        Edge("psychoanalytic_act", "act_concept", EdgeType.SUBLATION),
        Edge("psychoanalytic_act", "transference_concept", EdgeType.DEPENDENCY),
        Edge("pass_procedure", "psychoanalytic_act", EdgeType.DEPENDENCY),
        Edge("pass_procedure", "analysand_to_analyst", EdgeType.DEPENDENCY),
        Edge("destitution", "psychoanalytic_act", EdgeType.DEPENDENCY),
        Edge("destitution", "fantasy", EdgeType.NEGATION),
        Edge("analyst_position", "desire_analyst", EdgeType.DEPENDENCY),
        Edge("analyst_position", "objet_a", EdgeType.DEPENDENCY),
        Edge("analysand_to_analyst", "destitution", EdgeType.DEPENDENCY),
    ]
    return "XV: The Psychoanalytic Act (1967-68)", vertices, edges


def sem_XVI():
    """Seminar XVI: From an Other to the other (1968-69).

    Surplus jouissance, the discourse, Marx and Freud.
    """
    vertices = [
        Vertex("surplus_jouissance", content="surplus jouissance (plus-de-jouir)"),
        Vertex("plus_de_jouir", content="plus-de-jouir (as objet a)"),
        Vertex("discourse_concept", content="discourse (concept)"),
        Vertex("surplus_value", content="surplus value (Mehrwert)"),
        Vertex("Other_barred", content="the barred Other (A-barred)"),
        Vertex("other_small", content="the other (small a)"),
        Vertex("knowledge_market", content="knowledge as market commodity"),
    ]
    edges = [
        Edge("surplus_jouissance", "jouissance", EdgeType.DEPENDENCY),
        Edge("surplus_jouissance", "objet_a", EdgeType.DEPENDENCY),
        Edge("plus_de_jouir", "surplus_jouissance", EdgeType.DEPENDENCY),
        Edge("plus_de_jouir", "surplus_value", EdgeType.REFERENCE),
        Edge("discourse_concept", "signifier", EdgeType.DEPENDENCY),
        Edge("discourse_concept", "Other_A", EdgeType.DEPENDENCY),
        Edge("surplus_value", "surplus_jouissance", EdgeType.REFERENCE),
        Edge("Other_barred", "Other_A", EdgeType.NEGATION),
        Edge("Other_barred", "jouissance", EdgeType.DEPENDENCY),
        Edge("other_small", "objet_a", EdgeType.REFERENCE),
        Edge("knowledge_market", "surplus_value", EdgeType.DEPENDENCY),
        Edge("knowledge_market", "discourse_concept", EdgeType.DEPENDENCY),
    ]
    return "XVI: From an Other to the other (1968-69)", vertices, edges


def sem_XVII():
    """Seminar XVII: The Other Side of Psychoanalysis (1969-70).

    The four discourses: master, university, hysteric, analyst.
    """
    vertices = [
        Vertex("four_discourses", content="four discourses"),
        Vertex("master_discourse", content="discourse of the master"),
        Vertex("university_discourse", content="discourse of the university"),
        Vertex("hysteric_discourse", content="discourse of the hysteric"),
        Vertex("analyst_discourse", content="discourse of the analyst"),
        Vertex("master_signifier_S1", content="master signifier (S1)"),
        Vertex("knowledge_S2", content="knowledge (S2)"),
        Vertex("surplus_jouissance_a", content="surplus jouissance (a) in discourse"),
        Vertex("divided_subject", content="divided subject ($)"),
        Vertex("truth_position", content="truth (position in discourse)"),
        Vertex("production_position", content="production (position in discourse)"),
    ]
    edges = [
        Edge("four_discourses", "discourse_concept", EdgeType.SUBLATION),
        Edge("master_discourse", "four_discourses", EdgeType.DEPENDENCY),
        Edge("university_discourse", "four_discourses", EdgeType.DEPENDENCY),
        Edge("hysteric_discourse", "four_discourses", EdgeType.DEPENDENCY),
        Edge("analyst_discourse", "four_discourses", EdgeType.DEPENDENCY),
        Edge("master_discourse", "master_signifier_S1", EdgeType.DEPENDENCY),
        Edge("master_discourse", "knowledge_S2", EdgeType.DEPENDENCY),
        Edge("university_discourse", "knowledge_S2", EdgeType.DEPENDENCY),
        Edge("university_discourse", "surplus_jouissance_a", EdgeType.DEPENDENCY),
        Edge("hysteric_discourse", "divided_subject", EdgeType.DEPENDENCY),
        Edge("hysteric_discourse", "master_signifier_S1", EdgeType.DEPENDENCY),
        Edge("analyst_discourse", "surplus_jouissance_a", EdgeType.DEPENDENCY),
        Edge("analyst_discourse", "divided_subject", EdgeType.DEPENDENCY),
        Edge("analyst_discourse", "analyst_position", EdgeType.REFERENCE),
        Edge("divided_subject", "signifier", EdgeType.DEPENDENCY),
        Edge("master_signifier_S1", "signifier", EdgeType.SUBLATION),
        Edge("truth_position", "four_discourses", EdgeType.DEPENDENCY),
        Edge("production_position", "four_discourses", EdgeType.DEPENDENCY),
        Edge("surplus_jouissance_a", "objet_a", EdgeType.DEPENDENCY),
        Edge("master_discourse", "master_signifier_S1", EdgeType.DEPENDENCY),
    ]
    return "XVII: The Other Side of Psychoanalysis (1969-70)", vertices, edges


def sem_XVIII():
    """Seminar XVIII: On a Discourse That Might Not Be a Semblance (1971).

    Semblance, lalangue, the impossible sexual relation.
    """
    vertices = [
        Vertex("semblance", content="semblance (semblant)"),
        Vertex("lalangue", content="lalangue"),
        Vertex("impossible_sexual_relation", content="there is no sexual relation"),
        Vertex("matheme", content="matheme"),
        Vertex("formulas_sexuation", content="formulas of sexuation (anticipated)"),
        Vertex("writing", content="writing (ecriture)"),
    ]
    edges = [
        Edge("semblance", "symbolic", EdgeType.DEPENDENCY),
        Edge("semblance", "real", EdgeType.NEGATION),
        Edge("lalangue", "signifier", EdgeType.SUBLATION),
        Edge("lalangue", "jouissance", EdgeType.DEPENDENCY),
        Edge("impossible_sexual_relation", "sexual_act", EdgeType.SUBLATION),
        Edge("impossible_sexual_relation", "real", EdgeType.DEPENDENCY),
        Edge("impossible_sexual_relation", "castration", EdgeType.REFERENCE),
        Edge("matheme", "writing", EdgeType.DEPENDENCY),
        Edge("matheme", "signifier", EdgeType.DEPENDENCY),
        Edge("formulas_sexuation", "impossible_sexual_relation", EdgeType.DEPENDENCY),
        Edge("writing", "semblance", EdgeType.NEGATION),
    ]
    return "XVIII: On a Discourse That Might Not Be a Semblance (1971)", vertices, edges


def sem_XIX():
    """Seminar XIX: ...ou pire / ...or Worse (1971-72).

    Yad'lun, existence, the One.
    """
    vertices = [
        Vertex("Yad_lun", content="Yad'lun (there is One)"),
        Vertex("existence_sem19", content="existence (vs essence)"),
        Vertex("one", content="the One"),
        Vertex("all_not_all", content="all / not-all (anticipated)"),
        Vertex("universe_discourse", content="there is no universe of discourse"),
    ]
    edges = [
        Edge("Yad_lun", "one", EdgeType.DEPENDENCY),
        Edge("Yad_lun", "signifier", EdgeType.DEPENDENCY),
        Edge("existence_sem19", "Yad_lun", EdgeType.DEPENDENCY),
        Edge("existence_sem19", "real", EdgeType.DEPENDENCY),
        Edge("one", "master_signifier_S1", EdgeType.REFERENCE),
        Edge("all_not_all", "formulas_sexuation", EdgeType.DEPENDENCY),
        Edge("all_not_all", "impossible_sexual_relation", EdgeType.DEPENDENCY),
        Edge("universe_discourse", "Other_barred", EdgeType.DEPENDENCY),
        Edge("universe_discourse", "all_not_all", EdgeType.REFERENCE),
    ]
    return "XIX: ...or Worse (1971-72)", vertices, edges


def sem_XX():
    """Seminar XX: Encore (1972-73).

    Feminine jouissance, not-all, Borromean, love.
    """
    vertices = [
        Vertex("feminine_jouissance", content="feminine jouissance (Other jouissance)"),
        Vertex("not_all", content="not-all (pas-tout)"),
        Vertex("phallic_jouissance", content="phallic jouissance"),
        Vertex("borromean", content="Borromean knot (introduced)"),
        Vertex("love_sem20", content="love (Encore)"),
        Vertex("sexuation_formulas", content="formulas of sexuation"),
        Vertex("masculine_side", content="masculine side (all/exception)"),
        Vertex("feminine_side", content="feminine side (not-all)"),
        Vertex("God_jouissance", content="God and jouissance"),
        Vertex("mystic_jouissance", content="mystic jouissance"),
    ]
    edges = [
        Edge("feminine_jouissance", "jouissance", EdgeType.SUBLATION),
        Edge("feminine_jouissance", "not_all", EdgeType.DEPENDENCY),
        Edge("feminine_jouissance", "phallic_jouissance", EdgeType.NEGATION),
        Edge("not_all", "all_not_all", EdgeType.SUBLATION),
        Edge("phallic_jouissance", "castration", EdgeType.DEPENDENCY),
        Edge("phallic_jouissance", "symbolic_phallus", EdgeType.DEPENDENCY),
        Edge("borromean", "imaginary", EdgeType.DEPENDENCY),
        Edge("borromean", "symbolic", EdgeType.DEPENDENCY),
        Edge("borromean", "real", EdgeType.DEPENDENCY),
        Edge("love_sem20", "transference_love", EdgeType.REFERENCE),
        Edge("love_sem20", "impossible_sexual_relation", EdgeType.DEPENDENCY),
        Edge("sexuation_formulas", "formulas_sexuation", EdgeType.SUBLATION),
        Edge("sexuation_formulas", "masculine_side", EdgeType.DEPENDENCY),
        Edge("sexuation_formulas", "feminine_side", EdgeType.DEPENDENCY),
        Edge("masculine_side", "phallic_jouissance", EdgeType.DEPENDENCY),
        Edge("feminine_side", "not_all", EdgeType.DEPENDENCY),
        Edge("feminine_side", "feminine_jouissance", EdgeType.DEPENDENCY),
        Edge("God_jouissance", "feminine_jouissance", EdgeType.REFERENCE),
        Edge("mystic_jouissance", "feminine_jouissance", EdgeType.DEPENDENCY),
        Edge("mystic_jouissance", "God_jouissance", EdgeType.REFERENCE),
    ]
    return "XX: Encore (1972-73)", vertices, edges


def sem_XXI():
    """Seminar XXI: Les non-dupes errent / The Names-of-the-Father (1973-74).

    Pluralization of the Name-of-the-Father, non-dupes err.
    """
    vertices = [
        Vertex("Names_of_Father", content="Names-of-the-Father (plural)"),
        Vertex("non_dupes_errent", content="les non-dupes errent (non-dupes err)"),
        Vertex("nomination", content="nomination"),
        Vertex("symptom_as_name", content="symptom as name"),
        Vertex("real_symbolic_imaginary_knot", content="R.S.I. knotting (anticipated)"),
    ]
    edges = [
        Edge("Names_of_Father", "Name_of_Father", EdgeType.SUBLATION),
        Edge("Names_of_Father", "borromean", EdgeType.DEPENDENCY),
        Edge("non_dupes_errent", "Names_of_Father", EdgeType.REFERENCE),
        Edge("nomination", "Names_of_Father", EdgeType.DEPENDENCY),
        Edge("nomination", "signifier", EdgeType.DEPENDENCY),
        Edge("symptom_as_name", "nomination", EdgeType.DEPENDENCY),
        Edge("symptom_as_name", "formations_unconscious", EdgeType.REFERENCE),
        Edge("real_symbolic_imaginary_knot", "borromean", EdgeType.DEPENDENCY),
    ]
    return "XXI: The Names-of-the-Father / Les non-dupes errent (1973-74)", vertices, edges


def sem_XXII():
    """Seminar XXII: R.S.I. (1974-75).

    Borromean knot formalized, Real-Symbolic-Imaginary topology.
    """
    vertices = [
        Vertex("RSI", content="R.S.I."),
        Vertex("borromean_knot", content="Borromean knot (formalized)"),
        Vertex("imaginary_ring", content="imaginary ring"),
        Vertex("symbolic_ring", content="symbolic ring"),
        Vertex("real_ring", content="real ring"),
        Vertex("sense_jouis_sense", content="jouis-sens (enjoy-meant)"),
        Vertex("inhibition_symptom_anxiety", content="inhibition, symptom, anxiety (topologized)"),
    ]
    edges = [
        Edge("RSI", "borromean", EdgeType.SUBLATION),
        Edge("RSI", "borromean_knot", EdgeType.DEPENDENCY),
        Edge("borromean_knot", "imaginary_ring", EdgeType.DEPENDENCY),
        Edge("borromean_knot", "symbolic_ring", EdgeType.DEPENDENCY),
        Edge("borromean_knot", "real_ring", EdgeType.DEPENDENCY),
        Edge("imaginary_ring", "imaginary", EdgeType.SUBLATION),
        Edge("symbolic_ring", "symbolic", EdgeType.SUBLATION),
        Edge("real_ring", "real", EdgeType.SUBLATION),
        Edge("sense_jouis_sense", "jouissance", EdgeType.DEPENDENCY),
        Edge("sense_jouis_sense", "lalangue", EdgeType.REFERENCE),
        Edge("inhibition_symptom_anxiety", "anxiety", EdgeType.REFERENCE),
        Edge("inhibition_symptom_anxiety", "borromean_knot", EdgeType.DEPENDENCY),
        Edge("real_symbolic_imaginary_knot", "borromean_knot", EdgeType.DEPENDENCY),
    ]
    return "XXII: R.S.I. (1974-75)", vertices, edges


def sem_XXIII():
    """Seminar XXIII: The Sinthome (1975-76).

    The sinthome, Joyce, the fourth ring, reparation of the knot.
    """
    vertices = [
        Vertex("sinthome", content="sinthome"),
        Vertex("Joyce", content="Joyce"),
        Vertex("fourth_ring", content="fourth ring"),
        Vertex("reparation", content="reparation of the knot"),
        Vertex("ego_sinthome", content="ego as sinthome (Joyce)"),
        Vertex("symptom_sinthome", content="symptom -> sinthome"),
        Vertex("epiphany", content="epiphany (Joyce)"),
        Vertex("knot_error", content="knotting error"),
    ]
    edges = [
        Edge("sinthome", "borromean_knot", EdgeType.SUBLATION),
        Edge("sinthome", "fourth_ring", EdgeType.DEPENDENCY),
        Edge("sinthome", "symptom_as_name", EdgeType.SUBLATION),
        Edge("Joyce", "sinthome", EdgeType.REFERENCE),
        Edge("Joyce", "ego_sinthome", EdgeType.REFERENCE),
        Edge("fourth_ring", "borromean_knot", EdgeType.DEPENDENCY),
        Edge("reparation", "knot_error", EdgeType.NEGATION),
        Edge("reparation", "sinthome", EdgeType.DEPENDENCY),
        Edge("ego_sinthome", "ego", EdgeType.SUBLATION),
        Edge("ego_sinthome", "sinthome", EdgeType.DEPENDENCY),
        Edge("symptom_sinthome", "sinthome", EdgeType.DEPENDENCY),
        Edge("symptom_sinthome", "formations_unconscious", EdgeType.SUBLATION),
        Edge("epiphany", "Joyce", EdgeType.DEPENDENCY),
        Edge("epiphany", "jouissance", EdgeType.REFERENCE),
        Edge("knot_error", "borromean_knot", EdgeType.DEPENDENCY),
        Edge("knot_error", "foreclosure", EdgeType.REFERENCE),
    ]
    return "XXIII: The Sinthome (1975-76)", vertices, edges


# ---------------------------------------------------------------------------
# Full seminar list in order
# ---------------------------------------------------------------------------

ALL_SEMINARS = [
    sem_I,
    sem_II,
    sem_III,
    sem_IV,
    sem_V,
    sem_VI,
    sem_VII,
    sem_VIII,
    sem_IX,
    sem_X,
    sem_XI,
    sem_XII,
    sem_XIII,
    sem_XIV,
    sem_XV,
    sem_XVI,
    sem_XVII,
    sem_XVIII,
    sem_XIX,
    sem_XX,
    sem_XXI,
    sem_XXII,
    sem_XXIII,
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_seminars = len(ALL_SEMINARS)
    print("=" * 70)
    print("LACAN SEMINARS I-XXIII — FULL INCREMENTAL TRAVERSAL")
    print(f"{total_seminars} seminars, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    seminar_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for sem_idx, sem_fn in enumerate(ALL_SEMINARS):
        seminar_name, sem_vertices, sem_edges = sem_fn()
        print(f"\n{'─' * 60}")
        print(f"Seminar {sem_idx + 1}/{total_seminars}: {seminar_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in sem_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in sem_edges:
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
        steps_this_seminar = 0
        max_steps = 2000  # safety cap per seminar

        while stable_count < 50 and steps_this_seminar < max_steps:
            engine.run_step()
            steps_this_seminar += 1
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

        print(f"  Steps to digest: {steps_this_seminar} {'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} (delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        # Count operations in this seminar's digestion
        chapter_logs = engine.logs[-steps_this_seminar:] if steps_this_seminar > 0 else []
        op_counts = {}
        for log in chapter_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record
        result = {
            "seminar": seminar_name,
            "seminar_index": sem_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_seminar": blocked_count,
            "steps_to_digest": steps_this_seminar,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        seminar_results.append(result)

        beta_1_curve.append({
            "seminar_index": sem_idx + 1,
            "seminar_name": seminar_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        # Update graph reference for next seminar injection
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
    print(f"\n  beta_1 growth curve (by seminar):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    Sem{entry['seminar_index']:3d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"           {entry['seminar_name']}")

    # Biggest jumps
    deltas = [(r["seminar"], r["delta_beta_1"], r["seminar_index"]) for r in seminar_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by seminar:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    Sem{idx:3d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by seminar:")
    for r in seminar_results:
        print(f"    Sem{r['seminar_index']:3d} {r['seminar']}: "
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

    # Sublation density (distinctive for Lacan's progressive reworking)
    sub_count = type_counts.get("sublation", 0)
    sub_density = sub_count / final_e if final_e > 0 else 0
    print(f"  Sublation density: {sub_density:.4f} ({sub_count}/{final_e})")

    # ---------------------------------------------------------------------------
    # Save
    # ---------------------------------------------------------------------------
    output = {
        "experiment": "lacan_seminars_incremental",
        "source": "Lacan, Seminars I-XXIII (1953-1976)",
        "methodology": "manual conceptual complex per seminar, incremental injection, "
                        "digestion until 50-step beta_1 stability",
        "seminar_results": seminar_results,
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
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_lacan_seminars.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
