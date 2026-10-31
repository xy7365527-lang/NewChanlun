import json
import sqlite3
from dataclasses import asdict
from pathlib import Path

import pytest
from conftest import FAKE, MODELS, TASK, git
from langchain_core.messages import AIMessage
from langgraph.checkpoint.sqlite import SqliteSaver
from test_agent import call, scripted
from test_boat import config

from pilot1742 import run as run_module
from pilot1742.agent import build_agent, initial_message, system_prompt
from pilot1742.boat_execution import (
    BoatConfig,
    BoatExecution,
    new_execution,
    parse_execution,
)
from pilot1742.cloud_package import build_package
from pilot1742.domain import WorkerKey, parse_run_id
from pilot1742.run import PilotRun, RunSpec


def configure(tmp_path, monkeypatch, **changes):
    path = tmp_path / 'boat-config.json'
    path.write_text(json.dumps({**config(), **changes}))
    monkeypatch.setenv('PILOT_BOAT_CONFIG', str(path))
    monkeypatch.setattr(run_module, 'new_execution', new_execution)
    return path


@pytest.mark.parametrize('version', [1, 2])
def test_launcher_cannot_select_local_backend_for_new_production_run(env, tmp_path, monkeypatch, version):
    monkeypatch.delenv('PILOT_BOAT_CONFIG', raising=False)
    monkeypatch.setattr(run_module, 'new_execution', new_execution)
    if version == 1:
        create = lambda: PilotRun.create(env.home, parse_run_id('production'), TASK, env.base, MODELS,
                                         source_repo=env.source, launcher=FAKE)
    else:
        from v2env import V2Env
        (tmp_path / 'v2').mkdir()
        v2 = V2Env(tmp_path / 'v2')
        unit = v2.register()
        create = lambda: v2.run(unit)
    with pytest.raises(ValueError, match='Boat'):
        create()
    configure(tmp_path, monkeypatch)
    run = create()
    assert run.spec.to_json()['execution']['backend'] == 'boat'
    assert isinstance(run.execution, BoatExecution)
    assert not run._children
    run.release()


def test_execution_wire_parses_closed_variants_and_round_trips(env, tmp_path, monkeypatch):
    configure(tmp_path, monkeypatch)
    run = PilotRun.create(env.home, parse_run_id('typed'), TASK, env.base, MODELS, source_repo=env.source)
    raw = run.spec.to_json()
    assert RunSpec.from_json(raw).execution == run.spec.execution
    assert raw['execution'] == {'backend': 'boat', 'config': asdict(run.spec.execution.config), 'runtime_head': env.base}
    for execution in ({'backend': 'other'}, {'backend': 'test-launcher', 'config': {}},
                      {**raw['execution'], 'runtime_head': 'short'}, {**raw['execution'], 'unexpected': True}):
        with pytest.raises(ValueError):
            RunSpec.from_json({**raw, 'execution': execution})
    run.release()
    monkeypatch.delenv('PILOT_BOAT_CONFIG')
    resumed = PilotRun.create(env.home, parse_run_id('typed'), TASK, env.base, MODELS, source_repo=env.source)
    assert resumed.spec.to_json() == raw and isinstance(resumed.execution, BoatExecution)
    resumed.release()


@pytest.mark.parametrize('legacy', [None, {'backend': 'test-launcher'}])
def test_historical_run_resumes_in_place_without_current_boat_config(env, tmp_path, monkeypatch, legacy):
    original = env.create('historical')
    raw = original.spec.to_json()
    if legacy is None:
        raw.pop('execution', None)
    else:
        raw['execution'] = legacy
    path = original.dir / 'run.json'
    path.write_text(json.dumps(raw))
    persisted = path.read_bytes()
    original.release()
    configure(tmp_path, monkeypatch)
    monkeypatch.delenv('PILOT_BOAT_CONFIG')
    resumed = PilotRun.create(env.home, parse_run_id('historical'), TASK, env.base, MODELS,
                              source_repo=env.source, launcher=FAKE)
    assert resumed.execution is None and path.read_bytes() == persisted
    assert resumed.spec.to_json().get('execution') == legacy
    resumed.release()


def test_controller_and_worker_read_same_frozen_skills_after_source_and_checkout_drift(env, tmp_path, monkeypatch):
    skill = '.agents/skills/project/SKILL.md'
    frozen = {skill: '---\nname: project\ndescription: frozen project\n---\nPROJECT_FROZEN\n',
              'AGENTS.md': 'DOC_FROZEN\n', 'docs/agents/old.md': 'OLD_FROZEN\n'}
    for name, content in frozen.items():
        path = env.source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
    source_root = Path(__file__).resolve().parents[3]
    for name in ('package.json', 'package-lock.json', '.sandcastle/boat-worker.mts', '.sandcastle/codex-child.ts',
                 '.sandcastle/docker-isolation.mts', '.sandcastle/acceptance-dir.mts',
                 '.sandcastle/git-environment.ts', '.sandcastle/main-model.mts',
                 '.sandcastle/deepagents-pilot/pilot1742/boat_remote.py'):
        target = env.source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((source_root / name).read_bytes())
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'skills base')
    base = git(env.source, 'rev-parse', 'HEAD')
    pstack = tmp_path / 'pstack'
    entry = pstack / 'installed/skills/poteto-mode/SKILL.md'
    entry.parent.mkdir(parents=True)
    entry.write_text('---\nname: pstack-upstream-poteto-mode\ndescription: frozen\n---\nPSTACK_FROZEN\n')
    (pstack / 'upstream').mkdir()
    (pstack / 'upstream/LICENSE').write_text('LICENSE_FROZEN')
    (pstack / 'aliases.json').write_text('{}')
    rules = tmp_path / 'rules.md'
    rules.write_text('RULES_FROZEN')
    configure(tmp_path, monkeypatch, pstack_source=str(pstack), model_rules=str(rules))
    run = PilotRun.create(env.home, parse_run_id('frozen'), TASK, base, MODELS, source_repo=env.source)
    for root in (env.source, Path(run.spec.author_checkout)):
        (root / skill).write_text('PROJECT_DRIFT')
        (root / 'AGENTS.md').write_text('DOC_DRIFT')
        (root / 'docs/agents/old.md').unlink()
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'live source drift')
    roots = ['/workspace/repo/.agents/skills/project/SKILL.md',
             '/opt/newchanlun/pstack/installed/skills/poteto-mode/SKILL.md',
             '/project/AGENTS.md', '/project/docs/agents/old.md',
             '/opt/newchanlun/rules/pstack-models.md', '/opt/newchanlun/pstack/upstream/LICENSE']
    expected = ['PROJECT_FROZEN', 'PSTACK_FROZEN', 'DOC_FROZEN', 'OLD_FROZEN', 'RULES_FROZEN', 'LICENSE_FROZEN']
    db = SqliteSaver(sqlite3.connect(run.dir / 'graph.sqlite', check_same_thread=False))
    for index in range(2):
        model = scripted(*(call('read_file', file_path=path) for path in roots), AIMessage('done'))
        agent = build_agent(run, db, model)
        result = agent.invoke({'messages': [('user', 'read frozen files')]},
                              {'configurable': {'thread_id': f'frozen-{index}'}}, durability='sync')
        replies = [m.content for m in result['messages'] if getattr(m, 'name', None) == 'read_file']
        assert len(replies) == len(expected)
        assert all(value in reply for value, reply in zip(expected, replies, strict=True))
        entry.write_text('PSTACK_DRIFT')
        rules.write_text('RULES_DRIFT')
    assert 'RULES_FROZEN' in system_prompt(run) and 'RULES_DRIFT' not in system_prompt(run)
    assert '/opt/newchanlun/pstack/installed/skills/poteto-mode/SKILL.md' in initial_message(run)
    package = tmp_path / 'worker-package'
    worker = build_package(env.source, base, package, run.dir / 'boat/skill-bundle', pstack, rules)
    controller = json.loads((run.dir / 'boat/controller-context/context.json').read_text())
    assert controller['head'] == base and worker['project_skill_id'] == controller['id']
    assert (package / 'project-skills/project/SKILL.md').read_text() == frozen[skill]
    assert (package / 'pstack/installed/skills/poteto-mode/SKILL.md').read_text().endswith('PSTACK_FROZEN\n')
    changed = git(env.source, 'rev-parse', 'HEAD')
    key = WorkerKey(run.spec.run_id, 'review', changed)
    binding = run.execution._intent(key)
    (run.dir / 'intents').mkdir(exist_ok=True)
    prompt = run.dir / 'review.md'
    prompt.write_text('review candidate')
    (run.dir / 'intents' / f'{key.name}.json').write_text(json.dumps({
        'prompt_file': str(prompt), 'model': 'claude-opus-5-5', 'mode': 'review'}))
    package = run.execution._build(key, binding)
    manifest = json.loads((package / 'package.json').read_text())
    assert manifest['head'] == changed and manifest['execution_skill_head'] == base
    assert manifest['project_skill_id'] == controller['id']
    assert (package / 'project-skills/project/SKILL.md').read_text() == frozen[skill]
    assert (package / 'repo' / skill).read_text() == 'PROJECT_DRIFT'
    run.release()


@pytest.mark.parametrize('raw', [None, [], {'extra': 1}, {**config(), 'extra': 1},
                                  {**config(), 'endpoint': []}, {**config(), 'ttl': '1800'},
                                  {**config(), 'image': None}, {**config(), 'ttl': True}])
def test_malformed_boat_config_has_value_error_boundary(raw):
    with pytest.raises(ValueError):
        BoatConfig.parse(raw)


@pytest.mark.parametrize('raw', [[], 'boat', 1, {'backend': 'boat', 'config': config(), 'runtime_head': None}])
def test_malformed_execution_has_value_error_boundary(raw):
    with pytest.raises(ValueError):
        parse_execution(raw)


def test_boat_design_prompt_uses_cloud_root_with_custom_source(env, monkeypatch):
    from test_boat import adapter
    run, engine, key, _ = adapter(env)
    from dataclasses import replace
    engine.config = replace(engine.config, pstack_source='/custom/pstack')
    facts = run.facts(reconcile=False)
    prompt = run._prompt(WorkerKey(run.spec.run_id, 'design-a', env.base), 'design', facts)
    assert '/opt/newchanlun/pstack/installed/skills/architect/SKILL.md' in prompt
    assert '/Users/' not in prompt
    run.release()
