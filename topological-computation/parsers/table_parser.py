"""Table parser: row-column structured data → entity vertices + attribute edges.

Parses CSV/TSV tables and markdown tables. Each row becomes an entity vertex,
column headers become attribute labels, cell values become edge annotations.

Pure Python. Zero LLM.
"""

from __future__ import annotations

import csv
import io
import re

from engine import Vertex, Edge, EdgeType, VertexStatus


def parse_table(text: str, source: str = "table") -> tuple[list[Vertex], list[Edge]]:
    """Parse a table (CSV, TSV, or markdown) into vertices and edges.

    First column = entity (vertex). Remaining columns = attributes.
    Each non-empty cell creates a DEPENDENCY edge from entity to attribute value.

    Returns:
        (vertices, edges)
    """
    rows = _detect_and_parse(text)
    if not rows or len(rows) < 2:
        return [], []

    headers = rows[0]
    data_rows = rows[1:]

    vertices: list[Vertex] = []
    edges: list[Edge] = []
    seen_vids: set[str] = set()

    def _add_vertex(name: str, content: str | None = None) -> str:
        vid = f"{source}:table:{name}"
        if vid not in seen_vids:
            seen_vids.add(vid)
            vertices.append(Vertex(id=vid, status=VertexStatus.ACTIVE, content=content or name))
        return vid

    # Header vertices (attribute names)
    header_vids: list[str | None] = [None]  # Skip first column (entity)
    for h in headers[1:]:
        h = h.strip()
        if h:
            vid = _add_vertex(f"attr_{h}", f"attribute: {h}")
            header_vids.append(vid)
        else:
            header_vids.append(None)

    # Row vertices (entities) + edges
    for row in data_rows:
        if not row or not row[0].strip():
            continue

        entity_name = row[0].strip()
        entity_vid = _add_vertex(entity_name, entity_name)

        for col_idx in range(1, min(len(row), len(headers))):
            cell = row[col_idx].strip() if col_idx < len(row) else ""
            if not cell or cell == "-" or cell == "—":
                continue

            # Cell value as vertex
            value_vid = _add_vertex(f"{entity_name}_{headers[col_idx].strip()}_{cell[:30]}",
                                    f"{headers[col_idx].strip()}: {cell}")

            # Entity → value
            edges.append(Edge(source=entity_vid, target=value_vid, edge_type=EdgeType.DEPENDENCY))

            # Value → attribute header
            if col_idx < len(header_vids) and header_vids[col_idx]:
                edges.append(Edge(source=value_vid, target=header_vids[col_idx],
                                  edge_type=EdgeType.REFERENCE))

    return vertices, edges


def _detect_and_parse(text: str) -> list[list[str]]:
    """Detect table format and parse into list of rows."""
    text = text.strip()

    # Markdown table: lines with | separators
    if "|" in text and text.count("\n") >= 1:
        return _parse_markdown_table(text)

    # TSV: tabs present
    if "\t" in text:
        reader = csv.reader(io.StringIO(text), delimiter="\t")
        return [row for row in reader if any(cell.strip() for cell in row)]

    # CSV: commas with multiple lines
    if "," in text and "\n" in text:
        reader = csv.reader(io.StringIO(text))
        return [row for row in reader if any(cell.strip() for cell in row)]

    return []


_MD_SEP = re.compile(r'^[\s|:-]+$')


def _parse_markdown_table(text: str) -> list[list[str]]:
    """Parse markdown table (| col1 | col2 | ...)."""
    rows: list[list[str]] = []
    for line in text.strip().split("\n"):
        line = line.strip()
        if not line:
            continue
        # Skip separator lines (|---|---|)
        if _MD_SEP.match(line.replace("|", "").replace("-", "").replace(":", "")):
            continue
        # Split by | and strip
        cells = [c.strip() for c in line.split("|")]
        # Remove empty leading/trailing cells from | borders
        if cells and not cells[0]:
            cells = cells[1:]
        if cells and not cells[-1]:
            cells = cells[:-1]
        if cells:
            rows.append(cells)
    return rows
