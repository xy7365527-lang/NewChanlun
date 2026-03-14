"""test_snet_params.py — S_net 参数节点化测试.

覆盖 442号谱系的三条下游推论：
  442-1: 参数节点注册
  442-2: 参数变更检测
  442-3: 自修改安全边界

认识论等级：L1（合成数据验证管线正确性）
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from signifier_net import SNet, Signifier
from snet_params import (
    SNET_PARAM_SPECS,
    register_param_nodes,
    detect_param_negation,
    apply_param_change,
    get_current_params,
    validate_param_value,
    _param_node_id,
    ParamChangeEvent,
)


# ---------------------------------------------------------------------------
# 442-1: Parameter node registration
# ---------------------------------------------------------------------------

def test_register_param_nodes_default():
    snet = SNet()
    new_snet, registered = register_param_nodes(snet)

    assert len(registered) == len(SNET_PARAM_SPECS)
    for spec in SNET_PARAM_SPECS:
        node_id = _param_node_id(spec.name, spec.default)
        assert node_id in registered
        assert new_snet.has_signifier(node_id)

        # Check signifier properties
        sig = new_snet.signifiers[node_id]
        assert sig.source == "snet_parameter"
        assert sig.domain == "snet_config"
        assert spec.description in sig.surface_forms


def test_register_param_nodes_custom_values():
    snet = SNet()
    custom = {"degree_alpha": 0.8, "pmi_threshold": 1.5}
    new_snet, registered = register_param_nodes(snet, param_values=custom)

    # Custom values should be used
    assert _param_node_id("degree_alpha", 0.8) in registered
    assert _param_node_id("pmi_threshold", 1.5) in registered

    # Non-custom params should use defaults
    for spec in SNET_PARAM_SPECS:
        if spec.name not in custom:
            assert _param_node_id(spec.name, spec.default) in registered


def test_register_param_nodes_preserves_existing():
    snet = SNet()
    snet = snet.add_signifier(Signifier(id="existing_term", source="test"))

    new_snet, _ = register_param_nodes(snet)
    assert new_snet.has_signifier("existing_term")


def test_register_param_nodes_immutable():
    snet = SNet()
    original_count = len(snet.signifiers)
    new_snet, _ = register_param_nodes(snet)

    # Original should not be modified
    assert len(snet.signifiers) == original_count
    assert len(new_snet.signifiers) > original_count


# ---------------------------------------------------------------------------
# 442-2: Parameter negation detection
# ---------------------------------------------------------------------------

def test_detect_param_negation_basic():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    old_id = _param_node_id("degree_alpha", 0.5)
    event = detect_param_negation(snet, old_id, 0.8)

    assert event is not None
    assert event.param_name == "degree_alpha"
    assert event.old_value == 0.5
    assert event.new_value == 0.8
    assert event.old_node_id == old_id
    assert event.new_node_id == _param_node_id("degree_alpha", 0.8)
    assert event.requires_recalc is True


def test_detect_param_negation_not_param_node():
    snet = SNet()
    event = detect_param_negation(snet, "regular_signifier", 1.0)
    assert event is None


def test_detect_param_negation_same_value():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    old_id = _param_node_id("degree_alpha", 0.5)
    event = detect_param_negation(snet, old_id, 0.5)
    assert event is None  # No change


def test_detect_param_negation_safety_clamp():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    old_id = _param_node_id("degree_alpha", 0.5)
    # Proposed value exceeds max (2.0)
    event = detect_param_negation(snet, old_id, 5.0)

    assert event is not None
    assert event.new_value == 2.0  # Clamped to max


def test_apply_param_change():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    old_id = _param_node_id("degree_alpha", 0.5)
    event = detect_param_negation(snet, old_id, 0.8)
    assert event is not None

    new_snet = apply_param_change(snet, event)

    # New node should exist
    assert new_snet.has_signifier(event.new_node_id)
    # Old node should still exist (history)
    assert new_snet.has_signifier(old_id)


# ---------------------------------------------------------------------------
# 442-3: Safety boundaries
# ---------------------------------------------------------------------------

def test_validate_param_value_in_range():
    result = validate_param_value("degree_alpha", 0.7)
    assert result == 0.7


def test_validate_param_value_below_min():
    result = validate_param_value("degree_alpha", -1.0)
    assert result == 0.0  # Clamped to min


def test_validate_param_value_above_max():
    result = validate_param_value("degree_alpha", 10.0)
    assert result == 2.0  # Clamped to max


def test_validate_param_value_unknown_param():
    result = validate_param_value("nonexistent_param", 1.0)
    assert result is None


def test_validate_param_value_int_type():
    result = validate_param_value("neighbors_per_concept", 5)
    assert result == 5
    assert isinstance(result, int)


def test_validate_param_value_int_clamped():
    result = validate_param_value("neighbors_per_concept", 0)
    assert result == 1  # Clamped to min


def test_validate_param_value_type_mismatch():
    # Float param with int input should still work
    result = validate_param_value("degree_alpha", 1)
    assert result == 1.0


# ---------------------------------------------------------------------------
# get_current_params
# ---------------------------------------------------------------------------

def test_get_current_params_default():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    params = get_current_params(snet)

    for spec in SNET_PARAM_SPECS:
        assert spec.name in params
        assert params[spec.name] == spec.default


def test_get_current_params_empty_snet():
    snet = SNet()
    params = get_current_params(snet)

    # Should return defaults for all params
    for spec in SNET_PARAM_SPECS:
        assert spec.name in params
        assert params[spec.name] == spec.default


def test_get_current_params_after_negation():
    snet = SNet()
    snet, _ = register_param_nodes(snet)

    # Negate degree_alpha from 0.5 to 0.8
    old_id = _param_node_id("degree_alpha", 0.5)
    event = detect_param_negation(snet, old_id, 0.8)
    assert event is not None
    snet = apply_param_change(snet, event)

    params = get_current_params(snet)
    # After negation, the latest value should be 0.8
    assert params["degree_alpha"] == 0.8


# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))
