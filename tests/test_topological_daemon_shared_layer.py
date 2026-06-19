"""SharedLayer regression tests for topological daemon."""

from __future__ import annotations

import subprocess
import sys
import textwrap
from pathlib import Path


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"


def _run_isolated_daemon_check(body: str) -> None:
    script = f"""
import sys
from types import SimpleNamespace

sys.path.insert(0, {str(TOPO_DIR)!r})

from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex


class NoopWriter:
    def record(self, **_kwargs):
        pass

    def record_settlement(self, **_kwargs):
        pass


class NoopCheckpoint:
    def __init__(self):
        self.encounters = NoopWriter()
        self.settlements = NoopWriter()

    def load_state(self):
        return None

    def save_state(self, _state):
        pass

    def close(self):
        pass


class FakeSharedLayer:
    def __init__(self, fail=False):
        self.fail = fail
        self.blocks = []

    def write_block(self, block):
        if self.fail:
            raise TimeoutError("ipfs write timed out")
        self.blocks.append(block)
        return f"fake-cid-{{len(self.blocks)}}"


class FakeSync:
    def __init__(self):
        self.known_blocks = set()


class FakeEngine:
    def __init__(self, graph):
        self.k_active = graph
        self.k_full = graph
        self.terrain = {{}}
        self.position = "A"
        self.logs = []

    def run_step(self):
        log = SimpleNamespace(
            step=1,
            operation="move",
            position="A",
            encounter=None,
            f_value=0.0,
            g_value=0.0,
            beta_1_before=0,
            beta_1_after=0,
            delta_beta_1=0,
            blocked=False,
            vertices_active=2,
            edges_active=1,
        )
        self.logs.append(log)
        return log


def make_graph():
    graph = Graph()
    graph = graph.add_vertex(Vertex("A"))
    graph = graph.add_vertex(Vertex("B"))
    graph = graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    return graph


def make_daemon():
    graph = make_graph()
    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.settlement = SimpleNamespace(settled_cycles=[])
    daemon.engine = FakeEngine(graph)
    daemon.terrain = {{}}
    daemon.snet_activation = None
    daemon._persist = None
    daemon.total_steps = 0
    daemon.total_events = 0
    daemon.total_gaps_detected = 0
    daemon.event_log = []
    daemon._callbacks = {{"on_gap": [], "on_event": [], "on_feed": [], "on_step": []}}
    daemon._checkpoint = NoopCheckpoint()
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0.0
    daemon._crystallization_count = 0
    daemon._last_gap_check_step = 0
    daemon._shared_layer = None
    daemon._cross_instance_sync = None
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._last_graph_delta_write_time = 0.0
    daemon._last_snet_write_time = 0.0
    daemon._last_snet_active_snapshot = set()
    daemon._instance_id = "test-instance"
    daemon._session_id = "test-session"
    daemon.concept_names = {{}}
    daemon.peer_positions = {{}}
    return daemon


{textwrap.indent(body, "")}
"""
    subprocess.run([sys.executable, "-c", script], check=True, capture_output=True, text=True)


def test_shared_layer_step_without_new_settlement_does_not_crash() -> None:
    _run_isolated_daemon_check(
        """
daemon = make_daemon()
daemon._shared_layer = FakeSharedLayer()
daemon._cross_instance_sync = FakeSync()
daemon._step()
assert daemon.total_steps == 1
"""
    )


def test_shared_layer_position_write_failure_is_non_fatal() -> None:
    _run_isolated_daemon_check(
        """
daemon = make_daemon()
daemon._shared_layer = FakeSharedLayer(fail=True)
daemon._cross_instance_sync = FakeSync()
daemon._write_traversal_position(SimpleNamespace(operation="fold"))
assert daemon._last_position_write_label == ""
"""
    )
