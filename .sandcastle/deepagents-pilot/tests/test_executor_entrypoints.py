import os
import plistlib
import shlex
import subprocess
import sys
from pathlib import Path
from xml.sax.saxutils import escape

import pytest

REPO = Path(__file__).resolve().parents[3]


@pytest.fixture
def cli_environment(tmp_path):
    venv = tmp_path / 'venv with spaces'
    (venv / 'bin').mkdir(parents=True)
    bootstrap = tmp_path / 'sitecustomize.py'
    bootstrap.write_text('''import os
import sys
from importlib.machinery import PathFinder

class EntrypointOnlyApprovalSigning:
    def find_spec(self, fullname, path=None, target=None):
        if fullname != 'approval_signing':
            return None
        spec = PathFinder.find_spec(fullname, os.environ.get('PYTHONPATH', '').split(os.pathsep))
        if spec is None:
            raise ModuleNotFoundError("No module named 'approval_signing' in entrypoint PYTHONPATH")
        return spec

sys.meta_path.insert(0, EntrypointOnlyApprovalSigning())
''')
    python = venv / 'bin/python'
    python.write_text('#!/bin/sh\n'
                      f'export PYTHONPATH="$PYTHONPATH":{shlex.quote(str(tmp_path))}\n'
                      f'exec {shlex.quote(sys.executable)} "$@"\n')
    python.chmod(0o700)
    env = {'PATH': os.defpath, 'HOME': str(tmp_path), 'VENV': str(venv),
           'PYTHONDONTWRITEBYTECODE': '1', 'LANGSMITH_TRACING': 'false',
           'LANGCHAIN_TRACING_V2': 'false', 'OTEL_SDK_DISABLED': 'true'}
    return venv, env


def assert_help(command, env, cwd=REPO):
    child = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=30)
    assert child.returncode == 0, child.stdout + child.stderr
    assert 'usage:' in child.stdout
    assert 'register' in child.stdout and 'serve' in child.stdout


def test_manual_function_runs_real_executor_help(cli_environment):
    _, env = cli_environment
    manual = (REPO / '.sandcastle/deepagents-pilot/EXECUTOR.md').read_text()
    function = next(line for line in manual.splitlines() if line.startswith('executor()'))
    assert_help(['bash', '-c', function + '\nexecutor --help'], env)


def test_launchd_template_runs_real_executor_help(cli_environment, tmp_path):
    venv, env = cli_environment
    template = (REPO / '.sandcastle/launchd/com.newchanlun.deepagents-executor.plist').read_text()
    for placeholder, value in {'__REPO__': REPO, '__VENV__': venv,
                               '__PRIVATE_KEY_PATH__': tmp_path / 'absent.pem',
                               '__BOAT_CONFIG__': tmp_path / 'absent.json',
                               '__BOAT_API_KEY_FILE__': tmp_path / 'absent.key'}.items():
        template = template.replace(placeholder, escape(str(value)))
    config = plistlib.loads(template.encode())
    args = config['ProgramArguments']
    assert_help(args[:args.index('pilot1742.executor') + 1] + ['--help'],
                env | config['EnvironmentVariables'], config['WorkingDirectory'])


def test_runbook_commands_run_real_executor_help(cli_environment, tmp_path):
    venv, env = cli_environment
    preview = subprocess.run(['bash', str(REPO / '.sandcastle/cutover-runbook.sh'), str(REPO), str(venv),
                              str(tmp_path / 'absent.pem'), str(tmp_path / 'absent.json'),
                              str(tmp_path / 'absent.key')], env=env, capture_output=True,
                             text=True, check=True, timeout=10)
    prefixes = {line.split(' -m pilot1742.executor ')[0] + ' -m pilot1742.executor --help'
                for line in preview.stdout.splitlines() if ' -m pilot1742.executor ' in line}
    assert prefixes
    for command in prefixes:
        assert_help(['bash', '-c', command], env)
