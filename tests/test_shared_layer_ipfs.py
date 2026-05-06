from __future__ import annotations

import sys
from types import SimpleNamespace
from pathlib import Path

import pytest


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
if str(TOPO_DIR) not in sys.path:
    sys.path.insert(0, str(TOPO_DIR))

from swarm.cross_instance import CrossInstanceSync  # noqa: E402
from daemon import TopologicalDaemon  # noqa: E402
from swarm.shared_layer import SharedLayer  # noqa: E402
import swarm.identity as swarm_identity  # noqa: E402


class FakeIPFS:
    def __init__(self, *, pin_ok: bool = True):
        self.pin_ok = pin_ok
        self.written_paths: list[str] = []

    def is_available(self) -> bool:
        return True

    def files_mkdir(self, path: str) -> None:
        pass

    def upload(self, data: str) -> str:
        return "bafy-test-cid"

    def pin(self, cid: str) -> bool:
        return self.pin_ok

    def files_write(self, path: str, data: bytes, *, create: bool, truncate: bool) -> None:
        self.written_paths.append(path)


def test_write_block_does_not_index_unpinned_cid() -> None:
    ipfs = FakeIPFS(pin_ok=False)
    shared = SharedLayer(ipfs)  # type: ignore[arg-type]

    with pytest.raises(RuntimeError, match="pin"):
        shared.write_block({"type": "critical"})

    assert ipfs.written_paths == []


class FailingSharedLayer:
    def write_block(self, content: dict) -> str:
        raise RuntimeError("IPFS pin failed for CID bafy-response")

    def write_relation(self, **kwargs) -> None:
        raise AssertionError("relation must not be written when response block is unpinned")


def test_interpretation_write_failure_does_not_escape(monkeypatch) -> None:
    response = SimpleNamespace(
        response="agree",
        operation="fold",
        my_f=0,
        other_f=0,
        target_v="v1",
        target_w="v2",
        reason="test",
    )
    monkeypatch.setattr(swarm_identity, "process_other_operation", lambda snapshot, block: response)

    sync = CrossInstanceSync.__new__(CrossInstanceSync)
    sync.shared = FailingSharedLayer()
    sync.instance_id = "local"
    sync.daemon = SimpleNamespace(k_active=object(), settlement=object(), total_steps=1)
    sync.known_blocks = set()
    sync.agreed_count = 0
    sync.negated_count = 0
    sync.deferred_count = 0

    sync._interpret_and_respond(
        {
            "instance": "peer",
            "vertices": [],
            "edges": [],
        },
        "bafy-source",
    )

    assert sync.known_blocks == set()


class FakeGraph:
    def vertex(self, vertex_id: str):
        return SimpleNamespace(content=f"label:{vertex_id}")


def test_topological_daemon_traversal_write_failure_does_not_escape() -> None:
    daemon = TopologicalDaemon.__new__(TopologicalDaemon)
    daemon.engine = SimpleNamespace(position="v1")
    daemon.k_active = FakeGraph()
    daemon._last_position_write_time = 0.0
    daemon._last_position_write_label = ""
    daemon._instance_id = "local"
    daemon.total_steps = 1
    daemon._shared_layer = FailingSharedLayer()
    daemon._cross_instance_sync = SimpleNamespace(known_blocks=set())
    log = SimpleNamespace(operation="fold")

    daemon._write_traversal_position(log)

    assert daemon._cross_instance_sync.known_blocks == set()
    assert daemon._last_position_write_label == ""
