"""Tests for block_topology_persistence — BlockTopologyWriter + JSONL + Merkle DAG hash."""

import hashlib
import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "topological-computation"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus
from block_topology_persistence import (
    BlockTopologyWriter,
    load_graph_from_block_topology,
    rebuild_block_topology_from_jsonl,
    verify_jsonl,
)
from persistence import PersistentKFull


@pytest.fixture
def tmp_bt(tmp_path):
    """Create a temporary block topology directory."""
    bt_base = tmp_path / "block-topology"
    bt_base.mkdir()
    (bt_base / "blocks").mkdir()
    return bt_base


@pytest.fixture
def tmp_jsonl(tmp_path):
    """Create a temporary jsonl backup path."""
    return tmp_path / "k_full.jsonl"


def _make_vertex(vid, content=None, status=VertexStatus.ACTIVE):
    return Vertex(id=vid, status=status, content=content, created_at=0)


def _make_edge(src, tgt, edge_type=EdgeType.DEPENDENCY):
    return Edge(source=src, target=tgt, edge_type=edge_type, created_at=0)


class TestBlockTopologyWriter:
    """Test BlockTopologyWriter writes JSONL with Merkle DAG hashes."""

    def test_append_vertex_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        v = _make_vertex("v1", content="test vertex")
        writer.append_vertex(v)
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "vertex"
        assert rec["id"] == "v1"
        assert "block_hash" in rec

    def test_append_edge_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        writer.append_vertex(_make_vertex("a"))
        writer.append_vertex(_make_vertex("b"))
        e = _make_edge("a", "b")
        writer.append_edge(e)
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 3

    def test_append_operation_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        writer.append_operation(42, "fold", {"position": "v1", "beta_1_before": 3, "beta_1_after": 2, "blocked": False})
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "operation"
        assert rec["step"] == 42
        assert rec["operation"] == "fold"
        assert "block_hash" in rec

    def test_append_vertex_status_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        writer.append_vertex_status("v1", "contested", step=10)
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "vertex_status"
        assert rec["id"] == "v1"
        assert "block_hash" in rec

    def test_append_merge_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        writer.append_merge("keep_v", "remove_v", step=20)
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "merge"
        assert rec["keep"] == "keep_v"
        assert rec["remove"] == "remove_v"
        assert "block_hash" in rec

    def test_append_settlement_writes_jsonl(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        edges = [["a", "b", "dependency"], ["b", "c", "negation"]]
        writer.append_settlement(step=30, cycle_edges=edges)
        writer.close()

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "settlement"
        assert rec["step"] == 30
        assert "block_hash" in rec

    def test_context_manager(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("cm_test"))
        assert writer._jsonl_file is None

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1

    def test_no_write_without_jsonl_file(self, tmp_bt):
        """Writer without jsonl_backup_path should not crash."""
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex(_make_vertex("no_jsonl"))


class TestMerkleDAGHash:
    """Test Merkle DAG SHA-256 hashing in _write_event."""

    def test_block_hash_present(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("h1", content="hash test"))

        rec = json.loads(tmp_jsonl.read_text(encoding="utf-8").strip())
        assert "block_hash" in rec
        assert len(rec["block_hash"]) == 64  # SHA-256 hex digest

    def test_block_hash_is_correct(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("h2", content="verify hash"))

        rec = json.loads(tmp_jsonl.read_text(encoding="utf-8").strip())
        stored_hash = rec.pop("block_hash")
        canonical = json.dumps(rec, sort_keys=True, ensure_ascii=False)
        expected = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
        assert stored_hash == expected

    def test_block_hash_deterministic(self, tmp_bt, tmp_jsonl):
        """Same record content produces same hash."""
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("det", content="deterministic"))

        rec = json.loads(tmp_jsonl.read_text(encoding="utf-8").strip())

        # Recompute independently
        rec_without_hash = {k: v for k, v in rec.items() if k != "block_hash"}
        canonical = json.dumps(rec_without_hash, sort_keys=True, ensure_ascii=False)
        expected = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
        assert rec["block_hash"] == expected

    def test_different_records_different_hashes(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("a", content="alpha"))
            writer.append_vertex(_make_vertex("b", content="beta"))

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        h1 = json.loads(lines[0])["block_hash"]
        h2 = json.loads(lines[1])["block_hash"]
        assert h1 != h2

    def test_unicode_content_hashed_correctly(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("zh", content="缠论测试"))

        rec = json.loads(tmp_jsonl.read_text(encoding="utf-8").strip())
        stored_hash = rec.pop("block_hash")
        canonical = json.dumps(rec, sort_keys=True, ensure_ascii=False)
        expected = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
        assert stored_hash == expected


class TestVerifyJsonl:
    """Test verify_jsonl integrity checking."""

    def test_verify_valid_file(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("v1", content="valid"))
            writer.append_vertex(_make_vertex("v2", content="also valid"))
            writer.append_edge(_make_edge("v1", "v2"))

        assert verify_jsonl(tmp_jsonl) is True

    def test_verify_nonexistent_file(self, tmp_path):
        assert verify_jsonl(tmp_path / "no_such_file.jsonl") is True

    def test_verify_empty_file(self, tmp_jsonl):
        tmp_jsonl.write_text("", encoding="utf-8")
        assert verify_jsonl(tmp_jsonl) is True

    def test_verify_tampered_content(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("v1", content="original"))

        # Tamper with content
        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        rec = json.loads(lines[0])
        rec["content"] = "tampered"
        tmp_jsonl.write_text(json.dumps(rec) + "\n", encoding="utf-8")

        assert verify_jsonl(tmp_jsonl) is False

    def test_verify_tampered_hash(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("v1", content="original"))

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        rec = json.loads(lines[0])
        rec["block_hash"] = "0" * 64
        tmp_jsonl.write_text(json.dumps(rec) + "\n", encoding="utf-8")

        assert verify_jsonl(tmp_jsonl) is False

    def test_verify_missing_hash(self, tmp_jsonl):
        rec = {"type": "vertex", "id": "v1", "content": "no hash"}
        tmp_jsonl.write_text(json.dumps(rec) + "\n", encoding="utf-8")

        assert verify_jsonl(tmp_jsonl) is False

    def test_verify_malformed_json(self, tmp_jsonl):
        tmp_jsonl.write_text("not json at all\n", encoding="utf-8")
        assert verify_jsonl(tmp_jsonl) is False

    def test_verify_multiple_records_one_bad(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("v1", content="good"))
            writer.append_vertex(_make_vertex("v2", content="good too"))

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        rec = json.loads(lines[1])
        rec["content"] = "evil"
        lines[1] = json.dumps(rec)
        tmp_jsonl.write_text("\n".join(lines) + "\n", encoding="utf-8")

        assert verify_jsonl(tmp_jsonl) is False


class TestLoadGraphFromBlockTopology:
    """Test loading Graph from JSONL (preferred) or legacy block files."""

    def test_empty_directory(self, tmp_bt):
        graph, ops = load_graph_from_block_topology(tmp_bt)
        assert len(graph.active_vertex_ids()) == 0
        assert ops == []

    def test_jsonl_wins_over_stale_legacy_blocks(self, tmp_bt, tmp_jsonl):
        """Leftover per-file blocks must not shadow JSONL-native persist."""
        _write_legacy_vertex_block(tmp_bt, "stale_only", "stale leftover")
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("fresh_only", content="jsonl native"))

        graph, _ = load_graph_from_block_topology(tmp_bt, jsonl_path=tmp_jsonl)
        vids = set(graph.active_vertex_ids())
        assert "fresh_only" in vids
        assert "stale_only" not in vids

    def test_snapshot_wins_over_stale_legacy_blocks(self, tmp_bt, tmp_jsonl):
        graph = Graph()
        graph.add_vertex(_make_vertex("snap_v", content="from snapshot"))
        snap_path = PersistentKFull.snapshot_path_for(tmp_jsonl)
        PersistentKFull.dump_snapshot(graph, snap_path)
        _write_legacy_vertex_block(tmp_bt, "stale_only", "stale leftover")

        recovered, _ = load_graph_from_block_topology(tmp_bt, jsonl_path=tmp_jsonl)
        vids = set(recovered.active_vertex_ids())
        assert "snap_v" in vids
        assert "stale_only" not in vids

    def test_small_legacy_dir_still_loads_without_jsonl(self, tmp_bt):
        _write_legacy_vertex_block(tmp_bt, "legacy_v", "only legacy")
        graph, _ = load_graph_from_block_topology(tmp_bt)
        assert "legacy_v" in set(graph.active_vertex_ids())

    def test_inode_bomb_legacy_dir_is_skipped(self, tmp_bt, monkeypatch):
        import block_topology_persistence as btp
        monkeypatch.setattr(btp, "LEGACY_BLOCK_FILE_CAP", 2)
        for i in range(3):
            _write_legacy_vertex_block(tmp_bt, f"bomb_{i}", f"leftover {i}")
        graph, ops = load_graph_from_block_topology(tmp_bt)
        assert graph.active_vertex_ids() == []
        assert ops == []


def _write_legacy_vertex_block(bt_base: Path, vertex_id: str, content: str) -> None:
    """Write one pre-JSONL per-file block (the leftover recovery poison)."""
    blocks = bt_base / "blocks"
    blocks.mkdir(parents=True, exist_ok=True)
    payload = {
        "id": vertex_id,
        "type": "event",
        "timestamp": "2026-03-13T00:00:00+00:00",
        "source": "cc",
        "content": {
            "event_type": "vertex",
            "domain": "graph",
            "vertex_id": vertex_id,
            "status": "active",
            "content": content,
            "created_at": 0,
        },
    }
    (blocks / f"{vertex_id}.json").write_text(json.dumps(payload), encoding="utf-8")


class TestRebuildFromJsonl:
    """Test rebuild_block_topology_from_jsonl (no-op since JSONL migration)."""

    def test_rebuild_returns_zero(self, tmp_bt, tmp_jsonl):
        records = [
            {"type": "vertex", "id": "r1", "status": "active", "content": "rebuilt 1", "created_at": 0},
        ]
        with open(tmp_jsonl, "w", encoding="utf-8") as f:
            for r in records:
                f.write(json.dumps(r) + "\n")

        count = rebuild_block_topology_from_jsonl(tmp_jsonl, tmp_bt)
        assert count == 0

    def test_rebuild_nonexistent_jsonl(self, tmp_bt, tmp_path):
        count = rebuild_block_topology_from_jsonl(tmp_path / "nope.jsonl", tmp_bt)
        assert count == 0


class TestWriteAndVerifyRoundTrip:
    """Test write → verify round trip."""

    def test_full_round_trip_with_verification(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("rt1", content="round trip 1"))
            writer.append_vertex(_make_vertex("rt2", content="round trip 2"))
            writer.append_vertex(_make_vertex("rt3", content="round trip 3"))
            writer.append_edge(_make_edge("rt1", "rt2"))
            writer.append_edge(_make_edge("rt2", "rt3", EdgeType.NEGATION))
            writer.append_vertex_status("rt1", "contested", step=1)
            writer.append_operation(2, "negate", {"position": "rt1"})

        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 7  # 3 vertices + 2 edges + 1 status + 1 operation

        # All records should have block_hash
        for line in lines:
            rec = json.loads(line)
            assert "block_hash" in rec

        # Verify integrity
        assert verify_jsonl(tmp_jsonl) is True
