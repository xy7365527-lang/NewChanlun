"""#1527 复用固定 R2 的 H1 组；只取固定对象与显式候选纯源码。"""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
BASE = "52a86c684768b884aec24a3f794a89a4fe8a3402"
REVIEW = "e395c60180501fcc639e80ad24a8f955e75b3d8d"
PROBE = ".chanlun/review-results/issue1527-python-r4-r2-review-20261018/probe_review.py"


def fixed(ref, path):
    return subprocess.check_output(["git", "show", f"{ref}:{path}"], cwd=ROOT)


def main():
    mode = sys.argv[1]
    assert mode in ("baseline", "candidate")
    probe = fixed(REVIEW, PROBE)
    hashes = {}
    with tempfile.TemporaryDirectory(prefix="issue1527-h1-r3-") as tmp:
        here = Path(tmp).resolve()
        source = here / "source/s_session"
        source.mkdir(parents=True)
        for name in ("recovery_admission.py", "s_service_control.py", "s_socket_client.py"):
            path = "s_session/" + name
            raw = fixed(BASE, path) if mode == "baseline" else (ROOT / path).read_bytes()
            hashes[path] = hashlib.sha256(raw).hexdigest()
            (source / name).write_bytes(raw)
        ns = {"__file__": str(here / "probe_review.py"), "__name__": "fixed_h1_probe"}
        exec(compile(probe, PROBE, "exec"), ns)
        c = ns["c"]
        with patch.dict(os.environ, {}, clear=True), \
             patch.object(c, "process_identity", side_effect=AssertionError("禁止真实 ps")), \
             patch.object(c.os, "kill", side_effect=AssertionError("禁止真实信号")), \
             patch.object(c.Controller, "start_process", side_effect=AssertionError("禁止正式进程")):
            rows = ns["h1_control_matrix"](here)
        print(json.dumps(dict(mode=mode, review_source=REVIEW, probe_sha256=hashlib.sha256(probe).hexdigest(),
                              product_sha256=hashes, group="h1_control_matrix", evidence=rows),
                         ensure_ascii=False, indent=2))
        failures = [r["variant"] for r in rows if r["result"] == "REPRODUCED"]
        assert failures == (["float_mode", "float_payload"] if mode == "baseline" else []), failures


if __name__ == "__main__":
    main()
