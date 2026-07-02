"""D′ goal reducer：纯函数 (events + facts) → ready workstations projection。
确定性：同输入同输出（无 IO、无时间、无随机）。这是 RTAS 蜂群递归展开的确定性规则。"""

def _validate_acceptance(acceptance: list[dict]) -> None:
    """校验每个 acceptance 项必须 falsifiable（lesson 0011 锋利问题②：验收不可证伪则 goal 永动空转）。"""
    for acc in acceptance:
        if not acc.get("falsifiable", False):
            raise ValueError(f"acceptance 项必须 falsifiable: {acc.get('check')}")


def amend_gid(e: dict) -> str | None:
    """goal id：规范 goal_id 优先，退化历史事件 fallback sub_goal_id（reader 有效域）。"""
    return e.get("goal_id") or e.get("sub_goal_id")


def apply_acceptance_amendments(
    acceptance: list[dict], events: list[dict], gid: str
) -> list[dict]:
    """对 GOAL_SET 原始 acceptance 应用该 gid 的所有 GOAL_AMEND，得到当前 acceptance vector。

    单一权威口径（#102）：reducer 与 writer 归属校验共用此函数，杜绝两份发散逻辑。
    按物理序遍历：ACCEPTANCE_ID_BINDING 注入缺失 id；ACCEPTANCE_REPLACE 整体替换 vector
    （最后写者胜）。reader 宽容——结构错误的 binding（ordinal 越界/slot 已绑）忠实跳过。
    输入 acceptance 不被原地修改（返回新列表，dict 浅拷贝）。"""
    current = [dict(a) for a in acceptance]
    for e in events:
        if e["event"] != "GOAL_AMEND" or amend_gid(e) != gid:
            continue
        kind = e.get("amendment_kind")
        if kind == "ACCEPTANCE_ID_BINDING":
            for b in e.get("bindings", []):
                o = b.get("ordinal")
                if isinstance(o, int) and 0 <= o < len(current) \
                        and current[o].get("id") is None:
                    current[o]["id"] = b.get("acceptance_id")
        elif kind == "ACCEPTANCE_REPLACE":
            new_acc = e.get("acceptance", [])
            _validate_acceptance(new_acc)
            current = [dict(a) for a in new_acc]
    return current


def _build_pass_state(events: list[dict]) -> tuple[dict, dict, set, set]:
    """从 CHECK_PASS/CHECK_FAIL 重建验收态（goal 无关，一次构建供全体 gid 复用）。

    两键都按 sub_goal_id 作用域隔离（codex MAJOR-2：acceptance_id 非全局唯一，任意
    sub_goal 的 CHECK_PASS 不得误闭合 goal acceptance）。两键互斥（codex v2 MAJOR）：带
    acceptance_id → 只走稳定身份键 (sub_goal_id, acceptance_id)；否则 fallback check 文本键
    (sub_goal_id, check)。值=最后写者（PASS=True/FAIL=False）——658 补偿事件序：CHECK_FAIL
    对同一键写 False，PASS→FAIL→PASS 正确反映最终态。contested 集记曾被 CHECK_FAIL 撤销的
    键，供严格闭合区分「从未通过」与「曾通过后被争议撤销」。"""
    passed_accept_ids: dict = {}
    passed_checks: dict = {}
    contested_accept_ids: set = set()
    contested_checks: set = set()
    for e in events:
        if e["event"] not in ("CHECK_PASS", "CHECK_FAIL"):
            continue
        sid = e.get("sub_goal_id")
        ok = e["event"] == "CHECK_PASS"
        if e.get("acceptance_id"):
            passed_accept_ids[(sid, e["acceptance_id"])] = ok
            if not ok:
                contested_accept_ids.add((sid, e["acceptance_id"]))
        else:
            passed_checks[(sid, e.get("check"))] = ok
            if not ok:
                contested_checks.add((sid, e.get("check")))
    return passed_accept_ids, passed_checks, contested_accept_ids, contested_checks


def _apply_pass_state(
    acceptance: list[dict], gid: str,
    pass_state: tuple[dict, dict, set, set],
) -> list[dict]:
    """把验收态应用到 gid 的 acceptance 向量，返回带 passed/contested 的新向量（不改入参）。

    单一权威口径（#102）：active-set 剔 terminated 与主路径闭合判定共用此函数，杜绝两份
    发散的闭合逻辑。匹配优先稳定身份 (gid, acc.id)，无 id 时 fallback check 文本 (gid, check)，
    两键都 gid-scoped（防跨 sub_goal 的 id/check 名碰撞误闭合）。"""
    passed_accept_ids, passed_checks, contested_accept_ids, contested_checks = pass_state
    out = []
    for a in acceptance:
        acc = dict(a, passed=a.get("passed", False))
        akey = (gid, acc["id"]) if acc.get("id") else None
        ckey = (gid, acc["check"])
        if akey is not None and akey in passed_accept_ids:
            acc["passed"] = passed_accept_ids[akey]
            if akey in contested_accept_ids:
                acc["contested"] = True
        elif ckey in passed_checks:
            acc["passed"] = passed_checks[ckey]
            if ckey in contested_checks:
                acc["contested"] = True
        out.append(acc)
    return out


def _is_terminated(acceptance: list[dict]) -> bool:
    """严格闭合（codex #7 MAJOR + 编排者裁定 CHOICE）：非空且每项 passed 且 **not contested**。

    曾被 CHECK_FAIL 争议的 acceptance 即使后续 CHECK_PASS（passed=True+contested=True）仍不
    闭合——保守安全，防 PASS→FAIL→PASS 误闭合。复判路径（CHECK_RESOLVE 显式清 contested）
    是未实装的升级点（编排者裁定保守优先）。"""
    return bool(acceptance) and all(
        a["passed"] and not a.get("contested") for a in acceptance)


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
    # reader/writer 有效域分层（formalization-validity-domain 显式标注，非兼容垫片）：
    # reducer 是 event-sourcing reader，有效域=「全部历史 events.jsonl（含 actor 手搓退化
    # 事件，用 sub_goal_id 当 goal id）」——必须读不崩（630 开口①止血目的）。writer
    # (goal_events.append_event) 是新写路径，有效域=「仅新事件」，严格拒绝退化。两者有效域
    # 不同是分工不是矛盾：reader 宽容读历史（有效域更大），writer 严格守新写（有效域更小）。
    # _gid 的 fallback 只服务 reader 的历史有效域；新事件由 writer 保证 goal_id 存在，永不走
    # fallback。改 reducer 严格化会让历史行25/53/61 崩，破坏已结算止血（no-workaround）。
    def _gid(e):  # goal id：规范 goal_id 优先，退化历史事件 fallback sub_goal_id（reader 有效域）
        return e.get("goal_id") or e.get("sub_goal_id")
    superseded = {e.get("old_goal_id") or e.get("sub_goal_id")
                  for e in events if e["event"] == "SUPERSEDE"}
    closed = {_gid(e) for e in events if e["event"] == "CLOSED"}
    # current goal = active set 语义（codex #1 CRITICAL silent goal-loss 修复）：
    # active = {所有 GOAL_SET 的 gid} − superseded − closed。旧 last-writer-wins（for 循环
    # 物理最后一个未 superseded 的 GOAL_SET）会静默吞掉多个未显式终止的 active goal（实证：
    # mutexlevel 被 overfitframework 静默盖掉）。物理 append order 是 event-sourcing 因果序，
    # 但「最后一个」不能用作单一 current goal 选择——两个都 active 是状态机歧义，须显式暴露。
    #   len(active)==0 → current_goal=None（无 active goal，seed bootloader 不阻塞冷启动）
    #   len(active)==1 → 该 goal 为 current
    #   len(active)>1  → current_goal=None + AMBIGUOUS_ACTIVE_GOALS（不产 ready、不 fallback，
    #                    须显式 SUPERSEDE 消歧；编排者裁定走 /escalate 而非静默选一个）
    # 退化历史事件经 _gid fallback 仍参与 active 集（reader 有效域：sub_goal_id 当 gid）。
    goal_sets = {}  # gid → 最后一个同 gid 的 GOAL_SET 事件（同 gid 重复 SET 取最后，幂等续写）
    for e in events:
        if e["event"] == "GOAL_SET":
            goal_sets[_gid(e)] = e
    # 验收态一次构建（goal 无关），供 active-set 剔 terminated 与主路径闭合判定复用（#102）。
    pass_state = _build_pass_state(events)

    def _terminated_gid(g):
        # g 的全部 acceptance 是否 CHECK_PASS 且未 contested（严格闭合，从验收事实派生）。
        # 供下方歧义判定剔 terminated——terminated goal 已完成、无未竟工作，不是竞争中的方向。
        amended = apply_acceptance_amendments(goal_sets[g].get("acceptance", []), events, g)
        return _is_terminated(_apply_pass_state(amended, g, pass_state))

    active_ids = sorted(gid for gid in goal_sets if gid not in superseded and gid not in closed)
    # dangling-active 根治（codex#1 active-set 语义的精化）：歧义只在「有未竟工作」的 live goal
    # 间判定。terminated
    # goal（全 acceptance CHECK_PASS 且未 contested）已完成，不是竞争中的方向——不参与歧义计数。
    # 原实现把 terminated-未-CLOSED goal 计入 active 歧义：一旦新 GOAL_SET 抢在 loop 写 CLOSED 前
    # 追加，active 集出现两元素 → AMBIGUOUS_ACTIVE_GOALS → current_goal=None（本 session 实发）。
    # 根因是歧义域误用（等冗余 CLOSED 事件补录才消歧，而非从验收事实派生「已完成→不竞争」）。
    live_ids = [gid for gid in active_ids if not _terminated_gid(gid)]
    if len(live_ids) > 1:
        return {"current_goal": None, "ready_workstations": [], "ready_details": [],
                "blocked": [{"type": "AMBIGUOUS_ACTIVE_GOALS", "ids": live_ids}],
                "terminated": False}
    # 选择：优先唯一 live goal；无 live 但 active 非空时取字典序末位 terminated goal——它待闭合
    # （status=closed/terminated=True），供 ceremony_scan 停 spawn（codex#5）+ loop 写 CLOSED 审计
    # 并逐个排空（done goal 无 goal-loss 风险，与 codex#1 只防 live goal 静默丢失一致）。全空→None。
    sel_gid = live_ids[0] if len(live_ids) == 1 else (active_ids[-1] if active_ids else None)
    goal = None
    if sel_gid is not None:
        e = goal_sets[sel_gid]
        acceptance = e.get("acceptance", [])
        _validate_acceptance(acceptance)
        goal = {"goal_id": _gid(e),
                "description": e.get("description") or e.get("artifact", ""),
                "acceptance": [dict(a, passed=False) for a in acceptance],
                "base_head": e.get("base_head"), "status": "active",
                "base_head_stale": e.get("base_head") != facts.get("git_head")}
    if goal is None:
        return {"current_goal": None, "ready_workstations": [], "ready_details": [],
                "blocked": [], "terminated": False}

    gid = goal["goal_id"]
    # GOAL_AMEND 应用（codex 严格解法 2026-06-30）：寻址层补 id——把 bindings 的
    # ordinal→acceptance_id 注入 acceptance slot（in-memory 补 id），使后续 CHECK_PASS 的
    # acceptance_id 能稳定匹配。goal_id/base_head/acceptance 语义不变（非 SUPERSEDE）。
    # reader 宽容：writer 已严格校验 amend（hash/ordinal/唯一/幂等）；reducer 只应用结构
    # 正确的 binding——ordinal 越界或 slot 已绑 id 时忠实跳过（不崩，不覆盖已有 id）。
    # ACCEPTANCE_REPLACE（#3）：整体替换 acceptance vector（DRAFT 转正/验收改写，goal_id
    # /base_head 不变）。重建 vector 后旧 acceptance 的 CHECK_PASS 自然失配（验收标准变了，
    # 旧通过作废=正确语义，codex「重建 acceptance vector」）。writer 守门 old_acceptance_vector_hash
    # 匹配 + 防篡改；reducer 只忠实应用结构正确的 REPLACE。多个 REPLACE 按物理序，最后写者胜。
    # 单一权威口径（#102）：amendment 应用抽到 apply_acceptance_amendments，writer 归属校验复用同函数。
    amended = apply_acceptance_amendments(goal["acceptance"], events, gid)
    goal["acceptance"] = [dict(a, passed=a.get("passed", False)) for a in amended]
    # base_head 是不可变历史锚（650 裁决，codex 议题二 verdict=B）：GOAL_SET 时刻的 git sha，
    # 永不被 RESUME 改写。base_head≠git_head 是正确的降级信号（base_head_stale=True，spec §9）——
    # 旧 EVIDENCE 可能未覆盖当前 HEAD。GOAL_RESUME 是纯审计事件（session 恢复留痕），不参与
    # base_head/acceptance/闭合语义。删 RESUME 重锚（A 立场=抹掉漂移预警，不自洽已否决）后，
    # 历史中带 base_head 的 RESUME 事件被 reader 忠实忽略（不当语义输入，符合有效域分层）。
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
    # 3. 应用 CHECK_PASS / CHECK_FAIL / BLOCKED 到 sub-goal 树（sub_goal_id 缺失的退化事件
    # 安全跳过）。goal acceptance 的闭合态已由 pass_state（_build_pass_state）统一重建，此处只
    # 把 CHECK_PASS/FAIL 的最后写者态与 BLOCKED 落到 subs（DECOMPOSE 声明的 sub-goal）。
    for e in events:
        if e["event"] in ("CHECK_PASS", "CHECK_FAIL") and e.get("sub_goal_id") in subs:
            subs[e["sub_goal_id"]]["passed"] = e["event"] == "CHECK_PASS"
        elif e["event"] == "BLOCKED" and e.get("sub_goal_id") in subs:
            subs[e["sub_goal_id"]]["blocker"] = e.get("blocker")
    # 4. ready = 未 passed + 未 blocker + 所有 blocked_by 已 passed
    # blocked_by 指向不存在的 sub_goal 时 subs.get(b,{}) 返回空 dict → passed=False → 永久 blocked（by-design：
    # 依赖未声明的前置即视为未满足，宁可保守不 ready，不静默放行）
    ready = [s["id"] for s in subs.values()
             if not s["passed"] and s["blocker"] is None
             and all(subs.get(b, {}).get("passed", False) for b in s["blocked_by"])]
    blocked = [{"id": s["id"], "blocker": s["blocker"]} for s in subs.values() if s["blocker"]]
    # ready_details：ready sub_goal 的 {id, desc}，供 ceremony_scan 生成有意义的
    # workstation name/description（开口② reducer 驱动 spawn 的前置——id 列表不带 desc，
    # 工位无从知道做什么）。与 ready_workstations(id 列表)并存，按 id 同序，保持确定性。
    ready_details = [{"id": s["id"], "desc": subs[s["id"]]["desc"]} for s in
                     sorted((subs[i] for i in ready), key=lambda x: x["id"])]
    # 5. goal 验收：把统一重建的 pass_state 应用到本 goal acceptance（闭合逻辑见
    # _apply_pass_state：稳定身份优先、gid-scoped、658 最后写者+contested 撤销语义）。
    goal["acceptance"] = _apply_pass_state(goal["acceptance"], gid, pass_state)
    # 严格闭合（codex #7 MAJOR + 编排者裁定 CHOICE：严格闭合 vs 复判）：terminated 要求每个
    # acceptance passed 且 **not contested**。曾被 CHECK_FAIL 争议的 acceptance 即使后续
    # CHECK_PASS（passed=True+contested=True），仍不闭合 goal——保守安全，防 PASS→FAIL→PASS
    # 序列误闭合（注释「not CHECK_FAIL'd」此前未在代码执行，是真 bug）。
    # 当前取严格闭合。复判路径（需 CHECK_RESOLVE 事件显式清 contested 后才闭合）是未实装的
    # 扩展点：新增 amendment_kind/event 清 contested 标记，再让 terminated 忽略已 resolve 的
    # contested。本工位不实装复判（编排者裁定保守优先），留此注释为升级路径锚点。
    # 4b. acceptance-驱动 ready（630 开口②闭合）：goal 有 acceptance 但**无 DECOMPOSE**
    # （subs 空）时，open(passed=False) 的 acceptance 项**本身就是工位单元**——每项一个
    # ready 条目，蜂群 spawn 去把该验收做 pass。此前 ready 只从 DECOMPOSE sub_goals 派生，
    # 无分解的 goal 恒产 ready=[]（实证：g-20260701T200047Z-8f4f50e7 有 4 open acceptance
    # 却 ready 空）→ /goal 循环无工位可 spawn。DECOMPOSE 是可选的更细分解：subs 非空时
    # 由 sub_goals 驱动（不变），subs 空时 fall back 到 acceptance。二者互斥不重复计数。
    # id 取 acceptance_id（无则 fallback check 文本，与闭合匹配同口径）。blocked 不虚构：
    # BLOCKED 事件键在 sub_goal_id，acceptance 项无对应机器可读 blocker，acceptance 文本里
    # 的依赖声明（如「κ未定则blocked」）是自然语言，不发明 NLP 解析——照实全 open 项为 ready。
    if not subs:
        open_acc = [a for a in goal["acceptance"] if not a["passed"]]
        ready_details = [{"id": a.get("id") or a["check"], "desc": a["check"]}
                         for a in open_acc]
        ready = sorted(d["id"] for d in ready_details)
        ready_details.sort(key=lambda d: d["id"])
    # current_goal 优先取 live goal（此时 terminated=False）；仅当无 live goal 时取待闭合的
    # terminated goal（terminated=True/status=closed，供 loop 写 CLOSED 审计 + ceremony_scan 停
    # spawn）。dangling-active 根治把「歧义」限定在 live goal 间，见上方 live_ids 注释。
    terminated = _is_terminated(goal["acceptance"])
    # gid 不再可能 in closed（active-set 已剔 closed），保留 status=closed 仅由 terminated 驱动。
    if terminated:
        goal["status"] = "closed"
    elif blocked and not ready:
        goal["status"] = "blocked"
    return {"current_goal": goal, "ready_workstations": sorted(ready),
            "ready_details": ready_details, "blocked": blocked, "terminated": terminated}
