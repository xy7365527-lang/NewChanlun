from __future__ import annotations

import argparse
import json
import sqlite3
import subprocess
import sys
from pathlib import Path

from langchain_core.messages import HumanMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command

from .domain import parse_run_id, parse_sha
from .run import (
    CRASH_POINTS,
    DEFAULT_HOME,
    PSTACK_MODELS,
    SOURCE_REPO,
    Busy,
    PilotRun,
    parse_task,
)

EXIT_BUSY = 75


def _graph_state(agent, config: dict) -> dict:
    snapshot = agent.get_state(config)
    interrupts = [i.value for task in snapshot.tasks for i in getattr(task, "interrupts", ())]
    return {"has_checkpoint": bool(snapshot.values), "next": list(snapshot.next), "interrupts": interrupts}


def _run(args: argparse.Namespace) -> int:
    from .agent import (
        TraceRecorder,
        build_agent,
        disable_external_tracing,
        initial_message,
    )

    disable_external_tracing()
    home = Path(args.home)
    run_id = parse_run_id(args.run_id)
    kw = {"crash_at": args.crash_at}
    try:
        if (PilotRun.run_dir(home, run_id) / "run.json").exists():
            if args.task:
                print("该 run 已存在，任务已冻结；恢复时不要再传 --task", file=sys.stderr)
                return 2
            run = PilotRun.open(home, run_id, **kw)
            run.acquire()
            run.ensure_checkouts()
        else:
            if not args.task:
                print("新 run 需要 --task", file=sys.stderr)
                return 2
            base = parse_sha(args.base or subprocess.run(["git", "-C", str(SOURCE_REPO), "rev-parse", "HEAD"],
                                                         check=True, capture_output=True, text=True).stdout.strip())
            run = PilotRun.create(home, run_id, parse_task(Path(args.task).read_text()), base,
                                  PSTACK_MODELS.read_text(), **kw)
    except Busy as error:
        print(str(error), file=sys.stderr)
        return EXIT_BUSY

    conn = sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)
    agent = build_agent(run, SqliteSaver(conn))
    config = {"configurable": {"thread_id": run_id}, "callbacks": [TraceRecorder(run.dir / "agent-trace.jsonl")],
              "recursion_limit": 400}
    graph = _graph_state(agent, config)
    if not graph["has_checkpoint"]:
        run.trail("graph_start")
        agent.invoke({"messages": [HumanMessage(initial_message(run))]}, config, durability="sync")
    elif graph["interrupts"]:
        if not args.recheck_approval:
            print(json.dumps({"graph": graph, "status": run.status()}, ensure_ascii=False, indent=2))
            return 0
        run.trail("graph_resume_approval_recheck")
        agent.invoke(Command(resume={"recheck": True}), config, durability="sync")
    elif graph["next"]:
        run.trail("graph_resume_after_crash", next=graph["next"])
        agent.invoke(None, config, durability="sync")
    else:
        run.trail("graph_already_finished")
    report = {"graph": _graph_state(agent, config), "status": run.status()}
    if not report["graph"]["next"] and report["status"]["phase"] not in ("gate", "halted"):
        last = agent.get_state(config).values["messages"][-1]
        report["brain_stopped_early"] = {"finish_reason": last.response_metadata.get("finish_reason"),
                                         "text": str(last.content)[:2000]}
        run.trail("brain_stopped_early", **report["brain_stopped_early"])
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


def _status(args: argparse.Namespace) -> int:
    run = PilotRun.open(Path(args.home), parse_run_id(args.run_id))
    print(json.dumps(run.status(), ensure_ascii=False, indent=2))
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="pilot1742")
    sub = parser.add_subparsers(dest="command", required=True)
    run = sub.add_parser("run", help="首跑或恢复一个 run")
    run.add_argument("--run-id", required=True)
    run.add_argument("--task", help="新 run 的任务文件（--- 头含 title 与 allowed_prefixes）")
    run.add_argument("--base", help="新 run 的基线提交，缺省为源工作区 HEAD")
    run.add_argument("--home", default=str(DEFAULT_HOME))
    run.add_argument("--crash-at", choices=CRASH_POINTS, help="受控故障注入：在该点对本进程 SIGKILL")
    run.add_argument("--recheck-approval", action="store_true", help="停在 awaiting_approval 时重读批准账本")
    status = sub.add_parser("status", help="只读状态，不启动模型或工蜂")
    status.add_argument("--run-id", required=True)
    status.add_argument("--home", default=str(DEFAULT_HOME))
    args = parser.parse_args(argv)
    return _run(args) if args.command == "run" else _status(args)
