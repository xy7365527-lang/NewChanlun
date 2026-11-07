"""Regression tests for SharedLayer synchronization failure modes."""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent))

from daemon import TopologicalDaemon
from engine import Graph, Vertex, VertexStatus
from swarm.cross_instance import CrossInstanceSync
from traversal import StepLog


class FakeSharedLayer:
    def __init__(self) -> None:
        self.blocks: list[dict] = []

    def all_block_hashes(self) -> set[str]:
        return set()

    def write_block(self, block: dict) -> str:
        self.blocks.append(block)
        return f"cid-{len(self.blocks)}"


def _graph(*vertex_ids: str) -> Graph:
    graph = Graph()
    for vid in vertex_ids:
        graph = graph.add_vertex(Vertex(id=vid, status=VertexStatus.ACTIVE, content=vid))
    return graph


def test_shared_layer_step_tracks_settlements_without_name_error(monkeypatch) -> None:
    graph = _graph("A", "B")
    log = StepLog(
        step=1,
        position="A",
        encounter="nothing",
        operation="walk",
        beta_1_before=0,
        beta_1_after=0,
        delta_beta_1=0,
        settled_count=0,
        blocked=False,
        vertices_active=2,
        edges_active=0,
        vertices_full=2,
        edges_full=0,
    )
    daemon = object.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon.k_active = graph.copy()
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.engine = SimpleNamespace(
        run_step=lambda: log,
        k_active=daemon.k_active,
        k_full=daemon.k_full,
        terrain={},
        position="A",
    )
    daemon.settlement = SimpleNamespace(settled_cycles=[])
    daemon.snet_activation = None
    daemon._persist = None
    daemon._shared_layer = FakeSharedLayer()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    daemon._last_settlement_write_count = 0
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0

    settlement_events: list[list] = []
    monkeypatch.setattr(TopologicalDaemon, "_write_snet_cooccurrence_blocks", lambda self: None)
    monkeypatch.setattr(TopologicalDaemon, "_is_locally_crystallized", lambda self: False)
    monkeypatch.setattr(TopologicalDaemon, "_should_check_gaps", lambda self: False)
    monkeypatch.setattr(TopologicalDaemon, "_is_significant", lambda self, log: False)
    monkeypatch.setattr(TopologicalDaemon, "_fire", lambda self, event_type, *args: None)
    monkeypatch.setattr(TopologicalDaemon, "_write_traversal_position", lambda self, log: None)
    monkeypatch.setattr(TopologicalDaemon, "_write_graph_delta", lambda self, log, vids, edges: None)
    monkeypatch.setattr(
        TopologicalDaemon,
        "_write_settlement_event",
        lambda self, new_settled: settlement_events.append(new_settled),
    )
    monkeypatch.setattr(TopologicalDaemon, "_write_snet_update", lambda self: None)

    TopologicalDaemon._step(daemon)

    assert settlement_events == [[]]


def test_graph_delta_writes_for_negate_operations() -> None:
    graph = _graph("A", "B", "C")
    daemon = object.__new__(TopologicalDaemon)
    daemon.k_active = graph
    daemon.total_steps = 1
    daemon._instance_id = "local"
    daemon._last_graph_delta_write_time = 0.0
    daemon._shared_layer = FakeSharedLayer()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    log = StepLog(
        step=1,
        position="A",
        encounter="B",
        operation="negate_a",
        beta_1_before=0,
        beta_1_after=1,
        delta_beta_1=1,
        settled_count=0,
        blocked=False,
        vertices_active=3,
        edges_active=0,
        vertices_full=3,
        edges_full=0,
        f_value=3,
    )

    daemon._write_graph_delta(log, {"C"}, [])

    assert [block["type"] for block in daemon._shared_layer.blocks] == ["graph_delta"]
    assert daemon._shared_layer.blocks[0]["operation"] == "negate_a"


def test_cross_instance_injection_preserves_full_graph_history() -> None:
    full_graph = _graph("A", "B", "history_only")
    active_graph = _graph("A", "B")
    engine = SimpleNamespace(k_active=active_graph, k_full=full_graph)
    daemon = SimpleNamespace(
        k_active=active_graph,
        k_full=full_graph,
        engine=engine,
        _initialize_engine=lambda: None,
    )
    sync = CrossInstanceSync(FakeSharedLayer(), "local", daemon)

    sync._inject_external_operation({
        "instance": "peer",
        "type": "graph_delta",
        "vertices": [
            {"id": "C", "status": "active", "content": "C", "created_at": 1},
        ],
        "edges": [
            {"source": "A", "target": "C", "edge_type": "reference", "created_at": 1},
        ],
    })

    assert daemon.k_active.vertex("C") is not None
    assert daemon.k_full.vertex("C") is not None
    assert daemon.k_full.vertex("history_only") is not None
    assert daemon.engine.k_full is daemon.k_full
