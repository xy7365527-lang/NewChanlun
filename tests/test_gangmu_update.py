"""tests for scripts/gangmu_update.py — 纲目动态更新工具。"""
from __future__ import annotations

import json
import os
import textwrap

import pytest
import yaml

from scripts.gangmu_update import (
    append_action,
    apply_proposed_transitions,
    complete_action,
    transition_mu,
)


# ═══════════════════════════════════════════════════════════════
# Fixtures
# ═══════════════════════════════════════════════════════════════

MINIMAL_GANGMU = {
    "version": "1.0",
    "gang": [
        {
            "id": "test-gang",
            "name": "测试纲",
            "mu": [
                {
                    "id": "test-mu-active",
                    "name": "活跃目",
                    "status": "active",
                    "opened_at": "100",
                    "next_actions": [
                        {
                            "type": "engineering",
                            "target": "action-a",
                            "description": "测试动作A",
                            "blocked_by": None,
                            "completion_check": {
                                "type": "file_exists",
                                "path": "scripts/test_file.py",
                            },
                        },
                        {
                            "type": "engineering",
                            "target": "action-b",
                            "description": "测试动作B",
                            "blocked_by": "前置条件未满足",
                            "completion_check": {
                                "type": "file_exists",
                                "path": "scripts/other_file.py",
                            },
                        },
                    ],
                },
                {
                    "id": "test-mu-closed",
                    "name": "已关闭目",
                    "status": "closed",
                    "next_actions": [],
                },
                {
                    "id": "test-mu-empty",
                    "name": "空动作目",
                    "status": "active",
                    "opened_at": "200",
                    "next_actions": [],
                },
            ],
        },
    ],
}


@pytest.fixture()
def gangmu_file(tmp_path):
    """创建临时 gangmu.yaml 并返回路径。"""
    gangmu_dir = tmp_path / ".chanlun"
    gangmu_dir.mkdir(parents=True)
    gangmu_path = gangmu_dir / "gangmu.yaml"
    with open(gangmu_path, "w", encoding="utf-8") as f:
        yaml.dump(MINIMAL_GANGMU, f, allow_unicode=True,
                  default_flow_style=False, sort_keys=False)
    return str(gangmu_path)


def _reload(gangmu_path):
    """重新读取 gangmu.yaml。"""
    with open(gangmu_path, encoding="utf-8") as f:
        return yaml.safe_load(f)


# ═══════════════════════════════════════════════════════════════
# complete_action 测试
# ═══════════════════════════════════════════════════════════════


def test_complete_action_marks_completed_at(gangmu_file):
    """完成动作后，action 应有 completed_at 字段。"""
    result = complete_action(gangmu_file, "test-mu-active", "action-a")
    assert result["status"] == "completed"
    assert result["target"] == "action-a"

    data = _reload(gangmu_file)
    mu = data["gang"][0]["mu"][0]
    action = mu["next_actions"][0]
    assert "completed_at" in action
    assert action["target"] == "action-a"


def test_complete_action_idempotent(gangmu_file):
    """重复完成同一动作应返回 already_completed。"""
    complete_action(gangmu_file, "test-mu-active", "action-a")
    result = complete_action(gangmu_file, "test-mu-active", "action-a")
    assert result["status"] == "already_completed"


def test_complete_action_nonexistent_target_raises(gangmu_file):
    """完成不存在的 target 应抛出 ValueError。"""
    with pytest.raises(ValueError, match="未找到 target=nonexistent"):
        complete_action(gangmu_file, "test-mu-active", "nonexistent")


def test_complete_action_nonexistent_mu_raises(gangmu_file):
    """完成不存在的目应抛出 ValueError。"""
    with pytest.raises(ValueError, match="未找到目"):
        complete_action(gangmu_file, "nonexistent-mu", "action-a")


# ═══════════════════════════════════════════════════════════════
# append_action 测试
# ═══════════════════════════════════════════════════════════════


def test_append_action_adds_to_next_actions(gangmu_file):
    """追加动作后，next_actions 应增长。"""
    new_action = {
        "target": "action-c",
        "description": "新追加的动作",
        "completion_check": {"type": "file_exists", "path": "new_file.py"},
    }
    result = append_action(gangmu_file, "test-mu-active", new_action)
    assert result["status"] == "appended"

    data = _reload(gangmu_file)
    mu = data["gang"][0]["mu"][0]
    assert len(mu["next_actions"]) == 3
    assert mu["next_actions"][2]["target"] == "action-c"
    # 默认值检查
    assert mu["next_actions"][2]["type"] == "engineering"
    assert mu["next_actions"][2]["blocked_by"] is None


def test_append_action_to_empty_list(gangmu_file):
    """向空 next_actions 追加动作。"""
    new_action = {
        "target": "first-action",
        "completion_check": {"type": "file_exists", "path": "x.py"},
    }
    result = append_action(gangmu_file, "test-mu-empty", new_action)
    assert result["status"] == "appended"

    data = _reload(gangmu_file)
    mu = data["gang"][0]["mu"][2]
    assert len(mu["next_actions"]) == 1


def test_append_action_duplicate_raises(gangmu_file):
    """追加重复 target 应抛出 ValueError。"""
    dup = {
        "target": "action-a",
        "completion_check": {"type": "file_exists", "path": "x.py"},
    }
    with pytest.raises(ValueError, match="已存在"):
        append_action(gangmu_file, "test-mu-active", dup)


def test_append_action_missing_required_fields_raises(gangmu_file):
    """追加缺少必需字段的动作应抛出 ValueError。"""
    incomplete = {"target": "action-x"}
    with pytest.raises(ValueError, match="缺少必需字段"):
        append_action(gangmu_file, "test-mu-active", incomplete)


# ═══════════════════════════════════════════════════════════════
# transition_mu 测试
# ═══════════════════════════════════════════════════════════════


def test_transition_active_to_closed(gangmu_file):
    """active → closed 应设置 closed_at 和 closed_reason。"""
    result = transition_mu(
        gangmu_file, "test-mu-active", "closed", "所有动作已完成")
    assert result["status"] == "transitioned"
    assert result["from"] == "active"
    assert result["to"] == "closed"

    data = _reload(gangmu_file)
    mu = data["gang"][0]["mu"][0]
    assert mu["status"] == "closed"
    assert "closed_at" in mu
    assert mu["closed_reason"] == "所有动作已完成"


def test_transition_active_to_blocked(gangmu_file):
    """active → blocked 应设置 blocked_by。"""
    result = transition_mu(
        gangmu_file, "test-mu-active", "blocked", "等待外部数据")
    assert result["to"] == "blocked"

    data = _reload(gangmu_file)
    mu = data["gang"][0]["mu"][0]
    assert mu["status"] == "blocked"
    assert mu["blocked_by"] == "等待外部数据"


def test_transition_no_change(gangmu_file):
    """相同状态转换应返回 no_change。"""
    result = transition_mu(
        gangmu_file, "test-mu-active", "active", "无变化")
    assert result["status"] == "no_change"


def test_transition_invalid_status_raises(gangmu_file):
    """无效状态应抛出 ValueError。"""
    with pytest.raises(ValueError, match="无效的目标状态"):
        transition_mu(gangmu_file, "test-mu-active", "invalid", "test")


# ═══════════════════════════════════════════════════════════════
# apply_proposed_transitions 测试
# ═══════════════════════════════════════════════════════════════


def test_apply_proposed_transitions_batch(gangmu_file):
    """批量执行 proposed_transitions。"""
    transitions = [
        {"line": "test-mu-active", "from": "active", "to": "closed",
         "reason": "全部完成"},
        {"line": "test-mu-empty", "from": "active", "to": "blocked",
         "reason": "阻塞"},
    ]
    results = apply_proposed_transitions(gangmu_file, transitions)
    assert len(results) == 2
    assert results[0]["status"] == "transitioned"
    assert results[1]["status"] == "transitioned"

    data = _reload(gangmu_file)
    assert data["gang"][0]["mu"][0]["status"] == "closed"
    assert data["gang"][0]["mu"][2]["status"] == "blocked"


def test_apply_proposed_transitions_skips_invalid(gangmu_file):
    """缺少字段的 transition 应被跳过。"""
    transitions = [
        {"line": "", "to": "closed", "reason": "test"},
    ]
    results = apply_proposed_transitions(gangmu_file, transitions)
    assert results[0]["status"] == "skipped"


# ═══════════════════════════════════════════════════════════════
# 原子性写入测试
# ═══════════════════════════════════════════════════════════════


def test_file_not_found_raises(tmp_path):
    """不存在的 gangmu.yaml 应抛出 FileNotFoundError。"""
    fake_path = str(tmp_path / ".chanlun" / "gangmu.yaml")
    with pytest.raises(FileNotFoundError):
        complete_action(fake_path, "test", "action")
