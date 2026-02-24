"""tests for scripts/ceremony_scan.py — detect_pending_topo_effects（177号）+ compute_delta_blocks（178号）+ get_frozen_nodes/detect_genealogy_anomalies 迁移。"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import textwrap

import pytest

from scripts.ceremony_scan import (
    compute_delta_blocks,
    detect_genealogy_anomalies,
    detect_pending_topo_effects,
    get_frozen_nodes,
    main,
)


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


# ═══════════════════════════════════════════════════════════════
# Helpers for block-topology tests
# ═══════════════════════════════════════════════════════════════


def _make_sha(label: str) -> str:
    return hashlib.sha256(label.encode()).hexdigest()


def _setup_block_topology(root, id_mapping=None):
    """创建 block-topology 目录结构 + 可选 meta.json。"""
    base = os.path.join(root, ".chanlun", "block-topology")
    blocks_dir = os.path.join(base, "blocks")
    os.makedirs(blocks_dir, exist_ok=True)
    if id_mapping is not None:
        meta_path = os.path.join(base, "meta.json")
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump({"id_mapping": id_mapping}, f)
    return base


def _write_relation(base, rel_dict):
    """追加一条 relation 到 relations.jsonl。"""
    jsonl_path = os.path.join(base, "relations.jsonl")
    with open(jsonl_path, "a", encoding="utf-8") as f:
        f.write(json.dumps(rel_dict, ensure_ascii=False, separators=(",", ":")) + "\n")


def _write_block_file(base, block_id, content_dict):
    """写入一个 block JSON 文件。"""
    blocks_dir = os.path.join(base, "blocks")
    path = os.path.join(blocks_dir, f"{block_id}.json")
    with open(path, "w", encoding="utf-8") as f:
        json.dump(content_dict, f, ensure_ascii=False, indent=2)


# ═══════════════════════════════════════════════════════════════
# get_frozen_nodes — 从 block-topology 读取（178号-2 迁移）
# ═══════════════════════════════════════════════════════════════


class TestGetFrozenNodes:
    def test_no_block_topology(self, tmp_path) -> None:
        """无 block-topology 目录 → 空集合。"""
        result = get_frozen_nodes(str(tmp_path))
        assert result == set()

    def test_no_freezes_relations(self, tmp_path) -> None:
        """有 relations.jsonl 但无 freezes 关系 → 空集合。"""
        base = _setup_block_topology(tmp_path)
        _write_relation(base, {
            "from": _make_sha("A"), "to": _make_sha("B"),
            "relation": "depends_on", "order": 1,
            "created_by": _make_sha("creator"),
        })
        result = get_frozen_nodes(str(tmp_path))
        assert result == set()

    def test_local_freeze(self, tmp_path) -> None:
        """scope=local freeze → 只返回 direct target。"""
        target_sha = _make_sha("target-062")
        event_sha = _make_sha("freeze-event")
        base = _setup_block_topology(tmp_path, {"062": target_sha})

        # 写 freeze event 区块（content.scope=local）
        _write_block_file(base, event_sha, {
            "id": event_sha, "type": "event", "source": "cc",
            "content": {"action": "freeze", "target": target_sha, "scope": "local"},
        })
        # 写 freezes 关系
        _write_relation(base, {
            "from": event_sha, "to": target_sha,
            "relation": "freezes", "order": 1,
            "created_by": event_sha,
        })

        result = get_frozen_nodes(str(tmp_path))
        assert "62" in result or "062" in result

    def test_downstream_freeze_expands(self, tmp_path) -> None:
        """scope=downstream freeze → target + 所有下游 BFS 展开。"""
        target_sha = _make_sha("target-001")
        child_sha = _make_sha("child-002")
        grandchild_sha = _make_sha("grandchild-003")
        event_sha = _make_sha("freeze-event-ds")
        base = _setup_block_topology(tmp_path, {
            "001": target_sha, "002": child_sha, "003": grandchild_sha,
        })

        # freeze event 区块: scope=downstream
        _write_block_file(base, event_sha, {
            "id": event_sha, "type": "event", "source": "cc",
            "content": {"action": "freeze", "target": target_sha, "scope": "downstream"},
        })
        # freezes 关系
        _write_relation(base, {
            "from": event_sha, "to": target_sha,
            "relation": "freezes", "order": 1,
            "created_by": event_sha,
        })
        # depends_on: child depends_on target, grandchild depends_on child
        creator = _make_sha("creator")
        _write_relation(base, {
            "from": child_sha, "to": target_sha,
            "relation": "depends_on", "order": 1,
            "created_by": creator,
        })
        _write_relation(base, {
            "from": grandchild_sha, "to": child_sha,
            "relation": "depends_on", "order": 1,
            "created_by": creator,
        })

        result = get_frozen_nodes(str(tmp_path))
        # target + child + grandchild 全部被冻结
        assert "1" in result or "001" in result
        assert "2" in result or "002" in result
        assert "3" in result or "003" in result

    def test_reverse_mapping_fallback(self, tmp_path) -> None:
        """新区块（无旧映射）→ 返回 SHA id。"""
        new_sha = _make_sha("new-block")
        event_sha = _make_sha("freeze-new")
        base = _setup_block_topology(tmp_path, {})  # 空 mapping

        _write_block_file(base, event_sha, {
            "id": event_sha, "type": "event", "source": "cc",
            "content": {"action": "freeze", "target": new_sha, "scope": "local"},
        })
        _write_relation(base, {
            "from": event_sha, "to": new_sha,
            "relation": "freezes", "order": 1,
            "created_by": event_sha,
        })

        result = get_frozen_nodes(str(tmp_path))
        assert new_sha in result


# ═══════════════════════════════════════════════════════════════
# detect_genealogy_anomalies — block-topology 完整性检查
# ═══════════════════════════════════════════════════════════════


class TestDetectGenealogyAnomaliesBlockTopology:
    def test_missing_block_mapping(self, tmp_path) -> None:
        """settled 文件编号不在 id_mapping → 检出 missing_block_mapping。"""
        _write_settled(tmp_path, "001-test.md", textwrap.dedent("""\
            ---
            id: "001"
            status: "已结算"
            type: "定理"
            date: "2026-01-01"
            ---
        """))
        _write_settled(tmp_path, "002-test.md", textwrap.dedent("""\
            ---
            id: "002"
            status: "已结算"
            type: "定理"
            date: "2026-01-01"
            ---
        """))
        # 只映射 001，不映射 002
        _setup_block_topology(tmp_path, {"001": _make_sha("001")})

        anomalies = detect_genealogy_anomalies(str(tmp_path))
        mapping_anomalies = [a for a in anomalies if a["type"] == "missing_block_mapping"]
        assert len(mapping_anomalies) == 1
        assert "2" in mapping_anomalies[0]["detail"]

    def test_all_mapped_no_anomaly(self, tmp_path) -> None:
        """所有编号都在 id_mapping → 无 mapping 异常。"""
        _write_settled(tmp_path, "001-test.md", textwrap.dedent("""\
            ---
            id: "001"
            status: "已结算"
            type: "定理"
            date: "2026-01-01"
            ---
        """))
        _setup_block_topology(tmp_path, {"001": _make_sha("001")})

        anomalies = detect_genealogy_anomalies(str(tmp_path))
        mapping_anomalies = [a for a in anomalies if a["type"] == "missing_block_mapping"]
        assert len(mapping_anomalies) == 0

    def test_no_meta_json_no_crash(self, tmp_path) -> None:
        """无 meta.json → 不 crash，只是没有 mapping 检查。"""
        _write_settled(tmp_path, "001-test.md", textwrap.dedent("""\
            ---
            id: "001"
            status: "已结算"
            type: "定理"
            date: "2026-01-01"
            ---
        """))

        anomalies = detect_genealogy_anomalies(str(tmp_path))
        mapping_anomalies = [a for a in anomalies if a["type"] == "missing_block_mapping"]
        assert len(mapping_anomalies) == 0


# ═══════════════════════════════════════════════════════════════
# clean_terminate 与 workstations 一致性（186号下游推论2）
# ═══════════════════════════════════════════════════════════════


def _run_main_in_tmp(tmp_path, monkeypatch):
    """在 tmp_path 环境中运行 main()，返回解析后的 JSON 输出。

    mock 策略：
    - chdir → tmp_path（main 用 os.getcwd() 获取 root）
    - sys.argv → 无额外参数（根 ceremony 模式）
    - subprocess.run → 返回空 pytest 输出（无测试失败）
    - subprocess.check_output → 返回假 git HEAD
    """
    import io
    from unittest import mock

    monkeypatch.chdir(str(tmp_path))
    monkeypatch.setattr(sys, "argv", ["ceremony_scan.py"])

    fake_proc = mock.MagicMock()
    fake_proc.stdout = "0 passed in 0.01s\n"
    fake_proc.stderr = ""

    with mock.patch("scripts.ceremony_scan.subprocess.run", return_value=fake_proc), \
         mock.patch("scripts.ceremony_scan.subprocess.check_output", return_value="abc1234\n"):
        buf = io.StringIO()
        monkeypatch.setattr(sys, "stdout", buf)
        main()
        buf.seek(0)
        return json.loads(buf.read())


class TestCleanTerminateConsistency:
    """186号下游推论2：clean_terminate 与 workstations 的一致性。"""

    def test_clean_terminate_false_when_anomalies_exist(self, tmp_path, monkeypatch) -> None:
        """settled 文件存在但 block-topology 无 id_mapping → genealogy_anomalies 工位 → clean_terminate=False。"""
        _write_settled(tmp_path, "001-test.md", textwrap.dedent("""\
            ---
            id: "001"
            status: "已结算"
            type: "定理"
            date: "2026-01-01"
            ---
        """))
        # 创建 block-topology 但不映射 001 → 产生 missing_block_mapping anomaly
        _setup_block_topology(tmp_path, {})

        output = _run_main_in_tmp(tmp_path, monkeypatch)

        assert output["clean_terminate"] is False
        assert len(output["workstations"]) > 0
        # 应存在 genealogy_anomalies 驱动的工位
        anomaly_ws = [w for w in output["workstations"] if w.get("source") == "genealogy_anomaly_detection"]
        assert len(anomaly_ws) > 0

    def test_clean_terminate_true_when_no_workstations(self, tmp_path, monkeypatch) -> None:
        """无 settled、无 pending、无 roadmap → workstations 为空 → clean_terminate=True。"""
        output = _run_main_in_tmp(tmp_path, monkeypatch)

        assert output["clean_terminate"] is True
        assert output["workstations"] == []
