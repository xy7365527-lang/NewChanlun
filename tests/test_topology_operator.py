"""tests for scripts/topology_operator.py — 矛盾→拓扑操作自动映射。

覆盖三种否定类型的基本场景 + 边界条件 + auto_execute_from_file（177号）。
"""
from __future__ import annotations

import copy
import os
import textwrap

import pytest
import yaml

from scripts.topology_operator import (
    NEGATION_FORM_TO_TOPO,
    apply_freeze,
    apply_sever,
    apply_split,
    auto_execute_from_file,
    execute_topo_effect,
    extract_genealogy_fields,
    infer_topo_type_from_negation_form,
    parse_topo_effect,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


@pytest.fixture()
def sample_dag() -> dict:
    """最小可用 dag 结构。"""
    return {
        "nodes": [
            {"id": "001", "title": "节点1", "status": "已结算", "type": "矛盾记录"},
            {"id": "002", "title": "节点2", "status": "已结算", "type": "定理"},
            {"id": "003", "title": "节点3", "status": "已结算", "type": "概念分离"},
        ],
        "edges": {
            "depends_on": [
                {"from": "002", "to": "001"},
                {"from": "003", "to": "001"},
            ],
            "related": [
                {"between": ["001", "002"]},
            ],
            "negates": [
                {"from": "003", "to": "002"},
            ],
            "tensions_with": [],
        },
    }


GENEALOGY_WITH_TOPO_EFFECT = textwrap.dedent("""\
    ---
    id: "147"
    title: "矛盾具有拓扑效力"
    status: "已结算"
    type: "语法记录"
    negation_form: "separation（三个预设选项被分离）"
    topo_effect: "sever:062:local"
    depends_on: ["005", "040"]
    negates: []
    ---

    # 147 — 矛盾具有拓扑效力
""")

GENEALOGY_WITH_NEGATION_FORM_ONLY = textwrap.dedent("""\
    ---
    id: "090"
    title: "严格性是蜂群的语法规则"
    status: "已结算"
    type: "语法记录"
    negation_form: ""
    negates: []
    ---

    # 090 — 严格性是蜂群的语法规则

    - **negation_form**: expansion（从个案否定到永久原则）
""")

GENEALOGY_EMPTY_TOPO = textwrap.dedent("""\
    ---
    id: "153"
    title: "RTAS 蜂群持久化"
    status: "已结算"
    topo_effect: ""
    negation_form: ""
    ---
""")


# ═══════════════════════════════════════════════════════════════
# parse_topo_effect
# ═══════════════════════════════════════════════════════════════


class TestParsTopoEffect:
    def test_valid_freeze(self) -> None:
        result = parse_topo_effect("freeze:062:downstream")
        assert result == ("freeze", "062", "downstream")

    def test_valid_split(self) -> None:
        result = parse_topo_effect("split:005:local")
        assert result == ("split", "005", "local")

    def test_valid_sever(self) -> None:
        result = parse_topo_effect("sever:040:local")
        assert result == ("sever", "040", "local")

    def test_empty_string(self) -> None:
        assert parse_topo_effect("") is None

    def test_non_structured(self) -> None:
        assert parse_topo_effect("引入 topo_effect 方案") is None

    def test_invalid_type(self) -> None:
        assert parse_topo_effect("destroy:001:local") is None

    def test_wrong_parts_count(self) -> None:
        assert parse_topo_effect("freeze:001") is None
        assert parse_topo_effect("freeze:001:local:extra") is None

    def test_quoted_string(self) -> None:
        result = parse_topo_effect('"sever:040:local"')
        assert result == ("sever", "040", "local")


# ═══════════════════════════════════════════════════════════════
# extract_genealogy_fields
# ═══════════════════════════════════════════════════════════════


class TestExtractGenealogyFields:
    def test_with_topo_effect_in_frontmatter(self) -> None:
        fields = extract_genealogy_fields(GENEALOGY_WITH_TOPO_EFFECT)
        assert fields["id"] == "147"
        assert fields["topo_effect"] == "sever:062:local"
        assert "separation" in str(fields.get("negation_form", ""))

    def test_negation_form_from_body(self) -> None:
        fields = extract_genealogy_fields(GENEALOGY_WITH_NEGATION_FORM_ONLY)
        assert fields["id"] == "090"
        assert fields["negation_form"] == "expansion"

    def test_empty_topo_effect(self) -> None:
        fields = extract_genealogy_fields(GENEALOGY_EMPTY_TOPO)
        assert fields["id"] == "153"
        # topo_effect 是空字符串
        assert not fields.get("topo_effect")


# ═══════════════════════════════════════════════════════════════
# infer_topo_type_from_negation_form
# ═══════════════════════════════════════════════════════════════


class TestInferTopoType:
    def test_waiting(self) -> None:
        assert infer_topo_type_from_negation_form("waiting") == "freeze"

    def test_expansion(self) -> None:
        assert infer_topo_type_from_negation_form("expansion") == "split"

    def test_separation(self) -> None:
        assert infer_topo_type_from_negation_form("separation") == "sever"

    def test_expansion_with_description(self) -> None:
        assert (
            infer_topo_type_from_negation_form("expansion（从个案到原则）")
            == "split"
        )

    def test_unclassified(self) -> None:
        assert infer_topo_type_from_negation_form("unclassified") is None

    def test_empty(self) -> None:
        assert infer_topo_type_from_negation_form("") is None

    def test_unknown_form(self) -> None:
        assert infer_topo_type_from_negation_form("dialectical") is None

    def test_mapping_completeness(self) -> None:
        """验证映射表覆盖了 040号定义的三种基本否定形式。"""
        assert set(NEGATION_FORM_TO_TOPO.keys()) == {
            "waiting",
            "expansion",
            "separation",
        }


# ═══════════════════════════════════════════════════════════════
# apply_freeze — 等待型否定
# ═══════════════════════════════════════════════════════════════


class TestApplyFreeze:
    def test_freeze_local(self, sample_dag: dict) -> None:
        log = apply_freeze(sample_dag, "099", "001", "local")
        node = next(n for n in sample_dag["nodes"] if n["id"] == "001")
        assert node["frozen"] is True
        assert node["frozen_by"] == "099"
        assert any("冻结节点 001" in entry for entry in log)

    def test_freeze_downstream(self, sample_dag: dict) -> None:
        log = apply_freeze(sample_dag, "099", "001", "downstream")
        # 节点被冻结
        node = next(n for n in sample_dag["nodes"] if n["id"] == "001")
        assert node["frozen"] is True
        # 下游依赖边被冻结
        frozen_edges = [
            e
            for e in sample_dag["edges"]["depends_on"]
            if e.get("frozen_by") == "099"
        ]
        assert len(frozen_edges) == 2  # 002→001 和 003→001

    def test_freeze_nonexistent_target(self, sample_dag: dict) -> None:
        log = apply_freeze(sample_dag, "099", "999", "local")
        assert any("不存在" in entry for entry in log)


# ═══════════════════════════════════════════════════════════════
# apply_split — 扩张型否定
# ═══════════════════════════════════════════════════════════════


class TestApplySplit:
    def test_split_creates_two_nodes(self, sample_dag: dict) -> None:
        original_count = len(sample_dag["nodes"])
        log = apply_split(sample_dag, "099", "002", "local")
        assert len(sample_dag["nodes"]) == original_count + 2
        ids = {str(n["id"]) for n in sample_dag["nodes"]}
        assert "002-a" in ids
        assert "002-b" in ids

    def test_split_marks_original(self, sample_dag: dict) -> None:
        apply_split(sample_dag, "099", "002", "local")
        original = next(n for n in sample_dag["nodes"] if n["id"] == "002")
        assert original["split_into"] == ["002-a", "002-b"]
        assert original["split_by"] == "099"

    def test_split_copies_metadata(self, sample_dag: dict) -> None:
        apply_split(sample_dag, "099", "002", "local")
        node_a = next(n for n in sample_dag["nodes"] if n["id"] == "002-a")
        assert node_a["title"] == "节点2"
        assert node_a["split_from"] == "002"
        assert node_a["split_by"] == "099"

    def test_split_nonexistent_target(self, sample_dag: dict) -> None:
        original_count = len(sample_dag["nodes"])
        log = apply_split(sample_dag, "099", "999", "local")
        assert len(sample_dag["nodes"]) == original_count
        assert any("不存在" in entry for entry in log)

    def test_split_deep_copy_isolation(self, sample_dag: dict) -> None:
        """分裂的两个节点互不影响。"""
        apply_split(sample_dag, "099", "002", "local")
        node_a = next(n for n in sample_dag["nodes"] if n["id"] == "002-a")
        node_b = next(n for n in sample_dag["nodes"] if n["id"] == "002-b")
        node_a["title"] = "modified"
        assert node_b["title"] == "节点2"


# ═══════════════════════════════════════════════════════════════
# apply_sever — 分离型否定
# ═══════════════════════════════════════════════════════════════


class TestApplySever:
    def test_sever_directed_edges(self, sample_dag: dict) -> None:
        log = apply_sever(sample_dag, "099", "002", "local")
        # depends_on: 002→001 被切断
        dep_edge = next(
            e
            for e in sample_dag["edges"]["depends_on"]
            if e.get("from") == "002"
        )
        assert dep_edge["severed_by"] == "099"
        # negates: 003→002 被切断
        neg_edge = next(
            e
            for e in sample_dag["edges"]["negates"]
            if e.get("to") == "002"
        )
        assert neg_edge["severed_by"] == "099"

    def test_sever_undirected_edges(self, sample_dag: dict) -> None:
        apply_sever(sample_dag, "099", "001", "local")
        rel_edge = sample_dag["edges"]["related"][0]
        assert rel_edge["severed_by"] == "099"

    def test_sever_no_edges(self, sample_dag: dict) -> None:
        # 节点 003 只有 depends_on 和 negates 边
        # 先处理：构造一个无边节点
        sample_dag["nodes"].append(
            {"id": "999", "title": "孤立节点", "status": "已结算"}
        )
        log = apply_sever(sample_dag, "099", "999", "local")
        assert any("无涉及" in entry for entry in log)


# ═══════════════════════════════════════════════════════════════
# execute_topo_effect — 统一入口
# ═══════════════════════════════════════════════════════════════


class TestExecuteTopoEffect:
    def test_freeze_via_execute(self, sample_dag: dict) -> None:
        log = execute_topo_effect(sample_dag, "099", "freeze", "001", "local")
        assert any("冻结" in entry for entry in log)

    def test_split_via_execute(self, sample_dag: dict) -> None:
        log = execute_topo_effect(sample_dag, "099", "split", "002", "local")
        assert any("分裂" in entry for entry in log)

    def test_sever_via_execute(self, sample_dag: dict) -> None:
        log = execute_topo_effect(sample_dag, "099", "sever", "002", "local")
        assert any("切断" in entry for entry in log)

    def test_unknown_type(self, sample_dag: dict) -> None:
        log = execute_topo_effect(
            sample_dag, "099", "destroy", "002", "local"
        )
        assert any("未知" in entry for entry in log)

    def test_immutability_of_unaffected_nodes(
        self, sample_dag: dict
    ) -> None:
        """确保操作不影响无关节点。"""
        original = copy.deepcopy(sample_dag)
        execute_topo_effect(sample_dag, "099", "freeze", "001", "local")
        # 节点 002、003 应不受影响
        for node_id in ("002", "003"):
            orig_node = next(
                n for n in original["nodes"] if n["id"] == node_id
            )
            new_node = next(
                n for n in sample_dag["nodes"] if n["id"] == node_id
            )
            assert orig_node == new_node


# ═══════════════════════════════════════════════════════════════
# auto_execute_from_file — 177号 RTAS 循环入口
# ═══════════════════════════════════════════════════════════════


def _make_dag_file(tmp_path, dag_data: dict) -> str:
    """创建临时 dag.yaml 并返回路径。"""
    dag_path = os.path.join(tmp_path, "dag.yaml")
    with open(dag_path, "w", encoding="utf-8") as f:
        yaml.dump(dag_data, f, allow_unicode=True, default_flow_style=False)
    return dag_path


def _make_genealogy_file(tmp_path, filename: str, content: str) -> str:
    """创建临时谱系文件并返回路径。"""
    path = os.path.join(tmp_path, filename)
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)
    return path


SMALL_DAG = {
    "nodes": [
        {"id": "001", "title": "节点1", "status": "已结算", "type": "矛盾记录"},
        {"id": "062", "title": "节点62", "status": "已结算", "type": "定理"},
        {"id": "005", "title": "节点5", "status": "已结算", "type": "概念分离"},
    ],
    "edges": {
        "depends_on": [
            {"from": "062", "to": "001"},
        ],
        "related": [],
        "negates": [],
        "tensions_with": [],
    },
}


class TestAutoExecuteFromFile:
    def test_normal_execution(self, tmp_path) -> None:
        """正常执行：结构化 topo_effect + 非承重点 → executed=True。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-topo.md",
            textwrap.dedent("""\
                ---
                id: '147'
                title: 矛盾具有拓扑效力
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        result = auto_execute_from_file(genealogy, dag_path)
        assert result["executed"] is True
        assert result["effect"] == "freeze:062:local"
        assert result["source_id"] == "147"
        assert result["load_bearing"] is False
        assert any("冻结" in entry for entry in result["log"])
        assert any("dag.yaml 已更新" in entry for entry in result["log"])

    def test_topo_executed_at_written(self, tmp_path) -> None:
        """执行后谱系文件应有 topo_executed_at 字段。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-topo.md",
            textwrap.dedent("""\
                ---
                id: '147'
                title: 矛盾具有拓扑效力
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        auto_execute_from_file(genealogy, dag_path)
        # 读取文件确认 topo_executed_at 已写入
        with open(genealogy, encoding="utf-8") as f:
            content = f.read()
        assert "topo_executed_at:" in content

    def test_already_executed_skipped(self, tmp_path) -> None:
        """已有 topo_executed_at → 不执行。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "148-done.md",
            textwrap.dedent("""\
                ---
                id: '148'
                topo_effect: "sever:040:local"
                topo_executed_at: "2026-02-22"
                ---

                # 148
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        result = auto_execute_from_file(genealogy, dag_path)
        assert result["executed"] is False
        assert any("已执行" in entry for entry in result["log"])

    def test_no_topo_effect(self, tmp_path) -> None:
        """无 topo_effect → 不执行。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "153-empty.md",
            textwrap.dedent("""\
                ---
                id: '153'
                topo_effect: ""
                ---

                # 153
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        result = auto_execute_from_file(genealogy, dag_path)
        assert result["executed"] is False
        assert result["effect"] is None

    def test_descriptive_topo_effect_skipped(self, tmp_path) -> None:
        """非结构化描述性 topo_effect → 不执行。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "141-desc.md",
            textwrap.dedent("""\
                ---
                id: '141'
                topo_effect: "引入 topo_effect retrospective 标注方案"
                ---

                # 141
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        result = auto_execute_from_file(genealogy, dag_path)
        assert result["executed"] is False
        assert result["effect"] is None

    def test_load_bearing_warning(self, tmp_path) -> None:
        """承重点 → 不执行，返回 load_bearing=True。"""
        # 构造一个 DAG 使 001 成为承重点（很多后代依赖它）
        heavy_dag = {
            "nodes": [
                {"id": "001", "title": "根节点", "status": "已结算"},
            ]
            + [
                {"id": f"{i:03d}", "title": f"节点{i}", "status": "已结算"}
                for i in range(10, 22)
            ],
            "edges": {
                "depends_on": [
                    {"from": f"{i:03d}", "to": "001"} for i in range(10, 22)
                ],
                "related": [],
                "negates": [],
                "tensions_with": [],
            },
        }
        genealogy = _make_genealogy_file(
            tmp_path,
            "099-lb.md",
            textwrap.dedent("""\
                ---
                id: '099'
                topo_effect: "freeze:001:local"
                ---

                # 099
            """),
        )
        dag_path = _make_dag_file(tmp_path, heavy_dag)
        result = auto_execute_from_file(genealogy, dag_path)
        assert result["executed"] is False
        assert result["load_bearing"] is True
        assert any("承重点" in entry for entry in result["log"])

    def test_nonexistent_genealogy(self, tmp_path) -> None:
        """谱系文件不存在 → 不执行。"""
        result = auto_execute_from_file(
            os.path.join(tmp_path, "nonexistent.md"),
            os.path.join(tmp_path, "dag.yaml"),
        )
        assert result["executed"] is False
        assert any("不存在" in entry for entry in result["log"])

    def test_dag_updated_after_execution(self, tmp_path) -> None:
        """执行后 dag.yaml 中目标节点应被冻结。"""
        import copy as _copy

        dag_data = _copy.deepcopy(SMALL_DAG)
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-freeze.md",
            textwrap.dedent("""\
                ---
                id: '147'
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        dag_path = _make_dag_file(tmp_path, dag_data)
        auto_execute_from_file(genealogy, dag_path)
        # 验证 dag.yaml 中 062 被冻结
        with open(dag_path, encoding="utf-8") as f:
            updated_dag = yaml.safe_load(f)
        node_062 = next(n for n in updated_dag["nodes"] if str(n["id"]) == "062")
        assert node_062.get("frozen") is True

    def test_idempotent_double_call(self, tmp_path) -> None:
        """第二次调用应跳过（topo_executed_at 已写入）。"""
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-idem.md",
            textwrap.dedent("""\
                ---
                id: '147'
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        dag_path = _make_dag_file(tmp_path, SMALL_DAG)
        r1 = auto_execute_from_file(genealogy, dag_path)
        assert r1["executed"] is True
        r2 = auto_execute_from_file(genealogy, dag_path)
        assert r2["executed"] is False
        assert any("已执行" in entry for entry in r2["log"])
