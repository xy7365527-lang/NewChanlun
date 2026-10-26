from __future__ import annotations

import base64
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

import pytest
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from approval_signing.protocol import (
    ApprovalPayload,
    ApprovalRequest,
    CommitTarget,
    Decision,
    Scope,
    TrustRoot,
    VerifiedReceipt,
    key_id_for,
    parse_trust,
    verify_receipt,
)

HEAD = "120421fe338a9fcf8467b63aa7d6a28e3f871b8b"
OTHER_HEAD = "a" * 40
REPO = 1150335160


class SoftwareKey:
    """Software P-256 key standing in for the Secure Enclave key in offline tests."""

    def __init__(self) -> None:
        self.private = ec.generate_private_key(ec.SECP256R1())
        self.spki = self.private.public_key().public_bytes(
            serialization.Encoding.DER, serialization.PublicFormat.SubjectPublicKeyInfo
        )
        self.key_id = key_id_for(self.spki)

    def receipt(self, payload: bytes) -> bytes:
        signature = self.private.sign(payload, ec.ECDSA(hashes.SHA256()))
        return json.dumps({
            "format": "newchanlun-approval-receipt/v1",
            "algorithm": "ecdsa-p256-sha256",
            "key_id": self.key_id,
            "payload": base64.b64encode(payload).decode(),
            "signature": base64.b64encode(signature).decode(),
        }).encode()

    def trust(self, scopes: tuple[str, ...] = ("candidate_head", "spec_authorization")) -> TrustRoot:
        return parse_trust(json.dumps({
            "format": "newchanlun-approval-trust/v1",
            "repository_id": REPO,
            "allowed_scopes": list(scopes),
            "keys": [{
                "key_id": self.key_id,
                "public_key_spki_der_b64": base64.b64encode(self.spki).decode(),
                "label": "test",
            }],
        }).encode())


def make_request(head: str = HEAD, nonce: str = "0123456789abcdef0123456789abcdef",
                 expires: datetime = datetime(2099, 1, 1, tzinfo=timezone.utc)) -> ApprovalRequest:
    return ApprovalRequest(
        repository_id=REPO, ticket=1742, run_id="t1742-run.1", claim_generation=3,
        scope=Scope.CANDIDATE_HEAD, target=CommitTarget(head), request_nonce=nonce, expires_at=expires,
    )


class Chain:
    """Signs successive decisions for one request, the way the helper chains them."""

    def __init__(self, key: SoftwareKey, request: ApprovalRequest) -> None:
        self.key, self.request, self.trust = key, request, key.trust()
        self.links: list[VerifiedReceipt] = []

    def add(self, decision: Decision) -> VerifiedReceipt:
        previous = self.links[-1].receipt_id if self.links else None
        payload = ApprovalPayload(self.request, decision, len(self.links) + 1, previous)
        link = verify_receipt(self.key.receipt(payload.encode()), self.trust)
        self.links.append(link)
        return link


@pytest.fixture
def key() -> SoftwareKey:
    return SoftwareKey()
