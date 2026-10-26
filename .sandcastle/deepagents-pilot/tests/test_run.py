import json
import os
import signal
import time

import pytest
from conftest import DESIGN_TASK, MODELS, git

from pilot1742.domain import Approved, Awaiting, Refused
from pilot1742.run import Busy, Halted, ToolRefusal, _ps_rows, supervisor_alive

DEMO = {"demo/a.py": "def f():\n    return 1\n"}


def ledger(run, head, decision="approve"):
    row = {"ticket": 1742, "scope": "pilot_acceptance", "head": head, "decision": decision,
           "source": {"kind": "user_message", "reference": "pytest-registered", "registered_by": "pytest"}}
    with open(run.dir / "approvals.jsonl", "a") as fh:
        fh.write(json.dumps(row) + "\n")


def wait_for(predicate, timeout=20):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if predicate():
            return
        time.sleep(0.1)
    raise AssertionError("条件未在时限内出现")


def test_block_rework_pass_then_gate_binds_exact_head(env):
    run = env.create()
    env.control({"author": {"write": DEMO},
                 "review": {"summary": "VERDICT: BLOCK\n缺测试", "findings": ["缺测试"]}})
    assert run.run_worker("author", "实现").status == "completed"
    first = run.commit_candidate()
    assert first.intent.parent == env.base and not first.recovered
    with pytest.raises(ToolRefusal):
        run.run_worker("author", "在没有 BLOCK 的候选上重派")
    run.run_worker("review", "审")
    assert run.status()["phase"] == "rework"
    env.control({"author": {"write": {"demo/test_a.py": "assert True\n"}},
                 "review": {"summaries": {}, "summary": "VERDICT: PASS\n可以"}})
    run.run_worker("author", "按 findings 返修")
    second = run.commit_candidate()
    assert second.intent.parent == first.commit
    run.run_worker("review", "复审")
    assert run.status()["phase"] == "gate"
    assert isinstance(run.gate(), Awaiting)
    ledger(run, first.commit)
    decision = run.gate()
    assert isinstance(decision, Awaiting) and decision.reasons == ("approval_for_other_head",)
    ledger(run, second.commit)
    assert isinstance(run.gate(), Approved)
    ledger(run, second.commit, "reject")
    assert run.gate().reasons == ("revoked",)
    assert len(env.claims()) == 4
    assert git(env.source, "rev-list", "--count", f"{env.base}..{second.commit}") == "2"


def test_repeated_tool_calls_adopt_instead_of_redispatching(env):
    run = env.create()
    env.control({"author": {"write": DEMO}, "review": {"summary": "VERDICT: PASS"}})
    a = run.run_worker("author", "第一次措辞")
    b = run.run_worker("author", "重放时换了措辞")
    assert a == b
    c1 = run.commit_candidate()
    c2 = run.commit_candidate()
    assert c1 == c2
    run.run_worker("review", "x")
    run.run_worker("review", "y")
    assert env.claims() == [f"author-{env.base}", f"review-{c1.commit}"]


def test_controller_sigkill_while_worker_runs_then_resume_adopts_same_dispatch(env):
    env.create().release()
    env.control({"author": {"sleep": 4, "write": DEMO}})
    out = env.home / "runs" / "r1" / "workers" / f"author-{env.base}"
    driver = env.driver("author")
    wait_for(lambda: (out / "state.json").exists())
    os.kill(driver.pid, signal.SIGKILL)
    driver.wait()
    assert supervisor_alive(out, _ps_rows())
    run = env.reopen()
    receipt = run.run_worker("author", "恢复后再次请求")
    assert receipt.status == "completed"
    assert env.claims() == [f"author-{env.base}"]


def test_crash_after_commit_before_receipt_recovers_the_same_commit(env):
    run = env.create()
    env.control({"author": {"write": DEMO}})
    run.run_worker("author", "实现")
    run.release()
    driver = env.driver("commit", crash="after_commit_before_receipt")
    assert driver.wait() == -signal.SIGKILL
    head_after_crash = git(env.home / "checkouts" / "r1" / "author", "rev-parse", "HEAD")
    assert head_after_crash != env.base
    assert not list((env.home / "runs" / "r1" / "commits").glob("*.receipt.json"))
    window = env.reopen()
    status = window.status()
    assert status["conflict"] is None and status["phase"] == "needs_commit"
    assert status["recorded_head"] == env.base and status["observed_head"] == head_after_crash
    receipt = window.commit_candidate()
    assert receipt.recovered and receipt.commit == head_after_crash
    assert git(env.source, "rev-list", "--count", f"{env.base}..{receipt.commit}") == "1"


def test_crash_after_intent_before_spawn_dispatches_same_key_once(env):
    run = env.create()
    run.release()
    env.control({"author": {"write": DEMO}})
    assert env.driver("author", crash="after_intent_before_spawn").wait() == -signal.SIGKILL
    assert env.claims() == []
    assert env.reopen().run_worker("author", "恢复").status == "completed"
    assert env.claims() == [f"author-{env.base}"]


def test_crash_after_spawn_before_launch_record_adopts_the_detached_worker(env):
    run = env.create()
    run.release()
    env.control({"author": {"sleep": 2, "write": DEMO}})
    assert env.driver("author", crash="after_spawn_before_launch_record").wait() == -signal.SIGKILL
    assert not list((env.home / "runs" / "r1" / "launch").glob("*.json"))
    assert env.reopen().run_worker("author", "恢复").status == "completed"
    assert env.claims() == [f"author-{env.base}"]


def test_claimed_dir_without_result_or_supervisor_halts_without_new_dispatch(env):
    run = env.create()
    out = run.dir / "workers" / f"author-{env.base}"
    out.mkdir(parents=True)
    (out / "state.json").write_text(json.dumps({"status": "running", "process_pid": 999999}))
    with pytest.raises(Halted):
        run.run_worker("author", "x")
    assert env.claims() == []
    assert run.status()["phase"] == "halted"


def test_second_controller_is_refused_before_any_side_effect(env):
    env.create()
    with pytest.raises(Busy):
        env.reopen()


def test_out_of_scope_change_is_never_committed(env):
    run = env.create()
    env.control({"author": {"write": {**DEMO, "README": "changed\n"}}})
    run.run_worker("author", "x")
    with pytest.raises(Halted):
        run.commit_candidate()
    assert git(env.home / "checkouts" / "r1" / "author", "rev-parse", "HEAD") == env.base


def test_same_family_models_are_rejected_at_creation(env):
    with pytest.raises(ValueError):
        env.create(models=MODELS.replace("judgment and prose: claude-opus-5-5", "judgment and prose: gpt-6.1-sol"))
    with pytest.raises(ValueError, match="architect runners"):
        env.create(models=MODELS.replace("architect runners: claude-opus-5-5, gpt-6.1-sol", "architect runners: gpt-6.1-sol"))
    with pytest.raises(ValueError, match="缺少角色行"):
        env.create(models=MODELS.replace("arena cross-judge pool", "other pool"))


def approved_run(env):
    run = env.create()
    env.control({"author": {"write": DEMO}, "review": {"summary": "VERDICT: PASS\n可以"}})
    run.run_worker("author", "实现")
    head = run.commit_candidate().commit
    run.run_worker("review", "审")
    ledger(run, head)
    assert isinstance(run.gate(), Approved)
    return run, head


def test_external_commit_on_author_checkout_revokes_the_old_approval(env):
    run, head = approved_run(env)
    author = env.home / "checkouts" / "r1" / "author"
    (author / "demo" / "extra.py").write_text("x = 2\n")
    git(author, "add", "demo/extra.py")
    git(author, "commit", "-qm", "外部提交")
    observed = git(author, "rev-parse", "HEAD")
    decision = run.gate()
    assert isinstance(decision, Refused) and decision.reasons == ("candidate_head_drift",)
    status = run.status()
    assert (status["recorded_head"], status["observed_head"]) == (head, observed)
    assert status["conflict"] == "candidate_head_drift" and status["phase"] == "conflict"
    assert status["gate"]["decision"] == "refused"
    claims = env.claims()
    with pytest.raises(ToolRefusal, match="candidate_head_drift"):
        run.run_worker("author", "在外部提交上继续")
    assert env.claims() == claims


def test_author_branch_switch_refuses_gate_and_commit(env):
    run, head = approved_run(env)
    author = env.home / "checkouts" / "r1" / "author"
    git(author, "switch", "-q", "-c", "codex/someone-else")
    assert run.gate().reasons == ("author_branch_changed",)
    assert run.status()["observed_branch"] == "codex/someone-else"


DESIGNS = {"design-a": {"summary": "设计包甲：正则加遮蔽", "findings": ["拒绝状态机"]},
           "design-b": {"summary": "设计包乙：逐字符状态机", "findings": ["拒绝正则"]},
           "judge": {"summary": "BASE: 候选 1\n更简单"}}


def test_design_arena_runs_two_fixed_readonly_slots_then_judge_before_author(env):
    run = env.create(task=DESIGN_TASK)
    env.control({**DESIGNS, "author": {"write": DEMO}})
    with pytest.raises(ToolRefusal, match="设计 arena"):
        run.run_worker("author", "跳过设计")
    with pytest.raises(ToolRefusal, match="两个设计候选"):
        run.run_worker("judge", "rubric")
    first = run.run_design("同一个设计任务")
    assert [(r.key.role, r.model) for r in first] == [("design-a", "claude-opus-5-5"), ("design-b", "gpt-6.1-sol")]
    assert run.run_design("重放时换了措辞") == first
    intents = {k: json.loads((run.dir / "intents" / f"{k}-{env.base}.json").read_text()) for k in ("design-a", "design-b")}
    assert {i["mode"] for i in intents.values()} == {"review"}
    assert intents["design-a"]["checkout"] != intents["design-b"]["checkout"]
    for slot in ("design-a", "design-b"):
        prompt = (run.dir / "prompts" / f"{slot}-{env.base}.md").read_text()
        assert "同一个设计任务" in prompt and "runner-prompt.md" in prompt
        assert git(env.home / "checkouts" / "r1" / slot, "rev-parse", "HEAD") == env.base
    judge = run.run_worker("judge", "rubric：简单、可测")
    assert judge.model == "gpt-6.1-sol"
    judge_prompt = (run.dir / "prompts" / f"judge-{env.base}.md").read_text()
    assert "设计包甲" in judge_prompt and "设计包乙" in judge_prompt and "rubric：简单、可测" in judge_prompt
    assert "claude-opus-5-5" not in judge_prompt and "gpt-6.1-sol" not in judge_prompt
    assert run.run_worker("author", "按合成设计实现").status == "completed"
    assert run.run_design("重放基线上的设计") == first
    run.commit_candidate()
    with pytest.raises(ToolRefusal, match="首个作者之前"):
        run.run_design("候选提交之后再开设计")
    assert sorted(env.claims()) == sorted([f"design-a-{env.base}", f"design-b-{env.base}", f"judge-{env.base}",
                                           f"author-{env.base}"])


def test_design_crash_between_the_two_intents_dispatches_each_slot_once(env):
    env.create(task=DESIGN_TASK).release()
    env.control(DESIGNS)
    assert env.driver("design", crash="after_intent_before_spawn").wait() == -signal.SIGKILL
    assert env.claims() == []
    receipts = env.reopen().run_design("恢复")
    assert [r.status for r in receipts] == ["completed", "completed"]
    assert sorted(env.claims()) == [f"design-a-{env.base}", f"design-b-{env.base}"]


def test_controller_sigkill_during_design_fan_out_adopts_both_runners(env):
    env.create(task=DESIGN_TASK).release()
    env.control({slot: {**step, "sleep": 4} for slot, step in DESIGNS.items()})
    workers = env.home / "runs" / "r1" / "workers"
    driver = env.driver("design")
    wait_for(lambda: all((workers / f"{slot}-{env.base}" / "state.json").exists() for slot in ("design-a", "design-b")))
    os.kill(driver.pid, signal.SIGKILL)
    driver.wait()
    assert all(supervisor_alive(workers / f"{slot}-{env.base}", _ps_rows()) for slot in ("design-a", "design-b"))
    receipts = env.reopen().run_design("恢复后再次请求")
    assert [r.status for r in receipts] == ["completed", "completed"]
    assert sorted(env.claims()) == [f"design-a-{env.base}", f"design-b-{env.base}"]
