"""Canonical bytes, receipts and trust roots for user-presence approvals (#1742 D8).

The signed object is a fixed line format. A parser accepts a byte string only if
re-encoding the parsed value reproduces exactly the same bytes, so one payload
has one encoding and the signature covers what the helper displayed.
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import re
import stat
from dataclasses import dataclass
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Final

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

PAYLOAD_HEADER: Final = "newchanlun-approval/v1"
REQUEST_HEADER: Final = "newchanlun-approval-request/v1"
RECEIPT_FORMAT: Final = "newchanlun-approval-receipt/v1"
TRUST_FORMAT: Final = "newchanlun-approval-trust/v1"
ALGORITHM: Final = "ecdsa-p256-sha256"
DEFAULT_TRUST_PATH: Final = Path("/Library/Application Support/NewChanlun/approval-trust.json")

_DECIMAL = re.compile(r"0|[1-9][0-9]{0,17}")
_RUN_ID = re.compile(r"[A-Za-z0-9._-]{1,64}")
_HEX32 = re.compile(r"[0-9a-f]{32}")
_HEX64 = re.compile(r"[0-9a-f]{64}")
# Years 2000-2099 only: there Python's proleptic calendar and Foundation's Gregorian agree.
_TIMESTAMP = re.compile(r"20[0-9]{2}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z")
_COMMIT_TARGET = re.compile(r"commit:([0-9a-f]{40})")
_SPEC_TARGET = re.compile(r"spec:([1-9][0-9]{0,17}):sha256:([0-9a-f]{64})")


class ProtocolError(ValueError):
    """Bytes, receipt or trust configuration that the protocol does not accept."""


class Scope(Enum):
    CANDIDATE_HEAD = "candidate_head"
    SPEC_AUTHORIZATION = "spec_authorization"


class Decision(Enum):
    APPROVE = "approve"
    REJECT = "reject"


@dataclass(frozen=True)
class CommitTarget:
    sha: str

    def encode(self) -> str:
        return f"commit:{self.sha}"


@dataclass(frozen=True)
class SpecTarget:
    spec_issue: int
    body_sha256: str

    def encode(self) -> str:
        return f"spec:{self.spec_issue}:sha256:{self.body_sha256}"


Target = CommitTarget | SpecTarget


def spec_target(spec_issue: int, body: bytes) -> SpecTarget:
    """Target for a SPEC body exactly as GitHub returned it (raw UTF-8, untrimmed)."""
    return SpecTarget(spec_issue, hashlib.sha256(body).hexdigest())


def _parse_target(scope: Scope, text: str) -> Target:
    if scope is Scope.CANDIDATE_HEAD:
        m = _COMMIT_TARGET.fullmatch(text)
        if m:
            return CommitTarget(m.group(1))
    else:
        m = _SPEC_TARGET.fullmatch(text)
        if m:
            return SpecTarget(int(m.group(1)), m.group(2))
    raise ProtocolError(f"target does not match scope {scope.value}")


def _format_time(value: datetime) -> str:
    if value.tzinfo is None or value.utcoffset() != timezone.utc.utcoffset(None):
        raise ProtocolError("expires_at must be UTC")
    if value.microsecond:
        raise ProtocolError("expires_at has whole seconds only")
    return value.strftime("%Y-%m-%dT%H:%M:%SZ")


def _parse_time(text: str) -> datetime:
    if not _TIMESTAMP.fullmatch(text):
        raise ProtocolError("expires_at is not 20YY-MM-DDTHH:MM:SSZ")
    try:
        return datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError as exc:
        raise ProtocolError("expires_at is not a valid time") from exc


def _parse_decimal(text: str) -> int:
    if not _DECIMAL.fullmatch(text):
        raise ProtocolError("not a canonical decimal")
    return int(text)


@dataclass(frozen=True)
class ApprovalRequest:
    """What the executor asks the human to decide. A new target or run state is a new request."""

    repository_id: int
    ticket: int
    run_id: str
    claim_generation: int
    scope: Scope
    target: Target
    request_nonce: str
    expires_at: datetime

    def __post_init__(self) -> None:
        if not _RUN_ID.fullmatch(self.run_id):
            raise ProtocolError("run_id")
        if not _HEX32.fullmatch(self.request_nonce):
            raise ProtocolError("request_nonce must be 32 lowercase hex")
        _parse_target(self.scope, self.target.encode())
        _format_time(self.expires_at)
        for value in (self.repository_id, self.ticket, self.claim_generation):
            _parse_decimal(str(value))

    def fields(self) -> list[tuple[str, str]]:
        return [
            ("repository_id", str(self.repository_id)),
            ("ticket", str(self.ticket)),
            ("run_id", self.run_id),
            ("claim_generation", str(self.claim_generation)),
            ("scope", self.scope.value),
            ("target", self.target.encode()),
        ]

    def encode(self) -> bytes:
        fields = [*self.fields(), ("request_nonce", self.request_nonce),
                  ("expires_at", _format_time(self.expires_at))]
        return _encode_lines(REQUEST_HEADER, fields)

    @staticmethod
    def parse(data: bytes) -> ApprovalRequest:
        request = _request_from(_parse_lines(data, REQUEST_HEADER, _REQUEST_KEYS))
        if request.encode() != data:
            raise ProtocolError("request bytes are not canonical")
        return request


@dataclass(frozen=True)
class ApprovalPayload:
    """The exact object a human signs: a request plus one decision in its chain."""

    request: ApprovalRequest
    decision: Decision
    sequence: int
    previous_receipt: str | None

    def __post_init__(self) -> None:
        if self.sequence < 1:
            raise ProtocolError("sequence starts at 1")
        if (self.sequence == 1) != (self.previous_receipt is None):
            raise ProtocolError("sequence 1 has no previous receipt; later ones must")
        if self.previous_receipt is not None and not _HEX64.fullmatch(self.previous_receipt):
            raise ProtocolError("previous_receipt must be 64 lowercase hex")

    def encode(self) -> bytes:
        r = self.request
        fields = [
            *r.fields(),
            ("decision", self.decision.value),
            ("request_nonce", r.request_nonce),
            ("sequence", str(self.sequence)),
            ("previous_receipt", self.previous_receipt or "none"),
            ("expires_at", _format_time(r.expires_at)),
        ]
        return _encode_lines(PAYLOAD_HEADER, fields)

    @staticmethod
    def parse(data: bytes) -> ApprovalPayload:
        values = _parse_lines(data, PAYLOAD_HEADER, _PAYLOAD_KEYS)
        try:
            decision = Decision(values["decision"])
        except ValueError as exc:
            raise ProtocolError("decision must be approve or reject") from exc
        previous = values["previous_receipt"]
        payload = ApprovalPayload(
            request=_request_from(values),
            decision=decision,
            sequence=_parse_decimal(values["sequence"]),
            previous_receipt=None if previous == "none" else previous,
        )
        if payload.encode() != data:
            raise ProtocolError("payload bytes are not canonical")
        return payload


_REQUEST_KEYS: Final = ("repository_id", "ticket", "run_id", "claim_generation", "scope",
                        "target", "request_nonce", "expires_at")
_PAYLOAD_KEYS: Final = ("repository_id", "ticket", "run_id", "claim_generation", "scope",
                        "target", "decision", "request_nonce", "sequence", "previous_receipt",
                        "expires_at")


def _encode_lines(header: str, fields: list[tuple[str, str]]) -> bytes:
    return ("\n".join([header, *(f"{k}={v}" for k, v in fields)]) + "\n").encode("ascii")


def _parse_lines(data: bytes, header: str, keys: tuple[str, ...]) -> dict[str, str]:
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError as exc:
        raise ProtocolError("payload must be ASCII") from exc
    if not text.endswith("\n"):
        raise ProtocolError("payload must end with LF")
    lines = text[:-1].split("\n")
    if lines[0] != header:
        raise ProtocolError(f"expected header {header}")
    if len(lines) - 1 != len(keys):
        raise ProtocolError("wrong number of fields")
    values: dict[str, str] = {}
    for key, line in zip(keys, lines[1:], strict=True):
        name, sep, value = line.partition("=")
        if name != key or not sep or not value:
            raise ProtocolError(f"expected field {key}")
        values[key] = value
    return values


def _request_from(values: dict[str, str]) -> ApprovalRequest:
    try:
        scope = Scope(values["scope"])
    except ValueError as exc:
        raise ProtocolError("unknown scope") from exc
    return ApprovalRequest(
        repository_id=_parse_decimal(values["repository_id"]),
        ticket=_parse_decimal(values["ticket"]),
        run_id=values["run_id"],
        claim_generation=_parse_decimal(values["claim_generation"]),
        scope=scope,
        target=_parse_target(scope, values["target"]),
        request_nonce=values["request_nonce"],
        expires_at=_parse_time(values["expires_at"]),
    )


def receipt_id(payload_bytes: bytes) -> str:
    """Identity of one decision. Two signatures over the same payload are the same decision."""
    return hashlib.sha256(payload_bytes).hexdigest()


def key_id_for(spki_der: bytes) -> str:
    return hashlib.sha256(spki_der).hexdigest()


@dataclass(frozen=True)
class TrustedKey:
    key_id: str
    public_key: ec.EllipticCurvePublicKey
    label: str


@dataclass(frozen=True)
class TrustRoot:
    repository_id: int
    allowed_scopes: frozenset[Scope]
    keys: dict[str, TrustedKey]


@dataclass(frozen=True)
class VerifiedReceipt:
    receipt_id: str
    key_id: str
    payload: ApprovalPayload


def _b64(value: object, field: str) -> bytes:
    if not isinstance(value, str):
        raise ProtocolError(f"{field} must be a base64 string")
    try:
        decoded = base64.b64decode(value, validate=True)
    except ValueError as exc:  # binascii.Error, or non-ASCII characters
        raise ProtocolError(f"{field} is not base64") from exc
    if base64.b64encode(decoded).decode("ascii") != value:
        raise ProtocolError(f"{field} is not canonical base64")
    return decoded


def _strict_json(data: bytes, what: str) -> object:
    """JSON with no duplicate keys, so every reader sees the same object."""

    def no_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
        if len({key for key, _ in pairs}) != len(pairs):
            raise ProtocolError(f"{what} has duplicate keys")
        return dict(pairs)

    try:
        return json.loads(data, object_pairs_hook=no_duplicates)
    except ProtocolError:
        raise
    except (ValueError, RecursionError) as exc:
        raise ProtocolError(f"{what} is not JSON") from exc


def parse_trust(data: bytes) -> TrustRoot:
    raw = _strict_json(data, "trust config")
    if not isinstance(raw, dict) or set(raw) != {"format", "repository_id", "allowed_scopes", "keys"}:
        raise ProtocolError("trust config fields")
    if raw["format"] != TRUST_FORMAT:
        raise ProtocolError("trust config format")
    repository_id = raw["repository_id"]
    if not isinstance(repository_id, int) or isinstance(repository_id, bool):
        raise ProtocolError("trust repository_id")
    scopes = raw["allowed_scopes"]
    if not isinstance(scopes, list) or not scopes:
        raise ProtocolError("trust allowed_scopes")
    try:
        allowed = frozenset(Scope(s) for s in scopes)
    except ValueError as exc:
        raise ProtocolError("trust allowed_scopes has unknown scope") from exc
    if not isinstance(raw["keys"], list) or not raw["keys"]:
        raise ProtocolError("trust keys")
    keys: dict[str, TrustedKey] = {}
    for entry in raw["keys"]:
        if not isinstance(entry, dict) or set(entry) != {"key_id", "public_key_spki_der_b64", "label"}:
            raise ProtocolError("trust key fields")
        spki = _b64(entry["public_key_spki_der_b64"], "public_key_spki_der_b64")
        if entry["key_id"] != key_id_for(spki):
            raise ProtocolError("trust key_id does not match public key")
        try:
            public_key = serialization.load_der_public_key(spki)
        except ValueError as exc:
            raise ProtocolError("trust public key is not SPKI DER") from exc
        if not isinstance(public_key, ec.EllipticCurvePublicKey) or not isinstance(
            public_key.curve, ec.SECP256R1
        ):
            raise ProtocolError("trust key must be P-256")
        if not isinstance(entry["label"], str):
            raise ProtocolError("trust key label")
        keys[entry["key_id"]] = TrustedKey(entry["key_id"], public_key, entry["label"])
    return TrustRoot(repository_id, allowed, keys)


def trust_path_problem(path: Path) -> str | None:
    """Why a non-root account could change `path`, or None if root controls it end to end.

    sudo on this machine asks for the operator's password, so a root-owned file
    is outside the executor's and the workers' write surface.
    """
    current = path
    while True:
        st = os.lstat(current)
        if stat.S_ISLNK(st.st_mode):
            return f"{current} is a symlink"
        if st.st_uid != 0:
            return f"{current} is not owned by root"
        if st.st_mode & (stat.S_IWGRP | stat.S_IWOTH):
            return f"{current} is group or world writable"
        if current == current.parent:
            return None
        current = current.parent


def load_trust(path: Path = DEFAULT_TRUST_PATH) -> TrustRoot:
    """The trust loader executors use. Refuses any config a non-root account can edit."""
    problem = trust_path_problem(path)
    if problem is not None:
        raise ProtocolError(f"trust config not operator-controlled: {problem}")
    return parse_trust(path.read_bytes())


def canonical_receipt(key_id: str, payload: bytes, signature: bytes) -> bytes:
    """The one accepted spelling of a receipt; the Swift helper writes exactly these bytes."""
    return (json.dumps({
        "format": RECEIPT_FORMAT,
        "algorithm": ALGORITHM,
        "key_id": key_id,
        "payload": base64.b64encode(payload).decode("ascii"),
        "signature": base64.b64encode(signature).decode("ascii"),
    }, sort_keys=True, separators=(",", ":")) + "\n").encode("ascii")


def verify_receipt(data: bytes, trust: TrustRoot) -> VerifiedReceipt:
    raw = _strict_json(data, "receipt")
    if not isinstance(raw, dict) or set(raw) != {"format", "algorithm", "key_id", "payload", "signature"}:
        raise ProtocolError("receipt fields")
    if raw["format"] != RECEIPT_FORMAT or raw["algorithm"] != ALGORITHM:
        raise ProtocolError("receipt format or algorithm")
    key = trust.keys.get(raw["key_id"]) if isinstance(raw["key_id"], str) else None
    if key is None:
        raise ProtocolError("receipt key is not trusted")
    payload_bytes = _b64(raw["payload"], "payload")
    signature = _b64(raw["signature"], "signature")
    try:
        key.public_key.verify(signature, payload_bytes, ec.ECDSA(hashes.SHA256()))
    except InvalidSignature as exc:
        raise ProtocolError("receipt signature is invalid") from exc
    if data != canonical_receipt(raw["key_id"], payload_bytes, signature):
        raise ProtocolError("receipt is not canonical")
    payload = ApprovalPayload.parse(payload_bytes)
    if payload.request.repository_id != trust.repository_id:
        raise ProtocolError("receipt is for another repository")
    if payload.request.scope not in trust.allowed_scopes:
        raise ProtocolError("receipt scope is not allowed")
    return VerifiedReceipt(receipt_id(payload_bytes), key.key_id, payload)
