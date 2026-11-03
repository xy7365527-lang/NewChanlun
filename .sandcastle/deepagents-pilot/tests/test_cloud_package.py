import os
import shlex
import subprocess

import pytest
from conftest import git

from pilot1742.boat_execution import _at_base
from pilot1742.cloud_package import build_project_context, export_repository


def vm_git(root, *args):
    env = {key: os.environ[key] for key in ('PATH', 'LANG') if key in os.environ}
    env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
    return subprocess.run(['git', '-C', str(root), *args], env=env,
                          check=True, capture_output=True).stdout


@pytest.mark.parametrize('scope', ['local', 'global', 'system', 'environment'])
@pytest.mark.parametrize('exporter', ['repository', 'context'])
def test_frozen_exports_do_not_execute_host_filters(env, tmp_path, monkeypatch, scope, exporter):
    skill = '.agents/skills/project/SKILL.md'
    contents = {'filtered': b'hello\n', 'AGENTS.md': b'project rules\n', skill: b'project skill\n'}
    for name, data in contents.items():
        path = env.source / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    (env.source / '.gitattributes').write_text('* filter=fx\n')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'filtered content')
    head = git(env.source, 'rev-parse', 'HEAD')
    marker = tmp_path / 'filter-executed'
    command = f'printf invoked >> {shlex.quote(str(marker))}; tr a-z A-Z'
    monkeypatch.setenv('GIT_CONFIG_GLOBAL', str(tmp_path / 'global.config'))
    monkeypatch.setenv('GIT_CONFIG_SYSTEM', str(tmp_path / 'system.config'))
    monkeypatch.delenv('GIT_CONFIG_NOSYSTEM', raising=False)
    if scope == 'environment':
        monkeypatch.setenv('GIT_CONFIG_COUNT', '1')
        monkeypatch.setenv('GIT_CONFIG_KEY_0', 'filter.fx.smudge')
        monkeypatch.setenv('GIT_CONFIG_VALUE_0', command)
    else:
        git(env.source, 'config', '--' + scope, 'filter.fx.smudge', command)
    destination = tmp_path / 'export'
    if exporter == 'repository':
        export_repository(env.source, head, destination)
        snapshot = destination
    else:
        build_project_context(env.source, head, destination, tmp_path / 'pstack', tmp_path / 'models')
        snapshot = destination / 'repo'
    assert not marker.exists(), 'frozen export executed a host filter'
    for name in contents if exporter == 'repository' else ('AGENTS.md', skill):
        assert (snapshot / name).read_bytes() == contents[name]
    if exporter == 'repository':
        assert vm_git(destination, 'rev-parse', 'HEAD').decode().strip() == head
        assert vm_git(destination, 'status', '--porcelain') == b''
        assert vm_git(destination, 'show', 'HEAD:filtered') == contents['filtered']
        assert vm_git(destination, 'cat-file', '-t', env.base) == b'commit\n'
        assert vm_git(destination, 'show', f'{env.base}:README') == b'x\n'
        assert 'filter' not in (destination / '.git/config').read_text()
        (destination / 'filtered').write_bytes(b'changed\n')
        assert vm_git(destination, 'status', '--porcelain') == b' M filtered\n'
    else:
        assert (destination / 'project-skills/project/SKILL.md').read_bytes() == contents[skill]


def test_frozen_lfs_pointer_does_not_require_host_lfs_object(env, tmp_path):
    pointer = b'version https://git-lfs.github.com/spec/v1\noid sha256:' + b'a' * 64 + b'\nsize 125000000\n'
    (env.source / 'relations.jsonl').write_bytes(pointer)
    (env.source / '.gitattributes').write_text('*.jsonl filter=lfs diff=lfs merge=lfs -text\n')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'lfs pointer without local payload')
    head = git(env.source, 'rev-parse', 'HEAD')
    marker = tmp_path / 'lfs-started'
    git(env.source, 'config', 'filter.lfs.process', f'touch {shlex.quote(str(marker))}; exit 1')
    git(env.source, 'config', 'filter.lfs.required', 'true')
    destination = tmp_path / 'export'
    export_repository(env.source, head, destination)
    assert not marker.exists()
    assert (destination / 'relations.jsonl').read_bytes() == pointer
    assert vm_git(destination, 'show', 'HEAD:relations.jsonl') == pointer
    assert vm_git(destination, 'status', '--porcelain') == b''


def test_export_linked_worktree_uses_shared_objects_not_local_attributes(env, tmp_path, monkeypatch):
    source = tmp_path / '源 repo'
    env.source.rename(source)
    worktree = tmp_path / 'linked worktree'
    git(source, 'worktree', 'add', '--detach', str(worktree), env.base)
    (source / '.git/info/attributes').write_text('README export-ignore\n')
    monkeypatch.setenv('GIT_DIR', str(tmp_path / 'unrelated.git'))
    monkeypatch.setenv('GIT_WORK_TREE', str(tmp_path / 'unrelated'))
    destination = tmp_path / 'export'
    export_repository(worktree, env.base, destination)
    assert (destination / 'README').read_bytes() == b'x\n'
    assert vm_git(destination, 'rev-parse', 'HEAD').decode().strip() == env.base
    assert vm_git(destination, 'status', '--porcelain') == b''
    assert not (destination / '.git/objects/info/alternates').exists()


@pytest.mark.parametrize('dirty', [False, True])
def test_host_directory_check_uses_lfs_clean_filter_after_stat_invalidation(env, tmp_path, monkeypatch, dirty):
    for key in tuple(os.environ):
        if key.startswith('GIT_'):
            monkeypatch.delenv(key)
    config = tmp_path / 'host.gitconfig'
    config.write_text('[filter "lfs"]\n'
                      '    clean = git-lfs clean -- %f\n'
                      '    smudge = git-lfs smudge -- %f\n'
                      '    process = git-lfs filter-process\n'
                      '    required = true\n')
    monkeypatch.setenv('HOME', str(tmp_path))
    monkeypatch.setenv('GIT_CONFIG_GLOBAL', str(config))
    monkeypatch.setenv('GIT_CONFIG_NOSYSTEM', '1')
    git(env.source, 'lfs', 'version')
    git(env.source, 'config', 'lfs.url', 'http://127.0.0.1:9/never')
    tracked = '.chanlun/block-topology/relations.jsonl'
    entity = env.source / tracked
    entity.parent.mkdir(parents=True)
    data = b'{"from":"a","to":"b"}\n' * 1000
    entity.write_bytes(data)
    (env.source / '.gitattributes').write_text('*.jsonl filter=lfs diff=lfs merge=lfs -text\n')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'real LFS entity')
    head = git(env.source, 'rev-parse', 'HEAD')
    pointer = git(env.source, 'show', f'{head}:{tracked}')
    assert pointer.startswith('version https://git-lfs.github.com/spec/v1\n')
    assert entity.read_bytes() == data
    assert git(env.source, 'status', '--porcelain') == ''
    if dirty:
        entity.write_bytes(data.replace(b'"b"', b'"c"'))
    before = entity.stat()
    os.utime(entity, ns=(before.st_atime_ns, before.st_mtime_ns + 2_000_000_000))
    assert entity.stat().st_mtime_ns != before.st_mtime_ns
    assert _at_base(env.source, head, '.chanlun/block-topology') is (not dirty)
    expected = f'M {tracked}' if dirty else ''
    assert git(env.source, 'status', '--porcelain') == expected
