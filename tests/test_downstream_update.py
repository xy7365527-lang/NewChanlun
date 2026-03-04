"""tests for scripts/downstream_update.py — 下游推论状态更新工具。"""
from __future__ import annotations

import json
import os
import textwrap

import pytest
import yaml

from scripts.downstream_update import (
    batch_update,
    resolve_downstream,
    update_status,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════

GENEALOGY_WITH_YAML_DOWNSTREAM = textwrap.dedent("""\
    ---
    id: '330'
    number: 330
    title: "K4拓扑与资本循环映射"
    status: 已结算
    downstream_inferences:
      - id: "330-1"
        description: "金融循环断裂位点→边冻结模式"
        status: "unresolved"
      - id: "330-2"
        description: "Equity/Commodity长期趋势"
        status: "unresolved"
      - id: "330-3"
        description: "Au/$长期趋势"
        status: "unresolved"
      - id: "330-4"
        description: "K4不应被用于诊断向心坍缩的原因"
        status: "unresolved"
    ---

    # 330号：K4拓扑与资本循环映射

    ## 下游推论

    1. **金融循环断裂位点→边冻结模式**（可检验）
    2. **Equity/Commodity长期趋势**（可监控）
    3. **Au/$长期趋势**（可监控）
    4. **K4不应被用于诊断向心坍缩的原因**：管辖范围约束
""")

GENEALOGY_WITHOUT_YAML_DOWNSTREAM = textwrap.dedent("""\
    ---
    id: '335'
    number: 335
    title: "测试谱系"
    status: 已结算
    ---

    # 335号：测试谱系

    ## 下游推论

    1. **推论一**：可检验
    2. **推论二**：可监控
""")


@pytest.fixture()
def project_root(tmp_path):
    """创建包含谱系文件和 overrides 的临时项目结构。"""
    settled_dir = tmp_path / ".chanlun" / "genealogy" / "settled"
    settled_dir.mkdir(parents=True)

    # 330号——有 YAML downstream_inferences
    with open(settled_dir / "330-k4-capital-circuit-mapping.md", "w",
              encoding="utf-8") as f:
        f.write(GENEALOGY_WITH_YAML_DOWNSTREAM)

    # 335号——无 YAML downstream_inferences
    with open(settled_dir / "335-test-genealogy.md", "w",
              encoding="utf-8") as f:
        f.write(GENEALOGY_WITHOUT_YAML_DOWNSTREAM)

    # 创建空的 overrides 文件
    chanlun_dir = tmp_path / ".chanlun"
    with open(chanlun_dir / "downstream-action-overrides.yaml", "w",
              encoding="utf-8") as f:
        f.write("# 下游行动手动覆盖\n")

    return str(tmp_path)


# ═══════════════════════════════════════════════════════════════
# resolve_downstream 测试
# ═══════════════════════════════════════════════════════════════


def test_resolve_via_yaml_frontmatter(project_root):
    """有 YAML frontmatter 的谱系，resolve 应就地更新 YAML。"""
    result = resolve_downstream(
        project_root, "330", 4, "范畴约束已内化为管辖范围规则")
    assert result["status"] == "resolved"
    assert result["method"] == "yaml_frontmatter"

    # 验证文件已更新
    filepath = os.path.join(
        project_root, ".chanlun", "genealogy", "settled",
        "330-k4-capital-circuit-mapping.md")
    with open(filepath, encoding="utf-8") as f:
        content = f.read()
    assert "resolved" in content
    assert "范畴约束已内化为管辖范围规则" in content


def test_resolve_fallback_to_overrides(project_root):
    """无 YAML downstream_inferences 的谱系，应 fallback 到 overrides。"""
    result = resolve_downstream(
        project_root, "335", 1, "推论已被后续谱系覆盖")
    assert result["status"] == "resolved"
    assert result["method"] == "overrides"

    # 验证 overrides 文件已更新
    override_path = os.path.join(
        project_root, ".chanlun", "downstream-action-overrides.yaml")
    with open(override_path, encoding="utf-8") as f:
        content = f.read()
    assert "335-1: resolved" in content
    assert "推论已被后续谱系覆盖" in content


def test_resolve_nonexistent_genealogy_uses_overrides(project_root):
    """不存在的谱系编号应 fallback 到 overrides。"""
    result = resolve_downstream(
        project_root, "999", 1, "测试")
    assert result["status"] == "resolved"
    assert result["method"] == "overrides"


def test_resolve_preserves_other_downstream_statuses(project_root):
    """更新一条时不应影响其他下游推论。"""
    resolve_downstream(
        project_root, "330", 4, "范畴约束")

    filepath = os.path.join(
        project_root, ".chanlun", "genealogy", "settled",
        "330-k4-capital-circuit-mapping.md")
    with open(filepath, encoding="utf-8") as f:
        content = f.read()

    # 330-1/2/3 应保持 unresolved
    fm_end = content.find("\n---", 3)
    fm_text = content[3:fm_end]
    fm = yaml.safe_load(fm_text)
    inferences = fm["downstream_inferences"]
    assert inferences[0]["status"] == "unresolved"
    assert inferences[1]["status"] == "unresolved"
    assert inferences[2]["status"] == "unresolved"
    assert "resolved" in inferences[3]["status"]


# ═══════════════════════════════════════════════════════════════
# update_status 测试
# ═══════════════════════════════════════════════════════════════


def test_update_status_blocked(project_root):
    """标记下游推论为 blocked。"""
    result = update_status(
        project_root, "330", 1, "blocked", "需新体制数据")
    assert result["status"] == "blocked"


def test_update_status_invalid_raises(project_root):
    """无效状态应抛出 ValueError。"""
    with pytest.raises(ValueError, match="无效状态"):
        update_status(project_root, "330", 1, "invalid_status", "test")


def test_update_status_superseded(project_root):
    """标记下游推论为 superseded。"""
    result = update_status(
        project_root, "335", 2, "superseded", "被新谱系覆盖")
    assert result["status"] == "superseded"


# ═══════════════════════════════════════════════════════════════
# batch_update 测试
# ═══════════════════════════════════════════════════════════════


def test_batch_update_mixed(project_root):
    """批量更新：一条 resolve + 一条 blocked。"""
    updates = [
        {"genealogy_id": "330", "action_index": 4,
         "status": "resolved", "reason": "管辖范围"},
        {"genealogy_id": "335", "action_index": 1,
         "status": "blocked", "reason": "阻塞中"},
    ]
    results = batch_update(project_root, updates)
    assert len(results) == 2
    assert results[0]["status"] == "resolved"
    assert results[1]["status"] == "blocked"


def test_batch_update_error_handling(project_root):
    """批量更新中的错误不阻塞其他条目。"""
    updates = [
        {"genealogy_id": "330", "action_index": 1,
         "status": "invalid", "reason": "test"},
        {"genealogy_id": "330", "action_index": 2,
         "status": "resolved", "reason": "ok"},
    ]
    results = batch_update(project_root, updates)
    assert results[0]["status"] == "error"
    assert results[1]["status"] == "resolved"


# ═══════════════════════════════════════════════════════════════
# overrides 追加格式测试
# ═══════════════════════════════════════════════════════════════


def test_overrides_creates_section_header(project_root):
    """新谱系编号应创建 section header。"""
    resolve_downstream(project_root, "400", 1, "测试")

    override_path = os.path.join(
        project_root, ".chanlun", "downstream-action-overrides.yaml")
    with open(override_path, encoding="utf-8") as f:
        content = f.read()
    assert "# === 400号 ===" in content
    assert "400-1: resolved" in content


def test_overrides_updates_existing_entry(project_root):
    """已存在的条目应被更新而非追加。"""
    resolve_downstream(project_root, "335", 1, "第一次")
    update_status(project_root, "335", 1, "blocked", "改为阻塞")

    override_path = os.path.join(
        project_root, ".chanlun", "downstream-action-overrides.yaml")
    with open(override_path, encoding="utf-8") as f:
        content = f.read()

    # 应该只有一条 335-1
    assert content.count("335-1:") == 1
    assert "blocked" in content
