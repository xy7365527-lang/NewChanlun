import json
import shlex
import subprocess
import time
from dataclasses import replace
from pathlib import Path

import pytest
from test_boat import adapter, launches, supervise, supervisor_env, wait_for
from test_boat_drain import failure_receipt

from pilot1742.boat_execution import BoatExecution
from pilot1742.domain import EffectUnknown


def test_cancel_before_claim_erases_key_and_late_submit_is_idempotent(tmp_path, monkeypatch):
    remote, root, request, path = supervisor_env(tmp_path, monkeypatch)
    assert remote.cancel(request, path)['state'] == 'receipt'
    assert not (root.parent / 'private/model-key').exists()
    before = (root / 'claim/receipt.json').read_bytes()
    assert remote.run(request, path, time.time() + 60)['state'] == 'receipt'
    assert remote.cancel(request, path)['state'] == 'receipt'
    assert (root / 'claim/receipt.json').read_bytes() == before
    assert launches(tmp_path) == []


@pytest.mark.parametrize('entry', ['observe', 'cancel'])
@pytest.mark.parametrize('old_stop', [False, True])
def test_stopped_cached_receipt_without_erasure_never_drains(env, entry, old_stop):
    run, engine, key, binding = adapter(env)
    if old_stop:
        binding = replace(binding, stop_id='stop-old', stop_confirmed=True)
        engine._store(key, binding)
    engine._path(key, '.remote.json').write_text(json.dumps(failure_receipt(binding, 143, 'supervisor')))
    engine.cli.handlers['info'] = {'state': 'stopped', 'stop': {'id': 'stop-old'}}
    fact = getattr(engine, entry)(key)
    assert isinstance(fact, EffectUnknown)
    assert 'key删除未确认' in fact.reason
    assert not engine.is_drained(key)
    assert not engine.load(key).key_erased
    assert all(call[0] == 'info' for call in engine.cli.calls)
    run.release()


class Interrupted(BaseException):
    pass


@pytest.mark.parametrize('crash_at', ['cached', 'stop'])
def test_real_cancel_cached_then_stop_and_restart_controller(env, tmp_path, monkeypatch, crash_at):
    remote, root, request, path = supervisor_env(tmp_path, monkeypatch)
    run, engine, key, binding = adapter(env)
    binding = replace(binding, remote_root=str(root))
    engine._store(key, binding)
    request.update(identity=binding.identity, package_id=binding.package_id)
    path.write_text(json.dumps(request))
    cli = engine.cli

    def execute(command):
        if command.startswith('rm -f '):
            return json.loads(subprocess.check_output(['sh', '-c', command]))
        action = shlex.split(command)[-2]
        assert action in ('observe', 'cancel')
        return getattr(remote, action)(request, path)

    def copy_receipt(source, destination):
        assert source == binding.sandbox_id + ':' + str(root / 'claim/receipt.json')
        Path(destination).write_bytes((root / 'claim/receipt.json').read_bytes())
        return [{}]

    def interrupt_stop(sandbox):
        assert sandbox == binding.sandbox_id
        raise Interrupted

    cli.handlers.update(info={'state': 'running'}, exec=execute, scp=copy_receipt, stop=interrupt_stop)
    if crash_at == 'cached':
        def interrupt_reclaim(*args):
            raise Interrupted
        monkeypatch.setattr(engine, '_reclaim', interrupt_reclaim)
    with pytest.raises(Interrupted):
        engine.cancel(key)
    assert engine._path(key, '.remote.json').exists()
    assert not (root.parent / 'private/model-key').exists()
    assert remote.run(request, path, time.time() + 60)['state'] == 'receipt'
    assert launches(tmp_path) == []
    assert engine.load(key).key_erased is (crash_at == 'stop')
    cli.handlers['info'] = {'state': 'stopped', 'stop': {'id': 'stop-original'}}
    restored = BoatExecution(run, cli)
    assert restored.load(key).sandbox_id == binding.sandbox_id
    assert isinstance(restored.cancel(key), EffectUnknown)
    assert restored.is_drained(key) is (crash_at == 'stop')
    if crash_at == 'cached':
        states = iter([{'state': 'running'}, {'state': 'stopped'}])
        cli.handlers.update(info=lambda: next(states), stop=[{'operation': {'id': 'stop-original'}}])
        restored.cancel(key)
        assert restored.is_drained(key)

    cli.calls.clear()
    restored.cancel(key)
    restored.submit(key)
    assert cli.calls == []
    assert restored.load(key).key_erased and restored.load(key).stop_confirmed
    assert not (run._key_dir(key) / 'result.json').exists()
    run.release()



def test_duplicate_active_supervisor_does_not_erase_key(tmp_path, monkeypatch):
    remote, root, request, path = supervisor_env(tmp_path, monkeypatch, npm='sleep 60')
    process = supervise(remote, path, time.time() + 120)
    try:
        wait_for(lambda: launches(tmp_path) == ['npm'])
        assert remote.run(request, path, time.time() + 120)['state'] == 'running'
        assert (root.parent / 'private/model-key').exists()
        remote.cancel(request, path)
        assert not (root.parent / 'private/model-key').exists()
        process.communicate(timeout=15)
        assert launches(tmp_path) == ['npm']
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()


@pytest.mark.parametrize('terminal', [False, True])
def test_cancel_key_delete_failure_is_not_silent(tmp_path, monkeypatch, terminal):
    remote, root, request, path = supervisor_env(tmp_path, monkeypatch)
    if terminal:
        remote.run(request, path, time.time() + 60)
    key_file = root.parent / 'private/model-key'
    key_file.unlink(missing_ok=True)
    key_file.mkdir()
    with pytest.raises(IsADirectoryError):
        remote.cancel(request, path)
