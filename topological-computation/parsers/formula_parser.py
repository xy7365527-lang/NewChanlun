"""Formula parser: LaTeX / sympy expressions → variable vertices + dependency edges.

Extracts variables and their dependency relationships from mathematical formulas.
sympy.parse_latex converts LaTeX to symbolic expressions; we walk the expression
tree to find free symbols and their structural relationships.

Pure deterministic. Zero LLM.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

from engine import Vertex, Edge, EdgeType, VertexStatus


def parse_formula(latex_or_expr: str, source: str = "formula") -> tuple[list[Vertex], list[Edge]]:
    """Parse a mathematical formula into vertices (variables) and edges (dependencies).

    Args:
        latex_or_expr: LaTeX string (e.g. r"\\frac{\\partial L}{\\partial \\theta}")
                       or plain expression (e.g. "y = f(x) + g(z)")
        source: Source label for vertex IDs.

    Returns:
        (vertices, edges) ready for K_active injection.
    """
    vertices: list[Vertex] = []
    edges: list[Edge] = []
    seen_vids: set[str] = set()

    def _add_vertex(name: str, content: str | None = None) -> str:
        vid = f"{source}:formula:{name}"
        if vid not in seen_vids:
            seen_vids.add(vid)
            vertices.append(Vertex(id=vid, status=VertexStatus.ACTIVE, content=content or name))
        return vid

    # Try sympy parse first
    try:
        import sympy
        from sympy.parsing.latex import parse_latex as _parse_latex

        expr = _parse_latex(latex_or_expr)
        symbols = expr.free_symbols

        # Each symbol is a vertex
        sym_vids: dict[str, str] = {}
        for sym in symbols:
            vid = _add_vertex(str(sym), f"variable: {sym}")
            sym_vids[str(sym)] = vid

        # Expression-level vertex (the formula itself)
        expr_vid = _add_vertex(f"expr_{hash(latex_or_expr) % 10**8}", latex_or_expr[:120])

        # All symbols depend on the expression
        for sym_name, sym_vid in sym_vids.items():
            edges.append(Edge(source=expr_vid, target=sym_vid, edge_type=EdgeType.DEPENDENCY))

        # Detect derivatives: if ∂L/∂θ, then L depends on θ
        expr_str = str(expr)
        if "Derivative" in expr_str:
            # sympy Derivative(f, x) means df/dx → f depends on x
            for arg in expr.args if hasattr(expr, 'args') else []:
                if hasattr(arg, 'free_symbols'):
                    inner_syms = list(arg.free_symbols)
                    for i, s1 in enumerate(inner_syms):
                        for s2 in inner_syms[i+1:]:
                            vid1 = sym_vids.get(str(s1))
                            vid2 = sym_vids.get(str(s2))
                            if vid1 and vid2:
                                edges.append(Edge(source=vid1, target=vid2, edge_type=EdgeType.DEPENDENCY))

        return vertices, edges

    except Exception:
        pass

    # Fallback: regex-based extraction for plain expressions
    # Extract variable-like tokens: single letters, Greek names, subscripted vars
    var_pattern = re.compile(r'\b([a-zA-Z_]\w*)\b')
    _SKIP = {"sin", "cos", "tan", "log", "exp", "sqrt", "sum", "prod", "max", "min",
             "def", "for", "in", "if", "else", "return", "import", "from", "class"}

    found_vars: list[str] = []
    for match in var_pattern.finditer(latex_or_expr):
        name = match.group(1)
        if name not in _SKIP and len(name) < 20:
            found_vars.append(name)

    # Deduplicate preserving order
    seen: set[str] = set()
    unique_vars: list[str] = []
    for v in found_vars:
        if v not in seen:
            seen.add(v)
            unique_vars.append(v)

    # Create vertices
    var_vids: dict[str, str] = {}
    for name in unique_vars:
        vid = _add_vertex(name, f"variable: {name}")
        var_vids[name] = vid

    # Detect "y = f(x)" patterns → y depends on x
    assign_pattern = re.compile(r'(\w+)\s*=\s*(.+)')
    m = assign_pattern.match(latex_or_expr)
    if m:
        lhs = m.group(1)
        rhs_vars = [v for v in unique_vars if v != lhs and v in m.group(2)]
        if lhs in var_vids:
            for rv in rhs_vars:
                if rv in var_vids:
                    edges.append(Edge(source=var_vids[lhs], target=var_vids[rv], edge_type=EdgeType.DEPENDENCY))

    return vertices, edges
