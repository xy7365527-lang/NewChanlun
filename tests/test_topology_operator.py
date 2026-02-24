"""tests for scripts/topology_operator.py — 矛盾→拓扑操作自动映射。

178号-2 升格后：所有拓扑操作通过 block-topology 写入，不再操作 dag.yaml。
覆盖三种否定类型 + 承重点评分 + id 解析 + auto_execute_from_file。
"""
from __future__ import annotations

import json
import textwrap

import pytest

from scripts.block_topology import (
    compute_block_id,
    make_block,
    make_relation,
    read_all_relations,
    read_block,
    write_block_with_relations,
    write_meta,
)
from scripts.topology_operator import (
    NEGATION_FORM_TO_TOPO,
    auto_execute_from_file,
    compute_load_bearing_score,
    execute_topo_effect,
    extract_genealogy_fields,
    infer_topo_type_from_negation_form,
    parse_topo_effect,
    resolve_genealogy_id,
    write_freeze,
    write_sever,
    write_split,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════


@pytest.fixture()
def tmp_base(tmp_path):
    """临时 block-topology 目录。"""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


def _make_sha(label: str) -> str:
    """生成确定性 SHA256 用于测试。"""
    import hashlib
    return hashlib.sha256(label.encode()).hexdigest()


SOURCE_SHA = _make_sha("source-099")
TARGET_SHA = _make_sha("target-062")
TARGET2_SHA = _make_sha("target-005")


# ═══════════════════════════════════════════════════════════════
# parse_topo_effect（纯解析，保持不变）
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
# extract_genealogy_fields（纯解析，保持不变）
# ═══════════════════════════════════════════════════════════════


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
        assert not fields.get("topo_effect")


# ═══════════════════════════════════════════════════════════════
# infer_topo_type_from_negation_form（纯解析，保持不变）
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
# write_freeze — 冻结操作写入 block-topology
# ═══════════════════════════════════════════════════════════════


class TestWriteFreeze:
    def test_creates_block_and_relation(self, tmp_base) -> None:
        block = write_freeze(SOURCE_SHA, TARGET_SHA, "local", tmp_base)
        assert block["type"] == "event"
        assert block["source"] == "cc"
        assert block["content"]["action"] == "freeze"
        assert block["content"]["target"] == TARGET_SHA
        assert block["content"]["scope"] == "local"
        assert block["refs"] == [SOURCE_SHA]
        # 验证区块文件存在
        assert read_block(block["id"], tmp_base) is not None
        # 验证 freezes 关系
        rels = read_all_relations(tmp_base)
        freeze_rels = [r for r in rels if r["relation"] == "freezes"]
        assert len(freeze_rels) == 1
        assert freeze_rels[0]["from"] == block["id"]
        assert freeze_rels[0]["to"] == TARGET_SHA

    def test_downstream_scope_in_content(self, tmp_base) -> None:
        block = write_freeze(SOURCE_SHA, TARGET_SHA, "downstream", tmp_base)
        assert block["content"]["scope"] == "downstream"
        # 只写一条 freezes 关系（不展开 downstream）
        rels = read_all_relations(tmp_base)
        freeze_rels = [r for r in rels if r["relation"] == "freezes"]
        assert len(freeze_rels) == 1

    def test_idempotent(self, tmp_base) -> None:
        b1 = write_freeze(SOURCE_SHA, TARGET_SHA, "local", tmp_base)
        b2 = write_freeze(SOURCE_SHA, TARGET_SHA, "local", tmp_base)
        assert b1["id"] == b2["id"]


# ═══════════════════════════════════════════════════════════════
# write_split — 分裂操作写入 block-topology
# ═══════════════════════════════════════════════════════════════


class TestWriteSplit:
    def test_creates_block_and_relation(self, tmp_base) -> None:
        block = write_split(SOURCE_SHA, TARGET_SHA, "local", tmp_base)
        assert block["type"] == "event"
        assert block["content"]["action"] == "split"
        assert block["content"]["target"] == TARGET_SHA
        # 验证 splits 关系
        rels = read_all_relations(tmp_base)
        split_rels = [r for r in rels if r["relation"] == "splits"]
        assert len(split_rels) == 1
        assert split_rels[0]["from"] == block["id"]
        assert split_rels[0]["to"] == TARGET_SHA


# ═══════════════════════════════════════════════════════════════
# write_sever — 切断操作写入 block-topology
# ═══════════════════════════════════════════════════════════════


class TestWriteSever:
    def test_creates_block_and_relation(self, tmp_base) -> None:
        block = write_sever(SOURCE_SHA, TARGET_SHA, "local", tmp_base)
        assert block["type"] == "event"
        assert block["content"]["action"] == "sever"
        assert block["content"]["target"] == TARGET_SHA
        # 验证 severs 关系
        rels = read_all_relations(tmp_base)
        sever_rels = [r for r in rels if r["relation"] == "severs"]
        assert len(sever_rels) == 1
        assert sever_rels[0]["from"] == block["id"]
        assert sever_rels[0]["to"] == TARGET_SHA


# ═══════════════════════════════════════════════════════════════
# execute_topo_effect — 统一调度
# ═══════════════════════════════════════════════════════════════


class TestExecuteTopoEffect:
    def test_freeze_dispatch(self, tmp_base) -> None:
        result = execute_topo_effect(
            SOURCE_SHA, "freeze", TARGET_SHA, "local", tmp_base,
        )
        assert result["executed"] is True
        assert result["block"] is not None
        assert result["block"]["content"]["action"] == "freeze"

    def test_split_dispatch(self, tmp_base) -> None:
        result = execute_topo_effect(
            SOURCE_SHA, "split", TARGET_SHA, "local", tmp_base,
        )
        assert result["executed"] is True
        assert result["block"]["content"]["action"] == "split"

    def test_sever_dispatch(self, tmp_base) -> None:
        result = execute_topo_effect(
            SOURCE_SHA, "sever", TARGET_SHA, "local", tmp_base,
        )
        assert result["executed"] is True
        assert result["block"]["content"]["action"] == "sever"

    def test_unknown_type(self, tmp_base) -> None:
        result = execute_topo_effect(
            SOURCE_SHA, "destroy", TARGET_SHA, "local", tmp_base,
        )
        assert result["executed"] is False
        assert result["block"] is None
        assert any("未知" in entry for entry in result["log"])


# ═══════════════════════════════════════════════════════════════
# compute_load_bearing_score — 从 relations.jsonl 构建图
# ═══════════════════════════════════════════════════════════════


def _seed_relations_graph(tmp_base, node_ids: list[str],
                          depends: list[tuple[str, str]],
                          negates: list[tuple[str, str]] | None = None):
    """种子关系数据（用于承重点测试）。

    depends: [(child, parent), ...]  — child depends_on parent
    negates: [(from, to), ...]
    """
    # 需要一个合法 created_by（任意 SHA256）
    creator = _make_sha("creator")
    # 写一个假的 creator block 使 created_by 合法
    make_block("event", "cc", {"seed": True}, [])

    jsonl_path = tmp_base / "relations.jsonl"
    lines = []
    for child, parent in depends:
        rel = make_relation(child, parent, "depends_on", order=1,
                            created_by=creator)
        lines.append(json.dumps(rel, ensure_ascii=False, separators=(",", ":")))
    for from_id, to_id in (negates or []):
        rel = make_relation(from_id, to_id, "negates", order=1,
                            created_by=creator)
        lines.append(json.dumps(rel, ensure_ascii=False, separators=(",", ":")))
    jsonl_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


class TestComputeLoadBearingScore:
    def test_basic_descendants(self, tmp_base) -> None:
        """A←B←C：A 有 2 个后代。"""
        a, b, c = _make_sha("A"), _make_sha("B"), _make_sha("C")
        _seed_relations_graph(tmp_base, [a, b, c],
                              depends=[(b, a), (c, b)])
        score = compute_load_bearing_score(a, tmp_base)
        assert score["descendants"] == 2

    def test_cascade_impact(self, tmp_base) -> None:
        """A negates B，B←C：A 的 cascade_impact = 2（B + C）。"""
        a, b, c = _make_sha("A"), _make_sha("B"), _make_sha("C")
        _seed_relations_graph(tmp_base, [a, b, c],
                              depends=[(c, b)],
                              negates=[(a, b)])
        score = compute_load_bearing_score(a, tmp_base)
        assert score["cascade_impact"] == 2  # B + C

    def test_threshold_calculation(self, tmp_base) -> None:
        """10 个拓扑参与者 → threshold = 1。"""
        nodes = [_make_sha(f"N{i}") for i in range(10)]
        # 链式依赖 N0←N1←...←N9
        deps = [(nodes[i + 1], nodes[i]) for i in range(9)]
        _seed_relations_graph(tmp_base, nodes, depends=deps)
        score = compute_load_bearing_score(nodes[0], tmp_base)
        assert score["threshold"] == 1  # 10 * 0.10 = 1
        assert score["descendants"] == 9
        assert score["is_load_bearing"] is True

    def test_leaf_node_not_load_bearing(self, tmp_base) -> None:
        """叶节点（无后代）不是承重点。"""
        a, b = _make_sha("A"), _make_sha("B")
        _seed_relations_graph(tmp_base, [a, b], depends=[(b, a)])
        score = compute_load_bearing_score(b, tmp_base)
        assert score["descendants"] == 0
        assert score["is_load_bearing"] is False

    def test_empty_relations(self, tmp_base) -> None:
        """空 relations → score=0。"""
        score = compute_load_bearing_score(_make_sha("X"), tmp_base)
        assert score["score"] == 0
        assert score["is_load_bearing"] is False


# ═══════════════════════════════════════════════════════════════
# resolve_genealogy_id — meta.json id 映射
# ═══════════════════════════════════════════════════════════════


class TestResolveGenealogyId:
    def test_found(self, tmp_base) -> None:
        write_meta({"id_mapping": {"062": TARGET_SHA}}, tmp_base)
        assert resolve_genealogy_id("062", tmp_base) == TARGET_SHA

    def test_not_found(self, tmp_base) -> None:
        write_meta({"id_mapping": {"062": TARGET_SHA}}, tmp_base)
        assert resolve_genealogy_id("999", tmp_base) is None

    def test_no_meta(self, tmp_base) -> None:
        assert resolve_genealogy_id("062", tmp_base) is None

    def test_no_id_mapping_key(self, tmp_base) -> None:
        write_meta({"version": 1}, tmp_base)
        assert resolve_genealogy_id("062", tmp_base) is None


# ═══════════════════════════════════════════════════════════════
# auto_execute_from_file — 端到端
# ═══════════════════════════════════════════════════════════════


def _make_genealogy_file(tmp_path, filename: str, content: str) -> str:
    """创建临时谱系文件并返回路径。"""
    path = tmp_path / filename
    path.write_text(content, encoding="utf-8")
    return str(path)


def _setup_meta_with_mapping(tmp_base, mapping: dict) -> None:
    """写入 meta.json 和 id_mapping。"""
    write_meta({"id_mapping": mapping}, tmp_base)


class TestAutoExecuteFromFile:
    def test_normal_execution(self, tmp_path, tmp_base) -> None:
        """正常执行：结构化 topo_effect + 非承重点 → executed=True。"""
        _setup_meta_with_mapping(
            tmp_base, {"147": SOURCE_SHA, "062": TARGET_SHA},
        )
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
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is True
        assert result["effect"] == "freeze:062:local"
        assert result["source_id"] == "147"
        assert result["load_bearing"] is False

    def test_topo_executed_at_written(self, tmp_path, tmp_base) -> None:
        """执行后谱系文件应有 topo_executed_at 字段。"""
        _setup_meta_with_mapping(
            tmp_base, {"147": SOURCE_SHA, "062": TARGET_SHA},
        )
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
        auto_execute_from_file(genealogy, tmp_base)
        content = (tmp_path / "147-topo.md").read_text(encoding="utf-8")
        assert "topo_executed_at:" in content

    def test_already_executed_skipped(self, tmp_path, tmp_base) -> None:
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
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert any("已执行" in entry for entry in result["log"])

    def test_no_topo_effect(self, tmp_path, tmp_base) -> None:
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
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert result["effect"] is None

    def test_descriptive_topo_effect_skipped(self, tmp_path, tmp_base) -> None:
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
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert result["effect"] is None

    def test_load_bearing_blocks_execution(self, tmp_path, tmp_base) -> None:
        """承重点 → 不执行，返回 load_bearing=True。"""
        # 构造使 target 成为承重点的关系图
        # target 有 12 个直接后代（13个节点，10%=1，score=12 >= 1）
        child_shas = [_make_sha(f"child-{i}") for i in range(12)]
        creator = _make_sha("creator")

        _setup_meta_with_mapping(
            tmp_base, {"099": SOURCE_SHA, "001": TARGET_SHA},
        )

        # 写入 depends_on 关系
        jsonl_path = tmp_base / "relations.jsonl"
        lines = []
        for child_sha in child_shas:
            rel = make_relation(child_sha, TARGET_SHA, "depends_on", order=1,
                                created_by=creator)
            lines.append(
                json.dumps(rel, ensure_ascii=False, separators=(",", ":"))
            )
        jsonl_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

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
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert result["load_bearing"] is True
        assert any("承重点" in entry for entry in result["log"])

    def test_nonexistent_genealogy(self, tmp_path, tmp_base) -> None:
        """谱系文件不存在 → 不执行。"""
        result = auto_execute_from_file(
            str(tmp_path / "nonexistent.md"), tmp_base,
        )
        assert result["executed"] is False
        assert any("不存在" in entry for entry in result["log"])

    def test_source_id_not_in_mapping(self, tmp_path, tmp_base) -> None:
        """source id 不在 id_mapping → 不执行。"""
        _setup_meta_with_mapping(tmp_base, {"062": TARGET_SHA})  # 无 147
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-no-source.md",
            textwrap.dedent("""\
                ---
                id: '147'
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert any("id_mapping" in entry for entry in result["log"])

    def test_target_id_not_in_mapping(self, tmp_path, tmp_base) -> None:
        """target id 不在 id_mapping → 不执行。"""
        _setup_meta_with_mapping(tmp_base, {"147": SOURCE_SHA})  # 无 062
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-no-target.md",
            textwrap.dedent("""\
                ---
                id: '147'
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is False
        assert any("id_mapping" in entry for entry in result["log"])

    def test_idempotent_double_call(self, tmp_path, tmp_base) -> None:
        """第二次调用应跳过（topo_executed_at 已写入）。"""
        _setup_meta_with_mapping(
            tmp_base, {"147": SOURCE_SHA, "062": TARGET_SHA},
        )
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
        r1 = auto_execute_from_file(genealogy, tmp_base)
        assert r1["executed"] is True
        r2 = auto_execute_from_file(genealogy, tmp_base)
        assert r2["executed"] is False
        assert any("已执行" in entry for entry in r2["log"])

    def test_writes_block_topology(self, tmp_path, tmp_base) -> None:
        """端到端：执行后 block-topology 中存在 event 区块和 freezes 关系。"""
        _setup_meta_with_mapping(
            tmp_base, {"147": SOURCE_SHA, "062": TARGET_SHA},
        )
        genealogy = _make_genealogy_file(
            tmp_path,
            "147-e2e.md",
            textwrap.dedent("""\
                ---
                id: '147'
                topo_effect: "freeze:062:local"
                ---

                # 147
            """),
        )
        result = auto_execute_from_file(genealogy, tmp_base)
        assert result["executed"] is True

        # 验证区块存在
        rels = read_all_relations(tmp_base)
        freeze_rels = [r for r in rels if r["relation"] == "freezes"]
        assert len(freeze_rels) >= 1
        assert freeze_rels[0]["to"] == TARGET_SHA
