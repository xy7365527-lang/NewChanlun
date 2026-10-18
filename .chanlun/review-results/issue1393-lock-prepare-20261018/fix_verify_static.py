#!/usr/bin/env python3
"""#1393 尾差静态核验；不执行 Rust、网络或产品，不改写历史验收脚本。"""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

import yaml

ROOT = Path(__file__).resolve().parents[3]
BASE = 'a1a2aded058fc208217e8b701ec50f78b6b0aeeb'
BASE_TREE = '9ba120542f45048bfda948e2acfc9dec7258f230'
REPORT = '.chanlun/review-results/issue1393-lock-prepare-20261018/'
RUNNER = '.github/scripts/issue1393_lock_prepare.py'
TEST = '.github/scripts/test_issue1393_lock_prepare.py'
WORKFLOW = '.github/workflows/issue1393-lock-prepare.yml'


def git(*args):
    return subprocess.run(['git', *args], cwd=ROOT, check=True, capture_output=True).stdout


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


assert git('rev-parse', 'HEAD').decode().strip() == BASE
assert git('rev-parse', 'HEAD^{tree}').decode().strip() == BASE_TREE
assert git('branch', '--show-current').decode().strip() == 'codex/1393-lock-prepare-20261018'
assert not git('diff', '--cached', '--name-only')
changed = git('diff', '--name-only', BASE).decode().splitlines()
new = git('ls-files', '--others', '--exclude-standard').decode().splitlines()
assert set(changed) == {RUNNER, TEST}, changed
assert all(p.startswith(REPORT) for p in new), new
git('diff', '--check')
print('PASS：固定 HEAD/tree/codex 分支；仅两个指定源码尾差和本票新报告；无暂存，空白通过')

before = git('show', BASE + ':' + RUNNER).decode()
after = (ROOT / RUNNER).read_text()
old_functions = {n.name: ast.dump(n) for n in ast.parse(before).body if isinstance(n, ast.FunctionDef)}
new_functions = {n.name: ast.dump(n) for n in ast.parse(after).body if isinstance(n, ast.FunctionDef)}
assert {n for n in old_functions if old_functions[n] != new_functions[n]} == {'bind_source', 'main'}
assert new_functions.keys() - old_functions.keys() == {'check_budget', 'file_digest', 'package', 'failed_package'}
for path in (RUNNER, TEST, REPORT + 'fix_verify_static.py'):
    ast.parse((ROOT / path).read_text(), filename=path)
print('PASS：Python AST；旧入口仅 bind_source/main 改动，新增四个局部收尾函数')

preserved = {}
for path in git('ls-tree', '-r', '--name-only', BASE, '--', REPORT).decode().splitlines():
    raw = git('show', BASE + ':' + path)
    assert (ROOT / path).read_bytes() == raw, path
    preserved[path] = {'sha256': sha(raw), 'bytes': len(raw)}
assert len(preserved) == 23, len(preserved)
print('PASS：23 份既有报告/独审/失败史原字节保留')

workflow_raw = (ROOT / WORKFLOW).read_bytes()
assert workflow_raw == git('show', BASE + ':' + WORKFLOW)
doc = yaml.load(workflow_raw, Loader=yaml.BaseLoader)
assert doc['env']['ISSUE1393_LOCK_EXECUTE'] == 'False'
assert doc['on'] == {'push': {'branches': ['codex/1393-lock-once-20261018']}}
assert doc['jobs']['lock-prepare']['timeout-minutes'] == '15'
for step in doc['jobs']['lock-prepare']['steps']:
    if 'run' not in step:
        continue
    body = step['run']
    subprocess.run(['bash', '-n'], input=body.encode(), check=True, capture_output=True)
    if "<<'PY'" in body:
        ast.parse(body.split("<<'PY'\n", 1)[1].rsplit('\nPY', 1)[0])
print('PASS：workflow 原字节、False/精确 push/15 分钟；YAML/内嵌 Python AST/bash -n')

spec = importlib.util.spec_from_file_location('lock_fix_static', ROOT / RUNNER)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
assert m.v.package is m.package
assert m.v.TOTAL_SECONDS == m.TOTAL_SECONDS == 900
assert m.v.FINAL_RESERVE == m.FINAL_RESERVE == 90
assert m.DIGEST_CHUNK_BYTES == 1024**2
assert sha((ROOT / m.HELPER).read_bytes()) == m.HELPER_SHA256
assert (ROOT / m.HELPER).read_bytes() == git('show', BASE + ':' + m.HELPER)
assert git('rev-parse', m.SOURCE_COMMIT + '^{tree}').decode().strip() == m.SOURCE_TREE
paths = git('diff', '--name-only', m.SOURCE_COMMIT, BASE).decode().splitlines()
assert all(not m.v.is_product_path(p) for p in paths), paths
source = m.v.product_entries(git('ls-tree', '-rz', '--full-tree', m.SOURCE_COMMIT))
prepared = m.v.product_entries(git('ls-tree', '-rz', '--full-tree', BASE))
assert source == prepared
print('PASS：固定 helper/私有 package 覆盖/900秒/90秒预留/1MiB；固定 S 实际 tree 和全树写集')

draft = json.loads((ROOT / REPORT / 'FIX-CONTRACT-DRAFT.json').read_text())
assert draft['source_commit'] == m.SOURCE_COMMIT and draft['source_tree'] == m.SOURCE_TREE
for key in ('prepared_commit', 'prepared_tree', 'approval_text', 'review_text', 'approval_sha256', 'review_sha256'):
    assert draft[key] == 'UNBOUND', key
for path, key in ((RUNNER, 'runner_sha256'), (WORKFLOW, 'workflow_false_sha256'),
                  (m.HELPER, 'helper_sha256'), (m.TELEMETRY, 'telemetry_sha256'),
                  ('.github/workflows/ci.yml', 'ci_sha256'), ('rust/Cargo.toml', 'manifest_sha256'),
                  ('rust/Cargo.lock', 'old_lock_sha256'),
                  ('.github/workflows/issue1393-validation.yml', 'validation_workflow_sha256'),
                  (REPORT + 'RUST-TOOLCHAIN-39091453686.txt', 'toolchain_evidence_sha256')):
    assert draft[key] == sha((ROOT / path).read_bytes()), key
assert draft['product_manifest_sha256'] == m.digest(m.v.canonical(source))
try:
    m.v.validate_contract('True', draft)
except m.v.Refused:
    pass
else:
    raise AssertionError('UNBOUND 草案被误准入')
print('PASS：新草案固定源/已知摘要准确；E/批准/独审仍 UNBOUND 且准确拒绝')
print(json.dumps({'preserved_originals': preserved,
                  'source_entries': len(source), 'product_manifest_sha256': draft['product_manifest_sha256'],
                  'tracked_delta': changed, 'new_report_files': new}, ensure_ascii=False, indent=2))
