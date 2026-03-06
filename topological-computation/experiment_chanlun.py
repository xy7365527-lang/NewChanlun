"""Chanlun (缠论) 108 Lessons — incremental topological computation experiment.

Reads the original 缠师博文 (108 lessons + preface) from docs/chanlun/text/blog/,
strips markdown formatting to plain text, and injects each lesson incrementally
into a single simplicial complex via phi_L.

After each lesson injection, the traversal engine runs until beta_1 convergence.
Records per-lesson beta_1 curve + cumulative complex parameters.

Output: experiment_chanlun.json
"""

from __future__ import annotations

import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, compute_beta_1
from phi_L import extract_vertices, extract_edges, extract_negations, merge_cross_sentence
from nlp_preprocess_zh import preprocess_zh, detect_language
from phi_L_batch import _inject_subgraph, _run_to_convergence, phi_L_multilingual
from traversal import TraversalEngine


# ---------------------------------------------------------------------------
# Markdown -> plain text
# ---------------------------------------------------------------------------

_IMG_RE = re.compile(r'!\[.*?\]\(.*?\)')
_LINK_RE = re.compile(r'\[([^\]]*)\]\([^\)]*\)')
_HEADING_RE = re.compile(r'^#{1,6}\s+', re.MULTILINE)
_BLOCKQUOTE_RE = re.compile(r'^>\s*', re.MULTILINE)
_HR_RE = re.compile(r'^---+\s*$', re.MULTILINE)
_BOLD_ITALIC_RE = re.compile(r'[*_]{1,3}([^*_]+)[*_]{1,3}')


def _md_to_text(md: str) -> str:
    """Strip markdown formatting to extract plain Chinese/text content."""
    text = md

    # Remove metadata block (everything before first ---)
    parts = text.split('---', 1)
    if len(parts) > 1:
        text = parts[1]

    # Remove images
    text = _IMG_RE.sub('', text)
    # Convert links to just their text
    text = _LINK_RE.sub(r'\1', text)
    # Remove heading markers
    text = _HEADING_RE.sub('', text)
    # Remove blockquote markers
    text = _BLOCKQUOTE_RE.sub('', text)
    # Remove horizontal rules
    text = _HR_RE.sub('', text)
    # Remove bold/italic markers but keep text
    text = _BOLD_ITALIC_RE.sub(r'\1', text)

    # Remove lines that are just the author/date pattern
    lines = text.split('\n')
    filtered = []
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        # Skip author line
        if stripped.startswith('作者：缠中说禅'):
            continue
        # Skip source/date metadata lines that might leak through
        if stripped.startswith('来源：') or stripped.startswith('发表日期：'):
            continue
        filtered.append(stripped)

    return '\n'.join(filtered)


# ---------------------------------------------------------------------------
# Collect blog files
# ---------------------------------------------------------------------------

def _collect_blog_files(blog_dir: str) -> list[tuple[str, str]]:
    """Collect blog markdown files sorted by lesson number.

    Returns list of (filename, filepath) tuples sorted by numeric prefix.
    """
    files = []
    for f in os.listdir(blog_dir):
        if f.endswith('.md') and f != 'INDEX.md':
            files.append(f)

    # Sort by numeric prefix (000, 001, ..., 108)
    def sort_key(fname: str) -> int:
        match = re.match(r'^(\d+)', fname)
        return int(match.group(1)) if match else 999

    files.sort(key=sort_key)
    return [(f, os.path.join(blog_dir, f)) for f in files]


# ---------------------------------------------------------------------------
# Main experiment
# ---------------------------------------------------------------------------

def run_experiment(
    blog_dir: str | None = None,
    output_json: str | None = None,
    settlement_threshold: int = 10,
    convergence_window: int = 50,
    seed: int = 42,
) -> dict:
    """Run the chanlun incremental experiment.

    Args:
        blog_dir: Path to docs/chanlun/text/blog/ directory.
                  Auto-detected if None.
        output_json: Output path. Defaults to experiment_chanlun.json
                     in the topological-computation directory.
        settlement_threshold: Steps a cycle must persist to settle.
        convergence_window: Steps beta_1 must be stable for convergence.
        seed: Random seed.

    Returns:
        Result dict with per-lesson and aggregate metrics.
    """
    # Auto-detect paths
    script_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.dirname(script_dir)

    if blog_dir is None:
        blog_dir = os.path.join(repo_root, 'docs', 'chanlun', 'text', 'blog')
    if output_json is None:
        output_json = os.path.join(script_dir, 'experiment_chanlun.json')

    if not os.path.isdir(blog_dir):
        raise FileNotFoundError(f"Blog directory not found: {blog_dir}")

    # Collect files
    blog_files = _collect_blog_files(blog_dir)
    print(f"Found {len(blog_files)} blog files in {blog_dir}")

    # Initialize
    cumulative_graph = Graph()
    engine = None
    results: dict = {
        "config": {
            "blog_dir": os.path.abspath(blog_dir),
            "settlement_threshold": settlement_threshold,
            "convergence_window": convergence_window,
            "seed": seed,
            "lesson_count": len(blog_files),
        },
        "lessons": [],
        "aggregate": {},
    }

    total_concepts_extracted = 0
    total_edges_extracted = 0

    for idx, (filename, filepath) in enumerate(blog_files):
        # Read and convert markdown to plain text
        with open(filepath, 'r', encoding='utf-8') as f:
            md_content = f.read()

        plain_text = _md_to_text(md_content)
        if not plain_text.strip():
            print(f"[{idx + 1}/{len(blog_files)}] {filename} — empty after stripping, skipped")
            continue

        t0 = time.time()

        # Detect language and build sub-graph
        lang = detect_language(plain_text)
        sub_graph = phi_L_multilingual(plain_text, lang)

        sub_v = len(sub_graph.active_vertex_ids())
        sub_e = len(sub_graph.active_edges())
        total_concepts_extracted += sub_v
        total_edges_extracted += sub_e

        # Record pre-injection state
        beta_before = compute_beta_1(cumulative_graph)
        v_before = len(cumulative_graph.active_vertex_ids())
        e_before = len(cumulative_graph.active_edges())

        # Inject into cumulative complex
        cumulative_graph = _inject_subgraph(cumulative_graph, sub_graph)

        beta_after_inject = compute_beta_1(cumulative_graph)
        v_after_inject = len(cumulative_graph.active_vertex_ids())
        e_after_inject = len(cumulative_graph.active_edges())

        # New vertices/edges from this lesson (after dedup/merge)
        new_v = v_after_inject - v_before
        new_e = e_after_inject - e_before

        # Run traversal to convergence
        active_ids = cumulative_graph.active_vertex_ids()
        steps = 0
        settled_count = 0
        if active_ids:
            start_vertex = active_ids[0]
            engine = TraversalEngine(
                cumulative_graph, start_vertex,
                settlement_threshold=settlement_threshold,
                seed=seed + idx,
            )
            steps = _run_to_convergence(engine, convergence_window)
            cumulative_graph = engine.k_active

        beta_after_traversal = compute_beta_1(cumulative_graph)
        settled_count = len(engine.settlement.settled_cycles) if engine else 0

        elapsed = time.time() - t0

        # Extract lesson number from filename
        lesson_match = re.match(r'^(\d+)', filename)
        lesson_num = int(lesson_match.group(1)) if lesson_match else idx

        lesson_result = {
            "lesson": lesson_num,
            "filename": filename,
            "language": lang,
            "text_length": len(plain_text),
            "sub_vertices": sub_v,
            "sub_edges": sub_e,
            "new_vertices": new_v,
            "new_edges": new_e,
            "cumulative_vertices": v_after_inject,
            "cumulative_edges": e_after_inject,
            "beta_1_before": beta_before,
            "beta_1_after_inject": beta_after_inject,
            "beta_1_after_traversal": beta_after_traversal,
            "traversal_steps": steps,
            "settled_cycles": settled_count,
            "elapsed_seconds": round(elapsed, 3),
        }
        results["lessons"].append(lesson_result)

        print(
            f"[{idx + 1}/{len(blog_files)}] {filename} "
            f"({lang}) +{new_v}v/+{new_e}e "
            f"total={v_after_inject}v/{e_after_inject}e "
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

    # beta_1 curve (lesson_num -> beta_1)
    beta_curve = [
        {"lesson": lr["lesson"], "beta_1": lr["beta_1_after_traversal"]}
        for lr in results["lessons"]
    ]

    results["aggregate"] = {
        "total_vertices": final_v,
        "total_edges": final_e,
        "final_beta_1": final_beta,
        "final_settled_cycles": final_settled,
        "edge_type_distribution": edge_type_counts,
        "total_concepts_extracted": total_concepts_extracted,
        "total_edges_extracted": total_edges_extracted,
        "total_steps": sum(lr["traversal_steps"] for lr in results["lessons"]),
        "total_elapsed": round(sum(lr["elapsed_seconds"] for lr in results["lessons"]), 3),
        "beta_1_curve": beta_curve,
    }

    # Write output
    with open(output_json, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    print(f"\n{'=' * 60}")
    print(f"Results written to {output_json}")
    print(f"Final: v={final_v} e={final_e} beta_1={final_beta} settled={final_settled}")
    print(f"Total concepts extracted (pre-dedup): {total_concepts_extracted}")
    print(f"Total edges extracted (pre-dedup): {total_edges_extracted}")
    print(f"Edge type distribution: {edge_type_counts}")
    print(f"{'=' * 60}")

    return results


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

if __name__ == '__main__':
    import argparse

    parser = argparse.ArgumentParser(
        description="Run chanlun 108 lessons incremental topological experiment"
    )
    parser.add_argument(
        '--blog-dir',
        default=None,
        help="Path to blog markdown directory (auto-detected if not set)",
    )
    parser.add_argument(
        '--output', '-o',
        default=None,
        help="Output JSON path (default: experiment_chanlun.json)",
    )
    parser.add_argument(
        '--settlement-threshold', '-s',
        type=int, default=10,
        help="Settlement threshold (default: 10)",
    )
    parser.add_argument(
        '--convergence-window', '-c',
        type=int, default=50,
        help="Convergence window (default: 50)",
    )
    parser.add_argument(
        '--seed',
        type=int, default=42,
        help="Random seed (default: 42)",
    )

    args = parser.parse_args()

    run_experiment(
        blog_dir=args.blog_dir,
        output_json=args.output,
        settlement_threshold=args.settlement_threshold,
        convergence_window=args.convergence_window,
        seed=args.seed,
    )
