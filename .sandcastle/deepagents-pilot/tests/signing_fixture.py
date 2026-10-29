from __future__ import annotations

import base64
import json
from pathlib import Path

from approval_signing import authority, decision
from approval_signing.protocol import (
    ApprovalPayload,
    ApprovalRequest,
    Decision,
    TrustRoot,
    canonical_receipt,
    key_id_for,
    parse_trust,
)
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

from pilot1742.approval import REPOSITORY_ID, SignedApprovals


class Signer:
    """离线替身：软件 P-256 钥代替 Secure Enclave，按 publish 的写法往临时权威目录追加链节。

    只在测试里把 root 属主检查替换掉；生产代码没有任何参数能跳过它。"""

    def __init__(self, root: Path):
        self.private = ec.generate_private_key(ec.SECP256R1())
        spki = self.private.public_key().public_bytes(serialization.Encoding.DER,
                                                      serialization.PublicFormat.SubjectPublicKeyInfo)
        self.key_id = key_id_for(spki)
        self.trust: TrustRoot = parse_trust(json.dumps({
            "format": "newchanlun-approval-trust/v1", "repository_id": REPOSITORY_ID,
            "allowed_scopes": ["candidate_head", "spec_authorization"],
            "keys": [{"key_id": self.key_id, "public_key_spki_der_b64": base64.b64encode(spki).decode(),
                      "label": "pytest"}]}).encode())
        self.root = root
        root.mkdir(parents=True, exist_ok=True)

    def protect(self, monkeypatch) -> SignedApprovals:
        monkeypatch.setattr(decision, "load_trust", lambda path: self.trust)
        monkeypatch.setattr(authority, "trust_path_problem", lambda path: None)
        return SignedApprovals(authority_root=self.root)

    def receipt(self, request: ApprovalRequest, choice: Decision, sequence: int, previous: str | None) -> bytes:
        payload = ApprovalPayload(request, choice, sequence, previous).encode()
        return canonical_receipt(self.key_id, payload, self.private.sign(payload, ec.ECDSA(hashes.SHA256())))

    def publish(self, request_file: Path | str, choice: Decision) -> None:
        request = ApprovalRequest.parse(Path(request_file).read_bytes())
        folder = self.root / request.request_nonce
        folder.mkdir(exist_ok=True)
        chain = authority.load_chain(request.request_nonce, self.trust, self.root)
        previous = chain[-1].receipt_id if chain else None
        (folder / f"{len(chain) + 1}.json").write_bytes(self.receipt(request, choice, len(chain) + 1, previous))
