import json
import sqlite3
from dataclasses import replace
from pathlib import Path

import pytest
from langchain_core.messages import AIMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_openai import ChatOpenAI
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command
from pydantic import SecretStr, ValidationError

from pilot1742 import agent as pilot_agent
from pilot1742.agent import LEGACY_CAVEAT, SubagentResponse, build_agent, build_tools
from pilot1742.domain import WorkerKey, WorkerReceipt
from pilot1742.run import ROLE_MODE, TICKET, ToolRefusal, parse_result

ALLOWED = {"run_worker", "commit_candidate", "check_gate", "run_status", "write_todos", "ls", "read_file", "glob", "grep"}


class ScriptedOpenAI(ChatOpenAI):
    script: list = []
    requests: list = []

    def bind_tools(self, tools, **kwargs):
        self.requests.append(sorted(t.name if hasattr(t, "name") else t["function"]["name"] for t in tools))
        return super().bind_tools(tools, **kwargs)

    def _generate(self, messages, stop=None, run_manager=None, **kwargs):
        reply = self.script.pop(0)
        return ChatResult(generations=[ChatGeneration(message=reply(messages) if callable(reply) else reply)])

    async def _agenerate(self, messages, stop=None, run_manager=None, **kwargs):
        return self._generate(messages, stop)


def scripted(*messages):
    model = ScriptedOpenAI(model="claude-opus-5-5", api_key=SecretStr("not-used"), base_url="http://127.0.0.1:9/v1")
    model.script, model.requests = list(messages), []
    return model


def call(name, **args):
    return AIMessage("", tool_calls=[{"name": name, "args": args, "id": f"call-{name}"}])


def test_request_tool_surface_has_no_execute_task_or_write_tools(env):
    run = env.create()
    model = scripted(call("read_file", file_path=pilot_agent.POTETO_MODE, limit=5), AIMessage("读到了"))
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


def receipt_for(env, role="author", **overrides):
    receipt = WorkerReceipt(WorkerKey("r1", role, env.base), "child", "completed", "gpt-6.1-sol",
                            env.base, env.base, ("demo/a.py",), "session", "/worker/log",
                            "VERDICT: PASS\n完整结论", ("发现甲", "发现乙"), ("核查甲", "核查乙"), None, "/missing")
    return replace(receipt, **overrides)


def worker_tool(run):
    return next(t for t in build_tools(run) if t.name == "run_worker")


@pytest.mark.parametrize("role", ["author", "review", "judge", "design"])
def test_worker_tool_returns_canonical_response_and_preserves_receipt(env, monkeypatch, role):
    run = env.create()
    receipt = receipt_for(env, role if role != "design" else "design-a")
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    monkeypatch.setattr(run, "run_design", lambda *_: [receipt, replace(receipt, key=WorkerKey("r1", "design-b", env.base))])
    result = json.loads(worker_tool(run).invoke({"role": role, "brief": "完整任务"}))
    assert result["ok"] is True
    receipts = result["receipts"] if role == "design" else [result["receipt"]]
    for row in receipts:
        response = row["response"]
        assert set(response) == {"conclusion", "evidence", "actions_taken", "confidence", "caveats"}
        assert SubagentResponse.model_validate(response).model_dump() == response
        assert response["conclusion"] == receipt.summary
        assert response["confidence"] == "low" and response["caveats"] == [LEGACY_CAVEAT]
        assert response["evidence"][:2] == ["核查甲", "核查乙"]
        assert all(any(f in item for item in response["evidence"]) for f in receipt.findings)
        assert response["actions_taken"] == ["宿主观察到修改路径：demo/a.py"]
        assert row["summary"] == receipt.summary and row["findings"] == list(receipt.findings)
        assert row["validation"] == list(receipt.validation)
        assert row["target_head"] == env.base and row["model"] == receipt.model
        assert row["output_dir"] == "/missing" and row["external_session_log"] == "/worker/log"


@pytest.mark.parametrize("args", [{"brief": "任务"}, {"role": "author"},
                                  {"role": "author", "brief": " \n\t"}, {"role": " ", "brief": "任务"}])
def test_worker_tool_refuses_missing_or_blank_inputs_before_dispatch(env, monkeypatch, args):
    run = env.create()
    dispatches = []
    monkeypatch.setattr(run, "run_worker", lambda *args: dispatches.append(args))
    with pytest.raises(ValidationError):
        worker_tool(run).invoke(args)
    assert dispatches == [] and env.claims() == []


@pytest.mark.parametrize("change", [{"findings": "不是列表"}, {"findings": (3,)},
                                     {"validation": (None,)}, {"summary": 42}])
def test_malformed_nonempty_receipt_is_diagnostic_not_success(env, monkeypatch, change):
    run = env.create()
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt_for(env, **change))
    result = json.loads(worker_tool(run).invoke({"role": "author", "brief": "任务"}))
    assert result["ok"] is False
    response = result["receipt"]["response"]
    assert response["confidence"] == "low"
    assert any("未通过校验" in c for c in response["caveats"])
    assert response["actions_taken"] == []


@pytest.mark.parametrize("summary", ["", " \n\t"])
def test_empty_receipt_returns_error(env, monkeypatch, summary):
    run = env.create()
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt_for(env, summary=summary))
    result = json.loads(worker_tool(run).invoke({"role": "author", "brief": "任务"}))
    assert result["ok"] is False and "EmptyWorkerResponse" in result
    assert "receipt" not in result


@pytest.mark.parametrize("status", ["failed", "cancelled"])
def test_failed_and_cancelled_receipts_keep_failure_caveats(env, monkeypatch, status):
    run = env.create()
    receipt = receipt_for(env, status=status, failure_reason="实际失败原因")
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    result = json.loads(worker_tool(run).invoke({"role": "review", "brief": "任务"}))
    assert result["ok"] is False
    assert "实际失败原因" in result["receipt"]["response"]["caveats"]
    assert any(status in c for c in result["receipt"]["response"]["caveats"])


def null_result_receipt(env, role, status):
    output = env.home / f"{role}-{status}"
    output.mkdir()
    data = {
        "ticket": TICKET, "status": status, "mode": ROLE_MODE[role], "model": "gpt-6.1-sol",
        "expected_head": env.base, "checkout_after": None, "external_session_id": None,
        "external_session_log": None, "result": None, "failure_reason": f"原样原因：{status}\n工蜂未返回结果",
        "run_id": "child",
    }
    (output / "result.json").write_text(json.dumps(data))
    return parse_result(WorkerKey("r1", role, env.base), data, output)


@pytest.mark.parametrize("role", ["author", "review", "judge"])
@pytest.mark.parametrize("status", ["failed", "cancelled"])
def test_null_failed_result_returns_terminal_diagnostic_after_parse_result(env, monkeypatch, role, status):
    run = env.create()
    receipt = null_result_receipt(env, role, status)
    assert receipt.summary == ""
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    result = json.loads(worker_tool(run).invoke({"role": role, "brief": "读取真实失败回执"}))
    assert result["ok"] is False and "EmptyWorkerResponse" not in result
    row = result["receipt"]
    response = SubagentResponse.model_validate(row["response"])
    assert status in response.conclusion
    assert response.confidence == "low"
    assert LEGACY_CAVEAT in response.caveats and receipt.failure_reason in response.caveats
    assert any(status in caveat for caveat in response.caveats)
    assert row["status"] == status and row["failure_reason"] == receipt.failure_reason
    assert row["summary"] == "" and row["target_head"] == env.base
    assert row["output_dir"] == receipt.output_dir and row["model"] == receipt.model
    assert row["external_session_id"] is None and row["external_session_log"] is None
    assert json.loads((Path(receipt.output_dir) / "result.json").read_text())["result"] is None
    assert env.claims() == []


@pytest.mark.parametrize("failed_index", [0, 1])
@pytest.mark.parametrize("status", ["failed", "cancelled"])
def test_design_retains_both_slots_when_null_result_fails(env, monkeypatch, failed_index, status):
    run = env.create()
    roles = ["design-a", "design-b"]
    receipts = [receipt_for(env, role) for role in roles]
    receipts[failed_index] = null_result_receipt(env, roles[failed_index], status)
    monkeypatch.setattr(run, "run_design", lambda *_: receipts)
    result = json.loads(worker_tool(run).invoke({"role": "design", "brief": "保留两份设计"}))
    assert result["ok"] is False
    assert [row["role"] for row in result["receipts"]] == roles
    for index, row in enumerate(result["receipts"]):
        response = SubagentResponse.model_validate(row["response"])
        if index == failed_index:
            assert row["status"] == status and status in response.conclusion
            assert receipts[index].failure_reason in response.caveats
        else:
            assert row["status"] == "completed" and row["response_validated"] is True
            assert response.conclusion == receipts[index].summary


@pytest.mark.parametrize("empty_index", [0, 1])
def test_design_retains_other_slot_when_completed_result_is_empty(env, monkeypatch, empty_index):
    run = env.create()
    roles = ["design-a", "design-b"]
    receipts = [receipt_for(env, role) for role in roles]
    receipts[empty_index] = replace(receipts[empty_index], summary="")
    monkeypatch.setattr(run, "run_design", lambda *_: receipts)
    result = json.loads(worker_tool(run).invoke({"role": "design", "brief": "保留另一份设计"}))
    assert result["ok"] is False
    assert [row["role"] for row in result["receipts"]] == roles
    empty = result["receipts"][empty_index]
    assert "EmptyWorkerResponse" in empty and "response" not in empty
    assert empty["summary"] == "" and empty["target_head"] == env.base
    assert empty["model"] == receipts[empty_index].model
    assert empty["output_dir"] == receipts[empty_index].output_dir
    other = result["receipts"][1 - empty_index]
    assert other["status"] == "completed"
    assert other["response"]["conclusion"] == receipts[1 - empty_index].summary


def test_canonical_schema_rejects_extra_fields_and_wrong_types():
    canonical = dict(conclusion="结论", evidence=[], actions_taken=[], confidence="low", caveats=[])
    for extra in ({"extra": True}, {"evidence": "不是列表"}, {"evidence": [3]}, {"confidence": "certain"}):
        with pytest.raises(ValidationError):
            SubagentResponse.model_validate({**canonical, **extra})
    for field in canonical:
        with pytest.raises(ValidationError):
            SubagentResponse.model_validate({k: v for k, v in canonical.items() if k != field})


@pytest.mark.parametrize("change", [{"findings": "不是列表"}, {"validation": [3]}, {"summary": 42}, {"extra": True}])
def test_original_result_fields_are_checked_before_receipt_coercion(env, change):
    run = env.create()
    receipt = run.run_worker("author", "任务")
    path = Path(receipt.output_dir) / "result.json"
    original = json.loads(path.read_text())
    original["result"].update(change)
    path.write_text(json.dumps(original))
    result = json.loads(worker_tool(run).invoke({"role": "author", "brief": "收养"}))
    assert result["ok"] is False
    assert any("未通过校验" in c for c in result["receipt"]["response"]["caveats"])
    assert result["receipt"]["unvalidated_output"] == original["result"]
    assert json.loads(path.read_text()) == original
    assert len(env.claims()) == 1


@pytest.mark.parametrize("role", ["author", "design"])
def test_scripted_main_loop_consumes_each_response_caveat(env, monkeypatch, role):
    run = env.create()
    receipt = receipt_for(env, failure_reason="本次未检查远端")
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    monkeypatch.setattr(run, "run_design", lambda *_: [receipt, receipt])

    def consume(messages):
        reply = next(m for m in reversed(messages) if getattr(m, "name", None) == "run_worker")
        payload = json.loads(reply.content)
        receipts = payload["receipts"] if role == "design" else [payload["receipt"]]
        return AIMessage("\n".join(r["response"]["conclusion"] + "\n" + "\n".join(r["response"]["caveats"])
                                   for r in receipts))

    model = scripted(call("run_worker", role=role, brief="任务"), consume)
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    result = agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": "r1"}}, durability="sync")
    expected = 2 if role == "design" else 1
    assert result["messages"][-1].content.count(LEGACY_CAVEAT) == expected
    assert result["messages"][-1].content.count("本次未检查远端") == expected
    assert all(set(request) == ALLOWED for request in model.requests)


def test_worker_input_keeps_nonblank_brief_unchanged_and_does_not_accept_host_fields(env, monkeypatch):
    run = env.create()
    dispatches = []

    def dispatch(role, brief):
        dispatches.append((role, brief))
        return receipt_for(env, summary="BASE: 候选 2\n" + "完整设计包" * 1000)

    monkeypatch.setattr(run, "run_worker", dispatch)
    result = json.loads(worker_tool(run).invoke({"role": "judge", "brief": " \n原样问题\n "}))
    assert dispatches == [("judge", " \n原样问题\n ")]
    assert result["receipt"]["response"]["conclusion"] == "BASE: 候选 2\n" + "完整设计包" * 1000
    with pytest.raises(ValidationError):
        worker_tool(run).invoke({"role": "judge", "brief": "问题", "model": "伪造宿主字段"})
    assert len(dispatches) == 1
