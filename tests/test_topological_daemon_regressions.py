from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "topological-computation"))

from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex, VertexStatus
from persistence import PersistentKFull


class _FakeEngine:
    def __init__(self, daemon: TopologicalDaemon, new_cycle: object | None = None):
        self._daemon = daemon
        self._new_cycle = new_cycle
        self.k_active = daemon.k_active
        self.k_full = daemon.k_full
        self.terrain = {}
        self.position = "A"

    def run_step(self):
        if self._new_cycle is not None:
            self._daemon.settlement.settled_cycles.append(self._new_cycle)
        return SimpleNamespace(
            beta_1_after=0,
            delta_beta_1=0,
            operation="walk",
            position="A",
            encounter=None,
            blocked=False,
            f_value=0,
            g_value=0,
            beta_1_before=0,
            vertices_active=2,
            edges_active=1,
            step=1,
        )


class _FakeSettlement:
    def __init__(self):
        self.settled_cycles = []


def _minimal_daemon_for_step(new_cycle: object | None = None) -> TopologicalDaemon:
    graph = Graph()
    graph.add_vertex(Vertex("A"))
    graph.add_vertex(Vertex("B"))
    graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))

    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.snet_activation = None
    daemon.snet = None
    daemon.settlement = _FakeSettlement()
    daemon.engine = _FakeEngine(daemon, new_cycle)
    daemon._persist = None
    daemon._shared_layer = object()
    daemon._beta_1_history = []
    daemon._cumulative_delta_beta_1 = 0.0
    daemon._local_f_history = []
    daemon._crystallization_count = 0
    daemon._last_gap_check_step = 0
    daemon.total_gaps_detected = 0
    daemon.total_events = 0

    daemon._write_snet_cooccurrence_blocks = lambda: None
    daemon._is_locally_crystallized = lambda: False
    daemon._should_check_gaps = lambda: False
    daemon._fire = lambda *args, **kwargs: None
    daemon._is_significant = lambda log: False
    daemon._write_traversal_position = lambda log: None
    daemon._write_graph_delta = lambda log, new_vids, new_edges: None
    daemon._write_snet_update = lambda: None
    return daemon


def test_shared_layer_step_passes_new_settlements_without_name_error():
    new_cycle = SimpleNamespace(settled_at_step=1, edges={("A", "B")}, residue=None)
    daemon = _minimal_daemon_for_step(new_cycle)
    captured = []
    daemon._write_settlement_event = lambda new_settled: captured.extend(new_settled)

    TopologicalDaemon._step(daemon)

    assert captured == [new_cycle]


def test_close_preserves_incremental_history_needed_after_snapshot(tmp_path):
    jsonl_path = tmp_path / "k_full.jsonl"

    graph = Graph()
    graph.add_vertex(Vertex("A"))
    graph.add_vertex(Vertex("B"))
    graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))

    persist = PersistentKFull(jsonl_path).open()
    persist.append_vertex(graph.vertex("A"))
    persist.append_vertex(graph.vertex("B"))
    persist.append_edge(graph.edges[0])
    persist.append_merge("A", "B", step=1)
    persist.append_operation(1, "fold", {"position": "A", "encounter": "B"})

    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon._persist_path = jsonl_path
    daemon._persist = persist
    daemon.k_full = graph

    TopologicalDaemon.close(daemon)

    snapshot_path = PersistentKFull.snapshot_path_for(jsonl_path)
    recovered, operations = PersistentKFull.load_snapshot_then_incremental(
        snapshot_path,
        jsonl_path,
    )

    assert recovered.vertex("B").status == VertexStatus.FOLDED
    assert operations and operations[-1]["operation"] == "fold"
