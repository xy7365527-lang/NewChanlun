import json
import subprocess
from dataclasses import asdict
from types import SimpleNamespace

import pytest

from pilot1742 import cutover, executor
from pilot1742.cutover import CutoverError, CutoverStore, FrozenSnapshot, RemoteBarrier


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
    state = SimpleNamespace(rows=[], commands=[], failure=None, output=None)
    original = subprocess.run

    def command(args, **kwargs):
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


def container(source, *, name="/unlogged-author", running=True):
    return {"id": "c" * 64, "name": name, "running": running, "mounts": [{"Source": str(source)}]}


@pytest.mark.parametrize("operation", ["verify", "record"])
@pytest.mark.parametrize("mount", ["root", "worktree", "alias"])
def test_unlogged_container_blocks_cutover(container_barrier, tmp_path, operation, mount):
    env = container_barrier
    source = env.legacy
    if mount == "worktree":
        source = source / ".sandcastle/worktrees/issue-1"
    elif mount == "alias":
        source = tmp_path / "alias"
        source.symlink_to(env.legacy, target_is_directory=True)
    env.rows = [container(source)]
    assert env.cli(operation) != 0
    assert not env.store.path.exists()
    assert env.store.workers_log.read_text() == ""
    assert env.commands
    assert all("Env" not in " ".join(command) for command in env.commands)


@pytest.mark.parametrize("name", ["/sandcastle-new-review", "/a9-repair1742"])
def test_unrelated_container_name_does_not_block(container_barrier, tmp_path, name):
    env = container_barrier
    env.rows = [container(tmp_path / "new-execution", name=name)]
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
