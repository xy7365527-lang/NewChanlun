"""Regression: SharedLayer position/event writes must degrade on IPFS failure.

Sibling writers (graph_delta / settlement / snet_update) already swallow
runtime IPFS errors. traversal_position (and SwarmDaemon event blocks) must
match that contract — otherwise a mid-run IPFS blip aborts _step().
"""

from __future__ import annotations

import sys
from pathlib import Path
from types import SimpleNamespace

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))

from daemon import TopologicalDaemon
from engine import Graph, Vertex, VertexStatus
from swarm.swarm_daemon import SwarmDaemon
from traversal import StepLog


class ExplodingSharedLayer:
    """SharedLayer stand-in that fails every write (simulates IPFS outage)."""

    def write_block(self, block: dict) -> str:
        raise TimeoutError("ipfs upload timed out")

    def all_block_hashes(self) -> set[str]:
        return set()


def _graph(*vertex_ids: str) -> Graph:
    graph = Graph()
    for vid in vertex_ids:
        graph = graph.add_vertex(
            Vertex(id=vid, status=VertexStatus.ACTIVE, content=vid)
        )
    return graph


def _walk_log(position: str = "A") -> StepLog:
    return StepLog(
        step=1,
        position=position,
        encounter="nothing",
        operation="walk",
        beta_1_before=0,
        beta_1_after=0,
        delta_beta_1=0,
        settled_count=0,
        blocked=False,
        vertices_active=2,
        edges_active=0,
        vertices_full=2,
        edges_full=0,
    )


def test_daemon_traversal_position_survives_ipfs_timeout() -> None:
    daemon = object.__new__(TopologicalDaemon)
    daemon.engine = SimpleNamespace(position="A")
    daemon.k_active = _graph("A", "B")
    daemon.total_steps = 1
    daemon._instance_id = "local"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._shared_layer = ExplodingSharedLayer()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())

    # Must not raise — graceful degradation matches other SharedLayer writers.
    TopologicalDaemon._write_traversal_position(daemon, _walk_log())

    assert daemon._last_position_write_label == ""
    assert daemon._cross_instance_sync.known_blocks == set()


def test_swarm_traversal_position_survives_ipfs_timeout() -> None:
    daemon = object.__new__(SwarmDaemon)
    daemon.engine = SimpleNamespace(position="A", logs=[])
    daemon.k_active = _graph("A", "B")
    daemon.total_steps = 1
    daemon.instance_id = "inst_0"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon.shared = ExplodingSharedLayer()
    daemon.syncer = SimpleNamespace(known_blocks=set())

    SwarmDaemon._write_traversal_position(daemon)

    assert daemon._last_position_write_label == ""
    assert daemon.syncer.known_blocks == set()


def test_swarm_event_block_survives_ipfs_timeout() -> None:
    daemon = object.__new__(SwarmDaemon)
    log = _walk_log()
    log = StepLog(
        step=1,
        position="A",
        encounter="B",
        operation="fold",
        beta_1_before=0,
        beta_1_after=1,
        delta_beta_1=1,
        settled_count=0,
        blocked=False,
        vertices_active=2,
        edges_active=0,
        vertices_full=2,
        edges_full=0,
    )
    graph = _graph("A", "B")
    # Mark a vertex as created this step so the event writer emits a block.
    v = graph.vertex("B")
    assert v is not None
    graph = graph.add_vertex(
        Vertex(id=v.id, status=v.status, content=v.content, created_at=1)
    )

    daemon.shared = ExplodingSharedLayer()
    daemon.syncer = SimpleNamespace(known_blocks=set())
    daemon.engine = SimpleNamespace(logs=[log])
    daemon.k_active = graph
    daemon.total_steps = 1
    daemon.instance_id = "inst_0"
    daemon._blocks_written = 0

    SwarmDaemon._write_event_block(daemon)

    assert daemon._blocks_written == 0
    assert daemon.syncer.known_blocks == set()


def test_daemon_traversal_position_still_writes_when_ipfs_ok() -> None:
    class OkSharedLayer:
        def __init__(self) -> None:
            self.blocks: list[dict] = []

        def write_block(self, block: dict) -> str:
            self.blocks.append(block)
            return f"cid-{len(self.blocks)}"

    shared = OkSharedLayer()
    daemon = object.__new__(TopologicalDaemon)
    daemon.engine = SimpleNamespace(position="A")
    daemon.k_active = _graph("A")
    daemon.total_steps = 3
    daemon._instance_id = "local"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._shared_layer = shared
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())

    TopologicalDaemon._write_traversal_position(daemon, _walk_log())

    assert len(shared.blocks) == 1
    assert shared.blocks[0]["type"] == "traversal_position"
    assert "cid-1" in daemon._cross_instance_sync.known_blocks
    assert daemon._last_position_write_label == "A"


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))
