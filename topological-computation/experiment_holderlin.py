"""Friedrich Hölderlin — collected works incremental traversal experiment.

Each work is injected as a conceptual complex (vertices + edges).
After injection, the traversal engine "digests" the work
(runs until beta_1 is stable for 50 consecutive steps).
Then the next work is injected.

This models the progressive development of Hölderlin's poetic
and philosophical thought as an incremental topological growth process.

Covers ~14 works spanning 1788-1806 (poetry, novels, drama, theory).
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
#   nature, divine, mortal, caesura, tragic_transport, aorgic, organic,
#   sobriety, fire, return, fatherland, poetic_dwelling, beauty,
#   love, separation, freedom, fate, Greece, Hesperien, spirit,
#   infinite, finite, being, judgment, unity, totality,
#   Empedocles, Christ, Dionysus, river, heaven, earth, night
# ---------------------------------------------------------------------------


def work_01_hymns_of_tubingen():
    """1788-1793: Tübingen Hymns (early period).

    Hymns to Freedom, Humanity, Harmony — youthful idealism,
    influence of Schiller and Klopstock, revolutionary enthusiasm.
    """
    vertices = [
        Vertex("freedom", content="freedom — political and spiritual liberation"),
        Vertex("humanity", content="humanity — the human ideal"),
        Vertex("harmony", content="harmony — unity of all things"),
        Vertex("beauty", content="beauty — the appearance of the absolute"),
        Vertex("nature", content="nature — the living totality"),
        Vertex("love", content="love — the force that binds"),
        Vertex("Greece", content="Greece — the ideal past"),
        Vertex("spirit", content="spirit — the animating principle"),
        Vertex("divine", content="the divine — das Göttliche"),
    ]
    edges = [
        Edge("freedom", "humanity", EdgeType.DEPENDENCY),
        Edge("beauty", "harmony", EdgeType.DEPENDENCY),
        Edge("nature", "beauty", EdgeType.DEPENDENCY),
        Edge("love", "harmony", EdgeType.DEPENDENCY),
        Edge("Greece", "beauty", EdgeType.REFERENCE),
        Edge("divine", "nature", EdgeType.DEPENDENCY),
        Edge("spirit", "freedom", EdgeType.DEPENDENCY),
        Edge("humanity", "harmony", EdgeType.DEPENDENCY),
        Edge("nature", "divine", EdgeType.REFERENCE),
    ]
    return "1788-93: Tübingen Hymns", vertices, edges


def work_02_hyperion_fragment():
    """1794: Fragment of Hyperion (Thalia version).

    First attempt at the novel — Hyperion's longing for Greece,
    the split between ideal and real.
    """
    vertices = [
        Vertex("Hyperion", content="Hyperion — the protagonist seeking unity"),
        Vertex("longing", content="Sehnsucht — longing for lost wholeness"),
        Vertex("separation", content="separation — Trennung from the All"),
        Vertex("infinite", content="the infinite — das Unendliche"),
        Vertex("finite", content="the finite — das Endliche"),
    ]
    edges = [
        Edge("Hyperion", "longing", EdgeType.DEPENDENCY),
        Edge("longing", "Greece", EdgeType.REFERENCE),
        Edge("separation", "longing", EdgeType.DEPENDENCY),
        Edge("infinite", "finite", EdgeType.NEGATION),
        Edge("Hyperion", "beauty", EdgeType.DEPENDENCY),
        Edge("Hyperion", "nature", EdgeType.REFERENCE),
        Edge("longing", "harmony", EdgeType.DEPENDENCY),
        Edge("separation", "harmony", EdgeType.NEGATION),
    ]
    return "1794: Fragment of Hyperion", vertices, edges


def work_03_judgment_and_being():
    """1795: Urteil und Seyn (Judgment and Being).

    Key philosophical fragment — judgment as Ur-teilung (original
    division), Being as prior to subject-object split.
    """
    vertices = [
        Vertex("judgment", content="Urteil — original division (Ur-teilung)"),
        Vertex("being", content="Being — Seyn, prior to subject-object"),
        Vertex("subject", content="subject — one side of the division"),
        Vertex("object", content="object — the other side of the division"),
        Vertex("unity", content="unity — the pre-reflective ground"),
        Vertex("intellectual_intuition", content="intellectual intuition — access to Being"),
    ]
    edges = [
        Edge("judgment", "being", EdgeType.NEGATION),
        Edge("judgment", "subject", EdgeType.DEPENDENCY),
        Edge("judgment", "object", EdgeType.DEPENDENCY),
        Edge("being", "unity", EdgeType.DEPENDENCY),
        Edge("subject", "object", EdgeType.NEGATION),
        Edge("intellectual_intuition", "being", EdgeType.DEPENDENCY),
        Edge("unity", "separation", EdgeType.NEGATION),
        Edge("being", "infinite", EdgeType.REFERENCE),
        Edge("beauty", "being", EdgeType.REFERENCE),
    ]
    return "1795: Judgment and Being (Urteil und Seyn)", vertices, edges


def work_04_poetic_spirit():
    """1795: On the Operations of the Poetic Spirit.

    Theory of poetry — the poetic spirit mediates between
    the infinite and finite through harmonische Entgegensetzung.
    """
    vertices = [
        Vertex("poetic_spirit", content="poetic spirit — mediating force of poetry"),
        Vertex("harmonious_opposition", content="harmonische Entgegensetzung — harmonious opposition"),
        Vertex("ideal", content="the ideal — transcendent ground"),
        Vertex("real", content="the real — empirical actuality"),
        Vertex("tone", content="tone — Grundton (naive, heroic, ideal)"),
    ]
    edges = [
        Edge("poetic_spirit", "harmonious_opposition", EdgeType.DEPENDENCY),
        Edge("harmonious_opposition", "infinite", EdgeType.DEPENDENCY),
        Edge("harmonious_opposition", "finite", EdgeType.DEPENDENCY),
        Edge("ideal", "real", EdgeType.NEGATION),
        Edge("poetic_spirit", "beauty", EdgeType.DEPENDENCY),
        Edge("tone", "poetic_spirit", EdgeType.DEPENDENCY),
        Edge("poetic_spirit", "nature", EdgeType.REFERENCE),
        Edge("poetic_spirit", "spirit", EdgeType.REFERENCE),
    ]
    return "1795: On the Operations of the Poetic Spirit", vertices, edges


def work_05_hyperion():
    """1797-1799: Hyperion, or The Hermit in Greece.

    The completed novel — Hyperion's journey through love (Diotima),
    revolution (Alabanda), loss, and the eccentric path (exzentrische Bahn).
    """
    vertices = [
        Vertex("Diotima", content="Diotima — love, beauty incarnate, tragic death"),
        Vertex("Alabanda", content="Alabanda — revolutionary action, violent path"),
        Vertex("eccentric_path", content="exzentrische Bahn — the eccentric orbit of life"),
        Vertex("all_in_one", content="Hen kai Pan — the One and All"),
        Vertex("return", content="return — Rückkehr, homecoming to nature"),
        Vertex("mortal", content="mortal — das Sterbliche, human finitude"),
        Vertex("fate", content="fate — Schicksal, the necessity of separation"),
    ]
    edges = [
        Edge("Hyperion", "Diotima", EdgeType.DEPENDENCY),
        Edge("Hyperion", "Alabanda", EdgeType.DEPENDENCY),
        Edge("Diotima", "beauty", EdgeType.DEPENDENCY),
        Edge("Diotima", "nature", EdgeType.DEPENDENCY),
        Edge("Alabanda", "freedom", EdgeType.DEPENDENCY),
        Edge("Alabanda", "Diotima", EdgeType.NEGATION),
        Edge("eccentric_path", "return", EdgeType.DEPENDENCY),
        Edge("all_in_one", "nature", EdgeType.DEPENDENCY),
        Edge("all_in_one", "divine", EdgeType.DEPENDENCY),
        Edge("Diotima", "mortal", EdgeType.REFERENCE),
        Edge("separation", "return", EdgeType.NEGATION),
        Edge("fate", "separation", EdgeType.DEPENDENCY),
        Edge("fate", "mortal", EdgeType.DEPENDENCY),
        Edge("love", "Diotima", EdgeType.REFERENCE),
        Edge("Hyperion", "eccentric_path", EdgeType.DEPENDENCY),
        Edge("return", "all_in_one", EdgeType.DEPENDENCY),
    ]
    return "1797-99: Hyperion", vertices, edges


def work_06_death_of_empedocles():
    """1798-1800: The Death of Empedocles (three versions).

    Tragic drama — Empedocles leaps into Etna, sacrificial death
    to reunite nature and art, organic and aorgic.
    """
    vertices = [
        Vertex("Empedocles", content="Empedocles — the philosopher who leaps into Etna"),
        Vertex("organic", content="the organic — human art, culture, consciousness"),
        Vertex("aorgic", content="the aorgic — nature's formless power, das Aorgische"),
        Vertex("sacrifice", content="sacrifice — Opfer, the self-dissolution"),
        Vertex("tragic_death", content="tragic death — reconciliation through destruction"),
    ]
    edges = [
        Edge("Empedocles", "sacrifice", EdgeType.DEPENDENCY),
        Edge("sacrifice", "nature", EdgeType.DEPENDENCY),
        Edge("Empedocles", "nature", EdgeType.DEPENDENCY),
        Edge("organic", "aorgic", EdgeType.NEGATION),
        Edge("sacrifice", "organic", EdgeType.SUBLATION),
        Edge("sacrifice", "aorgic", EdgeType.SUBLATION),
        Edge("Empedocles", "divine", EdgeType.DEPENDENCY),
        Edge("Empedocles", "mortal", EdgeType.NEGATION),
        Edge("tragic_death", "sacrifice", EdgeType.DEPENDENCY),
        Edge("tragic_death", "return", EdgeType.REFERENCE),
        Edge("Empedocles", "all_in_one", EdgeType.REFERENCE),
    ]
    return "1798-1800: The Death of Empedocles", vertices, edges


def work_07_ground_for_empedocles():
    """1799: Ground for Empedocles (Grund zum Empedokles).

    Theoretical essay — the tragic structure: organic and aorgic
    achieve mutual interpenetration, dissolution is necessary.
    """
    vertices = [
        Vertex("tragic", content="the tragic — das Tragische, necessary dissolution"),
        Vertex("interpenetration", content="interpenetration of organic and aorgic"),
        Vertex("dissolution", content="dissolution — Auflösung, the return to ground"),
        Vertex("totality", content="totality — the whole that includes opposition"),
    ]
    edges = [
        Edge("tragic", "organic", EdgeType.DEPENDENCY),
        Edge("tragic", "aorgic", EdgeType.DEPENDENCY),
        Edge("interpenetration", "organic", EdgeType.DEPENDENCY),
        Edge("interpenetration", "aorgic", EdgeType.DEPENDENCY),
        Edge("dissolution", "interpenetration", EdgeType.DEPENDENCY),
        Edge("dissolution", "sacrifice", EdgeType.REFERENCE),
        Edge("tragic", "dissolution", EdgeType.DEPENDENCY),
        Edge("totality", "interpenetration", EdgeType.SUBLATION),
        Edge("totality", "all_in_one", EdgeType.REFERENCE),
    ]
    return "1799: Ground for Empedocles", vertices, edges


def work_08_remarks_oedipus():
    """1803: Remarks on Oedipus (Anmerkungen zum Oedipus).

    Late theoretical text — the caesura, tragic transport,
    the turning-away of the god, categorical reversal.
    """
    vertices = [
        Vertex("caesura", content="caesura — Zäsur, the counter-rhythmic interruption"),
        Vertex("tragic_transport", content="tragic transport — the human torn from its sphere"),
        Vertex("turning_away", content="turning-away of the god — Abkehr des Göttlichen"),
        Vertex("categorical_reversal", content="categorical reversal — kategorische Umkehr"),
        Vertex("Oedipus", content="Oedipus — who seeks to know too much"),
        Vertex("nefas", content="nefas — the monstrous coupling with the divine"),
    ]
    edges = [
        Edge("caesura", "tragic_transport", EdgeType.DEPENDENCY),
        Edge("tragic_transport", "mortal", EdgeType.DEPENDENCY),
        Edge("tragic_transport", "divine", EdgeType.NEGATION),
        Edge("turning_away", "divine", EdgeType.NEGATION),
        Edge("categorical_reversal", "caesura", EdgeType.DEPENDENCY),
        Edge("Oedipus", "nefas", EdgeType.DEPENDENCY),
        Edge("nefas", "divine", EdgeType.DEPENDENCY),
        Edge("nefas", "mortal", EdgeType.NEGATION),
        Edge("caesura", "tragic", EdgeType.DEPENDENCY),
        Edge("Oedipus", "fate", EdgeType.DEPENDENCY),
        Edge("turning_away", "separation", EdgeType.REFERENCE),
    ]
    return "1803: Remarks on Oedipus", vertices, edges


def work_09_remarks_antigone():
    """1803: Remarks on Antigone (Anmerkungen zur Antigone).

    Companion to Oedipus — anti-theos, the Hesperian turn,
    national reversal, sobriety as the law of the West.
    """
    vertices = [
        Vertex("Antigone", content="Antigone — the anti-theos, defiant of divine law"),
        Vertex("Hesperien", content="Hesperien — the West, the Hesperian land"),
        Vertex("sobriety", content="Junonian sobriety — nüchterne Klarheit"),
        Vertex("fire", content="fire from heaven — das Feuer vom Himmel"),
        Vertex("national_reversal", content="national reversal — vaterländische Umkehr"),
    ]
    edges = [
        Edge("Antigone", "divine", EdgeType.NEGATION),
        Edge("Antigone", "mortal", EdgeType.DEPENDENCY),
        Edge("Hesperien", "Greece", EdgeType.NEGATION),
        Edge("sobriety", "fire", EdgeType.NEGATION),
        Edge("fire", "Greece", EdgeType.REFERENCE),
        Edge("sobriety", "Hesperien", EdgeType.DEPENDENCY),
        Edge("national_reversal", "Hesperien", EdgeType.DEPENDENCY),
        Edge("national_reversal", "categorical_reversal", EdgeType.REFERENCE),
        Edge("Antigone", "tragic_transport", EdgeType.REFERENCE),
        Edge("Antigone", "caesura", EdgeType.REFERENCE),
        Edge("fire", "divine", EdgeType.DEPENDENCY),
    ]
    return "1803: Remarks on Antigone", vertices, edges


def work_10_late_hymns():
    """1800-1803: Late Hymns (Bread and Wine, The Rhine, Patmos, etc.).

    The great hymns — the flight and return of the gods, the night
    of the gods' absence, the poet as mediator, rivers as paths.
    """
    vertices = [
        Vertex("night", content="night — the gods' absence, the sacred night"),
        Vertex("bread_and_wine", content="bread and wine — Dionysian gifts, holy signs"),
        Vertex("poet", content="the poet — mediator between gods and mortals"),
        Vertex("river", content="the river — Rhine, Danube — path of destiny"),
        Vertex("Dionysus", content="Dionysus — the coming god, brother of Christ"),
        Vertex("Christ", content="Christ — the last god, departure and promise"),
        Vertex("fatherland", content="Vaterland — the homeland, the Hesperian ground"),
        Vertex("demigod", content="demigod — Halbgott, between divine and mortal"),
    ]
    edges = [
        Edge("night", "divine", EdgeType.NEGATION),
        Edge("bread_and_wine", "Dionysus", EdgeType.DEPENDENCY),
        Edge("bread_and_wine", "Christ", EdgeType.REFERENCE),
        Edge("poet", "divine", EdgeType.DEPENDENCY),
        Edge("poet", "mortal", EdgeType.DEPENDENCY),
        Edge("river", "nature", EdgeType.DEPENDENCY),
        Edge("river", "fatherland", EdgeType.DEPENDENCY),
        Edge("Dionysus", "Christ", EdgeType.REFERENCE),
        Edge("Christ", "divine", EdgeType.DEPENDENCY),
        Edge("Christ", "mortal", EdgeType.SUBLATION),
        Edge("demigod", "divine", EdgeType.DEPENDENCY),
        Edge("demigod", "mortal", EdgeType.DEPENDENCY),
        Edge("poet", "demigod", EdgeType.REFERENCE),
        Edge("fatherland", "Hesperien", EdgeType.REFERENCE),
        Edge("night", "return", EdgeType.DEPENDENCY),
        Edge("Dionysus", "fire", EdgeType.REFERENCE),
    ]
    return "1800-03: Late Hymns (Bread and Wine, Rhine, Patmos)", vertices, edges


def work_11_homecoming():
    """1801: Homecoming / To the Kindred (Heimkunft / An die Verwandten).

    The poem of return — joy withheld, the nearness of the divine
    in the Hesperian landscape, the 'care' of the poet.
    """
    vertices = [
        Vertex("homecoming", content="homecoming — Heimkunft, return to the near"),
        Vertex("joy", content="joy — Freude, the fitting name for the divine"),
        Vertex("care", content="care — Sorge, the poet's task of preserving"),
        Vertex("nearness", content="nearness — Nähe, the divine in the everyday"),
    ]
    edges = [
        Edge("homecoming", "fatherland", EdgeType.DEPENDENCY),
        Edge("homecoming", "return", EdgeType.DEPENDENCY),
        Edge("joy", "divine", EdgeType.DEPENDENCY),
        Edge("joy", "nearness", EdgeType.DEPENDENCY),
        Edge("care", "poet", EdgeType.DEPENDENCY),
        Edge("nearness", "nature", EdgeType.REFERENCE),
        Edge("nearness", "divine", EdgeType.REFERENCE),
        Edge("homecoming", "Hesperien", EdgeType.REFERENCE),
        Edge("care", "night", EdgeType.REFERENCE),
    ]
    return "1801: Homecoming", vertices, edges


def work_12_celebration_of_peace():
    """1801: Celebration of Peace (Friedensfeier).

    The peace poem — the prince of the feast, reconciliation
    of nature and art, the festive gathering of gods and mortals.
    """
    vertices = [
        Vertex("peace", content="peace — Frieden, reconciliation of opposites"),
        Vertex("feast", content="the feast — das Fest, gathering of gods and mortals"),
        Vertex("prince_of_feast", content="prince of the feast — the unnamed reconciler"),
    ]
    edges = [
        Edge("feast", "divine", EdgeType.DEPENDENCY),
        Edge("feast", "mortal", EdgeType.DEPENDENCY),
        Edge("peace", "feast", EdgeType.DEPENDENCY),
        Edge("prince_of_feast", "Christ", EdgeType.REFERENCE),
        Edge("prince_of_feast", "Dionysus", EdgeType.REFERENCE),
        Edge("peace", "harmony", EdgeType.REFERENCE),
        Edge("feast", "nature", EdgeType.SUBLATION),
        Edge("peace", "fire", EdgeType.SUBLATION),
        Edge("peace", "sobriety", EdgeType.SUBLATION),
    ]
    return "1801: Celebration of Peace (Friedensfeier)", vertices, edges


def work_13_in_lovely_blueness():
    """1803-06: In Lovely Blueness (In lieblicher Bläue).

    Late text — poetic dwelling, the measure-taking of mortals,
    man as the image of the divine, the blue of heaven.
    """
    vertices = [
        Vertex("poetic_dwelling", content="poetic dwelling — dichterisch wohnet der Mensch"),
        Vertex("measure", content="measure — Maass, the measure the poet takes"),
        Vertex("image", content="image — Bild, man as image of divinity"),
        Vertex("heaven", content="heaven — Himmel, the blue above"),
        Vertex("earth", content="earth — Erde, the ground below"),
    ]
    edges = [
        Edge("poetic_dwelling", "mortal", EdgeType.DEPENDENCY),
        Edge("poetic_dwelling", "divine", EdgeType.DEPENDENCY),
        Edge("measure", "heaven", EdgeType.DEPENDENCY),
        Edge("measure", "earth", EdgeType.DEPENDENCY),
        Edge("image", "divine", EdgeType.REFERENCE),
        Edge("image", "mortal", EdgeType.REFERENCE),
        Edge("poetic_dwelling", "measure", EdgeType.DEPENDENCY),
        Edge("poetic_dwelling", "poet", EdgeType.REFERENCE),
        Edge("heaven", "earth", EdgeType.NEGATION),
        Edge("poetic_dwelling", "beauty", EdgeType.REFERENCE),
    ]
    return "1803-06: In Lovely Blueness", vertices, edges


def work_14_mnemosyne():
    """1803: Mnemosyne (late fragment).

    The poem of memory and death — 'we are a sign, without meaning',
    the impossibility of mourning, the mortals' heavenly passage.
    """
    vertices = [
        Vertex("Mnemosyne", content="Mnemosyne — memory, mother of the Muses"),
        Vertex("sign", content="sign — Zeichen, 'we are a sign, without meaning'"),
        Vertex("mourning", content="mourning — Trauer, the impossible grief"),
        Vertex("passage", content="passage — Übergang, mortals' crossing"),
    ]
    edges = [
        Edge("Mnemosyne", "mortal", EdgeType.DEPENDENCY),
        Edge("Mnemosyne", "divine", EdgeType.REFERENCE),
        Edge("sign", "mortal", EdgeType.DEPENDENCY),
        Edge("sign", "poet", EdgeType.REFERENCE),
        Edge("mourning", "separation", EdgeType.DEPENDENCY),
        Edge("mourning", "night", EdgeType.REFERENCE),
        Edge("passage", "return", EdgeType.DEPENDENCY),
        Edge("passage", "mortal", EdgeType.DEPENDENCY),
        Edge("passage", "divine", EdgeType.NEGATION),
        Edge("Mnemosyne", "poetic_dwelling", EdgeType.REFERENCE),
        Edge("sign", "caesura", EdgeType.REFERENCE),
    ]
    return "1803: Mnemosyne", vertices, edges


# ---------------------------------------------------------------------------
# Work list (chronological)
# ---------------------------------------------------------------------------

ALL_WORKS = [
    work_01_hymns_of_tubingen,        # 1788-93
    work_02_hyperion_fragment,         # 1794
    work_03_judgment_and_being,        # 1795
    work_04_poetic_spirit,            # 1795
    work_05_hyperion,                  # 1797-99
    work_06_death_of_empedocles,       # 1798-1800
    work_07_ground_for_empedocles,     # 1799
    work_08_remarks_oedipus,           # 1803
    work_09_remarks_antigone,          # 1803
    work_10_late_hymns,                # 1800-03
    work_11_homecoming,                # 1801
    work_12_celebration_of_peace,      # 1801
    work_13_in_lovely_blueness,        # 1803-06
    work_14_mnemosyne,                 # 1803
]


# ---------------------------------------------------------------------------
# Incremental injection + digestion
# ---------------------------------------------------------------------------


def run_experiment():
    total_works = len(ALL_WORKS)
    print("=" * 70)
    print("FRIEDRICH HÖLDERLIN — COLLECTED WORKS INCREMENTAL TRAVERSAL")
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
        "experiment": "holderlin_collected_works_incremental",
        "source": "Friedrich Hölderlin — collected works, 14 entries (1788-1806)",
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

    output_path = "experiment_holderlin.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
