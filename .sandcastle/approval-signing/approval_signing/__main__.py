"""Check receipts against the installed root-owned trust config.

    python -m approval_signing verify RECEIPT [RECEIPT ...]
"""

from __future__ import annotations

import sys
from pathlib import Path

from .protocol import DEFAULT_TRUST_PATH, ProtocolError, load_trust, verify_receipt


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[0] != "verify":
        print(__doc__, file=sys.stderr)
        return 2
    try:
        trust = load_trust(DEFAULT_TRUST_PATH)
    except (OSError, ProtocolError) as exc:
        print(f"trust config {DEFAULT_TRUST_PATH}: {exc}", file=sys.stderr)
        return 1
    status = 0
    for name in argv[1:]:
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


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
