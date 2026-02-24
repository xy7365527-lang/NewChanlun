"""tests for scripts/ceremony_scan.py — detect_pending_topo_effects（177号）+ compute_delta_blocks（178号）。"""
from __future__ import annotations

import json
import os
import textwrap

import pytest

from scripts.ceremony_scan import compute_delta_blocks, detect_pending_topo_effects


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


# ═══════════════════════════════════════════════════════════════
# Helpers for compute_delta_blocks
# ═══════════════════════════════════════════════════════════════


def _setup_blocks(root, block_count, meta_block_count=None):
    """在 root/.chanlun/block-topology/ 下创建 block 文件和可选的 meta.json。"""
    block_dir = os.path.join(root, ".chanlun", "block-topology", "blocks")
    os.makedirs(block_dir, exist_ok=True)
    for i in range(block_count):
        path = os.path.join(block_dir, f"block_{i:03d}.json")
        with open(path, "w", encoding="utf-8") as f:
            json.dump({"id": i}, f)
    if meta_block_count is not None:
        meta_path = os.path.join(root, ".chanlun", "block-topology", "meta.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"block_count": meta_block_count}, f)


# ═══════════════════════════════════════════════════════════════
# compute_delta_blocks（178号）
# ═══════════════════════════════════════════════════════════════


class TestComputeDeltaBlocks:
    def test_no_block_dir(self, tmp_path) -> None:
        """block 目录不存在 → current=0, delta=0。"""
        result = compute_delta_blocks(str(tmp_path))
        assert result["current_block_count"] == 0
        assert result["migration_block_count"] == 0
        assert result["delta"] == 0
        assert result["warning"] is None

    def test_blocks_with_meta(self, tmp_path) -> None:
        """block 目录有 N 个文件，meta 有 M → delta=N-M。"""
        _setup_blocks(tmp_path, block_count=5, meta_block_count=3)
        result = compute_delta_blocks(str(tmp_path))
        assert result["current_block_count"] == 5
        assert result["migration_block_count"] == 3
        assert result["delta"] == 2
        assert result["warning"] is None

    def test_no_meta_json(self, tmp_path) -> None:
        """无 meta.json → migration_count=0。"""
        _setup_blocks(tmp_path, block_count=4)
        result = compute_delta_blocks(str(tmp_path))
        assert result["current_block_count"] == 4
        assert result["migration_block_count"] == 0
        assert result["delta"] == 4
        assert result["warning"] is None

    def test_zero_delta_warning(self, tmp_path) -> None:
        """delta=0 且有区块 → warning 提示无新区块。"""
        _setup_blocks(tmp_path, block_count=3, meta_block_count=3)
        result = compute_delta_blocks(str(tmp_path))
        assert result["delta"] == 0
        assert result["warning"] == "区块拓扑无新区块"

    def test_empty_blocks_dir(self, tmp_path) -> None:
        """blocks 目录存在但为空 → current=0, 无 warning。"""
        _setup_blocks(tmp_path, block_count=0, meta_block_count=0)
        result = compute_delta_blocks(str(tmp_path))
        assert result["current_block_count"] == 0
        assert result["delta"] == 0
        assert result["warning"] is None  # current=0 不触发 warning
