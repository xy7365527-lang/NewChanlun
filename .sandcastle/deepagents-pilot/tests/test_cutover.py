import json
from dataclasses import replace

import pytest

from pilot1742.cutover import (
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    RemoteBarrier,
    worker_claims,
)


@pytest.fixture
def barrier(tmp_path):
    drain = tmp_path / 'drain'
    drain.mkdir()
    (drain / 'state.json').write_text(json.dumps({'v': 1, 'mode': 'draining', 'generation': 1}))
    log = tmp_path / 'workers.jsonl'
    log.write_text('')
    store = CutoverStore(tmp_path / 'exec', drain, log)
    snap = FrozenSnapshot(1, 'fixture', (), 0, str(log.resolve()))
    remote = RemoteBarrier(True, '1', ('com.newchanlun.sandcastle-claimer', 'com.newchanlun.sandcastle-harvest'))
    return store, snap, remote


def test_record_exclusive_idempotent_and_resume_invalidates(barrier):
    store, snap, remote = barrier
    record = store.verify(snap, remote, (), 'a' * 40)
    store.record(record)
    assert store.record(record) == store.load()
    assert store.path.stat().st_mode & 0o777 == 0o600
    with pytest.raises(CutoverError, match='different'):
        store.record(replace(record, engine_head='b' * 40))
    (store.drain_root / 'state.json').write_text('{"v":1,"mode":"open","generation":1}')
    with pytest.raises(CutoverError, match='drain'):
        store.load()


@pytest.mark.parametrize('bad', ['open', 'generation', 'alive', 'log_missing', 'log_short', 'tail',
                                 'open_claim', 'unknown', 'remote_jobs', 'variable', 'service'])
def test_each_missing_barrier_refuses(barrier, bad):
    store, snap, remote = barrier
    alive = ()
    if bad == 'open':
        (store.drain_root / 'state.json').write_text('{"v":1,"mode":"open","generation":1}')
    elif bad == 'generation':
        snap = replace(snap, drain_generation=2)
    elif bad == 'alive':
        proc = LegacyProcess(123, 'start-1', 'main.mts')
        snap, alive = replace(snap, legacy_snapshot=(proc,)), (proc,)
    elif bad == 'log_missing':
        store.workers_log.unlink()
    elif bad == 'log_short':
        snap = replace(snap, workers_log_length=10)
    elif bad in ('tail', 'open_claim', 'unknown'):
        row = {'ticket': 1, 'phase': 'claim' if bad != 'unknown' else 'future', 'status': 'claimed'}
        store.workers_log.write_text(json.dumps(row) + '\n')
        if bad != 'tail':
            snap = replace(snap, workers_log_length=store.workers_log.stat().st_size)
    elif bad == 'remote_jobs':
        remote = replace(remote, remote_jobs_idle=False)
    elif bad == 'variable':
        remote = replace(remote, remote_freeze_var='')
    else:
        remote = replace(remote, legacy_services_unloaded=())
    with pytest.raises(CutoverError):
        store.verify(snap, remote, alive, 'a' * 40)
    assert not store.path.exists()


def test_terminal_phases_are_not_all_completed_events():
    claimed = {'ticket': 1, 'phase': 'claim', 'status': 'claimed'}
    author = {'ticket': 1, 'phase': 'implementer', 'status': 'completed'}
    closed = {'ticket': 1, 'phase': 'sandbox', 'status': 'closed'}
    with pytest.raises(CutoverError, match='open'):
        worker_claims([claimed, author])
    assert worker_claims([claimed, author, closed]) == ()
    assert worker_claims([claimed, {'ticket': 1, 'phase': 'claim', 'status': 'rolled_back'}]) == ()
    with pytest.raises(CutoverError, match='unknown'):
        worker_claims([{'ticket': 1, 'phase': 'invented', 'status': 'completed'}])


def test_pid_reuse_is_not_the_snapshot_process(barrier):
    store, snap, remote = barrier
    snap = replace(snap, legacy_snapshot=(LegacyProcess(123, 'old', 'main.mts'),))
    assert store.verify(snap, remote, (LegacyProcess(123, 'new', 'main.mts'),), 'a' * 40)


def test_remote_barrier_reads_actions_and_variable_without_github_writes(monkeypatch):
    from pathlib import Path
    from types import SimpleNamespace

    from pilot1742.cutover import LEGACY_SERVICES, remote_barrier

    workflows = [{'id': i, 'path': str(path)} for i, path in enumerate(
        Path(__file__).resolve().parents[3].joinpath('.github/workflows').glob('agent-*.yml'), 1)]
    requests, commands = [], []

    def api(method, path, params):
        requests.append((method, path))
        data = {'workflows': workflows} if path.endswith('/workflows') else {'workflow_runs': [{'status': 'completed'}]}
        return SimpleNamespace(data=data, has_next=False)

    def command(args, **kwargs):
        commands.append(args)
        if args[0] == 'gh':
            assert args[:4] == ['gh', 'variable', 'get', 'SANDCASTLE_LEGACY_FREEZE']
            return SimpleNamespace(stdout='1\n', stderr='', returncode=0)
        assert args[:2] == ['launchctl', 'print']
        return SimpleNamespace(stdout='', stderr='Could not find service', returncode=113)

    monkeypatch.setattr('pilot1742.cutover.subprocess.run', command)
    gateway = SimpleNamespace(repo='xy7365527-lang/NewChanlun', client=SimpleNamespace(api=api), _p=lambda p: p)
    observed = remote_barrier(gateway)
    assert observed.remote_jobs_idle and observed.remote_freeze_var == '1'
    assert observed.variable_source == 'user-token-readonly'
    assert observed.legacy_services_unloaded == LEGACY_SERVICES
    assert all(method == 'GET' for method, _ in requests)
    assert len(commands) == 3
    gateway.client.api = lambda *args, **kwargs: SimpleNamespace(data={'workflows': []}, has_next=False)
    with pytest.raises(CutoverError, match='inventory incomplete'):
        remote_barrier(gateway)


def test_reclaimed_lifecycle_needs_a_terminal_not_perfect_historical_pairing():
    rows = [{'ticket': 1, 'phase': 'claim', 'status': 'claimed'},
            {'ticket': 1, 'phase': 'claim', 'status': 'claimed'}]
    with pytest.raises(CutoverError, match='open legacy work: 1'):
        worker_claims(rows)
    assert worker_claims([*rows, {'ticket': 1, 'phase': 'sandbox', 'status': 'closed'}]) == ()


@pytest.mark.parametrize('status', ['failed', 'recovery_returned'])
def test_recovery_and_author_failure_are_not_drain_evidence(status):
    rows = [{'ticket': 1, 'phase': 'claim', 'status': 'claimed'},
            {'ticket': 1, 'phase': 'implementer', 'status': status}]
    with pytest.raises(CutoverError, match='open legacy work: 1'):
        worker_claims(rows)
    assert worker_claims([*rows, {'ticket': 1, 'phase': 'sandbox', 'status': 'closed'}]) == ()


def test_audit_correction_does_not_close_or_reopen_work():
    correction = {'ticket': 1, 'phase': 'correction', 'status': 'error_record_52009'}
    assert worker_claims([correction]) == ()
    with pytest.raises(CutoverError, match='open legacy work: 1'):
        worker_claims([{'ticket': 1, 'phase': 'claim', 'status': 'claimed'}, correction])
    assert worker_claims([{'ticket': 1, 'phase': 'sandbox', 'status': 'closed'}, correction]) == ()


@pytest.mark.parametrize('branch', ['', 'sandcastle/issue-1-fix'])
def test_branch_close_does_not_hide_independent_work_for_same_ticket(branch):
    rows = [{'ticket': 1, 'branch': branch, 'phase': 'implementer', 'status': 'started'},
            {'ticket': 1, 'branch': 'sandcastle/issue-1', 'phase': 'sandbox', 'status': 'closed'}]
    with pytest.raises(CutoverError, match='open legacy work: 1'):
        worker_claims(rows)


def test_legacy_prefix_is_accepted_but_unfinished_work_stays_pending(barrier):
    from pathlib import Path

    from pilot1742.cutover import worker_rows

    raw = Path(__file__).with_name('fixtures').joinpath('legacy_workers_prefix.jsonl').read_bytes()
    rows = worker_rows(raw)
    with pytest.raises(CutoverError, match='open legacy work:') as error:
        worker_claims(rows)
    assert '1234' in str(error.value) and '1719' in str(error.value)
    tickets = sorted({row['ticket'] for row in rows if row['phase'] != 'correction'})
    closed = [{'ticket': ticket, 'phase': 'sandbox', 'status': 'closed'} for ticket in tickets]
    store, snap, remote = barrier
    store.workers_log.write_bytes(raw)
    snap = replace(snap, workers_log_length=len(raw))
    with pytest.raises(CutoverError, match='open legacy work:'):
        store.verify(snap, remote, (), 'a' * 40)
    with store.workers_log.open('a') as stream:
        stream.writelines(json.dumps(row) + '\n' for row in closed)
    record = store.verify(snap, remote, (), 'a' * 40)
    store.record(record)
    store.check_ticket(9999, record)


def test_runbook_stops_healer_first_and_restores_it_last():
    import subprocess
    from pathlib import Path

    script = Path(__file__).resolve().parents[2] / 'cutover-runbook.sh'
    output = subprocess.run(['bash', str(script)], capture_output=True, text=True, check=True).stdout
    healer_stop = output.index('launchctl bootout gui/')
    assert 'health-monitor' in output[healer_stop:].splitlines()[0]
    assert output.index('health-monitor') < output.index('cutover snapshot')
    assert output.rstrip().endswith('先恢复旧服务，最后恢复 health-monitor 自愈。')


@pytest.mark.parametrize('field,value', [('open_claims', ''), ('v', True), ('drain_generation', True),
                                       ('engine_head', 123), ('legacy_snapshot', [{'pid': True}])])
def test_bad_record_never_activates_queue(barrier, field, value):
    from dataclasses import asdict
    store, snap, remote = barrier
    data = asdict(store.verify(snap, remote, (), 'a' * 40))
    data[field] = value
    store.state_root.mkdir()
    store.path.write_text(json.dumps(data))
    with pytest.raises(CutoverError):
        store.load()


def test_snapshot_binds_log_source_and_ticket_checks_use_that_source(barrier, tmp_path):
    store, snap, remote = barrier
    other = tmp_path / 'another-workers.jsonl'
    other.write_text('')
    with pytest.raises(CutoverError, match='workers_log_path mismatch'):
        store.verify(replace(snap, workers_log_path=str(other)), remote, (), 'a' * 40)
    record = store.verify(snap, remote, (), 'a' * 40)
    store.record(record)
    reader = CutoverStore(store.state_root, store.drain_root, other)
    store.workers_log.write_text('{"ticket":3,"phase":"sandbox","status":"closed"}\n')
    with pytest.raises(CutoverError, match='after snapshot'):
        reader.check_ticket(3, reader.load())
