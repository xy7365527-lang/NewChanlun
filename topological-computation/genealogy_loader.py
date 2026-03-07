"""Load block-topology data into the topological computation engine.

Reads blocks (vertices) and relations (edges) from the .chanlun/block-topology
directory and constructs an engine.Graph for analysis.

Also loads genealogy data from dag.yaml + block-topology/relations.jsonl for
proper genealogy graph construction.

Pure Python, no external dependencies.
"""

from __future__ import annotations

import json
import os
from collections import Counter
from pathlib import Path

from engine import (
    Edge,
    EdgeType,
    Graph,
    Vertex,
    compute_beta_1,
    _connected_components,
)

# ---------------------------------------------------------------------------
# Relation type → EdgeType mapping
# ---------------------------------------------------------------------------

_RELATION_MAP: dict[str, EdgeType] = {
    "depends_on": EdgeType.DEPENDENCY,
    "defines": EdgeType.DEPENDENCY,
    "refines": EdgeType.DEPENDENCY,
    "extends": EdgeType.DEPENDENCY,
    "references": EdgeType.REFERENCE,
    "records": EdgeType.REFERENCE,
    "related": EdgeType.REFERENCE,
    "residue_of": EdgeType.REFERENCE,
    "splits": EdgeType.REFERENCE,
    "negates": EdgeType.NEGATION,
    "negated_by": EdgeType.NEGATION,
    "tensions_with": EdgeType.NEGATION,
    "supersedes": EdgeType.DEPENDENCY,
}


# ---------------------------------------------------------------------------
# Content summarization
# ---------------------------------------------------------------------------

def _summarize_content(block: dict) -> str:
    """Extract a summary string from a block's type + content (max 200 chars)."""
    block_type = block.get("type", "")
    content = block.get("content", {})

    if not isinstance(content, dict):
        raw = f"[{block_type}] {content}"
        return raw[:200]

    # Try common fields in order of informativeness
    title = content.get("title", "")
    if title:
        raw = f"[{block_type}] {title}"
        return raw[:200]

    genealogy_id = content.get("genealogy_id") or content.get("id", "")
    if genealogy_id:
        status = content.get("status", "")
        sub_type = content.get("type", "")
        parts = [f"[{block_type}]", f"#{genealogy_id}"]
        if sub_type:
            parts.append(sub_type)
        if status:
            parts.append(status)
        raw = " ".join(parts)
        return raw[:200]

    # For tension blocks with unresolved list
    unresolved = content.get("unresolved")
    if isinstance(unresolved, list):
        items = ", ".join(unresolved[:5])
        raw = f"[{block_type}] unresolved: {items}"
        return raw[:200]

    # For rewrite blocks
    action = content.get("action", "")
    original = content.get("original_block", "")
    if action:
        raw = f"[{block_type}] {action}"
        if original:
            raw += f" of {original[:16]}..."
        return raw[:200]

    # Fallback: stringify keys
    keys = list(content.keys())[:5]
    raw = f"[{block_type}] keys={keys}"
    return raw[:200]


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------

def load_blocks(blocks_dir: str | Path) -> dict[str, dict]:
    """Load all block JSON files from the blocks directory.

    Returns {block_id: raw_dict}.
    """
    blocks: dict[str, dict] = {}
    blocks_path = Path(blocks_dir)
    if not blocks_path.is_dir():
        return blocks

    for f in blocks_path.iterdir():
        if f.suffix != ".json":
            continue
        with open(f, encoding="utf-8") as fh:
            data = json.load(fh)
        block_id = data.get("id", f.stem)
        blocks[block_id] = data
    return blocks


def load_relations(relations_path: str | Path) -> list[dict]:
    """Load relations from a JSONL file.

    Returns list of raw relation dicts.
    """
    relations: list[dict] = []
    path = Path(relations_path)
    if not path.is_file():
        return relations

    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            relations.append(json.loads(line))
    return relations


def build_graph(
    blocks: dict[str, dict],
    relations: list[dict],
) -> tuple[Graph, dict[str, int]]:
    """Construct a Graph from blocks and relations.

    Returns (graph, relation_type_counts).

    Vertices are created from blocks. Relations that reference vertices not
    present in blocks cause auto-creation of vertices with empty content.
    """
    graph = Graph()
    relation_counts: dict[str, int] = Counter()

    # Step 1: Add all block vertices
    for block_id, block_data in blocks.items():
        summary = _summarize_content(block_data)
        v = Vertex(id=block_id, content=summary)
        graph = graph.add_vertex(v)

    # Step 2: Process relations
    seen_vertex_ids = set(blocks.keys())

    for rel in relations:
        src = rel.get("from", "")
        tgt = rel.get("to", "")
        relation_type = rel.get("relation", "")

        if not src or not tgt:
            continue

        # Auto-create missing vertices
        for vid in (src, tgt):
            if vid not in seen_vertex_ids:
                v = Vertex(id=vid, content="")
                graph = graph.add_vertex(v)
                seen_vertex_ids.add(vid)

        # Map relation type to EdgeType
        edge_type = _RELATION_MAP.get(relation_type)
        if edge_type is None:
            # Unknown or empty relation type → skip
            relation_counts[f"(skipped:{relation_type or 'empty'})"] += 1
            continue

        edge = Edge(source=src, target=tgt, edge_type=edge_type)
        graph = graph.add_edge(edge)
        relation_counts[relation_type] += 1

    return graph, dict(relation_counts)


# ---------------------------------------------------------------------------
# Statistics
# ---------------------------------------------------------------------------

def print_statistics(graph: Graph, relation_counts: dict[str, int]) -> None:
    """Print graph statistics: vertices, edges, relation distribution, β₁, components."""
    active_vids = graph.active_vertex_ids()
    edges = graph.active_edges()
    undirected = graph.undirected_active_edges()

    print(f"Vertices:   {len(active_vids)}")
    print(f"Edges:      {len(edges)} (directed), {len(undirected)} (undirected)")
    print()

    print("Relation type distribution:")
    for rel_type, count in sorted(relation_counts.items(), key=lambda x: -x[1]):
        print(f"  {rel_type}: {count}")
    print()

    # Edge type distribution
    edge_type_counts: dict[str, int] = Counter()
    for e in edges:
        edge_type_counts[e.edge_type.value] += 1
    print("EdgeType distribution:")
    for et, count in sorted(edge_type_counts.items(), key=lambda x: -x[1]):
        print(f"  {et}: {count}")
    print()

    beta_1 = compute_beta_1(graph)
    print(f"beta_1:     {beta_1}")

    components = _connected_components(active_vids, undirected)
    print(f"Components: {components}")


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def load_from_repo(repo_root: str | Path) -> tuple[Graph, dict[str, int]]:
    """Load graph from the standard repo layout."""
    root = Path(repo_root)
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    relations_path = root / ".chanlun" / "block-topology" / "relations.jsonl"

    blocks = load_blocks(blocks_dir)
    relations = load_relations(relations_path)
    return build_graph(blocks, relations)


def main() -> None:
    # Determine repo root: two levels up from this file
    here = Path(__file__).resolve().parent
    repo_root = here.parent

    print(f"Loading from: {repo_root}")
    print()

    graph, relation_counts = load_from_repo(repo_root)
    print_statistics(graph, relation_counts)


if __name__ == "__main__":
    main()
