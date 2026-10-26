import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from pilot1742.run import PilotRun  # noqa: E402

home, run_id, action, crash, source = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]
run = PilotRun.open(Path(home), run_id, launcher=[sys.executable, str(Path(__file__).with_name("fake_codex_child.py"))],
                    crash_at=crash or None, source_repo=Path(source))
run.acquire()
if action == "commit":
    run.commit_candidate()
else:
    run.run_worker(action, "测试 brief")
