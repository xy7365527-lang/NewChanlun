from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import subprocess
import sys
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

STATE_ROOT = Path('/Volumes/AgentStorage/deepagents-exec')
DRAIN_ROOT = Path('/Volumes/AgentStorage/newchanlun-legacy-drain')
RELEASE_LABEL = str(json.loads((Path(__file__).resolve().parents[2] / 'queue-policy.json').read_text())['release_label'])
LEGACY_SERVICES = ('com.newchanlun.sandcastle-claimer', 'com.newchanlun.sandcastle-harvest')
# main.mts, claim-loop.mts, review-context.mts, swarm-managed.mts and swarm-merge.mts emit these pairs.
WORKER_EVENTS = {
    'claim': {'claimed', 'rolled_back'},
    'implementer': {'started', 'no_commit', 'completed', 'verification_failed'},
    'reviewer': {'started', 'sealed', 'completed', 'failed'},
    'managed-stage': {'failed'},
    'sandbox': {'closed'},
    'merger': {'ready', 'merging', 'conflict', 'prepared', 'committed', 'failed', 'done'},
}


class CutoverError(ValueError):
    pass


@dataclass(frozen=True)
class LegacyProcess:
    pid: int
    start_id: str
    cmd: str

    def __post_init__(self) -> None:
        if (type(self.pid) is not int or self.pid < 1 or not isinstance(self.start_id, str) or not self.start_id
                or not isinstance(self.cmd, str) or not self.cmd):
            raise CutoverError('invalid legacy process identity')


@dataclass(frozen=True)
class FrozenSnapshot:
    drain_generation: int
    frozen_at: str
    legacy_snapshot: tuple[LegacyProcess, ...]
    workers_log_length: int
    workers_log_path: str

    def __post_init__(self) -> None:
        if (type(self.drain_generation) is not int or self.drain_generation < 1
                or type(self.workers_log_length) is not int or self.workers_log_length < 0
                or not isinstance(self.frozen_at, str) or not self.frozen_at
                or not isinstance(self.workers_log_path, str) or not Path(self.workers_log_path).is_absolute()):
            raise CutoverError('invalid frozen snapshot')


@dataclass(frozen=True)
class RemoteBarrier:
    remote_jobs_idle: bool
    remote_freeze_var: str
    legacy_services_unloaded: tuple[str, ...]
    variable_source: str = 'user-token-readonly'


@dataclass(frozen=True)
class CutoverRecord:
    drain_generation: int
    frozen_at: str
    legacy_snapshot: tuple[LegacyProcess, ...]
    workers_log_length: int
    remote_jobs_idle: bool
    remote_freeze_var: str
    legacy_services_unloaded: tuple[str, ...]
    engine_head: str
    workers_log_path: str
    variable_source: str = 'user-token-readonly'
    release_label: str = RELEASE_LABEL
    open_claims: tuple[()] = ()
    v: int = 1


def worker_rows(raw: bytes) -> list[dict[str, Any]]:
    try:
        rows = [json.loads(line) for line in raw.decode('utf-8').splitlines() if line.strip()]
    except (UnicodeError, ValueError) as error:
        raise CutoverError(f'workers.jsonl unparsed: {error}') from None
    for row in rows:
        if (not isinstance(row, dict) or not isinstance(row.get('phase'), str)
                or row['phase'] not in WORKER_EVENTS or not isinstance(row.get('status'), str)
                or row['status'] not in WORKER_EVENTS[row['phase']]):
            raise CutoverError('workers.jsonl unknown phase/status; reconcile the legacy event schema')
    return rows


def worker_claims(rows: list[dict[str, Any]]) -> tuple[()]:
    worker_rows(('\n'.join(json.dumps(row) for row in rows)).encode())
    pending: set[str] = set()
    claimed: set[str] = set()
    for row in rows:
        key = str(row.get('ticket') or row.get('branch') or '')
        if not key:
            raise CutoverError('workers.jsonl missing ticket/branch')
        pair = row['phase'], row['status']
        if pair == ('claim', 'claimed'):
            if key in pending:
                raise CutoverError(f'workers.jsonl overlapping open claim: {key}')
            claimed.add(key)
            pending.add(key)
        elif pair in (('claim', 'rolled_back'), ('sandbox', 'closed')):
            claimed.discard(key)
            pending.discard(key)
        elif pair in (('reviewer', 'completed'), ('reviewer', 'failed')) and key not in claimed:
            pending.discard(key)
        elif pair == ('merger', 'done'):
            pending.discard(key)
        else:
            pending.add(key)
    if pending:
        raise CutoverError('workers.jsonl open legacy work: ' + ','.join(sorted(pending)))
    return ()


def legacy_processes() -> tuple[LegacyProcess, ...]:
    output = subprocess.run(['ps', '-axww', '-o', 'pid=,lstart=,stat=,command='], check=True,
                            capture_output=True, text=True).stdout
    rows = []
    needles = ('.sandcastle/main.mts', '.sandcastle/main-swarm.mts', 'codex-child', 'decision-claimer',
               'claim-loop.mts', 'harvest.sh', 'wayfinder_engine.mts')
    for line in output.splitlines():
        parts = line.split(None, 7)
        if len(parts) != 8 or parts[6].startswith('Z'):
            continue
        pid, start, cmd = int(parts[0]), ' '.join(parts[1:6]), parts[7]
        if pid != os.getpid() and any(n in cmd for n in needles):
            rows.append(LegacyProcess(pid, start, cmd))
    return tuple(rows)


class CutoverStore:
    def __init__(self, state_root: Path = STATE_ROOT, drain_root: Path = DRAIN_ROOT,
                 workers_log: Path | None = None):
        self.state_root, self.drain_root = state_root, drain_root
        self.path = state_root / 'cutover.json'
        self.workers_log = workers_log or Path('/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/workers.jsonl')

    def generation(self) -> int:
        try:
            data = json.loads((self.drain_root / 'state.json').read_text())
            if (data['v'] != 1 or data['mode'] != 'draining'
                    or type(data['generation']) is not int or data['generation'] < 1):
                raise ValueError('not draining')
            return int(data['generation'])
        except (OSError, ValueError, KeyError, TypeError) as error:
            raise CutoverError(f'drain state unavailable: {error}') from None

    @contextmanager
    def frozen(self) -> Iterator[CutoverRecord]:
        with (self.drain_root / 'admission.lock').open('a') as lock:
            fcntl.flock(lock, fcntl.LOCK_SH)
            yield self.load()

    def snapshot(self) -> FrozenSnapshot:
        from datetime import datetime, timezone
        return FrozenSnapshot(self.generation(), datetime.now(timezone.utc).isoformat(), legacy_processes(),
                              len(self.workers_log.read_bytes()), str(self.workers_log.resolve()))

    def verify(self, snapshot: FrozenSnapshot, remote: RemoteBarrier, processes: tuple[LegacyProcess, ...],
               engine_head: str) -> CutoverRecord:
        if str(self.workers_log.resolve()) != snapshot.workers_log_path:
            raise CutoverError('snapshot workers_log_path mismatch')
        if self.generation() != snapshot.drain_generation:
            raise CutoverError('drain generation mismatch')
        if {(p.pid, p.start_id) for p in snapshot.legacy_snapshot} & {(p.pid, p.start_id) for p in processes}:
            raise CutoverError('snapshot process still alive')
        try:
            raw = self.workers_log.read_bytes()
        except OSError as error:
            raise CutoverError(f'workers.jsonl unreadable: {error}') from None
        length = snapshot.workers_log_length
        if length < 0 or len(raw) < length or (length and raw[length - 1:length] != b'\n'):
            raise CutoverError('workers.jsonl truncated or snapshot splits a row')
        tail = worker_rows(raw[length:])
        if any(r['phase'] == 'claim' and r['status'] == 'claimed' for r in tail):
            raise CutoverError('new claim after frozen snapshot')
        worker_claims(worker_rows(raw))
        if (remote.remote_jobs_idle is not True or remote.remote_freeze_var != '1'
                or set(remote.legacy_services_unloaded) != set(LEGACY_SERVICES)
                or remote.variable_source != 'user-token-readonly'):
            raise CutoverError('remote jobs, freeze variable or legacy services not drained')
        if not re.fullmatch('[a-f0-9]{40}', engine_head):
            raise CutoverError('engine_head must be an exact commit')
        return CutoverRecord(snapshot.drain_generation, snapshot.frozen_at, snapshot.legacy_snapshot, length,
                             True, '1', remote.legacy_services_unloaded, engine_head, snapshot.workers_log_path)

    def record(self, record: CutoverRecord) -> CutoverRecord:
        if self.generation() != record.drain_generation:
            raise CutoverError('drain generation mismatch')
        self.state_root.mkdir(parents=True, exist_ok=True)
        payload = json.dumps(asdict(record), ensure_ascii=False, indent=2) + '\n'
        try:
            fd = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError:
            if json.loads(self.path.read_text()) != json.loads(payload):
                raise CutoverError('existing cutover record is different') from None
            return self.load()
        with os.fdopen(fd, 'w') as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        return self.load()

    def load(self) -> CutoverRecord:
        try:
            data = json.loads(self.path.read_text())
            if not isinstance(data, dict) or not all(isinstance(data.get(key), list)
                    for key in ('legacy_snapshot', 'legacy_services_unloaded', 'open_claims')):
                raise ValueError('cutover arrays missing')
            processes = tuple(LegacyProcess(**p) for p in data.pop('legacy_snapshot'))
            data['legacy_services_unloaded'] = tuple(data['legacy_services_unloaded'])
            data['open_claims'] = tuple(data['open_claims'])
            record = CutoverRecord(legacy_snapshot=processes, **data)
        except (OSError, ValueError, KeyError, TypeError) as error:
            raise CutoverError(f'cutover unavailable: {error}') from None
        if (type(record.v) is not int or record.v != 1
                or type(record.drain_generation) is not int or record.drain_generation < 1
                or not isinstance(record.frozen_at, str) or not record.frozen_at
                or not isinstance(record.workers_log_path, str) or not Path(record.workers_log_path).is_absolute()
                or any(type(p.pid) is not int or p.pid < 1 or not isinstance(p.start_id, str) or not p.start_id
                       or not isinstance(p.cmd, str) or not p.cmd for p in processes)
                or record.release_label != RELEASE_LABEL or record.open_claims != ()
                or record.remote_jobs_idle is not True or record.remote_freeze_var != '1'
                or record.variable_source != 'user-token-readonly'
                or len(record.legacy_services_unloaded) != len(LEGACY_SERVICES)
                or set(record.legacy_services_unloaded) != set(LEGACY_SERVICES)
                or type(record.workers_log_length) is not int or record.workers_log_length < 0
                or not isinstance(record.engine_head, str) or not re.fullmatch('[a-f0-9]{40}', record.engine_head)
                or record.drain_generation != self.generation()):
            raise CutoverError('cutover record/drain mismatch')
        return record

    def check_ticket(self, ticket: int, record: CutoverRecord) -> None:
        raw = Path(record.workers_log_path).read_bytes()
        if len(raw) < record.workers_log_length:
            raise CutoverError('workers.jsonl truncated')
        rows = worker_rows(raw)
        worker_claims(rows)
        if any(str(row.get('ticket')) == str(ticket) or row.get('branch') == f'sandcastle/issue-{ticket}'
               for row in worker_rows(raw[record.workers_log_length:])):
            raise CutoverError('ticket has legacy events after snapshot')


def remote_barrier(gateway: Any) -> RemoteBarrier:
    def pages(path: str, field: str) -> list[dict[str, Any]]:
        rows: list[dict[str, Any]] = []
        for page in range(1, 51):
            result = gateway.client.api('GET', gateway._p(path), params={'per_page': 100, 'page': page})
            chunk = result.data.get(field) if isinstance(result.data, dict) else None
            if not isinstance(chunk, list) or not all(isinstance(row, dict) for row in chunk):
                raise CutoverError(f'incomplete Actions {field} response')
            rows.extend(chunk)
            if not result.has_next:
                return rows
        raise CutoverError('Actions pagination limit exceeded')
    workflows = pages('/actions/workflows', 'workflows')
    required = {p.name for p in Path(__file__).resolve().parents[3].joinpath('.github/workflows').glob('agent-*.yml')}
    observed = {Path(str(w.get('path', ''))).name for w in workflows}
    if not required or not required <= observed:
        raise CutoverError('legacy workflow inventory incomplete')
    for workflow in workflows:
        if not Path(str(workflow.get('path', ''))).name.startswith('agent-'):
            continue
        runs = pages(f"/actions/workflows/{workflow['id']}/runs", 'workflow_runs')
        if any(run.get('status') != 'completed' for run in runs):
            raise CutoverError('legacy remote workflow still running')
    variable = subprocess.run(['gh', 'variable', 'get', 'SANDCASTLE_LEGACY_FREEZE', '--repo', gateway.repo],
                              check=True, capture_output=True, text=True).stdout.strip()
    unloaded = []
    for label in LEGACY_SERVICES:
        result = subprocess.run(['launchctl', 'print', f'gui/{os.getuid()}/{label}'], capture_output=True, text=True)
        if result.returncode != 0 and 'Could not find service' in result.stderr:
            unloaded.append(label)
        elif result.returncode != 0:
            raise CutoverError(f'launchctl could not inspect {label}: {result.stderr.strip()}')
    return RemoteBarrier(True, variable, tuple(unloaded))


def hold() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--state-root', type=Path, default=STATE_ROOT)
    parser.add_argument('--drain-root', type=Path, default=DRAIN_ROOT)
    args = parser.parse_args()
    try:
        with CutoverStore(args.state_root, args.drain_root).frozen():
            print('admitted', flush=True)
            sys.stdin.buffer.read()
        return 0
    except (OSError, CutoverError) as error:
        print(str(error), file=sys.stderr)
        return 75


if __name__ == '__main__':
    raise SystemExit(hold())
