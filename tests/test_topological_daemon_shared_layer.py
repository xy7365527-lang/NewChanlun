"""SharedLayer regression tests for topological daemon."""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
sys.path.insert(0, str(TOPO_DIR))

import daemon as daemon_module
from daemon import TopologicalDaemon
from engine import Edge, EdgeType, Graph, Vertex


class _NoopWriter:
    def record(self, **_kwargs) -> None:
        pass

    def record_settlement(self, **_kwargs) -> None:
        pass


class _NoopCheckpoint:
    def __init__(self, *_args, **_kwargs) -> None:
        self.encounters = _NoopWriter()
        self.settlements = _NoopWriter()

    def load_state(self) -> None:
        return None

    def save_state(self, _state: dict) -> None:
        pass

    def close(self) -> None:
        pass


class _FakeSharedLayer:
    def __init__(self, *, fail: bool = False) -> None:
        self.fail = fail
        self.blocks: list[dict] = []

    def write_block(self, block: dict) -> str:
        if self.fail:
            raise TimeoutError("ipfs write timed out")
        self.blocks.append(block)
        return f"fake-cid-{len(self.blocks)}"


class _FakeSync:
    def __init__(self) -> None:
        self.known_blocks: set[str] = set()


class _FakeEngine:
    def __init__(self, graph: Graph) -> None:
        self.k_active = graph
        self.k_full = graph
        self.terrain: dict[tuple[str, str], str] = {}
        self.position = "A"
        self.logs: list[object] = []

    def run_step(self) -> SimpleNamespace:
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


def _graph() -> Graph:
    graph = Graph()
    graph = graph.add_vertex(Vertex("A"))
    graph = graph.add_vertex(Vertex("B"))
    graph = graph.add_edge(Edge("A", "B", EdgeType.DEPENDENCY))
    return graph


def _daemon(monkeypatch) -> TopologicalDaemon:
    monkeypatch.setattr(daemon_module, "TraversalCheckpoint", _NoopCheckpoint)
    return TopologicalDaemon(graph=_graph(), seed=1, require_chain=False)


def test_shared_layer_step_without_new_settlement_does_not_crash(monkeypatch) -> None:
    daemon = _daemon(monkeypatch)
    daemon.engine = _FakeEngine(daemon.k_active)
    daemon.snet_activation = None
    daemon._shared_layer = _FakeSharedLayer()
    daemon._cross_instance_sync = _FakeSync()

    daemon._step()

    assert daemon.total_steps == 1


def test_shared_layer_position_write_failure_is_non_fatal(monkeypatch) -> None:
    daemon = _daemon(monkeypatch)
    daemon._shared_layer = _FakeSharedLayer(fail=True)
    daemon._cross_instance_sync = _FakeSync()

    daemon._write_traversal_position(SimpleNamespace(operation="fold"))

    assert daemon._last_position_write_label == ""
