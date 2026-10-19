#!/usr/bin/env python3
"""#1394：从冻结现金原件独立核账，不导入任何生产 reducer/查询投影。"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sqlite3


def canonical(value):
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode()


def sha(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def amount(value):
    if not isinstance(value, str) or not re.fullmatch(r"0|[1-9][0-9]*", value):
        raise ValueError("oracle 数值不是规范非负整数文本")
    result = int(value)
    if result > 2**63 - 1:
        raise ValueError("oracle 数值溢出")
    return result


def read_domain(path):
    """只读原 DB 的历史、原消息和回执；不运行产品审计或借它生成 expected。"""
    with sqlite3.connect(Path(path).resolve().as_uri() + "?mode=ro", uri=True) as db:
        db.execute("PRAGMA query_only=ON")
        db.execute("BEGIN")
        meta = dict(db.execute("SELECT k,v FROM meta"))
        commits = [{"seq": seq, "root": root, "commit_ns": ns, "image": json.loads(image)}
                   for seq, root, ns, image in db.execute("SELECT seq,root,commit_ns,image FROM commits ORDER BY seq")]
        receipts = {key: json.loads(raw) for key, raw in db.execute("SELECT k,json FROM receipts")}
        requests = {key: json.loads(raw) for key, raw in db.execute("SELECT k,request_json FROM messages")}
        db.rollback()
    return {"meta": meta, "commits": commits, "receipts": receipts, "requests": requests}


def check(manifest_path, expect="committed"):
    manifest = json.loads(Path(manifest_path).read_text())
    data_raw = Path(manifest["finance"]["input_path"]).read_bytes()
    assert hashlib.sha256(data_raw).hexdigest() == manifest["finance"]["input_sha256"]
    data = json.loads(data_raw)
    assert data["kind"] == "TestOnly" and data["policy"]["model"] == "TestOnlyUnleveragedCashBuy"
    # ADR 0029 裁定一：具名政策 id。
    assert data["policy"]["allocation_policy_id"] == "#1349/atomic-slot-wait/v1"
    rights = {r["right_id"]: r for r in data["rights"]}
    baseline, cap = amount(data["baseline"]["amount"]), amount(data["policy"]["cap"])
    expected = {}
    for plan in data["plans"]:
        quantity, price, tail = (amount(plan[k]) for k in ("quantity", "limit_price", "fee_tail"))
        bound = quantity * price + tail
        assert bound <= amount(rights[plan["right_id"]]["amount"])
        expected[plan["decision_id"]] = {"bound": str(bound), "quantity": str(quantity), "owner": plan["owner"], "plan_hash": sha(plan)}
    required = baseline + sum(int(e["bound"]) for e in expected.values())
    domains = {"E": read_domain(manifest["e"]["db"])}
    domains.update({"B:" + b["chong_id"]: read_domain(b["db"]) for b in manifest["chongs"]})
    e_image = domains["E"]["commits"][-1]["image"]["finance"]
    actual_liability = sum(amount(r["amount"]) for r in e_image["responsibilities"])
    slot = e_image["slots"][0]
    if expect == "awaiting_policy":
        assert required > cap
        assert slot["state"] == "AwaitingAllocationPolicy"
        assert slot["waiting_for"] == data["policy"]["allocation_policy_id"]
        assert slot["grants"] == [] and actual_liability == baseline
    else:
        assert required <= cap and actual_liability == required
        assert slot["state"] == "Reserved" and len(slot["grants"]) == len(expected)
    observations = {}
    for decision_id, exp in expected.items():
        domain = domains[exp["owner"]]
        history = domain["commits"]
        state = history[-1]["image"]
        initial = next(c["image"]["book"] for c in history if c["image"]["book"] is not None)
        book = state["book"]
        # 实际 q、固定 Q、成本和所有原账分量不因财务状态改变。
        assert book["actual_quantity"] == initial["actual_quantity"]
        assert book["fixed_q"] == initial["fixed_q"]
        assert book["known_cost_totals"] == initial["known_cost_totals"]
        assert book["income_book"] == initial["income_book"]
        assert book["unknown"]["principal_baseline"]["status"] == "Unknown"
        assert book["unknown"]["capital_phase"]["status"] == "Unknown"
        d = next(d for d in state["finance"]["decisions"] if d["decision_id"] == decision_id)
        assert d["prepared"]["locked_amount"] == exp["bound"]
        worst = amount(initial["actual_quantity"]) + amount(exp["quantity"])
        assert d["prepared"]["worst_quantity"] == str(worst) and worst <= amount(book["fixed_q"])
        assert d["execution_eligibility"] == "None"
        should_commit = expect == "committed" or expect == "unavailable_alpha" and exp["owner"] != "B:alpha"
        assert d["state"] == ("Committed" if should_commit else "Prepared")
        if should_commit:
            receipt = next(r for r in domain["receipts"].values() if r["kind"] == "LocalCommitReceipt" and r["record"]["decision_id"] == decision_id)
            grant = next(g for g in slot["grants"] if g["decision_id"] == decision_id)
            assert receipt["record"]["grant"]["original"] == grant
            assert grant["amount"] == exp["bound"] and grant["plan_hash"] == exp["plan_hash"]
            assert amount(grant["reserved_ns"]) <= amount(receipt["commit_ns"])
            assert receipt["record"]["local_commit_receipt"]["e_cut"]["commit_seq"] == grant["reserved_seq"]
            for c in history:
                for old in c["image"]["finance"]["decisions"]:
                    if old["state"] == "Prepared":
                        assert old["grant"] is None and old["local_commit_receipt"] is None
        observations[exp["owner"]] = {"q": book["actual_quantity"], "Q": book["fixed_q"], "lock": exp["bound"], "worst_q": str(worst), "state": d["state"]}
    semantic = {name: {"commits": value["commits"], "receipts": value["receipts"], "requests": value["requests"]} for name, value in domains.items()}
    return {"scope": expect, "input_sha256": hashlib.sha256(data_raw).hexdigest(), "required_total": str(required),
            "retained_total": str(actual_liability), "observations": observations, "domain_semantic_hashes": {name: sha(v) for name, v in semantic.items()},
            "semantic": semantic}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--expect", choices=("committed", "awaiting_policy", "unavailable_alpha", "reserved"), default="committed")
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    result = check(args.manifest, args.expect)
    with args.output.open("x") as stream:
        json.dump(result, stream, ensure_ascii=False, indent=2)
    print(json.dumps({k: v for k, v in result.items() if k != "semantic"}, ensure_ascii=False))


if __name__ == "__main__":
    main()
