"""tests for scripts/topology_preprocess.py — 拓扑前处理脚本。

179号-3 下游推论：将 block-topology JSONL 关系数据转换为 networkx 图，
计算拓扑指标，格式化为分析家可读输入。

测试策略：使用小图（手工构建）验证核心计算逻辑，不依赖真实 block-topology 数据。
"""
from __future__ import annotations

import hashlib
import json

import pytest


# ── 测试辅助 ──


def _sha(label: str) -> str:
    """生成确定性 SHA256 用于测试。"""
    return hashlib.sha256(label.encode()).hexdigest()


# 节点 id
N_A = _sha("node-a")
N_B = _sha("node-b")
N_C = _sha("node-c")
N_D = _sha("node-d")
N_E = _sha("node-e")
N_ISOLATED = _sha("node-isolated")


@pytest.fixture()
def tmp_base(tmp_path):
    """临时 block-topology 目录，含 meta.json 和 relations.jsonl。"""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()

    # id_mapping: 旧谱系编号 → SHA256
    meta = {
        "version": "1.0.0",
        "block_count": 6,
        "relation_count": 5,
        "id_mapping": {
            "001": N_A,
            "002": N_B,
            "003": N_C,
            "004": N_D,
            "005": N_E,
            "006": N_ISOLATED,
        },
    }
    (base / "meta.json").write_text(
        json.dumps(meta, ensure_ascii=False, indent=2), encoding="utf-8"
    )

    # 构建小图:
    #   A → B (depends_on)
    #   A → C (depends_on)
    #   B → D (depends_on)
    #   C → D (depends_on)
    #   A → E (negates)
    #   N_ISOLATED 无连接
    #
    # 最长路径: A → B → D (或 A → C → D), 长度 2
    # A 的 betweenness 应该最高（所有到D的路径都经过A的子节点）
    # N_ISOLATED 是孤立节点
    ts = "2026-02-24T00:00:00+00:00"
    creator = N_A
    relations = [
        {"from": N_B, "to": N_A, "relation": "depends_on", "order": 1,
         "created_by": creator, "timestamp": ts},
        {"from": N_C, "to": N_A, "relation": "depends_on", "order": 1,
         "created_by": creator, "timestamp": ts},
        {"from": N_D, "to": N_B, "relation": "depends_on", "order": 1,
         "created_by": creator, "timestamp": ts},
        {"from": N_D, "to": N_C, "relation": "depends_on", "order": 1,
         "created_by": creator, "timestamp": ts},
        {"from": N_A, "to": N_E, "relation": "negates", "order": 1,
         "created_by": creator, "timestamp": ts},
    ]
    lines = [
        json.dumps(r, ensure_ascii=False, separators=(",", ":"))
        for r in relations
    ]
    (base / "relations.jsonl").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )

    # 为孤立节点写入区块文件（使其存在于 blocks 目录）
    for label, sha in [
        ("node-a", N_A), ("node-b", N_B), ("node-c", N_C),
        ("node-d", N_D), ("node-e", N_E), ("node-isolated", N_ISOLATED),
    ]:
        block = {
            "id": sha, "type": "event", "source": "migration",
            "content": {"label": label}, "refs": [],
            "timestamp": ts, "git_ref": "",
        }
        (base / "blocks" / f"{sha}.json").write_text(
            json.dumps(block, ensure_ascii=False, indent=2), encoding="utf-8"
        )

    return base


@pytest.fixture()
def empty_base(tmp_path):
    """空的 block-topology 目录。"""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    (base / "meta.json").write_text(
        json.dumps({"version": "1.0.0", "id_mapping": {}}, ensure_ascii=False),
        encoding="utf-8",
    )
    (base / "relations.jsonl").write_text("", encoding="utf-8")
    return base


# ═══════════════════════════════════════════════════════════════
# 1. load_relations 测试
# ═══════════════════════════════════════════════════════════════


class TestLoadRelations:
    def test_load_from_jsonl(self, tmp_base):
        from scripts.topology_preprocess import load_relations

        rels = load_relations(tmp_base)
        assert len(rels) == 5
        assert all("from" in r and "to" in r for r in rels)

    def test_load_empty(self, empty_base):
        from scripts.topology_preprocess import load_relations

        rels = load_relations(empty_base)
        assert rels == []

    def test_load_nonexistent_file(self, tmp_path):
        from scripts.topology_preprocess import load_relations

        base = tmp_path / "nonexistent"
        base.mkdir()
        rels = load_relations(base)
        assert rels == []


# ═══════════════════════════════════════════════════════════════
# 2. build_graph 测试
# ═══════════════════════════════════════════════════════════════


class TestBuildGraph:
    def test_graph_node_count(self, tmp_base):
        from scripts.topology_preprocess import build_graph, load_relations

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        # 5 节点来自关系 + 1 孤立节点来自 blocks 目录 = 6
        assert g.number_of_nodes() == 6

    def test_graph_edge_count(self, tmp_base):
        from scripts.topology_preprocess import build_graph, load_relations

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        assert g.number_of_edges() == 5

    def test_edge_attributes(self, tmp_base):
        from scripts.topology_preprocess import build_graph, load_relations

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        # B depends_on A → 图中边 B→A
        edge_data = g.get_edge_data(N_B, N_A)
        assert edge_data is not None
        assert edge_data["relation"] == "depends_on"


# ═══════════════════════════════════════════════════════════════
# 3. compute_graph_summary 测试
# ═══════════════════════════════════════════════════════════════


class TestComputeGraphSummary:
    def test_basic_summary(self, tmp_base):
        from scripts.topology_preprocess import (
            build_graph,
            compute_graph_summary,
            load_relations,
        )

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        summary = compute_graph_summary(g)

        assert summary["nodes"] == 6
        assert summary["edges"] == 5
        # 图是弱连通的（A→E negates, 加上 depends_on 链），N_ISOLATED 独立
        assert summary["components"] >= 1
        assert summary["longest_path_length"] >= 2

    def test_empty_graph(self, empty_base):
        from scripts.topology_preprocess import (
            build_graph,
            compute_graph_summary,
            load_relations,
        )

        rels = load_relations(empty_base)
        blocks_dir = empty_base / "blocks"
        g = build_graph(rels, blocks_dir)
        summary = compute_graph_summary(g)

        assert summary["nodes"] == 0
        assert summary["edges"] == 0
        assert summary["components"] == 0
        assert summary["longest_path_length"] == 0


# ═══════════════════════════════════════════════════════════════
# 4. find_load_bearing_nodes 测试
# ═══════════════════════════════════════════════════════════════


class TestFindLoadBearingNodes:
    def test_returns_sorted_list(self, tmp_base):
        from scripts.topology_preprocess import (
            build_graph,
            find_load_bearing_nodes,
            load_relations,
        )

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        nodes = find_load_bearing_nodes(g)

        assert isinstance(nodes, list)
        assert len(nodes) > 0
        # 按 betweenness 降序
        betweenness_values = [n["betweenness"] for n in nodes]
        assert betweenness_values == sorted(betweenness_values, reverse=True)

    def test_node_fields(self, tmp_base):
        from scripts.topology_preprocess import (
            build_graph,
            find_load_bearing_nodes,
            load_relations,
        )

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        nodes = find_load_bearing_nodes(g)

        for node in nodes:
            assert "id" in node
            assert "betweenness" in node
            assert "in_degree" in node
            assert "out_degree" in node
            assert isinstance(node["betweenness"], float)

    def test_empty_graph_returns_empty(self, empty_base):
        from scripts.topology_preprocess import (
            build_graph,
            find_load_bearing_nodes,
            load_relations,
        )

        rels = load_relations(empty_base)
        blocks_dir = empty_base / "blocks"
        g = build_graph(rels, blocks_dir)
        nodes = find_load_bearing_nodes(g)
        assert nodes == []


# ═══════════════════════════════════════════════════════════════
# 5. find_isolated_nodes 测试
# ═══════════════════════════════════════════════════════════════


class TestFindIsolatedNodes:
    def test_finds_isolated(self, tmp_base):
        from scripts.topology_preprocess import (
            build_graph,
            find_isolated_nodes,
            load_relations,
        )

        rels = load_relations(tmp_base)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        isolated = find_isolated_nodes(g)

        assert len(isolated) == 1
        assert isolated[0]["id"] == N_ISOLATED

    def test_no_isolated(self, tmp_base):
        """当所有节点都有边时，返回空列表。"""
        from scripts.topology_preprocess import (
            build_graph,
            find_isolated_nodes,
            load_relations,
        )

        rels = load_relations(tmp_base)
        # 添加一条连接孤立节点的关系
        extra = {
            "from": N_ISOLATED, "to": N_A, "relation": "related", "order": 1,
            "created_by": N_A, "timestamp": "2026-02-24T00:00:00+00:00",
        }
        rels.append(extra)
        blocks_dir = tmp_base / "blocks"
        g = build_graph(rels, blocks_dir)
        isolated = find_isolated_nodes(g)
        assert isolated == []


# ═══════════════════════════════════════════════════════════════
# 6. compute_relation_type_distribution 测试
# ═══════════════════════════════════════════════════════════════


class TestRelationTypeDistribution:
    def test_distribution(self, tmp_base):
        from scripts.topology_preprocess import (
            compute_relation_type_distribution,
            load_relations,
        )

        rels = load_relations(tmp_base)
        dist = compute_relation_type_distribution(rels)

        assert dist["depends_on"] == 4
        assert dist["negates"] == 1

    def test_empty(self, empty_base):
        from scripts.topology_preprocess import (
            compute_relation_type_distribution,
            load_relations,
        )

        rels = load_relations(empty_base)
        dist = compute_relation_type_distribution(rels)
        assert dist == {}


# ═══════════════════════════════════════════════════════════════
# 7. translate_ids 测试
# ═══════════════════════════════════════════════════════════════


class TestTranslateIds:
    def test_translate_known_ids(self, tmp_base):
        from scripts.topology_preprocess import load_id_mapping, translate_id

        mapping = load_id_mapping(tmp_base)
        # 反向映射 SHA→谱系编号
        assert translate_id(N_A, mapping) == "001"
        assert translate_id(N_B, mapping) == "002"

    def test_translate_unknown_id(self, tmp_base):
        from scripts.topology_preprocess import load_id_mapping, translate_id

        mapping = load_id_mapping(tmp_base)
        unknown = _sha("unknown")
        # 未知 id 返回截断的 SHA
        result = translate_id(unknown, mapping)
        assert result == unknown[:12]

    def test_empty_mapping(self, empty_base):
        from scripts.topology_preprocess import load_id_mapping, translate_id

        mapping = load_id_mapping(empty_base)
        result = translate_id(N_A, mapping)
        assert result == N_A[:12]


# ═══════════════════════════════════════════════════════════════
# 8. generate_report 集成测试
# ═══════════════════════════════════════════════════════════════


class TestGenerateReport:
    def test_report_structure(self, tmp_base):
        from scripts.topology_preprocess import generate_report

        report = generate_report(tmp_base)

        assert "graph_summary" in report
        assert "load_bearing_nodes" in report
        assert "isolated_nodes" in report
        assert "relation_type_distribution" in report

        gs = report["graph_summary"]
        assert gs["nodes"] == 6
        assert gs["edges"] == 5

    def test_report_load_bearing_has_readable_id(self, tmp_base):
        from scripts.topology_preprocess import generate_report

        report = generate_report(tmp_base)
        for node in report["load_bearing_nodes"]:
            assert "readable_id" in node

    def test_report_isolated_has_readable_id(self, tmp_base):
        from scripts.topology_preprocess import generate_report

        report = generate_report(tmp_base)
        for node in report["isolated_nodes"]:
            assert "readable_id" in node

    def test_report_json_serializable(self, tmp_base):
        from scripts.topology_preprocess import generate_report

        report = generate_report(tmp_base)
        # 确保可以序列化为 JSON
        serialized = json.dumps(report, ensure_ascii=False, indent=2)
        assert isinstance(serialized, str)
        roundtrip = json.loads(serialized)
        assert roundtrip == report

    def test_report_empty_base(self, empty_base):
        from scripts.topology_preprocess import generate_report

        report = generate_report(empty_base)
        assert report["graph_summary"]["nodes"] == 0
        assert report["graph_summary"]["edges"] == 0
        assert report["load_bearing_nodes"] == []
        assert report["isolated_nodes"] == []
        assert report["relation_type_distribution"] == {}
