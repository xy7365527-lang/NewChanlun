from __future__ import annotations

import asyncio
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
from starlette.types import ASGIApp, Receive, Scope, Send

from .agent import CLIPROXY_BASE_URL, build_agent, disable_external_tracing
from .domain import RunId
from .run import DEFAULT_HOME, PilotRun


@dataclass(frozen=True)
class StudioTarget:
    home: Path
    run_id: RunId


TARGET = StudioTarget(DEFAULT_HOME, RunId("real4"))
READ_POST_PATHS = frozenset({"/assistants/search", "/assistants/count"})


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
                path.startswith("/assistants/") and path.endswith("/versions"))
            if method not in {"GET", "HEAD", "OPTIONS"} and not (method == "POST" and read_post):
                response = JSONResponse({"detail": "此 Studio 入口只读。执行与恢复请用 pilot1742 CLI。"}, status_code=403)
                await response(scope, receive, send)
                return
        await self.app(scope, receive, send)


app = Starlette(middleware=[
    Middleware(CORSMiddleware, allow_origins=["https://smith.langchain.com"],
               allow_methods=["GET", "POST", "OPTIONS"], allow_headers=["*"],
               allow_credentials=True, allow_private_network=True),
    Middleware(ReadOnlyStudio),
])
