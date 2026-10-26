"""Check approval receipts and the root-owned record.

    python -m approval_signing verify RECEIPT [RECEIPT ...]
    python -m approval_signing status REQUEST_FILE

`status` prints what the executor would decide for the request's own target now.
"""

from __future__ import annotations

import sys
from datetime import datetime, timezone
from pathlib import Path

from .decision import decide
from .protocol import DEFAULT_TRUST_PATH, ApprovalRequest, ProtocolError, load_trust, verify_receipt


def verify(names: list[str]) -> int:
    try:
        trust = load_trust(DEFAULT_TRUST_PATH)
    except (OSError, ProtocolError) as exc:
        print(f"trust config {DEFAULT_TRUST_PATH}: {exc}", file=sys.stderr)
        return 1
    status = 0
    for name in names:
        try:
            receipt = verify_receipt(Path(name).read_bytes(), trust)
        except (OSError, ProtocolError) as exc:
            print(f"INVALID {name}: {exc}")
            status = 1
            continue
        p = receipt.payload
        print(f"VALID {name} receipt_id={receipt.receipt_id} ticket=#{p.request.ticket} "
              f"scope={p.request.scope.value} target={p.request.target.encode()} decision={p.decision.value} "
              f"sequence={p.sequence} nonce={p.request.request_nonce}")
    return status


def status(name: str) -> int:
    try:
        request = ApprovalRequest.parse(Path(name).read_bytes())
        result = decide(request, request.target, now=datetime.now(timezone.utc))
    except (OSError, ProtocolError) as exc:
        print(f"{name}: {exc}", file=sys.stderr)
        return 1
    print(f"{result.status.value}: {result.reason}")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) >= 2 and argv[0] == "verify":
        return verify(argv[1:])
    if len(argv) == 2 and argv[0] == "status":
        return status(argv[1])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
