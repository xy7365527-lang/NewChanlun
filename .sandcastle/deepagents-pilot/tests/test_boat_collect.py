import base64
import json
import os
from pathlib import Path

import pytest
from conftest import git
from test_boat import adapter, payload

from pilot1742 import boat_execution
from pilot1742.boat_execution import ExecutionUnknown
from pilot1742.domain import parse_sha


def directory_artifact(env):
    root = env.source / 'demo/pkg'
    (root / 'nested').mkdir(parents=True)
    (root / 'a.py').write_text('old a')
    (root / 'nested/旧 名.py').write_text('old nested')
    (root.parent / 'sibling.py').write_text('keep sibling')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'directory baseline')
    env.base = parse_sha(git(env.source, 'rev-parse', 'HEAD'))
    return [
        dict(path='demo/pkg', mode='100755', content=base64.b64encode(b'new executable\x00').decode()),
        dict(path='demo/pkg/a.py', mode='deleted'),
        dict(path='demo/pkg/nested/旧 名.py', mode='deleted'),
    ]


def file_artifact(env):
    (env.source / 'demo').mkdir()
    (env.source / 'demo/pkg').write_text('old file')
    git(env.source, 'add', '.')
    git(env.source, 'commit', '-qm', 'file baseline')
    env.base = parse_sha(git(env.source, 'rev-parse', 'HEAD'))
    return [dict(path='demo/pkg/nested/a.py', mode='100644', content='bmV3'),
            dict(path='demo/pkg', mode='deleted')]


def assert_collected(run, engine, key, receipt):
    assert engine.load(key).submission == 'collected'
    assert json.loads((run._key_dir(key) / 'result.json').read_text()) == receipt['record']
    assert engine.cli.calls == []


@pytest.mark.parametrize('reverse', [False, True])
def test_collect_directory_to_file_preserves_content_mode_and_retry(env, reverse):
    files = directory_artifact(env)
    if reverse:
        files.reverse()
    run, engine, key, _ = adapter(env)
    receipt = payload(run, engine, key, files)
    try:
        engine.collect(key, receipt)
        engine.collect(key, receipt)
        path = Path(run.spec.author_checkout) / 'demo/pkg'
        assert path.read_bytes() == b'new executable\x00'
        assert path.stat().st_mode & 0o111
        assert (path.parent / 'sibling.py').read_text() == 'keep sibling'
        assert_collected(run, engine, key, receipt)
    finally:
        run.release()


@pytest.mark.parametrize('stage', ['partial_delete', 'empty_directory', 'directory_removed', 'file_written'])
def test_collect_directory_to_file_resumes_each_partial_state(env, monkeypatch, stage):
    files = directory_artifact(env)
    run, engine, key, _ = adapter(env)
    receipt = payload(run, engine, key, files)
    checkout = Path(run.spec.author_checkout)
    root = checkout / 'demo/pkg'
    unlink, rmdir, replace = Path.unlink, Path.rmdir, os.replace

    def interrupted_unlink(path, *args, **kwargs):
        unlink(path, *args, **kwargs)
        if stage == 'partial_delete' and path == root / 'nested/旧 名.py':
            raise OSError('collect interrupted')

    def interrupted_rmdir(path, *args, **kwargs):
        if stage == 'empty_directory' and path == root:
            raise OSError('collect interrupted')
        rmdir(path, *args, **kwargs)
        if stage == 'directory_removed' and path == root:
            raise OSError('collect interrupted')

    def interrupted_replace(source, destination):
        replace(source, destination)
        if stage == 'file_written' and Path(destination) == root:
            raise OSError('collect interrupted')

    try:
        with monkeypatch.context() as patch:
            patch.setattr(Path, 'unlink', interrupted_unlink)
            patch.setattr(Path, 'rmdir', interrupted_rmdir)
            patch.setattr(boat_execution.os, 'replace', interrupted_replace)
            with pytest.raises(OSError, match='collect interrupted'):
                engine.collect(key, receipt)
        assert not (run._key_dir(key) / 'result.json').exists()
        engine.collect(key, receipt)
        assert root.read_bytes() == b'new executable\x00'
        assert root.stat().st_mode & 0o111
        assert_collected(run, engine, key, receipt)
    finally:
        run.release()


@pytest.mark.parametrize('extra', ['untracked', 'ignored', 'empty_directory', 'changed_tracked'])
def test_collect_directory_to_file_rejects_unowned_content(env, extra):
    files = directory_artifact(env)
    run, engine, key, _ = adapter(env)
    root = Path(run.spec.author_checkout) / 'demo/pkg'
    unexpected = root / {'untracked': 'notes.txt', 'ignored': '__pycache__/cache',
                         'empty_directory': 'empty', 'changed_tracked': 'a.py'}[extra]
    if extra == 'empty_directory':
        unexpected.mkdir()
    else:
        unexpected.parent.mkdir(parents=True, exist_ok=True)
        unexpected.write_bytes(b'keep my data')
    try:
        with pytest.raises(ExecutionUnknown):
            engine.collect(key, payload(run, engine, key, files))
        if extra == 'empty_directory':
            assert unexpected.is_dir()
        else:
            assert unexpected.read_bytes() == b'keep my data'
        assert (root / 'nested/旧 名.py').read_text() == 'old nested'
        assert not (run._key_dir(key) / 'result.json').exists()
    finally:
        run.release()


def test_collect_directory_to_file_requires_all_tracked_children_deleted(env):
    files = directory_artifact(env)
    files.pop()
    run, engine, key, _ = adapter(env)
    root = Path(run.spec.author_checkout) / 'demo/pkg'
    try:
        with pytest.raises(ExecutionUnknown):
            engine.collect(key, payload(run, engine, key, files))
        assert (root / 'a.py').read_text() == 'old a'
        assert (root / 'nested/旧 名.py').read_text() == 'old nested'
    finally:
        run.release()


def test_collect_directory_to_file_preserves_writer_after_intent(env, monkeypatch):
    files = directory_artifact(env)
    run, engine, key, _ = adapter(env)
    target = Path(run.spec.author_checkout) / 'demo/pkg/nested/旧 名.py'
    save = boat_execution.save

    def concurrent_write(path, content):
        save(path, content)
        if path == engine._path(key, '.collect.json'):
            target.write_text('concurrent writer')

    try:
        monkeypatch.setattr(boat_execution, 'save', concurrent_write)
        with pytest.raises(ExecutionUnknown):
            engine.collect(key, payload(run, engine, key, files))
        assert target.read_text() == 'concurrent writer'
        assert not (run._key_dir(key) / 'result.json').exists()
    finally:
        run.release()


def test_collect_directory_to_file_keeps_file_created_before_rmdir(env, monkeypatch):
    files = directory_artifact(env)
    run, engine, key, _ = adapter(env)
    root = Path(run.spec.author_checkout) / 'demo/pkg'
    rmdir = Path.rmdir

    def concurrent_write(path):
        if path == root:
            (root / 'concurrent.txt').write_text('keep')
        return rmdir(path)

    try:
        monkeypatch.setattr(Path, 'rmdir', concurrent_write)
        with pytest.raises((ExecutionUnknown, OSError)):
            engine.collect(key, payload(run, engine, key, files))
        assert (root / 'concurrent.txt').read_text() == 'keep'
        assert not (run._key_dir(key) / 'result.json').exists()
    finally:
        run.release()


@pytest.mark.parametrize('reverse', [False, True])
@pytest.mark.parametrize('interrupt', [False, True])
def test_collect_file_to_directory_is_order_independent_and_resumable(env, monkeypatch, reverse, interrupt):
    files = file_artifact(env)
    if reverse:
        files.reverse()
    run, engine, key, _ = adapter(env)
    receipt = payload(run, engine, key, files)
    root = Path(run.spec.author_checkout) / 'demo/pkg'
    replace = os.replace

    def interrupted_replace(source, destination):
        replace(source, destination)
        if Path(destination) == root / 'nested/a.py':
            raise OSError('collect interrupted')

    try:
        if interrupt:
            with monkeypatch.context() as patch:
                patch.setattr(boat_execution.os, 'replace', interrupted_replace)
                with pytest.raises(OSError, match='collect interrupted'):
                    engine.collect(key, receipt)
        engine.collect(key, receipt)
        assert (root / 'nested/a.py').read_bytes() == b'new'
        assert not (root / 'nested/a.py').stat().st_mode & 0o111
        assert_collected(run, engine, key, receipt)
    finally:
        run.release()


@pytest.mark.parametrize('extra', ['untracked', 'ignored', 'empty_directory', 'changed_output'])
def test_collect_file_to_directory_recovery_keeps_unowned_content(env, extra):
    files = file_artifact(env)
    run, engine, key, binding = adapter(env)
    root = Path(run.spec.author_checkout) / 'demo/pkg'
    root.unlink()
    (root / 'nested').mkdir(parents=True)
    (root / 'nested/a.py').write_bytes(b'new')
    unexpected = root / {'untracked': 'notes.txt', 'ignored': '__pycache__/cache',
                         'empty_directory': 'empty', 'changed_output': 'nested/a.py'}[extra]
    if extra == 'empty_directory':
        unexpected.mkdir()
    else:
        unexpected.parent.mkdir(parents=True, exist_ok=True)
        unexpected.write_bytes(b'keep my data')
    boat_execution.save(engine._path(key, '.collect.json'), {'identity': binding.identity, 'files': files})
    try:
        with pytest.raises(ExecutionUnknown):
            engine.collect(key, payload(run, engine, key, files))
        if extra == 'empty_directory':
            assert unexpected.is_dir()
        else:
            assert unexpected.read_bytes() == b'keep my data'
        assert not (run._key_dir(key) / 'result.json').exists()
    finally:
        run.release()
