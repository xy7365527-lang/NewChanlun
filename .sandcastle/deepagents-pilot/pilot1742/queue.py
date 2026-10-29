from __future__ import annotations

import fcntl
import json
import os
import re
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .approval import SpecAuthorization
from .claims import ClaimStore
from .cutover import CutoverError, CutoverStore
from .delivery import RepoGateway
from .domain import parse_sha
from .run import PilotRun, TaskSpec
from .workunit import (
    RELEASE_LABEL,
    Queued,
    QueueProof,
    WorkUnit,
    forbidden_for,
    parse_unit_run_id,
    parse_work_unit,
)

# claim-loop.mts runs one main child at a time. Queue intake preserves that production limit.
MAX_IN_FLIGHT = 1


class QueueWaiting(ValueError):
    pass


def approved_scope(body: str) -> tuple[str, ...]:
    declarations = re.findall(r'^deepagents-scope:[ \t]*(.+)$', body, re.MULTILINE)
    if len(declarations) != 1:
        raise QueueWaiting('awaiting_human: missing or ambiguous deepagents-scope in signed SPEC')
    prefixes = tuple(declarations[0].split())
    if not prefixes or not all(p.endswith('/') and not p.startswith('/') and '..' not in p and p != './'
                               and '\\' not in p and '//' not in p for p in prefixes):
        raise QueueWaiting('awaiting_human: deepagents-scope must contain explicit relative directory prefixes')
    return prefixes


def prove_queue(gateway: RepoGateway, ticket: int, cutover: CutoverStore, expected: QueueProof | None = None) -> QueueProof:
    from .executor import AdmissionRefused
    record = cutover.load()
    try:
        cutover.check_ticket(ticket, record)
    except (CutoverError, OSError) as error:
        raise AdmissionRefused(str(error)) from None
    issue = gateway.issue(ticket)
    labels = gateway.label_names(issue)
    linked = re.findall(r'Part of \[SPEC #(\d+)\]', str(issue.get('body') or ''))
    if len(set(linked)) != 1:
        raise AdmissionRefused('missing or ambiguous Part of [SPEC #n] contract')
    spec_issue = int(linked[0])
    proof = QueueProof(ticket, record.drain_generation, spec_issue)
    hits = forbidden_for(Queued(proof, SpecAuthorization(spec_issue)), labels)
    if issue.get('state') != 'open' or 'pull_request' in issue or RELEASE_LABEL not in labels or hits:
        raise AdmissionRefused('queue forbidden/not released: ' + ','.join(hits))
    if not isinstance(issue.get('assignees'), list) or issue['assignees']:
        raise AdmissionRefused('queue assignee present or unreadable')
    blocked = (issue.get('issue_dependencies_summary') or {}).get('blocked_by')
    if type(blocked) is not int or blocked != 0:
        raise AdmissionRefused('queue blockers present or unreadable')
    parent = gateway.parent(ticket)
    seen = {ticket}
    while parent is not None and parent != spec_issue:
        if parent in seen or len(seen) >= 30:
            raise AdmissionRefused('parent chain cyclic or incomplete')
        seen.add(parent)
        parent = gateway.parent(parent)
    if parent != spec_issue:
        spec_parent = gateway.parent(spec_issue)
        if (spec_parent is None or spec_parent not in seen or spec_parent == ticket
                or 'wayfinder:map' not in gateway.label_names(gateway.issue(spec_parent))):
            raise AdmissionRefused('SPEC link does not match parent chain or shared map')
    if expected is not None and proof != expected:
        raise AdmissionRefused('queue proof changed; register a new run after reconciliation')
    return proof


@dataclass
class QueueIntake:
    state_root: Path
    gateway: RepoGateway
    claims: ClaimStore
    cutover: CutoverStore
    source_repo: Path

    def register(self, ticket: int) -> WorkUnit:
        from .executor import AdmissionRefused, _task_json, load_unit, unit_path
        with self.claims.locked(ticket) as txn, self.cutover.frozen():
            proof = prove_queue(self.gateway, ticket, self.cutover)
            if txn.record is not None:
                record = txn.record
                if record.state != 'released':
                    unit = load_unit(self.state_root, record.run_id)[0]
                    if (record.run_dir != str(PilotRun.run_dir(self.state_root, unit.run_id).resolve())
                            or not isinstance(unit.admission, Queued)):
                        raise AdmissionRefused('ticket held by another executor/state root')
                    return unit
            units = self.state_root / 'units'
            existing = []
            for path in sorted(units.glob(f't{ticket}-*.json')):
                unit = load_unit(self.state_root, path.stem)[0]
                existing.append(unit)
            unreleased = [u for u in existing if txn.record is None
                          or int(u.run_id.rsplit('-', 1)[1]) > int(txn.record.run_id.rsplit('-', 1)[1])]
            if unreleased:
                unit = max(unreleased, key=lambda u: int(u.run_id.rsplit('-', 1)[1]))
                if not isinstance(unit.admission, Queued):
                    raise AdmissionRefused('ticket already registered outside queue')
                if unit.admission.proof != proof:
                    raise AdmissionRefused('registered queue proof changed')
                return unit
            issue, spec = self.gateway.issue(ticket), self.gateway.issue(proof.spec_issue)
            scope = approved_scope(str(spec.get('body') or ''))
            base = self.gateway.get_ref('main')
            if base is None:
                raise AdmissionRefused('main head unreadable')
            if subprocess.run(['git', '-C', str(self.source_repo), 'cat-file', '-e', f'{base}^{{commit}}'],
                              capture_output=True).returncode:
                raise QueueWaiting(f'awaiting_human: source checkout lacks main commit {base}; deploy/fetch main')
            run_id = parse_unit_run_id(ticket, f't{ticket}-{max([int(u.run_id.rsplit("-", 1)[1]) for u in existing], default=0) + 1}')
            labels = self.gateway.label_names(issue)
            graph = self.gateway.parent(proof.spec_issue)
            if graph is not None and 'wayfinder:map' not in self.gateway.label_names(self.gateway.issue(graph)):
                graph = None
            unit = WorkUnit(ticket, graph, parse_sha(base), scope,
                            'bug-fix' if 'bug' in labels else 'feature', False,
                            Queued(proof, SpecAuthorization(proof.spec_issue)), run_id)
            unit = parse_work_unit(unit.to_json())
            task = TaskSpec(str(issue['title']), str(issue.get('body') or ''), scope, False)
            path = unit_path(self.state_root, run_id)
            path.parent.mkdir(parents=True, exist_ok=True)
            data = {'work_unit': unit.to_json(), 'task': _task_json(task)}
            fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, 'w') as stream:
                json.dump(data, stream, ensure_ascii=False, indent=2)
                stream.write('\n')
                stream.flush()
                os.fsync(stream.fileno())
            return unit

    def once(self, run: Callable[[WorkUnit], int]) -> dict[int, dict[str, Any]]:
        from .executor import (
            AdmissionRefused,
            TicketExecutor,
            delivery_status,
            load_unit,
        )
        from .github_app import IdentityError
        from .run import Busy, Halted
        try:
            self.cutover.load()
        except CutoverError:
            return {}
        self.claims.root.mkdir(parents=True, exist_ok=True)
        fd = os.open(self.claims.root / 'serve.lock', os.O_RDWR | os.O_CREAT, 0o600)
        try:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                return {}
            tickets = {int(issue['number']) for issue in self.gateway.queue(RELEASE_LABEL)}
            for path in (self.state_root / 'units').glob('t*-*.json'):
                registered_unit = load_unit(self.state_root, path.stem)[0]
                if isinstance(registered_unit.admission, Queued):
                    tickets.add(registered_unit.ticket)
            results: dict[int, dict[str, Any]] = {ticket: {'phase': 'queued'} for ticket in tickets}
            for path in self.claims.root.glob('[0-9]*.json'):
                holder = self.claims.read(int(path.stem))
                if holder is None or holder.admission_kind != 'queued' or holder.state not in ('intent', 'mirrored', 'active'):
                    continue
                lock_path = Path(holder.run_dir) / 'owner.lock'
                if not lock_path.exists():
                    continue
                with lock_path.open('r+') as control_lock:
                    try:
                        fcntl.flock(control_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    except BlockingIOError:
                        results[holder.ticket] = {'phase': 'active', 'run_id': holder.run_id, 'reason': 'controller busy'}
                        return self._status(results)
            def priority(ticket: int) -> tuple[bool, int]:
                holder = self.claims.read(ticket)
                return (holder is None or holder.state == 'released', ticket)
            for ticket in sorted(tickets, key=priority):
                try:
                    self.cutover.load()
                except CutoverError:
                    break
                try:
                    claim = self.claims.read(ticket)
                    unit = load_unit(self.state_root, claim.run_id)[0] if claim and claim.state != 'released' else None
                    if claim is not None and unit is not None and (not isinstance(unit.admission, Queued)
                            or claim.run_dir != str(PilotRun.run_dir(self.state_root, unit.run_id).resolve())):
                        raise AdmissionRefused('ticket held by another executor/state root')
                    if unit is None or delivery_status(PilotRun.run_dir(self.state_root, unit.run_id))['phase'] not in (
                            'awaiting_merge', 'merged', 'stale'):
                        unit = self.register(ticket)
                    run_dir = PilotRun.run_dir(self.state_root, unit.run_id)
                    state = delivery_status(run_dir)
                    claim = self.claims.read(ticket)
                    if claim and claim.state in ('halted', 'released'):
                        results[ticket] = {'phase': 'halted', 'reason': claim.reason, 'run_id': unit.run_id}
                        continue
                    if state['phase'] in ('awaiting_merge', 'merged', 'stale'):
                        try:
                            state = TicketExecutor(unit, self.gateway, claims=self.claims).reconcile_delivery(run_dir)
                        except IdentityError as error:
                            results[ticket] = {'phase': 'awaiting_human', 'run_id': unit.run_id,
                                               'reason': f'delivery read failed; retry next round: {error}'}
                            continue
                        except Halted as error:
                            results[ticket] = {'phase': 'halted', 'run_id': unit.run_id, 'reason': str(error)}
                            continue
                        except Busy:
                            results[ticket] = {'phase': 'active', 'run_id': unit.run_id, 'reason': 'controller busy'}
                            return self._status(results)
                        results[ticket] = {**(state['state'] or {}), 'run_id': unit.run_id,
                                           'delivery_phase': state['phase'],
                                           'phase': 'awaiting_merge' if state['phase'] != 'stale' else 'halted'}
                        continue
                    if (run_dir / 'interrupt.request').exists():
                        results[ticket] = {'phase': 'awaiting_human', 'run_id': unit.run_id,
                                           'reason': 'operator interrupt; executor run --resume is required'}
                        continue
                    code = run(unit)
                    state = delivery_status(run_dir)
                    phase = state['phase']
                    reason = None
                    requests = sorted((run_dir / 'approvals').glob('*.status.json'))
                    pending = [json.loads(p.read_text()) for p in requests]
                    waiting = [d for d in pending if d.get('status') != 'approved']
                    if waiting:
                        phase, reason = 'awaiting_human', waiting[-1]
                    elif code == 75 or code < 0:
                        phase, reason = 'active', f'child exit {code}; same run resumes next round'
                    elif phase not in ('awaiting_merge', 'merged'):
                        phase, reason = 'halted', f'child exit {code}; inspect run status and checkpoint'
                    results[ticket] = {**(state['state'] or {}), 'run_id': unit.run_id, 'reason': reason,
                                       'phase': 'awaiting_merge' if phase == 'merged' else phase}
                    if code == 75 or code < 0:
                        break
                except QueueWaiting as error:
                    results[ticket] = {'phase': 'awaiting_human', 'reason': str(error)}
                except (AdmissionRefused, CutoverError) as error:
                    results[ticket] = {'phase': 'halted', 'reason': str(error)}
            return self._status(results)
        finally:
            os.close(fd)

    def _status(self, results: dict[int, dict[str, Any]]) -> dict[int, dict[str, Any]]:
        from .executor import _write_json
        _write_json(self.state_root / 'queue-status.json', {str(k): v for k, v in results.items()})
        status_path = self.source_repo / '.sandcastle' / 'logs' / 'deepagents-status.md'
        lines = ['# 新执行器队列', '']
        for phase in ('queued', 'active', 'awaiting_human', 'awaiting_merge', 'halted'):
            lines.extend([f'## {phase}', ''])
            lines.extend(f"- #{ticket} `{row.get('run_id', '')}` {row.get('head', '')} {row.get('reason') or ''}"
                         for ticket, row in results.items() if row['phase'] == phase)
            lines.append('')
        status_path.parent.mkdir(parents=True, exist_ok=True)
        tmp = status_path.with_name(f'{status_path.name}.{os.getpid()}.tmp')
        tmp.write_text('\n'.join(lines))
        os.replace(tmp, status_path)
        return results
