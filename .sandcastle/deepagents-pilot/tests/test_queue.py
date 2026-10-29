import json

import pytest
from approval_signing.protocol import Decision
from signing_fixture import Signer
from v2env import TICKET, V2Env

from pilot1742.claims import ClaimStore
from pilot1742.cutover import CutoverStore, FrozenSnapshot, RemoteBarrier
from pilot1742.executor import AdmissionRefused, ApprovalRequired
from pilot1742.queue import QueueIntake, QueueWaiting
from pilot1742.run import Halted, PilotRun
from pilot1742.workunit import DedicatedPilot, Queued, forbidden_for, parse_work_unit

SPEC = 1340


@pytest.fixture
def queued(tmp_path, monkeypatch):
    v2 = V2Env(tmp_path)
    v2.workers_log.write_text('')
    drain = tmp_path / 'drain'
    drain.mkdir()
    (drain / 'state.json').write_text('{"v":1,"mode":"draining","generation":1}')
    cutover = CutoverStore(v2.home, drain, v2.workers_log)
    cutover.record(cutover.verify(FrozenSnapshot(1, 'fixture', (), 0, str(v2.workers_log.resolve())),
                                  RemoteBarrier(True, '1', ('com.newchanlun.sandcastle-claimer',
                                                           'com.newchanlun.sandcastle-harvest')), (), v2.base))
    v2.github.add_issue(SPEC, body='# SPEC\ndeepagents-scope: demo/\n')
    issue = v2.github.issues[TICKET]
    issue.update(body=f'Part of [SPEC #{SPEC}](https://github.com/xy7365527-lang/NewChanlun/issues/{SPEC})',
                 parent=SPEC, labels=['ready-for-agent', 'deepagents:queue'], assignees=[])
    signer = Signer(tmp_path / 'approval-receipts')
    v2.approvals = signer.protect(monkeypatch)
    intake = QueueIntake(v2.home, v2.gateway(), ClaimStore(v2.claims_root), cutover, v2.source)
    return v2, cutover, intake, signer


def executor(v2, cutover, unit):
    result = v2.executor(unit)
    result.cutover = cutover
    return result


def authorize(v2, cutover, unit, signer):
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    with pytest.raises(ApprovalRequired):
        executor(v2, cutover, unit).claim(run_dir)
    signer.publish(run_dir / 'approvals' / 'spec_authorization.request', Decision.APPROVE)
    assert v2.writes() == []


def test_queue_roundtrip_does_not_change_dedicated_rules(queued):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    assert isinstance(unit.admission, Queued) and not unit.requires_candidate_approval
    assert parse_work_unit(unit.to_json()) == unit
    assert forbidden_for(unit.admission, {'ready-for-agent', 'wayfinder:impl'}) == ()
    assert forbidden_for(unit.admission, {'needs-approval', 'agent:explore', 'sandcastle'}) == (
        'agent:explore', 'needs-approval', 'sandcastle')
    proof = v2.github.issues[TICKET]
    proof.update(body='deepagents:dedicated-pilot #1742', parent=None, labels=[])
    dedicated = v2.register(f't{TICKET}-99')
    assert isinstance(dedicated.admission, DedicatedPilot)
    assert forbidden_for(dedicated.admission, {'ready-for-agent'}) == ('ready-for-agent',)


@pytest.mark.parametrize('bad', ['sandcastle', 'agent:explore', 'needs-approval', 'assignee', 'parent', 'tail',
                                 'scope', 'ticket_scope_only'])
def test_queue_refuses_missing_contract_without_writes(queued, bad):
    v2, cutover, intake, signer = queued
    issue = v2.github.issues[TICKET]
    if bad == 'assignee':
        issue['assignees'] = [{'login': 'other'}]
    elif bad == 'parent':
        issue['parent'] = 999
    elif bad == 'tail':
        v2.workers_log.write_text(json.dumps({'ticket': TICKET, 'phase': 'sandbox', 'status': 'closed'}) + '\n')
    elif bad in ('scope', 'ticket_scope_only'):
        v2.github.issues[SPEC]['body'] = '# SPEC'
        if bad == 'ticket_scope_only':
            issue['body'] += '\ndeepagents-scope: demo/\n'
    else:
        issue['labels'].append(bad)
    with pytest.raises((AdmissionRefused, QueueWaiting)):
        intake.register(TICKET)
    assert v2.writes() == []


def test_signed_queue_delivers_exact_head_and_replays_without_duplicate_effects(queued):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    assert intake.register(TICKET) == unit
    authorize(v2, cutover, unit, signer)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS, launcher=FAKE, source_repo=v2.source)
    head = v2.to_pass(run)
    result = run.publish_candidate()
    assert result['phase'] == 'awaiting_merge' and result['head'] == head
    before = v2.writes()
    assert run.publish_candidate()['head'] == head and v2.writes() == before
    run.release()
    results = intake.once(lambda _unit: pytest.fail('completed run was restarted'))
    assert results[TICKET]['phase'] == 'awaiting_merge'
    assert results[TICKET]['head'] == v2.github.ref(unit.branch) == head


def test_spec_changes_wait_for_new_signature_and_old_label_halts(queued):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    ex = executor(v2, cutover, unit)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    claim = ex.claim(run_dir)
    before = v2.writes()
    v2.github.issues[SPEC]['body'] += 'changed\n'
    with pytest.raises(ApprovalRequired):
        with ex.action('dispatch', run_dir, claim.generation):
            pytest.fail('new SPEC body accepted')
    assert v2.writes() == before and v2.claim_record()['state'] == 'active'
    signer.publish(run_dir / 'approvals' / 'spec_authorization.request', Decision.APPROVE)
    v2.github.issues[TICKET]['labels'].append('sandcastle')
    with pytest.raises(Halted, match='forbidden'):
        with ex.action('dispatch', run_dir, claim.generation):
            pytest.fail('old label accepted')


def test_no_cutover_means_no_read_registration_or_status_write(queued):
    v2, cutover, intake, signer = queued
    cutover.path.unlink()
    before = len(v2.github.requests)
    assert intake.once(lambda _: pytest.fail('spawned')) == {}
    assert len(v2.github.requests) == before and v2.writes() == []
    assert not (v2.home / 'queue-status.json').exists()


def test_busy_run_is_skipped_and_same_run_is_recovered_next_round(queued):
    v2, cutover, intake, signer = queued
    seen = []
    for code in (75, -9, 0):
        rows = intake.once(lambda unit, code=code: seen.append(unit.run_id) or code)
        assert rows[TICKET]['phase'] == ('halted' if code == 0 else 'active')
    assert seen == [f't{TICKET}-1'] * 3


def test_engine_emission_to_queue_child_sigkill_recovery_and_exact_delivery(queued):
    import base64
    import multiprocessing
    import subprocess
    import sys
    import threading
    from pathlib import Path

    import httpx
    from cryptography.hazmat.primitives import serialization

    v2, cutover, intake, signer = queued
    module = Path(__file__).resolve().parents[3] / 'scripts' / 'wayfinder_engine.mts'
    script = '''const m = await import(process.argv[1]);
      const label = m.releaseLabel(process.argv[2], process.argv[3]);
      m.releaseTicket(4242, {read: () => JSON.stringify({state:'OPEN',labels:[],assignees:[]}),
        write: cmd => { console.log(cmd); return ''; }}, label);'''
    emitted = subprocess.run(['node', '--experimental-strip-types', '--input-type=module', '-e', script,
                              module.as_uri(), str(v2.home), str(cutover.drain_root)],
                             check=True, capture_output=True, text=True).stdout.strip()
    assert emitted == 'issue edit 4242 --add-label deepagents:queue'
    v2.github.add_issue(1323, body='map', labels=('wayfinder:map',))
    v2.github.issues[SPEC]['parent'] = 1323
    v2.github.issues[TICKET]['parent'] = 1323
    v2.github.issues[TICKET]['labels'] = ['ready-for-agent', emitted.split()[-1]]
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    v2.control({'author': {'write': {'demo/a.py': 'x = 1\n'}}, 'review': {'summary': 'VERDICT: PASS\n可以'}})
    seen = []
    public_trust = v2.tmp / 'public-trust.json'
    spki = signer.private.public_key().public_bytes(serialization.Encoding.DER,
                                                   serialization.PublicFormat.SubjectPublicKeyInfo)
    public_trust.write_text(json.dumps({'format': 'newchanlun-approval-trust/v1', 'repository_id': 1150335160,
        'allowed_scopes': ['candidate_head', 'spec_authorization'],
        'keys': [{'key_id': signer.key_id, 'public_key_spki_der_b64': base64.b64encode(spki).decode(), 'label': 'pytest'}]}))

    def runner(unit):
        seen.append(unit.run_id)
        parent, child = multiprocessing.Pipe()

        def server():
            try:
                while True:
                    method, url, headers, body = parent.recv()
                    response = v2.github(httpx.Request(method, url, headers=headers, content=body))
                    parent.send((response.status_code, dict(response.headers), response.content))
            except (EOFError, OSError):
                pass
            finally:
                parent.close()

        config = v2.tmp / 'queue-child.json'
        config.write_text(json.dumps({'home': str(v2.home), 'source': str(v2.source), 'run_id': unit.run_id,
            'claims': str(v2.claims_root), 'workers': str(v2.workers_log), 'drain': str(cutover.drain_root),
            'pem': str(v2.identity.private_key_path), 'authority': str(signer.root), 'trust': str(public_trust),
            'crash': 'after_push_before_receipt' if len(seen) == 1 else None}))
        proc = subprocess.Popen([sys.executable, str(Path(__file__).with_name('queue_child.py')),
                                 str(config), str(child.fileno())], pass_fds=(child.fileno(),))
        child.close()
        thread = threading.Thread(target=server, daemon=True)
        thread.start()
        proc.wait(timeout=30)
        thread.join(5)
        return proc.returncode

    first = intake.once(runner)
    assert first[TICKET]['phase'] == 'active'
    assert v2.github.ref(unit.branch) is not None and not v2.github.pulls
    second = intake.once(runner)
    assert second[TICKET]['phase'] == 'awaiting_merge'
    head = second[TICKET]['head']
    assert head == v2.github.ref(unit.branch) == v2.gateway().get_pr(next(iter(v2.github.pulls)))['head']['sha']
    assert seen == [unit.run_id, unit.run_id] and v2.claim_record()['generation'] == 1
    assert v2.github.count('POST', rf'.*/issues/{TICKET}/comments') == 2
    assert v2.github.count('POST', r'.*/git/refs') == 1
    assert v2.github.count('POST', r'.*/pulls') == 1
    assert intake.once(lambda _: pytest.fail('delivered run dispatched again'))[TICKET]['head'] == head


def test_serve_modes_do_not_open_app_without_cutover(tmp_path, monkeypatch, capsys):
    from pilot1742 import executor as cli
    monkeypatch.setattr(cli, '_client', lambda _: pytest.fail('App opened without cutover'))
    for mode in ('--once', '--loop'):
        assert cli.main(['--state-root', str(tmp_path / 'absent'), 'serve', mode,
                         '--private-key', str(tmp_path / 'absent.pem')]) == 0
    assert 'queue inactive' in capsys.readouterr().out
    assert not (tmp_path / 'absent').exists()


def test_spec_recheck_interrupts_graph_and_resume_payload_does_not_authorize(queued):
    import sqlite3

    from conftest import FAKE, MODELS
    from langchain_core.messages import AIMessage
    from langgraph.checkpoint.sqlite import SqliteSaver
    from langgraph.types import Command
    from test_agent import call, scripted

    from pilot1742.agent import build_agent
    from pilot1742.executor import load_unit, open_v2_run

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS, launcher=FAKE, source_repo=v2.source)
    v2.github.issues[SPEC]['body'] += 'changed\n'
    v2.control({'author': {'write': {'demo/a.py': 'x = 1\n'}}})
    model = scripted(call('run_worker', role='author', brief='实现'), AIMessage('完'))
    agent = build_agent(run, SqliteSaver(sqlite3.connect(run.dir / 'graph.sqlite', check_same_thread=False)), model)
    config = {'configurable': {'thread_id': unit.run_id}}
    try:
        agent.invoke({'messages': [('user', '开始')]}, config, durability='sync')
        assert agent.get_state(config).next and v2.claims() == []
        agent.invoke(Command(resume={'approved': True}), config, durability='sync')
        assert agent.get_state(config).next and v2.claims() == []
        signer.publish(run.dir / 'approvals' / 'spec_authorization.request', Decision.APPROVE)
        agent.invoke(Command(resume={}), config, durability='sync')
        assert not agent.get_state(config).next and v2.claims() == [f'author-{v2.base}']
    finally:
        run.release()


def test_global_serve_lock_and_single_inflight_limit(queued):
    import threading

    from pilot1742.queue import MAX_IN_FLIGHT

    v2, cutover, intake, signer = queued
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    entered, finish = threading.Event(), threading.Event()
    seen = []

    def runner(unit):
        seen.append(unit.ticket)
        entered.set()
        assert finish.wait(10)
        return 75

    thread = threading.Thread(target=lambda: intake.once(runner))
    thread.start()
    assert entered.wait(10)
    try:
        assert MAX_IN_FLIGHT == 1
        assert intake.once(lambda _: pytest.fail('second controller spawned')) == {}
    finally:
        finish.set()
        thread.join(10)
    assert not thread.is_alive() and seen == [TICKET]
    snapshot = json.loads((v2.home / 'queue-status.json').read_text())
    assert snapshot[str(TICKET)]['phase'] == 'active' and snapshot[str(TICKET + 1)]['phase'] == 'queued'


def test_shared_map_contract_refuses_cross_map_spec(queued):
    v2, cutover, intake, signer = queued
    v2.github.add_issue(1323, body='map', labels=('wayfinder:map',))
    v2.github.add_issue(999, body='other map', labels=('wayfinder:map',))
    v2.github.issues[TICKET]['parent'] = 1323
    v2.github.issues[SPEC]['parent'] = 999
    with pytest.raises(AdmissionRefused, match='shared map'):
        intake.register(TICKET)
    v2.github.issues[SPEC]['parent'] = 1323
    assert intake.register(TICKET).admission.proof.spec_issue == SPEC
    assert v2.writes() == []


def test_queue_post_check_conflict_halts_and_withdraws_own_mirror(queued):
    from pilot1742.claims import ClaimRefused

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)

    def concurrent_actor(method, path):
        if method == 'POST' and path.endswith(f'/issues/{TICKET}/comments'):
            v2.github.issues[TICKET]['labels'].append('sandcastle')

    v2.github.on_request.append(concurrent_actor)
    with pytest.raises(ClaimRefused, match='post_check_failed'):
        executor(v2, cutover, unit).claim(PilotRun.run_dir(v2.home, unit.run_id))
    assert v2.claim_record()['state'] == 'halted'
    assert 'deepagents' not in v2.github.issues[TICKET]['labels']


@pytest.fixture
def delivered(queued):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                      launcher=FAKE, source_repo=v2.source)
    try:
        head = v2.to_pass(run)
        assert run.publish_candidate()['phase'] == 'awaiting_merge'
    finally:
        run.release()
    return v2, intake, unit, head, run.dir


@pytest.mark.parametrize('change', ['closed', 'draft', 'head'])
def test_delivered_queue_rechecks_remote_without_dispatch_or_writes(delivered, change):
    from conftest import git

    v2, intake, unit, head, run_dir = delivered
    (number,) = v2.github.pulls
    if change == 'closed':
        v2.github.close_pull(number)
    elif change == 'draft':
        v2.github.pulls[number]['draft'] = False
    else:
        git(v2.source, 'push', '-q', '--force', str(v2.bare), f'{v2.base}:refs/heads/{unit.branch}')
    before = v2.writes()
    rows = intake.once(lambda _: pytest.fail('delivered run dispatched again'))
    assert rows[TICKET]['phase'] == 'halted'
    if change == 'head':
        assert rows[TICKET]['delivery_phase'] == 'stale'
        assert json.loads((run_dir / 'delivery' / 'stale.json').read_text())['observed'] == v2.base
        assert v2.claim_record()['state'] == 'active'
        git(v2.source, 'push', '-q', str(v2.bare), f'{head}:refs/heads/{unit.branch}')
        assert intake.once(lambda _: pytest.fail('stale run dispatched again'))[TICKET]['phase'] == 'halted'
    else:
        assert v2.claim_record()['state'] == 'halted'
        assert ('closed_pull_request' if change == 'closed' else 'not_draft') in rows[TICKET]['reason']
    assert rows[TICKET]['run_id'] == unit.run_id
    assert v2.writes() == before and v2.claim_record()['generation'] == 1


@pytest.mark.parametrize('ticket_closed', [False, True])
def test_delivered_queue_observes_merge_even_after_ticket_closes(delivered, ticket_closed):
    v2, intake, unit, head, run_dir = delivered
    (number,) = v2.github.pulls
    v2.github.merge_pull(number)
    if ticket_closed:
        from conftest import git
        v2.github.issues[TICKET]['state'] = 'closed'
        git(v2.source, 'update-ref', '-d', f'refs/heads/{unit.branch}')
        git(v2.bare, 'update-ref', '-d', f'refs/heads/{unit.branch}')
        v2.github.pulls[number]['draft'] = False
    before = v2.writes()
    rows = intake.once(lambda _: pytest.fail('merged run dispatched again'))
    assert rows[TICKET]['phase'] == 'awaiting_merge'
    assert rows[TICKET]['delivery_phase'] == 'merged'
    assert json.loads((run_dir / 'delivery' / 'state.json').read_text())['phase'] == 'merged'
    assert rows[TICKET]['head'] == head and rows[TICKET]['run_id'] == unit.run_id
    assert v2.writes() == before and v2.claim_record()['state'] == 'active'


@pytest.mark.parametrize('fault', ['status_500', 'drop_before'])
def test_delivery_read_failure_is_retryable_not_stale(delivered, fault):
    v2, intake, unit, head, run_dir = delivered
    (number,) = v2.github.pulls
    state_path = run_dir / 'delivery' / 'state.json'
    before_state, before_writes = state_path.read_bytes(), v2.writes()
    v2.github.fault('GET', rf'.*/pulls/{number}', fault)
    rows = intake.once(lambda _: pytest.fail('delivered run dispatched again'))
    assert rows[TICKET]['phase'] == 'awaiting_human'
    assert rows[TICKET]['run_id'] == unit.run_id and rows[TICKET]['reason']
    assert state_path.read_bytes() == before_state
    assert not (run_dir / 'delivery' / 'stale.json').exists()
    assert v2.claim_record()['state'] == 'active'
    rows = intake.once(lambda _: pytest.fail('delivered run dispatched again'))
    assert rows[TICKET]['phase'] == 'awaiting_merge'
    assert rows[TICKET]['head'] == head and rows[TICKET]['run_id'] == unit.run_id
    assert v2.writes() == before_writes
