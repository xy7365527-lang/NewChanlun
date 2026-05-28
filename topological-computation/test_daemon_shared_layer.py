"""Tests for TopologicalDaemon SharedLayer synchronization."""

from __future__ import annotations

from types import SimpleNamespace

from daemon import TopologicalDaemon
from engine import Graph, Vertex


def test_step_passes_new_settlements_to_shared_layer_writer() -> None:
    """SharedLayer-enabled steps must not crash when settlement memory injection is off."""
    graph = Graph().add_vertex(Vertex("A"))
    new_cycle = SimpleNamespace(
        settled_at_step=1,
        edges=frozenset({("A", "A", "dependency")}),
        residue=[],
    )
    settlement = SimpleNamespace(settled_cycles=[])

    class FakeEngine:
        def __init__(self) -> None:
            self.k_active = graph
            self.k_full = graph
            self.terrain = {}
            self.position = "A"
            self._last_articulation = None

        def run_step(self):
            settlement.settled_cycles.append(new_cycle)
            return SimpleNamespace(
                step=1,
                operation="nothing",
                position="A",
                encounter=None,
                f_value=0.0,
                g_value=0.0,
                beta_1_before=0,
                beta_1_after=0,
                delta_beta_1=0,
                blocked=False,
                vertices_active=1,
                edges_active=0,
            )

    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon._persist = None
    daemon._shared_layer = object()
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.engine = FakeEngine()
    daemon.terrain = {}
    daemon.snet_activation = None
    daemon.snet = None
    daemon.settlement = settlement
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0.0
    daemon.total_gaps_detected = 0

    captured_settlements = []
    daemon._write_snet_cooccurrence_blocks = lambda: None
    daemon._is_locally_crystallized = lambda: False
    daemon._should_check_gaps = lambda: False
    daemon._is_significant = lambda log: False
    daemon._fire = lambda *args, **kwargs: None
    daemon._write_traversal_position = lambda log: None
    daemon._write_graph_delta = lambda log, new_vids, new_edges: None
    daemon._write_settlement_event = lambda new_settled: captured_settlements.extend(new_settled)
    daemon._write_snet_update = lambda: None

    daemon._step()

    assert captured_settlements == [new_cycle]
