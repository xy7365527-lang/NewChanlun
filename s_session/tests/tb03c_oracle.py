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


def apply(book, doc, pointer):
    """返回 (新 book, outcome)；失败抛 Typed 且调用方保留旧 book。"""
    no_float(doc)
    event = doc["events"][int(pointer.rsplit("/", 1)[1])]
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
    for case in hand["apportion_cases"]:
        seq = [apportion(f, case["weights"], case["tie_rank"]) for f in range(case["total"] + 1)]
        if seq != case["sequence"]:
            mismatches.append({"case": case["name"], "want": case["sequence"], "got": seq})
    return {"schema_revision": "tb03c-oracle-output/1", "steps": count, "mismatches": mismatches, "results": results}


if __name__ == "__main__":
    out = run(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent / "fixtures" / "tb03c")
    sys.stdout.write(json.dumps(out, ensure_ascii=False, sort_keys=True, indent=1) + "\n")
    sys.exit(0 if not out["mismatches"] else 1)
