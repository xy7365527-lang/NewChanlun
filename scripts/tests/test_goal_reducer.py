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


def test_ready_from_open_acceptance_when_no_decompose():
    # 630 开口②：无 DECOMPOSE 但有 open acceptance → 每项 open acceptance 派生一个 ready 工位。
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"}]
    out = reduce_goal(events, _facts())
    assert out["ready_details"] == [{"id": "c", "desc": "c"}]
    assert out["ready_workstations"] == ["c"]


def test_ready_details_empty_when_no_decompose_and_no_open_acceptance():
    # 无 DECOMPOSE 且无 acceptance → ready 为空（真退化数据）。
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "x",
               "acceptance": [], "base_head": "abc123", "ts": "t0"}]
    out = reduce_goal(events, _facts())
    assert out["ready_details"] == []
    assert out["ready_workstations"] == []


def test_check_fail_reverts_passed_and_reopens_goal():
    # 658 修复：CHECK_PASS 后 CHECK_FAIL（补偿事件）→ acceptance.passed 翻 False →
    # goal 不再 closed。模拟 acc-delta-r-alpha 被 codex 异质审判 underpowered 后争议撤销。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "acc-1", "check": "L3 实证", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "L3 实证",
         "acceptance_id": "acc-1", "method": "auto", "command": "x", "verifier": "ci", "ts": "t1"},
        {"event": "CHECK_FAIL", "sub_goal_id": "g1", "check": "L3 实证",
         "acceptance_id": "acc-1", "reason": "codex 异质审 underpowered，n=5 功效不足", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["acceptance"][0]["passed"] is False  # 撤销
    assert out["current_goal"]["acceptance"][0]["contested"] is True  # 标记争议
    assert out["current_goal"]["status"] != "closed"  # goal 重开
    assert out["terminated"] is False


# 注：裁定前的 test_check_fail_then_pass_recloses_goal（编码被否决的「FAIL 后 PASS 即重闭合」
# 复判语义）已删除——#7 严格闭合裁定取代之，由 test_contested_prevents_close 覆盖
# PASS→FAIL→PASS 后 contested 永久、goal 不自动重闭合。


def test_check_fail_via_check_fallback_no_acceptance_id():
    # 历史有效域：无 acceptance_id 的 CHECK_FAIL 走 check 文本键撤销（与 CHECK_PASS fallback 对称）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
        {"event": "CHECK_FAIL", "sub_goal_id": "g1", "check": "c1", "reason": "撤销", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["acceptance"][0]["passed"] is False
    assert out["current_goal"]["status"] != "closed"


def test_multiple_active_goals_blocks():
    # codex #1 CRITICAL silent goal-loss：两个 GOAL_SET 都未 SUPERSEDE/CLOSED → 都 active。
    # 旧 last-writer-wins 静默选物理最后一个，吞掉前一个 active goal（实证 bug：
    # mutexlevel 被 overfitframework 静默盖掉）。修复：active>1 → current_goal=None +
    # AMBIGUOUS_ACTIVE_GOALS，不产 ready，不 fallback。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "目标一",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "目标二",
         "acceptance": [{"check": "c2", "falsifiable": True}], "base_head": "abc123", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"] is None  # 不静默选 g2
    assert out["ready_workstations"] == []
    amb = [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]
    assert len(amb) == 1
    assert amb[0]["ids"] == ["g1", "g2"]  # sorted


def test_single_active_normal_after_supersede():
    # 多 GOAL_SET 但只剩一个未 SUPERSEDE → 正常选中（active-set 语义不破坏单 active 场景）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t1"},
        {"event": "SUPERSEDE", "old_goal_id": "g1", "new_goal_id": "g2", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g2"
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]


def test_closed_excluded_from_active_set():
    # CLOSED 的 GOAL_SET 从 active 集剔除：g1 closed + g2 active → 仅 g2 active，不歧义。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c", "falsifiable": True}], "base_head": "abc123", "ts": "t1"},
        {"event": "CLOSED", "goal_id": "g1", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g2"
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]


def test_contested_prevents_close():
    # codex #7 MAJOR：PASS→FAIL→PASS 后 passed=True 但 contested=True。严格闭合（编排者裁定
    # 保守安全）：曾被 CHECK_FAIL 争议的 acceptance 即使后续 CHECK_PASS，仍需显式 CHECK_RESOLVE
    # 才闭合 → terminated=False。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "acc-1", "check": "c", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c",
         "acceptance_id": "acc-1", "method": "auto", "command": "x", "verifier": "ci", "ts": "t1"},
        {"event": "CHECK_FAIL", "sub_goal_id": "g1", "check": "c",
         "acceptance_id": "acc-1", "reason": "争议", "ts": "t2"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c",
         "acceptance_id": "acc-1", "method": "auto", "command": "x", "verifier": "ci", "ts": "t3"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["acceptance"][0]["passed"] is True  # 最后写者=PASS
    assert out["current_goal"]["acceptance"][0]["contested"] is True  # 曾被争议
    assert out["terminated"] is False  # 严格闭合：contested 阻止闭合
    assert out["current_goal"]["status"] != "closed"


def test_goal_draft_not_active(self_ignored=None):
    # #2 GOAL_DRAFT 两阶段：DRAFT 是非 active 占位（skeleton acceptance 落这里），reducer
    # 不收集 DRAFT 进 active 集 → 不进 current_goal、不产 ready、不触发 AMBIGUOUS。
    events = [
        {"event": "GOAL_DRAFT", "goal_id": "g-draft", "description": "草稿目标",
         "acceptance": [{"id": "a0-skeleton", "check": "待 Lead 补全", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"] is None  # DRAFT 不进 current_goal
    assert out["ready_workstations"] == []
    # 不触发 AMBIGUOUS（DRAFT 根本不在 active 集）
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]


def test_goal_draft_does_not_count_toward_ambiguity():
    # DRAFT + 一个真 GOAL_SET 共存：只有 GOAL_SET 是 active，DRAFT 被忽略 → 不歧义、正常选中。
    events = [
        {"event": "GOAL_DRAFT", "goal_id": "g-draft", "description": "草稿",
         "acceptance": [{"id": "a0-skeleton", "check": "待补全", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_SET", "goal_id": "g-real", "description": "真目标",
         "acceptance": [{"check": "真验收", "falsifiable": True}], "base_head": "abc123", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g-real"
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]


def test_draft_to_goalset_promotion():
    # 转正路径（最简自洽，ponytail）：DRAFT 补全后写真 GOAL_SET（同或异 goal_id）。
    # GOAL_SET 进 active，DRAFT 留历史作审计。转正后 current_goal=真 GOAL_SET。
    events = [
        {"event": "GOAL_DRAFT", "goal_id": "g1", "description": "目标",
         "acceptance": [{"id": "a0-skeleton", "check": "待补全", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_SET", "goal_id": "g1", "description": "目标",
         "acceptance": [{"id": "acc-real", "check": "lake build 绿", "falsifiable": True}],
         "base_head": "abc123", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g1"
    assert out["current_goal"]["status"] == "active"
    assert out["current_goal"]["acceptance"][0]["check"] == "lake build 绿"


def test_acceptance_replace_rebuilds_acceptance_vector():
    # #3 ACCEPTANCE_REPLACE：GOAL_AMEND 替换整个 acceptance vector（DRAFT 转正/验收改写）。
    # reducer 应用 REPLACE → current_goal.acceptance 变为新 vector（旧 skeleton 被替换）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "a0-skeleton", "check": "待 Lead 补全", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "GOAL_AMEND", "goal_id": "g1", "amendment_kind": "ACCEPTANCE_REPLACE",
         "reason": "Lead 补全真实可证伪验收",
         "old_acceptance_vector_hash": None,  # 占位，测试在 GREEN 后用真 hash 替换
         "acceptance": [
             {"id": "acc-1", "check": "cargo test --lib 全绿", "falsifiable": True},
             {"id": "acc-2", "check": "L3 多标的实证 Sharpe>1", "falsifiable": True},
         ], "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    accs = out["current_goal"]["acceptance"]
    assert len(accs) == 2
    assert accs[0]["check"] == "cargo test --lib 全绿"
    assert accs[1]["id"] == "acc-2"
    # 旧 skeleton 不再出现
    assert all(a["check"] != "待 Lead 补全" for a in accs)


def test_acceptance_replace_resets_stale_check_pass():
    # REPLACE 重建 vector → 旧 acceptance 的 CHECK_PASS 失配（验收标准变了，旧通过作废）。
    # 这是正确语义（codex「重建 acceptance vector」）：改验收 = 重置闭合状态。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"id": "old-acc", "check": "旧验收", "falsifiable": True}],
         "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "旧验收",
         "acceptance_id": "old-acc", "method": "auto", "command": "x", "verifier": "ci", "ts": "t1"},
        {"event": "GOAL_AMEND", "goal_id": "g1", "amendment_kind": "ACCEPTANCE_REPLACE",
         "reason": "改验收", "old_acceptance_vector_hash": None,
         "acceptance": [{"id": "new-acc", "check": "新验收", "falsifiable": True}], "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    # 新 acceptance 未被旧 CHECK_PASS 闭合
    assert out["current_goal"]["acceptance"][0]["passed"] is False
    assert out["current_goal"]["status"] != "closed"
    assert out["terminated"] is False


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


def test_open_acceptance_derives_ready_without_decompose():
    """630 开口②：active goal 有 open acceptance 但无 DECOMPOSE → ready 从 acceptance 派生。
    此前 ready 只从 sub_goals 派生，无分解 goal 恒空。"""
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "d",
               "base_head": "h", "acceptance": [
                   {"id": "a1", "check": "c1", "falsifiable": True},
                   {"id": "a2", "check": "c2", "falsifiable": True}]}]
    out = reduce_goal(events, {"git_head": "h"})
    assert out["ready_workstations"] == ["a1", "a2"]
    assert out["ready_details"] == [{"id": "a1", "desc": "c1"}, {"id": "a2", "desc": "c2"}]
    assert out["blocked"] == []
    assert out["terminated"] is False


def test_passed_acceptance_excluded_from_ready():
    """已 CHECK_PASS 的 acceptance 不再进 ready；全 pass → terminated。"""
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "d", "base_head": "h",
               "acceptance": [{"id": "a1", "check": "c1", "falsifiable": True},
                              {"id": "a2", "check": "c2", "falsifiable": True}]},
              {"event": "CHECK_PASS", "sub_goal_id": "g1", "acceptance_id": "a1", "check": "c1"}]
    out = reduce_goal(events, {"git_head": "h"})
    assert out["ready_workstations"] == ["a2"]


def test_decompose_still_drives_ready_when_present():
    """回归：有 DECOMPOSE 时仍由 sub_goals 驱动 ready，不被 acceptance fallback 覆盖。"""
    events = [{"event": "GOAL_SET", "goal_id": "g1", "description": "d", "base_head": "h",
               "acceptance": [{"id": "a1", "check": "c1", "falsifiable": True}]},
              {"event": "DECOMPOSE", "goal_id": "g1",
               "sub_goals": [{"id": "s1", "desc": "sub1", "blocked_by": []}]}]
    out = reduce_goal(events, {"git_head": "h"})
    assert out["ready_workstations"] == ["s1"]


def test_dangling_terminated_does_not_block_new_goal():
    # dangling-active 根治（核心回归）：g1 全 acceptance CHECK_PASS（terminated）但
    # loop 尚未写 CLOSED，此时新 GOAL_SET g2 抢先追加。旧实现把 terminated-未-CLOSED 的 g1 计入
    # active 歧义 → AMBIGUOUS_ACTIVE_GOALS → current_goal=None（本 session 实发，靠手工补录
    # CLOSED 解除）。修复：歧义只在 live goal（有未竟工作）间判定，terminated g1 不参与 → 命中 g2。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "旧目标",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "新目标",
         "acceptance": [{"check": "c2", "falsifiable": True}], "base_head": "abc123", "ts": "t2"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g2"  # 命中新 goal，非 None，非 g1
    assert out["current_goal"]["status"] == "active"
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]


def test_terminated_goal_surfaces_when_no_live_goal():
    # 无 live goal 时 terminated goal 仍作 current_goal 上报（status=closed/terminated=True），
    # 供 ceremony_scan 停 spawn（codex#5）+ loop 写 CLOSED 审计——CLOSED 的显式审计价值不因根治丢失。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"]["goal_id"] == "g1"
    assert out["current_goal"]["status"] == "closed"
    assert out["terminated"] is True


def test_ambiguity_only_among_live_goals():
    # 根治不削弱 codex#1：两个 live goal 仍 AMBIGUOUS。terminated g0 已完成、非竞争方向，不进
    # 歧义 ids——歧义只含真正竞争的 g1/g2。
    events = [
        {"event": "GOAL_SET", "goal_id": "g0", "description": "done",
         "acceptance": [{"check": "c0", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g0", "check": "c0", "ts": "t1"},
        {"event": "GOAL_SET", "goal_id": "g1", "description": "live一",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t2"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "live二",
         "acceptance": [{"check": "c2", "falsifiable": True}], "base_head": "abc123", "ts": "t3"},
    ]
    out = reduce_goal(events, _facts())
    assert out["current_goal"] is None
    amb = [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]
    assert len(amb) == 1
    assert amb[0]["ids"] == ["g1", "g2"]  # g0(terminated) 不在内


def test_multiple_terminated_no_ambiguity_drains_one():
    # 两个 terminated goal、零 live：不歧义（done goal 不竞争）。current 取字典序末位待闭合，
    # loop 逐个写 CLOSED 排空——done goal 无 goal-loss 风险，与 codex#1 只防 live goal 静默丢失一致。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c2", "falsifiable": True}], "base_head": "abc123", "ts": "t2"},
        {"event": "CHECK_PASS", "sub_goal_id": "g2", "check": "c2", "ts": "t3"},
    ]
    out = reduce_goal(events, _facts())
    assert not [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]
    assert out["current_goal"]["goal_id"] == "g2"  # 字典序末位
    assert out["terminated"] is True


def test_contested_goal_stays_live_not_drained():
    # 658 交互：g1 曾 CHECK_PASS 后被 CHECK_FAIL 争议 → 未闭合（contested）→ 仍 live。它与新 g2
    # 都 live → AMBIGUOUS（争议撤销的 goal 不被误当 terminated 排空，严格闭合优先安全）。
    events = [
        {"event": "GOAL_SET", "goal_id": "g1", "description": "x",
         "acceptance": [{"check": "c1", "falsifiable": True}], "base_head": "abc123", "ts": "t0"},
        {"event": "CHECK_PASS", "sub_goal_id": "g1", "check": "c1", "ts": "t1"},
        {"event": "CHECK_FAIL", "sub_goal_id": "g1", "check": "c1", "reason": "争议", "ts": "t2"},
        {"event": "GOAL_SET", "goal_id": "g2", "description": "y",
         "acceptance": [{"check": "c2", "falsifiable": True}], "base_head": "abc123", "ts": "t3"},
    ]
    out = reduce_goal(events, _facts())
    amb = [b for b in out["blocked"] if b.get("type") == "AMBIGUOUS_ACTIVE_GOALS"]
    assert len(amb) == 1
    assert amb[0]["ids"] == ["g1", "g2"]  # g1 仍 live（contested 未闭合）
