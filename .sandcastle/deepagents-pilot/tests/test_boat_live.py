import json
import os
import re
import sys
import tarfile
from pathlib import Path
from unittest.mock import Mock

import boat_live
from test_boat import FakeBoat

from pilot1742 import run as run_module
from pilot1742.boat_execution import new_execution
from pilot1742.run import Halted


def test_live_setup_dispatch_uploads_complete_frozen_runtime(tmp_path, monkeypatch, capsys):
    pstack = tmp_path / "pstack"
    skill = pstack / "installed/skills/poteto-mode/SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text("---\nname: pstack-upstream-poteto-mode\ndescription: fixture\n---\nfixture\n")
    (pstack / "upstream").mkdir()
    (pstack / "upstream/LICENSE").write_text("fixture")
    (pstack / "aliases.json").write_text("{}")
    rules = tmp_path / "models.md"
    rules.write_text(boat_live.MODELS)
    work = tmp_path / "live"
    monkeypatch.setenv("PILOT_BOAT_CONFIG", "")

    def fixture_execution(*, source_repo):
        config_path = Path(os.environ["PILOT_BOAT_CONFIG"])
        config = json.loads(config_path.read_text())
        config.update(pstack_source=str(pstack), model_rules=str(rules))
        config_path.write_text(json.dumps(config))
        return new_execution(source_repo=source_repo)

    monkeypatch.setattr(run_module, "new_execution", fixture_execution)
    monkeypatch.setattr(sys, "argv", ["boat_live.py", "setup", str(work), "bx_fixture", "sha256:" + "a" * 64])
    assert boat_live.main() == 0
    created = json.loads(capsys.readouterr().out)
    frozen_head = created["base"]
    for name in ("scheduler-config.mts", "scheduler-config.json"):
        (work / "source/.sandcastle" / name).write_text("live checkout drift\n")
    uploaded = []

    def capture_upload(archive, destination):
        assert destination.startswith("bx_fixture:")
        with tarfile.open(archive) as package:
            files = {member.name.removeprefix("./"): package.extractfile(member).read()
                     for member in package.getmembers() if member.isfile()}
        for name in ("scheduler-config.mts", "scheduler-config.json"):
            expected = boat_live.git(work / "source", "show", f"{frozen_head}:.sandcastle/{name}") + "\n"
            assert files[f"runtime/.sandcastle/{name}"] == expected.encode()
        for path, content in files.items():
            if not path.startswith("runtime/.sandcastle/") or Path(path).suffix not in (".ts", ".mts"):
                continue
            for imported in re.findall(r'(?:from|import)\s*["\'](\./[^"\']+)["\']', content.decode()):
                assert str(Path(path).parent / imported) in files, (path, imported)
        assert files["runtime/.sandcastle/boat-worker.mts"] == (
            boat_live.HERE / "fixtures/boat_fixture_worker.mts").read_bytes()
        assert "controller/boat_remote.py" in files
        request = json.loads(files["request.json"])
        assert json.loads(files[".complete.json"]) == request["identity"]
        uploaded.append(request)
        raise Halted("test stopped at remote upload boundary")

    cli = FakeBoat(limits=[{"canStart": True}], rename=[{}],
                   info={"state": "ready", "hydrated": True}, exec={}, scp=capture_upload)
    fork = Mock()
    fork.fork.return_value = "bx_fixture"
    monkeypatch.setattr(boat_live, "LoggedCLI", lambda _path: cli)
    monkeypatch.setattr(boat_live, "LoggedForkAPI", lambda _path: fork)
    monkeypatch.setattr(sys, "argv", ["boat_live.py", "dispatch", str(work), "author"])
    assert boat_live.main() == 2
    result = json.loads(capsys.readouterr().out)
    assert result == {"error": "Halted", "reason": "test stopped at remote upload boundary"}
    assert len(uploaded) == 1
    fork.fork.assert_called_once()
    assert not any(call[0] == "detach" for call in cli.calls)
