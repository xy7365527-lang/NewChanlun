"""Reader for the root-owned record of published approval decisions (#1742 D8).

A signed receipt takes effect only after the owner publishes it with
`sudo newchanlun-approve publish`, which appends it as the next link of its
request's chain under AUTHORITY_ROOT. Only root can add or remove entries
there, so this record, not GitHub or a user-writable watermark, is what the
executor reads.
"""

from __future__ import annotations

import os
import re
import stat
from pathlib import Path
from typing import Final

from .protocol import ProtocolError, TrustRoot, VerifiedReceipt, trust_path_problem, verify_receipt

AUTHORITY_ROOT: Final = Path("/Library/Application Support/NewChanlun/approval-receipts")
PENDING_PREFIX: Final = ".pending-"
_ENTRY = re.compile(r"([1-9][0-9]{0,17})\.json")
_NONCE = re.compile(r"[0-9a-f]{32}")


def load_chain(nonce: str, trust: TrustRoot, root: Path = AUTHORITY_ROOT) -> list[VerifiedReceipt]:
    """Every published decision for `nonce`, in order. Empty if none has been published.

    Raises ProtocolError for anything the publisher would never have written:
    an unprotected path, a foreign entry, a bad signature, an entry stored under
    the wrong sequence or nonce, a gap, or a broken link.
    """
    if not _NONCE.fullmatch(nonce):
        raise ProtocolError("nonce must be 32 lowercase hex")
    problem = trust_path_problem(root)
    if problem is not None:
        raise ProtocolError(f"authority root not operator-controlled: {problem}")
    directory = root / nonce
    try:
        st = os.lstat(directory)
    except FileNotFoundError:
        return []
    problem = trust_path_problem(directory)
    if problem is not None:
        raise ProtocolError(f"authority record not operator-controlled: {problem}")
    if not stat.S_ISDIR(st.st_mode):
        raise ProtocolError(f"{directory} is not a directory")

    by_sequence: dict[int, VerifiedReceipt] = {}
    for name in os.listdir(directory):
        if name.startswith(PENDING_PREFIX):
            continue
        match = _ENTRY.fullmatch(name)
        if match is None:
            raise ProtocolError(f"unexpected entry {name} in the authority record")
        path = directory / name
        problem = trust_path_problem(path)
        if problem is not None:
            raise ProtocolError(f"authority record not operator-controlled: {problem}")
        if not stat.S_ISREG(os.lstat(path).st_mode):
            raise ProtocolError(f"{path} is not a regular file")
        receipt = verify_receipt(path.read_bytes(), trust)
        if receipt.payload.sequence != int(match.group(1)) or receipt.payload.request.request_nonce != nonce:
            raise ProtocolError(f"{name} does not match its place in the record")
        by_sequence[receipt.payload.sequence] = receipt

    chain = [by_sequence[s] for s in sorted(by_sequence)]
    for index, link in enumerate(chain):
        previous = chain[index - 1].receipt_id if index else None
        if (link.payload.sequence != index + 1 or link.payload.previous_receipt != previous
                or link.payload.request != chain[0].payload.request):
            raise ProtocolError(f"authority record breaks at sequence {index + 1}")
    return chain
