import base64
import io
import json
import os
import subprocess
import sys
import time
from dataclasses import replace
from datetime import datetime, timedelta, timezone
from pathlib import Path
from tempfile import NamedTemporaryFile
from types import SimpleNamespace

import pytest
from conftest import git

from pilot1742.boat_execution import (
    BoatBinding,
    BoatCLI,
    BoatExecution,
    BoatForkAPI,
    ExecutionUnknown,
    ForkIntent,
    HistoricalLocal,
    new_execution,
    parse_execution,
    save,
)
from pilot1742.cloud_package import build_skill_bundle, export_repository, safe_path
from pilot1742.domain import EffectUnknown, Running, Terminal, WorkerKey
from pilot1742.run import SOURCE_REPO, _status_text_paths


def test_frozen_source_preserves_sha_without_config(env, tmp_path):
    (env.source / 'demo').mkdir()
    (env.source / 'demo/binary').write_bytes(bytes(range(256)))
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'second')
    (env.source / 'README').write_text('third\n')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'third')
    head = git(env.source, 'rev-parse', 'HEAD')
    target = tmp_path / 'cloud'
    export_repository(env.source, head, target)
    assert git(target, 'rev-parse', 'HEAD') == head
    assert git(target, 'status', '--porcelain') == ''
    assert 'fixture@example.invalid' not in (target / '.git/config').read_text()
    assert (target / 'demo/binary').read_bytes() == bytes(range(256))


def test_paths_reject_metadata_escape():
    for path in ['../a', '/a', '.git/config', 'x/.agents/a', '.agents/skills/project/SKILL.md', 'a\\b', './']:
        with pytest.raises(ValueError):
            safe_path(path)


def test_new_default_requires_cloud_no_fallback(env, monkeypatch):
    monkeypatch.delenv('PILOT_BOAT_CONFIG', raising=False)
    with pytest.raises(ValueError, match='Boat'):
        new_execution(source_repo=env.source)


def test_existing_test_run_keeps_binding_without_current_config(env, monkeypatch):
    run = env.create()
    run.release()
    monkeypatch.delenv('PILOT_BOAT_CONFIG', raising=False)
    same = env.create()
    assert same.spec.execution == HistoricalLocal()
    same.release()


@pytest.fixture
def fork_credentials(tmp_path, monkeypatch):
    path = tmp_path / 'controller-key'
    path.write_text('fixture-controller-only\n')
    path.chmod(0o600)
    monkeypatch.delenv('BOAT_API_KEY', raising=False)
    monkeypatch.setenv('PILOT_BOAT_API_KEY_FILE', str(path))
    requests = []

    def request(req, timeout):
        requests.append(req)
        return io.BytesIO(b'{"id":"bx_original"}')

    monkeypatch.setattr('pilot1742.boat_execution.urlopen', request)
    return path, requests, ForkIntent('bx_base', 'same-key', '{}', time.time())


def test_boat_fork_reads_private_file_outside_repo(fork_credentials):
    _, requests, intent = fork_credentials
    assert BoatForkAPI().fork(intent) == 'bx_original'
    assert requests[0].headers['Authorization'] == 'Bearer fixture-controller-only'


@pytest.mark.parametrize('mode', [0o640, 0o620, 0o610, 0o604, 0o602, 0o601])
def test_boat_fork_rejects_shared_key_file(fork_credentials, mode):
    path, requests, intent = fork_credentials
    path.chmod(mode)
    with pytest.raises(ValueError):
        BoatForkAPI().fork(intent)
    assert requests == []


def test_boat_fork_rejects_symlink_and_directory(fork_credentials, tmp_path, monkeypatch):
    path, requests, intent = fork_credentials
    link = tmp_path / 'key-link'
    link.symlink_to(path)
    for invalid in (link, tmp_path):
        monkeypatch.setenv('PILOT_BOAT_API_KEY_FILE', str(invalid))
        with pytest.raises(ValueError):
            BoatForkAPI().fork(intent)
    assert requests == []


def test_boat_fork_rejects_other_owner(fork_credentials, monkeypatch):
    from pilot1742 import boat_execution

    _, requests, intent = fork_credentials
    real_fstat = os.fstat
    with monkeypatch.context() as patched:
        patched.setattr(boat_execution.os, 'fstat', lambda fd: SimpleNamespace(
            st_mode=real_fstat(fd).st_mode, st_uid=os.getuid() + 1))
        with pytest.raises(ValueError):
            BoatForkAPI().fork(intent)
    assert requests == []


def test_boat_fork_rejects_repo_key_after_resolving_parent(fork_credentials, tmp_path, monkeypatch):
    _, requests, intent = fork_credentials
    repo_link = tmp_path / 'repo-link'
    repo_link.symlink_to(SOURCE_REPO, target_is_directory=True)
    with NamedTemporaryFile(dir=SOURCE_REPO) as stream:
        stream.write(b'fixture-controller-only\n')
        stream.flush()
        for path in (Path(stream.name), repo_link / Path(stream.name).name):
            monkeypatch.setenv('PILOT_BOAT_API_KEY_FILE', str(path))
            with pytest.raises(ValueError):
                BoatForkAPI().fork(intent)
    assert requests == []


def test_boat_fork_keeps_environment_token_priority(fork_credentials, monkeypatch):
    path, requests, intent = fork_credentials
    path.unlink()
    monkeypatch.setenv('BOAT_API_KEY', 'fixture-environment')
    assert BoatForkAPI().fork(intent) == 'bx_original'
    assert requests[0].headers['Authorization'] == 'Bearer fixture-environment'


def test_boat_fork_rejects_fifo_without_waiting(tmp_path, monkeypatch):
    pipe = tmp_path / 'key-pipe'
    os.mkfifo(pipe, 0o600)
    monkeypatch.delenv('BOAT_API_KEY', raising=False)
    monkeypatch.setenv('PILOT_BOAT_API_KEY_FILE', str(pipe))
    child = (
        'from pilot1742.boat_execution import BoatForkAPI, ForkIntent\n'
        'try:\n'
        '    BoatForkAPI().fork(ForkIntent("bx_base", "same-key", "{}", 0))\n'
        'except ValueError:\n'
        '    raise SystemExit(0)\n'
        'raise SystemExit(1)\n'
    )
    result = subprocess.run([sys.executable, '-c', child],
                            cwd=Path(__file__).resolve().parents[1], capture_output=True, text=True, timeout=5)
    assert result.returncode == 0, result.stderr


def config():
    return dict(endpoint='https://example.invalid/v1', image='sha256:' + '1' * 64, base_sandbox='bx_base',
                key_source_sandbox='bx_abc', key_source_path='/private/client-key',
                pstack_source='/pstack', model_rules='/rules/models.md')


class FakeBoat:
    """Scripted Boat CLI. Every call is recorded; an unscripted call fails the test."""
    executable = 'boat'

    def __init__(self, **handlers):
        self.calls = []
        self.handlers = handlers

    def _handle(self, name, *args):
        self.calls.append((name, *args))
        if name not in self.handlers:
            raise AssertionError(f'unexpected Boat call {name} {args}')
        handler = self.handlers[name]
        return handler(*args) if callable(handler) else handler

    def call(self, *args):
        return self._handle('detach' if args[0] == 'exec' and '--detach' in args else args[0], *args[1:])

    def exec(self, sandbox, command):
        return self._handle('exec', command)

    def info(self, sandbox):
        return self._handle('info')


def adapter(env, cli=None, submission='submitted'):
    run = env.create()
    object.__setattr__(run.spec, 'execution', parse_execution({'backend': 'boat', 'config': config(), 'runtime_head': env.base}))
    engine = BoatExecution(run, cli or FakeBoat())
    run.execution = engine
    key = WorkerKey(run.spec.run_id, 'author', env.base)
    binding = BoatBinding(engine._identity(key), 'vm-author', 'p1', '/home/user/newchanlun-r1/p1', submission,
                          sandbox_id='bx_abc', deadline=time.time() + 1500)
    engine._store(key, binding)
    return run, engine, key, binding


def payload(run, engine, key, files, status=None):
    status = '\n'.join('?? ' + item['path'] for item in files) if status is None else status
    return dict(identity=engine._identity(key), package_id='p1', exit_code=0, files=files,
                record=dict(ticket=run.spec.ticket, status='completed', expected_head=str(key.target_head),
                            model=str(run.spec.author_model), actual_model=str(run.spec.author_model), mode='execute',
                            run_id='child1', checkout_after={'head': str(key.target_head), 'status': status},
                            result={'summary': 'done', 'findings': [], 'validation': []}))


def test_collect_twice_preserves_binary_and_mode(env):
    run, engine, key, _ = adapter(env)
    files = [dict(path='demo/new', mode='100755', content=base64.b64encode(b'\x00binary').decode())]
    data = payload(run, engine, key, files)
    engine.collect(key, data)
    engine.collect(key, data)
    path = Path(run.spec.author_checkout) / 'demo/new'
    assert path.read_bytes() == b'\x00binary'
    assert path.stat().st_mode & 0o111
    run.release()


def test_collected_worker_stays_terminal_without_touching_stopped_vm(env):
    run, engine, key, _ = adapter(env)
    engine.collect(key, payload(run, engine, key, [dict(path='demo/a', mode='100644', content='YQ==')]))
    assert isinstance(engine.observe(key), Terminal)
    assert isinstance(run.facts().workers[key], Terminal)
    assert engine.cli.calls == []
    run.release()


def test_status_is_local_only_and_writes_nothing(env, monkeypatch):
    run, engine, key, _ = adapter(env)
    (run.dir / 'intents').mkdir(exist_ok=True)
    (run.dir / 'intents' / f'{key.name}.json').write_text('{}')

    def fenced(effect):
        raise AssertionError('status entered a side-effect fence')
    monkeypatch.setattr(run, '_fence', fenced)
    before = {p: p.read_bytes() for p in run.dir.rglob('*') if p.is_file() and p.name != 'owner.lock'}
    report = run.status()
    after = {p: p.read_bytes() for p in run.dir.rglob('*') if p.is_file() and p.name != 'owner.lock'}
    assert report['workers'][key.name] == {'fact': 'running'}
    assert report['execution']['bindings'][key.name]['submission'] == 'submitted'
    assert engine.cli.calls == [] and before == after
    run.release()


def test_quoted_status_paths_equal_raw_paths(tmp_path):
    repo = tmp_path / 'repo'
    repo.mkdir()
    git(repo, 'init', '-q')
    (repo / 'demo').mkdir()
    for name in ('旧 名.md', '定义.md', 'keep.md'):
        (repo / 'demo' / name).write_text(name)
    git(repo, 'add', '.')
    git(repo, '-c', 'user.name=F', '-c', 'user.email=f@example.invalid', 'commit', '-qm', 'base')
    git(repo, 'mv', 'demo/旧 名.md', 'demo/新 名.md')
    (repo / 'demo/定义.md').write_text('changed')
    (repo / 'demo/引号"与\\反斜杠.md').write_text('x')
    (repo / 'demo/tab\tname').write_text('x')
    text = subprocess.run(['git', '-C', str(repo), '-c', 'core.quotepath=true', 'status', '--porcelain=v1',
                           '--untracked-files=all'], capture_output=True, text=True, check=True).stdout
    assert '\\346' in text
    raw = subprocess.run(['git', '-C', str(repo), 'status', '--porcelain=v1', '-z', '--untracked-files=all'],
                         capture_output=True, text=True, check=True).stdout.split('\0')
    expected, i = set(), 0
    while i < len(raw):
        if raw[i]:
            expected.add(raw[i][3:])
            if raw[i][0] in 'RC':
                i += 1
                expected.add(raw[i])
        i += 1
    assert set(_status_text_paths(text)) == expected
    assert 'demo/新 名.md' in expected and 'demo/旧 名.md' in expected


def test_collect_resume_rejects_other_writer_same_path(env):
    run, engine, key, binding = adapter(env)
    files = [dict(path='demo/new', mode='100644', content=base64.b64encode(b'ours').decode())]
    data = payload(run, engine, key, files)
    save(engine._path(key, '.collect.json'), {'identity': binding.identity, 'files': files})
    path = Path(run.spec.author_checkout) / 'demo/new'
    path.parent.mkdir()
    path.write_text('other writer')
    with pytest.raises(ExecutionUnknown, match='并发'):
        engine.collect(key, data)
    assert path.read_text() == 'other writer'
    run.release()


def test_collect_crash_between_files_recovers_without_temp_pollution(env, monkeypatch):
    from pilot1742 import boat_execution
    run, engine, key, _ = adapter(env)
    files = [dict(path='demo/' + name, mode='100644', content=base64.b64encode(name.encode()).decode()) for name in ('a', 'b')]
    data = payload(run, engine, key, files)
    replace_file = os.replace

    def crash(source, destination):
        if str(destination).endswith('/demo/b'):
            raise OSError('controlled collection interruption')
        return replace_file(source, destination)
    monkeypatch.setattr(boat_execution.os, 'replace', crash)
    with pytest.raises(OSError, match='interruption'):
        engine.collect(key, data)
    monkeypatch.setattr(boat_execution.os, 'replace', replace_file)
    engine.collect(key, data)
    checkout = Path(run.spec.author_checkout)
    assert (checkout / 'demo/a').read_bytes() == b'a'
    assert (checkout / 'demo/b').read_bytes() == b'b'
    assert not list(checkout.rglob('*.tmp'))
    run.release()


def test_unknown_exit_and_stale_generation_do_not_collect_or_cancel(env):
    run, engine, key, binding = adapter(env)
    engine.submit(key)
    assert engine.cli.calls == []
    data = payload(run, engine, key, [])
    data['exit_code'] = None
    with pytest.raises(ExecutionUnknown):
        engine.collect(key, data)
    engine._store(key, replace(binding, identity={**binding.identity, 'generation': 2}))
    with pytest.raises(ExecutionUnknown, match='代次'):
        engine.cancel(key)
    run.release()


def stamp(seconds):
    return (datetime.now(timezone.utc) + timedelta(seconds=seconds)).isoformat().replace('+00:00', 'Z')


def prepared_engine(env, monkeypatch, cli, submission='creating', sandbox_id=None):
    run, engine, key, binding = adapter(env, cli, submission)
    engine._store(key, replace(binding, sandbox_id=sandbox_id, deadline=None))
    package = run.dir / 'pkg'
    package.mkdir()
    (package / 'request.json').write_text('{}')
    monkeypatch.setattr(engine, '_build', lambda key, binding: package)
    provisioned = []
    monkeypatch.setattr(engine, '_provision_key', lambda binding: provisioned.append(binding.sandbox_id))
    return run, engine, key, provisioned


def test_upload_failure_stops_before_key_or_worker(env, monkeypatch):
    commands = []

    def remote(command):
        commands.append(command)
        if 'tar -xzf' in command:
            raise ExecutionUnknown('远端命令输出不完整或执行非成功')
        return {}
    cli = FakeBoat(limits=[{'canStart': True, 'creditUsedSeconds': 0}],
                   info={'state': 'running', 'hydrated': True}, rename=[{}], exec=remote, scp=[])
    run, engine, key, provisioned = prepared_engine(env, monkeypatch, cli)
    from types import SimpleNamespace
    engine.fork_api = SimpleNamespace(fork=lambda intent: 'bx_new')
    with pytest.raises(ExecutionUnknown):
        engine.submit(key)
    binding = engine.load(key)
    assert binding.submission == 'creating' and binding.sandbox_id == 'bx_new'
    assert binding.fork_intent.source_sandbox == 'bx_base'
    assert json.loads(binding.fork_intent.body)['noEnv'] is True
    assert ('rename', 'bx_new', binding.sandbox_name) in cli.calls
    assert all(command.startswith('set -e; test ! -e') for command in commands)
    assert provisioned == [] and not any(call[0] == 'exec' and '--detach' in call for call in cli.calls)
    run.release()


def test_lost_fork_response_adopts_by_name_or_refuses_to_fork_again(env, monkeypatch):
    cli = FakeBoat(list=[{'sandboxes': []}])
    run, engine, key, _ = prepared_engine(env, monkeypatch, cli)
    engine._store(key, replace(engine.load(key), create_requested=True))
    with pytest.raises(ExecutionUnknown, match='不能安全重建'):
        engine.submit(key)
    assert [call[0] for call in cli.calls] == ['list']
    binding = engine.load(key)
    cli.handlers['list'] = [{'sandboxes': [{'name': binding.sandbox_name, 'id': 'bx_lost'}]}]
    assert engine._create_sandbox(key, binding) == 'bx_lost'
    run.release()


def test_admission_refuses_short_vm_life_without_extending(env, monkeypatch):
    cli = FakeBoat(info={'state': 'running', 'archiveAfter': stamp(600)})
    run, engine, key, provisioned = prepared_engine(env, monkeypatch, cli, 'prepared', 'bx_abc')
    with pytest.raises(ExecutionUnknown, match='剩余寿命'):
        engine.submit(key)
    assert engine.load(key).submission == 'prepared' and provisioned == []
    assert [call[0] for call in cli.calls] == ['info']
    run.release()


def test_submit_records_process_id_and_deadline(env, monkeypatch):
    launched = []
    cli = FakeBoat(info={'state': 'running', 'archiveAfter': stamp(1700)},
                   detach=lambda *a: launched.append(a) or [{'commandId': 'cmd_x', 'processId': 7, 'pid': 7}])
    run, engine, key, provisioned = prepared_engine(env, monkeypatch, cli, 'prepared', 'bx_abc')
    engine.submit(key)
    binding = engine.load(key)
    assert binding.submission == 'submitted' and binding.command_id == 'cmd_x' and binding.process_id == 7
    assert 1700 - 130 < binding.deadline - time.time() < 1700 - 110
    assert provisioned == ['bx_abc'] and launched[0][1] == '--detach'
    assert launched[0][-1].endswith(' ' + str(int(binding.deadline)))
    run.release()


def history_engine(env, submission, history):
    launches = []
    cli = FakeBoat(info={'state': 'running'}, history=[{'commands': history}],
                   detach=lambda *a: launches.append(a) or [{'commandId': 'cmd_new', 'processId': 9}])
    run, engine, key, binding = adapter(env, cli, submission)
    cli.handlers['exec'] = lambda command: {'identity': binding.identity, 'package_id': 'p1', 'state': 'unclaimed'}
    return run, engine, key, binding, launches


def test_lost_submit_response_without_remote_command_relaunches_once(env):
    run, engine, key, binding, launches = history_engine(env, 'submitting', [])
    assert isinstance(engine.observe(key), Running)
    assert len(launches) == 1 and engine.load(key).submission == 'submitted'
    run.release()


def test_unclaimed_but_running_command_is_adopted_not_relaunched(env):
    run, engine, key, binding, launches = history_engine(env, 'submitting', [])
    engine.cli.handlers['history'] = [{'commands': [{'status': 'running', 'commandId': 'cmd_orig', 'processId': 5,
                                                     'command': "bash -lc '" + engine._command(binding, 'submit') + " 1'"}]}]
    assert isinstance(engine.observe(key), Running)
    assert launches == []
    adopted = engine.load(key)
    assert (adopted.submission, adopted.command_id, adopted.process_id) == ('submitted', 'cmd_orig', 5)
    run.release()


def test_supervisor_that_never_claimed_is_unknown_not_redispatched(env):
    run, engine, key, binding, launches = history_engine(env, 'submitted', [])
    engine.cli.handlers['history'] = [{'commands': [{'status': 'exited', 'command': engine._command(binding, 'submit') + ' 1'}]}]
    fact = engine.observe(key)
    assert isinstance(fact, EffectUnknown) and '未启动' in fact.reason and launches == []
    run.release()


def test_stopped_vm_is_unknown_without_resume(env):
    run, engine, key, _ = adapter(env, FakeBoat(info={'state': 'stopped'}))
    fact = engine.observe(key)
    assert isinstance(fact, EffectUnknown) and '显式恢复' in fact.reason
    assert [call[0] for call in engine.cli.calls] == ['info']
    run.release()


def test_terminal_receipt_is_cached_vm_reclaimed_then_collected(env, monkeypatch):
    run, engine, key, binding = adapter(env)
    files = [dict(path='demo/新.md', mode='100644', content=base64.b64encode('内容'.encode()).decode())]
    receipt = payload(run, engine, key, files, status='?? "demo/\\346\\226\\260.md"')
    stops = []

    def scp(source, target):
        Path(target).write_text(json.dumps(receipt))
        return []
    engine.cli = FakeBoat(info={'state': 'running'}, scp=scp, stop=lambda *a: stops.append(a) or [{'operation': {'id': 'stop_1'}}],
                          exec=lambda command: {} if command.startswith('rm -f') else
                          {'identity': binding.identity, 'package_id': 'p1', 'state': 'receipt'})
    assert isinstance(engine.observe(key), Terminal)
    assert (Path(run.spec.author_checkout) / 'demo/新.md').read_text() == '内容'
    assert engine.load(key).stop_id == 'stop_1' and len(stops) == 1
    assert any(call[0] == 'exec' and call[1].startswith('rm -f') for call in engine.cli.calls)
    engine.cli = FakeBoat()
    assert isinstance(engine.observe(key), Terminal)
    run.release()


def test_skill_bundle_is_frozen_once_per_run_with_content_identity(env, tmp_path):
    pstack = tmp_path / 'pstack'
    (pstack / 'installed/skills/a').mkdir(parents=True)
    (pstack / 'installed/skills/a/SKILL.md').write_text('v1')
    (pstack / 'upstream').mkdir()
    (pstack / 'upstream/LICENSE').write_text('l')
    (pstack / 'aliases.json').write_text('{}')
    rules = tmp_path / 'models.md'
    rules.write_text('r')
    first = build_skill_bundle(pstack, rules, tmp_path / 'b1')
    (pstack / 'installed/skills/a/SKILL.md').write_text('v2')
    second = build_skill_bundle(pstack, rules, tmp_path / 'b2')
    assert first['id'] != second['id']
    run, engine, key, _ = adapter(env)
    object.__setattr__(engine, 'config', replace(engine.config, pstack_source=str(pstack), model_rules=str(rules)))
    author = engine._intent(key)
    (pstack / 'installed/skills/a/SKILL.md').write_text('v3')
    review = engine._intent(WorkerKey(run.spec.run_id, 'review', env.base))
    assert author.package_id.rsplit('-', 1)[1] == review.package_id.rsplit('-', 1)[1] == second['id'][:12]
    assert author.sandbox_name != review.sandbox_name
    run.release()


def test_boat_wire_rejects_truncation_and_uses_explicit_shell(monkeypatch):
    from pilot1742 import boat_execution
    invoked = []

    def command(args, **kw):
        invoked.append(args)
        return SimpleNamespace(returncode=0, stdout=json.dumps({'exitCode': 0, 'stdout': '{}', 'stdoutTruncated': True}))
    monkeypatch.setattr(boat_execution.subprocess, 'run', command)
    with pytest.raises(ExecutionUnknown, match='不完整'):
        BoatCLI().exec('bx_abc', 'echo test')
    assert invoked[0][:3] == ['boat', '--json', 'exec']
    assert invoked[0][-4:] == ['--', 'bash', '-lc', 'echo test']


def test_skill_symlink_closure_and_cycles(tmp_path):
    from pilot1742.cloud_package import copy_skill_tree
    source = tmp_path / 'symlink-skills'
    (source / 'canonical').mkdir(parents=True)
    (source / 'canonical/SKILL.md').write_text('skill')
    (source / 'alias').symlink_to(source / 'canonical', target_is_directory=True)
    rows = copy_skill_tree(source, tmp_path / 'package', {})
    assert {row['path'] for row in rows} == {'canonical/SKILL.md', 'alias/SKILL.md'}
    (source / 'canonical/loop').symlink_to(source, target_is_directory=True)
    with pytest.raises(ValueError, match='循环'):
        copy_skill_tree(source, tmp_path / 'cycle-package', {})


def test_v2_cancel_runs_under_ticket_action_lock(tmp_path, monkeypatch):
    from v2env import V2Env

    from pilot1742 import boat_execution
    from pilot1742.cli import cancel_worker
    from pilot1742.run import Halted, PilotRun

    v2 = V2Env(tmp_path)
    unit = v2.register()
    run = v2.run(unit)
    run.release()
    spec = json.loads((run.dir / 'run.json').read_text())
    spec['execution'] = {'backend': 'boat', 'config': config(), 'runtime_head': v2.base}
    (run.dir / 'run.json').write_text(json.dumps(spec))
    actions = []
    cli = FakeBoat(info={'state': 'running'})
    monkeypatch.setattr(boat_execution, 'BoatCLI', lambda: cli)
    executor = v2.executor(unit)
    original = executor.action

    def action(effect, *args, **kw):
        actions.append(effect)
        return original(effect, *args, **kw)
    monkeypatch.setattr(executor, 'action', action)
    bound = PilotRun.open(v2.home, unit.run_id, executor=executor, launcher=None, source_repo=v2.source)
    key = WorkerKey(unit.run_id, 'author', v2.base)
    binding = BoatBinding(bound.execution._identity(key), 'vm', 'p1', '/home/user/x/p1', 'submitted', sandbox_id='bx_abc')
    bound.execution._store(key, binding)
    (bound.dir / 'intents').mkdir(exist_ok=True)
    (bound.dir / 'intents' / f'{key.name}.json').write_text('{}')
    cancelled = []

    def remote(command):
        if ' cancel ' in command:
            cancelled.append(command)
            return {}
        return {'identity': binding.identity, 'package_id': 'p1', 'state': 'running'}
    cli.handlers['exec'] = remote
    assert cancel_worker(bound, 'author', v2.base) == 0
    assert len(cancelled) == 1 and actions == ['dispatch']
    unbound = PilotRun.open(v2.home, unit.run_id, source_repo=v2.source)
    calls = len(cli.calls)
    assert unbound.status()['workers'][key.name] == {'fact': 'running'}
    assert len(cli.calls) == calls
    unbound.acquire()
    with pytest.raises(Halted):
        unbound.execution.cancel(key)
    unbound.release()
    assert len(cancelled) == 1


def supervisor_env(tmp_path, monkeypatch, npm='exit 0', node='exit 0'):
    from pilot1742 import boat_remote
    root = tmp_path / 'run/package'
    (root / 'runtime').mkdir(parents=True)
    (root.parent / 'private').mkdir()
    (root.parent / 'private/model-key').write_text('fixture-not-a-key')
    tools = tmp_path / 'bin'
    tools.mkdir()
    for name, body in (('npm', npm), ('node', node)):
        (tools / name).write_text('#!/bin/sh\necho ' + name + ' >> "' + str(tmp_path / 'launches') + '"\n' + body + '\n')
        (tools / name).chmod(0o755)
    (tools / 'docker').write_text('#!/bin/sh\n[ "$1 $2" = "image inspect" ] && echo "$3"\n')
    (tools / 'docker').chmod(0o755)
    monkeypatch.setenv('PATH', str(tools) + ':' + os.environ['PATH'])
    request = dict(root=str(root), identity={'run_id': 'r1', 'role': 'author', 'target_head': 'a' * 40, 'generation': 1},
                   package_id='p1', image='sha256:' + 'f' * 64)
    path = root / 'request.json'
    path.write_text(json.dumps(request))
    return boat_remote, root, request, path


def launches(tmp_path):
    path = tmp_path / 'launches'
    return path.read_text().split() if path.exists() else []


def supervise(boat_remote, path, deadline):
    return subprocess.Popen([sys.executable, boat_remote.__file__, 'submit', str(path), str(deadline)], stdout=subprocess.PIPE)


def wait_for(predicate, seconds=15):
    deadline = time.monotonic() + seconds
    while not predicate():
        assert time.monotonic() < deadline
        time.sleep(.02)


def test_remote_cancel_during_setup_and_duplicate_claim(tmp_path, monkeypatch):
    boat_remote, root, request, path = supervisor_env(tmp_path, monkeypatch, npm='sleep 60')
    process = supervise(boat_remote, path, time.time() + 300)
    try:
        wait_for(lambda: (root / 'claim/child.json').exists())
        wait_for(lambda: launches(tmp_path) == ['npm'])
        assert boat_remote.observe(request, path)['state'] == 'running'
        boat_remote.cancel(request, path)
        process.communicate(timeout=15)
        receipt = json.loads((root / 'claim/receipt.json').read_text())
        assert receipt['exit_code'] == 143 and receipt['state'] == 'unknown' and '取消' in receipt['reason']
        assert boat_remote.run(request, path, time.time() + 300)['state'] == 'receipt'
        assert launches(tmp_path) == ['npm']
        assert not (root.parent / 'private/model-key').exists()
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()


def test_remote_cancel_between_setup_and_runner_never_lets_runner_finish(tmp_path, monkeypatch):
    gate = tmp_path / 'go'
    boat_remote, root, request, path = supervisor_env(
        tmp_path, monkeypatch, npm=f'while [ ! -e "{gate}" ]; do sleep 0.05; done',
        node=f'sleep 20; echo finished >> "{tmp_path / "launches"}"')
    process = supervise(boat_remote, path, time.time() + 300)
    try:
        wait_for(lambda: (root / 'claim/child.json').exists())
        gate.touch()
        wait_for(lambda: not boat_remote.same_process(json.loads((root / 'claim/child.json').read_text()))
                 or 'node' in launches(tmp_path))
        boat_remote.cancel(request, path)
        process.communicate(timeout=30)
        receipt = json.loads((root / 'claim/receipt.json').read_text())
        assert receipt['exit_code'] == 143 and '取消' in receipt['reason']
        assert 'finished' not in launches(tmp_path)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()


def test_remote_cancel_before_claim_blocks_late_supervisor(tmp_path, monkeypatch):
    boat_remote, root, request, path = supervisor_env(tmp_path, monkeypatch)
    assert boat_remote.observe(request, path)['state'] == 'unclaimed'
    assert boat_remote.cancel(request, path)['state'] == 'receipt'
    assert boat_remote.run(request, path, time.time() + 300)['state'] == 'receipt'
    assert launches(tmp_path) == []
    assert '模型任务未启动' in json.loads((root / 'claim/receipt.json').read_text())['reason']


def test_remote_deadline_writes_receipt_before_vm_stop(tmp_path, monkeypatch):
    boat_remote, root, request, path = supervisor_env(tmp_path, monkeypatch, node='sleep 60')
    started = time.monotonic()
    process = supervise(boat_remote, path, time.time() + 3)
    process.communicate(timeout=30)
    receipt = json.loads((root / 'claim/receipt.json').read_text())
    assert receipt['timed_out'] is True and receipt['exit_code'] == 143
    assert time.monotonic() - started < 20
    assert launches(tmp_path) == ['npm', 'node']
    assert not (root.parent / 'private/model-key').exists()
