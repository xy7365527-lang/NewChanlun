"""Content router: detect modality → dispatch to appropriate parser.

Unified entry point for all content types. Detects input modality from
file extension, MIME type, or content inspection, then delegates to
the specialized parser.

Returns unified (vertices, edges) for K_active injection.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

from engine import Graph, Vertex, Edge


# ---------------------------------------------------------------------------
# Modality detection
# ---------------------------------------------------------------------------

_IMAGE_EXTS = {".png", ".jpg", ".jpeg", ".bmp", ".tiff", ".tif", ".webp"}
_AUDIO_EXTS = {".wav", ".mp3", ".flac", ".ogg", ".m4a"}
_VIDEO_EXTS = {".mp4", ".avi", ".mov", ".mkv", ".webm"}
_CODE_EXTS = {".py", ".js", ".ts", ".go", ".rs", ".java", ".c", ".cpp", ".h"}
_MESH_EXTS = {".obj", ".stl", ".ply", ".pcd"}
_TABLE_EXTS = {".csv", ".tsv"}


def detect_modality(input_data) -> str:
    """Detect the modality of input data.

    Returns one of: text, code, formula, citation, table, diagram, chart,
                    image, audio, video, mesh, unknown.
    """
    if isinstance(input_data, (str, Path)):
        path = Path(str(input_data))

        # File path detection
        if path.exists() and path.is_file():
            ext = path.suffix.lower()
            if ext in _IMAGE_EXTS:
                return "image"  # Could be diagram, chart, or generic image
            if ext in _AUDIO_EXTS:
                return "audio"
            if ext in _VIDEO_EXTS:
                return "video"
            if ext in _CODE_EXTS:
                return "code"
            if ext in _MESH_EXTS:
                return "mesh"
            if ext in _TABLE_EXTS:
                return "table"
            if ext == ".tex":
                return "formula"
            # Default for text files
            return "text"

        # String content detection
        text = str(input_data)

        # Table check BEFORE formula (markdown tables contain | which triggers formula)
        if _is_table(text):
            return "table"

        # LaTeX formula
        if _is_formula(text):
            return "formula"

        # Citations present
        if _has_citations(text):
            return "citation"  # Will also be processed as text

        # Default: text
        return "text"

    # Numpy array: image
    try:
        import numpy as np
        if isinstance(input_data, np.ndarray):
            if input_data.ndim in (2, 3):
                return "image"
    except ImportError:
        pass

    return "unknown"


def _is_formula(text: str) -> bool:
    """Check if text is primarily a mathematical formula."""
    latex_markers = [r"\frac", r"\partial", r"\sum", r"\int", r"\nabla",
                     r"\alpha", r"\beta", r"\theta", r"\sigma", r"\lambda"]
    latex_count = sum(1 for m in latex_markers if m in text)
    if latex_count >= 2:
        return True
    # Plain math: lots of = + - * / and few regular words
    math_chars = sum(1 for c in text if c in "=+-*/^(){}[]<>∂∑∫∇αβθσλ")
    alpha_chars = sum(1 for c in text if c.isalpha())
    if alpha_chars > 0 and math_chars / alpha_chars > 0.3:
        return True
    return False


def _is_table(text: str) -> bool:
    """Check if text is a table."""
    lines = text.strip().split("\n")
    if len(lines) < 2:
        return False
    # Markdown table: multiple | in multiple lines
    pipe_lines = sum(1 for line in lines if line.count("|") >= 2)
    if pipe_lines >= 2:
        return True
    # CSV: consistent comma count across lines
    if len(lines) >= 3:
        comma_counts = [line.count(",") for line in lines[:5]]
        if min(comma_counts) >= 1 and max(comma_counts) - min(comma_counts) <= 1:
            return True
    return False


_CITATION_PATTERN = re.compile(r'\[\d+\]|\([A-Z][a-z]+\s+\d{4}\)')


def _has_citations(text: str) -> bool:
    """Check if text contains citation markers."""
    return len(_CITATION_PATTERN.findall(text)) >= 2


# ---------------------------------------------------------------------------
# Dispatch
# ---------------------------------------------------------------------------

def parse(input_data, source: str = "auto", modality: str | None = None) -> tuple[list[Vertex], list[Edge]]:
    """Parse any input into vertices and edges.

    Args:
        input_data: File path, text string, or numpy array.
        source: Source label for vertex IDs.
        modality: Override modality detection (optional).

    Returns:
        (vertices, edges) ready for K_active injection.
    """
    if modality is None:
        modality = detect_modality(input_data)

    if modality == "text":
        from phi_L import phi_L
        g = phi_L(str(input_data))
        return list(g.vertices.values()), list(g.edges)

    if modality == "code":
        from code_ingest import ingest_file
        from engine import Graph
        g = ingest_file(str(input_data), Graph(), source=source)
        return list(g.vertices.values()), list(g.edges)

    if modality == "formula":
        from parsers.formula_parser import parse_formula
        return parse_formula(str(input_data), source=source)

    if modality == "citation":
        from parsers.citation_parser import parse_citations
        return parse_citations(str(input_data), source=source)

    if modality == "table":
        from parsers.table_parser import parse_table
        return parse_table(str(input_data), source=source)

    if modality == "diagram":
        from parsers.diagram_parser import parse_diagram
        return parse_diagram(input_data, source=source)

    if modality == "chart":
        from parsers.chart_parser import parse_chart
        return parse_chart(input_data, source=source)

    if modality == "image":
        # Distinguish diagram vs generic image
        # For now: if file exists, try diagram first; if few boxes, fall back to image
        from parsers.diagram_parser import parse_diagram
        verts, edges = parse_diagram(input_data, source=source)
        if len(verts) >= 3:
            return verts, edges
        # Fall back to persistent homology
        from parsers.image_parser import parse_image
        return parse_image(input_data, source=source)

    if modality in ("audio", "video", "mesh"):
        # Future modalities — return empty for now
        return [], []

    return [], []


def parse_and_inject(input_data, graph: Graph, source: str = "auto",
                     modality: str | None = None) -> Graph:
    """Parse input and inject results into an existing graph.

    Convenience function that combines parse() + graph injection.
    """
    vertices, edges = parse(input_data, source=source, modality=modality)

    existing_ids = set(graph.active_vertex_ids())
    for v in vertices:
        if v.id not in existing_ids:
            graph = graph.add_vertex(v)
            existing_ids.add(v.id)

    existing_edges = {(e.source, e.target, e.edge_type.value) for e in graph.edges}
    for e in edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_ids and e.target in existing_ids:
                graph = graph.add_edge(e)
                existing_edges.add(key)

    return graph
