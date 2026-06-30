from goal_reducer import reduce_goal

def _facts(head="abc123"):
    return {"git_head": head, "genealogy_pending": [], "genealogy_settled": [], "roadmap": []}

def test_empty_events_no_goal():
    out = reduce_goal([], _facts())
    assert out["current_goal"] is None
    assert out["ready_workstations"] == []

def test_goal_set_rebuilds_current():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "形式化",
               "acceptance": [{"check": "lake build 绿", "falsifiable": True}],
               "base_head": "abc123", "ts": "2026-06-27T00:00:00Z"}]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g1"
    assert out["current_goal"]["status"] == "active"

def test_goal_resume_does_not_rewrite_base_head():
    # base_head 不可变（650 verdict=B）：GOAL_RESUME 是纯审计事件，不重锚 base_head。
    # 即使历史中存在带 base_head 的 RESUME（旧实现的遗留），reader 忠实忽略——base_head
    # 永久停在 GOAL_SET 锚，base_head≠git_head 是正确的漂移降级信号（不被 RESUME 抹掉）。
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "old", "ts": "t0"},
              {"event": "GOAL_RESUME", "goal_id": "g1", "base_head": "new", "ts": "t1"}]
    # git_head=new（=RESUME 企图重锚的值）但 base_head 仍=GOAL_SET 锚 old → stale 信号保留
    out = reduce_goal(events, _facts(head="new"))
    assert out["current_goal"]["base_head"] == "old"  # 不被 RESUME 改写
    assert out["current_goal"]["base_head_stale"] is True  # 漂移预警未被抹掉
    # git_head 回到 GOAL_SET 锚 → 不 stale
    assert reduce_goal(events, _facts(head="old"))["current_goal"]["base_head_stale"] is False


def test_check_pass_acceptance_id_closes_goal():
    # CHECK_PASS 带 acceptance_id → 优先稳定身份匹配（650）。check 文本不同也能闭合。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "acc-1", "check": "原始 check 文本", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "改写过的 check",
         "acceptance_id": "acc-1", "method": "auto", "command": "cargo test",
         "verifier": "ci", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["acceptance"][0]["passed"] is True
    assert out["current_goal"]["status"] == "closed"


def test_check_pass_wrong_acceptance_id_does_not_close_via_check_fallback():
    # 两键互斥（codex v2 MAJOR）：CHECK_PASS 带 acceptance_id 时只走稳定身份键，不再
    # fallback check 文本。acceptance_id 写错（不匹配 acc.id）即使 check 文本相同也不闭合。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "acc-1", "check": "同名 check", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "同名 check",
         "acceptance_id": "acc-WRONG", "method": "auto", "command": "x",
         "verifier": "ci", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["acceptance"][0]["passed"] is False
    assert out["current_goal"]["status"] != "closed"


def test_decompose_ready_excludes_blocked():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "a", "blocked_by": []},
                       {"id": "g1.2", "desc": "b", "blocked_by": ["g1.1"]}]},
    ]
    out = reduce_goal(events, _facts())
    assert "g1.1" in out["ready_workstations"]
    assert "g1.2" not in out["ready_workstations"]  # blocked by g1.1

def test_check_pass_unblocks_dependent():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "a", "blocked_by": []},
                       {"id": "g1.2", "desc": "b", "blocked_by": ["g1.1"]}]},
        {"event": "CHECK_PASS", "sub_goal_id": "g1.1", "check": "done", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert "g1.2" in out["ready_workstations"]  # g1.1 passed → g1.2 unblocked

def test_all_acceptance_pass_closes_goal():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["status"] == "closed"

def test_supersede_invalidates_old():
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "SUPERSEDE", "old_goal_id": "g1", "new_goal_id": "g2", "ts": "t1"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g2"

def test_determinism_same_input_same_output():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"}]
    assert reduce_goal(events, _facts()) == reduce_goal(events, _facts())

def test_goal_set_rejects_non_falsifiable_acceptance():
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "差不多", "falsifiable": False}], "base_head": "abc123", "ts": "t0"}]
    import pytest
    with pytest.raises(ValueError, match="falsifiable"):
        reduce_goal(events, _facts())

def test_base_head_mismatch_flags_stale():
    # facts.git_head 与 goal.base_head 不匹配 → current_goal 标记 base_head_stale=True（spec §9：快照降级信号）
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"}]
    out_match = reduce_goal(events, _facts(head="abc123"))
    out_stale = reduce_goal(events, _facts(head="deadbeef"))
    assert out_match["current_goal"]["base_head_stale"] is False
    assert out_stale["current_goal"]["base_head_stale"] is True

def test_reads_legacy_degraded_schema_without_crash():
    # 真实 events.jsonl 形态（630 开口①：写路径未实装→actor 手搓后期事件退化为
    # sub_goal_id+artifact，丢失 goal_id/description/acceptance/base_head）。reducer 作为
    # event-sourcing reader 必须向后兼容多 schema 版本：算出最新未被 supersede 的 goal id
    # 不崩；退化事件无结构化 acceptance → [] 诚实暴露（terminated 保守 False，不静默当
    # 0 验收已通过——非补丁掩盖）。这是 L2 真实数据（不是 L1 合成）才暴露的 reader 缺口。
    events = [
        {"event": "GOAL_SET", "goal_id": "g-old", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "h0", "ts": "t0"},
        {"event": "SUPERSEDE", "sub_goal_id": "g-old", "artifact": "升级替代", "ts": "t1"},
        {"event": "GOAL_SET", "sub_goal_id": "g-new", "artifact": "新 goal 叙事", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g-new"  # 最新未被 supersede 的 goal
    assert out["current_goal"]["status"] == "active"  # acceptance 缺失 → 保守 not closed
    assert out["current_goal"]["acceptance"] == []  # 退化事件无结构化验收（诚实暴露）
    assert out["current_goal"]["description"] == "新 goal 叙事"  # artifact fallback 作 description
    assert out["terminated"] is False

def test_ready_details_carry_desc_for_workstation_naming():
    # ready_details：ready sub_goal 的 {id, desc}，供 ceremony_scan 生成有意义的
    # workstation name/description（开口②驱动 spawn 的前置——光有 id 列表不够，
    # workstation 需要 desc 才能让工位知道做什么）。与 ready_workstations(id 列表)并存，
    # 不破坏向后兼容。顺序与 ready_workstations 一致（sorted by id）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "做 A", "blocked_by": []},
                       {"id": "g1.2", "desc": "做 B", "blocked_by": ["g1.1"]}]},
    ]
    out = reduce_goal(events, _facts())
    assert out["ready_workstations"] == ["g1.1"]
    assert out["ready_details"] == [{"id": "g1.1", "desc": "做 A"}]  # g1.2 blocked，不在 ready


def test_ready_details_empty_when_no_decompose():
    # 退化数据（无结构化 DECOMPOSE）→ ready_details 为空，与 ready_workstations 一致。
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"}]
    out = reduce_goal(events, _facts())
    assert out["ready_details"] == []


def test_sub_goal_check_name_collision_does_not_close_goal():
    # sub_goal 的 check 名与 goal acceptance check 名相同，但 CHECK_PASS 只 target sub_goal，
    # 不应误判 goal acceptance 通过（gid-only 匹配，SCHEMA 未定义跨节点 rollup）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "DECOMPOSE", "goal_id": "g1", "ts": "t1",
         "sub_goals": [{"id": "g1.1", "desc": "a", "blocked_by": []}]},
        {"event": "CHECK_PASS", "sub_goal_id": "g1.1", "check": "c1", "ts": "t2"},  # sub_goal，非 goal
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["status"] != "closed"  # 不应因 sub_goal 同名 check 而闭合
    assert out["current_goal"]["acceptance"][0]["passed"] is False
