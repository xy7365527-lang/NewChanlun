import base64
import json
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import Mock

import pytest

from conftest import git
from pilot1742.boat_execution import BoatCLI, BoatConfig, BoatExecution, ExecutionUnknown, new_execution, save
from pilot1742.cloud_package import export_repository, safe_path
from pilot1742.domain import WorkerKey


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
    for path in ['../a', '/a', '.git/config', 'x/.agents/a', 'a\\b', './']:
        with pytest.raises(ValueError):
            safe_path(path)


def test_new_default_requires_cloud_no_fallback(env, monkeypatch):
    monkeypatch.delenv('PILOT_BOAT_CONFIG', raising=False)
    with pytest.raises(ValueError, match='Boat'):
        new_execution(source_repo=env.source)
    assert new_execution(test_launcher=True)['backend'] == 'test-launcher'


def test_existing_test_run_keeps_binding_without_current_config(env, monkeypatch):
    run = env.create()
    run.release()
    monkeypatch.delenv('PILOT_BOAT_CONFIG', raising=False)
    same = env.create()
    assert same.spec.execution == {'backend': 'test-launcher'}
    same.release()


def config():
    return dict(endpoint='https://example.invalid/v1', image='sha256:' + '1' * 64,
                key_source_sandbox='bx_abc', key_source_path='/private/client-key',
                pstack_source='/pstack', model_rules='/rules/models.md')


def adapter(env):
    run = env.create()
    execution = {'backend': 'boat', 'config': config()}
    object.__setattr__(run.spec, 'execution', execution)
    engine = BoatExecution(run, Mock())
    key = WorkerKey(run.spec.run_id, 'author', env.base)
    binding = dict(identity=engine._identity(key), package_id='p1', submission='submitted', sandbox_id='bx_abc')
    save(engine._path(key), binding)
    return run, engine, key, binding


def payload(run, engine, key, binding, files):
    status = '\n'.join('?? ' + item['path'] for item in files)
    return dict(identity=engine._identity(key), package_id='p1', exit_code=0, files=files,
                record=dict(ticket=run.spec.ticket, status='completed', expected_head=str(key.target_head),
                            model=str(run.spec.author_model), actual_model=str(run.spec.author_model), mode='execute', run_id='child1',
                            checkout_after={'head': str(key.target_head), 'status': status},
                            result={'summary': 'done', 'findings': [], 'validation': []}))


def test_collect_twice_preserves_binary_and_mode(env):
    run, engine, key, binding = adapter(env)
    files = [dict(path='demo/new', mode='100755', content=base64.b64encode(b'\x00binary').decode())]
    data = payload(run, engine, key, binding, files)
    engine.collect(key, data)
    engine.collect(key, data)
    path = Path(run.spec.author_checkout) / 'demo/new'
    assert path.read_bytes() == b'\x00binary'
    assert path.stat().st_mode & 0o111
    run.release()


def test_collect_resume_rejects_other_writer_same_path(env):
    run, engine, key, binding = adapter(env)
    files = [dict(path='demo/new', mode='100644', content=base64.b64encode(b'ours').decode())]
    data = payload(run, engine, key, binding, files)
    save(run.dir / 'boat' / (key.name + '.collect.json'), {'identity': binding['identity'], 'files': files})
    path = Path(run.spec.author_checkout) / 'demo/new'
    path.parent.mkdir()
    path.write_text('other writer')
    with pytest.raises(ExecutionUnknown, match='并发'):
        engine.collect(key, data)
    assert path.read_text() == 'other writer'
    run.release()


def test_unknown_exit_generation_and_response_loss_do_not_dispatch(env):
    run, engine, key, binding = adapter(env)
    engine.submit(key)
    engine.cli.call.assert_not_called()
    data = payload(run, engine, key, binding, [])
    data['exit_code'] = None
    with pytest.raises(ExecutionUnknown):
        engine.collect(key, data)
    save(engine._path(key), {**binding, 'identity': {**binding['identity'], 'generation': 2}})
    with pytest.raises(ExecutionUnknown, match='代次'):
        engine.cancel(key)
    run.release()


def test_remote_cancel_setup_and_duplicate_claim(tmp_path, monkeypatch):
    import os
    import subprocess
    import sys
    import time
    from pilot1742 import boat_remote

    root = tmp_path / 'worker'
    (root / 'runtime').mkdir(parents=True)
    tools = tmp_path / 'bin'
    tools.mkdir()
    npm = tools / 'npm'
    npm.write_text('#!/bin/sh\necho launch >> "' + str(tmp_path / 'launches') + '"\nsleep 60\n')
    npm.chmod(0o755)
    monkeypatch.setenv('PATH', str(tools) + ':' + os.environ['PATH'])
    request = dict(root=str(root), identity={'run_id': 'r1', 'role': 'author', 'target_head': 'a' * 40, 'generation': 1},
                   package_id='p1', timeout=10)
    path = root / 'request.json'
    path.write_text(json.dumps(request))
    process = subprocess.Popen([sys.executable, boat_remote.__file__, 'submit', str(path)], stdout=subprocess.PIPE)
    try:
        deadline = time.monotonic() + 10
        while not (root / 'claim/child.json').exists():
            assert time.monotonic() < deadline
            time.sleep(.02)
        assert boat_remote.observe(request, path)['state'] == 'running'
        boat_remote.cancel(request, path)
        process.communicate(timeout=10)
        receipt = json.loads((root / 'claim/receipt.json').read_text())
        assert receipt['exit_code'] == 143
        assert receipt['state'] == 'unknown'
        assert boat_remote.run(request, path) == receipt
        assert (tmp_path / 'launches').read_text().splitlines() == ['launch']
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()


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



def test_collect_crash_between_files_recovers_without_temp_pollution(env, monkeypatch):
    import os
    from pilot1742 import boat_execution
    run, engine, key, binding = adapter(env)
    files = [dict(path='demo/' + name, mode='100644', content=base64.b64encode(name.encode()).decode()) for name in ('a', 'b')]
    data = payload(run, engine, key, binding, files)
    replace = os.replace
    def crash(source, destination):
        if str(destination).endswith('/demo/b'):
            raise OSError('controlled collection interruption')
        return replace(source, destination)
    monkeypatch.setattr(boat_execution.os, 'replace', crash)
    with pytest.raises(OSError, match='interruption'):
        engine.collect(key, data)
    monkeypatch.setattr(boat_execution.os, 'replace', replace)
    engine.collect(key, data)
    checkout = Path(run.spec.author_checkout)
    assert (checkout / 'demo/a').read_bytes() == b'a'
    assert (checkout / 'demo/b').read_bytes() == b'b'
    assert not list(checkout.rglob('*.tmp'))
    run.release()



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
