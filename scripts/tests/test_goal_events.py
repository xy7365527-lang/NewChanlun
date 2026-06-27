"""D′ 开口① writer：goal_events.append_event 严格 schema 校验（拒绝退化写入）。

写路径退化（630 开口①）的根因：actor 手搓叙事事件 append，关键结构化事件退化
（GOAL_SET 丢 acceptance/base_head、DECOMPOSE 丢 sub_goals[]、SUPERSEDE 丢
old_goal_id/new_goal_id）。本 writer 是 events.jsonl 的唯一合法写者：在 append 前
按 SCHEMA.md 强制校验，退化事件 raise ValueError（no-patch：不容退化 schema 通过，
fail fast 在系统边界）。

测试分两类：
1. 合法规范事件 → 写入成功，回读 JSON 一致；
2. 退化/非法事件 → ValueError，文件不被污染（append-only 不写半截）。
"""
import json

import pytest

from goal_events import append_event, validate_event


# ── 合法事件：每种 SCHEMA 事件类型的规范形态 ──────────────────────────────

def _read_lines(p):
    with open(p, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def test_goal_set_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event(
        "GOAL_SET",
        ev_path=str(ev_path),
        goal_id="g1",
        description="形式化",
        acceptance=[{"check": "lake build 绿", "falsifiable": True}],
        base_head="abc123",
    )
    lines = _read_lines(ev_path)
    assert len(lines) == 1
    e = lines[0]
    assert e["event"] == "GOAL_SET"
    assert e["goal_id"] == "g1"
    assert e["base_head"] == "abc123"
    assert e["acceptance"][0]["falsifiable"] is True
    assert "ts" in e  # writer 自动盖时间戳


def test_decompose_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event(
        "DECOMPOSE",
        ev_path=str(ev_path),
        goal_id="g1",
        sub_goals=[
            {"id": "g1.1", "desc": "a", "blocked_by": []},
            {"id": "g1.2", "desc": "b", "blocked_by": ["g1.1"]},
        ],
    )
    e = _read_lines(ev_path)[0]
    assert e["event"] == "DECOMPOSE"
    assert len(e["sub_goals"]) == 2
    assert e["sub_goals"][1]["blocked_by"] == ["g1.1"]


def test_supersede_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event("SUPERSEDE", ev_path=str(ev_path), old_goal_id="g1", new_goal_id="g2")
    e = _read_lines(ev_path)[0]
    assert e["old_goal_id"] == "g1"
    assert e["new_goal_id"] == "g2"


def test_check_pass_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done")
    e = _read_lines(ev_path)[0]
    assert e["sub_goal_id"] == "g1.1"
    assert e["check"] == "done"


def test_blocked_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event("BLOCKED", ev_path=str(ev_path), sub_goal_id="g1.1", blocker="缺数据")
    e = _read_lines(ev_path)[0]
    assert e["blocker"] == "缺数据"


def test_evidence_valid_appends(tmp_path):
    # EVIDENCE 本就是叙事 schema（sub_goal_id + artifact），合法。
    ev_path = tmp_path / "events.jsonl"
    append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1", artifact="commit abc")
    e = _read_lines(ev_path)[0]
    assert e["artifact"] == "commit abc"


def test_closed_valid_appends(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event("CLOSED", ev_path=str(ev_path), goal_id="g1")
    e = _read_lines(ev_path)[0]
    assert e["event"] == "CLOSED"
    assert e["goal_id"] == "g1"


def test_append_is_additive(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"check": "c", "falsifiable": True}], base_head="h")
    append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1", artifact="a")
    assert len(_read_lines(ev_path)) == 2  # append-only，不覆盖


def test_caller_ts_preserved(tmp_path):
    # 调用方显式给 ts → 保留（补历史事件用），不强制覆盖。
    ev_path = tmp_path / "events.jsonl"
    append_event("CLOSED", ev_path=str(ev_path), goal_id="g1", ts="2026-06-27T00:00:00Z")
    assert _read_lines(ev_path)[0]["ts"] == "2026-06-27T00:00:00Z"


# ── 退化/非法事件：必须 ValueError，且文件不被污染 ────────────────────────

def test_goal_set_missing_acceptance_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="acceptance"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1",
                     description="x", base_head="h")
    assert not ev_path.exists() or _read_lines(ev_path) == []  # 未污染


def test_goal_set_empty_acceptance_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="acceptance"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1",
                     description="x", acceptance=[], base_head="h")


def test_goal_set_non_falsifiable_acceptance_rejected(tmp_path):
    # acceptance 项缺 falsifiable=true → 拒绝（lesson 0011：永动空转防护）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="falsifiable"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"check": "差不多", "falsifiable": False}], base_head="h")


def test_goal_set_acceptance_missing_check_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="check"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"falsifiable": True}], base_head="h")


def test_goal_set_missing_base_head_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="base_head"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"check": "c", "falsifiable": True}])


def test_goal_set_missing_goal_id_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="goal_id"):
        append_event("GOAL_SET", ev_path=str(ev_path), description="x",
                     acceptance=[{"check": "c", "falsifiable": True}], base_head="h")


def test_decompose_missing_sub_goals_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="sub_goals"):
        append_event("DECOMPOSE", ev_path=str(ev_path), goal_id="g1")


def test_decompose_empty_sub_goals_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="sub_goals"):
        append_event("DECOMPOSE", ev_path=str(ev_path), goal_id="g1", sub_goals=[])


def test_decompose_sub_goal_missing_id_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="id"):
        append_event("DECOMPOSE", ev_path=str(ev_path), goal_id="g1",
                     sub_goals=[{"desc": "a", "blocked_by": []}])


def test_decompose_sub_goal_missing_desc_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="desc"):
        append_event("DECOMPOSE", ev_path=str(ev_path), goal_id="g1",
                     sub_goals=[{"id": "g1.1", "blocked_by": []}])


def test_decompose_blocked_by_not_list_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="blocked_by"):
        append_event("DECOMPOSE", ev_path=str(ev_path), goal_id="g1",
                     sub_goals=[{"id": "g1.1", "desc": "a", "blocked_by": "g1.0"}])


def test_supersede_missing_new_goal_id_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="new_goal_id"):
        append_event("SUPERSEDE", ev_path=str(ev_path), old_goal_id="g1")


def test_check_pass_missing_check_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="check"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1")


def test_blocked_missing_blocker_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="blocker"):
        append_event("BLOCKED", ev_path=str(ev_path), sub_goal_id="g1.1")


def test_evidence_missing_artifact_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="artifact"):
        append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1")


def test_unknown_event_type_rejected(tmp_path):
    # SCHEMA.md 外的事件类型（CEREMONY/ESCALATE/ROUTING 等手搓叙事）→ 拒绝。
    # no-patch：writer 只写 SCHEMA 定义的 7 种事件，不容退化事件类型扩散。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="event"):
        append_event("CEREMONY", ev_path=str(ev_path), sub_goal_id="g1", artifact="叙事")


def test_extra_fields_rejected(tmp_path):
    # 规范事件 + 多余字段 → 拒绝（防止 sub_goal_id 误用在 GOAL_SET 等退化模式渗入）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="unexpected|多余|未知"):
        append_event("CLOSED", ev_path=str(ev_path), goal_id="g1", sub_goal_id="混入")


def test_non_string_ts_rejected(tmp_path):
    # CRITICAL-1：ts 非字符串（dict/list/int）→ 拒绝。否则绕过必填校验与多余字段检测，
    # 污染 events.jsonl 让下游按字符串消费 ts 时类型错乱。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="ts"):
        append_event("CLOSED", ev_path=str(ev_path), goal_id="g1", ts={"bad": True})
    with pytest.raises(ValueError, match="ts"):
        append_event("CLOSED", ev_path=str(ev_path), goal_id="g1", ts=12345)
    with pytest.raises(ValueError, match="ts"):
        append_event("CLOSED", ev_path=str(ev_path), goal_id="g1", ts="")


def test_validate_event_pure_function():
    # validate_event 是纯函数：合法返回规范化 dict，非法 raise。可独立测，无 IO。
    e = validate_event("CLOSED", {"goal_id": "g1", "ts": "t0"})
    assert e == {"event": "CLOSED", "goal_id": "g1", "ts": "t0"}
    with pytest.raises(ValueError):
        validate_event("GOAL_SET", {"goal_id": "g1"})
