from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import os
import re
import shlex
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
# 与旧 launchd / 受管 launcher 的真实执行 cwd 对齐，不接受调用者路径覆盖。
LEGACY_HOST_REPO = Path('/Users/silencehan/Projects/NewChanlun-sandcastle-host')
# #1481：launch[-s1d..s1j].pl 以 sandcastle-host 为 cwd，exec tsx + 绝对 runner 路径。
# 精确旧入口仅作 cwd/后代结构识别的补充，不作为动态受管写者的完整清单。
_MANAGED_ROOT = Path('/Volumes/AgentStorage/h1-managed43r')
MANAGED_LEGACY_ENTRYPOINTS = frozenset(
    str(_MANAGED_ROOT / f'{stem}{suffix}.{extension}')
    for stem, extension in (('runner', 'mts'), ('launch', 'pl'))
    for suffix in ('', '-s1d', '-s1e', '-s1f', '-s1g', '-s1h', '-s1i', '-s1j')
)
# Production emitters plus explicitly classified historical recovery/audit events.
# Recovery/failure is not a lifecycle terminal; corrections never change pending work.
WORKER_EVENTS = {
    'claim': {'claimed', 'rolled_back'},
    'implementer': {'started', 'no_commit', 'completed', 'verification_failed', 'failed', 'recovery_returned'},
    'correction': {'error_record_52009', 'error_record_53495'},
    'reviewer': {'started', 'sealed', 'completed', 'failed'},
    'managed-stage': {'failed'},
    # #1742: H1 runners finish the merge phase before authoring; neither event releases the sandbox.
    'merge': {'started', 'completed'},
    'reconciliation': {'released'},
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
    workers_log_sha256: str | None = None

    def __post_init__(self) -> None:
        if (type(self.drain_generation) is not int or self.drain_generation < 1
                or type(self.workers_log_length) is not int or self.workers_log_length < 0
                or not isinstance(self.frozen_at, str) or not self.frozen_at
                or not isinstance(self.workers_log_path, str) or not Path(self.workers_log_path).is_absolute()):
            raise CutoverError('invalid frozen snapshot')
        if self.workers_log_sha256 is not None and (not isinstance(self.workers_log_sha256, str)
                or not re.fullmatch('[a-f0-9]{64}', self.workers_log_sha256)):
            raise CutoverError('invalid frozen snapshot log digest')


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
    workers_log_sha256: str | None = None


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


def worker_key(row: dict[str, Any]) -> tuple[str, str]:
    ticket = str(row.get('ticket') or row.get('branch') or '')
    if not ticket:
        raise CutoverError('workers.jsonl missing ticket/branch')
    return ticket, str(row.get('branch') or '')


def worker_lifecycles(rows: list[dict[str, Any]]) -> dict[tuple[str, str], dict[str, Any]]:
    """#1742: source indices are 1-based parsed-event positions, not wall-clock guesses."""
    worker_rows(('\n'.join(json.dumps(row) for row in rows)).encode())
    pending: dict[tuple[str, str], dict[str, Any]] = {}
    claimed: set[tuple[str, str]] = set()
    for index, row in enumerate(rows, 1):
        key = worker_key(row)
        pair = row['phase'], row['status']
        if pair[0] == 'correction':
            continue
        if pair == ('reconciliation', 'released'):
            from .audit_release import validate_release
            if validate_release(row, pending.get(key)):
                claimed.discard(key)
                del pending[key]
            continue
        if (pair in (('claim', 'rolled_back'), ('sandbox', 'closed'), ('merger', 'done'))
                or pair in (('reviewer', 'completed'), ('reviewer', 'failed')) and key not in claimed):
            if pair != ('merger', 'done'):
                claimed.discard(key)
            pending.pop(key, None)
            continue
        if pair == ('claim', 'claimed'):
            claimed.add(key)
        if key not in pending or pair == ('claim', 'claimed'):
            pending[key] = {'first_index': index, 'first_event': row}
        pending[key] = {**pending[key], 'last_index': index, 'last_event': row}
    return pending


def worker_claims(rows: list[dict[str, Any]]) -> tuple[()]:
    pending = worker_lifecycles(rows)
    if pending:
        raise CutoverError('workers.jsonl open legacy work: ' + ','.join(sorted({key[0] for key in pending})))
    return ()


def legacy_processes() -> tuple[LegacyProcess, ...]:
    """从内核 cwd 与父子关系识别旧写者；控制端须位于旧宿主树外。"""
    host = LEGACY_HOST_REPO.resolve()
    if Path.cwd().resolve().is_relative_to(host):
        raise CutoverError('run cutover inspection outside legacy host; cd / first')

    def inventory(args: list[str]) -> str:
        try:
            result = subprocess.run(args, check=False, capture_output=True, text=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired):
            raise CutoverError(f'legacy inventory failed: {args[0]}') from None
        if result.returncode != 0 or result.stderr.strip() or not result.stdout.strip():
            raise CutoverError(f'legacy inventory failed: {args[0]}')
        return result.stdout

    output = inventory(['ps', '-axww', '-o', 'pid=,ppid=,lstart=,stat=,command='])
    processes: dict[int, tuple[int, str, str, str]] = {}
    for line in output.splitlines():
        parts = line.split(None, 8)
        if (len(parts) != 9 or not parts[0].isdigit() or not parts[1].isdigit()
                or int(parts[0]) < 1 or int(parts[0]) in processes):
            raise CutoverError('invalid legacy process inventory')
        processes[int(parts[0])] = int(parts[1]), ' '.join(parts[2:7]), parts[7], parts[8]

    def descendants(seeds: set[int]) -> set[int]:
        result = set(seeds)
        while True:
            expanded = result | {pid for pid, (parent, _, _, _) in processes.items() if parent in result}
            if expanded == result:
                return result
            result = expanded

    # 只排除当前检查器与它自己的采样子进程，祖先 shell 和其他业务 shell 仍在清单中。
    inspecting = descendants({os.getpid()})
    cwd_output = inventory(['lsof', '-nP', '-d', 'cwd', '-Fpn'])
    cwd_roots: set[int] = set()
    owner = None
    cwd_count = 0
    for line in cwd_output.splitlines():
        if line.startswith('p') and line[1:].isdigit() and int(line[1:]) > 0:
            owner = int(line[1:])
        elif line == 'fcwd' and owner is not None:
            continue  # lsof 即使 -Fpn 也会输出文件描述符标签。
        elif line.startswith('n') and owner is not None and Path(line[1:]).is_absolute():
            cwd_count += 1
            if Path(line[1:]).resolve().is_relative_to(host):
                cwd_roots.add(owner)
        else:
            raise CutoverError('invalid legacy cwd inventory')
    if not cwd_count:
        raise CutoverError('empty legacy cwd inventory')

    needles = ('.sandcastle/main.mts', '.sandcastle/main-swarm.mts', 'codex-child', 'decision-claimer',
               'claim-loop.mts', 'harvest.sh', 'wayfinder_engine.mts', 'health-monitor.mts')
    roots = cwd_roots - inspecting
    for pid, (_, _, _, cmd) in processes.items():
        try:
            argv = shlex.split(cmd)
        except ValueError:
            argv = []
        managed = (bool(argv) and Path(argv[0]).name in ('node', 'tsx', 'bun', 'perl')
                   and any(arg in MANAGED_LEGACY_ENTRYPOINTS for arg in argv[1:]))
        if pid not in inspecting and (any(n in cmd for n in needles) or managed):
            roots.add(pid)
    selected = descendants(roots) - inspecting
    return tuple(LegacyProcess(pid, start, cmd)
                 for pid, (_, start, stat, cmd) in processes.items()
                 if pid in selected and not stat.startswith('Z'))


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
        from datetime import UTC, datetime
        with (self.drain_root / 'admission.lock').open('a') as lock:
            fcntl.flock(lock, fcntl.LOCK_SH)
            generation = self.generation()
            processes = legacy_processes()
            raw = self.workers_log.read_bytes()
            return FrozenSnapshot(generation, datetime.now(UTC).isoformat(), processes,
                                  len(raw), str(self.workers_log.resolve()), hashlib.sha256(raw).hexdigest())

    def verify(self, snapshot: FrozenSnapshot, remote: RemoteBarrier, processes: tuple[LegacyProcess, ...],
               engine_head: str) -> CutoverRecord:
        if str(self.workers_log.resolve()) != snapshot.workers_log_path:
            raise CutoverError('snapshot workers_log_path mismatch')
        if self.generation() != snapshot.drain_generation:
            raise CutoverError('drain generation mismatch')
        if {(p.pid, p.start_id) for p in snapshot.legacy_snapshot} & {(p.pid, p.start_id) for p in processes}:
            raise CutoverError('snapshot process still alive')
        if processes:
            raise CutoverError('legacy writer still alive after frozen snapshot')
        try:
            raw = self.workers_log.read_bytes()
        except OSError as error:
            raise CutoverError(f'workers.jsonl unreadable: {error}') from None
        length = snapshot.workers_log_length
        if length < 0 or len(raw) < length or (length and raw[length - 1:length] != b'\n'):
            raise CutoverError('workers.jsonl truncated or snapshot splits a row')
        if snapshot.workers_log_sha256 is not None and hashlib.sha256(raw[:length]).hexdigest() != snapshot.workers_log_sha256:
            raise CutoverError('snapshot workers log prefix changed')
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
                             True, '1', remote.legacy_services_unloaded, engine_head, snapshot.workers_log_path,
                             workers_log_sha256=snapshot.workers_log_sha256)

    def record(self, record: CutoverRecord) -> CutoverRecord:
        if self.generation() != record.drain_generation:
            raise CutoverError('drain generation mismatch')
        self.state_root.mkdir(parents=True, exist_ok=True)
        payload = json.dumps(asdict(record), ensure_ascii=False, indent=2) + '\n'
        try:
            fd = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError:
            if self.load() != record:
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
                or (record.workers_log_sha256 is not None and
                    (not isinstance(record.workers_log_sha256, str) or not re.fullmatch('[a-f0-9]{64}', record.workers_log_sha256)))
                or record.drain_generation != self.generation()):
            raise CutoverError('cutover record/drain mismatch')
        return record

    def check_ticket(self, ticket: int, record: CutoverRecord) -> None:
        raw = Path(record.workers_log_path).read_bytes()
        if len(raw) < record.workers_log_length:
            raise CutoverError('workers.jsonl truncated')
        if record.workers_log_sha256 is not None and hashlib.sha256(raw[:record.workers_log_length]).hexdigest() != record.workers_log_sha256:
            raise CutoverError('snapshot workers log prefix changed')
        rows = worker_rows(raw)
        worker_claims(rows)
        if any(row['phase'] != 'reconciliation' and
               (str(row.get('ticket')) == str(ticket) or row.get('branch') == f'sandcastle/issue-{ticket}')
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
        result = subprocess.run(['launchctl', 'print', f'gui/{os.getuid()}/{label}'], check=False, capture_output=True, text=True)
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
