"""Batch code ingestion: parse directory of .py files into K_active.

Each vertex carries its full AST subtree as metadata for transplant operations.
Enhanced version of code_topology.parse_file — additionally stores source code
in vertex content for extraction during transplant.

Pure Python, only stdlib (ast, pathlib). No external dependencies.
"""

from __future__ import annotations

import ast
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus


# ---------------------------------------------------------------------------
# AST visitor with full source extraction
# ---------------------------------------------------------------------------

class _IngestVisitor(ast.NodeVisitor):
    """Walk a Python AST and collect vertices with full source bodies."""

    def __init__(self, module_name: str, source_lines: list[str]) -> None:
        self.module_name = module_name
        self.source_lines = source_lines
        self.vertices: list[Vertex] = []
        self.edges: list[Edge] = []
        self._defined_names: set[str] = set()
        self._scope_stack: list[str] = []

    def _make_id(self, name: str) -> str:
        return f"{self.module_name}.{name}"

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

    def _handle_funcdef(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        if self._scope_stack:
            qualified = f"{self._scope_stack[-1]}.{node.name}"
        else:
            qualified = node.name

        vid = self._make_id(qualified)
        source_body = self._extract_source(node)
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=source_body,
        ))
        self._defined_names.add(vid)

        parent_id = self._current_scope_id()
        if parent_id:
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
        source_body = self._extract_source(node)
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=source_body,
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
        if parent_id:
            self.edges.append(Edge(
                source=parent_id,
                target=vid,
                edge_type=EdgeType.DEPENDENCY,
            ))

        self._scope_stack.append(qualified)
        self.generic_visit(node)
        self._scope_stack.pop()

    def visit_Import(self, node: ast.Import) -> None:
        scope_id = self._current_scope_id() or self._make_id("<module>")
        for alias in node.names:
            mod_id = f"<import>.{alias.name}"
            if mod_id not in self._defined_names:
                self.vertices.append(Vertex(
                    id=mod_id,
                    status=VertexStatus.ACTIVE,
                    content=f"import {alias.name}",
                ))
                self._defined_names.add(mod_id)
            self.edges.append(Edge(
                source=scope_id,
                target=mod_id,
                edge_type=EdgeType.DEPENDENCY,
            ))

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        scope_id = self._current_scope_id() or self._make_id("<module>")
        module = node.module or ""
        for alias in (node.names or []):
            mod_id = f"<import>.{module}.{alias.name}"
            if mod_id not in self._defined_names:
                self.vertices.append(Vertex(
                    id=mod_id,
                    status=VertexStatus.ACTIVE,
                    content=f"from {module} import {alias.name}",
                ))
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

def ingest_file(filepath: str, graph: Graph) -> Graph:
    """Parse single .py file, inject functions/classes/imports with full source.

    Unlike code_topology.parse_file, each vertex's content contains the full
    source code of the function/class body — not just the signature or docstring.
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    source_lines = source.split("\n")
    module_name = Path(filepath).stem
    tree = ast.parse(source, filename=filepath)

    visitor = _IngestVisitor(module_name, source_lines)

    # Module-level vertex with module docstring
    module_id = f"{module_name}.<module>"
    module_docstring = ast.get_docstring(tree) or module_name
    visitor.vertices.append(Vertex(
        id=module_id,
        status=VertexStatus.ACTIVE,
        content=module_docstring[:400] if module_docstring else module_name,
    ))
    visitor._defined_names.add(module_id)

    visitor.visit(tree)

    # Merge into existing graph
    existing_ids = set(graph.active_vertex_ids())
    for v in visitor.vertices:
        if v.id not in existing_ids:
            graph = graph.add_vertex(v)
            existing_ids.add(v.id)

    existing_edges: set[tuple[str, str, str]] = {
        (e.source, e.target, e.edge_type.value)
        for e in graph.edges
    }
    for e in visitor.edges:
        key = (e.source, e.target, e.edge_type.value)
        if key not in existing_edges:
            if e.source in existing_ids and e.target in existing_ids:
                graph = graph.add_edge(e)
                existing_edges.add(key)

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
