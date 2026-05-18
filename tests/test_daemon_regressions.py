"""Daemon regression tests for recent shared-layer and memory-purge fixes."""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
if str(TOPO_DIR) not in sys.path:
    sys.path.insert(0, str(TOPO_DIR))

import daemon as daemon_mod  # noqa: E402
from engine import Edge, EdgeType, Graph, SettlementTracker, Vertex  # noqa: E402


class _DummyWriter:
    def record(self, *args, **kwargs):
        return None

    def record_settlement(self, *args, **kwargs):
        return None


class _DummyCheckpoint:
    _settlement_path = Path("/tmp/newchan-test-settlements.jsonl")

    def __init__(self, *args, **kwargs):
        self.encounters = _DummyWriter()
        self.settlements = _DummyWriter()

    def load_state(self):
        return None

    def save_state(self, state):
        return None


def _disable_external_state(monkeypatch):
    monkeypatch.setattr(daemon_mod, "TraversalCheckpoint", _DummyCheckpoint)

    import chain.ipfs_client as ipfs_client

    monkeypatch.setattr(ipfs_client.IPFSClient, "is_available", lambda self: False)


def test_memory_purge_resyncs_initialized_engine(monkeypatch):
    """Purge must update the already-created TraversalEngine graph reference."""
    _disable_external_state(monkeypatch)
    old_graph = Graph(
        vertices={
            "a": Vertex("a", content="A"),
            "b": Vertex("b", content="B"),
            "memory:old": Vertex("memory:old", content="old memory"),
        },
        edges=[
            Edge("a", "b", EdgeType.REFERENCE),
            Edge("memory:old", "a", EdgeType.REFERENCE),
        ],
    )
    purged_graph = daemon_mod._purge_memory_nodes(old_graph)
    daemon = daemon_mod.TopologicalDaemon.__new__(daemon_mod.TopologicalDaemon)
    daemon.k_active = purged_graph
    daemon.k_full = purged_graph
    daemon.terrain = {}
    daemon.engine = SimpleNamespace(
        k_active=old_graph,
        k_full=old_graph,
        position="memory:old",
        visit_history=["memory:old"],
    )

    daemon._sync_engine_after_graph_rewrite()

    assert "memory:old" not in daemon.k_active.vertices
    assert "memory:old" not in daemon.engine.k_active.vertices
    assert daemon.engine.position in daemon.engine.k_active.active_vertex_ids()


def test_shared_layer_step_handles_no_new_settlements(monkeypatch):
    """SharedLayer path must pass an empty settlement list, not an undefined name."""
    _disable_external_state(monkeypatch)
    graph = Graph(vertices={"a": Vertex("a", content="A")})
    daemon = daemon_mod.TopologicalDaemon.__new__(daemon_mod.TopologicalDaemon)

    log = SimpleNamespace(
        beta_1_after=0,
        delta_beta_1=0,
        position="a",
        operation="walk",
        encounter="nothing",
        f_value=0,
        g_value=0,
        blocked=False,
        step=1,
        vertices_active=1,
        edges_active=0,
    )
    daemon.total_steps = 0
    daemon.total_events = 0
    daemon.total_gaps_detected = 0
    daemon.event_log = []
    daemon._callbacks = {"on_step": [], "on_event": [], "on_gap": [], "on_feed": []}
    daemon._persist = None
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._last_graph_delta_write_time = 0.0
    daemon._last_settlement_write_count = 0
    daemon._last_snet_write_time = 0.0
    daemon._last_snet_active_snapshot = set()
    daemon.snet_activation = None
    daemon._beta_1_history = []
    daemon._local_f_history = []
    daemon._cumulative_delta_beta_1 = 0
    daemon.engine = SimpleNamespace(
        run_step=lambda: log,
        k_active=graph,
        k_full=graph,
        terrain={},
        position="a",
        _nothing_streak=0,
        _blocked_streak=0,
    )
    daemon.k_active = graph
    daemon.k_full = graph
    daemon.terrain = {}
    daemon.settlement = SettlementTracker(threshold=15)
    daemon.concept_names = {"a": "A"}
    daemon._shared_layer = SimpleNamespace(write_block=lambda block: "cid")
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    daemon._checkpoint = SimpleNamespace(encounters=_DummyWriter(), settlements=_DummyWriter())
    daemon.encounter_log = _DummyWriter()
    daemon._instance_id = "test-instance"
    daemon._session_id = "test-session"

    monkeypatch.setattr(daemon, "_write_snet_cooccurrence_blocks", lambda: None)
    monkeypatch.setattr(daemon, "_is_locally_crystallized", lambda: False)
    monkeypatch.setattr(daemon, "_should_check_gaps", lambda: False)
    monkeypatch.setattr(daemon, "_is_significant", lambda log: False)

    daemon._step()
