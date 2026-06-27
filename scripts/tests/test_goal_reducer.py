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
