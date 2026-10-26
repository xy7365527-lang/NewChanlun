from __future__ import annotations

import json
import shutil
from datetime import datetime, timezone
from pathlib import Path

import pytest
from conftest import HEAD, OTHER_HEAD, Chain, SoftwareKey, make_request

from approval_signing import authority, decision
from approval_signing.decision import Evaluation, Status, decide
from approval_signing.protocol import (
    ApprovalPayload,
    ApprovalRequest,
    CommitTarget,
    Decision,
    parse_trust,
)

NOW = datetime(2026, 10, 27, tzinfo=timezone.utc)
FIXTURES = Path(__file__).parent / "fixtures" / "swift"


class Record:
    """A stand-in for the root-owned record, written the way `publish` writes it."""

    def __init__(self, root: Path, key: SoftwareKey, request: ApprovalRequest) -> None:
        self.root, self.key, self.chain = root, key, Chain(key, request)
        self.dir = root / request.request_nonce

    def publish(self, decision_: Decision) -> None:
        link = self.chain.add(decision_)
        self.dir.mkdir(parents=True, exist_ok=True)
        (self.dir / f"{link.payload.sequence}.json").write_bytes(self.key.receipt(link.payload.encode()))

    def decide(self, head: str = HEAD) -> Evaluation:
        return decide(self.chain.request, CommitTarget(head), now=NOW, authority_root=self.root)


@pytest.fixture
def record(tmp_path: Path, key: SoftwareKey, monkeypatch: pytest.MonkeyPatch) -> Record:
    """Record in a temp directory, with the root-ownership check treating it as protected."""
    monkeypatch.setattr(decision, "load_trust", lambda path: key.trust())
    monkeypatch.setattr(authority, "trust_path_problem", lambda path: None)
    root = tmp_path / "approval-receipts"
    root.mkdir()
    return Record(root, key, make_request())


def test_nothing_published_waits(record: Record) -> None:
    assert record.decide().status is Status.AWAITING_HUMAN


def test_only_published_decisions_count_and_the_latest_wins(record: Record) -> None:
    record.publish(Decision.APPROVE)
    assert record.decide().releases
    record.chain.add(Decision.REJECT)  # signed, never published
    assert record.decide().releases
    record.chain.links.pop()
    record.publish(Decision.REJECT)
    assert record.decide().status is Status.REJECTED
    assert record.decide(OTHER_HEAD).status is Status.AWAITING_HUMAN


@pytest.mark.parametrize("damage", [
    lambda d: (d / "1.json").unlink(),
    lambda d: (d / "2.json").write_bytes((d / "2.json").read_bytes().replace(b'"signature": "M', b'"signature": "N')),
    lambda d: (d / "2.json").rename(d / "3.json"),
    lambda d: (d / "notes.txt").write_text(""),
    lambda d: (d / "3.json").write_text("{}"),
    lambda d: ((d / "2.json").unlink(), (d / "2.json").symlink_to(d / "1.json")),
])
def test_damaged_record_fails_closed(record: Record, damage) -> None:  # type: ignore[no-untyped-def]
    record.publish(Decision.APPROVE)
    record.publish(Decision.REJECT)
    damage(record.dir)
    result = record.decide()
    assert result.status is Status.NEEDS_RECONCILIATION and not result.releases


def test_record_entry_from_an_untrusted_key_fails_closed(record: Record) -> None:
    record.publish(Decision.APPROVE)
    payload = ApprovalPayload(record.chain.request, Decision.APPROVE, 2, record.chain.links[0].receipt_id)
    (record.dir / "2.json").write_bytes(SoftwareKey().receipt(payload.encode()))
    assert record.decide().status is Status.NEEDS_RECONCILIATION


def test_pending_files_are_not_part_of_the_record(record: Record) -> None:
    record.publish(Decision.APPROVE)
    (record.dir / ".pending-2-123").write_text("half written")
    assert record.decide().releases


def test_a_user_writable_record_is_never_authoritative(tmp_path: Path, key: SoftwareKey,
                                                         monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(decision, "load_trust", lambda path: key.trust())
    record = Record(tmp_path, key, make_request())
    record.publish(Decision.APPROVE)
    result = record.decide()
    assert result.status is Status.NEEDS_RECONCILIATION and "not owned by root" in result.reason


def test_record_published_by_the_swift_helper(monkeypatch: pytest.MonkeyPatch) -> None:
    trust = parse_trust((FIXTURES / "trust.json").read_bytes())
    monkeypatch.setattr(decision, "load_trust", lambda path: trust)
    monkeypatch.setattr(authority, "trust_path_problem", lambda path: None)
    request = ApprovalRequest.parse((FIXTURES / "request.txt").read_bytes())
    chain = authority.load_chain(request.request_nonce, trust, FIXTURES / "authority")
    assert [link.payload.decision for link in chain] == [Decision.APPROVE, Decision.REJECT]
    stored = json.loads((FIXTURES / "authority" / request.request_nonce / "2.json").read_bytes())
    assert set(stored) == {"format", "algorithm", "key_id", "payload", "signature"}
    result = decide(request, request.target, now=NOW, authority_root=FIXTURES / "authority")
    assert result.status is Status.REJECTED


def test_copy_of_swift_record_with_reject_removed_is_detected_by_owner_check(tmp_path: Path,
                                                                             monkeypatch: pytest.MonkeyPatch) -> None:
    trust = parse_trust((FIXTURES / "trust.json").read_bytes())
    monkeypatch.setattr(decision, "load_trust", lambda path: trust)
    request = ApprovalRequest.parse((FIXTURES / "request.txt").read_bytes())
    copy = tmp_path / "authority"
    shutil.copytree(FIXTURES / "authority", copy)
    (copy / request.request_nonce / "2.json").unlink()
    result = decide(request, request.target, now=NOW, authority_root=copy)
    assert result.status is Status.NEEDS_RECONCILIATION
