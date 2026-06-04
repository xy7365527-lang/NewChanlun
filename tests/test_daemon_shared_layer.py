from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def test_shared_layer_step_defines_new_settled_before_write():
    """SharedLayer enabled path must not crash before settlement write throttling."""
    topo_path = Path(__file__).resolve().parents[1] / "topological-computation"
    script = """
from types import MethodType, SimpleNamespace
from daemon import TopologicalDaemon

class FakeGraph:
    def __init__(self):
        self._vertices = {}
        self._edges = []
        self.vertices = {}

class FakeEngine:
    def __init__(self, graph):
        self.k_active = graph
        self.k_full = graph
        self.terrain = {}
        self._last_articulation = None

    def run_step(self):
        return SimpleNamespace(beta_1_after=0, delta_beta_1=0, step=1)

graph = FakeGraph()
daemon = TopologicalDaemon.__new__(TopologicalDaemon)
daemon.total_steps = 0
daemon._persist = None
daemon._shared_layer = object()
daemon.engine = FakeEngine(graph)
daemon.k_active = graph
daemon.k_full = graph
daemon.terrain = {}
daemon.snet_activation = None
daemon._beta_1_history = []
daemon._local_f_history = []
daemon._cumulative_delta_beta_1 = 0
daemon.settlement = SimpleNamespace(settled_cycles=[])
calls = []
daemon._write_snet_cooccurrence_blocks = MethodType(lambda self: None, daemon)
daemon._is_locally_crystallized = MethodType(lambda self: False, daemon)
daemon._fire = MethodType(lambda self, event, *args: None, daemon)
daemon._is_significant = MethodType(lambda self, log: False, daemon)
daemon._should_check_gaps = MethodType(lambda self: False, daemon)
daemon._write_traversal_position = MethodType(
    lambda self, log: calls.append(("position", None)), daemon
)
daemon._write_graph_delta = MethodType(
    lambda self, log, vids, edges: calls.append(("graph", len(vids) + len(edges))),
    daemon,
)
daemon._write_settlement_event = MethodType(
    lambda self, new_settled: calls.append(("settlement", len(new_settled))), daemon
)
daemon._write_snet_update = MethodType(lambda self: calls.append(("snet", None)), daemon)
daemon._step()
assert ("settlement", 0) in calls, calls
"""
    env = {**os.environ, "PYTHONPATH": str(topo_path)}
    result = subprocess.run(
        [sys.executable, "-c", script],
        env=env,
        capture_output=True,
        text=True,
        timeout=20,
        check=False,
    )

    assert result.returncode == 0, result.stderr or result.stdout
