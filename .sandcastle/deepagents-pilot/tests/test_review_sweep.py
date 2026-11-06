import copy

import pytest
import test_external_review as _external_review
from conftest import git

from pilot1742 import review_sweep as module
from pilot1742.review_sweep import Sweep, process_alive

lane = _external_review.lane  # 复用真实审查 lane 与 PR 替身


class Host:
    """进程表与子进程替身。run=True 时 spawn 在进程内同步跑完 review_pr，模拟审查子进程结束。"""
    def __init__(self, lane, *, run=True):
        self.lane, self.run, self.rows, self.spawned, self.next_pid = lane, run, [], [], 4000

    def spawn(self, number, attempt, log):
        self.next_pid += 1
        self.spawned.append((number, attempt))
        if self.run:
            self.lane.review_pr(number, attempt)
        else:
            self.rows.append((self.next_pid, self.next_pid, "S",
                              " ".join(module.review_command(number, attempt, "/k")[1:])))
        return self.next_pid


def make(lane, *, run=True, pulls=None, **kwargs):
    host = Host(lane, run=run)
    sweep = Sweep(lane.gateway, lane, host.spawn, state_root=lane.state_root, rows=lambda: list(host.rows), **kwargs)
    sweep.open_pulls = lambda: copy.deepcopy(pulls if pulls is not None else [lane.gateway.pr])
    return sweep, host


def test_new_head_is_reviewed_without_operator(lane):
    sweep, host = make(lane)
    assert [a["action"] for a in sweep.tick()] == ["launch"]
    assert host.spawned == [(1810, 1)] and lane.gateway.sent == ["pending", "success"]
    assert [a["action"] for a in sweep.tick()] == ["done"]  # 去重：同一 head 不再发起
    assert host.spawned == [(1810, 1)]


def test_live_review_is_not_duplicated(lane):
    sweep, host = make(lane, run=False)
    assert [a["action"] for a in sweep.tick()] == ["launch"]
    assert [a["action"] for a in sweep.tick()] == ["running"]
    assert host.spawned == [(1810, 1)]


def test_new_head_after_push_gets_its_own_review(lane):
    sweep, host = make(lane)
    sweep.tick()
    (lane.source / "changed-1").write_text("pushed again\n")
    git(lane.source, "commit", "-qam", "push")
    lane.gateway.pr["head"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    assert [a["action"] for a in sweep.tick()] == ["launch"]
    assert host.spawned == [(1810, 1), (1810, 1)]
    rows = lane.status(1810)
    assert {r["target"]["head_sha"] for r in rows} == {r["target"]["head_sha"] for r in rows if r["outcome"]}
    assert len(rows) == 2


def test_interrupted_review_resumes_same_attempt_once_then_needs_operator(lane):
    sweep, host = make(lane, run=False)
    sweep.tick()
    lane.target(1810, 1)              # 子进程已登记 attempt 1……
    host.rows.clear()                 # ……然后中断，没有结论
    assert [a["action"] for a in sweep.tick()] == ["resume"]
    assert host.spawned == [(1810, 1), (1810, 1)]
    host.rows.clear()
    assert [a["action"] for a in sweep.tick()] == ["needs-operator"]
    assert host.spawned == [(1810, 1), (1810, 1)]


def test_explicit_reattempt_by_operator_is_left_alone(lane):
    sweep, host = make(lane)
    sweep.tick()
    lane.review_pr(1810, 2)           # 操作员对同一 head 显式重审
    assert [a["action"] for a in sweep.tick()] == ["done"]
    assert host.spawned == [(1810, 1)]


def test_concurrency_cap_counts_every_live_review(lane):
    second = copy.deepcopy(lane.gateway.pr)
    second["number"] = 1811
    sweep, host = make(lane, run=False, pulls=[lane.gateway.pr, second], max_running=1)
    assert [a["action"] for a in sweep.tick()] == ["launch", "queued"]
    assert host.spawned == [(1810, 1)]


@pytest.mark.parametrize("change", ["draft", "foreign"])
def test_drafts_and_foreign_heads_are_not_reviewed(lane, change):
    pr = copy.deepcopy(lane.gateway.pr)
    if change == "draft":
        pr["draft"] = True
    else:
        pr["head"]["repo"]["id"] = 1
    sweep, host = make(lane, pulls=[pr])
    assert [a["action"] for a in sweep.tick()] == ["draft" if change == "draft" else "foreign-head"]
    assert host.spawned == []


def test_missing_objects_wait_for_owner_sync(lane):
    pr = copy.deepcopy(lane.gateway.pr)
    pr["head"]["sha"] = "f" * 40
    sweep, host = make(lane, pulls=[pr])
    assert [a["action"] for a in sweep.tick()] == ["needs-source-sync"]
    synced = []
    sweep.sync = lambda number, base, head: synced.append((number, base, head))
    assert [a["action"] for a in sweep.tick()] == ["needs-source-sync"]
    assert synced == [(1810, pr["base"]["sha"], "f" * 40)] and host.spawned == []


def test_process_identity_rejects_pid_reuse_and_zombies():
    record = {"pr": 5, "attempt": 1, "pid": 42}
    command = "-m pilot1742.external_review review-pr --pr 5 --attempt 1 --private-key /k"
    assert process_alive(record, [(42, 42, "S", command)])
    assert not process_alive(record, [(42, 42, "Z", command)])
    assert not process_alive(record, [(42, 42, "S", "vim notes.txt")])
    assert not process_alive(record, [(42, 42, "S", command.replace("--pr 5 ", "--pr 50 "))])
    assert not process_alive(record, [(43, 43, "S", command)])
