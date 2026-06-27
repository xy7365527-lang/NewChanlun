"""D′ goal reducer：纯函数 (events + facts) → ready workstations projection。
确定性：同输入同输出（无 IO、无时间、无随机）。这是 RTAS 蜂群递归展开的确定性规则。"""

def _validate_acceptance(acceptance: list[dict]) -> None:
    """校验每个 acceptance 项必须 falsifiable（lesson 0011 锋利问题②：验收不可证伪则 goal 永动空转）。"""
    for acc in acceptance:
        if not acc.get("falsifiable", False):
            raise ValueError(f"acceptance 项必须 falsifiable: {acc.get('check')}")

def reduce_goal(events: list[dict], facts: dict) -> dict:
    """events: list[dict]（events.jsonl 解析）；
    facts: {git_head(消费：base_head 对账), genealogy_pending/genealogy_settled/roadmap(§4 数据流输入契约
            预留字段，当前 T4 传空、readiness 规则未依赖)}。
    返回 {current_goal: dict|None, ready_workstations: list[str], blocked: list[dict], terminated: bool}。
    current_goal 携 base_head_stale：facts.git_head≠goal.base_head 时为 True（spec §9 快照降级信号）。"""
    # 1. 重建 current goal：最后一个未被 SUPERSEDE/CLOSED 的 GOAL_SET。
    # schema 演进容错（event-sourcing reader 向后兼容，630 开口①「写路径未实装」止血）：
    # 早期事件用规范 schema(goal_id/old_goal_id/description/acceptance/base_head)，actor
    # 手搓后期事件退化为 sub_goal_id+artifact。reader 必须读两版。退化事件无结构化
    # acceptance → [] 诚实暴露：terminated 保守 False（不静默当 0 验收已通过=非补丁掩盖），
    # base_head 缺失 → None（None≠git_head 自然标记 base_head_stale=True，降级提示正确）。
    # ★ 有效域诚实标注：止血让 reducer 算出 current goal id 不崩；但退化事件无结构化
    #   acceptance，reducer 不能机器判定验收闭合——完整修复（结构化 acceptance + 写路径
    #   writer）是 630 开口①残留，待编排者 /ritual。
    def _gid(e):  # goal id：规范 goal_id 优先，退化事件 fallback sub_goal_id
        return e.get("goal_id") or e.get("sub_goal_id")
    superseded = {e.get("old_goal_id") or e.get("sub_goal_id")
                  for e in events if e["event"] == "SUPERSEDE"}
    closed = {_gid(e) for e in events if e["event"] == "CLOSED"}
    goal = None
    for e in events:
        if e["event"] == "GOAL_SET" and _gid(e) not in superseded:
            acceptance = e.get("acceptance", [])
            _validate_acceptance(acceptance)
            goal = {"goal_id": _gid(e),
                    "description": e.get("description") or e.get("artifact", ""),
                    "acceptance": [dict(a, passed=False) for a in acceptance],
                    "base_head": e.get("base_head"), "status": "active",
                    "base_head_stale": e.get("base_head") != facts.get("git_head")}
    if goal is None:
        return {"current_goal": None, "ready_workstations": [], "blocked": [], "terminated": False}

    gid = goal["goal_id"]
    # 2. 重建 sub-goal 树（DECOMPOSE）。schema 容错（630 开口①）：只处理含规范
    # goal_id + sub_goals[] 的 DECOMPOSE；退化事件（{event,sub_goal_id,artifact}，无
    # goal_id/sub_goals）忠实跳过——数据里确实无结构化分解，reducer 不伪造 sub-goal 树
    # （ready=[] 是当前退化数据的诚实投影，非补丁掩盖；完整修复=写路径 writer，待 /ritual）。
    subs = {}
    for e in events:
        if e["event"] == "DECOMPOSE" and e.get("goal_id") == gid:
            for sg in e.get("sub_goals", []):
                subs[sg["id"]] = {"id": sg["id"], "desc": sg["desc"],
                                  "blocked_by": list(sg.get("blocked_by", [])), "passed": False, "blocker": None}
    # 3. 应用 CHECK_PASS / BLOCKED（sub_goal_id 缺失的退化事件安全跳过）
    passed_checks = set()
    for e in events:
        if e["event"] == "CHECK_PASS":
            sid = e.get("sub_goal_id")
            passed_checks.add((sid, e.get("check")))
            if sid in subs:
                subs[sid]["passed"] = True
        elif e["event"] == "BLOCKED" and e.get("sub_goal_id") in subs:
            subs[e["sub_goal_id"]]["blocker"] = e.get("blocker")
    # 4. ready = 未 passed + 未 blocker + 所有 blocked_by 已 passed
    # blocked_by 指向不存在的 sub_goal 时 subs.get(b,{}) 返回空 dict → passed=False → 永久 blocked（by-design：
    # 依赖未声明的前置即视为未满足，宁可保守不 ready，不静默放行）
    ready = [s["id"] for s in subs.values()
             if not s["passed"] and s["blocker"] is None
             and all(subs.get(b, {}).get("passed", False) for b in s["blocked_by"])]
    blocked = [{"id": s["id"], "blocker": s["blocker"]} for s in subs.values() if s["blocker"]]
    # 5. goal 验收：所有 acceptance CHECK_PASS → closed
    for acc in goal["acceptance"]:
        if (gid, acc["check"]) in passed_checks:
            acc["passed"] = True
    terminated = bool(goal["acceptance"]) and all(a["passed"] for a in goal["acceptance"])
    if terminated or gid in closed:
        goal["status"] = "closed"
    elif blocked and not ready:
        goal["status"] = "blocked"
    return {"current_goal": goal, "ready_workstations": sorted(ready), "blocked": blocked, "terminated": terminated}
