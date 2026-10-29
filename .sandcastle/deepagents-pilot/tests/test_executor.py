import json
import sqlite3
import threading
import time
from contextlib import contextmanager

import pytest
from conftest import git
from langchain_core.messages import AIMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from test_agent import call, scripted
from v2env import DEMO, TICKET, V2_TASK, SimulatedCrash, V2Env, crash_at

from pilot1742.agent import build_agent
from pilot1742.approval import SpecAuthorization
from pilot1742.claims import ClaimRefused, ClaimStore, Fenced
from pilot1742.delivery import EffectPending
from pilot1742.executor import (
    AdmissionRefused,
    reconcile_effect,
    register,
    release_claim,
)
from pilot1742.github_app import IdentityError, OutcomeUnknown, RequestNotSent
from pilot1742.run import Busy, Halted, PilotRun
from pilot1742.workunit import Takeover, WorkUnit

BRANCH = f"codex/deepagents/t{TICKET}"
V2_TOOLS = {"run_worker", "commit_candidate", "publish_candidate", "run_status", "write_todos", "ls", "read_file",
            "glob", "grep"}


@pytest.fixture
def v2(tmp_path):
    return V2Env(tmp_path)


def posts(v2, pattern):
    return v2.github.count("POST", pattern)


def test_registrar_reads_every_timeline_page_and_old_forbidden_label_blocks(v2):
    events = [{"event": "commented"} for _ in range(230)]
    events[220] = {"event": "labeled", "label": {"name": "sandcastle"}}
    events[225] = {"event": "unlabeled", "label": {"name": "sandcastle"}}
    v2.github.issues[TICKET]["timeline"] = events
    with pytest.raises(AdmissionRefused, match="forbidden_label_history:sandcastle"):
        v2.register()
    pages = [page for m, p, page in v2.github.requests if p.endswith(f"/issues/{TICKET}/timeline")]
    assert pages == [1, 2, 3]
    assert not (v2.home / "units").exists() or not list((v2.home / "units").iterdir())


@pytest.mark.parametrize("break_it", ["timeline_page_500", "parent_500", "workers_missing", "workers_garbled",
                                      "workers_hit", "has_parent", "parent_event", "agent_label", "no_marker"])
def test_missing_or_bad_evidence_is_never_an_empty_set(v2, break_it):
    issue = v2.github.issues[TICKET]
    issue["timeline"] = [{"event": "commented"} for _ in range(150)]
    if break_it == "timeline_page_500":
        v2.github.fault("GET", rf".*/issues/{TICKET}/timeline", "status_500", page=2)
    elif break_it == "parent_500":
        v2.github.fault("GET", rf".*/issues/{TICKET}/parent", "status_500")
    elif break_it == "workers_missing":
        v2.workers_log.unlink()
    elif break_it == "workers_garbled":
        v2.workers_log.write_text("{not json\n")
    elif break_it == "workers_hit":
        v2.workers_log.write_text(json.dumps({"ticket": TICKET, "status": "claim"}) + "\n")
    elif break_it == "has_parent":
        issue["parent"] = 1323
    elif break_it == "parent_event":
        issue["timeline"].append({"event": "parent_issue_added"})
    elif break_it == "agent_label":
        issue["timeline"].append({"event": "labeled", "label": {"name": "agent:explore"}})
    else:
        issue["body"] = "普通票"
    with pytest.raises((AdmissionRefused, IdentityError)):
        v2.register()
    assert v2.writes() == []


def test_takeover_waits_for_human_never_claims_and_ignores_local_receipts(v2):
    v2.github.add_issue(1340, body="SPEC r3\n")
    unit = WorkUnit(TICKET, 1323, v2.base, ("demo/",), "feature", False, Takeover({}, SpecAuthorization(1340)),
                    f"t{TICKET}-1")
    (v2.home / "approvals.jsonl").write_text(json.dumps({"ticket": TICKET, "decision": "approve"}) + "\n")
    with pytest.raises(AdmissionRefused, match="not_configured"):
        v2.run(unit)
    assert v2.writes() == [] and not (v2.claims_root / f"{TICKET}.json").exists()


def test_tool_surface_has_publish_but_no_registrar_claim_or_release(v2):
    run = v2.run(v2.register())
    model = scripted(AIMessage("完"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": run.spec.run_id}}, durability="sync")
    assert set(model.requests[0]) == V2_TOOLS


def test_deep_agent_drives_v2_run_through_publish_tool(v2):
    run = v2.run(v2.register())
    v2.control({"author": {"write": DEMO}, "review": {"summary": "VERDICT: PASS\n可以"}})
    model = scripted(call("run_worker", role="author", brief="实现"), call("commit_candidate"),
                     call("run_worker", role="review", brief="审"), call("publish_candidate"), AIMessage("已交付"))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / "graph.sqlite", check_same_thread=False)), model)
    result = agent.invoke({"messages": [("user", "开始")]}, {"configurable": {"thread_id": run.spec.run_id}},
                          durability="sync")
    reply = [m for m in result["messages"] if getattr(m, "name", None) == "publish_candidate"][-1]
    published = json.loads(reply.content)
    assert published["ok"] and published["phase"] == "awaiting_merge"
    assert v2.github.ref(BRANCH) == published["head"] == run.status()["recorded_head"]


def test_v2_run_refuses_side_effects_without_executor(v2):
    unit = v2.register()
    v2.run(unit).release()
    bare = PilotRun.open(v2.home, unit.run_id, launcher=["false"], source_repo=v2.source)
    bare.acquire()
    with pytest.raises(Halted, match="新执行器"):
        bare.run_worker("author", "绕过执行器")
    assert v2.claims() == []


def test_two_run_ids_race_for_one_ticket_only_one_dispatches(v2):
    units = [v2.register(f"t{TICKET}-{n}") for n in (1, 2)]
    results: dict[int, object] = {}

    def attempt(n):
        try:
            results[n] = v2.run(units[n])
        except ClaimRefused as error:
            results[n] = error

    threads = [threading.Thread(target=attempt, args=(n,)) for n in (0, 1)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    winners = [r for r in results.values() if isinstance(r, PilotRun)]
    assert len(winners) == 1 and sum(isinstance(r, ClaimRefused) for r in results.values()) == 1
    v2.control({"author": {"write": DEMO}})
    winners[0].run_worker("author", "实现")
    assert v2.claims() == [f"author-{v2.base}"]
    assert posts(v2, rf".*/issues/{TICKET}/comments") == 1


def test_another_state_root_cannot_hold_the_same_ticket(v2, tmp_path):
    unit = v2.register()
    v2.run(unit)
    other_root = tmp_path / "other-root"
    other_root.mkdir()
    (other_root / "fake-control.json").write_text("{}")
    same_run = v2.register(state_root=other_root)
    with pytest.raises(ClaimRefused, match="另一状态根"):
        v2.run(same_run, state_root=other_root)
    with pytest.raises(ClaimRefused, match="已由"):
        v2.run(v2.register(f"t{TICKET}-2", state_root=other_root), state_root=other_root)
    assert v2.claim_record()["run_id"] == unit.run_id and v2.claim_record()["state"] == "active"


@pytest.mark.parametrize("point", ["after_label_before_receipt", "after_claim_before_receipt"])
def test_claim_crash_windows_resume_without_duplicate_mirror(v2, point):
    unit = v2.register()
    with pytest.raises(SimulatedCrash):
        v2.run(unit, crash=crash_at(point))
    assert v2.claim_record()["state"] == "intent"
    run = v2.run(unit)
    assert v2.claim_record()["state"] == "active" and v2.claim_record()["generation"] == 1
    assert run.generation == 1
    assert posts(v2, rf".*/issues/{TICKET}/comments") == 1
    assert v2.github.issues[TICKET]["labels"] == ["deepagents"]


def test_mirror_without_local_record_is_not_adopted(v2):
    unit = v2.register()
    v2.github.issues[TICKET]["labels"].append("deepagents")
    with pytest.raises(ClaimRefused, match="拒绝收养"):
        v2.run(unit)
    assert v2.writes() == []


def test_post_check_failure_halts_and_withdraws_own_mirror(v2):
    unit = v2.register()

    def concurrent_actor(method, path):
        if method == "POST" and path.endswith(f"/issues/{TICKET}/comments"):
            v2.github.issues[TICKET]["labels"].append("ready-for-agent")
    v2.github.on_request.append(concurrent_actor)
    with pytest.raises(ClaimRefused, match="post_check_failed"):
        v2.run(unit)
    record = v2.claim_record()
    assert record["state"] == "halted" and "forbidden:ready-for-agent" in record["reason"]
    assert "deepagents" not in v2.github.issues[TICKET]["labels"]
    bodies = [c["body"] for c in v2.github.issues[TICKET]["comments"]]
    assert any("deepagents:withdraw" in b for b in bodies)
    with pytest.raises(ClaimRefused, match="halted"):
        v2.run(unit)


def test_forbidden_label_after_admission_halts_the_next_dispatch(v2):
    run = v2.run(v2.register())
    v2.control({"author": {"write": DEMO}})
    run.run_worker("author", "实现")
    run.commit_candidate()
    v2.github.issues[TICKET]["labels"].append("sandcastle")
    with pytest.raises(Halted, match="forbidden_label:sandcastle"):
        run.run_worker("review", "审")
    assert v2.claim_record()["state"] == "halted"
    assert v2.claims() == [f"author-{v2.base}"]


def test_happy_path_publishes_exact_head_and_replay_adds_nothing(v2):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    result = run.publish_candidate()
    assert result["ok"] and result["phase"] == "awaiting_merge" and result["head"] == head
    assert v2.github.ref(BRANCH) == head
    assert git(v2.bare, "rev-parse", f"{head}^{{tree}}") == git(v2.source, "rev-parse", f"{head}^{{tree}}")
    trailers = git(v2.source, "log", "-1", "--format=%(trailers:key=Deepagents-Run,valueonly)", head)
    assert trailers == f"t{TICKET}-1/author-{v2.base} gen=1"
    assert git(v2.source, "log", "-1", "--format=%ci", head).endswith("+0000")
    (pr,) = v2.github.pulls.values()
    assert pr["draft"] and pr["base"] == "main" and v2.github.ref(pr["branch"]) == head
    assert [s["context"] for s in v2.github.statuses[head]] == ["deepagents/review"]
    assert set(v2.github.statuses) == {head}
    delivers = [c for c in v2.github.issues[TICKET]["comments"] if "deepagents:deliver" in c["body"]]
    assert len(delivers) == 1 and delivers[0]["user"]["type"] == "Bot"
    before = v2.writes()
    again = run.publish_candidate()
    assert again["ok"] and again["head"] == head and v2.writes() == before
    assert run.status()["phase"] == "awaiting_merge"
    v2.github.merge_pull(pr["number"])
    assert run.publish_candidate()["phase"] == "merged" and v2.writes() == before
    assert run.status()["phase"] == "merged"


def test_pull_request_closed_after_delivery_halts_on_replay(v2):
    run = v2.run(v2.register())
    v2.to_pass(run)
    assert run.publish_candidate()["ok"]
    (number,) = v2.github.pulls
    v2.github.close_pull(number)
    with pytest.raises(Halted, match="closed_pull_request"):
        run.publish_candidate()
    assert v2.claim_record()["state"] == "halted" and len(v2.github.pulls) == 1


def test_remote_descendant_is_stale_and_never_adopted(v2):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    tree = git(v2.source, "rev-parse", f"{head}^{{tree}}")
    descendant = git(v2.source, "commit-tree", tree, "-p", head, "-m", "别人推的后代")
    git(v2.source, "push", "-q", str(v2.bare), f"{descendant}:refs/heads/{BRANCH}")
    result = run.publish_candidate()
    assert result["decision"] == "stale" and result["observed"] == descendant
    assert v2.github.ref(BRANCH) == descendant
    assert v2.github.pulls == {} and v2.github.statuses == {}
    assert posts(v2, r".*/git/refs") == 0
    assert run.status()["phase"] == "stale"


def test_local_tip_drift_after_pass_refuses_with_no_remote_writes(v2):
    run = v2.run(v2.register())
    v2.to_pass(run)
    checkout = run.spec.checkout_for("author")
    (checkout / "demo" / "b.py").write_text("x = 2\n")
    git(checkout, "add", "-A")
    git(checkout, "commit", "-qm", "冻结后外部提交")
    before = v2.writes()
    result = run.publish_candidate()
    assert result["decision"] == "refused" and result["reasons"] == ["candidate_head_drift"]
    assert v2.writes() == before


def test_second_ticket_branch_in_clone_is_not_published(v2):
    git(v2.source, "branch", "codex/deepagents/t999", v2.base)
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    assert run.publish_candidate()["ok"]
    refs = git(v2.bare, "for-each-ref", "--format=%(refname)").split()
    assert sorted(refs) == ["refs/heads/codex/deepagents/t4242", "refs/heads/main"]
    assert v2.github.ref(BRANCH) == head


def test_closed_pull_request_is_not_reopened(v2):
    old = v2.github.add_pull(BRANCH, v2.base, state="closed", user=APP_USER,
                             body=f"<!-- deepagents:pr ticket={TICKET} run=t{TICKET}-0 gen=1 head={v2.base} -->")
    run = v2.run(v2.register())
    v2.to_pass(run)
    with pytest.raises(Halted, match="closed_pull_request"):
        run.publish_candidate()
    assert posts(v2, r".*/pulls") == 0 and list(v2.github.pulls) == [old]
    assert v2.claim_record()["state"] == "halted"


def test_old_pass_cannot_release_a_new_head(v2):
    run = v2.run(v2.register())
    first = v2.to_pass(run, review="VERDICT: BLOCK\n缺测试")
    assert run.publish_candidate()["reasons"] == ["review_block"]
    v2.control({"author": {"write": {"demo/test_a.py": "assert True\n"}}, "review": {"summary": "VERDICT: PASS"}})
    run.run_worker("author", "返修")
    second = run.commit_candidate().commit
    assert run.publish_candidate()["reasons"] == ["no_review_at_head"]
    run.run_worker("review", "复审")
    assert run.publish_candidate()["head"] == second
    assert first not in v2.github.statuses and v2.github.ref(BRANCH) == second
    tree = git(v2.source, "rev-parse", f"{second}^{{tree}}")
    moved = git(v2.source, "commit-tree", tree, "-p", second, "-m", "PR 头被改")
    git(v2.source, "push", "-q", "-f", str(v2.bare), f"{moved}:refs/heads/{BRANCH}")
    assert run.publish_candidate()["decision"] == "stale"
    assert set(v2.github.statuses) == {second}


@pytest.mark.parametrize("point", ["after_push_before_receipt", "after_pr_before_receipt",
                                   "after_status_before_receipt", "after_deliver_before_receipt"])
def test_publish_crash_windows_recover_without_duplicates(v2, point):
    unit = v2.register()
    run = v2.run(unit, crash=crash_at(point))
    head = v2.to_pass(run)
    with pytest.raises(SimulatedCrash):
        run.publish_candidate()
    run.release()
    resumed = v2.run(unit)
    assert resumed.publish_candidate()["ok"]
    assert posts(v2, r".*/git/refs") == 1 and posts(v2, r".*/pulls") == 1
    assert posts(v2, rf".*/statuses/{head}") == 1
    assert sum("deepagents:deliver" in c["body"] for c in v2.github.issues[TICKET]["comments"]) == 1
    assert v2.claims() == [f"author-{v2.base}", f"review-{head}"]


def test_lost_responses_are_read_back_not_resent(v2):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    v2.github.fault("POST", r".*/git/refs", "drop_after")
    v2.github.fault("POST", r".*/pulls", "drop_after")
    v2.github.fault("POST", rf".*/issues/{TICKET}/comments", "drop_after")
    assert run.publish_candidate()["ok"]
    assert posts(v2, r".*/git/refs") == 1 and posts(v2, r".*/pulls") == 1
    assert sum("deepagents:deliver" in c["body"] for c in v2.github.issues[TICKET]["comments"]) == 1
    assert v2.github.ref(BRANCH) == head


def deliveries(v2):
    return sum("deepagents:deliver" in c["body"] for c in v2.github.issues[TICKET]["comments"])


def test_accepted_comment_with_lost_response_and_late_visibility_is_never_resent(v2):
    unit = v2.register()
    run = v2.run(unit)
    v2.to_pass(run)
    v2.github.hidden.add("deepagents:deliver")
    v2.github.fault("POST", rf".*/issues/{TICKET}/comments", "drop_after")
    with pytest.raises(EffectPending):
        run.publish_candidate()
    sent = posts(v2, rf".*/issues/{TICKET}/comments")
    for _ in range(3):
        with pytest.raises(EffectPending):
            run.publish_candidate()
    run.release()
    pending = next((run.dir / "delivery").glob("deliver-*.json"))
    record = json.loads(pending.read_text())
    assert record["state"] == "pending" and record["what"]["kind"] == "comment"
    pending.write_text(json.dumps({**record, "sent_at": record["sent_at"] - 86400}))
    later = v2.run(unit)
    with pytest.raises(EffectPending):
        later.publish_candidate()
    assert posts(v2, rf".*/issues/{TICKET}/comments") == sent and deliveries(v2) == 1
    v2.github.hidden.clear()
    assert later.publish_candidate()["phase"] == "awaiting_merge"
    assert posts(v2, rf".*/issues/{TICKET}/comments") == sent and deliveries(v2) == 1


def test_connection_never_established_is_safely_retried(v2):
    run = v2.run(v2.register())
    v2.to_pass(run)
    v2.github.fault("POST", rf".*/issues/{TICKET}/comments", "drop_before")
    with pytest.raises(RequestNotSent):
        run.publish_candidate()
    assert run.publish_candidate()["ok"] and deliveries(v2) == 1


def test_server_error_stays_pending_until_operator_reconciles(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    v2.github.fault("POST", rf".*/statuses/{head}", "status_500")
    with pytest.raises(EffectPending):
        run.publish_candidate()
    with pytest.raises(EffectPending):
        run.publish_candidate()
    assert posts(v2, rf".*/statuses/{head}") == 1 and head not in v2.github.statuses
    store = ClaimStore(v2.claims_root)
    with pytest.raises(Busy):
        reconcile_effect(v2.home, store, unit.run_id, f"status-{head}", "已在 GitHub 核实无此状态")
    run.release()
    record = reconcile_effect(v2.home, store, unit.run_id, f"status-{head}", "已在 GitHub 核实无此状态")
    assert record["state"] == "reconciled_not_applied" and record["what"]["sha"] == head
    assert v2.run(unit).publish_candidate()["ok"]
    assert posts(v2, rf".*/statuses/{head}") == 2 and len(v2.github.statuses[head]) == 1
    journal = json.loads((run.dir / "delivery" / f"status-{head}.json").read_text())
    assert journal["state"] == "done"


WRITE_SENDS = {"push": r".*/git/refs", "pr": r".*/pulls", "status": r".*/statuses/[0-9a-f]{40}",
               "deliver": rf".*/issues/{TICKET}/comments"}


@pytest.mark.parametrize("effect", sorted(WRITE_SENDS))
def test_accepted_write_with_unreadable_body_is_confirmed_by_readback_not_resent(v2, effect):
    run = v2.run(v2.register())
    v2.to_pass(run)
    before = posts(v2, WRITE_SENDS[effect])
    v2.github.fault("POST", WRITE_SENDS[effect], "garble_after")
    assert run.publish_candidate()["phase"] == "awaiting_merge"
    assert posts(v2, WRITE_SENDS[effect]) == before + 1 and deliveries(v2) == 1


def test_accepted_comment_with_unreadable_body_and_late_visibility_stays_pending(v2):
    unit = v2.register()
    run = v2.run(unit)
    v2.to_pass(run)
    v2.github.hidden.add("deepagents:deliver")
    v2.github.fault("POST", WRITE_SENDS["deliver"], "garble_after")
    for _ in range(3):
        with pytest.raises(EffectPending):
            run.publish_candidate()
    sent = posts(v2, WRITE_SENDS["deliver"])
    assert deliveries(v2) == 1
    v2.github.hidden.clear()
    assert run.publish_candidate()["ok"]
    assert posts(v2, WRITE_SENDS["deliver"]) == sent and deliveries(v2) == 1


def test_readback_failure_after_a_sent_comment_keeps_it_pending(v2):
    run = v2.run(v2.register())
    v2.to_pass(run)

    def fail_next_readback(method, path):
        if method == "POST" and path.endswith(f"/issues/{TICKET}/comments"):
            v2.github.fault("GET", rf".*/issues/{TICKET}/comments", "status_500")
            v2.github.on_request.remove(fail_next_readback)
    v2.github.on_request.append(fail_next_readback)
    with pytest.raises(OutcomeUnknown):
        run.publish_candidate()
    pending = json.loads(next((run.dir / "delivery").glob("deliver-*.json")).read_text())
    assert pending["state"] == "pending"
    sent = posts(v2, WRITE_SENDS["deliver"])
    assert run.publish_candidate()["ok"]
    assert posts(v2, WRITE_SENDS["deliver"]) == sent and deliveries(v2) == 1


def test_definitive_rejection_revokes_pending_and_allows_one_send(v2):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    v2.github.fault("POST", WRITE_SENDS["status"], "reject_422")
    with pytest.raises(IdentityError, match="HTTP 422"):
        run.publish_candidate()
    assert not (run.dir / "delivery" / f"status-{head}.json").exists()
    assert run.publish_candidate()["ok"]
    assert posts(v2, WRITE_SENDS["status"]) == 2 and len(v2.github.statuses[head]) == 1


def test_reconcile_holds_the_run_lock_for_the_whole_transaction(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    v2.github.fault("POST", WRITE_SENDS["status"], "status_500")
    with pytest.raises(EffectPending):
        run.publish_candidate()
    run.release()
    (run.dir / "owner.lock").unlink()
    store = ClaimStore(v2.claims_root)
    entered, proceed = threading.Event(), threading.Event()
    real_locked = store.locked

    @contextmanager
    def held_ticket_lock(ticket):
        entered.set()
        assert proceed.wait(10)
        with real_locked(ticket) as txn:
            yield txn
    store.locked = held_ticket_lock
    outcome = {}

    def reconcile():
        outcome["result"] = reconcile_effect(v2.home, store, unit.run_id, f"status-{head}", "已核实无此状态")
    worker = threading.Thread(target=reconcile)
    worker.start()
    assert entered.wait(10)
    with pytest.raises(Busy):
        run.acquire()
    with pytest.raises(Busy):
        v2.run(unit)
    proceed.set()
    worker.join(10)
    assert outcome["result"]["state"] == "reconciled_not_applied"
    resumed = v2.run(unit)
    assert resumed.publish_candidate()["ok"] and len(v2.github.statuses[head]) == 1


def test_reconcile_failure_still_releases_the_run_lock(v2):
    unit = v2.register()
    run = v2.run(unit)
    run.release()
    with pytest.raises(EffectPending, match="找不到"):
        reconcile_effect(v2.home, ClaimStore(v2.claims_root), unit.run_id, "deliver-none", "x")
    v2.run(unit).release()


def test_forbidden_label_after_author_blocks_the_local_commit(v2):
    run = v2.run(v2.register())
    v2.control({"author": {"write": DEMO}})
    run.run_worker("author", "实现")
    v2.github.issues[TICKET]["labels"].append("sandcastle")
    with pytest.raises(Halted, match="commit:forbidden_label:sandcastle"):
        run.commit_candidate()
    assert git(run.spec.checkout_for("author"), "rev-parse", "HEAD") == v2.base
    assert v2.claim_record()["state"] == "halted"


def stop_before(run, effect):
    original = run.executor.action

    @contextmanager
    def wrapped(name, *args, **kwargs):
        if name == effect:
            raise SimulatedCrash(f"before {effect}")
        with original(name, *args, **kwargs) as boundary:
            yield boundary
    run.executor.action = wrapped


@pytest.mark.parametrize("effect", ["status", "deliver"])
def test_resume_after_remote_drift_goes_stale_before_any_further_write(v2, effect):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    stop_before(run, effect)
    with pytest.raises(SimulatedCrash):
        run.publish_candidate()
    run.release()
    tree = git(v2.source, "rev-parse", f"{head}^{{tree}}")
    moved = git(v2.source, "commit-tree", tree, "-p", head, "-m", "外部漂移")
    git(v2.source, "push", "-q", "-f", str(v2.bare), f"{moved}:refs/heads/{BRANCH}")
    before = v2.writes()
    result = v2.run(unit).publish_candidate()
    assert result["decision"] == "stale" and result["observed"] == moved
    assert v2.writes() == before and deliveries(v2) == 0


APP_USER = {"login": "newchanlun-deepagents-1742[bot]", "type": "Bot"}


@pytest.mark.parametrize("change, reason", [
    ({"base": "unrelated"}, "base_not_main"), ({"draft": False}, "not_draft"),
    ({"user": {"login": "xy7365527-lang", "type": "User"}}, "not_opened_by_app"), ({"body": ""}, "missing_pr_marker"),
])
def test_existing_pr_outside_the_contract_is_not_adopted(v2, change, reason):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    fields = {"user": APP_USER, "body": f"<!-- deepagents:pr ticket={TICKET} run=x gen=1 head={head} -->", **change}
    number = v2.github.add_pull(BRANCH, head, state="open", **fields)
    with pytest.raises(Halted, match=reason):
        run.publish_candidate()
    assert posts(v2, r".*/pulls") == 0 and v2.github.statuses == {} and deliveries(v2) == 0
    assert list(v2.github.pulls) == [number] and v2.claim_record()["state"] == "halted"


def test_pr_retargeted_after_delivery_halts_on_replay(v2):
    run = v2.run(v2.register())
    v2.to_pass(run)
    assert run.publish_candidate()["ok"]
    (number,) = v2.github.pulls
    v2.github.pulls[number]["base"] = "unrelated"
    before = v2.writes()
    with pytest.raises(Halted, match="base_not_main"):
        run.publish_candidate()
    assert v2.writes() == before


def test_registrar_accepts_a_remote_feature_branch_base_and_rejects_an_unpushed_one(v2):
    (v2.source / "EXECUTOR.md").write_text("已审实现\n")
    git(v2.source, "add", "EXECUTOR.md")
    git(v2.source, "commit", "-qm", "功能分支")
    feature = git(v2.source, "rev-parse", "HEAD")
    git(v2.source, "commit", "-q", "--allow-empty", "-m", "只在本地")
    local_only = git(v2.source, "rev-parse", "HEAD")
    git(v2.source, "push", "-q", str(v2.bare), f"{feature}:refs/heads/codex/deepagents-pilot-1742")
    with pytest.raises(AdmissionRefused, match="远端没有基线"):
        register(v2.home, v2.gateway(), ticket=TICKET, run_id=f"t{TICKET}-1", base=local_only, task=V2_TASK,
                 playbook="feature", scope_ref="s", workers_log=v2.workers_log, source_repo=v2.source)
    unit = register(v2.home, v2.gateway(), ticket=TICKET, run_id=f"t{TICKET}-1", base=feature, task=V2_TASK,
                    playbook="feature", scope_ref="s", workers_log=v2.workers_log, source_repo=v2.source)
    assert unit.base == feature
    run = v2.run(unit)
    head = v2.to_pass(run)
    assert run.publish_candidate()["ok"] and v2.github.ref(BRANCH) == head
    (pr,) = v2.github.pulls.values()
    assert pr["base"] == "main" and git(v2.bare, "rev-parse", f"{head}~1") == feature


def test_server_error_on_ref_create_is_reconciled(v2):
    run = v2.run(v2.register())
    head = v2.to_pass(run)
    v2.github.fault("POST", r".*/git/refs", "status_500")
    with pytest.raises(EffectPending):
        run.publish_candidate()
    assert v2.github.ref(BRANCH) is None
    assert run.publish_candidate()["ok"] and v2.github.ref(BRANCH) == head
    assert posts(v2, r".*/git/refs") == 2


def test_release_waits_for_the_inflight_effect_then_old_controller_stops(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    hold = v2.github.fault("POST", r".*/git/refs", "hold")
    outcome: dict[str, object] = {}
    released = threading.Event()
    original = run.executor.action

    @contextmanager
    def after_release_for_next_effect(effect, *args, **kwargs):
        if effect != "push":
            assert released.wait(20)
        with original(effect, *args, **kwargs) as boundary:
            yield boundary

    run.executor.action = after_release_for_next_effect

    def publish():
        try:
            outcome["publish"] = run.publish_candidate()
        except BaseException as error:  # noqa: BLE001
            outcome["publish"] = error

    def release():
        outcome["release"] = release_claim(ClaimStore(v2.claims_root, lock_timeout=30), TICKET, unit.run_id, "主控退回")
        released.set()

    worker = threading.Thread(target=publish)
    worker.start()
    assert hold.entered.wait(10)
    releaser = threading.Thread(target=release)
    releaser.start()
    time.sleep(0.5)
    assert "release" not in outcome and v2.claim_record()["state"] == "active"
    hold.release.set()
    worker.join(20)
    releaser.join(20)
    assert v2.claim_record()["state"] == "released"
    assert isinstance(outcome["publish"], Fenced)
    assert v2.github.ref(BRANCH) == head and v2.github.pulls == {} and v2.github.statuses == {}
    assert not any("deepagents:deliver" in c["body"] for c in v2.github.issues[TICKET]["comments"])
    before = v2.writes()
    with pytest.raises(Fenced):
        run.publish_candidate()
    assert v2.writes() == before


def test_reclaim_after_release_needs_a_new_run_and_bumps_generation(v2):
    unit = v2.register()
    run = v2.run(unit)
    run.release()
    release_claim(ClaimStore(v2.claims_root), TICKET, unit.run_id, "主控退回")
    with pytest.raises(ClaimRefused, match="released"):
        v2.run(unit)
    second = v2.run(v2.register(f"t{TICKET}-2"))
    assert second.generation == 2 and v2.claim_record()["run_id"] == f"t{TICKET}-2"
    markers = [c["body"] for c in v2.github.issues[TICKET]["comments"] if "deepagents:claim" in c["body"]]
    assert len(markers) == 2 and "gen=2" in markers[1]


def test_registration_is_idempotent_and_frozen(v2):
    first = v2.register()
    assert v2.register() == first
    v2.github.issues[TICKET]["labels"].append("unrelated")
    with pytest.raises(AdmissionRefused, match="不同的 WorkUnit"):
        v2.register()
    assert V2_TASK.allowed_prefixes == first.allowed_prefixes
