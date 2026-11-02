"""#1742：隔离目录的真实 append/锁；外部现场读取使用受控响应，不触生产。"""
import copy
import json
import os
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, replace
from types import SimpleNamespace

import pytest

from pilot1742 import audit_release as audit
from pilot1742.cutover import (
    LEGACY_SERVICES,
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    RemoteBarrier,
    worker_claims,
    worker_lifecycles,
    worker_rows,
)


def encode(rows):
    return (''.join(json.dumps(row) + '\n' for row in rows)).encode()


@pytest.fixture
def env(tmp_path, monkeypatch):
    repo = tmp_path / 'repo'
    log = repo / '.sandcastle/logs/workers.jsonl'
    log.parent.mkdir(parents=True)
    drain = tmp_path / 'drain'
    drain.mkdir()
    (drain / 'state.json').write_text('{"v":1,"mode":"draining","generation":3}')
    checkout = tmp_path / 'worktree'
    checkout.mkdir()
    rows = [
        {'ticket': 1240, 'branch': 'sandcastle/issue-1240', 'phase': 'claim', 'status': 'claimed'},
        {'ticket': 1240, 'branch': 'sandcastle/issue-1240', 'phase': 'implementer', 'status': 'started'},
    ]
    log.write_bytes(encode(rows))
    evidence = tmp_path / 'terminal.md'
    evidence.write_text('原退出码未记录；冻结后审计释放，保留工位。')
    store = CutoverStore(tmp_path / 'state', drain, log)
    remote = RemoteBarrier(True, '1', LEGACY_SERVICES)
    output = {
        'git': f'worktree {checkout}\0HEAD aaaaa\0branch refs/heads/sandcastle/issue-1240\0\0',
        'ps': '1 Mon Nov 2 01:02:03 2026 S\n',
        'lsof': 'p1\nn/unrelated\n',
        'docker': '',
    }
    commands = []
    def command(args):
        commands.append(args)
        return output[args[0]]
    monkeypatch.setattr(audit, '_command', command)
    monkeypatch.setattr(audit, 'LEGACY_HOST_REPO', repo)
    monkeypatch.setattr(audit, 'legacy_processes', lambda: ())
    monkeypatch.setattr(audit, 'remote_barrier', lambda _: remote)
    def fresh(disposition='abandoned_no_exit'):
        raw = log.read_bytes()
        snapshot = FrozenSnapshot(3, '2026-11-02T00:00:00Z', (), len(raw), str(log), audit.digest(raw))
        source = next(iter(worker_lifecycles(worker_rows(raw)).values()))
        key = (str(source['last_event']['ticket']), source['last_event'].get('branch', ''))
        request = {'v': 1, 'ticket': key[0], 'branch': key[1], 'source': copy.deepcopy(source),
                   'disposition': disposition, 'reason': '冻结后逐代次核对，保留旧产物及未知退出码。',
                   'evidence': [{'path': str(evidence), 'sha256': audit.digest(evidence.read_bytes())}]}
        return snapshot, request
    return SimpleNamespace(store=store, log=log, checkout=checkout, rows=rows, evidence=evidence,
                           remote=remote, output=output, commands=commands, fresh=fresh)


def release(env, snapshot=None, request=None):
    if snapshot is None:
        snapshot, request = env.fresh()
    return audit.audit_release(env.store, snapshot, request, object())


@pytest.mark.parametrize('ticket,branch,disposition,pid,exit_code', [
    (1240, 'sandcastle/issue-1240', 'abandoned_no_exit', None, None),
    (1394, 'sandcastle/issue-1394', 'abandoned_no_exit', None, None),
    (1488, '', 'returned', 53295, None),
    (1621, '', 'terminated', 52634, None),
    (1622, 'sandcastle/issue-1622-fix', 'returned', 54418, None),
    (9999, '', 'terminated', 55555, 143),
])
def test_exact_legacy_shapes_keep_original_and_unknown_exit(env, ticket, branch, disposition, pid, exit_code):
    rows = [{'ticket': ticket, 'branch': branch, 'phase': 'implementer', 'status': 'started',
             'checkout': str(env.checkout), 'pid': pid, 'exit_code': exit_code}]
    env.log.write_bytes(encode(rows))
    snap, request = env.fresh(disposition)
    original = env.log.read_bytes()
    record = release(env, snap, request)
    assert env.log.read_bytes().startswith(original)
    assert len(worker_rows(env.log.read_bytes())) == 2
    assert worker_claims(worker_rows(env.log.read_bytes())) == ()
    assert record['phase'] == 'reconciliation' and record['status'] == 'released'
    assert record['release']['original_exit_code'] == exit_code
    assert record['release']['source']['first_event'] == rows[0]
    assert record['release']['operator']['uid'] == os.getuid()
    assert record['release']['observed']['targets'] == [str(env.checkout)]
    assert not env.store.path.exists()  # 审计释放不生成切流准入证。
    assert env.checkout.exists()


def test_repeat_is_idempotent_and_different_disposition_rejected(env):
    snap, request = env.fresh()
    first = release(env, snap, request)
    raw = env.log.read_bytes()
    assert release(env, snap, request) == first
    assert env.log.read_bytes() == raw
    with pytest.raises(CutoverError, match='different prior'):
        release(env, snap, request | {'disposition': 'returned'})
    assert env.log.read_bytes() == raw


def test_two_concurrent_requests_append_once(env):
    snap, request = env.fresh()
    with ThreadPoolExecutor(2) as pool:
        results = list(pool.map(lambda _: release(env, snap, request), range(2)))
    assert results[0] == results[1]
    assert len(worker_rows(env.log.read_bytes())) == 3


@pytest.mark.parametrize('bad', ['open', 'generation', 'source', 'prefix', 'missing_digest', 'partial',
                                 'new_claim', 'reference', 'evidence', 'extra_field', 'disposition'])
def test_invalid_request_or_snapshot_does_not_append(env, bad):
    snap, request = env.fresh()
    if bad == 'open':
        (env.store.drain_root / 'state.json').write_text('{"v":1,"mode":"open","generation":3}')
    elif bad == 'generation':
        snap = replace(snap, drain_generation=4)
    elif bad == 'source':
        snap = replace(snap, workers_log_path=str(env.log) + '.other')
    elif bad == 'prefix':
        env.log.write_bytes(env.log.read_bytes().replace(b'1240', b'1241'))
    elif bad == 'missing_digest':
        snap = replace(snap, workers_log_sha256=None)
    elif bad == 'partial':
        with env.log.open('ab') as stream:
            stream.write(b'{"ticket":1240')
    elif bad == 'new_claim':
        with env.log.open('ab') as stream:
            stream.write(encode([env.rows[0]]))
    elif bad == 'reference':
        request['source']['last_index'] += 1
    elif bad == 'evidence':
        env.evidence.write_text('changed')
    elif bad == 'extra_field':
        request['processes_clear'] = True
    elif bad == 'disposition':
        request['disposition'] = 'completed'
    raw = env.log.read_bytes()
    with pytest.raises(CutoverError):
        release(env, snap, request)
    assert env.log.read_bytes() == raw


def test_unknown_disposition_cannot_erase_known_exit(env):
    env.rows[-1]['exit_code'] = 143
    env.log.write_bytes(encode(env.rows))
    snap, request = env.fresh()
    with pytest.raises(CutoverError, match='cannot replace'):
        release(env, snap, request)


def test_lifecycle_change_since_snapshot_needs_new_snapshot(env):
    snap, _ = env.fresh()
    with env.log.open('ab') as stream:
        stream.write(encode([dict(env.rows[-1], status='completed')]))
    _, request = env.fresh('returned')
    with pytest.raises(CutoverError, match='changed since'):
        release(env, snap, request)


def test_same_ticket_other_branch_and_blank_branch_are_independent(env):
    others = [dict(env.rows[-1], branch=''), dict(env.rows[-1], branch='sandcastle/issue-1240-fix')]
    env.log.write_bytes(encode(env.rows + others))
    snap, request = env.fresh()
    release(env, snap, request)
    pending = worker_lifecycles(worker_rows(env.log.read_bytes()))
    assert set(pending) == {('1240', ''), ('1240', 'sandcastle/issue-1240-fix')}


def test_released_then_new_generation_cannot_reuse_old_request(env):
    snap, request = env.fresh()
    release(env, snap, request)
    with env.log.open('ab') as stream:
        stream.write(encode([env.rows[-1]]))
    with pytest.raises(CutoverError, match='stale audit'):
        release(env, snap, request)
    assert ('1240', request['branch']) in worker_lifecycles(worker_rows(env.log.read_bytes()))


@pytest.mark.parametrize('bad', ['legacy_process', 'snapshot_process', 'pid', 'cwd', 'cwd_descendant',
                                 'container_mount', 'container_parent_mount', 'container_name',
                                 'remote_jobs', 'remote_variable', 'remote_service', 'probe_failure',
                                 'no_target', 'bad_pid', 'ps_empty', 'lsof_empty', 'docker_invalid'])
def test_live_or_unobservable_resources_refuse(env, monkeypatch, bad):
    env.rows[-1]['pid'] = 88
    env.log.write_bytes(encode(env.rows))
    snap, request = env.fresh()
    if bad == 'legacy_process':
        monkeypatch.setattr(audit, 'legacy_processes', lambda: (LegacyProcess(99, 'start', 'main.mts'),))
    elif bad == 'snapshot_process':
        snap = replace(snap, legacy_snapshot=(LegacyProcess(1, 'Mon Nov 2 01:02:03 2026', 'old'),))
    elif bad == 'pid':
        env.output['ps'] = '88 Mon Nov 2 01:02:03 2026 Z\n'
    elif bad in ('cwd', 'cwd_descendant'):
        env.output['lsof'] = 'p77\nn' + str(env.checkout) + ('/nested' if bad.endswith('descendant') else '') + '\n'
    elif bad.startswith('container_') or bad == 'docker_invalid':
        old = audit._command
        source = str(env.checkout.parent if bad == 'container_parent_mount' else env.checkout)
        container = {'id': 'c1', 'name': '/issue-1240' if bad == 'container_name' else '/other',
                     'running': True, 'mounts': [] if bad == 'container_name' else [{'Source': source}]}
        def command(args):
            env.commands.append(args)
            if args[:2] == ['docker', 'ps']:
                return 'c1\n'
            if args[:2] == ['docker', 'inspect']:
                return '{}' if bad == 'docker_invalid' else json.dumps(container)
            return old(args)
        monkeypatch.setattr(audit, '_command', command)
    elif bad.startswith('remote_'):
        remote = replace(env.remote, **{'remote_jobs': {'remote_jobs_idle': False},
                                        'remote_variable': {'remote_freeze_var': '0'},
                                        'remote_service': {'legacy_services_unloaded': ()}}[bad])
        monkeypatch.setattr(audit, 'remote_barrier', lambda _: remote)
    elif bad == 'probe_failure':
        def fail(_):
            raise CutoverError('observation failed')
        monkeypatch.setattr(audit, '_command', fail)
    elif bad == 'no_target':
        env.output['git'] = ''
    elif bad == 'bad_pid':
        env.rows[-1]['pid'] = True
        env.log.write_bytes(encode(env.rows))
        snap, request = env.fresh()
    elif bad == 'ps_empty':
        env.output['ps'] = ''
    elif bad == 'lsof_empty':
        env.output['lsof'] = ''
    original = env.log.read_bytes()
    with pytest.raises(CutoverError):
        release(env, snap, request)
    assert env.log.read_bytes() == original
    assert not any('Env' in str(c) for c in env.commands)


def test_unrelated_container_is_not_a_global_block(env, monkeypatch):
    original = audit._command
    def command(args):
        if args[:2] == ['docker', 'ps']:
            return 'c1'
        if args[:2] == ['docker', 'inspect']:
            return json.dumps({'id': 'c1', 'name': '/unrelated', 'running': True,
                               'mounts': [{'Source': '/elsewhere'}]})
        return original(args)
    monkeypatch.setattr(audit, '_command', command)
    assert release(env)['release']['observed']['container_count'] == 1


@pytest.mark.parametrize('change', ['append', 'replace', 'evidence', 'generation'])
def test_changed_during_observation_never_writes_release(env, monkeypatch, change):
    original = audit.observe
    def observe(*args):
        result = original(*args)
        if change == 'append':
            with env.log.open('ab') as stream:
                stream.write(encode([env.rows[-1]]))
        elif change == 'replace':
            other = env.log.with_suffix('.new')
            other.write_bytes(env.log.read_bytes())
            other.replace(env.log)
        elif change == 'evidence':
            env.evidence.write_text('edited')
        else:
            (env.store.drain_root / 'state.json').write_text('{"v":1,"mode":"draining","generation":4}')
        return result
    monkeypatch.setattr(audit, 'observe', observe)
    with pytest.raises(CutoverError):
        release(env)
    assert all(row['phase'] != 'reconciliation' for row in worker_rows(env.log.read_bytes()))


@pytest.mark.parametrize('when', ['before', 'after'])
def test_producer_race_never_clears_new_activity(env, monkeypatch, when):
    write = os.write
    extra = encode([dict(env.rows[-1], status='completed')])
    def racing_write(fd, payload):
        if when == 'before':
            write(fd, extra)
        result = write(fd, payload)
        if when == 'after':
            write(fd, extra)
        return result
    monkeypatch.setattr(audit.os, 'write', racing_write)
    with pytest.raises(CutoverError, match='stale|resumed'):
        release(env)
    rows = worker_rows(env.log.read_bytes())
    assert len(rows) == 4  # 不删除竞态原件，也不伪造回滚。
    with pytest.raises(CutoverError, match='open legacy'):
        worker_claims(rows)
    # 陈旧释放保留为审计，但不永久毒化流；真实关闭仍可正常归约。
    closed = dict(env.rows[-1], phase='sandbox', status='closed')
    assert worker_claims(rows + [closed]) == ()


@pytest.mark.parametrize('field,value', [
    ('operator', {}), ('observed', {}), ('evidence', []), ('v', True),
    ('drain_generation', True), ('original_exit_code', 0),
    ('snapshot', {}), ('disposition', 'success'),
])
def test_incomplete_or_forged_release_is_rejected_by_reducer(env, field, value):
    record = release(env)
    record['release'][field] = value
    with pytest.raises(CutoverError):
        worker_claims(env.rows + [record])


def test_valid_release_is_consumed_by_cutover_and_not_ticket_activity(env):
    snap, request = env.fresh()
    release(env, snap, request)
    record = env.store.verify(snap, env.remote, (), 'a' * 40)
    env.store.record(record)
    env.store.check_ticket(1240, env.store.load())
    with env.log.open('ab') as stream:
        stream.write(encode([dict(env.rows[-1], status='failed')]))
    with pytest.raises(CutoverError, match='open legacy'):
        env.store.check_ticket(1240, record)


def test_new_snapshots_and_cutover_record_preserve_prefix_integrity(env, monkeypatch):
    monkeypatch.setattr('pilot1742.cutover.legacy_processes', lambda: ())
    snap = env.store.snapshot()
    assert snap.workers_log_sha256 == audit.digest(env.log.read_bytes())
    _, request = env.fresh()
    release(env, snap, request)
    record = env.store.verify(snap, env.remote, (), 'a' * 40)
    env.store.record(record)
    env.log.write_bytes(env.log.read_bytes().replace(b'claimed', b'claimXd', 1))
    with pytest.raises(CutoverError, match='prefix changed'):
        env.store.verify(snap, env.remote, (), 'a' * 40)
    with pytest.raises(CutoverError, match='prefix changed'):
        env.store.check_ticket(1240, record)


def test_pending_cli_is_read_only_and_audit_cli_uses_real_entry(env, monkeypatch, capsys, tmp_path):
    from pilot1742 import executor
    def no_client(_):
        raise AssertionError('pending must not request credentials')
    monkeypatch.setattr(executor, '_client', no_client)
    common = ['--state-root', str(env.store.state_root), '--drain-root', str(env.store.drain_root),
              'cutover']
    assert executor.main(common + ['pending', '--workers-log', str(env.log)]) == 0
    pending = json.loads(capsys.readouterr().out)
    assert pending == audit.pending_releases(env.store)
    assert len(worker_rows(env.log.read_bytes())) == 2
    snap, request = env.fresh()
    snapshot_path, request_path = tmp_path / 'snapshot.json', tmp_path / 'request.json'
    snapshot_path.write_text(json.dumps(asdict(snap)))
    request_path.write_text(json.dumps(request))
    class Client:
        identity = SimpleNamespace(repository='owner/repo')
        bot_login = 'test-app[bot]'
        def __enter__(self):
            return self
        def __exit__(self, *args):
            pass
    monkeypatch.setattr(executor, '_client', lambda _: Client())
    assert executor.main(common + ['audit-release', '--workers-log', str(env.log),
                                  '--snapshot', str(snapshot_path), '--request', str(request_path),
                                  '--private-key', str(tmp_path / 'not-read.pem')]) == 0
    result = json.loads(capsys.readouterr().out)
    assert result['release']['source'] == request['source']
    assert worker_claims(worker_rows(env.log.read_bytes())) == ()


def test_legacy_record_without_digest_still_loads_and_retries(env):
    # 添加完整性字段不能使既有冻结记录的原入口失效。
    env.log.write_bytes(b'')
    snap = FrozenSnapshot(3, 'old-format', (), 0, str(env.log))
    record = env.store.verify(snap, env.remote, (), 'a' * 40)
    env.store.state_root.mkdir()
    payload = asdict(record)
    del payload['workers_log_sha256']
    env.store.path.write_text(json.dumps(payload))
    assert env.store.load() == record
    assert env.store.record(record) == record


def test_actual_git_inventory_and_ps_parser_inside_container(env, monkeypatch, tmp_path):
    import subprocess
    # 真实 Git 元数据查询和 ps（仅限测试容器），其余外部系统仍用响应夹具。
    repo = env.log.parents[2]
    subprocess.run(['git', 'init', '-q', str(repo)], check=True)
    subprocess.run(['git', '-C', str(repo), 'symbolic-ref', 'HEAD',
                    'refs/heads/sandcastle/issue-1240'], check=True)
    original = audit._command
    monkeypatch.setattr(audit, '_command', lambda args: subprocess.run(
        args, check=True, text=True, capture_output=True).stdout
        if args[0] in ('git', 'ps') else original(args))
    record = release(env)
    assert record['release']['observed']['targets'] == [str(repo)]
    assert record['release']['observed']['process_count'] > 0


def test_external_probe_fails_closed_without_echoing_output(monkeypatch):
    def failed(*args, **kwargs):
        return SimpleNamespace(returncode=1, stdout='private-stdout', stderr='private-stderr')
    monkeypatch.setattr(audit.subprocess, 'run', failed)
    with pytest.raises(CutoverError) as error:
        audit._command(['lsof', '-nP', '-d', 'cwd', '-Fpn'])
    assert str(error.value) == 'audit observation failed: lsof'


def test_audit_holds_same_exclusive_lock_as_freeze_and_resume(env, monkeypatch):
    import fcntl
    original = audit.observe
    def observe(*args):
        with (env.store.drain_root / 'admission.lock').open('a') as other, pytest.raises(BlockingIOError):
            fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)
        return original(*args)
    monkeypatch.setattr(audit, 'observe', observe)
    release(env)
    with (env.store.drain_root / 'admission.lock').open('a') as other:
        fcntl.flock(other, fcntl.LOCK_EX | fcntl.LOCK_NB)


def test_invalid_snapshot_digest_is_rejected_at_parse(env):
    snap, _ = env.fresh()
    with pytest.raises(CutoverError, match='invalid frozen snapshot'):
        replace(snap, workers_log_sha256='unchecked')


def test_last_duplicate_claim_is_the_only_releasable_generation(env):
    earlier = [env.rows[0], dict(env.rows[0], status='rolled_back')]
    env.log.write_bytes(encode(earlier + earlier + env.rows))
    snap, request = env.fresh()
    assert request['source']['first_index'] == 5
    record = release(env, snap, request)
    assert record['release']['source']['last_index'] == 6
    assert worker_claims(worker_rows(env.log.read_bytes())) == ()


def test_documented_request_recipe_generates_readable_request(env, tmp_path):
    import subprocess
    import sys
    from pathlib import Path
    doc = Path(__file__).resolve().parents[2] / 'docs/migration-drain-1742.md'
    body = doc.read_text().split('<<\'PY\'\n', 1)[1].split('\nPY\n', 1)[0]
    pending, output = tmp_path / 'pending.json', tmp_path / 'request.json'
    pending.write_text(json.dumps(audit.pending_releases(env.store)))
    subprocess.run([sys.executable, '-', str(pending), str(output), str(env.evidence)],
                   input=body, text=True, capture_output=True, check=True)
    request = audit.validate_request(json.loads(output.read_text()))
    snap, _ = env.fresh()
    assert release(env, snap, request)['release']['disposition'] == 'abandoned_no_exit'


@pytest.mark.parametrize('log_alias', ['host-symlink', 'canonical'])
def test_real_symlink_log_uses_independent_execution_host_inventory(env, monkeypatch, tmp_path, log_alias):
    import subprocess
    shared, host = env.log.parents[2], tmp_path / 'old-host'
    host.mkdir()
    for repo in (shared, host):
        subprocess.run(['git', 'init', '-q', str(repo)], check=True)
        subprocess.run(['git', '-C', str(repo), '-c', 'user.name=fixture',
                        '-c', 'user.email=fixture@example.invalid', 'commit', '-q',
                        '--allow-empty', '-m', 'fixture'], check=True)
    subprocess.run(['git', '-C', str(host), 'worktree', 'add', '-q', '-b',
                    'sandcastle/issue-1240', str(env.checkout), 'HEAD'], check=True)
    (host / '.sandcastle').mkdir()
    (host / '.sandcastle/logs').symlink_to(env.log.parent, target_is_directory=True)
    log = host / '.sandcastle/logs/workers.jsonl' if log_alias == 'host-symlink' else env.log
    env.store = CutoverStore(env.store.state_root, env.store.drain_root, log)
    monkeypatch.setattr(audit, 'LEGACY_HOST_REPO', host)
    original = audit._command
    def command(args):
        if args[0] == 'git':
            assert args[2] == str(host)
            return subprocess.run(args, check=True, text=True, capture_output=True).stdout
        return original(args)
    monkeypatch.setattr(audit, '_command', command)
    record = release(env)
    assert record['release']['observed']['targets'] == [str(env.checkout)]
    assert record['release']['snapshot']['workers_log_path'] == str(env.log)


def test_stale_well_formed_release_is_audit_only_and_can_be_reconciled_later(env):
    snap, request = env.fresh()
    first = release(env, snap, request)
    new_event = dict(env.rows[-1], status='failed', reason='later-stage')
    env.log.write_bytes(encode(env.rows + [new_event, first]))
    assert ('1240', request['branch']) in worker_lifecycles(worker_rows(env.log.read_bytes()))
    new_snap, new_request = env.fresh()
    second = release(env, new_snap, new_request)
    assert second['release']['source']['last_index'] == 3
    assert len(worker_rows(env.log.read_bytes())) == 5
    assert worker_claims(worker_rows(env.log.read_bytes())) == ()
    assert release(env, new_snap, new_request) == second


def test_snapshot_change_does_not_rewrite_prior_release(env):
    snap, request = env.fresh()
    release(env, snap, request)
    raw = env.log.read_bytes()
    newer = replace(snap, workers_log_length=len(raw), workers_log_sha256=audit.digest(raw))
    with pytest.raises(CutoverError, match='different prior audit'):
        release(env, newer, request)
    assert env.log.read_bytes() == raw


def test_trailing_unapplied_release_does_not_break_idempotent_receipt(env):
    snap, request = env.fresh()
    first = release(env, snap, request)
    with env.log.open('ab') as stream:
        stream.write(encode([first]))  # 合法结构，但原 pending 已由第一条清除。
    raw = env.log.read_bytes()
    assert worker_claims(worker_rows(raw)) == ()
    assert release(env, snap, request) == first
    assert env.log.read_bytes() == raw


def test_actual_managed_runner_classifier_blocks_audit(env, monkeypatch):
    from pilot1742.cutover import LEGACY_HOST_REPO, legacy_processes
    command = '/opt/homebrew/bin/node --import tsx /Volumes/AgentStorage/i4-managed-resume42z/runner-fix2.mts'
    def inventory(args, **kwargs):
        output = (f'99999 1 Mon Nov 2 01:02:03 2026 S {command}\n' if args[0] == 'ps'
                  else f'p99999\nn{LEGACY_HOST_REPO}\n')
        return SimpleNamespace(stdout=output, stderr='', returncode=0)
    monkeypatch.setattr('pilot1742.cutover.subprocess.run', inventory)
    monkeypatch.setattr(audit, 'legacy_processes', legacy_processes)
    original = env.log.read_bytes()
    with pytest.raises(CutoverError, match='legacy control process still alive'):
        release(env)
    assert env.log.read_bytes() == original
