import copy
import hashlib
import json
import subprocess
import sys
from dataclasses import asdict
from pathlib import Path
from types import SimpleNamespace

import httpx
import pytest
from conftest import git

from pilot1742 import external_review as module
from pilot1742.delivery import EffectPending, RepoGateway, Stale
from pilot1742.external_review import (
    CONTEXT,
    MODELS,
    TOKEN_PERMISSIONS,
    ExternalReview,
    ReviewTarget,
    StatusPublisher,
    load_target,
    validate_review,
)
from pilot1742.github_app import (
    AppIdentity,
    GitHubAppClient,
    IdentityError,
    OutcomeUnknown,
)


class Gateway:
    bot = "newchanlun-deepagents-1742[bot]"
    def __init__(self, env):
        self.client = SimpleNamespace(identity=SimpleNamespace(repository_id=1150335160))
        self.pr = {"number": 1810, "state": "open", "draft": False, "user": {"login": "business-owner"},
                   "base": {"repo": {"id": 1150335160}, "ref": "main", "sha": env.base},
                   "head": {"repo": {"id": 1150335160}, "ref": "feature/business", "sha": git(env.source, "rev-parse", "HEAD")}}
        self.rows, self.sent = [], []
        self.fault, self.hide, self.after_write = False, False, None
    def get_pr(self, number):
        assert number == 1810
        return copy.deepcopy(self.pr)
    def statuses(self, sha):
        if self.hide:
            return []
        return copy.deepcopy([r for r in self.rows if r["sha"] == sha])
    def set_status(self, sha, context, state, description):
        self.sent.append(state)
        if self.fault:
            raise OutcomeUnknown("已发送结果不明")
        self.rows.insert(0, {"id": len(self.rows) + 1, "sha": sha, "context": context, "state": state,
                             "description": description, "creator": {"login": self.bot, "type": "Bot"}})
        if self.after_write:
            self.after_write()


@pytest.fixture
def lane(env, monkeypatch):
    for i in range(2):
        (env.source / f"changed-{i}").write_text(str(i))
        git(env.source, "add", ".")
        git(env.source, "commit", "-qm", f"change-{i}")
    monkeypatch.setattr(module, "SOURCE_REPO", env.source)
    gateway = Gateway(env)
    return ExternalReview(gateway, source=env.source, state_root=env.home / "review",
                          checkout_root=env.home / "clones", poll_seconds=0.01,
                          launcher=[sys.executable, str(Path(__file__).with_name("external_review_child.py"))])


def test_business_owner_multi_commit_pr_and_idempotent_resume(lane):
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS"
    assert [r["model"] for r in result["reviews"]] == list(MODELS)
    assert len({r["session_id"] for r in result["reviews"]}) == 2
    assert lane.gateway.sent == ["pending", "success"]
    target = ReviewTarget(**result["target"])
    for model in MODELS:
        folder = lane.state_root / "runs" / target.key / model
        launch = json.loads((folder / "launch.json").read_text())
        assert "changed-0" in launch["prompt"] and "changed-1" in launch["prompt"]
        assert target.merge_base_sha == lane.gateway.pr["base"]["sha"]
        assert not Path(launch["checkout"]).exists()
        assert (folder / "events.jsonl").is_file() and (folder / "session.jsonl").is_file()
    before = sorted(lane.checkout_root.glob("*/*"))
    assert lane.review_pr(1810)["run_id"] == result["run_id"]
    assert sorted(lane.checkout_root.glob("*/*")) == before
    assert lane.gateway.sent == ["pending", "success"]


@pytest.mark.parametrize("draft", [True, False])
def test_draft_and_non_app_author_are_accepted(lane, draft):
    lane.gateway.pr["draft"] = draft
    assert load_target(lane.gateway, 1810)["head_ref"] == "feature/business"


@pytest.mark.parametrize("change", [lambda p: p.update(state="closed"),
    lambda p: p["base"].update(ref="other"), lambda p: p["head"]["repo"].update(id=9),
    lambda p: p["head"].update(sha="missing"), lambda p: p.pop("draft")])
def test_invalid_target_does_not_launch_or_publish(lane, change):
    change(lane.gateway.pr)
    with pytest.raises(IdentityError):
        lane.review_pr(1810)
    assert lane.gateway.sent == []
    assert not lane.checkout_root.exists()


def test_missing_source_objects_fail_before_pending(lane):
    lane.gateway.pr["head"]["sha"] = "f" * 40
    with pytest.raises(IdentityError, match="needs_source_sync"):
        lane.review_pr(1810)
    assert not lane.gateway.sent


def test_complete_block_maps_failure_and_is_not_retried(lane):
    lane.target(1810, 1)
    (lane.state_root / "runs" / "fake-mode").write_text("BLOCK")
    assert lane.review_pr(1810)["verdict"] == "BLOCK"
    assert lane.gateway.sent == ["pending", "failure"]
    assert lane.review_pr(1810)["verdict"] == "BLOCK"
    assert lane.gateway.sent == ["pending", "failure"]


@pytest.mark.parametrize("field", ["base", "head"])
def test_drift_prevents_publish_even_if_done(lane, field):
    target = lane.target(1810, 1)
    folder = lane.state_root / "runs" / target.key
    publisher = StatusPublisher(lane.gateway, target, folder, lane.state_root / "locks")
    publisher.publish("success")
    lane.gateway.pr[field]["sha"] = "e" * 40
    with pytest.raises(Stale):
        publisher.publish("success")
    assert lane.gateway.sent == ["success"]


def test_post_write_drift_is_stale_not_success(lane):
    lane.gateway.after_write = lambda: lane.gateway.pr["head"].update(sha="e" * 40)
    result = lane.review_pr(1810)
    assert result["verdict"] == "STALE"
    assert lane.gateway.sent == ["pending"]


def test_unknown_write_only_reconciles_without_republish(lane):
    target = lane.target(1810, 1)
    lane.gateway.fault = True
    for _ in range(2):
        with pytest.raises(EffectPending):
            lane.review_pr(1810)
    assert lane.gateway.sent == ["pending"]
    lane.reconcile(target.run_id, "已核远端请求确未生效")
    lane.gateway.fault = False
    assert lane.review_pr(1810)["verdict"] == "PASS"
    assert lane.gateway.sent == ["pending", "pending", "success"]


def test_newer_failure_blocks_old_success_and_done(lane):
    lane.review_pr(1810)
    lane.gateway.rows.insert(0, {**lane.gateway.rows[0], "id": 999, "state": "failure"})
    with pytest.raises(EffectPending):
        lane.review_pr(1810)
    assert lane.gateway.sent == ["pending", "success"]


def test_upload_pass_is_not_a_launch_and_missing_supervisor_never_redispatches(lane):
    target = lane.target(1810, 1)
    folder = lane.state_root / "runs" / target.key
    worker = folder / MODELS[0]
    worker.mkdir()
    (worker / "result.json").write_text('{"summary":"VERDICT: PASS"}')
    (worker / "launch.json").write_text(json.dumps({"model": MODELS[0], "launch_id": "test"}))
    for _ in range(2):
        with pytest.raises(EffectPending):
            lane.review_pr(1810)
    assert lane.gateway.sent == ["pending"]
    assert not lane.checkout_root.exists()


@pytest.mark.parametrize("mutation", [lambda r: r.update(status="failed"), lambda r: r.update(status="cancelled"),
    lambda r: r.update(turn_completed=False), lambda r: r.update(exit_code=1),
    lambda r: r.update(model="other"), lambda r: r.update(session_id="wrong"),
    lambda r: r.pop("filesystem"), lambda r: r["filesystem"].update(source_write="ALLOWED"),
    lambda r: r["after"].update(status=" M README"), lambda r: r.update(result={"summary": "VERDICT: PASS"})])
def test_terminal_receipt_requires_supervision_evidence(lane, mutation):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key / MODELS[0]
    launch = json.loads((folder / "launch.json").read_text())
    receipt = json.loads((folder / "receipt.json").read_text())
    mutation(receipt)
    assert validate_review(target, launch, receipt, folder).verdict == "FAILED"


RECONNECT_CASES = json.loads((Path(__file__).parent / "fixtures" / "codex-reconnect.json").read_text())


@pytest.mark.parametrize("scenario", RECONNECT_CASES["cases"], ids=lambda case: case["name"])
def test_reconnect_protocol_matches_child_supervision(lane, scenario):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key / MODELS[0]
    launch = json.loads((folder / "launch.json").read_text())
    receipt = json.loads((folder / "receipt.json").read_text())
    session = RECONNECT_CASES["session_id"]
    events = [{"type": "thread.started", "thread_id": session}, *scenario["events"]]
    (folder / "events.jsonl").write_text("\n".join(json.dumps(e) for e in events) + "\n")
    (folder / "session.jsonl").write_text(json.dumps({"type": "session_meta", "payload": {"id": session}}) + "\n")
    receipt.update(session_id=session, result=scenario["result"], exit_code=scenario["exit_code"])
    outcome = validate_review(target, launch, receipt, folder)
    assert outcome.verdict == scenario["verdict"], outcome.reason
    assert outcome.session_id == (None if scenario["verdict"] == "FAILED" else session)


def test_symlink_session_cannot_prove_pass(lane, tmp_path):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key / MODELS[0]
    session = folder / "session.jsonl"
    other = tmp_path / "uploaded-pass"
    session.rename(other)
    session.symlink_to(other)
    assert validate_review(target, json.loads((folder / "launch.json").read_text()),
                           json.loads((folder / "receipt.json").read_text()), folder).verdict == "FAILED"


def test_status_pagination_error_cannot_mean_absent_or_publish(lane):
    lane.gateway.statuses = lambda sha: (_ for _ in ()).throw(IdentityError("分页失败"))
    with pytest.raises(IdentityError):
        lane.review_pr(1810)
    assert not lane.gateway.sent


def test_set_status_retains_old_success_api():
    requests = []
    client = SimpleNamespace(identity=SimpleNamespace(repository="owner/repo"), bot_login="app[bot]",
                             api=lambda method, path, **kw: requests.append((method, path, kw)))
    gateway = RepoGateway(client)
    gateway.create_status("a" * 40, "old", "description")
    gateway.set_status("b" * 40, CONTEXT, "error", "review failed")
    assert [r[2]["json"]["state"] for r in requests] == ["success", "error"]
    assert "target_url" not in requests[1][2]["json"]
    with pytest.raises(ValueError):
        gateway.set_status("b" * 40, CONTEXT, "neutral", "bad")


def test_token_requests_only_external_review_permissions(tmp_path, monkeypatch):
    identity = AppIdentity(5486877, 179444545, 1150335160, "Newchanlun/NewChanlun",
                           "newchanlun-deepagents-1742", tmp_path / "unused.pem")
    captured = []
    import pilot1742.github_app as app
    monkeypatch.setattr(app, "_read_private_key", lambda path: b"fixture")
    monkeypatch.setattr(app.jwt, "encode", lambda *a, **kw: "fixture-jwt")
    def transport(request):
        if request.url.path == "/app":
            value = {"id": identity.app_id, "slug": identity.app_slug}
        elif request.url.path.endswith("/access_tokens"):
            captured.append(json.loads(request.content))
            value = {"token": "fixture", "permissions": TOKEN_PERMISSIONS, "expires_at": "2099-01-01T00:00:00Z"}
        else:
            value = {"id": identity.installation_id, "app_id": identity.app_id, "app_slug": identity.app_slug,
                     "account": {"login": "Newchanlun"}, "repository_selection": "selected",
                     "suspended_at": None, "permissions": app.INSTALLATION_PERMISSIONS}
        return httpx.Response(200, json=value)
    with GitHubAppClient(identity, permissions=TOKEN_PERMISSIONS, transport=httpx.MockTransport(transport)) as client:
        client._ensure_token()
    assert captured == [{"repository_ids": [1150335160], "permissions": {
        "metadata": "read", "contents": "read", "pull_requests": "read", "statuses": "write"}}]


def test_recover_after_spawn_without_second_model_call(lane, monkeypatch):
    original = module._save
    crashed = False
    def save(path, value):
        nonlocal crashed
        if path.name == "process.json" and not crashed:
            crashed = True
            raise KeyboardInterrupt("crash after spawn before process record")
        return original(path, value)
    monkeypatch.setattr(module, "_save", save)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS"
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()) == sorted(MODELS)
    assert lane.gateway.sent == ["pending", "success"]


def test_new_attempt_supersedes_inflight_old_run(lane):
    first = lane.target(1810, 1)
    second = lane.target(1810, 2)
    old = StatusPublisher(lane.gateway, first, lane.state_root / "runs" / first.key, lane.state_root / "locks")
    new = StatusPublisher(lane.gateway, second, lane.state_root / "runs" / second.key, lane.state_root / "locks")
    old.publish("pending")
    new.publish("pending")
    new.publish("failure")
    with pytest.raises(Stale):
        old.publish("success")
    with pytest.raises(Stale):
        old.publish("pending")
    assert lane.gateway.sent == ["pending", "pending", "failure"]


def test_status_transport_pages_are_complete_and_failure_does_not_publish(lane):
    requests = []
    pr = lane.gateway.pr
    identity = AppIdentity(5486877, 179444545, 1150335160, "Newchanlun/NewChanlun",
                           "newchanlun-deepagents-1742", Path("/unused-fixture.pem"))
    def transport(request):
        requests.append(request)
        if request.url.path.endswith("/pulls/1810"):
            return httpx.Response(200, json=pr)
        page = int(request.url.params.get("page", "1"))
        if page == 1:
            return httpx.Response(200, json=[], headers={"Link": '<https://api.github.com/x?page=2>; rel="next"'})
        return httpx.Response(500, json={"error": "fixture"})
    with GitHubAppClient(identity, permissions=TOKEN_PERMISSIONS, transport=httpx.MockTransport(transport)) as client:
        client._token = "fixture"
        client._expires_at = 99999999999
        lane.gateway = RepoGateway(client)
        with pytest.raises(IdentityError):
            lane.review_pr(1810)
    assert len([r for r in requests if r.url.path.endswith("/statuses")]) == 2
    assert all(r.method == "GET" for r in requests)


def test_older_unstarted_target_cannot_supersede_newer_attempt(lane):
    first = lane.target(1810, 1)
    second = lane.target(1810, 2)
    new = StatusPublisher(lane.gateway, second, lane.state_root / "runs" / second.key, lane.state_root / "locks")
    new.publish("pending")
    old = StatusPublisher(lane.gateway, first, lane.state_root / "runs" / first.key, lane.state_root / "locks")
    with pytest.raises(Stale):
        old.publish("pending")
    assert lane.gateway.sent == ["pending"]


def test_publication_rechecks_controller_code(lane):
    target = lane.target(1810, 1)
    publisher = StatusPublisher(lane.gateway, target, lane.state_root / "runs" / target.key, lane.state_root / "locks")
    git(lane.source, "commit", "--allow-empty", "-qm", "controller advanced")
    with pytest.raises(Stale, match="controller"):
        publisher.publish("success")
    assert not lane.gateway.sent


@pytest.mark.parametrize("args", [["--help"], ["review-pr", "--help"], ["status", "--help"]])
def test_cli_help_is_offline(args):
    with pytest.raises(SystemExit) as caught:
        module.main(args)
    assert caught.value.code == 0


@pytest.mark.parametrize("number", ["0", "-1", "abc"])
def test_cli_rejects_invalid_pr_before_auth(number):
    with pytest.raises(SystemExit) as caught:
        module.main(["review-pr", "--pr", number, "--private-key", "/never-read.pem"])
    assert caught.value.code == 2


def test_cli_status_uses_no_auth(lane, monkeypatch, capsys):
    monkeypatch.setattr(module, "ExternalReview", lambda gateway: lane)
    monkeypatch.setattr(module, "GitHubAppClient", lambda *a, **kw: pytest.fail("status must stay offline"))
    assert module.main(["status", "--pr", "1810"]) == 0
    assert json.loads(capsys.readouterr().out) == []


@pytest.mark.parametrize("history_count", [1, 4])
def test_snapshot_omits_obsolete_blobs_but_preserves_graph_and_diff(lane, tmp_path, history_count):
    import os
    old = lane.source / "obsolete.bin"
    old_blobs = []
    for i in range(history_count):
        old.write_bytes(os.urandom(2 * 1024 * 1024))
        git(lane.source, "add", ".")
        git(lane.source, "commit", "-qm", f"unrelated old payload {i}")
        old_blobs.append(git(lane.source, "rev-parse", "HEAD:obsolete.bin"))
    old.unlink()
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "remove old payload")
    base = git(lane.source, "rev-parse", "HEAD")
    git(lane.source, "checkout", "-qb", "candidate")
    (lane.source / "later").write_text("new PR\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "PR first")
    (lane.source / "later").write_text("whole PR\n")
    git(lane.source, "commit", "-am", "PR second", "-q")
    head = git(lane.source, "rev-parse", "HEAD")
    git(lane.source, "checkout", "-q", "main")
    (lane.source / "base-only").write_text("main advanced\n")
    git(lane.source, "add", ".")
    git(lane.source, "commit", "-qm", "base advanced")
    lane.gateway.pr["base"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    lane.gateway.pr["head"]["sha"] = head
    target = lane.target(1810, 1)
    checkout = tmp_path / "snapshot"
    frozen = module.prepare_review(target, lane.source, checkout)
    assert frozen.changed_files == ("later",)
    assert git(checkout, "status", "--porcelain", "--untracked-files=all") == ""
    assert git(checkout, "rev-parse", "--abbrev-ref", "HEAD") == "HEAD"
    assert git(checkout, "merge-base", target.base_sha, head) == base
    assert git(checkout, "diff", "--binary", base, head) == git(lane.source, "diff", "--binary", base, head)
    assert git(checkout, "log", "--format=%H", head) == git(lane.source, "log", "--format=%H", head)
    import subprocess
    for old_blob in old_blobs:
        assert subprocess.run(["git", "-C", str(checkout), "cat-file", "-e", old_blob], capture_output=True).returncode != 0
    assert "remote" not in (checkout / ".git/config").read_text()
    assert not (checkout / ".git/objects/info/alternates").exists()
    assert not (checkout / ".git/hooks").exists()
    assert sum(p.stat().st_size for p in (checkout / ".git/objects").rglob("*.pack")) < 128 * 1024


def test_terminal_reclaims_only_checkout_and_retains_evidence(lane):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    for model in MODELS:
        worker = lane.state_root / "runs" / target.key / model
        assert not Path(module._read(worker / "launch.json")["checkout"]).exists()
        for name in ("launch.json", "receipt.json", "events.jsonl", "session.jsonl"):
            assert (worker / name).is_file()
    assert lane.review_pr(1810)["verdict"] == "PASS"
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()) == sorted(MODELS)


@pytest.mark.parametrize("crash_point", ["during-prepare", "after-prepare"])
def test_prepare_interruption_recovers_without_duplicate_dispatch(lane, monkeypatch, crash_point):
    original = module.prepare_review
    crashed = False
    def prepare(target, source, checkout):
        nonlocal crashed
        if not crashed:
            crashed = True
            if crash_point == "during-prepare":
                checkout.mkdir(parents=True)
                (checkout / "partial").write_text("interrupted snapshot")
            else:
                original(target, source, checkout)
            raise KeyboardInterrupt("prepare interrupted")
        return original(target, source, checkout)
    monkeypatch.setattr(module, "prepare_review", prepare)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    assert lane.review_pr(1810)["verdict"] == "PASS"
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()) == sorted(MODELS)


def test_fresh_reviewers_run_concurrently_and_any_failure_fails_the_run(lane):
    lane.target(1810, 1)
    (lane.state_root / "runs" / "fake-mode").write_text("failed")
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()) == sorted(MODELS)
    assert not list(lane.checkout_root.iterdir())


def test_reviewers_overlap_in_time(lane, monkeypatch):
    import threading
    started, both = [], threading.Event()
    real = lane.run_review
    def run_review(target, model, folder):
        started.append(model)
        if len(started) == len(MODELS):
            both.set()
        assert both.wait(30), "第二位 reviewer 没有在第一位结束前启动"
        return real(target, model, folder)
    monkeypatch.setattr(lane, "run_review", run_review)
    assert lane.review_pr(1810)["verdict"] == "PASS"


def test_resume_after_first_failure_stays_serial(lane):
    target = lane.target(1810, 1)
    (lane.state_root / "runs" / target.key / MODELS[0]).mkdir()
    (lane.state_root / "runs" / "fake-mode").write_text("failed")
    assert lane.review_pr(1810)["verdict"] == "FAILED"
    assert (lane.state_root / "runs" / "launches").read_text().splitlines() == [MODELS[0]]


def test_exited_supervisor_before_receipt_is_terminal_not_pending(lane):
    lane.launcher = [sys.executable, "-c", "raise SystemExit(3)"]
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert lane.gateway.sent == ["pending", "error"]
    assert lane.review_pr(1810)["verdict"] == "FAILED"


def test_stale_publication_preserves_completed_reviews(lane):
    def drift_after_success():
        if lane.gateway.sent[-1] == "success":
            lane.gateway.pr["head"]["sha"] = "e" * 40
    lane.gateway.after_write = drift_after_success
    result = lane.review_pr(1810)
    assert result["verdict"] == "STALE"
    assert [r["verdict"] for r in result["reviews"]] == ["PASS", "PASS"]


def test_low_space_refuses_registration_before_attempt_or_status(lane, monkeypatch):
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))
    with pytest.raises(IdentityError, match="空间不足：2 份并发快照共需 .*可用 1 bytes；未登记 attempt"):
        lane.review_pr(1810)
    assert not list((lane.state_root / "runs").glob("*/target.json"))
    assert lane.gateway.sent == []
    assert not lane.checkout_root.exists()


def test_admission_counts_every_reviewer_snapshot(lane, monkeypatch):
    data = module.load_target(lane.gateway, 1810)
    merge_base = git(lane.source, "merge-base", data["base_sha"], data["head_sha"])
    probe = module.ReviewTarget(**data, merge_base_sha=merge_base, run_id="probe", attempt=1,
                                policy_version=module.POLICY, controller_code_head="x", registered_at_ns=0)
    _, single = module.snapshot_plan(lane.source, probe)
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=single * len(MODELS) - 1))
    with pytest.raises(IdentityError, match=f"共需 {single * len(MODELS)} bytes"):
        lane.review_pr(1810)
    assert lane.gateway.sent == []


def test_cleanup_preserves_live_or_unknown_activity(lane, monkeypatch):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key
    worker = folder / MODELS[0]
    checkout = lane.checkout_root / target.run_id / MODELS[0]
    checkout.mkdir(parents=True)
    (checkout / "retained").write_text("active source")
    monkeypatch.setattr(module, "_ps_rows", lambda: [(8123, 8123, "S", f"unknown-tool {checkout}")])
    assert not lane.cleanup_checkout(target, MODELS[0], folder)
    assert (checkout / "retained").exists()
    monkeypatch.setattr(module, "_ps_rows", lambda: [])
    assert lane.cleanup_checkout(target, MODELS[0], folder)
    assert (worker / "session.jsonl").is_file()


def test_cleanup_preserves_unconfirmed_container(lane):
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key
    worker = folder / MODELS[0]
    checkout = lane.checkout_root / target.run_id / MODELS[0]
    checkout.mkdir(parents=True)
    receipt = module._read(worker / "receipt.json")
    receipt["container_closed"] = False
    module._save(worker / "receipt.json", receipt)
    assert not lane.cleanup_checkout(target, MODELS[0], folder)
    assert checkout.exists()


def test_multiple_attempts_reclaim_each_checkout(lane):
    for attempt in (1, 2, 3):
        assert lane.review_pr(1810, attempt)["verdict"] == "PASS"
        assert not list(lane.checkout_root.iterdir())
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()) == sorted(list(MODELS) * 3)
    assert len(list((lane.state_root / "runs").glob("*/outcome.json"))) == 3


def test_no_intent_never_grants_checkout_deletion(lane):
    target = lane.target(1810, 1)
    checkout = lane.checkout_root / target.run_id / MODELS[0]
    checkout.mkdir(parents=True)
    assert not lane.cleanup_checkout(target, MODELS[0], lane.state_root / "runs" / target.key)
    assert checkout.exists()



def test_failed_spawn_has_terminal_receipt_and_reclaims_checkout(lane):
    lane.launcher = ["/nonexistent-fixture-launcher"]
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert lane.gateway.sent == ["pending", "error"]
    assert not list(lane.checkout_root.iterdir())
    target = ReviewTarget(**result["target"])
    worker = lane.state_root / "runs" / target.key / MODELS[0]
    assert module._read(worker / "receipt.json")["failure_reason"] == "SupervisorNotStarted"


def test_space_exhausted_after_prepare_stops_before_model(lane, monkeypatch):
    def disk_usage(path):  # 只有模型启动前对 checkout 本身的复查看到空间耗尽
        return SimpleNamespace(free=1 if Path(path).name in MODELS else 10**12)
    monkeypatch.setattr(module.shutil, "disk_usage", disk_usage)
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert "模型启动前" in result["reviews"][0]["reason"]
    assert not (lane.state_root / "runs" / "launches").exists()
    assert not list(lane.checkout_root.iterdir())


def _launches(lane):
    path = lane.state_root / "runs" / "launches"
    return len(path.read_text().splitlines()) if path.exists() else 0


def _advance_main(lane, env, *, conflict=False, rewrite=None):
    """main 前进后把 main 合进 PR head，模拟 strict 下的追平。"""
    src, head = lane.source, git(lane.source, "rev-parse", "HEAD")
    git(src, "checkout", "-q", "-b", f"main-{len(lane.gateway.rows)}", str(env.base))
    # 每次前进写不同内容，免得同一秒内两次前进生成同一个提交（head 不变就测不到「新 head」）。
    (src / ("changed-0" if conflict else "unrelated")).write_text("main side\n" if conflict else f"main side {len(lane.gateway.rows)}\n")
    git(src, "add", ".")
    git(src, "commit", "-qm", "main advances")
    base = git(src, "rev-parse", "HEAD")
    git(src, "checkout", "-q", head)
    if conflict:
        assert subprocess.run(["git", "-C", str(src), "merge", "-q", "--no-edit", base],
                              capture_output=True).returncode != 0
        (src / "changed-0").write_text("0\nmain side\n")
        git(src, "add", ".")
        git(src, "commit", "-qm", "resolve conflict")
    else:
        git(src, "merge", "-q", "--no-edit", base)
    if rewrite is not None:
        (src / "changed-1").write_text(rewrite)
        git(src, "commit", "-qam", "patch changes")
    lane.gateway.pr["base"]["sha"], lane.gateway.pr["head"]["sha"] = base, git(src, "rev-parse", "HEAD")
    git(src, "checkout", "-q", head)  # 宿主 controller 不随 PR 追平而变，只在部署时变


def test_same_patch_on_new_base_reuses_pass_without_model(lane, env):
    first = lane.review_pr(1810)
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and result["reviews"] == []
    assert result["reused_from"]["run_id"] == first["run_id"]
    assert result["target"]["patch_id"] == first["target"]["patch_id"]
    assert result["target"]["merge_base_sha"] != first["target"]["merge_base_sha"]
    assert _launches(lane) == len(MODELS)
    assert lane.gateway.sent == ["pending", "success", "pending", "success"]
    row = lane.gateway.rows[0]
    assert row["sha"] == lane.gateway.pr["head"]["sha"]
    assert f"reuse={first['run_id']} p={first['target']['patch_id'][7:19]}" in row["description"]
    assert len(row["description"]) <= 140
    assert lane.review_pr(1810)["publication"]["id"] == row["id"]  # 重跑只读回，不重发
    assert lane.gateway.sent == ["pending", "success", "pending", "success"]


# 只改空白也必须重审：git patch-id 会忽略空白，所以补丁身份不用它。
@pytest.mark.parametrize("change", ["patch", "whitespace", "conflict"])
def test_changed_patch_or_conflict_resolution_is_reviewed_again(lane, env, change):
    first = lane.review_pr(1810)
    _advance_main(lane, env, conflict=change == "conflict",
                  rewrite={"patch": "changed again\n", "whitespace": " 1"}.get(change))
    result = lane.review_pr(1810)
    assert result["target"]["patch_id"] != first["target"]["patch_id"]
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)
    assert _launches(lane) == 2 * len(MODELS)


@pytest.mark.parametrize("first_verdict", ["BLOCK", "FAILED"])
def test_non_pass_conclusion_is_not_reused(lane, env, first_verdict):
    launcher = lane.launcher
    if first_verdict == "BLOCK":
        (lane.state_root / "runs").mkdir(parents=True)
        (lane.state_root / "runs" / "fake-mode").write_text("BLOCK")
    else:
        lane.launcher = ["/nonexistent-fixture-launcher"]
    assert lane.review_pr(1810)["verdict"] == first_verdict
    (lane.state_root / "runs" / "fake-mode").unlink(missing_ok=True)
    lane.launcher = launcher
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and "reused_from" not in result
    assert len(result["reviews"]) == len(MODELS)


@pytest.mark.parametrize("field", ["POLICY", "REVIEW_CONTRACT"])
def test_policy_or_contract_change_is_not_reused(lane, env, monkeypatch, field):
    lane.review_pr(1810)
    monkeypatch.setattr(module, field, "changed")
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_new_attempt_on_reused_head_is_reviewed_again(lane, env):
    lane.review_pr(1810)
    _advance_main(lane, env)
    assert "reused_from" in lane.review_pr(1810)
    again = lane.review_pr(1810, 2)
    assert "reused_from" not in again and len(again["reviews"]) == len(MODELS)
    assert _launches(lane) == 2 * len(MODELS)


def test_reuse_needs_no_snapshot_space(lane, env, monkeypatch):
    lane.review_pr(1810)
    _advance_main(lane, env)
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))
    assert "reused_from" in lane.review_pr(1810)


def _second_opinion(lane, env, monkeypatch, verdicts, *, stale=False):
    """同一补丁先双 PASS，再在同一 head 上显式重审得到 verdicts（可选让发布时 head 漂移成 STALE），然后追平 main。"""
    first = lane.review_pr(1810)
    real = lane.run_reviewers
    def scripted(target, folder):
        outcomes = [module.ReviewOutcome(m, v, f"s-{m}" if v != "FAILED" else None, f"VERDICT: {v}")
                    for m, v in zip(MODELS, verdicts, strict=True)]
        if stale:
            _advance_main(lane, env)
        return outcomes
    monkeypatch.setattr(lane, "run_reviewers", scripted)
    second = lane.review_pr(1810, 2)
    monkeypatch.setattr(lane, "run_reviewers", real)
    if not stale:
        _advance_main(lane, env)
    return first, second


def test_reviewer_block_hidden_by_failed_run_still_vetoes_reuse(lane, env, monkeypatch):
    _, second = _second_opinion(lane, env, monkeypatch, ["BLOCK", "FAILED"])
    assert second["verdict"] == "FAILED"
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_reviewer_block_hidden_by_stale_run_still_vetoes_reuse(lane, env, monkeypatch):
    _, second = _second_opinion(lane, env, monkeypatch, ["PASS", "BLOCK"], stale=True)
    assert second["verdict"] == "STALE" and [r["verdict"] for r in second["reviews"]] == ["PASS", "BLOCK"]
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_block_on_legacy_run_without_patch_id_still_vetoes_reuse(lane, env, monkeypatch):
    _, second = _second_opinion(lane, env, monkeypatch, ["BLOCK", "BLOCK"])
    path = lane.state_root / "runs" / ReviewTarget(**second["target"]).key / "target.json"
    legacy = json.loads(path.read_text())
    legacy.pop("patch_id"), legacy.pop("review_contract")
    path.write_text(json.dumps(legacy))
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_interrupted_registration_without_target_json_is_still_admitted(lane, monkeypatch):
    target = lane.target(1810, 1)
    folder = lane.state_root / "runs" / target.key
    (folder / "target.json").unlink()  # _lock 已建目录、target.json 未写时中断
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))
    with pytest.raises(IdentityError, match="未登记 attempt"):
        lane.review_pr(1810)
    assert not (folder / "target.json").exists() and lane.gateway.sent == []


@pytest.mark.parametrize("change", ["controller", "admission-fails", "image"])
def test_reuse_requires_current_deployment_admission(lane, env, change):
    lane.review_pr(1810)
    _advance_main(lane, env)
    if change == "controller":
        git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed")
    else:
        (lane.source / ".git" / "fake-admission").write_text("fail" if change == "admission-fails" else "sha256:" + "2" * 64)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_reuse_records_admission(lane, env):
    lane.review_pr(1810)
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert result["reused_from"]["admission"] == {"controller_code_head": git(lane.source, "rev-parse", "HEAD"),
                                                  "image_id": "sha256:" + "1" * 64}



def _interrupt_before_aggregation(lane, monkeypatch):
    """reviewer 全部写完回执后、汇总写 outcome.json 前控制器中断。"""
    real = lane.run_reviewers
    def interrupted(target, folder):
        real(target, folder)
        raise KeyboardInterrupt
    monkeypatch.setattr(lane, "run_reviewers", interrupted)
    return real


def _modes(lane, **by_model):
    runs = lane.state_root / "runs"
    runs.mkdir(parents=True, exist_ok=True)
    for model, mode in by_model.items():
        (runs / f"fake-mode-{model}").write_text(mode)


def _clear_modes(lane):
    for p in (lane.state_root / "runs").glob("fake-mode-*"):
        p.unlink()


def test_seq_sol_interrupted_reattempt_with_unaggregated_block_vetoes_reuse(lane, env, monkeypatch):
    """D5 c24b9537 Sol：同补丁双 PASS 后显式重审得 [FAILED, BLOCK]，汇总前中断；新 head 不得沿用。"""
    lane.review_pr(1810)
    _modes(lane, **{MODELS[0]: "failed", MODELS[1]: "BLOCK"})
    real = _interrupt_before_aggregation(lane, monkeypatch)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810, 2)
    monkeypatch.setattr(lane, "run_reviewers", real)
    _clear_modes(lane)
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_seq_opus_current_run_with_worker_state_never_reuses(lane, env, monkeypatch):
    """D5 c24b9537 Opus-1：本 run 已有 reviewer BLOCK 回执、未汇总即中断，恢复时不得改走沿用。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    (lane.source / ".git" / "fake-admission").write_text("fail")   # 首轮准入抖动 → 全新双审
    _modes(lane, **{MODELS[0]: "BLOCK"})
    real = _interrupt_before_aggregation(lane, monkeypatch)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    monkeypatch.setattr(lane, "run_reviewers", real)
    (lane.source / ".git" / "fake-admission").unlink()             # 准入恢复
    result = lane.review_pr(1810)
    assert "reused_from" not in result and result["verdict"] == "BLOCK"


def test_seq_opus_in_flight_reattempt_of_same_patch_vetoes_reuse(lane, env):
    """D5 c24b9537 Opus-2：同补丁显式重审仍在途（无终态），新 head 不得沿用。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    assert "reused_from" in lane.review_pr(1810)
    in_flight = lane.target(1810, 2)
    (lane.state_root / "runs" / in_flight.key / MODELS[0]).mkdir()
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_block_run_with_unknown_patch_vetoes_reuse(lane, env, monkeypatch):
    """补丁身份算不出的 BLOCK run 按同补丁处理（fail closed）。"""
    _, second = _second_opinion(lane, env, monkeypatch, ["BLOCK", "BLOCK"])
    path = lane.state_root / "runs" / ReviewTarget(**second["target"]).key / "target.json"
    legacy = json.loads(path.read_text())
    legacy.pop("patch_id"), legacy.pop("review_contract")
    legacy["head_sha"] = "e" * 40                                   # 冻结 head 已不在对象库
    path.write_text(json.dumps(legacy))
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_block_only_in_events_of_failed_receipt_vetoes_reuse(lane, env):
    """#1912 run 75aad755 的形状：receipt 停在 transcript 失败、result=None，BLOCK 只在 events 的最终消息里。"""
    first = lane.review_pr(1810)
    second = lane.target(1810, 2)
    worker = lane.state_root / "runs" / second.key / MODELS[1]
    worker.mkdir(parents=True)
    (worker / "receipt.json").write_text(json.dumps({"status": "failed", "phase": "transcript", "result": None}))
    message = "```json\n" + json.dumps({"status": "blocked", "summary": "VERDICT: BLOCK\nx"}) + "\n```"
    (worker / "events.jsonl").write_text(json.dumps({"type": "item.completed",
                                                     "item": {"type": "agent_message", "text": message}}) + "\n")
    (lane.state_root / "runs" / second.key / "outcome.json").write_text(json.dumps(
        {"run_id": second.run_id, "verdict": "FAILED", "state": "error", "target": first["target"], "reviews": []}))
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_planned_reuse_falling_back_to_review_runs_total_space_admission(lane, env, monkeypatch):
    lane.review_pr(1810)
    _advance_main(lane, env)
    target = lane.target(1810, 1)
    assert target.reuse_planned
    (lane.source / ".git" / "fake-admission").write_text("fail")
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))
    sent = list(lane.gateway.sent)
    with pytest.raises(IdentityError, match="共需"):
        lane.review_pr(1810)
    assert lane.gateway.sent == sent  # 没有为本 run 发 pending


def test_ctrl_c_during_concurrent_review_returns_without_waiting_for_receipts(lane, monkeypatch):
    import os
    import signal
    import time as clock
    lane.launcher = [sys.executable, "-c", "import time; time.sleep(120)"]
    calls = []
    real_wait = module.wait
    def interrupted(futures):
        calls.append(1)
        if len(calls) == 1:
            deadline = clock.monotonic() + 30  # 等两位都已派发（有 launch.json），不靠固定 sleep
            while clock.monotonic() < deadline and len(list((lane.state_root / "runs").glob("*/*/launch.json"))) < 2:
                clock.sleep(0.05)
            raise KeyboardInterrupt
        return real_wait(futures)
    monkeypatch.setattr(module, "wait", interrupted)
    started = clock.monotonic()
    try:
        with pytest.raises(KeyboardInterrupt):
            lane.review_pr(1810)
        assert clock.monotonic() - started < 30
        target = ReviewTarget(**json.loads(next((lane.state_root / "runs").glob("*/target.json")).read_text()))
        folder = lane.state_root / "runs" / target.key
        assert all((folder / m / "launch.json").exists() for m in MODELS)  # 已派发，留待收养
        assert not (folder / "outcome.json").exists()
    finally:
        for process in (lane.state_root / "runs").glob("*/*/process.json"):
            os.killpg(json.loads(process.read_text())["pid"], signal.SIGKILL)


def _dispatched_without_receipt(folder, model, run_id):
    worker = folder / model
    worker.mkdir(parents=True, exist_ok=True)
    (worker / "launch.json").write_text(json.dumps({"run_id": run_id, "model": model, "launch_id": "x"}))
    (worker / "process.json").write_text(json.dumps({"pid": 2 ** 22 + 7, "launch_id": "x"}))  # 监督进程已不在


def test_interleave_resume_adopts_dispatched_second_reviewer_after_first_failed(lane, monkeypatch):
    """D5 9842a6d5 Sol：恢复时首位 FAILED、次位已派发未出回执，不得提前聚合；次位须收养到终态再汇总。"""
    target = lane.target(1810, 1)
    folder = lane.state_root / "runs" / target.key
    _modes(lane, **{MODELS[0]: "failed"})
    real = lane.run_review
    def first_only(t, model, f):
        if model == MODELS[1]:
            _dispatched_without_receipt(f, model, t.run_id)
            raise module.EffectPending("模拟：次位已派发，控制器中断")
        return real(t, model, f)
    monkeypatch.setattr(lane, "run_review", first_only)
    with pytest.raises(module.EffectPending):
        lane.review_pr(1810)
    assert not (folder / "outcome.json").exists()
    monkeypatch.setattr(lane, "run_review", real)
    result = lane.review_pr(1810)
    assert [r["model"] for r in result["reviews"]] == list(MODELS)
    assert result["verdict"] == "FAILED" and (folder / MODELS[1] / "receipt.json").exists()


def test_interleave_outcome_with_unfinished_dispatched_reviewer_never_counts_as_settled(lane, env):
    """旧聚合可能在次位仍运行时写出 outcome：这样的 run 不算完结，同补丁新 head 不沿用；次位之后补出 BLOCK 也不沿用。"""
    first = lane.review_pr(1810)
    second = lane.target(1810, 2)
    folder = lane.state_root / "runs" / second.key
    _dispatched_without_receipt(folder, MODELS[1], second.run_id)
    (folder / "outcome.json").write_text(json.dumps({"run_id": second.run_id, "verdict": "FAILED", "state": "error",
        "target": first["target"], "reviews": [{"model": MODELS[0], "verdict": "FAILED"}]}))
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)
    (folder / MODELS[1] / "receipt.json").write_text(json.dumps(
        {"status": "completed", "result": {"status": "blocked", "summary": "VERDICT: BLOCK\n迟到"}}))
    _advance_main(lane, env)
    later = lane.review_pr(1810)
    assert "reused_from" not in later and len(later["reviews"]) == len(MODELS)


def test_prepared_but_never_dispatched_reviewer_does_not_keep_run_unsettled(lane):
    first = lane.review_pr(1810)
    second = lane.target(1810, 2)
    folder = lane.state_root / "runs" / second.key
    (folder / MODELS[0]).mkdir(parents=True)
    (folder / MODELS[0] / "prepare.json").write_text(json.dumps({"run_id": second.run_id}))
    (folder / "outcome.json").write_text(json.dumps({"run_id": second.run_id, "verdict": "FAILED", "state": "error",
        "target": first["target"], "reviews": [{"model": MODELS[0], "verdict": "FAILED"}]}))
    row = next(r for r in lane.status(1810) if r["target"]["run_id"] == second.run_id)
    assert lane._settled(row)  # 未派发的 prepare 不让 run 永远处于未完结
    _dispatched_without_receipt(folder, MODELS[1], second.run_id)
    assert not lane._settled(next(r for r in lane.status(1810) if r["target"]["run_id"] == second.run_id))


@pytest.mark.parametrize("ending", ["FAILED", "STALE"])
def test_superseding_reattempt_without_block_keeps_old_pass_from_reviving(lane, env, monkeypatch, ending):
    """D5 d47722cf Opus：同补丁显式重审以 FAILED/STALE 收场且无 BLOCK，更早的 PASS 不得在追平后复活。"""
    _, second = _second_opinion(lane, env, monkeypatch, ["FAILED", "FAILED"] if ending == "FAILED" else ["PASS", "PASS"],
                                stale=ending == "STALE")
    assert second["verdict"] == ending
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_unreadable_evidence_vetoes_reuse_without_blocking_registration(lane, env):
    """_read 对权限、属主不符抛 IdentityError：按 BLOCK 处理（不沿用），但不能让登记失败。"""
    first = lane.review_pr(1810)
    other = lane.target(1810, 2)
    worker = lane.state_root / "runs" / other.key / MODELS[0]
    worker.mkdir(parents=True)
    (worker / "receipt.json").write_text("{}")
    (worker / "receipt.json").chmod(0o666)  # 组/他人可写：_read 抛 IdentityError
    (lane.state_root / "runs" / other.key / "outcome.json").write_text(json.dumps(
        {"run_id": other.run_id, "verdict": "PASS", "state": "success", "target": first["target"], "reviews": []}))
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_serial_resume_after_interrupt_is_not_stuck_on_stale_stop_flag(lane):
    target = lane.target(1810, 1)
    (lane.state_root / "runs" / target.key / MODELS[0]).mkdir()  # 走串行恢复
    lane._stop.set()                                                # 同一实例上一次被 Ctrl-C
    assert lane.review_pr(1810)["verdict"] == "PASS"


def test_race_superseding_registration_during_reuse_admission_blocks_reuse(lane, env, monkeypatch):
    """D5 f19adb21 Sol P1：H2 读完历史、等部署准入时，另一进程登记同补丁 H1 attempt2 并以 STALE 收场；H2 不得发布沿用。"""
    lane.review_pr(1810)
    h1 = lane.gateway.pr["head"]["sha"]
    _advance_main(lane, env)
    h2 = lane.gateway.pr["head"]["sha"]
    real = lane.reuse_admission
    calls = []
    def racing(target, source):
        calls.append(1)
        if len(calls) == 1:                           # 登记时那次判定不插入；竞态窗口在派发阶段读完历史之后
            return real(target, source)
        lane.gateway.pr["head"]["sha"] = h1           # 另一进程眼中的 PR 仍在 H1
        late = lane.target(1810, 2)
        lane.gateway.pr["head"]["sha"] = h2
        folder = lane.state_root / "runs" / late.key
        (folder / "outcome.json").write_text(json.dumps({"run_id": late.run_id, "verdict": "STALE",
                                                          "target": asdict(late), "reviews": []}))
        (folder / "outcome.json").chmod(0o600)
        return real(target, source)
    monkeypatch.setattr(lane, "reuse_admission", racing)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_pass_message_quoting_block_is_not_block_evidence(lane, env):
    """D5 f19adb21 Sol P2：合法 PASS 在说明里提到 VERDICT: BLOCK，不得被当成 BLOCK 证据。"""
    first = lane.review_pr(1810)
    worker = lane.state_root / "runs" / ReviewTarget(**first["target"]).key / MODELS[0]
    text = json.dumps({"status": "completed", "summary": "VERDICT: PASS\n未发现会导致 VERDICT: BLOCK 的问题",
                       "findings": [], "validation": ["检查过 VERDICT: BLOCK 的触发条件"]})
    (worker / "events.jsonl").write_text(json.dumps({"type": "item.completed",
                                                     "item": {"type": "agent_message", "text": text}}) + "\n")
    _advance_main(lane, env)
    assert lane.review_pr(1810)["reused_from"]["run_id"] == first["run_id"]


@pytest.mark.parametrize("damage", ["json", "mode"])
def test_damaged_history_outcome_vetoes_reuse_but_registration_proceeds(lane, env, damage):
    """D5 f19adb21 Sol P2：status() 读到损坏的历史 outcome 时退回双审，不让登记失败。"""
    first = lane.review_pr(1810)
    outcome = lane.state_root / "runs" / ReviewTarget(**first["target"]).key / "outcome.json"
    if damage == "json":
        outcome.write_text("{broken")
    else:
        outcome.chmod(0o666)
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)


def test_registration_waits_for_inflight_reuse_publication(lane, env, monkeypatch):
    """并发合同：沿用从重读历史到发布 success 持有 PR 锁；期间另一实例的同 PR 登记必须等到发布之后。"""
    import threading
    import time as clock
    lane.review_pr(1810)
    h1 = lane.gateway.pr["head"]["sha"]
    _advance_main(lane, env)
    other_gateway = copy.copy(lane.gateway)
    other_gateway.pr = copy.deepcopy(lane.gateway.pr)
    other_gateway.pr["head"]["sha"] = h1
    other = ExternalReview(other_gateway, source=lane.source, state_root=lane.state_root,
                           checkout_root=lane.checkout_root, launcher=lane.launcher, poll_seconds=0.01)
    registered = {}
    def register():
        registered["target"] = other.target(1810, 2)
        registered["at"] = clock.monotonic()
    real = lane._publish_reuse
    def publishing(*args):
        thread = threading.Thread(target=register)
        thread.start()
        clock.sleep(0.5)
        assert "at" not in registered, "持 PR 锁期间另一实例完成了登记"
        result = real(*args)
        registered["published"] = clock.monotonic()
        publishing.thread = thread
        return result
    monkeypatch.setattr(lane, "_publish_reuse", publishing)
    result = lane.review_pr(1810)
    publishing.thread.join(30)
    assert "reused_from" in result and registered["at"] >= registered["published"] - 0.05

def _auth_broken(lane, monkeypatch, broken):
    real = lane.gateway.get_pr
    def get_pr(number):
        if broken["on"]:
            raise IdentityError("GitHub 返回 HTTP 401，未切换凭据来源")
        return real(number)
    monkeypatch.setattr(lane.gateway, "get_pr", get_pr)


def test_auth_failure_before_final_publish_is_pending_not_stale(lane, monkeypatch):
    """#1938 run 63919f86：双 PASS 后发布前预检 401，不得记 STALE；认证恢复后同 attempt 重跑补发，不重跑模型。"""
    broken = {"on": False}
    _auth_broken(lane, monkeypatch, broken)
    real_publish = module.StatusPublisher.publish
    def publish(self, state, *args):
        if state == "success" and not broken.get("done"):
            broken["on"] = broken["done"] = True  # 只在第一次发布 success 前断开认证
        return real_publish(self, state, *args)
    monkeypatch.setattr(module.StatusPublisher, "publish", publish)
    with pytest.raises(EffectPending, match="未判定漂移"):
        lane.review_pr(1810)
    target = ReviewTarget(**json.loads(next((lane.state_root / "runs").glob("*/target.json")).read_text()))
    saved = json.loads((lane.state_root / "runs" / target.key / "outcome.json").read_text())
    assert saved["verdict"] == "PASS" and "publication" not in saved
    broken["on"] = False
    launches = (lane.state_root / "runs" / "launches").read_text()
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and result["publication"]["state"] == "success"
    assert (lane.state_root / "runs" / "launches").read_text() == launches  # 没有重跑模型
    assert lane.gateway.sent[-1] == "success"


def _legacy_auth_stale(lane):
    """构造旧版本留下的形状：双审完整，发布前 401 被记为 STALE，只发过 pending。"""
    result = lane.review_pr(1810)
    target = ReviewTarget(**result["target"])
    folder = lane.state_root / "runs" / target.key
    saved = json.loads((folder / "outcome.json").read_text())
    saved.pop("publication")
    saved.update(verdict="STALE", reason="GitHub 返回 HTTP 401，未切换凭据来源")
    (folder / "outcome.json").write_text(json.dumps(saved))
    (folder / "outcome.json").chmod(0o600)
    lane.gateway.rows = [r for r in lane.gateway.rows if r["state"] != "success"]
    (folder / "effects" / "status-success.json").unlink()  # 原件 effects 只有 status-pending
    return folder


def test_legacy_auth_stale_is_recovered_by_same_attempt_without_rerunning_models(lane):
    folder = _legacy_auth_stale(lane)
    launches = (lane.state_root / "runs" / "launches").read_text()
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and result["publication"]["state"] == "success"
    assert result["recovered_from_stale"]["reason"].startswith("GitHub 返回 HTTP 401")
    history = json.loads((folder / result["recovered_from_stale"]["history"]).read_text())
    assert history["verdict"] == "STALE"  # 原失败记录保留
    assert (lane.state_root / "runs" / "launches").read_text() == launches


def test_legacy_auth_stale_is_not_recovered_after_real_drift(lane):
    _legacy_auth_stale(lane)
    (lane.source / "changed-1").write_text("pushed\n")
    git(lane.source, "commit", "-qam", "push")
    lane.gateway.pr["head"]["sha"] = git(lane.source, "rev-parse", "HEAD")
    sent = list(lane.gateway.sent)
    assert lane.review_pr(1810, 1)["verdict"] in ("STALE", "PASS")  # 新 head 是另一个 run 键
    old = [r for r in lane.status(1810) if r["outcome"] and r["outcome"]["verdict"] == "STALE"]
    assert old and all("recovered_from_stale" not in r["outcome"] for r in old)
    assert sent == lane.gateway.sent[:len(sent)]


def test_legacy_auth_stale_with_tampered_receipt_is_not_recovered(lane):
    folder = _legacy_auth_stale(lane)
    receipt = json.loads((folder / MODELS[0] / "receipt.json").read_text())
    receipt["exit_code"] = 1
    (folder / MODELS[0] / "receipt.json").write_text(json.dumps(receipt))
    (folder / MODELS[0] / "receipt.json").chmod(0o600)
    sent = list(lane.gateway.sent)
    result = lane.review_pr(1810)
    assert result["verdict"] == "STALE" and "recovered_from_stale" not in result
    assert lane.gateway.sent == sent


def test_stale_recovery_is_decided_by_reverification_not_reason_text(lane):
    """恢复与否只看重验：原因文案写成漂移，但 PR、controller、取代关系与回执都仍成立时照常补发；
    真漂移（head 已动）由 test_legacy_auth_stale_is_not_recovered_after_real_drift 覆盖。"""
    folder = _legacy_auth_stale(lane)
    saved = json.loads((folder / "outcome.json").read_text())
    saved["reason"] = "PR base、head 或目标已漂移"
    (folder / "outcome.json").write_text(json.dumps(saved))
    (folder / "outcome.json").chmod(0o600)
    assert lane.review_pr(1810)["verdict"] == "PASS"


def _interrupt_before_reuse_success(lane, env, monkeypatch):
    """沿用路径：pending 已发、outcome 已存（PASS，reused_from），success 发布前被杀。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    real = module.StatusPublisher.publish
    def publish(self, state, *args):
        if state == "success":
            raise KeyboardInterrupt
        return real(self, state, *args)
    monkeypatch.setattr(module.StatusPublisher, "publish", publish)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    monkeypatch.setattr(module.StatusPublisher, "publish", real)
    target = ReviewTarget(**json.loads(next(p for p in (lane.state_root / "runs").glob("*/target.json")
                                              if json.loads(p.read_text())["head_sha"] == lane.gateway.pr["head"]["sha"]).read_text()))
    folder = lane.state_root / "runs" / target.key
    saved = json.loads((folder / "outcome.json").read_text())
    assert "reused_from" in saved and not (folder / "effects" / "status-success.json").exists()
    return folder


def test_resumed_reuse_does_not_publish_after_newer_same_head_attempt(lane, env, monkeypatch):
    """D5 b32b8b86 Sol P1：恢复旧 attempt 不得在同 head 新 attempt 登记后直接发布沿用 success。"""
    _interrupt_before_reuse_success(lane, env, monkeypatch)
    lane.target(1810, 2)                       # 同 head 新 attempt：已登记，尚未发 pending
    sent = list(lane.gateway.sent)
    result = lane.review_pr(1810, 1)
    assert result["verdict"] != "PASS" and "success" not in lane.gateway.sent[len(sent):]


def test_resumed_reuse_does_not_publish_after_admission_lost(lane, env, monkeypatch):
    """同一序列的另一反例：恢复前部署准入失效，不得发布沿用 success。"""
    _interrupt_before_reuse_success(lane, env, monkeypatch)
    (lane.source / ".git" / "fake-admission").write_text("fail")
    sent = list(lane.gateway.sent)
    result = lane.review_pr(1810, 1)
    assert result["verdict"] != "PASS" and "success" not in lane.gateway.sent[len(sent):]


def test_completed_publication_is_only_reconciled_not_regated(lane, env):
    """已完成发布的 run：之后即使有同 head 新 attempt 登记，再调用也只对账，不改写成 STALE。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    first = lane.review_pr(1810)
    assert first["publication"]["state"] == "success"
    lane.target(1810, 2)
    again = lane.review_pr(1810, 1)
    assert again["verdict"] == "PASS" and again["publication"] == first["publication"]


def test_resume_after_planned_reuse_fell_back_is_not_blocked_by_total_admission(lane, env, monkeypatch):
    """D5 b32b8b86 Opus 中：已转双审且有 worker 后，恢复不再按两份快照准入挡收养。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    target = lane.target(1810, 1)
    assert target.reuse_planned
    (lane.source / ".git" / "fake-admission").write_text("fail")       # 派发时不能沿用 → 双审
    real = _interrupt_before_aggregation(lane, monkeypatch)
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    monkeypatch.setattr(lane, "run_reviewers", real)
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))  # 恢复时空间偏紧
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and len(result["reviews"]) == len(MODELS)


def test_wrongly_typed_history_outcome_does_not_block_registration(lane, env):
    """D5 b32b8b86 Opus 低：合法 JSON 但 reviews/publication 类型错误，不沿用，登记照常。"""
    first = lane.review_pr(1810)
    outcome = lane.state_root / "runs" / ReviewTarget(**first["target"]).key / "outcome.json"
    bad = json.loads(outcome.read_text())
    bad["reviews"], bad["publication"] = ["x"], "y"
    outcome.write_text(json.dumps(bad))
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and len(result["reviews"]) == len(MODELS)



def _dirty_executing_checkout(tmp_path):
    repo = tmp_path / "exec-checkout"
    (repo / ".sandcastle").mkdir(parents=True)
    git(tmp_path, "init", "-q", "-b", "main", str(repo))
    git(repo, "config", "user.name", "x")
    git(repo, "config", "user.email", "x@example.invalid")
    (repo / ".sandcastle" / "controller.py").write_text("# committed\n")
    git(repo, "add", ".")
    git(repo, "commit", "-qm", "controller B")
    (repo / ".sandcastle" / "controller.py").write_text("# committed\n# uncommitted edit\n")
    return repo


def test_controller_identity_is_the_executing_checkout_not_the_object_source(lane, monkeypatch, tmp_path):
    """D5 外独审 P1：执行代码所在检出不干净时拒绝；对象来源（source）不能替它出具身份。"""
    monkeypatch.setattr(module, "SOURCE_REPO", _dirty_executing_checkout(tmp_path))
    with pytest.raises(IdentityError, match="未提交"):
        lane.review_pr(1810)
    assert lane.gateway.sent == []


def test_cli_has_no_source_override():
    with pytest.raises(SystemExit):
        module.main(["review-pr", "--pr", "1810", "--private-key", "/never-read.pem", "--source", "/x"])


def test_migration_records_both_identities_requires_admission_and_is_never_reused(lane, env):
    """新部署补发旧 controller 下的 STALE：记录原审查与当前发布者两个身份，发布者须过部署准入，不冒充旧 controller。"""
    folder = _legacy_auth_stale(lane)
    review_controller = json.loads((folder / "target.json").read_text())["controller_code_head"]
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed")  # 新部署
    publisher = git(lane.source, "rev-parse", "HEAD")
    (lane.source / ".git" / "fake-admission").write_text("fail")
    with pytest.raises(IdentityError, match="部署准入"):
        lane.migrate_stale(1810, 1)
    (lane.source / ".git" / "fake-admission").unlink()
    launches = (lane.state_root / "runs" / "launches").read_text()
    result = lane.migrate_stale(1810, 1)
    assert result["verdict"] == "PASS" and result["publication"]["state"] == "success"
    assert result["migrated"]["review_controller"] == review_controller
    assert result["migrated"]["publisher_controller"] == publisher != review_controller
    assert json.loads((folder / "target.json").read_text())["controller_code_head"] == review_controller
    assert (lane.state_root / "runs" / "launches").read_text() == launches  # 不重跑模型
    _advance_main(lane, env)
    git(lane.source, "checkout", "-q", publisher)
    later = lane.review_pr(1810)
    assert "reused_from" not in later and len(later["reviews"]) == len(MODELS)  # 迁移结论不作沿用来源


def test_reuse_success_delivered_but_journal_pending_is_claimed_not_restaled(lane, env, monkeypatch):
    """D5 外独审 N1：沿用 success 已送达、journal 仍 pending 时中断；恢复时即使准入失效，也先读回认领，不改记 STALE、不重发。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    def crash():
        if lane.gateway.sent[-1:] == ["success"]:
            raise KeyboardInterrupt
    lane.gateway.after_write = crash
    with pytest.raises(KeyboardInterrupt):
        lane.review_pr(1810)
    lane.gateway.after_write = None
    (lane.source / ".git" / "fake-admission").write_text("fail")
    sent = list(lane.gateway.sent)
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and result["publication"]["state"] == "success"
    assert lane.gateway.sent == sent


def test_any_pr_read_failure_at_final_publish_is_pending_not_stale(lane, monkeypatch):
    """D5 外独审 N2：按出处分类——最终发布时读取 PR 失败（如 token 刷新时安装身份不符）不判 STALE；恢复后同 attempt 补发。"""
    broken = {"on": False}
    real_get = lane.gateway.get_pr
    def get_pr(number):
        if broken["on"]:
            raise IdentityError("安装身份不符，字段 permissions")
        return real_get(number)
    monkeypatch.setattr(lane.gateway, "get_pr", get_pr)
    real_publish = module.StatusPublisher.publish
    def publish(self, state, *args):
        if state == "success" and not broken.get("done"):
            broken["on"] = broken["done"] = True
        return real_publish(self, state, *args)
    monkeypatch.setattr(module.StatusPublisher, "publish", publish)
    with pytest.raises(EffectPending, match="未判定漂移"):
        lane.review_pr(1810)
    broken["on"] = False
    launches = (lane.state_root / "runs" / "launches").read_text()
    result = lane.review_pr(1810)
    assert result["verdict"] == "PASS" and result["publication"]["state"] == "success"
    assert (lane.state_root / "runs" / "launches").read_text() == launches


def test_superseded_old_run_does_not_dispatch_models(lane):
    """D5 外独审 N3：同 head 更晚的 attempt 已登记时，旧 attempt 不发 pending、不派模型。"""
    lane.target(1810, 1)
    lane.target(1810, 2)
    result = lane.review_pr(1810, 1)
    assert result["verdict"] == "STALE" and lane.gateway.sent == []
    assert not (lane.state_root / "runs" / "launches").exists()


def _unknown_success_then_reconciled(lane, attempt=1):
    """终态 success 写入结果不明且读回不见（401 形态），主控按流程 reconcile 为未生效。"""
    real = lane.gateway.set_status
    def set_status(sha, context, state, description):
        if state == "success":
            lane.gateway.sent.append(state)
            raise OutcomeUnknown("已发送结果不明")
        return real(sha, context, state, description)
    lane.gateway.set_status = set_status
    with pytest.raises(EffectPending):
        lane.review_pr(1810, attempt)
    lane.gateway.set_status = real
    head = lane.gateway.pr["head"]["sha"]
    target = next(json.loads(p.read_text()) for p in (lane.state_root / "runs").glob(f"*-a{attempt}/target.json")
                  if json.loads(p.read_text())["head_sha"] == head)
    lane.reconcile(target["run_id"], "主控已核实 GitHub 上没有该 success")
    return list(lane.gateway.sent)


def test_reconciled_reuse_success_regates_admission_before_resend(lane, env):
    """#1913 独审 B1：对账为未生效后恢复，沿用结果必须重做部署准入；准入失效就不发 success。"""
    lane.review_pr(1810)
    _advance_main(lane, env)
    sent = _unknown_success_then_reconciled(lane)
    (lane.source / ".git" / "fake-admission").write_text("fail")
    result = lane.review_pr(1810)
    assert "success" not in lane.gateway.sent[len(sent):] and result["verdict"] == "STALE"


def test_reconciled_success_is_not_resent_after_newer_same_head_attempt(lane):
    """#1913 独审 B1：对账为未生效后，同 head 已登记更晚 attempt，旧 run 不再发 success。"""
    sent = _unknown_success_then_reconciled(lane)
    lane.target(1810, 2)
    result = lane.review_pr(1810, 1)
    assert "success" not in lane.gateway.sent[len(sent):] and result["verdict"] == "STALE"


def test_reconciled_success_passing_the_gate_is_resent(lane):
    """B1 正向：对账为未生效、门仍成立时照常补发。"""
    _unknown_success_then_reconciled(lane)
    result = lane.review_pr(1810)
    assert result["publication"]["state"] == "success"


def test_migration_is_retryable_after_transient_failure_mid_publish(lane, monkeypatch):
    """#1913 独审 N-a：迁移已重验写下结论、发布时读 PR 401；认证恢复后再迁移能补发，不必重跑模型。"""
    folder = _legacy_auth_stale(lane)
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed")
    real_get = lane.gateway.get_pr
    calls = {"n": 0}
    def get_pr(number):
        calls["n"] += 1
        if calls["n"] == 3:  # load_target、recover_stale.check_current 之后：publish 内的 check_current
            raise IdentityError("GitHub 返回 HTTP 401")
        return real_get(number)
    monkeypatch.setattr(lane.gateway, "get_pr", get_pr)
    before = list(lane.gateway.sent)
    with pytest.raises(EffectPending):
        lane.migrate_stale(1810, 1)
    monkeypatch.setattr(lane.gateway, "get_pr", real_get)
    assert lane.gateway.sent == before
    launches = (lane.state_root / "runs" / "launches").read_text()
    result = lane.migrate_stale(1810, 1)
    assert result["publication"]["state"] == "success" and result["recovered_from_stale"]["reason"].startswith("GitHub")
    assert (lane.state_root / "runs" / "launches").read_text() == launches
    with pytest.raises(IdentityError, match="只迁移未发布"):
        lane.migrate_stale(1810, 1)  # 已发布的不再迁移
    assert json.loads((folder / "outcome.json").read_text())["publication"]["state"] == "success"


def test_refused_migration_leaves_stale_on_disk(lane):
    """#1913 独审 N-b：迁移被同 head 更晚 attempt 取代时，磁盘 outcome 回到 STALE，不留未发布的 PASS。"""
    folder = _legacy_auth_stale(lane)
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed")
    lane.target(1810, 2)
    sent = list(lane.gateway.sent)
    result = lane.migrate_stale(1810, 1)
    assert "migration_refused" in result and lane.gateway.sent == sent
    assert json.loads((folder / "outcome.json").read_text())["verdict"] == "STALE"


def test_migration_refuses_changed_review_contract(lane, monkeypatch):
    """#1913 独审 N-c：合同（policy、模型、提示词）已变的旧结论不迁移，须新审。"""
    _legacy_auth_stale(lane)
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed with new contract")
    monkeypatch.setattr(module, "REVIEW_CONTRACT", "sha256:" + "f" * 64)
    with pytest.raises(IdentityError, match="合同"):
        lane.migrate_stale(1810, 1)


def test_migration_of_contractless_legacy_target_checks_dispatched_prompt(lane, monkeypatch):
    """旧 controller 登记的 target 未记合同：以实际派发给模型的提示词核对，提示词不同就不迁移。"""
    folder = _legacy_auth_stale(lane)
    target = json.loads((folder / "target.json").read_text())
    target["review_contract"] = ""
    (folder / "target.json").write_text(json.dumps(target))
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed")
    original = module.PROMPT
    monkeypatch.setattr(module, "PROMPT", original + "\n新增一条审查要求")
    with pytest.raises(IdentityError, match="合同"):
        lane.migrate_stale(1810, 1)
    monkeypatch.setattr(module, "PROMPT", original)
    assert lane.migrate_stale(1810, 1)["publication"]["state"] == "success"


# ---------------- Codex 额度耗尽时的临时组合（Kimi + Opus） ----------------

LEGACY_MODELS = ("gpt-6.1-sol", "claude-opus-5-5")
LEGACY_POLICY = "external-pr-v1"


def _contract(policy, models):
    return "sha256:" + hashlib.sha256(json.dumps([policy, models, module.PROMPT]).encode()).hexdigest()


def _use_contract(monkeypatch, models, policy):
    monkeypatch.setattr(module, "MODELS", models)
    monkeypatch.setattr(module, "POLICY", policy)
    monkeypatch.setattr(module, "REVIEW_CONTRACT", _contract(policy, models))


def test_fallback_pair_is_two_families_under_its_own_contract():
    assert MODELS == ("kimi-k3.1-pro", "claude-opus-5-5")
    assert len({model.split("-")[0] for model in MODELS}) == len(MODELS) == 2
    assert module.POLICY == "external-pr-v1-kimi" != LEGACY_POLICY
    assert module.REVIEW_CONTRACT == _contract(module.POLICY, MODELS) != _contract(LEGACY_POLICY, LEGACY_MODELS)


def test_pass_under_legacy_contract_is_not_reused_after_switch(lane, env, monkeypatch):
    current = (module.MODELS, module.POLICY)
    _use_contract(monkeypatch, LEGACY_MODELS, LEGACY_POLICY)
    first = lane.review_pr(1810)
    assert first["verdict"] == "PASS" and [r["model"] for r in first["reviews"]] == list(LEGACY_MODELS)
    _use_contract(monkeypatch, *current)
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and [r["model"] for r in result["reviews"]] == list(MODELS)
    assert result["target"]["policy_version"] == "external-pr-v1-kimi"
    assert sorted((lane.state_root / "runs" / "launches").read_text().splitlines()[2:]) == sorted(MODELS)


@pytest.mark.parametrize("recorded_contract", [True, False])
def test_legacy_contract_stale_is_not_migrated(lane, monkeypatch, recorded_contract):
    current = (module.MODELS, module.POLICY)
    _use_contract(monkeypatch, LEGACY_MODELS, LEGACY_POLICY)
    folder = _legacy_auth_stale(lane)
    if not recorded_contract:  # 旧 controller 登记的 target 未记合同
        target = json.loads((folder / "target.json").read_text())
        target["review_contract"] = ""
        (folder / "target.json").write_text(json.dumps(target))
    _use_contract(monkeypatch, *current)
    git(lane.source, "commit", "--allow-empty", "-qm", "controller redeployed with fallback pair")
    sent = list(lane.gateway.sent)
    with pytest.raises(IdentityError, match="policy"):
        lane.migrate_stale(1810, 1)
    assert lane.gateway.sent == sent
    assert json.loads((folder / "outcome.json").read_text())["verdict"] == "STALE"


def test_block_left_by_retired_reviewer_still_vetoes_reuse(lane, env, monkeypatch):
    """换组合后，旧 run 里已退役 reviewer 留下的 BLOCK 证据仍算同补丁 BLOCK。"""
    current = (module.MODELS, module.POLICY)
    _use_contract(monkeypatch, LEGACY_MODELS, LEGACY_POLICY)
    _modes(lane, **{LEGACY_MODELS[0]: "BLOCK"})
    legacy = lane.review_pr(1810)
    _clear_modes(lane)
    folder = lane.state_root / "runs" / ReviewTarget(**legacy["target"]).key
    assert (folder / LEGACY_MODELS[0] / "receipt.json").exists()
    (folder / "outcome.json").write_text(json.dumps({**legacy, "verdict": "FAILED", "state": "error", "reviews": []}))
    _use_contract(monkeypatch, *current)
    assert lane.review_pr(1810, 2)["verdict"] == "PASS"
    _advance_main(lane, env)
    result = lane.review_pr(1810)
    assert "reused_from" not in result and [r["model"] for r in result["reviews"]] == list(MODELS)
