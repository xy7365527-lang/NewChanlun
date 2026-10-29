"""#1768：用原公共 AuditReader 只读核冻结旧规则 oracle 重建夹具（私有副本，不改原件）。"""

import json
import shutil
import sqlite3
import sys
import tempfile
from contextlib import closing
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "s_session"))
sys.path.insert(0, str(ROOT / "s_session/tests"))
import s_readonly_server as reader
from tb01c_query_tests import resources

audit = reader._query_integrity()
query = reader._query_module("s_query")
EXPECT = {"g0": (0, 0), "g1": (1, 1), "g2": (2, 2), "g3": (3, 3), "g4": (4, 4),
          "g0-pending": (0, 1), "g4-pending": (4, 5)}


def check(path):
    with tempfile.TemporaryDirectory() as tmp:
        copy = Path(tmp) / path.name
        with closing(reader.open_readonly(path)) as src, closing(sqlite3.connect(copy)) as dst:
            src.backup(dst)
        r = query.AuditReader(copy, resources(), vars(reader))
        try:
            proof = r.capture_verified()
            gen, raw = proof["generation"], len(proof["raw"])
            cuts = {}
            with closing(reader.open_readonly(copy)) as conn:
                conn.execute("BEGIN")
                for cut in [None, *range(gen + 1)]:
                    expected = reader._project_wire_integers(reader._read_snapshot_in_tx(conn, cut))
                    cuts[str(cut)] = audit.project_state(proof, cut, vars(reader))["snapshot"] == expected
        finally:
            r.close()
    return {"generation": gen, "raw": raw, "capture_digest": proof.get("capture_digest"),
            "cut_projection_equal": cuts}


def main(root):
    out, ok = {}, True
    for mode in sorted(p for p in Path(root).iterdir() if p.is_dir()):
        for name, (gen, raw) in EXPECT.items():
            f = mode / "frozen" / f"{name}.sqlite"
            key = f"{mode.name}/{name}"
            try:
                res = check(f)
                res["ok"] = res["generation"] == gen and res["raw"] == raw and all(res["cut_projection_equal"].values())
            except Exception as error:
                res = {"ok": False, "error": f"{type(error).__name__}: {error}"}
            ok &= res["ok"]
            out[key] = res
    print(json.dumps(out, indent=1, ensure_ascii=False, default=str))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
