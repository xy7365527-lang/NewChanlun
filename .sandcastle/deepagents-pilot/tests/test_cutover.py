import json
import os
import plistlib
import subprocess
import sys
from dataclasses import replace
from pathlib import Path
from xml.sax.saxutils import escape

import pytest

from pilot1742.cutover import (
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    RemoteBarrier,
    worker_claims,
    worker_rows,
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


def test_pid_reuse_is_not_the_snapshot_process(barrier, legacy_inventory):
    from pilot1742.cutover import legacy_processes
    store, snap, remote = barrier
    snap = replace(snap, legacy_snapshot=(LegacyProcess(123, 'old', 'main.mts'),))
    legacy_inventory([(123, 1, '/usr/bin/unrelated', '/unrelated')])
    assert legacy_processes() == ()
    assert store.verify(snap, remote, legacy_processes(), 'a' * 40)


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


@pytest.fixture
def deployed_executor(tmp_path):
    repo = tmp_path / 'deployed repo'
    module = repo / '.sandcastle/deepagents-pilot/pilot1742'
    module.mkdir(parents=True)
    (module / '__init__.py').write_text('')
    marker = tmp_path / 'executed'
    (module / 'executor.py').write_text(
        'import json, os\n'
        'from pathlib import Path\n'
        f'Path({str(marker)!r}).write_text("executed")\n'
        'print(json.dumps({name: os.environ.get(name) for name in '
        '("PILOT_BOAT_CONFIG", "PILOT_BOAT_API_KEY_FILE")}))\n')
    venv = tmp_path / 'venv'
    (venv / 'bin').mkdir(parents=True)
    (venv / 'bin/python').symlink_to(sys.executable)
    for command in ('gh', 'launchctl', 'boat'):
        executable = venv / 'bin' / command
        executable.write_text(f'#!{sys.executable}\nfrom pathlib import Path\n'
                              f'Path({str(marker)!r}).write_text({command!r})\nraise SystemExit(91)\n')
        executable.chmod(0o700)
    paths = {
        '__REPO__': str(repo),
        '__VENV__': str(venv),
        '__PRIVATE_KEY_PATH__': str(tmp_path / 'app.pem'),
        '__BOAT_CONFIG__': str(tmp_path / 'Boat config.json'),
        '__BOAT_API_KEY_FILE__': str(tmp_path / 'Boat controller.key'),
    }
    environment = {key: value for key, value in os.environ.items()
                   if key not in ('PILOT_BOAT_CONFIG', 'PILOT_BOAT_API_KEY_FILE')}
    environment['PATH'] = str(venv / 'bin') + os.pathsep + environment.get('PATH', '')
    expected = {'PILOT_BOAT_CONFIG': paths['__BOAT_CONFIG__'],
                'PILOT_BOAT_API_KEY_FILE': paths['__BOAT_API_KEY_FILE__']}
    return paths, environment, expected, marker


def test_launchd_template_passes_boat_paths_to_executor(deployed_executor):
    paths, environment, expected, _ = deployed_executor
    template = Path(__file__).resolve().parents[2] / 'launchd/com.newchanlun.deepagents-executor.plist'
    rendered = template.read_text()
    for placeholder, value in paths.items():
        rendered = rendered.replace(placeholder, escape(value))
    config = plistlib.loads(rendered.encode())
    child = subprocess.run(config['ProgramArguments'], cwd=config['WorkingDirectory'],
                           env=environment | config['EnvironmentVariables'],
                           capture_output=True, text=True, check=True)
    assert json.loads(child.stdout) == expected


def test_runbook_prints_boat_paths_without_executing(deployed_executor):
    paths, environment, expected, marker = deployed_executor
    script = Path(__file__).resolve().parents[2] / 'cutover-runbook.sh'
    preview = subprocess.run(['bash', str(script), *paths.values()], env=environment,
                             capture_output=True, text=True, check=True)
    assert not marker.exists()
    line = next(line for line in preview.stdout.splitlines() if ' serve --once ' in line)
    child = subprocess.run(['bash', '-c', line], cwd=paths['__REPO__'], env=environment,
                           capture_output=True, text=True, check=True)
    assert json.loads(child.stdout) == expected
    assert marker.read_text() == 'executed'


def test_runbook_stops_healer_first_and_restores_it_last():
    import subprocess
    from pathlib import Path

    script = Path(__file__).resolve().parents[2] / 'cutover-runbook.sh'
    output = subprocess.run(['bash', str(script)], capture_output=True, text=True, check=True).stdout
    healer_stop = output.index('launchctl bootout gui/')
    assert 'health-monitor' in output[healer_stop:].splitlines()[0]
    assert output.index('health-monitor') < output.index('cutover snapshot')
    assert output.index('cd /') < output.index('cutover snapshot')
    assert output.rstrip().endswith('先恢复旧服务，最后恢复 health-monitor 自愈。')


def test_runbook_drains_hotzone_and_shells_before_record_then_restores_on_rollback():
    script = Path(__file__).resolve().parents[2] / 'cutover-runbook.sh'
    output = subprocess.run(['bash', str(script)], capture_output=True, text=True, check=True).stdout
    lines = output.splitlines()
    hotzone_stop = next(i for i, line in enumerate(lines)
                        if line.startswith('launchctl bootout ') and line.endswith('/com.newchanlun.arch-hotzone '))
    verify = next(i for i, line in enumerate(lines) if ' cutover verify ' in line)
    record = next(i for i, line in enumerate(lines) if ' cutover record ' in line)
    bootstrap = next(i for i, line in enumerate(lines) if line.startswith('launchctl bootstrap '))
    assert hotzone_stop < verify < record < bootstrap
    assert '旧宿主内业务 shell 由其 owner 离开该目录或结束' in output
    rollback = output.split('# 回退先让新执行器', 1)[1]
    assert '回退恢复清单' in rollback and 'arch-hotzone' in rollback
    assert rollback.index('arch-hotzone') < rollback.index('health-monitor')
    assert 'arch-hotzone 是否重启由部署 owner 在 record 后决定' in output


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


@pytest.fixture
def merge_lifecycle():
    # #1742: production #1481 merge failure/close followed by a new author stage.
    return worker_rows(Path(__file__).with_name('fixtures').joinpath('legacy_merge_lifecycle.jsonl').read_bytes())


def test_merge_lifecycle_closes_only_at_sandbox_close_and_reopens(merge_lifecycle):
    for length in (1, 2, 4, 5):
        with pytest.raises(CutoverError, match='open legacy work: 1481$'):
            worker_claims(merge_lifecycle[:length])
    assert worker_claims(merge_lifecycle[:3]) == ()
    assert worker_claims(merge_lifecycle) == ()


@pytest.mark.parametrize('status', ['started', 'completed'])
def test_merge_stage_with_unknown_exit_stays_pending(merge_lifecycle, status):
    # runner-s1d.mts emits completed before starting the author, not at resource release.
    row = dict(merge_lifecycle[0], status=status, exit_code=None)
    with pytest.raises(CutoverError, match='open legacy work: 1481$'):
        worker_claims([merge_lifecycle[0], row])
    assert worker_claims([merge_lifecycle[0], row, merge_lifecycle[2]]) == ()


@pytest.mark.parametrize('branch', ['', 'sandcastle/issue-1481-fix'])
def test_merge_close_does_not_release_another_branch(merge_lifecycle, branch):
    started = dict(merge_lifecycle[0], branch=branch)
    with pytest.raises(CutoverError, match='open legacy work: 1481$'):
        worker_claims([started, merge_lifecycle[2]])
    assert worker_claims([started, dict(merge_lifecycle[2], branch=branch)]) == ()


@pytest.mark.parametrize('phase,status', [('merge', 'finished'), ('merge-future', 'started')])
def test_merge_unknown_events_are_still_rejected(merge_lifecycle, phase, status):
    with pytest.raises(CutoverError, match='unknown phase/status'):
        worker_rows(json.dumps(dict(merge_lifecycle[0], phase=phase, status=status)).encode())


def test_merge_lifecycle_does_not_clear_other_open_legacy_keys(merge_lifecycle):
    open_rows = [
        {'ticket': 1240, 'branch': 'sandcastle/issue-1240', 'phase': 'implementer', 'status': 'started'},
        {'ticket': 1394, 'branch': 'sandcastle/issue-1394', 'phase': 'implementer', 'status': 'started'},
        {'ticket': 1488, 'phase': 'implementer', 'status': 'started'},
        {'ticket': 1621, 'phase': 'implementer', 'status': 'started'},
        {'ticket': 1622, 'branch': 'sandcastle/issue-1622-fix', 'phase': 'implementer', 'status': 'completed'},
    ]
    with pytest.raises(CutoverError, match='open legacy work: 1240,1394,1488,1621,1622$'):
        worker_claims([*open_rows, *merge_lifecycle])


def test_merge_events_are_enforced_by_cutover_consumers(barrier, merge_lifecycle):
    store, snap, remote = barrier
    raw = ('\n'.join(json.dumps(row) for row in merge_lifecycle) + '\n').encode()
    store.workers_log.write_bytes(raw)
    snap = replace(snap, workers_log_length=len(raw))
    record = store.verify(snap, remote, (), 'a' * 40)
    store.record(record)
    with store.workers_log.open('a') as stream:
        stream.write(json.dumps(merge_lifecycle[0]) + '\n')
    with pytest.raises(CutoverError, match='open legacy work: 1481$'):
        store.verify(snap, remote, (), 'a' * 40)
    with pytest.raises(CutoverError, match='open legacy work: 1481$'):
        store.check_ticket(9999, record)
    with store.workers_log.open('a') as stream:
        stream.write(json.dumps(merge_lifecycle[2]) + '\n')
    assert store.verify(snap, remote, (), 'a' * 40) == record
    store.check_ticket(9999, record)
    with pytest.raises(CutoverError, match='after snapshot'):
        store.check_ticket(1481, record)


@pytest.mark.parametrize('command,expected', [
    ('/opt/homebrew/bin/node --import tsx /Volumes/AgentStorage/h1-managed43r/runner-s1d.mts', True),
    ('./node_modules/.bin/tsx /Volumes/AgentStorage/h1-managed43r/runner-s1j.mts', True),
    ('/usr/bin/perl /Volumes/AgentStorage/h1-managed43r/launch-s1d.pl', True),
    ('/opt/homebrew/bin/node /Volumes/AgentStorage/h1-managed43r/runner.mts', True),
    ('/opt/homebrew/bin/node /unrelated/runner-s1d.mts', False),
    ('/opt/homebrew/bin/node /Volumes/AgentStorage/h1-managed43r/runner-s1d.mts.backup', False),
    ('/usr/bin/grep /Volumes/AgentStorage/h1-managed43r/runner-s1d.mts', False),
    ('/bin/echo /Volumes/AgentStorage/h1-managed43r/runner-s1d.mts', False),
    ('/opt/homebrew/bin/node /Volumes/AgentStorage/h1-managed43r/S1D-RUNTIME.json', False),
])
def test_managed_legacy_processes_use_bounded_entrypoint_inventory(legacy_inventory, command, expected):
    from pilot1742.cutover import legacy_processes
    legacy_inventory([(99999, 1, command, '/unrelated')])
    assert bool(legacy_processes()) is expected


def test_managed_runner_is_captured_in_snapshot_and_blocks_verify(barrier, legacy_inventory):
    from pilot1742.cutover import LEGACY_HOST_REPO, legacy_processes
    command = '/opt/homebrew/bin/node --import tsx /Volumes/AgentStorage/l2-fix45y/runner.mts'
    legacy_inventory([(99999, 1, command, str(LEGACY_HOST_REPO))])
    store, _, remote = barrier
    snap = store.snapshot()
    assert [p.pid for p in snap.legacy_snapshot] == [99999]
    with pytest.raises(CutoverError, match='snapshot process still alive'):
        store.verify(snap, remote, legacy_processes(), 'a' * 40)
    legacy_inventory([(99998, 1, command, str(LEGACY_HOST_REPO))])
    with pytest.raises(CutoverError, match='legacy writer still alive after frozen snapshot'):
        store.verify(snap, remote, legacy_processes(), 'a' * 40)


@pytest.fixture
def legacy_inventory(monkeypatch):
    from types import SimpleNamespace
    responses = {}
    def populate(rows):
        responses['ps'] = ''.join(f'{pid} {ppid} Mon Nov 2 01:02:03 2026 S {cmd}\n'
                                  for pid, ppid, cmd, _ in rows)
        responses['lsof'] = ''.join(f'p{pid}\nfcwd\nn{cwd}\n' for pid, _, _, cwd in rows)
        return responses
    def command(args, **kwargs):
        assert args[0] in ('ps', 'lsof')
        return SimpleNamespace(stdout=responses[args[0]], stderr='', returncode=0)
    monkeypatch.setattr('pilot1742.cutover.subprocess.run', command)
    return populate


@pytest.mark.parametrize('entrypoint', [
    '/Volumes/AgentStorage/i4-managed-resume42z/runner-fix2.mts',
    '/Volumes/AgentStorage/l2-fix45y/runner.mts',
    '/Volumes/AgentStorage/l2-resume45w/runner.mts',
    '/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1719-recover-runner26.mts',
    '/Volumes/AgentStorage/future-round/launch.pl',
])
def test_dynamic_legacy_writer_uses_real_host_layout(tmp_path, monkeypatch, legacy_inventory, entrypoint):
    from pilot1742 import cutover
    host = tmp_path / 'host'
    host.mkdir()
    child = host / '.sandcastle/worktrees/child'
    child.mkdir(parents=True)
    alias = tmp_path / 'host-alias'
    alias.symlink_to(host, target_is_directory=True)
    monkeypatch.setattr(cutover, 'LEGACY_HOST_REPO', host)
    legacy_inventory([(99991, 1, f'/usr/bin/node {entrypoint}', str(alias)),
                      (99992, 1, '/bin/bash -c sleep', str(child)),
                      (99993, 1, f'/usr/bin/node {entrypoint}', str(tmp_path / 'host-other'))])
    assert {p.pid for p in cutover.legacy_processes()} == {99991, 99992}


def test_legacy_descendants_cannot_escape_by_changing_cwd(legacy_inventory):
    from pilot1742.cutover import LEGACY_HOST_REPO, legacy_processes
    legacy_inventory([(99991, 1, '/usr/bin/perl /unknown/launcher.pl', str(LEGACY_HOST_REPO)),
                      (99992, 99991, '/usr/bin/node /dynamic/runner.mts', '/elsewhere'),
                      (99993, 99992, '/usr/bin/worker', '/outside'),
                      (99994, 1, '/bin/bash', '/unrelated')])
    assert {p.pid for p in legacy_processes()} == {99991, 99992, 99993}


def test_legacy_inspection_excludes_only_own_process_tree(legacy_inventory):
    from pilot1742.cutover import LEGACY_HOST_REPO, legacy_processes
    own = os.getpid()
    legacy_inventory([(99991, 1, '/bin/bash', str(LEGACY_HOST_REPO)),
                      (own, 99991, '/usr/bin/python -m pilot1742.executor', '/'),
                      (99992, own, '/usr/bin/lsof', str(LEGACY_HOST_REPO)),
                      (99993, 99991, '/bin/bash', str(LEGACY_HOST_REPO))])
    assert {p.pid for p in legacy_processes()} == {99991, 99993}


def test_legacy_inspector_inside_host_requires_explicit_control_location(tmp_path, monkeypatch):
    from pilot1742 import cutover
    host = tmp_path / 'host'
    host.mkdir()
    alias = tmp_path / 'alias'
    alias.symlink_to(host, target_is_directory=True)
    monkeypatch.setattr(cutover, 'LEGACY_HOST_REPO', host)
    monkeypatch.chdir(alias)
    with pytest.raises(CutoverError, match='cd / first'):
        cutover.legacy_processes()


@pytest.mark.parametrize('program,output', [
    ('ps', ''), ('ps', 'bad row\n'), ('lsof', ''), ('lsof', 'p99991\n'),
    ('lsof', 'n/unowned\n'), ('lsof', 'p99991\nnrelative\n'),
])
def test_legacy_inventory_invalid_or_missing_refuses(legacy_inventory, program, output):
    from pilot1742.cutover import legacy_processes
    responses = legacy_inventory([(99991, 1, '/usr/bin/unrelated', '/')])
    responses[program] = output
    with pytest.raises(CutoverError, match='inventory'):
        legacy_processes()


@pytest.mark.parametrize('failure', ['returncode', 'stderr', 'timeout', 'missing'])
def test_legacy_inventory_sampling_failure_refuses(monkeypatch, failure):
    from types import SimpleNamespace

    from pilot1742.cutover import legacy_processes
    def command(*args, **kwargs):
        if failure == 'timeout':
            raise subprocess.TimeoutExpired('ps', 30)
        if failure == 'missing':
            raise FileNotFoundError('ps')
        return SimpleNamespace(stdout='ignored', stderr='warning' if failure == 'stderr' else '',
                               returncode=1 if failure == 'returncode' else 0)
    monkeypatch.setattr('pilot1742.cutover.subprocess.run', command)
    with pytest.raises(CutoverError, match='legacy inventory failed'):
        legacy_processes()
