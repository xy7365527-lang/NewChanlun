"""Regression tests for TopologicalDaemon SharedLayer writeback."""

from __future__ import annotations

import os
import sys
from types import SimpleNamespace

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex, VertexStatus


def _small_graph() -> Graph:
    graph = Graph()
    for vid in ("alpha", "beta", "gamma"):
        graph = graph.add_vertex(
            Vertex(id=vid, status=VertexStatus.ACTIVE, content=vid, created_at=0)
        )
    graph = graph.add_edge(
        Edge("alpha", "beta", EdgeType.DEPENDENCY, created_at=0)
    )
    graph = graph.add_edge(
        Edge("beta", "gamma", EdgeType.DEPENDENCY, created_at=0)
    )
    return graph


def test_shared_layer_step_without_new_settlement_does_not_crash():
    graph = _small_graph()
    settlement_events: list[list] = []
    daemon = object.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.settlement = SimpleNamespace(settled_cycles=[])
    daemon.snet_activation = None
    daemon.snet = SimpleNamespace(edges=[])
    daemon._persist = None
    daemon._shared_layer = object()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    daemon.engine = SimpleNamespace(
        k_active=graph,
        k_full=graph,
        terrain={},
        position="alpha",
        _last_articulation=None,
        run_step=lambda: SimpleNamespace(
            beta_1_after=0,
            delta_beta_1=0,
        ),
    )
    daemon._write_snet_cooccurrence_blocks = lambda: None
    daemon._compute_local_f_terrain = lambda: 0.0
    daemon._is_locally_crystallized = lambda: False
    daemon._should_check_gaps = lambda: False
    daemon._fire = lambda *args, **kwargs: None
    daemon._is_significant = lambda log: False
    daemon._write_traversal_position = lambda log: None
    daemon._write_graph_delta = lambda log, new_vids, new_edges: None
    daemon._write_settlement_event = settlement_events.append
    daemon._write_snet_update = lambda: None
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0.0

    daemon._step()

    assert daemon.total_steps == 1
    assert settlement_events == [[]]
