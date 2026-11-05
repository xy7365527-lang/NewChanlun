import json
from dataclasses import replace

import pytest
from test_agent import receipt_for, worker_tool

from pilot1742.domain import (
    DEEPAGENTS_OUTPUT_PROTOCOL,
    LEGACY_CAVEAT,
    WorkerKey,
    WorkerSelfReport,
    parse_self_report,
)
from pilot1742.run import Halted, parse_result


@pytest.mark.parametrize("confidence", ["high", "medium", "low"])
@pytest.mark.parametrize("caveats", [[], ["不能核验生产", "保留原句\n第二行"]])
@pytest.mark.parametrize("role", ["author", "review", "judge", "design"])
def test_model_self_report_reaches_main_unchanged(env, monkeypatch, confidence, caveats, role):
    run = env.create()
    receipt = receipt_for(
        env, role if role != "design" else "design-a",
        output_protocol=DEEPAGENTS_OUTPUT_PROTOCOL,
        self_report=WorkerSelfReport(confidence, tuple(caveats)),
    )
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    monkeypatch.setattr(run, "run_design", lambda *_: [
        receipt, replace(receipt, key=WorkerKey("r1", "design-b", env.base))])
    result = json.loads(worker_tool(run).invoke({"role": role, "brief": "任务"}))
    assert result["ok"] is True
    rows = result["receipts"] if role == "design" else [result["receipt"]]
    for row in rows:
        assert row["response"]["confidence"] == confidence
        assert row["response"]["caveats"] == caveats
        assert LEGACY_CAVEAT not in row["response"]["caveats"]
    run.release()


@pytest.mark.parametrize("report", [
    {}, {"confidence": "high"}, {"caveats": []},
    {"confidence": None, "caveats": []}, {"confidence": "certain", "caveats": []},
    {"confidence": "low", "caveats": None}, {"confidence": "low", "caveats": "局限"},
    {"confidence": "low", "caveats": [1]}, {"confidence": ["high"], "caveats": []},
])
def test_invalid_or_incomplete_self_report_is_not_legacy(report):
    with pytest.raises(ValueError, match="自报"):
        parse_self_report(report, DEEPAGENTS_OUTPUT_PROTOCOL)


def test_legacy_output_protocol_stays_four_keys():
    assert parse_self_report({}, "legacy") is None
    with pytest.raises(ValueError, match="四键"):
        parse_self_report({"confidence": "high", "caveats": []}, "legacy")
    with pytest.raises(ValueError, match="未知"):
        parse_self_report({}, "unknown")


def test_new_dispatch_binds_protocol_through_launcher_and_resume(env):
    env.control({"author": {"confidence": "high", "caveats": ["真实未核项"]}})
    run = env.create()
    receipt = run.run_worker("author", "实现")
    intent = json.loads((run.dir / "intents" / f"{receipt.key.name}.json").read_text())
    raw = json.loads((run._key_dir(receipt.key) / "result.json").read_text())
    assert intent["output_protocol"] == raw["output_protocol"] == DEEPAGENTS_OUTPUT_PROTOCOL
    assert receipt.self_report == WorkerSelfReport("high", ("真实未核项",))
    run.release()
    same = env.reopen()
    assert same.run_worker("author", "重连").self_report == receipt.self_report
    assert len(env.claims()) == 1
    same.release()


def test_legacy_pending_intent_is_not_upgraded_on_resume(env):
    run = env.create()
    key = WorkerKey(run.spec.run_id, "author", env.base)
    run._prepare_dispatch(key, "旧任务", run.facts())
    path = run.dir / "intents" / f"{key.name}.json"
    intent = json.loads(path.read_text())
    intent.pop("output_protocol")
    path.write_text(json.dumps(intent))
    prompt = run.dir / "prompts" / f"{key.name}.md"
    prompt.write_text("旧四键任务")
    run.release()
    same = env.reopen()
    receipt = same.run_worker("author", "不应覆盖旧任务")
    assert receipt.output_protocol == "legacy" and receipt.self_report is None
    assert prompt.read_text() == "旧四键任务"
    assert "output_protocol" not in json.loads(path.read_text())
    same.release()
    resumed = env.reopen()
    result = json.loads(worker_tool(resumed).invoke({"role": "author", "brief": "收养旧回执"}))
    assert result["receipt"]["response"]["caveats"] == [LEGACY_CAVEAT]
    assert len(env.claims()) == 1
    resumed.release()


def test_stripping_new_receipt_protocol_cannot_downgrade_frozen_intent(env):
    run = env.create()
    receipt = run.run_worker("author", "实现")
    path = run._key_dir(receipt.key) / "result.json"
    raw = json.loads(path.read_text())
    raw.pop("output_protocol")
    raw["result"].pop("confidence")
    raw["result"].pop("caveats")
    path.write_text(json.dumps(raw))
    with pytest.raises(Halted, match="输出协议"):
        run.run_worker("author", "恢复")
    assert len(env.claims()) == 1
    run.release()


@pytest.mark.parametrize("status", ["failed", "cancelled"])
def test_new_protocol_null_result_preserves_terminal_diagnostic(env, monkeypatch, status):
    key = WorkerKey("r1", "author", env.base)
    receipt = parse_result(key, {
        "ticket": 1742, "status": status, "expected_head": env.base, "mode": "execute",
        "model": "gpt-6.1-sol", "output_protocol": DEEPAGENTS_OUTPUT_PROTOCOL,
        "result": None, "failure_reason": "真实原因",
    }, env.home)
    run = env.create()
    monkeypatch.setattr(run, "run_worker", lambda *_: receipt)
    result = json.loads(worker_tool(run).invoke({"role": "author", "brief": "诊断"}))
    assert result["ok"] is False
    response = result["receipt"]["response"]
    assert response["confidence"] == "low" and "真实原因" in response["caveats"]
    assert LEGACY_CAVEAT not in response["caveats"]
    run.release()
