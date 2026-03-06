"""Slavoj Žižek — collected works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Žižek's philosophical
thought as an incremental topological growth process.

Covers ~25 works spanning 1989-2020.
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
#   ideology, fantasy, objet_a, parallax, Real, big_Other, enjoyment,
#   symptom, sublime_object, act, drive, death_drive, antagonism,
#   universality, subject, desire, lack, surplus_enjoyment,
#   symbolic, imaginary, castration, master_signifier, fetish,
#   Hegel, Lacan, Marx, dialectics, negation, negativity,
#   freedom, revolution, violence, capitalism, class_struggle
# ---------------------------------------------------------------------------


def work_01_sublime_object():
    """1989: The Sublime Object of Ideology.

    Žižek's breakthrough — Lacanian reading of ideology via Hegel.
    Fantasy as the screen that structures social reality. The symptom
    as the point where ideology fails. Sublime object = objet a in
    the ideological field.
    """
    vertices = [
        Vertex("ideology", content="ideology — not false consciousness but the fantasy structuring reality"),
        Vertex("fantasy", content="fantasy — the screen organizing desire and masking the Real"),
        Vertex("sublime_object", content="sublime object — objet a elevated to the dignity of the Thing"),
        Vertex("symptom", content="symptom — the point of ideological failure, return of the repressed"),
        Vertex("Real", content="the Real — the traumatic kernel resisting symbolization"),
        Vertex("symbolic", content="the Symbolic — the order of language and law"),
        Vertex("imaginary", content="the Imaginary — the register of images and identification"),
        Vertex("objet_a", content="objet a — the object-cause of desire"),
        Vertex("subject", content="the subject — the barred subject ($), constituted by lack"),
        Vertex("lack", content="lack — constitutive incompleteness"),
        Vertex("Hegel", content="Hegel — dialectical logic as the logic of the signifier"),
        Vertex("Lacan", content="Lacan — the return to Freud through structural linguistics"),
        Vertex("Marx", content="Marx — commodity fetishism as the paradigm of ideological fantasy"),
        Vertex("fetish", content="fetish — the disavowed belief, 'I know well, but nevertheless...'"),
    ]
    edges = [
        Edge("ideology", "fantasy", EdgeType.DEPENDENCY),
        Edge("fantasy", "Real", EdgeType.NEGATION),
        Edge("sublime_object", "objet_a", EdgeType.DEPENDENCY),
        Edge("symptom", "ideology", EdgeType.NEGATION),
        Edge("symptom", "Real", EdgeType.DEPENDENCY),
        Edge("subject", "lack", EdgeType.DEPENDENCY),
        Edge("subject", "symbolic", EdgeType.DEPENDENCY),
        Edge("fantasy", "objet_a", EdgeType.DEPENDENCY),
        Edge("fetish", "ideology", EdgeType.DEPENDENCY),
        Edge("Hegel", "Lacan", EdgeType.REFERENCE),
        Edge("Marx", "ideology", EdgeType.REFERENCE),
        Edge("ideology", "symbolic", EdgeType.DEPENDENCY),
        Edge("Real", "symbolic", EdgeType.NEGATION),
        Edge("imaginary", "symbolic", EdgeType.NEGATION),
    ]
    return "1989: The Sublime Object of Ideology", vertices, edges


def work_02_they_know_not():
    """1991: For They Know Not What They Do — Enjoyment as a Political Factor.

    Deepening of the Hegel-Lacan connection. Enjoyment (jouissance)
    as the hidden stake of ideology. The nation as a form of organized
    enjoyment. The Hegelian 'negation of negation' reread through Lacan.
    """
    vertices = [
        Vertex("enjoyment", content="enjoyment — jouissance, the excessive satisfaction beyond pleasure"),
        Vertex("big_Other", content="the big Other — the symbolic order, the supposed guarantee"),
        Vertex("master_signifier", content="master signifier — S1, the quilting point of ideology"),
        Vertex("nation", content="the nation — the way a community organizes its enjoyment"),
        Vertex("negation", content="negation — Hegelian determinate negation"),
        Vertex("negation_of_negation", content="negation of negation — not return but self-relating negativity"),
    ]
    edges = [
        Edge("enjoyment", "ideology", EdgeType.DEPENDENCY),
        Edge("enjoyment", "Real", EdgeType.DEPENDENCY),
        Edge("big_Other", "symbolic", EdgeType.DEPENDENCY),
        Edge("big_Other", "lack", EdgeType.DEPENDENCY),
        Edge("master_signifier", "ideology", EdgeType.DEPENDENCY),
        Edge("master_signifier", "big_Other", EdgeType.DEPENDENCY),
        Edge("nation", "enjoyment", EdgeType.DEPENDENCY),
        Edge("negation", "Hegel", EdgeType.DEPENDENCY),
        Edge("negation_of_negation", "negation", EdgeType.SUBLATION),
        Edge("enjoyment", "fantasy", EdgeType.DEPENDENCY),
        Edge("nation", "fantasy", EdgeType.REFERENCE),
    ]
    return "1991: For They Know Not What They Do", vertices, edges


def work_03_tarrying_negative():
    """1993: Tarrying with the Negative.

    Hegel's negativity as the core of subjectivity. The subject as
    'the night of the world'. Democracy and its discontents.
    """
    vertices = [
        Vertex("negativity", content="negativity — the power of the negative, tarrying with it"),
        Vertex("night_of_world", content="night of the world — the subject's radical negativity"),
        Vertex("democracy", content="democracy — the regime of the empty place of power"),
        Vertex("antagonism", content="antagonism — the constitutive social impossibility"),
    ]
    edges = [
        Edge("negativity", "Hegel", EdgeType.DEPENDENCY),
        Edge("negativity", "subject", EdgeType.DEPENDENCY),
        Edge("night_of_world", "subject", EdgeType.DEPENDENCY),
        Edge("night_of_world", "Real", EdgeType.REFERENCE),
        Edge("democracy", "big_Other", EdgeType.DEPENDENCY),
        Edge("democracy", "lack", EdgeType.DEPENDENCY),
        Edge("antagonism", "Real", EdgeType.DEPENDENCY),
        Edge("antagonism", "ideology", EdgeType.NEGATION),
        Edge("negativity", "negation", EdgeType.DEPENDENCY),
        Edge("night_of_world", "negativity", EdgeType.DEPENDENCY),
    ]
    return "1993: Tarrying with the Negative", vertices, edges


def work_04_plague_of_fantasies():
    """1997: The Plague of Fantasies.

    Fantasy as the support of reality. Cyberspace and its discontents.
    The interpassive subject. Fetishist disavowal in late capitalism.
    """
    vertices = [
        Vertex("interpassivity", content="interpassivity — the Other enjoys for me"),
        Vertex("cyberspace", content="cyberspace — virtual reality as ideological fantasy"),
        Vertex("surplus_enjoyment", content="surplus-enjoyment — plus-de-jouir, Mehrlust"),
        Vertex("desire", content="desire — the metonymy of the signifier, always desire of the Other"),
    ]
    edges = [
        Edge("interpassivity", "big_Other", EdgeType.DEPENDENCY),
        Edge("interpassivity", "enjoyment", EdgeType.DEPENDENCY),
        Edge("cyberspace", "fantasy", EdgeType.DEPENDENCY),
        Edge("cyberspace", "Real", EdgeType.NEGATION),
        Edge("surplus_enjoyment", "objet_a", EdgeType.DEPENDENCY),
        Edge("surplus_enjoyment", "enjoyment", EdgeType.DEPENDENCY),
        Edge("desire", "lack", EdgeType.DEPENDENCY),
        Edge("desire", "big_Other", EdgeType.DEPENDENCY),
        Edge("fantasy", "desire", EdgeType.DEPENDENCY),
        Edge("fetish", "fantasy", EdgeType.REFERENCE),
    ]
    return "1997: The Plague of Fantasies", vertices, edges


def work_05_ticklish_subject():
    """1999: The Ticklish Subject — The Absent Centre of Political Ontology.

    The Cartesian subject redeemed via Hegel and Lacan. Critique of
    Heidegger, Derrida, and postmodern anti-subjectivism. The act.
    """
    vertices = [
        Vertex("act", content="the act — the genuine ethical/political act, breaking the symbolic coordinates"),
        Vertex("Cartesian_subject", content="Cartesian subject — cogito as the empty point of self-relating negativity"),
        Vertex("drive", content="drive — Trieb, beyond desire, acephalous, headless"),
        Vertex("political_act", content="political act — authentic revolutionary intervention"),
    ]
    edges = [
        Edge("act", "subject", EdgeType.DEPENDENCY),
        Edge("act", "Real", EdgeType.DEPENDENCY),
        Edge("act", "symbolic", EdgeType.NEGATION),
        Edge("Cartesian_subject", "subject", EdgeType.DEPENDENCY),
        Edge("Cartesian_subject", "negativity", EdgeType.DEPENDENCY),
        Edge("drive", "desire", EdgeType.NEGATION),
        Edge("drive", "Real", EdgeType.DEPENDENCY),
        Edge("political_act", "act", EdgeType.DEPENDENCY),
        Edge("political_act", "antagonism", EdgeType.DEPENDENCY),
        Edge("drive", "objet_a", EdgeType.DEPENDENCY),
    ]
    return "1999: The Ticklish Subject", vertices, edges


def work_06_looking_awry():
    """1991: Looking Awry — An Introduction to Lacan through Popular Culture.

    Hitchcock, film noir, detective fiction as illustrations of
    Lacanian concepts. The gaze, anamorphosis, the Thing.
    """
    vertices = [
        Vertex("gaze", content="the gaze — the object-gaze, the stain in the picture"),
        Vertex("Thing", content="the Thing — das Ding, the impossible-real object of desire"),
        Vertex("anamorphosis", content="anamorphosis — looking awry to see the truth"),
    ]
    edges = [
        Edge("gaze", "objet_a", EdgeType.DEPENDENCY),
        Edge("Thing", "Real", EdgeType.DEPENDENCY),
        Edge("Thing", "desire", EdgeType.DEPENDENCY),
        Edge("anamorphosis", "gaze", EdgeType.DEPENDENCY),
        Edge("anamorphosis", "Real", EdgeType.REFERENCE),
        Edge("Thing", "sublime_object", EdgeType.REFERENCE),
        Edge("gaze", "subject", EdgeType.NEGATION),
    ]
    return "1991: Looking Awry", vertices, edges


def work_07_enjoy_your_symptom():
    """1992: Enjoy Your Symptom! — Lacan in and out of Hollywood.

    The symptom as sinthome — the kernel of enjoyment that cannot
    be dissolved. Cinema as the art of the gaze.
    """
    vertices = [
        Vertex("sinthome", content="sinthome — the irreducible kernel of enjoyment, beyond analysis"),
        Vertex("traversing_fantasy", content="traversing the fantasy — le traversée du fantasme"),
    ]
    edges = [
        Edge("sinthome", "symptom", EdgeType.SUBLATION),
        Edge("sinthome", "enjoyment", EdgeType.DEPENDENCY),
        Edge("sinthome", "Real", EdgeType.DEPENDENCY),
        Edge("traversing_fantasy", "fantasy", EdgeType.NEGATION),
        Edge("traversing_fantasy", "act", EdgeType.REFERENCE),
        Edge("sinthome", "drive", EdgeType.REFERENCE),
    ]
    return "1992: Enjoy Your Symptom!", vertices, edges


def work_08_parallax_view():
    """2006: The Parallax View.

    The parallax gap — the irreducible shift between two perspectives
    that cannot be mediated. Stellar (cosmological), solar (biological),
    lunar (political) parallax. Against vulgar dialectical synthesis.
    """
    vertices = [
        Vertex("parallax", content="parallax — the irreducible gap between two perspectives"),
        Vertex("parallax_gap", content="parallax gap — not a third position but the gap itself"),
        Vertex("materialism", content="dialectical materialism — materialism of the gap, not substance"),
        Vertex("brain_mind", content="brain/mind parallax — the neuronal and the mental"),
    ]
    edges = [
        Edge("parallax", "Real", EdgeType.DEPENDENCY),
        Edge("parallax", "antagonism", EdgeType.DEPENDENCY),
        Edge("parallax_gap", "parallax", EdgeType.DEPENDENCY),
        Edge("parallax_gap", "negativity", EdgeType.DEPENDENCY),
        Edge("materialism", "Hegel", EdgeType.DEPENDENCY),
        Edge("materialism", "parallax", EdgeType.DEPENDENCY),
        Edge("brain_mind", "parallax", EdgeType.REFERENCE),
        Edge("parallax_gap", "lack", EdgeType.REFERENCE),
        Edge("materialism", "Marx", EdgeType.REFERENCE),
    ]
    return "2006: The Parallax View", vertices, edges


def work_09_less_than_nothing():
    """2012: Less Than Nothing — Hegel and the Shadow of Dialectical Materialism.

    Žižek's magnum opus on Hegel. Less than nothing = the void that is
    less than zero. Quantum physics, Hegel's logic, Lacan's formulae
    of sexuation. The ontological incompleteness of reality.
    """
    vertices = [
        Vertex("less_than_nothing", content="less than nothing — the void prior to any something"),
        Vertex("ontological_incompleteness", content="ontological incompleteness — reality is not-all"),
        Vertex("dialectical_materialism", content="dialectical materialism — the gap IS the substance"),
        Vertex("not_all", content="not-all — pas-tout, the feminine logic of non-totalization"),
        Vertex("void", content="the void — the constitutive gap in being"),
        Vertex("retroactivity", content="retroactivity — the past changes retroactively"),
    ]
    edges = [
        Edge("less_than_nothing", "void", EdgeType.DEPENDENCY),
        Edge("less_than_nothing", "negativity", EdgeType.DEPENDENCY),
        Edge("ontological_incompleteness", "Real", EdgeType.DEPENDENCY),
        Edge("ontological_incompleteness", "not_all", EdgeType.DEPENDENCY),
        Edge("dialectical_materialism", "materialism", EdgeType.SUBLATION),
        Edge("dialectical_materialism", "Hegel", EdgeType.DEPENDENCY),
        Edge("not_all", "big_Other", EdgeType.NEGATION),
        Edge("void", "subject", EdgeType.DEPENDENCY),
        Edge("retroactivity", "Hegel", EdgeType.DEPENDENCY),
        Edge("retroactivity", "negation_of_negation", EdgeType.REFERENCE),
        Edge("less_than_nothing", "Lacan", EdgeType.REFERENCE),
    ]
    return "2012: Less Than Nothing", vertices, edges


def work_10_absolute_recoil():
    """2014: Absolute Recoil — Towards a New Foundation of Dialectical Materialism.

    The recoil of the cause into its effects. The parallax of
    causes and reasons. Recoil as the motor of dialectics.
    """
    vertices = [
        Vertex("absolute_recoil", content="absolute recoil — the cause that is the effect of its effects"),
        Vertex("den", content="den — the less-than-nothing, the pre-ontological X"),
        Vertex("interface", content="interface — the surface IS the depth"),
    ]
    edges = [
        Edge("absolute_recoil", "negation_of_negation", EdgeType.DEPENDENCY),
        Edge("absolute_recoil", "retroactivity", EdgeType.DEPENDENCY),
        Edge("den", "less_than_nothing", EdgeType.DEPENDENCY),
        Edge("den", "void", EdgeType.DEPENDENCY),
        Edge("interface", "parallax", EdgeType.DEPENDENCY),
        Edge("absolute_recoil", "dialectical_materialism", EdgeType.DEPENDENCY),
        Edge("interface", "Real", EdgeType.REFERENCE),
    ]
    return "2014: Absolute Recoil", vertices, edges


def work_11_sex_failed_absolute():
    """2020: Sex and the Failed Absolute.

    The absolute fails — not a deficiency but its positive mode.
    Sexuation as the model for ontological incompleteness.
    The gap between the two formulae of sexuation IS the Real.
    """
    vertices = [
        Vertex("failed_absolute", content="the failed absolute — the absolute that exists only as its own failure"),
        Vertex("sexuation", content="sexuation — the Lacanian formulae, masculine exception vs feminine not-all"),
        Vertex("death_drive", content="death drive — Todestrieb, the drive to repeat failure"),
    ]
    edges = [
        Edge("failed_absolute", "ontological_incompleteness", EdgeType.DEPENDENCY),
        Edge("failed_absolute", "absolute_recoil", EdgeType.DEPENDENCY),
        Edge("sexuation", "not_all", EdgeType.DEPENDENCY),
        Edge("sexuation", "Real", EdgeType.DEPENDENCY),
        Edge("death_drive", "drive", EdgeType.DEPENDENCY),
        Edge("death_drive", "Real", EdgeType.DEPENDENCY),
        Edge("failed_absolute", "Hegel", EdgeType.REFERENCE),
        Edge("sexuation", "parallax", EdgeType.REFERENCE),
        Edge("death_drive", "enjoyment", EdgeType.NEGATION),
    ]
    return "2020: Sex and the Failed Absolute", vertices, edges


def work_12_living_end_times():
    """2010: Living in the End Times.

    The five stages of grief applied to global capitalism's crisis.
    Ecology, biogenetics, intellectual property, new forms of apartheid.
    The communist hypothesis renewed.
    """
    vertices = [
        Vertex("end_times", content="end times — the crisis of global capitalism"),
        Vertex("capitalism", content="capitalism — the Real of our age"),
        Vertex("communist_hypothesis", content="the communist hypothesis — the Idea that refuses to die"),
        Vertex("class_struggle", content="class struggle — the antagonism that structures all others"),
    ]
    edges = [
        Edge("end_times", "capitalism", EdgeType.DEPENDENCY),
        Edge("capitalism", "antagonism", EdgeType.DEPENDENCY),
        Edge("communist_hypothesis", "act", EdgeType.DEPENDENCY),
        Edge("communist_hypothesis", "class_struggle", EdgeType.DEPENDENCY),
        Edge("class_struggle", "Marx", EdgeType.DEPENDENCY),
        Edge("class_struggle", "antagonism", EdgeType.DEPENDENCY),
        Edge("end_times", "Real", EdgeType.REFERENCE),
        Edge("capitalism", "enjoyment", EdgeType.REFERENCE),
    ]
    return "2010: Living in the End Times", vertices, edges


def work_13_violence():
    """2008: Violence — Six Sideways Reflections.

    Subjective vs objective (systemic + symbolic) violence.
    The violence of language. Liberal humanitarianism as ideology.
    """
    vertices = [
        Vertex("violence", content="violence — subjective and objective (systemic + symbolic)"),
        Vertex("systemic_violence", content="systemic violence — the violence of the normal functioning"),
        Vertex("symbolic_violence", content="symbolic violence — the violence inherent in language"),
    ]
    edges = [
        Edge("violence", "systemic_violence", EdgeType.DEPENDENCY),
        Edge("violence", "symbolic_violence", EdgeType.DEPENDENCY),
        Edge("systemic_violence", "capitalism", EdgeType.DEPENDENCY),
        Edge("symbolic_violence", "symbolic", EdgeType.DEPENDENCY),
        Edge("symbolic_violence", "big_Other", EdgeType.DEPENDENCY),
        Edge("violence", "Real", EdgeType.REFERENCE),
        Edge("systemic_violence", "ideology", EdgeType.REFERENCE),
    ]
    return "2008: Violence", vertices, edges


def work_14_first_as_tragedy():
    """2009: First as Tragedy, Then as Farce.

    Post-2008 crisis diagnosis. Liberal communists (Soros, Gates).
    The return of the revolutionary Idea. Four riders of the
    apocalypse: ecology, biogenetics, property, social division.
    """
    vertices = [
        Vertex("tragedy_farce", content="first as tragedy, then as farce — Marx's dictum inverted"),
        Vertex("liberal_communists", content="liberal communists — philanthropic capitalism as ideology"),
        Vertex("revolutionary_idea", content="the Idea — the communist Idea as the horizon of emancipation"),
    ]
    edges = [
        Edge("tragedy_farce", "Marx", EdgeType.REFERENCE),
        Edge("tragedy_farce", "capitalism", EdgeType.DEPENDENCY),
        Edge("liberal_communists", "ideology", EdgeType.DEPENDENCY),
        Edge("liberal_communists", "fetish", EdgeType.REFERENCE),
        Edge("revolutionary_idea", "communist_hypothesis", EdgeType.DEPENDENCY),
        Edge("revolutionary_idea", "act", EdgeType.DEPENDENCY),
        Edge("tragedy_farce", "end_times", EdgeType.REFERENCE),
    ]
    return "2009: First as Tragedy, Then as Farce", vertices, edges


def work_15_in_defense_lost_causes():
    """2008: In Defense of Lost Causes.

    Rehabilitating revolutionary terror. Robespierre, Mao, Stalin
    reread. The emancipatory kernel in 'totalitarian' projects.
    """
    vertices = [
        Vertex("lost_causes", content="lost causes — the emancipatory kernel in failed revolutions"),
        Vertex("revolutionary_terror", content="revolutionary terror — divine violence, the founding act"),
        Vertex("universality", content="universality — the concrete universal, not abstract"),
        Vertex("revolution", content="revolution — the authentic political Event"),
    ]
    edges = [
        Edge("lost_causes", "revolution", EdgeType.DEPENDENCY),
        Edge("revolutionary_terror", "act", EdgeType.DEPENDENCY),
        Edge("revolutionary_terror", "violence", EdgeType.DEPENDENCY),
        Edge("universality", "Hegel", EdgeType.DEPENDENCY),
        Edge("universality", "antagonism", EdgeType.NEGATION),
        Edge("revolution", "act", EdgeType.DEPENDENCY),
        Edge("revolution", "class_struggle", EdgeType.DEPENDENCY),
        Edge("lost_causes", "communist_hypothesis", EdgeType.REFERENCE),
    ]
    return "2008: In Defense of Lost Causes", vertices, edges


def work_16_courage_of_hopelessness():
    """2017: The Courage of Hopelessness.

    Against false hopes. Courage not despite hopelessness but
    because of it. Populism, refugees, digital control.
    """
    vertices = [
        Vertex("hopelessness", content="the courage of hopelessness — act without guarantee"),
        Vertex("populism", content="populism — the false answer to a real question"),
    ]
    edges = [
        Edge("hopelessness", "act", EdgeType.DEPENDENCY),
        Edge("hopelessness", "drive", EdgeType.DEPENDENCY),
        Edge("populism", "ideology", EdgeType.DEPENDENCY),
        Edge("populism", "master_signifier", EdgeType.DEPENDENCY),
        Edge("hopelessness", "revolution", EdgeType.REFERENCE),
        Edge("populism", "fantasy", EdgeType.REFERENCE),
    ]
    return "2017: The Courage of Hopelessness", vertices, edges


def work_17_disparities():
    """2016: Disparities.

    Against the ontology of harmonious multiplicity. Disparity as
    the minimal form of ontological difference. Quantum ontology.
    """
    vertices = [
        Vertex("disparity", content="disparity — the minimal ontological gap, not between two things but within one"),
        Vertex("quantum_ontology", content="quantum ontology — the wave function collapse as the act of subjectivation"),
    ]
    edges = [
        Edge("disparity", "parallax_gap", EdgeType.DEPENDENCY),
        Edge("disparity", "negativity", EdgeType.DEPENDENCY),
        Edge("quantum_ontology", "ontological_incompleteness", EdgeType.DEPENDENCY),
        Edge("quantum_ontology", "void", EdgeType.REFERENCE),
        Edge("disparity", "Real", EdgeType.REFERENCE),
        Edge("disparity", "den", EdgeType.REFERENCE),
    ]
    return "2016: Disparities", vertices, edges


def work_18_fragile_absolute():
    """2000: The Fragile Absolute — Or, Why is the Christian Legacy Worth Fighting For?

    Christianity as the religion of atheism. Christ's death as the
    death of God himself. The Holy Spirit as the community of believers.
    """
    vertices = [
        Vertex("fragile_absolute", content="the fragile absolute — the absolute as inherently fragile"),
        Vertex("Christianity", content="Christianity — the religion that implies its own overcoming"),
        Vertex("death_of_God", content="death of God — God dies to himself on the cross"),
    ]
    edges = [
        Edge("fragile_absolute", "failed_absolute", EdgeType.REFERENCE),
        Edge("Christianity", "big_Other", EdgeType.NEGATION),
        Edge("death_of_God", "Christianity", EdgeType.DEPENDENCY),
        Edge("death_of_God", "negation_of_negation", EdgeType.REFERENCE),
        Edge("fragile_absolute", "Hegel", EdgeType.DEPENDENCY),
        Edge("Christianity", "universality", EdgeType.REFERENCE),
    ]
    return "2000: The Fragile Absolute", vertices, edges


def work_19_puppet_and_dwarf():
    """2003: The Puppet and the Dwarf — The Perverse Core of Christianity.

    The materialist theology. The 'perverse core' of Christianity:
    God who does not believe in himself. Against New Age spirituality.
    """
    vertices = [
        Vertex("perverse_core", content="perverse core — the materialist kernel within theology"),
        Vertex("belief", content="belief — 'the big Other does not exist'"),
    ]
    edges = [
        Edge("perverse_core", "Christianity", EdgeType.DEPENDENCY),
        Edge("perverse_core", "materialism", EdgeType.DEPENDENCY),
        Edge("belief", "big_Other", EdgeType.NEGATION),
        Edge("belief", "ideology", EdgeType.DEPENDENCY),
        Edge("perverse_core", "death_of_God", EdgeType.REFERENCE),
    ]
    return "2003: The Puppet and the Dwarf", vertices, edges


def work_20_organs_without_bodies():
    """2004: Organs without Bodies — On Deleuze and Consequences.

    Žižek vs Deleuze. The 'organs without bodies' (inversion of BwO).
    Deleuze as a closet Hegelian. Virtuality and the act.
    """
    vertices = [
        Vertex("organs_without_bodies", content="organs without bodies — the Žižekian inversion of Deleuze"),
        Vertex("virtuality", content="virtuality — the virtual as the Real, not possibility"),
    ]
    edges = [
        Edge("organs_without_bodies", "drive", EdgeType.DEPENDENCY),
        Edge("organs_without_bodies", "Real", EdgeType.REFERENCE),
        Edge("virtuality", "Real", EdgeType.DEPENDENCY),
        Edge("virtuality", "ontological_incompleteness", EdgeType.REFERENCE),
        Edge("organs_without_bodies", "Hegel", EdgeType.REFERENCE),
    ]
    return "2004: Organs without Bodies", vertices, edges


def work_21_monstrosity_of_christ():
    """2009: The Monstrosity of Christ (with John Milbank).

    Debate with Milbank. Žižek's materialist Christianity vs
    Milbank's radical orthodoxy. The monstrous Christ = the gap.
    """
    vertices = [
        Vertex("monstrous_Christ", content="the monstrous Christ — the gap incarnate"),
    ]
    edges = [
        Edge("monstrous_Christ", "Christianity", EdgeType.DEPENDENCY),
        Edge("monstrous_Christ", "parallax_gap", EdgeType.DEPENDENCY),
        Edge("monstrous_Christ", "death_of_God", EdgeType.REFERENCE),
        Edge("monstrous_Christ", "failed_absolute", EdgeType.REFERENCE),
    ]
    return "2009: The Monstrosity of Christ", vertices, edges


def work_22_event():
    """2014: Event — A Philosophical Journey Through a Concept.

    What is an event? The three modalities: frame-changing,
    religious conversion, the fall in love, the revolutionary rupture.
    """
    vertices = [
        Vertex("event", content="the Event — the rupture that retroactively changes its conditions"),
    ]
    edges = [
        Edge("event", "act", EdgeType.DEPENDENCY),
        Edge("event", "retroactivity", EdgeType.DEPENDENCY),
        Edge("event", "Real", EdgeType.DEPENDENCY),
        Edge("event", "revolution", EdgeType.REFERENCE),
    ]
    return "2014: Event", vertices, edges


def work_23_incontinence_of_void():
    """2017: Incontinence of the Void — Economico-Philosophical Spandrels.

    Late synthesis — the void that cannot contain itself. Spandrels
    as unintended byproducts that become essential. Hegel + Marx + Lacan
    re-fusion at the level of ontological incompleteness.
    """
    vertices = [
        Vertex("incontinence_of_void", content="incontinence of the void — the void overflows, producing reality"),
        Vertex("spandrel", content="spandrel — the necessary byproduct that becomes the main thing"),
    ]
    edges = [
        Edge("incontinence_of_void", "void", EdgeType.DEPENDENCY),
        Edge("incontinence_of_void", "ontological_incompleteness", EdgeType.DEPENDENCY),
        Edge("spandrel", "absolute_recoil", EdgeType.REFERENCE),
        Edge("spandrel", "retroactivity", EdgeType.REFERENCE),
        Edge("incontinence_of_void", "less_than_nothing", EdgeType.REFERENCE),
        Edge("incontinence_of_void", "dialectical_materialism", EdgeType.DEPENDENCY),
    ]
    return "2017: Incontinence of the Void", vertices, edges


def work_24_hegel_in_wired_brain():
    """2020: Hegel in a Wired Brain.

    Neuralink and the Hegelian subject. Digital control, brain-machine
    interface, the fantasy of direct thought-reading. Why Hegel matters
    for AI and neural technology.
    """
    vertices = [
        Vertex("wired_brain", content="the wired brain — direct neural interface as the fantasy of full transparency"),
    ]
    edges = [
        Edge("wired_brain", "cyberspace", EdgeType.DEPENDENCY),
        Edge("wired_brain", "Hegel", EdgeType.DEPENDENCY),
        Edge("wired_brain", "subject", EdgeType.DEPENDENCY),
        Edge("wired_brain", "fantasy", EdgeType.NEGATION),
        Edge("wired_brain", "Real", EdgeType.REFERENCE),
    ]
    return "2020: Hegel in a Wired Brain", vertices, edges


def work_25_pandemic():
    """2020: Pandemic! — COVID-19 Shakes the World.

    The pandemic as an event that exposes systemic violence. Against
    both conservative denial and liberal moralism. The virus as Real.
    """
    vertices = [
        Vertex("pandemic", content="the pandemic — the event that exposes the Real of global capitalism"),
    ]
    edges = [
        Edge("pandemic", "event", EdgeType.DEPENDENCY),
        Edge("pandemic", "capitalism", EdgeType.DEPENDENCY),
        Edge("pandemic", "systemic_violence", EdgeType.REFERENCE),
        Edge("pandemic", "Real", EdgeType.REFERENCE),
        Edge("pandemic", "ideology", EdgeType.NEGATION),
    ]
    return "2020: Pandemic!", vertices, edges


# ---------------------------------------------------------------------------
# Work list (chronological)
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_01_sublime_object,            # 1989
    work_02_they_know_not,             # 1991
    work_06_looking_awry,              # 1991
    work_07_enjoy_your_symptom,        # 1992
    work_03_tarrying_negative,         # 1993
    work_04_plague_of_fantasies,       # 1997
    work_05_ticklish_subject,          # 1999
    work_18_fragile_absolute,          # 2000
    work_19_puppet_and_dwarf,          # 2003
    work_20_organs_without_bodies,     # 2004
    work_08_parallax_view,             # 2006
    work_15_in_defense_lost_causes,    # 2008
    work_13_violence,                  # 2008
    work_14_first_as_tragedy,          # 2009
    work_21_monstrosity_of_christ,     # 2009
    work_12_living_end_times,          # 2010
    work_09_less_than_nothing,         # 2012
    work_10_absolute_recoil,           # 2014
    work_22_event,                     # 2014
    work_17_disparities,              # 2016
    work_16_courage_of_hopelessness,   # 2017
    work_23_incontinence_of_void,      # 2017
    work_11_sex_failed_absolute,       # 2020
    work_24_hegel_in_wired_brain,      # 2020
    work_25_pandemic,                  # 2020
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("SLAVOJ ŽIŽEK — COLLECTED WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "zizek_collected_works_incremental",
        "source": "Slavoj Žižek — collected works, 25 entries (1989-2020)",
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

    output_path = "experiment_zizek.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
