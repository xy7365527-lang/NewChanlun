import json
import sqlite3

import pytest
from approval_signing.protocol import ApprovalRequest, Decision
from langchain_core.messages import AIMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from langgraph.types import Command
from signing_fixture import Signer
from test_agent import call, scripted
from v2env import TICKET, SimulatedCrash, V2Env, crash_at

from pilot1742.agent import build_agent
from pilot1742.approval import SpecAuthorization
from pilot1742.executor import AdmissionRefused
from pilot1742.workunit import Takeover, WorkUnit

BRANCH = f"codex/deepagents/t{TICKET}"


@pytest.fixture
def v2(tmp_path):
    return V2Env(tmp_path)


@pytest.fixture
def signer(tmp_path, v2, monkeypatch):
    signer = Signer(tmp_path / "approval-receipts")
    v2.approvals = signer.protect(monkeypatch)
    return signer


def request_file(run):
    return run.dir / "approvals" / "candidate_head.request"


def test_candidate_gate_waits_then_publishes_only_after_published_approval(v2, signer):
    run = v2.run(v2.register(candidate_approval=True))
    head = v2.to_pass(run)
    before = v2.writes()
    waiting = run.publish_candidate()
    assert waiting["decision"] == "awaiting_approval" and waiting["approval"]["status"] == "awaiting_human"
    assert waiting["approval"]["request_path"] == str(request_file(run)) and v2.writes() == before
    request = ApprovalRequest.parse(request_file(run).read_bytes())
    assert (request.target.encode(), request.run_id, request.claim_generation) == (f"commit:{head}", run.spec.run_id, 1)
    assert run.status()["approvals"][0]["status"] == "awaiting_human"
    signer.publish(request_file(run), Decision.APPROVE)
    published = run.publish_candidate()
    assert published["ok"] and v2.github.ref(BRANCH) == head


def test_published_reject_between_effects_stops_the_next_write(v2, signer):
    unit = v2.register(candidate_approval=True)
    run = v2.run(unit, crash=crash_at("after_push_before_receipt"))
    head = v2.to_pass(run)
    run.publish_candidate()
    signer.publish(request_file(run), Decision.APPROVE)
    with pytest.raises(SimulatedCrash):
        run.publish_candidate()
    run.release()
    signer.publish(request_file(run), Decision.REJECT)
    resumed = v2.run(unit)
    result = resumed.publish_candidate()
    assert result["decision"] == "awaiting_approval" and result["approval"]["status"] == "rejected"
    assert v2.github.ref(BRANCH) == head and v2.github.pulls == {} and head not in v2.github.statuses


def test_new_head_reissues_and_the_old_approval_releases_nothing(v2, signer):
    run = v2.run(v2.register(candidate_approval=True))
    head = v2.to_pass(run)
    run.publish_candidate()
    signer.publish(request_file(run), Decision.APPROVE)
    drifted = run.executor.candidate_approval(run.dir, run.generation, "f" * 40)
    assert drifted is not None and drifted.status == "awaiting_human"
    assert ApprovalRequest.parse(request_file(run).read_bytes()).target.encode() == "commit:" + "f" * 40
    result = run.publish_candidate()
    assert result["decision"] == "awaiting_approval" and result["approval"]["status"] == "awaiting_human"
    assert ApprovalRequest.parse(request_file(run).read_bytes()).target.encode() == f"commit:{head}"
    assert v2.github.ref(BRANCH) is None


def test_resume_payload_claiming_approval_is_ignored_by_the_graph(v2, signer):
    run = v2.run(v2.register(candidate_approval=True))
    v2.to_pass(run)
    model = scripted(call("publish_candidate"), AIMessage("已交付"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    config = {"configurable": {"thread_id": run.spec.run_id}}
    agent.invoke({"messages": [("user", "开始")]}, config, durability="sync")
    assert agent.get_state(config).next
    agent.invoke(Command(resume={"approved": True, "decision": "approve"}), config, durability="sync")
    assert agent.get_state(config).next and v2.github.ref(BRANCH) is None
    signer.publish(request_file(run), Decision.APPROVE)
    result = agent.invoke(Command(resume={}), config, durability="sync")
    reply = [m for m in result["messages"] if getattr(m, "name", None) == "publish_candidate"][-1]
    assert json.loads(reply.content)["ok"] and not agent.get_state(config).next


def test_takeover_with_valid_spec_signature_is_still_refused_at_the_barrier(v2, signer):
    v2.github.add_issue(1340, body="SPEC r3\n")
    unit = WorkUnit(TICKET, 1323, v2.base, ("demo/",), "feature", False, Takeover({}, SpecAuthorization(1340)),
                    f"t{TICKET}-1")
    run_dir = v2.home / "runs" / unit.run_id
    with pytest.raises(AdmissionRefused, match="awaiting_human"):
        v2.executor(unit).claim(run_dir)
    signer.publish(run_dir / "approvals" / "spec_authorization.request", Decision.APPROVE)
    with pytest.raises(AdmissionRefused, match="takeover_barrier_unimplemented"):
        v2.executor(unit).claim(run_dir)
    v2.github.issues[1340]["body"] = "SPEC r3 已改\n"
    with pytest.raises(AdmissionRefused, match="awaiting_human"):
        v2.executor(unit).claim(run_dir)
    assert v2.writes() == [] and not (v2.claims_root / f"{TICKET}.json").exists()


def test_operator_interrupt_stops_at_the_next_tool_without_killing_workers(v2):
    run = v2.run(v2.register())
    v2.control({"author": {"write": {"demo/a.py": "x = 1\n"}}})
    model = scripted(call("run_worker", role="author", brief="实现"), call("commit_candidate"), AIMessage("完"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    config = {"configurable": {"thread_id": run.spec.run_id}}
    run.request_interrupt("pytest")
    agent.invoke({"messages": [("user", "开始")]}, config, durability="sync")
    assert agent.get_state(config).next and run.status()["workers"] == {}
    agent.invoke(Command(resume={}), config, durability="sync")
    assert agent.get_state(config).next
    run.clear_interrupt()
    agent.invoke(Command(resume={}), config, durability="sync")
    assert not agent.get_state(config).next and run.status()["commits"]
