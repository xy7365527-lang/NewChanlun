#!/usr/bin/env python3
"""Mount surface forms onto k_merged.json edges.

Reads data/chanlun_surface_forms.jsonl, matches each record to edges in
k_merged.json by comparing vertex content to term_a/term_b, then updates
the edge's surface and context fields. Saves the result.

The matching strategy:
  1. Build content -> vertex_id index from k_merged vertices
  2. Also build phi_L hash index (c_ + sha256[:12] of term)
  3. For each surface form record:
     a. Resolve term_a -> vertex_id(s) via content or phi_L hash
     b. Resolve term_b -> vertex_id(s)
     c. Find edge(s) connecting those vertex IDs (any direction)
     d. Attach surface + context to the first matching edge
  4. Save updated k_merged.json

Usage:
    python mount_surface_forms.py
    python mount_surface_forms.py --dry-run          # show stats without writing
    python mount_surface_forms.py --merged-path PATH  # custom k_merged.json path
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import time
from pathlib import Path

SURFACE_FORMS_PATH = Path(__file__).parent / "data" / "chanlun_surface_forms.jsonl"
DEFAULT_MERGED_PATH = Path.home() / ".swarm" / "k_merged.json"


def _phi_L_hash(term: str) -> str:
    """Compute the phi_L vertex ID for a term: c_ + sha256[:12]."""
    return "c_" + hashlib.sha256(term.encode("utf-8")).hexdigest()[:12]


def _load_surface_forms(path: Path) -> list[dict]:
    """Load surface forms from JSONL."""
    records: list[dict] = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                records.append(json.loads(line))
    return records


def _build_term_index(data: dict) -> dict[str, list[str]]:
    """Build term -> [vertex_id, ...] index from vertex content fields.

    Uses both exact content match and phi_L hash match.
    """
    content_to_ids: dict[str, list[str]] = {}

    for v in data["vertices"]:
        content = v.get("content")
        if content:
            content_to_ids.setdefault(content, []).append(v["id"])

    return content_to_ids


def _build_edge_index(data: dict) -> dict[tuple[str, str], int]:
    """Build (source, target) -> edge_index for fast lookup."""
    index: dict[tuple[str, str], int] = {}
    for i, e in enumerate(data["edges"]):
        key = (e["source"], e["target"])
        if key not in index:
            index[key] = i
    return index


def _resolve_term(
    term: str,
    content_index: dict[str, list[str]],
    vertex_ids: set[str],
    content_lookup: dict[str, str] | None = None,
) -> list[str]:
    """Resolve a surface form term to vertex IDs.

    Tries:
      1. Exact content match
      2. phi_L hash match (c_ + sha256[:12])
      3. Substring match: term appears in vertex content (for concept terms)
    """
    results: list[str] = []

    # Strategy 1: exact content match
    if term in content_index:
        results.extend(content_index[term])

    # Strategy 2: phi_L hash
    phi_id = _phi_L_hash(term)
    if phi_id in vertex_ids and phi_id not in results:
        results.append(phi_id)

    # Strategy 3: substring match (only if no exact/hash match found)
    if not results and content_lookup:
        for vid, content in content_lookup.items():
            if term in content:
                results.append(vid)
                if len(results) >= 5:
                    break

    return results


def mount_surface_forms(
    surface_forms_path: Path = SURFACE_FORMS_PATH,
    merged_path: Path = DEFAULT_MERGED_PATH,
    dry_run: bool = False,
) -> dict:
    """Mount surface forms onto k_merged.json edges.

    Returns a report dict with match statistics.
    """
    t0 = time.time()

    # Load inputs
    if not surface_forms_path.exists():
        raise FileNotFoundError(f"Surface forms not found: {surface_forms_path}")
    if not merged_path.exists():
        raise FileNotFoundError(f"k_merged.json not found: {merged_path}")

    print(f"Loading surface forms from {surface_forms_path}...")
    sf_records = _load_surface_forms(surface_forms_path)
    print(f"  {len(sf_records)} records loaded")

    print(f"Loading k_merged.json from {merged_path}...")
    data = json.loads(merged_path.read_text(encoding="utf-8"))
    n_vertices = len(data["vertices"])
    n_edges = len(data["edges"])
    print(f"  {n_vertices} vertices, {n_edges} edges")

    # Build indices
    content_index = _build_term_index(data)
    vertex_ids = {v["id"] for v in data["vertices"]}
    edge_index = _build_edge_index(data)

    # Build content lookup for substring matching: vid -> content
    content_lookup: dict[str, str] = {}
    for v in data["vertices"]:
        content = v.get("content")
        if content and v.get("status", "active") != "folded":
            content_lookup[v["id"]] = content

    # Also build reverse edge index for bidirectional matching
    reverse_edge_index: dict[tuple[str, str], int] = {}
    for i, e in enumerate(data["edges"]):
        key = (e["target"], e["source"])
        if key not in reverse_edge_index:
            reverse_edge_index[key] = i

    # Match and mount
    matched = 0
    unmatched_term_a = 0
    unmatched_term_b = 0
    unmatched_no_edge = 0
    edges_updated: set[int] = set()
    term_a_hits: set[str] = set()
    term_b_hits: set[str] = set()

    for rec in sf_records:
        term_a = rec["term_a"]
        term_b = rec["term_b"]
        surface = rec["surface"]
        context = rec["context"]

        # Resolve terms to vertex IDs
        vids_a = _resolve_term(term_a, content_index, vertex_ids, content_lookup)
        if not vids_a:
            unmatched_term_a += 1
            continue

        vids_b = _resolve_term(term_b, content_index, vertex_ids, content_lookup)
        if not vids_b:
            unmatched_term_b += 1
            continue

        # Find matching edge (try all combinations of resolved IDs)
        found_edge = False
        for va in vids_a:
            for vb in vids_b:
                if va == vb:
                    continue

                # Try forward direction
                edge_idx = edge_index.get((va, vb))
                if edge_idx is not None:
                    edge = data["edges"][edge_idx]
                    # Only update if surface is not already set
                    if not edge.get("surface"):
                        edge["surface"] = surface
                        edge["context"] = context
                        edges_updated.add(edge_idx)
                    matched += 1
                    found_edge = True
                    term_a_hits.add(term_a)
                    term_b_hits.add(term_b)
                    break

                # Try reverse direction
                edge_idx = edge_index.get((vb, va))
                if edge_idx is not None:
                    edge = data["edges"][edge_idx]
                    if not edge.get("surface"):
                        edge["surface"] = surface
                        edge["context"] = context
                        edges_updated.add(edge_idx)
                    matched += 1
                    found_edge = True
                    term_a_hits.add(term_a)
                    term_b_hits.add(term_b)
                    break

            if found_edge:
                break

        if not found_edge:
            unmatched_no_edge += 1

    elapsed = time.time() - t0

    # Report
    report = {
        "surface_forms_total": len(sf_records),
        "matched": matched,
        "edges_updated": len(edges_updated),
        "unmatched_term_a": unmatched_term_a,
        "unmatched_term_b": unmatched_term_b,
        "unmatched_no_edge": unmatched_no_edge,
        "unique_terms_matched": len(term_a_hits | term_b_hits),
        "elapsed_seconds": round(elapsed, 3),
        "dry_run": dry_run,
    }

    print(f"\n{'='*60}")
    print(f"  Surface Form Mounting Summary")
    print(f"{'='*60}")
    print(f"  Total surface forms:    {len(sf_records)}")
    print(f"  Matched to edges:       {matched}")
    print(f"  Unique edges updated:   {len(edges_updated)}")
    print(f"  Unmatched (term_a):     {unmatched_term_a}")
    print(f"  Unmatched (term_b):     {unmatched_term_b}")
    print(f"  Unmatched (no edge):    {unmatched_no_edge}")
    print(f"  Unique terms matched:   {len(term_a_hits | term_b_hits)}")
    print(f"  Time:                   {elapsed:.2f}s")

    if matched > 0:
        print(f"\n  Terms matched: {sorted(term_a_hits | term_b_hits)}")

    if dry_run:
        print(f"\n  --dry-run: not writing to {merged_path}")
    else:
        # Save updated k_merged.json
        merged_path.write_text(
            json.dumps(data, ensure_ascii=False),
            encoding="utf-8",
        )
        size_mb = merged_path.stat().st_size / 1024 / 1024
        print(f"\n  Saved to: {merged_path} ({size_mb:.1f} MB)")

    return report


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Mount surface forms onto k_merged.json edges"
    )
    parser.add_argument(
        "--surface-forms",
        default=str(SURFACE_FORMS_PATH),
        help=f"Surface forms JSONL path (default: {SURFACE_FORMS_PATH})",
    )
    parser.add_argument(
        "--merged-path",
        default=str(DEFAULT_MERGED_PATH),
        help=f"k_merged.json path (default: {DEFAULT_MERGED_PATH})",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show stats without writing",
    )

    args = parser.parse_args()

    mount_surface_forms(
        surface_forms_path=Path(args.surface_forms),
        merged_path=Path(args.merged_path),
        dry_run=args.dry_run,
    )


if __name__ == "__main__":
    main()
