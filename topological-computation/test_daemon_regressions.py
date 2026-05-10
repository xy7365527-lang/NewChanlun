"""Regression tests for daemon startup and SharedLayer sync safety."""

from __future__ import annotations

import sys
import types

import pytest

import daemon as daemon_module
from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex
from traversal import StepLog


class _Recorder:
    def record(self, *args, **kwargs) -> None:
        pass

    def record_settlement(self, *args, **kwargs) -> None:
        pass


class _NullCheckpoint:
    def __init__(self, *args, **kwargs) -> None:
        self.settlements = _Recorder()
        self.encounters = _Recorder()

    def load_state(self):
        return None

    def save_state(self, *args, **kwargs) -> None:
        pass


def _disable_external_state(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(daemon_module, "TraversalCheckpoint", _NullCheckpoint)
    monkeypatch.setattr(TopologicalDaemon, "_bootstrap_snet", lambda self: None)
    monkeypatch.setattr(TopologicalDaemon, "_register_snet_param_nodes", lambda self: None)
    monkeypatch.setattr(TopologicalDaemon, "_setup_snet_activation", lambda self: None)


def _install_fake_ipfs(
    monkeypatch: pytest.MonkeyPatch,
    *,
    available: bool,
    shared_layer_cls: type | None = None,
) -> None:
    ipfs_mod = types.ModuleType("chain.ipfs_client")

    class FakeIPFSClient:
        api_url = "http://127.0.0.1:5001"

        def is_available(self) -> bool:
            return available

    ipfs_mod.IPFSClient = FakeIPFSClient

    shared_mod = types.ModuleType("swarm.shared_layer")
    shared_mod.SharedLayer = shared_layer_cls or type(
        "FakeSharedLayer",
        (),
        {"__init__": lambda self, ipfs: None},
    )

    cross_mod = types.ModuleType("swarm.cross_instance")
    cross_mod.CrossInstanceSync = type(
        "FakeCrossInstanceSync",
        (),
        {"__init__": lambda self, **kwargs: setattr(self, "known_blocks", set())},
    )

    monkeypatch.setitem(sys.modules, "chain.ipfs_client", ipfs_mod)
    monkeypatch.setitem(sys.modules, "swarm.shared_layer", shared_mod)
    monkeypatch.setitem(sys.modules, "swarm.cross_instance", cross_mod)


def test_require_chain_fails_when_shared_layer_init_fails(monkeypatch):
    """require_chain=True must not silently continue without SharedLayer sync."""
    _disable_external_state(monkeypatch)

    class RaisingSharedLayer:
        def __init__(self, ipfs) -> None:
            raise TimeoutError("mfs mkdir timed out")

    _install_fake_ipfs(monkeypatch, available=True, shared_layer_cls=RaisingSharedLayer)

    with pytest.raises(RuntimeError, match="SharedLayer 初始化失败"):
        TopologicalDaemon(graph=Graph(), require_chain=True)


def test_memory_purge_resyncs_initialized_engine(monkeypatch):
    """Purged memory:* vertices must not be restored from TraversalEngine on step 1."""
    _disable_external_state(monkeypatch)
    _install_fake_ipfs(monkeypatch, available=False)

    graph = Graph()
    graph = graph.add_vertex(Vertex("A", content="A"))
    graph = graph.add_vertex(Vertex("memory:stale", content="stale"))
    graph = graph.add_edge(Edge("A", "memory:stale", EdgeType.DEPENDENCY))

    daemon = TopologicalDaemon(graph=graph, require_chain=False)

    assert "memory:stale" not in daemon.k_active.vertices
    assert "memory:stale" not in daemon.k_full.vertices
    assert daemon.engine is not None
    assert "memory:stale" not in daemon.engine.k_active.vertices
    assert "memory:stale" not in daemon.engine.k_full.vertices


def test_shared_layer_step_passes_new_settlements_without_name_error():
    """SharedLayer-enabled _step should compute new_settled before writing events."""
    graph = Graph().add_vertex(Vertex("A", content="A"))
    settlement = types.SimpleNamespace(settled_cycles=[])

    class FakeEngine:
        def __init__(self) -> None:
            self.k_active = graph
            self.k_full = graph
            self.terrain = {}
            self.position = "A"
            self._last_articulation = None

        def run_step(self):
            settlement.settled_cycles.append("cycle-1")
            return StepLog(
                step=1,
                position="A",
                encounter="",
                operation="nothing",
                beta_1_before=0,
                beta_1_after=0,
                delta_beta_1=0,
                settled_count=1,
                blocked=False,
                vertices_active=1,
                edges_active=0,
                vertices_full=1,
                edges_full=0,
            )

    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon._persist = None
    daemon._shared_layer = object()
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.engine = FakeEngine()
    daemon.snet_activation = None
    daemon.snet = None
    daemon.settlement = settlement
    daemon._beta_1_history = []
    daemon._cumulative_delta_beta_1 = 0.0
    daemon.total_gaps_detected = 0
    daemon.total_events = 0
    daemon.event_log = []
    daemon.concept_names = {}
    daemon._write_snet_cooccurrence_blocks = lambda: None
    daemon._is_locally_crystallized = lambda: False
    daemon._should_check_gaps = lambda: False
    daemon._is_significant = lambda log: False
    daemon._fire = lambda *args, **kwargs: None
    daemon._write_traversal_position = lambda log: None
    daemon._write_graph_delta = lambda log, new_vids, new_edges: None
    daemon._write_snet_update = lambda: None

    captured = {}
    daemon._write_settlement_event = lambda new_settled: captured.setdefault(
        "new_settled", new_settled
    )

    TopologicalDaemon._step(daemon)

    assert captured["new_settled"] == ["cycle-1"]
