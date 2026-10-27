from __future__ import annotations

import asyncio
import json
import re
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from langchain_core.runnables import RunnableConfig
from langchain_openai import ChatOpenAI
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
from .studio_binding import ASSISTANT_IDS, HELD, run_for_thread, state_root


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

    run = await asyncio.to_thread(OPEN_BOUND, run_id)

    def prepare() -> Any:
        run.clear_interrupt()
        run.trail("studio_official_run", thread_id=str(configurable.get("thread_id")))
        return build_agent(run, None, EXECUTION_MODEL)

    def finish() -> None:
        run.wait_tools_idle()
        HELD.discard(run_id)
        run.release()

    try:
        HELD.add(run_id)
        agent = await asyncio.to_thread(prepare)
        yield agent.with_config(recursion_limit=STUDIO_RECURSION_LIMIT,
                                callbacks=[TraceRecorder(run.dir / "agent-trace.jsonl")])
    finally:
        await asyncio.to_thread(finish)


def _refuse(detail: str) -> JSONResponse:
    return JSONResponse({"detail": detail}, status_code=403)


def run_request_problem(body: dict[str, Any]) -> str | None:
    """绑定线程上的官方 run 请求只准继续或恢复，不准改状态、分叉、注入输入或打断在飞 run。"""
    if body.get("assistant_id") not in ASSISTANT_IDS:
        return "assistant_not_pilot1742"
    if body.get("input") not in (None, {}):
        return "input_not_allowed：首跑由执行器 CLI 发起，Studio 只继续、恢复与中断"
    command = body.get("command")
    if command is not None and (not isinstance(command, dict) or set(command) - {"resume"}):
        return "command_only_resume"
    for key in ("checkpoint", "checkpoint_id", "webhook", "after_seconds"):
        if body.get(key) not in (None, {}):
            return f"{key}_not_allowed"
    if body.get("multitask_strategy") not in (None, "reject", "enqueue"):
        return "multitask_strategy_not_allowed"
    configurable = (body.get("config") or {}).get("configurable") or {}
    if any(k.startswith("__") or k.startswith("checkpoint") for k in configurable):
        return "configurable_override_not_allowed"
    return None


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
        body, replay = await _buffer(receive)
        verdict = await self._judge(method, path, body)
        if isinstance(verdict, JSONResponse):
            await verdict(scope, receive, send)
            return
        await self.app(scope, replay, send)

    async def _judge(self, method: str, path: str, raw: bytes) -> JSONResponse | None:
        try:
            body = json.loads(raw) if raw else {}
        except json.JSONDecodeError:
            return _refuse("请求体不是 JSON")
        if method == "POST" and path == "/threads":
            if run_for_thread(body.get("thread_id")) is None:
                return _refuse("只能创建绑定已登记 v2 run 的线程，见 STUDIO.md")
            return None
        match = THREAD_PATH.fullmatch(path)
        if method != "POST" or match is None:
            return _refuse("此 Studio 入口只开放绑定 run 的继续、恢复与中断")
        run_id, rest = run_for_thread(match.group(1)), match.group(2) or ""
        if run_id is None:
            return _refuse("该线程未绑定已登记的 v2 run")
        if rest == "/history":
            return None
        if rest in RUN_CREATE:
            problem = run_request_problem(body)
            return _refuse(problem) if problem else None
        if CANCEL.fullmatch(rest):
            request = await asyncio.to_thread(lambda: PilotRun.open(state_root(), run_id).request_interrupt("studio"))
            return JSONResponse({"interrupt_requested": request}, status_code=202)
        return _refuse("该操作会改写线程状态或存储，不开放")


async def _buffer(receive: Receive) -> tuple[bytes, Receive]:
    chunks: list[bytes] = []
    while True:
        message = await receive()
        if message["type"] != "http.request":
            break
        chunks.append(message.get("body", b""))
        if not message.get("more_body"):
            break
    body = b"".join(chunks)
    sent = False

    async def replay() -> Message:
        nonlocal sent
        if not sent:
            sent = True
            return {"type": "http.request", "body": body, "more_body": False}
        return await receive()

    return body, replay


app = Starlette(middleware=[
    Middleware(CORSMiddleware, allow_origins=["https://smith.langchain.com"],
               allow_methods=["GET", "POST", "OPTIONS"], allow_headers=["*"],
               allow_credentials=True, allow_private_network=True),
    Middleware(BoundStudio),
])
