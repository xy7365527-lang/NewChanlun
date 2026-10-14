#!/usr/bin/env python3
"""#1419 MIN12 最小流的 parser 实测（离线 S CLI：init → 逐根 accept/advance → 第 50/51/52 条后 snapshot）。不起服务、不计入帽。"""
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "min12" / "parser-run"
BIN = Path("/private/tmp/goal1323-native-20261012/tb1419-impl/artifacts/s_structure_session-48e2386463")
SS = Path("/Volumes/AgentStorage/issue1419-s0b-20261014/copy/s_session")
sys.path.insert(0, str(SS))
from s_service_control import canonical  # noqa: E402

design = json.loads((HERE / "min12/s0b-MIN12.json").read_text())
OUT.mkdir(parents=True, exist_ok=False)
db = OUT / "s.sqlite"
sid = "tb1419-min12-parser"
ns = "testonly.ohlc." + sid


def call(*args):
    t = time.monotonic()
    p = subprocess.run([str(BIN), *map(str, args)], capture_output=True, text=True)
    return p, (time.monotonic() - t) * 1000


p, _ = call("init", "--db", db, "--session", sid, "--catalog", SS / "catalog/signed-catalog.json")
assert p.returncode == 0, p.stdout + p.stderr
log = []
snaps = {}
for i, r in enumerate(design["rows"]):
    ev = {"event_id": r["raw_id"], "revision": "1", "seq": str(r["raw_index"]), "received_at": "2000-01-01T00:00:00Z",
          "raw_text": canonical(r).decode(), "timestamp": str(r["raw_index"] * 60), "volume": "1",
          **{k: str(r[k]) for k in ("open", "high", "low", "close")}}
    raw = {"schema_revision": "s-ohlc/1", "session_id": sid, "source_namespace": ns, "source_epoch": "1",
           "instrument": "TEST-OHLC", "profile": "ohlc_integer_tb02a_v1", "events": [ev]}
    f = OUT / f"in-{i:03}.json"
    f.write_text(json.dumps(raw, ensure_ascii=False))
    pa, ta = call("accept", "--db", db, "--input", f, "--profile", SS / "profiles/ohlc_integer_tb02a_v1.json")
    pv, tv = call("advance", "--db", db)
    log.append({"input": i + 1, "accept_ms": round(ta, 1), "advance_ms": round(tv, 1), "accept_rc": pa.returncode, "advance_rc": pv.returncode,
                "err": (pa.stdout + pa.stderr + pv.stderr)[-300:] if (pa.returncode or pv.returncode) else None})
    if pa.returncode or pv.returncode:
        break
    if i + 1 in (50, 51, 52):
        ps, _ = call("snapshot", "--db", db)
        (OUT / f"snapshot-{i + 1}.json").write_text(ps.stdout)
        snaps[i + 1] = ps.returncode
(OUT / "LOG.json").write_text(json.dumps({"log": log, "snapshots": snaps}, ensure_ascii=False, indent=1))
print(json.dumps({"inputs": len(log), "last": log[-1], "max_advance_ms": max(x["advance_ms"] for x in log), "snapshots": snaps}, ensure_ascii=False))
