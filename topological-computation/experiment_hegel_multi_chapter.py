"""Hegel multi-chapter experiment: Preface + Introduction + Consciousness (I-III).

Two-path approach:
  Path A: phi_L on assembled key sentences from 5 chapters
  Path B: Manually constructed dialectical complex encoding the argument structure

If Path A yields sufficient edge density (>0.5 edges/vertex), use that complex.
Otherwise, use Path B (the manual complex).
"""

from __future__ import annotations

import json
import sys
sys.path.insert(0, '.')

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Text: Key sentences from 5 chapters of Phenomenology of Spirit
# (A.V. Miller translation, marxists.org public domain)
# ---------------------------------------------------------------------------

PREFACE_TEXT = """
The truth is the whole. The whole is nothing other than the essence consummating itself through its development. Of the Absolute it must be said that it is essentially a result, that only in the end is it what it truly is. Its nature is to be actual, subject, the spontaneous becoming of itself.

Everything depends on grasping and expressing the ultimate truth not as substance but as subject. The living substance is that being which is in truth subject, or what is the same thing, is in truth actual. The truth of a thing is not its immediate being but its mediated reality.

The bud disappears in the bursting-forth of the blossom, and one might say that the former is refuted by the latter. Similarly the fruit declares the blossom to be a false existence of the plant, and the fruit replaces the blossom as the truth of the plant. These forms are distinguished from one another, but at the same time each is necessary. This mutual necessity constitutes the life of the whole.

The True is thus the Bacchanalian revel in which no member is not drunk. Yet because each member collapses as soon as he drops out, the revel is just as much transparent and simple repose. The dissolution of the distinction is the simple whole.

The power of spirit is only as great as its expression, its depth only as deep as it dares to spread out and lose itself in its exposition. The appearance is the arising and passing away that does not itself arise and pass away, but is in itself and constitutes the actuality and the movement of the life of truth.

What is rational is actual; what is actual is rational. The spirit that knows itself as spirit is science. Science is the crown; it is the spirit that knows itself in the shape of spirit. Spirit which knows its own self is science. Science is its actuality and the kingdom which it builds for itself in its own element.

Dogmatism in the manner of thinking consists in the conviction that the true consists in a proposition which is a fixed result, or which is directly known. To questions like when was Caesar born, or how many feet make a plethora, a precise answer should be given. But the nature of a so-called truth of that kind is different from the nature of philosophical truths.

The False in the mode of truth is a moment of the truth. The form of a proposition destroys its inner meaning. The proposition or judgment is abstract, fixed, and finite. What has been said can be expressed by saying that the nature of judgment or the proposition in general, which involves the distinction of subject and predicate, is destroyed by the speculative proposition.

The method of the whole movement of the notion will not be something distinct from its object and content; for it is the content in itself, the dialectic which it possesses within itself, which is the mainspring of its advance. It is clear that no expositions can be accepted as scientifically valid which do not pursue this path.
"""

INTRODUCTION_TEXT = """
It is a natural assumption that in philosophy, before we start to deal with its proper subject-matter, viz. the actual cognition of what truly is, one must first of all come to an understanding about cognition. Cognition is regarded either as the instrument to get hold of the Absolute, or as the medium through which one discovers it.

But if we are to concern ourselves with this fear that errs, we should have to enquire whether this fear of error is not already the error itself. For if cognition is the instrument for getting hold of absolute being, the suggestion immediately presents itself that the use of an instrument on a thing certainly does not let it be what it is for itself.

Consciousness simultaneously distinguishes itself from something and at the same time relates itself to it. There is something for consciousness and the knowledge of the object is something different. On this distinction rests the concept of experience.

Inasmuch as the new true object issues from it, this dialectical movement which consciousness exercises on itself and which affects both its knowledge and its object, is precisely what is called experience. Consciousness produces this new object out of itself.

The series of configurations which consciousness goes through along this road is the detailed history of the education of consciousness itself to the level of science. Natural consciousness proves itself to be only the concept of knowledge, or not real knowledge. Since it takes itself to be real knowledge, this path has a negative significance for it.

What is called the fear of error reveals itself as the fear of truth. The Absolute alone is true. Or the truth alone is absolute. But this does not mean that the Absolute is just sitting there waiting for us.

The exposition of untrue consciousness in its untruth is not merely a negative procedure. Natural consciousness will show itself to be only the concept of knowledge. Unreal knowledge, at the same time, necessarily advances towards true knowledge. What appears to be the end is already there in the beginning, as a hidden tendency.
"""

SENSE_CERTAINTY_TEXT = """
The knowledge or knowing which is at the beginning or is immediately our object cannot be anything else but immediate knowledge itself, a knowledge of the immediate or of what simply is. Our approach to the object must also be immediate or receptive; we must alter nothing in the object as it presents itself. In apprehending it, we must refrain from trying to comprehend it.

What is this? What is the Now? Let us answer this: Now is Night. In order to test the truth of this sense-certainty a simple experiment will suffice. We write down this truth; a truth cannot lose anything by being written down. If now, this noon, we look again at the written truth we shall have to say that it has become stale.

The Now that is Night is preserved, but as something that is not Night; likewise, the Now that is Day is preserved as something that is also not Day. The Now, and pointing out the Now, are thus so constituted that neither the one nor the other is something immediate and simple, but a movement which contains various moments.

The This is thus established as not This, or as something superseded; and hence not as Nothing, but as a determinate Nothing, the Nothing of a specific content, namely, of the This. As thus negated, the sense-element is still present, but not as it was supposed to be in immediate certainty.

The This is posited as not This, or as superseded; but what is superseded is at the same time preserved. The This has shown itself to be a mediated simplicity, or a universality.

Sense-certainty thus comes to know by experience that its essential nature is neither in the object nor in the I, and that its immediacy is neither an immediacy of the one nor of the other. What I do mean, what I do perceive, is not this or that particular thing, but a universal.

The truth of immediate certainty is the universal, and what was meant to be the most concrete turns out to be the most abstract.

Universal language says that this is, that is, and it says what is this or what is that. But language expresses the universal. What I merely mean, I alone mean. What cannot be said, the thing which is meant, cannot be reached by language, which expresses only the universal.

The force of its truth thus lies now in the I, in the immediacy of my seeing, hearing, and so on. But the I that sees is also a universal. This particular I that sees is also a universal I, just as this particular Now and Here are universals.

When I say 'I', this singular 'I', I say in general all 'I's; everyone is what I say, everyone is 'I', this singular 'I'. The same applies equally to 'Now' and 'Here'. They are universals.

Sense-certainty has thus been driven from the object and from the I to its own self. What remains is the pure relating of the immediate to itself, a pure being which constitutes the truth of sense-certainty. This pure being is the universal.
"""

PERCEPTION_TEXT = """
Perception takes as its object what sense-certainty has left behind: the universal as its principle. Perception is a universality grounded in negation. The object has the character of a thing with many properties.

The single property is its own specific property; it is independent and free from the other properties. But the thing is the togetherness of many properties. The thing is the Also, the general medium in which many properties subsist as sensuous universals, each for itself and excluding the others.

Simplicity is thinghood or pure essence. The thing is the One, the excluding unity. The One is the moment of negation; it is self-relation through excluding an other. Through this the thing constitutes itself as a determinate being.

Salt is white, and also pungent, and also cubical. All these properties are in one simple Here, and they interpenetrate each other. The whiteness does not affect the pungency, nor does either affect the cubical shape. Each is a simple relating to itself that leaves the others alone.

The thing is the One, but also it is the Also. These two moments constitute the thing. But they contradict each other. The One excludes, the Also includes. The thing thus has a contradictory nature.

Consciousness discovers that it must take on itself the contradictory nature of the thing. It tries attributing diversity to its own perception while keeping the object unified. But this pushes the contradiction into consciousness itself.

The property must be a free matter, existing on its own account. But then the thing dissolves into merely a surface enclosure. If the thing is the One, it excludes other things. Each thing defines itself against others through its determinate properties.

But each property is also a universal, related to other properties through negation. White is white only insofar as it is opposed to black. The property itself is therefore self-contradictory: it is independent and at the same time dependent on its opposite.

We thus see that the thing collapses through its own essential property. What emerges is the unconditioned universal. The sensible thing gives way to the universal which is no longer the abstract universal of sense-certainty, but a concrete universal containing negation and determination within itself.

Sound common sense operates within the very abstractions it claims to despise. It employs qualifications like insofar as and in this respect to paper over contradictions. Philosophy recognizes these contradictions as the essential movement of thought.
"""

FORCE_UNDERSTANDING_TEXT = """
The unconditioned universal is first the inner being of the thing. Understanding has for its object the concept that has emerged from perception. The universal is no longer a mere property but the essence underlying appearance.

Force is the universal which remains with itself. But force must also express itself. Force proper and the expression of force are two moments which are distinct yet inseparable. Force is driven into expression and expression is driven back into force.

Force exists only in its expression. Yet the expression is supposed to be different from force itself. This is the fundamental contradiction of force: it is what it is only through its other, yet the other is supposed to be different from it.

The play of forces consists in the mutual determining of the two sides. One force incites the other to expression, but each side is itself both inciting and incited. The distinction between them constantly collapses.

What remains stable in this flux is Law. Law is the stable image of unstable appearance. In the law, understanding finds the quiet realm of universality beneath the restless play of forces.

But explanation through law is tautological. The force of attraction explains why a stone falls, but attraction is nothing other than the stone falling. Understanding merely redoubles the same content in two forms without advancing.

The inner world must be grasped as the supersensible world. Initially it seems to be a tranquil kingdom of laws, a permanent image behind the flux of appearance. But this first inner world is merely the immediate raising of the perceived world into the universal.

The second supersensible world is the inverted world. What is sweet in the first world is sour in the second. What is the north pole of a magnet in the world of appearance is the south pole in the inner world. What is crime in the first is punishment in the second.

This inversion must not be understood as if there were two separate substances. The distinction is internal. The selfsame is repelled from itself; and the selfunlike is attracted to itself. The essential nature of the thing is internal distinction within identity.

This simple infinity is the absolute concept. Infinity is self-related negation. Infinite life is the selfsame which repels itself from itself and the like which attracts itself. This is the movement of consciousness becoming self-consciousness.

When consciousness looks into the inner world, it finds nothing but itself looking back. The curtain that hangs before the inner world is drawn away, and there is nothing to be seen behind it unless we ourselves go behind it. Behind the curtain there is nothing to be seen unless we ourselves go behind there, as much in order that there may be something to see, as that there may be something behind there which can be seen.
"""


def build_multi_chapter_complex() -> Graph:
    """Construct a dialectical complex encoding all 5 chapters' argument structure.

    Organized by chapter, with cross-chapter connections capturing the
    progressive development of concepts.
    """
    vertices = [
        # ===== PREFACE =====
        Vertex("truth_whole", content="the truth is the whole"),
        Vertex("substance", content="substance"),
        Vertex("subject", content="subject (self-developing)"),
        Vertex("bud", content="the bud"),
        Vertex("blossom", content="the blossom"),
        Vertex("fruit", content="the fruit"),
        Vertex("mutual_necessity", content="mutual necessity"),
        Vertex("bacchanalian_revel", content="Bacchanalian revel"),
        Vertex("simple_repose", content="transparent simple repose"),
        Vertex("spirit", content="spirit"),
        Vertex("science", content="science"),
        Vertex("dogmatism", content="dogmatism"),
        Vertex("speculative_proposition", content="speculative proposition"),
        Vertex("dialectic_method", content="dialectic as method"),
        Vertex("fixed_result", content="fixed result/proposition"),
        Vertex("false_moment", content="the false as moment of truth"),
        Vertex("appearance_truth", content="appearance as actuality"),

        # ===== INTRODUCTION =====
        Vertex("cognition_instrument", content="cognition as instrument"),
        Vertex("cognition_medium", content="cognition as medium"),
        Vertex("absolute_being", content="the Absolute"),
        Vertex("fear_of_error", content="fear of error"),
        Vertex("fear_of_truth", content="fear of truth"),
        Vertex("experience", content="experience (dialectical movement)"),
        Vertex("natural_consciousness", content="natural consciousness"),
        Vertex("real_knowledge", content="real knowledge"),
        Vertex("concept_of_knowledge", content="concept of knowledge"),
        Vertex("negative_procedure", content="negative procedure"),
        Vertex("hidden_tendency", content="hidden tendency toward truth"),

        # ===== SENSE-CERTAINTY =====
        Vertex("sense_certainty", content="sense-certainty"),
        Vertex("immediate_knowledge", content="immediate knowledge"),
        Vertex("the_now", content="the Now"),
        Vertex("now_night", content="Now is Night"),
        Vertex("now_day", content="Now is Day"),
        Vertex("now_universal", content="Now as universal"),
        Vertex("the_this", content="the This"),
        Vertex("the_here", content="the Here"),
        Vertex("this_not_this", content="This as not-This (superseded)"),
        Vertex("determinate_nothing", content="determinate Nothing"),
        Vertex("mediated_simplicity", content="mediated simplicity"),
        Vertex("the_I", content="the I"),
        Vertex("singular_I", content="singular I"),
        Vertex("universal_I", content="universal I"),
        Vertex("language_sc", content="language (expresses universal)"),
        Vertex("what_is_meant", content="what is meant (das Meinen)"),
        Vertex("pure_being", content="pure being"),
        Vertex("universal", content="the universal"),
        Vertex("particular", content="the particular"),
        Vertex("immediacy", content="immediacy"),
        Vertex("mediation", content="mediation"),

        # ===== PERCEPTION =====
        Vertex("perception", content="perception"),
        Vertex("thing", content="the thing"),
        Vertex("many_properties", content="many properties"),
        Vertex("thinghood", content="thinghood (pure essence)"),
        Vertex("the_one", content="the One (excluding unity)"),
        Vertex("the_also", content="the Also (general medium)"),
        Vertex("salt_example", content="salt (white, pungent, cubical)"),
        Vertex("one_excludes", content="One excludes"),
        Vertex("also_includes", content="Also includes"),
        Vertex("contradictory_thing", content="contradictory nature of thing"),
        Vertex("free_matter", content="free matter/property"),
        Vertex("surface_enclosure", content="surface enclosure"),
        Vertex("white_vs_black", content="white opposed to black"),
        Vertex("property_contradiction", content="property as self-contradictory"),
        Vertex("unconditioned_universal", content="unconditioned universal"),
        Vertex("concrete_universal", content="concrete universal"),
        Vertex("sound_common_sense", content="sound common sense"),
        Vertex("insofar_as", content="insofar-as qualification"),

        # ===== FORCE AND UNDERSTANDING =====
        Vertex("inner_being", content="inner being of thing"),
        Vertex("force", content="force"),
        Vertex("force_expression", content="expression of force"),
        Vertex("force_proper", content="force proper"),
        Vertex("play_of_forces", content="play of forces"),
        Vertex("inciting", content="inciting force"),
        Vertex("incited", content="incited force"),
        Vertex("law", content="law (stable image)"),
        Vertex("appearance_flux", content="appearance (flux)"),
        Vertex("explanation_tautology", content="explanation as tautology"),
        Vertex("supersensible_world", content="supersensible world"),
        Vertex("kingdom_of_laws", content="tranquil kingdom of laws"),
        Vertex("inverted_world", content="inverted world"),
        Vertex("sweet_sour", content="sweet becomes sour"),
        Vertex("crime_punishment", content="crime becomes punishment"),
        Vertex("internal_distinction", content="internal distinction"),
        Vertex("simple_infinity", content="simple infinity"),
        Vertex("self_consciousness_emerges", content="self-consciousness emerges"),
        Vertex("curtain", content="the curtain"),
        Vertex("nothing_behind", content="nothing behind unless we go there"),
        Vertex("self_related_negation", content="self-related negation"),
    ]

    edges = [
        # ===== PREFACE INTERNAL =====
        # Subject supersedes substance
        Edge("subject", "substance", EdgeType.NEGATION),
        Edge("substance", "subject", EdgeType.DEPENDENCY),
        Edge("truth_whole", "subject", EdgeType.DEPENDENCY),
        Edge("truth_whole", "substance", EdgeType.REFERENCE),

        # Bud-blossom-fruit dialectic
        Edge("blossom", "bud", EdgeType.NEGATION),
        Edge("fruit", "blossom", EdgeType.NEGATION),
        Edge("mutual_necessity", "bud", EdgeType.DEPENDENCY),
        Edge("mutual_necessity", "blossom", EdgeType.DEPENDENCY),
        Edge("mutual_necessity", "fruit", EdgeType.DEPENDENCY),

        # Bacchanalian revel = dissolution + repose
        Edge("bacchanalian_revel", "simple_repose", EdgeType.NEGATION),
        Edge("simple_repose", "bacchanalian_revel", EdgeType.NEGATION),

        # Spirit and science
        Edge("science", "spirit", EdgeType.DEPENDENCY),
        Edge("spirit", "truth_whole", EdgeType.DEPENDENCY),

        # Dogmatism vs speculative
        Edge("dogmatism", "fixed_result", EdgeType.DEPENDENCY),
        Edge("speculative_proposition", "dogmatism", EdgeType.NEGATION),
        Edge("speculative_proposition", "dialectic_method", EdgeType.DEPENDENCY),

        # False as moment
        Edge("false_moment", "truth_whole", EdgeType.DEPENDENCY),
        Edge("fixed_result", "false_moment", EdgeType.NEGATION),

        # Appearance
        Edge("appearance_truth", "truth_whole", EdgeType.REFERENCE),

        # ===== INTRODUCTION INTERNAL =====
        # Cognition problem
        Edge("cognition_instrument", "absolute_being", EdgeType.DEPENDENCY),
        Edge("cognition_medium", "absolute_being", EdgeType.DEPENDENCY),
        Edge("cognition_instrument", "cognition_medium", EdgeType.NEGATION),

        # Fear
        Edge("fear_of_error", "fear_of_truth", EdgeType.NEGATION),
        Edge("fear_of_truth", "fear_of_error", EdgeType.NEGATION),

        # Experience
        Edge("experience", "natural_consciousness", EdgeType.DEPENDENCY),
        Edge("natural_consciousness", "concept_of_knowledge", EdgeType.DEPENDENCY),
        Edge("natural_consciousness", "real_knowledge", EdgeType.NEGATION),
        Edge("concept_of_knowledge", "real_knowledge", EdgeType.NEGATION),

        # Negative procedure
        Edge("negative_procedure", "natural_consciousness", EdgeType.REFERENCE),
        Edge("hidden_tendency", "experience", EdgeType.DEPENDENCY),
        Edge("hidden_tendency", "real_knowledge", EdgeType.REFERENCE),

        # ===== SENSE-CERTAINTY INTERNAL =====
        Edge("sense_certainty", "immediate_knowledge", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "the_this", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "the_I", EdgeType.DEPENDENCY),
        Edge("immediate_knowledge", "immediacy", EdgeType.DEPENDENCY),

        # Now dialectic
        Edge("the_now", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("now_night", "the_now", EdgeType.DEPENDENCY),
        Edge("now_day", "the_now", EdgeType.DEPENDENCY),
        Edge("now_night", "now_day", EdgeType.NEGATION),
        Edge("now_day", "now_night", EdgeType.NEGATION),
        Edge("now_universal", "now_night", EdgeType.SUBLATION),
        Edge("now_universal", "now_day", EdgeType.SUBLATION),
        Edge("now_universal", "universal", EdgeType.DEPENDENCY),

        # This dialectic
        Edge("the_this", "particular", EdgeType.DEPENDENCY),
        Edge("this_not_this", "the_this", EdgeType.NEGATION),
        Edge("determinate_nothing", "this_not_this", EdgeType.DEPENDENCY),
        Edge("mediated_simplicity", "this_not_this", EdgeType.SUBLATION),
        Edge("mediated_simplicity", "the_this", EdgeType.SUBLATION),
        Edge("mediated_simplicity", "universal", EdgeType.DEPENDENCY),

        # Here
        Edge("the_here", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("the_here", "particular", EdgeType.DEPENDENCY),

        # I dialectic
        Edge("the_I", "singular_I", EdgeType.DEPENDENCY),
        Edge("singular_I", "universal_I", EdgeType.NEGATION),
        Edge("universal_I", "singular_I", EdgeType.NEGATION),
        Edge("universal_I", "universal", EdgeType.DEPENDENCY),

        # Language
        Edge("language_sc", "universal", EdgeType.DEPENDENCY),
        Edge("what_is_meant", "particular", EdgeType.DEPENDENCY),
        Edge("language_sc", "what_is_meant", EdgeType.NEGATION),

        # Core dialectical tension
        Edge("particular", "universal", EdgeType.NEGATION),
        Edge("universal", "particular", EdgeType.NEGATION),
        Edge("immediacy", "mediation", EdgeType.NEGATION),
        Edge("mediation", "immediacy", EdgeType.NEGATION),

        # Pure being resolution
        Edge("pure_being", "universal", EdgeType.DEPENDENCY),
        Edge("pure_being", "sense_certainty", EdgeType.REFERENCE),

        # ===== PERCEPTION INTERNAL =====
        Edge("perception", "universal", EdgeType.DEPENDENCY),
        Edge("perception", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("thing", "many_properties", EdgeType.DEPENDENCY),
        Edge("thing", "thinghood", EdgeType.DEPENDENCY),

        # One vs Also
        Edge("the_one", "thing", EdgeType.DEPENDENCY),
        Edge("the_also", "thing", EdgeType.DEPENDENCY),
        Edge("the_one", "the_also", EdgeType.NEGATION),
        Edge("the_also", "the_one", EdgeType.NEGATION),
        Edge("one_excludes", "the_one", EdgeType.DEPENDENCY),
        Edge("also_includes", "the_also", EdgeType.DEPENDENCY),
        Edge("one_excludes", "also_includes", EdgeType.NEGATION),

        # Salt example
        Edge("salt_example", "many_properties", EdgeType.REFERENCE),
        Edge("salt_example", "the_also", EdgeType.REFERENCE),

        # Contradictory thing
        Edge("contradictory_thing", "the_one", EdgeType.DEPENDENCY),
        Edge("contradictory_thing", "the_also", EdgeType.DEPENDENCY),

        # Property contradiction
        Edge("free_matter", "many_properties", EdgeType.DEPENDENCY),
        Edge("surface_enclosure", "free_matter", EdgeType.NEGATION),
        Edge("white_vs_black", "property_contradiction", EdgeType.REFERENCE),
        Edge("property_contradiction", "free_matter", EdgeType.DEPENDENCY),
        Edge("property_contradiction", "the_one", EdgeType.REFERENCE),

        # Resolution: unconditioned universal
        Edge("unconditioned_universal", "contradictory_thing", EdgeType.SUBLATION),
        Edge("unconditioned_universal", "property_contradiction", EdgeType.SUBLATION),
        Edge("concrete_universal", "unconditioned_universal", EdgeType.DEPENDENCY),
        Edge("concrete_universal", "universal", EdgeType.DEPENDENCY),
        Edge("concrete_universal", "particular", EdgeType.DEPENDENCY),

        # Common sense critique
        Edge("sound_common_sense", "insofar_as", EdgeType.DEPENDENCY),
        Edge("insofar_as", "contradictory_thing", EdgeType.REFERENCE),

        # ===== FORCE AND UNDERSTANDING INTERNAL =====
        Edge("inner_being", "thing", EdgeType.DEPENDENCY),
        Edge("inner_being", "unconditioned_universal", EdgeType.REFERENCE),

        # Force dialectic
        Edge("force", "force_expression", EdgeType.NEGATION),
        Edge("force_expression", "force", EdgeType.NEGATION),
        Edge("force_proper", "force", EdgeType.DEPENDENCY),
        Edge("force_expression", "force_proper", EdgeType.NEGATION),

        # Play of forces
        Edge("play_of_forces", "inciting", EdgeType.DEPENDENCY),
        Edge("play_of_forces", "incited", EdgeType.DEPENDENCY),
        Edge("inciting", "incited", EdgeType.NEGATION),
        Edge("incited", "inciting", EdgeType.NEGATION),

        # Law
        Edge("law", "play_of_forces", EdgeType.DEPENDENCY),
        Edge("law", "appearance_flux", EdgeType.DEPENDENCY),
        Edge("appearance_flux", "play_of_forces", EdgeType.REFERENCE),
        Edge("explanation_tautology", "law", EdgeType.NEGATION),

        # Supersensible world
        Edge("supersensible_world", "law", EdgeType.DEPENDENCY),
        Edge("kingdom_of_laws", "supersensible_world", EdgeType.DEPENDENCY),

        # Inverted world
        Edge("inverted_world", "supersensible_world", EdgeType.NEGATION),
        Edge("sweet_sour", "inverted_world", EdgeType.REFERENCE),
        Edge("crime_punishment", "inverted_world", EdgeType.REFERENCE),

        # Internal distinction
        Edge("internal_distinction", "inverted_world", EdgeType.SUBLATION),
        Edge("internal_distinction", "supersensible_world", EdgeType.SUBLATION),

        # Infinity
        Edge("simple_infinity", "internal_distinction", EdgeType.DEPENDENCY),
        Edge("simple_infinity", "self_related_negation", EdgeType.DEPENDENCY),
        Edge("self_related_negation", "simple_infinity", EdgeType.REFERENCE),

        # Self-consciousness emerges
        Edge("self_consciousness_emerges", "simple_infinity", EdgeType.DEPENDENCY),
        Edge("curtain", "supersensible_world", EdgeType.REFERENCE),
        Edge("nothing_behind", "curtain", EdgeType.NEGATION),
        Edge("self_consciousness_emerges", "curtain", EdgeType.SUBLATION),
        Edge("self_consciousness_emerges", "nothing_behind", EdgeType.DEPENDENCY),

        # ===== CROSS-CHAPTER CONNECTIONS =====
        # Preface -> Introduction
        Edge("dialectic_method", "experience", EdgeType.REFERENCE),
        Edge("false_moment", "negative_procedure", EdgeType.REFERENCE),
        Edge("truth_whole", "absolute_being", EdgeType.REFERENCE),

        # Introduction -> Sense-Certainty
        Edge("natural_consciousness", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("experience", "sense_certainty", EdgeType.REFERENCE),
        Edge("real_knowledge", "immediate_knowledge", EdgeType.NEGATION),

        # Sense-Certainty -> Perception
        Edge("perception", "mediated_simplicity", EdgeType.DEPENDENCY),
        Edge("thing", "universal", EdgeType.DEPENDENCY),
        Edge("many_properties", "particular", EdgeType.REFERENCE),

        # Perception -> Force/Understanding
        Edge("force", "unconditioned_universal", EdgeType.DEPENDENCY),
        Edge("inner_being", "concrete_universal", EdgeType.DEPENDENCY),
        Edge("law", "universal", EdgeType.REFERENCE),

        # Self-consciousness back to spirit/science (Preface)
        Edge("self_consciousness_emerges", "spirit", EdgeType.REFERENCE),
        Edge("self_consciousness_emerges", "subject", EdgeType.REFERENCE),

        # Mediation thread (across all chapters)
        Edge("mediation", "experience", EdgeType.REFERENCE),
        Edge("mediation", "dialectic_method", EdgeType.REFERENCE),
        Edge("play_of_forces", "mediation", EdgeType.REFERENCE),

        # Negation thread
        Edge("self_related_negation", "particular", EdgeType.REFERENCE),
        Edge("determinate_nothing", "negative_procedure", EdgeType.REFERENCE),
    ]

    g = Graph()
    for v in vertices:
        g = g.add_vertex(v)
    for e in edges:
        g = g.add_edge(e)

    return g


def run_phi_L_path():
    """Path A: Run phi_L on assembled text."""
    from phi_L import phi_L, _format_graph

    combined_text = (
        PREFACE_TEXT + "\n"
        + INTRODUCTION_TEXT + "\n"
        + SENSE_CERTAINTY_TEXT + "\n"
        + PERCEPTION_TEXT + "\n"
        + FORCE_UNDERSTANDING_TEXT
    )

    print("=" * 70)
    print("PATH A: phi_L on multi-chapter text")
    print("=" * 70)

    graph = phi_L(combined_text)
    vertices = graph.active_vertex_ids()
    edges = graph.active_edges()
    beta_1 = compute_beta_1(graph)

    print(f"\nVertices: {len(vertices)}")
    print(f"Edges: {len(edges)}")
    edge_density = len(edges) / len(vertices) if vertices else 0
    print(f"Edge density: {edge_density:.3f} edges/vertex")
    print(f"beta_1: {beta_1}")

    # Edge type distribution
    type_counts = {}
    for e in edges:
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    print(f"\nEdge type distribution:")
    for et, count in sorted(type_counts.items()):
        print(f"  {et}: {count}")

    neg_count = type_counts.get("negation", 0)
    neg_density = neg_count / len(edges) if edges else 0
    print(f"\nNegation density: {neg_density:.4f}")

    return graph, {
        "vertices": len(vertices),
        "edges": len(edges),
        "edge_density": edge_density,
        "beta_1": beta_1,
        "edge_type_distribution": type_counts,
        "negation_density": neg_density,
    }


def run_manual_path():
    """Path B: Run traversal on manually constructed complex."""
    print("\n" + "=" * 70)
    print("PATH B: Manually constructed multi-chapter complex")
    print("=" * 70)

    graph = build_multi_chapter_complex()
    vertices = graph.active_vertex_ids()
    edges = graph.active_edges()
    beta_1 = compute_beta_1(graph)
    edge_density = len(edges) / len(vertices) if vertices else 0

    print(f"\nVertices: {len(vertices)}")
    print(f"Edges: {len(edges)}")
    print(f"Edge density: {edge_density:.3f} edges/vertex")
    print(f"beta_1 (initial): {beta_1}")

    # Edge type distribution
    type_counts = {}
    for e in edges:
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    print(f"\nEdge type distribution:")
    for et, count in sorted(type_counts.items()):
        print(f"  {et}: {count}")

    # Negation edges
    neg_edges = [e for e in edges if e.edge_type == EdgeType.NEGATION]
    verts = graph.vertices
    print(f"\nNegation edges ({len(neg_edges)}):")
    for e in neg_edges:
        print(f"  '{verts[e.source].content}' --[negation]--> '{verts[e.target].content}'")

    # Sublation edges
    sub_edges = [e for e in edges if e.edge_type == EdgeType.SUBLATION]
    print(f"\nSublation edges ({len(sub_edges)}):")
    for e in sub_edges:
        print(f"  '{verts[e.source].content}' --[sublation]--> '{verts[e.target].content}'")

    # Terrain
    terrain = compute_terrain(graph)
    terrain_counts = {"tree": 0, "critical": 0}
    for mark in terrain.values():
        terrain_counts[mark] = terrain_counts.get(mark, 0) + 1
    print(f"\nTerrain: {terrain_counts}")
    crit_ratio = terrain_counts["critical"] / (terrain_counts["tree"] + terrain_counts["critical"]) \
        if (terrain_counts["tree"] + terrain_counts["critical"]) > 0 else 0
    print(f"Critical edge ratio: {crit_ratio:.4f}")

    return graph, {
        "vertices": len(vertices),
        "edges": len(edges),
        "edge_density": edge_density,
        "beta_1_initial": beta_1,
        "edge_type_distribution": type_counts,
        "negation_count": len(neg_edges),
        "sublation_count": len(sub_edges),
        "terrain_distribution": terrain_counts,
        "critical_edge_ratio": crit_ratio,
    }


def run_traversal(graph: Graph, total_steps: int):
    """Run traversal engine on the given graph."""
    print("\n" + "=" * 70)
    print(f"TRAVERSAL: {total_steps} steps")
    print("=" * 70)

    # Start from sense_certainty (the natural starting point of the Phenomenology)
    start = "sense_certainty"
    if start not in [v for v in graph.active_vertex_ids()]:
        start = graph.active_vertex_ids()[0]

    engine = TraversalEngine(graph, start=start, settlement_threshold=10, seed=42)

    beta_trajectory = []
    encounter_counts = {}
    operation_counts = {}

    for i in range(total_steps):
        log = engine.run_step()
        encounter_counts[log.encounter] = encounter_counts.get(log.encounter, 0) + 1
        operation_counts[log.operation] = operation_counts.get(log.operation, 0) + 1

        if (i + 1) % 100 == 0:
            beta_trajectory.append({
                "step": i + 1,
                "beta_1": log.beta_1_after,
                "settled": log.settled_count,
                "vertices_active": log.vertices_active,
                "edges_active": log.edges_active,
            })
            print(f"  Step {i+1:5d}: beta_1={log.beta_1_after:3d}, "
                  f"settled={log.settled_count:2d}, "
                  f"V={log.vertices_active}, E={log.edges_active}")

    # Check growth and extend if needed
    last_beta = engine.logs[-1].beta_1_after
    beta_at_80pct = None
    check_step = int(total_steps * 0.8)
    for log_entry in engine.logs:
        if log_entry.step == check_step:
            beta_at_80pct = log_entry.beta_1_after
            break

    extended = False
    extended_to = total_steps
    if beta_at_80pct and last_beta > beta_at_80pct * 1.1:
        extend_to = total_steps + 3000
        print(f"\n  beta_1 still growing ({beta_at_80pct} -> {last_beta}), extending to {extend_to}...")
        extended = True
        for i in range(total_steps, extend_to):
            log = engine.run_step()
            encounter_counts[log.encounter] = encounter_counts.get(log.encounter, 0) + 1
            operation_counts[log.operation] = operation_counts.get(log.operation, 0) + 1
            if (i + 1) % 100 == 0:
                beta_trajectory.append({
                    "step": i + 1,
                    "beta_1": log.beta_1_after,
                    "settled": log.settled_count,
                    "vertices_active": log.vertices_active,
                    "edges_active": log.edges_active,
                })
                print(f"  Step {i+1:5d}: beta_1={log.beta_1_after:3d}, "
                      f"settled={log.settled_count:2d}, "
                      f"V={log.vertices_active}, E={log.edges_active}")
        extended_to = extend_to

    # -----------------------------------------------------------------------
    # Analysis
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("ANALYSIS")
    print("=" * 70)

    final_beta = engine.logs[-1].beta_1_after
    final_settled = engine.logs[-1].settled_count
    final_v = engine.logs[-1].vertices_active
    final_e = engine.logs[-1].edges_active

    print(f"\nFinal state after {extended_to} steps:")
    print(f"  beta_1: {final_beta}")
    print(f"  Settled cycles: {final_settled}")
    print(f"  Active vertices: {final_v}")
    print(f"  Active edges: {final_e}")

    print(f"\nEncounter distribution:")
    for enc, count in sorted(encounter_counts.items()):
        pct = count / extended_to * 100
        print(f"  {enc}: {count} ({pct:.1f}%)")

    print(f"\nOperation distribution:")
    for op, count in sorted(operation_counts.items()):
        pct = count / extended_to * 100
        print(f"  {op}: {count} ({pct:.1f}%)")

    # Beta trajectory visualization
    print(f"\nBeta_1 trajectory:")
    for entry in beta_trajectory:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"  {entry['step']:5d}: {entry['beta_1']:3d} {bar}")

    # Dialectical events
    neg_events = [log for log in engine.logs if "negate" in log.operation]
    sublate_events = [log for log in engine.logs if "sublate" in log.operation]
    fold_events = [log for log in engine.logs if "fold" in log.operation]
    blocked_events = [log for log in engine.logs if log.blocked]
    jumps = [log for log in engine.logs if log.delta_beta_1 > 0]

    print(f"\nDialectical events:")
    print(f"  Negation events: {len(neg_events)}")
    print(f"  Sublation events: {len(sublate_events)}")
    print(f"  Fold events: {len(fold_events)}")
    print(f"  Blocked events: {len(blocked_events)}")
    print(f"  Beta_1 jumps: {len(jumps)}")

    # Phase analysis: divide trajectory into thirds
    third = extended_to // 3
    early_jumps = [j for j in jumps if j.step <= third]
    mid_jumps = [j for j in jumps if third < j.step <= 2 * third]
    late_jumps = [j for j in jumps if j.step > 2 * third]

    early_ops = [l for l in engine.logs if l.step <= third and l.operation != "walk"]
    mid_ops = [l for l in engine.logs if third < l.step <= 2 * third and l.operation != "walk"]
    late_ops = [l for l in engine.logs if l.step > 2 * third and l.operation != "walk"]

    print(f"\n  Phase analysis (thirds):")
    print(f"    Early (1-{third}):")
    print(f"      beta_1 jumps: {len(early_jumps)}")
    print(f"      operations: {len(early_ops)}")
    print(f"    Mid ({third+1}-{2*third}):")
    print(f"      beta_1 jumps: {len(mid_jumps)}")
    print(f"      operations: {len(mid_ops)}")
    print(f"    Late ({2*third+1}-{extended_to}):")
    print(f"      beta_1 jumps: {len(late_jumps)}")
    print(f"      operations: {len(late_ops)}")

    # Development arc detection
    has_exploration = len(early_jumps) > 0
    has_contraction = len(fold_events) > 0
    has_settlement = final_settled > 0
    has_arc = has_exploration and has_contraction and has_settlement

    print(f"\n  Development arc:")
    print(f"    Exploration (beta_1 growth): {'YES' if has_exploration else 'NO'}")
    print(f"    Contraction (folds):         {'YES' if has_contraction else 'NO'}")
    print(f"    Settlement (stable cycles):  {'YES' if has_settlement else 'NO'}")
    print(f"    Full arc detected:           {'YES' if has_arc else 'NO'}")

    if jumps[:20]:
        print(f"\n  First 20 beta_1 jumps:")
        for log in jumps[:20]:
            pos_content = engine.k_active.vertex(log.position)
            pos_label = pos_content.content if pos_content else log.position
            print(f"    Step {log.step}: +{log.delta_beta_1} (op={log.operation}, "
                  f"at='{pos_label}')")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({len(engine.settlement.settled_cycles)}):")
        for sc in engine.settlement.settled_cycles[:15]:
            print(f"    settled@step={sc.settled_at_step}: {sorted(sc.edges)}")

    # Top 20 tension pairs (negation edges created during traversal)
    created_neg = [e for e in engine.k_active.active_edges()
                   if e.edge_type == EdgeType.NEGATION and e.created_at > 0]
    if created_neg:
        print(f"\n  Engine-discovered negation pairs ({len(created_neg)} total), top 20:")
        for e in created_neg[:20]:
            src = engine.k_active.vertex(e.source)
            tgt = engine.k_active.vertex(e.target)
            src_label = src.content if src else e.source
            tgt_label = tgt.content if tgt else e.target
            print(f"    step={e.created_at}: '{src_label}' vs '{tgt_label}'")

    # Negation density
    neg_count_final = sum(1 for e in engine.k_active.active_edges()
                         if e.edge_type == EdgeType.NEGATION)
    total_e_final = len(engine.k_active.active_edges())
    neg_density = neg_count_final / total_e_final if total_e_final > 0 else 0

    print(f"\n--- Comparison with Genealogy Data ---")
    print(f"  Hegel negation density:     {neg_density:.4f} ({neg_count_final}/{total_e_final})")
    print(f"  Genealogy negation density: 0.0164 (111/6751)")
    print(f"  Ratio: {neg_density / 0.0164:.2f}x" if neg_density > 0 else "  Ratio: 0x")

    encounter_density = sum(1 for log in engine.logs if log.encounter != "nothing") / extended_to
    print(f"  Encounter density: {encounter_density:.4f}")

    # Final terrain
    final_terrain = compute_terrain(engine.k_active)
    ft_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"]) \
        if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0
    print(f"  Final terrain: {ft_counts}")
    print(f"  Critical edge ratio: {crit_ratio:.4f}")

    return engine, {
        "total_steps": extended_to,
        "extended": extended,
        "final_beta_1": final_beta,
        "final_settled_cycles": final_settled,
        "final_vertices_active": final_v,
        "final_edges_active": final_e,
        "encounter_distribution": encounter_counts,
        "operation_distribution": operation_counts,
        "beta_1_trajectory": beta_trajectory,
        "negation_events": len(neg_events),
        "sublation_events": len(sublate_events),
        "fold_events": len(fold_events),
        "blocked_events": len(blocked_events),
        "beta_1_jumps": len(jumps),
        "phase_analysis": {
            "early_jumps": len(early_jumps),
            "mid_jumps": len(mid_jumps),
            "late_jumps": len(late_jumps),
            "early_ops": len(early_ops),
            "mid_ops": len(mid_ops),
            "late_ops": len(late_ops),
        },
        "development_arc": {
            "exploration": has_exploration,
            "contraction": has_contraction,
            "settlement": has_settlement,
            "full_arc": has_arc,
        },
        "negation_density": neg_density,
        "encounter_density": encounter_density,
        "terrain_distribution": ft_counts,
        "critical_edge_ratio": crit_ratio,
    }


def run_experiment():
    print("=" * 70)
    print("HEGEL MULTI-CHAPTER EXPERIMENT")
    print("Preface + Introduction + Sense-Certainty + Perception + Force/Understanding")
    print("=" * 70)

    # -----------------------------------------------------------------------
    # Path A: phi_L
    # -----------------------------------------------------------------------
    phi_graph, phi_stats = run_phi_L_path()

    # -----------------------------------------------------------------------
    # Path B: Manual complex
    # -----------------------------------------------------------------------
    manual_graph, manual_stats = run_manual_path()

    # -----------------------------------------------------------------------
    # Decision: which complex to traverse
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("PATH SELECTION")
    print("=" * 70)

    phi_density = phi_stats["edge_density"]
    manual_density = manual_stats["edge_density"]

    print(f"\n  phi_L: {phi_stats['vertices']}V / {phi_stats['edges']}E, density={phi_density:.3f}")
    print(f"  Manual: {manual_stats['vertices']}V / {manual_stats['edges']}E, density={manual_density:.3f}")

    # Use manual complex (it properly encodes the dialectical structure)
    # phi_L is good for simple texts but can't handle Hegel's prose
    use_graph = manual_graph
    path_used = "manual"
    vertex_count = manual_stats["vertices"]

    if phi_density >= 0.5 and phi_stats["vertices"] >= 50:
        use_graph = phi_graph
        path_used = "phi_L"
        vertex_count = phi_stats["vertices"]
        print(f"\n  -> Using phi_L output (sufficient density)")
    else:
        print(f"\n  -> Using manual complex (phi_L density {phi_density:.3f} < 0.5 or too few vertices)")

    # -----------------------------------------------------------------------
    # Traversal
    # -----------------------------------------------------------------------
    if vertex_count < 100:
        steps = 2000
    elif vertex_count <= 300:
        steps = 5000
    else:
        steps = 2000

    print(f"\n  Vertex count: {vertex_count} -> {steps} steps")

    engine, traversal_stats = run_traversal(use_graph, steps)

    # -----------------------------------------------------------------------
    # Save
    # -----------------------------------------------------------------------
    result = {
        "experiment": "hegel_multi_chapter",
        "text_source": "Hegel, Phenomenology of Spirit, A.V. Miller translation",
        "chapters": [
            "Preface",
            "Introduction",
            "A. Consciousness: I. Sense-Certainty",
            "A. Consciousness: II. Perception",
            "A. Consciousness: III. Force and Understanding",
        ],
        "methodology": f"Path used: {path_used}",
        "phi_L_output": phi_stats,
        "manual_complex": manual_stats,
        "path_used": path_used,
        "traversal": traversal_stats,
        "settled_cycles": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_hegel_multi_chapter.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    print(f"\n\nResults saved to {output_path}")
    return result


if __name__ == "__main__":
    run_experiment()
