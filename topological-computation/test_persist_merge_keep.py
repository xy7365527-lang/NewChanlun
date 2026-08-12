"""Regression: persist merge keep must be the real absorber, not position.

When crystallization absorbs anti_*/syn_* into a settled neighbor, or Morse
_contract folds a new vertex into a neighbor, keep != position. Writing
append_merge(position, remove) corrupts JSONL replay topology.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

from engine import (
    Edge,
    EdgeType,
    Graph,
    SettledCycle,
    SettlementTracker,
    Vertex,
    VertexStatus,
    fold,
)
from persistence import PersistentKFull


def _edge_keys(graph: Graph) -> list[tuple[str, str, str]]:
    return sorted(
        (e.source, e.target, e.edge_type.value) for e in graph.active_edges()
    )


def test_wrong_keep_position_corrupts_replay_topology():
    """Document the failure mode: keep=position redirects external edges wrongly."""
    g = Graph()
    for vid in ["P", "S", "anti_X", "Q", "R"]:
        g = g.add_vertex(Vertex(vid, content=vid))
    g = g.add_edge(Edge("P", "anti_X", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("anti_X", "S", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("Q", "anti_X", EdgeType.REFERENCE, 0))
    g = g.add_edge(Edge("R", "S", EdgeType.REFERENCE, 0))

    live = fold(g, ["S", "anti_X"], step=1, settlement=SettlementTracker()).graph
    assert "S" in live.neighbors("Q")

    td = Path(tempfile.mkdtemp())
    path = td / "wrong.jsonl"
    with PersistentKFull(path) as p:
        for vid in ["P", "S", "anti_X", "Q", "R"]:
            p.append_vertex(Vertex(vid, content=vid))
        p.append_edge(Edge("P", "anti_X", EdgeType.DEPENDENCY, 0))
        p.append_edge(Edge("anti_X", "S", EdgeType.DEPENDENCY, 0))
        p.append_edge(Edge("Q", "anti_X", EdgeType.REFERENCE, 0))
        p.append_edge(Edge("R", "S", EdgeType.REFERENCE, 0))
        p.append_merge("P", "anti_X", 1)  # buggy: position as keep

    wrong, _ = PersistentKFull.load(path)
    assert "P" in wrong.neighbors("Q")
    assert "S" not in wrong.neighbors("Q")


def test_correct_keep_absorber_preserves_replay_connectivity():
    td = Path(tempfile.mkdtemp())
    path = td / "right.jsonl"
    with PersistentKFull(path) as p:
        for vid in ["P", "S", "anti_X", "Q", "R"]:
            p.append_vertex(Vertex(vid, content=vid))
        p.append_edge(Edge("P", "anti_X", EdgeType.DEPENDENCY, 0))
        p.append_edge(Edge("anti_X", "S", EdgeType.DEPENDENCY, 0))
        p.append_edge(Edge("Q", "anti_X", EdgeType.REFERENCE, 0))
        p.append_edge(Edge("R", "S", EdgeType.REFERENCE, 0))
        p.append_merge("S", "anti_X", 1)

    right, _ = PersistentKFull.load(path)
    assert right.vertex("anti_X").status == VertexStatus.FOLDED
    assert "S" in right.neighbors("Q")
    assert "P" not in right.neighbors("Q")


def test_daemon_persist_uses_step_merges_not_position():
    """Daemon must record absorber from _step_merges when absorbing process verts."""
    from daemon import TopologicalDaemon

    g = Graph()
    for vid in ["P", "S", "anti_X", "Q"]:
        g = g.add_vertex(Vertex(vid, content=vid))
    g = g.add_edge(Edge("P", "anti_X", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("anti_X", "S", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("P", "S", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("Q", "anti_X", EdgeType.REFERENCE, 0))

    td = Path(tempfile.mkdtemp())
    persist_path = td / "k.jsonl"

    # Avoid full TopologicalDaemon ctor (heavy ingest); stub the persist path.
    d = object.__new__(TopologicalDaemon)
    d.k_active = g.copy()
    d.k_full = g.copy()
    d.total_steps = 7
    d.settlement = SettlementTracker()
    # Only S is settled (plus helper T). P stays unsettled so absorb cannot
    # pick position as keep.
    d.k_active = d.k_active.add_vertex(Vertex("T", content="T"))
    d.k_full = d.k_full.add_vertex(Vertex("T", content="T"))
    d.k_active = d.k_active.add_edge(Edge("S", "T", EdgeType.DEPENDENCY, 0))
    d.k_full = d.k_full.add_edge(Edge("S", "T", EdgeType.DEPENDENCY, 0))
    d.settlement._settled.append(
        SettledCycle(
            edges=frozenset({("S", "T"), ("T", "S")}),
            settled_at_step=1,
            residue=(),
            status="active",
        )
    )
    d.engine = type("E", (), {"_step_merges": [], "position": "P"})()
    d._persist = PersistentKFull(persist_path).open()

    # Seed JSONL with pre-absorb graph so merge replay has vertices/edges.
    for vid in ["P", "S", "anti_X", "Q"]:
        d._persist.append_vertex(Vertex(vid, content=vid))
    d._persist.append_edge(Edge("P", "anti_X", EdgeType.DEPENDENCY, 0))
    d._persist.append_edge(Edge("anti_X", "S", EdgeType.DEPENDENCY, 0))
    d._persist.append_edge(Edge("P", "S", EdgeType.DEPENDENCY, 0))
    d._persist.append_edge(Edge("Q", "anti_X", EdgeType.REFERENCE, 0))

    absorbed = d._absorb_process_vertices({"P", "S", "anti_X", "Q"})
    assert absorbed == 1
    assert ("S", "anti_X") in d.engine._step_merges

    # Simulate the persist merge loop from _step
    pre_active_statuses = {
        "P": VertexStatus.ACTIVE,
        "S": VertexStatus.ACTIVE,
        "anti_X": VertexStatus.ACTIVE,
        "Q": VertexStatus.ACTIVE,
    }
    merge_by_remove = {remove: keep for keep, remove in d.engine._step_merges}
    for vid, v in d.k_active.vertices.items():
        old_status = pre_active_statuses.get(vid)
        if old_status is not None and old_status != v.status and v.status == VertexStatus.FOLDED:
            keep = merge_by_remove[vid]
            assert keep == "S"
            assert keep != "P"
            d._persist.append_merge(keep, vid, d.total_steps)

    d._persist.close()
    recovered, _ = PersistentKFull.load(persist_path)
    assert "S" in recovered.neighbors("Q")
    assert "P" not in recovered.neighbors("Q")
