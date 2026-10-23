"""Regression tests for SharedLayer runtime failure degradation."""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from daemon import TopologicalDaemon
from swarm.swarm_daemon import SwarmDaemon


class _FailingSharedLayer:
    def write_block(self, block: dict) -> str:
        raise TimeoutError("ipfs write timed out")


class _FailingSyncer:
    def __init__(self) -> None:
        self.known_blocks: set[str] = set()

    def sync(self) -> int:
        raise TimeoutError("ipfs ls timed out")


class _Vertex:
    id = "v1"
    content = "vertex one"
    created_at = 1

    class status:
        value = "active"


class _Graph:
    vertices = {"v1": _Vertex()}
    edges = []

    def vertex(self, vid: str):
        return self.vertices.get(vid)


class _Engine:
    position = "v1"

    def __init__(self) -> None:
        self.logs = [_Log()]


class _Log:
    operation = "fold"
    delta_beta_1 = 1
    blocked = False
    beta_1_before = 0
    beta_1_after = 1


def test_topological_daemon_ignores_shared_layer_position_write_failure() -> None:
    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.engine = _Engine()
    daemon.k_active = _Graph()
    daemon.total_steps = 1
    daemon._instance_id = "daemon-a"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._shared_layer = _FailingSharedLayer()
    daemon._cross_instance_sync = _FailingSyncer()

    daemon._write_traversal_position(_Log())

    assert daemon._cross_instance_sync.known_blocks == set()


def test_swarm_daemon_ignores_shared_layer_position_write_failure() -> None:
    daemon = SwarmDaemon.__new__(SwarmDaemon)
    daemon.engine = _Engine()
    daemon.k_active = _Graph()
    daemon.total_steps = 1
    daemon.instance_id = "swarm-a"
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon.shared = _FailingSharedLayer()
    daemon.syncer = _FailingSyncer()

    daemon._write_traversal_position()

    assert daemon.syncer.known_blocks == set()


def test_swarm_daemon_ignores_shared_layer_event_write_failure() -> None:
    daemon = SwarmDaemon.__new__(SwarmDaemon)
    daemon.engine = _Engine()
    daemon.k_active = _Graph()
    daemon.total_steps = 1
    daemon.instance_id = "swarm-a"
    daemon._blocks_written = 0
    daemon.shared = _FailingSharedLayer()
    daemon.syncer = _FailingSyncer()

    daemon._write_event_block()

    assert daemon._blocks_written == 0
