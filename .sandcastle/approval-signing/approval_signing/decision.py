"""Current human decision for one approval request, from verified receipts.

Every receipt for a request carries a sequence and the id of the receipt before
it, so the decisions form one chain. The latest link wins: a later reject
overrides an earlier approve. The executor persists the chain head it has seen
(the watermark); receipts deleted after they were seen then show up as a
shorter chain. That only holds for decisions the executor already observed and
only while the watermark itself cannot be rolled back.
"""

from __future__ import annotations

import re
from collections.abc import Iterable
from dataclasses import dataclass, replace
from datetime import datetime
from enum import Enum
from pathlib import Path

from .protocol import (
    DEFAULT_TRUST_PATH,
    ApprovalRequest,
    Decision,
    ProtocolError,
    Target,
    VerifiedReceipt,
    load_trust,
    verify_receipt,
)

_HEX64 = re.compile(r"[0-9a-f]{64}")


@dataclass(frozen=True)
class Watermark:
    """Chain head already observed. A stored watermark that does not parse means stop; never replace it with None."""

    sequence: int
    receipt_id: str

    def __post_init__(self) -> None:
        if isinstance(self.sequence, bool) or not isinstance(self.sequence, int) or self.sequence < 1:
            raise ProtocolError("watermark sequence must be a positive integer")
        if not isinstance(self.receipt_id, str) or not _HEX64.fullmatch(self.receipt_id):
            raise ProtocolError("watermark receipt_id must be 64 lowercase hex")


class Status(Enum):
    APPROVED = "approved"
    REJECTED = "rejected"
    AWAITING_HUMAN = "awaiting_human"
    NEEDS_RECONCILIATION = "needs_reconciliation"


@dataclass(frozen=True)
class Evaluation:
    status: Status
    reason: str
    watermark: Watermark | None
    """Persist this before acting. None means nothing new was observed."""
    ignored: tuple[str, ...] = ()
    """Why each receipt blob passed to `decide` failed verification; those blobs count for nothing."""

    @property
    def releases(self) -> bool:
        return self.status is Status.APPROVED


def evaluate(
    request: ApprovalRequest,
    current_target: Target,
    receipts: Iterable[VerifiedReceipt],
    *,
    now: datetime,
    watermark: Watermark | None,
    receipts_complete: bool,
) -> Evaluation:
    """Decide whether `request` is approved right now.

    `current_target` is what the action would act on at this moment. A request
    for any other target is stale and approves nothing; the executor issues a
    new request with a new nonce. `receipts_complete` is False when the caller
    could not read every place receipts live (for example a truncated
    timeline); that can still reject but never approve.
    """
    if request.target != current_target:
        return Evaluation(Status.AWAITING_HUMAN, "target changed; issue a new request", watermark)

    chain: dict[int, VerifiedReceipt] = {}
    for receipt in receipts:
        if receipt.payload.request.request_nonce != request.request_nonce:
            continue
        if receipt.payload.request != request:
            return Evaluation(Status.NEEDS_RECONCILIATION,
                              f"receipt {receipt.receipt_id} has this nonce but other fields", watermark)
        first_at_sequence = chain.setdefault(receipt.payload.sequence, receipt)
        if first_at_sequence.receipt_id != receipt.receipt_id:
            return Evaluation(Status.NEEDS_RECONCILIATION,
                              f"two decisions at sequence {receipt.payload.sequence}", watermark)

    if not chain:
        if watermark is not None:
            return Evaluation(Status.NEEDS_RECONCILIATION, "previously seen receipts are gone", watermark)
        return Evaluation(Status.AWAITING_HUMAN, "no decision yet", None)

    head_sequence = max(chain)
    previous_id: str | None = None
    for sequence in range(1, head_sequence + 1):
        link = chain.get(sequence)
        if link is None or link.payload.previous_receipt != previous_id:
            return Evaluation(Status.NEEDS_RECONCILIATION, f"chain broken at sequence {sequence}", watermark)
        previous_id = link.receipt_id

    if watermark is not None:
        if watermark.sequence > head_sequence:
            return Evaluation(Status.NEEDS_RECONCILIATION,
                              f"chain shrank below seen sequence {watermark.sequence}", watermark)
        if chain[watermark.sequence].receipt_id != watermark.receipt_id:
            return Evaluation(Status.NEEDS_RECONCILIATION,
                              f"sequence {watermark.sequence} differs from the one seen", watermark)

    head = chain[head_sequence]
    seen = Watermark(head_sequence, head.receipt_id)
    if head.payload.decision is Decision.REJECT:
        return Evaluation(Status.REJECTED, f"rejected by {head.receipt_id}", seen)
    if now >= request.expires_at:
        return Evaluation(Status.AWAITING_HUMAN, "approval expired", seen)
    if not receipts_complete:
        return Evaluation(Status.AWAITING_HUMAN, "receipt set incomplete; cannot rule out a reject", seen)
    return Evaluation(Status.APPROVED, f"approved by {head.receipt_id}", seen)


def decide(
    request: ApprovalRequest,
    current_target: Target,
    receipt_blobs: Iterable[bytes],
    *,
    now: datetime,
    watermark: Watermark | None,
    receipts_complete: bool,
    trust_path: Path = DEFAULT_TRUST_PATH,
) -> Evaluation:
    """Executor entry point: load the root-owned trust config, verify every receipt blob, evaluate.

    Blobs that fail verification are reported in `ignored` and otherwise count
    for nothing. A trust config that is missing or not root-controlled raises.
    """
    trust = load_trust(trust_path)
    verified: list[VerifiedReceipt] = []
    ignored: list[str] = []
    for blob in receipt_blobs:
        try:
            verified.append(verify_receipt(blob, trust))
        except ProtocolError as exc:
            ignored.append(str(exc))
    result = evaluate(request, current_target, verified, now=now, watermark=watermark,
                      receipts_complete=receipts_complete)
    return replace(result, ignored=tuple(ignored))
