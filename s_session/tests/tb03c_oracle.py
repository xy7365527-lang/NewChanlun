#!/usr/bin/env python3
"""#1406 TB-03-C：tb03c-book/1 独立账簿 oracle。

只读 fixtures/tb03c/ 的原件字节与 HAND-ORACLE.json 的步骤清单，自行解析、自行核账；
不导入、不调用任何生产 reducer、查询投影或数据库。口径：
- 成本基 K（多头）= Σacquisition_cash + Σcommission + Σfee_only − Σdisposal_cash − Σdividend 净额，有符号不 clamp
  （ADR 0024 裁定四；ADR 0031 裁定三/五）。
- 本金 P = Σ principal_contribution 批；P_recovered 只由 recover_capital 增加且不超过 P（ADR 0031 裁定一；根裁定 D-8）。
- growth_eligible：无本金批 NotEvaluable；多头 K≤0 ∧ P_recovered≥P；空头 费后累计已实现利润 ≥ ΣM_b（ADR 0031 裁定一/三）。
- cost_remaining：多头 K；空头 ΣM_b − (Σ具名实现毛利 − Σ全部费用分量)（ADR 0036 裁定五）。
- 单位量：多头 q=0 ⟹ NotComputable{quantity_zero}，否则整数对 {K,q}（ADR 0031 裁定二/四）；空头无单位成本。
- 阶段事件史只追加；latch 只升不降；首次资格为真记 SimulatedGateReached（TestOnly），
  growth_enabled 在 PostGatePolicy 未签时为 AwaitingPolicy（ADR 0036 裁定七/八）。
  空头无赚股第三相（SPEC C05.5）：latch 最高到 CapitalRecovered。
- 分配：Balinski–Young 配额法，并列先给冻结请求量较大者、再按 tie_break_order（根裁定 OQ-1，ADR 0032 修订）。
"""
from fractions import Fraction
import json
from pathlib import Path
import sys

I64 = (-(2 ** 63), 2 ** 63 - 1)
DU0_KINDS = {"fee_only", "principal_contribution", "recover_capital", "dividend_cash", "withholding_tax"}
DU0_ONLY = {"fee_only", "recover_capital", "dividend_cash", "withholding_tax"}
SHORT_KINDS = {"commission", "fee_only", "principal_contribution"}
LATCH = ["CostReduction", "CapitalRecovered", "EarningShares"]


class Typed(Exception):
    def __init__(self, kind):
        super().__init__(kind)
        self.kind = kind


def no_float(v):
    if isinstance(v, float):
        raise Typed("InvalidDomain")
    if isinstance(v, list):
        for x in v:
            no_float(x)
    if isinstance(v, dict):
        for x in v.values():
            no_float(x)


def num(text):
    if not isinstance(text, str):
        raise Typed("InvalidDomain")
    value = int(text)
    if str(value) != text or not I64[0] <= value <= I64[1]:
        raise Typed("InvalidDomain")
    return value


def ceil_div(a, b):
    return -((-a) // b)


def apportion(total, weights, tie_rank):
    """配额法：逐单位在 a_i+1 ≤ ⌈f·q_i/Q⌉ 者中取 q_i/(a_i+1) 最大者；并列取 q_i 大者，再取 tie_rank 小者。"""
    q_sum, a = sum(weights), [0] * len(weights)
    for f in range(1, total + 1):
        eligible = [i for i, w in enumerate(weights) if w > 0 and a[i] + 1 <= ceil_div(f * w, q_sum)]
        best = min(eligible, key=lambda i: (-Fraction(weights[i], a[i] + 1), -weights[i], tie_rank[i]))
        a[best] += 1
    return a


def largest_remainder(total, weights, tie_rank):
    """ADR 0032 原文的最大余数法，只用于复现其单调性反例。"""
    q_sum = sum(weights)
    base = [total * w // q_sum for w in weights]
    rem = [Fraction(total * w, q_sum) - base[i] for i, w in enumerate(weights)]
    order = sorted(range(len(weights)), key=lambda i: (-rem[i], -weights[i], tie_rank[i]))
    for i in order[: total - sum(base)]:
        base[i] += 1
    return base


class Book:
    def __init__(self, ident, family, units, sp):
        self.side = ident["side"]
        if self.side not in ("long", "short"):
            raise Typed("SchemaUnsupported")
        self.family, self.fixed_q, self.q = family, num(ident["fixed_q"]), num(units)
        self.K, self.batches, self.p_rec = 0, [], 0
        self.Pi = self.A = self.W = 0
        self.fees, self.gross = 0, 0
        self.latch, self.gate, self.events, self.sp = 0, None, [], sp
        self.prev_status, self.prev_positive = None, None

    def P(self):
        return None if not self.batches else sum(self.batches)

    def cost_remaining(self):
        if self.side == "long":
            return self.K
        return "NotEvaluable" if self.P() is None else self.P() - (self.gross - self.fees)

    def status(self):
        if self.P() is None:
            return "NotEvaluable"
        if self.side == "long":
            ok = self.K <= 0 and self.p_rec >= self.P()
        else:
            ok = self.gross - self.fees >= self.P()
        return "Eligible" if ok else "NotEligible"

    def after(self, sp):
        self.sp = sp
        status, cr = self.status(), self.cost_remaining()
        if self.prev_status is not None and status != self.prev_status:
            self.events.append({"kind": "growth_eligible", "from": self.prev_status, "to": status, "stable_point": sp})
        if isinstance(cr, int):
            positive = cr > 0
            if self.prev_positive is not None and positive != self.prev_positive:
                self.events.append({"kind": "cost_remaining_positive" if positive else "cost_remaining_nonpositive",
                                    "stable_point": sp})
            self.prev_positive = positive
        self.prev_status = status
        rank = 0
        if self.side == "long" and self.P() is not None and self.p_rec >= self.P():
            rank = 1
        if status == "Eligible":
            rank = 2 if self.side == "long" else 1
            if self.gate is None:
                self.gate = "SimulatedGateReached"
        self.latch = max(self.latch, rank)

    def view(self):
        st = self.status()
        if self.side == "long":
            unit = {"K": self.K, "q": self.q} if self.q else {"status": "NotComputable", "reason": "quantity_zero", "K": self.K, "q": 0}
        else:
            unit = {"status": "NotComputable", "reason": "short_side_has_no_unit_cost"}
        enabled = "AwaitingPolicy" if st == "Eligible" else st
        return {"q": self.q, "K": self.K if self.side == "long" else None, "P": self.P(), "P_recovered": self.p_rec,
                "Pi": self.Pi, "A": self.A, "W": self.W, "R": self.Pi - self.A - self.W, "fixed_q": self.fixed_q,
                "fees": self.fees, "realized_gross": self.gross, "cost_remaining": self.cost_remaining(),
                "growth_eligible": st, "latch": LATCH[self.latch], "gate": self.gate, "growth_enabled": enabled,
                "unit_cost": unit, "stable_point": self.sp}


def fact_identity(event):
    source = event["source"]
    return tuple(source[k] for k in ("source_namespace", "source_epoch", "external_fact_id"))


def dependency_identity(event):
    dependency = event.get("depends_on")
    if dependency is None:
        return None
    return tuple(dependency[k] for k in ("source_namespace", "source_epoch", "external_fact_id"))


def fill_bindings(event):
    """独立原件规则：fill 费用显式指名原分配，依赖指针不能自行换成别的事实。"""
    references = []
    for component in event.get("known_cost_components", []):
        if component["kind"] != "fee_only" or component.get("cause") != "fill":
            continue
        if num(event.get("deltas", {}).get("units", "0")) != 0:
            raise Typed("SchemaUnsupported")
        reference = component.get("fill_ref")
        dependency = dependency_identity(event)
        if reference is None or dependency is None:
            raise Typed("MissingDependency")
        fields = {"source_namespace", "source_epoch", "external_fact_id", "allocation_id", "family_id", "chong_id"}
        if set(reference) != fields or any(not isinstance(reference[k], str) or not reference[k] for k in fields):
            raise Typed("SchemaUnsupported")
        identity = tuple(reference[k] for k in ("source_namespace", "source_epoch", "external_fact_id"))
        if identity != dependency or reference["family_id"] != event["family_id"] or reference["chong_id"] != event.get("chong_id"):
            raise Typed("IdentityConflict")
        references.append(reference)
    return references


def verify_fill_predecessors(event, records):
    for reference in fill_bindings(event):
        identity = tuple(reference[k] for k in ("source_namespace", "source_epoch", "external_fact_id"))
        predecessor = records.get(identity)
        if predecessor is None:
            raise Typed("AwaitingDependency")
        if ("observed_existing_book" in predecessor or "corporate_action" in predecessor
                or "valuation_observation" in predecessor or num(predecessor["deltas"]["units"]) == 0):
            raise Typed("IdentityConflict")
        if any(reference[k] != predecessor[k] for k in ("allocation_id", "family_id", "chong_id")):
            raise Typed("IdentityConflict")


def apply(book, doc, pointer):
    """返回 (新 book, outcome)；失败抛 Typed 且调用方保留旧 book。"""
    no_float(doc)
    event = doc["events"][int(pointer.rsplit("/", 1)[1])]
    fill_bindings(event)
    sp = event["stable_point"]["seq"]
    num(sp)
    if "valuation_observation" in event:
        raise Typed("SchemaUnsupported")
    comps = event.get("known_cost_components", [])
    opening = "observed_existing_book" in event
    if opening:
        if book is not None:
            raise Typed("IdentityConflict")
        b = Book(event["book_identity"], event["family_id"], event["observed_existing_book"]["units"], sp)
        du = 0
    else:
        if book is None or book.family != event["family_id"]:
            return book, "Unmatched"
        b = json.loads(json.dumps(book.__dict__))
        nb = Book.__new__(Book)
        nb.__dict__.update(b)
        b = nb
        du = num(event["deltas"]["units"])
    kinds = [c["kind"] for c in comps]
    if not opening:
        if du == 0 and "corporate_action" not in event and (not kinds or any(k not in DU0_KINDS for k in kinds)):
            raise Typed("SchemaUnsupported")
        if du != 0 and any(k in DU0_ONLY for k in kinds):
            raise Typed("SchemaUnsupported")
    if b.side == "short" and any(k not in SHORT_KINDS for k in kinds):
        raise Typed("SchemaUnsupported")
    ca = event.get("corporate_action")
    if ca is not None:
        n, d = num(ca["num"]), num(ca["den"])
        if ca["kind"] != "split" or n <= 0 or d <= 0 or du != 0:
            raise Typed("SchemaUnsupported")
        if (b.q * n) % d or (b.fixed_q * n) % d:
            raise Typed("Unsupported")
        b.q, b.fixed_q = b.q * n // d, b.fixed_q * n // d
    b.q += du
    if b.q < 0:
        raise Typed("InvalidDomain")
    for c in comps:
        k, amt = c["kind"], num(c["canonical_amount"])
        if amt < 0:
            raise Typed("InvalidDomain")
        if k == "acquisition_cash":
            b.K += amt
        elif k == "disposal_cash":
            b.K -= amt
        elif k in ("commission", "fee_only"):
            b.K += amt
            b.fees += amt
        elif k == "dividend_cash":
            if num(c["gross_amount"]) - num(c["withheld_amount"]) != amt:
                raise Typed("IdentityConflict")
            b.K -= amt
        elif k == "withholding_tax":
            pass
        elif k == "principal_contribution":
            b.batches.append(amt)
        elif k == "recover_capital":
            if b.P() is None:
                raise Typed("MissingDependency")
            if b.p_rec + amt > b.P():
                raise Typed("InvalidDomain")
            b.p_rec += amt
        else:
            raise Typed("SchemaUnsupported")
    if opening and b.side == "short" and b.P() is None:
        raise Typed("MissingDependency")
    if not opening:
        d = event["deltas"]
        b.Pi += num(d["Pi"])
        b.A += num(d["A"])
        b.W += num(d["W"])
        rp = event.get("declared_realized_profit")
        if rp is not None:
            b.gross += num(rp["gross"])
    b.after(sp)
    return b, "Applied"


def run_e_case(fixtures, case):
    """E 的 Recorded/Resolved 与每个 B 的 Applied 分开记；重复终态只返回原结果。"""
    facts, pending, resolved_evidence, books = {}, {}, {}, {}
    applied, allocations, mismatches, count = {}, {}, [], 0
    for step in case["steps"]:
        count += 1
        doc = json.loads((fixtures / step["raw"]).read_bytes()) if "raw" in step else None
        got = {}
        if step["op"] == "capture":
            ev = doc["events"][int(step["pointer"].rsplit("/", 1)[1])]
            key = fact_identity(ev)
            try:
                fill_bindings(ev)
                target = ev.get("chong_id") or ev["book_identity"]["chong_id"]
                dep = dependency_identity(ev)
                reason = "AwaitingDependency" if dep is not None and dep not in facts else "Unresolved"
                candidate = {"event": ev, "doc": doc, "token": "t-" + key[2],
                             "cursor": num(ev["source"]["source_cursor"]), "dep": dep}
                if key in facts and facts[key] != candidate:
                    raise Typed("IdentityConflict")
                scope = "B:" + target if target in case["routed"] else "account:*"
                if key not in facts:
                    facts[key] = candidate
                    pending[key] = {"token": candidate["token"], "reason": reason, "scope": scope}
                got = {"outcome": "Recorded", "pending_reason": reason, "scope": scope}
            except Typed as error:
                got = {"outcome": error.kind}
        elif step["op"] == "resolve":
            doc = json.loads((fixtures / "raw" / (case["chong_id"] + ".json")).read_bytes())
            items = []
            for covered in step["covered"]:
                ev = doc["events"][int(covered["pointer"].rsplit("/", 1)[1])]
                items.append((fact_identity(ev), covered["token"], covered["fact_hash"]))
            signature = json.dumps([items, step["frontier_cursor"]])
            try:
                if step["evidence_id"] in resolved_evidence:
                    if resolved_evidence[step["evidence_id"]] != signature:
                        raise Typed("IdentityConflict")
                else:
                    keys = {key for key, _, _ in items}
                    for key, token, fact_hash in items:
                        entry = pending.get(key)
                        if entry is None or entry["token"] != (token or facts[key]["token"]):
                            raise Typed("MissingDependency")
                        if fact_hash != "recorded":
                            raise Typed("IdentityConflict")
                        if entry["scope"] == "account:*":
                            raise Typed("MissingDependency")
                        if entry["reason"] == "AwaitingPolicy":
                            raise Typed("AwaitingPolicy")
                        verify_fill_predecessors(facts[key]["event"], {k: f["event"] for k, f in facts.items()})
                        if step["frontier_cursor"] < facts[key]["cursor"]:
                            raise Typed("MissingDependency")
                        if entry["reason"] == "AwaitingDependency":
                            dep = facts[key]["dep"]
                            if dep not in facts or (dep in pending and dep not in keys):
                                raise Typed("AwaitingDependency")
                    for key in keys:
                        del pending[key]
                    resolved_evidence[step["evidence_id"]] = signature
                got = {"outcome": "Resolved"}
            except Typed as error:
                got = {"outcome": error.kind}
            got["pending_after"] = sorted(p["token"] for p in pending.values() if p["scope"] == "B:" + case["chong_id"])
        else:
            ev = doc["events"][int(step["pointer"].rsplit("/", 1)[1])]
            key, chong, allocation = fact_identity(ev), case["chong_id"], ev["allocation_id"]
            local = applied.setdefault(chong, {})
            local_allocations = allocations.setdefault(chong, {})
            try:
                if allocation in local_allocations:
                    old = local_allocations[allocation]
                    if old["key"] != key or old["doc"] != doc:
                        raise Typed("IdentityConflict")
                    outcome = old["outcome"]
                else:
                    if key in local:
                        raise Typed("IdentityConflict")
                    if key not in facts:
                        raise Typed("MissingDependency")
                    if facts[key]["doc"] != doc:
                        raise Typed("IdentityConflict")
                    entry = pending.get(key)
                    if entry is not None and entry["reason"] in ("AwaitingDependency", "AwaitingPolicy"):
                        raise Typed(entry["reason"])
                    fill_bindings(ev)
                    dependency = dependency_identity(ev)
                    if dependency is not None and (dependency not in local or local[dependency]["outcome"] != "Applied"):
                        raise Typed("AwaitingDependency")
                    verify_fill_predecessors(ev, {k: value["event"] for k, value in local.items() if value["outcome"] == "Applied"})
                    books[chong], outcome = apply(books.get(chong), doc, step["pointer"])
                    receipt = {"key": key, "doc": doc, "event": ev, "allocation_id": allocation, "outcome": outcome}
                    local[key] = receipt
                    local_allocations[allocation] = receipt
                got = {"outcome": outcome}
            except Typed as error:
                got = {"outcome": error.kind}
            if "K" in step and books.get(chong) is not None:
                view = books[chong].view()
                got.update({"K": view["K"], "q": view["q"]})
        want = {k: step[k] for k in got if k in step}
        want["outcome"] = step["outcome"]
        if got != want:
            mismatches.append({"case": case["name"], "step": step, "got": got})
    return count, mismatches


def project(view, want):
    return {k: view.get(k) for k in want}


def run(fixtures):
    fixtures = Path(fixtures)
    hand = json.loads((fixtures / "HAND-ORACLE.json").read_text("utf-8"))
    mismatches, results, count = [], [], 0
    for case in hand["book_cases"]:
        book = None
        for step in case["steps"]:
            count += 1
            doc = json.loads((fixtures / step["raw"]).read_bytes())
            try:
                book, outcome = apply(book, doc, step["pointer"])
            except Typed as t:
                outcome = t.kind
            got = {"outcome": outcome}
            if "book" in step:
                got["book"] = project(book.view(), step["book"]) if book is not None else None
            want = {k: step[k] for k in ("outcome", "book") if k in step}
            results.append({"case": case["chong_id"], "label": step["label"], **got})
            if got != want:
                mismatches.append({"case": case["chong_id"], "label": step["label"], "want": want, "got": got})
        if case.get("phase_events") is not None:
            got = [dict(e) for e in book.events] if book is not None else []
            if got != case["phase_events"]:
                mismatches.append({"case": case["chong_id"], "label": "phase_events", "want": case["phase_events"], "got": got})
    for case in hand.get("e_cases", []):
        n, bad = run_e_case(fixtures, case)
        count += n
        mismatches += bad
    for case in hand["apportion_cases"]:
        seq = [apportion(f, case["weights"], case["tie_rank"]) for f in range(case["total"] + 1)]
        if seq != case["sequence"]:
            mismatches.append({"case": case["name"], "want": case["sequence"], "got": seq})
    return {"schema_revision": "tb03c-oracle-output/1", "steps": count, "mismatches": mismatches, "results": results}


def apportion_table():
    """小域穷举：n=2..4 的冻结请求量与两种 tie_break_order，逐累计量给出配额法分配（供生产对拍）。"""
    import itertools
    rows = []
    for n, hi in ((2, 6), (3, 5), (4, 3)):
        for weights in itertools.product(range(1, hi + 1), repeat=n):
            for tie_rank in (list(range(n)), list(range(n))[::-1]):
                rows.append({"weights": list(weights), "tie_rank": tie_rank,
                             "sequence": [apportion(f, list(weights), tie_rank) for f in range(sum(weights) + 1)]})
    return {"schema_revision": "tb03c-apportion-oracle/1", "method": "balinski-young-quota", "rows": rows}


if __name__ == "__main__" and sys.argv[1:] == ["--apportion-table"]:
    sys.stdout.write(json.dumps(apportion_table(), sort_keys=True, separators=(",", ":")) + "\n")
    sys.exit(0)
if __name__ == "__main__":
    out = run(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent / "fixtures" / "tb03c")
    sys.stdout.write(json.dumps(out, ensure_ascii=False, sort_keys=True, indent=1) + "\n")
    sys.exit(0 if not out["mismatches"] else 1)
