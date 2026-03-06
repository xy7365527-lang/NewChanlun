"""Persistence test: write, crash-recover, verify consistency.

Outputs results to tmp/persistence_test.txt.
"""

from __future__ import annotations

import os
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus, compute_beta_1
from persistence import PersistentKFull
from daemon import TopologicalDaemon


def build_small_complex() -> Graph:
    """Build a small test complex: 8 vertices with dependency/negation edges."""
    g = Graph()
    names = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"]
    for i, name in enumerate(names):
        g = g.add_vertex(Vertex(id=name, status=VertexStatus.ACTIVE, content=name, created_at=i))

    edges = [
        ("alpha", "beta", EdgeType.DEPENDENCY),
        ("beta", "gamma", EdgeType.DEPENDENCY),
        ("gamma", "delta", EdgeType.DEPENDENCY),
        ("delta", "alpha", EdgeType.DEPENDENCY),  # cycle
        ("epsilon", "zeta", EdgeType.DEPENDENCY),
        ("zeta", "eta", EdgeType.DEPENDENCY),
        ("eta", "epsilon", EdgeType.DEPENDENCY),   # second cycle
        ("theta", "alpha", EdgeType.REFERENCE),
        ("theta", "epsilon", EdgeType.REFERENCE),
        ("beta", "zeta", EdgeType.NEGATION),
    ]
    for src, tgt, etype in edges:
        g = g.add_edge(Edge(source=src, target=tgt, edge_type=etype, created_at=0))

    return g


def test_1_write_and_count(tmpdir: Path, report: list[str]) -> None:
    """Test 1: Run 100 steps, verify JSONL file size and line count."""
    report.append("=" * 60)
    report.append("TEST 1: Write 100 steps, check JSONL")
    report.append("=" * 60)

    jsonl_path = tmpdir / "test1.jsonl"
    graph = build_small_complex()

    daemon = TopologicalDaemon(
        graph=graph, settlement_threshold=15, seed=42,
        persist_path=jsonl_path,
    )

    # Write initial graph
    for vid, v in graph.vertices.items():
        daemon._persist.append_vertex(v)
    for e in graph.edges:
        daemon._persist.append_edge(e)

    t0 = time.monotonic()
    daemon.run(max_steps=100)
    elapsed = time.monotonic() - t0
    daemon.close()

    # Count lines
    with open(jsonl_path, encoding="utf-8") as f:
        lines = [l for l in f if l.strip()]
    line_count = len(lines)
    file_size = jsonl_path.stat().st_size

    status = daemon.status()

    report.append(f"Steps run: 100")
    report.append(f"Time: {elapsed:.3f}s")
    report.append(f"JSONL lines: {line_count}")
    report.append(f"JSONL size: {file_size} bytes")
    report.append(f"Final beta_1: {status['beta_1']}")
    report.append(f"Events: {status['total_events']}")
    report.append(f"Settled cycles: {status['settled_cycles']}")

    # Verify: at least initial graph records (vertices + edges)
    initial_records = len(graph.vertices) + len(graph.edges)
    assert line_count >= initial_records, (
        f"Expected >= {initial_records} lines (initial graph), got {line_count}"
    )
    report.append(f"PASS: line_count ({line_count}) >= initial records ({initial_records})")
    report.append("")


def test_2_crash_recovery(tmpdir: Path, report: list[str]) -> None:
    """Test 2: Run 50 steps, 'crash', recover, verify Graph consistency."""
    report.append("=" * 60)
    report.append("TEST 2: Crash at 50 steps, recover, verify consistency")
    report.append("=" * 60)

    jsonl_path = tmpdir / "test2.jsonl"
    graph = build_small_complex()

    # Phase 1: run 50 steps
    daemon1 = TopologicalDaemon(
        graph=graph, settlement_threshold=15, seed=42,
        persist_path=jsonl_path,
    )

    # Write initial graph
    for vid, v in graph.vertices.items():
        daemon1._persist.append_vertex(v)
    for e in graph.edges:
        daemon1._persist.append_edge(e)

    daemon1.run(max_steps=50)

    # Capture state before "crash"
    state_before_crash = daemon1.status()
    beta_1_before = compute_beta_1(daemon1.k_full)
    verts_before = set(daemon1.k_full.vertices.keys())
    edges_before = len(daemon1.k_full.edges)

    report.append(f"Before crash (50 steps):")
    report.append(f"  beta_1: {beta_1_before}")
    report.append(f"  vertices: {len(verts_before)}")
    report.append(f"  edges (full): {edges_before}")
    report.append(f"  events: {state_before_crash['total_events']}")

    # "Crash" — close without cleanup
    daemon1.close()

    # Phase 2: recover from JSONL
    recovered_graph, operations = PersistentKFull.load(jsonl_path)
    beta_1_recovered = compute_beta_1(recovered_graph)
    verts_recovered = set(recovered_graph.vertices.keys())
    edges_recovered = len(recovered_graph.edges)

    report.append(f"After recovery:")
    report.append(f"  beta_1: {beta_1_recovered}")
    report.append(f"  vertices: {len(verts_recovered)}")
    report.append(f"  edges (full): {edges_recovered}")
    report.append(f"  operations log: {len(operations)} records")

    # Verify: recovered graph contains at least the initial vertices
    initial_vids = set(graph.vertices.keys())
    assert initial_vids.issubset(verts_recovered), (
        f"Recovery lost initial vertices: {initial_vids - verts_recovered}"
    )
    report.append(f"PASS: all initial vertices recovered")

    # Verify: recovered graph has at least the initial edges
    assert edges_recovered >= len(graph.edges), (
        f"Recovery lost edges: {edges_recovered} < {len(graph.edges)}"
    )
    report.append(f"PASS: edge count ({edges_recovered}) >= initial ({len(graph.edges)})")

    # Verify: beta_1 is non-negative (sanity)
    assert beta_1_recovered >= 0, f"Negative beta_1: {beta_1_recovered}"
    report.append(f"PASS: beta_1 ({beta_1_recovered}) >= 0")
    report.append("")


def test_3_daemon_persist_flag(tmpdir: Path, report: list[str]) -> None:
    """Test 3: Verify --persist path integration in TopologicalDaemon."""
    report.append("=" * 60)
    report.append("TEST 3: Daemon --persist integration")
    report.append("=" * 60)

    jsonl_path = tmpdir / "test3.jsonl"
    graph = build_small_complex()

    # First run
    daemon1 = TopologicalDaemon(
        graph=graph, settlement_threshold=15, seed=42,
        persist_path=jsonl_path,
    )
    for vid, v in graph.vertices.items():
        daemon1._persist.append_vertex(v)
    for e in graph.edges:
        daemon1._persist.append_edge(e)

    daemon1.run(max_steps=30)
    daemon1.close()

    # Second run: should recover from JSONL
    daemon2 = TopologicalDaemon(
        graph=None, settlement_threshold=15, seed=42,
        persist_path=jsonl_path,
    )
    assert daemon2.k_active.active_vertex_ids(), "Recovery should produce non-empty graph"
    report.append(f"PASS: daemon2 recovered non-empty graph")

    beta_1 = compute_beta_1(daemon2.k_active)
    report.append(f"Recovered beta_1: {beta_1}")
    report.append(f"Recovered vertices: {len(daemon2.k_active.active_vertex_ids())}")

    daemon2.close()
    report.append(f"PASS: daemon2 closed cleanly")
    report.append("")


def main():
    tmpdir = Path(tempfile.mkdtemp(prefix="topo_persist_test_"))
    report: list[str] = [
        "TOPOLOGICAL COMPUTATION — PERSISTENCE TEST REPORT",
        f"Date: {time.strftime('%Y-%m-%d %H:%M:%S')}",
        f"Temp dir: {tmpdir}",
        "",
    ]

    try:
        test_1_write_and_count(tmpdir, report)
        test_2_crash_recovery(tmpdir, report)
        test_3_daemon_persist_flag(tmpdir, report)
        report.append("ALL TESTS PASSED")
    except Exception as e:
        report.append(f"FAILED: {e}")
        import traceback
        report.append(traceback.format_exc())

    report_text = "\n".join(report)
    print(report_text)

    # Write to tmp/
    project_root = Path(__file__).resolve().parent.parent
    out_path = project_root / "tmp" / "persistence_test.txt"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(report_text)
    print(f"\nReport saved to {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
