"""#1527 返修定向命令记录器；只写本轮独占证据目录。"""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

REPORT = Path(__file__).resolve().parent
ROOT = REPORT.parents[2]
SOURCES = ("s_session/recovery_admission.py", "s_session/s_service_control.py",
           "s_session/tests/issue1527_recovery/test_python_r4.py")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    label, *argv = sys.argv[1:]
    path = REPORT / "CHECKS.json"
    checks = json.loads(path.read_text()) if path.exists() else {
        "issue": 1527, "baseline": "52a86c684768b884aec24a3f794a89a4fe8a3402",
        "independent_review": "NOT_RUN", "executions": []}
    assert label not in [item["label"] for item in checks["executions"]]
    hashes = {name: digest(ROOT / name) for name in SOURCES}
    started = time.monotonic()
    result = subprocess.run(argv, cwd=ROOT, capture_output=True)
    record = dict(label=label, argv=argv, cwd=str(ROOT), exit_code=result.returncode,
                  elapsed_seconds=time.monotonic() - started, source_sha256=hashes)
    for stream in ("stdout", "stderr"):
        output = REPORT / (label + "." + stream + ".log")
        output.write_bytes(getattr(result, stream))
        record[stream] = dict(path=output.name, sha256=digest(output), bytes=output.stat().st_size)
    checks["executions"].append(record)
    path.write_text(json.dumps(checks, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps(record, ensure_ascii=False, indent=2))
    sys.stdout.buffer.write(result.stdout)
    sys.stderr.buffer.write(result.stderr)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
