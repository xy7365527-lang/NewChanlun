"""Regression tests for TopologicalDaemon SharedLayer behavior."""

from __future__ import annotations

import sys
import types

import pytest

from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex


class _FakeIPFSClient:
    api_url = "http://127.0.0.1:5001"

    def is_available(self) -> bool:
        return True


class _FakeSharedLayer:
    def __init__(self, ipfs: _FakeIPFSClient) -> None:
        self.ipfs = ipfs
        self.blocks: list[dict] = []

    def write_block(self, block: dict) -> str:
        self.blocks.append(block)
        return f"cid-{len(self.blocks)}"


class _FailingSharedLayer:
    def __init__(self, ipfs: _FakeIPFSClient) -> None:
        raise TimeoutError("mfs init timed out")


class _FakeCrossInstanceSync:
    def __init__(self, shared_layer, instance_id: str, daemon) -> None:
        self.shared_layer = shared_layer
        self.instance_id = instance_id
        self.daemon = daemon


def _install_shared_layer_modules(monkeypatch, shared_layer_cls) -> None:
    chain_pkg = types.ModuleType("chain")
    chain_pkg.__path__ = []
    ipfs_mod = types.ModuleType("chain.ipfs_client")
    ipfs_mod.IPFSClient = _FakeIPFSClient
    swarm_pkg = types.ModuleType("swarm")
    swarm_pkg.__path__ = []
    shared_mod = types.ModuleType("swarm.shared_layer")
    shared_mod.SharedLayer = shared_layer_cls
    cross_mod = types.ModuleType("swarm.cross_instance")
    cross_mod.CrossInstanceSync = _FakeCrossInstanceSync

    monkeypatch.setitem(sys.modules, "chain", chain_pkg)
    monkeypatch.setitem(sys.modules, "chain.ipfs_client", ipfs_mod)
    monkeypatch.setitem(sys.modules, "swarm", swarm_pkg)
    monkeypatch.setitem(sys.modules, "swarm.shared_layer", shared_mod)
    monkeypatch.setitem(sys.modules, "swarm.cross_instance", cross_mod)


def _small_graph() -> Graph:
    graph = Graph()
    graph = graph.add_vertex(Vertex("A"))
    graph = graph.add_vertex(Vertex("B"))
    graph = graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    return graph


def test_shared_layer_step_without_new_settlements_does_not_crash(monkeypatch) -> None:
    """SharedLayer enabled steps must pass an empty settlement delta, not crash."""
    graph = _small_graph()
    log = types.SimpleNamespace(
        beta_1_after=0,
        delta_beta_1=0,
        operation="walk",
        position="A",
        encounter=None,
    )
    engine = types.SimpleNamespace(
        run_step=lambda: log,
        k_active=graph,
        k_full=graph,
        terrain={},
        position="A",
    )
    daemon = object.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.engine = engine
    daemon.snet_activation = None
    daemon.settlement = types.SimpleNamespace(settled_cycles=[])
    daemon._persist = None
    daemon._shared_layer = object()
    daemon._beta_1_history = []
    daemon._cumulative_delta_beta_1 = 0
    daemon._local_f_history = []
    settlement_events: list[list] = []

    monkeypatch.setattr(daemon, "_write_snet_cooccurrence_blocks", lambda: None)
    monkeypatch.setattr(daemon, "_is_locally_crystallized", lambda: False)
    monkeypatch.setattr(daemon, "_should_check_gaps", lambda: False)
    monkeypatch.setattr(daemon, "_fire", lambda event, *args: None)
    monkeypatch.setattr(daemon, "_is_significant", lambda step_log: False)
    monkeypatch.setattr(daemon, "_write_traversal_position", lambda log: None)
    monkeypatch.setattr(daemon, "_write_graph_delta", lambda log, vids, edges: None)
    monkeypatch.setattr(daemon, "_write_settlement_event", settlement_events.append)
    monkeypatch.setattr(daemon, "_write_snet_update", lambda: None)

    daemon._step()

    assert settlement_events == [[]]


def test_require_chain_raises_when_shared_layer_init_fails(monkeypatch) -> None:
    """require_chain=True must not silently continue without a SharedLayer."""
    _install_shared_layer_modules(monkeypatch, _FailingSharedLayer)

    with pytest.raises(RuntimeError, match="SharedLayer init failed"):
        TopologicalDaemon(
            graph=_small_graph(),
            settlement_threshold=99,
            require_chain=True,
        )
