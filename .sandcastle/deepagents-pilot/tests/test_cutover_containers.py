import json
import subprocess
from dataclasses import asdict, replace
from types import SimpleNamespace

import pytest

from pilot1742 import cutover, executor
from pilot1742.cutover import (
    CutoverError,
    CutoverStore,
    FrozenSnapshot,
    LegacyProcess,
    RemoteBarrier,
)


@pytest.fixture
def container_barrier(tmp_path, monkeypatch):
    drain = tmp_path / "drain"
    drain.mkdir()
    (drain / "state.json").write_text('{"v":1,"mode":"draining","generation":1}')
    log = tmp_path / "workers.jsonl"
    log.write_text("")
    store = CutoverStore(tmp_path / "state", drain, log)
    snap = FrozenSnapshot(1, "fixture", (), 0, str(log))
    snapshot = tmp_path / "snapshot.json"
    snapshot.write_text(json.dumps(asdict(snap)))
    legacy = tmp_path / "legacy-host"
    legacy.mkdir()
    monkeypatch.setattr(cutover, "LEGACY_HOST_REPO", legacy)
    remote = RemoteBarrier(True, "1", cutover.LEGACY_SERVICES)
    monkeypatch.setattr(executor, "legacy_processes", lambda: ())
    monkeypatch.setattr(executor, "remote_barrier", lambda _: remote)

    class Client:
        identity = SimpleNamespace(repository="owner/repo")
        bot_login = "test-app[bot]"

        def __enter__(self):
            return self

        def __exit__(self, *args):
            pass

    monkeypatch.setattr(executor, "_client", lambda _: Client())
    state = SimpleNamespace(rows=[], commands=[], failure=None, output=None,
                            ps='1 Mon Nov 2 01:02:03 2026 S\n', ps_failure=None,
                            snapshot_path=snapshot)
    original = subprocess.run

    def command(args, **kwargs):
        if args == ['ps', '-axo', 'pid=,lstart=,stat=']:
            if state.ps_failure == 'missing':
                raise FileNotFoundError('private path')
            if state.ps_failure == 'timeout':
                raise subprocess.TimeoutExpired(args, 30)
            return SimpleNamespace(returncode=1 if state.ps_failure == 'exit' else 0,
                                   stdout=state.ps, stderr='private stderr' if state.ps_failure == 'stderr' else '')
        if args[0] == "docker":
            state.commands.append(args)
            if state.failure == "missing":
                raise FileNotFoundError("private path")
            if state.failure == "timeout":
                raise subprocess.TimeoutExpired(args, 30)
            if state.failure == "daemon":
                return SimpleNamespace(returncode=1, stdout="private stdout", stderr="private stderr")
            output = "\n".join(row["id"] for row in state.rows)
            if args[1] == "inspect":
                output = state.output if state.output is not None else "\n".join(map(json.dumps, state.rows))
            return SimpleNamespace(returncode=0, stdout=output, stderr="")
        if args[0] == "git" and args[-2:] == ["rev-parse", "HEAD"]:
            return SimpleNamespace(returncode=0, stdout="a" * 40, stderr="")
        return original(args, **kwargs)

    monkeypatch.setattr(cutover.subprocess, "run", command)
    state.store, state.snap, state.remote, state.legacy = store, snap, remote, legacy

    def cli(operation):
        return executor.main(["--state-root", str(store.state_root), "--drain-root", str(drain),
                              "cutover", operation, "--workers-log", str(log),
                              "--snapshot", str(snapshot), "--private-key", str(tmp_path / "not-read.pem")])

    state.cli = cli
    return state


def container(source, *, name="/unlogged-author", running=True, writable=True):
    return {"id": "c" * 64, "name": name, "running": running,
            "mounts": [{"Type": "bind", "Source": str(source), "RW": writable}]}


@pytest.mark.parametrize("operation", ["verify", "record"])
@pytest.mark.parametrize("mount", ["root", "worktree", "alias", "ancestor", "ancestor-alias", "filesystem-root"])
@pytest.mark.parametrize("writable", [True, False])
def test_unlogged_container_blocks_cutover(container_barrier, tmp_path, operation, mount, writable):
    env = container_barrier
    source = env.legacy
    if mount == "worktree":
        source = source / ".sandcastle/worktrees/issue-1"
    elif mount in ("alias", "ancestor-alias"):
        source = tmp_path / "alias"
        target = env.legacy if mount == "alias" else env.legacy.parent
        source.symlink_to(target, target_is_directory=True)
    elif mount == "ancestor":
        source = env.legacy.parent
    elif mount == "filesystem-root":
        source = env.legacy.parents[-1]
    env.rows = [container(source, writable=writable)]
    assert env.cli(operation) != 0
    assert not env.store.path.exists()
    assert env.store.workers_log.read_text() == ""
    assert env.commands
    assert all("Env" not in " ".join(command) for command in env.commands)


@pytest.mark.parametrize("name", ["/sandcastle-new-review", "/a9-repair1742"])
@pytest.mark.parametrize("source_name", ["new-execution", "legacy-host-backup", "legacy"])
@pytest.mark.parametrize("writable", [True, False])
def test_unrelated_container_name_does_not_block(container_barrier, tmp_path, name, source_name, writable):
    env = container_barrier
    env.rows = [container(tmp_path / source_name, name=name, writable=writable)]
    assert env.cli("record") == 0
    assert env.store.load().engine_head == "a" * 40


def test_stopped_container_cannot_be_a_live_legacy_writer(container_barrier):
    env = container_barrier
    env.rows = [container(env.legacy, running=False)]
    assert env.cli("record") == 0


@pytest.mark.parametrize("failure", ["missing", "timeout", "daemon"])
def test_unobservable_docker_refuses_without_exposing_output(container_barrier, capsys, failure):
    env = container_barrier
    env.failure = failure
    assert env.cli("record") != 0
    assert not env.store.path.exists()
    assert "private" not in capsys.readouterr().err


@pytest.mark.parametrize("output", ["", "{}", "[]", "not-json",
    '{"id":"' + "c" * 64 + '","name":"/old","running":true,"mounts":[{"Source":"relative"}]}'])
def test_invalid_container_inventory_refuses(container_barrier, output):
    env = container_barrier
    env.rows = [container(env.legacy)]
    env.output = output
    assert env.cli("record") != 0
    assert not env.store.path.exists()


def test_record_rechecks_after_prior_verify(container_barrier):
    env = container_barrier
    assert env.cli("verify") == 0
    env.rows = [container(env.legacy)]
    assert env.cli("record") != 0
    assert not env.store.path.exists()


def test_store_verify_owns_container_guard(container_barrier):
    env = container_barrier
    env.rows = [container(env.legacy)]
    with pytest.raises(CutoverError, match="legacy worker container still present"):
        env.store.verify(env.snap, env.remote, (), "a" * 40)


@pytest.mark.parametrize("operation", ["verify", "record"])
@pytest.mark.parametrize("mount", [{"Type": "tmpfs"}, {"Type": "tmpfs", "Source": ""}])
def test_unrelated_tmpfs_does_not_block_cutover(container_barrier, operation, mount):
    env = container_barrier
    env.rows = [container('/unrelated')]
    env.rows[0]['mounts'].append(mount)
    assert env.cli(operation) == 0


@pytest.mark.parametrize('operation', ['verify', 'record'])
@pytest.mark.parametrize('stat', ['S', 'Z'])
def test_snapshot_identity_blocks_cli_without_classified_writer(container_barrier, operation, stat, capsys):
    env = container_barrier
    snap = replace(env.snap, legacy_snapshot=(LegacyProcess(88, 'Mon Nov 2 01:02:03 2026', 'old-child'),))
    env.snapshot_path.write_text(json.dumps(asdict(snap)))
    env.ps = f'88 Mon Nov 2 01:02:03 2026 {stat}\n'
    assert env.cli(operation) != 0
    assert 'snapshot process still alive' in capsys.readouterr().err
    assert not env.store.path.exists()


def test_reused_pid_passes_verify_but_original_identity_blocks_record(container_barrier, capsys):
    env = container_barrier
    snap = replace(env.snap, legacy_snapshot=(LegacyProcess(88, 'Mon Nov 2 01:02:03 2026', 'old-child'),))
    env.snapshot_path.write_text(json.dumps(asdict(snap)))
    env.ps = '88 Mon Nov 2 01:02:04 2026 S\n'
    assert env.cli('verify') == 0
    env.ps = '88 Mon Nov 2 01:02:03 2026 S\n'
    assert env.cli('record') != 0
    assert 'snapshot process still alive' in capsys.readouterr().err
    assert not env.store.path.exists()


@pytest.mark.parametrize('failure', ['missing', 'timeout', 'exit', 'stderr'])
def test_process_sampling_failure_blocks_record(container_barrier, capsys, failure):
    env = container_barrier
    env.ps_failure = failure
    assert env.cli('record') != 0
    assert not env.store.path.exists()
    assert 'private' not in capsys.readouterr().err


@pytest.mark.parametrize('output', ['', 'garbled', '0 Mon Nov 2 01:02:03 2026 S\n',
    '1 Mon Nov 2 01:02:03 2026 S\n1 Mon Nov 2 01:02:04 2026 S\n'])
def test_invalid_process_inventory_blocks_record(container_barrier, output):
    env = container_barrier
    env.ps = output
    assert env.cli('record') != 0
    assert not env.store.path.exists()


@pytest.mark.parametrize('mount', [
    {'Type': 'bind'}, {'Type': 'bind', 'Source': ''}, {'Type': 'volume', 'Source': 'relative'},
    {'Source': '/unrelated'}, {'Type': 'future', 'Source': '/unrelated'},
    {'Type': 'tmpfs', 'Source': '/unexpected'}, {'Type': 'tmpfs', 'Source': None},
])
def test_tmpfs_does_not_hide_invalid_neighbor_mount(container_barrier, mount):
    env = container_barrier
    env.rows = [container('/unrelated')]
    env.rows[0]['mounts'] = [{'Type': 'tmpfs'}, mount]
    assert env.cli('record') != 0
    assert not env.store.path.exists()


@pytest.mark.parametrize('kind', ['bind', 'volume'])
def test_tmpfs_does_not_hide_host_mount(container_barrier, kind):
    env = container_barrier
    env.rows = [container(env.legacy)]
    env.rows[0]['mounts'][0]['Type'] = kind
    env.rows[0]['mounts'].insert(0, {'Type': 'tmpfs'})
    assert env.cli('record') != 0
    assert not env.store.path.exists()
