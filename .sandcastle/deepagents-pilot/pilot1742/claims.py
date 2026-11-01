from __future__ import annotations

import fcntl
import json
import os
import time
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import asdict, dataclass, replace
from datetime import datetime, timezone
from pathlib import Path
from typing import Literal, cast

from .domain import RunId, parse_run_id

CLAIMS_ROOT = Path("/Volumes/AgentStorage/deepagents-exec/claims")
LOCK_TIMEOUT_SECONDS = 300.0

ClaimState = Literal["intent", "mirrored", "active", "halted", "released"]
ClaimEvent = Literal["mirror", "activate", "halt", "release"]
TRANSITIONS: dict[ClaimEvent, dict[ClaimState, ClaimState]] = {
    "mirror": {"intent": "mirrored"},
    "activate": {"mirrored": "active"},
    "halt": {"intent": "halted", "mirrored": "halted", "active": "halted", "halted": "halted"},
    "release": {"intent": "released", "mirrored": "released", "active": "released", "halted": "released",
                "released": "released"},
}


class ClaimRefused(Exception):
    """这张票现在不能由本 run 持有；没有发生任何副作用。"""


class Fenced(Exception):
    """本 run 已不是这张票的 active 持有者；不执行该副作用。"""


class LockTimeout(Exception):
    """票级动作锁在时限内取不到；没有发生任何副作用。"""


@dataclass(frozen=True)
class TicketClaim:
    repo: str
    ticket: int
    run_id: RunId
    run_dir: str
    generation: int
    state: ClaimState
    admission_kind: str
    mirror_label: bool
    marker_comment: str | None
    reason: str | None
    updated_at: str

    def holds(self, run_id: RunId, run_dir: Path, generation: int) -> bool:
        return (self.state == "active" and self.run_id == run_id and self.generation == generation
                and self.run_dir == str(run_dir.resolve()))


def _now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def advance(claim: TicketClaim, event: ClaimEvent, **changes: object) -> TicketClaim:
    target = TRANSITIONS[event].get(claim.state)
    if target is None:
        raise ClaimRefused(f"持有记录 {claim.state} 不能执行 {event}")
    return replace(claim, state=target, updated_at=_now(), **changes)  # type: ignore[arg-type]


def _parse(data: dict) -> TicketClaim:
    if data.get("state") not in TRANSITIONS["release"]:
        raise ValueError("持有记录状态无效")
    return TicketClaim(str(data["repo"]), int(data["ticket"]), parse_run_id(data["run_id"]), str(data["run_dir"]),
                       int(data["generation"]), cast(ClaimState, data["state"]), str(data["admission_kind"]),
                       bool(data["mirror_label"]), data.get("marker_comment"), data.get("reason"),
                       str(data["updated_at"]))


class ClaimTxn:
    """持有票级动作锁期间的读改写。所有状态转换与外部副作用都在一个 txn 内完成。"""

    def __init__(self, path: Path):
        self.path = path
        self.record = _parse(json.loads(path.read_text())) if path.exists() else None

    def write(self, claim: TicketClaim) -> TicketClaim:
        tmp = self.path.with_name(f"{self.path.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(asdict(claim), ensure_ascii=False, indent=2) + "\n")
        os.replace(tmp, self.path)
        self.record = claim
        return claim

    def require_holder(self, run_id: RunId, run_dir: Path, generation: int) -> TicketClaim:
        record = self.record
        if record is None or not record.holds(run_id, run_dir, generation):
            state = "absent" if record is None else f"{record.state} run={record.run_id} gen={record.generation}"
            raise Fenced(f"本 run {run_id} gen={generation} 不是 active 持有者（记录：{state}）")
        return record


class ClaimStore:
    """本机唯一的票级持有记录与动作锁。位置固定，不随 run 的状态根变化。"""

    def __init__(self, root: Path = CLAIMS_ROOT, *, lock_timeout: float = LOCK_TIMEOUT_SECONDS):
        self.root = root
        self.lock_timeout = lock_timeout

    @contextmanager
    def locked(self, ticket: int) -> Iterator[ClaimTxn]:
        self.root.mkdir(parents=True, exist_ok=True)
        fd = os.open(self.root / f"{ticket}.lock", os.O_RDWR | os.O_CREAT, 0o600)
        try:
            deadline = time.monotonic() + self.lock_timeout
            while True:
                try:
                    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    if time.monotonic() >= deadline:
                        raise LockTimeout(f"票 {ticket} 的动作锁在 {self.lock_timeout}s 内未释放") from None
                    time.sleep(0.05)
            yield ClaimTxn(self.root / f"{ticket}.json")
        finally:
            os.close(fd)

    def read(self, ticket: int) -> TicketClaim | None:
        path = self.root / f"{ticket}.json"
        return _parse(json.loads(path.read_text())) if path.exists() else None
