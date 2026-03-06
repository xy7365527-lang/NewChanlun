"""everything-claude-code repository -- incremental phi_L traversal experiment.

Processes key .md documents from the everything-claude-code repository through
the phi_L NLP pipeline (dependency parse -> typed simplicial complex).

Each document is:
1. Read with code-block stripping (``` blocks removed to avoid NLP noise)
2. Capped at 10000 characters if too large
3. Processed through phi_L (English NLP)
4. Injected incrementally into the cumulative complex
5. Traversal engine runs until beta_1 converges (50-step stability)

Documents are ordered from most architectural to most specific:
  CLAUDE.md -> AGENTS.md -> README.md -> guides -> rules
"""

from __future__ import annotations

import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1
from phi_L import phi_L
from traversal import TraversalEngine
from morse import compute_terrain


# ---------------------------------------------------------------------------
# Document list (ordered from core to periphery)
# ---------------------------------------------------------------------------

REPO_ROOT = os.path.join(os.path.expanduser("~"), "AppData", "Local", "Temp", "everything-claude-code")

DOCUMENTS = [
    ("CLAUDE.md", "CLAUDE.md"),
    ("AGENTS.md", "AGENTS.md"),
    ("README.md", "README.md"),
    ("the-longform-guide.md", "the-longform-guide.md"),
    ("the-shortform-guide.md", "the-shortform-guide.md"),
    ("the-security-guide.md", "the-security-guide.md"),
    ("the-openclaw-guide.md", "the-openclaw-guide.md"),
    ("CONTRIBUTING.md", "CONTRIBUTING.md"),
    ("rules/common/agents.md", "rules/common/agents.md"),
    ("rules/common/coding-style.md", "rules/common/coding-style.md"),
    ("rules/common/security.md", "rules/common/security.md"),
    ("rules/common/testing.md", "rules/common/testing.md"),
    ("rules/common/performance.md", "rules/common/performance.md"),
    ("rules/common/patterns.md", "rules/common/patterns.md"),
]

MAX_CHARS = 10000


# ---------------------------------------------------------------------------
# Text preprocessing: strip code blocks
# ---------------------------------------------------------------------------

_CODE_BLOCK_RE = re.compile(r"```[\s\S]*?```", re.MULTILINE)


def strip_code_blocks(text: str) -> str:
    """Remove fenced code blocks (``` ... ```) from markdown text."""
    return _CODE_BLOCK_RE.sub("", text)


# ---------------------------------------------------------------------------
# Incremental injection (from phi_L_batch.py)
# ---------------------------------------------------------------------------

def _inject_subgraph(base: Graph, sub: Graph) -> Graph:
    """Inject sub-graph into base, merging vertices by content label."""
    base_content_to_id: dict[str, str] = {}
    for vid, v in base.vertices.items():
        if v.content:
            base_content_to_id[v.content] = vid

    sub_to_base: dict[str, str] = {}
    next_id = len(base.vertices)
    result = base

    for vid, v in sub.vertices.items():
        if v.content and v.content in base_content_to_id:
            sub_to_base[vid] = base_content_to_id[v.content]
        else:
            new_id = f"v{next_id}"
            next_id += 1
            sub_to_base[vid] = new_id
            new_v = Vertex(id=new_id, content=v.content)
            result = result.add_vertex(new_v)
            if v.content:
                base_content_to_id[v.content] = new_id

    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value) for e in result.edges
    }
    for e in sub.edges:
        src = sub_to_base.get(e.source, e.source)
        tgt = sub_to_base.get(e.target, e.target)
        if src == tgt:
            continue
        key = (src, tgt, e.edge_type.value)
        if key not in existing_edges:
            existing_edges.add(key)
            result = result.add_edge(Edge(source=src, target=tgt, edge_type=e.edge_type))

    return result


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_experiment() -> dict:
    total_docs = len(DOCUMENTS)
    print("=" * 70)
    print("EVERYTHING-CLAUDE-CODE -- phi_L INCREMENTAL TRAVERSAL")
    print(f"{total_docs} documents, NLP pipeline, incremental injection")
    print("=" * 70)

    cumulative_graph = Graph()
    engine = None
    doc_results = []
    beta_1_curve = []
    cumulative_steps = 0

    for d_idx, (doc_label, doc_path) in enumerate(DOCUMENTS):
        filepath = os.path.join(REPO_ROOT, doc_path)
        if not os.path.isfile(filepath):
            print(f"\n  SKIP: {doc_label} (file not found)")
            continue

        with open(filepath, "r", encoding="utf-8") as f:
            raw_text = f.read()

        # Strip code blocks and cap at MAX_CHARS
        clean_text = strip_code_blocks(raw_text)
        if len(clean_text) > MAX_CHARS:
            clean_text = clean_text[:MAX_CHARS]

        print(f"\n{'─' * 60}")
        print(f"Doc {d_idx + 1}/{total_docs}: {doc_label}")
        print(f"  raw={len(raw_text)} chars, clean={len(clean_text)} chars")
        print(f"{'─' * 60}")

        t0 = time.time()

        # phi_L: text -> typed simplicial complex
        sub_graph = phi_L(clean_text)
        sub_v = len(sub_graph.active_vertex_ids())
        sub_e = len(sub_graph.active_edges())
        print(f"  phi_L extracted: {sub_v} vertices, {sub_e} edges")

        # Record pre-injection state
        beta_before = compute_beta_1(cumulative_graph)
        v_before = len(cumulative_graph.active_vertex_ids())
        e_before = len(cumulative_graph.active_edges())

        # Inject into cumulative complex
        cumulative_graph = _inject_subgraph(cumulative_graph, sub_graph)

        beta_after_inject = compute_beta_1(cumulative_graph)
        v_after = len(cumulative_graph.active_vertex_ids())
        e_after = len(cumulative_graph.active_edges())

        print(f"  After injection: {v_after} V (+{v_after - v_before}), "
              f"{e_after} E (+{e_after - e_before})")
        print(f"  beta_1 after injection: {beta_after_inject}")

        # Traversal to convergence
        active_ids = cumulative_graph.active_vertex_ids()
        if active_ids:
            start_vertex = active_ids[0]
            engine = TraversalEngine(
                cumulative_graph, start_vertex,
                settlement_threshold=10,
                seed=42 + d_idx,
            )
            stable_count = 0
            last_beta = compute_beta_1(engine.k_active)
            steps_this_doc = 0
            max_steps = 2000

            while stable_count < 50 and steps_this_doc < max_steps:
                engine.run_step()
                steps_this_doc += 1
                cumulative_steps += 1
                current_beta = compute_beta_1(engine.k_active)
                if current_beta == last_beta:
                    stable_count += 1
                else:
                    stable_count = 0
                    last_beta = current_beta

            converged = stable_count >= 50
            cumulative_graph = engine.k_active
        else:
            steps_this_doc = 0
            converged = True

        beta_after_traversal = compute_beta_1(cumulative_graph)
        settled_count = len(engine.settlement.settled_cycles) if engine else 0

        elapsed = time.time() - t0

        print(f"  Steps to digest: {steps_this_doc} "
              f"{'(CONVERGED)' if converged else '(MAX REACHED)'}")
        print(f"  beta_1 after traversal: {beta_after_traversal}")
        print(f"  Settled cycles: {settled_count}")
        print(f"  Active complex: {len(cumulative_graph.active_vertex_ids())} V, "
              f"{len(cumulative_graph.active_edges())} E")
        print(f"  Elapsed: {elapsed:.2f}s")

        # Count operations
        if engine and steps_this_doc > 0:
            work_logs = engine.logs[-steps_this_doc:]
            op_counts: dict[str, int] = {}
            for log in work_logs:
                op_counts[log.operation] = op_counts.get(log.operation, 0) + 1
        else:
            op_counts = {}

        result = {
            "document": doc_label,
            "doc_index": d_idx + 1,
            "raw_chars": len(raw_text),
            "clean_chars": len(clean_text),
            "sub_vertices": sub_v,
            "sub_edges": sub_e,
            "vertices_added": v_after - v_before,
            "edges_added": e_after - e_before,
            "vertices_total": len(cumulative_graph.active_vertex_ids()),
            "edges_total": len(cumulative_graph.active_edges()),
            "beta_1_before": beta_before,
            "beta_1_after_inject": beta_after_inject,
            "beta_1_after_traversal": beta_after_traversal,
            "delta_beta_1": beta_after_traversal - beta_before,
            "settled_cycles": settled_count,
            "steps_to_digest": steps_this_doc,
            "converged": converged,
            "cumulative_steps": cumulative_steps,
            "operation_counts": op_counts,
            "elapsed_seconds": round(elapsed, 3),
        }
        doc_results.append(result)

        beta_1_curve.append({
            "doc_index": d_idx + 1,
            "document": doc_label,
            "beta_1": beta_after_traversal,
            "cumulative_steps": cumulative_steps,
        })

    # ---------------------------------------------------------------------------
    # Final summary
    # ---------------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("FINAL SUMMARY")
    print("=" * 70)

    final_beta = compute_beta_1(cumulative_graph)
    final_v = len(cumulative_graph.active_vertex_ids())
    final_e = len(cumulative_graph.active_edges())
    final_settled = len(engine.settlement.settled_cycles) if engine else 0

    print(f"\n  Total documents: {len(doc_results)}")
    print(f"  Total steps: {cumulative_steps}")
    print(f"  Final beta_1: {final_beta}")
    print(f"  Final complex: {final_v} V, {final_e} E")
    print(f"  Settled cycles: {final_settled}")

    # Beta_1 growth curve
    print(f"\n  beta_1 growth curve (by document):")
    for entry in beta_1_curve:
        bar = "#" * min(entry["beta_1"], 80)
        print(f"    D{entry['doc_index']:2d}: beta_1={entry['beta_1']:4d} "
              f"steps={entry['cumulative_steps']:5d} {bar}")
        print(f"          {entry['document']}")

    # Largest beta_1 jumps
    deltas = [(r["document"], r["delta_beta_1"], r["doc_index"]) for r in doc_results]
    deltas_sorted = sorted(deltas, key=lambda x: -x[1])
    print(f"\n  Largest beta_1 jumps by document:")
    for name, delta, idx in deltas_sorted[:5]:
        print(f"    D{idx:2d} {name}: +{delta}")

    # Digestion effort
    print(f"\n  Digestion effort by document:")
    for r in doc_results:
        print(f"    D{r['doc_index']:2d} {r['document']}: "
              f"{r['steps_to_digest']} steps {'OK' if r['converged'] else 'MAX'}")

    # Settled cycles detail
    if engine and engine.settlement.settled_cycles:
        print(f"\n  Settled cycles ({final_settled}):")
        for i, sc in enumerate(engine.settlement.settled_cycles[:20]):
            print(f"    [{i+1}] settled@step={sc.settled_at_step}: "
                  f"{sorted(sc.edges)[:3]}...")

    # Final terrain
    final_terrain = compute_terrain(cumulative_graph)
    ft_counts: dict[str, int] = {"tree": 0, "critical": 0}
    for mark in final_terrain.values():
        ft_counts[mark] = ft_counts.get(mark, 0) + 1
    crit_ratio = (ft_counts["critical"] / (ft_counts["tree"] + ft_counts["critical"])
                  if (ft_counts["tree"] + ft_counts["critical"]) > 0 else 0)
    print(f"\n  Final terrain: {ft_counts}")
    print(f"  Critical edge ratio: {crit_ratio:.4f}")

    # Edge type distribution
    type_counts: dict[str, int] = {}
    for e in cumulative_graph.active_edges():
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
        "experiment": "everything_claude_code_phi_L",
        "source": "everything-claude-code repository, 14 core documents",
        "methodology": "phi_L NLP pipeline (spaCy dependency parse), "
                       "code-block stripping, 10000-char cap, "
                       "incremental injection, traversal to 50-step beta_1 stability",
        "doc_results": doc_results,
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
            for sc in (engine.settlement.settled_cycles if engine else [])
        ],
        "blocked_log": engine.settlement.blocked_log if engine else [],
    }

    output_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "experiment_claude_code.json",
    )
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(output, f, indent=2, ensure_ascii=False)

    print(f"\n  Results saved to {output_path}")
    return output


if __name__ == "__main__":
    run_experiment()
