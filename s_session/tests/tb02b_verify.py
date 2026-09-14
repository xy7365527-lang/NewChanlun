#!/usr/bin/env python3
"""#1392：只读真实 S 库→公共 Q 投影→冻结手算账簿；本项不冒充 launcher/GUI。"""
import argparse
from contextlib import closing
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as reader


def check_snapshot(snapshot, expected):
    objects = snapshot["objects"]
    pairs = [o["payload"]["data"] for o in objects if o["kind"] == "CC-008.new_bi_pair"]
    bis = [o["payload"]["data"] for o in objects if o["kind"] == "CC-010.bi"]
    if "pair" in expected:
        a, b = map(str, expected["pair"])
        found = [p for p in pairs if p["old"]["group_anchor"] == a and p["new"]["group_anchor"] == b]
        assert len(found) == 1, "公共事实缺唯一具名端点对"
        conditions = found[0]["conditions"]
        assert conditions["vector"] == expected["vector"]
        assert conditions["merged_gap"] == str(expected["merged_gap"])
        assert conditions["raw_between_actual_extrema"] == str(expected["raw_between"])
    if "strokes" in expected:
        actual = [[int(b["start"]["extreme_roots"][0]), int(b["end"]["extreme_roots"][0])] for b in bis]
        assert actual == expected["strokes"], (actual, expected["strokes"])
    if "ambiguous_root_anchor" in expected:
        endpoints = [o["payload"]["data"] for o in objects if o["kind"] == "CC-008.endpoint"]
        endpoint = next(e for e in endpoints if e["group_anchor"] == str(expected["ambiguous_root_anchor"]))
        assert endpoint["extreme_roots"] == list(map(str, expected["ambiguous_roots"]))
        assert endpoint["raw_position"] is None and "raw_extreme_identity_tie" in endpoint["waiting_reasons"]
    if "confirmed" in expected:
        assert [b["state"] == "CONFIRMED" for b in bis] == expected["confirmed"]
    if "same_pair" in expected:
        same = [o["payload"]["data"] for o in objects if o["kind"] == "CC-009.same_kind"]
        a, b = map(str, expected["same_pair"])
        found = [p for p in same if p["old"]["group_anchor"] == a and p["new"]["group_anchor"] == b]
        assert len(found) == 1 and found[0]["selection"] == expected["selection"]
        assert found[0]["conditions"] is None
        if "retained_anchors" in expected:
            assert found[0]["retained_anchors"] == list(map(str, expected["retained_anchors"]))


def verify(database, case=None, node=None):
    audit = reader._query_integrity()
    with closing(reader.open_readonly(database)) as conn, conn:
        conn.execute("BEGIN")
        proof = audit.verify(audit.capture(conn), vars(reader))
    states = [audit.project_state(proof, n, vars(reader)) for n in range(1, proof["generation"] + 1)]
    if case:
        expected = json.loads((ROOT / "tests/fixtures/tb02b/hand-oracle.json").read_text())["cases"][case]
        check_snapshot(states[-1]["snapshot"], expected)
    if node:
        script = """const fs=require('node:fs'); const API=require(process.argv[1]);
const states=JSON.parse(fs.readFileSync(0,'utf8'));
for(const s of states){API.validateTB02Catalog(s.catalog);API.validateTB02CutSources(s.snapshot);API.validateTB02Axes(s.snapshot.catalog_evidence.axes,s.snapshot.objects);}
console.log(JSON.stringify({validated_cuts:states.length}));"""
        child = subprocess.run([str(node), "-e", script, str(ROOT / "browser/tb01c-client.js")],
                               input=json.dumps(states), text=True, capture_output=True, timeout=30)
        if child.returncode:
            raise ValueError("真实公共投影的前端合同校验失败：" + child.stderr)
    return {"status": "passed", "cuts": len(states), "case": case,
            "scope": "真实库的Q/前端合同定点验证；非launcher/HTTP/GUI验收"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--database", type=Path, required=True)
    parser.add_argument("--case")
    parser.add_argument("--node", type=Path)
    print(json.dumps(verify(**vars(parser.parse_args())), ensure_ascii=False))


if __name__ == "__main__":
    main()
