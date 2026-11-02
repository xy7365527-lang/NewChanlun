"""测试替身：与 codex-child CLI 同参数、同认领语义（排他 mkdir outputDir），行为由 <home>/fake-control.json 控制。"""
import argparse
import json
import os
import subprocess
import sys
import time
import uuid
from pathlib import Path

parser = argparse.ArgumentParser()
for name in ("ticket", "parent-thread-id", "manager-agent-id", "checkout", "expected-head", "mode", "model",
             "idle-timeout-seconds", "timeout-seconds", "prompt-file", "output-dir"):
    parser.add_argument(f"--{name}")
args = parser.parse_args()
out = Path(args.output_dir)
home = out.parents[3]
control = json.loads((home / "fake-control.json").read_text())
step = control.get(out.name.rpartition("-")[0], {})
os.mkdir(out)
with open(home / "fake-claims.log", "a") as fh:
    fh.write(f"{out.name}\n")
state = {"status": "running", "process_pid": os.getpid()}
(out / "state.json").write_text(json.dumps(state))
time.sleep(step.get("sleep", 0))
for rel, content in step.get("write", {}).items():
    target = Path(args.checkout) / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content)
head = subprocess.run(["git", "-C", args.checkout, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
status = subprocess.run(["git", "-C", args.checkout, "status", "--porcelain=v1", "--untracked-files=all"],
                        capture_output=True, text=True).stdout
summary = step.get("summaries", {}).get(args.expected_head, step.get("summary", "完成"))
(out / "result.json").write_text(json.dumps({
    "ticket": int(args.ticket), "run_id": str(uuid.uuid4()), "status": step.get("status", "completed"),
    "mode": args.mode, "model": args.model, "expected_head": args.expected_head,
    "checkout_after": {"head": head, "status": status}, "external_session_id": "fake-session",
    "external_session_log": "/dev/null", "failure_reason": None,
    "result": {"status": "completed", "summary": summary, "findings": step.get("findings", []), "validation": []},
}))
sys.exit(0)
