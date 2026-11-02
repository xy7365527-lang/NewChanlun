"""#1742：冻结后对遗留执行持有做当下审计释放，不补造历史完成事件。"""
from __future__ import annotations

import fcntl
import hashlib
import json
import os
import pwd
import re
import socket
import subprocess
from dataclasses import asdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from .cutover import (
    LEGACY_HOST_REPO,
    LEGACY_SERVICES,
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    legacy_processes,
    remote_barrier,
    worker_key,
    worker_lifecycles,
    worker_rows,
)

DISPOSITIONS = ('returned', 'terminated', 'abandoned_no_exit')


def digest(raw: bytes) -> str:
    """只用于日志同步冲突及审计证据内容绑定。"""
    return hashlib.sha256(raw).hexdigest()


def _sha(value: Any) -> bool:
    return isinstance(value, str) and re.fullmatch('[a-f0-9]{64}', value) is not None


def _absolute(value: Any) -> bool:
    return isinstance(value, str) and Path(value).is_absolute()


def validate_request(request: Any) -> dict[str, Any]:
    fields = {'v', 'ticket', 'branch', 'source', 'disposition', 'reason', 'evidence'}
    if (not isinstance(request, dict) or set(request) != fields
            or type(request['v']) is not int or request['v'] != 1
            or not isinstance(request['ticket'], str) or not request['ticket']
            or not isinstance(request['branch'], str)
            or request['disposition'] not in DISPOSITIONS
            or not isinstance(request['reason'], str) or not request['reason'].strip()
            or not isinstance(request['source'], dict)):
        raise CutoverError('invalid audit release request')
    source = request['source']
    if (set(source) != {'first_index', 'first_event', 'last_index', 'last_event'}
            or any(type(source[k]) is not int or source[k] < 1 for k in ('first_index', 'last_index'))
            or source['first_index'] > source['last_index']
            or any(not isinstance(source[k], dict) for k in ('first_event', 'last_event'))
            or any(worker_key(source[k]) != (request['ticket'], request['branch'])
                   for k in ('first_event', 'last_event'))):
        raise CutoverError('invalid audit lifecycle reference')
    evidence = request['evidence']
    if (not isinstance(evidence, list) or not evidence or any(
            not isinstance(e, dict) or set(e) != {'path', 'sha256'}
            or not _absolute(e['path']) or not _sha(e['sha256']) for e in evidence)
            or len({e['path'] for e in evidence}) != len(evidence)):
        raise CutoverError('invalid audit evidence identity')
    if request['disposition'] == 'abandoned_no_exit' and source['last_event'].get('exit_code') is not None:
        raise CutoverError('abandoned_no_exit cannot replace a recorded exit code')
    return request


def validate_release(row: dict[str, Any], source: dict[str, Any] | None) -> bool:
    """非法结构报错；合法但陈旧的审计记录返回 False，不改变当前持有。"""
    release = row.get('release')
    try:
        if (not isinstance(release, dict) or set(release) !=
                {'v', 'source', 'disposition', 'reason', 'evidence', 'drain_generation',
                 'snapshot', 'operator', 'observed', 'original_exit_code'}
                or set(row) != {'ts', 'ticket', 'branch', 'phase', 'status', 'release'}):
            raise ValueError()
        request = validate_request({k: release[k] for k in
                                    ('v', 'source', 'disposition', 'reason', 'evidence')} |
                                   {'ticket': str(row['ticket']), 'branch': row.get('branch', '')})
        snap = release['snapshot']
        snap = FrozenSnapshot(**(snap | {'legacy_snapshot': tuple(LegacyProcess(**p) for p in snap['legacy_snapshot'])}))
        observed, operator = release['observed'], release['operator']
        if (type(release['drain_generation']) is not int or
                release['drain_generation'] != snap.drain_generation or not _sha(snap.workers_log_sha256)
                or release['original_exit_code'] != request['source']['last_event'].get('exit_code')
                or not isinstance(row.get('ts'), str) or not row['ts']
                or not isinstance(operator, dict) or set(operator) != {'uid', 'name', 'host'}
                or type(operator['uid']) is not int or operator['uid'] < 0
                or any(not isinstance(operator[k], str) or not operator[k] for k in ('name', 'host'))
                or not isinstance(observed, dict)
                or set(observed) != {'at', 'targets', 'process_count', 'container_count', 'cwd_count',
                                    'legacy_processes', 'matched_pids', 'matched_cwds', 'matched_containers', 'remote'}
                or not isinstance(observed['at'], str) or not observed['at']
                or any(type(observed[k]) is not int or observed[k] < 0
                       for k in ('process_count', 'container_count', 'cwd_count'))
                or observed['process_count'] < 1 or observed['cwd_count'] < 1
                or any(observed[k] != [] for k in
                       ('legacy_processes', 'matched_pids', 'matched_cwds', 'matched_containers'))
                or not isinstance(observed['targets'], list) or not observed['targets']
                or not all(_absolute(p) for p in observed['targets'])
                or observed['remote'] != {'remote_jobs_idle': True, 'remote_freeze_var': '1',
                                         'legacy_services_unloaded': list(LEGACY_SERVICES),
                                         'variable_source': 'user-token-readonly'}):
            raise ValueError()
    except (KeyError, TypeError, ValueError):
        raise CutoverError('invalid reconciliation release') from None
    return source is not None and request['source'] == source


def pending_releases(store: CutoverStore) -> list[dict[str, Any]]:
    """只读导出真实生命周期，不猜 branch、PID 或历史退出码。"""
    return [{'ticket': key[0], 'branch': key[1], 'source': source}
            for key, source in sorted(worker_lifecycles(worker_rows(store.workers_log.read_bytes())).items())]


def _command(args: list[str]) -> str:
    try:
        result = subprocess.run(args, check=False, capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        raise CutoverError(f'audit observation timed out: {args[0]}') from None
    # 探针失败时不输出可能含私密进程参数的 stdout/stderr。
    if result.returncode != 0 or result.stderr.strip():
        raise CutoverError(f'audit observation failed: {args[0]}')
    return result.stdout


def lifecycle_targets(store: CutoverStore, rows: list[dict[str, Any]],
                      request: dict[str, Any]) -> tuple[list[str], set[int]]:
    source, key = request['source'], (request['ticket'], request['branch'])
    targets: set[str] = set()
    pids: set[int] = set()
    for row in rows[source['first_index'] - 1:source['last_index']]:
        if worker_key(row) != key or row['phase'] == 'correction':
            continue
        if row.get('pid') is not None:
            if type(row['pid']) is not int or row['pid'] < 1:
                raise CutoverError('invalid original worker pid')
            pids.add(row['pid'])
        for name in ('checkout', 'cwd', 'worktree'):
            if row.get(name) is not None:
                if not _absolute(row[name]):
                    raise CutoverError('invalid original worker checkout')
                targets.add(str(Path(row[name]).resolve()))
    # 清单来自旧执行宿主，不跟随共享日志 symlink，也不接受请求指定仓库。
    if key[1]:
        output = _command(['git', '-C', str(LEGACY_HOST_REPO), 'worktree', 'list', '--porcelain', '-z'])
        current = None
        for field in output.split('\0'):
            if field.startswith('worktree '):
                current = field[9:]
            elif field == 'branch refs/heads/' + key[1] and current:
                targets.add(str(Path(current).resolve()))
            elif not field:
                current = None
    if not targets:
        raise CutoverError('no authoritative checkout identity for lifecycle')
    return sorted(targets), pids


def observe(store: CutoverStore, rows: list[dict[str, Any]], request: dict[str, Any],
            gateway: Any, snapshot: FrozenSnapshot) -> dict[str, Any]:
    targets, pids = lifecycle_targets(store, rows, request)
    processes = legacy_processes()
    if processes:
        raise CutoverError('legacy control process still alive')
    remote = remote_barrier(gateway)
    if (remote.remote_jobs_idle is not True or remote.remote_freeze_var != '1'
            or set(remote.legacy_services_unloaded) != set(LEGACY_SERVICES)
            or remote.variable_source != 'user-token-readonly'):
        raise CutoverError('remote jobs, freeze variable or legacy services not drained')
    process_rows = _command(['ps', '-axo', 'pid=,lstart=,stat=']).splitlines()
    live_pids, identities = set(), set()
    for line in process_rows:
        parts = line.split()
        if len(parts) != 7 or not parts[0].isdigit():
            raise CutoverError('invalid process inventory')
        # 即使是 zombie 仍保留在 PID 清单，不能凭其年龄认定已释放。
        live_pids.add(int(parts[0]))
        identities.add((int(parts[0]), ' '.join(parts[1:6])))
    if identities & {(p.pid, p.start_id) for p in snapshot.legacy_snapshot}:
        raise CutoverError('snapshot process still alive')
    if not live_pids:
        raise CutoverError('empty process inventory')
    if pids & live_pids:
        raise CutoverError('original worker pid still present')

    cwd_rows = _command(['lsof', '-nP', '-d', 'cwd', '-Fpn']).splitlines()
    cwds = [line[1:] for line in cwd_rows if line.startswith('n')]
    if not cwds:
        raise CutoverError('empty cwd inventory')
    def overlaps(left: str, right: str) -> bool:
        a, b = Path(left).resolve(), Path(right).resolve()
        return a == b or a in b.parents or b in a.parents
    if any(Path(cwd).is_absolute() and any(
            Path(target).resolve() == Path(cwd).resolve() or Path(target).resolve() in Path(cwd).resolve().parents
            for target in targets) for cwd in cwds):
        raise CutoverError('worker checkout has a live cwd handle')

    ids = _command(['docker', 'ps', '-q', '--no-trunc']).split()
    containers = []
    if ids:
        # 只请求必要字段，绝不读取或记录 Docker Env/模型凭据。
        template = '{"id":{{json .Id}},"name":{{json .Name}},"running":{{json .State.Running}},"mounts":{{json .Mounts}}}'
        output = _command(['docker', 'inspect', '--format', template, *ids])
        try:
            containers = [json.loads(line) for line in output.splitlines()]
            if len(containers) != len(ids) or {c['id'] for c in containers} != set(ids):
                raise ValueError()
            for container in containers:
                if type(container['running']) is not bool or not isinstance(container['name'], str):
                    raise ValueError()
                if not isinstance(container['mounts'], list):
                    raise TypeError()
                sources = [m['Source'] for m in container['mounts']]
                if not all(_absolute(s) for s in sources):
                    raise ValueError()
                ticket_name = re.search(r'(?<![0-9])' + re.escape(request['ticket']) + r'(?![0-9])',
                                        container['name'])
                if ticket_name or any(overlaps(s, t) for s in sources for t in targets):
                    raise CutoverError('worker container still present')
        except (KeyError, TypeError, ValueError) as error:
            if isinstance(error, CutoverError):
                raise
            raise CutoverError('invalid container inventory') from None
    return {'at': datetime.now(UTC).isoformat(), 'targets': targets,
            'process_count': len(live_pids), 'container_count': len(containers), 'cwd_count': len(cwds),
            'legacy_processes': [], 'matched_pids': [], 'matched_cwds': [], 'matched_containers': [],
            'remote': {'remote_jobs_idle': True, 'remote_freeze_var': '1',
                       'legacy_services_unloaded': list(LEGACY_SERVICES), 'variable_source': 'user-token-readonly'}}


def _snapshot_matches(store: CutoverStore, snapshot: FrozenSnapshot, raw: bytes) -> None:
    length = snapshot.workers_log_length
    if (store.generation() != snapshot.drain_generation
            or str(store.workers_log.resolve()) != snapshot.workers_log_path
            or not _sha(snapshot.workers_log_sha256) or length > len(raw)
            or (length and raw[length - 1:length] != b'\n')
            or digest(raw[:length]) != snapshot.workers_log_sha256):
        raise CutoverError('audit snapshot generation, source or prefix changed')
    if raw and not raw.endswith(b'\n'):
        raise CutoverError('workers log has a partial final event')
    if any(r['phase'] == 'claim' and r['status'] == 'claimed' for r in worker_rows(raw[length:])):
        raise CutoverError('new claim after frozen snapshot')


def _evidence(request: dict[str, Any]) -> None:
    for item in request['evidence']:
        if digest(Path(item['path']).read_bytes()) != item['sha256']:
            raise CutoverError('audit evidence changed')


def audit_release(store: CutoverStore, snapshot: FrozenSnapshot, request: dict[str, Any],
                  gateway: Any) -> dict[str, Any]:
    request = validate_request(request)
    key = request['ticket'], request['branch']
    # 与 freeze/resume、旧准入同一把锁；没有 cutover.json 时也能审计遗留持有。
    with (store.drain_root / 'admission.lock').open('a') as admission:
        fcntl.flock(admission, fcntl.LOCK_EX)
        fd = os.open(store.workers_log, os.O_RDWR | os.O_APPEND | os.O_NOFOLLOW)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            stat = os.fstat(fd)
            raw = store.workers_log.read_bytes()
            _snapshot_matches(store, snapshot, raw)
            rows = worker_rows(raw)
            pending = worker_lifecycles(rows)
            prior = None
            if key not in pending:
                for index in range(len(rows) - 1, -1, -1):
                    row = rows[index]
                    if worker_key(row) != key or row['phase'] == 'correction':
                        continue
                    if row['phase'] != 'reconciliation':
                        break
                    if validate_release(row, worker_lifecycles(rows[:index]).get(key)):
                        prior = row
                        break
            if prior is not None:
                release = prior['release']
                if ({k: release[k] for k in ('v', 'source', 'disposition', 'reason', 'evidence')}
                        == {k: request[k] for k in ('v', 'source', 'disposition', 'reason', 'evidence')}
                        and release['snapshot'] == json.loads(json.dumps(asdict(snapshot)))):
                    _evidence(request)
                    return prior
                raise CutoverError('different prior audit release')
            if pending.get(key) != request['source']:
                raise CutoverError('stale audit lifecycle reference')
            _evidence(request)
            if worker_lifecycles(worker_rows(raw[:snapshot.workers_log_length])).get(key) != request['source']:
                raise CutoverError('lifecycle changed since frozen snapshot')
            observed = observe(store, rows, request, gateway, snapshot)
            record = {'ts': datetime.now(UTC).isoformat(), 'ticket': request['ticket'],
                      'branch': request['branch'], 'phase': 'reconciliation', 'status': 'released',
                      'release': {k: request[k] for k in ('v', 'source', 'disposition', 'reason', 'evidence')}}
            record['release'].update(
                drain_generation=snapshot.drain_generation,
                snapshot=json.loads(json.dumps(asdict(snapshot))),
                operator={'uid': os.getuid(), 'name': pwd.getpwuid(os.getuid()).pw_name, 'host': socket.gethostname()},
                observed=observed, original_exit_code=request['source']['last_event'].get('exit_code'))
            validate_release(record, pending[key])
            # 在现场取证期间任何追加、截断、换 inode 都要求重新取证；不覆盖生产者数据。
            current = store.workers_log.stat()
            if (current.st_dev, current.st_ino) != (stat.st_dev, stat.st_ino) or store.workers_log.read_bytes() != raw:
                raise CutoverError('workers log changed during audit observation')
            if store.generation() != snapshot.drain_generation:
                raise CutoverError('drain generation changed during audit observation')
            _evidence(request)
            payload = (json.dumps(record, ensure_ascii=False, separators=(',', ':')) + '\n').encode()
            if os.write(fd, payload) != len(payload):
                raise CutoverError('partial audit append; preserve log and inspect before retry')
            os.fsync(fd)
            # 旧 producer 不共享 flock；最终 CAS 后的新事件使旧 source 仅作审计，
            # pending 仍在。保留竞态原件，允许之后合法收尾，不永久毒化日志。
            after = store.workers_log.stat()
            if (after.st_dev, after.st_ino) != (stat.st_dev, stat.st_ino):
                raise CutoverError('workers log replaced after audit append')
            if key in worker_lifecycles(worker_rows(store.workers_log.read_bytes())):
                raise CutoverError('worker activity resumed after audit append')
            return record
        finally:
            os.close(fd)
