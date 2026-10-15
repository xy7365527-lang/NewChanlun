#!/usr/bin/env python3
"""#1424 S2 Q/JS 变异：在临时副本里逐个植入判据错误，跑 test_tb02f_contract（Python 判语料 + 与 JS 逐条对拍）与
test_tb02f_contract.cjs；任何一项转红即算抓到。输出 JSON 收据到 stdout。"""
import json, shutil, subprocess, sys, tempfile, pathlib

SRC = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(__file__).resolve().parents[1]  # 工作树 s_session
PY, JS = "s_tb02_contract.py", "browser/tb01c-client.js"
MUTANTS = [
    ("py_down_expansion_strict_outer", PY, 'b["ZG"] < a["ZD"] and b["GG"] >= a["DD"]', 'b["ZG"] < a["ZD"] and b["GG"] > a["DD"]'),
    ("py_low_touch_as_leave", PY, '"STRICT_BELOW" if hi < zd else', '"STRICT_BELOW" if hi <= zd else'),
    ("py_core_tangent_one_side", PY, '"core_tangent": b["ZD"] == a["ZG"] or b["ZG"] == a["ZD"]', '"core_tangent": b["ZD"] == a["ZG"]'),
    ("py_inputs_unchecked", PY, '_needf(got == want, "PAIR_INPUTS"', '_needf(True, "PAIR_INPUTS"'),
    ("py_pending_null_gate_dropped", PY, '_needf(a["DD"] is None or b["DD"] is None, "PENDING"', '_needf(True, "PENDING"'),
    # #1424 C1：恢复原 pending iff 漏检义务；替换点是中枢门实际承重的 upgrade ⇒ null 方向。
    ("py_pending_iff_dropped", PY, '_need(not upgrade, "ROLE_NULL"', '_need(True, "ROLE_NULL"'),
    ("py_provisional_flipped", PY, 'd["provisional"] is (not b["settled"])', 'd["provisional"] is b["settled"]'),
    ("py_leave_side_swapped", PY, 'side = "DOWN" if pos == "STRICT_BELOW" else "UP"', 'side = "UP" if pos == "STRICT_BELOW" else "DOWN"'),
    ("py_break_role_unbound", PY, 'want_tested[(c["entity_id"], c["break_unit"])] = "BREAK_UNIT"', 'pass'),
    ("js_down_expansion_strict_outer", JS, 'b.ZG < a.ZD && b.GG >= a.DD', 'b.ZG < a.ZD && b.GG > a.DD'),
    ("js_low_touch_as_leave", JS, '(hi < zd ? "STRICT_BELOW"', '(hi <= zd ? "STRICT_BELOW"'),
    ("js_core_tangent_one_side", JS, 'core_tangent: b.ZD === a.ZG || b.ZG === a.ZD', 'core_tangent: b.ZD === a.ZG'),
    ("js_inputs_unchecked", JS, 'needF(PAIR_ENDPOINTS.every(k => got[k] === want[k]) && Object.keys(got).length === 4, "PAIR_INPUTS"', 'needF(true, "PAIR_INPUTS"'),
    ("js_pending_null_gate_dropped", JS, 'needF(a.DD === null || b.DD === null, "PENDING"', 'needF(true, "PENDING"'),
    ("js_pending_iff_dropped", JS, 'needE(!upgrade, "ROLE_NULL"', 'needE(true, "ROLE_NULL"'),
    ("js_provisional_flipped", JS, 'd.provisional === !b.settled', 'd.provisional === b.settled'),
    ("js_leave_side_swapped", JS, 'const side = pos === "STRICT_BELOW" ? "DOWN" : pos === "STRICT_ABOVE" ? "UP" : null', 'const side = pos === "STRICT_BELOW" ? "UP" : pos === "STRICT_ABOVE" ? "DOWN" : null'),
    ("js_connectors_off_by_one", JS, 'pos.get(a.members[a.members.length - 1]) + 1, pos.get(b.members[0])', 'pos.get(a.members[a.members.length - 1]) + 2, pos.get(b.members[0])'),
    ("py_relation_witness_unbound", PY, 'set(got_edges) == want_edges', 'set(e[:4] for e in got_edges) == set(e[:4] for e in want_edges)'),
    ("js_relation_witness_unbound", JS, 'gotEdges.every(e => wantEdges.has(e))', 'gotEdges.every(e => [...wantEdges].some(w => JSON.stringify(JSON.parse(e).slice(0, 4)) === JSON.stringify(JSON.parse(w).slice(0, 4))))'),
    ("py_event_chain_unchecked", PY, '        _validate_pair_change(d, p)', '        pass'),
    ("js_event_chain_unchecked", JS, '      validatePairChange(d, p);', '      /* removed change check */'),
    ("py_history_before_unbound", PY, '_needf(d["before"] == (None if old is None else old["payload"]["data"]) and d["after"] == (None if new is None else new["payload"]["data"]), "CHANGE"', '_needf(True, "CHANGE"'),
    ("js_history_before_unbound", JS, 'needF(equal(d.before, old?.payload.data ?? null) && equal(d.after, current?.payload.data ?? null), "CHANGE"', 'needF(true, "CHANGE"'),
    ("py_witness_bool_untyped", PY, '_needf(type(w["core_tangent"]) is bool and type(w["outer_touch"]) is bool, "WITNESS"', '_needf(True, "WITNESS"'),
    ("js_witness_bool_untyped", JS, 'needF(typeof d.witnesses.core_tangent === "boolean" && typeof d.witnesses.outer_touch === "boolean", "WITNESS"', 'needF(true, "WITNESS"'),
    ("py_publication_generation_unbound", PY, '_needf(record["payload"]["data"]["known_at"]["generation"] == generation,', '_needf(True,'),
    ("js_publication_generation_unbound", JS, 'needF(record.payload.data.known_at.generation === generation,', 'needF(true,'),
    ("js_history_rescans_stable_entities", JS, '[...group.births, ...group.deaths, ...group.events].map', '[...activeByEntity.values(), ...group.births, ...group.deaths, ...group.events].map'),
]


def run(tmp):
    py = subprocess.run([sys.executable, "-m", "unittest", "tests.test_tb02f_contract"], cwd=tmp, capture_output=True, text=True, timeout=300)
    js = subprocess.run(["node", "--test", "tests/test_tb02f_contract.cjs"], cwd=tmp, capture_output=True, text=True, timeout=300)
    return py.returncode, js.returncode, py.stdout + py.stderr, js.stdout + js.stderr


out = []
with tempfile.TemporaryDirectory() as d:
    base = pathlib.Path(d) / "s_session"
    shutil.copytree(SRC, base, ignore=shutil.ignore_patterns("__pycache__", "node_modules"))
    clean = run(base)
    assert clean[:2] == (0, 0), ("干净副本须全绿", clean)
    for name, rel, old, new in MUTANTS:
        path = base / rel
        text = path.read_text(encoding="utf-8")
        assert text.count(old) == 1, (name, text.count(old))
        path.write_text(text.replace(old, new), encoding="utf-8")
        py, js, py_output, js_output = run(base)
        out.append({"mutant": name, "file": rel, "python_suite_rc": py, "node_suite_rc": js, "caught": py != 0 or js != 0, "python_output": py_output, "node_output": js_output})
        path.write_text(text, encoding="utf-8")
    assert run(base)[:2] == (0, 0), "还原后须全绿"
json.dump({"clean": "green", "mutants": out, "caught": sum(m["caught"] for m in out), "total": len(out)}, sys.stdout, ensure_ascii=False, indent=1)
print()
if not all(m["caught"] for m in out):
    raise SystemExit(1)
