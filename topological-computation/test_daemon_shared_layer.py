"""Regression tests for TopologicalDaemon SharedLayer settlement sync."""

from __future__ import annotations

import types

from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex


def _small_graph() -> Graph:
    graph = Graph()
    graph = graph.add_vertex(Vertex("A"))
    graph = graph.add_vertex(Vertex("B"))
    graph = graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    return graph


def test_shared_layer_step_without_new_settlements_does_not_crash(monkeypatch) -> None:
    """SharedLayer enabled steps must pass an empty settlement delta, not crash.

    After settlement memory injection was disabled, `_step()` still calls
    `_write_settlement_event(new_settled)`. Without capturing pre-step settled
    count, `new_settled` is undefined and the first SharedLayer step raises
    NameError.
    """
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
    monkeypatch.setattr(daemon, "_write_traversal_position", lambda step_log: None)
    monkeypatch.setattr(daemon, "_write_graph_delta", lambda step_log, vids, edges: None)
    monkeypatch.setattr(daemon, "_write_settlement_event", settlement_events.append)
    monkeypatch.setattr(daemon, "_write_snet_update", lambda: None)

    daemon._step()

    assert settlement_events == [[]]
