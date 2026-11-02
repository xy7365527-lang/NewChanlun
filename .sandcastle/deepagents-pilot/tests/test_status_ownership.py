import time
import uuid

import pytest
from test_executor import posts
from v2env import TICKET, SimulatedCrash, V2Env, crash_at

from pilot1742 import external_review
from pilot1742.claims import ClaimStore
from pilot1742.delivery import EffectPending, Stale
from pilot1742.executor import reconcile_effect
from pilot1742.external_review import (
    POLICY,
    ReviewTarget,
    StatusPublisher,
    check_controller,
    load_target,
)


@pytest.fixture
def v2(tmp_path, monkeypatch):
    env = V2Env(tmp_path)
    monkeypatch.setattr(external_review, "SOURCE_REPO", env.source)
    return env


class ExternalGateway:
    def __init__(self, v2):
        self.inner = v2.gateway()
        self.client, self.bot, self.base = self.inner.client, self.inner.bot, v2.base

    def get_pr(self, number):
        pr = self.inner.get_pr(number)
        repo = self.client.identity.repository_id
        pr["base"].update(sha=self.base, repo={"id": repo})
        pr["head"]["repo"]["id"] = repo
        return pr

    def statuses(self, sha):
        return self.inner.statuses(sha)

    def set_status(self, sha, context, state, description):
        self.inner.set_status(sha, context, state, description)


def external(v2, attempt):
    gateway = ExternalGateway(v2)
    (number,) = [n for n, pr in v2.github.pulls.items() if pr["state"] == "open"]
    target = ReviewTarget(**load_target(gateway, number), merge_base_sha=v2.base, run_id=str(uuid.uuid4()),
                          attempt=attempt, policy_version=POLICY, controller_code_head=check_controller(),
                          registered_at_ns=time.time_ns())
    return StatusPublisher(gateway, target, v2.tmp / "external" / target.key, v2.tmp / "status-locks")


def states(v2, head):
    return [s["state"] for s in v2.github.statuses.get(head, [])]


def delivered(v2):
    return sum("deepagents:deliver" in c["body"] for c in v2.github.issues[TICKET]["comments"])


def interrupted(v2, point):
    unit = v2.register()
    run = v2.run(unit, crash=crash_at(point))
    head = v2.to_pass(run)
    with pytest.raises(SimulatedCrash):
        run.publish_candidate()
    run.release()
    return unit, head


def test_external_failure_before_first_executor_status_is_not_overwritten(v2):
    unit, head = interrupted(v2, "after_pr_before_receipt")
    review = external(v2, 1)
    assert review.target.head_sha == head
    review.publish("pending")
    review.publish("failure")
    result = v2.run(unit).publish_candidate()
    assert states(v2, head) == ["failure", "pending"]
    assert result.get("decision") == "stale" and delivered(v2) == 0


def test_executor_resuming_during_external_pending_does_not_publish(v2):
    unit, head = interrupted(v2, "after_pr_before_receipt")
    review = external(v2, 1)
    review.publish("pending")
    result = v2.run(unit).publish_candidate()
    assert states(v2, head) == ["pending"]
    assert result.get("decision") == "stale" and delivered(v2) == 0
    assert review.publish("failure")["state"] == "failure"
    assert states(v2, head) == ["failure", "pending"]


def test_older_executor_success_cannot_recover_over_external_failure(v2):
    unit, head = interrupted(v2, "after_status_before_receipt")
    assert states(v2, head) == ["success"]
    review = external(v2, 1)
    review.publish("pending")
    review.publish("failure")
    result = v2.run(unit).publish_candidate()
    assert (result.get("decision"), delivered(v2)) == ("stale", 0)
    assert states(v2, head) == ["failure", "pending", "success"]


def test_delivered_executor_replay_reports_external_failure_instead_of_ok(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    assert run.publish_candidate()["ok"]
    review = external(v2, 1)
    review.publish("pending")
    review.publish("failure")
    sent = posts(v2, rf".*/statuses/{head}")
    result = run.publish_candidate()
    assert (result.get("decision"), result.get("phase")) == ("stale", None)
    assert run.status()["phase"] == "stale"
    assert posts(v2, rf".*/statuses/{head}") == sent and states(v2, head)[0] == "failure"


def test_reverse_order_old_external_run_cannot_overwrite_executor_and_new_run_takes_over(v2):
    unit, head = interrupted(v2, "after_pr_before_receipt")
    older = external(v2, 1)
    run = v2.run(unit)
    assert run.publish_candidate()["ok"]
    assert states(v2, head) == ["success"]
    with pytest.raises(Stale):
        older.publish("pending")
    with pytest.raises(Stale):
        older.publish("failure")
    assert states(v2, head) == ["success"]
    newer = external(v2, 2)
    newer.publish("pending")
    newer.publish("failure")
    assert states(v2, head) == ["failure", "pending", "success"]
    assert run.publish_candidate()["decision"] == "stale"
    assert states(v2, head) == ["failure", "pending", "success"]


def test_lost_executor_response_cannot_resend_after_new_external_run_takes_over(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    v2.github.fault("POST", rf".*/statuses/{head}", "status_500")
    with pytest.raises(EffectPending):
        run.publish_candidate()
    run.release()
    review = external(v2, 1)
    review.publish("pending")
    reconcile_effect(v2.home, ClaimStore(v2.claims_root), unit.run_id, f"status-{head}", "已核远端无此状态")
    result = v2.run(unit).publish_candidate()
    assert states(v2, head) == ["pending"] and posts(v2, rf".*/statuses/{head}") == 2
    assert result.get("decision") == "stale"


def test_newer_external_success_keeps_delivered_executor_replay_quiet(v2):
    unit = v2.register()
    run = v2.run(unit)
    head = v2.to_pass(run)
    assert run.publish_candidate()["ok"]
    review = external(v2, 1)
    review.publish("pending")
    review.publish("success")
    sent = posts(v2, rf".*/statuses/{head}")
    assert run.publish_candidate()["ok"]
    assert posts(v2, rf".*/statuses/{head}") == sent and states(v2, head) == ["success", "pending", "success"]
