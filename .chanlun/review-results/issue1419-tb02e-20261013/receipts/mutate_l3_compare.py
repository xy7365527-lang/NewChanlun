#!/usr/bin/env python3
"""#1419 L3 复跑：对拍比较器自身的变异检查（全量 cut），证明 S~塔、S~oracle、Q 三道比较不是空过。"""
import copy, importlib.util, json
from pathlib import Path
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("a", HERE / "analyze_l3.py"); a = importlib.util.module_from_spec(spec); spec.loader.exec_module(a)
cut = json.loads((HERE / "l3-dump.json").read_text())["cuts"][-1]
us = a.s_units(cut["segment_records"]); ut = cut["tower_units"]
ou = [dict(u, unit_id=s["unit_id"]) for u, s in zip(ut, us)]
oc, oa, ocur, us_t = a.oracle_scan(ut); a.oracle_us_cache["us"] = us_t
assert not a.compare_s_tower(cut["s_records"], ou, cut["tower_l0_centers"])["diffs"]
out = {}
tc = copy.deepcopy(cut["tower_l0_centers"]); tc[0]["zg"] += 1
out["M1_tower_zg_plus1_S~T_diffs"] = len(a.compare_s_tower(cut["s_records"], ou, tc)["diffs"])
recs = copy.deepcopy(cut["s_records"])
rr = next(x for x in recs if x["kind"] == "CC-017.center_roles")
rr["payload"]["data"]["roles"]["OUTER"]["DD"]["value"] = str(int(rr["payload"]["data"]["roles"]["OUTER"]["DD"]["value"]) - 1)
out["M2_S_DD_minus1_S~T_diffs"] = len(a.compare_s_tower(recs, ou, cut["tower_l0_centers"])["diffs"])
out["M2_S_DD_minus1_S~O_diffs"] = len(a.compare_s_oracle(recs, ou, oc, oa, ocur)["diffs"])
out["M3_drop_tower_center0_S~T_diffs"] = len(a.compare_s_tower(cut["s_records"], ou, cut["tower_l0_centers"][1:])["diffs"])
recs = copy.deepcopy(cut["s_records"])
k = next(i for i, s in enumerate(us) if (s["geo_lo"], s["geo_hi"]) != (s["lo"], s["hi"]))
for x in recs:
    d = x["payload"].get("data", {})
    for u in (d.get("units") or []) + ([d["unit"]] if isinstance(d.get("unit"), dict) else []):
        if u.get("unit_id") == us[k]["unit_id"]:
            u["lo"], u["hi"] = str(us[k]["geo_lo"]), str(us[k]["geo_hi"])
try:
    a.contract.validate_center_cut(recs, a.contract.CENTER_RULE, cut["axes"]); out["M4_geometric_unit_in_S_objects_Q"] = "ACCEPT"
except ValueError as e:
    out["M4_geometric_unit_in_S_objects_Q"] = str(e).split(":")[0]
print(json.dumps(out, ensure_ascii=False, indent=1))
ok = all(v >= 1 for kk, v in out.items() if kk.startswith(("M1", "M2", "M3"))) and out["M4_geometric_unit_in_S_objects_Q"] != "ACCEPT"
print("ALL_MUTANTS_CAUGHT", ok)
