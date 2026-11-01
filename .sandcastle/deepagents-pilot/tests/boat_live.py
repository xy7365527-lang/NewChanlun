"""Live Boat closure driver with a no-model fixture worker. Mac runs only this control plane; every build,
install and worker process runs on Boat VMs created from the configured saved snapshot.

    python3 boat_live.py setup <work> <base-sandbox> <image>
    python3 boat_live.py dispatch <work> <role> [--crash after_boat_submit_before_record] [--sleep N]
    python3 boat_live.py observe|status|commit|cancel <work> [role]
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
REPO = HERE.parents[2]

from pilot1742.boat_execution import (  # noqa: E402
    BoatCLI,
    BoatExecution,
    BoatForkAPI,
    ExecutionUnknown,
)
from pilot1742.domain import WorkerKey, parse_run_id  # noqa: E402
from pilot1742.run import (  # noqa: E402
    Halted,
    PilotRun,
    TaskSpec,
    ToolRefusal,
    _fact_json,
    current_head,
)

MODELS = ("feature, refactoring: gpt-6.1-sol\njudgment and prose: claude-opus-5-5\nhardest tasks: claude-opus-5-5\n"
          "architect runners: claude-opus-5-5, gpt-6.1-sol\narena cross-judge pool: claude-opus-5-5, gpt-6.1-sol\n")
TASK = TaskSpec("Boat夹具闭环", "夹具修改 demo/ 下的中文路径文件。", ("demo/",), False)
RUNTIME = ["package.json", "package-lock.json", ".sandcastle/boat-worker.mts", ".sandcastle/codex-child.ts",
           ".sandcastle/docker-isolation.mts", ".sandcastle/acceptance-dir.mts", ".sandcastle/git-environment.ts",
           ".sandcastle/main-model.mts", ".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.json",
           ".sandcastle/deepagents-pilot/pilot1742/boat_remote.py"]


class LoggedCLI(BoatCLI):
    """Real Boat CLI that appends every invocation to calls.jsonl so a read-only claim is checkable."""

    def __init__(self, log: Path):
        super().__init__()
        self.log = log

    def call(self, *args: str) -> list[dict]:
        with self.log.open("a") as handle:
            handle.write(json.dumps({"pid": os.getpid(), "args": list(args)}, ensure_ascii=False) + "\n")
        return super().call(*args)


class LoggedForkAPI(BoatForkAPI):
    def __init__(self, log: Path):
        self.log = log

    def fork(self, intent):
        sandbox = super().fork(intent)
        with self.log.open('a') as handle:
            handle.write(json.dumps({'api': 'fork', 'idempotency_key': intent.idempotency_key,
                                     'body': json.loads(intent.body), 'sandbox_id': sandbox}) + '\n')
        return sandbox


class FixtureExecution(BoatExecution):
    def __init__(self, run, work: Path):
        super().__init__(run, LoggedCLI(work / "calls.jsonl"), LoggedForkAPI(work / "fork-api.jsonl"))
        self.work = work

    def _key_source(self) -> list[str]:
        return ["printf", "fixture-not-a-model-key"]

    def _build(self, key, binding):
        local = super()._build(key, binding)
        (local / "runtime/.sandcastle/boat-worker.mts").write_bytes((HERE / "fixtures/boat_fixture_worker.mts").read_bytes())
        control = json.loads((self.work / "fixture-mode.json").read_text())
        (local / "runtime/fixture-mode.json").write_text(json.dumps(control))
        return local


def git(root: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True).stdout.strip()


def setup(work: Path, snapshot: str, image: str) -> None:
    source = work / "source"
    source.mkdir(parents=True)
    git(source, "init", "-q", "-b", "main")
    for relative in RUNTIME:
        (source / relative).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(REPO / relative, source / relative)
    (source / ".agents/skills/demo").mkdir(parents=True)
    (source / ".agents/skills/demo/SKILL.md").write_text("---\nname: demo\ndescription: fixture\n---\n夹具项目技能\n")
    (source / "demo").mkdir()
    (source / "demo/中文 定义.md").write_text("原文\n")
    (source / "demo/删除我.md").write_text("将被删除\n")
    (source / "demo/keep.sh").write_text("echo keep\n")
    (source / "AGENTS.md").write_text("夹具仓库\n")
    git(source, "add", ".")
    git(source, "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "fixture base")
    config = {"endpoint": "https://example.invalid/v1", "image": image, "base_sandbox": snapshot,
              "key_source_sandbox": "bx_fixture", "key_source_path": "/fixture/not-read",
              "pstack_source": "/Users/silencehan/.codex/pstack-upstream-source",
              "model_rules": str(Path.home() / ".agents/pstack-models.md"), "ttl": 1800}
    (work / "boat-config.json").write_text(json.dumps(config, indent=2))
    (work / "fixture-mode.json").write_text(json.dumps({"sleepSeconds": 5}))
    os.environ["PILOT_BOAT_CONFIG"] = str(work / "boat-config.json")
    run = PilotRun.create(work / "home", parse_run_id("live1"), TASK, git(source, "rev-parse", "HEAD"), MODELS,
                          source_repo=source)
    run.release()
    print(json.dumps({"run": str(run.dir), "base": run.spec.base, "execution": run.spec.to_json().get("execution")}, ensure_ascii=False))


def open_run(work: Path, crash: str | None = None) -> PilotRun:
    run = PilotRun.open(work / "home", parse_run_id("live1"), source_repo=work / "source", crash_at=crash)
    run.execution = FixtureExecution(run, work)
    return run


def main() -> int:
    action, work = sys.argv[1], Path(sys.argv[2]).resolve()
    if action == "setup":
        setup(work, sys.argv[3], sys.argv[4])
        return 0
    args = sys.argv[3:]
    crash = args[args.index("--crash") + 1] if "--crash" in args else None
    if "--sleep" in args:
        (work / "fixture-mode.json").write_text(json.dumps({"sleepSeconds": int(args[args.index("--sleep") + 1])}))
    run = open_run(work, crash)
    if action == "status":
        print(json.dumps(run.status(), ensure_ascii=False, indent=2))
        return 0
    run.acquire()
    try:
        if action == "dispatch":
            receipt = run.run_worker(args[0], "夹具闭环：按 fixture-mode 执行")
            print(json.dumps({"status": receipt.status, "summary": receipt.summary,
                              "changed": list(receipt.changed_paths_after)}, ensure_ascii=False))
        elif action == "observe":
            facts = run.facts()
            print(json.dumps({key.name: _fact_json(fact) for key, fact in facts.workers.items()}, ensure_ascii=False))
        elif action == "commit":
            print(json.dumps({"commit": run.commit_candidate().commit}))
        elif action == "cancel":
            key = WorkerKey(run.spec.run_id, args[0], current_head(run.facts(reconcile=False)))
            print(json.dumps(_fact_json(run.execution.cancel(key)), ensure_ascii=False))
        return 0
    except (Halted, ToolRefusal, ExecutionUnknown) as error:
        print(json.dumps({"error": type(error).__name__, "reason": str(error)}, ensure_ascii=False))
        return 2
    finally:
        run.release()


if __name__ == "__main__":
    raise SystemExit(main())
