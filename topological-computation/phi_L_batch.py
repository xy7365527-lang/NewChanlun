"""phi_L_batch: Batch processing pipeline for text directories.

Processes all text files in a directory, sorted by filename, injecting each
incrementally into a single simplicial complex. After each file injection,
the traversal engine runs until convergence (beta_1 stable for convergence_window steps).

CLI: python phi_L_batch.py /path/to/texts/ --output results.json
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1
from phi_L import phi_L, extract_vertices, extract_edges, extract_negations, merge_cross_sentence
from nlp_preprocess import preprocess, SentenceTree
from nlp_preprocess_zh import preprocess_zh, detect_language
from traversal import TraversalEngine
from vertex_cleaning import CHANLUN_WHITELIST


# ---------------------------------------------------------------------------
# Incremental injection
# ---------------------------------------------------------------------------

def _inject_subgraph(base: Graph, sub: Graph) -> Graph:
    """Inject sub-graph into base graph, merging vertices by content label.

    Vertices with the same content (normalized label) are identified.
    New vertices and edges are added; existing ones are preserved.
    """
    # Build content -> vertex_id mapping for base
    base_content_to_id: dict[str, str] = {}
    for vid, v in base.vertices.items():
        if v.content:
            base_content_to_id[v.content] = vid

    # Map sub vertex ids to base vertex ids (or new ids)
    sub_to_base: dict[str, str] = {}
    next_id = len(base.vertices)
    result = base

    for vid, v in sub.vertices.items():
        if v.content and v.content in base_content_to_id:
            # Existing concept — map to base vertex
            sub_to_base[vid] = base_content_to_id[v.content]
        else:
            # New concept — add with new id
            new_id = f"v{next_id}"
            next_id += 1
            sub_to_base[vid] = new_id
            new_v = Vertex(id=new_id, content=v.content)
            result = result.add_vertex(new_v)
            if v.content:
                base_content_to_id[v.content] = new_id

    # Add edges, remapping vertex ids
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


def _run_to_convergence(
    engine: TraversalEngine,
    convergence_window: int,
    max_steps: int = 2000,
) -> int:
    """Run traversal engine until beta_1 is stable for convergence_window steps.

    Returns the number of steps taken.
    """
    stable_count = 0
    last_beta = compute_beta_1(engine.k_active)
    steps_taken = 0

    for _ in range(max_steps):
        engine.run_step()
        steps_taken += 1
        current_beta = compute_beta_1(engine.k_active)
        if current_beta == last_beta:
            stable_count += 1
        else:
            stable_count = 0
            last_beta = current_beta
        if stable_count >= convergence_window:
            break

    return steps_taken


# ---------------------------------------------------------------------------
# Batch processing
# ---------------------------------------------------------------------------

def batch_process(
    text_dir: str,
    output_json: str,
    settlement_threshold: int = 10,
    convergence_window: int = 50,
    seed: int = 42,
) -> dict:
    """Batch process all text files in a directory.

    Files are sorted by name. Each file is:
    1. Read and preprocessed (auto-detect Chinese/English)
    2. Mapped to a sub-graph via phi_L pipeline
    3. Injected into the cumulative complex
    4. Traversal engine runs to convergence (beta_1 stable for convergence_window steps)
    5. Parameters recorded

    Returns a result dict with per-file and aggregate metrics.
    """
    # Collect and sort text files
    text_files = sorted(
        f for f in os.listdir(text_dir)
        if f.endswith(".txt") and os.path.isfile(os.path.join(text_dir, f))
    )

    if not text_files:
        raise ValueError(f"No .txt files found in {text_dir}")

    cumulative_graph = Graph()
    engine = None
    results: dict = {
        "config": {
            "text_dir": os.path.abspath(text_dir),
            "settlement_threshold": settlement_threshold,
            "convergence_window": convergence_window,
            "seed": seed,
            "file_count": len(text_files),
        },
        "files": [],
        "aggregate": {},
    }

    for file_idx, filename in enumerate(text_files):
        filepath = os.path.join(text_dir, filename)
        with open(filepath, "r", encoding="utf-8") as f:
            text = f.read()

        t0 = time.time()

        # Auto-detect language and preprocess
        lang = detect_language(text)
        sub_graph = phi_L_multilingual(text, lang)

        # Record pre-injection state
        beta_before = compute_beta_1(cumulative_graph)
        v_before = len(cumulative_graph.active_vertex_ids())
        e_before = len(cumulative_graph.active_edges())

        # Inject into cumulative complex
        cumulative_graph = _inject_subgraph(cumulative_graph, sub_graph)

        beta_after_inject = compute_beta_1(cumulative_graph)
        v_after_inject = len(cumulative_graph.active_vertex_ids())
        e_after_inject = len(cumulative_graph.active_edges())

        # Run traversal to convergence
        active_ids = cumulative_graph.active_vertex_ids()
        if active_ids:
            start_vertex = active_ids[0]
            engine = TraversalEngine(
                cumulative_graph, start_vertex,
                settlement_threshold=settlement_threshold,
                seed=seed + file_idx,
            )
            steps = _run_to_convergence(engine, convergence_window)
            cumulative_graph = engine.k_active
        else:
            steps = 0

        beta_after_traversal = compute_beta_1(cumulative_graph)
        settled_count = len(engine.settlement.settled_cycles) if engine else 0

        elapsed = time.time() - t0

        file_result = {
            "filename": filename,
            "language": lang,
            "text_length": len(text),
            "sub_vertices": len(sub_graph.active_vertex_ids()),
            "sub_edges": len(sub_graph.active_edges()),
            "cumulative_vertices_before": v_before,
            "cumulative_vertices_after": v_after_inject,
            "cumulative_edges_before": e_before,
            "cumulative_edges_after": e_after_inject,
            "beta_1_before": beta_before,
            "beta_1_after_inject": beta_after_inject,
            "beta_1_after_traversal": beta_after_traversal,
            "traversal_steps": steps,
            "settled_cycles": settled_count,
            "elapsed_seconds": round(elapsed, 3),
        }
        results["files"].append(file_result)

        print(
            f"[{file_idx + 1}/{len(text_files)}] {filename} "
            f"({lang}) v={v_after_inject} e={e_after_inject} "
            f"beta_1={beta_after_traversal} settled={settled_count} "
            f"steps={steps} ({elapsed:.2f}s)"
        )

    # Aggregate metrics
    final_beta = compute_beta_1(cumulative_graph)
    final_v = len(cumulative_graph.active_vertex_ids())
    final_e = len(cumulative_graph.active_edges())
    final_settled = len(engine.settlement.settled_cycles) if engine else 0

    edge_type_counts: dict[str, int] = {}
    for e in cumulative_graph.active_edges():
        edge_type_counts[e.edge_type.value] = edge_type_counts.get(e.edge_type.value, 0) + 1

    results["aggregate"] = {
        "total_vertices": final_v,
        "total_edges": final_e,
        "final_beta_1": final_beta,
        "final_settled_cycles": final_settled,
        "edge_type_distribution": edge_type_counts,
        "total_steps": sum(f["traversal_steps"] for f in results["files"]),
        "total_elapsed": round(sum(f["elapsed_seconds"] for f in results["files"]), 3),
    }

    # Write output
    with open(output_json, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    print(f"\nResults written to {output_json}")
    print(
        f"Final: v={final_v} e={final_e} beta_1={final_beta} "
        f"settled={final_settled}"
    )

    return results


# ---------------------------------------------------------------------------
# Multilingual phi_L wrapper
# ---------------------------------------------------------------------------

def phi_L_multilingual(text: str, lang: str | None = None) -> Graph:
    """phi_L with automatic language detection.

    If lang is None, auto-detects. Uses Chinese preprocessor for 'zh',
    English preprocessor for 'en'.

    Only whitelisted terms produce vertices — phi_L is a term matcher, not a generator.
    """
    if lang is None:
        lang = detect_language(text)

    if lang == "zh":
        trees = preprocess_zh(text)
    else:
        trees = preprocess(text)

    vertices, token_to_vertex = extract_vertices(trees, CHANLUN_WHITELIST)
    edges = extract_edges(trees, token_to_vertex)
    neg_edges = extract_negations(trees, token_to_vertex)

    all_edges = edges + neg_edges
    vertices, all_edges = merge_cross_sentence(vertices, all_edges)

    # Deduplicate edges
    seen: set[tuple[str, str, str]] = set()
    deduped: list[Edge] = []
    for e in all_edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in seen:
            seen.add(key)
            deduped.append(e)

    g = Graph()
    for v in vertices:
        g = g.add_vertex(v)
    for e in deduped:
        if g.vertex(e.source) is not None and g.vertex(e.target) is not None:
            g = g.add_edge(e)

    return g


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Batch process text files through phi_L topological pipeline"
    )
    parser.add_argument(
        "text_dir",
        help="Directory containing .txt files to process",
    )
    parser.add_argument(
        "--output", "-o",
        default="batch_results.json",
        help="Output JSON file path (default: batch_results.json)",
    )
    parser.add_argument(
        "--settlement-threshold", "-s",
        type=int, default=10,
        help="Settlement threshold (steps a cycle must persist) (default: 10)",
    )
    parser.add_argument(
        "--convergence-window", "-c",
        type=int, default=50,
        help="Convergence window (steps beta_1 must be stable) (default: 50)",
    )
    parser.add_argument(
        "--seed",
        type=int, default=42,
        help="Random seed (default: 42)",
    )

    args = parser.parse_args()

    batch_process(
        text_dir=args.text_dir,
        output_json=args.output,
        settlement_threshold=args.settlement_threshold,
        convergence_window=args.convergence_window,
        seed=args.seed,
    )


if __name__ == "__main__":
    main()
