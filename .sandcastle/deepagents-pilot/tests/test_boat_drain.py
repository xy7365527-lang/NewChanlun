import json
from dataclasses import replace

import pytest
from conftest import FAKE, MODELS, git
from test_boat import FakeBoat, config, payload
from test_queue import SPEC, authorize, executor
from test_queue import queued as queued
from v2env import TICKET, V2Env

from pilot1742 import boat_execution, boat_remote
from pilot1742.boat_execution import (
    BoatBinding,
    BoatExecution,
    ExecutionUnknown,
    parse_execution,
)
from pilot1742.claims import ClaimRefused, ClaimStore
from pilot1742.domain import EffectUnknown, WorkerKey
from pilot1742.executor import load_unit, open_v2_run, release_claim


def stopped_worker(run, monkeypatch):
    run.release()
    run.spec = replace(run.spec, execution=parse_execution({
        'backend': 'boat', 'config': config(), 'runtime_head': run.spec.base}))
    (run.dir / 'run.json').write_text(json.dumps(run.spec.to_json()))
    cli = FakeBoat()
    monkeypatch.setattr(boat_execution, 'BoatCLI', lambda: cli)
    run.execution = BoatExecution(run, cli)
    key = WorkerKey(run.spec.run_id, 'author', run.spec.base)
    binding = BoatBinding(run.execution._identity(key), 'vm', 'p1', '/home/user/x/p1',
                          'submitted', sandbox_id='bx_abc', stop_id='stop-original')
    run.execution._store(key, binding)
    return run.execution, key, binding, cli


def failure_receipt(binding, exit_code, kind='worker'):
    receipt = {'identity': dict(binding.identity), 'package_id': binding.package_id,
               'state': 'terminal', 'exit_code': exit_code,
               'record': {'status': 'failed', 'process_exit': {'code': exit_code, 'signal': None}},
               'files': [{'path': 'demo/forbidden', 'mode': '100644', 'content': 'bm8='}]}
    if kind == 'supervisor':
        receipt.pop('record')
        receipt.update(state='unknown', reason='管理者取消；未产生成功业务回执')
    return receipt


CASES = [
    'worker', 'cancelled', 'supervisor', 'missing_stop', 'missing_receipt', 'identity', 'package',
    'missing_exit', 'exit_mismatch', 'bool_exit', 'process_exit_null', 'process_exit_missing',
    'process_exit_mismatch', 'process_exit_bool', 'completed', 'running', 'unknown', 'bad_binding',
]
DRAINED = {'worker', 'cancelled', 'supervisor'}


def stage_evidence(engine, key, binding, case, exit_code):
    receipt = failure_receipt(binding, exit_code, 'supervisor' if case == 'supervisor' else 'worker')
    if case == 'missing_stop':
        engine._store(key, replace(binding, stop_id=None))
    elif case == 'identity':
        receipt['identity']['generation'] += 1
    elif case == 'package':
        receipt['package_id'] = 'other'
    elif case == 'missing_exit':
        receipt.pop('exit_code')
    elif case == 'exit_mismatch':
        receipt['exit_code'] = 2 if exit_code == 0 else 0
    elif case == 'bool_exit':
        receipt['exit_code'] = bool(exit_code)
    elif case == 'process_exit_null':
        receipt['record']['process_exit'] = None
    elif case == 'process_exit_missing':
        receipt['record'].pop('process_exit')
    elif case == 'process_exit_mismatch':
        receipt['record']['process_exit']['code'] = 3
    elif case == 'process_exit_bool':
        receipt['record']['process_exit']['code'] = bool(exit_code)
    elif case in ('cancelled', 'completed'):
        receipt['record']['status'] = case
    elif case in ('running', 'unknown'):
        receipt['state'] = case
    elif case == 'bad_binding':
        engine._store(key, replace(binding, identity={**binding.identity, 'generation': 999}))
    if case != 'missing_receipt':
        engine._path(key, '.remote.json').write_text(json.dumps(receipt))
    return receipt


def assert_no_collection(run, key, cli):
    assert not (run._key_dir(key) / 'result.json').exists()
    assert not run.execution._path(key, '.collect.json').exists()
    assert git(run.spec.checkout_for(key.role), 'status', '--porcelain') == ''
    assert cli.calls == []


@pytest.mark.parametrize('case', CASES)
@pytest.mark.parametrize('exit_code', [0, 2])
def test_manual_generation_uses_only_stopped_failure_evidence(tmp_path, monkeypatch, case, exit_code):
    v2 = V2Env(tmp_path)
    unit = v2.register()
    run = v2.run(unit)
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    receipt = stage_evidence(engine, key, binding, case, exit_code)
    release_claim(ClaimStore(v2.claims_root), TICKET, unit.run_id, 'controller released')
    second_unit = v2.register(f't{TICKET}-2')
    before = v2.writes()
    if case in DRAINED and not (case == 'supervisor' and exit_code == 0):
        with pytest.raises(ExecutionUnknown):
            engine.collect(key, receipt)
        assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
        second = v2.run(second_unit)
        assert second.generation == 2
        second.release()
    else:
        with pytest.raises(ClaimRefused, match='Boat'):
            v2.run(second_unit)
        assert v2.claim_record()['generation'] == 1
        assert v2.writes() == before
    assert_no_collection(run, key, cli)


@pytest.mark.parametrize('case', CASES)
@pytest.mark.parametrize('exit_code', [0, 2])
@pytest.mark.parametrize('has_intent', [True, False])
def test_queue_slot_uses_same_stopped_failure_evidence(queued, monkeypatch, case, exit_code, has_intent):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit),
                      MODELS, launcher=FAKE, source_repo=v2.source)
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    stage_evidence(engine, key, binding, case, exit_code)
    if has_intent:
        (run.dir / 'intents').mkdir(exist_ok=True)
        (run.dir / 'intents' / f'{key.name}.json').write_text('{}')
    release_claim(intake.claims, TICKET, unit.run_id, 'controller released')
    v2.github.issues[TICKET]['labels'].remove('deepagents:queue')
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    seen = []
    before = v2.writes()
    rows = intake.once(lambda item: seen.append(item.ticket) or 0)
    if case in DRAINED and not (case == 'supervisor' and exit_code == 0):
        assert seen == [TICKET + 1]
    else:
        assert seen == []
        assert rows[TICKET]['phase'] == 'active'
        assert rows[TICKET + 1]['phase'] == 'queued'
    assert v2.writes() == before
    assert_no_collection(run, key, cli)


def test_actual_prelaunch_cancel_receipt_is_drained_but_never_collectible(env, tmp_path, monkeypatch):
    run = env.create()
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    remote = tmp_path / 'remote'
    remote.mkdir()
    request = {'identity': binding.identity, 'package_id': binding.package_id, 'root': str(remote)}
    assert boat_remote.cancel(request, remote / 'request.json')['state'] == 'receipt'
    receipt = json.loads((remote / 'claim/receipt.json').read_text())
    engine._path(key, '.remote.json').write_text(json.dumps(receipt))
    with pytest.raises(ExecutionUnknown):
        engine.collect(key, receipt)
    assert engine.is_drained(key)
    assert_no_collection(run, key, cli)


@pytest.mark.parametrize('status', ['failed', 'cancelled'])
def test_failed_with_zero_exit_drains_without_collecting_incomplete_files(env, monkeypatch, status):
    run = env.create()
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    files = [{'path': 'demo/not-imported', 'mode': '100644', 'content': 'bm8='}]
    receipt = payload(run, engine, key, files, status='?? demo/not-imported\n?? outside-scope.txt')
    receipt['state'] = 'terminal'
    receipt['record'].update(status=status, process_exit={'code': 0, 'signal': None})
    engine._path(key, '.remote.json').write_text(json.dumps(receipt))
    before = engine._path(key, '.remote.json').read_bytes()

    with pytest.raises(ExecutionUnknown, match='业务回执与产物清单不同'):
        engine.collect(key, receipt)
    assert engine.is_drained(key)
    assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
    assert engine._path(key, '.remote.json').read_bytes() == before
    assert engine.load(key) == binding
    assert_no_collection(run, key, cli)
