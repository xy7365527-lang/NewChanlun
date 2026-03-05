"""tests for scripts/tension_sublation.py — 张力扬弃（Aufhebung）处理器。"""
from __future__ import annotations

import json
import os
import textwrap

import pytest
import yaml

from scripts.tension_sublation import (
    derive_sublation_event,
    generate_elevated_gangmu_candidates,
    load_tension_audit,
    scan_historical_sublation_events,
    scan_ongoing_tensions,
    scan_sublation_events,
    update_tension_audit_with_sublation,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════

RESOLVED_TENSION = {
    "from": "007",
    "to": "008",
    "description": "趋势是独立实体 vs 唯一有效的就是中枢",
    "classification": "resolved",
    "resolved_by": "010",
    "evidence": "010号将007/008/009三条谱系的表面矛盾综合为构造层/分类层二层架构",
}

ONGOING_TENSION = {
    "from": "139",
    "to": "原则0的ESC权",
    "description": "ESC/分类权分离提案",
    "classification": "ongoing",
    "notes": "选择类张力",
}

HISTORICAL_TENSION = {
    "from": "062",
    "to": "065",
    "description": "异质性作为Sinthome vs 六条连续流形张力",
    "classification": "historical",
    "valid_until": "068",
    "evidence": "065被068否定后张力自动失效",
}


MINIMAL_AUDIT = {
    "tensions": [
        RESOLVED_TENSION,
        ONGOING_TENSION,
        HISTORICAL_TENSION,
    ],
    "summary": {
        "total_tensions": 3,
        "resolved": 1,
        "historical": 1,
        "ongoing": 1,
    },
}


def _write_genealogy(root, gid, title, downstream=None):
    """Helper: 写入一条谱系条目。"""
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    os.makedirs(settled_dir, exist_ok=True)
    content = textwrap.dedent(f"""\
        ---
        id: '{gid}'
        title: {title}
        status: settled
        ---

        # {gid}号：{title}

        ## downstream_implications

    """)
    if downstream:
        for i, impl in enumerate(downstream, 1):
            content += f"    {i}. {impl}\n"
    path = os.path.join(settled_dir, f"{gid}-test.md")
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)


@pytest.fixture()
def audit_root(tmp_path):
    """创建包含 tension-audit.yaml 和最小谱系的临时项目根。"""
    chanlun = tmp_path / ".chanlun"
    chanlun.mkdir()
    audit_path = chanlun / "tension-audit.yaml"
    with open(audit_path, "w", encoding="utf-8") as f:
        yaml.dump(MINIMAL_AUDIT, f, allow_unicode=True, default_flow_style=False)

    # 写入 resolver 谱系
    _write_genealogy(
        str(tmp_path), "010", "趋势与中枢的二层架构",
        downstream=[
            "构造层和分类层的分离使得后续级别定义可以独立演化",
            "二层架构为009号盘整双重性提供了存在论位置",
        ],
    )
    return str(tmp_path)


@pytest.fixture()
def empty_root(tmp_path):
    """空项目根——无 tension-audit.yaml。"""
    chanlun = tmp_path / ".chanlun"
    chanlun.mkdir()
    return str(tmp_path)


# ═══════════════════════════════════════════════════════════════
# Tests: derive_sublation_event
# ═══════════════════════════════════════════════════════════════

class TestDeriveSublationEvent:
    def test_resolved_tension_produces_event(self, audit_root):
        event = derive_sublation_event(RESOLVED_TENSION, audit_root)
        assert event is not None
        assert event["type"] == "sublation"
        assert event["tension_from"] == "007"
        assert event["tension_to"] == "008"
        assert event["resolved_by"] == "010"

    def test_negated_field(self, audit_root):
        event = derive_sublation_event(RESOLVED_TENSION, audit_root)
        negated = event["negated"]
        assert "007↔008" in negated["tension_edge"]
        assert "010" in negated["reason"]

    def test_preserved_field(self, audit_root):
        event = derive_sublation_event(RESOLVED_TENSION, audit_root)
        preserved = event["preserved"]
        assert "010号" in preserved["insight"]
        assert preserved["from_entry"] == "007"
        assert preserved["to_entry"] == "008"

    def test_elevated_field_with_downstream(self, audit_root):
        event = derive_sublation_event(RESOLVED_TENSION, audit_root)
        elevated = event["elevated"]
        assert elevated["level"] == "cross_layer"
        assert len(elevated["implications"]) == 2
        assert "构造层" in elevated["implications"][0]

    def test_elevated_field_without_downstream(self, empty_root):
        """resolver 无下游推论时 -> structural_insight。"""
        event = derive_sublation_event(RESOLVED_TENSION, empty_root)
        assert event is not None
        elevated = event["elevated"]
        assert elevated["level"] == "structural_insight"

    def test_ongoing_returns_none(self, audit_root):
        event = derive_sublation_event(ONGOING_TENSION, audit_root)
        assert event is None

    def test_historical_returns_none(self, audit_root):
        event = derive_sublation_event(HISTORICAL_TENSION, audit_root)
        assert event is None

    def test_timestamp_present(self, audit_root):
        event = derive_sublation_event(RESOLVED_TENSION, audit_root)
        assert "timestamp" in event
        assert "T" in event["timestamp"]  # ISO format


# ═══════════════════════════════════════════════════════════════
# Tests: scan functions
# ═══════════════════════════════════════════════════════════════

class TestScanFunctions:
    def test_scan_sublation_events(self, audit_root):
        events = scan_sublation_events(audit_root)
        assert len(events) == 1
        assert events[0]["tension_from"] == "007"

    def test_scan_historical_sublation_events(self, audit_root):
        events = scan_historical_sublation_events(audit_root)
        assert len(events) == 1
        assert events[0]["type"] == "historical_sublation"
        assert events[0]["valid_until"] == "068"

    def test_scan_ongoing_tensions(self, audit_root):
        ongoing = scan_ongoing_tensions(audit_root)
        assert len(ongoing) == 1
        assert ongoing[0]["from"] == "139"

    def test_empty_root_returns_empty(self, empty_root):
        assert scan_sublation_events(empty_root) == []
        assert scan_historical_sublation_events(empty_root) == []
        assert scan_ongoing_tensions(empty_root) == []


# ═══════════════════════════════════════════════════════════════
# Tests: gangmu candidate generation
# ═══════════════════════════════════════════════════════════════

class TestGangmuCandidates:
    def test_cross_layer_produces_candidate(self, audit_root):
        events = scan_sublation_events(audit_root)
        candidates = generate_elevated_gangmu_candidates(events)
        assert len(candidates) == 1
        assert candidates[0]["resolved_by"] == "010"
        assert len(candidates[0]["implications"]) == 2

    def test_structural_insight_no_candidate(self, empty_root):
        """structural_insight 级别不注入纲目。"""
        events = scan_sublation_events(empty_root)
        candidates = generate_elevated_gangmu_candidates(events)
        assert candidates == []


# ═══════════════════════════════════════════════════════════════
# Tests: update_tension_audit_with_sublation
# ═══════════════════════════════════════════════════════════════

class TestUpdateTensionAudit:
    def test_writes_sublation_to_resolved_tensions(self, audit_root):
        events = scan_sublation_events(audit_root)
        result = update_tension_audit_with_sublation(audit_root, events)
        assert result["updated_count"] == 1
        assert result["total_events"] == 1

        # 验证文件已写入
        audit_path = os.path.join(audit_root, ".chanlun", "tension-audit.yaml")
        with open(audit_path, encoding="utf-8") as f:
            content = f.read()
        assert "sublation" in content

        # 重新加载验证结构
        data = load_tension_audit(audit_root)
        resolved = [t for t in data["tensions"] if t.get("classification") == "resolved"]
        assert len(resolved) == 1
        assert "sublation" in resolved[0]
        assert "negated" in resolved[0]["sublation"]
        assert "preserved" in resolved[0]["sublation"]
        assert "elevated" in resolved[0]["sublation"]

    def test_ongoing_unchanged_after_update(self, audit_root):
        events = scan_sublation_events(audit_root)
        update_tension_audit_with_sublation(audit_root, events)
        data = load_tension_audit(audit_root)
        ongoing = [t for t in data["tensions"] if t.get("classification") == "ongoing"]
        assert len(ongoing) == 1
        assert "sublation" not in ongoing[0]

    def test_empty_events_no_write(self, audit_root):
        result = update_tension_audit_with_sublation(audit_root, [])
        assert result["updated_count"] == 0

    def test_summary_updated(self, audit_root):
        events = scan_sublation_events(audit_root)
        update_tension_audit_with_sublation(audit_root, events)
        data = load_tension_audit(audit_root)
        assert data["summary"]["sublation_events"] == 1


# ═══════════════════════════════════════════════════════════════
# Tests: historical sublation event structure
# ═══════════════════════════════════════════════════════════════

class TestHistoricalSublation:
    def test_historical_negated_references_valid_until(self, audit_root):
        events = scan_historical_sublation_events(audit_root)
        event = events[0]
        assert "068" in event["negated"]["reason"]

    def test_historical_preserved_is_historical_record(self, audit_root):
        events = scan_historical_sublation_events(audit_root)
        assert events[0]["preserved"]["historical_record"] is True

    def test_historical_elevated_is_paradigm_shift(self, audit_root):
        events = scan_historical_sublation_events(audit_root)
        assert events[0]["elevated"]["level"] == "paradigm_shift"


# ═══════════════════════════════════════════════════════════════
# Tests: load_tension_audit edge cases
# ═══════════════════════════════════════════════════════════════

class TestLoadTensionAudit:
    def test_nonexistent_file(self, empty_root):
        result = load_tension_audit(empty_root)
        assert result == {"tensions": [], "summary": {}}

    def test_valid_file(self, audit_root):
        result = load_tension_audit(audit_root)
        assert len(result["tensions"]) == 3
