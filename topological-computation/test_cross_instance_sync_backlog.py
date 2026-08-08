"""Regression: CrossInstanceSync must not permanently drop unread CIDs."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from engine import Graph
from swarm.cross_instance import CrossInstanceSync


class _FakeSharedLayer:
    """Minimal SharedLayer stand-in: in-memory CID → block map."""

    def __init__(self, blocks: dict[str, dict]):
        self._blocks = dict(blocks)
        self.read_attempts: list[str] = []
        self.fail_once: set[str] = set()

    def all_block_hashes(self) -> set[str]:
        return set(self._blocks)

    def read_block(self, cid: str) -> dict | None:
        self.read_attempts.append(cid)
        if cid in self.fail_once:
            self.fail_once.discard(cid)
            return None
        return dict(self._blocks[cid])

    def write_block(self, content: dict) -> str:
        raise AssertionError("write_block should not be called in these tests")

    def write_relation(self, *args, **kwargs) -> None:
        raise AssertionError("write_relation should not be called in these tests")


class _StubDaemon:
    def __init__(self) -> None:
        self.k_active = Graph()
        self.k_full = Graph()
        self.engine = None
        self.peer_positions: dict = {}
        self.total_steps = 0
        self.settlement = None


def _make_syncer(shared: _FakeSharedLayer) -> CrossInstanceSync:
    # Bypass __init__ prefill side effects by constructing manually after
    # SharedLayer would have returned historical hashes.
    syncer = CrossInstanceSync.__new__(CrossInstanceSync)
    syncer.shared = shared
    syncer.instance_id = "local"
    syncer.daemon = _StubDaemon()
    syncer.known_blocks = shared.all_block_hashes()
    syncer.injected_count = 0
    syncer.agreed_count = 0
    syncer.negated_count = 0
    syncer.deferred_count = 0
    syncer.peer_snet_states = {}
    syncer.peer_settlements = {}
    return syncer


def test_backlog_over_cap_is_retried_not_dropped() -> None:
    """Warm-start: >100 new peer blocks must all eventually be readable."""
    historical = {f"hist-{i:04d}": {"instance": "peer", "type": "snet_update"} for i in range(20)}
    shared = _FakeSharedLayer(historical)
    syncer = _make_syncer(shared)
    assert len(syncer.known_blocks) == 20

    backlog = {
        f"new-{i:04d}": {
            "instance": "peer",
            "type": "settlement_event",
            "settled_at_step": i,
            "cycle_edges": [],
            "step": i,
            "timestamp": float(i),
        }
        for i in range(150)
    }
    shared._blocks.update(backlog)

    first = syncer.sync()
    assert first == 0  # settlement_event is tracked, not injected into graph
    assert len(shared.read_attempts) == CrossInstanceSync._MAX_BLOCKS_PER_SYNC
    unread = set(backlog) - syncer.known_blocks
    assert len(unread) == 50, f"expected 50 deferred CIDs, got {len(unread)}"

    # Drain remaining backlog across subsequent syncs.
    while set(backlog) - syncer.known_blocks:
        before = len(syncer.known_blocks)
        syncer.sync()
        assert len(syncer.known_blocks) > before

    assert set(backlog) <= syncer.known_blocks
    peer_events = syncer.peer_settlements.get("peer", [])
    assert len(peer_events) == 150


def test_failed_read_is_retried() -> None:
    """IPFS timeout/None must not mark CID known forever."""
    shared = _FakeSharedLayer({})
    syncer = _make_syncer(shared)
    shared._blocks["cid-fail"] = {
        "instance": "peer",
        "type": "settlement_event",
        "settled_at_step": 1,
        "cycle_edges": [],
        "step": 1,
        "timestamp": 1.0,
    }
    shared.fail_once.add("cid-fail")

    assert syncer.sync() == 0
    assert "cid-fail" not in syncer.known_blocks
    assert syncer.peer_settlements.get("peer", []) == []

    assert syncer.sync() == 0
    assert "cid-fail" in syncer.known_blocks
    assert len(syncer.peer_settlements["peer"]) == 1


if __name__ == "__main__":
    test_backlog_over_cap_is_retried_not_dropped()
    test_failed_read_is_retried()
    print("PASS")
