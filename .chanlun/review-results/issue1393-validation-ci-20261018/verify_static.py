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
assert workflow['on'] == {'push': {'branches': ['codex/1393-validate-once-20261018']}}
assert workflow['permissions'] == {'contents': 'read'}
assert workflow['concurrency']['cancel-in-progress'] == 'false'
assert 'inputs.contract' not in (ROOT / '.github/workflows/issue1393-validation.yml').read_text()
assert 'github.event.head_commit.message' not in (ROOT / '.github/workflows/issue1393-validation.yml').read_text()
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
transport_base = '41e8a326743eabb10c1b07df01ce956235c5ad3d'
def git_bytes(*args, input=None):
    return subprocess.run(['git', *args], cwd=ROOT, input=input, capture_output=True, check=True).stdout

assert git_bytes('rev-parse', 'HEAD').decode().strip() == transport_base
assert git_bytes('show', transport_base + ':.github/workflows/ci.yml') == (ROOT / '.github/workflows/ci.yml').read_bytes()
old_ast = ast.parse(git_bytes('show', transport_base + ':' + CODE[0]))
new_ast = ast.parse((ROOT / CODE[0]).read_text())
preserved = {'Budget', 'execute_checks', 'telemetry_command', 'select_binary', 'verify_list',
             'verify_exact', 'file_identity', 'planned_steps', 'package', 'verify_source_names'}
for name in preserved:
    old_node = next(n for n in old_ast.body if getattr(n, 'name', None) == name)
    new_node = next(n for n in new_ast.body if getattr(n, 'name', None) == name)
    assert ast.dump(old_node) == ast.dump(new_node), name
print('fixed base + unchanged existing CI / build / exact / Lean / fixture / budget / package AST PASS')

head, parent = git_bytes('rev-parse', 'HEAD').strip(), git_bytes('rev-parse', 'HEAD^').strip()
assert git_bytes('rev-parse', 'HEAD^1^{tree}') == git_bytes('rev-parse', 'HEAD^^{tree}')
stdin_args = ('diff-tree', '--stdin', '--no-commit-id', '--name-only', '--no-renames', '-r', '-z')
assert git_bytes(*stdin_args, input=parent + b' ' + head + b'\n') == git_bytes('diff', '--name-only', '--no-renames', '-z', 'HEAD^', 'HEAD')
assert git_bytes(*stdin_args, input=head + b' ' + head + b'\n') == b''
print('read-only Git stdin identity protocol + parent tree syntax PASS; no Git mutation')

report = '.chanlun/review-results/issue1393-validation-ci-20261018/'
allowed = {*CODE, '.github/workflows/issue1393-validation.yml'}
changed = git_bytes('diff', '--name-only', '-z', transport_base).decode().split('\0')
untracked = git_bytes('ls-files', '--others', '--exclude-standard', '-z').decode().split('\0')
assert all(not p or p in allowed or p.startswith(report) for p in changed + untracked)
history = git_bytes('ls-tree', '-r', '--name-only', transport_base, '--', report).decode().splitlines()
for name in history:
    if name not in (report + 'CONTRACT-DRAFT.json', report + 'verify_static.py'):
        assert git_bytes('show', transport_base + ':' + name) == (ROOT / name).read_bytes(), name
print('write scope + product unchanged + previous 19-toy history byte preservation PASS')
subprocess.run(['git', 'diff', '--check'], cwd=ROOT, check=True)
for relative in CODE + ['.github/workflows/issue1393-validation.yml']:
    result = subprocess.run(['git', 'diff', '--no-index', '--check', '/dev/null', relative],
                            cwd=ROOT, capture_output=True, text=True)
    assert result.returncode in (0, 1) and not result.stdout and not result.stderr, result
print('tracked + new code whitespace check PASS')
