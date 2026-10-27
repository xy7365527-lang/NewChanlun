from __future__ import annotations

import asyncio
import json
import re
import sqlite3
from collections.abc import AsyncIterator, Callable
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, TypeVar

from langchain_core.runnables import RunnableConfig
from langchain_openai import ChatOpenAI
from langgraph.checkpoint.sqlite import SqliteSaver
from pydantic import SecretStr
from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.middleware.cors import CORSMiddleware
from starlette.responses import JSONResponse
from starlette.types import ASGIApp, Message, Receive, Scope, Send

from .agent import (
    CLIPROXY_BASE_URL,
    TraceRecorder,
    build_agent,
    disable_external_tracing,
)
from .domain import RunId
from .run import DEFAULT_HOME, PilotRun, ToolRefusal
from .studio_binding import (
    ASSISTANT_IDS,
    Lease,
    RunBoundCheckpointer,
    grant,
    run_for_thread,
    state_root,
)


@dataclass(frozen=True)
class StudioTarget:
    home: Path
    run_id: RunId


TARGET = StudioTarget(DEFAULT_HOME, RunId("real4"))
READ_POST_PATHS = frozenset({"/assistants/search", "/assistants/count", "/threads/search", "/threads/count"})
THREAD_PATH = re.compile(r"/threads/([^/]+)(/.*)?")
RUN_CREATE = frozenset({"/runs", "/runs/stream", "/runs/wait"})
CANCEL = re.compile(r"/runs/[^/]+/cancel")
STUDIO_RECURSION_LIMIT = 400
EXPECT = "pilot1742_expect"
T = TypeVar("T")


def _read_only_model(run: PilotRun) -> ChatOpenAI:
    return ChatOpenAI(model=run.spec.brain_model, base_url=CLIPROXY_BASE_URL,
                      api_key=SecretStr("studio-read-only-not-a-credential"))


def _open_bound(run_id: RunId) -> PilotRun:
    """与 `python -m pilot1742.executor run` 同一入口：先 run 锁，再在票级锁内复核持有。"""
    import os

    from .claims import ClaimStore
    from .delivery import RepoGateway
    from .executor import WORKERS_LOG, TicketExecutor, load_unit, open_v2_run
    from .github_app import WRITE_PERMISSIONS, AppIdentity, GitHubAppClient
    from .run import PSTACK_MODELS
    from .workunit import REPO

    key = os.environ.get("PILOT1742_APP_PRIVATE_KEY")
    if not key:
        raise ToolRefusal("Studio 服务未配置 PILOT1742_APP_PRIVATE_KEY，不能执行 v2 run")
    unit, task = load_unit(state_root(), run_id)
    identity = AppIdentity(5486877, 175617978, 1150335160, REPO, "newchanlun-deepagents-1742", Path(key))
    client = GitHubAppClient(identity, permissions=WRITE_PERMISSIONS)
    client.verify()
    executor = TicketExecutor(unit, RepoGateway(client), claims=ClaimStore(), workers_log=WORKERS_LOG)
    return open_v2_run(state_root(), unit, task, executor, PSTACK_MODELS.read_text())


OPEN_BOUND = _open_bound
EXECUTION_MODEL: Any = None


async def _to_completion(fn: Callable[[], T]) -> tuple[asyncio.Future[T], bool]:
    """在线程里把阻塞调用跑完。线程不能被取消，等待期间到达的取消先记下，等线程真正结束后再交回调用方，
    否则调用方的清理会和仍在运行的线程并发，线程取得的资源也会无人接管。"""
    done: asyncio.Future[T] = asyncio.ensure_future(asyncio.to_thread(fn))
    cancelled = False
    while not done.done():
        try:
            await asyncio.wait({done})
        except asyncio.CancelledError:
            cancelled = True
    return done, cancelled


async def _blocking(fn: Callable[[], T], *, abandon: Callable[[T], object] | None = None) -> T:
    """跑完 fn 再返回其结果。期间被取消时，fn 取得的资源交给 abandon 收回（同样跑完），然后把取消传下去。"""
    done, cancelled = await _to_completion(fn)
    if not cancelled:
        return done.result()
    if abandon is not None and not done.cancelled() and done.exception() is None:
        resource = done.result()
        await _to_completion(lambda: abandon(resource))
    raise asyncio.CancelledError


@asynccontextmanager
async def graph(config: RunnableConfig) -> AsyncIterator[Any]:
    """官方图工厂。绑定线程的执行请求打开同一个 v2 PilotRun，在整个官方 run 期间持有 run 锁；
    其余请求（看图、读状态、未绑定线程）只构建只读图，不取锁、不调模型。"""
    configurable = config.get("configurable", {})
    run_id = run_for_thread(configurable.get("thread_id"))
    executing = bool(configurable.get("__is_for_execution__"))
    if run_id is None or not executing:
        def read_only() -> Any:
            disable_external_tracing()
            run = PilotRun.open(state_root(), run_id) if run_id else PilotRun.open(TARGET.home, TARGET.run_id)
            return build_agent(run, checkpointer=None, model=_read_only_model(run))

        if executing:
            raise ToolRefusal("该线程未绑定已登记的 v2 run，不能执行")
        yield await asyncio.to_thread(read_only)
        return

    expect = configurable.get(EXPECT)
    if not isinstance(expect, dict):
        raise ToolRefusal("执行请求未经本服务的 Studio 入口归一化，拒绝")
    run = await _blocking(lambda: OPEN_BOUND(run_id), abandon=lambda opened: opened.release())

    def prepare() -> Any:
        problem = continuation_problem(run, resume=bool(expect.get("resume")), checkpoint=expect.get("checkpoint"))
        if problem:
            raise ToolRefusal(problem)
        run.clear_interrupt()
        run.trail("studio_official_run", thread_id=str(configurable.get("thread_id")))
        return build_agent(run, None, EXECUTION_MODEL)

    lease: Lease | None = None

    def finish() -> None:
        run.wait_tools_idle()
        if lease is not None:
            lease.revoke()
        run.release()

    try:
        agent = await _blocking(prepare)
        lease = grant(run_id)
        yield agent.with_config(recursion_limit=STUDIO_RECURSION_LIMIT,
                                callbacks=[TraceRecorder(run.dir / "agent-trace.jsonl")])
    finally:
        await _blocking(finish)


def continuation_problem(run: PilotRun, *, resume: bool, checkpoint: str | None) -> str | None:
    """在 run 锁内判定这次官方执行能否推进：只推进已有 checkpoint 且未结束的 run，checkpoint 必须仍是请求时那个。"""
    conn = sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)
    try:
        agent = build_agent(run, SqliteSaver(conn), _read_only_model(run))
        snapshot = agent.get_state({"configurable": {"thread_id": run.spec.run_id}})
    finally:
        conn.close()
    interrupts = [i for task in snapshot.tasks for i in getattr(task, "interrupts", ())]
    if not snapshot.values:
        return "run_not_started：首跑由执行器 CLI 发起，Studio 不创造任务"
    if not interrupts and not snapshot.next:
        return "run_finished：该 run 的图已结束，Studio 只读"
    current = snapshot.config["configurable"].get("checkpoint_id")
    if checkpoint is not None and checkpoint != current:
        return f"checkpoint_moved：请求基于 {checkpoint}，当前为 {current}，不分叉"
    if resume and not interrupts:
        return "nothing_to_resume：当前没有停在中断处，用继续而不是恢复"
    if not resume and interrupts:
        return "interrupted：该 run 停在中断处，只能在中断处恢复"
    return None


def _refuse(detail: str) -> JSONResponse:
    return JSONResponse({"detail": detail}, status_code=403)


THREAD_CREATE_FIELDS = frozenset({"thread_id", "metadata", "if_exists"})
RUN_FIELDS = frozenset({"assistant_id", "input", "command", "config", "metadata", "stream_mode", "stream_subgraphs",
                        "stream_resumable", "on_disconnect", "multitask_strategy", "if_not_exists", "interrupt_before",
                        "interrupt_after", "feedback_keys", "durability", "checkpoint", "checkpoint_during"})
CHECKPOINT_ALIASES = ("thread_ts", "checkpoint_id")
CONFIG_FIELDS = frozenset({"configurable", "tags", "recursion_limit", "metadata"})


def _unknown(body: dict[str, Any], allowed: frozenset[str]) -> str | None:
    extra = sorted(set(body) - allowed)
    return "fields_not_allowed:" + ",".join(extra) if extra else None


def thread_create_problem(body: dict[str, Any]) -> str | None:
    """建线程只准声明绑定线程的 id 与元数据；supersteps、ttl 等会初始化状态或改生命周期的字段一律拒绝。"""
    problem = _unknown(body, THREAD_CREATE_FIELDS)
    if problem:
        return problem
    if run_for_thread(body.get("thread_id")) is None:
        return "thread_not_bound：只能创建绑定已登记 v2 run 的线程，见 STUDIO.md"
    if not isinstance(body.get("metadata", {}), dict):
        return "metadata_not_object"
    if body.get("if_exists", "raise") not in ("raise", "do_nothing"):
        return "if_exists_not_allowed"
    return None


@dataclass(frozen=True)
class RunRequest:
    """归一化后的官方 run 请求：body 去掉了 checkpoint 指针与别名，claimed 是请求声称所基于的 checkpoint。"""

    body: dict[str, Any]
    resume: bool
    claimed: str | None


def normalize_run_request(body: dict[str, Any], thread_id: str) -> RunRequest | str:
    """把官方 Studio 实际发出的继续或恢复请求归一成一种语义，或返回拒绝理由。

    继续：Submit 发 input={}，等同 CLI --resume 的 input=None，只推进已有 checkpoint 的待执行任务。
    恢复：中断处 Resume 发 command.resume。checkpoint 指针、thread_ts、checkpoint_id 只是声明“基于当前头”，
    核对后剥掉，执行总从当时的最新 checkpoint 继续，不能分叉。rollback 改为 reject：在飞 run 时冲突拒绝，
    不取消已有任务、不回滚状态。"""
    problem = _unknown(body, RUN_FIELDS)
    if problem:
        return problem
    if body.get("assistant_id") not in ASSISTANT_IDS:
        return "assistant_not_pilot1742"
    command = body.get("command")
    if command is not None and (not isinstance(command, dict) or set(command) != {"resume"}):
        return "command_only_resume"
    if body.get("input") not in (None, {}) or (command is not None and body.get("input") is not None):
        return "input_not_allowed：首跑由执行器 CLI 发起，Studio 只继续、恢复与中断"
    strategy = body.get("multitask_strategy")
    if strategy not in (None, "reject", "enqueue", "rollback"):
        return "multitask_strategy_not_allowed"
    if body.get("checkpoint_during") not in (None, True):
        return "checkpoint_during_not_allowed"
    if body.get("durability") not in (None, "sync", "async"):
        return "durability_not_allowed"
    if body.get("if_not_exists") not in (None, "reject"):
        return "if_not_exists_not_allowed"
    config = body.get("config") or {}
    if not isinstance(config, dict) or _unknown(config, CONFIG_FIELDS):
        return "config_not_allowed"
    if not isinstance(config.get("configurable") or {}, dict):
        return "configurable_override_not_allowed"
    configurable = dict(config.get("configurable") or {})
    claimed: set[str] = set()
    for key in CHECKPOINT_ALIASES:
        if key in configurable:
            claimed.add(str(configurable.pop(key)))
    if configurable.pop("thread_id", thread_id) != thread_id or configurable.pop("checkpoint_ns", "") != "":
        return "configurable_override_not_allowed"
    if any(k.startswith(("__", "checkpoint", "pilot1742")) for k in configurable):
        return "configurable_override_not_allowed"
    pointer = body.get("checkpoint")
    if pointer is not None:
        if not isinstance(pointer, dict) or set(pointer) - {"checkpoint_id", "thread_id", "checkpoint_ns"}:
            return "checkpoint_not_allowed"
        if pointer.get("thread_id", thread_id) != thread_id or pointer.get("checkpoint_ns", "") != "":
            return "checkpoint_not_allowed"
        if "checkpoint_id" in pointer:
            claimed.add(str(pointer["checkpoint_id"]))
    if len(claimed) > 1:
        return "checkpoint_ambiguous"
    out = {k: v for k, v in body.items() if k not in ("input", "checkpoint", "checkpoint_during")}
    if strategy == "rollback":
        out["multitask_strategy"] = "reject"
    resume = command is not None
    out["config"] = {**config, "configurable": {**configurable, EXPECT: {
        "resume": resume, "checkpoint": next(iter(claimed), None)}}}
    return RunRequest(out, resume, next(iter(claimed), None))


async def current_checkpoint(thread_id: str) -> str | None:
    item = await RunBoundCheckpointer().aget_tuple({"configurable": {"thread_id": thread_id, "checkpoint_ns": ""}})
    return item.checkpoint["id"] if item else None


class BoundStudio:
    """官方协议的收窄面：读取放行；写入只放行绑定线程上的建线程、继续或恢复 run 与中断。

    中断请求不取消服务端任务，只写 run 的中断请求，图在下一次工具调用前停在 checkpoint；工蜂不被杀。"""

    def __init__(self, app: ASGIApp) -> None:
        self.app = app

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] == "websocket":
            await send({"type": "websocket.close", "code": 4403})
            return
        if scope["type"] != "http" or scope["method"] in {"GET", "HEAD", "OPTIONS"}:
            await self.app(scope, receive, send)
            return
        method, path = scope["method"], scope["path"]
        if method == "POST" and (path in READ_POST_PATHS or (
                path.startswith("/assistants/") and path.endswith("/versions"))):
            await self.app(scope, receive, send)
            return
        body = await _buffer(receive)
        verdict = await self._judge(method, path, body)
        if isinstance(verdict, JSONResponse):
            await verdict(scope, receive, send)
            return
        if verdict is not None:
            body = verdict
            headers = [(k, v) for k, v in scope["headers"] if k.lower() != b"content-length"]
            scope = {**scope, "headers": [*headers, (b"content-length", str(len(body)).encode())]}
        await self.app(scope, _replay(body, receive), send)

    async def _judge(self, method: str, path: str, raw: bytes) -> JSONResponse | bytes | None:
        """返回拒绝、改写后的请求体，或 None（原样放行）。"""
        try:
            body = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            return _refuse("请求体不是 JSON")
        if not isinstance(body, dict):
            return _refuse("请求体必须是 JSON 对象")
        if method == "POST" and path == "/threads":
            problem = thread_create_problem(body)
            return _refuse(problem) if problem else None
        match = THREAD_PATH.fullmatch(path)
        if method != "POST" or match is None:
            return _refuse("此 Studio 入口只开放绑定 run 的继续、恢复与中断")
        run_id, rest = run_for_thread(match.group(1)), match.group(2) or ""
        if run_id is None:
            return _refuse("该线程未绑定已登记的 v2 run")
        if rest == "/history":
            return None
        if rest in RUN_CREATE:
            request = normalize_run_request(body, match.group(1))
            if isinstance(request, str):
                return _refuse(request)
            if request.claimed is not None:
                current = await current_checkpoint(match.group(1))
                if request.claimed != current:
                    return _refuse(f"checkpoint_not_current：请求基于 {request.claimed}，当前为 {current}，不分叉")
            return json.dumps(request.body, ensure_ascii=False).encode()
        if CANCEL.fullmatch(rest):
            interrupt = await asyncio.to_thread(lambda: PilotRun.open(state_root(), run_id).request_interrupt("studio"))
            return JSONResponse({"interrupt_requested": interrupt}, status_code=202)
        return _refuse("该操作会改写线程状态或存储，不开放")


async def _buffer(receive: Receive) -> bytes:
    chunks: list[bytes] = []
    while True:
        message = await receive()
        if message["type"] != "http.request":
            break
        chunks.append(message.get("body", b""))
        if not message.get("more_body"):
            break
    return b"".join(chunks)


def _replay(body: bytes, receive: Receive) -> Receive:
    sent = False

    async def replay() -> Message:
        nonlocal sent
        if not sent:
            sent = True
            return {"type": "http.request", "body": body, "more_body": False}
        return await receive()

    return replay


app = Starlette(middleware=[
    Middleware(CORSMiddleware, allow_origins=["https://smith.langchain.com"],
               allow_methods=["GET", "POST", "OPTIONS"], allow_headers=["*"],
               allow_credentials=True, allow_private_network=True),
    Middleware(BoundStudio),
])
