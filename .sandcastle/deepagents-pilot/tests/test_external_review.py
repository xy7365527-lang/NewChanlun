import copy
import json
import sys
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
    identity = AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
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
                     "account": {"login": "xy7365527-lang"}, "repository_selection": "selected",
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
    assert (lane.state_root / "runs" / "launches").read_text().splitlines() == list(MODELS)
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
    identity = AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
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
    assert (lane.state_root / "runs" / "launches").read_text().splitlines() == list(MODELS)


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
    assert (lane.state_root / "runs" / "launches").read_text().splitlines() == list(MODELS)


def test_failed_first_reviewer_does_not_dispatch_second(lane):
    lane.target(1810, 1)
    (lane.state_root / "runs" / "fake-mode").write_text("failed")
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
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


def test_low_space_stops_before_checkout_or_model(lane, monkeypatch):
    monkeypatch.setattr(module.shutil, "disk_usage", lambda path: SimpleNamespace(free=1))
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert "空间不足" in result["reviews"][0]["reason"]
    assert not (lane.state_root / "runs" / "launches").exists()
    assert not list(lane.checkout_root.glob("*/*"))


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
    assert (lane.state_root / "runs" / "launches").read_text().splitlines() == list(MODELS) * 3
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
    calls = 0
    def disk_usage(path):
        nonlocal calls
        calls += 1
        return SimpleNamespace(free=10**12 if calls == 1 else 1)
    monkeypatch.setattr(module.shutil, "disk_usage", disk_usage)
    result = lane.review_pr(1810)
    assert result["verdict"] == "FAILED"
    assert "模型启动前" in result["reviews"][0]["reason"]
    assert not (lane.state_root / "runs" / "launches").exists()
    assert not list(lane.checkout_root.iterdir())
