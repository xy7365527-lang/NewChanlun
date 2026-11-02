import json
import re
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from conftest import git
from test_boat import adapter

from pilot1742.domain import parse_sha

ROOT = Path(__file__).resolve().parents[3]
RUNTIME = ("boat-worker.mts", "codex-child.ts", "docker-isolation.mts", "acceptance-dir.mts",
           "git-environment.ts", "main-model.mts")


@pytest.mark.parametrize(("scheduler", "missing_record"), [(False, False), (True, False), (True, True)])
def test_frozen_runtime_package_keeps_its_import_closure(env, tmp_path, scheduler, missing_record):
    paths = ["package.json", "package-lock.json", ".sandcastle/deepagents-pilot/pilot1742/boat_remote.py"]
    paths.extend(".sandcastle/" + name for name in RUNTIME)
    if scheduler:
        paths.extend((".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.json"))
    for name in paths:
        target = env.source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((ROOT / name).read_bytes() if scheduler or not name.endswith((".mts", ".ts"))
                           else b"export {};\n")
    if missing_record:
        (env.source / ".sandcastle/scheduler-config.json").unlink()
    project_skill = env.source / ".agents/skills/fixture/SKILL.md"
    project_skill.parent.mkdir(parents=True)
    project_skill.write_text("fixture")
    git(env.source, "add", ".")
    git(env.source, "commit", "-qm", "frozen runtime")
    env.base = parse_sha(git(env.source, "rev-parse", "HEAD"))
    if scheduler:
        (env.source / ".sandcastle/scheduler-config.json").write_text("{}")
    else:
        (env.source / ".sandcastle/scheduler-config.mts").write_text("throw new Error('not frozen');")
    pstack = tmp_path / "pstack"
    skill = pstack / "installed/skills/a/SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text("fixture")
    (pstack / "aliases.json").write_text("{}")
    (pstack / "upstream").mkdir()
    (pstack / "upstream/LICENSE").write_text("fixture")
    rules = tmp_path / "models.md"
    rules.write_text("fixture")
    run, engine, key, binding = adapter(env)
    engine.config = replace(engine.config, pstack_source=str(pstack), model_rules=str(rules))
    prompt = run.dir / "prompt.md"
    prompt.write_text("fixture")
    intent = run.dir / "intents" / f"{key.name}.json"
    intent.parent.mkdir(exist_ok=True)
    intent.write_text(json.dumps({"prompt_file": str(prompt), "model": str(run.spec.author_model), "mode": "execute"}))
    try:
        if missing_record:
            with pytest.raises(subprocess.CalledProcessError):
                engine._build(key, binding)
            assert not (run.dir / "boat/packages" / binding.package_id).exists()
            assert engine.cli.calls == []
            return
        package = engine._build(key, binding)
        runtime = package / "runtime/.sandcastle"
        for name in RUNTIME:
            assert (runtime / name).read_text() == git(env.source, "show", f"{env.base}:.sandcastle/{name}") + "\n"
        for module in runtime.iterdir():
            if module.suffix not in (".ts", ".mts"):
                continue
            for imported in re.findall(r'(?:from|import)\s*["\'](\./[^"\']+)["\']', module.read_text()):
                assert (runtime / imported).is_file(), (module.name, imported)
        if scheduler:
            result = subprocess.run(
                ["node", "--experimental-strip-types", "--input-type=module", "-e",
                 'import {readSchedulerConfig} from "./scheduler-config.mts";'
                 'console.log(JSON.stringify(readSchedulerConfig(undefined, {}).values));'],
                cwd=runtime, capture_output=True, text=True, timeout=10)
            assert result.returncode == 0, result.stderr
            expected = json.loads(git(env.source, "show", f"{env.base}:.sandcastle/scheduler-config.json"))
            assert json.loads(result.stdout) == expected["values"]
        else:
            assert not (runtime / "scheduler-config.mts").exists()
            assert not (runtime / "scheduler-config.json").exists()
        assert engine.cli.calls == []
        assert engine._build(key, binding) == package
    finally:
        run.release()
