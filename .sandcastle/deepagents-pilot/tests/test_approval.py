from datetime import datetime, timedelta, timezone

import pytest
from approval_signing.protocol import (
    ApprovalRequest,
    CommitTarget,
    Decision,
    Scope,
    spec_target,
)
from signing_fixture import Signer

from pilot1742.approval import (
    ApprovalDesk,
    Approved,
    Pending,
    SignedApprovals,
    SpecAuthorization,
)
from pilot1742.delivery import Unpublishable, commit_payload

HEAD = "1" * 40
OTHER = "2" * 40
NOW = datetime(2026, 10, 27, 12, tzinfo=timezone.utc)
CANDIDATE = CommitTarget(HEAD)


@pytest.fixture
def signer(tmp_path):
    return Signer(tmp_path / "approval-receipts")


@pytest.fixture
def desk(tmp_path, signer, monkeypatch):
    return ApprovalDesk(tmp_path / "run" / "approvals", 4242, "t4242-1", signer.protect(monkeypatch))


def check(desk, target=CANDIDATE, generation=1, now=NOW, scope=Scope.CANDIDATE_HEAD):
    return desk.check(scope, generation, target, now=now)


def test_without_trust_or_protected_root_nothing_releases(tmp_path, signer):
    real = ApprovalDesk(tmp_path / "a", 4242, "t4242-1", SignedApprovals(tmp_path / "missing-trust.json"))
    assert check(real) == Pending("not_configured", str(real.path(Scope.CANDIDATE_HEAD)), check(real).reason)
    default = check(ApprovalDesk(tmp_path / "b", 4242, "t4242-1"))
    assert isinstance(default, Pending) and default.status == "not_configured"


def test_user_owned_record_is_never_authoritative(tmp_path, signer, monkeypatch):
    from approval_signing import decision

    monkeypatch.setattr(decision, "load_trust", lambda path: signer.trust)
    desk = ApprovalDesk(tmp_path / "run", 4242, "t4242-1", SignedApprovals(authority_root=signer.root))
    check(desk)
    request = ApprovalRequest.parse(desk.path(Scope.CANDIDATE_HEAD).read_bytes())
    folder = signer.root / request.request_nonce
    folder.mkdir()
    (folder / "1.json").write_bytes(signer.receipt(request, Decision.APPROVE, 1, None))
    result = check(desk)
    assert isinstance(result, Pending) and result.status == "needs_reconciliation"


def test_request_is_the_exact_bytes_the_cli_signs_and_binds_every_field(desk):
    assert check(desk).status == "awaiting_human"
    request = ApprovalRequest.parse(desk.path(Scope.CANDIDATE_HEAD).read_bytes())
    assert (request.repository_id, request.ticket, request.run_id, request.claim_generation, request.scope,
            request.target) == (1150335160, 4242, "t4242-1", 1, Scope.CANDIDATE_HEAD, CommitTarget(HEAD))
    assert request.expires_at == NOW + timedelta(hours=24)
    assert check(desk, now=NOW + timedelta(hours=1)).request_path == str(desk.path(Scope.CANDIDATE_HEAD))
    assert ApprovalRequest.parse(desk.path(Scope.CANDIDATE_HEAD).read_bytes()) == request


def test_published_approve_releases_and_later_published_reject_dominates(desk, signer):
    check(desk)
    signer.publish(desk.path(Scope.CANDIDATE_HEAD), Decision.APPROVE)
    assert isinstance(check(desk), Approved)
    signer.publish(desk.path(Scope.CANDIDATE_HEAD), Decision.REJECT)
    assert check(desk).status == "rejected"
    signer.publish(desk.path(Scope.CANDIDATE_HEAD), Decision.APPROVE)
    assert isinstance(check(desk), Approved)


def test_signed_but_unpublished_decision_does_not_exist(desk, signer):
    check(desk)
    request = ApprovalRequest.parse(desk.path(Scope.CANDIDATE_HEAD).read_bytes())
    pending = desk.folder / "pending-receipt.json"
    pending.write_bytes(signer.receipt(request, Decision.APPROVE, 1, None))
    assert check(desk).status == "awaiting_human"


@pytest.mark.parametrize("change", ["head", "generation", "expiry"])
def test_new_target_generation_or_expiry_reissues_and_old_approval_releases_nothing(desk, signer, change):
    check(desk)
    old = desk.path(Scope.CANDIDATE_HEAD).read_bytes()
    signer.publish(desk.path(Scope.CANDIDATE_HEAD), Decision.APPROVE)
    kw = {"head": {"target": CommitTarget(OTHER)}, "generation": {"generation": 2},
          "expiry": {"now": NOW + timedelta(hours=24)}}[change]
    result = check(desk, **kw)
    assert result.status == "awaiting_human"
    assert desk.path(Scope.CANDIDATE_HEAD).read_bytes() != old
    assert old in [p.read_bytes() for p in (desk.folder / "history").iterdir()]
    if change == "head":
        assert check(desk).status == "awaiting_human"


def test_spec_body_digest_change_reissues(desk, signer):
    first = spec_target(1340, "SPEC r3\n".encode())
    check(desk, target=first, scope=Scope.SPEC_AUTHORIZATION)
    signer.publish(desk.path(Scope.SPEC_AUTHORIZATION), Decision.APPROVE)
    assert isinstance(check(desk, target=first, scope=Scope.SPEC_AUTHORIZATION), Approved)
    edited = spec_target(1340, "SPEC r3 \n".encode())
    assert check(desk, target=edited, scope=Scope.SPEC_AUTHORIZATION).status == "awaiting_human"


def test_tampered_or_replayed_request_file_never_borrows_an_approval(desk, signer, tmp_path):
    other = ApprovalDesk(tmp_path / "other" / "approvals", 4242, "t4242-2", desk.approvals)
    check(other)
    signer.publish(other.path(Scope.CANDIDATE_HEAD), Decision.APPROVE)
    desk.folder.mkdir(parents=True, exist_ok=True)
    desk.path(Scope.CANDIDATE_HEAD).write_bytes(other.path(Scope.CANDIDATE_HEAD).read_bytes())
    assert check(desk).status == "awaiting_human"
    desk.path(Scope.CANDIDATE_HEAD).write_bytes(b"newchanlun-approval-request/v1\nticket=4242\n")
    assert check(desk).status == "awaiting_human"


def test_spec_authorization_shape_is_only_the_spec_issue():
    assert SpecAuthorization.from_json({"spec_issue": 1340}) == SpecAuthorization(1340)
    for bad in ({"spec_issue": 0}, {"spec_issue": 1340, "github_object": "c-1"}, {"spec_issue": "1340"}):
        with pytest.raises(ValueError):
            SpecAuthorization.from_json(bad)


def test_signed_or_encoded_commits_are_unpublishable_before_any_ref():
    head = b"tree " + b"a" * 40 + b"\nauthor A <a@x> 1 +0000\ncommitter A <a@x> 1 +0000\n"
    assert commit_payload(head + b"\nmsg\n")["author"]["date"] == "1970-01-01T00:00:01Z"
    with pytest.raises(Unpublishable, match="gpgsig"):
        commit_payload(head + b"gpgsig -----BEGIN-----\n \n -----END-----\n\nmsg\n")
    with pytest.raises(Unpublishable, match="encoding"):
        commit_payload(head + b"encoding GBK\n\nmsg\n")
