"""Batch code ingestion: parse directory of .py files into K_active.

Each vertex carries its full AST subtree as metadata for transplant operations.
Enhanced version of code_topology.parse_file — additionally stores source code
in vertex content for extraction during transplant.

Pure Python, only stdlib (ast, pathlib, argparse, json). No external dependencies.
"""

from __future__ import annotations

import argparse
import ast
import json
import sys
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

_EXCLUDED_DIRS = {"__pycache__", ".venv", "node_modules", ".git", "venv", "env"}


# ---------------------------------------------------------------------------
# AST visitor with full source extraction
# ---------------------------------------------------------------------------

class _IngestVisitor(ast.NodeVisitor):
    """Walk a Python AST and collect vertices with full source bodies."""

    def __init__(self, module_name: str, source_lines: list[str], source: str = "", domain_tag: str = "[domain:code]") -> None:
        self.module_name = module_name
        self.source_lines = source_lines
        self.source = source
        self.domain_tag = domain_tag
        self.vertices: list[Vertex] = []
        self.edges: list[Edge] = []
        self._defined_names: set[str] = set()
        self._scope_stack: list[str] = []

    def _make_id(self, name: str) -> str:
        prefix = f"{self.source}:" if self.source else ""
        return f"{prefix}{self.module_name}.{name}"

    def _current_scope_id(self) -> str | None:
        if self._scope_stack:
            return self._make_id(self._scope_stack[-1])
        return None

    def _extract_source(self, node: ast.AST) -> str:
        """Extract source code for an AST node using line numbers."""
        if not hasattr(node, 'lineno') or not hasattr(node, 'end_lineno'):
            return ""
        start = node.lineno - 1  # 0-indexed
        end = node.end_lineno     # end_lineno is 1-indexed, inclusive
        return "\n".join(self.source_lines[start:end])

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        self._handle_funcdef(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        self._handle_funcdef(node)

    def _annotation_to_str(self, node: ast.expr | None) -> str:
        """Convert an annotation AST node to a readable string."""
        if node is None:
            return ""
        if isinstance(node, ast.Constant):
            return repr(node.value)
        name = _extract_name(node)
        if name:
            return name
        # Fallback: unparse if available (Python 3.9+)
        try:
            return ast.unparse(node)
        except Exception:
            return "?"

    def _extract_signature(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
        """Extract function signature + docstring as concept-level content."""
        # Build signature: name(arg1, arg2, ...) -> return_type
        args = []
        for arg in node.args.args:
            name = arg.arg
            if arg.annotation:
                ann = self._annotation_to_str(arg.annotation)
                name = f"{name}: {ann}"
            args.append(name)
        sig = f"{node.name}({', '.join(args)})"
        if node.returns:
            ret = self._annotation_to_str(node.returns)
            sig += f" -> {ret}"
        # Append docstring if present
        docstring = ast.get_docstring(node)
        if docstring:
            # First line of docstring only
            first_line = docstring.split("\n")[0].strip()
            sig = f"{sig}\n{first_line}"
        return sig

    def _extract_class_signature(self, node: ast.ClassDef) -> str:
        """Extract class signature + docstring as concept-level content."""
        bases = [_extract_name(b) or "?" for b in node.bases]
        sig = f"class {node.name}" + (f"({', '.join(bases)})" if bases else "")
        docstring = ast.get_docstring(node)
        if docstring:
            first_line = docstring.split("\n")[0].strip()
            sig = f"{sig}\n{first_line}"
        return sig

    def _handle_funcdef(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        if self._scope_stack:
            qualified = f"{self._scope_stack[-1]}.{node.name}"
        else:
            qualified = node.name

        vid = self._make_id(qualified)
        content = f"{self.domain_tag} {self._extract_signature(node)}"
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=content,
        ))
        self._defined_names.add(vid)

        parent_id = self._current_scope_id()
        if parent_id is None:
            # Top-level definition: connect to module vertex
            parent_id = self._make_id("<module>")
        self.edges.append(Edge(
            source=parent_id,
            target=vid,
            edge_type=EdgeType.DEPENDENCY,
        ))

        self._scope_stack.append(qualified)
        self.generic_visit(node)
        self._scope_stack.pop()

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        if self._scope_stack:
            qualified = f"{self._scope_stack[-1]}.{node.name}"
        else:
            qualified = node.name

        vid = self._make_id(qualified)
        content = f"{self.domain_tag} {self._extract_class_signature(node)}"
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=content,
        ))
        self._defined_names.add(vid)

        for base in node.bases:
            base_name = _extract_name(base)
            if base_name:
                base_id = self._make_id(base_name)
                self.edges.append(Edge(
                    source=vid,
                    target=base_id,
                    edge_type=EdgeType.DEPENDENCY,
                ))

        parent_id = self._current_scope_id()
        if parent_id is None:
            # Top-level class: connect to module vertex
            parent_id = self._make_id("<module>")
        self.edges.append(Edge(
            source=parent_id,
            target=vid,
            edge_type=EdgeType.DEPENDENCY,
        ))

        self._scope_stack.append(qualified)
        self.generic_visit(node)
        self._scope_stack.pop()

    def visit_Import(self, node: ast.Import) -> None:
        """Import: only create edges, no vertices. Imports are relationships, not concepts."""
        scope_id = self._current_scope_id() or self._make_id("<module>")
        for alias in node.names:
            mod_id = f"<import>.{alias.name}"
            self._defined_names.add(mod_id)
            self.edges.append(Edge(
                source=scope_id,
                target=mod_id,
                edge_type=EdgeType.DEPENDENCY,
            ))

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        """ImportFrom: only create edges, no vertices."""
        scope_id = self._current_scope_id() or self._make_id("<module>")
        module = node.module or ""
        for alias in (node.names or []):
            mod_id = f"<import>.{module}.{alias.name}"
            self._defined_names.add(mod_id)
            self.edges.append(Edge(
                source=scope_id,
                target=mod_id,
                edge_type=EdgeType.DEPENDENCY,
            ))

    def visit_Call(self, node: ast.Call) -> None:
        caller = self._current_scope_id()
        if not caller:
            self.generic_visit(node)
            return
        callee_name = _extract_name(node.func)
        if callee_name:
            callee_id = self._make_id(callee_name)
            self.edges.append(Edge(
                source=caller,
                target=callee_id,
                edge_type=EdgeType.DEPENDENCY,
            ))
        self.generic_visit(node)


def _extract_name(node: ast.expr) -> str | None:
    """Extract a simple name from an AST expression node."""
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        base = _extract_name(node.value)
        if base:
            return f"{base}.{node.attr}"
        return node.attr
    return None


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def _detect_domain_tag(filepath: str) -> str:
    """Determine domain tag based on file path.

    Files inside topological-computation/ are the system's own code (domain:self).
    All other code is tagged as domain:code.
    """
    normalized = filepath.replace("\\", "/")
    if "topological-computation/" in normalized or "topological-computation\\" in filepath:
        return "[domain:self]"
    # Also check if the file IS in topological-computation (relative path)
    parts = Path(filepath).resolve().parts
    if "topological-computation" in parts:
        return "[domain:self]"
    return "[domain:code]"


def ingest_file(filepath: str, graph: Graph, source: str = "") -> Graph:
    """Parse single .py file, inject functions/classes/imports with full source.

    Unlike code_topology.parse_file, each vertex's content contains the full
    source code of the function/class body — not just the signature or docstring.

    Files in topological-computation/ are tagged [domain:self] (system's own code).
    All other files are tagged [domain:code].

    Args:
        filepath: Path to .py file.
        graph: Existing graph to merge into.
        source: Optional source label prefix for vertex IDs (e.g. "DeepSeek-V3").
    """
    with open(filepath, "r", encoding="utf-8") as f:
        file_source = f.read()

    source_lines = file_source.split("\n")
    module_name = Path(filepath).stem
    tree = ast.parse(file_source, filename=filepath)

    domain_tag = _detect_domain_tag(filepath)
    visitor = _IngestVisitor(module_name, source_lines, source=source, domain_tag=domain_tag)

    # Module-level vertex with module docstring
    prefix = f"{source}:" if source else ""
    module_id = f"{prefix}{module_name}.<module>"
    module_docstring = ast.get_docstring(tree) or module_name
    module_content = f"{domain_tag} {module_docstring[:400]}" if module_docstring else f"{domain_tag} {module_name}"
    visitor.vertices.append(Vertex(
        id=module_id,
        status=VertexStatus.ACTIVE,
        content=module_content,
    ))
    visitor._defined_names.add(module_id)

    visitor.visit(tree)

    # Merge into existing graph (batch to avoid O(n²) per-item copies)
    existing_ids = set(graph.active_vertex_ids())
    new_vertices = []
    for v in visitor.vertices:
        if v.id not in existing_ids:
            new_vertices.append(v)
            existing_ids.add(v.id)

    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in graph.edges
    }
    new_edges = []
    for e in visitor.edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_ids and e.target in existing_ids:
                new_edges.append(e)
                existing_edges.add(key)

    if new_vertices or new_edges:
        graph = graph.add_vertices_and_edges_batch(new_vertices, new_edges)

    return graph


def ingest_directory(dirpath: str, graph: Graph | None = None) -> Graph:
    """Parse all .py files in directory, inject into graph.

    Each function vertex carries its full source code in content field,
    enabling transplant operations to extract and adapt code.
    """
    if graph is None:
        graph = Graph()

    py_files = sorted(Path(dirpath).glob("*.py"))
    for py_file in py_files:
        graph = ingest_file(str(py_file), graph)

    return graph


def ingest_tree(dirpath: str, graph: Graph | None = None, source: str = "") -> Graph:
    """Recursively scan directory tree for .py files and ingest into graph.

    Excludes common non-source directories (__pycache__, .venv, node_modules,
    .git, venv, env). Prints progress every 50 files.

    Args:
        dirpath: Root directory to scan recursively.
        graph: Existing graph to merge into (creates new if None).
        source: Source label prefix for all vertex IDs (e.g. "DeepSeek-V3").
    """
    if graph is None:
        graph = Graph()

    root = Path(dirpath)
    count = 0
    skipped = 0

    for py_file in sorted(root.rglob("*.py")):
        # Skip excluded directories
        if any(part in _EXCLUDED_DIRS for part in py_file.parts):
            continue

        try:
            graph = ingest_file(str(py_file), graph, source=source)
        except SyntaxError:
            print(f"WARNING: skipping {py_file} (SyntaxError)", file=sys.stderr)
            skipped += 1
            count += 1
            continue

        count += 1
        if count % 50 == 0:
            print(f"  ingested {count} files...", file=sys.stderr)

    print(
        f"ingest_tree complete: {count} files processed, {skipped} skipped "
        f"({len(graph.active_vertex_ids())} vertices, {len(graph.edges)} edges)",
        file=sys.stderr,
    )
    return graph


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

def _graph_to_dict(graph: Graph) -> dict:
    """Serialize a Graph to a JSON-compatible dict."""
    return {
        "vertices": [
            {"id": v.id, "status": v.status.value, "content_length": len(v.content) if v.content else 0}
            for v in graph.vertices.values()
        ],
        "edges": [
            {"source": e.source, "target": e.target, "type": e.edge_type.value}
            for e in graph.edges
        ],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Ingest Python source tree into topological graph")
    parser.add_argument("--path", required=True, help="Root directory to scan")
    parser.add_argument("--source", required=True, help="Source label (e.g. DeepSeek-V3)")
    parser.add_argument("--output", default=None, help="Optional JSON output path")
    args = parser.parse_args()

    g = ingest_tree(args.path, source=args.source)

    print(f"\nStats: {len(g.active_vertex_ids())} active vertices, {len(g.edges)} edges")

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            json.dump(_graph_to_dict(g), f, indent=2, ensure_ascii=False)
        print(f"Graph exported to {args.output}")
