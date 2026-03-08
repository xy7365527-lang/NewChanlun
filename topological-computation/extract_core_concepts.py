"""Extract handwritten concept graphs from experiment_*.py files into k_merged.json.

Parses Python AST to find Vertex(...) and Edge(...) literal calls.
Outputs core-layer vertices and edges for K_merged.

Usage:
    python topological-computation/extract_core_concepts.py
"""

from __future__ import annotations

import ast
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path


# ---------------------------------------------------------------------------
# Files to skip (no handwritten concept graphs)
# ---------------------------------------------------------------------------

SKIP_FILES = {
    "experiment_hegel.py",          # phi_L on text
    "experiment_chanlun.py",        # phi_L on markdown
    "experiment_claude_code.py",    # phi_L on .md files
    "experiment_real.py",           # loads from JSON
    "experiment_long_run.py",       # loads from JSON
    "experiment_f_criterion.py",    # loads from JSON / no concept literals
    "experiment_merge_hegel_lacan.py",  # imports from other experiments (duplicates)
    "experiment_dev.py",            # synthetic test data, not philosophical concepts
}

# ---------------------------------------------------------------------------
# Domain inference from filename
# ---------------------------------------------------------------------------

DOMAIN_MAP = {
    "experiment_hegel_v2.py": "hegel",
    "experiment_hegel_multi_chapter.py": "hegel",
    "experiment_phenomenology_full.py": "hegel",
    "experiment_hegel_full.py": "hegel",
    "experiment_lacan_ecrits.py": "lacan",
    "experiment_lacan_seminars.py": "lacan",
    "experiment_lacan_full.py": "lacan",
    "experiment_marx.py": "marx",
    "experiment_lenin.py": "lenin",
    "experiment_mao.py": "mao",
    "experiment_mao_full.py": "mao",
    "experiment_simmel.py": "simmel",
    "experiment_spinoza.py": "spinoza",
    "experiment_schelling.py": "schelling",
    "experiment_wittgenstein.py": "wittgenstein",
    "experiment_heidegger.py": "heidegger",
    "experiment_holderlin.py": "holderlin",
    "experiment_zizek.py": "zizek",
    "experiment_postman.py": "postman",
    "experiment_merleau_ponty.py": "merleau-ponty",
    "experiment_benjamin.py": "benjamin",
    "experiment_adorno.py": "adorno",
    "experiment_derrida.py": "derrida",
    "experiment_deleuze.py": "deleuze",
    "experiment_nietzsche.py": "nietzsche",
    "experiment_foucault.py": "foucault",
}

# For lacan_full which duplicates lacan_ecrits + lacan_seminars,
# and mao_full which extends mao, and hegel_full which extends phenomenology:
# We process all files but deduplicate vertices/edges by (id, domain)

# Files that are supersets of smaller ones — process the larger one,
# skip the smaller to avoid duplicate processing time
SUPERSEDED = {
    # lacan_full is a superset of lacan_ecrits + lacan_seminars
    "experiment_lacan_ecrits.py": "experiment_lacan_full.py",
    "experiment_lacan_seminars.py": "experiment_lacan_full.py",
    # mao_full is a superset of mao
    "experiment_mao.py": "experiment_mao_full.py",
}


# ---------------------------------------------------------------------------
# AST extraction
# ---------------------------------------------------------------------------

def _get_string_value(node: ast.expr) -> str | None:
    """Extract string value from an AST node."""
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    return None


def _get_attribute_value(node: ast.expr) -> str | None:
    """Extract EdgeType.X value from ast.Attribute."""
    if isinstance(node, ast.Attribute):
        return node.attr
    return None


def extract_from_file(filepath: str) -> tuple[list[dict], list[dict]]:
    """Parse a Python file and extract all Vertex(...) and Edge(...) calls.

    Returns (vertices, edges) where each is a list of dicts.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    try:
        tree = ast.parse(source)
    except SyntaxError:
        print(f"  WARN: SyntaxError in {filepath}, skipping")
        return [], []

    vertices: list[dict] = []
    edges: list[dict] = []

    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue

        # Identify Vertex(...) calls
        func = node.func
        func_name = None
        if isinstance(func, ast.Name):
            func_name = func.id
        elif isinstance(func, ast.Attribute):
            func_name = func.attr

        if func_name == "Vertex":
            vertex_id = None
            content = None

            # Positional args
            if len(node.args) >= 1:
                vertex_id = _get_string_value(node.args[0])
            if len(node.args) >= 2:
                content = _get_string_value(node.args[1])

            # Keyword args
            for kw in node.keywords:
                if kw.arg == "content":
                    content = _get_string_value(kw.value)
                elif kw.arg == "id" and vertex_id is None:
                    vertex_id = _get_string_value(kw.value)

            if vertex_id:
                vertices.append({
                    "id": vertex_id,
                    "content": content or vertex_id,
                })

        elif func_name == "Edge":
            source_id = None
            target_id = None
            edge_type = None

            # Positional args
            if len(node.args) >= 1:
                source_id = _get_string_value(node.args[0])
            if len(node.args) >= 2:
                target_id = _get_string_value(node.args[1])
            if len(node.args) >= 3:
                edge_type = _get_attribute_value(node.args[2])

            # Keyword args
            for kw in node.keywords:
                if kw.arg == "source" and source_id is None:
                    source_id = _get_string_value(kw.value)
                elif kw.arg == "target" and target_id is None:
                    target_id = _get_string_value(kw.value)
                elif kw.arg == "edge_type" and edge_type is None:
                    edge_type = _get_attribute_value(kw.value)

            if source_id and target_id:
                edges.append({
                    "source": source_id,
                    "target": target_id,
                    "edge_type": edge_type or "DEPENDENCY",
                })

    return vertices, edges


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    script_dir = Path(__file__).parent
    experiment_dir = script_dir

    # Collect experiment files
    experiment_files = sorted(
        f for f in os.listdir(experiment_dir)
        if f.startswith("experiment_") and f.endswith(".py")
    )

    # Filter
    to_process = []
    skipped_superseded = set()
    for f in experiment_files:
        if f in SKIP_FILES:
            continue
        if f in SUPERSEDED:
            # Skip the smaller file if its superset is available
            superset = SUPERSEDED[f]
            if superset in experiment_files:
                skipped_superseded.add(f)
                continue
        if f not in DOMAIN_MAP:
            print(f"  WARN: No domain mapping for {f}, skipping")
            continue
        to_process.append(f)

    print(f"Files to process: {len(to_process)}")
    if skipped_superseded:
        print(f"Skipped (superseded): {sorted(skipped_superseded)}")

    # Extract
    all_vertices: dict[str, dict] = {}  # id -> vertex dict (dedup by id)
    all_edges: set[tuple[str, str, str]] = set()  # (source, target, type) dedup
    edges_list: list[dict] = []
    domain_stats: dict[str, dict] = defaultdict(lambda: {"vertices": 0, "edges": 0})

    for filename in to_process:
        filepath = os.path.join(experiment_dir, filename)
        domain = DOMAIN_MAP[filename]

        vertices, edges = extract_from_file(filepath)

        v_count = 0
        for v in vertices:
            vid = v["id"]
            if vid not in all_vertices:
                all_vertices[vid] = {
                    "id": vid,
                    "content": v["content"],
                    "source": "core",
                    "domain": domain,
                }
                v_count += 1
            # If vertex already exists from same domain, skip
            # If from different domain, note multi-domain but keep first

        e_count = 0
        for e in edges:
            key = (e["source"], e["target"], e["edge_type"])
            if key not in all_edges:
                all_edges.add(key)
                edges_list.append({
                    "source": e["source"],
                    "target": e["target"],
                    "edge_type": e["edge_type"],
                    "origin_domain": domain,
                    "layer": "core",
                })
                e_count += 1

        domain_stats[domain]["vertices"] += v_count
        domain_stats[domain]["edges"] += e_count

        print(f"  {filename}: {len(vertices)} vertices ({v_count} new), "
              f"{len(edges)} edges ({e_count} new) [{domain}]")

    # Build k_merged.json
    k_merged = {
        "metadata": {
            "description": "Merged concept graph from handwritten experiment files",
            "extraction_method": "AST parsing of Vertex/Edge literals",
            "source_files": to_process,
            "skipped_files": sorted(SKIP_FILES),
            "superseded_files": sorted(skipped_superseded),
        },
        "vertices": list(all_vertices.values()),
        "edges": edges_list,
        "stats": {
            "total_vertices": len(all_vertices),
            "total_edges": len(edges_list),
            "domains": {
                domain: stats
                for domain, stats in sorted(domain_stats.items())
            },
        },
    }

    # Write
    output_path = os.path.join(script_dir, "data", "k_merged.json")
    os.makedirs(os.path.dirname(output_path), exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(k_merged, f, indent=2, ensure_ascii=False)

    print(f"\n{'=' * 60}")
    print(f"k_merged.json written to {output_path}")
    print(f"Total vertices: {len(all_vertices)}")
    print(f"Total edges: {len(edges_list)}")
    print(f"\nDomain breakdown:")
    for domain, stats in sorted(domain_stats.items()):
        print(f"  {domain}: {stats['vertices']} vertices, {stats['edges']} edges")
    print(f"{'=' * 60}")

    return k_merged


if __name__ == "__main__":
    main()
