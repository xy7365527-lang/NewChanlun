"""隔离 Studio 实测夹具：在官方 Agent Server 里挂一个明确的测试 run。

GitHub 是进程内 FakeGitHub，工蜂是 fake_codex_child，模型是脚本替身，签批信任未安装。图工厂、自定义
checkpointer、HTTP 收窄面与 run 锁都是生产代码。只由 langgraph.live-fixture.json 加载。
"""

from __future__ import annotations

import os
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from langchain_core.messages import AIMessage  # noqa: E402
from langgraph.checkpoint.sqlite import SqliteSaver  # noqa: E402
from test_agent import call, scripted  # noqa: E402
from v2env import V2Env  # noqa: E402

from pilot1742 import studio  # noqa: E402
from pilot1742.agent import build_agent  # noqa: E402
from pilot1742.studio_binding import thread_for  # noqa: E402

ROOT = Path(tempfile.mkdtemp(prefix="pilot1742-studio-live-", dir=os.environ.get("PILOT1742_LIVE_PARENT")))
ENV = V2Env(ROOT)
UNIT = ENV.register(candidate_approval=True)


def _start_like_cli() -> None:
    run = ENV.run(UNIT)
    try:
        ENV.to_pass(run)
        agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)),
                            scripted(call("publish_candidate"), AIMessage("已交付")))
        agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": UNIT.run_id}}, durability="sync")
    finally:
        run.release()


_start_like_cli()
os.environ["PILOT1742_STATE_ROOT"] = str(ENV.home)
os.environ["PILOT1742_STUDIO_RUNS"] = UNIT.run_id
studio.OPEN_BOUND = lambda run_id: ENV.run(ENV.load(run_id))
studio.EXECUTION_MODEL = scripted(*[AIMessage("已交付") for _ in range(50)])
(ROOT / "LIVE.txt").write_text(f"run_id={UNIT.run_id}\nthread_id={thread_for(UNIT.run_id)}\nstate_root={ENV.home}\n")
print(f"pilot1742 live fixture run_id={UNIT.run_id} thread_id={thread_for(UNIT.run_id)} root={ROOT}", flush=True)

graph = studio.graph
