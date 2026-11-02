import base64
import json
import subprocess
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

import pytest
from conftest import FAKE, MODELS
from test_boat import FakeBoat, adapter, payload, prepared_engine, stamp
from test_boat_drain import assert_no_collection, stopped_worker
from test_queue import SPEC, authorize, executor
from test_queue import queued as queued
from v2env import TICKET, V2Env

from pilot1742 import boat_execution
from pilot1742 import run as run_module
from pilot1742.agent import build_tools
from pilot1742.boat_execution import BoatExecution, ExecutionUnknown, ForkIntent
from pilot1742.claims import ClaimRefused, ClaimStore
from pilot1742.domain import EffectUnknown, NotStarted
from pilot1742.executor import load_unit, open_v2_run, release_claim
from pilot1742.run import Halted


def cleanup_cli(remote):
    def execute(command):
        assert command.startswith('rm -f ') and 'test ! -e ' in command
        remote['key'] = False
        return {}

    def stop(sandbox):
        assert sandbox == 'bx_abc' and not remote['key']
        remote['running'] = False
        return [{'operation': {'id': 'stop_1'}}]

    return FakeBoat(info=lambda: {'state': 'running' if remote['running'] else 'stopped',
                                 'archiveAfter': stamp(1700)},
                    exec=execute, stop=stop)


def test_prepared_cancel_recovers_key_transfer_before_submitting_record(env, monkeypatch):
    remote = {'key': False, 'running': True}
    cli = cleanup_cli(remote)
    run, engine, key, _ = prepared_engine(env, monkeypatch, cli, 'prepared', 'bx_abc')
    store = engine._store

    def fail_submitting(key, binding):
        if binding.submission == 'submitting':
            raise OSError('fixture crash after key transfer')
        return store(key, binding)

    monkeypatch.setattr(engine, '_provision_key', lambda binding: remote.update(key=True))
    monkeypatch.setattr(engine, '_store', fail_submitting)
    with pytest.raises(OSError, match='after key transfer'):
        engine.submit(key)
    assert engine.load(key).submission == 'prepared' and remote['key']
    reopened = BoatExecution(run, cli)
    assert isinstance(reopened.cancel(key), NotStarted)
    assert not remote['key'] and not remote['running']
    assert reopened.load(key).stop_id == 'stop_1'
    calls = list(cli.calls)
    assert isinstance(reopened.cancel(key), NotStarted) and cli.calls == calls
    with pytest.raises(ExecutionUnknown):
        reopened.submit(key)
    assert cli.calls == calls
    run.release()


@pytest.mark.parametrize('submission', ['creating', 'prepared'])
def test_prelaunch_cleanup_unknown_does_not_release_resources(env, submission):
    remote = {'key': True, 'running': True}
    cli = cleanup_cli(remote)

    def fail(command):
        raise ExecutionUnknown('fixture key cleanup failed')

    cli.handlers['exec'] = fail
    run, engine, key, binding = adapter(env, cli, submission)
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
    assert not engine.is_drained(key)
    assert remote['key'] and remote['running'] and engine.load(key) == replace(binding, cancel_requested=True)
    with pytest.raises(ExecutionUnknown):
        engine.submit(key)
    assert isinstance(engine.observe(key), EffectUnknown)
    assert not any(call[0] == 'stop' for call in cli.calls)
    run.release()


@pytest.mark.parametrize('submission', ['creating', 'prepared'])
def test_prelaunch_cancel_known_vm_does_not_require_uploaded_controller(env, submission):
    remote = {'key': True, 'running': True}
    cli = cleanup_cli(remote)
    run, engine, key, _ = adapter(env, cli, submission)
    assert isinstance(engine.cancel(key), NotStarted)
    assert not remote['key'] and not remote['running']
    assert engine.load(key).stop_id == 'stop_1'
    assert isinstance(engine.observe(key, reconcile=False), NotStarted)
    run.release()


@pytest.mark.parametrize('matches', [0, 1, 2])
def test_cancel_recovers_lost_fork_by_label_without_creating(env, matches):
    remote = {'key': False, 'running': True}
    cli = cleanup_cli(remote)
    run, engine, key, binding = adapter(env, cli, 'creating')
    engine._store(key, replace(binding, sandbox_id=None, create_requested=True,
                               fork_intent=ForkIntent('bx_base', 'same-key', '{}', 0)))
    cli.handlers['list'] = [{'sandboxes': [
        {'id': 'bx_abc', 'name': 'before-rename', 'labels': {'newchanlun.worker': binding.sandbox_name}}
        for _ in range(matches)]}]
    engine.fork_api = SimpleNamespace(fork=lambda _: pytest.fail('cancel created a VM'))
    fact = engine.cancel(key)
    if matches == 1:
        assert isinstance(fact, NotStarted)
        assert not remote['running'] and engine.load(key).sandbox_id == 'bx_abc'
    else:
        assert isinstance(fact, EffectUnknown) and remote['running']
        assert engine.load(key).sandbox_id is None
        assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
        assert not engine.is_drained(key)
    run.release()


def test_prelaunch_stopped_vm_does_not_imply_key_erasure(env):
    run, engine, key, binding = adapter(env, FakeBoat(info={'state': 'stopped'}), 'prepared')
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert engine.load(key) == replace(binding, cancel_requested=True) and not engine.is_drained(key)
    assert [call[0] for call in engine.cli.calls] == ['info']
    run.release()


def test_cancel_before_fork_request_is_not_started(env):
    run, engine, key, binding = adapter(env, FakeBoat(), 'creating')
    engine._store(key, replace(binding, sandbox_id=None))
    assert isinstance(engine.cancel(key), NotStarted)
    assert engine.cli.calls == []
    with pytest.raises(ExecutionUnknown):
        engine.submit(key)
    run.release()


def test_stop_ack_without_stopped_state_keeps_slot_until_reconciled(env):
    remote = {'key': True, 'running': True}
    cli = cleanup_cli(remote)
    cli.handlers['stop'] = [{'operation': {'id': 'stop_pending'}}]
    run, engine, key, _ = adapter(env, cli, 'prepared')
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert not remote['key'] and remote['running']
    assert engine.load(key).key_erased and engine.load(key).stop_id is None
    assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
    assert not engine.is_drained(key)
    remote['running'] = False
    assert isinstance(engine.cancel(key), NotStarted)
    assert sum(call[0] == 'stop' for call in cli.calls) == 1
    assert engine.load(key).stop_id and isinstance(engine.observe(key, reconcile=False), NotStarted)
    run.release()


def test_real_key_delete_failure_cannot_be_hidden_by_printf(env, tmp_path):
    remote = {'key': True, 'running': True}
    cli = cleanup_cli(remote)
    run, engine, key, binding = adapter(env, cli, 'prepared')
    binding = engine._store(key, replace(binding, remote_root=str(tmp_path / 'remote/p1')))
    Path(binding.key_file).mkdir(parents=True)

    def execute(command):
        result = subprocess.run(['/bin/sh', '-c', command], capture_output=True, text=True)
        if result.returncode:
            raise ExecutionUnknown('key cleanup command failed')
        remote['key'] = False
        return json.loads(result.stdout)

    cli.handlers['exec'] = execute
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert Path(binding.key_file).is_dir()
    assert not engine.load(key).key_erased and engine.load(key).stop_id is None
    assert not any(call[0] == 'stop' for call in cli.calls)
    run.release()



@pytest.mark.parametrize("state", ["stopping", "archiving"])
def test_prelaunch_stop_transition_keeps_slot_until_terminal(env, state):
    remote = {"key": True, "running": True, "state": "running"}
    cli = cleanup_cli(remote)
    cli.handlers["info"] = lambda: {"state": remote["state"]}

    def stop(sandbox):
        assert sandbox == "bx_abc" and not remote["key"]
        remote["state"] = state
        return [{"operation": {"id": "stop_pending"}}]

    cli.handlers["stop"] = stop
    run, engine, key, _ = adapter(env, cli, "prepared")
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert engine.load(key).key_erased and engine.load(key).stop_id is None
    assert isinstance(engine.observe(key, reconcile=False), EffectUnknown)
    assert not engine.is_drained(key)
    calls = list(cli.calls)
    assert isinstance(engine.cancel(key), EffectUnknown)
    assert cli.calls == calls + [("info",)]
    remote["state"] = "stopped" if state == "stopping" else "archived"
    assert isinstance(engine.cancel(key), NotStarted)
    assert isinstance(engine.observe(key, reconcile=False), NotStarted)
    assert sum(call[0] == "stop" for call in cli.calls) == 1
    run.release()


@pytest.mark.parametrize("has_vm", [True, False])
def test_prelaunch_cancelled_worker_resume_halts_without_redispatch(env, has_vm):
    remote = {"key": True, "running": True}
    cli = cleanup_cli(remote)
    run, engine, key, binding = adapter(env, cli, "prepared" if has_vm else "creating")
    if not has_vm:
        engine._store(key, replace(binding, sandbox_id=None))
    assert isinstance(engine.cancel(key), NotStarted)
    calls = list(cli.calls)
    with pytest.raises(Halted, match="取消"):
        run.run_worker("author", "恢复已取消工蜂")
    assert cli.calls == calls
    trail = [json.loads(line) for line in (run.dir / "trail.jsonl").read_text().splitlines()]
    assert trail[-1]["event"] == "halted" and trail[-1]["key"] == key.name
    assert "取消" in trail[-1]["reason"]
    assert isinstance(engine.observe(key, reconcile=False), NotStarted)
    run.release()



@pytest.mark.parametrize("stage", ["creating", "prepared", "lost_fork"])
def test_agent_status_resumes_preparation_through_dispatch_without_new_vm(env, monkeypatch, stage):
    cli = FakeBoat(info={"state": "running", "hydrated": True, "archiveAfter": stamp(1700)},
                   detach=[{"commandId": "cmd_resumed", "processId": 7}],
                   stop=[{"operation": {"id": "stop_resumed"}}])
    run, engine, key, _ = prepared_engine(env, monkeypatch, cli,
                                        "prepared" if stage == "prepared" else "creating", "bx_abc")
    binding = engine.load(key)
    if stage == "lost_fork":
        binding = engine._store(key, replace(binding, sandbox_id=None, create_requested=True))
        cli.handlers["list"] = [{"sandboxes": [{"id": "bx_abc", "name": binding.sandbox_name}]}]
    run._prepare_dispatch(key, "原任务", run.facts())
    (run.dir / "run.json").write_text(json.dumps(run.spec.to_json()))
    run.release()

    monkeypatch.setattr(boat_execution, "BoatCLI", lambda: cli)
    resumed = env.reopen()
    engine = resumed.execution
    monkeypatch.setattr(engine, "_build", lambda *_: resumed.dir / "pkg")
    provisioned = []
    monkeypatch.setattr(engine, "_provision_key", lambda b: provisioned.append(b.sandbox_id))
    engine.fork_api = SimpleNamespace(fork=lambda _: pytest.fail("恢复创建了新 VM"))
    files = [{"path": "demo/resumed.txt", "mode": "100644", "content": base64.b64encode(b"done").decode()}]
    receipt = payload(resumed, engine, key, files)
    receipt["record"]["result"]["status"] = "completed"
    cli.handlers["exec"] = lambda command: (
        {"identity": binding.identity, "package_id": binding.package_id, "state": "receipt"}
        if " observe " in command else {})

    def scp(source, target):
        if source.endswith("/claim/receipt.json"):
            Path(target).write_text(json.dumps(receipt))
        return []

    cli.handlers["scp"] = scp
    tools = {tool.name: tool for tool in build_tools(resumed)}
    before = list(cli.calls)
    state = json.loads(tools["run_status"].invoke({}))
    assert state["phase"] == "needs_author"
    assert state["workers"][key.name]["fact"] == "not_started" and cli.calls == before
    monkeypatch.setattr(run_module.time, "sleep", lambda _: None)
    result = json.loads(tools["run_worker"].invoke({"role": "author", "brief": "恢复原任务"}))
    assert result["ok"], result
    assert (Path(resumed.spec.author_checkout) / "demo/resumed.txt").read_text() == "done"
    assert provisioned == ["bx_abc"] and engine.load(key).sandbox_id == "bx_abc"
    assert sum(call[0] == "detach" for call in cli.calls) == 1
    trail = [json.loads(line) for line in (resumed.dir / "trail.jsonl").read_text().splitlines()]
    assert any(row["event"] == "adopt" and row["key"] == key.name for row in trail)
    assert not any(row["event"] == "halted" for row in trail)
    resumed.release()


PRELAUNCH_RESOURCES = ["pending_vm", "pending_fork", "cancel_pending", "cleaned", "unallocated"]


def stage_prelaunch_resources(run, monkeypatch, submission, case):
    engine, key, binding, cli = stopped_worker(run, monkeypatch)
    binding = replace(binding, submission=submission, stop_id=None)
    if case in ("pending_fork", "unallocated"):
        binding = replace(binding, sandbox_id=None, create_requested=case == "pending_fork")
    if case in ("cancel_pending", "cleaned"):
        binding = replace(binding, cancel_requested=True)
    if case == "cleaned":
        binding = replace(binding, stop_id="stop_original", key_erased=True)
    engine._store(key, binding)
    return engine, key, cli


@pytest.mark.parametrize("submission", ["creating", "prepared"])
@pytest.mark.parametrize("case", PRELAUNCH_RESOURCES)
def test_preparation_resources_block_generation_until_drained(tmp_path, monkeypatch, submission, case):
    v2 = V2Env(tmp_path)
    unit = v2.register()
    run = v2.run(unit)
    engine, key, cli = stage_prelaunch_resources(run, monkeypatch, submission, case)
    assert engine.is_drained(key) == (case == "cleaned" or (case == "unallocated" and submission == "creating"))
    release_claim(ClaimStore(v2.claims_root), TICKET, unit.run_id, "controller released")
    next_unit = v2.register(f"t{TICKET}-2")
    if case == "cleaned" or (case == "unallocated" and submission == "creating"):
        second = v2.run(next_unit)
        assert second.generation == 2
        second.release()
    else:
        with pytest.raises(ClaimRefused, match="Boat"):
            v2.run(next_unit)
        assert v2.claim_record()["generation"] == 1
    assert_no_collection(run, key, cli)


@pytest.mark.parametrize("submission", ["creating", "prepared"])
@pytest.mark.parametrize("case", PRELAUNCH_RESOURCES)
def test_preparation_resources_keep_queue_slot_until_drained(queued, monkeypatch, submission, case):
    v2, cutover, intake, signer = queued
    unit = intake.register(TICKET)
    authorize(v2, cutover, unit, signer)
    run = open_v2_run(v2.home, unit, load_unit(v2.home, unit.run_id)[1], executor(v2, cutover, unit),
                      MODELS, launcher=FAKE, source_repo=v2.source)
    engine, key, cli = stage_prelaunch_resources(run, monkeypatch, submission, case)
    release_claim(intake.claims, TICKET, unit.run_id, "controller released")
    v2.github.issues[TICKET]["labels"].remove("deepagents:queue")
    v2.github.add_issue(TICKET + 1, body=f"Part of [SPEC #{SPEC}]", parent=SPEC,
                        labels=("ready-for-agent", "deepagents:queue"))
    seen = []
    rows = intake.once(lambda item: seen.append(item.ticket) or 0)
    if case == "cleaned" or (case == "unallocated" and submission == "creating"):
        assert seen == [TICKET + 1]
    else:
        assert seen == []
        assert rows[TICKET]["phase"] == "active"
        assert rows[TICKET + 1]["phase"] == "queued"
    assert_no_collection(run, key, cli)
