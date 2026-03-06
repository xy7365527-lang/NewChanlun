"""Directed code search: find matching code patterns in K_active.

Two modes:
- Keyword search: vertex labels containing keywords
- Topological search: f-value based similarity in local neighborhood

Pure Python, only stdlib. No external dependencies.
"""

from __future__ import annotations

from engine import Graph, EdgeType, compute_beta_1


# ---------------------------------------------------------------------------
# Keyword search
# ---------------------------------------------------------------------------

def search_by_keyword(graph: Graph, keyword: str) -> list[dict]:
    """Find vertices whose label/content contains keyword (case-insensitive).

    Returns list of {id, content_preview, neighbor_count, edge_types}.
    """
    keyword_lower = keyword.lower()
    results: list[dict] = []

    for vid in graph.active_vertex_ids():
        v = graph.vertex(vid)
        if v is None:
            continue
        searchable = f"{v.id} {v.content or ''}".lower()
        if keyword_lower in searchable:
            neighbors = graph.neighbors(vid)
            edge_types = _edge_types_for(graph, vid)
            content_preview = (v.content or "")[:200]
            results.append({
                "id": vid,
                "content_preview": content_preview,
                "neighbor_count": len(neighbors),
                "edge_types": edge_types,
            })

    # Sort by neighbor count descending (more connected = more central)
    results.sort(key=lambda r: r["neighbor_count"], reverse=True)
    return results


# ---------------------------------------------------------------------------
# Topological search (f-value similarity)
# ---------------------------------------------------------------------------

def search_by_topology(
    graph: Graph,
    target_vertex: str,
    max_results: int = 10,
) -> list[dict]:
    """Find vertices topologically similar to target (based on local structure).

    Similarity is measured by comparing local neighborhood structure:
    - Same number of in/out edges
    - Similar edge type distribution
    - Similar local beta_1 contribution

    Returns list of {id, similarity_score, content_preview}.
    """
    if graph.vertex(target_vertex) is None:
        return []

    target_profile = _vertex_profile(graph, target_vertex)
    results: list[dict] = []

    for vid in graph.active_vertex_ids():
        if vid == target_vertex:
            continue
        profile = _vertex_profile(graph, vid)
        score = _profile_similarity(target_profile, profile)
        v = graph.vertex(vid)
        results.append({
            "id": vid,
            "similarity_score": round(score, 4),
            "content_preview": ((v.content or "")[:200]) if v else "",
        })

    results.sort(key=lambda r: r["similarity_score"], reverse=True)
    return results[:max_results]


def find_pattern(graph: Graph, pattern_description: str) -> list[dict]:
    """High-level: extract keywords from description, search both modes.

    Combines keyword search and topological search results.
    Returns deduplicated list sorted by relevance.
    """
    # Extract keywords: split on spaces, filter short words
    words = pattern_description.lower().split()
    keywords = [w for w in words if len(w) >= 3]

    if not keywords:
        keywords = words[:3] if words else []

    # Keyword search across all keywords
    seen: set[str] = set()
    results: list[dict] = []

    for kw in keywords:
        for hit in search_by_keyword(graph, kw):
            if hit["id"] not in seen:
                seen.add(hit["id"])
                hit["match_keywords"] = [kw]
                results.append(hit)
            else:
                # Update existing hit with additional keyword match
                for r in results:
                    if r["id"] == hit["id"]:
                        r.setdefault("match_keywords", []).append(kw)
                        break

    # Sort by number of keyword matches, then neighbor count
    results.sort(
        key=lambda r: (len(r.get("match_keywords", [])), r["neighbor_count"]),
        reverse=True,
    )

    # If we have a top hit, also do topological search from it
    if results:
        top_id = results[0]["id"]
        topo_hits = search_by_topology(graph, top_id, max_results=5)
        for hit in topo_hits:
            if hit["id"] not in seen:
                seen.add(hit["id"])
                hit["match_keywords"] = ["(topological_neighbor)"]
                hit["neighbor_count"] = 0
                results.append(hit)

    return results


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _edge_types_for(graph: Graph, vid: str) -> list[str]:
    """Get the set of edge types connected to a vertex."""
    types: set[str] = set()
    for e in graph.active_edges():
        if e.source == vid or e.target == vid:
            types.add(e.edge_type.value)
    return sorted(types)


def _vertex_profile(graph: Graph, vid: str) -> dict:
    """Compute a structural profile for a vertex."""
    in_neighbors = graph.in_neighbors(vid)
    out_neighbors = graph.out_neighbors(vid)

    # Edge type counts
    in_types: dict[str, int] = {}
    out_types: dict[str, int] = {}
    for e in graph.active_edges():
        if e.target == vid:
            in_types[e.edge_type.value] = in_types.get(e.edge_type.value, 0) + 1
        if e.source == vid:
            out_types[e.edge_type.value] = out_types.get(e.edge_type.value, 0) + 1

    return {
        "in_degree": len(in_neighbors),
        "out_degree": len(out_neighbors),
        "total_degree": len(in_neighbors) + len(out_neighbors),
        "in_types": in_types,
        "out_types": out_types,
    }


def _profile_similarity(a: dict, b: dict) -> float:
    """Compute similarity between two vertex profiles (0.0 to 1.0)."""
    # Degree similarity (gaussian kernel)
    degree_diff = abs(a["total_degree"] - b["total_degree"])
    degree_sim = 1.0 / (1.0 + degree_diff)

    # In/out ratio similarity
    a_ratio = a["in_degree"] / max(a["out_degree"], 1)
    b_ratio = b["in_degree"] / max(b["out_degree"], 1)
    ratio_diff = abs(a_ratio - b_ratio)
    ratio_sim = 1.0 / (1.0 + ratio_diff)

    # Edge type overlap
    a_types = set(a["in_types"].keys()) | set(a["out_types"].keys())
    b_types = set(b["in_types"].keys()) | set(b["out_types"].keys())
    if a_types or b_types:
        type_sim = len(a_types & b_types) / len(a_types | b_types)
    else:
        type_sim = 1.0

    return (degree_sim + ratio_sim + type_sim) / 3.0
