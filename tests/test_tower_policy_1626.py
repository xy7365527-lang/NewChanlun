"""#1626：tower_base 的 Python 记录读取与递归底消费。"""

import json
from unittest.mock import patch

import pytest

from newchan.a_nested_divergence import nested_divergence_search
from newchan.tower_policy import TowerBasePolicy, TowerPolicyError, active_tower_base, parse_tower_base


def test_active_tower_base_is_segment_with_version() -> None:
    policy = active_tower_base()
    assert policy.tower_base == "segment"
    assert policy.base_level == 1
    assert policy.label == "tower-base/1"


@pytest.mark.parametrize(
    "fields, expected",
    [
        ({}, "tower_base"),
        ({"tower_base": {"value": None}}, "tower_base"),
        ({"tower_base": {"value": "AwaitingPolicy"}}, "tower_base"),
        ({"tower_base": {"value": "stroke"}}, "tower_base"),
    ],
)
def test_missing_unsigned_or_invalid_never_falls_back(fields: dict, expected: str) -> None:
    text = json.dumps({"id": "tower-base", "version": 1, "fields": fields})
    with pytest.raises(TowerPolicyError, match=expected):
        parse_tower_base(text)


@pytest.mark.parametrize("value", ["kline3", "trade3"])
def test_alternate_base_is_typed_reject_not_fake_segment(value: str) -> None:
    text = json.dumps({"id": "tower-base", "version": 2, "fields": {"tower_base": {"value": value}}})
    policy = parse_tower_base(text)
    assert policy.tower_base == value
    assert policy.label == "tower-base/2"
    with pytest.raises(TowerPolicyError, match="尚无真实构件"):
        _ = policy.base_level


def test_nested_search_consumes_active_record_before_recursion() -> None:
    policy = TowerBasePolicy(record_id="tower-base", version=7, tower_base="kline3")
    with patch("newchan.a_nested_divergence.active_tower_base", return_value=policy) as active:
        with pytest.raises(TowerPolicyError, match="尚无真实构件"):
            nested_divergence_search(object())
    active.assert_called_once_with()


def test_nested_search_default_uses_current_record_without_changing_results() -> None:
    snap = type("Snap", (), {"recursive_snapshots": []})()
    with patch("newchan.a_nested_divergence.active_tower_base", wraps=active_tower_base) as active:
        assert nested_divergence_search(snap) == []
    active.assert_called_once_with()
