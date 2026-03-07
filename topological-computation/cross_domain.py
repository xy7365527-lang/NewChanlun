"""Cross-domain edge detection: find semantic links between different source codebases.

Given a Graph with vertices from different source prefixes (e.g. "DeepSeek-V3:model.MoEGate"
and "Qwen:model.MoERouter"), detect candidate REFERENCE edges based on:
1. Content keyword overlap (Jaccard similarity on token sets)
2. ID name fuzzy matching (shared class/function name tokens)

Pure Python stdlib only.
"""

from __future__ import annotations

import re
from collections import defaultdict

from engine import Graph, Edge, EdgeType


# ---------------------------------------------------------------------------
# Tokenization
# ---------------------------------------------------------------------------

_CAMEL_SPLIT = re.compile(r"(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])")
_NON_ALPHA = re.compile(r"[^a-zA-Z0-9]+")


def _tokenize_id(vertex_id: str) -> set[str]:
    """Extract meaningful tokens from a vertex ID.

    "DeepSeek-V3:model.MoEGate.forward" -> {"deepseek", "v3", "model", "moe", "gate", "forward"}
    """
    # Remove source prefix (everything before first ':')
    _, _, name_part = vertex_id.partition(":")
    if not name_part:
        name_part = vertex_id

    # Split on dots, underscores, hyphens, etc.
    parts = _NON_ALPHA.sub(" ", name_part).split()

    tokens: set[str] = set()
    for part in parts:
        # CamelCase split
        sub_parts = _CAMEL_SPLIT.sub(" ", part).split()
        for sp in sub_parts:
            low = sp.lower()
            if len(low) >= 2:  # skip single chars
                tokens.add(low)

    return tokens


def _tokenize_content(content: str | None) -> set[str]:
    """Extract keyword tokens from vertex content (source code or text)."""
    if not content:
        return set()

    words = _NON_ALPHA.sub(" ", content).lower().split()
    # Filter short tokens and common noise
    return {w for w in words if len(w) >= 3}


# ---------------------------------------------------------------------------
# Scoring
# ---------------------------------------------------------------------------

def _jaccard(a: set[str], b: set[str]) -> float:
    """Jaccard similarity between two sets."""
    if not a or not b:
        return 0.0
    intersection = len(a & b)
    union = len(a | b)
    return intersection / union if union > 0 else 0.0


def _source_prefix(vertex_id: str) -> str:
    """Extract source prefix from vertex ID. Returns "" if no prefix."""
    prefix, _, _ = vertex_id.partition(":")
    return prefix if _ else ""


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def _adaptive_min_score(
    graph: Graph,
    existing_cross_edges: list[tuple[Edge, float]],
) -> float:
    """Derive min_score from existing cross-domain edge score distribution.

    If there are existing cross-domain edges, use the 10th percentile of their
    scores. Otherwise, fall back to 0.15.
    """
    if not existing_cross_edges:
        return 0.15
    scores = [s for _, s in existing_cross_edges]
    scores.sort()
    n = len(scores)
    if n == 1:
        return max(0.05, scores[0] * 0.8)
    k = 0.10 * (n - 1)
    f = int(k)
    c = min(f + 1, n - 1)
    d = k - f
    pct_10 = scores[f] * (1.0 - d) + scores[c] * d
    return max(0.05, pct_10)


def detect_cross_domain_edges(
    graph: Graph,
    min_score: float = 0.15,
) -> list[tuple[Edge, float]]:
    """Detect candidate cross-domain REFERENCE edges.

    Vertices are grouped by source prefix. For each pair of vertices from
    different sources, a similarity score is computed from:
    - Content token Jaccard similarity (weight 0.6)
    - ID token Jaccard similarity (weight 0.4)

    Returns (Edge, score) pairs sorted by score descending.
    """
    # Group vertices by source prefix
    groups: dict[str, list[str]] = defaultdict(list)
    for vid in graph.active_vertex_ids():
        groups[_source_prefix(vid)].append(vid)

    sources = list(groups.keys())
    if len(sources) < 2:
        return []

    # Pre-compute token sets
    id_tokens: dict[str, set[str]] = {}
    content_tokens: dict[str, set[str]] = {}
    for vid in graph.active_vertex_ids():
        id_tokens[vid] = _tokenize_id(vid)
        v = graph.vertex(vid)
        content_tokens[vid] = _tokenize_content(v.content if v else None)

    # Compare across source boundaries
    candidates: list[tuple[Edge, float]] = []

    for i in range(len(sources)):
        for j in range(i + 1, len(sources)):
            src_a_vids = groups[sources[i]]
            src_b_vids = groups[sources[j]]

            for vid_a in src_a_vids:
                id_tok_a = id_tokens[vid_a]
                cont_tok_a = content_tokens[vid_a]

                for vid_b in src_b_vids:
                    id_tok_b = id_tokens[vid_b]
                    cont_tok_b = content_tokens[vid_b]

                    content_score = _jaccard(cont_tok_a, cont_tok_b)
                    id_score = _jaccard(id_tok_a, id_tok_b)
                    combined = 0.6 * content_score + 0.4 * id_score

                    if combined >= min_score:
                        edge = Edge(
                            source=vid_a,
                            target=vid_b,
                            edge_type=EdgeType.REFERENCE,
                        )
                        candidates.append((edge, combined))

    candidates.sort(key=lambda pair: pair[1], reverse=True)
    return candidates


def inject_cross_domain_edges(
    graph: Graph,
    min_score: float | None = None,
    max_edges: int | None = None,
) -> Graph:
    """Detect and inject cross-domain REFERENCE edges into graph.

    min_score and max_edges are adaptive when not specified:
    - min_score: derived from existing cross-domain edge distribution (10th pct)
    - max_edges: int(active_vertices * 0.1), at least 100

    Returns a new Graph with cross-domain edges added.
    """
    active_vids = graph.active_vertex_ids()

    if max_edges is None:
        max_edges = max(100, int(len(active_vids) * 0.1))

    # First pass: detect with default threshold to get score distribution
    if min_score is None:
        initial_candidates = detect_cross_domain_edges(graph, min_score=0.15)
        min_score = _adaptive_min_score(graph, initial_candidates)
        # Re-detect with adaptive threshold if it changed
        if min_score < 0.15:
            candidates = detect_cross_domain_edges(graph, min_score=min_score)
        else:
            candidates = initial_candidates
    else:
        candidates = detect_cross_domain_edges(graph, min_score=min_score)

    # Avoid duplicate edges
    existing: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value) for e in graph.edges
    }

    added = 0
    for edge, _score in candidates:
        if added >= max_edges:
            break
        key = (edge.source, edge.target, edge.edge_type.value)
        if key not in existing:
            graph = graph.add_edge(edge)
            existing.add(key)
            added += 1

    return graph
