"""冻结原输入 → 独算资金不变量 → 每个持久中间态，禁止导入生产 reducer。

此核验器只处理已冻结的 TestOnlyUnleveragedCashBuy 六种财务操作；初始账、完整
操作顺序和各次持久 seq 均须外部冻结，不能从 actual 倒推。S/Voice 权威独立阻断。
"""
from collections import defaultdict
import copy
from driver import canonical, sha, strict, require, ReadBudget
from oracle import amount


KINDS = {"ImportResponsibility": "ResponsibilityImportedReceipt",
         "PrepareDecision": "PreparedReceipt", "Reserve": "ReserveReceipt",
         "CommitDecision": "LocalCommitReceipt", "AbortDecision": "AbortReceipt",
         "CloseApplicants": "ApplicantsClosedReceipt"}


def unique(rows, key):
    result = {}
    require(type(rows) is list, "核账集合不是数组")
    for row in rows:
        require(row[key] not in result, "核账集合重复：" + key)
        result[row[key]] = row
    return result


def total(values):
    value = sum(values)
    require(0 <= value < 2**63, "核账精确整数溢出")
    return value


def audit(finance_raw, finance_sha, *, session_id, session_generation,
          initial_books, initial_sequences, operations, histories, budget,
          e_original_raw=None, e_original_sha=None):
    require(type(budget) is ReadBudget, "核账必须沿用共享 Job 预算")
    budget.charge((finance_raw,))
    require(sha(finance_raw) == finance_sha, "冻结资金输入摘要不符")
    data = strict(finance_raw)
    require(data["schema_revision"] == "tb03b-finance/2" and data["kind"] == "TestOnly",
            "不是当前具名资金输入")
    require(data["policy"]["model"] == "TestOnlyUnleveragedCashBuy"
            and data["policy"]["allocation_policy_id"] == "#1349/atomic-slot-wait/v1",
            "核账不选择新资金政策")
    rights, plans, slots = (unique(data[k], key) for k, key in
                           (("rights", "right_id"), ("plans", "decision_id"), ("slots", "slot_id")))
    require(plans and operations and initial_books, "缺冻结计划/完整操作/原开局")
    owners = {p["owner"] for p in plans.values()}
    require(set(initial_books) == owners and set(initial_sequences) == owners | {"E"}
            and set(histories) == owners | {"E"}, "核账 owner/原库全集不同")
    bounds = {}
    for ident, plan in plans.items():
        budget.check()
        right = rights[plan["right_id"]]
        require(right["owner"] == plan["owner"], "私有权利 owner 错绑")
        bounds[ident] = total([amount(plan["quantity"]) * amount(plan["limit_price"]), amount(plan["fee_tail"])])
        require(bounds[ident] <= amount(right["amount"]), "计划超私有权利")
    for slot in slots.values():
        require(len(slot["applicants"]) == len(set(slot["applicants"])) and slot["applicants"],
                "完整申请集合重复/为空")
        require(all(i in plans and plans[i]["slot_id"] == slot["slot_id"] for i in slot["applicants"]),
                "完整申请集合与计划不同")
    indexed, expected_seqs = {}, {}
    for owner, rows in histories.items():
        indexed[owner] = unique(rows, "seq")
        require(rows and [r["seq"] for r in rows] == sorted(indexed[owner]), "历史顺序不符")
        expected_seqs[owner] = [initial_sequences[owner]]
        require(rows[0]["seq"] == initial_sequences[owner], "原开局 seq 不同")
    decisions, grants, slot_states, closed = {}, {}, {}, set()
    liabilities, imported, seen_requests = {}, False, {}
    cap, baseline = amount(data["policy"]["cap"]), amount(data["baseline"]["amount"])
    observations = []
    e_cuts = {}
    if any(e["op"] == "CommitDecision" and e["outcome"] == "LocalCommitReceipt" for e in operations):
        require(type(e_original_raw) is bytes and e_original_sha is not None,
                "本地提交缺独立E持久原件/pin")
        budget.charge((e_original_raw,))
        require(sha(e_original_raw) == e_original_sha, "独立E原件摘要不符")
        original = strict(e_original_raw)
        meta = original["meta"]
        require(meta.get("domain_id") == "E" and meta.get("session_id") == session_id
                and meta.get("session_generation") == session_generation, "独立E原库域/会话/代际不同")
        previous = ""
        for seq, row in enumerate(original["commits"]):
            budget.check()
            require(row["seq"] == seq, "独立E历史必须从0完整连续")
            root = sha(canonical({"domain_id": "E", "session_id": session_id,
                "session_generation": session_generation, "commit_seq": str(seq),
                "previous_root": previous, "commit_ns": row["commit_ns"],
                "image_hash": sha(canonical(row["image"]))}))
            require(row["root"] == root, "独立E提交根链不符")
            require(seq in indexed["E"] and row["image"] == indexed["E"][seq]["image"],
                    "独立E原库与核账历史不符")
            e_cuts[seq] = {"domain_id": "E", "commit_seq": str(seq), "root_hash": root}
            previous = root
        require(set(e_cuts) == set(indexed["E"]), "独立E原库历史全集缺漏")
        for event in operations:
            if event["domain"] == "E" and event["op"] == "Reserve" and event["outcome"] == "ReserveReceipt":
                row = original["commits"][event["seq"]]
                receipt = original["receipts"].get(event["receipt_key"])
                require(type(receipt) is dict and receipt.get("kind") == "ReserveReceipt"
                        and receipt.get("domain_id") == "E" and receipt.get("record", {}).get("slot_id") == event["id"]
                        and receipt.get("receipt_key") == event["receipt_key"]
                        and receipt.get("commit_ns") == row["commit_ns"]
                        and receipt.get("cut") == e_cuts[event["seq"]]
                        and row["commit_ns"] == event["commit_ns"], "Reserve原提交/回执/时间身份不同")

    def check_image(owner, seq):
        budget.check()
        require(seq in indexed[owner], "持久库漏提交")
        image = indexed[owner][seq]["image"]
        budget.charge((canonical(image),))
        finance = image["finance"]
        require(finance["schema_revision"] == "tb03b-finance/2", "持久财务版本不同")
        actual_decisions = unique(finance["decisions"], "decision_id")
        actual_slots = unique(finance["slots"], "slot_id")
        actual_rights = unique(finance["rights"], "right_id")
        actual_liabilities = unique(finance["responsibilities"], "responsibility_id")
        expected_decisions = {k: v for k, v in decisions.items() if plans[k]["owner"] == owner}
        require(set(actual_decisions) == set(expected_decisions), "额外/遗漏 decision 或责任 owner 不符")
        if owner == "E":
            closed_rows = finance.get("closed_applicants", [])
            actual_closed = [(r["slot_id"], r["owner"], r["applicant_epoch"], r["applicant_generation"]) for r in closed_rows]
            require(len(actual_closed) == len(set(actual_closed)) and set(actual_closed) == closed,
                    "关闭申请者持久集合遗漏/额外/重复")
            require(not actual_rights and set(actual_slots) == set(slot_states), "E slot/私有权利错置")
            require(set(actual_liabilities) == set(liabilities), "责任身份全集不符")
            for ident, (who, value) in liabilities.items():
                require(actual_liabilities[ident]["owner"] == who and
                        actual_liabilities[ident]["amount"] == str(value), "责任金额/归属不符")
            for ident, wanted in slot_states.items():
                actual = actual_slots[ident]
                for field in ("state", "required_total", "retained_total", "waiting_for"):
                    require(actual[field] == wanted[field], "slot 中间态不符：" + field)
                actual_grants = unique(actual["grants"], "decision_id")
                require(set(actual_grants) == set(wanted["grants"]), "授信额外/遗漏")
                for decision_id in wanted["grants"]:
                    g = actual_grants[decision_id]
                    for field, value in grants[decision_id].items():
                        require(g[field] == value, "grant 绑定不符：" + field)
                    require(actual_liabilities[g["grant_id"]]["grant"] == g, "责任未保留完整 grant 原件")
        else:
            require(not actual_slots and not actual_liabilities, "B 不得持 E 责任/slot")
            require(image["book"] == initial_books[owner], "q/固定Q/成本/收入/unknown 原开局改变")
            locked, count = defaultdict(int), defaultdict(int)
            for ident, wanted in expected_decisions.items():
                budget.check()
                actual, plan = actual_decisions[ident], plans[ident]
                require(actual["state"] == wanted["state"] and actual["execution_eligibility"] == "None",
                        "决定中间态/场所准入错误")
                prepared = actual["prepared"]
                require(prepared["plan"] == plan and prepared["plan_hash"] == sha(canonical(plan)), "原计划变化")
                require(prepared["locked_amount"] == str(bounds[ident])
                        and prepared["worst_quantity"] == str(wanted["worst"]), "锁/最坏量中间态错误")
                if wanted["state"] != "Committed":
                    require(actual["grant"] is None and actual["local_commit_receipt"] is None,
                            "Prepared/Aborted 提前取得 grant/commit")
                else:
                    require(actual["grant"] is not None and actual["local_commit_receipt"] is not None,
                            "Committed 缺原 grant/receipt")
                    for field, value in grants[ident].items():
                        require(actual["grant"]["original"][field] == value, "本地 grant 内容不符")
                    require(actual["local_commit_receipt"]["e_cut"] == actual["grant"]["e_cut"]
                            == e_cuts[int(grants[ident]["reserved_seq"])],
                            "本地提交 E cut 不符")
                    require(amount(actual["local_commit_receipt"]["commit_ns"]) >= amount(grants[ident]["reserved_ns"]),
                            "B Commit 早于 E 保留责任")
                locked[plan["right_id"]] += bounds[ident]
                count[plan["right_id"]] += 1
            require(set(actual_rights) == set(locked), "私有锁权利全集不符")
            for ident, value in locked.items():
                require(actual_rights[ident]["owner"] == owner and
                        actual_rights[ident]["locked_amount"] == str(value) and
                        actual_rights[ident]["current_version"] == str(amount(rights[ident]["version"]) + count[ident]),
                        "私有锁被退还/版本错误")
        return image

    for owner, seq in initial_sequences.items():
        check_image(owner, seq)
    for event in operations:
        budget.charge((canonical(event),))
        owner, op, key = event["domain"], event["op"], event["receipt_key"]
        require(owner in histories and op in KINDS, "未定义操作/域；不能生成 expected")
        req_identity = (owner, key)
        occurrence = seen_requests.get(req_identity)
        if occurrence is not None:
            prior, ordinal = occurrence
            require(event["ordinal"] == ordinal + 1 and
                    {k: v for k, v in event.items() if k != "ordinal"} == prior, "幂等重复身份/结果变化")
            seen_requests[req_identity] = (prior, event["ordinal"])
            continue
        require(event["ordinal"] == 0, "初次请求出现序号错误")
        seen_requests[req_identity] = ({k: v for k, v in event.items() if k != "ordinal"}, 0)
        if event["outcome"] != KINDS[op]:
            require(event["outcome"].startswith("Rejected/") and event["seq"] is None,
                    "不支持/Received 不能当业务结果")
            continue
        seq = event["seq"]
        require(type(seq) is int and seq == expected_seqs[owner][-1] + 1, "财务提交 seq 缺漏/额外")
        expected_seqs[owner].append(seq)
        ident = event["id"]
        if op == "ImportResponsibility":
            require(owner == "E" and not imported and ident == data["baseline"]["responsibility_id"], "基责重导/错绑")
            imported = True
            liabilities[ident] = ("E:baseline", baseline)
        elif op == "PrepareDecision":
            require(ident in plans and plans[ident]["owner"] == owner and ident not in decisions, "准备决定重复/owner 不符")
            plan = plans[ident]
            locked = total(bounds[i] for i in decisions if plans[i]["right_id"] == plan["right_id"])
            require(total([locked, bounds[ident]]) <= amount(rights[plan["right_id"]]["amount"]), "累计私有锁超权利")
            worst = total([amount(initial_books[owner]["actual_quantity"]), amount(plan["quantity"])] +
                          [amount(plans[i]["quantity"]) for i in decisions if plans[i]["owner"] == owner])
            require(worst <= amount(initial_books[owner]["fixed_q"]), "累计最坏量超固定 Q")
            decisions[ident] = {"state": "Prepared", "worst": worst}
        elif op == "Reserve":
            require(owner == "E" and imported and ident in slots and ident not in slot_states, "Reserve 缺基责/slot重复")
            require(all(s["state"] != "AwaitingAllocationPolicy" for s in slot_states.values()), "等待期间其他 slot 插队")
            active = [i for i in slots[ident]["applicants"] if (ident, plans[i]["owner"], plans[i]["applicant_epoch"], plans[i]["applicant_generation"]) not in closed]
            require(active and all(decisions.get(i, {}).get("state") == "Prepared" for i in active), "完整 Prepared 不齐")
            existing = total(value for _, value in liabilities.values())
            needed = total([existing] + [bounds[i] for i in active])
            fits = needed <= cap
            slot_states[ident] = {"state": "Reserved" if fits else "AwaitingAllocationPolicy",
                                  "required_total": str(needed), "retained_total": str(needed if fits else existing),
                                  "waiting_for": None if fits else data["policy"]["allocation_policy_id"],
                                  "grants": active if fits else []}
            if fits:
                for i in active:
                    p = plans[i]
                    gid = sha(canonical(["tb03b-finance/1", session_id, session_generation, ident, i, sha(canonical(p))]))
                    grants[i] = {"grant_id": gid, "decision_id": i, "owner": p["owner"],
                                 "plan_hash": sha(canonical(p)), "amount": str(bounds[i]),
                                 "quantity": p["quantity"], "reserved_seq": str(seq),
                                 "reserved_ns": event["commit_ns"],
                                 "applicant_epoch": p["applicant_epoch"], "applicant_generation": p["applicant_generation"],
                                 "execution_eligibility": "None"}
                    liabilities[gid] = (p["owner"], bounds[i])
        elif op in ("CommitDecision", "AbortDecision"):
            require(ident in decisions and plans[ident]["owner"] == owner
                    and decisions[ident]["state"] == "Prepared", "提交/Abort 状态或 owner 不合法")
            if op == "CommitDecision":
                p = plans[ident]
                require(ident in grants and (p["slot_id"], owner, p["applicant_epoch"], p["applicant_generation"]) not in closed,
                        "提交缺共同责任或申请代际已关闭")
            decisions[ident]["state"] = "Committed" if op == "CommitDecision" else "AbortedBeforeCommit"
        else:
            require(owner == "E" and ident in slots and event["closed"], "关闭申请集合未冻结")
            for item in event["closed"]:
                entry = (ident, item["owner"], item["applicant_epoch"], item["applicant_generation"])
                require(any((p["slot_id"], p["owner"], p["applicant_epoch"], p["applicant_generation"]) == entry
                            for p in plans.values()), "关闭外域申请者")
                closed.add(entry)
        check_image(owner, seq)
        observations.append({"domain": owner, "seq": seq, "op": op})
    for owner in histories:
        budget.check()
        require([r["seq"] for r in histories[owner]] == expected_seqs[owner], "持久提交全集额外/遗漏")
    budget.check()
    return {"scope": "INDEPENDENT_FINANCIAL_INVARIANTS_ONLY", "input_sha256": finance_sha,
            "checked_intermediates": observations, "product_status": "NOT_RUN", "execution_authorized": False,
            "not_verified": ["S/Voice权威", "六族结构业务语义", "真实完整历史", "资金原件的外部授权"]}
