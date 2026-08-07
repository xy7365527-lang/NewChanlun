"""Regression: reinit after fold must not collapse k_full onto k_active."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

from engine import (
    Edge,
    EdgeType,
    Graph,
    SettlementTracker,
    Vertex,
    VertexStatus,
    fold,
)
from daemon import TopologicalDaemon


def _triangle_graph() -> Graph:
    g = Graph()
    for name in ("A", "B", "C"):
        g = g.add_vertex(
            Vertex(id=name, status=VertexStatus.ACTIVE, content=name, created_at=0)
        )
    g = g.add_edge(
        Edge(source="A", target="B", edge_type=EdgeType.DEPENDENCY, created_at=0)
    )
    g = g.add_edge(
        Edge(source="B", target="C", edge_type=EdgeType.DEPENDENCY, created_at=0)
    )
    g = g.add_edge(
        Edge(source="A", target="C", edge_type=EdgeType.DEPENDENCY, created_at=0)
    )
    return g


def _daemon_shell(k_full: Graph, k_active: Graph) -> TopologicalDaemon:
    """Minimal daemon instance that can call _initialize_engine without S_net ingest."""
    daemon = object.__new__(TopologicalDaemon)
    daemon.k_full = k_full
    daemon.k_active = k_active
    daemon.settlement = SettlementTracker(threshold=999)
    daemon._seed = 0
    daemon.encounter_log = None
    daemon.engine = None
    daemon.snet_activation = None
    daemon.terrain = {}
    daemon.registry = None
    daemon.concept_names = {}
    daemon._snet_block_watermark = 0
    daemon._snet_hyperedge_watermark = 0
    daemon.snet = MagicMock()
    daemon.snet._signifiers = {}
    daemon.snet._edges = []
    daemon.snet.hyperedges = []
    daemon.snet.edge_count = 0
    daemon.snet.hyperedge_count = 0
    return daemon


def test_initialize_engine_preserves_k_full_history_after_fold() -> None:
    """fold then reinit (ingest_code path) must keep k_full[B]=ACTIVE."""
    k_full = _triangle_graph()
    k_active = k_full.copy()

    result = fold(
        k_active,
        ["A", "B"],
        step=1,
        settlement=SettlementTracker(threshold=999),
    )
    assert not result.blocked
    k_active = result.graph
    assert k_active.vertex("B").status == VertexStatus.FOLDED
    assert k_full.vertex("B").status == VertexStatus.ACTIVE

    daemon = _daemon_shell(k_full, k_active)

    with (
        patch.object(TopologicalDaemon, "_bootstrap_snet", lambda self: None),
        patch.object(TopologicalDaemon, "_register_snet_param_nodes", lambda self: None),
    ):
        daemon._initialize_engine()

    assert daemon.engine is not None
    assert daemon.engine.k_full.vertex("B").status == VertexStatus.ACTIVE
    assert daemon.engine.k_active.vertex("B").status == VertexStatus.FOLDED

    # Next _step sync must not overwrite historical k_full with folded active.
    daemon.k_active = daemon.engine.k_active
    daemon.k_full = daemon.engine.k_full
    assert daemon.k_full.vertex("B").status == VertexStatus.ACTIVE
    assert daemon.k_active.vertex("B").status == VertexStatus.FOLDED


if __name__ == "__main__":
    test_initialize_engine_preserves_k_full_history_after_fold()
    print("PASS")
