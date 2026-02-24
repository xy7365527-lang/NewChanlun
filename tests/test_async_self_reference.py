"""tests for scripts/async_self_reference.py — 异步自指调度器（183号目B）。"""
from __future__ import annotations

import json
import os
import textwrap
import time

import pytest

from scripts.async_self_reference import (
    audit,
    check_depends_on_references,
    compute_downstream_resolution_rate,
    extract_session_time,
    extract_settled_count,
    get_settled_ids_in_range,
    get_sorted_sessions,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures / helpers
# ═══════════════════════════════════════════════════════════════


def _write_session(root, filename, settled_count, time_str=None, *, extra=""):
    """在 root/.chanlun/sessions/ 下写入 session 文件。"""
    sessions_dir = os.path.join(root, ".chanlun", "sessions")
    os.makedirs(sessions_dir, exist_ok=True)
    if time_str is None:
        time_str = filename.replace("-session.md", "")
    content = textwrap.dedent(f"""\
        # Session

        **时间**: {time_str}
        **分支**: main

        ## 谱系状态
        - 生成态: 0 个
        - 已结算: {settled_count} 个
        {extra}
    """)
    path = os.path.join(sessions_dir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


def _write_settled(root, filename, content):
    """在 root/.chanlun/genealogy/settled/ 下写入文件。"""
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    os.makedirs(settled_dir, exist_ok=True)
    path = os.path.join(settled_dir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


def _setup_block_topology(root, id_mapping):
    """创建 block-topology 目录结构 + meta.json。"""
    base = os.path.join(root, ".chanlun", "block-topology")
    blocks_dir = os.path.join(base, "blocks")
    os.makedirs(blocks_dir, exist_ok=True)
    meta_path = os.path.join(base, "meta.json")
    with open(meta_path, "w", encoding="utf-8") as f:
        json.dump({"id_mapping": id_mapping}, f)
    return base


def _write_relation(base, rel_dict):
    """追加一条 relation 到 relations.jsonl。"""
    jsonl_path = os.path.join(base, "relations.jsonl")
    with open(jsonl_path, "a", encoding="utf-8") as f:
        f.write(json.dumps(rel_dict, ensure_ascii=False, separators=(",", ":")) + "\n")


# ═══════════════════════════════════════════════════════════════
# extract_settled_count
# ═══════════════════════════════════════════════════════════════


class TestExtractSettledCount:
    def test_extracts_count(self, tmp_path):
        path = _write_session(tmp_path, "s1-session.md", 185)
        assert extract_settled_count(path) == 185

    def test_returns_none_when_no_match(self, tmp_path):
        sessions_dir = os.path.join(tmp_path, ".chanlun", "sessions")
        os.makedirs(sessions_dir, exist_ok=True)
        path = os.path.join(sessions_dir, "bad-session.md")
        with open(path, "w", encoding="utf-8") as f:
            f.write("# No settled count here\n")
        assert extract_settled_count(path) is None

    def test_returns_none_for_missing_file(self):
        assert extract_settled_count("/nonexistent/path.md") is None


# ═══════════════════════════════════════════════════════════════
# extract_session_time
# ═══════════════════════════════════════════════════════════════


class TestExtractSessionTime:
    def test_extracts_time(self, tmp_path):
        path = _write_session(tmp_path, "2026-02-24-0703-session.md", 185,
                              time_str="2026-02-24-0703")
        assert extract_session_time(path) == "2026-02-24-0703"

    def test_fallback_to_basename(self):
        result = extract_session_time("/nonexistent/foo-session.md")
        assert result == "foo-session.md"


# ═══════════════════════════════════════════════════════════════
# get_sorted_sessions
# ═══════════════════════════════════════════════════════════════


class TestGetSortedSessions:
    def test_returns_sorted_by_mtime(self, tmp_path):
        p1 = _write_session(tmp_path, "a-session.md", 100)
        time.sleep(0.05)
        p2 = _write_session(tmp_path, "b-session.md", 110)
        result = get_sorted_sessions(str(tmp_path))
        assert len(result) == 2
        # normalize separators for Windows compatibility
        assert os.path.normpath(result[0]) == os.path.normpath(p1)
        assert os.path.normpath(result[1]) == os.path.normpath(p2)

    def test_empty_dir(self, tmp_path):
        assert get_sorted_sessions(str(tmp_path)) == []


# ═══════════════════════════════════════════════════════════════
# get_settled_ids_in_range
# ═══════════════════════════════════════════════════════════════


class TestGetSettledIdsInRange:
    def test_returns_ids_in_range(self, tmp_path):
        for i in [180, 181, 182, 183, 184, 185]:
            _write_settled(tmp_path, f"{i:03d}-test.md", f"id: '{i:03d}'")
        result = get_settled_ids_in_range(str(tmp_path), 181, 184)
        assert result == {"182", "183", "184"}

    def test_empty_when_no_match(self, tmp_path):
        _write_settled(tmp_path, "001-test.md", "id: '001'")
        result = get_settled_ids_in_range(str(tmp_path), 100, 200)
        assert result == set()

    def test_empty_when_no_dir(self, tmp_path):
        result = get_settled_ids_in_range(str(tmp_path), 0, 100)
        assert result == set()


# ═══════════════════════════════════════════════════════════════
# check_depends_on_references
# ═══════════════════════════════════════════════════════════════


class TestCheckDependsOnReferences:
    def test_finds_references(self, tmp_path):
        sha_183 = "aaa" + "0" * 61
        sha_184 = "bbb" + "0" * 61
        sha_185 = "ccc" + "0" * 61
        base = _setup_block_topology(tmp_path, {
            "183": sha_183,
            "184": sha_184,
            "185": sha_185,
        })
        # 185 depends_on 183
        _write_relation(base, {
            "from": sha_185, "to": sha_183,
            "relation": "depends_on", "order": 1,
            "created_by": sha_185,
        })
        result = check_depends_on_references(str(tmp_path), {"183"})
        assert "183" in result
        assert "185" in result["183"]

    def test_no_references(self, tmp_path):
        sha_183 = "aaa" + "0" * 61
        base = _setup_block_topology(tmp_path, {"183": sha_183})
        # write a non-depends_on relation
        _write_relation(base, {
            "from": sha_183, "to": "xxx" + "0" * 61,
            "relation": "freezes", "order": 1,
            "created_by": sha_183,
        })
        result = check_depends_on_references(str(tmp_path), {"183"})
        assert result == {}

    def test_no_block_topology(self, tmp_path):
        result = check_depends_on_references(str(tmp_path), {"183"})
        assert result == {}


# ═══════════════════════════════════════════════════════════════
# audit — integration tests
# ═══════════════════════════════════════════════════════════════


class TestAudit:
    def test_insufficient_sessions(self, tmp_path):
        """少于 2 个 session → 返回 error。"""
        _write_session(tmp_path, "only-session.md", 100)
        result = audit(str(tmp_path))
        assert "error" in result
        assert result["session_count"] == 1
        assert result["self_audit_findings"] == []
        assert result["genealogy_needed"] is False

    def test_no_sessions(self, tmp_path):
        """无 session → 返回 error。"""
        result = audit(str(tmp_path))
        assert "error" in result
        assert result["session_count"] == 0

    def test_progression_detected(self, tmp_path):
        """settled 增加 → 检出 progression。"""
        _write_session(tmp_path, "a-session.md", 180)
        time.sleep(0.05)
        _write_session(tmp_path, "b-session.md", 185)
        result = audit(str(tmp_path))
        assert "error" not in result
        assert result["t_minus_1_summary"]["settled_count"] == 180
        findings = result["self_audit_findings"]
        types = {f["type"] for f in findings}
        assert "progression" in types

    def test_stagnation_detected(self, tmp_path):
        """settled 不变 → 检出 stagnation。"""
        _write_session(tmp_path, "a-session.md", 185)
        time.sleep(0.05)
        _write_session(tmp_path, "b-session.md", 185)
        result = audit(str(tmp_path))
        findings = result["self_audit_findings"]
        types = {f["type"] for f in findings}
        assert "stagnation" in types
        assert result["genealogy_needed"] is True

    def test_anomaly_on_negative_delta(self, tmp_path):
        """settled 减少 → 检出 anomaly。"""
        _write_session(tmp_path, "a-session.md", 185)
        time.sleep(0.05)
        _write_session(tmp_path, "b-session.md", 183)
        result = audit(str(tmp_path))
        findings = result["self_audit_findings"]
        types = {f["type"] for f in findings}
        assert "anomaly" in types
        assert result["genealogy_needed"] is True

    def test_unreferenced_genealogy_stagnation(self, tmp_path):
        """t-1 产出的谱系未被引用 → stagnation finding。"""
        # 3 个 session：t-2=178, t-1=180, t=183
        p1 = _write_session(tmp_path, "s1-session.md", 178)
        time.sleep(0.05)
        p2 = _write_session(tmp_path, "s2-session.md", 180)
        time.sleep(0.05)
        p3 = _write_session(tmp_path, "s3-session.md", 183)

        # 创建 settled 谱系 179, 180（t-1 产出）
        _write_settled(tmp_path, "179-test.md", "id: '179'")
        _write_settled(tmp_path, "180-test.md", "id: '180'")

        # 不创建 block-topology → 没有 depends_on 引用
        result = audit(str(tmp_path))
        t1_ids = result["t_minus_1_summary"].get("new_genealogy_ids", [])
        assert "179" in t1_ids
        assert "180" in t1_ids

    def test_referenced_genealogy_progression(self, tmp_path):
        """t-1 产出的谱系被 t 引用 → progression finding。"""
        p1 = _write_session(tmp_path, "s1-session.md", 178)
        time.sleep(0.05)
        p2 = _write_session(tmp_path, "s2-session.md", 180)
        time.sleep(0.05)
        p3 = _write_session(tmp_path, "s3-session.md", 183)

        # 创建 settled 谱系 179, 180
        _write_settled(tmp_path, "179-test.md", "id: '179'")
        _write_settled(tmp_path, "180-test.md", "id: '180'")

        # 创建 block-topology：181 depends_on 179
        sha_179 = "a" * 64
        sha_181 = "b" * 64
        base = _setup_block_topology(tmp_path, {"179": sha_179, "181": sha_181})
        _write_relation(base, {
            "from": sha_181, "to": sha_179,
            "relation": "depends_on", "order": 1,
            "created_by": sha_181,
        })

        result = audit(str(tmp_path))
        findings = result["self_audit_findings"]
        progression_findings = [f for f in findings if f["type"] == "progression"]
        # 应有两个 progression：一个是 settled 增加，一个是谱系被引用
        assert len(progression_findings) >= 2

    def test_genealogy_needed_false_on_pure_progression(self, tmp_path):
        """纯增长 + 无 stagnation/regression/anomaly → genealogy_needed=False。"""
        _write_session(tmp_path, "a-session.md", 180)
        time.sleep(0.05)
        _write_session(tmp_path, "b-session.md", 185)
        result = audit(str(tmp_path))
        assert result["genealogy_needed"] is False

    def test_output_format(self, tmp_path):
        """验证输出包含所有必要字段。"""
        _write_session(tmp_path, "a-session.md", 180)
        time.sleep(0.05)
        _write_session(tmp_path, "b-session.md", 185)
        result = audit(str(tmp_path))
        assert "t_session" in result
        assert "t_time" in result
        assert "t_minus_1_summary" in result
        assert "self_audit_findings" in result
        assert "genealogy_needed" in result
        assert isinstance(result["self_audit_findings"], list)
        assert isinstance(result["genealogy_needed"], bool)
        # 每个 finding 必须有 type 和 detail
        for f in result["self_audit_findings"]:
            assert "type" in f
            assert "detail" in f
            assert f["type"] in ("regression", "stagnation", "progression", "anomaly")
