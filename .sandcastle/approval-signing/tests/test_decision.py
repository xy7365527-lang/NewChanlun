from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

import pytest
from conftest import HEAD, OTHER_HEAD, Chain, SoftwareKey, make_request

from approval_signing import decision
from approval_signing.decision import Status, Watermark, decide, evaluate
from approval_signing.protocol import (
    ApprovalPayload,
    ApprovalRequest,
    CommitTarget,
    Decision,
    ProtocolError,
    Scope,
    VerifiedReceipt,
    spec_target,
    verify_receipt,
)

NOW = datetime(2026, 10, 27, tzinfo=timezone.utc)


def judge(request: ApprovalRequest, receipts: list[VerifiedReceipt], *, head: str = HEAD,
          watermark: Watermark | None = None, complete: bool = True, now: datetime = NOW):  # type: ignore[no-untyped-def]
    return evaluate(request, CommitTarget(head), receipts, now=now, watermark=watermark, receipts_complete=complete)


def test_nothing_signed_waits_for_the_human(key: SoftwareKey) -> None:
    result = judge(make_request(), [])
    assert result.status is Status.AWAITING_HUMAN and not result.releases


def test_latest_decision_wins(key: SoftwareKey) -> None:
    chain = Chain(key, make_request())
    approve = chain.add(Decision.APPROVE)
    first = judge(chain.request, chain.links)
    assert first.releases and first.watermark == Watermark(1, approve.receipt_id)

    chain.add(Decision.REJECT)
    assert judge(chain.request, chain.links).status is Status.REJECTED

    again = chain.add(Decision.APPROVE)
    result = judge(chain.request, chain.links)
    assert result.releases and result.watermark == Watermark(3, again.receipt_id)


def test_replaying_a_receipt_changes_nothing(key: SoftwareKey) -> None:
    chain = Chain(key, make_request())
    approve = chain.add(Decision.APPROVE)
    assert judge(chain.request, [approve, approve, approve]).watermark == Watermark(1, approve.receipt_id)


def test_new_head_invalidates_the_old_approval(key: SoftwareKey) -> None:
    old = Chain(key, make_request(HEAD))
    old.add(Decision.APPROVE)
    assert judge(old.request, old.links, head=OTHER_HEAD).status is Status.AWAITING_HUMAN

    new_request = make_request(OTHER_HEAD, nonce="f" * 32)
    assert judge(new_request, old.links, head=OTHER_HEAD).status is Status.AWAITING_HUMAN


def test_changed_spec_body_invalidates_spec_approval(key: SoftwareKey) -> None:
    base = make_request()
    request = ApprovalRequest(base.repository_id, 1742, base.run_id, 1, Scope.SPEC_AUTHORIZATION,
                              spec_target(1340, b"SPEC r3\n"), base.request_nonce, base.expires_at)
    chain = Chain(key, request)
    chain.add(Decision.APPROVE)
    assert evaluate(request, spec_target(1340, b"SPEC r3\n"), chain.links, now=NOW, watermark=None,
                    receipts_complete=True).releases
    edited = evaluate(request, spec_target(1340, b"SPEC r3 \n"), chain.links, now=NOW, watermark=None,
                      receipts_complete=True)
    assert edited.status is Status.AWAITING_HUMAN


def test_deleting_a_later_reject_does_not_revive_the_approve(key: SoftwareKey) -> None:
    chain = Chain(key, make_request())
    approve = chain.add(Decision.APPROVE)
    chain.add(Decision.REJECT)
    seen = judge(chain.request, chain.links).watermark
    assert judge(chain.request, [approve], watermark=seen).status is Status.NEEDS_RECONCILIATION
    assert judge(chain.request, [], watermark=seen).status is Status.NEEDS_RECONCILIATION


def test_forks_gaps_and_foreign_fields_need_reconciliation(key: SoftwareKey) -> None:
    chain = Chain(key, make_request())
    first = chain.add(Decision.APPROVE)
    second = chain.add(Decision.REJECT)

    other = Chain(key, chain.request)
    other.links = [first]
    fork = other.add(Decision.APPROVE)
    assert judge(chain.request, [first, second, fork]).status is Status.NEEDS_RECONCILIATION
    assert judge(chain.request, [second]).status is Status.NEEDS_RECONCILIATION

    wrong_previous = ApprovalPayload(chain.request, Decision.APPROVE, 2, "0" * 64)
    forged = verify_receipt(key.receipt(wrong_previous.encode()), key.trust())
    assert judge(chain.request, [first, forged]).status is Status.NEEDS_RECONCILIATION

    same_nonce_other_head = Chain(key, make_request(OTHER_HEAD))
    stray = same_nonce_other_head.add(Decision.APPROVE)
    assert judge(chain.request, [first, stray]).status is Status.NEEDS_RECONCILIATION


def test_watermark_must_match_the_chain(key: SoftwareKey) -> None:
    chain = Chain(key, make_request())
    chain.add(Decision.APPROVE)
    assert judge(chain.request, chain.links, watermark=Watermark(1, "0" * 64)).status is Status.NEEDS_RECONCILIATION


def test_expired_or_incomplete_never_approves_but_reject_still_holds(key: SoftwareKey) -> None:
    chain = Chain(key, make_request(expires=datetime(2026, 10, 28, tzinfo=timezone.utc)))
    chain.add(Decision.APPROVE)
    assert judge(chain.request, chain.links, now=datetime(2026, 10, 28, tzinfo=timezone.utc)).status \
        is Status.AWAITING_HUMAN
    assert judge(chain.request, chain.links, complete=False).status is Status.AWAITING_HUMAN

    chain.add(Decision.REJECT)
    assert judge(chain.request, chain.links, complete=False).status is Status.REJECTED


@pytest.mark.parametrize("sequence, receipt", [(0, "a" * 64), (-1, "a" * 64), (True, "a" * 64), (1, "A" * 64), (1, "")])
def test_malformed_watermark_cannot_exist(sequence: int, receipt: str) -> None:
    with pytest.raises(ProtocolError, match="watermark"):
        Watermark(sequence, receipt)


def test_decide_verifies_blobs_and_reports_the_ones_it_ignores(key: SoftwareKey, monkeypatch, tmp_path: Path) -> None:  # type: ignore[no-untyped-def]
    request = make_request()
    approve = ApprovalPayload(request, Decision.APPROVE, 1, None).encode()
    blobs = [key.receipt(approve), SoftwareKey().receipt(approve), b"not json"]

    untrusted = tmp_path / "trust.json"
    untrusted.write_bytes(b"{}")
    with pytest.raises(ProtocolError, match="operator-controlled"):
        decide(request, CommitTarget(HEAD), blobs, now=NOW, watermark=None, receipts_complete=True,
               trust_path=untrusted)

    monkeypatch.setattr(decision, "load_trust", lambda path: key.trust())
    result = decide(request, CommitTarget(HEAD), blobs, now=NOW, watermark=None, receipts_complete=True)
    assert result.releases and len(result.ignored) == 2
