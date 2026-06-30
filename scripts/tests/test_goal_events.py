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

from goal_events import (
    acceptance_hash,
    acceptance_vector_hash,
    append_event,
    validate_event,
)


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


def test_check_pass_auto_valid_appends(tmp_path):
    # CHECK_PASS 必带来源（650）：method=auto + command + verifier。
    ev_path = tmp_path / "events.jsonl"
    append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                 method="auto", command="cargo test --lib", verifier="ci")
    e = _read_lines(ev_path)[0]
    assert e["sub_goal_id"] == "g1.1"
    assert e["check"] == "done"
    assert e["method"] == "auto"
    assert e["command"] == "cargo test --lib"
    assert e["verifier"] == "ci"


def test_check_pass_manual_valid_appends(tmp_path):
    # CHECK_PASS method=manual + judge + rationale + 非空 evidence_ids（须先有 EVIDENCE 可指）。
    ev_path = tmp_path / "events.jsonl"
    append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1",
                 artifact="证据材料", evidence_id="ev-1")
    append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                 method="manual", judge="orchestrator", rationale="证据齐备",
                 evidence_ids=["ev-1"], acceptance_id="acc-1")
    e = _read_lines(ev_path)[1]
    assert e["method"] == "manual"
    assert e["evidence_ids"] == ["ev-1"]
    assert e["acceptance_id"] == "acc-1"


def test_check_fail_valid_appends(tmp_path):
    # CHECK_FAIL（658 修复）：撤销 CHECK_PASS，必带 reason。可带 acceptance_id（稳定身份撤销）。
    ev_path = tmp_path / "events.jsonl"
    append_event("CHECK_FAIL", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                 acceptance_id="acc-1", reason="codex 异质审 underpowered")
    e = _read_lines(ev_path)[0]
    assert e["event"] == "CHECK_FAIL"
    assert e["sub_goal_id"] == "g1.1"
    assert e["acceptance_id"] == "acc-1"
    assert e["reason"] == "codex 异质审 underpowered"


def test_check_fail_requires_reason(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="reason"):
        append_event("CHECK_FAIL", ev_path=str(ev_path), sub_goal_id="g1.1", check="done")


def test_check_fail_evidence_ids_must_exist(tmp_path):
    # evidence_ids 若提供，须真指向既有 EVIDENCE（引用完整性，与 CHECK_PASS manual 一致）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="不存在的 EVIDENCE"):
        append_event("CHECK_FAIL", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                     reason="争议", evidence_ids=["ev-nonexistent"])


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
    # SCHEMA.md 外的事件类型（CEREMONY/ESCALATE/ROUTING/DECISION 等手搓治理/叙事）→ 拒绝。
    # no-patch：writer 只写 SCHEMA 定义的 8 种事件，不容退化事件类型扩散。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="event"):
        append_event("CEREMONY", ev_path=str(ev_path), sub_goal_id="g1", artifact="叙事")
    with pytest.raises(ValueError, match="event"):
        append_event("DECISION", ev_path=str(ev_path), goal_id="g1", artifact="裁决")


# ── 650 提升协议：GOAL_RESUME / CHECK_PASS 来源 / EVIDENCE evidence_id ────────

def test_goal_resume_valid_appends(tmp_path):
    # GOAL_RESUME 纯审计事件（650）：goal_id + note，不带 base_head。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_RESUME", ev_path=str(ev_path), goal_id="g1", note="session 恢复续跑")
    e = _read_lines(ev_path)[0]
    assert e["event"] == "GOAL_RESUME"
    assert e["goal_id"] == "g1"
    assert e["note"] == "session 恢复续跑"
    assert "base_head" not in e


def test_goal_resume_with_base_head_rejected(tmp_path):
    # base_head 不可变（650 verdict=B）：RESUME 带 base_head=重锚企图，拒绝。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="base_head"):
        append_event("GOAL_RESUME", ev_path=str(ev_path), goal_id="g1",
                     note="续跑", base_head="newsha")


def test_goal_resume_missing_note_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="note"):
        append_event("GOAL_RESUME", ev_path=str(ev_path), goal_id="g1")


def test_check_pass_bare_rejected(tmp_path):
    # 裸写 CHECK_PASS（无 method）→ 拒绝（650：EVIDENCE=材料，CHECK_PASS=裁决，禁裸写）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="来源|method"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done")


def test_check_pass_auto_missing_command_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="command"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                     method="auto", verifier="ci")


def test_check_pass_manual_empty_evidence_ids_rejected(tmp_path):
    # method=manual 须非空 evidence_ids（机器溯源，禁裸裁决）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="evidence_ids"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                     method="manual", judge="o", rationale="r", evidence_ids=[])


def test_evidence_with_id_appends(tmp_path):
    # EVIDENCE 可带 evidence_id（稳定身份，供 CHECK_PASS.evidence_ids 溯源）。
    ev_path = tmp_path / "events.jsonl"
    append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1",
                 artifact="commit abc", evidence_id="ev-1")
    e = _read_lines(ev_path)[0]
    assert e["evidence_id"] == "ev-1"


def test_goal_set_acceptance_item_id_appends(tmp_path):
    # GOAL_SET acceptance 稳定身份在 acceptance[].id（嵌套，非顶层；codex MAJOR-1）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-1", "check": "c", "falsifiable": True}],
                 base_head="h")
    e = _read_lines(ev_path)[0]
    assert e["acceptance"][0]["id"] == "acc-1"


def test_goal_set_top_level_acceptance_id_rejected(tmp_path):
    # 顶层 acceptance_id 是 reducer 读不到的废字段（codex MAJOR-1）→ 多余字段拒绝。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="acceptance_id|多余|unexpected"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"check": "c", "falsifiable": True}],
                     base_head="h", acceptance_id="废字段")


def test_check_pass_manual_dangling_evidence_rejected(tmp_path):
    # evidence_ids 指向不存在的 EVIDENCE → 拒绝（codex CRITICAL：机器溯源禁指向空气）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="不存在|evidence_id"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                     method="manual", judge="o", rationale="r", evidence_ids=["ev-ghost"])


def test_evidence_id_uniqueness_enforced(tmp_path):
    # 重复 evidence_id → 拒绝（codex CRITICAL：溯源歧义防护）。
    ev_path = tmp_path / "events.jsonl"
    append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.1",
                 artifact="a", evidence_id="ev-1")
    with pytest.raises(ValueError, match="已存在|唯一"):
        append_event("EVIDENCE", ev_path=str(ev_path), sub_goal_id="g1.2",
                     artifact="b", evidence_id="ev-1")


def test_check_pass_auto_with_manual_field_rejected(tmp_path):
    # method=auto 夹带 manual 专属字段 → 拒绝（codex MINOR-2：字段互斥）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="manual 专属"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="done",
                     method="auto", command="cargo test", verifier="ci", judge="混入")


def test_goal_set_duplicate_acceptance_id_rejected(tmp_path):
    # 同一 GOAL_SET 内 acceptance[].id 重复 → 拒绝（codex v2 MAJOR：重复 id 让一个
    # (gid,acceptance_id) 同时闭合多个 acceptance）。
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="重复|唯一"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"id": "dup", "check": "c1", "falsifiable": True},
                                 {"id": "dup", "check": "c2", "falsifiable": True}],
                     base_head="h")


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


# ── GOAL_AMEND（codex 严格解法）：寻址层补 acceptance_id ──────────────────────

def _seed_goal(ev_path, acceptance):
    """写一个 GOAL_SET 作为 amend 目标。返回其 acceptance 数组。"""
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=acceptance, base_head="fab1f08a")
    return acceptance


def test_goal_amend_binds_acceptance_id(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True},
                               {"check": "c1", "falsifiable": True}])
    e = append_event(
        "GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
        amendment_kind="ACCEPTANCE_ID_BINDING", reason="bind ids",
        acceptance_vector_hash=acceptance_vector_hash(acc),
        bindings=[{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[0]), "acceptance_id": "acc-1"},
                  {"ordinal": 1, "acceptance_hash": acceptance_hash(acc[1]), "acceptance_id": "acc-2"}],
    )
    assert e["event"] == "GOAL_AMEND"
    assert e["bindings"][0]["acceptance_id"] == "acc-1"


def test_goal_amend_wrong_vector_hash_rejected(tmp_path):
    # acceptance_vector_hash 与目标 GOAL_SET 不符 → 拒绝（防对错误版本补 id）。
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True}])
    with pytest.raises(ValueError, match="acceptance_vector_hash"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                     acceptance_vector_hash="sha256:篡改",
                     bindings=[{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[0]),
                                "acceptance_id": "acc-1"}])


def test_goal_amend_ordinal_hash_mismatch_rejected(tmp_path):
    # ordinal 指向的 slot 的 hash 不符（ordinal/hash 错位）→ 拒绝。
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True},
                               {"check": "c1", "falsifiable": True}])
    with pytest.raises(ValueError, match="acceptance_hash 不匹配|错位"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                     acceptance_vector_hash=acceptance_vector_hash(acc),
                     bindings=[{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[1]),  # 错位
                                "acceptance_id": "acc-1"}])


def test_goal_amend_duplicate_id_rejected(tmp_path):
    # acceptance_id 在 goal 内重复 → 拒绝。
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True},
                               {"check": "c1", "falsifiable": True}])
    with pytest.raises(ValueError, match="唯一|重复"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                     acceptance_vector_hash=acceptance_vector_hash(acc),
                     bindings=[{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[0]), "acceptance_id": "dup"},
                               {"ordinal": 1, "acceptance_hash": acceptance_hash(acc[1]), "acceptance_id": "dup"}])


def test_goal_amend_idempotent_rebind_noop(tmp_path):
    # 重复 amend 绑同名 id → no-op 通过（幂等，重放安全）。
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True}])
    b = [{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[0]), "acceptance_id": "acc-1"}]
    append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                 amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                 acceptance_vector_hash=acceptance_vector_hash(acc), bindings=b)
    # 第二次同绑定——目标 slot 仍无 id（amend 不改 GOAL_SET 事件本身），所以再次合法
    # （reducer 应用层幂等；writer 校验只看 GOAL_SET 的 slot 是否已被自带 id 占用）。
    append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                 amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                 acceptance_vector_hash=acceptance_vector_hash(acc), bindings=b)


def test_goal_amend_unknown_kind_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    acc = _seed_goal(ev_path, [{"check": "c0", "falsifiable": True}])
    with pytest.raises(ValueError, match="amendment_kind"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="GOAL_RENAME", reason="x",
                     acceptance_vector_hash=acceptance_vector_hash(acc),
                     bindings=[{"ordinal": 0, "acceptance_hash": acceptance_hash(acc[0]),
                                "acceptance_id": "acc-1"}])


def test_goal_amend_no_goal_set_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="无对应 GOAL_SET"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="ghost",
                     amendment_kind="ACCEPTANCE_ID_BINDING", reason="x",
                     acceptance_vector_hash="sha256:x",
                     bindings=[{"ordinal": 0, "acceptance_hash": "sha256:y", "acceptance_id": "acc-1"}])


# ── #6 MAJOR: CHECK_PASS/CHECK_FAIL.acceptance_id 归属校验（写入前精确解析当前 goal）──

def test_check_pass_acceptance_id_belongs_to_goal_appends(tmp_path):
    # CHECK_PASS 针对 goal-level acceptance（sub_goal_id==goal_id），带的 acceptance_id 恰在
    # 该 goal 的 acceptance 集里 → 合法写入。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-1", "check": "c", "falsifiable": True}], base_head="h")
    append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1", check="c",
                 acceptance_id="acc-1", method="auto", command="x", verifier="ci")
    e = _read_lines(ev_path)[1]
    assert e["acceptance_id"] == "acc-1"


def test_check_pass_acceptance_id_not_in_goal_rejected(tmp_path):
    # #6：CHECK_PASS 针对 goal（sub_goal_id==goal_id）带的 acceptance_id 不属于该 goal 的
    # acceptance 集 → 解析 0 项 → 拒绝（机器溯源：禁指向不存在的 acceptance）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-1", "check": "c", "falsifiable": True}], base_head="h")
    with pytest.raises(ValueError, match="acceptance_id|不属于|不存在"):
        append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1", check="c",
                     acceptance_id="acc-GHOST", method="auto", command="x", verifier="ci")


def test_check_fail_acceptance_id_not_in_goal_rejected(tmp_path):
    # #6：CHECK_FAIL 同样校验 acceptance_id 归属（对齐 CHECK_PASS）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-1", "check": "c", "falsifiable": True}], base_head="h")
    with pytest.raises(ValueError, match="acceptance_id|不属于|不存在"):
        append_event("CHECK_FAIL", ev_path=str(ev_path), sub_goal_id="g1", check="c",
                     acceptance_id="acc-GHOST", reason="争议")


# ── #2 GOAL_DRAFT 两阶段：skeleton acceptance 落 DRAFT（非 active），补齐后转正 GOAL_SET ──

def test_goal_draft_valid_appends(tmp_path):
    # GOAL_DRAFT 与 GOAL_SET 同结构（goal_id+description+acceptance+base_head），但语义=非 active
    # 占位（skeleton 落这里，reducer 不计入 active 集）。falsifiable 仍强制（占位也须可证伪结构）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g-draft", description="草稿目标",
                 acceptance=[{"id": "a0-skeleton", "check": "待 Lead 补全", "falsifiable": True}],
                 base_head="abc123")
    e = _read_lines(ev_path)[0]
    assert e["event"] == "GOAL_DRAFT"
    assert e["goal_id"] == "g-draft"
    assert e["acceptance"][0]["id"] == "a0-skeleton"
    assert e["base_head"] == "abc123"


def test_goal_draft_non_falsifiable_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="falsifiable"):
        append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"check": "差不多", "falsifiable": False}], base_head="h")


def test_goal_draft_missing_base_head_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="base_head"):
        append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g1", description="x",
                     acceptance=[{"check": "c", "falsifiable": True}])


def test_cli_goal_set_writes_draft_not_active(tmp_path, monkeypatch):
    # #2：裸 /goal（_cli_goal_set）默认写 GOAL_DRAFT（skeleton 不进 active），不写 GOAL_SET。
    import goal_events
    ev_path = tmp_path / "events.jsonl"
    monkeypatch.setattr(goal_events.subprocess, "run",
                        lambda *a, **k: type("R", (), {"stdout": "deadbeef\n"})())
    event = goal_events._cli_goal_set("我的目标", ev_path=str(ev_path))
    assert event["event"] == "GOAL_DRAFT"  # 不是 GOAL_SET
    assert event["base_head"] == "deadbeef"
    assert event["acceptance"][0]["falsifiable"] is True


# ── #8 幂等（655号）：重复 goal_id / idempotency_key 防护 ─────────────────────

def test_duplicate_goal_id_goal_set_raises(tmp_path):
    # 重复 goal_id 的 GOAL_SET → raise（防同 goal_id 多次 SET 制造歧义/重复 active）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"check": "c", "falsifiable": True}], base_head="h")
    with pytest.raises(ValueError, match="goal_id|已存在|重复"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="y",
                     acceptance=[{"check": "d", "falsifiable": True}], base_head="h")


def test_duplicate_goal_id_draft_raises(tmp_path):
    # 重复 goal_id 的 GOAL_DRAFT 也 raise（同身份不可重复声明）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"check": "c", "falsifiable": True}], base_head="h")
    with pytest.raises(ValueError, match="goal_id|已存在|重复"):
        append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g1", description="y",
                     acceptance=[{"check": "d", "falsifiable": True}], base_head="h")


def test_draft_then_goalset_same_goal_id_allowed(tmp_path):
    # DRAFT → GOAL_SET 转正（同 goal_id）：合法。goal_id 唯一性是「同事件类型内」唯一，
    # DRAFT 转 GOAL_SET 是跨类型的合法状态推进（DRAFT 占位 → GOAL_SET 正式）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_DRAFT", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "a0-skeleton", "check": "待补全", "falsifiable": True}],
                 base_head="h")
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-real", "check": "真验收", "falsifiable": True}],
                 base_head="h")
    assert len(_read_lines(ev_path)) == 2


def test_idempotency_key_same_payload_noop(tmp_path):
    # 同 idempotency_key + 同 payload → 返回既有事件，不重复 append（重试安全）。
    ev_path = tmp_path / "events.jsonl"
    e1 = append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                      acceptance=[{"check": "c", "falsifiable": True}], base_head="h",
                      idempotency_key="k1")
    e2 = append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                      acceptance=[{"check": "c", "falsifiable": True}], base_head="h",
                      idempotency_key="k1")
    assert len(_read_lines(ev_path)) == 1  # 第二次 noop，不 append
    assert e1["goal_id"] == e2["goal_id"]


def test_idempotency_key_diff_payload_raises(tmp_path):
    # 同 idempotency_key + 不同 payload → raise（key 复用但内容变了=调用方逻辑错误）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"check": "c", "falsifiable": True}], base_head="h",
                 idempotency_key="k1")
    with pytest.raises(ValueError, match="idempotency_key|payload"):
        append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g2", description="不同",
                     acceptance=[{"check": "d", "falsifiable": True}], base_head="h",
                     idempotency_key="k1")


# ── #3 GOAL_AMEND ACCEPTANCE_REPLACE：替换整个 acceptance vector（DRAFT 转正/验收改写）──

def test_amend_replace_rebuilds_acceptance_vector(tmp_path):
    # ACCEPTANCE_REPLACE：old_acceptance_vector_hash 匹配目标 GOAL_SET → 替换为新 vector。
    ev_path = tmp_path / "events.jsonl"
    old_acc = [{"id": "a0-skeleton", "check": "待 Lead 补全", "falsifiable": True}]
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=old_acc, base_head="h")
    e = append_event(
        "GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
        amendment_kind="ACCEPTANCE_REPLACE", reason="Lead 补全真实可证伪验收",
        old_acceptance_vector_hash=acceptance_vector_hash(old_acc),
        acceptance=[{"id": "acc-1", "check": "cargo test 全绿", "falsifiable": True},
                    {"id": "acc-2", "check": "L3 Sharpe>1", "falsifiable": True}],
    )
    assert e["amendment_kind"] == "ACCEPTANCE_REPLACE"
    assert len(e["acceptance"]) == 2


def test_amend_replace_wrong_old_hash_rejected(tmp_path):
    # old_acceptance_vector_hash 不匹配目标 GOAL_SET → 拒绝（防对错误版本/并发改写 acceptance）。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"check": "c", "falsifiable": True}], base_head="h")
    with pytest.raises(ValueError, match="acceptance_vector_hash|不符"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="ACCEPTANCE_REPLACE", reason="x",
                     old_acceptance_vector_hash="sha256:篡改",
                     acceptance=[{"check": "新", "falsifiable": True}])


def test_amend_replace_non_falsifiable_rejected(tmp_path):
    # 新 acceptance 须仍 falsifiable（REPLACE 不能引入不可证伪验收）。
    ev_path = tmp_path / "events.jsonl"
    old_acc = [{"check": "c", "falsifiable": True}]
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=old_acc, base_head="h")
    with pytest.raises(ValueError, match="falsifiable"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="g1",
                     amendment_kind="ACCEPTANCE_REPLACE", reason="x",
                     old_acceptance_vector_hash=acceptance_vector_hash(old_acc),
                     acceptance=[{"check": "差不多", "falsifiable": False}])


def test_amend_replace_no_goal_set_rejected(tmp_path):
    ev_path = tmp_path / "events.jsonl"
    with pytest.raises(ValueError, match="无对应 GOAL_SET"):
        append_event("GOAL_AMEND", ev_path=str(ev_path), goal_id="ghost",
                     amendment_kind="ACCEPTANCE_REPLACE", reason="x",
                     old_acceptance_vector_hash="sha256:x",
                     acceptance=[{"check": "c", "falsifiable": True}])


def test_check_pass_sub_goal_acceptance_id_not_goal_scoped(tmp_path):
    # 边界：CHECK_PASS 针对 sub_goal（sub_goal_id != goal_id）的 acceptance_id 不受 goal
    # acceptance 集约束——sub_goal 有自己的验收标识，不在 goal acceptance 集里是合法的。
    # 只有针对 goal-level（sub_goal_id==当前 goal_id）的事件才校验归属。
    ev_path = tmp_path / "events.jsonl"
    append_event("GOAL_SET", ev_path=str(ev_path), goal_id="g1", description="x",
                 acceptance=[{"id": "acc-1", "check": "c", "falsifiable": True}], base_head="h")
    # sub_goal_id="g1.1" != goal_id="g1" → 不校验 goal acceptance 归属
    append_event("CHECK_PASS", ev_path=str(ev_path), sub_goal_id="g1.1", check="子目标完成",
                 acceptance_id="sg-local-id", method="auto", command="x", verifier="ci")
    assert len(_read_lines(ev_path)) == 2
