"""手写小型负例；不产正式金融输入、Voice 或 P11 PASS。"""
import copy
import unittest
import time
from driver import canonical, sha, Rejected, ReadBudget
import financial_oracle as f


def fixture():
    data = {"schema_revision": "tb03b-finance/2", "kind": "TestOnly",
            "policy": {"model": "TestOnlyUnleveragedCashBuy", "allocation_policy_id": "#1349/atomic-slot-wait/v1", "cap": "80"},
            "baseline": {"responsibility_id": "base", "amount": "20"},
            "rights": [{"right_id": "ra", "owner": "B:alpha", "amount": "40", "version": "1"},
                       {"right_id": "rb", "owner": "B:beta", "amount": "50", "version": "1"}],
            "plans": [], "slots": [{"slot_id": "slot", "applicants": ["a1", "a2", "b1"]}]}
    for ident, owner, right, quantity, price, fee in (
            ("a1", "B:alpha", "ra", "1", "10", "0"),
            ("a2", "B:alpha", "ra", "2", "9", "2"),
            ("b1", "B:beta", "rb", "3", "12", "4")):
        data["plans"].append({"decision_id": ident, "owner": owner, "right_id": right,
                              "quantity": quantity, "limit_price": price, "fee_tail": fee,
                              "slot_id": "slot", "applicant_epoch": "1", "applicant_generation": "1"})
    books = {o: {"actual_quantity": "1", "fixed_q": "10", "known_cost_totals": "7",
                 "income_book": [], "unknown": {"principal_baseline": "Unknown", "capital_phase": "Unknown"}}
             for o in ("B:alpha", "B:beta")}
    def image(owner):
        return {"book": books.get(owner), "finance": {"schema_revision": "tb03b-finance/2", "rights": [],
                "decisions": [], "slots": [], "responsibilities": []}}
    images = {o: image(o) for o in ("E", *books)}
    histories = {o: [{"seq": 0, "image": copy.deepcopy(im)}] for o, im in images.items()}
    ops = []
    def event(owner, op, ident):
        seq = len(histories[owner])
        ops.append({"domain": owner, "op": op, "id": ident, "receipt_key": op + "/" + ident,
                    "outcome": f.KINDS[op], "seq": seq, "ordinal": 0})
        histories[owner].append({"seq": seq, "image": copy.deepcopy(images[owner])})
    images["E"]["finance"]["responsibilities"] = [{"responsibility_id": "base", "owner": "E:baseline", "amount": "20"}]
    event("E", "ImportResponsibility", "base")
    # actual toy 中间态独立手写数值；从不调用 audit 生成 expected 或 actual。
    for index, locked, worst, total_lock, version in ((0, "10", "2", "10", "2"),
                                                    (1, "20", "4", "30", "3"),
                                                    (2, "40", "4", "40", "2")):
        p = data["plans"][index]
        owner = p["owner"]
        images[owner]["finance"]["decisions"].append({"decision_id": p["decision_id"], "state": "Prepared",
            "execution_eligibility": "None", "grant": None, "local_commit_receipt": None,
            "prepared": {"plan": p, "plan_hash": sha(canonical(p)), "locked_amount": locked, "worst_quantity": worst}})
        images[owner]["finance"]["rights"] = [{"right_id": p["right_id"], "owner": owner,
                                                 "locked_amount": total_lock, "current_version": version}]
        event(owner, "PrepareDecision", p["decision_id"])
    images["E"]["finance"]["slots"] = [{"slot_id": "slot", "state": "AwaitingAllocationPolicy", "required_total": "90",
        "retained_total": "20", "waiting_for": "#1349/atomic-slot-wait/v1", "grants": []}]
    event("E", "Reserve", "slot")
    return data, dict(session_id="UNIT_TEST_ONLY", session_generation="1", initial_books=books,
                      initial_sequences={o: 0 for o in histories}, operations=ops, histories=histories)


class FinancialTests(unittest.TestCase):
    def audit(self, data, args):
        raw = canonical(data)
        return f.audit(raw, sha(raw), **args, budget=ReadBudget(time.monotonic()+3,1000,1000000))

    def test_multi_owner_multi_decision_waiting_middle_states(self):
        data, args = fixture()
        self.assertEqual(len(self.audit(data, args)["checked_intermediates"]), 5)

    def test_duplicate_extra_missing_and_wrong_owner_decisions(self):
        for change in ("duplicate", "extra", "missing", "wrong_owner"):
            data, args = fixture()
            rows = args["histories"]["B:alpha"][-1]["image"]["finance"]["decisions"]
            if change == "duplicate": rows.append(copy.deepcopy(rows[0]))
            if change == "extra": rows.append({**copy.deepcopy(rows[0]), "decision_id": "extra"})
            if change == "missing": rows.pop()
            if change == "wrong_owner": rows[0]["prepared"]["plan"]["owner"] = "B:beta"
            with self.subTest(change=change), self.assertRaises(Rejected):
                self.audit(data, args)

    def test_private_lock_Q_worst_and_intermediate_corruption(self):
        for change in ("q", "Q", "lock", "worst", "version"):
            data, args = fixture()
            image = args["histories"]["B:alpha"][1]["image"]
            if change == "q": image["book"]["actual_quantity"] = "2"
            if change == "Q": image["book"]["fixed_q"] = "11"
            if change == "lock": image["finance"]["rights"][0]["locked_amount"] = "0"
            if change == "worst": image["finance"]["decisions"][0]["prepared"]["worst_quantity"] = "1"
            if change == "version": image["finance"]["rights"][0]["current_version"] = "1"
            with self.subTest(change=change), self.assertRaises(Rejected):
                self.audit(data, args)

    def test_wait_does_not_grant_and_responsibility_owner_exact(self):
        for change in ("owner", "amount", "extra", "grant"):
            data, args = fixture()
            finance = args["histories"]["E"][-1]["image"]["finance"]
            if change == "owner": finance["responsibilities"][0]["owner"] = "B:alpha"
            if change == "amount": finance["responsibilities"][0]["amount"] = "0"
            if change == "extra": finance["responsibilities"].append({"responsibility_id": "extra", "amount": "1", "owner": "E"})
            if change == "grant": finance["slots"][0]["grants"] = [{"decision_id": "a1"}]
            with self.subTest(change=change), self.assertRaises(Rejected):
                self.audit(data, args)

    def test_extra_missing_reordered_persistent_commits(self):
        for change in ("extra", "missing", "reordered"):
            data, args = fixture()
            rows = args["histories"]["B:alpha"]
            if change == "extra": rows.append({"seq": 3, "image": rows[-1]["image"]})
            if change == "missing": rows.pop(1)
            if change == "reordered": rows.reverse()
            with self.subTest(change=change), self.assertRaises(Rejected):
                self.audit(data, args)

    def test_idempotent_occurrence_does_not_add_commit(self):
        data, args = fixture()
        args["operations"].append({**args["operations"][-1], "ordinal": 1})
        self.audit(data, args)
        args["operations"][-1]["ordinal"] = 2
        with self.assertRaises(Rejected): self.audit(data, args)

    def test_missing_frozen_initial_book_or_changed_input_refused(self):
        data, args = fixture()
        with self.assertRaises(Rejected):
            f.audit(canonical(data), "0"*64, **args, budget=ReadBudget(time.monotonic()+3,1000,1000000))
        args["initial_books"] = {}
        with self.assertRaises(Rejected): self.audit(data, args)

    def test_abort_keeps_private_lock_and_received_not_success(self):
        data, args = fixture()
        last = copy.deepcopy(args["histories"]["B:alpha"][-1])
        last["seq"] = 3
        last["image"]["finance"]["decisions"][0]["state"] = "AbortedBeforeCommit"
        args["histories"]["B:alpha"].append(last)
        args["operations"].append({"domain": "B:alpha", "op": "AbortDecision", "id": "a1", "receipt_key": "abort/a1",
                                    "outcome": "AbortReceipt", "seq": 3, "ordinal": 0})
        self.audit(data, args)
        last["image"]["finance"]["rights"][0]["locked_amount"] = "20"
        with self.assertRaises(Rejected): self.audit(data, args)
        data, args = fixture()
        args["operations"][0]["outcome"] = "Received"
        with self.assertRaises(Rejected): self.audit(data, args)

    def reserved_fixture(self):
        data,args=fixture()
        data["policy"]["cap"]="100"
        args["operations"][-1]["commit_ns"]="10"
        state=args["histories"]["E"][-1]["image"]["finance"]
        state["slots"][0].update(state="Reserved",retained_total="90",waiting_for=None)
        amounts={"a1":"10","a2":"20","b1":"40"}
        grants=[]
        for p in data["plans"]:
            ident=p["decision_id"]
            gid=sha(canonical(["tb03b-finance/1","UNIT_TEST_ONLY","1","slot",ident,sha(canonical(p))]))
            grant={"grant_id":gid,"decision_id":ident,"owner":p["owner"],"plan_hash":sha(canonical(p)),
                   "amount":amounts[ident],"quantity":p["quantity"],"reserved_seq":"2","reserved_ns":"10",
                   "applicant_epoch":"1","applicant_generation":"1","execution_eligibility":"None"}
            grants.append(grant)
            state["responsibilities"].append({"responsibility_id":gid,"owner":p["owner"],"amount":amounts[ident],"grant":grant})
        state["slots"][0]["grants"]=grants
        # 独立手写E原库快照；并非由audit或B的actual反推。
        rows=copy.deepcopy(args["histories"]["E"])
        previous=""
        for row in rows:
            row["commit_ns"]="10" if row["seq"]==2 else str(row["seq"])
            row["root"]=sha(canonical({"domain_id":"E","session_id":"UNIT_TEST_ONLY",
                "session_generation":"1","commit_seq":str(row["seq"]),"previous_root":previous,
                "commit_ns":row["commit_ns"],"image_hash":sha(canonical(row["image"]))}))
            previous=row["root"]
        cut={"domain_id":"E","commit_seq":"2","root_hash":rows[-1]["root"]}
        e_original={"meta":{"domain_id":"E","session_id":"UNIT_TEST_ONLY","session_generation":"1"},
                    "commits":rows,"receipts":{"Reserve/slot":{"kind":"ReserveReceipt","domain_id":"E",
                    "record":{"slot_id":"slot"},"receipt_key":"Reserve/slot","commit_ns":"10","cut":cut}}}
        args["e_original_raw"]=canonical(e_original)
        args["e_original_sha"]=sha(args["e_original_raw"])
        return data,args,grants

    def test_reserved_commit_requires_original_liability_and_time_order(self):
        data,args,grants=self.reserved_fixture()
        final=copy.deepcopy(args["histories"]["B:beta"][-1]);final["seq"]=2
        cut=f.strict(args["e_original_raw"])["receipts"]["Reserve/slot"]["cut"]
        decision=final["image"]["finance"]["decisions"][0]
        decision.update(state="Committed",grant={"original":grants[2],"e_cut":cut},
                        local_commit_receipt={"e_cut":cut,"commit_ns":"11"})
        args["histories"]["B:beta"].append(final)
        args["operations"].append({"domain":"B:beta","op":"CommitDecision","id":"b1","receipt_key":"commit/b1",
                                   "outcome":"LocalCommitReceipt","seq":2,"ordinal":0})
        self.audit(data,args)
        decision["local_commit_receipt"]["commit_ns"]="9"
        with self.assertRaises(Rejected):self.audit(data,args)

    def test_closed_applicants_keep_liability_and_persistent_set(self):
        data,args,grants=self.reserved_fixture()
        final=copy.deepcopy(args["histories"]["E"][-1]);final["seq"]=3
        closed={"owner":"B:alpha","applicant_epoch":"1","applicant_generation":"1"}
        final["image"]["finance"]["closed_applicants"]=[{"slot_id":"slot",**closed}]
        args["histories"]["E"].append(final)
        args["operations"].append({"domain":"E","op":"CloseApplicants","id":"slot","receipt_key":"close/slot",
                                   "outcome":"ApplicantsClosedReceipt","seq":3,"ordinal":0,"closed":[closed]})
        self.audit(data,args)
        final["image"]["finance"]["responsibilities"].pop()
        with self.assertRaises(Rejected):self.audit(data,args)
