#!/usr/bin/env python3
"""#1393 本机静态验收：不触发 Rust、网络、产品或 Git 写入。"""
import ast
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
from unittest.mock import patch

import yaml

root = Path(__file__).resolve().parents[3]
report = '.chanlun/review-results/issue1393-lock-prepare-20261018/'
workflow = '.github/workflows/issue1393-lock-prepare.yml'
runner = '.github/scripts/issue1393_lock_prepare.py'
test = '.github/scripts/test_issue1393_lock_prepare.py'

for path in (runner, test, report + 'verify_static.py'):
    ast.parse((root / path).read_text(), filename=path)
print('PASS：新 Python 文件 AST')

doc = yaml.load((root / workflow).read_text(), Loader=yaml.BaseLoader)
assert doc['on'] == {'push': {'branches': ['codex/1393-lock-once-20261018']}}
assert doc['permissions'] == {'contents': 'read'}
assert doc['env']['ISSUE1393_LOCK_EXECUTE'] == 'False'
assert doc['concurrency']['cancel-in-progress'] == 'false'
job = doc['jobs']['lock-prepare']
assert job['timeout-minutes'] == '15' and job['runs-on'] == 'ubuntu-24.04'
steps = job['steps']
assert "!= 'True'" in steps[0]['if']
assert steps[0]['run'].splitlines() == ["echo 'PREPARED_NOT_AUTHORIZED：所有真实命令 NOT_RUN'", 'exit 1']
assert all("env.ISSUE1393_LOCK_EXECUTE == 'True'" in s['if'] for s in steps[1:])
checkout = next(s for s in steps if s.get('uses', '').startswith('actions/checkout@'))
assert checkout['with']['fetch-depth'] == '0'
for key in ('persist-credentials', 'lfs', 'submodules', 'sparse-checkout-cone-mode'):
    assert checkout['with'][key] == 'false'
assert len([s for s in steps if s.get('uses', '').startswith('actions/checkout@')]) == 1
assert len([s for s in steps if s.get('uses', '').startswith('actions/upload-artifact@')]) == 2
assert (root / workflow).read_bytes().count(b"  ISSUE1393_LOCK_EXECUTE: 'False'\n") == 1
print('PASS：YAML、精确 push、False 零外部命令、15 分钟、只读 token、sparse、无 LFS/submodules')

for s in steps:
    if 'run' not in s:
        continue
    body = s['run']
    assert '${{' not in body
    subprocess.run(['bash', '-n'], input=body.encode(), check=True)
    if "<<'PY'" in body:
        code = body.split("<<'PY'\n", 1)[1].rsplit('\nPY', 1)[0]
        ast.parse(code)
print('PASS：全部内嵌 Python AST 与 bash -n；message 不插入 shell')

# 两个收尾片段用临时目录执行，验证上传预算和实际结论，不调用外部命令。
for age, conclusion in ((0, 'success'), (901, 'skipped'), (0, 'failure')):
    with tempfile.TemporaryDirectory() as directory:
        p = Path(directory)
        (p / 'issue1393-lock-start.json').write_text(json.dumps(
            {'monotonic': time.monotonic() - age, 'wall': time.time() - age, 'total_seconds': 900}))
        env = {'RUNNER_TEMP': directory, 'GITHUB_OUTPUT': str(p / 'outputs'),
               'GITHUB_STEP_SUMMARY': str(p / 'summary'), 'PAYLOAD_OUTCOME': conclusion,
               'PAYLOAD_CONCLUSION': conclusion, 'PAYLOAD_ID': 'toy-id',
               'PAYLOAD_DIGEST': 'toy-digest', 'PAYLOAD_URL': 'toy-url'}
        for ident in ('archive', 'receipt'):
            body = next(s['run'] for s in steps if s.get('id') == ident)
            code = body.split("<<'PY'\n", 1)[1].rsplit('\nPY', 1)[0]
            with patch.dict(os.environ, env, clear=True), contextlib.redirect_stdout(io.StringIO()), \
                    patch.object(subprocess, 'Popen') as spawn:
                try:
                    exec(compile(code, ident, 'exec'), {})
                except SystemExit:
                    assert ident == 'receipt' and conclusion != 'success'
                else:
                    assert ident != 'receipt' or conclusion == 'success'
                spawn.assert_not_called()
        requested = json.loads((p / 'issue1393-lock-evidence/UPLOAD-REQUEST.json').read_text())
        assert requested['status'] == ('NOT_RUN' if age > 900 else 'REQUESTED')
        actual = json.loads((p / 'issue1393-lock-UPLOAD-ACTUAL.json').read_text())
        assert actual['conclusion'] == conclusion
print('PASS：失败启动保件、过期不上传、真实 upload 成功/失败/跳过收据 toy')

spec = importlib.util.spec_from_file_location('static_lock', root / runner)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
assert m.v.TOTAL_SECONDS == 900 and m.v.FINAL_RESERVE == 90
assert m.v.CONTRACT_FIXED is m.FIXED
assert m.v.PUSH_REF != 'refs/heads/codex/1393-validate-once-20261018'
assert hashlib.sha256((root / m.HELPER).read_bytes()).hexdigest() == m.HELPER_SHA256
assert m.resolution_argv(Path('/fixed-temp')) == [
    'cargo', '+1.99.0-x86_64-unknown-linux-gnu', 'generate-lockfile',
    '--manifest-path', '/fixed-temp/rust/Cargo.toml']
source = (root / runner).read_text()
assert 'v.execute_checks(' not in source and 'v.main(' not in source
assert 'stable' not in source and 'shell=True' not in source
print('PASS：固定 helper 摘要、identity/Budget/Supervisor 复用、固定工具链与唯一解析 argv')

base = '8e50d40933c2a6a25f882481f3c46aa885ce6407'
def git(*args):
    return subprocess.check_output(['git', *args], cwd=root).decode().strip()
assert git('rev-parse', 'HEAD') == base
assert git('rev-parse', 'HEAD^{tree}') == '0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0'
assert git('branch', '--show-current') == 'codex/1393-lock-prepare-20261018'
assert not git('diff', '--name-only', base)
assert not git('diff', '--cached', '--name-only')
paths = git('ls-files', '--others', '--exclude-standard').splitlines()
assert all(p in (workflow, runner, test) or p.startswith(report) for p in paths), paths
print('PASS：固定 commit/tree/分支、无 tracked 改动、无暂存、完整新写集在授权范围')
for path in (workflow, runner, test):
    lines = (root / path).read_text().splitlines()
    assert all(line == line.rstrip() for line in lines)
print('PASS：空白检查；既有 runner/workflow/telemetry/Rust/Lean/manifest/lock 保持')
