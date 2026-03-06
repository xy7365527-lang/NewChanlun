"""Parse Python source files into the topological computation engine.

Code IS topology: functions are vertices, calls are edges, imports are dependency edges.

Pure Python, only stdlib (ast module). No external dependencies.
"""

from __future__ import annotations

import ast
import os
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus


# ---------------------------------------------------------------------------
# AST visitor: collect functions, classes, calls, imports
# ---------------------------------------------------------------------------

class _CodeVisitor(ast.NodeVisitor):
    """Walk a Python AST and collect vertices (definitions) and edges (references)."""

    def __init__(self, module_name: str) -> None:
        self.module_name = module_name
        self.vertices: list[Vertex] = []
        self.edges: list[Edge] = []
        self._defined_names: set[str] = set()
        self._scope_stack: list[str] = []  # current enclosing function/class

    def _make_id(self, name: str) -> str:
        return f"{self.module_name}.{name}"

    def _current_scope_id(self) -> str | None:
        if self._scope_stack:
            return self._make_id(self._scope_stack[-1])
        return None

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
        content = ast.get_docstring(node) or _first_line_summary(node)
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=content[:200] if content else qualified,
        ))
        self._defined_names.add(vid)

        # If nested inside a class/function, add containment edge
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
        content = ast.get_docstring(node) or node.name
        self.vertices.append(Vertex(
            id=vid,
            status=VertexStatus.ACTIVE,
            content=content[:200] if content else qualified,
        ))
        self._defined_names.add(vid)

        # Inheritance edges
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


def _first_line_summary(node: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
    """Return function signature as summary."""
    args = []
    for arg in node.args.args:
        args.append(arg.arg)
    return f"{node.name}({', '.join(args)})"


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def parse_file(filepath: str) -> Graph:
    """Parse a Python file into a Graph.

    Vertices:
    - Each function/method definition -> vertex (id=module.funcname, content=docstring or first line)
    - Each class definition -> vertex
    - Each module-level import -> vertex (the imported module)

    Edges:
    - Function A calls function B -> A --[dependency]--> B
    - Module imports module -> importer --[dependency]--> imported
    - Class inherits from base -> child --[dependency]--> base
    """
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()

    module_name = Path(filepath).stem
    tree = ast.parse(source, filename=filepath)

    visitor = _CodeVisitor(module_name)

    # Add module-level vertex
    module_id = f"{module_name}.<module>"
    module_content = ast.get_docstring(tree) or module_name
    visitor.vertices.append(Vertex(
        id=module_id,
        status=VertexStatus.ACTIVE,
        content=module_content[:200] if module_content else module_name,
    ))
    visitor._defined_names.add(module_id)

    visitor.visit(tree)

    # Build graph, resolving edges
    graph = Graph()
    all_ids: set[str] = set()

    for v in visitor.vertices:
        if v.id not in all_ids:
            graph = graph.add_vertex(v)
            all_ids.add(v.id)

    # Only add edges where both endpoints exist
    for e in visitor.edges:
        if e.source in all_ids and e.target in all_ids:
            graph = graph.add_edge(e)

    return graph


def parse_directory(dirpath: str, pattern: str = "*.py") -> Graph:
    """Parse all Python files in a directory into a single Graph."""
    graph = Graph()
    existing_ids: set[str] = set()
    existing_edges: set[tuple[str, str, str]] = set()

    py_files = sorted(Path(dirpath).glob(pattern))
    for py_file in py_files:
        sub = parse_file(str(py_file))

        for vid in sub.active_vertex_ids():
            if vid not in existing_ids:
                v = sub.vertex(vid)
                graph = graph.add_vertex(v)
                existing_ids.add(vid)

        for e in sub.edges:
            key = (e.source, e.target, e.edge_type.value)
            if key not in existing_edges:
                if e.source in existing_ids and e.target in existing_ids:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    return graph


def code_to_complex(filepaths: list[str]) -> Graph:
    """Parse multiple files into one complex."""
    graph = Graph()
    existing_ids: set[str] = set()
    existing_edges: set[tuple[str, str, str]] = set()

    for filepath in filepaths:
        sub = parse_file(filepath)

        for vid in sub.active_vertex_ids():
            if vid not in existing_ids:
                v = sub.vertex(vid)
                graph = graph.add_vertex(v)
                existing_ids.add(vid)

        for e in sub.edges:
            key = (e.source, e.target, e.edge_type.value)
            if key not in existing_edges:
                if e.source in existing_ids and e.target in existing_ids:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    return graph
