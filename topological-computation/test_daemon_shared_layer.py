"""Regression tests for TopologicalDaemon SharedLayer failure paths."""

from __future__ import annotations

from types import SimpleNamespace

import pytest

from daemon import TopologicalDaemon
from engine import Graph, Vertex
from traversal import StepLog


class _FakeEngine:
    def __init__(self, graph: Graph) -> None:
        self.k_active = graph
        self.k_full = graph
        self.terrain = {}
        self.position = "A"

    def run_step(self) -> StepLog:
        return StepLog(
            step=1,
            position="A",
            encounter="",
            operation="nothing",
            beta_1_before=0,
            beta_1_after=0,
            delta_beta_1=0,
            settled_count=0,
            blocked=False,
            vertices_active=1,
            edges_active=0,
            vertices_full=1,
            edges_full=0,
        )


class _RecordingSharedLayer:
    def __init__(self, *, fail: bool = False) -> None:
        self.fail = fail
        self.blocks: list[dict] = []

    def write_block(self, block: dict) -> str:
        if self.fail:
            raise TimeoutError("ipfs write timed out")
        self.blocks.append(block)
        return f"block-{len(self.blocks)}"


def _daemon_with_shared_layer(shared_layer: _RecordingSharedLayer) -> TopologicalDaemon:
    graph = Graph().add_vertex(Vertex("A", content="alpha"))
    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.engine = _FakeEngine(graph)
    daemon.terrain = {}
    daemon.snet_activation = None
    daemon.settlement = SimpleNamespace(settled_cycles=[])
    daemon._persist = None
    daemon._shared_layer = shared_layer
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    daemon._instance_id = "test-instance"
    daemon._session_id = "test-session"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._last_graph_delta_write_time = 0.0
    daemon._last_snet_write_time = 0.0
    daemon._last_snet_active_snapshot = set()
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0.0
    daemon._crystallization_count = 0
    daemon._last_gap_check_step = 0
    daemon.total_steps = 0
    daemon.total_events = 0
    daemon.total_gaps_detected = 0
    daemon.concept_names = {}
    daemon.event_log = []
    daemon.encounter_log = SimpleNamespace(record_encounter=lambda **_: None)
    daemon._checkpoint = SimpleNamespace(
        encounters=SimpleNamespace(record=lambda **_: None),
        settlements=SimpleNamespace(record_settlement=lambda **_: None),
        save_state=lambda *_: None,
    )
    daemon._fire = lambda *_: None
    daemon._is_locally_crystallized = lambda: False
    daemon._should_check_gaps = lambda: False
    daemon._detect_gaps = lambda: []
    return daemon


def test_shared_layer_step_handles_no_new_settlements() -> None:
    shared_layer = _RecordingSharedLayer()
    daemon = _daemon_with_shared_layer(shared_layer)

    daemon._step()

    assert [block["type"] for block in shared_layer.blocks] == ["traversal_position"]


def test_traversal_position_write_failure_does_not_crash_step() -> None:
    shared_layer = _RecordingSharedLayer(fail=True)
    daemon = _daemon_with_shared_layer(shared_layer)

    daemon._step()

    assert shared_layer.blocks == []


def test_require_chain_fails_when_shared_layer_init_fails(monkeypatch: pytest.MonkeyPatch) -> None:
    class AvailableIPFS:
        api_url = "http://127.0.0.1:5001"

        def is_available(self) -> bool:
            return True

    class FailingSharedLayer:
        def __init__(self, _ipfs: AvailableIPFS) -> None:
            raise TimeoutError("files_mkdir timed out")

    monkeypatch.setattr("chain.ipfs_client.IPFSClient", AvailableIPFS)
    monkeypatch.setattr("swarm.shared_layer.SharedLayer", FailingSharedLayer)

    with pytest.raises(RuntimeError, match="SharedLayer init failed"):
        TopologicalDaemon(graph=Graph(), require_chain=True)


def test_startup_memory_purge_resyncs_initialized_engine(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class UnavailableIPFS:
        api_url = "http://127.0.0.1:5001"

        def is_available(self) -> bool:
            return False

    monkeypatch.setattr("chain.ipfs_client.IPFSClient", UnavailableIPFS)
    monkeypatch.setattr(
        TopologicalDaemon,
        "_initialize_engine",
        lambda self: setattr(
            self,
            "engine",
            SimpleNamespace(k_active=self.k_active, k_full=self.k_full),
        ),
    )

    graph = Graph()
    graph = graph.add_vertex(Vertex("A", content="alpha"))
    graph = graph.add_vertex(Vertex("memory:old", content="stale memory"))

    daemon = TopologicalDaemon(graph=graph, require_chain=False)

    assert daemon.engine is not None
    assert "memory:old" not in daemon.k_active.vertices
    assert "memory:old" not in daemon.engine.k_active.vertices
