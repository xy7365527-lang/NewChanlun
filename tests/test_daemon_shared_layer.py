from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
if str(TOPO_DIR) not in sys.path:
    sys.path.insert(0, str(TOPO_DIR))

from daemon import TopologicalDaemon
from engine import Graph, SettlementTracker


class FakeSharedLayer:
    def write_block(self, block: dict) -> str:
        return f"fake:{block['type']}"


def test_shared_layer_step_uses_empty_settlement_delta_without_crashing():
    graph = Graph()
    log = SimpleNamespace(
        step=1,
        operation="walk",
        encounter="nothing",
        position="root",
        beta_1_before=0,
        beta_1_after=0,
        delta_beta_1=0,
        blocked=False,
        vertices_active=0,
        edges_active=0,
        f_value=-99,
        g_value=-99,
    )
    engine = SimpleNamespace(
        run_step=lambda: log,
        k_active=graph,
        k_full=graph,
        terrain={},
        position="root",
    )

    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.total_steps = 0
    daemon.total_events = 0
    daemon.total_gaps_detected = 0
    daemon.event_log = []
    daemon.concept_names = {}
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.engine = engine
    daemon.settlement = SettlementTracker(threshold=1)
    daemon.snet = None
    daemon.snet_activation = None
    daemon._persist = None
    daemon._shared_layer = FakeSharedLayer()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0.0

    captured_settlement_deltas = []
    daemon._write_snet_cooccurrence_blocks = lambda: None
    daemon._is_significant = lambda step_log: False
    daemon._should_check_gaps = lambda: False
    daemon._is_locally_crystallized = lambda: False
    daemon._write_traversal_position = lambda step_log: None
    daemon._write_graph_delta = lambda step_log, new_vids, new_edges: None
    daemon._write_settlement_event = (
        lambda new_settled: captured_settlement_deltas.append(list(new_settled))
    )
    daemon._write_snet_update = lambda: None

    daemon._step()

    assert captured_settlement_deltas == [[]]
