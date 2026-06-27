"""D′ goal reducer：纯函数 (events + facts) → ready workstations projection。
确定性：同输入同输出（无 IO、无时间、无随机）。这是 RTAS 蜂群递归展开的确定性规则。"""

def reduce_goal(events, facts):
    """events: list[dict]（events.jsonl 解析）；facts: {git_head, genealogy_pending, genealogy_settled, roadmap}。
    返回 {current_goal: dict|None, ready_workstations: list[str], blocked: list[dict], terminated: bool}。"""
    # 1. 重建 current goal：最后一个未被 SUPERSEDE/CLOSED 的 GOAL_SET
    superseded = {e["old_goal_id"] for e in events if e["event"] == "SUPERSEDE"}
    closed = {e["goal_id"] for e in events if e["event"] == "CLOSED"}
    goal = None
    for e in events:
        if e["event"] == "GOAL_SET" and e["goal_id"] not in superseded:
            for acc in e.get("acceptance", []):
                if not acc.get("falsifiable", False):
                    raise ValueError(f"acceptance 项必须 falsifiable: {acc.get('check')}")
            goal = {"goal_id": e["goal_id"], "description": e["description"],
                    "acceptance": [dict(a, passed=False) for a in e["acceptance"]],
                    "base_head": e["base_head"], "status": "active"}
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
    ready = [s["id"] for s in subs.values()
             if not s["passed"] and s["blocker"] is None
             and all(subs.get(b, {}).get("passed", False) for b in s["blocked_by"])]
    blocked = [{"id": s["id"], "blocker": s["blocker"]} for s in subs.values() if s["blocker"]]
    # 5. goal 验收：所有 acceptance CHECK_PASS → closed
    for acc in goal["acceptance"]:
        if (gid, acc["check"]) in passed_checks or any(c == acc["check"] for _, c in passed_checks):
            acc["passed"] = True
    terminated = bool(goal["acceptance"]) and all(a["passed"] for a in goal["acceptance"])
    if terminated or gid in closed:
        goal["status"] = "closed"
    elif blocked and not ready:
        goal["status"] = "blocked"
    return {"current_goal": goal, "ready_workstations": sorted(ready), "blocked": blocked, "terminated": terminated}
