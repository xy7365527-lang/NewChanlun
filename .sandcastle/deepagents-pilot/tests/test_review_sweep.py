import copy
import json

import pytest
import test_external_review as _external_review
from conftest import git

from pilot1742 import review_sweep as module
from pilot1742.external_review import ReviewTarget
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


class FakeMerge:
    def __init__(self, required, states, files=None, contents=None):
        self.required, self.states, self.calls = set(required), states, []
        self.files, self.contents = files or {}, contents or {}

    def file_at(self, ref, path):
        return self.contents.get((ref, path))

    def pr_files(self, number):
        return self.files.get(number, [".sandcastle/x.py"])

    def disable_auto_merge(self, number):
        self.calls.append(("disable", number))

    def required_checks(self):
        return self.required

    def pr_state(self, number):
        return copy.deepcopy(self.states[number])

    def enable_auto_merge(self, number, head):
        self.calls.append(("auto", number, head))

    def update_branch(self, number, head):
        self.calls.append(("update", number, head))


def _merge_state(head, *, behind=False, auto=False, review="success", ci="success"):
    return {"head": head, "merge_state": "BEHIND" if behind else "BLOCKED", "auto_merge": auto,
            "checks": {"deepagents/review": review, "ci-ok": ci}}


def test_auto_merge_is_not_enabled_unless_ci_ok_is_required(lane):
    head = lane.gateway.pr["head"]["sha"]
    merge = FakeMerge({"deepagents/review"}, {1810: _merge_state(head)})
    sweep, _ = make(lane, merge=merge)
    assert sweep.tick()[-1] == {"action": "auto-merge-unsafe", "reason": "main 必需检查须含 ['ci-ok', 'deepagents/review']"}
    assert merge.calls == []


def test_auto_merge_enabled_once_and_only_green_behind_pr_is_updated(lane):
    head = lane.gateway.pr["head"]["sha"]
    a, b, c = (copy.deepcopy(lane.gateway.pr) for _ in range(3))
    b["number"], c["number"] = 1811, 1812
    merge = FakeMerge({"deepagents/review", "ci-ok"}, {
        1810: _merge_state(head, behind=True, ci="failure"),     # CI 红：不追平
        1811: _merge_state(head, behind=True, auto=True),        # 审查与 ci-ok 都绿：追平
        1812: _merge_state(head, behind=True, auto=True)})       # 同轮只追平一个
    sweep, _ = make(lane, pulls=[a, b, c], merge=merge, max_running=0)
    sweep.tick()
    assert merge.calls == [("auto", 1810, head), ("update", 1811, head)]


def test_moved_head_is_left_for_next_tick(lane):
    merge = FakeMerge({"deepagents/review", "ci-ok"}, {1810: _merge_state("0" * 40, behind=True)})
    sweep, _ = make(lane, merge=merge, max_running=0)
    sweep.tick()
    assert merge.calls == []


def test_stale_and_draft_pulls_are_neither_reviewed_nor_merged(lane):
    from datetime import datetime, timezone
    stale, draft = copy.deepcopy(lane.gateway.pr), copy.deepcopy(lane.gateway.pr)
    stale["updated_at"] = "2026-01-01T00:00:00Z"
    draft["number"], draft["draft"] = 1811, True
    head = lane.gateway.pr["head"]["sha"]
    merge = FakeMerge({"deepagents/review", "ci-ok"}, {1810: _merge_state(head), 1811: _merge_state(head)})
    sweep, host = make(lane, pulls=[stale, draft], merge=merge, now=lambda: datetime(2026, 11, 7, tzinfo=timezone.utc))
    assert [a["action"] for a in sweep.tick()] == ["stale-pr", "draft"]
    assert host.spawned == [] and merge.calls == []


def test_stale_outcome_after_base_drift_is_reviewed_again_on_current_base(lane):
    """Codex P1（#1939）：审查期间 main 前进，结果记为 STALE 且不发布终态；不能把它当作完成而永久卡在 pending。"""
    sweep, host = make(lane)
    assert [a["action"] for a in sweep.tick()] == ["launch"]
    first = lane.status(1810)[0]
    outcome = lane.state_root / "runs" / ReviewTarget(**first["target"]).key / "outcome.json"
    stale = json.loads(outcome.read_text())
    stale["verdict"] = "STALE"
    outcome.write_text(json.dumps(stale))
    git(lane.source, "checkout", "-q", "-b", "main-next", lane.gateway.pr["base"]["sha"])
    (lane.source / "unrelated").write_text("main moved\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "main moved")
    lane.gateway.pr["base"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    git(lane.source, "checkout", "-q", lane.gateway.pr["head"]["sha"])
    assert [a["action"] for a in sweep.tick()] == ["launch"]
    assert [a["action"] for a in sweep.tick()] == ["done"]


def test_stale_on_current_base_is_not_done(lane):
    sweep, host = make(lane, run=False)
    sweep.tick()
    target = lane.target(1810, 1)
    folder = lane.state_root / "runs" / target.key
    (folder / "outcome.json").write_text(json.dumps({"run_id": target.run_id, "verdict": "STALE"}))
    host.rows.clear()
    assert [a["action"] for a in sweep.tick()] != ["done"]


@pytest.mark.parametrize(("path", "category"), [
    ("rust/src/bin/x_execution_session.rs", "live-trading-or-funds"),   # 用户原例：下单逻辑
    ("rust/src/economic_session/book.rs", "live-trading-or-funds"),     # 账本
    ("rust/src/bin/production_session.rs", "live-trading-or-funds"),
    ("trading_system/execution/lmt_executor.py", "live-trading-or-funds"),
    (".github/workflows/ci.yml", "ci-or-branch-protection"),
    (".github/1488-uo-locks.txt", "ci-or-branch-protection"),
    ("deploy/.env.production", "secrets-or-credentials"),
    ("keys/app.pem", "secrets-or-credentials"),
])
def test_human_gated_pr_is_never_auto_merged_and_existing_auto_merge_is_withdrawn(lane, path, category):
    """用户选择的窄口子：实盘下单或资金、密钥或凭证、CI 与分支保护配置，须人工确认。"""
    head = lane.gateway.pr["head"]["sha"]
    merge = FakeMerge({"deepagents/review", "ci-ok"}, {1810: _merge_state(head, behind=True, auto=True)},
                      files={1810: [".sandcastle/a.py", path]})
    sweep, _ = make(lane, merge=merge, max_running=0)
    action = sweep.tick()[-1]
    assert action["action"] == "needs-human-confirm" and action["gated"] == {category: [path]}
    assert merge.calls == [("disable", 1810)]  # 撤掉已开的 auto-merge，也不追平


@pytest.mark.parametrize("path", [
    ".sandcastle/deepagents-pilot/pilot1742/external_review.py", ".sandcastle/codex-child.ts",
    "docs/adr/0061-x.md", ".github/copilot-instructions.md", ".github/PULL_REQUEST_TEMPLATE.md",
    "topological-computation/frontend/src/tokens.ts", "rust/src/diagnostic_fill.rs", "docs/ci_secrets.md",
    "src/newchan/data_ibkr.py", "rust/src/bin/s_structure_session.rs"])
def test_ordinary_code_and_docs_are_not_human_gated(path):
    from pilot1742.review_sweep import human_gated
    assert human_gated([path]) == {}


@pytest.mark.parametrize(("base_keys", "head_keys", "gated"), [
    (["A|x|1", "B|y|2"], ["A|x|1"], False),        # 只删条目（文件出仓）：不放宽
    (["A|x|1"], ["A|x|1", "C|z|3"], True),         # 新增被放行的发现：放宽，须确认
    (["A|x|139"], ["A|x|138"], False),             # 同一发现行号迁移：不放宽
    (["A|x|1"], ["A|x|1", "A|x|9"], True),         # 同一文件同一规则多放行一条：放宽
])
def test_devskim_baseline_is_gated_only_when_admission_widens(lane, base_keys, head_keys, gated):
    head, base = lane.gateway.pr["head"]["sha"], lane.gateway.pr["base"]["sha"]
    path = ".github/devskim-baseline.json"
    doc = lambda keys: json.dumps({"schema": "devskim-baseline/v1", "findings": keys})
    merge = FakeMerge({"deepagents/review", "ci-ok"}, {1810: _merge_state(head)}, files={1810: [path]},
                      contents={(head, path): doc(head_keys), (base, path): doc(base_keys)})
    sweep, _ = make(lane, merge=merge, max_running=0)
    actions = [a["action"] for a in sweep.tick()]
    assert ("needs-human-confirm" in actions) == gated


