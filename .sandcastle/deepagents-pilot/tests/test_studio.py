import asyncio
import json
from pathlib import Path
from unittest.mock import patch

import pytest
from langchain_openai import ChatOpenAI
from pilot1742 import studio
from pydantic import SecretStr
from starlette.applications import Starlette
from starlette.middleware import Middleware
from starlette.responses import JSONResponse
from starlette.routing import Route
from starlette.testclient import TestClient


def test_factory_uses_frozen_run_without_acquiring_or_writing(env, monkeypatch):
    run = env.create()
    monkeypatch.setattr(studio, "TARGET", studio.StudioTarget(run.home, run.spec.run_id))
    model = ChatOpenAI(model="claude-opus-5-5", api_key=SecretStr("not-used"))
    before = {str(p): p.read_bytes() for p in run.dir.rglob("*") if p.is_file()}
    with patch("pilot1742.agent.managed_cliproxy_key", side_effect=AssertionError("Studio 不得取真实key")), patch.object(
        studio.PilotRun, "acquire", side_effect=AssertionError("Studio 不得认领执行锁")
    ), patch.object(model.__class__, "invoke", side_effect=AssertionError("Studio 不得调用模型")):
        graph = asyncio.run(studio.graph({"configurable": {"run_id": "other"}}))
    assert {"model", "tools", "SkillsMiddleware.before_agent"} <= set(graph.get_graph().nodes)
    assert graph.checkpointer is None
    assert {str(p): p.read_bytes() for p in run.dir.rglob("*") if p.is_file()} == before


@pytest.mark.parametrize("method,path,allowed", [
    ("GET", "/assistants/pilot1742/graph", True),
    ("GET", "/assistants/pilot1742/schemas", True),
    ("POST", "/assistants/search", True),
    ("POST", "/assistants/count", True),
    ("POST", "/assistants/pilot1742/versions", True),
    ("POST", "/assistants", False),
    ("PATCH", "/assistants/pilot1742", False),
    ("DELETE", "/assistants/pilot1742", False),
    ("POST", "/assistants/pilot1742/latest", False),
    ("POST", "/threads/x/state", False),
    ("POST", "/threads/x/runs", False),
    ("POST", "/runs", False),
    ("PUT", "/store/items", False),
])
def test_http_boundary_blocks_writes(method, path, allowed):
    async def endpoint(request):
        return JSONResponse({"reached": True})

    app = Starlette(routes=[Route(path, endpoint, methods=[method])],
                    middleware=[Middleware(studio.ReadOnlyStudio)])
    response = TestClient(app).request(method, path)
    assert response.status_code == (200 if allowed else 403)


def test_official_config_disables_execution_transports():
    config = json.loads((Path(__file__).parents[1] / "langgraph.json").read_text())
    for name in ("runs", "threads", "store", "ui", "mcp", "a2a", "webhooks"):
        assert config["http"][f"disable_{name}"] is True


def test_browser_private_network_preflight():
    app = studio.app
    response = TestClient(app).options("/assistants/search", headers={
        "Origin": "https://smith.langchain.com",
        "Access-Control-Request-Method": "POST",
        "Access-Control-Request-Headers": "content-type,x-api-key",
        "Access-Control-Request-Private-Network": "true",
    })
    assert response.status_code == 200
    assert response.headers["Access-Control-Allow-Origin"] == "https://smith.langchain.com"
    assert response.headers["Access-Control-Allow-Private-Network"] == "true"
    denied = TestClient(app).options("/assistants/search", headers={
        "Origin": "https://other.example",
        "Access-Control-Request-Method": "POST",
    })
    assert denied.status_code == 400
