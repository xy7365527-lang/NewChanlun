"""Citation parser: extract bibliographic references → REFERENCE edges.

Detects citation patterns in text:
- Numbered: [1], [2,3], [1-5]
- Author-year: (Smith 2020), (Smith & Jones, 2020), (Smith et al., 2020)
- Inline: Smith (2020), Smith and Jones (2020)

Each unique citation becomes a vertex. Citations in the same sentence
get REFERENCE edges to the sentence's concept vertices.

Pure regex. Zero LLM. Zero external dependencies.
"""

from __future__ import annotations

import re

from engine import Vertex, Edge, EdgeType, VertexStatus


# Patterns
_NUMBERED = re.compile(r'\[(\d+(?:\s*[-,]\s*\d+)*)\]')
_AUTHOR_YEAR_PAREN = re.compile(
    r'\(([A-Z][a-z]+(?:\s+(?:&|and)\s+[A-Z][a-z]+)?(?:\s+et\s+al\.?)?),?\s*(\d{4}[a-z]?)\)'
)
_AUTHOR_YEAR_INLINE = re.compile(
    r'([A-Z][a-z]+(?:\s+(?:&|and)\s+[A-Z][a-z]+)?(?:\s+et\s+al\.?)?)\s*\((\d{4}[a-z]?)\)'
)


def parse_citations(text: str, source: str = "citation") -> tuple[list[Vertex], list[Edge]]:
    """Extract citation vertices and reference edges from text.

    Returns:
        (vertices, edges) — citations as vertices, co-occurrence as REFERENCE edges.
    """
    citations: list[tuple[str, int]] = []  # (citation_label, char_position)

    # Numbered citations: [1], [2,3], [1-5]
    for m in _NUMBERED.finditer(text):
        inner = m.group(1)
        pos = m.start()
        # Expand ranges: [1-3] → [1], [2], [3]
        for part in inner.split(","):
            part = part.strip()
            if "-" in part:
                try:
                    lo, hi = part.split("-")
                    for n in range(int(lo.strip()), int(hi.strip()) + 1):
                        citations.append((f"ref_{n}", pos))
                except ValueError:
                    citations.append((f"ref_{part}", pos))
            else:
                citations.append((f"ref_{part.strip()}", pos))

    # Author-year (parenthetical): (Smith 2020)
    for m in _AUTHOR_YEAR_PAREN.finditer(text):
        author = m.group(1).strip()
        year = m.group(2).strip()
        citations.append((f"{author}_{year}", m.start()))

    # Author-year (inline): Smith (2020)
    for m in _AUTHOR_YEAR_INLINE.finditer(text):
        author = m.group(1).strip()
        year = m.group(2).strip()
        label = f"{author}_{year}"
        # Avoid duplicates from parenthetical pattern
        if not any(c[0] == label for c in citations):
            citations.append((label, m.start()))

    if not citations:
        return [], []

    # Create vertices
    vertices: list[Vertex] = []
    seen_vids: set[str] = set()
    vid_map: dict[str, str] = {}

    for label, _ in citations:
        vid = f"{source}:{label}"
        if vid not in seen_vids:
            seen_vids.add(vid)
            vertices.append(Vertex(id=vid, status=VertexStatus.ACTIVE, content=label))
            vid_map[label] = vid

    # Edges: citations that co-occur within 200 characters get REFERENCE edges
    edges: list[Edge] = []
    seen_edges: set[tuple[str, str]] = set()

    for i, (label_a, pos_a) in enumerate(citations):
        for label_b, pos_b in citations[i+1:]:
            if label_a == label_b:
                continue
            if abs(pos_a - pos_b) <= 200:
                vid_a = vid_map[label_a]
                vid_b = vid_map[label_b]
                key = (min(vid_a, vid_b), max(vid_a, vid_b))
                if key not in seen_edges:
                    seen_edges.add(key)
                    edges.append(Edge(source=vid_a, target=vid_b, edge_type=EdgeType.REFERENCE))

    return vertices, edges
