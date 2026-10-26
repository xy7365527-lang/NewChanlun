import json
import sqlite3

import pytest
from langchain_core.messages import AIMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_openai import ChatOpenAI
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command
from pydantic import SecretStr

from pilot1742.agent import POTETO_MODE, build_agent
from pilot1742.run import ToolRefusal

ALLOWED = {"run_worker", "commit_candidate", "check_gate", "run_status", "write_todos", "ls", "read_file", "glob", "grep"}


class ScriptedOpenAI(ChatOpenAI):
    script: list = []
    requests: list = []

    def bind_tools(self, tools, **kwargs):
        self.requests.append(sorted(t.name if hasattr(t, "name") else t["function"]["name"] for t in tools))
        return super().bind_tools(tools, **kwargs)

    def _generate(self, messages, stop=None, run_manager=None, **kwargs):
        return ChatResult(generations=[ChatGeneration(message=self.script.pop(0))])


def scripted(*messages):
    model = ScriptedOpenAI(model="claude-opus-5-5", api_key=SecretStr("not-used"), base_url="http://127.0.0.1:9/v1")
    model.script, model.requests = list(messages), []
    return model


def call(name, **args):
    return AIMessage("", tool_calls=[{"name": name, "args": args, "id": f"call-{name}"}])


def test_request_tool_surface_has_no_execute_task_or_write_tools(env):
    run = env.create()
    model = scripted(call("read_file", file_path=POTETO_MODE, limit=5), AIMessage("读到了"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    result = agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": "r1"}}, durability="sync")
    assert set(model.requests[0]) == ALLOWED
    assert all(set(r) == ALLOWED for r in model.requests)
    read = [m for m in result["messages"] if getattr(m, "name", None) == "read_file"][0]
    assert "Poteto mode" in read.content or "pstack-upstream-poteto-mode" in read.content


def test_gate_pauses_as_interrupt_and_resumes_from_checkpoint_after_ledger_entry(env):
    run = env.create()
    env.control({"author": {"write": {"demo/a.py": "x = 1\n"}}, "review": {"summary": "VERDICT: PASS\n可以"}})
    run.run_worker("author", "实现")
    head = run.commit_candidate().commit
    run.run_worker("review", "审")
    config = {"configurable": {"thread_id": "r1"}}
    db = run.dir / "graph.sqlite"
    first = build_agent(run, SqliteSaver(sqlite3.connect(db, check_same_thread=False)), scripted(call("check_gate")))
    first.invoke({"messages": [("user", "查门")]}, config, durability="sync")
    paused = first.get_state(config)
    assert paused.tasks and paused.tasks[0].interrupts
    assert paused.tasks[0].interrupts[0].value["awaiting_approval"]["reasons"] == ["no_approval"]

    row = {"ticket": 1742, "scope": "pilot_acceptance", "head": head, "decision": "approve",
           "source": {"kind": "user_message", "reference": "pytest-registered", "registered_by": "pytest"}}
    (run.dir / "approvals.jsonl").write_text(json.dumps(row) + "\n")
    second = build_agent(run, SqliteSaver(sqlite3.connect(db, check_same_thread=False)), scripted(AIMessage("门已过")))
    result = second.invoke(Command(resume={"recheck": True}), config, durability="sync")
    gate_reply = [m for m in result["messages"] if getattr(m, "name", None) == "check_gate"][-1]
    assert json.loads(gate_reply.content)["decision"] == "approved"
    assert not second.get_state(config).next


def test_run_created_before_the_design_channel_still_builds_and_refuses_design(env):
    run = env.create()
    spec = json.loads((run.dir / "run.json").read_text())
    for field in ("design_models", "judge_model"):
        spec.pop(field)
    (run.dir / "run.json").write_text(json.dumps(spec))
    run.release()
    old = env.reopen()
    agent = build_agent(old, SqliteSaver(sqlite3.connect(old.dir / "graph.sqlite", check_same_thread=False)),
                        scripted(AIMessage("完")))
    agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": "r1"}}, durability="sync")
    with pytest.raises(ToolRefusal, match="设计通道"):
        old.run_design("x")
    assert env.claims() == []
