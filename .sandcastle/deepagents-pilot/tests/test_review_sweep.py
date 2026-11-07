import copy
import json
import os
from datetime import datetime, timezone

import pytest
import test_external_review as _external_review
from conftest import git

from pilot1742 import review_sweep as module
from pilot1742.external_review import ReviewTarget
from pilot1742.review_sweep import (
    EvidenceInsufficient,
    Sweep,
    admission_keys,
    human_gated,
    reviewing,
)

lane = _external_review.lane  # 复用真实审查 lane 与 PR 替身


class FakeGitHub:
    """GitHub 端口替身：PR、检查、文件、内容与写操作记录。"""
    def __init__(self, lane, *, required=("deepagents/review", "ci-ok")):
        self.lane = lane
        self.required = set(required)
        self.extra_pulls = []
        self.states, self.checks_by_head, self.files, self.contents, self.calls, self.fail = {}, {}, {}, {}, [], {}

    def open_pulls(self):
        return copy.deepcopy([self.lane.gateway.pr, *self.extra_pulls])

    def _pr(self, number):
        return next(p for p in [self.lane.gateway.pr, *self.extra_pulls] if p["number"] == number)

    def pr(self, number):
        if "pr" in self.fail.get(number, ()):
            raise RuntimeError("HTTP 502")
        p = self._pr(number)
        state = {"head": p["head"]["sha"], "base": p["base"]["sha"], "changed_files": 1,
                 "mergeable_state": "clean", "auto_merge": False}
        state.update(self.states.get(number, {}))
        return state

    def checks(self, head):
        return dict(self.checks_by_head.get(head, {}))

    def required_checks(self):
        return set(self.required)

    def pr_files(self, number):
        return copy.deepcopy(self.files.get(number, [{"filename": ".sandcastle/x.py", "status": "modified", "changes": 1}]))

    def file_at(self, ref, path):
        value = self.contents.get((ref, path))
        if isinstance(value, Exception):
            raise value
        return value

    def merge(self, number, head):
        self.calls.append(("merge", number, head))

    def update_branch(self, number, head):
        self.calls.append(("update", number, head))

    def disable_auto_merge(self, number):
        self.calls.append(("disable", number))


class Host:
    """进程表与子进程替身。run=True 时 spawn 在进程内同步跑完 review_pr，模拟审查子进程结束。"""
    def __init__(self, lane, *, run=True):
        self.lane, self.run, self.rows, self.spawned, self.next_pid = lane, run, [], [], 4000

    def spawn(self, number, attempt, head, log):
        self.next_pid += 1
        self.spawned.append((number, attempt, head))
        if self.run:
            self.lane.review_pr(number, attempt)
        else:
            self.rows.append((self.next_pid, self.next_pid, "S",
                              " ".join(module.run_review_command(number, attempt, head, "/k")[1:])))
        return self.next_pid


def make(lane, *, run=True, github=None, **kwargs):
    host = Host(lane, run=run)
    github = github or FakeGitHub(lane)
    sweep = Sweep(github, lane, host.spawn, state_root=lane.state_root, rows=lambda: list(host.rows), **kwargs)
    return sweep, host, github


def acts(actions, pr=1810):
    return [a["action"] for a in actions if a.get("pr") == pr]


def green(github, head, **override):
    github.checks_by_head[head] = {"deepagents/review": "success", "ci-ok": "success", "DevSkim": "success", **override}


# ---- 审查派发 ----

def test_new_head_is_reviewed_without_operator(lane):
    sweep, host, _ = make(lane)
    assert acts(sweep.tick()) == ["launch"]
    assert lane.gateway.sent == ["pending", "success"]
    assert acts(sweep.tick()) == ["done"]
    assert len(host.spawned) == 1


def test_one_live_reviewer_per_pr_regardless_of_head(lane):
    """F5：同一 PR 已有审查进程在跑（不论它审哪个 head）就不再派发，免得重复派发撞 run 锁、误报 needs-operator。"""
    sweep, host, _ = make(lane, run=False)
    assert acts(sweep.tick()) == ["launch"]
    lane.gateway.pr["head"]["sha"] = "f" * 40  # 决策后 head 变了
    assert acts(sweep.tick()) == ["running"]
    assert len(host.spawned) == 1


def test_new_head_after_push_gets_its_own_review(lane):
    sweep, host, _ = make(lane)
    sweep.tick()
    (lane.source / "changed-1").write_text("pushed again\n")
    git(lane.source, "commit", "-qam", "push")
    lane.gateway.pr["head"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    assert acts(sweep.tick()) == ["launch"]
    assert len(host.spawned) == 2


def test_main_advance_does_not_rereview_the_same_head(lane):
    """F4：「已审完」按 head 判定；main 前进不把同一 head 拉回 pending、不重开双审。"""
    sweep, host, _ = make(lane)
    sweep.tick()
    git(lane.source, "checkout", "-q", "-b", "main-next", lane.gateway.pr["base"]["sha"])
    (lane.source / "unrelated").write_text("main moved\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "main moved")
    lane.gateway.pr["base"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    git(lane.source, "checkout", "-q", lane.gateway.pr["head"]["sha"])
    assert acts(sweep.tick()) == ["done"]
    assert len(host.spawned) == 1 and lane.gateway.sent == ["pending", "success"]


def test_only_stale_on_head_is_reviewed_again_with_next_attempt(lane):
    sweep, host, _ = make(lane)
    sweep.tick()
    row = lane.status(1810)[0]
    path = lane.state_root / "runs" / ReviewTarget(**row["target"]).key / "outcome.json"
    stale = json.loads(path.read_text())
    stale["verdict"] = "STALE"
    stale.pop("publication")
    path.write_text(json.dumps(stale))
    actions = sweep.tick()
    assert acts(actions) == ["launch"] and actions[0]["attempt"] == 2


def test_interrupted_review_resumes_same_attempt_once_then_needs_operator(lane):
    sweep, host, _ = make(lane, run=False)
    sweep.tick()
    lane.target(1810, 1)              # 子进程已登记 attempt 1……
    host.rows.clear()                 # ……然后中断，没有结论
    assert acts(sweep.tick()) == ["resume-review"]
    host.rows.clear()
    assert acts(sweep.tick()) == ["needs-operator"]
    assert [s[1] for s in host.spawned] == [1, 1]


def test_unpublished_outcome_is_resumed_once_then_needs_reconcile(lane):
    """F6：outcome 已存但终态没发布，不算完成；同一 attempt 续跑一次走受管对账与发布，再不成报 needs-reconcile。"""
    sweep, host, _ = make(lane, run=False)
    lane.review_pr(1810)
    row = lane.status(1810)[0]
    path = lane.state_root / "runs" / ReviewTarget(**row["target"]).key / "outcome.json"
    saved = json.loads(path.read_text())
    saved.pop("publication")
    path.write_text(json.dumps(saved))
    assert acts(sweep.tick()) == ["resume-publish"]
    host.rows.clear()
    assert acts(sweep.tick()) == ["needs-reconcile"]


def test_explicit_reattempt_by_operator_is_left_alone(lane):
    sweep, host, _ = make(lane)
    sweep.tick()
    lane.review_pr(1810, 2)
    assert acts(sweep.tick()) == ["done"]
    assert len(host.spawned) == 1


def test_concurrency_cap_counts_every_live_reviewer(lane):
    github = FakeGitHub(lane)
    second = copy.deepcopy(lane.gateway.pr)
    second["number"] = 1811
    github.extra_pulls = [second]
    sweep, host, _ = make(lane, run=False, github=github, max_running=1)
    actions = sweep.tick()
    assert acts(actions, 1810) == ["launch"] and acts(actions, 1811) == ["queued"]


@pytest.mark.parametrize("change", ["draft", "foreign", "stale"])
def test_drafts_foreign_and_stale_pulls_are_not_reviewed(lane, change):
    if change == "draft":
        lane.gateway.pr["draft"] = True
    elif change == "foreign":
        lane.gateway.pr["head"]["repo"]["id"] = 1
    else:
        lane.gateway.pr["updated_at"] = "2026-01-01T00:00:00Z"
    sweep, host, _ = make(lane, now=lambda: datetime(2026, 11, 7, tzinfo=timezone.utc))
    assert acts(sweep.tick()) == [{"draft": "draft", "foreign": "foreign-head", "stale": "stale-pr"}[change]]
    assert host.spawned == []


def test_missing_objects_wait_for_owner_sync(lane):
    lane.gateway.pr["head"]["sha"] = "f" * 40
    synced = []
    sweep, host, _ = make(lane)
    assert acts(sweep.tick()) == ["needs-source-sync"]
    sweep.sync = lambda number, base, head: synced.append((number, base, head))
    assert acts(sweep.tick()) == ["needs-source-sync"] and synced and host.spawned == []


def test_reviewer_identity_rejects_other_prs_zombies_and_unrelated_commands():
    command = " ".join(module.run_review_command(5, 1, "a" * 40, "/k")[1:])
    assert reviewing([(42, 42, "S", command)]) == {5}
    assert reviewing([(42, 42, "Z", command)]) == set()
    assert reviewing([(42, 42, "S", command.replace("--pr 5 ", "--pr 50 "))]) == {50}
    assert reviewing([(42, 42, "S", "-m pilot1742.external_review review-pr --pr 7 --attempt 1")]) == {7}
    assert reviewing([(42, 42, "S", "vim notes.txt")]) == set()


def test_run_review_refuses_when_head_moved(monkeypatch):
    """F5：子进程绑定决策时的 head；head 已变就不审（不登记、不派模型）。"""
    called = []
    monkeypatch.setattr("pilot1742.external_review.main", lambda argv: called.append(argv) or 0)
    class GitHub:
        def pr(self, number):
            return {"head": "b" * 40}
    assert module.run_review(["--pr", "5", "--attempt", "1", "--expect-head", "a" * 40, "--private-key", "/k"],
                             GitHub()) == 3
    assert called == []
    assert module.run_review(["--pr", "5", "--attempt", "1", "--expect-head", "b" * 40, "--private-key", "/k"],
                             GitHub()) == 0
    assert called == [["review-pr", "--pr", "5", "--attempt", "1", "--private-key", "/k"]]


def test_behind_head_is_caught_up_before_review(lane):
    """F4：启用合入时，落后的 PR 先追平、再审新 head，不审一个合入前必然被替换的 head。"""
    github = FakeGitHub(lane)
    github.states[1810] = {"mergeable_state": "behind"}
    sweep, host, _ = make(lane, github=github, auto_merge=True)
    actions = sweep.tick()
    assert acts(actions) == ["behind-awaiting-update", "update-branch"] and host.spawned == []
    assert github.calls == [("update", 1810, lane.gateway.pr["head"]["sha"])]


# ---- 合入 ----

def test_no_merge_unless_ci_ok_is_required(lane):
    github = FakeGitHub(lane, required=("deepagents/review",))
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    green(github, lane.gateway.pr["head"]["sha"])
    actions = sweep.tick()
    assert any(a["action"] == "auto-merge-unsafe" for a in actions)
    assert not [c for c in github.calls if c[0] == "merge"]


def test_merges_only_exact_head_with_review_ci_ok_and_devskim_green(lane):
    """F1：不预挂 auto-merge；精确 head 三项全绿、清单可判定且非人工确认类时，带 sha 直接合入。"""
    github = FakeGitHub(lane)
    head = lane.gateway.pr["head"]["sha"]
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    green(github, head, DevSkim="pending")
    assert "waiting" in acts(sweep.tick())
    assert github.calls == []
    green(github, head)
    assert "merged" in acts(sweep.tick())
    assert github.calls == [("merge", 1810, head)]


def test_moved_head_is_not_merged(lane):
    github = FakeGitHub(lane)
    github.states[1810] = {"head": "0" * 40}
    green(github, "0" * 40)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    assert "head-moved" in acts(sweep.tick()) and github.calls == []


@pytest.mark.parametrize(("path", "category"), [
    ("rust/src/bin/x_execution_session.rs", "live-trading-or-funds"),   # 用户原例：下单逻辑
    ("rust/src/economic_session/book.rs", "live-trading-or-funds"),     # 账本
    ("trading_system/config/broker_config.py", "live-trading-or-funds"),  # F3：IBKR/Hyperliquid 实盘配置
    ("analysis/buy_28316_warrant.py", "live-trading-or-funds"),         # F3：TWS 7496 下单脚本
    (".github/workflows/ci.yml", "ci-or-branch-protection"),
    ("scripts/devskim_sarif_gate.sh", "ci-or-branch-protection"),       # F3：DevSkim 门本体
    ("scripts/devskim_reviewed_false_positives.py", "ci-or-branch-protection"),
    ("deploy/.env.production", "secrets-or-credentials"),
])
def test_human_gated_pr_is_never_merged_and_armed_auto_merge_is_withdrawn(lane, path, category):
    github = FakeGitHub(lane)
    head = lane.gateway.pr["head"]["sha"]
    github.states[1810] = {"auto_merge": True, "changed_files": 2}
    github.files[1810] = [{"filename": ".sandcastle/a.py", "status": "modified", "changes": 1},
                          {"filename": path, "status": "modified", "changes": 1}]
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    result = next(a for a in sweep.tick() if a["action"] == "needs-human-confirm")
    assert result["gated"] == {category: [path]}
    assert github.calls == [("disable", 1810)]


@pytest.mark.parametrize("path", [
    ".sandcastle/deepagents-pilot/pilot1742/external_review.py", ".sandcastle/codex-child.ts",
    "docs/adr/0061-x.md", ".github/copilot-instructions.md", ".github/PULL_REQUEST_TEMPLATE.md",
    "topological-computation/frontend/src/tokens.ts", "rust/src/diagnostic_fill.rs", "docs/ci_secrets.md",
    "src/newchan/data_ibkr.py", "rust/src/bin/s_structure_session.rs", "analysis/some_research.py"])
def test_ordinary_code_and_docs_are_not_human_gated(path):
    assert human_gated([path]) == {}


def test_truncated_file_list_falls_back_to_exact_local_diff(lane):
    """F2：接口清单达到 3000 上限或与 changed_files 不符时不放行；用本地精确 diff 看到排在后面的人工确认类文件。"""
    (lane.source / ".github" / "workflows").mkdir(parents=True)
    (lane.source / ".github" / "workflows" / "ci.yml").write_text("on: push\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "touch ci")
    head = git(lane.source, "rev-parse", "HEAD")
    lane.gateway.pr["head"]["sha"] = head
    github = FakeGitHub(lane)
    github.states[1810] = {"changed_files": 3001}
    github.files[1810] = [{"filename": f".chanlun/review-results/a/f{i:05d}.json", "status": "removed", "changes": 1}
                          for i in range(3000)]
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    result = next(a for a in sweep.tick() if a.get("pr") == 1810 and a["action"] == "needs-human-confirm")
    assert ".github/workflows/ci.yml" in result["gated"]["ci-or-branch-protection"]
    assert not [c for c in github.calls if c[0] == "merge"]


def test_incomplete_file_list_without_local_objects_is_evidence_insufficient(lane):
    github = FakeGitHub(lane)
    github.states[1810] = {"changed_files": 3001, "base": "e" * 40, "auto_merge": True}
    github.files[1810] = [{"filename": f"x/{i}", "status": "added", "changes": 1} for i in range(3000)]
    green(github, lane.gateway.pr["head"]["sha"])
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    result = next(a for a in sweep.tick() if a["action"] == "evidence-insufficient")
    assert "文件清单不全" in result["reason"]
    assert github.calls == [("disable", 1810)]  # 不合入；挂着的 auto-merge 撤掉；不升级成人工确认


def _baseline(keys):
    return json.dumps({"schema": "devskim-baseline/v1", "findings": keys})


@pytest.mark.parametrize(("base_keys", "head_keys", "gated"), [
    (["A|x|1", "B|y|2"], ["A|x|1"], False),        # 只删条目（文件出仓）：不放宽
    (["A|x|1"], ["A|x|1", "C|z|3"], True),         # 新增被放行的发现：放宽，须确认
    (["A|x|139"], ["A|x|138"], False),             # 同一发现行号迁移：不放宽
    (["A|x|1"], ["A|x|1", "A|x|9"], True),         # 同一文件同一规则多放行一条：放宽
])
def test_devskim_baseline_is_gated_only_when_admission_widens(lane, base_keys, head_keys, gated):
    github = FakeGitHub(lane)
    head, base = lane.gateway.pr["head"]["sha"], lane.gateway.pr["base"]["sha"]
    path = ".github/devskim-baseline.json"
    github.files[1810] = [{"filename": path, "status": "modified", "changes": 1}]
    github.contents = {(head, path): _baseline(head_keys), (base, path): _baseline(base_keys)}
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    actions = acts(sweep.tick())
    assert ("needs-human-confirm" in actions) == gated and ("merged" in actions) == (not gated)


@pytest.mark.parametrize(("status", "changes", "gated"), [("renamed", 0, False), ("renamed", 3, True)])
def test_baseline_identity_follows_only_provably_equivalent_renames(lane, status, changes, gated):
    """F7：内容完全相同的纯改名，基线条目跟随新路径、不算放宽；改名且改了内容的照常算新增。"""
    github = FakeGitHub(lane)
    head, base = lane.gateway.pr["head"]["sha"], lane.gateway.pr["base"]["sha"]
    path = ".github/devskim-baseline.json"
    github.states[1810] = {"changed_files": 2}
    github.files[1810] = [{"filename": path, "status": "modified", "changes": 1},
                          {"filename": "new/a.py", "previous_filename": "old/a.py", "status": status, "changes": changes}]
    github.contents = {(head, path): _baseline(["DS1|new/a.py|5"]), (base, path): _baseline(["DS1|old/a.py|5"])}
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    assert ("needs-human-confirm" in acts(sweep.tick())) == gated


@pytest.mark.parametrize("failure", ["read-error", "malformed"])
def test_unreadable_or_malformed_admission_files_never_merge(lane, failure):
    """F2b：读取失败（非 404）是证据不足，不当作文件不存在；内容读不懂按放宽交人工确认。"""
    github = FakeGitHub(lane)
    head, base = lane.gateway.pr["head"]["sha"], lane.gateway.pr["base"]["sha"]
    path = ".github/devskim-reviewed-1488.json"
    github.files[1810] = [{"filename": path, "status": "modified", "changes": 1}]
    github.contents = {(base, path): json.dumps({"files": []}),
                       (head, path): EvidenceInsufficient("HTTP 502") if failure == "read-error" else '{"schema": 1}'}
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    expected = "evidence-insufficient" if failure == "read-error" else "needs-human-confirm"
    assert expected in acts(sweep.tick())
    assert not [c for c in github.calls if c[0] == "merge"]


def test_404_admission_file_is_absent():
    assert admission_keys(".github/devskim-baseline.json", None) == {}


def test_one_prs_failure_does_not_stop_others(lane):
    """F1：单个 PR 的异常隔离，不中止其他 PR 的处理。"""
    github = FakeGitHub(lane)
    other = copy.deepcopy(lane.gateway.pr)
    other["number"] = 1805
    github.extra_pulls = [other]
    github.fail[1805] = {"pr"}
    head = lane.gateway.pr["head"]["sha"]
    green(github, head)
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    actions = sweep.tick()
    assert acts(actions, 1805) == ["error"]
    assert "merged" in acts(actions, 1810)


def test_main_loop_survives_tick_errors(monkeypatch, capsys):
    """F1b：整轮失败只记下，循环不退出。"""
    ticks = []
    class Boom(Sweep):
        def __init__(self, *args, **kwargs):
            pass
        def tick(self):
            ticks.append(1)
            if len(ticks) == 1:
                raise KeyError("files")
            raise SystemExit(0)  # 第二轮到达即说明循环活下来了
    monkeypatch.setattr(module, "Sweep", Boom)
    monkeypatch.setattr(module.time, "sleep", lambda s: None)
    with pytest.raises(SystemExit):
        module.main(["--private-key", "/k", "--loop", "180"])
    assert len(ticks) == 2 and "tick-error" in capsys.readouterr().out


def test_behind_pr_with_failed_or_inflight_review_is_not_caught_up(lane):
    github = FakeGitHub(lane)
    github.states[1810] = {"mergeable_state": "behind"}
    github.checks_by_head[lane.gateway.pr["head"]["sha"]] = {"deepagents/review": "failure"}
    sweep, _, _ = make(lane, github=github, auto_merge=True)
    assert "behind-waiting" in acts(sweep.tick()) and github.calls == []


def test_sweep_carries_no_app_identity_and_uses_shared_repo():
    """F8：sweep 不构造 App 身份（审查身份归 external_review）；仓库名取共享的 workunit.REPO。"""
    from pathlib import Path

    from pilot1742 import workunit
    source = Path(module.__file__).read_text()
    assert "AppIdentity" not in source and "175617978" not in source and "179444545" not in source
    assert module.GhCli().repository == workunit.REPO


@pytest.mark.skipif(not hasattr(_external_review, "_advance_main"), reason="需要 #1913 的补丁沿用与 _advance_main")
def test_main_advance_then_catch_up_costs_no_extra_model_calls(lane, env):
    """组合（独审 test_zz_1939_x_1913）：审过 → main 前进（不重审同 head）→ 先追平 → 新 head 按补丁沿用，模型只调用一轮。"""
    github = FakeGitHub(lane)
    sweep, host, _ = make(lane, github=github, auto_merge=True)
    assert "launch" in acts(sweep.tick())
    first = lane.gateway.pr["head"]["sha"]
    green(github, first, **{"ci-ok": "pending"})
    launches = (lane.state_root / "runs" / "launches").read_text()
    _external_review._advance_main(lane, env)            # main 前进且 PR 追平出新 head（模拟 update-branch 落地）
    caught_up = lane.gateway.pr["head"]["sha"]
    lane.gateway.pr["head"]["sha"] = first               # update 落地前：PR 仍是旧 head，落后
    github.states[1810] = {"mergeable_state": "behind"}
    actions = sweep.tick()
    assert acts(actions) == ["done", "update-branch"]    # 不重审旧 head；先追平
    lane.gateway.pr["head"]["sha"] = caught_up
    github.states[1810] = {"mergeable_state": "clean"}
    actions = sweep.tick()
    assert "launch" in acts(actions)
    assert (lane.state_root / "runs" / "launches").read_text() == launches   # 新 head 沿用，不再调用模型
    # 目录名以 base 开头，按名字排序取 [-1] 是随机的；按 head 找追平后的那个 run
    assert next(r for r in lane.status(1810) if r["target"]["head_sha"] == caught_up)["outcome"].get("reused_from")


# ---- 第二轮独审（d0fa06f9）I1/I2 与 Nit ----

def _advance_main_mid_flight(lane):
    git(lane.source, "checkout", "-q", "-b", "main-moved", lane.gateway.pr["base"]["sha"])
    (lane.source / "unrelated-moved").write_text("main moved\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "main moved")
    lane.gateway.pr["base"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    git(lane.source, "checkout", "-q", lane.gateway.pr["head"]["sha"])


def _stale_mid_flight(lane, github, **kwargs):
    """审查进行中 main 前进：终态判 Stale，GitHub 上该 head 停在 pending，strict 下 PR 落后。"""
    sweep, host, _ = make(lane, github=github, **kwargs)
    head = lane.gateway.pr["head"]["sha"]
    lane.gateway.after_write = lambda: lane.gateway.sent == ["pending"] and _advance_main_mid_flight(lane)
    assert "launch" in acts(sweep.tick())
    lane.gateway.after_write = None
    assert lane.gateway.sent == ["pending"] and lane.status(1810)[-1]["outcome"]["verdict"] == "STALE"
    github.checks_by_head[head] = {"deepagents/review": "pending", "ci-ok": "success", "DevSkim": "success"}
    github.states[1810] = {"mergeable_state": "behind"}
    return sweep, host, head


def test_stale_mid_flight_behind_pr_is_caught_up_not_deadlocked(lane):
    """I1：GitHub 上停住的 pending 不再挡追平；本地没有在途审查就追平，追平后的新 head 由下一轮重审。"""
    github = FakeGitHub(lane)
    sweep, host, head = _stale_mid_flight(lane, github, auto_merge=True)
    assert acts(sweep.tick()) == ["behind-awaiting-update", "update-branch"]
    assert github.calls == [("update", 1810, head)] and len(host.spawned) == 1


def test_live_local_reviewer_still_blocks_catch_up(lane):
    """I1 反向：本地确有该 PR 的审查进程在跑时，落后也不追平。"""
    github = FakeGitHub(lane)
    github.states[1810] = {"mergeable_state": "behind"}
    sweep, host, _ = make(lane, run=False, github=github, auto_merge=True)
    host.rows.append((4999, 4999, "S", " ".join(module.run_review_command(1810, 1, "a" * 40, "/k")[1:])))
    result = next(a for a in sweep.tick() if a["action"] == "behind-waiting")
    assert result["reviewing"] is True and github.calls == []


@pytest.mark.parametrize(("rules", "trusted"), [
    ({"strict": True, "checks": [{"context": "deepagents/review", "app_id": 5486877},
                                 {"context": "ci-ok", "app_id": 15368}]}, {"deepagents/review", "ci-ok"}),
    ({"strict": False, "checks": [{"context": "deepagents/review", "app_id": 5486877},
                                  {"context": "ci-ok", "app_id": 15368}]}, set()),
    ({"strict": True, "checks": [{"context": "deepagents/review", "app_id": 5486877},
                                 {"context": "ci-ok", "app_id": None}, {"context": "x", "app_id": -1}]},
     {"deepagents/review"}),
])
def test_required_checks_count_only_strict_and_app_bound(monkeypatch, rules, trusted):
    """I2：strict 关着一个都不算（会拿旧 base 上的结论合入）；没绑定来源 app 的检查谁都能伪造，不算。"""
    cli = module.GhCli("Newchanlun/NewChanlun")
    monkeypatch.setattr(cli, "_gh", lambda *args: json.dumps(rules))
    assert cli.required_checks() == trusted


def test_child_dying_before_registration_backs_off(lane):
    """Nit 2：子进程登记前就退出，同一 attempt 退避一段时间再派，不每轮派一个必死的子进程。"""
    sweep, host, _ = make(lane, run=False)
    assert acts(sweep.tick()) == ["launch"]
    host.rows.clear()  # 子进程没登记就退出
    assert acts(sweep.tick()) == ["launch-backoff"] and len(host.spawned) == 1
    for marker in (lane.state_root / "sweep").glob("*.launched"):
        old = marker.stat().st_mtime - module.LAUNCH_BACKOFF_SECONDS - 1
        os.utime(marker, (old, old))
    assert acts(sweep.tick()) == ["launch"] and len(host.spawned) == 2


@pytest.mark.parametrize("loop", ["60", "-5", "nan", "inf"])
def test_loop_interval_must_respect_api_rate_limit(loop):
    """r3 Nit 2：每轮每个 PR 约 4 次读接口；循环间隔低于 3 分钟会超过 GitHub 每小时 5000 次的核心限额。
    负数不得被当成单轮，nan 不得混过校验到 sleep 才崩；inf 会让循环永不再跑，同样拒绝。"""
    with pytest.raises(SystemExit):
        module.main(["--private-key", "/k", "--loop", loop])


@pytest.mark.parametrize("path", ["deploy/.envrc", ".netrc", "web/.npmrc", "home/.ssh/id_ed25519", "id_rsa",
                                  "keys/AuthKey_ABC.p8"])
def test_more_credential_files_are_human_gated(path):
    assert human_gated([path]) == {"secrets-or-credentials": [path]}
