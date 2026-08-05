"""Regression tests for daemon persist recovery correctness.

Covers:
1. JSONL must not be wiped after successful snapshot/JSONL recovery
2. type=articulation events must restore ARTICULATED edges on replay
3. SharedLayer settlement sync must not NameError when no new settlements
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from unittest.mock import MagicMock

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from block_topology_persistence import BlockTopologyWriter
from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex, VertexStatus
from persistence import PersistentKFull


def _tiny_graph() -> Graph:
    g = Graph()
    g = g.add_vertex(Vertex("a", VertexStatus.ACTIVE, "a", 0))
    g = g.add_vertex(Vertex("b", VertexStatus.ACTIVE, "b", 1))
    g = g.add_vertex(Vertex("c", VertexStatus.ACTIVE, "c", 2))
    g = g.add_edge(Edge("a", "b", EdgeType.DEPENDENCY, 0))
    g = g.add_edge(Edge("b", "c", EdgeType.DEPENDENCY, 1))
    g = g.add_edge(Edge("c", "a", EdgeType.DEPENDENCY, 2))
    return g


def _write_jsonl_graph(path: Path, graph: Graph) -> None:
    writer = BlockTopologyWriter(jsonl_backup_path=path)
    writer.open()
    for v in graph.vertices.values():
        writer.append_vertex(v)
    for e in graph.edges:
        writer.append_edge(e)
    # Runtime-evolved vertex not present in a fresh seed of equal size
    writer.append_vertex(Vertex("runtime_x", VertexStatus.ACTIVE, "runtime_x", 99))
    writer.append_articulation(
        source_vid="a",
        target_vid="c",
        score=0.87,
        step=12,
        imbalance_type="A_dense_B_sparse",
        reason="test articulation",
        timestamp="2026-08-05T00:00:00Z",
    )
    writer.close()


class _EmptySNet:
    """Minimal stand-in so daemon engine init skips heavy dictionary bootstrap."""

    def __init__(self) -> None:
        self._edges: list = []
        self._signifiers: dict = {}
        self.hyperedges: list = []
        self.edge_count = 0
        self.hyperedge_count = 0

    def add_signifier(self, *args, **kwargs):
        return None


@pytest.fixture()
def isolated_bt(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Point DAEMON_BT_BASE at an empty temp dir so real host state cannot leak in."""
    bt = tmp_path / "block-topology"
    bt.mkdir()
    monkeypatch.setattr("daemon.DAEMON_BT_BASE", bt)
    monkeypatch.setattr("block_topology_persistence.DAEMON_BT_BASE", bt)

    def _fake_bootstrap(self):
        self.snet = _EmptySNet()
        self.snet_activation = None

    monkeypatch.setattr(TopologicalDaemon, "_bootstrap_snet", _fake_bootstrap)
    monkeypatch.setattr(TopologicalDaemon, "_register_snet_param_nodes", lambda self: None)
    monkeypatch.setattr(TopologicalDaemon, "_setup_snet_activation", lambda self: None)
    return bt


def test_jsonl_recovery_sets_flag_and_keeps_runtime_vertices(tmp_path: Path, isolated_bt):
    """Recovering from JSONL must mark recovery and keep runtime vertices."""
    jsonl_path = tmp_path / "k.jsonl"
    evolved = _tiny_graph()
    _write_jsonl_graph(jsonl_path, evolved)

    seed = _tiny_graph()  # same size as base, without runtime_x
    daemon = TopologicalDaemon(
        graph=seed,
        settlement_threshold=15,
        seed=42,
        persist_path=jsonl_path,
    )
    try:
        assert daemon._recovered_from_persist is True
        assert daemon.k_full.vertex("runtime_x") is not None
        # Simulate main()'s guard: recovered daemons must not truncate.
        assert jsonl_path.stat().st_size > 0
        before = jsonl_path.read_text(encoding="utf-8")
        if not daemon._recovered_from_persist:
            daemon._persist.close()
            jsonl_path.write_text("", encoding="utf-8")
            daemon._persist.open()
        after = jsonl_path.read_text(encoding="utf-8")
        assert after == before
        assert "runtime_x" in after
        assert '"type": "articulation"' in after
    finally:
        if daemon._persist:
            daemon._persist.close()


def test_main_seed_write_skipped_when_recovered(tmp_path: Path, isolated_bt):
    """main()-equivalent seed write must not run after recovery."""
    jsonl_path = tmp_path / "k.jsonl"
    _write_jsonl_graph(jsonl_path, _tiny_graph())
    before_lines = [
        ln for ln in jsonl_path.read_text(encoding="utf-8").splitlines() if ln.strip()
    ]

    seed = _tiny_graph()
    daemon = TopologicalDaemon(
        graph=seed, settlement_threshold=15, seed=42, persist_path=jsonl_path,
    )
    try:
        # Exact guard used in daemon.main()
        if (
            daemon._persist
            and seed is not None
            and not daemon._recovered_from_persist
        ):
            daemon._persist.close()
            jsonl_path.write_text("", encoding="utf-8")
            daemon._persist.open()
            for v in seed.vertices.values():
                daemon._persist.append_vertex(v)
            for e in seed.edges:
                daemon._persist.append_edge(e)

        after_lines = [
            ln for ln in jsonl_path.read_text(encoding="utf-8").splitlines() if ln.strip()
        ]
        assert len(after_lines) == len(before_lines)
        assert any(
            json.loads(ln).get("id") == "runtime_x"
            for ln in after_lines
            if json.loads(ln).get("type") == "vertex"
        )
    finally:
        if daemon._persist:
            daemon._persist.close()


def test_articulation_replayed_on_full_jsonl_load(tmp_path: Path):
    jsonl_path = tmp_path / "art.jsonl"
    _write_jsonl_graph(jsonl_path, _tiny_graph())

    graph, _ops = PersistentKFull.load(jsonl_path)
    assert graph.has_edge_key("a", "c", EdgeType.ARTICULATED)


def test_articulation_replayed_on_incremental_after_snapshot(tmp_path: Path):
    jsonl_path = tmp_path / "inc.jsonl"
    snapshot_path = PersistentKFull.snapshot_path_for(jsonl_path)

    base = _tiny_graph()
    PersistentKFull.dump_snapshot(base, snapshot_path)

    # Incremental-only articulation after snapshot (crash before close)
    writer = BlockTopologyWriter(jsonl_backup_path=jsonl_path)
    writer.open()
    writer.append_articulation(
        source_vid="a",
        target_vid="b",
        score=0.5,
        step=3,
        imbalance_type="B_dense_A_sparse",
        reason="incremental articulation",
        timestamp="2026-08-05T00:00:01Z",
    )
    writer.close()

    graph, _ops = PersistentKFull.load_snapshot_then_incremental(
        snapshot_path, jsonl_path,
    )
    assert graph.has_edge_key("a", "b", EdgeType.ARTICULATED)


def test_shared_layer_step_without_new_settlement(isolated_bt):
    """Enabling SharedLayer must not crash when a step adds no settlements."""
    graph = _tiny_graph()
    daemon = TopologicalDaemon(graph=graph, settlement_threshold=15, seed=42)
    assert daemon.engine is not None

    shared = MagicMock()
    shared.write_block = MagicMock(return_value=True)
    daemon._shared_layer = shared
    daemon._instance_id = "test-instance"
    # Isolate the settlement-delta NameError from downstream IPFS sync details.
    daemon._write_traversal_position = MagicMock()
    daemon._write_graph_delta = MagicMock()
    daemon._write_snet_update = MagicMock()
    daemon._write_settlement_event = MagicMock()

    daemon._step()
    daemon._write_settlement_event.assert_called_once()
    # Empty delta is valid — the critical contract is that the name exists.
    assert daemon._write_settlement_event.call_args.args[0] == []
