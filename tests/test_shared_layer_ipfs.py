from __future__ import annotations

import sys
from pathlib import Path

import pytest


TOPO_DIR = Path(__file__).resolve().parents[1] / "topological-computation"
if str(TOPO_DIR) not in sys.path:
    sys.path.insert(0, str(TOPO_DIR))

from swarm.shared_layer import SharedLayer  # noqa: E402


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
