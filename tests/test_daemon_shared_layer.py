from __future__ import annotations

import importlib
import sys
from pathlib import Path


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
if str(TOPO_DIR) not in sys.path:
    sys.path.insert(0, str(TOPO_DIR))


def test_shared_layer_step_without_new_settlements_does_not_crash(monkeypatch):
    daemon_mod = importlib.import_module("daemon")
    engine_mod = importlib.import_module("engine")

    class FakeCheckpoint:
        def __init__(self, *args, **kwargs):
            self.encounters = self
            self.settlements = self

        def load_state(self):
            return None

        def save_state(self, *args, **kwargs):
            pass

        def close(self):
            pass

        def record(self, *args, **kwargs):
            pass

        def record_settlement(self, *args, **kwargs):
            pass

    class FakeSharedLayer:
        def __init__(self, ipfs):
            self.blocks = []

        def write_block(self, block):
            self.blocks.append(block)
            return f"block-{len(self.blocks)}"

    class FakeSync:
        def __init__(self, *args, **kwargs):
            self.known_blocks = set()
            self.injected_count = 0

    class FakeEngine:
        def __init__(self, daemon):
            self.k_active = daemon.k_active
            self.k_full = daemon.k_full
            self.terrain = {}
            self.position = "A"
            self._last_articulation = None
            self._nothing_streak = 0
            self._blocked_streak = 0

        def run_step(self):
            return daemon_mod.StepLog(
                step=1,
                position="A",
                encounter="B",
                operation="walk",
                beta_1_before=0,
                beta_1_after=0,
                delta_beta_1=0,
                settled_count=0,
                blocked=False,
                vertices_active=2,
                edges_active=1,
                vertices_full=2,
                edges_full=1,
            )

    def fake_initialize(self):
        self.engine = FakeEngine(self)

    graph = engine_mod.Graph()
    graph = graph.add_vertex(engine_mod.Vertex("A"))
    graph = graph.add_vertex(engine_mod.Vertex("B"))
    graph = graph.add_edge(engine_mod.Edge("A", "B", engine_mod.EdgeType.DEPENDENCY))

    monkeypatch.setattr(daemon_mod, "TraversalCheckpoint", FakeCheckpoint)
    monkeypatch.setattr(daemon_mod.TopologicalDaemon, "_initialize_engine", fake_initialize)
    monkeypatch.setattr(
        daemon_mod.TopologicalDaemon,
        "_write_snet_cooccurrence_blocks",
        lambda self: None,
    )
    monkeypatch.setattr("chain.ipfs_client.IPFSClient.is_available", lambda self: True)
    monkeypatch.setattr("swarm.shared_layer.SharedLayer", FakeSharedLayer)
    monkeypatch.setattr("swarm.cross_instance.CrossInstanceSync", FakeSync)

    daemon = daemon_mod.TopologicalDaemon(graph=graph, require_chain=True)

    assert daemon._shared_layer is not None
    daemon._step()
    assert daemon.total_steps == 1
