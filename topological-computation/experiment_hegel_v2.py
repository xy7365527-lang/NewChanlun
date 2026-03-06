"""Hegel Sense-Certainty experiment v2: manually constructed complex.

The rule-based NLP parser cannot handle Hegel's complex philosophical prose.
This experiment constructs the simplicial complex by hand, encoding the actual
dialectical structure of the Sense-Certainty chapter, then runs the engine.

This is methodologically honest: we state that phi_L with rule-based fallback
is insufficient for this text, and show what happens when the complex correctly
represents the philosophical content.
"""

from __future__ import annotations

import json
import sys
sys.path.insert(0, '.')

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from traversal import TraversalEngine
from morse import compute_terrain


def build_hegel_complex() -> Graph:
    """Construct the Sense-Certainty dialectical complex by hand.

    Hegel's argument structure:
    1. Sense-certainty claims immediate knowledge of the This/Now/Here
    2. The Now is Night -> but Now is also Day -> Now is universal (negation of particular)
    3. The This is posited -> negated -> superseded -> preserved as universal
    4. The I claims singular certainty -> but I is also universal
    5. Language expresses only the universal, not the meant particular
    6. Pure being = the universal = truth of sense-certainty
    """
    vertices = [
        # Core concepts
        Vertex("sense_certainty", content="sense-certainty"),
        Vertex("immediate_knowledge", content="immediate knowledge"),
        Vertex("object", content="the object"),
        Vertex("the_I", content="the I"),

        # The Now dialectic
        Vertex("now", content="the Now"),
        Vertex("now_night", content="Now is Night"),
        Vertex("now_day", content="Now is Day"),
        Vertex("now_universal", content="Now as universal"),

        # The This dialectic
        Vertex("this", content="the This"),
        Vertex("this_negated", content="This as not-This"),
        Vertex("this_superseded", content="This as superseded"),
        Vertex("this_preserved", content="This as preserved/universal"),

        # The Here dialectic
        Vertex("here", content="the Here"),
        Vertex("here_universal", content="Here as universal"),

        # Universality
        Vertex("universal", content="the universal"),
        Vertex("particular", content="the particular"),
        Vertex("immediacy", content="immediacy"),
        Vertex("mediation", content="mediation"),

        # Language
        Vertex("language", content="language"),
        Vertex("what_is_meant", content="what is meant (das Meinen)"),

        # The I dialectic
        Vertex("singular_I", content="singular I"),
        Vertex("universal_I", content="universal I"),

        # Final resolution
        Vertex("pure_being", content="pure being"),
        Vertex("truth_sc", content="truth of sense-certainty"),
    ]

    edges = [
        # Sense-certainty's initial claims (dependency: SC depends on these)
        Edge("sense_certainty", "immediate_knowledge", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "object", EdgeType.DEPENDENCY),
        Edge("sense_certainty", "the_I", EdgeType.DEPENDENCY),
        Edge("immediate_knowledge", "immediacy", EdgeType.DEPENDENCY),

        # The Now dialectic
        Edge("now", "sense_certainty", EdgeType.DEPENDENCY),  # SC points to Now
        Edge("now_night", "now", EdgeType.DEPENDENCY),       # Night is a Now
        Edge("now_day", "now", EdgeType.DEPENDENCY),          # Day is a Now
        Edge("now_night", "now_day", EdgeType.NEGATION),      # Night negates Day
        Edge("now_day", "now_night", EdgeType.NEGATION),      # Day negates Night (bidirectional)
        Edge("now_universal", "now_night", EdgeType.SUBLATION),  # Universal sublates Night
        Edge("now_universal", "now_day", EdgeType.SUBLATION),    # Universal sublates Day
        Edge("now_universal", "universal", EdgeType.DEPENDENCY),

        # The This dialectic
        Edge("this", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("this", "particular", EdgeType.DEPENDENCY),
        Edge("this_negated", "this", EdgeType.NEGATION),       # This is not-This
        Edge("this_superseded", "this_negated", EdgeType.DEPENDENCY),
        Edge("this_preserved", "this_superseded", EdgeType.SUBLATION),  # preserved = sublated
        Edge("this_preserved", "this", EdgeType.SUBLATION),
        Edge("this_preserved", "universal", EdgeType.DEPENDENCY),

        # The Here dialectic
        Edge("here", "sense_certainty", EdgeType.DEPENDENCY),
        Edge("here", "particular", EdgeType.DEPENDENCY),
        Edge("here_universal", "here", EdgeType.SUBLATION),
        Edge("here_universal", "universal", EdgeType.DEPENDENCY),

        # Particular vs Universal (core dialectical tension)
        Edge("particular", "universal", EdgeType.NEGATION),
        Edge("universal", "particular", EdgeType.NEGATION),  # bidirectional
        Edge("immediacy", "mediation", EdgeType.NEGATION),
        Edge("mediation", "immediacy", EdgeType.NEGATION),   # bidirectional

        # Language
        Edge("language", "universal", EdgeType.DEPENDENCY),   # language expresses universal
        Edge("what_is_meant", "particular", EdgeType.DEPENDENCY),
        Edge("language", "what_is_meant", EdgeType.NEGATION),  # language cannot reach what is meant

        # The I dialectic
        Edge("the_I", "singular_I", EdgeType.DEPENDENCY),
        Edge("singular_I", "universal_I", EdgeType.NEGATION),  # singular I negated by universality
        Edge("universal_I", "singular_I", EdgeType.NEGATION),  # bidirectional
        Edge("universal_I", "universal", EdgeType.DEPENDENCY),

        # Immediacy negated by mediation in the object
        Edge("object", "immediacy", EdgeType.DEPENDENCY),
        Edge("object", "particular", EdgeType.REFERENCE),

        # Final resolution
        Edge("pure_being", "universal", EdgeType.DEPENDENCY),
        Edge("truth_sc", "pure_being", EdgeType.DEPENDENCY),
        Edge("truth_sc", "sense_certainty", EdgeType.REFERENCE),  # truth refers back
        Edge("sense_certainty", "truth_sc", EdgeType.DEPENDENCY),  # SC depends on its truth

        # Cross-links (the dialectical web)
        Edge("mediation", "universal", EdgeType.DEPENDENCY),   # mediation leads to universal
        Edge("now_universal", "this_preserved", EdgeType.REFERENCE),  # same structure
        Edge("the_I", "universal_I", EdgeType.REFERENCE),
    ]

    g = Graph()
    for v in vertices:
        g = g.add_vertex(v)
    for e in edges:
        g = g.add_edge(e)

    return g


def run_experiment():
    print("=" * 70)
    print("HEGEL SENSE-CERTAINTY: Manually Constructed Complex")
    print("=" * 70)

    graph = build_hegel_complex()
    vertices = graph.active_vertex_ids()
    edges = graph.active_edges()
    beta_1 = compute_beta_1(graph)

    print(f"\nVertices: {len(vertices)}")
    print(f"Edges: {len(edges)}")
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

    # Terrain
    terrain = compute_terrain(graph)
    terrain_counts = {"tree": 0, "critical": 0}
    for mark in terrain.values():
        terrain_counts[mark] = terrain_counts.get(mark, 0) + 1
    print(f"\nTerrain: {terrain_counts}")

    # -----------------------------------------------------------------------
    # Traversal
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("TRAVERSAL: 2000 steps")
    print("=" * 70)

    start = "sense_certainty"
    engine = TraversalEngine(graph, start=start, settlement_threshold=10, seed=42)

    beta_trajectory = []
    encounter_counts = {}
    operation_counts = {}

    max_steps = 2000
    for i in range(max_steps):
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
            print(f"  Step {i+1:4d}: beta_1={log.beta_1_after:3d}, "
                  f"settled={log.settled_count:2d}, "
                  f"V={log.vertices_active}, E={log.edges_active}")

    # Check growth and extend if needed
    last_beta = engine.logs[-1].beta_1_after
    beta_at_1500 = None
    for log in engine.logs:
        if log.step == 1500:
            beta_at_1500 = log.beta_1_after
            break

    extended = False
    if beta_at_1500 and last_beta > beta_at_1500 * 1.1:
        print(f"\n  beta_1 still growing ({beta_at_1500} -> {last_beta}), extending to 5000...")
        extended = True
        for i in range(max_steps, 5000):
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
                print(f"  Step {i+1:4d}: beta_1={log.beta_1_after:3d}, "
                      f"settled={log.settled_count:2d}, "
                      f"V={log.vertices_active}, E={log.edges_active}")
        max_steps = 5000

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

    print(f"\nFinal state after {max_steps} steps:")
    print(f"  beta_1: {final_beta}")
    print(f"  Settled cycles: {final_settled}")
    print(f"  Active vertices: {final_v}")
    print(f"  Active edges: {final_e}")

    print(f"\nEncounter distribution:")
    for enc, count in sorted(encounter_counts.items()):
        print(f"  {enc}: {count}")

    print(f"\nOperation distribution:")
    for op, count in sorted(operation_counts.items()):
        print(f"  {op}: {count}")

    # Beta trajectory visualization
    print(f"\nBeta_1 trajectory:")
    for entry in beta_trajectory:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"  {entry['step']:4d}: {entry['beta_1']:3d} {bar}")

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

    if jumps[:30]:
        print(f"\n  First 30 beta_1 jumps:")
        for log in jumps[:30]:
            print(f"    Step {log.step}: +{log.delta_beta_1} (op={log.operation}, "
                  f"pos={log.position}, beta={log.beta_1_before}->{log.beta_1_after})")

    # Settled cycles detail
    if engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({len(engine.settlement.settled_cycles)}):")
        for sc in engine.settlement.settled_cycles[:10]:
            print(f"    settled@step={sc.settled_at_step}: {sorted(sc.edges)}")

    # Negation density
    neg_count_final = sum(1 for e in engine.k_active.active_edges()
                         if e.edge_type == EdgeType.NEGATION)
    total_e_final = len(engine.k_active.active_edges())
    neg_density = neg_count_final / total_e_final if total_e_final > 0 else 0

    print(f"\n--- Comparison with Genealogy Data ---")
    print(f"  Hegel negation density:     {neg_density:.4f} ({neg_count_final}/{total_e_final})")
    print(f"  Genealogy negation density: 0.0164 (111/6751)")
    print(f"  Ratio: {neg_density / 0.0164:.2f}x" if neg_density > 0 else "  Ratio: 0x")

    encounter_density = sum(1 for log in engine.logs if log.encounter != "nothing") / max_steps
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

    # -----------------------------------------------------------------------
    # Verification points
    # -----------------------------------------------------------------------
    print(f"\n--- Verification Points ---")

    # 1. Does beta_1 growth show dialectical structure?
    if jumps:
        print(f"\n  1. Dialectical beta_1 structure:")
        # Group jumps by step ranges
        early = [j for j in jumps if j.step <= 100]
        mid = [j for j in jumps if 100 < j.step <= 500]
        late = [j for j in jumps if j.step > 500]
        print(f"     Early (1-100): {len(early)} jumps")
        print(f"     Mid (101-500): {len(mid)} jumps")
        print(f"     Late (501+):   {len(late)} jumps")
    else:
        print(f"\n  1. No beta_1 jumps detected")

    # 2. Do engine-discovered tensions match Hegel's argument transitions?
    print(f"\n  2. Engine-discovered tensions:")
    for log in engine.logs[:50]:
        if log.operation not in ("walk", "nothing"):
            pos_content = engine.k_active.vertex(log.position)
            pos_label = pos_content.content if pos_content else log.position
            print(f"     Step {log.step}: {log.operation} at '{pos_label}'")

    # 3. Terrain f-values at dialectical positions
    print(f"\n  3. f terrain at key dialectical positions:")
    key_pairs = [
        ("particular", "universal"),
        ("immediacy", "mediation"),
        ("now_night", "now_day"),
        ("singular_I", "universal_I"),
    ]
    for a, b in key_pairs:
        if a in [v for v in engine.k_active.active_vertex_ids()] and \
           b in [v for v in engine.k_active.active_vertex_ids()]:
            f_val = engine._compute_f(a, b)
            print(f"     f({a}, {b}) = {f_val}")

    # -----------------------------------------------------------------------
    # Save
    # -----------------------------------------------------------------------
    result = {
        "experiment": "hegel_sense_certainty_v2_manual_complex",
        "text_source": "Hegel, Phenomenology of Spirit, A.V. Miller translation",
        "chapter": "Sense-Certainty",
        "methodology": "Manually constructed simplicial complex encoding dialectical structure",
        "phi_L_note": "Rule-based NLP parser insufficient for Hegel's prose (81 vertices, 24 edges, 0 critical). Manual construction preserves philosophical content.",
        "initial_complex": {
            "vertices": len(vertices),
            "edges": len(edges),
            "beta_1": beta_1,
            "edge_type_distribution": type_counts,
        },
        "traversal": {
            "total_steps": max_steps,
            "extended": extended,
            "settlement_threshold": 10,
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
        },
        "analysis": {
            "negation_density": neg_density,
            "genealogy_negation_density": 0.0164,
            "negation_density_ratio": neg_density / 0.0164 if neg_density > 0 else 0,
            "encounter_density": encounter_density,
            "terrain_distribution": ft_counts,
            "critical_edge_ratio": crit_ratio,
        },
        "settled_cycles": [
            {"edges": sorted(sc.edges), "settled_at_step": sc.settled_at_step}
            for sc in engine.settlement.settled_cycles
        ],
        "blocked_log": engine.settlement.blocked_log,
    }

    output_path = "experiment_hegel_sense_certainty.json"
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    print(f"\n\nResults saved to {output_path}")
    return result


if __name__ == "__main__":
    run_experiment()
