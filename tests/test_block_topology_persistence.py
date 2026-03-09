"""Tests for block_topology_persistence — BlockTopologyWriter + loader."""

import json
import sys
import tempfile
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "topological-computation"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus
from block_topology_persistence import (
    BlockTopologyWriter,
    load_graph_from_block_topology,
    rebuild_block_topology_from_jsonl,
)


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
    """Test BlockTopologyWriter writes blocks and jsonl backup."""

    def test_append_vertex_creates_block(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        v = _make_vertex("v1", content="test vertex")
        writer.append_vertex(v)
        writer.close()

        # Block file created
        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1
        blk = json.loads(blocks[0].read_text(encoding="utf-8"))
        assert blk["content"]["event_type"] == "vertex"
        assert blk["content"]["vertex_id"] == "v1"
        assert blk["type"] == "event"

        # jsonl backup written
        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec["type"] == "vertex"
        assert rec["id"] == "v1"

    def test_append_edge_creates_block(self, tmp_bt, tmp_jsonl):
        writer = BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl)
        writer.open()
        writer.append_vertex(_make_vertex("a"))
        writer.append_vertex(_make_vertex("b"))
        e = _make_edge("a", "b")
        writer.append_edge(e)
        writer.close()

        blocks = list((tmp_bt / "blocks").iterdir())
        # 2 vertex blocks + 1 edge block
        assert len(blocks) == 3

        # jsonl should have 3 lines
        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 3

    def test_append_operation(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_operation(42, "fold", {"position": "v1", "beta_1_before": 3, "beta_1_after": 2, "blocked": False})

        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1
        blk = json.loads(blocks[0].read_text(encoding="utf-8"))
        assert blk["content"]["event_type"] == "operation"
        assert blk["content"]["step"] == 42
        assert blk["content"]["operation"] == "fold"

    def test_append_vertex_status(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex_status("v1", "contested", step=10)

        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1
        blk = json.loads(blocks[0].read_text(encoding="utf-8"))
        assert blk["content"]["event_type"] == "vertex_status"
        assert blk["content"]["vertex_id"] == "v1"

    def test_append_merge(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_merge("keep_v", "remove_v", step=20)

        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1
        blk = json.loads(blocks[0].read_text(encoding="utf-8"))
        assert blk["content"]["event_type"] == "merge"
        assert blk["content"]["keep"] == "keep_v"
        assert blk["content"]["remove"] == "remove_v"

    def test_append_settlement(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        edges = [["a", "b", "dependency"], ["b", "c", "negation"]]
        writer.append_settlement(step=30, cycle_edges=edges)

        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1
        blk = json.loads(blocks[0].read_text(encoding="utf-8"))
        assert blk["content"]["event_type"] == "settlement"
        assert blk["content"]["step"] == 30

    def test_context_manager(self, tmp_bt, tmp_jsonl):
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("cm_test"))
        # After context exit, file should be closed
        assert writer._jsonl_file is None
        # But block should exist
        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 1


class TestLoadGraphFromBlockTopology:
    """Test loading Graph from block topology blocks."""

    def test_empty_directory(self, tmp_bt):
        graph, ops = load_graph_from_block_topology(tmp_bt)
        assert len(graph.active_vertex_ids()) == 0
        assert ops == []

    def test_load_vertices_and_edges(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex(_make_vertex("x", content="node X"))
        writer.append_vertex(_make_vertex("y", content="node Y"))
        writer.append_edge(_make_edge("x", "y"))

        graph, ops = load_graph_from_block_topology(tmp_bt)
        assert sorted(graph.active_vertex_ids()) == ["x", "y"]
        assert len(graph.edges) == 1
        assert graph.vertex("x").content == "node X"

    def test_load_vertex_status_change(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex(_make_vertex("z", content="node Z"))
        writer.append_vertex_status("z", "contested", step=5)

        graph, _ = load_graph_from_block_topology(tmp_bt)
        assert graph.vertex("z").status == VertexStatus.CONTESTED

    def test_load_merge(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex(_make_vertex("a"))
        writer.append_vertex(_make_vertex("b"))
        writer.append_edge(_make_edge("b", "a"))
        writer.append_merge("a", "b", step=10)

        graph, _ = load_graph_from_block_topology(tmp_bt)
        assert graph.vertex("b").status == VertexStatus.FOLDED
        assert "b" not in graph.active_vertex_ids()
        assert "a" in graph.active_vertex_ids()

    def test_load_operations(self, tmp_bt):
        writer = BlockTopologyWriter(bt_base=tmp_bt)
        writer.append_vertex(_make_vertex("v1"))
        writer.append_operation(1, "sublate", {"position": "v1"})
        writer.append_settlement(2, [["v1", "v1", "dependency"]])

        _, ops = load_graph_from_block_topology(tmp_bt)
        assert len(ops) == 2
        assert ops[0]["event_type"] == "operation"
        assert ops[1]["event_type"] == "settlement"


class TestRebuildFromJsonl:
    """Test crash recovery: rebuild block topology from jsonl."""

    def test_rebuild_from_jsonl(self, tmp_bt, tmp_jsonl):
        # Write some jsonl records
        records = [
            {"type": "vertex", "id": "r1", "status": "active", "content": "rebuilt 1", "created_at": 0},
            {"type": "vertex", "id": "r2", "status": "active", "content": "rebuilt 2", "created_at": 0},
            {"type": "edge", "source": "r1", "target": "r2", "edge_type": "dependency", "created_at": 0},
            {"type": "operation", "step": 1, "operation": "fold", "position": "r1"},
            {"type": "vertex_status", "id": "r1", "status": "contested", "step": 2},
            {"type": "merge", "keep": "r1", "remove": "r2", "step": 3},
            {"type": "settlement", "step": 4, "edges": []},
        ]
        with open(tmp_jsonl, "w", encoding="utf-8") as f:
            for r in records:
                f.write(json.dumps(r) + "\n")

        count = rebuild_block_topology_from_jsonl(tmp_jsonl, tmp_bt)
        assert count == 7

        # Verify blocks were created
        blocks = list((tmp_bt / "blocks").iterdir())
        assert len(blocks) == 7

    def test_rebuild_nonexistent_jsonl(self, tmp_bt, tmp_path):
        count = rebuild_block_topology_from_jsonl(tmp_path / "nope.jsonl", tmp_bt)
        assert count == 0


class TestRoundTrip:
    """Test write-then-load round trip."""

    def test_full_round_trip(self, tmp_bt, tmp_jsonl):
        # Write via BlockTopologyWriter
        with BlockTopologyWriter(bt_base=tmp_bt, jsonl_backup_path=tmp_jsonl) as writer:
            writer.append_vertex(_make_vertex("rt1", content="round trip 1"))
            writer.append_vertex(_make_vertex("rt2", content="round trip 2"))
            writer.append_vertex(_make_vertex("rt3", content="round trip 3"))
            writer.append_edge(_make_edge("rt1", "rt2"))
            writer.append_edge(_make_edge("rt2", "rt3", EdgeType.NEGATION))
            writer.append_vertex_status("rt1", "contested", step=1)
            writer.append_operation(2, "negate", {"position": "rt1"})

        # Load from block topology
        graph, ops = load_graph_from_block_topology(tmp_bt)
        assert sorted(graph.active_vertex_ids()) == ["rt1", "rt2", "rt3"]
        assert len(graph.edges) == 2
        assert graph.vertex("rt1").status == VertexStatus.CONTESTED
        assert graph.vertex("rt1").content == "round trip 1"
        assert len(ops) == 1
        assert ops[0]["operation"] == "negate"

        # Verify jsonl backup also contains all records
        lines = tmp_jsonl.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 7  # 3 vertices + 2 edges + 1 status + 1 operation
