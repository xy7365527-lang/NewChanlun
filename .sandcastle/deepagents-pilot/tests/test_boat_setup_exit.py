import json
import subprocess
import time
from dataclasses import replace

import pytest
from conftest import FAKE, MODELS
from test_boat import launches, supervise, supervisor_env
from test_boat_drain import assert_no_collection, stopped_worker
from test_queue import SPEC, authorize, executor
from test_queue import queued as queued
from v2env import TICKET, V2Env

from pilot1742 import boat_remote
from pilot1742.boat_execution import ExecutionUnknown
from pilot1742.claims import ClaimRefused, ClaimStore
from pilot1742.domain import EffectUnknown
from pilot1742.executor import load_unit, open_v2_run, release_claim


def setup_receipt(tmp_path, monkeypatch, case, binding=None):
    remote = tmp_path / 'supervisor'
    remote.mkdir()
    module, root, request, path = supervisor_env(remote, monkeypatch, npm='exit 7' if case == 'npm' else 'exit 0')
    tools = remote / 'bin'
    if case == 'image':
        (tools / 'docker').write_text('#!/bin/sh\necho wrong-image\n')
    elif case == 'npm-signal':
        (tools / 'npm').write_text('#!/bin/sh\nkill -TERM $$\n')
    elif case == 'missing-runtime':
        (root / 'runtime').rmdir()
    elif case in ('bun', 'missing-lock'):
        skill = root / 'pstack/skill'
        skill.mkdir(parents=True)
        (skill / 'package.json').write_text('{}')
        if case == 'bun':
            (skill / 'bun.lock').write_text('{}')
            (tools / 'bun').write_text('#!/bin/sh\necho bun >> "' + str(remote / 'launches') + '"\nexit 23\n')
            (tools / 'bun').chmod(0o755)
    if binding is not None:
        request.update(identity=binding.identity, package_id=binding.package_id)
        path.write_text(json.dumps(request))
    process = supervise(module, path, time.time() + 60)
    try:
        output, _ = process.communicate(timeout=15)
        assert process.returncode == 0
        assert json.loads(output)['state'] == 'receipt'
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
    receipt = json.loads((root / 'claim/receipt.json').read_text())
    assert not (root.parent / 'private/model-key').exists()
    return remote, root, request, path, receipt


@pytest.mark.parametrize(('case', 'exit_code', 'expected_launches'), [
    ('image', 1, []),
    ('npm', 7, ['npm']),
    ('bun', 23, ['npm', 'bun']),
    ('npm-signal', -15, []),
    ('missing-runtime', None, []),
    ('missing-lock', None, ['npm']),
])
def test_remote_setup_receipt_preserves_known_exit_and_keeps_unknown(tmp_path, monkeypatch, case, exit_code,
                                                                    expected_launches):
    remote, root, request, path, receipt = setup_receipt(tmp_path, monkeypatch, case)
    assert receipt['state'] == 'unknown' and 'record' not in receipt
    assert receipt['exit_code'] == exit_code
    assert launches(remote) == expected_launches
    before = (root / 'claim/receipt.json').read_bytes()
    assert boat_remote.run(request, path, time.time() + 60)['state'] == 'receipt'
    assert boat_remote.cancel(request, path)['state'] == 'receipt'
    assert (root / 'claim/receipt.json').read_bytes() == before
    assert launches(remote) == expected_launches


def test_unresolved_subprocess_timeout_keeps_unknown_exit(tmp_path, monkeypatch):
    module, root, request, path = supervisor_env(tmp_path, monkeypatch)

    def unresolved(*args):
        raise subprocess.TimeoutExpired('unknown child', 1)

    monkeypatch.setattr(module, 'stage', unresolved)
    assert module.run(request, path, time.time() + 60)['state'] == 'receipt'
    receipt = json.loads((root / 'claim/receipt.json').read_text())
    assert receipt['exit_code'] is None and receipt['state'] == 'unknown'
    assert not (root.parent / 'private/model-key').exists()


@pytest.mark.parametrize('case', ['npm', 'missing-lock'])
def test_setup_receipt_requires_reclaim_before_queue_release(queued, tmp_path, monkeypatch, case):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit),
                      MODELS, launcher=FAKE, source_repo=v2.source)
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    binding = replace(binding, stop_id=None)
    engine._store(key, binding)
    _, _, _, _, receipt = setup_receipt(tmp_path, monkeypatch, case, binding)
    engine._path(key, '.remote.json').write_text(json.dumps(receipt))
    release_claim(intake.claims, TICKET, unit.run_id, 'controller released')
    v2.github.issues[TICKET]['labels'].remove('deepagents:queue')
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    seen = []
    before = v2.writes()
    assert not engine.is_drained(key)
    intake.once(lambda item: seen.append(item.ticket) or 0)
    assert seen == []

    def key_erasure_failed(command):
        raise OSError('fixture key deletion failed')

    cli.handlers.update(info={'state': 'running'}, exec=key_erasure_failed)
    with pytest.raises(OSError, match='key deletion'):
        engine._reclaim(key, binding)
    assert not engine.is_drained(key) and engine.load(key).stop_id is None
    assert all(call[0] != 'stop' for call in cli.calls)
    cli.handlers.update(exec={}, stop=[{'operation': {'id': 'stop-confirmed'}}])
    engine._reclaim(key, binding)
    assert [call[0] for call in cli.calls][-3:] == ['info', 'exec', 'stop']
    assert engine.is_drained(key) is (case == 'npm')
    cli.calls.clear()
    with pytest.raises(ExecutionUnknown):
        engine.collect(key, receipt)
    assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
    intake.once(lambda item: seen.append(item.ticket) or 0)
    assert seen == ([TICKET + 1] if case == 'npm' else [])
    assert v2.writes() == before
    assert_no_collection(run, key, cli)


@pytest.mark.parametrize('case', ['npm', 'missing-lock'])
def test_setup_receipt_controls_manual_generation(tmp_path, monkeypatch, case):
    v2 = V2Env(tmp_path)
    run = v2.run(v2.register())
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    _, _, _, _, receipt = setup_receipt(tmp_path, monkeypatch, case, binding)
    engine._path(key, '.remote.json').write_text(json.dumps(receipt))
    release_claim(ClaimStore(v2.claims_root), TICKET, run.spec.run_id, 'controller released')
    second_unit = v2.register(f't{TICKET}-2')
    before = v2.writes()
    if case == 'npm':
        second = v2.run(second_unit)
        assert second.generation == 2
        second.release()
    else:
        with pytest.raises(ClaimRefused, match='Boat'):
            v2.run(second_unit)
        assert v2.claim_record()['generation'] == 1
        assert v2.writes() == before
    assert_no_collection(run, key, cli)
