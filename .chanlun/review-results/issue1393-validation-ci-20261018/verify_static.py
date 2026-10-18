#!/usr/bin/env python3
"""#1393 静态检查；只读 Git/解析 shell，从不运行产品或构建。"""
import ast
from pathlib import Path
import py_compile
import subprocess
import tempfile

import yaml

ROOT = Path(__file__).resolve().parents[3]
BASE = 'ab3f3fb0e1638f1ef3543b8b1948cd1856dc8675'
CODE = ['.github/scripts/issue1393_validate.py', '.github/scripts/test_issue1393_validate.py']
with tempfile.TemporaryDirectory(prefix='issue1393-pycompile-') as directory:
    for index, relative in enumerate(CODE):
        path = ROOT / relative
        ast.parse(path.read_text(), filename=relative)
        py_compile.compile(str(path), cfile=str(Path(directory) / f'{index}.pyc'), doraise=True)
        print('AST + py_compile PASS:', relative)
workflow = yaml.load((ROOT / '.github/workflows/issue1393-validation.yml').read_text(), Loader=yaml.BaseLoader)
assert set(workflow['on']) == {'workflow_dispatch'}
assert workflow['env']['ISSUE1393_EXECUTE'] == 'False'
job = workflow['jobs']['necessary-validation']
assert job['timeout-minutes'] == '60'
assert all("env.ISSUE1393_EXECUTE == 'True'" in step['if'] for step in job['steps'][1:])
for step in job['steps']:
    shell = step.get('run')
    if shell:
        subprocess.run(['bash', '-n'], input=shell, text=True, check=True)
        if "<<'" in shell:
            marker = shell.split("<<'", 1)[1].split("'", 1)[0]
            embedded = shell.split('\n', 1)[1].rsplit('\n' + marker, 1)[0]
            ast.parse(embedded)
print('YAML + False gate + 60-minute job + embedded Python/shell syntax PASS; PyYAML', yaml.__version__)
old = subprocess.run(['git', 'show', BASE + ':.github/workflows/ci.yml'], cwd=ROOT,
                     capture_output=True, text=True, check=True).stdout
current = (ROOT / '.github/workflows/ci.yml').read_text()
addition = '''      # #1393：此单文件不在 lake 默认 roots；常规 CI 必须持续显式核验。
      - name: Check raw inclusion initialization (Lean, #1393)
        run: lake env lean Origin/RawInclusionInitialization.lean
        working-directory: formal

'''
assert current.count(addition) == 1 and current.replace(addition, '') == old
ci = yaml.load(current, Loader=yaml.BaseLoader)
runs = [step.get('run') for step in ci['jobs']['fixture-drift']['steps']]
assert runs.index('lake build') < runs.index('lake env lean Origin/RawInclusionInitialization.lean') < runs.index('python3 scripts/check_fixture_drift.py')
print('ci.yml exact additive-only comparison PASS; original build/fixture preserved')
subprocess.run(['git', 'diff', '--check'], cwd=ROOT, check=True)
for relative in CODE + ['.github/workflows/issue1393-validation.yml']:
    result = subprocess.run(['git', 'diff', '--no-index', '--check', '/dev/null', relative],
                            cwd=ROOT, capture_output=True, text=True)
    assert result.returncode in (0, 1) and not result.stdout and not result.stderr, result
print('tracked + new code whitespace check PASS')
