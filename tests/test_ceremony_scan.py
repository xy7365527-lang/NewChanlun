"""tests for scripts/ceremony_scan.py — detect_pending_topo_effects（177号）。"""
from __future__ import annotations

import os
import textwrap

import pytest

from scripts.ceremony_scan import detect_pending_topo_effects


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


def _write_settled(root, filename: str, content: str) -> str:
    """在 root/.chanlun/genealogy/settled/ 下写入文件，返回路径。"""
    settled_dir = os.path.join(root, ".chanlun", "genealogy", "settled")
    os.makedirs(settled_dir, exist_ok=True)
    path = os.path.join(settled_dir, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


GENEALOGY_STRUCTURED_PENDING = textwrap.dedent("""\
    ---
    id: '147'
    title: 矛盾具有拓扑效力
    status: 已结算
    topo_effect: "freeze:062:downstream"
    ---

    # 147 — 矛盾具有拓扑效力
""")

GENEALOGY_STRUCTURED_EXECUTED = textwrap.dedent("""\
    ---
    id: '148'
    title: 已执行的拓扑效果
    status: 已结算
    topo_effect: "sever:040:local"
    topo_executed_at: "2026-02-22"
    ---

    # 148 — 已执行
""")

GENEALOGY_DESCRIPTIVE_TOPO = textwrap.dedent("""\
    ---
    id: '141'
    title: 描述性 topo_effect
    status: 已结算
    topo_effect: "引入 topo_effect retrospective 标注方案"
    ---

    # 141 — 描述性
""")

GENEALOGY_NO_TOPO = textwrap.dedent("""\
    ---
    id: '153'
    title: 无 topo_effect
    status: 已结算
    topo_effect: ""
    ---

    # 153 — 无拓扑
""")

GENEALOGY_SPLIT_PENDING = textwrap.dedent("""\
    ---
    id: '160'
    title: 分裂操作
    status: 已结算
    topo_effect: "split:005:local"
    ---

    # 160 — 分裂
""")


# ═══════════════════════════════════════════════════════════════
# detect_pending_topo_effects
# ═══════════════════════════════════════════════════════════════


class TestDetectPendingTopoEffects:
    def test_detects_structured_pending(self, tmp_path) -> None:
        """含结构化 topo_effect 且无 topo_executed_at → 检出。"""
        _write_settled(tmp_path, "147-topo.md", GENEALOGY_STRUCTURED_PENDING)
        result = detect_pending_topo_effects(str(tmp_path))
        assert len(result) == 1
        assert result[0]["id"] == "147"
        assert result[0]["topo_effect"] == "freeze:062:downstream"
        assert "147-topo.md" in result[0]["file"]

    def test_skips_already_executed(self, tmp_path) -> None:
        """含 topo_executed_at → 跳过。"""
        _write_settled(tmp_path, "148-done.md", GENEALOGY_STRUCTURED_EXECUTED)
        result = detect_pending_topo_effects(str(tmp_path))
        assert len(result) == 0

    def test_skips_descriptive_topo_effect(self, tmp_path) -> None:
        """非结构化描述性 topo_effect → 跳过。"""
        _write_settled(tmp_path, "141-desc.md", GENEALOGY_DESCRIPTIVE_TOPO)
        result = detect_pending_topo_effects(str(tmp_path))
        assert len(result) == 0

    def test_skips_empty_topo_effect(self, tmp_path) -> None:
        """topo_effect 为空 → 跳过。"""
        _write_settled(tmp_path, "153-empty.md", GENEALOGY_NO_TOPO)
        result = detect_pending_topo_effects(str(tmp_path))
        assert len(result) == 0

    def test_empty_directory(self, tmp_path) -> None:
        """无 settled 目录 → 返回空列表。"""
        result = detect_pending_topo_effects(str(tmp_path))
        assert result == []

    def test_multiple_files_mixed(self, tmp_path) -> None:
        """混合场景：只检出未执行的结构化 topo_effect。"""
        _write_settled(tmp_path, "147-pending.md", GENEALOGY_STRUCTURED_PENDING)
        _write_settled(tmp_path, "148-done.md", GENEALOGY_STRUCTURED_EXECUTED)
        _write_settled(tmp_path, "141-desc.md", GENEALOGY_DESCRIPTIVE_TOPO)
        _write_settled(tmp_path, "160-split.md", GENEALOGY_SPLIT_PENDING)
        result = detect_pending_topo_effects(str(tmp_path))
        ids = {r["id"] for r in result}
        assert ids == {"147", "160"}

    def test_detects_all_three_types(self, tmp_path) -> None:
        """freeze/split/sever 三种类型都能检出。"""
        _write_settled(tmp_path, "a-freeze.md", textwrap.dedent("""\
            ---
            id: '201'
            topo_effect: "freeze:001:local"
            ---
        """))
        _write_settled(tmp_path, "b-split.md", textwrap.dedent("""\
            ---
            id: '202'
            topo_effect: "split:002:local"
            ---
        """))
        _write_settled(tmp_path, "c-sever.md", textwrap.dedent("""\
            ---
            id: '203'
            topo_effect: "sever:003:downstream"
            ---
        """))
        result = detect_pending_topo_effects(str(tmp_path))
        types = {r["topo_effect"].split(":")[0] for r in result}
        assert types == {"freeze", "split", "sever"}
