"""Lacan Full Corpus — Écrits + Seminars I-XXIII + other texts, merged chronologically.

Previous experiments ran Écrits (16 papers, final β₁=50) and Seminars (23 volumes,
final β₁=112) separately. This experiment merges all texts into a single incremental
timeline ordered by year of composition/delivery, injecting into one shared complex.

Where a year has both an Écrits paper and a seminar (e.g. 1953: Function of Speech +
Seminar I), they are injected in publication/delivery order within that year.

~45 entries total: 16 Écrits + 23 Seminars + Television (1973) + Radiophonie (1970)
+ Proposition of 9 October 1967 + L'Étourdit (1972) + Joyce le Symptôme (1975)
+ Lituraterre (1971).

Shared vertex IDs connect across all texts — modeling the unified evolution of
Lacan's conceptual apparatus from 1936 to 1976.
"""

from __future__ import annotations

import json
import sys

sys.path.insert(0, ".")

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain


# ===========================================================================
# ÉCRITS paper encodings (from experiment_lacan_ecrits.py)
# ===========================================================================

def ecrits_01_beyond_reality_principle():
    """1936: Beyond the Reality Principle."""
    vertices = [
        Vertex("reality_principle", content="Freudian reality principle"),
        Vertex("pleasure_principle", content="pleasure principle"),
        Vertex("imago", content="imago — precipitate of imaginary relation"),
        Vertex("identification", content="identification with the image"),
        Vertex("psychic_reality", content="psychic reality vs external reality"),
        Vertex("associationism", content="associationist psychology"),
        Vertex("imaginary", content="the imaginary register"),
        Vertex("desire", content="desire"),
        Vertex("ego", content="ego — moi"),
        Vertex("narcissism", content="narcissism — libidinal investment of ego"),
    ]
    edges = [
        Edge("reality_principle", "pleasure_principle", EdgeType.NEGATION),
        Edge("imago", "identification", EdgeType.DEPENDENCY),
        Edge("identification", "ego", EdgeType.DEPENDENCY),
        Edge("ego", "narcissism", EdgeType.DEPENDENCY),
        Edge("narcissism", "imaginary", EdgeType.DEPENDENCY),
        Edge("psychic_reality", "reality_principle", EdgeType.NEGATION),
        Edge("associationism", "psychic_reality", EdgeType.NEGATION),
        Edge("imago", "imaginary", EdgeType.DEPENDENCY),
        Edge("desire", "pleasure_principle", EdgeType.REFERENCE),
        Edge("desire", "imaginary", EdgeType.REFERENCE),
    ]
    return ("É01 — 1936: Beyond the Reality Principle", vertices, edges)


def ecrits_02_logical_time():
    """1945: Logical Time and the Assertion of Anticipated Certainty."""
    vertices = [
        Vertex("logical_time", content="logical time — not chronological"),
        Vertex("instant_of_glance", content="instant of the glance"),
        Vertex("time_for_comprehending", content="time for comprehending"),
        Vertex("moment_of_concluding", content="moment of concluding"),
        Vertex("anticipated_certainty", content="anticipated certainty"),
        Vertex("intersubjectivity", content="intersubjective logic"),
        Vertex("subject", content="subject — sujet"),
        Vertex("other", content="other — autre (lowercase)"),
        Vertex("haste", content="haste — precipitation in concluding"),
    ]
    edges = [
        Edge("logical_time", "instant_of_glance", EdgeType.DEPENDENCY),
        Edge("logical_time", "time_for_comprehending", EdgeType.DEPENDENCY),
        Edge("logical_time", "moment_of_concluding", EdgeType.DEPENDENCY),
        Edge("instant_of_glance", "time_for_comprehending", EdgeType.DEPENDENCY),
        Edge("time_for_comprehending", "moment_of_concluding", EdgeType.DEPENDENCY),
        Edge("moment_of_concluding", "anticipated_certainty", EdgeType.DEPENDENCY),
        Edge("anticipated_certainty", "haste", EdgeType.DEPENDENCY),
        Edge("intersubjectivity", "subject", EdgeType.DEPENDENCY),
        Edge("intersubjectivity", "other", EdgeType.DEPENDENCY),
        Edge("subject", "other", EdgeType.REFERENCE),
        Edge("haste", "subject", EdgeType.REFERENCE),
    ]
    return ("É02 — 1945: Logical Time", vertices, edges)


def ecrits_03_aggressiveness():
    """1948: Aggressiveness in Psychoanalysis."""
    vertices = [
        Vertex("aggressivity", content="aggressivity — structural, not instinctual"),
        Vertex("mirror_identification", content="identification with specular image"),
        Vertex("fragmented_body", content="corps morcelé — fragmented body image"),
        Vertex("jealousy", content="jealousy — transitivism"),
        Vertex("transitivism", content="transitivism — child attributes own acts to other"),
        Vertex("imago_body", content="imago of the body in pieces"),
        Vertex("tension", content="aggressive tension of ego-other pair"),
    ]
    edges = [
        Edge("aggressivity", "narcissism", EdgeType.DEPENDENCY),
        Edge("aggressivity", "mirror_identification", EdgeType.DEPENDENCY),
        Edge("mirror_identification", "ego", EdgeType.DEPENDENCY),
        Edge("mirror_identification", "imaginary", EdgeType.REFERENCE),
        Edge("fragmented_body", "aggressivity", EdgeType.DEPENDENCY),
        Edge("fragmented_body", "imago_body", EdgeType.DEPENDENCY),
        Edge("imago_body", "imago", EdgeType.REFERENCE),
        Edge("transitivism", "jealousy", EdgeType.DEPENDENCY),
        Edge("transitivism", "identification", EdgeType.REFERENCE),
        Edge("tension", "ego", EdgeType.DEPENDENCY),
        Edge("tension", "other", EdgeType.DEPENDENCY),
        Edge("aggressivity", "ego", EdgeType.NEGATION),
    ]
    return ("É03 — 1948: Aggressiveness in Psychoanalysis", vertices, edges)


def ecrits_04_mirror_stage():
    """1949: The Mirror Stage as Formative of the I Function."""
    vertices = [
        Vertex("mirror_stage", content="mirror stage — stade du miroir"),
        Vertex("specular_image", content="specular image — i(a)"),
        Vertex("jubilation", content="jubilant assumption of image"),
        Vertex("gestalt", content="gestalt — total form of the body"),
        Vertex("meconnaissance", content="méconnaissance — structural misrecognition"),
        Vertex("ideal_ego", content="ideal ego — Idealich — moi-idéal"),
        Vertex("alienation", content="alienation in the image"),
        Vertex("orthopaedic_totality", content="orthopaedic totality — armour"),
        Vertex("motor_prematurity", content="specific prematurity of birth"),
    ]
    edges = [
        Edge("mirror_stage", "specular_image", EdgeType.DEPENDENCY),
        Edge("specular_image", "jubilation", EdgeType.DEPENDENCY),
        Edge("specular_image", "gestalt", EdgeType.DEPENDENCY),
        Edge("gestalt", "fragmented_body", EdgeType.NEGATION),
        Edge("mirror_stage", "ego", EdgeType.DEPENDENCY),
        Edge("ego", "meconnaissance", EdgeType.DEPENDENCY),
        Edge("ego", "ideal_ego", EdgeType.DEPENDENCY),
        Edge("ideal_ego", "identification", EdgeType.REFERENCE),
        Edge("alienation", "mirror_stage", EdgeType.DEPENDENCY),
        Edge("alienation", "imaginary", EdgeType.REFERENCE),
        Edge("orthopaedic_totality", "gestalt", EdgeType.DEPENDENCY),
        Edge("motor_prematurity", "mirror_stage", EdgeType.DEPENDENCY),
        Edge("meconnaissance", "imaginary", EdgeType.DEPENDENCY),
        Edge("specular_image", "narcissism", EdgeType.REFERENCE),
    ]
    return ("É04 — 1949: The Mirror Stage", vertices, edges)


def ecrits_05_function_and_field():
    """1953: Function and Field of Speech and Language (Rome Discourse)."""
    vertices = [
        Vertex("symbolic", content="the symbolic register — order of the signifier"),
        Vertex("real", content="the real — what resists symbolization"),
        Vertex("signifier", content="signifier — signifiant"),
        Vertex("signified", content="signified — signifié"),
        Vertex("full_speech", content="full speech — parole pleine"),
        Vertex("empty_speech", content="empty speech — parole vide"),
        Vertex("Other", content="big Other — Autre — locus of the signifier"),
        Vertex("law", content="the law — symbolic prohibition"),
        Vertex("Name_of_Father", content="Name-of-the-Father — Nom-du-Père"),
        Vertex("unconscious_structured", content="the unconscious is structured like a language"),
        Vertex("truth", content="truth — emerges in speech"),
        Vertex("history", content="history — realization of subject in speech"),
    ]
    edges = [
        Edge("symbolic", "imaginary", EdgeType.NEGATION),
        Edge("symbolic", "real", EdgeType.NEGATION),
        Edge("imaginary", "real", EdgeType.NEGATION),
        Edge("signifier", "signified", EdgeType.NEGATION),
        Edge("signifier", "symbolic", EdgeType.DEPENDENCY),
        Edge("full_speech", "empty_speech", EdgeType.NEGATION),
        Edge("full_speech", "truth", EdgeType.DEPENDENCY),
        Edge("full_speech", "subject", EdgeType.DEPENDENCY),
        Edge("empty_speech", "ego", EdgeType.REFERENCE),
        Edge("Other", "symbolic", EdgeType.DEPENDENCY),
        Edge("Other", "law", EdgeType.DEPENDENCY),
        Edge("Name_of_Father", "law", EdgeType.DEPENDENCY),
        Edge("Name_of_Father", "symbolic", EdgeType.REFERENCE),
        Edge("unconscious_structured", "signifier", EdgeType.DEPENDENCY),
        Edge("unconscious_structured", "symbolic", EdgeType.REFERENCE),
        Edge("desire", "Other", EdgeType.REFERENCE),
        Edge("history", "subject", EdgeType.DEPENDENCY),
        Edge("history", "full_speech", EdgeType.REFERENCE),
    ]
    return ("É05 — 1953: Function and Field of Speech and Language", vertices, edges)


def ecrits_06_freudian_thing():
    """1953: The Freudian Thing."""
    vertices = [
        Vertex("freudian_thing", content="the Freudian thing — Freud's discovery"),
        Vertex("symptom", content="symptom — truth of the subject"),
        Vertex("ego_psychology", content="ego psychology — American deviation"),
        Vertex("resistance", content="resistance — of the ego"),
        Vertex("return_to_freud", content="return to Freud"),
    ]
    edges = [
        Edge("freudian_thing", "truth", EdgeType.DEPENDENCY),
        Edge("symptom", "signifier", EdgeType.DEPENDENCY),
        Edge("symptom", "truth", EdgeType.DEPENDENCY),
        Edge("ego_psychology", "ego", EdgeType.REFERENCE),
        Edge("ego_psychology", "meconnaissance", EdgeType.DEPENDENCY),
        Edge("return_to_freud", "ego_psychology", EdgeType.NEGATION),
        Edge("return_to_freud", "freudian_thing", EdgeType.DEPENDENCY),
        Edge("resistance", "ego", EdgeType.DEPENDENCY),
        Edge("resistance", "imaginary", EdgeType.REFERENCE),
        Edge("symptom", "subject", EdgeType.REFERENCE),
    ]
    return ("É06 — 1953: The Freudian Thing", vertices, edges)


def ecrits_07_purloined_letter():
    """1955: Seminar on The Purloined Letter."""
    vertices = [
        Vertex("letter", content="the letter — purloined letter as signifier"),
        Vertex("letter_always_arrives", content="a letter always arrives at its destination"),
        Vertex("intersubjective_position", content="intersubjective position determined by signifier"),
        Vertex("signifying_chain", content="signifying chain — autonomy of signifier"),
        Vertex("repetition_automatism", content="repetition automatism — insistence of chain"),
        Vertex("minister", content="the Minister — holds the letter"),
        Vertex("queen", content="the Queen — letter stolen from her"),
        Vertex("police", content="the Police — cannot find the letter"),
    ]
    edges = [
        Edge("letter", "signifier", EdgeType.DEPENDENCY),
        Edge("letter_always_arrives", "letter", EdgeType.DEPENDENCY),
        Edge("letter_always_arrives", "symbolic", EdgeType.REFERENCE),
        Edge("intersubjective_position", "signifier", EdgeType.DEPENDENCY),
        Edge("intersubjective_position", "subject", EdgeType.DEPENDENCY),
        Edge("signifying_chain", "signifier", EdgeType.DEPENDENCY),
        Edge("signifying_chain", "repetition_automatism", EdgeType.DEPENDENCY),
        Edge("repetition_automatism", "signifier", EdgeType.REFERENCE),
        Edge("minister", "intersubjective_position", EdgeType.REFERENCE),
        Edge("queen", "intersubjective_position", EdgeType.REFERENCE),
        Edge("police", "imaginary", EdgeType.REFERENCE),
        Edge("police", "intersubjective_position", EdgeType.NEGATION),
    ]
    return ("É07 — 1955: Seminar on The Purloined Letter", vertices, edges)


def ecrits_08_agency_of_letter():
    """1957: The Agency of the Letter in the Unconscious."""
    vertices = [
        Vertex("algorithm_S_over_s", content="algorithm S/s — signifier over signified"),
        Vertex("bar", content="the bar — resistance of signification"),
        Vertex("metaphor", content="metaphor — one signifier substitutes for another"),
        Vertex("metonymy", content="metonymy — word-to-word connection"),
        Vertex("condensation", content="condensation — Verdichtung"),
        Vertex("displacement", content="displacement — Verschiebung"),
        Vertex("point_de_capiton", content="point de capiton — quilting point"),
    ]
    edges = [
        Edge("algorithm_S_over_s", "signifier", EdgeType.DEPENDENCY),
        Edge("algorithm_S_over_s", "signified", EdgeType.DEPENDENCY),
        Edge("algorithm_S_over_s", "bar", EdgeType.DEPENDENCY),
        Edge("bar", "signifier", EdgeType.REFERENCE),
        Edge("metaphor", "signifier", EdgeType.DEPENDENCY),
        Edge("metaphor", "condensation", EdgeType.DEPENDENCY),
        Edge("metonymy", "signifier", EdgeType.DEPENDENCY),
        Edge("metonymy", "displacement", EdgeType.DEPENDENCY),
        Edge("metonymy", "desire", EdgeType.REFERENCE),
        Edge("metaphor", "metonymy", EdgeType.NEGATION),
        Edge("metaphor", "Name_of_Father", EdgeType.REFERENCE),
        Edge("condensation", "displacement", EdgeType.NEGATION),
        Edge("point_de_capiton", "signifier", EdgeType.DEPENDENCY),
        Edge("point_de_capiton", "signified", EdgeType.DEPENDENCY),
        Edge("metaphor", "symptom", EdgeType.REFERENCE),
    ]
    return ("É08 — 1957: The Agency of the Letter", vertices, edges)


def ecrits_09_direction_of_treatment():
    """1958: The Direction of the Treatment."""
    vertices = [
        Vertex("demand", content="demand — demande"),
        Vertex("need", content="need — besoin"),
        Vertex("frustration", content="frustration — imaginary lack of real object"),
        Vertex("privation", content="privation — real lack of symbolic object"),
        Vertex("castration", content="castration — symbolic lack of imaginary object"),
        Vertex("desire_of_analyst", content="desire of the analyst"),
        Vertex("transference", content="transference — subject-supposed-to-know"),
    ]
    edges = [
        Edge("desire", "demand", EdgeType.NEGATION),
        Edge("demand", "need", EdgeType.NEGATION),
        Edge("desire", "need", EdgeType.SUBLATION),
        Edge("desire", "Other", EdgeType.DEPENDENCY),
        Edge("frustration", "imaginary", EdgeType.DEPENDENCY),
        Edge("privation", "real", EdgeType.DEPENDENCY),
        Edge("castration", "symbolic", EdgeType.DEPENDENCY),
        Edge("frustration", "privation", EdgeType.NEGATION),
        Edge("privation", "castration", EdgeType.NEGATION),
        Edge("desire_of_analyst", "desire", EdgeType.DEPENDENCY),
        Edge("desire_of_analyst", "transference", EdgeType.DEPENDENCY),
        Edge("transference", "subject", EdgeType.DEPENDENCY),
        Edge("transference", "Other", EdgeType.REFERENCE),
    ]
    return ("É09 — 1958: The Direction of the Treatment", vertices, edges)


def ecrits_10_signification_of_phallus():
    """1958: The Signification of the Phallus."""
    vertices = [
        Vertex("phallus", content="phallus — signifier of desire"),
        Vertex("being_phallus", content="being the phallus — feminine position"),
        Vertex("having_phallus", content="having the phallus — masculine position"),
        Vertex("Bedeutung", content="Bedeutung — signification as such"),
        Vertex("lack", content="lack — manque — constitutive"),
    ]
    edges = [
        Edge("phallus", "signifier", EdgeType.DEPENDENCY),
        Edge("phallus", "desire", EdgeType.DEPENDENCY),
        Edge("phallus", "castration", EdgeType.DEPENDENCY),
        Edge("being_phallus", "having_phallus", EdgeType.NEGATION),
        Edge("being_phallus", "imaginary", EdgeType.REFERENCE),
        Edge("having_phallus", "symbolic", EdgeType.REFERENCE),
        Edge("Bedeutung", "phallus", EdgeType.DEPENDENCY),
        Edge("Bedeutung", "signifier", EdgeType.REFERENCE),
        Edge("lack", "desire", EdgeType.DEPENDENCY),
        Edge("lack", "castration", EdgeType.REFERENCE),
        Edge("phallus", "meconnaissance", EdgeType.REFERENCE),
    ]
    return ("É10 — 1958: The Signification of the Phallus", vertices, edges)


def ecrits_11_question_prior_to_treatment():
    """1958: On a Question Prior to Any Treatment of Psychosis."""
    vertices = [
        Vertex("foreclosure", content="foreclosure — Verwerfung — forclusion"),
        Vertex("paternal_metaphor", content="paternal metaphor — substitution of NdP for DdM"),
        Vertex("psychosis", content="psychosis — failure of paternal metaphor"),
        Vertex("hallucination", content="hallucination — return in the real"),
        Vertex("delusion", content="delusion — attempted restitution"),
        Vertex("schema_L", content="Schema L — imaginary-symbolic axes"),
        Vertex("schema_R", content="Schema R — field of reality"),
    ]
    edges = [
        Edge("foreclosure", "Name_of_Father", EdgeType.NEGATION),
        Edge("foreclosure", "psychosis", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "Name_of_Father", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "metaphor", EdgeType.DEPENDENCY),
        Edge("paternal_metaphor", "phallus", EdgeType.DEPENDENCY),
        Edge("psychosis", "paternal_metaphor", EdgeType.NEGATION),
        Edge("hallucination", "foreclosure", EdgeType.DEPENDENCY),
        Edge("hallucination", "real", EdgeType.DEPENDENCY),
        Edge("delusion", "hallucination", EdgeType.REFERENCE),
        Edge("delusion", "symbolic", EdgeType.REFERENCE),
        Edge("schema_L", "imaginary", EdgeType.DEPENDENCY),
        Edge("schema_L", "symbolic", EdgeType.DEPENDENCY),
        Edge("schema_R", "schema_L", EdgeType.DEPENDENCY),
        Edge("schema_R", "real", EdgeType.REFERENCE),
    ]
    return ("É11 — 1958: On a Question Prior to Any Treatment of Psychosis", vertices, edges)


def ecrits_12_subversion_of_subject():
    """1960: The Subversion of the Subject and the Dialectic of Desire."""
    vertices = [
        Vertex("graph_of_desire", content="graph of desire — completed graph"),
        Vertex("che_vuoi", content="Che vuoi? — what do you want?"),
        Vertex("fantasy", content="fantasy — $ ◇ a"),
        Vertex("objet_a", content="objet petit a — object-cause of desire"),
        Vertex("vel_of_alienation", content="vel of alienation — forced choice"),
        Vertex("separation", content="separation — second operation of subject"),
        Vertex("drive", content="drive — pulsion — Trieb"),
        Vertex("jouissance", content="jouissance — beyond the pleasure principle"),
    ]
    edges = [
        Edge("graph_of_desire", "signifier", EdgeType.DEPENDENCY),
        Edge("graph_of_desire", "desire", EdgeType.DEPENDENCY),
        Edge("graph_of_desire", "Other", EdgeType.DEPENDENCY),
        Edge("che_vuoi", "Other", EdgeType.DEPENDENCY),
        Edge("che_vuoi", "desire", EdgeType.DEPENDENCY),
        Edge("fantasy", "subject", EdgeType.DEPENDENCY),
        Edge("fantasy", "objet_a", EdgeType.DEPENDENCY),
        Edge("objet_a", "desire", EdgeType.DEPENDENCY),
        Edge("objet_a", "imaginary", EdgeType.REFERENCE),
        Edge("objet_a", "real", EdgeType.REFERENCE),
        Edge("vel_of_alienation", "subject", EdgeType.DEPENDENCY),
        Edge("vel_of_alienation", "Other", EdgeType.DEPENDENCY),
        Edge("separation", "vel_of_alienation", EdgeType.DEPENDENCY),
        Edge("separation", "lack", EdgeType.REFERENCE),
        Edge("drive", "demand", EdgeType.DEPENDENCY),
        Edge("drive", "jouissance", EdgeType.DEPENDENCY),
        Edge("jouissance", "pleasure_principle", EdgeType.NEGATION),
        Edge("jouissance", "real", EdgeType.REFERENCE),
        Edge("fantasy", "che_vuoi", EdgeType.REFERENCE),
    ]
    return ("É12 — 1960: The Subversion of the Subject", vertices, edges)


def ecrits_13_kant_with_sade():
    """1963: Kant with Sade."""
    vertices = [
        Vertex("moral_law", content="Kantian moral law — categorical imperative"),
        Vertex("sade_maxim", content="Sade's maxim — universal right to jouissance"),
        Vertex("voice", content="voice — objet a as superego command"),
        Vertex("superego", content="superego — Über-Ich"),
        Vertex("sadistic_will", content="sadistic will — instrument of jouissance"),
    ]
    edges = [
        Edge("sade_maxim", "moral_law", EdgeType.NEGATION),
        Edge("sade_maxim", "jouissance", EdgeType.DEPENDENCY),
        Edge("moral_law", "law", EdgeType.REFERENCE),
        Edge("moral_law", "desire", EdgeType.NEGATION),
        Edge("voice", "objet_a", EdgeType.DEPENDENCY),
        Edge("voice", "superego", EdgeType.DEPENDENCY),
        Edge("superego", "law", EdgeType.DEPENDENCY),
        Edge("superego", "jouissance", EdgeType.REFERENCE),
        Edge("sadistic_will", "drive", EdgeType.DEPENDENCY),
        Edge("sadistic_will", "jouissance", EdgeType.REFERENCE),
        Edge("sade_maxim", "moral_law", EdgeType.SUBLATION),
    ]
    return ("É13 — 1963: Kant with Sade", vertices, edges)


def ecrits_14_position_of_unconscious():
    """1960/1964: Position of the Unconscious."""
    vertices = [
        Vertex("subject_of_unconscious", content="subject of the unconscious — $"),
        Vertex("gap", content="gap — béance — where unconscious manifests"),
        Vertex("pulsation", content="pulsation — temporal opening/closing"),
        Vertex("lamella", content="lamella — libido as organ"),
    ]
    edges = [
        Edge("subject_of_unconscious", "subject", EdgeType.DEPENDENCY),
        Edge("subject_of_unconscious", "signifier", EdgeType.DEPENDENCY),
        Edge("subject_of_unconscious", "unconscious_structured", EdgeType.REFERENCE),
        Edge("gap", "subject_of_unconscious", EdgeType.DEPENDENCY),
        Edge("gap", "signifying_chain", EdgeType.REFERENCE),
        Edge("pulsation", "gap", EdgeType.DEPENDENCY),
        Edge("pulsation", "drive", EdgeType.REFERENCE),
        Edge("lamella", "drive", EdgeType.DEPENDENCY),
        Edge("lamella", "objet_a", EdgeType.REFERENCE),
        Edge("vel_of_alienation", "subject_of_unconscious", EdgeType.REFERENCE),
        Edge("separation", "subject_of_unconscious", EdgeType.REFERENCE),
    ]
    return ("É14 — 1964: Position of the Unconscious", vertices, edges)


def ecrits_15_science_and_truth():
    """1965: Science and Truth."""
    vertices = [
        Vertex("subject_of_science", content="subject of science — Cartesian cogito"),
        Vertex("truth_as_cause", content="truth as cause — not knowledge"),
        Vertex("magic", content="magic — efficient causality imaginarized"),
        Vertex("religion", content="religion — final cause"),
        Vertex("science", content="science — formal cause"),
        Vertex("psychoanalysis_cause", content="psychoanalysis — material cause (truth)"),
    ]
    edges = [
        Edge("subject_of_science", "subject", EdgeType.DEPENDENCY),
        Edge("subject_of_science", "subject_of_unconscious", EdgeType.REFERENCE),
        Edge("truth_as_cause", "truth", EdgeType.DEPENDENCY),
        Edge("truth_as_cause", "subject_of_science", EdgeType.DEPENDENCY),
        Edge("magic", "imaginary", EdgeType.REFERENCE),
        Edge("religion", "symbolic", EdgeType.REFERENCE),
        Edge("science", "signifier", EdgeType.REFERENCE),
        Edge("psychoanalysis_cause", "truth_as_cause", EdgeType.DEPENDENCY),
        Edge("psychoanalysis_cause", "subject_of_unconscious", EdgeType.DEPENDENCY),
        Edge("magic", "religion", EdgeType.NEGATION),
        Edge("religion", "science", EdgeType.NEGATION),
        Edge("science", "psychoanalysis_cause", EdgeType.NEGATION),
    ]
    return ("É15 — 1965: Science and Truth", vertices, edges)


def ecrits_16_of_structure():
    """1966: Of Structure as an Inmixing."""
    vertices = [
        Vertex("structure", content="structure — inmixing of otherness"),
        Vertex("S_of_A_barred", content="S(Ⱥ) — signifier of the lack in the Other"),
        Vertex("Other_barred", content="barred Other — Ⱥ — Other lacks"),
        Vertex("subject_barred", content="barred subject — $ — divided"),
        Vertex("no_metalanguage", content="there is no metalanguage"),
    ]
    edges = [
        Edge("structure", "signifier", EdgeType.DEPENDENCY),
        Edge("structure", "Other", EdgeType.DEPENDENCY),
        Edge("S_of_A_barred", "Other_barred", EdgeType.DEPENDENCY),
        Edge("S_of_A_barred", "signifier", EdgeType.DEPENDENCY),
        Edge("Other_barred", "Other", EdgeType.NEGATION),
        Edge("Other_barred", "lack", EdgeType.DEPENDENCY),
        Edge("subject_barred", "subject", EdgeType.DEPENDENCY),
        Edge("subject_barred", "signifier", EdgeType.DEPENDENCY),
        Edge("subject_barred", "subject_of_unconscious", EdgeType.REFERENCE),
        Edge("no_metalanguage", "signifier", EdgeType.DEPENDENCY),
        Edge("no_metalanguage", "Other", EdgeType.REFERENCE),
        Edge("S_of_A_barred", "jouissance", EdgeType.REFERENCE),
        Edge("subject_barred", "objet_a", EdgeType.REFERENCE),
    ]
    return ("É16 — 1966: Of Structure as an Inmixing", vertices, edges)


# ===========================================================================
# SEMINAR encodings (from experiment_lacan_seminars.py)
# ===========================================================================

def sem_I():
    """Seminar I: Freud's Papers on Technique (1953-54)."""
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
    return ("S01 — 1953: Sem I — Freud's Papers on Technique", vertices, edges)


def sem_II():
    """Seminar II: The Ego in Freud's Theory (1954-55)."""
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
    return ("S02 — 1954: Sem II — The Ego in Freud's Theory", vertices, edges)


def sem_III():
    """Seminar III: The Psychoses (1955-56)."""
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
    return ("S03 — 1955: Sem III — The Psychoses", vertices, edges)


def sem_IV():
    """Seminar IV: The Object Relation (1956-57)."""
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
    return ("S04 — 1956: Sem IV — The Object Relation", vertices, edges)


def sem_V():
    """Seminar V: Formations of the Unconscious (1957-58)."""
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
    return ("S05 — 1957: Sem V — Formations of the Unconscious", vertices, edges)


def sem_VI():
    """Seminar VI: Desire and Its Interpretation (1958-59)."""
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
    return ("S06 — 1958: Sem VI — Desire and Its Interpretation", vertices, edges)


def sem_VII():
    """Seminar VII: The Ethics of Psychoanalysis (1959-60)."""
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
    return ("S07 — 1959: Sem VII — The Ethics of Psychoanalysis", vertices, edges)


def sem_VIII():
    """Seminar VIII: Transference (1960-61)."""
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
    return ("S08 — 1960: Sem VIII — Transference", vertices, edges)


def sem_IX():
    """Seminar IX: Identification (1961-62)."""
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
    return ("S09 — 1961: Sem IX — Identification", vertices, edges)


def sem_X():
    """Seminar X: Anxiety (1962-63)."""
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
    return ("S10 — 1962: Sem X — Anxiety", vertices, edges)


def sem_XI():
    """Seminar XI: The Four Fundamental Concepts of Psychoanalysis (1964)."""
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
    return ("S11 — 1964: Sem XI — Four Fundamental Concepts", vertices, edges)


def sem_XII():
    """Seminar XII: Crucial Problems for Psychoanalysis (1964-65)."""
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
    return ("S12 — 1964: Sem XII — Crucial Problems", vertices, edges)


def sem_XIII():
    """Seminar XIII: The Object of Psychoanalysis (1965-66)."""
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
    return ("S13 — 1965: Sem XIII — The Object of Psychoanalysis", vertices, edges)


def sem_XIV():
    """Seminar XIV: The Logic of Phantasy (1966-67)."""
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
    return ("S14 — 1966: Sem XIV — The Logic of Phantasy", vertices, edges)


def other_proposition_1967():
    """1967: Proposition of 9 October 1967 on the Psychoanalyst of the School.

    Institutional text: the pass, the analytic school, authorization.
    """
    vertices = [
        Vertex("proposition_1967", content="Proposition of 9 October 1967"),
        Vertex("pass_procedure", content="the pass (la passe)"),
        Vertex("authorization", content="the analyst authorizes himself"),
        Vertex("school", content="the School (École)"),
        Vertex("AE", content="Analyst of the School (AE)"),
    ]
    edges = [
        Edge("proposition_1967", "pass_procedure", EdgeType.DEPENDENCY),
        Edge("proposition_1967", "school", EdgeType.DEPENDENCY),
        Edge("pass_procedure", "desire_analyst", EdgeType.DEPENDENCY),
        Edge("pass_procedure", "fantasy", EdgeType.DEPENDENCY),
        Edge("authorization", "pass_procedure", EdgeType.DEPENDENCY),
        Edge("authorization", "subject_unconscious", EdgeType.REFERENCE),
        Edge("AE", "pass_procedure", EdgeType.DEPENDENCY),
        Edge("AE", "school", EdgeType.DEPENDENCY),
        Edge("school", "four_discourses", EdgeType.REFERENCE),
    ]
    return ("OT01 — 1967: Proposition of 9 October 1967", vertices, edges)


def sem_XV():
    """Seminar XV: The Psychoanalytic Act (1967-68)."""
    vertices = [
        Vertex("psychoanalytic_act", content="the psychoanalytic act"),
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
    return ("S15 — 1967: Sem XV — The Psychoanalytic Act", vertices, edges)


def sem_XVI():
    """Seminar XVI: From an Other to the other (1968-69)."""
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
    return ("S16 — 1968: Sem XVI — From an Other to the other", vertices, edges)


def sem_XVII():
    """Seminar XVII: The Other Side of Psychoanalysis (1969-70)."""
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
    ]
    return ("S17 — 1969: Sem XVII — The Other Side of Psychoanalysis", vertices, edges)


def other_radiophonie():
    """1970: Radiophonie — radio interview, discourse theory, lalangue anticipated."""
    vertices = [
        Vertex("radiophonie", content="Radiophonie (1970)"),
        Vertex("four_discourses_applied", content="four discourses applied to social bond"),
        Vertex("linguisterie", content="linguisterie (vs linguistics)"),
    ]
    edges = [
        Edge("radiophonie", "four_discourses", EdgeType.REFERENCE),
        Edge("four_discourses_applied", "four_discourses", EdgeType.DEPENDENCY),
        Edge("four_discourses_applied", "surplus_jouissance", EdgeType.REFERENCE),
        Edge("linguisterie", "signifier", EdgeType.DEPENDENCY),
        Edge("linguisterie", "jouissance", EdgeType.DEPENDENCY),
        Edge("radiophonie", "linguisterie", EdgeType.DEPENDENCY),
    ]
    return ("OT02 — 1970: Radiophonie", vertices, edges)


def sem_XVIII():
    """Seminar XVIII: On a Discourse That Might Not Be a Semblance (1971)."""
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
    return ("S18 — 1971: Sem XVIII — On a Discourse That Might Not Be a Semblance", vertices, edges)


def other_lituraterre():
    """1971: Lituraterre — writing, the letter, jouissance and the literal."""
    vertices = [
        Vertex("lituraterre", content="Lituraterre (1971)"),
        Vertex("letter_lituraterre", content="letter as littoral (not literal)"),
        Vertex("littoral", content="littoral — edge between knowledge and jouissance"),
    ]
    edges = [
        Edge("lituraterre", "writing", EdgeType.DEPENDENCY),
        Edge("lituraterre", "jouissance", EdgeType.REFERENCE),
        Edge("letter_lituraterre", "letter", EdgeType.SUBLATION),
        Edge("letter_lituraterre", "writing", EdgeType.DEPENDENCY),
        Edge("littoral", "symbolic", EdgeType.DEPENDENCY),
        Edge("littoral", "real", EdgeType.DEPENDENCY),
        Edge("littoral", "letter_lituraterre", EdgeType.DEPENDENCY),
        Edge("lituraterre", "lalangue", EdgeType.REFERENCE),
    ]
    return ("OT03 — 1971: Lituraterre", vertices, edges)


def sem_XIX():
    """Seminar XIX: ...ou pire / ...or Worse (1971-72)."""
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
    return ("S19 — 1971: Sem XIX — ...or Worse", vertices, edges)


def other_letourdit():
    """1972: L'Étourdit — condensed statement of later teaching, dire vs dit."""
    vertices = [
        Vertex("letourdit", content="L'Étourdit (1972)"),
        Vertex("dire", content="dire (saying) vs dit (said)"),
        Vertex("topology_letourdit", content="topology — cross-cap, Boy surface"),
        Vertex("modal_logic", content="modal logic — necessary, impossible, contingent"),
    ]
    edges = [
        Edge("letourdit", "impossible_sexual_relation", EdgeType.REFERENCE),
        Edge("letourdit", "matheme", EdgeType.DEPENDENCY),
        Edge("dire", "signifier", EdgeType.DEPENDENCY),
        Edge("dire", "truth", EdgeType.DEPENDENCY),
        Edge("dire", "writing", EdgeType.NEGATION),
        Edge("topology_letourdit", "torus", EdgeType.SUBLATION),
        Edge("topology_letourdit", "real", EdgeType.DEPENDENCY),
        Edge("modal_logic", "impossible_sexual_relation", EdgeType.DEPENDENCY),
        Edge("modal_logic", "real", EdgeType.REFERENCE),
        Edge("letourdit", "dire", EdgeType.DEPENDENCY),
    ]
    return ("OT04 — 1972: L'Étourdit", vertices, edges)


def sem_XX():
    """Seminar XX: Encore (1972-73)."""
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
    return ("S20 — 1972: Sem XX — Encore", vertices, edges)


def other_television():
    """1973: Television — jouissance, discourse, the real, symptom."""
    vertices = [
        Vertex("television", content="Television (1973)"),
        Vertex("affect_discourse", content="affect determined by discourse"),
        Vertex("sadness_cowardice", content="sadness as moral cowardice"),
        Vertex("gay_savoir", content="gay savoir — satisfaction of the analyst"),
    ]
    edges = [
        Edge("television", "four_discourses", EdgeType.REFERENCE),
        Edge("television", "jouissance", EdgeType.REFERENCE),
        Edge("affect_discourse", "four_discourses", EdgeType.DEPENDENCY),
        Edge("affect_discourse", "jouissance", EdgeType.DEPENDENCY),
        Edge("sadness_cowardice", "affect_discourse", EdgeType.DEPENDENCY),
        Edge("sadness_cowardice", "ethics_psychoanalysis", EdgeType.REFERENCE),
        Edge("gay_savoir", "desire_analyst", EdgeType.DEPENDENCY),
        Edge("gay_savoir", "jouissance", EdgeType.REFERENCE),
        Edge("television", "real", EdgeType.REFERENCE),
    ]
    return ("OT05 — 1973: Television", vertices, edges)


def sem_XXI():
    """Seminar XXI: Les non-dupes errent / The Names-of-the-Father (1973-74)."""
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
    return ("S21 — 1973: Sem XXI — Names-of-the-Father / Les non-dupes errent", vertices, edges)


def sem_XXII():
    """Seminar XXII: R.S.I. (1974-75)."""
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
    return ("S22 — 1974: Sem XXII — R.S.I.", vertices, edges)


def other_joyce_le_symptome():
    """1975: Joyce le Symptôme — conference at the Joyce Symposium."""
    vertices = [
        Vertex("joyce_le_symptome", content="Joyce le Symptôme (1975 lecture)"),
        Vertex("joyce_writing", content="Joyce's writing as symptom"),
        Vertex("sinthome_anticipated", content="sinthome (anticipated in lecture)"),
    ]
    edges = [
        Edge("joyce_le_symptome", "jouissance", EdgeType.REFERENCE),
        Edge("joyce_le_symptome", "writing", EdgeType.DEPENDENCY),
        Edge("joyce_writing", "symptom_as_name", EdgeType.DEPENDENCY),
        Edge("joyce_writing", "lalangue", EdgeType.REFERENCE),
        Edge("sinthome_anticipated", "symptom_as_name", EdgeType.SUBLATION),
        Edge("sinthome_anticipated", "borromean_knot", EdgeType.REFERENCE),
        Edge("joyce_le_symptome", "joyce_writing", EdgeType.DEPENDENCY),
    ]
    return ("OT06 — 1975: Joyce le Symptôme", vertices, edges)


def sem_XXIII():
    """Seminar XXIII: The Sinthome (1975-76)."""
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
    return ("S23 — 1975: Sem XXIII — The Sinthome", vertices, edges)


# ===========================================================================
# Chronological sequence — interleaving Écrits, Seminars, and other texts
# ===========================================================================

ALL_TEXTS = [
    # 1936
    ecrits_01_beyond_reality_principle,
    # 1945
    ecrits_02_logical_time,
    # 1948
    ecrits_03_aggressiveness,
    # 1949
    ecrits_04_mirror_stage,
    # 1953: Rome Discourse then Seminar I (Écrits written for the discourse,
    #        Seminar I runs 1953-54)
    ecrits_05_function_and_field,
    ecrits_06_freudian_thing,
    sem_I,
    # 1954: Seminar II (1954-55)
    sem_II,
    # 1955: Purloined Letter + Seminar III (1955-56)
    ecrits_07_purloined_letter,
    sem_III,
    # 1956: Seminar IV (1956-57)
    sem_IV,
    # 1957: Agency of the Letter + Seminar V (1957-58)
    ecrits_08_agency_of_letter,
    sem_V,
    # 1958: Direction of Treatment + Signification of Phallus + Question Prior
    #        + Seminar VI (1958-59)
    ecrits_09_direction_of_treatment,
    ecrits_10_signification_of_phallus,
    ecrits_11_question_prior_to_treatment,
    sem_VI,
    # 1959: Seminar VII (1959-60)
    sem_VII,
    # 1960: Subversion of the Subject + Seminar VIII (1960-61)
    ecrits_12_subversion_of_subject,
    sem_VIII,
    # 1961: Seminar IX (1961-62)
    sem_IX,
    # 1962: Seminar X (1962-63)
    sem_X,
    # 1963: Kant with Sade
    ecrits_13_kant_with_sade,
    # 1964: Position of Unconscious + Seminar XI + Seminar XII (1964-65)
    ecrits_14_position_of_unconscious,
    sem_XI,
    sem_XII,
    # 1965: Science and Truth + Seminar XIII (1965-66)
    ecrits_15_science_and_truth,
    sem_XIII,
    # 1966: Of Structure + Seminar XIV (1966-67)
    ecrits_16_of_structure,
    sem_XIV,
    # 1967: Proposition of 9 October + Seminar XV (1967-68)
    other_proposition_1967,
    sem_XV,
    # 1968: Seminar XVI (1968-69)
    sem_XVI,
    # 1969: Seminar XVII (1969-70)
    sem_XVII,
    # 1970: Radiophonie
    other_radiophonie,
    # 1971: Seminar XVIII + Lituraterre + Seminar XIX (1971-72)
    sem_XVIII,
    other_lituraterre,
    sem_XIX,
    # 1972: L'Étourdit + Seminar XX (1972-73)
    other_letourdit,
    sem_XX,
    # 1973: Television + Seminar XXI (1973-74)
    other_television,
    sem_XXI,
    # 1974: Seminar XXII (1974-75)
    sem_XXII,
    # 1975: Joyce le Symptôme + Seminar XXIII (1975-76)
    other_joyce_le_symptome,
    sem_XXIII,
]


# ===========================================================================
# Incremental injection + digestion
# ===========================================================================

def run_experiment():
    n_texts = len(ALL_TEXTS)
    print("=" * 70)
    print("LACAN FULL CORPUS — ÉCRITS + SEMINARS I-XXIII + OTHER TEXTS")
    print(f"{n_texts} texts, chronologically interleaved, single complex")
    print("=" * 70)

    graph = Graph()
    engine = None
    text_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for t_idx, t_fn in enumerate(ALL_TEXTS):
        text_name, t_vertices, t_edges = t_fn()
        print(f"\n{'─' * 60}")
        print(f"Text {t_idx + 1}/{n_texts}: {text_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in t_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in t_edges:
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
        steps_this_text = 0
        max_steps = 2000

        while stable_count < 50 and steps_this_text < max_steps:
            engine.run_step()
            steps_this_text += 1
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

        print(f"  Steps to digest: {steps_this_text} {'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} (delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        # Count operations
        text_logs = engine.logs[-steps_this_text:] if steps_this_text > 0 else []
        op_counts = {}
        for log in text_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        result = {
            "text": text_name,
            "text_index": t_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_text": blocked_count,
            "steps_to_digest": steps_this_text,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        text_results.append(result)

        beta_1_curve.append({
            "text_index": t_idx + 1,
            "text_name": text_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        graph = engine.k_active

    # -----------------------------------------------------------------------
    # Final summary
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("FINAL SUMMARY")
    print("=" * 70)

    final_beta = compute_beta_1(engine.k_active)
    final_v = len(engine.k_active.active_vertex_ids())
    final_e = len(engine.k_active.active_edges())
    final_settled = len(engine.settlement.settled_cycles)

    print(f"\n  Total texts: {n_texts}")
    print(f"  Total steps: {cumulative_steps}")
    print(f"  Final beta_1: {final_beta}")
    print(f"  Final complex: {final_v} V, {final_e} E")
    print(f"  Settled cycles: {final_settled}")

    # Beta_1 growth curve
    print(f"\n  beta_1 growth curve (by text):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    T{entry['text_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"         {entry['text_name']}")

    # Largest jumps
    deltas = [(r["text"], r["delta_beta_1"], r["text_index"]) for r in text_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps:")
    for name, delta, idx in deltas_sorted[:8]:
        print(f"    T{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by text:")
    for r in text_results:
        print(f"    T{r['text_index']:2d} {r['text']}: "
              f"{r['steps_to_digest']} steps {'OK' if r['converged'] else 'MAX'}")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({final_settled}):")
        for i, sc in enumerate(engine.settlement.settled_cycles[:25]):
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

    # Sublation density
    sub_count = type_counts.get("sublation", 0)
    sub_density = sub_count / final_e if final_e > 0 else 0
    print(f"  Sublation density: {sub_density:.4f} ({sub_count}/{final_e})")

    # Source breakdown
    ecrits_count = sum(1 for r in text_results if r["text"].startswith("É"))
    sem_count = sum(1 for r in text_results if r["text"].startswith("S"))
    other_count = sum(1 for r in text_results if r["text"].startswith("OT"))
    print(f"\n  Source breakdown: {ecrits_count} Écrits + {sem_count} Seminars + {other_count} other texts = {n_texts}")

    # Comparison with separate experiments
    print(f"\n  (Previous separate runs: Écrits-only β₁=50, Seminars-only β₁=112)")
    print(f"  Merged β₁={final_beta} — {'higher' if final_beta > 112 else 'lower or equal'} than separate maximum")

    # -----------------------------------------------------------------------
    # Save
    # -----------------------------------------------------------------------
    output = {
        "experiment": "lacan_full_corpus_incremental",
        "source": "Lacan — Écrits (16) + Seminars I-XXIII (23) + other texts (6) = 45 texts",
        "methodology": "chronologically interleaved injection into single complex, "
                        "digestion until 50-step beta_1 stability per text",
        "text_results": text_results,
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
        "total_texts": n_texts,
        "source_breakdown": {
            "ecrits": ecrits_count,
            "seminars": sem_count,
            "other_texts": other_count,
        },
        "total_steps": cumulative_steps,
        "comparison_with_separate": {
            "ecrits_only_beta_1": 50,
            "seminars_only_beta_1": 112,
            "merged_beta_1": final_beta,
        },
        "settled_cycles_detail": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_lacan_full.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
