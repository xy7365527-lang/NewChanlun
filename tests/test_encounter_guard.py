"""tests/test_encounter_guard.py — 偶遇记录 + DAG 守护测试"""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent.parent / "scripts"))

from encounter_guard import (
    CircularDependencyError,
    check_dag_integrity,
    check_encounter,
)


# ---------------------------------------------------------------------------
# 辅助函数
# ---------------------------------------------------------------------------

def _write_relations(tmp_path: Path, edges: list[dict]) -> Path:
    """写入 relations.jsonl 文件，返回路径。"""
    p = tmp_path / "relations.jsonl"
    with open(p, "w", encoding="utf-8") as f:
        for e in edges:
            f.write(json.dumps(e, ensure_ascii=False) + "\n")
    return p


# ---------------------------------------------------------------------------
# 偶遇检测测试
# ---------------------------------------------------------------------------

class TestCheckEncounterCritical:
    """构造包含环的 relations -> 新边是 critical -> 返回 True + topo_event 写入"""

    def test_critical_edge_returns_true(self, tmp_path):
        # A->B, B->C, C->A: 第三条边 C->A 形成环，是 critical
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "C", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:03Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        result = check_encounter(
            "C", "A",
            timestamp="2026-01-01T00:00:03Z",
            relations_path=p,
            topo_events_path=events_path,
        )
        assert result is True
        assert events_path.exists()

    def test_critical_edge_writes_event(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "C", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:03Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        check_encounter(
            "C", "A",
            timestamp="2026-01-01T00:00:03Z",
            relations_path=p,
            topo_events_path=events_path,
        )
        lines = events_path.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 1
        event = json.loads(lines[0])
        assert event["from"] == "C"
        assert event["to"] == "A"
        assert event["event_type"] == "new_critical"
        assert event["timestamp"] == "2026-01-01T00:00:03Z"
        assert "detected_at" in event


class TestCheckEncounterTree:
    """构造不包含环的 relations -> 新边是 tree -> 返回 False + 无 topo_event"""

    def test_tree_edge_returns_false(self, tmp_path):
        # A->B: 单条边，tree
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        result = check_encounter(
            "A", "B",
            relations_path=p,
            topo_events_path=events_path,
        )
        assert result is False
        assert not events_path.exists()

    def test_chain_no_cycle(self, tmp_path):
        # A->B, B->C: 线性链，两条边都是 tree
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        result = check_encounter(
            "B", "C",
            relations_path=p,
            topo_events_path=events_path,
        )
        assert result is False
        assert not events_path.exists()


class TestTopoEventFormat:
    """验证 topo_events.jsonl 的格式正确"""

    def test_event_is_valid_jsonl(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "X", "to": "Y", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "Y", "to": "X", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        check_encounter(
            "Y", "X",
            timestamp="2026-01-01T00:00:02Z",
            relations_path=p,
            topo_events_path=events_path,
        )
        content = events_path.read_text(encoding="utf-8").strip()
        for line in content.split("\n"):
            event = json.loads(line)
            assert isinstance(event, dict)
            assert set(event.keys()) == {"from", "to", "timestamp", "event_type", "detected_at"}

    def test_event_no_trailing_whitespace(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "X", "to": "Y", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "Y", "to": "X", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        check_encounter(
            "Y", "X",
            timestamp="2026-01-01T00:00:02Z",
            relations_path=p,
            topo_events_path=events_path,
        )
        raw = events_path.read_text(encoding="utf-8")
        # 每行以 \n 结尾，无多余空白
        assert raw.endswith("\n")
        for line in raw.split("\n"):
            if line:
                assert line == line.strip()


class TestTopoEventAppend:
    """多次 check_encounter -> 追加多条事件"""

    def test_multiple_events_appended(self, tmp_path):
        # 构造两个独立的 critical 边
        # A-B-C-A 三角形 + D-E-F-D 三角形
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "C", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:03Z"},
            {"from": "D", "to": "E", "relation": "references", "timestamp": "2026-01-01T00:00:04Z"},
            {"from": "E", "to": "F", "relation": "references", "timestamp": "2026-01-01T00:00:05Z"},
            {"from": "F", "to": "D", "relation": "references", "timestamp": "2026-01-01T00:00:06Z"},
        ])
        events_path = tmp_path / "topo_events.jsonl"

        r1 = check_encounter("C", "A", timestamp="t1", relations_path=p, topo_events_path=events_path)
        r2 = check_encounter("F", "D", timestamp="t2", relations_path=p, topo_events_path=events_path)

        assert r1 is True
        assert r2 is True

        lines = events_path.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) == 2
        assert json.loads(lines[0])["from"] == "C"
        assert json.loads(lines[1])["from"] == "F"


# ---------------------------------------------------------------------------
# DAG 守护测试
# ---------------------------------------------------------------------------

class TestDagNoCycle:
    """线性依赖链 -> 静默返回"""

    def test_linear_chain(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "depends_on", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        # 添加 C->D 不会创建环
        check_dag_integrity("C", "D", relations_path=p)

    def test_branching(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "A", "to": "C", "relation": "depends_on", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        # 添加 B->D 不会创建环
        check_dag_integrity("B", "D", relations_path=p)


class TestDagDirectCycle:
    """A->B 已存在，添加 B->A -> raise CircularDependencyError"""

    def test_direct_cycle(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:01Z"},
        ])
        with pytest.raises(CircularDependencyError) as exc_info:
            check_dag_integrity("B", "A", relations_path=p)
        assert exc_info.value.from_id == "B"
        assert exc_info.value.to_id == "A"
        assert "B" in exc_info.value.cycle_path
        assert "A" in exc_info.value.cycle_path


class TestDagIndirectCycle:
    """A->B->C 已存在，添加 C->A -> raise CircularDependencyError + 正确的 cycle_path"""

    def test_indirect_cycle(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "depends_on", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        with pytest.raises(CircularDependencyError) as exc_info:
            check_dag_integrity("C", "A", relations_path=p)
        err = exc_info.value
        assert err.from_id == "C"
        assert err.to_id == "A"
        # cycle_path 应包含完整环：C -> A -> B -> C
        assert err.cycle_path[0] == "C"
        assert "A" in err.cycle_path
        assert "B" in err.cycle_path

    def test_longer_indirect_cycle(self, tmp_path):
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "depends_on", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "C", "relation": "depends_on", "timestamp": "2026-01-01T00:00:02Z"},
            {"from": "C", "to": "D", "relation": "depends_on", "timestamp": "2026-01-01T00:00:03Z"},
        ])
        with pytest.raises(CircularDependencyError) as exc_info:
            check_dag_integrity("D", "A", relations_path=p)
        err = exc_info.value
        assert err.from_id == "D"
        assert err.to_id == "A"


class TestDagEmptyGraph:
    """空图 -> 静默返回"""

    def test_empty_file(self, tmp_path):
        p = tmp_path / "relations.jsonl"
        p.write_text("", encoding="utf-8")
        check_dag_integrity("A", "B", relations_path=p)

    def test_no_file(self, tmp_path):
        p = tmp_path / "nonexistent.jsonl"
        check_dag_integrity("A", "B", relations_path=p)


class TestDagSelfLoop:
    """添加 A->A -> raise CircularDependencyError"""

    def test_self_loop(self, tmp_path):
        p = tmp_path / "relations.jsonl"
        p.write_text("", encoding="utf-8")
        with pytest.raises(CircularDependencyError) as exc_info:
            check_dag_integrity("A", "A", relations_path=p)
        err = exc_info.value
        assert err.from_id == "A"
        assert err.to_id == "A"
        assert "A" in str(err)


# ---------------------------------------------------------------------------
# DAG 守护与 references 边的隔离
# ---------------------------------------------------------------------------

class TestDagIgnoresReferences:
    """DAG 守护只检查 depends_on 边，忽略 references 边"""

    def test_references_not_checked(self, tmp_path):
        # references 形成环，但 depends_on 子图无环
        p = _write_relations(tmp_path, [
            {"from": "A", "to": "B", "relation": "references", "timestamp": "2026-01-01T00:00:01Z"},
            {"from": "B", "to": "A", "relation": "references", "timestamp": "2026-01-01T00:00:02Z"},
        ])
        # depends_on 子图为空，不应报错
        check_dag_integrity("A", "B", relations_path=p)
