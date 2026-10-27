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
    _LEASE,
    RunBoundCheckpointer,
    UnboundThreadWrite,
    grant,
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
    with pytest.raises(UnboundThreadWrite, match="不是持有 run"):
        asyncio.run(cp.aput({"configurable": {"thread_id": thread, "checkpoint_ns": ""}}, cli.checkpoint,
                            cli.metadata, {}))


def test_official_execution_holds_the_same_run_lock_and_continues_the_same_checkpoint(v2, bound, monkeypatch):
    run = cli_checkpoint(v2, bound)
    monkeypatch.setattr(studio, "EXECUTION_MODEL", scripted(call("run_status"), AIMessage("Studio 继续")))
    thread = thread_for(bound.run_id)
    config = {"configurable": {"thread_id": thread, "checkpoint_ns": "", "__is_for_execution__": True}}

    async def official_run():
        async with studio.graph(config) as agent:
            lease = _LEASE.get()
            assert lease is not None and lease.active and lease.run_id == bound.run_id
            with pytest.raises(Busy):
                v2.run(bound)
            served = agent.copy(update={"checkpointer": RunBoundCheckpointer()})
            await served.ainvoke({"messages": [("user", "继续")]}, {"configurable": {"thread_id": thread}})
        assert not lease.active

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
    assert _LEASE.get() is None
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


def test_checkpoint_write_needs_the_lease_of_the_execution_that_holds_the_lock(v2, bound, monkeypatch):
    """同进程并发：持锁执行的子任务可写；并发的另一请求、借旧上下文副本的写、别的 run 的写权都被拒。"""
    run = cli_checkpoint(v2, bound)
    saver = SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False))
    cli = saver.get_tuple({"configurable": {"thread_id": bound.run_id, "checkpoint_ns": ""}})
    monkeypatch.setenv("PILOT1742_STUDIO_RUNS", f"{bound.run_id},t4242-2")
    cp = RunBoundCheckpointer()
    thread = thread_for(bound.run_id)
    target = {"configurable": {"thread_id": thread, "checkpoint_ns": ""}}
    execution = {"configurable": {"thread_id": thread, "checkpoint_ns": "", "__is_for_execution__": True}}

    async def write(config=target):
        return await cp.aput(config, cli.checkpoint, cli.metadata, {})

    async def scenario():
        entered, finish = asyncio.Event(), asyncio.Event()
        captured = {}

        async def official_run():
            async with studio.graph(execution):
                entered.set()
                await asyncio.create_task(write())
                captured["context"] = __import__("contextvars").copy_context()
                await finish.wait()

        async def concurrent_request():
            await entered.wait()
            with pytest.raises(UnboundThreadWrite, match="不是持有 run"):
                await write()
            finish.set()

        await asyncio.gather(official_run(), concurrent_request())
        with pytest.raises(UnboundThreadWrite, match="不是持有 run"):
            await asyncio.create_task(write(), context=captured["context"])

    asyncio.run(scenario())

    async def other_runs_lease():
        grant("t4242-2")
        await write()

    with pytest.raises(UnboundThreadWrite, match="不是持有 run"):
        asyncio.run(other_runs_lease())
    v2.run(bound).release()


def test_thread_create_is_an_explicit_whitelist(v2, bound):
    thread, c = thread_for(bound.run_id), client()
    for body in ({"thread_id": thread},
                 {"thread_id": thread, "metadata": {"graph_id": "pilot1742"}, "if_exists": "do_nothing"}):
        assert c.post("/threads", json=body).json() == {"reached": True}, body
    superstep = [{"updates": [{"values": {"messages": [{"type": "human", "content": "批准"}]}, "as_node": "model"}]}]
    for body in ({"thread_id": thread, "supersteps": superstep}, {"thread_id": thread, "ttl": {"ttl": 1}},
                 {"thread_id": thread, "graph_id": "pilot1742"}, {"thread_id": thread, "metadata": ["x"]},
                 {"thread_id": thread, "if_exists": "update"}, [{"thread_id": thread}]):
        assert c.post("/threads", json=body).status_code == 403, body
    resume = {"assistant_id": "pilot1742", "command": {"resume": True}, "stream_mode": ["values"],
              "config": {"configurable": {}}, "on_disconnect": "continue"}
    assert c.post(f"/threads/{thread}/runs/stream", json=resume).json() == {"reached": True}
    for extra in ({"context": {"approved": True}}, {"if_not_exists": "create"},
                  {"config": {"configurable": {"thread_id": str(uuid.uuid4())}}}, {"config": {"callbacks": []}},
                  {"unknown": 1}):
        assert c.post(f"/threads/{thread}/runs/stream", json={**resume, **extra}).status_code == 403, extra


def _cancel_while_blocked(v2, bound, monkeypatch, stage):
    """官方 worker 用 wait_for 包住整个 run；取消或超时可能落在图工厂打开/准备 run 的阻塞线程里。
    线程跑完后取得的 run 必须被释放，同一 run 随后能再次打开。"""
    import threading

    started, proceed = threading.Event(), threading.Event()
    opened = []

    def slow_open(run_id):
        if stage == "open":
            started.set()
            proceed.wait(10)
        run = v2.run(v2.load(run_id))
        opened.append(run)
        return run

    monkeypatch.setattr(studio, "OPEN_BOUND", slow_open)
    if stage == "prepare":
        def slow_build(run, checkpointer, model):
            started.set()
            proceed.wait(10)
            return build_agent(run, checkpointer, scripted())
        monkeypatch.setattr(studio, "build_agent", slow_build)
    config = {"configurable": {"thread_id": thread_for(bound.run_id), "__is_for_execution__": True}}
    entered = []

    async def official_run():
        async with studio.graph(config):
            entered.append(True)

    async def scenario():
        task = asyncio.create_task(official_run())
        await asyncio.to_thread(started.wait, 10)
        task.cancel()
        await asyncio.sleep(0.05)
        assert not task.done(), "取消须等阻塞线程跑完、收回资源后才交回"
        proceed.set()
        with pytest.raises(asyncio.CancelledError):
            await task

    asyncio.run(scenario())
    assert not entered and len(opened) == 1 and _LEASE.get() is None
    v2.run(bound).release()


def test_cancel_while_opening_the_run_releases_the_lock_it_acquired(v2, bound, monkeypatch):
    _cancel_while_blocked(v2, bound, monkeypatch, "open")


def test_cancel_while_preparing_the_run_releases_the_lock(v2, bound, monkeypatch):
    _cancel_while_blocked(v2, bound, monkeypatch, "prepare")


def test_timeout_around_the_factory_releases_the_lock(v2, bound, monkeypatch):
    import threading

    gate = threading.Event()

    def slow_open(run_id):
        gate.wait(0.5)
        return v2.run(v2.load(run_id))

    monkeypatch.setattr(studio, "OPEN_BOUND", slow_open)
    config = {"configurable": {"thread_id": thread_for(bound.run_id), "__is_for_execution__": True}}

    async def official_run():
        async with studio.graph(config):
            pass

    async def scenario():
        with pytest.raises(asyncio.TimeoutError):
            await asyncio.wait_for(official_run(), 0.05)

    asyncio.run(scenario())
    v2.run(bound).release()
