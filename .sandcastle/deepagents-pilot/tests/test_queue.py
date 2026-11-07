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
    monkeypatch.setattr('pilot1742.cutover.container_inventory', lambda: ())
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


def publish_ruling(v2, verdict="PASS", **overrides):
    from pilot1742.approval import spec_ruling_digest
    fields = {"verdict": verdict, "specNumber": SPEC, "author": "author-session", "reviewer": "review-session",
              "digest": spec_ruling_digest(v2.github.issues[SPEC]['body']), "evidence": "/fixture/review.json"}
    fields.update(overrides)
    comments = v2.github.issues[SPEC]['comments']
    comments.append({"id": len(comments) + 100, "created_at": f"2026-11-07T00:00:{len(comments):02d}Z",
                     "user": {"login": "xy7365527-lang", "type": "User"},
                     "body": (f"**SPEC 独审裁定：{fields['verdict']}**\nSPEC：{fields['specNumber']}\n"
                              f"作者：{fields['author']}\n独审：{fields['reviewer']}\n"
                              f"正文 sha256：{fields['digest']}\n证据：{fields['evidence']}\n")})


def authorize(v2, cutover, unit, signer):
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    with pytest.raises(ApprovalRequired):
        executor(v2, cutover, unit).claim(run_dir)
    publish_ruling(v2)
    assert v2.writes() == []


@pytest.mark.parametrize('registered_before_restart', [False, True])
def test_released_drained_run_advances_to_next_generation(queued, registered_before_restart):
    from conftest import FAKE, MODELS

    from pilot1742.claims import advance
    from pilot1742.executor import load_unit, open_v2_run

    v2, cutover, intake, signer = queued
    previous = intake.register(TICKET)
    authorize(v2, cutover, previous, signer)
    old_run = open_v2_run(v2.home, previous, load_unit(v2.home, previous.run_id)[1],
                          executor(v2, cutover, previous), MODELS, launcher=FAKE, source_repo=v2.source)
    old_run.release()
    with intake.claims.locked(TICKET) as txn:
        txn.write(advance(txn.record, 'release'))
    if registered_before_restart:
        assert intake.register(TICKET).run_id == f't{TICKET}-2'
    seen = []

    def child(unit):
        seen.append(unit.run_id)
        assert unit.run_id == f't{TICKET}-2'
        ex = executor(v2, cutover, unit)
        run_dir = PilotRun.run_dir(v2.home, unit.run_id)
        claim = ex.claim(run_dir)
        assert claim.generation == 2 and claim.state == 'active'
        return 75

    rows = intake.once(child)
    assert seen == [f't{TICKET}-2'], 'released old claim must not halt the next run'
    assert rows[TICKET]['phase'] == 'active'
    assert v2.claim_record()['run_id'] == f't{TICKET}-2'
    writes = v2.writes()
    intake.once(child)
    assert seen == [f't{TICKET}-2', f't{TICKET}-2']
    assert v2.writes() == writes
    assert len(list((v2.home / 'units').glob('*.json'))) == 2


def test_queue_roundtrip_does_not_change_dedicated_rules(queued):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    assert isinstance(unit.admission, Queued) and not unit.requires_candidate_approval
    assert parse_work_unit(unit.to_json()) == unit
    assert forbidden_for(unit.admission, {'ready-for-agent', 'wayfinder:impl'}) == ()
    assert forbidden_for(unit.admission, {'needs-approval', 'agent:explore', 'sandcastle'}) == (
        'agent:explore', 'sandcastle')
    proof = v2.github.issues[TICKET]
    proof.update(body='deepagents:dedicated-pilot #1742', parent=None, labels=[])
    dedicated = v2.register(f't{TICKET}-99')
    assert isinstance(dedicated.admission, DedicatedPilot)
    assert forbidden_for(dedicated.admission, {'ready-for-agent'}) == ('ready-for-agent',)


@pytest.mark.parametrize('bad', ['sandcastle', 'agent:explore', 'ready-for-human', 'needs-info', 'needs-triage', 'wontfix', 'assignee', 'parent', 'tail',
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


def test_unsigned_queue_delivers_exact_head_and_replays_without_duplicate_effects(queued, monkeypatch):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    assert intake.register(TICKET) == unit
    authorize(v2, cutover, unit, signer)
    monkeypatch.setattr('pilot1742.approval.SignedApprovals.evaluate', lambda *a, **kw: pytest.fail('signer called'))
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


def test_spec_changes_wait_for_new_ruling_and_old_label_halts(queued):
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
    publish_ruling(v2)
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


def test_engine_emission_to_queue_child_sigkill_recovery_and_exact_delivery(queued, wayfinder_engine_path):
    import base64
    import multiprocessing
    import subprocess
    import sys
    import threading
    from pathlib import Path

    import httpx
    from cryptography.hazmat.primitives import serialization

    v2, cutover, intake, signer = queued
    module = wayfinder_engine_path
    script = '''const m = await import(process.argv[1]);
      const label = m.releaseLabel(process.argv[2], process.argv[3]);
      const spec = {number: 1340, title: 'SPEC: fixture', body: '# SPEC', state: 'OPEN'};
      const pass = {user: {login: 'fixture'}, created_at: '2026-11-07T01:00:00Z', body: m.specRulingBody({specNumber: 1340,
        verdict: 'PASS', author: 'draft', reviewer: 'review', digest: m.specBodyDigest(spec.body), evidence: '/fake/review'})};
      const read = args => args.startsWith('issue view 1340 ') ? JSON.stringify(spec)
        : args.includes('/issues/1340/comments') ? JSON.stringify([pass])
        : JSON.stringify({title: '[impl] fixture', body: 'Part of [SPEC #1340]', state: 'OPEN', labels: [], assignees: []});
      const why = m.releaseTicket(4242, {specs: [1340], orchestrator: 'fixture'},
        {read, write: cmd => { console.log(cmd); return ''; }}, label);
      if (why !== null) throw new Error(why);'''
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
        assert proc.returncode == (-9 if len(seen) == 1 else 0)
        return proc.returncode

    first = intake.once(runner)
    assert first[TICKET]['phase'] == 'active'
    assert v2.github.ref(unit.branch) is not None and not v2.github.pulls
    run_spec = json.loads((PilotRun.run_dir(v2.home, unit.run_id) / 'run.json').read_text())
    assert run_spec['execution'] == {'backend': 'test-launcher'}
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
        publish_ruling(v2)
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



def test_registration_never_publishes_partial_json(queued, monkeypatch):
    from pilot1742 import queue as module

    v2, _, intake, _ = queued
    original = module.json.dump

    def interrupted(data, stream, **kwargs):
        stream.write('{')
        stream.flush()
        raise OSError('injected unit write interruption')

    monkeypatch.setattr(module.json, 'dump', interrupted)
    with pytest.raises(OSError, match='injected'):
        intake.register(TICKET)
    assert not list((v2.home / 'units').glob('t*-*.json'))
    monkeypatch.setattr(module.json, 'dump', original)
    assert intake.register(TICKET).run_id == f't{TICKET}-1'


def test_bad_unit_is_isolated_without_stalling_unrelated_ticket(queued):
    v2, _, intake, _ = queued
    units = v2.home / 'units'
    units.mkdir(parents=True)
    bad = units / f't{TICKET}-1.json'
    bad.write_text('{')
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    seen = []
    rows = intake.once(lambda unit: seen.append(unit.ticket) or 0)
    assert seen == [TICKET + 1]
    assert rows[TICKET]['phase'] == 'halted' and str(bad) in rows[TICKET]['reason']
    assert bad.read_text() == '{'
    assert json.loads((v2.home / 'queue-status.json').read_text())[str(TICKET)]['reason']


@pytest.mark.parametrize('code', [0, 2])
@pytest.mark.parametrize('fact', ['running', 'effect_unknown'])
def test_child_exit_cannot_release_unfinished_worker_slot(queued, monkeypatch, code, fact):
    from conftest import FAKE, MODELS

    from pilot1742 import run as module
    from pilot1742.domain import WorkerKey
    from pilot1742.executor import load_unit, open_v2_run

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit),
                      MODELS, launcher=FAKE, source_repo=v2.source)
    run.release()
    key = WorkerKey(unit.run_id, 'author', v2.base)
    output = run._key_dir(key)
    if fact == 'running':
        monkeypatch.setattr(module, 'supervisor_alive', lambda *_: True)
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    seen = []

    def child(item):
        seen.append(item.ticket)
        output.mkdir(parents=True, exist_ok=True)
        return code

    rows = intake.once(child)
    assert seen == [TICKET]
    assert rows[TICKET]['phase'] == 'active'
    assert rows[TICKET + 1]['phase'] == 'queued'
    assert fact in rows[TICKET]['reason']
    rows = intake.once(child)
    assert seen == [TICKET, TICKET], 'same run must retain recovery ownership'
    assert rows[TICKET]['phase'] == 'active'
    (output / 'result.json').write_text(json.dumps({
        'ticket': TICKET, 'status': 'failed', 'expected_head': v2.base,
        'mode': 'execute', 'model': 'gpt-6.1-sol', 'result': {'summary': 'finished'}}))
    intake.once(child)
    assert seen == [TICKET, TICKET, TICKET, TICKET + 1]


@pytest.mark.parametrize('state', ['invalid_unit', 'interrupted', 'halted', 'released', 'removed_queue'])
def test_existing_boat_worker_keeps_slot_across_skip_paths(queued, monkeypatch, state):
    from dataclasses import replace

    from conftest import FAKE, MODELS
    from test_boat import config

    from pilot1742.boat_execution import BoatBinding, BoatConfig, BoatExecution, BoatRun
    from pilot1742.claims import advance
    from pilot1742.domain import WorkerKey
    from pilot1742.executor import load_unit, open_v2_run

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit),
                      MODELS, launcher=FAKE, source_repo=v2.source)
    run.release()
    run.spec = replace(run.spec, execution=BoatRun(BoatConfig.parse(config()), v2.base))
    (run.dir / 'run.json').write_text(json.dumps(run.spec.to_json()))
    run.execution = BoatExecution(run)
    key = WorkerKey(unit.run_id, 'author', v2.base)
    (run.dir / 'intents').mkdir(exist_ok=True)
    (run.dir / 'intents' / f'{key.name}.json').write_text('{}')
    binding = BoatBinding(run.execution._identity(key), 'vm-author', 'p1', '/home/user/fixture/p1',
                          'submitted', sandbox_id='bx_fixture')
    run.execution._store(key, binding)
    monkeypatch.setattr(BoatExecution, '_fetch', lambda *_: pytest.fail('queue observation contacted Boat'))
    monkeypatch.setattr(BoatExecution, '_reclaim', lambda *_: pytest.fail('queue observation reclaimed Boat'))
    if state == 'invalid_unit':
        (v2.home / 'units' / f'{unit.run_id}.json').write_text('{')
    elif state == 'interrupted':
        (run.dir / 'interrupt.request').write_text('{}')
    elif state in ('halted', 'released'):
        with intake.claims.locked(TICKET) as txn:
            txn.write(advance(txn.record, 'halt' if state == 'halted' else 'release'))
    else:
        v2.github.issues[TICKET]['labels'].remove('deepagents:queue')
    v2.github.add_issue(TICKET + 1, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    rows = intake.once(lambda _: pytest.fail('paused or unknown worker slot was reused'))
    expected = 'awaiting_human' if state == 'interrupted' else 'active'
    assert rows[TICKET]['phase'] == expected and 'running' in rows[TICKET]['reason']
    assert rows[TICKET + 1]['phase'] == 'queued'
    assert run.execution.load(key) == binding
    assert sorted(p.stem for p in (v2.home / 'units').glob('*.json')) == [unit.run_id]


@pytest.mark.parametrize('interrupt_at', [None, 'child_start', 'model_setup'])
def test_background_approval_recheck_preserves_operator_interrupt(queued, monkeypatch, interrupt_at):
    import sqlite3
    import subprocess

    from conftest import FAKE, MODELS
    from langchain_core.messages import AIMessage
    from langgraph.checkpoint.sqlite import SqliteSaver
    from test_agent import call, scripted

    from pilot1742 import agent as model_module
    from pilot1742 import executor as cli
    from pilot1742 import run as run_module

    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    task = cli.load_unit(v2.home, unit.run_id)[1]
    pilot = cli.open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                            launcher=FAKE, source_repo=v2.source)
    v2.github.issues[SPEC]['body'] += 'changed\n'
    v2.control({'author': {'write': {'demo/a.py': 'x = 1\n'}}})
    connection = sqlite3.connect(pilot.dir / 'graph.sqlite', check_same_thread=False)
    graph = model_module.build_agent(pilot, SqliteSaver(connection), scripted(
        call('run_worker', role='author', brief='实现'), AIMessage('完')))
    config = {'configurable': {'thread_id': unit.run_id}}
    try:
        graph.invoke({'messages': [('user', '开始')]}, config, durability='sync')
        assert graph.get_state(config).tasks[0].interrupts
        assert v2.claims() == []
    finally:
        pilot.release()
        connection.close()
    publish_ruling(v2)
    models_path = v2.tmp / 'pstack-models.md'
    models_path.write_text(MODELS)
    monkeypatch.setattr(run_module, 'PSTACK_MODELS', models_path)
    monkeypatch.setattr(model_module, 'PSTACK_MODELS', models_path)
    monkeypatch.setattr(cli, '__file__', str(v2.source / '.sandcastle/deepagents-pilot/pilot1742/executor.py'))
    monkeypatch.setattr(cli, 'ClaimStore', lambda: ClaimStore(v2.claims_root))
    monkeypatch.setattr(cli, 'SignedApprovals', lambda: v2.approvals)
    monkeypatch.setattr(cli, '_client', lambda _: v2.gateway().client)
    original_open = cli.open_v2_run
    opened = []

    def open_run(*args):
        result = original_open(*args, launcher=FAKE, source_repo=v2.source)
        opened.append(result)
        return result

    monkeypatch.setattr(cli, 'open_v2_run', open_run)
    pending_interrupt = interrupt_at

    def build_model(_model):
        nonlocal pending_interrupt
        if pending_interrupt == 'model_setup':
            pilot.request_interrupt('pytest-operator')
            pending_interrupt = None
        return scripted(AIMessage('完'))

    monkeypatch.setattr(model_module, 'build_model', build_model)
    common = ['--state-root', str(v2.home), '--drain-root', str(cutover.drain_root)]
    resources = ['--workers-log', str(v2.workers_log), '--private-key', str(v2.identity.private_key_path)]

    def invoke(args):
        try:
            return cli.main(args)
        finally:
            while opened:
                opened.pop().release()

    original_subprocess = subprocess.run
    children = []

    def child(command, *args, **kwargs):
        if command[1:3] != ['-m', 'pilot1742.executor']:
            return original_subprocess(command, *args, **kwargs)
        children.append(unit.run_id)
        if interrupt_at == 'child_start':
            pilot.request_interrupt('pytest-operator')
        return subprocess.CompletedProcess(command, invoke(command[3:]))

    monkeypatch.setattr(subprocess, 'run', child)
    writes_before = v2.writes()
    assert invoke(common + ['serve', '--once'] + resources) == 0
    assert children == [unit.run_id]
    if interrupt_at is not None:
        assert pilot.interrupt_requested() is not None, 'background approval recheck erased the operator interrupt'
        assert v2.claims() == [] and v2.writes() == writes_before
        status = json.loads((v2.home / 'queue-status.json').read_text())[str(TICKET)]
        assert status['phase'] == 'awaiting_human' and 'operator interrupt' in status['reason']
        assert invoke(common + ['run', '--run-id', unit.run_id] + resources) == 0
        assert pilot.interrupt_requested() is not None and v2.claims() == []
        assert invoke(common + ['run', '--run-id', unit.run_id, '--resume'] + resources) == 0
        assert pilot.interrupt_requested() is None
    assert v2.claims() == [f'author-{v2.base}']
    assert invoke(common + ['run', '--run-id', unit.run_id, '--resume'] + resources) == 0
    assert v2.claims() == [f'author-{v2.base}']
    assert v2.claim_record()['run_id'] == unit.run_id


def _low_disk(monkeypatch, free):
    from types import SimpleNamespace

    from pilot1742 import run
    monkeypatch.setattr(run, 'DISPATCH_FREE_BYTES', 100)
    monkeypatch.setattr(run.shutil, 'disk_usage', lambda _path: SimpleNamespace(free=free))


def test_low_disk_leaves_ticket_queued_without_claim_then_dispatches_when_space_returns(queued, monkeypatch):
    v2, cutover, intake, signer = queued
    _low_disk(monkeypatch, 99)
    rows = intake.once(lambda _: pytest.fail('dispatched below the disk threshold'))
    assert rows[TICKET]['phase'] == 'queued' and '派工门槛' in rows[TICKET]['reason']
    assert json.loads((v2.home / 'queue-status.json').read_text())[str(TICKET)]['phase'] == 'queued'
    assert not (v2.claims_root / f'{TICKET}.json').exists() and v2.writes() == []
    _low_disk(monkeypatch, 100)
    seen = []
    intake.once(lambda unit: seen.append(unit.run_id) or 75)
    assert seen == [f't{TICKET}-1']


def test_low_disk_still_reconciles_delivered_run(delivered, monkeypatch):
    v2, intake, unit, head, run_dir = delivered
    (number,) = v2.github.pulls
    v2.github.merge_pull(number)
    _low_disk(monkeypatch, 0)
    rows = intake.once(lambda _: pytest.fail('merged run dispatched again'))
    assert rows[TICKET]['delivery_phase'] == 'merged' and rows[TICKET]['phase'] == 'awaiting_merge'


@pytest.mark.parametrize('bad', ['untrusted', 'same_author', 'evidence', 'digest', 'block', 'number', 'free_text'])
@pytest.mark.parametrize('effect', ['claim', 'dispatch', 'commit', 'push', 'pr', 'status', 'deliver'])
def test_ruling_refuses_every_boundary_without_signature_fallback(queued, bad, effect):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    ex = executor(v2, cutover, unit)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    if effect != 'claim':
        publish_ruling(v2)
        claim = ex.claim(run_dir)
    publish_ruling(v2)
    comment = v2.github.issues[SPEC]['comments'][-1]
    if bad == 'untrusted':
        v2.github.issues[SPEC]['comments'] = [comment]
        comment['user']['login'] = 'untrusted'
    elif bad == 'same_author':
        comment['body'] = comment['body'].replace('review-session', 'AUTHOR-SESSION')
    elif bad == 'evidence':
        comment['body'] = comment['body'].replace('/fixture/review.json', '')
    elif bad == 'digest':
        v2.github.issues[SPEC]['body'] += 'changed'
    elif bad == 'block':
        comment['body'] = comment['body'].replace('PASS', 'BLOCK')
    elif bad == 'number':
        comment['body'] = comment['body'].replace(f'SPEC：{SPEC}', 'SPEC：999')
    else:
        v2.github.issues[SPEC]['comments'] = [dict(comment, body='批准 通过 approved')]
    before = v2.writes()
    with pytest.raises(ApprovalRequired):
        if effect == 'claim':
            ex.claim(run_dir)
        else:
            with ex.action(effect, run_dir, claim.generation):
                pytest.fail('invalid ruling crossed boundary')
    assert before == v2.writes()
    assert not (run_dir / 'approvals/spec_authorization.request').exists()
    status = json.loads((run_dir / 'approvals/spec_authorization.status.json').read_text())
    assert 'request_nonce' not in status and 'sign_command' not in status


def test_low_disk_continues_reconciling_later_merged_ticket(delivered, monkeypatch):
    from types import SimpleNamespace

    from pilot1742 import run as module
    from pilot1742.executor import delivery_status
    v2, intake, unit, head, run_dir = delivered
    lower = TICKET - 1
    v2.github.add_issue(lower, body=f'Part of [SPEC #{SPEC}]', parent=SPEC,
                        labels=('ready-for-agent', 'deepagents:queue'))
    lower_unit = intake.register(lower)
    executor(v2, intake.cutover, lower_unit).claim(PilotRun.run_dir(v2.home, lower_unit.run_id))
    (pr,) = v2.github.pulls
    v2.github.merge_pull(pr)
    monkeypatch.setattr(module, 'DISPATCH_FREE_BYTES', 100)
    monkeypatch.setattr(module.shutil, 'disk_usage', lambda _: SimpleNamespace(free=99))
    for _ in range(2):
        rows = intake.once(lambda _: pytest.fail('low disk dispatched work'))
        assert rows[lower]['phase'] == 'queued'
        assert rows[TICKET]['delivery_phase'] == 'merged'
        assert delivery_status(run_dir)['phase'] == 'merged'


def test_current_ruling_overwrites_historical_pending_and_preserves_raw_spec(queued):
    from approval_signing.protocol import spec_target

    from pilot1742.approval import signed_spec, spec_ruling_digest
    v2, cutover, intake, signer = queued
    v2.github.issues[SPEC]['body'] = '# 中文\r\ndeepagents-scope: demo/\r\n \t'
    unit = intake.register(TICKET)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    folder = run_dir / 'approvals'
    folder.mkdir(parents=True)
    (folder / 'spec_authorization.status.json').write_text('{"status":"awaiting_human","request_nonce":"old"}')
    publish_ruling(v2)
    ex = executor(v2, cutover, unit)
    ex.claim(run_dir)
    body = v2.github.issues[SPEC]['body'].encode()
    frozen = signed_spec(run_dir, SPEC)
    assert frozen.body == body and frozen.target == spec_target(SPEC, body).encode()
    note = json.loads((run_dir / f'spec/{SPEC}.json').read_text())
    assert note['digest'] == spec_ruling_digest(body.decode()) and 'request_nonce' not in note
    rows = intake.once(lambda _: 75)
    assert rows[TICKET]['phase'] == 'active'


@pytest.mark.parametrize('candidate_required', [False, True])
def test_push_then_revocation_resumes_same_delivery_without_duplicate_upload(queued, candidate_required):
    from dataclasses import replace

    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run
    v2, cutover, intake, signer = queued
    unit = replace(intake.register(TICKET), candidate_approval=candidate_required)
    publish_ruling(v2)
    ex = executor(v2, cutover, unit)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, ex, MODELS, launcher=FAKE, source_repo=v2.source)
    try:
        head = v2.to_pass(run)
        if candidate_required:
            assert run.publish_candidate()['decision'] == 'awaiting_approval'
            assert v2.github.ref(unit.branch) is None
            signer.publish(run.dir / 'approvals/candidate_head.request', Decision.APPROVE)
        revoked = False
        def revoke(point):
            nonlocal revoked
            if point == 'after_push_before_receipt' and not revoked:
                revoked = True
                publish_ruling(v2, 'BLOCK')
        ex.crash = revoke
        assert run.publish_candidate()['decision'] == 'awaiting_approval'
        assert revoked and v2.github.ref(unit.branch) == head and not v2.github.pulls
        before = v2.writes()
        assert run.publish_candidate()['decision'] == 'awaiting_approval'
        assert before == v2.writes()
        publish_ruling(v2)
        assert run.publish_candidate()['phase'] == 'awaiting_merge'
        assert v2.github.count('POST', r'.*/git/refs') == 1
        assert v2.github.count('POST', r'.*/pulls') == 1
    finally:
        run.release()


@pytest.mark.parametrize('label', ['needs-approval', 'ready-for-human', 'needs-info', 'needs-triage', 'wontfix'])
def test_typescript_release_and_python_intake_share_hold_policy(queued, label, wayfinder_engine_path):
    import subprocess
    v2, cutover, intake, signer = queued
    module = wayfinder_engine_path
    script = """const m = await import(process.argv[1]);
      let wrote = false;
      const body = 'fixture SPEC', specNumber = 7;
      const ruling = m.specRulingBody({specNumber, verdict:'PASS', author:'fixture-author',
        reviewer:'fixture-reviewer', digest:m.specBodyDigest(body), evidence:'/fixture/review.json'});
      const read = cmd => JSON.stringify(cmd.includes('/comments')
        ? [{user:{login:'fixture'}, body:ruling, created_at:'2026-11-07T00:00:00Z'}]
        : cmd.startsWith('issue view 7 ')
          ? {number:7, title:'SPEC: fixture', body, state:'OPEN'}
          : {title:'[impl] queue fixture', body:'', state:'OPEN', labels:[{name:process.argv[2]}], assignees:[]});
      m.releaseTicket(4242, {specs:[specNumber], orchestrator:'fixture'}, {read,
        write: () => {wrote = true; return ''; }}, 'deepagents:queue');
      console.log(JSON.stringify(wrote));"""
    result = subprocess.run(['node', '--experimental-strip-types', '--input-type=module', '-e', script,
                             module.as_uri(), label], text=True, capture_output=True, check=True)
    allowed = label == 'needs-approval'
    assert json.loads(result.stdout) == allowed
    v2.github.issues[TICKET]['labels'].append(label)
    if allowed:
        unit = intake.register(TICKET)
        with pytest.raises(ApprovalRequired):
            executor(v2, cutover, unit).claim(PilotRun.run_dir(v2.home, unit.run_id))
        publish_ruling(v2)
        assert executor(v2, cutover, unit).claim(PilotRun.run_dir(v2.home, unit.run_id)).state == 'active'
    else:
        with pytest.raises(AdmissionRefused):
            intake.register(TICKET)
        assert not v2.writes()


@pytest.mark.parametrize('body', ['# SPEC\ndeepagents-scope: other/\n', '# SPEC\n'])
def test_claim_halted_when_authorized_scope_changes(queued, body):
    v2, cutover, intake, _ = queued
    unit = intake.register(TICKET)
    publish_ruling(v2)
    ex = executor(v2, cutover, unit)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    claim = ex.claim(run_dir)
    v2.github.issues[SPEC]['body'] = body
    publish_ruling(v2)
    before = v2.writes()
    with pytest.raises(Halted):
        with ex.action('dispatch', run_dir, claim.generation):
            pytest.fail('scope change must fence dispatch')
    assert v2.claim_record()['state'] == 'halted'
    assert v2.writes() == before


@pytest.mark.parametrize('body', ['# SPEC\ndeepagents-scope: other/\n', '# SPEC\n'])
def test_restart_persists_scope_fence(queued, body):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run
    v2, cutover, intake, _ = queued
    unit = intake.register(TICKET)
    publish_ruling(v2)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                      launcher=FAKE, source_repo=v2.source)
    run.release()
    v2.github.issues[SPEC]['body'] = body
    publish_ruling(v2)
    before = v2.writes()
    with pytest.raises((AdmissionRefused, QueueWaiting, Halted)):
        open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                    launcher=FAKE, source_repo=v2.source)
    assert v2.claim_record()['state'] == 'halted'
    assert v2.writes() == before


def test_restart_pending_ruling_keeps_claim_and_resumes(queued):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run
    v2, cutover, intake, _ = queued
    unit = intake.register(TICKET)
    publish_ruling(v2)
    task = load_unit(v2.home, unit.run_id)[1]
    def reopen():
        return open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                           launcher=FAKE, source_repo=v2.source)
    reopen().release()
    publish_ruling(v2, 'BLOCK')
    before = v2.writes()
    with pytest.raises(ApprovalRequired):
        reopen()
    assert v2.claim_record()['state'] == 'active'
    publish_ruling(v2)
    reopen().release()
    assert v2.writes() == before


def test_low_disk_restart_collects_submitted_boat_without_new_dispatch(queued, monkeypatch):
    from dataclasses import replace
    from pathlib import Path

    from conftest import FAKE, MODELS
    from test_boat import FakeBoat, config, payload

    from pilot1742.boat_execution import BoatBinding, BoatConfig, BoatExecution, BoatRun
    from pilot1742.domain import Terminal, WorkerKey
    from pilot1742.executor import load_unit, open_v2_run
    from pilot1742.run import InsufficientSpace
    v2, cutover, intake, _ = queued
    unit = intake.register(TICKET)
    publish_ruling(v2)
    task = load_unit(v2.home, unit.run_id)[1]
    run = open_v2_run(v2.home, unit, task, executor(v2, cutover, unit), MODELS,
                      launcher=FAKE, source_repo=v2.source)
    run.spec = replace(run.spec, execution=BoatRun(BoatConfig.parse(config()), v2.base))
    (run.dir / 'run.json').write_text(json.dumps(run.spec.to_json()))
    run.execution = BoatExecution(run)
    key = WorkerKey(unit.run_id, 'author', v2.base)
    (run.dir / 'intents').mkdir(exist_ok=True)
    (run.dir / 'intents' / f'{key.name}.json').write_text('{}')
    binding = BoatBinding(run.execution._identity(key), 'vm-author', 'p1', '/home/user/fixture/p1',
                          'submitted', sandbox_id='bx_fixture')
    run.execution._store(key, binding)
    package = run.dir / 'boat/packages/p1'
    package.mkdir(parents=True)
    (package / 'request.json').write_text(json.dumps({'spec': run.signed_spec().target}))
    receipt = payload(run, run.execution, key, [dict(path='demo/recovered', mode='100644', content='b2s=')])
    run.release()
    stops = []
    def scp(source, target):
        Path(target).write_text(json.dumps(receipt))
        return []
    cli = FakeBoat(info=lambda: {'state': 'stopped' if stops else 'running'}, scp=scp,
                   stop=lambda *a: stops.append(a) or [{'operation': {'id': 'stop_1'}}],
                   exec=lambda command: {} if command.startswith('rm -f') else
                   {'identity': binding.identity, 'package_id': 'p1', 'state': 'receipt'})
    _low_disk(monkeypatch, 0)
    seen = []
    def child(current):
        seen.append(current.run_id)
        recovered = open_v2_run(v2.home, current, task, executor(v2, cutover, current), MODELS,
                                launcher=FAKE, source_repo=v2.source)
        try:
            recovered.execution.cli = cli
            assert isinstance(recovered.execution.observe(key), Terminal)
            assert (Path(recovered.spec.author_checkout) / 'demo/recovered').read_text() == 'ok'
            assert recovered.execution.load(key).stop_confirmed
            with pytest.raises(InsufficientSpace):
                recovered.execution.submit(WorkerKey(unit.run_id, 'review', v2.base))
        finally:
            recovered.release()
        return 75
    intake.once(child)
    assert seen == [unit.run_id], 'low disk must not block recovery of submitted workers'
    before = list(cli.calls)
    intake.once(child)
    assert len(stops) == 1 and cli.calls == before, 'terminal adoption must not repeat remote work'


@pytest.mark.parametrize('remote_running', [False, True])
def test_low_disk_unclaimed_boat_adopts_only_existing_command(env, monkeypatch, remote_running):
    from test_boat import history_engine

    from pilot1742.domain import Running
    from pilot1742.run import InsufficientSpace
    run, engine, key, binding, launches = history_engine(env, 'submitting', [])
    if remote_running:
        engine.cli.handlers['history'] = [{'commands': [{
            'status': 'running', 'commandId': 'original', 'processId': 5,
            'command': engine._command(binding, 'submit')}]}]
    _low_disk(monkeypatch, 0)
    try:
        if remote_running:
            assert isinstance(engine.observe(key), Running)
            assert engine.load(key).command_id == 'original'
            engine.submit(key)
        else:
            with pytest.raises(InsufficientSpace):
                engine.observe(key)
        assert launches == []
    finally:
        run.release()


def test_low_disk_existing_checkout_cannot_start_fresh_worker(env, monkeypatch):
    from pilot1742.run import InsufficientSpace
    run = env.create()
    _low_disk(monkeypatch, 0)
    try:
        with pytest.raises(InsufficientSpace):
            run.run_worker('author', 'implement')
        assert env.claims() == [] and not (run.dir / 'intents').exists()
    finally:
        run.release()


def engine_hold(v2):
    """引擎迁移旧票时的持有判定：真实 withTicketHold 读本机持有记录，migrationHold 读当前票面。"""
    import subprocess
    from pathlib import Path
    root = Path(__file__).resolve().parents[3]
    issue = v2.github.issues[TICKET]
    view = {'state': issue['state'].upper(), 'labels': [{'name': n} for n in issue['labels']],
            'assignees': issue.get('assignees') or []}
    script = '''const e = await import(process.argv[1]); const c = await import(process.argv[2]);
      const why = await c.withTicketHold(Number(process.argv[3]),
        claim => e.migrationHold(JSON.parse(process.argv[4]), claim), process.argv[5]);
      console.log(JSON.stringify(why));'''
    out = subprocess.run(['node', '--experimental-strip-types', '--input-type=module', '-e', script,
                          (root / 'scripts' / 'wayfinder_engine.mts').as_uri(),
                          (root / '.sandcastle' / 'deepagents-cutover.mts').as_uri(),
                          str(TICKET), json.dumps(view), str(v2.claims_root)],
                         check=True, capture_output=True, text=True).stdout
    return json.loads(out.strip().splitlines()[-1])


def test_release_after_merge_ends_hold_on_github_and_is_not_redispatched(queued):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run, release_claim
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit), MODELS,
                      launcher=FAKE, source_repo=v2.source)
    v2.to_pass(run)
    pr = run.publish_candidate()['pr']['number']
    run.release()
    assert 'deepagents' in v2.github.issues[TICKET]['labels']
    assert '正由' in engine_hold(v2), 'awaiting_merge 期间引擎仍视为持有'
    v2.github.merge_pull(pr)
    assert intake.once(lambda _unit: pytest.fail('merged run restarted'))[TICKET]['delivery_phase'] == 'merged'
    assert '正由' in engine_hold(v2), '合入不等于释放'

    released = release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '已合入')
    assert released.state == 'released' and not released.mirror_label
    assert 'deepagents' not in v2.github.issues[TICKET]['labels'], '释放摘镜像，持有结束在 GitHub 上可见'
    assert 'deepagents:queue' in v2.github.issues[TICKET]['labels'], '释放只摘镜像'
    assert engine_hold(v2) is None, '真实释放之后引擎可迁移'
    again = release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '重试')
    assert again.state == 'released' and not again.mirror_label, '释放可重试'

    rows = intake.once(lambda _unit: pytest.fail('merged delivery re-dispatched after release'))
    assert rows[TICKET]['phase'] == 'awaiting_human' and '已合入' in rows[TICKET]['reason']
    assert len(list((v2.home / 'units').glob('*.json'))) == 1


def test_release_after_unobserved_merge_is_not_redispatched(queued):
    from conftest import FAKE, MODELS

    from pilot1742.executor import load_unit, open_v2_run, release_claim
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit), MODELS,
                      launcher=FAKE, source_repo=v2.source)
    v2.to_pass(run)
    v2.github.merge_pull(run.publish_candidate()['pr']['number'])
    run.release()
    release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '释放早于队列观察到合入')
    rows = intake.once(lambda _unit: pytest.fail('merged delivery re-dispatched after release'))
    assert rows[TICKET]['phase'] == 'awaiting_human' and '已合入' in rows[TICKET]['reason']


def test_release_keeps_mirror_until_drained_and_old_release_cannot_strip_new_generation(queued):
    import fcntl
    import os

    from pilot1742.claims import ClaimRefused
    from pilot1742.executor import release_claim
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run_dir = PilotRun.run_dir(v2.home, unit.run_id)
    assert executor(v2, cutover, unit).claim(run_dir).state == 'active'
    controller = os.open(run_dir / 'owner.lock', os.O_RDWR | os.O_CREAT, 0o600)
    fcntl.flock(controller, fcntl.LOCK_EX)
    try:
        pending = release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '主控退回')
        assert pending.state == 'released' and pending.mirror_label, '控制进程未排空：记 released 但镜像保留'
        assert 'deepagents' in v2.github.issues[TICKET]['labels']
        assert '镜像' in engine_hold(v2), '释放未完成时引擎继续等待'
    finally:
        os.close(controller)
    done = release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '主控退回')
    assert not done.mirror_label and 'deepagents' not in v2.github.issues[TICKET]['labels'], '排空后重试完成释放'
    assert engine_hold(v2) is None

    second = intake.register(TICKET)
    assert second.run_id == f't{TICKET}-2'
    second_dir = PilotRun.run_dir(v2.home, second.run_id)
    assert executor(v2, cutover, second).claim(second_dir).generation == 2, '当前 SPEC 已有 PASS 裁定，第二代领取不再等人'
    assert 'deepagents' in v2.github.issues[TICKET]['labels']
    before = v2.writes()
    with pytest.raises(ClaimRefused):
        release_claim(intake.claims, v2.gateway(), TICKET, unit.run_id, '旧释放重试')
    assert v2.writes() == before and 'deepagents' in v2.github.issues[TICKET]['labels'], '旧释放不碰新代次镜像'
    assert '正由' in engine_hold(v2)
