#!/usr/bin/env python3
"""Ingest Chanlun 108 lessons (blog posts) into k_merged.json via phi_L.

Reads all .md files from docs/chanlun/text/blog/, strips markdown formatting,
runs phi_L (auto-detects Chinese), and merges into the existing k_merged.json
(same path and format used by ingest_all_and_traverse.py for the 19 thinkers).

Usage:
    python ingest_chanlun_blog.py
    python ingest_chanlun_blog.py --dry-run          # show stats without writing
    python ingest_chanlun_blog.py --traverse          # run daemon traversal after merge
    python ingest_chanlun_blog.py --max-files 10      # limit for testing
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
from pathlib import Path

# Force unbuffered output for progress visibility
os.environ["PYTHONUNBUFFERED"] = "1"

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from daemon import graph_to_dict, graph_from_dict
from phi_L import phi_L


# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

BLOG_DIR = Path(__file__).resolve().parent.parent / "docs" / "chanlun" / "text" / "blog"
SWARM_DIR = Path.home() / ".swarm"
MERGED_PATH = SWARM_DIR / "k_merged.json"
CACHE_DIR = SWARM_DIR / "ingest_cache"

SOURCE_LABEL = "chanlun-blog"


# ---------------------------------------------------------------------------
# Markdown stripping
# ---------------------------------------------------------------------------

def strip_markdown(text: str) -> str:
    """Remove markdown formatting, keeping meaningful text content.

    Strips: image tags, URLs, headers markers, emphasis markers,
    blockquote markers, horizontal rules, HTML tags.
    Preserves: the actual text content.
    """
    # Remove images ![alt](url)
    text = re.sub(r'!\[.*?\]\(.*?\)', '', text)
    # Remove links but keep text [text](url) -> text
    text = re.sub(r'\[([^\]]*)\]\([^)]*\)', r'\1', text)
    # Remove bare URLs
    text = re.sub(r'https?://\S+', '', text)
    # Remove HTML tags
    text = re.sub(r'<[^>]+>', '', text)
    # Remove horizontal rules
    text = re.sub(r'^---+\s*$', '', text, flags=re.MULTILINE)
    # Remove header markers but keep text
    text = re.sub(r'^#+\s*', '', text, flags=re.MULTILINE)
    # Remove emphasis markers
    text = re.sub(r'\*{1,3}([^*]+)\*{1,3}', r'\1', text)
    text = re.sub(r'_{1,3}([^_]+)_{1,3}', r'\1', text)
    # Remove blockquote markers
    text = re.sub(r'^>\s*', '', text, flags=re.MULTILINE)
    # Remove base64 image data
    text = re.sub(r'data:image/[^)]+', '', text)
    # Collapse multiple blank lines
    text = re.sub(r'\n{3,}', '\n\n', text)

    return text.strip()


# ---------------------------------------------------------------------------
# Graph merge (content-based dedup, same as ingest_all_and_traverse.py)
# ---------------------------------------------------------------------------

def merge_graphs(base: Graph, addition: Graph) -> tuple[Graph, int, int]:
    """Merge addition into base, skipping duplicate vertex IDs.

    Returns (merged_graph, new_vertices_count, new_edges_count).
    """
    existing_ids = set(base.active_vertex_ids())
    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in base.edges
    }

    new_v = 0
    new_e = 0

    for vid in addition.active_vertex_ids():
        if vid not in existing_ids:
            v = addition.vertex(vid)
            base = base.add_vertex(v)
            existing_ids.add(vid)
            new_v += 1

    for e in addition.edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_ids and e.target in existing_ids:
                base = base.add_edge(e)
                existing_edges.add(key)
                new_e += 1

    return base, new_v, new_e


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def ingest_chanlun_blog(
    blog_dir: Path = BLOG_DIR,
    merged_path: Path = MERGED_PATH,
    dry_run: bool = False,
    max_files: int = 0,
    run_traverse: bool = False,
) -> dict:
    """Ingest all Chanlun blog .md files into k_merged.json.

    Returns a report dict with per-file and aggregate metrics.
    """
    # Collect .md files (exclude INDEX.md)
    md_files = sorted(
        f for f in blog_dir.iterdir()
        if f.suffix == ".md" and f.name != "INDEX.md"
    )

    if max_files > 0:
        md_files = md_files[:max_files]

    if not md_files:
        raise ValueError(f"No .md files found in {blog_dir}")

    print(f"Found {len(md_files)} blog files in {blog_dir}")

    # Load existing merged graph (if exists)
    if merged_path.exists():
        print(f"Loading existing k_merged.json from {merged_path}...")
        data = json.loads(merged_path.read_text(encoding="utf-8"))
        merged = graph_from_dict(data)
        print(f"  Existing: {len(merged.active_vertex_ids())}V, "
              f"{len(merged.active_edges())}E, "
              f"beta_1={compute_beta_1(merged)}")
    else:
        print("No existing k_merged.json found, starting fresh.")
        merged = Graph()

    report: dict = {
        "source": SOURCE_LABEL,
        "blog_dir": str(blog_dir),
        "file_count": len(md_files),
        "dry_run": dry_run,
        "files": [],
        "aggregate": {},
    }

    total_new_v = 0
    total_new_e = 0
    total_sub_v = 0
    total_sub_e = 0

    for idx, md_file in enumerate(md_files):
        t0 = time.time()

        # Read and strip markdown
        raw_text = md_file.read_text(encoding="utf-8")
        clean_text = strip_markdown(raw_text)

        if len(clean_text) < 50:
            print(f"  [{idx+1}/{len(md_files)}] {md_file.name}: skipped (too short: {len(clean_text)} chars)")
            report["files"].append({
                "filename": md_file.name,
                "status": "skipped",
                "reason": "too_short",
                "text_length": len(clean_text),
            })
            continue

        # phi_L processing (auto-detects Chinese)
        try:
            sub_graph = phi_L(clean_text)
        except Exception as exc:
            print(f"  [{idx+1}/{len(md_files)}] {md_file.name}: phi_L error: {exc}")
            report["files"].append({
                "filename": md_file.name,
                "status": "error",
                "reason": str(exc),
                "text_length": len(clean_text),
            })
            continue

        sub_v = len(sub_graph.active_vertex_ids())
        sub_e = len(sub_graph.active_edges())
        total_sub_v += sub_v
        total_sub_e += sub_e

        # Merge into cumulative graph
        pre_v = len(merged.active_vertex_ids())
        pre_e = len(merged.active_edges())

        merged, new_v, new_e = merge_graphs(merged, sub_graph)
        total_new_v += new_v
        total_new_e += new_e

        elapsed = time.time() - t0

        file_result = {
            "filename": md_file.name,
            "status": "ok",
            "text_length": len(clean_text),
            "sub_vertices": sub_v,
            "sub_edges": sub_e,
            "new_vertices": new_v,
            "new_edges": new_e,
            "cumulative_vertices": len(merged.active_vertex_ids()),
            "cumulative_edges": len(merged.active_edges()),
            "elapsed_seconds": round(elapsed, 3),
        }
        report["files"].append(file_result)

        print(
            f"  [{idx+1}/{len(md_files)}] {md_file.name}: "
            f"phi_L={sub_v}V/{sub_e}E, new=+{new_v}V/+{new_e}E, "
            f"cumul={len(merged.active_vertex_ids())}V/{len(merged.active_edges())}E "
            f"({elapsed:.2f}s)"
        )

    # Aggregate
    final_v = len(merged.active_vertex_ids())
    final_e = len(merged.active_edges())
    final_beta = compute_beta_1(merged)

    report["aggregate"] = {
        "total_sub_vertices": total_sub_v,
        "total_sub_edges": total_sub_e,
        "total_new_vertices": total_new_v,
        "total_new_edges": total_new_e,
        "final_vertices": final_v,
        "final_edges": final_e,
        "final_beta_1": final_beta,
    }

    print(f"\n{'='*60}")
    print(f"  Chanlun Blog Ingestion Summary")
    print(f"{'='*60}")
    print(f"  Files processed: {len(md_files)}")
    print(f"  phi_L total:     {total_sub_v}V, {total_sub_e}E")
    print(f"  New (merged):    +{total_new_v}V, +{total_new_e}E")
    print(f"  Final graph:     {final_v}V, {final_e}E")
    print(f"  beta_1:          {final_beta}")

    if dry_run:
        print(f"\n  --dry-run: not writing to {merged_path}")
    else:
        # Save merged graph
        SWARM_DIR.mkdir(parents=True, exist_ok=True)
        merged_path.write_text(
            json.dumps(graph_to_dict(merged), ensure_ascii=False),
            encoding="utf-8",
        )
        size_mb = merged_path.stat().st_size / 1024 / 1024
        print(f"  Saved to: {merged_path} ({size_mb:.1f} MB)")

        # Also save report
        report_path = SWARM_DIR / "output" / "chanlun_blog_ingest_report.json"
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(
            json.dumps(report, indent=2, ensure_ascii=False),
            encoding="utf-8",
        )
        print(f"  Report:  {report_path}")

    # Optional: run daemon traversal after merge
    if run_traverse and not dry_run:
        print(f"\n{'='*60}")
        print(f"  Running daemon traversal...")
        print(f"{'='*60}")

        from daemon import TopologicalDaemon
        persist_path = SWARM_DIR / "k_merged_full.jsonl"
        daemon = TopologicalDaemon(
            graph=merged,
            settlement_threshold=15,
            persist_path=str(persist_path),
        )

        t0 = time.time()
        while daemon._crystallization_count < 1:
            daemon.run(max_steps=100)
            if len(daemon.k_active.active_vertex_ids()) < 3:
                break

        elapsed = time.time() - t0
        post_beta = compute_beta_1(daemon.k_active)

        print(f"  Steps:       {daemon.total_steps}")
        print(f"  Time:        {elapsed:.1f}s")
        print(f"  beta_1:      {final_beta} -> {post_beta}")
        print(f"  Events:      {daemon.total_events}")
        print(f"  Crystallized: {daemon._crystallization_count}")

    return report


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description="Ingest Chanlun 108 blog lessons into k_merged.json via phi_L"
    )
    parser.add_argument(
        "--blog-dir",
        default=str(BLOG_DIR),
        help=f"Blog directory (default: {BLOG_DIR})",
    )
    parser.add_argument(
        "--merged-path",
        default=str(MERGED_PATH),
        help=f"k_merged.json path (default: {MERGED_PATH})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show stats without writing to k_merged.json",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=0,
        help="Limit number of files to process (0 = all)",
    )
    parser.add_argument(
        "--traverse",
        action="store_true",
        help="Run daemon traversal after merge",
    )

    args = parser.parse_args()

    ingest_chanlun_blog(
        blog_dir=Path(args.blog_dir),
        merged_path=Path(args.merged_path),
        dry_run=args.dry_run,
        max_files=args.max_files,
        run_traverse=args.traverse,
    )


if __name__ == "__main__":
    main()
