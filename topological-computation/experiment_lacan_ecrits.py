"""Lacan Écrits — 16-paper incremental traversal experiment.

Each paper is injected as a dialectical complex (vertices + edges).
After injection, the traversal engine "digests" the paper
(runs until beta_1 is stable for 50 consecutive steps).
Then the next paper is injected.

Papers are ordered by publication date (1936-1966).
Shared concepts (signifier, subject, Other, desire, etc.) connect
across papers through shared vertex IDs — modeling the evolution
of Lacan's conceptual apparatus across three decades.
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
# Paper encodings: each returns (paper_name, vertices, edges)
# Cross-paper edges connect to vertices already introduced in prior papers.
# ---------------------------------------------------------------------------


def paper_01_beyond_reality_principle():
    """1936: Beyond the Reality Principle — critique of associationism,
    image as psychic reality, imago, identification."""
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
    return ("1936: Beyond the Reality Principle", vertices, edges)


def paper_02_logical_time():
    """1945: Logical Time and the Assertion of Anticipated Certainty —
    three prisoners, three moments of time, intersubjective logic."""
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
    return ("1945: Logical Time", vertices, edges)


def paper_03_aggressiveness():
    """1948: Aggressiveness in Psychoanalysis —
    aggressivity as correlative of narcissistic identification."""
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
    return ("1948: Aggressiveness in Psychoanalysis", vertices, edges)


def paper_04_mirror_stage():
    """1949: The Mirror Stage as Formative of the I Function —
    Lacan's foundational text on ego formation."""
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
    return ("1949: The Mirror Stage", vertices, edges)


def paper_05_function_and_field():
    """1953: Function and Field of Speech and Language —
    the Rome Discourse. Speech/language distinction, symbolic order, full/empty speech."""
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
    return ("1953: Function and Field of Speech and Language", vertices, edges)


def paper_06_freudian_thing():
    """1953: The Freudian Thing — truth speaks through the symptom,
    ego-psychology critique."""
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
    return ("1953: The Freudian Thing", vertices, edges)


def paper_07_purloined_letter():
    """1955: Seminar on The Purloined Letter —
    signifier determines subject positions, letter always arrives."""
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
    return ("1955: Seminar on The Purloined Letter", vertices, edges)


def paper_08_agency_of_letter():
    """1957: The Agency of the Letter in the Unconscious —
    metaphor/metonymy, algorithm S/s, bar."""
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
    return ("1957: The Agency of the Letter", vertices, edges)


def paper_09_direction_of_treatment():
    """1958: The Direction of the Treatment and the Principles of Its Power —
    desire of the analyst, demand vs desire, frustration/privation/castration."""
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
    return ("1958: The Direction of the Treatment", vertices, edges)


def paper_10_signification_of_phallus():
    """1958: The Signification of the Phallus — phallus as signifier
    of desire, being vs having, Bedeutung."""
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
    return ("1958: The Signification of the Phallus", vertices, edges)


def paper_11_question_prior_to_treatment():
    """1958: On a Question Prior to Any Possible Treatment of Psychosis —
    foreclosure, paternal metaphor, Schreber."""
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
    return ("1958: On a Question Prior to Any Treatment of Psychosis", vertices, edges)


def paper_12_subversion_of_subject():
    """1960: The Subversion of the Subject and the Dialectic of Desire —
    Graph of Desire, che vuoi, fantasy formula, objet a."""
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
    return ("1960: The Subversion of the Subject", vertices, edges)


def paper_13_kant_with_sade():
    """1963: Kant with Sade — Sade as truth of Kant,
    moral law and jouissance, voice as objet a."""
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
    return ("1963: Kant with Sade", vertices, edges)


def paper_14_position_of_unconscious():
    """1960/1964: Position of the Unconscious — subject of the unconscious,
    alienation/separation formalized, gap as locus."""
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
    return ("1960/1964: Position of the Unconscious", vertices, edges)


def paper_15_science_and_truth():
    """1965: Science and Truth — subject of science,
    four discourses foreshadowed, truth as cause."""
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
    return ("1965: Science and Truth", vertices, edges)


def paper_16_of_structure():
    """1966: Of Structure as an Inmixing — structure, the subject
    in the field of the Other, S(A-barred)."""
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
    return ("1966: Of Structure as an Inmixing", vertices, edges)


# ---------------------------------------------------------------------------
# Paper sequence
# ---------------------------------------------------------------------------

ALL_PAPERS = [
    paper_01_beyond_reality_principle,
    paper_02_logical_time,
    paper_03_aggressiveness,
    paper_04_mirror_stage,
    paper_05_function_and_field,
    paper_06_freudian_thing,
    paper_07_purloined_letter,
    paper_08_agency_of_letter,
    paper_09_direction_of_treatment,
    paper_10_signification_of_phallus,
    paper_11_question_prior_to_treatment,
    paper_12_subversion_of_subject,
    paper_13_kant_with_sade,
    paper_14_position_of_unconscious,
    paper_15_science_and_truth,
    paper_16_of_structure,
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    n_papers = len(ALL_PAPERS)
    print("=" * 70)
    print("LACAN ÉCRITS — FULL INCREMENTAL TRAVERSAL")
    print(f"{n_papers} papers, incremental injection, digestion until beta_1 stable")
    print("=" * 70)

    graph = Graph()
    engine = None
    paper_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for p_idx, p_fn in enumerate(ALL_PAPERS):
        paper_name, p_vertices, p_edges = p_fn()
        print(f"\n{'─' * 60}")
        print(f"Paper {p_idx + 1}/{n_papers}: {paper_name}")
        print(f"{'─' * 60}")

        # 1. Inject vertices
        new_v_count = 0
        existing_vids = set(graph.vertices.keys())
        for v in p_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
                new_v_count += 1

        # 2. Inject edges (skip duplicates)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        new_e_count = 0
        for e in p_edges:
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
        steps_this_paper = 0
        max_steps = 2000  # safety cap per paper

        while stable_count < 50 and steps_this_paper < max_steps:
            engine.run_step()
            steps_this_paper += 1
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

        print(f"  Steps to digest: {steps_this_paper} {'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after digestion: {beta_after} (delta from injection: {beta_after - beta_before})")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(engine.k_active.active_vertex_ids())} V, "
              f"{len(engine.k_active.active_edges())} E")

        # Count operations in this paper's digestion
        paper_logs = engine.logs[-steps_this_paper:] if steps_this_paper > 0 else []
        op_counts = {}
        for log in paper_logs:
            op_counts[log.operation] = op_counts.get(log.operation, 0) + 1

        # Record
        result = {
            "paper": paper_name,
            "paper_index": p_idx + 1,
            "vertices_added": new_v_count,
            "edges_added": new_e_count,
            "vertices_total": len(engine.k_active.active_vertex_ids()),
            "edges_total": len(engine.k_active.active_edges()),
            "beta_1_before_injection": beta_before,
            "beta_1_after_digestion": beta_after,
            "delta_beta_1": beta_after - beta_before,
            "settled_cycles": settled_count,
            "blocked_in_paper": blocked_count,
            "steps_to_digest": steps_this_paper,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
        }
        paper_results.append(result)

        beta_1_curve.append({
            "paper_index": p_idx + 1,
            "paper_name": paper_name,
            "beta_1": beta_after,
            "cumulative_steps": cumulative_steps,
        })

        # Update graph reference for next paper injection
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
    print(f"\n  beta_1 growth curve (by paper):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    P{entry['paper_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['paper_name']}")

    # Biggest jumps
    deltas = [(r["paper"], r["delta_beta_1"], r["paper_index"]) for r in paper_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by paper:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    P{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by paper:")
    for r in paper_results:
        print(f"    P{r['paper_index']:2d} {r['paper']}: "
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
        "experiment": "lacan_ecrits_incremental",
        "source": "Lacan, Écrits — 16 papers by publication date (1936-1966)",
        "methodology": "manual dialectical complex per paper, incremental injection, "
                        "digestion until 50-step beta_1 stability",
        "paper_results": paper_results,
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

    output_path = "experiment_lacan_ecrits.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
