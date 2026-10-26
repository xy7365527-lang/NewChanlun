from __future__ import annotations

import base64
import json
from pathlib import Path

import pytest
from conftest import HEAD, REPO, SoftwareKey, make_request

from approval_signing.protocol import (
    ApprovalPayload,
    ApprovalRequest,
    Decision,
    ProtocolError,
    Scope,
    SpecTarget,
    load_trust,
    parse_trust,
    receipt_id,
    spec_target,
    trust_path_problem,
    verify_receipt,
)

FIXTURES = Path(__file__).parent / "fixtures" / "swift"


def test_swift_signed_receipts_verify_and_match_python_encoding() -> None:
    trust = parse_trust((FIXTURES / "trust.json").read_bytes())
    request_bytes = (FIXTURES / "request.txt").read_bytes()
    request = ApprovalRequest.parse(request_bytes)
    assert request.encode() == request_bytes

    first = verify_receipt((FIXTURES / "receipt-1-approve.json").read_bytes(), trust)
    second = verify_receipt((FIXTURES / "receipt-2-reject.json").read_bytes(), trust)

    assert first.payload == ApprovalPayload(request, Decision.APPROVE, 1, None)
    assert first.payload.encode() == ApprovalPayload(request, Decision.APPROVE, 1, None).encode()
    assert second.payload == ApprovalPayload(request, Decision.REJECT, 2, first.receipt_id)


def test_changing_one_byte_of_the_head_breaks_the_signature(key: SoftwareKey) -> None:
    payload = ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode()
    raw = json.loads(key.receipt(payload))
    raw["payload"] = base64.b64encode(payload.replace(HEAD.encode(), ("0" + HEAD[1:]).encode())).decode()
    with pytest.raises(ProtocolError, match="signature"):
        verify_receipt(json.dumps(raw).encode(), key.trust())


def test_untrusted_key_is_refused(key: SoftwareKey) -> None:
    payload = ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode()
    with pytest.raises(ProtocolError, match="not trusted"):
        verify_receipt(SoftwareKey().receipt(payload), key.trust())


def test_signature_from_trusted_key_with_wrong_key_id_is_refused(key: SoftwareKey) -> None:
    other = SoftwareKey()
    raw = json.loads(other.receipt(ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode()))
    raw["key_id"] = key.key_id
    with pytest.raises(ProtocolError, match="signature"):
        verify_receipt(json.dumps(raw).encode(), key.trust())


def test_other_repository_and_disallowed_scope_are_refused(key: SoftwareKey) -> None:
    request = make_request()
    elsewhere = ApprovalRequest(REPO + 1, request.ticket, request.run_id, request.claim_generation,
                                request.scope, request.target, request.request_nonce, request.expires_at)
    with pytest.raises(ProtocolError, match="another repository"):
        verify_receipt(key.receipt(ApprovalPayload(elsewhere, Decision.APPROVE, 1, None).encode()), key.trust())

    spec = ApprovalRequest(REPO, 1742, "r", 1, Scope.SPEC_AUTHORIZATION, spec_target(1340, b"body"),
                           request.request_nonce, request.expires_at)
    receipt = key.receipt(ApprovalPayload(spec, Decision.APPROVE, 1, None).encode())
    with pytest.raises(ProtocolError, match="scope"):
        verify_receipt(receipt, key.trust(scopes=("candidate_head",)))
    assert verify_receipt(receipt, key.trust()).payload.request == spec


@pytest.mark.parametrize("mutate", [
    lambda t: t.replace("\n", "\r\n"),
    lambda t: t.replace("ticket=1742", "ticket=01742"),
    lambda t: t.replace(HEAD, HEAD.upper()),
    lambda t: t.replace("decision=approve", "decision=APPROVE"),
    lambda t: t.replace("scope=candidate_head", "scope=main_merge"),
    lambda t: t.replace("previous_receipt=none", "previous_receipt=" + "0" * 64),
    lambda t: t.replace("sequence=1", "sequence=0"),
    lambda t: t.replace("2099-01-01T00:00:00Z", "2099-02-30T00:00:00Z"),
    lambda t: t + "extra=1\n",
    lambda t: t[:-1],
    lambda t: t.replace("ticket=1742\nrun_id", "run_id").replace("\nexpires", "\nticket=1742\nexpires"),
])
def test_non_canonical_payloads_are_refused(key: SoftwareKey, mutate) -> None:  # type: ignore[no-untyped-def]
    text = ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode().decode()
    with pytest.raises(ProtocolError):
        verify_receipt(key.receipt(mutate(text).encode()), key.trust())


def test_receipt_envelope_is_strict(key: SoftwareKey) -> None:
    good = json.loads(key.receipt(ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode()))
    for change in ({"extra": "x"}, {"algorithm": "ecdsa-p384-sha384"}, {"signature": good["signature"] + "\n"}):
        with pytest.raises(ProtocolError):
            verify_receipt(json.dumps({**good, **change}).encode(), key.trust())


def test_spec_target_digests_the_raw_body() -> None:
    assert spec_target(1340, b"SPEC\n") != spec_target(1340, b"SPEC")
    assert spec_target(1340, b"SPEC") == SpecTarget(1340, receipt_id(b"SPEC"))


def test_trust_config_must_be_root_controlled(tmp_path: Path, key: SoftwareKey) -> None:
    path = tmp_path / "trust.json"
    path.write_bytes(json.dumps({"format": "x"}).encode())
    problem = trust_path_problem(path)
    assert problem is not None and "not owned by root" in problem
    with pytest.raises(ProtocolError, match="operator-controlled"):
        load_trust(path)
    assert trust_path_problem(Path("/bin/ls")) is None
    assert "symlink" in (trust_path_problem(Path("/etc/hosts")) or "")


def test_trust_key_id_must_match_the_key(key: SoftwareKey) -> None:
    raw = {
        "format": "newchanlun-approval-trust/v1", "repository_id": REPO, "allowed_scopes": ["candidate_head"],
        "keys": [{"key_id": "0" * 64, "public_key_spki_der_b64": base64.b64encode(key.spki).decode(), "label": ""}],
    }
    with pytest.raises(ProtocolError, match="key_id"):
        parse_trust(json.dumps(raw).encode())



@pytest.mark.parametrize("when, accepted", [
    ("2000-02-29T00:00:00Z", True),
    ("2096-02-29T23:59:59Z", True),
    ("2001-02-29T00:00:00Z", False),
    ("1500-02-29T00:00:00Z", False),
    ("1582-10-10T00:00:00Z", False),
    ("2100-01-01T00:00:00Z", False),
    ("2099-12-31T24:00:00Z", False),
])
def test_expiry_domain_matches_the_swift_helper(key: SoftwareKey, when: str, accepted: bool) -> None:
    text = ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode().decode()
    payload = text.replace("2099-01-01T00:00:00Z", when).encode()
    if accepted:
        assert ApprovalPayload.parse(payload).encode() == payload
    else:
        with pytest.raises(ProtocolError):
            ApprovalPayload.parse(payload)


def test_unreadable_receipts_raise_protocol_error(key: SoftwareKey) -> None:
    good = json.loads(key.receipt(ApprovalPayload(make_request(), Decision.APPROVE, 1, None).encode()))
    text = json.dumps(good)
    for blob in (json.dumps({**good, "payload": "雪"}).encode(), json.dumps({**good, "signature": "雪"}).encode(),
                 b"[" * 100_000, b"not json", text[:-1].encode() + b', "payload": "AA=="}'):
        with pytest.raises(ProtocolError):
            verify_receipt(blob, key.trust())
