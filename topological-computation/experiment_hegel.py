"""Hegel Sense-Certainty experiment: phi_L + engine 2000+ steps.

Maps the opening chapter of Phenomenology of Spirit through the
topological computation pipeline.
"""

from __future__ import annotations

import json
import sys
sys.path.insert(0, '.')

from phi_L import phi_L, _format_graph
from engine import compute_beta_1, EdgeType
from traversal import TraversalEngine
from morse import compute_terrain

# ---------------------------------------------------------------------------
# Text: Hegel, Phenomenology of Spirit, "Sense-Certainty" (A.V. Miller trans.)
# ---------------------------------------------------------------------------

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


def run_experiment():
    # -----------------------------------------------------------------------
    # Step 1: phi_L
    # -----------------------------------------------------------------------
    print("=" * 70)
    print("PHASE 1: phi_L mapping")
    print("=" * 70)

    graph = phi_L(SENSE_CERTAINTY_TEXT)
    print(_format_graph(graph))

    vertices = graph.active_vertex_ids()
    edges = graph.active_edges()
    beta_1_initial = compute_beta_1(graph)

    print(f"\n--- Summary ---")
    print(f"Vertices: {len(vertices)}")
    print(f"Edges: {len(edges)}")
    print(f"beta_1: {beta_1_initial}")

    # Edge type distribution
    type_counts = {}
    for e in edges:
        type_counts[e.edge_type.value] = type_counts.get(e.edge_type.value, 0) + 1
    print(f"\nEdge type distribution:")
    for et, count in sorted(type_counts.items()):
        print(f"  {et}: {count}")

    # Negation edges detail
    neg_edges = [e for e in edges if e.edge_type == EdgeType.NEGATION]
    if neg_edges:
        print(f"\nNegation edges ({len(neg_edges)}):")
        verts_dict = graph.vertices
        for e in neg_edges:
            src_label = verts_dict[e.source].content if e.source in verts_dict else e.source
            tgt_label = verts_dict[e.target].content if e.target in verts_dict else e.target
            print(f"  '{src_label}' --[negation]--> '{tgt_label}'")

    # Vertex list
    print(f"\nAll vertices:")
    verts_dict = graph.vertices
    for vid in sorted(verts_dict.keys(), key=lambda x: int(x[1:]) if x[1:].isdigit() else 0):
        v = verts_dict[vid]
        print(f"  {vid}: '{v.content}'")

    # -----------------------------------------------------------------------
    # Step 2: Traversal engine
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("PHASE 2: Traversal engine (2000+ steps)")
    print("=" * 70)

    start = vertices[0]
    terrain = compute_terrain(graph)

    engine = TraversalEngine(graph, start=start, settlement_threshold=10, seed=42)

    # Run 2000 steps, record beta_1 every 100 steps
    beta_trajectory = []
    encounter_counts = {}
    operation_counts = {}

    max_steps = 2000
    for i in range(max_steps):
        log = engine.run_step()
        enc_type = log.encounter
        op_type = log.operation
        encounter_counts[enc_type] = encounter_counts.get(enc_type, 0) + 1
        operation_counts[op_type] = operation_counts.get(op_type, 0) + 1

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

    # Check if still in exploration phase
    last_beta = engine.logs[-1].beta_1_after
    beta_at_1500 = None
    for log in engine.logs:
        if log.step == 1500:
            beta_at_1500 = log.beta_1_after
            break

    # If beta is still growing significantly, extend to 5000
    extended = False
    if beta_at_1500 and last_beta > beta_at_1500 * 1.1:
        print(f"\n  beta_1 still growing ({beta_at_1500} -> {last_beta}), extending to 5000...")
        extended = True
        for i in range(max_steps, 5000):
            log = engine.run_step()
            enc_type = log.encounter
            op_type = log.operation
            encounter_counts[enc_type] = encounter_counts.get(enc_type, 0) + 1
            operation_counts[op_type] = operation_counts.get(op_type, 0) + 1

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
    # Step 3: Analysis
    # -----------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("PHASE 3: Analysis")
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

    # Beta_1 growth analysis
    print(f"\nBeta_1 trajectory (every 100 steps):")
    for entry in beta_trajectory:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"  {entry['step']:4d}: {entry['beta_1']:3d} {bar}")

    # Dialectical structure analysis
    print(f"\n--- Dialectical Structure Analysis ---")

    # Count negation events in logs
    neg_events = [log for log in engine.logs if "negate" in log.operation]
    sublate_events = [log for log in engine.logs if "sublate" in log.operation]
    fold_events = [log for log in engine.logs if "fold" in log.operation]
    blocked_events = [log for log in engine.logs if log.blocked]

    print(f"  Negation events: {len(neg_events)}")
    print(f"  Sublation events: {len(sublate_events)}")
    print(f"  Fold events: {len(fold_events)}")
    print(f"  Blocked events: {len(blocked_events)}")

    # Beta_1 jumps (steps where beta increased)
    jumps = [log for log in engine.logs if log.delta_beta_1 > 0]
    print(f"\n  Beta_1 jump events: {len(jumps)}")
    if jumps[:20]:
        print(f"  First 20 jumps:")
        for log in jumps[:20]:
            print(f"    Step {log.step}: +{log.delta_beta_1} (op={log.operation}, "
                  f"beta={log.beta_1_before}->{log.beta_1_after})")

    # Negation density comparison
    total_edges_final = final_e
    neg_count_final = sum(1 for e in engine.k_active.active_edges()
                         if e.edge_type == EdgeType.NEGATION)
    neg_density = neg_count_final / total_edges_final if total_edges_final > 0 else 0

    print(f"\n--- Comparison with Genealogy Data ---")
    print(f"  Hegel text negation density: {neg_density:.4f} ({neg_count_final}/{total_edges_final})")
    print(f"  Genealogy negation density:  0.0164 (111/6751)")
    print(f"  Ratio: {neg_density / 0.0164:.2f}x" if neg_density > 0 else "  Ratio: 0x")

    # Encounter density
    non_nothing = sum(1 for log in engine.logs if log.encounter != "nothing")
    encounter_density = non_nothing / max_steps
    print(f"\n  Hegel encounter density: {encounter_density:.4f} ({non_nothing}/{max_steps})")

    # Terrain analysis
    final_terrain = compute_terrain(engine.k_active)
    terrain_counts = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        terrain_counts[mark] = terrain_counts.get(mark, 0) + 1
    print(f"\n  Final terrain: {terrain_counts}")
    crit_ratio = terrain_counts["critical"] / (terrain_counts["tree"] + terrain_counts["critical"]) \
        if (terrain_counts["tree"] + terrain_counts["critical"]) > 0 else 0
    print(f"  Critical edge ratio: {crit_ratio:.4f}")

    # -----------------------------------------------------------------------
    # Step 4: Save results
    # -----------------------------------------------------------------------
    result = {
        "experiment": "hegel_sense_certainty",
        "text_source": "Hegel, Phenomenology of Spirit, A.V. Miller translation",
        "chapter": "Sense-Certainty",
        "phi_L_output": {
            "vertices": len(vertices),
            "edges": len(edges),
            "beta_1_initial": beta_1_initial,
            "edge_type_distribution": type_counts,
            "negation_edges": [
                {
                    "source": graph.vertices[e.source].content,
                    "target": graph.vertices[e.target].content,
                }
                for e in neg_edges
            ],
            "vertex_list": [
                {"id": vid, "content": graph.vertices[vid].content}
                for vid in sorted(graph.vertices.keys(),
                                  key=lambda x: int(x[1:]) if x[1:].isdigit() else 0)
            ],
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
            "terrain_distribution": terrain_counts,
            "critical_edge_ratio": crit_ratio,
        },
        "settled_cycles": [
            {
                "edges": sorted(sc.edges),
                "settled_at_step": sc.settled_at_step,
            }
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
