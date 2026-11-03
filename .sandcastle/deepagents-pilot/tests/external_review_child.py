"""独审监督进程替身。只生成测试协议，不发模型请求。"""
import json
import os
import sys
import uuid
from pathlib import Path

folder = Path(sys.argv[sys.argv.index("--output-dir") + 1])
launch = json.loads((folder / "launch.json").read_text())
with open(folder.parents[1] / "launches", "a") as log:
    log.write(launch["model"] + "\n")
mode = (folder.parents[1] / "fake-mode").read_text().strip() if (folder.parents[1] / "fake-mode").exists() else "PASS"
session = str(uuid.uuid4())
output = {"status": "blocked" if mode == "BLOCK" else "completed", "summary": f"VERDICT: {mode}",
          "findings": [], "validation": ["测试替身"]}
events = [{"type": "thread.started", "thread_id": session},
          {"type": "item.completed", "item": {"type": "agent_message", "text": json.dumps(output)}},
          {"type": "turn.completed"}]
(folder / "events.jsonl").write_text("\n".join(json.dumps(e) for e in events) + "\n")
(folder / "session.jsonl").write_text(json.dumps({"type": "session_meta", "payload": {"id": session}}) + "\n")
record = {**launch, "status": "completed", "sandbox": "docker-read-only", "exit_code": 0, "container_closed": True,
          "image_id": "sha256:" + "1" * 64, "container_id": "a" * 64,
          "filesystem": {"source_write": "EROFS", "git_write": "EROFS", "host_read": "ENOENT", "state_write": "ENOENT"},
          "session_id": session, "turn_completed": True, "result": output,
          "before": {"head": launch["head_sha"], "branch": "HEAD", "status": ""},
          "after": {"head": launch["head_sha"], "branch": "HEAD", "status": ""}}
(folder / "receipt.tmp").write_text(json.dumps(record))
os.replace(folder / "receipt.tmp", folder / "receipt.json")
