#!/usr/bin/env python3
"""离线用 S CLI 为 tb02e 账簿某 case 建库（不起服务），供 tb02e_verify 离线自测。"""
import json, subprocess, sys
from pathlib import Path
WT = Path("/private/tmp/goal1323-native-20261012/tb1419-wip-wt/s_session")
sys.path.insert(0, str(WT))
from s_service_control import canonical
BIN = "/private/tmp/goal1323-native-20261012/tb1419-impl/artifacts/s_structure_session-48e2386463"
case, out = sys.argv[1], Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=False)
rows = json.loads((WT / "tests/fixtures/tb02e/raw-ledger.json").read_text())["cases"][case]
db, sid = out / "s.sqlite", "tb02e-" + case
run = lambda *a: subprocess.run(["nice", "-n", "10", BIN, *map(str, a)], capture_output=True, text=True)
p = run("init", "--db", db, "--session", sid, "--catalog", WT / "catalog/signed-catalog.json"); assert p.returncode == 0, p.stdout + p.stderr
for i, r in enumerate(rows):
    ev = {"event_id": r["raw_id"], "revision": "1", "seq": str(i), "received_at": "2000-01-01T00:00:00Z", "raw_text": canonical(r).decode(),
          "timestamp": str(i * 60), "volume": "1", **{k: str(r[k]) for k in ("open", "high", "low", "close")}}
    f = out / "in.json"
    f.write_text(json.dumps({"schema_revision": "s-ohlc/1", "session_id": sid, "source_namespace": "testonly.ohlc." + sid, "source_epoch": "1",
                             "instrument": "TEST-OHLC", "profile": "ohlc_integer_tb02a_v1", "events": [ev]}))
    a = run("accept", "--db", db, "--input", f, "--profile", WT / "profiles/ohlc_integer_tb02a_v1.json"); assert a.returncode == 0, a.stdout + a.stderr
    v = run("advance", "--db", db); assert v.returncode == 0, v.stdout + v.stderr
print(json.dumps({"case": case, "inputs": len(rows), "db": str(db)}))
