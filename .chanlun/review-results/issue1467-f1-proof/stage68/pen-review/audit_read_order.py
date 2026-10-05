#!/usr/bin/env python3
"""隔离输出重放仅审读取顺序；数学 oracle 在 independent_check.py。"""
from __future__ import annotations

import contextlib
import json
import resource
import runpy
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
AUTHOR = HERE.parent / "pen-author" / "check_pen.py"
events = []
step_count = 0


def profile(frame, event, arg):
    global step_count
    if event != "call" or frame.f_code.co_filename != str(AUTHOR):
        return
    name = frame.f_code.co_name
    values = frame.f_locals
    if name in ("sha", "load", "items"):
        events.append({"action": name, "path": str(values["path"]), "phase": values.get("phase"), "steps_already_run": step_count})
    elif name == "dump":
        events.append({"action": "dump", "name": values["name"], "steps_already_run": step_count})
    elif name == "step":
        assert values["obs"]["index"] == values["prefix"]["cut"] == step_count
        assert {row["cut"] for row in values["oldrows"].values()} == {step_count}
        if step_count in (0, 442):
            driver = frame.f_back.f_locals["source"]
            events.append({"action": "step-input-snapshot", "cut": step_count, "driver_observation_count": len(driver["observations"]), "driver_event_count": len(driver["events"]), "driver_observation_index_range": [driver["observations"][0]["index"], driver["observations"][-1]["index"]], "step_history_length_before_append": len(values["self"].prices)})
        step_count += 1


sys.dont_write_bytecode = True
sys.argv = [str(AUTHOR), str(HERE / "author-replay")]
with (HERE / "replay-stdout.log").open("w") as output, (HERE / "replay-stderr.log").open("w") as error:
    with contextlib.redirect_stdout(output), contextlib.redirect_stderr(error):
        sys.setprofile(profile)
        try:
            runpy.run_path(str(AUTHOR), run_name="__main__")
        finally:
            sys.setprofile(None)
assert step_count == 443
seal_index = next(i for i, row in enumerate(events) if row["action"] == "dump" and row["name"] == "online-seal.json")
final_loads = [(i, row) for i, row in enumerate(events) if row["action"] == "load" and row["phase"] == "post-seal"]
assert len(final_loads) == 4 and all(i > seal_index and row["steps_already_run"] == 443 for i, row in final_loads)
rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == "darwin" else 1024)
assert rss <= 96 * 1024 * 1024
summary = {"status": "read-order-replay-pass", "steps": step_count, "peak_rss_bytes": rss, "author_replay_count": 1, "additional_histories": 0, "rw_runs": 0, "new_independent_mathematical_samples": 0, "read_order": events, "physical_future_isolation": False, "post_seal_final_json_parse": True, "claim": "profile observes calls; sha reads complete file bytes, source loads all observations/events, prefix parser buffers 65536 characters and only yields current decoded object"}
(HERE / "runtime-read-order.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({key: value for key, value in summary.items() if key != "read_order"}, ensure_ascii=False))
