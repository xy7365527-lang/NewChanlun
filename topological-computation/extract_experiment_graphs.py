"""Extract concept graphs from experiment_*.py files into JSON data files.

Each experiment file defines hand-crafted Vertex/Edge definitions for a thinker's
works. This script imports them, builds the combined Graph per thinker, and
serializes to data/graph_{thinker}.json in the same format as k_merged.json.

Skipped experiments:
- experiment_dev.py: synthetic test graph
- experiment_long_run.py: loads existing graph, no concept definitions
- experiment_real.py: loads existing graph, no concept definitions
- experiment_f_criterion.py: loads existing graph, no concept definitions
- experiment_hegel.py: phi_L-based (NLP pipeline), no hand-crafted concepts
- experiment_chanlun.py: phi_L-based (NLP pipeline on blog posts)
- experiment_claude_code.py: phi_L-based (NLP pipeline on markdown docs)
- experiment_merge_hegel_lacan.py: imports from other experiments, no unique data
"""

from __future__ import annotations

import importlib
import json
import os
import sys

# Ensure topological-computation is on path
script_dir = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, script_dir)

from engine import Graph, Vertex, Edge


def graph_to_dict(graph: Graph) -> dict:
    """Serialize a Graph to a JSON-compatible dict (same format as daemon.py)."""
    vertices = []
    for vid, v in graph.vertices.items():
        vertices.append({
            "id": v.id,
            "status": v.status.value,
            "content": v.content,
            "created_at": v.created_at,
        })
    edges = []
    for e in graph.edges:
        ed: dict = {
            "source": e.source,
            "target": e.target,
            "edge_type": e.edge_type.value,
            "created_at": e.created_at,
        }
        if e.surface is not None:
            ed["surface"] = e.surface
        if e.context is not None:
            ed["context"] = e.context
        edges.append(ed)
    return {"vertices": vertices, "edges": edges}


def build_graph_from_unit_fns(unit_fns: list) -> Graph:
    """Build a Graph by calling each unit function and merging vertices/edges."""
    vertices: dict[str, Vertex] = {}
    edges: list[Edge] = []
    existing_edge_keys: set[tuple[str, str, str]] = set()

    for fn in unit_fns:
        _name, verts, edgs = fn()
        for v in verts:
            if v.id not in vertices:
                vertices[v.id] = v
        for e in edgs:
            key = (e.source, e.target, e.edge_type.value)
            if key not in existing_edge_keys:
                if e.source in vertices and e.target in vertices:
                    edges.append(e)
                    existing_edge_keys.add(key)

    return Graph(vertices, edges)


def build_graph_direct(vertices_list: list[Vertex], edges_list: list[Edge]) -> Graph:
    """Build a Graph from raw vertex and edge lists."""
    vertices: dict[str, Vertex] = {}
    for v in vertices_list:
        vertices[v.id] = v
    valid_edges = [e for e in edges_list if e.source in vertices and e.target in vertices]
    return Graph(vertices, valid_edges)


# ---------------------------------------------------------------------------
# Experiment registry: module_name -> (thinker_slug, list_attr_or_builder)
#
# For standard experiments: list_attr is the name of the ALL_WORKS/etc list
# For special experiments: builder is a callable string to eval
# ---------------------------------------------------------------------------

STANDARD_EXPERIMENTS = {
    "experiment_simmel":            ("simmel",            "ALL_WORKS"),
    "experiment_adorno":            ("adorno",            "ALL_WORKS"),
    "experiment_spinoza":           ("spinoza",           "ALL_WORKS"),
    "experiment_nietzsche":         ("nietzsche",         "ALL_WORKS"),
    "experiment_marx":              ("marx",              "ALL_WORKS"),
    "experiment_lenin":             ("lenin",             "ALL_WORKS"),
    "experiment_mao":               ("mao",              "ALL_WORKS"),
    "experiment_heidegger":         ("heidegger",         "ALL_WORKS"),
    "experiment_wittgenstein":      ("wittgenstein",      "ALL_WORKS"),
    "experiment_schelling":         ("schelling",         "ALL_WORKS"),
    "experiment_holderlin":         ("holderlin",         "ALL_WORKS"),
    "experiment_zizek":             ("zizek",             "ALL_WORKS"),
    "experiment_postman":           ("postman",           "ALL_WORKS"),
    "experiment_merleau_ponty":     ("merleau_ponty",     "ALL_WORKS"),
    "experiment_benjamin":          ("benjamin",          "ALL_WORKS"),
    "experiment_foucault":          ("foucault",          "ALL_WORKS"),
    "experiment_deleuze":           ("deleuze",           "ALL_WORKS"),
    "experiment_derrida":           ("derrida",           "ALL_WORKS"),
    "experiment_hegel_full":        ("hegel_full",        "ALL_WORKS"),
    "experiment_mao_full":          ("mao_full",          "ALL_WORKS"),
    "experiment_phenomenology_full": ("phenomenology",    "ALL_CHAPTERS"),
    "experiment_lacan_ecrits":      ("lacan_ecrits",      "ALL_PAPERS"),
    "experiment_lacan_seminars":    ("lacan_seminars",     "ALL_SEMINARS"),
    "experiment_lacan_full":        ("lacan_full",         "ALL_TEXTS"),
}

# Experiments with build_*() functions that return a Graph directly
BUILDER_EXPERIMENTS = {
    "experiment_hegel_v2":            ("hegel_v2",           "build_hegel_complex"),
    "experiment_hegel_multi_chapter": ("hegel_multi_chapter", "build_multi_chapter_complex"),
}


def main() -> None:
    os.chdir(script_dir)
    data_dir = os.path.join(script_dir, "data")
    os.makedirs(data_dir, exist_ok=True)

    results: list[tuple[str, int, int]] = []

    # Process standard experiments (ALL_WORKS pattern)
    for module_name, (slug, list_attr) in sorted(STANDARD_EXPERIMENTS.items()):
        try:
            mod = importlib.import_module(module_name)
            unit_fns = getattr(mod, list_attr)
            graph = build_graph_from_unit_fns(unit_fns)

            n_v = len(graph.active_vertex_ids())
            n_e = len(graph.edges)

            if n_v == 0:
                print(f"  SKIP {slug}: 0 vertices")
                continue

            out_path = os.path.join(data_dir, f"graph_{slug}.json")
            with open(out_path, "w", encoding="utf-8") as f:
                json.dump(graph_to_dict(graph), f, indent=2, ensure_ascii=False)

            results.append((slug, n_v, n_e))
            print(f"  OK   {slug}: {n_v} vertices, {n_e} edges -> {out_path}")

        except Exception as exc:
            print(f"  FAIL {module_name}: {exc}")

    # Process builder experiments (build_*_complex pattern)
    for module_name, (slug, builder_name) in sorted(BUILDER_EXPERIMENTS.items()):
        try:
            mod = importlib.import_module(module_name)
            builder_fn = getattr(mod, builder_name)
            graph = builder_fn()

            n_v = len(graph.active_vertex_ids())
            n_e = len(graph.edges)

            if n_v == 0:
                print(f"  SKIP {slug}: 0 vertices")
                continue

            out_path = os.path.join(data_dir, f"graph_{slug}.json")
            with open(out_path, "w", encoding="utf-8") as f:
                json.dump(graph_to_dict(graph), f, indent=2, ensure_ascii=False)

            results.append((slug, n_v, n_e))
            print(f"  OK   {slug}: {n_v} vertices, {n_e} edges -> {out_path}")

        except Exception as exc:
            print(f"  FAIL {module_name}: {exc}")

    # Summary
    print("\n" + "=" * 60)
    print(f"Extracted {len(results)} concept graphs:")
    print(f"{'Thinker':<25} {'Vertices':>10} {'Edges':>10}")
    print("-" * 47)
    total_v, total_e = 0, 0
    for slug, nv, ne in sorted(results):
        print(f"{slug:<25} {nv:>10} {ne:>10}")
        total_v += nv
        total_e += ne
    print("-" * 47)
    print(f"{'TOTAL':<25} {total_v:>10} {total_e:>10}")


if __name__ == "__main__":
    main()
