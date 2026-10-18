#!/usr/bin/env python3
"""#1393 准备静态核验；只读 Git、AST、现有 YAML 解析器、bash -n 与隔离上传 toy。"""
import ast
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from unittest.mock import patch

import yaml

ROOT = Path(__file__).resolve().parents[3]
BASE = '8542c00cb1b1013e8adc0ffdddc54e4d42f894a4'
BASE_TREE = '036351df2c741a6c6967945e38d4a1419f2eb3aa'
REPORT = '.chanlun/review-results/issue1393-lock-preserve-20261018/'
RUNNER = '.github/scripts/issue1393_lock_prepare.py'
TEST = '.github/scripts/test_issue1393_lock_prepare.py'
WORKFLOW = '.github/workflows/issue1393-lock-prepare.yml'


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, check=True, capture_output=True).stdout


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


assert git('rev-parse', 'HEAD').decode().strip() == BASE
assert git('rev-parse', 'HEAD^{tree}').decode().strip() == BASE_TREE
assert git('branch', '--show-current').decode().strip() == 'codex/1393-lock-preserve-prepare-20261018'
assert not git('diff', '--cached', '--name-only')
changed = git('diff', '--name-only', BASE).decode().splitlines()
new = git('ls-files', '--others', '--exclude-standard').decode().splitlines()
assert set(changed) == {RUNNER, TEST, WORKFLOW}, changed
assert all(p.startswith(REPORT) for p in new), new
git('diff', '--check')
print('PASS：固定 HEAD/tree/codex 分支；三个入口及新报告写集；无暂存，空白通过')

before = ast.parse(git('show', BASE + ':' + RUNNER).decode())
after = ast.parse((ROOT / RUNNER).read_text())
originals = {n.name: ast.dump(n) for n in before.body if isinstance(n, ast.FunctionDef)}
functions = {n.name: ast.dump(n) for n in after.body if isinstance(n, ast.FunctionDef)}
for name in ('load_fixed_helper', 'validate_push', 'tree_files', 'verified_bytes', 'target_sources',
             'isolated_environment', 'verify_toolchain', 'install_toolchain',
             'check_budget', 'file_digest', 'failed_package'):
    assert originals[name] == functions[name], name
for path in (RUNNER, TEST, REPORT + 'verify_prepare_static.py'):
    ast.parse((ROOT / path).read_text(), filename=path)
print('PASS：Python AST；身份推送门、隔离环境、工具链与最终摘要/失败函数复用原 AST')

spec = importlib.util.spec_from_file_location('preserve_static', ROOT / RUNNER)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
preserved = {}
for path in git('ls-tree', '-r', '--name-only', BASE, '--', m.LEGACY_REPORT).decode().splitlines():
    raw = git('show', BASE + ':' + path)
    assert (ROOT / path).read_bytes() == raw, path
    preserved[path] = {'sha256': sha(raw), 'bytes': len(raw)}
print('PASS：保全全部既有报告/独审/失败史原件，数量=' + str(len(preserved)))

workflow = yaml.load((ROOT / WORKFLOW).read_bytes(), Loader=yaml.BaseLoader)
old_workflow = yaml.load(git('show', BASE + ':' + WORKFLOW), Loader=yaml.BaseLoader)
assert workflow['env']['ISSUE1393_LOCK_EXECUTE'] == 'False'
assert workflow['on'] == {'push': {'branches': ['codex/1393-lock-preserve-once-20261018']}}
assert workflow['permissions'] == {'contents': 'read'}
assert workflow['jobs']['lock-prepare']['timeout-minutes'] == '15'
steps = workflow['jobs']['lock-prepare']['steps']
assert steps[2] == old_workflow['jobs']['lock-prepare']['steps'][2]
assert steps[2]['with']['persist-credentials'] == 'false'
assert len([s for s in steps if s.get('uses') == 'actions/upload-artifact@v4']) == 2
bodies = {}
for step in steps:
    if 'run' not in step:
        continue
    body = step['run']
    subprocess.run(['bash', '-n'], input=body.encode(), check=True, capture_output=True)
    if "<<'PY'" in body:
        code = body.split("<<'PY'\n", 1)[1].rsplit('\nPY', 1)[0]
        ast.parse(code)
        bodies[step.get('id', step['name'])] = code
print('PASS：YAML/内嵌 Python AST/bash -n；False/精确 push/只读 checkout/15 分钟')

# 上传代码仅运行在临时 toy 中；屏蔽 Popen，无 GitHub、网络或上传 action。
caps = {'single_bytes': 1024**2, 'total_bytes': 8*1024**2, 'files': 100,
        'items': 1000, 'upload_bytes': 8*1024**2}
for mode in ('bound', 'unbound', 'bytes', 'files', 'expired', 'missing-start'):
    with tempfile.TemporaryDirectory() as d:
        temp = Path(d).resolve()
        (temp / '.github/scripts').mkdir(parents=True)
        values = dict(caps)
        if mode == 'unbound':
            values = dict(m.RESOURCE_CAPS)
        if mode == 'bytes':
            values['upload_bytes'] = 1
        if mode == 'files':
            values['files'] = 1
        (temp / RUNNER).write_text('RESOURCE_CAPS = ' + repr(values) + '\n')
        start = {'monotonic': time.monotonic() - (901 if mode == 'expired' else 0),
                 'wall': time.time(), 'total_seconds': 900}
        if mode != 'missing-start':
            (temp / 'issue1393-lock-start.json').write_text(json.dumps(start))
        out = temp / 'issue1393-lock-evidence'
        out.mkdir()
        (out / 'raw.stdout').write_bytes(b'original')
        env = {'RUNNER_TEMP': str(temp), 'GITHUB_OUTPUT': str(temp / 'outputs'),
               'GITHUB_STEP_SUMMARY': str(temp / 'summary'), 'PAYLOAD_OUTCOME': 'success',
               'PAYLOAD_CONCLUSION': 'success', 'PAYLOAD_ID': 'toy', 'PAYLOAD_DIGEST': 'toy', 'PAYLOAD_URL': 'toy'}
        with contextlib.chdir(temp), patch.dict(os.environ, env, clear=True), \
                patch.object(subprocess, 'Popen') as spawn, contextlib.redirect_stdout(io.StringIO()):
            exec(compile(bodies['archive'], '<archive-toy>', 'exec'), {})
            requested = (temp / 'outputs').read_text()
            assert ('allowed=true' in requested) == (mode == 'bound'), mode
            assert (out / 'raw.stdout').read_bytes() == b'original'
            (temp / 'outputs').write_text('')
            exec(compile(bodies['receipt'], '<receipt-toy>', 'exec'), {})
            receipt = json.loads((temp / 'issue1393-lock-UPLOAD-ACTUAL.json').read_text())
            assert receipt['conclusion'] == 'success'
            assert ('allowed=true' in (temp / 'outputs').read_text()) == (mode == 'bound'), mode
            spawn.assert_not_called()
print('PASS：上传字节/数量/UNBOUND/截止/缺起点 toy；原件保留，Popen 为零')

assert m.v.package is m.package
assert m.v.TOTAL_SECONDS == m.TOTAL_SECONDS == 900
assert m.v.FINAL_RESERVE == m.FINAL_RESERVE == 90
assert m.DIGEST_CHUNK_BYTES == 1024**2
assert sha((ROOT / m.HELPER).read_bytes()) == m.HELPER_SHA256
assert (ROOT / m.HELPER).read_bytes() == git('show', BASE + ':' + m.HELPER)
for path in (m.TELEMETRY, '.github/workflows/ci.yml', '.github/workflows/issue1393-validation.yml', 'rust/Cargo.toml', 'rust/Cargo.lock'):
    assert (ROOT / path).read_bytes() == git('show', BASE + ':' + path), path
assert git('rev-parse', m.SOURCE_COMMIT + '^{tree}').decode().strip() == m.SOURCE_TREE
paths = git('diff', '--name-only', m.SOURCE_COMMIT, BASE).decode().splitlines()
assert all(not m.v.is_product_path(p) for p in paths), paths
source = m.v.product_entries(git('ls-tree', '-rz', '--full-tree', m.SOURCE_COMMIT))
prepared = m.v.product_entries(git('ls-tree', '-rz', '--full-tree', BASE))
assert source == prepared
print('PASS：固定 helper/源/toolchain/业务/旧锁未改；完整 Git 产品对象一致')

draft = json.loads((ROOT / REPORT / 'CONTRACT-DRAFT.json').read_text())
assert draft['schema'] == 'issue1393-lock-preserve/1' and draft['operation'] == m.OPERATION
for key in ('prepared_commit', 'prepared_tree', 'approval_text', 'review_text', 'approval_sha256', 'review_sha256'):
    assert draft[key] == 'UNBOUND', key
for path, key in ((RUNNER, 'runner_sha256'), (WORKFLOW, 'workflow_false_sha256'),
                  (m.HELPER, 'helper_sha256'), (m.TELEMETRY, 'telemetry_sha256'),
                  ('.github/workflows/ci.yml', 'ci_sha256'), ('rust/Cargo.toml', 'manifest_sha256'),
                  ('rust/Cargo.lock', 'old_lock_sha256'),
                  ('.github/workflows/issue1393-validation.yml', 'validation_workflow_sha256'),
                  (m.LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt', 'toolchain_evidence_sha256')):
    assert draft[key] == sha((ROOT / path).read_bytes()), key
assert draft['product_manifest_sha256'] == m.digest(m.v.canonical(source))
assert draft['resource_caps'] == m.RESOURCE_CAPS
try:
    m.v.validate_contract('True', draft)
except m.v.Refused:
    pass
else:
    raise AssertionError('UNBOUND 草案被误准入')
print('PASS：草案已知摘要准确，E/批准/独审/资源帽 UNBOUND，准确拒绝')
print(json.dumps({'preserved_originals': preserved, 'source_entries': len(source),
                  'product_manifest_sha256': draft['product_manifest_sha256'], 'tracked_delta': changed,
                  'resource_caps': m.RESOURCE_CAPS}, ensure_ascii=False, indent=2))
