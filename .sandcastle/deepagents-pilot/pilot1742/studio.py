from __future__ import annotations

import asyncio
import os
import threading
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from langchain_core.runnables import RunnableConfig
from langchain_openai import ChatOpenAI
from pydantic import SecretStr
from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.middleware.cors import CORSMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse
from starlette.routing import Route
from starlette.types import ASGIApp, Receive, Scope, Send

from .agent import CLIPROXY_BASE_URL, build_agent, disable_external_tracing
from .domain import RunId, parse_run_id
from .run import DEFAULT_HOME, Busy, Halted, PilotRun, ToolRefusal


@dataclass(frozen=True)
class StudioTarget:
    home: Path
    run_id: RunId


TARGET = StudioTarget(DEFAULT_HOME, RunId("real4"))
READ_POST_PATHS = frozenset({"/assistants/search", "/assistants/count"})
CONTROL_PREFIX = "/pilot1742/v2/"
CONTROL_ACTIONS = ("run", "interrupt")


async def graph(config: RunnableConfig) -> Any:
    def build() -> Any:
        disable_external_tracing()
        run = PilotRun.open(TARGET.home, TARGET.run_id)
        model = ChatOpenAI(model=run.spec.brain_model, base_url=CLIPROXY_BASE_URL,
                           api_key=SecretStr("studio-read-only-not-a-credential"))
        return build_agent(run, checkpointer=None, model=model)

    return await asyncio.to_thread(build)


class ReadOnlyStudio:
    def __init__(self, app: ASGIApp) -> None:
        self.app = app

    async def __call__(self, scope: Scope, receive: Receive, send: Send) -> None:
        if scope["type"] == "websocket":
            await send({"type": "websocket.close", "code": 4403})
            return
        if scope["type"] == "http":
            method, path = scope["method"], scope["path"]
            read_post = path in READ_POST_PATHS or (
                path.startswith("/assistants/") and path.endswith("/versions")) or (
                path.startswith(CONTROL_PREFIX) and path.rsplit("/", 1)[-1] in CONTROL_ACTIONS)
            if method not in {"GET", "HEAD", "OPTIONS"} and not (method == "POST" and read_post):
                response = JSONResponse({"detail": "此 Studio 入口只读。执行与恢复请用 pilot1742 CLI。"}, status_code=403)
                await response(scope, receive, send)
                return
        await self.app(scope, receive, send)


def _production_open(state_root: Path, run_id: RunId) -> PilotRun:
    """与 `python -m pilot1742.executor run` 同一入口：先 run 锁，再在票级锁内复核持有。"""
    from .claims import ClaimStore
    from .delivery import RepoGateway
    from .executor import WORKERS_LOG, TicketExecutor, load_unit, open_v2_run
    from .github_app import WRITE_PERMISSIONS, AppIdentity, GitHubAppClient
    from .run import PSTACK_MODELS
    from .workunit import REPO

    key = os.environ.get("PILOT1742_APP_PRIVATE_KEY")
    if not key:
        raise ToolRefusal("Studio 控制面未配置 PILOT1742_APP_PRIVATE_KEY，不能启动或恢复 v2 run")
    unit, task = load_unit(state_root, run_id)
    identity = AppIdentity(5486877, 175617978, 1150335160, REPO, "newchanlun-deepagents-1742", Path(key))
    client = GitHubAppClient(identity, permissions=WRITE_PERMISSIONS)
    client.verify()
    executor = TicketExecutor(unit, RepoGateway(client), claims=ClaimStore(), workers_log=WORKERS_LOG)
    return open_v2_run(state_root, unit, task, executor, PSTACK_MODELS.read_text())


def _production_drive(run: PilotRun, resume: bool) -> int:
    from .cli import drive

    return drive(run, recheck_approval=resume)


@dataclass
class ControlPlane:
    """Studio 的受限操作。只推进已登记的 v2 run，与 CLI 共用 run 锁与票级锁；点击与请求正文都不是批准。"""

    state_root: Path
    open_run: Callable[[Path, RunId], PilotRun] = _production_open
    drive: Callable[[PilotRun, bool], int] = _production_drive
    threads: dict[str, threading.Thread] = field(default_factory=dict)

    def _existing_v2(self, run_id: RunId) -> PilotRun:
        run = PilotRun.open(self.state_root, run_id)
        if run.spec.v != 2:
            raise ToolRefusal("v1 run 在 Studio 中只读")
        return run

    def status(self, run_id: RunId) -> dict[str, Any]:
        run = self._existing_v2(run_id)
        thread = self.threads.get(run_id)
        return {"status": run.status(), "controlled_here": bool(thread and thread.is_alive())}

    def interrupt(self, run_id: RunId) -> dict[str, Any]:
        return {"interrupt_requested": self._existing_v2(run_id).request_interrupt("studio")}

    def start(self, run_id: RunId, *, resume: bool) -> dict[str, Any]:
        self._existing_v2(run_id)
        run = self.open_run(self.state_root, run_id)

        def body() -> None:
            try:
                run.trail("studio_drive", resume=resume)
                self.drive(run, resume)
            finally:
                run.release()

        thread = threading.Thread(target=body, name=f"pilot1742-{run_id}", daemon=True)
        self.threads[run_id] = thread
        thread.start()
        return {"started": run_id, "resume": resume}


CONTROL = ControlPlane(Path(os.environ.get("PILOT1742_STATE_ROOT", "/Volumes/AgentStorage/deepagents-exec")))


def _control_route(handler: Callable[[ControlPlane, RunId, dict[str, Any]], dict[str, Any]], method: str) -> Any:
    async def endpoint(request: Request) -> JSONResponse:
        try:
            run_id = parse_run_id(request.path_params["run_id"])
            body = await request.json() if method == "POST" and await request.body() else {}
            result = await asyncio.to_thread(handler, request.app.state.control, run_id, body)
        except Busy as error:
            return JSONResponse({"exit": 75, "detail": str(error)}, status_code=409)
        except FileNotFoundError as error:
            return JSONResponse({"detail": str(error)}, status_code=404)
        except (ToolRefusal, Halted, ValueError) as error:
            return JSONResponse({"detail": f"{type(error).__name__}: {error}"}, status_code=409)
        except Exception as error:
            from .executor import EXECUTOR_ERRORS

            if isinstance(error, EXECUTOR_ERRORS):
                return JSONResponse({"detail": f"{type(error).__name__}: {error}"}, status_code=409)
            raise
        return JSONResponse(result)

    return endpoint


ROUTES = [
    Route(CONTROL_PREFIX + "{run_id}", _control_route(lambda c, r, b: c.status(r), "GET"), methods=["GET"]),
    Route(CONTROL_PREFIX + "{run_id}/interrupt", _control_route(lambda c, r, b: c.interrupt(r), "POST"),
          methods=["POST"]),
    Route(CONTROL_PREFIX + "{run_id}/run", _control_route(
        lambda c, r, b: c.start(r, resume=b.get("resume") is True), "POST"), methods=["POST"]),
]


app = Starlette(routes=ROUTES, middleware=[
    Middleware(CORSMiddleware, allow_origins=["https://smith.langchain.com"],
               allow_methods=["GET", "POST", "OPTIONS"], allow_headers=["*"],
               allow_credentials=True, allow_private_network=True),
    Middleware(ReadOnlyStudio),
])
app.state.control = CONTROL
