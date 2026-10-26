import json
import os
import signal
import time

import pytest
from conftest import git

from pilot1742.domain import Approved, Awaiting
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
    receipt = env.reopen().commit_candidate()
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
        env.create(models="feature, refactoring: gpt-6.1-sol\njudgment and prose: gpt-6.1-sol\nhardest tasks: claude-opus-5-5\n")
