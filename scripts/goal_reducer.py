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
    # 1. 重建 current goal：最后一个未被 SUPERSEDE/CLOSED 的 GOAL_SET
    superseded = {e["old_goal_id"] for e in events if e["event"] == "SUPERSEDE"}
    closed = {e["goal_id"] for e in events if e["event"] == "CLOSED"}
    goal = None
    for e in events:
        if e["event"] == "GOAL_SET" and e["goal_id"] not in superseded:
            _validate_acceptance(e.get("acceptance", []))
            goal = {"goal_id": e["goal_id"], "description": e["description"],
                    "acceptance": [dict(a, passed=False) for a in e["acceptance"]],
                    "base_head": e["base_head"], "status": "active",
                    "base_head_stale": e["base_head"] != facts.get("git_head")}
    if goal is None:
        return {"current_goal": None, "ready_workstations": [], "blocked": [], "terminated": False}

    gid = goal["goal_id"]
    # 2. 重建 sub-goal 树（DECOMPOSE）
    subs = {}
    for e in events:
        if e["event"] == "DECOMPOSE" and e["goal_id"] == gid:
            for sg in e["sub_goals"]:
                subs[sg["id"]] = {"id": sg["id"], "desc": sg["desc"],
                                  "blocked_by": list(sg.get("blocked_by", [])), "passed": False, "blocker": None}
    # 3. 应用 CHECK_PASS / BLOCKED
    passed_checks = set()
    for e in events:
        if e["event"] == "CHECK_PASS":
            sid = e["sub_goal_id"]
            passed_checks.add((sid, e.get("check")))
            if sid in subs:
                subs[sid]["passed"] = True
        elif e["event"] == "BLOCKED" and e["sub_goal_id"] in subs:
            subs[e["sub_goal_id"]]["blocker"] = e["blocker"]
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
