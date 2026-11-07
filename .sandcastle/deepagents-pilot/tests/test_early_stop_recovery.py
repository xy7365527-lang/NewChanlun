import inspect
import json
import time
from dataclasses import replace
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest
import test_queue
from approval_signing.protocol import Decision
from conftest import FAKE, MODELS, git
from langchain_core.messages import AIMessage, HumanMessage, SystemMessage
from test_agent import call, scripted
from test_boat import config
from test_queue import SPEC, authorize, executor
from v2env import TICKET, SimulatedCrash

from pilot1742 import agent as agent_module
from pilot1742 import approval, boat_execution, cli
from pilot1742 import run as run_module
from pilot1742.boat_execution import ExecutionUnknown, ForkIntent, new_execution
from pilot1742.domain import DEEPAGENTS_OUTPUT_PROTOCOL, EffectUnknown, WorkerKey
from pilot1742.executor import (
    ApprovalRequired,
    load_unit,
    open_v2_run,
    release_claim,
    run_quiet,
)
from pilot1742.run import Halted, PilotRun

SPEC_BODY = '# SPEC\ndeepagents-scope: demo/\n\n## 行为依据\n依据 EXECUTOR.md 与 BOAT.md。\n'
queued = test_queue.queued
RUNTIME = ('package.json', 'package-lock.json', '.sandcastle/boat-worker.mts', '.sandcastle/codex-child.ts',
           '.sandcastle/docker-isolation.mts', '.sandcastle/acceptance-dir.mts', '.sandcastle/git-environment.ts',
           '.sandcastle/main-model.mts', '.sandcastle/scheduler-config.mts', '.sandcastle/scheduler-config.json',
           '.sandcastle/deepagents-pilot/pilot1742/boat_remote.py')
LEGACY_PROMPT = '旧作者 prompt：用 gh issue view 1862 读 SPEC 正文。\n'


class ObservationsFailBoat:
    executable = 'boat'

    def __init__(self):
        self.calls, self.hydrated, self.receipt, self.stopped = [], False, None, False

    def names(self, name):
        return [c for c in self.calls if c[0] == name]

    def call(self, *args):
        name = 'detach' if args[0] == 'exec' and '--detach' in args else args[0]
        self.calls.append((name, *args[1:]))
        if name == 'scp' and self.receipt is not None and args[1].endswith('/claim/receipt.json'):
            Path(args[2]).write_text(json.dumps(self.receipt))
        if name == 'stop':
            self.stopped = True
            return [{'operation': {'id': 'stop_fixture'}}]
        stop_at = (datetime.now(timezone.utc) + timedelta(seconds=1700)).isoformat().replace('+00:00', 'Z')
        state = 'stopped' if self.stopped else 'running'
        return {'limits': [{'canStart': True, 'creditUsedSeconds': 0}], 'rename': [{}], 'scp': [],
                'detach': [{'commandId': 'cmd_1', 'processId': 7}],
                'info': [{'sandbox': {'state': state, 'hydrated': self.hydrated, 'archiveAfter': stop_at}}]}[name]

    def info(self, sandbox):
        return self.call('info', sandbox)[-1]['sandbox']

    def exec(self, sandbox, command, *, timeout=30):
        self.calls.append(('exec', command))
        if ' observe ' in command:
            if self.receipt is None:
                raise ExecutionUnknown('远端观察失败（夹具）')
            return {'identity': self.receipt['identity'], 'package_id': self.receipt['package_id'], 'state': 'receipt'}
        return {}

    def stage_receipt(self, receipt):
        self.receipt = receipt


class CountingFork:
    def __init__(self):
        self.intents = []

    def fork(self, intent):
        self.intents.append(intent.idempotency_key)
        return 'bx_worker'


class Bench:
    def __init__(self, v2, cutover, intake, signer, unit, boat, forks, monkeypatch):
        self.v2, self.cutover, self.intake, self.signer, self.unit = v2, cutover, intake, signer, unit
        self.boat, self.forks, self.monkeypatch = boat, forks, monkeypatch
        self.run: PilotRun | None = None
        self.key = WorkerKey(unit.run_id, 'author', v2.base)

    def open(self) -> PilotRun:
        task = load_unit(self.v2.home, self.unit.run_id)[1]
        self.dir = PilotRun.run_dir(self.v2.home, self.unit.run_id)
        self.run = open_v2_run(self.v2.home, self.unit, task, executor(self.v2, self.cutover, self.unit), MODELS,
                               launcher=FAKE, source_repo=self.v2.source)
        return self.run

    def release(self) -> None:
        if self.run is not None:
            self.run.release()
            self.run = None

    def brain(self, *messages):
        model = scripted(*messages)
        self.monkeypatch.setattr(agent_module, 'build_model', lambda _slug: model)
        return model

    def graph(self) -> dict:
        import sqlite3

        from langgraph.checkpoint.sqlite import SqliteSaver

        from pilot1742.agent import build_agent
        conn = sqlite3.connect(self.run.dir / 'graph.sqlite', check_same_thread=False)
        try:
            agent = build_agent(self.run, SqliteSaver(conn), scripted())
            snapshot = agent.get_state({'configurable': {'thread_id': self.unit.run_id}})
            return {'next': list(snapshot.next), 'messages': snapshot.values.get('messages', []),
                    'interrupts': [i.value for t in snapshot.tasks for i in getattr(t, 'interrupts', ())]}
        finally:
            conn.close()

    def trail(self) -> list[str]:
        return [json.loads(line)['event'] for line in (self.dir / 'trail.jsonl').read_text().splitlines()]

    def binding(self):
        return self.run.execution.load(self.key)

    def binding_for(self, key):
        return self.run.execution.load(key)

    def finished_graph_with_pending_binding(self, **binding_changes) -> None:
        import sqlite3

        from langgraph.checkpoint.sqlite import SqliteSaver

        from pilot1742.agent import build_agent
        run, key = self.run, self.key
        prompt = run.dir / 'prompts' / f'{key.name}.md'
        prompt.parent.mkdir(parents=True)
        prompt.write_text(LEGACY_PROMPT)
        (run.dir / 'intents').mkdir()
        (run.dir / 'intents' / f'{key.name}.json').write_text(json.dumps({
            'key': key.name, 'run_id': run.spec.run_id, 'role': 'author', 'target_head': key.target_head,
            'mode': 'execute', 'model': run.spec.author_model, 'checkout': run.spec.author_checkout,
            'prompt_file': str(prompt), 'created_at': '2026-11-03T23:11:20+00:00'}))
        engine = run.execution
        binding = replace(engine._intent(key), sandbox_id='bx_worker', create_requested=True,
                          fork_intent=ForkIntent('bx_base', 'legacy-key', '{}', time.time()))
        engine._store(key, replace(binding, **binding_changes))
        conn = sqlite3.connect(run.dir / 'graph.sqlite', check_same_thread=False)
        try:
            build_agent(run, SqliteSaver(conn), scripted(call('run_status'), AIMessage('卡在作者这一步，没有交付。'))).invoke(
                {'messages': [HumanMessage('开始')]}, {'configurable': {'thread_id': run.spec.run_id}}, durability='sync')
        finally:
            conn.close()


@pytest.fixture
def bench(queued, tmp_path, monkeypatch):
    v2, cutover, intake, signer = queued
    source_root = Path(__file__).resolve().parents[3]
    for name in RUNTIME:
        target = v2.source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((source_root / name).read_bytes())
    manuals = v2.source / '.sandcastle/deepagents-pilot'
    (manuals / 'EXECUTOR.md').write_text('EXECUTOR_FROZEN\n')
    (manuals / 'BOAT.md').write_text('BOAT_FROZEN\n')
    (v2.source / 'AGENTS.md').write_text('AGENTS_FROZEN\n')
    skill = v2.source / '.agents/skills/project/SKILL.md'
    skill.parent.mkdir(parents=True)
    skill.write_text('---\nname: project\ndescription: fixture\n---\nPROJECT\n')
    git(v2.source, 'add', '.')
    git(v2.source, 'commit', '-qm', 'runtime and manuals')
    git(v2.source, 'push', '-q', str(v2.bare), 'main')
    v2.base = git(v2.source, 'rev-parse', 'HEAD')
    v2.github.issues[SPEC]['body'] = SPEC_BODY
    pstack = tmp_path / 'pstack'
    entry = pstack / 'installed/skills/poteto-mode/SKILL.md'
    entry.parent.mkdir(parents=True)
    entry.write_text('---\nname: pstack-upstream-poteto-mode\ndescription: fixture\n---\nPSTACK\n')
    (pstack / 'upstream').mkdir()
    (pstack / 'upstream/LICENSE').write_text('LICENSE')
    (pstack / 'aliases.json').write_text('{}')
    rules = tmp_path / 'rules.md'
    rules.write_text(MODELS)
    path = tmp_path / 'boat-config.json'
    path.write_text(json.dumps({**config(), 'pstack_source': str(pstack), 'model_rules': str(rules)}))
    monkeypatch.setenv('PILOT_BOAT_CONFIG', str(path))
    monkeypatch.setattr(run_module, 'new_execution', new_execution)
    boat, forks = ObservationsFailBoat(), CountingFork()
    monkeypatch.setattr(boat_execution, 'BoatCLI', lambda: boat)
    monkeypatch.setattr(boat_execution, 'BoatForkAPI', lambda: forks)
    monkeypatch.setattr(boat_execution.BoatExecution, '_provision_key', lambda self, binding: None)
    monkeypatch.setattr(time, 'sleep', lambda _seconds: None)
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    item = Bench(v2, cutover, intake, signer, unit, boat, forks, monkeypatch)
    item.open()
    try:
        yield item
    finally:
        item.release()


def drive_with_baseline_signature_support(run, *, recheck_approval, resume=False):
    if 'resume' in inspect.signature(cli.drive).parameters:
        return cli.drive(run, recheck_approval=recheck_approval, resume=resume)
    return cli.drive(run, recheck_approval=recheck_approval)


def explicit_resume(bench) -> None:
    run = bench.open()
    run.clear_interrupt()
    drive_with_baseline_signature_support(run, recheck_approval=True, resume=True)


def test_hydration_timeout_is_a_formal_stop_and_explicit_resume_forks_once(bench):
    bench.brain(call('run_worker', role='author', brief='实现'))
    drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    graph = bench.graph()
    assert graph['next'] and graph['interrupts'], 'ExecutionUnknown must stop at the tool checkpoint'
    stop = bench.run.interrupt_requested()
    assert stop['by'] == 'execution_unknown' and 'hydrated' in stop['reason']
    assert bench.forks.intents and len(bench.forks.intents) == 1
    assert bench.binding().sandbox_id == 'bx_worker' and bench.binding().submission == 'creating'
    assert 'brain_stopped_early' not in bench.trail()

    calls = len(bench.boat.calls)
    bench.brain()
    drive_with_baseline_signature_support(bench.run, recheck_approval=True)
    assert bench.graph()['interrupts'] and bench.run.interrupt_requested()['by'] == 'execution_unknown'
    assert len(bench.boat.calls) == calls, 'background approval recheck must not touch Boat'
    bench.release()
    rows = bench.intake.once(lambda _unit: pytest.fail('background reopened a formal stop'))
    assert 'execution_unknown' in rows[TICKET]['reason']

    bench.boat.hydrated = True
    bench.brain()
    explicit_resume(bench)
    assert len(bench.forks.intents) == 1, 'same key must reuse the forked VM'
    assert len(bench.boat.names('detach')) == 1 and bench.binding().submission == 'submitted'
    assert bench.binding().fork_intent.idempotency_key == bench.forks.intents[0]
    stop = bench.run.interrupt_requested()
    assert stop['by'] == 'execution_unknown' and '远端观察失败' in stop['reason'] and bench.graph()['interrupts']
    assert 'halted' not in bench.trail()


def test_legacy_early_stop_resumes_the_same_run_only_through_explicit_resume(bench):
    bench.finished_graph_with_pending_binding()
    run = bench.run
    prompt_before = (run.dir / 'prompts' / f'{bench.key.name}.md').read_bytes()
    intent_before = (run.dir / 'intents' / f'{bench.key.name}.json').read_bytes()
    assert bench.graph()['next'] == [] and run.interrupt_requested() is None

    bench.brain()
    drive_with_baseline_signature_support(run, recheck_approval=True)
    assert bench.trail()[-3:] == ['graph_already_finished', 'brain_stopped_early', 'operator_interrupt_requested']
    assert run.interrupt_requested()['by'] == 'brain_stopped_early'
    from pilot1742.studio import continuation_problem
    assert '--resume' in continuation_problem(run, resume=False, checkpoint=None)
    assert bench.boat.calls == [] and bench.forks.intents == []
    bench.release()
    rows = bench.intake.once(lambda _unit: pytest.fail('background reopened an early-stopped run'))
    assert 'brain_stopped_early' in rows[TICKET]['reason']

    bench.brain()
    drive_with_baseline_signature_support(bench.open(), recheck_approval=False)
    assert bench.graph()['next'] == [] and bench.boat.calls == []
    bench.release()

    bench.boat.hydrated = True
    seen = []
    model = bench.brain(
        lambda messages: seen.append(messages) or call('read_file', file_path='/project/.sandcastle/deepagents-pilot/EXECUTOR.md'),
        call('run_worker', role='author', brief='实现'))
    explicit_resume(bench)
    run = bench.run
    assert model.script == [] and run.interrupt_requested()['by'] == 'execution_unknown'
    assert bench.graph()['interrupts']
    assert 'graph_resume_after_early_stop' in bench.trail()
    assert bench.forks.intents == [] and len(bench.boat.names('detach')) == 1
    assert bench.binding().sandbox_id == 'bx_worker' and bench.binding().fork_intent.idempotency_key == 'legacy-key'
    messages = bench.graph()['messages']
    assert any(isinstance(m, HumanMessage) and m.content == cli.recovery_message(run, 'needs_author') for m in messages)
    system = next(m.text for m in seen[0] if isinstance(m, SystemMessage))
    assert SPEC_BODY in system and '/project/.sandcastle/deepagents-pilot/' in system
    read = [m.content for m in messages if getattr(m, 'name', None) == 'read_file']
    assert 'EXECUTOR_FROZEN' in read[0]
    package = run.dir / 'boat/packages' / bench.binding().package_id
    request = json.loads((package / 'request.json').read_text())
    signed = json.loads((run.dir / 'approvals/spec_authorization.status.json').read_text())
    assert request['prompt'].startswith(LEGACY_PROMPT.strip()) and SPEC_BODY in request['prompt']
    assert '不要用 gh' in request['prompt'] and request['spec'] == signed['target'] and signed['status'] == 'approved'
    assert (run.dir / 'prompts' / f'{bench.key.name}.md').read_bytes() == prompt_before
    assert (run.dir / 'intents' / f'{bench.key.name}.json').read_bytes() == intent_before
    assert (package / 'repo/.sandcastle/deepagents-pilot/BOAT.md').read_text() == 'BOAT_FROZEN\n'


@pytest.mark.parametrize('case', ['no_binding', 'cancelled', 'collected', 'head_drift'])
def test_explicit_resume_does_not_reopen_a_finished_graph_without_a_pending_binding(bench, case):
    if case == 'collected':
        out = bench.run._key_dir(bench.key)
        out.mkdir(parents=True)
        (out / 'result.json').write_text(json.dumps({
            'ticket': TICKET, 'status': 'failed', 'expected_head': bench.v2.base, 'mode': 'execute',
            'model': str(bench.run.spec.author_model), 'result': {'summary': '失败'}}))
    if case == 'no_binding':
        bench.brain(call('run_status'), AIMessage('结束'))
        drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    else:
        bench.finished_graph_with_pending_binding(**({'cancel_requested': True} if case == 'cancelled' else
                                   {'submission': 'collected'} if case == 'collected' else {}))
    if case == 'head_drift':
        checkout = Path(bench.run.spec.author_checkout)
        (checkout / 'demo').mkdir()
        (checkout / 'demo/x.py').write_text('x = 1\n')
        git(checkout, 'add', '.')
        git(checkout, '-c', 'user.name=F', '-c', 'user.email=f@example.invalid', 'commit', '-qm', 'foreign')
    bench.release()
    before = bench.boat.calls[:]
    bench.brain()
    explicit_resume(bench)
    assert bench.trail()[-1] != 'operator_interrupt_requested' and bench.run.interrupt_requested() is None
    assert 'graph_resume_after_early_stop' not in bench.trail()
    assert bench.graph()['next'] == [] and bench.boat.calls == before


def test_signature_generation_and_spec_drift_refuse_before_any_recovery(bench):
    bench.finished_graph_with_pending_binding()
    run = bench.run
    frozen = (run.dir / f'spec/{SPEC}.md').read_bytes()
    assert frozen == SPEC_BODY.encode()
    bench.release()
    claim = run.dir / 'claim.json'
    bound = claim.read_text()
    claim.write_text(json.dumps({**json.loads(bound), 'generation': 2}))
    with pytest.raises(Halted, match='代次'):
        bench.open()
    claim.write_text(bound)
    bench.v2.github.issues[SPEC]['body'] = SPEC_BODY + '未签的改动\n'
    with pytest.raises(ApprovalRequired):
        bench.open()
    assert (run.dir / f'spec/{SPEC}.md').read_bytes() == frozen
    assert run.interrupt_requested() is None and 'graph_resume_after_early_stop' not in bench.trail()
    assert bench.boat.calls == [] and bench.forks.intents == []


def test_tampered_spec_copy_is_never_handed_to_brain_or_worker(bench):
    run = bench.run
    (run.dir / f'spec/{SPEC}.md').write_text(SPEC_BODY + '篡改\n')
    with pytest.raises(ValueError, match='签批目标不符'):
        run.signed_spec()
    with pytest.raises(ValueError, match='签批目标不符'):
        agent_module.system_prompt(run)


def test_old_run_on_new_controller_keeps_runtime_identity_and_records_the_upgrade(bench):
    run = bench.run
    spec_before = (run.dir / 'run.json').read_bytes()
    runtime = run.spec.execution.runtime_head
    assert not (run.dir / 'controller-upgrades.jsonl').exists()
    bench.release()
    (bench.v2.source / 'controller.txt').write_text('new controller\n')
    git(bench.v2.source, 'add', '.')
    git(bench.v2.source, 'commit', '-qm', 'controller patch')
    controller = git(bench.v2.source, 'rev-parse', 'HEAD')
    for _ in range(2):
        bench.open()
        bench.release()
    run = bench.open()
    records = run.status()['execution']['controller_upgrades']
    assert [(r['runtime_head'], r['controller_head']) for r in records] == [(runtime, controller)]
    assert (run.dir / 'run.json').read_bytes() == spec_before and run.spec.execution.runtime_head == runtime
    assert bench.trail().count('controller_upgrade') == 1


def test_operator_interrupt_survives_an_execution_unknown_stop(bench):
    bench.run.request_interrupt('pytest-operator')
    bench.brain(call('run_worker', role='author', brief='实现'))
    drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    assert bench.run.interrupt_requested()['by'] == 'pytest-operator'
    assert bench.boat.calls == [] and bench.forks.intents == []
    drive_with_baseline_signature_support(bench.run, recheck_approval=True)
    assert bench.run.interrupt_requested()['by'] == 'pytest-operator' and bench.boat.calls == []


def test_signed_spec_is_frozen_only_after_an_approved_check(queued):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    with pytest.raises(ApprovalRequired):
        executor(v2, cutover, unit).claim(run_dir)
    assert not (run_dir / 'spec').exists()
    signer.publish(run_dir / 'approvals' / 'spec_authorization.request', Decision.REJECT)
    with pytest.raises(ApprovalRequired):
        executor(v2, cutover, unit).claim(run_dir)
    assert not (run_dir / 'spec').exists()


def approve_changed_spec(bench, body):
    bench.release()
    bench.v2.github.issues[SPEC]['body'] = body
    with pytest.raises(ApprovalRequired):
        bench.open()
    bench.signer.publish(bench.dir / 'approvals/spec_authorization.request', Decision.APPROVE)


def packaged_request(bench):
    return json.loads((bench.run.dir / 'boat/packages' / bench.binding().package_id / 'request.json').read_text())


def prepared_under_spec_a(bench, monkeypatch):
    admit = boat_execution.BoatExecution._admit

    def short_life(self, sandbox):
        raise ExecutionUnknown('寿命不足，等待显式恢复')
    monkeypatch.setattr(boat_execution.BoatExecution, '_admit', short_life)
    bench.boat.hydrated = True
    bench.brain(call('run_worker', role='author', brief='实现'))
    drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    assert bench.binding().submission == 'prepared' and bench.boat.names('detach') == []
    monkeypatch.setattr(boat_execution.BoatExecution, '_admit', admit)
    return packaged_request(bench)


def test_reapproved_spec_cannot_launch_old_prepared_package(bench, monkeypatch):
    old = prepared_under_spec_a(bench, monkeypatch)
    approve_changed_spec(bench, SPEC_BODY + '\n必须增加新的业务要求 B。\n')
    bench.brain(AIMessage('结束'))
    explicit_resume(bench)
    approved = bench.run.signed_spec()
    assert approved.target != old['spec']
    assert bench.boat.names('detach') == [], 'old SPEC package must not launch under the new approval'
    stop = bench.run.interrupt_requested()
    assert stop['by'] == 'execution_unknown' and old['spec'] in stop['reason'] and approved.target in stop['reason']
    assert packaged_request(bench) == old
    assert bench.binding().submission == 'prepared' and bench.binding().sandbox_id == 'bx_worker'
    assert len(bench.forks.intents) == 1


def test_collect_refuses_output_packaged_under_an_older_spec(bench, monkeypatch):
    old = prepared_under_spec_a(bench, monkeypatch)
    approve_changed_spec(bench, SPEC_BODY + '\n必须增加新的业务要求 B。\n')
    run = bench.open()
    engine, binding = run.execution, bench.binding()
    engine._store(bench.key, replace(binding, submission='submitted'))
    model = str(run.spec.author_model)
    receipt = {'identity': binding.identity, 'package_id': binding.package_id, 'exit_code': 0, 'files': [
        {'path': 'demo/a.py', 'mode': '100644', 'content': 'eA=='}], 'record': {
        'ticket': TICKET, 'status': 'completed', 'expected_head': bench.key.target_head, 'model': model,
        'actual_model': model, 'mode': 'execute', 'run_id': 'child',
        'checkout_after': {'head': bench.key.target_head, 'status': '?? demo/a.py'},
        'result': {'summary': '完成', 'findings': [], 'validation': []}}}
    with pytest.raises(ExecutionUnknown, match='封包 SPEC'):
        engine.collect(bench.key, receipt)
    assert packaged_request(bench) == old and bench.binding().submission == 'submitted'
    assert not (Path(run.spec.author_checkout) / 'demo/a.py').exists()
    assert not run._key_dir(bench.key).exists()


def test_signed_spec_original_bytes_survive_worker_request(bench):
    body = '# SPEC\ndeepagents-scope: demo/\n\nExact path: ' + str(bench.v2.source) + '/demo/task.py\n'
    approve_changed_spec(bench, body)
    bench.open()
    bench.finished_graph_with_pending_binding()
    bench.release()
    bench.boat.hydrated = True
    bench.brain(call('run_worker', role='author', brief='实现'), AIMessage('结束'))
    explicit_resume(bench)
    request = packaged_request(bench)
    assert bench.run.signed_spec().body.decode() == body
    assert body in request['prompt'] and request['spec'] == bench.run.signed_spec().target
    assert request['prompt'].startswith(LEGACY_PROMPT.strip())


def test_repeated_unknown_resume_keeps_one_binding_and_formal_stop(bench):
    bench.brain(call('run_worker', role='author', brief='实现'))
    drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    key = bench.binding().fork_intent.idempotency_key
    for _ in range(2):
        bench.release()
        bench.brain()
        explicit_resume(bench)
        assert bench.graph()['interrupts']
        assert bench.run.interrupt_requested()['by'] == 'execution_unknown'
        assert bench.binding().fork_intent.idempotency_key == key
        assert len(bench.forks.intents) == 1


def receipt_for_intent(run, key, record):
    intent = json.loads((run.dir / 'intents' / f'{key.name}.json').read_text())
    protocol = intent.get('output_protocol', 'legacy')
    assert protocol in ('legacy', DEEPAGENTS_OUTPUT_PROTOCOL)
    record['result']['status'] = 'completed'
    if protocol == DEEPAGENTS_OUTPUT_PROTOCOL:
        record['output_protocol'] = protocol
        record['result'].update(confidence='medium', caveats=['仅夹具验证'])
    return record


def collect_completed(bench, key):
    run, engine = bench.run, bench.run.execution
    run._prepare_dispatch(key, '夹具', run.facts())
    run._launch(key)
    binding = engine.load(key)
    model = str(run.spec.model_for(key.role))
    author = key.role == 'author'
    receipt = {'identity': binding.identity, 'package_id': binding.package_id, 'exit_code': 0,
               'files': [{'path': 'demo/a.py', 'mode': '100644', 'content': 'eA=='}] if author else [],
               'record': {'ticket': TICKET, 'status': 'completed', 'expected_head': key.target_head,
                          'model': model, 'actual_model': model, 'mode': 'execute' if author else 'review',
                          'run_id': 'child', 'checkout_after': {'head': key.target_head,
                                                                'status': '?? demo/a.py' if author else ''},
                          'result': {'summary': '完成' if author else 'VERDICT: PASS\n审阅通过',
                                     'findings': [], 'validation': []}}}
    receipt['record'] = receipt_for_intent(run, key, receipt['record'])
    with run._fence('dispatch'):
        engine.collect(key, receipt)
    return json.loads((run.dir / 'boat/packages' / binding.package_id / 'request.json').read_text())['spec']


def passed_under_spec_a(bench):
    bench.boat.hydrated = True
    collect_completed(bench, bench.key)
    head = bench.run.commit_candidate().commit
    review = WorkerKey(bench.unit.run_id, 'review', head)
    target = collect_completed(bench, review)
    assert target == bench.run.signed_spec().target and bench.run.facts().verdicts[head].kind == 'PASS'
    return head, review, target


def publish_writes(bench, before):
    return [path for _method, path in bench.v2.writes()[before:]]


def test_review_pass_under_the_current_spec_publishes(bench):
    head, _review, _target = passed_under_spec_a(bench)
    result = bench.run.publish_candidate()
    assert result['ok'] is True and result['phase'] == 'awaiting_merge' and result['head'] == head


def test_review_pass_under_an_older_spec_does_not_publish(bench):
    head, review, target = passed_under_spec_a(bench)
    package = bench.run.dir / 'boat/packages' / bench.binding_for(review).package_id / 'request.json'
    original = package.read_bytes()
    approve_changed_spec(bench, SPEC_BODY + '\n必须新增业务要求 B。\n')
    run = bench.open()
    assert run.signed_spec().target != target
    before = len(bench.v2.writes())
    result = run.publish_candidate()
    assert result['ok'] is False and result['reasons'] == ['review_spec_mismatch']
    assert target in result['detail'] and run.signed_spec().target in result['detail']
    assert publish_writes(bench, before) == [] and bench.v2.github.ref(bench.unit.branch) is None
    assert package.read_bytes() == original and run.execution.load(review).submission == 'collected'
    assert run.facts().verdicts[head].kind == 'PASS' and run.facts().commits[-1].commit == head


@pytest.mark.parametrize('resign_after, stopped_before', [
    ('push', 'pr'), ('pr', 'status'), ('status', 'deliver')])
def test_spec_resigned_between_publish_effects_stops_before_the_next_write(bench, resign_after, stopped_before):
    head, _review, target = passed_under_spec_a(bench)
    run = bench.run
    executor_ = run.executor

    def resign(point):
        if point != f'after_{resign_after}_before_receipt':
            return
        bench.v2.github.issues[SPEC]['body'] = SPEC_BODY + f'\n发布中途（{resign_after} 之后）新增要求 B。\n'
        assert executor_.spec_approval(run.dir, run.generation) is not None
        bench.signer.publish(run.dir / 'approvals/spec_authorization.request', Decision.APPROVE)

    executor_.crash = resign
    before = len(bench.v2.writes())
    result = run.publish_candidate()
    assert result['ok'] is False and result['reasons'] == ['review_spec_mismatch']
    assert run.signed_spec().target != target and bench.v2.github.ref(bench.unit.branch) == head
    kinds = {'pr': '/pulls', 'status': '/statuses/', 'deliver': '/comments'}
    order = ['pr', 'status', 'deliver']
    done, blocked = order[:order.index(stopped_before)], order[order.index(stopped_before):]
    writes = publish_writes(bench, before)
    assert all(any(kinds[k] in p for p in writes) for k in done), writes
    assert not any(kinds[k] in p for k in blocked for p in writes), writes

    executor_.crash = lambda _point: None
    replay_before = len(bench.v2.writes())
    replay = run.publish_candidate()
    assert replay['ok'] is False and replay['reasons'] == ['review_spec_mismatch']
    assert publish_writes(bench, replay_before) == [], 'journal replay must re-check the review SPEC'


def completed_author_receipt(bench):
    binding, model = bench.binding(), str(bench.run.spec.author_model)
    record = {'ticket': TICKET, 'status': 'completed', 'expected_head': bench.key.target_head,
              'model': model, 'actual_model': model, 'mode': 'execute', 'run_id': 'child',
              'process_exit': {'code': 0, 'signal': None},
              'checkout_after': {'head': bench.key.target_head, 'status': '?? demo/a.py'},
              'result': {'summary': '完成', 'findings': [], 'validation': []}}
    return {'identity': binding.identity, 'package_id': binding.package_id, 'state': 'terminal', 'exit_code': 0,
            'files': [{'path': 'demo/a.py', 'mode': '100644', 'content': 'eA=='}],
            'record': receipt_for_intent(bench.run, bench.key, record)}


def author_submitted_under_spec_a(bench):
    bench.boat.hydrated = True
    bench.brain(call('run_worker', role='author', brief='实现'))
    drive_with_baseline_signature_support(bench.run, recheck_approval=False)
    stop = bench.run.interrupt_requested()
    assert bench.binding().submission == 'submitted' and len(bench.boat.names('detach')) == 1
    assert stop['by'] == 'execution_unknown' and '远端观察失败' in stop['reason'] and bench.graph()['interrupts']
    bench.boat.stage_receipt(completed_author_receipt(bench))


@pytest.mark.parametrize('legacy_intent', [False, True])
def test_same_spec_receipt_is_collected_once_through_the_tool_chain(bench, legacy_intent):
    if legacy_intent:
        bench.monkeypatch.setattr(bench.run, '_output_protocol', lambda: 'legacy')
    author_submitted_under_spec_a(bench)
    record = bench.boat.receipt['record']
    assert ('output_protocol' not in record) is legacy_intent
    assert ('confidence' not in record['result']) is legacy_intent
    bench.release()
    bench.brain(AIMessage('作者完成'))
    explicit_resume(bench)
    run = bench.run
    assert run.execution.load(bench.key).submission == 'collected' and run.interrupt_requested() is None
    assert (Path(run.spec.author_checkout) / 'demo/a.py').read_text() == 'x'
    first, again = run.commit_candidate(), run.commit_candidate()
    assert first.commit == again.commit and [c.commit for c in run.facts().commits] == [first.commit]
    assert len(bench.forks.intents) == 1 and len(bench.boat.names('detach')) == 1


def test_older_spec_receipt_stops_formally_then_cancel_drains_for_the_next_generation(bench):
    author_submitted_under_spec_a(bench)
    approve_changed_spec(bench, SPEC_BODY + '\n必须新增业务要求 B。\n')
    bench.brain()
    explicit_resume(bench)
    run, key = bench.run, bench.key
    stop = run.interrupt_requested()
    assert stop['by'] == 'execution_unknown' and '封包 SPEC' in stop['reason'] and bench.graph()['interrupts']
    assert 'halted' not in bench.trail()
    binding = bench.binding()
    assert binding.submission == 'submitted' and binding.stop_confirmed and binding.key_erased
    cache = run.dir / 'boat' / f'{key.name}.remote.json'
    receipt = cache.read_bytes()
    assert not (Path(run.spec.author_checkout) / 'demo/a.py').exists() and not run._key_dir(key).exists()
    assert not run.execution.is_drained(key)
    bench.release()
    assert 'worker slot retained' in bench.intake._worker_activity({bench.dir})

    owner = PilotRun.open(bench.v2.home, bench.unit.run_id, executor=executor(bench.v2, bench.cutover, bench.unit),
                          launcher=FAKE, source_repo=bench.v2.source)
    assert cli.cancel_worker(owner, 'author', key.target_head) == 0
    abandoned = owner.execution.load(key)
    assert abandoned.cancel_requested and abandoned.submission == 'submitted' and cache.read_bytes() == receipt
    assert owner.execution.is_drained(key)
    fact = owner.execution.observe(key)
    assert isinstance(fact, EffectUnknown) and '废弃' in fact.reason and not owner._key_dir(key).exists()
    assert owner.status()['workers'][key.name]['fact'] == 'effect_unknown' and owner.status()['verdicts'] == {}
    assert bench.intake._worker_activity({bench.dir}) is None
    owner.acquire()
    try:
        with pytest.raises(Halted, match='废弃'):
            owner.run_worker('author', '旧回执不得复活')
    finally:
        owner.release()
    record = release_claim(bench.intake.claims, bench.v2.gateway(), TICKET, bench.unit.run_id, '旧 SPEC 产物已废弃，换代')
    run_quiet(record)
    assert cache.read_bytes() == receipt and not (Path(owner.spec.author_checkout) / 'demo/a.py').exists()


def test_spec_freeze_interrupted_between_body_and_note_converges_on_the_next_approval(bench, monkeypatch):
    note, body = bench.dir / f'spec/{SPEC}.json', bench.dir / f'spec/{SPEC}.md'
    target_a = json.loads(note.read_text())['target']
    replace_bytes = approval._replace_bytes

    def killed_before_note(path, data):
        if path == note:
            raise SimulatedCrash('freeze')
        return replace_bytes(path, data)

    bench.release()
    bench.v2.github.issues[SPEC]['body'] = SPEC_BODY + 'B\n'
    with pytest.raises(ApprovalRequired):
        bench.open()
    bench.signer.publish(bench.dir / 'approvals/spec_authorization.request', Decision.APPROVE)
    monkeypatch.setattr(approval, '_replace_bytes', killed_before_note)
    with pytest.raises(SimulatedCrash):
        bench.open()
    monkeypatch.setattr(approval, '_replace_bytes', replace_bytes)
    assert body.read_bytes() == (SPEC_BODY + 'B\n').encode() and json.loads(note.read_text())['target'] == target_a
    approve_changed_spec(bench, SPEC_BODY)
    run = bench.open()
    spec = run.signed_spec()
    assert spec.body == SPEC_BODY.encode() and spec.target == target_a and body.read_bytes() == SPEC_BODY.encode()


def queued_neighbour(bench):
    bench.v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                              labels=('ready-for-agent', 'deepagents:queue'))


def status_page_section(bench, ticket):
    page = (bench.v2.source / '.sandcastle/logs/deepagents-status.md').read_text()
    section = None
    for line in page.splitlines():
        if line.startswith('## '):
            section = line[3:]
        elif line.startswith(f'- #{ticket} '):
            return section
    return None


@pytest.mark.parametrize('stop', ['execution_unknown', 'brain_stopped_early'])
def test_queue_tick_reports_a_stopped_run_as_awaiting_human_and_keeps_its_slot(bench, stop):
    if stop == 'execution_unknown':
        author_submitted_under_spec_a(bench)
    else:
        bench.finished_graph_with_pending_binding()
        bench.brain()
        drive_with_baseline_signature_support(bench.run, recheck_approval=True)
    assert bench.run.interrupt_requested()['by'] == stop
    binding = bench.binding()
    bench.release()
    queued_neighbour(bench)
    for _round in range(2):
        rows = bench.intake.once(lambda unit: pytest.fail(f'{unit.run_id} dispatched while a human stop holds the slot'))
        assert rows[TICKET]['phase'] == 'awaiting_human', rows[TICKET]
        assert stop in rows[TICKET]['reason'] and 'worker slot retained' in rows[TICKET]['reason']
        assert rows[TICKET + 1]['phase'] == 'queued'
        assert status_page_section(bench, TICKET) == 'awaiting_human'
        assert status_page_section(bench, TICKET + 1) == 'queued'
    unlocked = PilotRun.open(bench.v2.home, bench.unit.run_id, source_repo=bench.v2.source)
    assert unlocked.execution.load(bench.key) == binding and not (bench.v2.home / f'units/t{TICKET + 1}-1.json').exists()


def test_queue_tick_keeps_a_live_worker_without_a_human_stop_active(bench):
    author_submitted_under_spec_a(bench)
    bench.run.clear_interrupt()
    bench.release()
    queued_neighbour(bench)
    seen = []
    rows = bench.intake.once(lambda unit: seen.append(unit.run_id) or 75)
    assert seen == [bench.unit.run_id]
    assert rows[TICKET]['phase'] == 'active' and 'worker slot retained' in rows[TICKET]['reason']
    assert rows[TICKET + 1]['phase'] == 'queued' and status_page_section(bench, TICKET) == 'active'


def test_controller_audit_records_every_adjacent_change_including_the_return_to_runtime(bench):
    run = bench.run
    spec_before = (run.dir / 'run.json').read_bytes()
    runtime = run.spec.execution.runtime_head
    source = bench.v2.source
    bench.release()
    (source / 'controller.txt').write_text('controller B\n')
    git(source, 'add', '.')
    git(source, 'commit', '-qm', 'controller B')
    controller_b = git(source, 'rev-parse', 'HEAD')
    for head in (runtime, controller_b, controller_b, runtime, runtime, controller_b, controller_b):
        git(source, 'reset', '-q', '--hard', head)
        bench.open()
        bench.release()
    records = bench.open().status()['execution']['controller_upgrades']
    assert [(r['runtime_head'], r['controller_head']) for r in records] == [
        (runtime, controller_b), (runtime, runtime), (runtime, controller_b)]
    assert (run.dir / 'run.json').read_bytes() == spec_before and bench.run.spec.execution.runtime_head == runtime
    assert bench.trail().count('controller_upgrade') == 3
