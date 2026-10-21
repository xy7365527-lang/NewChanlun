#!/usr/bin/env python3
"""#1406 TB-03-C：生成 tb03c-book/1 的 TestOnly 经济原件夹具（原件逐字冻结在 raw/）。

格式沿 #1374 的中性 TestOnly 原件（来源五元、evidence_index 逐字段指针、分量见证），
只增 ADR 0031/0036 与根裁定 OQ-5/OQ-6 要的字段：book_identity.side、stable_point、
新成本分量（disposal_cash / fee_only / dividend_cash / withholding_tax /
principal_contribution / recover_capital）、corporate_action（拆股）、valuation_observation。
手算期望不在此生成，见 HAND-ORACLE.json。

用法：python3 generate_fixtures.py          重写 raw/ 与 MANIFEST.json
      python3 generate_fixtures.py --check  只核对磁盘字节与重新生成一致
"""
import hashlib
import json
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
FORMAT = "neutral TestOnly economic raw originals; not production transport wire"
UNITS = {"quantity": {"unit": "TEST-UNIT", "quantum": "1"},
         "money": {"unit": "TEST-CASH", "currency": "TEST-CASH", "quantum": "1"},
         "numeric_encoding": "exact_decimal_integer_text"}
NAMESPACE, EPOCH = "testonly-tb1406", "fixture-epoch-1"


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()


class Doc:
    """一份 TestOnly 原件；事件按追加顺序位于 /events/<i>。"""

    def __init__(self, chong, family, symbol, level, fixed_q, side):
        self.chong, self.family, self.symbol, self.level, self.fixed_q, self.side = chong, family, symbol, level, fixed_q, side
        self.events, self.fragments = [], {}

    def _ref(self, name, pointer):
        assert name not in self.fragments, name
        self.fragments[name] = [{"json_pointer": pointer}]
        return name

    def event(self, ext, cursor, stable_point, *, opening_units=None, du=None, pi=0, a=0, w=0,
              comps=(), realized=None, corporate_action=None, valuation=None, depends_on=None):
        i = len(self.events)
        p = f"/events/{i}"
        e = {"event": "tb03c-" + ext, "allocation_id": "alloc-" + ext, "family_id": self.family,
             "source": {"source_namespace": NAMESPACE, "source_epoch": EPOCH, "source_cursor": str(cursor),
                        "external_fact_id": ext, "received_time": f"source-received-{cursor}",
                        "claimed_origin": "TestOnly fixture tb03c", "source_event_time": f"source-event-{cursor}"},
             "stable_point": {"profile": "TestOnly", "seq": str(stable_point),
                              "source_ref": self._ref(f"{ext}:sp", f"{p}/stable_point")}}
        e["known_cost_components"] = []
        for j, c in enumerate(comps):
            kind, amount, extra = c[0], c[1], (c[2] if len(c) > 2 else {})
            text = amount if isinstance(amount, str) else (str(amount) if isinstance(amount, int) else amount)
            comp = {"kind": kind, "amount_text": text if isinstance(text, str) else str(text),
                    "canonical_amount": text, "currency": "TEST-CASH", "quantum": "1",
                    "source_ref": self._ref(f"{ext}:c{j}", f"{p}/known_cost_components/{j}")}
            comp.update(extra)
            e["known_cost_components"].append(comp)
        if opening_units is not None:
            e["book_identity"] = {"chong_id": self.chong, "symbol": self.symbol, "op_level": str(self.level),
                                  "fixed_q": str(self.fixed_q), "side": self.side}
            e["observed_existing_book"] = {"units": str(opening_units), "Pi": "0", "A": "0", "W": "0",
                                           "witness_refs": {k: self._ref(f"{ext}:o{k}", f"{p}/observed_existing_book/{k}")
                                                            for k in ("units", "Pi", "A", "W")}}
        else:
            e["chong_id"] = self.chong
            e["deltas"] = {"units": str(du), "Pi": str(pi), "A": str(a), "W": str(w)}
            ws = []
            if du != 0:
                ws.append({"kind": "settled_quantity_increase" if du > 0 else "settled_quantity_reduction", "value": str(du)})
            if realized is not None:
                gross, fees = realized
                ws.append({"kind": "settled_realized_gross_profit", "value": str(gross)})
                ws.append({"kind": "all_fees_for_this_named_realization", "value": str(fees)})
                e["declared_realized_profit"] = {"gross": str(gross), "listed_corresponding_fees": str(fees),
                                                 "net": str(gross - fees), "fee_scope_complete_for_this_realization": True,
                                                 "source_ref": self._ref(f"{ext}:rp", f"{p}/declared_realized_profit")}
            if a:
                ws.append({"kind": "completed_profit_capitalization", "value": str(a)})
            if w:
                ws.append({"kind": "completed_profit_withdrawal", "value": str(w)})
            if pi == 0 and a == 0 and w == 0:
                ws.append({"kind": "no_other_income_book_changes_in_this_allocation", "fields": ["Pi", "A", "W"]})
            for k, wit in enumerate(ws):
                wit["source_ref"] = self._ref(f"{ext}:w{k}", f"{p}/component_witnesses/{k}")
            e["component_witnesses"] = ws
        if corporate_action is not None:
            kind, num, den = corporate_action
            e["corporate_action"] = {"kind": kind, "num": str(num), "den": str(den),
                                     "source_ref": self._ref(f"{ext}:ca", f"{p}/corporate_action")}
        if depends_on is not None:
            e["depends_on"] = {"source_namespace": NAMESPACE, "source_epoch": EPOCH, "external_fact_id": depends_on,
                               "source_ref": self._ref(f"{ext}:dep", f"{p}/depends_on")}
        if valuation is not None:
            before, after = valuation
            e["valuation_observation"] = {"reference_price_before": str(before), "reference_price_after": str(after),
                                          "source_ref": self._ref(f"{ext}:val", f"{p}/valuation_observation")}
        self.events.append(e)
        return p

    def document(self):
        return {"format": FORMAT, "units": UNITS, "events": self.events, "evidence_index": {"fragments": self.fragments}}


def P(amount, batch):
    return ("principal_contribution", amount, {"batch_id": batch, "authorization_ref": "testonly-authorization/1"})


def fee(amount, fee_type, cause):
    return ("fee_only", amount, {"fee_type": fee_type, "cause": cause})


def documents():
    docs = {}
    # WE-1 / WE-2：回本前同量回补、跨闸、退本金、模拟触闸、费用回正（多头）
    d = Doc("chong-L1", "fam-L1", "TEST-L1", 1, 10, "long")
    d.event("L1-open", 1, 100, opening_units=10, comps=[("acquisition_cash", 9950), ("commission", 50), P(10000, "b-L1-0")])
    d.event("L1-s1", 2, 101, du=-4, pi=776, comps=[("disposal_cash", 4800), ("commission", 24)], realized=(800, 24))
    d.event("L1-b1", 3, 102, du=4, comps=[("acquisition_cash", 4200), ("commission", 21)])
    d.event("L1-s2", 4, 103, du=-4, pi=11920, comps=[("disposal_cash", 16000), ("commission", 80)], realized=(12000, 80))
    d.event("L1-b2", 5, 104, du=4, comps=[("acquisition_cash", 6000), ("commission", 30)])
    d.event("L1-rc", 6, 105, du=0, comps=[("recover_capital", 10000)])
    d.event("L1-fee", 7, 106, du=0, comps=[fee(600, "platform", "period")])
    d.event("L1-zero", 8, 106, du=0, comps=[])
    d.event("L1-zero-fill", 9, 106, du=0, comps=[("acquisition_cash", 10)])
    d.event("L1-fee-fill", 10, 107, du=1, comps=[("acquisition_cash", 100), fee(1, "exchange", "fill")])
    docs["raw/chong-L1.json"] = d
    # WE-2b：回本后无费用的亏损往返（OQ-2 反例）
    d = Doc("chong-E1", "fam-E1", "TEST-E1", 1, 10, "long")
    d.event("E1-open", 1, 200, opening_units=10, comps=[("acquisition_cash", 1000), P(1000, "b-E1-0")])
    d.event("E1-s1", 2, 201, du=-5, pi=1000, comps=[("disposal_cash", 1500)], realized=(1000, 0))
    d.event("E1-rc", 3, 202, du=0, comps=[("recover_capital", 1000)])
    d.event("E1-s2", 4, 203, du=-1, comps=[("disposal_cash", 100)])
    d.event("E1-b2", 5, 204, du=1, comps=[("acquisition_cash", 700)])
    docs["raw/chong-E1.json"] = d
    # WE-4：空头最小书账（OQ-5）
    d = Doc("chong-S1", "fam-S1", "TEST-S1", 1, 5, "short")
    d.event("S1-open", 1, 300, opening_units=5, comps=[("commission", 10), P(2000, "m-S1-0")])
    d.event("S1-borrow", 2, 301, du=0, comps=[fee(15, "borrow", "period")])
    d.event("S1-c1", 3, 302, du=-2, pi=194, comps=[("commission", 6)], realized=(200, 6))
    d.event("S1-r1", 4, 303, du=2, comps=[("commission", 7)])
    d.event("S1-div", 5, 304, du=0, comps=[fee(20, "dividend_paid", "period")])
    d.event("S1-acq", 6, 305, du=1, comps=[("acquisition_cash", 100)])
    d.event("S1-rc", 7, 305, du=0, comps=[("recover_capital", 10)])
    docs["raw/chong-S1.json"] = d
    d = Doc("chong-S2", "fam-S2", "TEST-S2", 1, 3, "short")
    d.event("S2-open", 1, 310, opening_units=3, comps=[("commission", 1)])
    docs["raw/chong-S2.json"] = d
    # WE-6：未登记本金时显式 NotEvaluable；开局前增量 Unmatched
    d = Doc("chong-W1", "fam-W1", "TEST-W1", 1, 3, "long")
    d.event("W1-open", 1, 500, opening_units=3, comps=[("acquisition_cash", 300)])
    d.event("W1-rc", 2, 501, du=0, comps=[("recover_capital", 100)])
    docs["raw/chong-W1.json"] = d
    d = Doc("chong-W2", "fam-W2", "TEST-W2", 1, 2, "long")
    d.event("W2-early", 1, 501, du=1, comps=[("acquisition_cash", 500), ("commission", 1)])
    d.event("W2-open", 2, 502, opening_units=2, comps=[("acquisition_cash", 1000), ("commission", 2)])
    docs["raw/chong-W2.json"] = d
    # WE-8：公司行动与估值
    d = Doc("chong-D1", "fam-D1", "TEST-D1", 1, 10, "long")
    d.event("D1-open", 1, 600, opening_units=10, comps=[("acquisition_cash", 1000), P(1000, "b-D1-0")])
    d.event("D1-div", 2, 601, du=0, comps=[("dividend_cash", 45, {"gross_amount": "50", "withheld_amount": "5"}),
                                         ("withholding_tax", 5, {"netted_in_dividend": True})])
    d.event("D1-split", 3, 602, du=0, corporate_action=("split", 2, 1))
    d.event("D1-rsplit", 4, 603, du=0, corporate_action=("split", 1, 2))
    d.event("D1-bad-split", 5, 604, du=0, corporate_action=("split", 2, 3))
    d.event("D1-val", 6, 605, du=0, valuation=(100, 60))
    d.event("D1-div-bad", 7, 606, du=0, comps=[("dividend_cash", 44, {"gross_amount": "50", "withheld_amount": "5"})])
    docs["raw/chong-D1.json"] = d
    d = Doc("chong-D1", "fam-D1", "TEST-D1", 1, 10, "long")
    d.event("D1-float", 1, 607, du=1, comps=[("acquisition_cash", 100.5)])
    docs["raw/chong-D1-float.json"] = d
    # WE-5：Pending→Resolve 与 fee-before-fill（E 侧 AwaitingDependency）
    d = Doc("chong-R1", "fam-R1", "TEST-R1", 1, 3, "long")
    d.event("R-open", 1, 400, opening_units=2, comps=[("acquisition_cash", 200)])
    d.event("R-buy", 2, 401, du=1, comps=[("acquisition_cash", 110), ("commission", 1)])
    d.event("R-sell", 3, 402, du=-1, pi=19, comps=[("disposal_cash", 130), ("commission", 1)], realized=(20, 1))
    d.event("R-fee", 4, 403, du=0, comps=[("fee_only", 3, {"fee_type": "exchange", "cause": "fill",
            "fill_ref": {"source_namespace": NAMESPACE, "source_epoch": EPOCH,
                         "external_fact_id": "R-late", "allocation_id": "alloc-R-late",
                         "family_id": "fam-R1", "chong_id": "chong-R1"}})], depends_on="R-late")
    d.event("R-late", 5, 404, du=1, comps=[("acquisition_cash", 120)])
    docs["raw/chong-R1.json"] = d
    # C06.4：目标重不在 manifest 的原件，E 照记并封账户范围
    d = Doc("chong-U1", "fam-U1", "TEST-U1", 1, 1, "long")
    d.event("U1-open", 6, 410, opening_units=1, comps=[("acquisition_cash", 10)])
    docs["raw/chong-U1.json"] = d
    return {name: canonical(doc.document()) for name, doc in docs.items()}


def manifest(files):
    return canonical({"schema_revision": "tb03c-fixtures/1", "book_revision": "tb03c-book/1",
                      "files": {name: hashlib.sha256((HERE / name).read_bytes()).hexdigest() for name in sorted(files)}})


def main(argv):
    check = argv[1:] == ["--check"]
    raws = documents()
    listed = sorted(list(raws) + ["APPORTION-ORACLE.json", "HAND-ORACLE.json", "generate_fixtures.py"])
    bad = []
    for name, raw in raws.items():
        path = HERE / name
        if check:
            if not path.exists() or path.read_bytes() != raw:
                bad.append(name)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(raw)
    want = manifest(listed)
    if check:
        if (HERE / "MANIFEST.json").read_bytes() != want:
            bad.append("MANIFEST.json")
        print("ok" if not bad else "drift: " + ", ".join(bad))
        return 1 if bad else 0
    (HERE / "MANIFEST.json").write_bytes(want)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
