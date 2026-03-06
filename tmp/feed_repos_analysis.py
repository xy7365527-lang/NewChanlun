"""Feed repos analysis v2: batch-build approach for performance.

Collects all vertices and edges first, then constructs Graph once.
"""

from __future__ import annotations

import os
import re
import sys
import tempfile
from pathlib import Path

TOPO_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                         "topological-computation")
sys.path.insert(0, TOPO_DIR)

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from code_search import search_by_keyword


# ---------------------------------------------------------------------------
# Regex patterns for TS/JS
# ---------------------------------------------------------------------------
_FUNC_RE = re.compile(r'(?:export\s+)?(?:async\s+)?function\s+(\w+)', re.MULTILINE)
_CLASS_RE = re.compile(r'(?:export\s+)?(?:abstract\s+)?class\s+(\w+)', re.MULTILINE)
_CONST_FUNC_RE = re.compile(r'(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(', re.MULTILINE)
_INTERFACE_RE = re.compile(r'(?:export\s+)?interface\s+(\w+)', re.MULTILINE)
_TYPE_RE = re.compile(r'(?:export\s+)?type\s+(\w+)\s*=', re.MULTILINE)
_IMPORT_RE = re.compile(r'import\s+.*?from\s+[\'"]([^\'"]+)[\'"]', re.MULTILINE)


def _should_skip(path_str: str) -> bool:
    p = path_str.replace("\\", "/")
    return any(x in p for x in ["node_modules", "/dist/", "/build/", "/.next/",
                                  "/.git/", "__pycache__"])


def batch_ingest_ts(dirpath: str, max_files: int = 2000):
    """Collect all vertices and edges from TS/JS files, return (verts_dict, edges_list)."""
    verts: dict[str, Vertex] = {}
    edges: list[Edge] = []
    edge_set: set[tuple[str, str]] = set()

    files = []
    for ext in ("*.ts", "*.js", "*.mjs"):
        files.extend(Path(dirpath).rglob(ext))
    files = [f for f in files if not _should_skip(str(f))]
    files = sorted(files)[:max_files]

    for fpath in files:
        try:
            source = fpath.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        module_name = fpath.stem
        module_id = f"{module_name}.<module>"
        if module_id not in verts:
            verts[module_id] = Vertex(id=module_id, status=VertexStatus.ACTIVE,
                                      content=module_name)

        for pattern in (_FUNC_RE, _CLASS_RE, _CONST_FUNC_RE, _INTERFACE_RE, _TYPE_RE):
            for match in pattern.finditer(source):
                name = match.group(1)
                vid = f"{module_name}.{name}"
                if vid not in verts:
                    verts[vid] = Vertex(id=vid, status=VertexStatus.ACTIVE, content=name)
                    ek = (module_id, vid)
                    if ek not in edge_set:
                        edges.append(Edge(source=module_id, target=vid,
                                          edge_type=EdgeType.DEPENDENCY))
                        edge_set.add(ek)

        for match in _IMPORT_RE.finditer(source):
            import_path = match.group(1)
            import_module = Path(import_path).stem
            if import_module.startswith("."):
                import_module = import_module.lstrip(".")
            if not import_module or import_module == module_name:
                continue
            target_id = f"{import_module}.<module>"
            if target_id not in verts:
                verts[target_id] = Vertex(id=target_id, status=VertexStatus.ACTIVE,
                                           content=import_module)
            ek = (module_id, target_id)
            if ek not in edge_set:
                edges.append(Edge(source=module_id, target=target_id,
                                  edge_type=EdgeType.DEPENDENCY))
                edge_set.add(ek)

    return verts, edges, len(files)


def batch_ingest_md(dirpath: str, max_files: int = 300):
    """Collect vertices/edges from markdown headings."""
    verts: dict[str, Vertex] = {}
    edges: list[Edge] = []

    files = sorted(Path(dirpath).rglob("*.md"))
    files = [f for f in files if not _should_skip(str(f))][:max_files]

    for md_file in files:
        try:
            content = md_file.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue

        module_name = md_file.stem
        module_id = f"doc:{module_name}"
        if module_id not in verts:
            preview = content[:200].replace("\n", " ")
            verts[module_id] = Vertex(id=module_id, status=VertexStatus.ACTIVE,
                                       content=preview)

        headings = re.findall(r'^(#{1,3})\s+(.+)$', content, re.MULTILINE)
        for _, heading_text in headings[:20]:
            heading_id = f"doc:{module_name}.{heading_text[:50].strip()}"
            if heading_id not in verts:
                verts[heading_id] = Vertex(id=heading_id, status=VertexStatus.ACTIVE,
                                            content=heading_text.strip()[:100])
                edges.append(Edge(source=module_id, target=heading_id,
                                  edge_type=EdgeType.DEPENDENCY))

    return verts, edges, len(files)


def batch_ingest_py(dirpath: str, max_files: int = 50):
    """Use AST-based ingestion for Python files, batch-collect."""
    import ast

    verts: dict[str, Vertex] = {}
    edges: list[Edge] = []
    edge_set: set[tuple[str, str]] = set()

    files = sorted(Path(dirpath).rglob("*.py"))
    files = [f for f in files if not _should_skip(str(f))][:max_files]

    for fpath in files:
        try:
            source = fpath.read_text(encoding="utf-8", errors="replace")
            tree = ast.parse(source)
        except (SyntaxError, OSError):
            continue

        module_name = fpath.stem
        module_id = f"{module_name}.<module>"
        if module_id not in verts:
            docstring = ast.get_docstring(tree) or module_name
            verts[module_id] = Vertex(id=module_id, status=VertexStatus.ACTIVE,
                                       content=docstring[:200])

        for node in ast.walk(tree):
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                vid = f"{module_name}.{node.name}"
                if vid not in verts:
                    verts[vid] = Vertex(id=vid, status=VertexStatus.ACTIVE,
                                        content=node.name)
                    ek = (module_id, vid)
                    if ek not in edge_set:
                        edges.append(Edge(source=module_id, target=vid,
                                          edge_type=EdgeType.DEPENDENCY))
                        edge_set.add(ek)
            elif isinstance(node, ast.ClassDef):
                vid = f"{module_name}.{node.name}"
                if vid not in verts:
                    verts[vid] = Vertex(id=vid, status=VertexStatus.ACTIVE,
                                        content=node.name)
                    ek = (module_id, vid)
                    if ek not in edge_set:
                        edges.append(Edge(source=module_id, target=vid,
                                          edge_type=EdgeType.DEPENDENCY))
                        edge_set.add(ek)

    return verts, edges, len(files)


def build_graph(verts: dict[str, Vertex], edges: list[Edge]) -> Graph:
    """Build Graph from pre-collected vertices and edges."""
    # Filter edges: both endpoints must exist
    valid_edges = [e for e in edges if e.source in verts and e.target in verts]
    return Graph(vertices=verts, edges=valid_edges)


def find_hubs(graph: Graph, top_n: int = 20) -> list[dict]:
    active = set(graph.active_vertex_ids())
    degree: dict[str, int] = {v: 0 for v in active}
    for e in graph.edges:
        if e.source in active and e.target in active:
            degree[e.source] = degree.get(e.source, 0) + 1
            degree[e.target] = degree.get(e.target, 0) + 1
    sorted_by_degree = sorted(degree.items(), key=lambda x: x[1], reverse=True)
    results = []
    for vid, deg in sorted_by_degree[:top_n]:
        v = graph.vertex(vid)
        results.append({"id": vid, "degree": deg,
                        "content": (v.content or "")[:80] if v else ""})
    return results


def analyze_repo(name: str, dirpath: str, lines: list[str]) -> Graph:
    lines.append(f"\n{'=' * 70}")
    lines.append(f"REPO: {name}")
    lines.append(f"Path: {dirpath}")
    lines.append(f"{'=' * 70}\n")

    all_verts: dict[str, Vertex] = {}
    all_edges: list[Edge] = []

    # Python
    py_verts, py_edges, py_count = batch_ingest_py(dirpath)
    all_verts.update(py_verts)
    all_edges.extend(py_edges)
    lines.append(f"Python files:     {py_count} -> {len(py_verts)} vertices, {len(py_edges)} edges")

    # TS/JS
    ts_verts, ts_edges, ts_count = batch_ingest_ts(dirpath)
    all_verts.update(ts_verts)
    all_edges.extend(ts_edges)
    lines.append(f"TS/JS files:      {ts_count} -> {len(ts_verts)} vertices, {len(ts_edges)} edges")

    # Markdown
    md_verts, md_edges, md_count = batch_ingest_md(dirpath)
    all_verts.update(md_verts)
    all_edges.extend(md_edges)
    lines.append(f"Markdown files:   {md_count} -> {len(md_verts)} vertices, {len(md_edges)} edges")

    lines.append("")

    # Build graph
    graph = build_graph(all_verts, all_edges)

    n_v = len(graph.active_vertex_ids())
    n_e_directed = len(graph.active_edges())
    n_e_undirected = len(graph.undirected_active_edges())
    beta_1 = compute_beta_1(graph)

    lines.append("--- Simplicial Complex Statistics ---")
    lines.append(f"  Vertices (V):           {n_v}")
    lines.append(f"  Directed edges:         {n_e_directed}")
    lines.append(f"  Undirected edges (E):   {n_e_undirected}")
    lines.append(f"  beta_1 (1-cycles):      {beta_1}")
    lines.append(f"  Self-loops:             {len(graph.self_loops())}")
    lines.append("")

    # Hubs
    lines.append("--- Top 15 Hub Vertices (highest degree) ---")
    hubs = find_hubs(graph, top_n=15)
    for i, h in enumerate(hubs, 1):
        lines.append(f"  {i:2d}. [{h['degree']:3d}] {h['id'][:60]}  ({h['content'][:40]})")
    lines.append("")

    # Fold candidates
    lines.append("--- Fold Candidates (high-degree non-module vertices) ---")
    fold_candidates = [h for h in hubs if ".<module>" not in h["id"] and "doc:" not in h["id"]]
    for i, h in enumerate(fold_candidates[:10], 1):
        lines.append(f"  {i:2d}. [{h['degree']:3d}] {h['id'][:60]}")
    lines.append("")

    # Keyword search
    lines.append("--- Code Search ---")
    for keyword in ["fold", "negate", "transform", "merge", "hook", "agent", "skill"]:
        results = search_by_keyword(graph, keyword)
        if results:
            lines.append(f"  '{keyword}': {len(results)} hits")
            for r in results[:3]:
                lines.append(f"    - {r['id'][:50]} (neighbors={r['neighbor_count']})")
    lines.append("")

    return graph


def main():
    lines: list[str] = []
    lines.append("=" * 70)
    lines.append("FEED REPOS ANALYSIS — Topological Code Ingestion v2")
    lines.append("=" * 70)
    lines.append("")

    tmp_base = tempfile.gettempdir()

    # everything-claude-code
    ecc_path = os.path.join(tmp_base, "everything-claude-code")
    if os.path.isdir(ecc_path):
        graph_ecc = analyze_repo("everything-claude-code", ecc_path, lines)
    else:
        lines.append(f"[SKIP] {ecc_path} not found")
        graph_ecc = Graph()

    # OpenClaw (src/ only to manage size)
    oc_path = os.path.join(tmp_base, "openclaw")
    if os.path.isdir(oc_path):
        oc_src = os.path.join(oc_path, "src")
        if os.path.isdir(oc_src):
            graph_oc = analyze_repo("OpenClaw (src/)", oc_src, lines)
        else:
            graph_oc = analyze_repo("OpenClaw", oc_path, lines)
    else:
        lines.append(f"[SKIP] {oc_path} not found")
        graph_oc = Graph()

    # Summary
    lines.append("\n" + "=" * 70)
    lines.append("SUMMARY COMPARISON")
    lines.append("=" * 70)
    lines.append("")

    ecc_v = len(graph_ecc.active_vertex_ids())
    ecc_e = len(graph_ecc.undirected_active_edges())
    ecc_b = compute_beta_1(graph_ecc)

    oc_v = len(graph_oc.active_vertex_ids())
    oc_e = len(graph_oc.undirected_active_edges())
    oc_b = compute_beta_1(graph_oc)

    lines.append(f"{'Metric':<30} {'everything-claude-code':>25} {'OpenClaw (src/)':>25}")
    lines.append("-" * 80)
    lines.append(f"{'Vertices':.<30} {ecc_v:>25} {oc_v:>25}")
    lines.append(f"{'Undirected Edges':.<30} {ecc_e:>25} {oc_e:>25}")
    lines.append(f"{'beta_1':.<30} {ecc_b:>25} {oc_b:>25}")
    lines.append(f"{'Edge density (E/V)':.<30} {ecc_e/max(ecc_v,1):>25.2f} {oc_e/max(oc_v,1):>25.2f}")
    lines.append(f"{'Cycle density (beta_1/V)':.<30} {ecc_b/max(ecc_v,1):>25.4f} {oc_b/max(oc_v,1):>25.4f}")
    lines.append("")

    report = "\n".join(lines)
    print(report)

    out_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "tmp",
    )
    os.makedirs(out_dir, exist_ok=True)
    out_path = os.path.join(out_dir, "feed_repos_test.txt")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\nSaved to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
