import asyncio
import sqlite3
import uuid

import pytest
from langchain_core.messages import AIMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.responses import JSONResponse
from starlette.routing import Route
from starlette.testclient import TestClient
from test_agent import call, scripted
from v2env import V2Env

from pilot1742 import studio
from pilot1742.agent import build_agent
from pilot1742.run import Busy
from pilot1742.studio_binding import (
    HELD,
    RunBoundCheckpointer,
    UnboundThreadWrite,
    thread_for,
)

ALL = ["GET", "POST", "PUT", "PATCH", "DELETE"]


@pytest.fixture
def v2(tmp_path):
    return V2Env(tmp_path)


@pytest.fixture
def bound(v2, monkeypatch):
    unit = v2.register()
    v2.run(unit).release()
    monkeypatch.setenv("PILOT1742_STATE_ROOT", str(v2.home))
    monkeypatch.setenv("PILOT1742_STUDIO_RUNS", unit.run_id)
    monkeypatch.setattr(studio, "OPEN_BOUND", lambda run_id: v2.run(v2.load(run_id)))
    return unit


def cli_checkpoint(v2, unit):
    run = v2.run(unit)
    model = scripted(call("run_status"), AIMessage("看过状态"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": unit.run_id}}, durability="sync")
    run.release()
    return run


def test_bound_thread_reads_the_cli_checkpoint_and_unbound_threads_store_nothing(v2, bound):
    run = cli_checkpoint(v2, bound)
    saver = SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False))
    cli = saver.get_tuple({"configurable": {"thread_id": bound.run_id, "checkpoint_ns": ""}})
    cp = RunBoundCheckpointer()
    thread = thread_for(bound.run_id)
    seen = asyncio.run(cp.aget_tuple({"configurable": {"thread_id": thread, "checkpoint_ns": ""}}))
    assert seen.checkpoint["id"] == cli.checkpoint["id"] and seen.config["configurable"]["thread_id"] == thread
    stranger = {"configurable": {"thread_id": str(uuid.uuid4()), "checkpoint_ns": ""}}
    assert asyncio.run(cp.aget_tuple(stranger)) is None
    with pytest.raises(UnboundThreadWrite, match="未绑定"):
        asyncio.run(cp.aput(stranger, cli.checkpoint, cli.metadata, {}))
    with pytest.raises(UnboundThreadWrite, match="锁不在本服务进程"):
        asyncio.run(cp.aput({"configurable": {"thread_id": thread, "checkpoint_ns": ""}}, cli.checkpoint,
                            cli.metadata, {}))


def test_official_execution_holds_the_same_run_lock_and_continues_the_same_checkpoint(v2, bound, monkeypatch):
    run = cli_checkpoint(v2, bound)
    monkeypatch.setattr(studio, "EXECUTION_MODEL", scripted(call("run_status"), AIMessage("Studio 继续")))
    thread = thread_for(bound.run_id)
    config = {"configurable": {"thread_id": thread, "checkpoint_ns": "", "__is_for_execution__": True}}

    async def official_run():
        async with studio.graph(config) as agent:
            assert bound.run_id in HELD
            with pytest.raises(Busy):
                v2.run(bound)
            served = agent.copy(update={"checkpointer": RunBoundCheckpointer()})
            await served.ainvoke({"messages": [("user", "继续")]}, {"configurable": {"thread_id": thread}})
        assert bound.run_id not in HELD

    asyncio.run(official_run())
    cli = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)),
                      scripted())
    messages = cli.get_state({"configurable": {"thread_id": bound.run_id}}).values["messages"]
    assert [m.content for m in messages if m.type == "ai" and m.content] == ["看过状态", "Studio 继续"]
    v2.run(bound).release()


def test_official_execution_is_refused_while_the_cli_holds_the_run(v2, bound):
    held = v2.run(bound)
    config = {"configurable": {"thread_id": thread_for(bound.run_id), "__is_for_execution__": True}}

    async def official_run():
        async with studio.graph(config):
            pass

    with pytest.raises(Busy):
        asyncio.run(official_run())
    assert bound.run_id not in HELD
    held.release()


def test_unbound_thread_execution_is_refused(bound):
    config = {"configurable": {"thread_id": str(uuid.uuid4()), "__is_for_execution__": True}}

    async def official_run():
        async with studio.graph(config):
            pass

    with pytest.raises(studio.ToolRefusal, match="未绑定"):
        asyncio.run(official_run())


def client():
    async def endpoint(request):
        return JSONResponse({"reached": True})

    app = Starlette(routes=[Route("/{path:path}", endpoint, methods=ALL)], middleware=[Middleware(studio.BoundStudio)])
    return TestClient(app)


def test_official_protocol_is_narrowed_to_the_bound_run(v2, bound):
    thread, other = thread_for(bound.run_id), str(uuid.uuid4())
    c = client()
    resume = {"assistant_id": "pilot1742", "command": {"resume": {"approved": True}}, "stream_mode": ["values"]}
    allowed = [("POST", "/threads", {"thread_id": thread}), ("POST", f"/threads/{thread}/runs/stream", resume),
               ("POST", f"/threads/{thread}/runs", {"assistant_id": "pilot1742", "input": None}),
               ("POST", f"/threads/{thread}/history", {}), ("POST", "/threads/search", {}),
               ("GET", f"/threads/{other}/state", None)]
    for method, path, body in allowed:
        assert c.request(method, path, json=body).json() == {"reached": True}, path
    refused = [("POST", "/threads", {}), ("POST", "/threads", {"thread_id": other}),
               ("POST", f"/threads/{other}/runs/stream", resume),
               ("POST", f"/threads/{thread}/runs/stream", {**resume, "input": {"messages": [["user", "批准"]]}}),
               ("POST", f"/threads/{thread}/runs/stream", {**resume, "command": {"resume": 1, "update": {}}}),
               ("POST", f"/threads/{thread}/runs/stream", {**resume, "checkpoint": {"checkpoint_id": "x"}}),
               ("POST", f"/threads/{thread}/runs/stream", {**resume, "multitask_strategy": "interrupt"}),
               ("POST", f"/threads/{thread}/runs/stream", {**resume, "assistant_id": "other"}),
               ("POST", f"/threads/{thread}/runs/stream",
                {**resume, "config": {"configurable": {"checkpoint_id": "x"}}}),
               ("POST", f"/threads/{thread}/state", {"values": {}}), ("POST", f"/threads/{thread}/copy", {}),
               ("PATCH", f"/threads/{thread}", {}), ("DELETE", f"/threads/{thread}", None),
               ("POST", "/runs/stream", resume), ("PUT", "/store/items", {}), ("POST", "/assistants", {})]
    for method, path, body in refused:
        assert c.request(method, path, json=body).status_code == 403, (path, body)
    cancel = c.post(f"/threads/{thread}/runs/{uuid.uuid4()}/cancel?action=interrupt")
    assert cancel.status_code == 202 and cancel.json()["interrupt_requested"]["by"] == "studio"
    assert (v2.home / "runs" / bound.run_id / "interrupt.request").exists()
