#!/usr/bin/env python3
"""#1393 必要验证。默认拒绝；正式入口仅供根核准的 GitHub 远端单次运行。"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
from pathlib import Path
import re
import signal
import shutil
import stat
import subprocess
import sys
import time
import zipfile

WORKFLOW = '.github/workflows/issue1393-validation.yml'
RUNNER = '.github/scripts/issue1393_validate.py'
TEST_FILE = '.github/scripts/test_issue1393_validate.py'
REPORT = '.chanlun/review-results/issue1393-validation-ci-20261018/'
GATE_FALSE = b"  ISSUE1393_EXECUTE: 'False'\n"
GATE_TRUE = b"  ISSUE1393_EXECUTE: 'True'\n"
TOTAL_SECONDS = 3600
# 回收、封包、upload 使用总窗口剩余时间，不给后续步骤续租。
FINAL_RESERVE = 30
LIB_PREFIX = 'theta_v0::parser::inclusion::'
LIB_CASES = {
    LIB_PREFIX + 'tb02u_tests::' + name:
        'rust/src/theta_v0/parser/inclusion_tb02u_tests.rs'
    for name in (
        'tb02u_independent_oracle_all_fields_and_first_pair',
        'tb02u_batch_incremental_and_every_boundary_resume',
        'tb02u_revision_retracts_and_reestablishes_without_mutating_prior_facts',
        'tb02u_exhaustive_small_raw_domain_keeps_established_geometry',
    )
}
LIB_CASES['theta_v0::parser::tb02u_tests::tb02u_parser_pending_activation_sparse_coords_and_resume'] = 'rust/src/theta_v0/parser/tb02u_tests.rs'
for _name in (
    'tb02a_trace_matches_existing_kernel_and_preserves_actual_roots',
    'tb02a_direction_is_from_actual_accumulator_not_previous_raw',
):
    LIB_CASES[LIB_PREFIX + 'tests::' + _name] = 'rust/src/theta_v0/parser/inclusion.rs'
S_CASES = {
    'tests::' + name: 'rust/src/bin/s_session_v2/tb02u_tests.rs'
    for name in (
        'tb02u_old_cut_explicit_replaces_revision_and_recovery',
        'tb02u_pending_append_keeps_first_known_and_policy_gate',
    )
}
TARGETS = {
    'lib': {'name': 'newchan_rust', 'kind': ['cdylib', 'rlib'],
            'src': 'rust/src/lib.rs', 'features': [], 'args': ['--lib'], 'cases': LIB_CASES},
    's': {'name': 's_structure_session', 'kind': ['bin'],
          'src': 'rust/src/bin/s_structure_session.rs',
          'features': ['default', 's_session'],
          'args': ['--features', 's_session', '--bin', 's_structure_session'], 'cases': S_CASES},
}
# Cargo 默认启用空的 default feature，artifact.features 仍含其名称。
TARGETS['lib']['features'] = ['default']
FIXTURE_EXITS = {0: '无漂移', 1: '真实漂移', 2: '内部错误', 3: '缺 lake',
                 4: '构建失败', 5: '导出失败'}


class Refused(RuntimeError):
    """缺绑定、原件不符或前序失败，均不得继续。"""


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + '\n').encode()


def save(path: Path, value) -> None:
    path.write_bytes(canonical(value))


def require(condition, message: str) -> None:
    if not condition:
        raise Refused(message)


def validate_contract(gate: str, contract: dict) -> None:
    """在任何外部命令之前执行；空值/默认值绝不等于授权。"""
    require(gate == 'True', 'False 门：真实命令 NOT_RUN')
    require(contract.get('schema') == 'issue1393-validation/1', '未知合同')
    require(contract.get('status') == 'AUTHORIZED_ONCE', '合同未准入')
    for key in ('source_commit', 'source_tree', 'prepared_commit', 'prepared_tree'):
        value = contract.get(key)
        require(isinstance(value, str) and re.fullmatch(r'[0-9a-f]{40}', value)
                and len(set(value)) > 1, '缺少固定 Git 身份：' + key)
    for key in ('product_manifest_sha256', 'runner_sha256', 'workflow_false_sha256',
                'ci_sha256', 'telemetry_sha256', 'approval_sha256', 'review_sha256'):
        value = contract.get(key)
        require(isinstance(value, str) and re.fullmatch(r'[0-9a-f]{64}', value)
                and len(set(value)) > 1, '缺少固定摘要：' + key)
    for name in ('approval', 'review'):
        value = contract.get(name + '_text')
        require(isinstance(value, str) and value.strip()
                and digest(value.encode()) == contract[name + '_sha256'], name + '原件不符')
    require(contract.get('run_attempt') == '1', '不自动重跑：只允许 attempt 1')
    require(contract.get('lib_features') == ['default']
            and contract.get('s_features') == ['default', 's_session'], 'features 域不符')
    require(contract.get('total_seconds') == TOTAL_SECONDS, '总边界不符')


class Budget:
    def __init__(self, start: dict):
        self.mono = start.get('monotonic')
        self.wall = start.get('wall')
        for value in (self.mono, self.wall):
            require(type(value) in (float, int) and math.isfinite(value), '非法共享起点')
        require(start.get('total_seconds') == TOTAL_SECONDS, '未知总边界')
        require(self.mono <= time.monotonic() and self.wall <= time.time(), '起点在未来')

    def remaining(self, reserve=0) -> float:
        return min(self.mono + TOTAL_SECONDS - time.monotonic(),
                   self.wall + TOTAL_SECONDS - time.time()) - reserve


class Supervisor:
    """同一预算监督所有子进程；回收失败也不能冒充成功。"""
    def __init__(self, output: Path, budget: Budget):
        self.output, self.budget = output, budget
        self.records = []
        self.stopped = False
        self.interrupted = None
        self.active = None
        if sys.platform == 'linux':
            # 领养同组孙进程以便回收 telemetry 的 sleep 等孤儿；不宣称可约束 setsid 逃逸。
            libc = ctypes.CDLL(None, use_errno=True)
            require(libc.prctl(36, 1, 0, 0, 0) == 0, '无法启用 child subreaper')

    def handle_signal(self, signum, _frame):
        self.interrupted = signum
        self.stopped = True

    def skip(self, name, reason):
        row = {'name': name, 'status': 'NOT_RUN', 'reason': reason, 'exit': None}
        self.records.append(row)
        save(self.output / 'steps.json', self.records)
        return row

    def _group_exists(self, pgid, events):
        try:
            os.killpg(pgid, 0)
            return True
        except ProcessLookupError:
            return False
        except OSError as exc:
            event = {'probe_error': repr(exc), 'pgid': pgid}
            if event not in events:
                events.append(event)
            # 权限错误不是“不存在”；继续有界等待，最终仍未知则清理失败。
            return True

    def _reap_adopted(self, pgid):
        if sys.platform == 'linux':
            while True:
                try:
                    pid, _ = os.waitpid(-pgid, os.WNOHANG)
                    if pid == 0:
                        break
                except ChildProcessError:
                    break

    def cleanup(self, process):
        events = []
        end = time.monotonic() + max(0, min(5, self.budget.remaining()))
        for sig in (signal.SIGTERM, signal.SIGKILL):
            try:
                if self._group_exists(process.pid, events):
                    os.killpg(process.pid, sig)
                    events.append({'signal': sig.name, 'pgid': process.pid})
            except OSError as exc:
                events.append({'error': repr(exc)})
            phase_end = min(end, time.monotonic() + (0.25 if sig == signal.SIGTERM else 5))
            while time.monotonic() < phase_end:
                process.poll()
                if process.returncode is not None:
                    self._reap_adopted(process.pid)
                if not self._group_exists(process.pid, events):
                    break
                time.sleep(min(0.02, max(0, phase_end - time.monotonic())))
        process.poll()
        self._reap_adopted(process.pid)
        gone = not self._group_exists(process.pid, events)
        events.append({'reaped': process.returncode is not None, 'group_gone': gone,
                       'returncode': process.returncode})
        return events, gone and process.returncode is not None

    def run(self, name, argv, cwd, env=None):
        if self.stopped or self.interrupted or self.budget.remaining(FINAL_RESERVE) <= 0:
            self.stopped = True
            return self.skip(name, '前步失败、信号或共享截止已到')
        row = {'name': name, 'argv': list(map(str, argv)), 'cwd': str(cwd),
               'started_monotonic': time.monotonic(), 'started_wall': time.time(),
               'remaining_seconds': self.budget.remaining(FINAL_RESERVE), 'exit': None,
               'status': 'RUNNING', 'stdout': name + '.stdout', 'stderr': name + '.stderr'}
        self.records.append(row)
        save(self.output / 'steps.json', self.records)
        process = None
        try:
            with (self.output / row['stdout']).open('xb') as out, (self.output / row['stderr']).open('xb') as err:
                process = subprocess.Popen(row['argv'], cwd=cwd, env=env,
                                           stdout=out, stderr=err, start_new_session=True)
                self.active = process
                row['pid'] = process.pid
                while process.poll() is None:
                    if self.interrupted:
                        row['status'] = 'SIGNALLED'
                        row['received_signal'] = self.interrupted
                        break
                    if self.budget.remaining(FINAL_RESERVE) <= 0:
                        row['status'] = 'TIMEOUT'
                        break
                    time.sleep(min(0.05, max(0, self.budget.remaining(FINAL_RESERVE))))
                else:
                    row['status'] = 'PASS' if process.returncode == 0 else 'FAILED'
                if self.interrupted:
                    row['status'] = 'SIGNALLED'
                    row['received_signal'] = self.interrupted
        except Exception as exc:
            row.update(status='ERROR', error=repr(exc))
        finally:
            if process is not None:
                row['cleanup'], clean = self.cleanup(process)
                row['exit'] = process.returncode
                row['exit_signal'] = -process.returncode if process.returncode is not None and process.returncode < 0 else None
                if not clean:
                    row['status'] = 'CLEANUP_FAILED'
            self.active = None
            row['ended_monotonic'], row['ended_wall'] = time.monotonic(), time.time()
            if row['status'] != 'PASS':
                self.stopped = True
            save(self.output / 'steps.json', self.records)
        return row


def successful(row):
    require(row['status'] == 'PASS', row['name'] + ' 未通过：' + row['status'])


def git_read(supervisor, root, name, *args):
    row = supervisor.run(name, ['git', *args], root)
    successful(row)
    return (supervisor.output / row['stdout']).read_bytes()


def product_entries(tree: bytes) -> list[str]:
    """完整 Git 文件对象清单；仅排除本票的验证写集，避免摘要自引用。"""
    entries = []
    for entry in tree.decode().split('\0'):
        if not entry:
            continue
        _, path = entry.split('\t', 1)
        if path not in (WORKFLOW, RUNNER, TEST_FILE, '.github/workflows/ci.yml') and not path.startswith(REPORT):
            entries.append(entry)
    return sorted(entries)


def verify_activation(prepared: bytes, executed: bytes) -> None:
    require(prepared.count(GATE_FALSE) == 1 and GATE_TRUE not in prepared, '准备版 False 门不唯一')
    require(executed == prepared.replace(GATE_FALSE, GATE_TRUE, 1), '执行尾不止 False→True 一处变化')


def bind_source(root, contract, supervisor):
    output = supervisor.output
    head = git_read(supervisor, root, 'git-head', 'rev-parse', 'HEAD').decode().strip()
    require(head == os.environ.get('GITHUB_SHA'), '执行 HEAD 与 GitHub SHA 不符')
    expected_ref = os.environ.get('GITHUB_REPOSITORY', '') + '/' + WORKFLOW + '@' + os.environ.get('GITHUB_REF', '')
    require(os.environ.get('GITHUB_WORKFLOW_REF') == expected_ref
            and os.environ.get('GITHUB_WORKFLOW_SHA') == head, '实际调用 workflow 身份不符')
    parents = git_read(supervisor, root, 'git-parent', 'rev-list', '--parents', '-n', '1', 'HEAD').decode().split()
    require(parents == [head, contract['prepared_commit']], '执行尾须为准备提交的单父直接子提交')
    for prefix in ('source', 'prepared'):
        tree = git_read(supervisor, root, 'git-' + prefix + '-tree', 'rev-parse', contract[prefix + '_commit'] + '^{tree}').decode().strip()
        require(tree == contract[prefix + '_tree'], prefix + ' tree 不符')
    successful(supervisor.run('git-ancestor', ['git', 'merge-base', '--is-ancestor',
                                            contract['source_commit'], contract['prepared_commit']], root))
    dirty = git_read(supervisor, root, 'git-dirty', 'status', '--porcelain', '--untracked-files=all')
    require(not dirty, '工作区非干净固定源')
    changed = git_read(supervisor, root, 'git-activation-paths', 'diff', '--name-only', '-z',
                       contract['prepared_commit'], head).decode().split('\0')
    require(changed == [WORKFLOW, ''], '准入提交修改了非门文件')
    prepared = git_read(supervisor, root, 'workflow-prepared', 'show', contract['prepared_commit'] + ':' + WORKFLOW)
    verify_activation(prepared, (root / WORKFLOW).read_bytes())
    require(digest(prepared) == contract['workflow_false_sha256'], 'workflow 摘要不符')
    for path, key in ((RUNNER, 'runner_sha256'), ('.github/workflows/ci.yml', 'ci_sha256'),
                      ('.github/scripts/run-cargo-with-telemetry.sh', 'telemetry_sha256')):
        require(digest((root / path).read_bytes()) == contract[key], path + ' 摘要不符')
    source = git_read(supervisor, root, 'source-ls-tree', 'ls-tree', '-rz', contract['source_commit'])
    current = git_read(supervisor, root, 'execution-ls-tree', 'ls-tree', '-rz', head)
    require(product_entries(source) == product_entries(current), '产品 source 与执行尾产品对象不一致')
    manifest = product_entries(source)
    require(digest(canonical(manifest)) == contract['product_manifest_sha256'], '产品清单摘要不符')
    save(output / 'product-manifest.json', manifest)
    execution_tree = git_read(supervisor, root, 'execution-tree', 'rev-parse', 'HEAD^{tree}').decode().strip()
    save(output / 'binding.json', {'source_commit': contract['source_commit'], 'source_tree': contract['source_tree'],
                                 'prepared_commit': contract['prepared_commit'], 'prepared_tree': contract['prepared_tree'],
                                 'execution_commit': head, 'execution_tree': execution_tree,
                                 'workflow_executed_sha256': digest((root / WORKFLOW).read_bytes()),
                                 'runner_sha256': digest((root / RUNNER).read_bytes()),
                                 'github': {k: os.environ.get(k) for k in ('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT',
                                            'GITHUB_JOB', 'GITHUB_WORKFLOW_REF', 'GITHUB_WORKFLOW_SHA', 'GITHUB_REPOSITORY')}})


def verify_source_names(root, cases):
    for full, relative in cases.items():
        short = full.split('::')[-1]
        source = (root / relative).read_text()
        require(len(re.findall(r'#\[test\]\s+fn\s+' + re.escape(short) + r'\s*\(', source)) == 1,
                '源码测试缺失或重复：' + full)


def select_binary(raw: bytes, target: dict, root: Path, target_dir: Path) -> Path:
    require(raw.strip(), '缺 Cargo JSON 原件')
    messages = [json.loads(line) for line in raw.splitlines() if line.strip()]
    finishes = [m for m in messages if m.get('reason') == 'build-finished']
    require(len(finishes) == 1 and finishes[0].get('success') is True
            and messages[-1] == finishes[0], '缺唯一末尾成功 build-finished')
    matches = []
    for message in messages:
        if message.get('reason') != 'compiler-artifact' or not message.get('profile', {}).get('test'):
            continue
        t = message.get('target', {})
        if t.get('name') != target['name']:
            continue
        require(t.get('kind') == target['kind'], 'Cargo target kind 不符')
        require(Path(t.get('src_path', '')).resolve() == (root / target['src']).resolve(), '非当前 source target')
        require(Path(message.get('manifest_path', '')).resolve() == root / 'rust/Cargo.toml', '非当前 crate')
        require(sorted(message.get('features', [])) == target['features'], 'Cargo features 不符')
        executable = message.get('executable')
        require(isinstance(executable, str) and executable, '缺实际 executable')
        path = Path(executable)
        require(path.is_absolute() and not path.is_symlink(), 'binary 路径无效')
        path = path.resolve()
        require(path.is_relative_to(target_dir.resolve() / 'debug/deps'), '非本次独占 target binary')
        require(str(path) in message.get('filenames', []), 'binary 不在 Cargo filenames')
        require(message.get('fresh') is False, '本次独占 target 不接受旧 fresh binary')
        require(path.is_file() and os.access(path, os.X_OK), 'binary 不存在或不可执行')
        matches.append(path)
    require(len(matches) == 1, '测试 binary 缺失或不唯一')
    return matches[0]


def file_identity(path):
    info = path.stat()
    require(stat.S_ISREG(info.st_mode) and not path.is_symlink(), 'binary 不是普通文件')
    return {'path': str(path), 'sha256': digest(path.read_bytes()), 'device': info.st_dev,
            'inode': info.st_ino, 'size': info.st_size, 'mtime_ns': info.st_mtime_ns, 'mode': info.st_mode}


def verify_list(raw: bytes, cases):
    require(raw.strip(), '缺 list 原件')
    names = []
    for line in raw.decode().splitlines():
        if line.endswith(': test'):
            names.append(line[:-6])
    require(names and len(names) == len(set(names)), 'list 为空或存在重复名')
    for full in cases:
        require(names.count(full) == 1, '完整测试名零匹配或重复：' + full)
        require(sum(n.split('::')[-1] == full.split('::')[-1] for n in names) == 1, '短名歧义：' + full)
    return names


def verify_exact(raw: bytes, name: str):
    text = raw.decode()
    require(re.search(r'^test ' + re.escape(name) + r' \.\.\. ok$', text, re.M), '缺具名 exact 成功原件')
    require(re.search(r'test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured;', text),
            'exact 零运行、ignored 或非单项')


def telemetry_command(supervisor, root, name, argv, cwd, env):
    """telemetry 原件和实际命令原件分开，不让 telemetry 文字混进 Cargo JSON。"""
    out = supervisor.output
    capture = ('out=$1; err=$2; receipt=$3; shift 3; '
               '"$@" > "$out" 2> "$err"; rc=$?; printf "%s\\n" "$rc" > "$receipt"; exit "$rc"')
    row = supervisor.run(name, [str(root / '.github/scripts/run-cargo-with-telemetry.sh'),
                               'bash', '-c', capture, '--', str(out / (name + '.command.stdout')),
                               str(out / (name + '.command.stderr')), str(out / (name + '.command.exit')),
                               *argv], cwd, env)
    row['command_argv'] = list(map(str, argv))
    receipt = out / (name + '.command.exit')
    row['command_exit'] = int(receipt.read_text()) if receipt.exists() else None
    row['command_status'] = ('NOT_RUN' if not (out / (name + '.command.stdout')).exists()
                             else 'INTERRUPTED' if row['command_exit'] is None else 'EXITED')
    if name == 'fixture':
        row['fixture_meaning'] = FIXTURE_EXITS.get(row['command_exit'], '未退出或未知退出码')
    save(out / 'steps.json', supervisor.records)
    successful(row)
    require(row['command_exit'] == 0, '缺实际命令成功退出：' + name)
    return (out / (name + '.command.stdout')).read_bytes()


def planned_steps():
    names = ['install-elan', 'rust-version', 'cargo-version', 'lean-version']
    for key, target in TARGETS.items():
        names += [key + '-build', key + '-list']
        names += [key + '-exact-' + str(i) for i in range(len(target['cases']))]
    return names + ['lean-single', 'fixture']


def execute_checks(root, supervisor):
    env = dict(os.environ)
    # 不继承外部 features/target 注入或 telemetry 测试采样覆盖。
    for key in ('CARGO_TARGET_DIR', 'CARGO_TELEMETRY_TEST_SAMPLE_SECONDS', 'RUSTFLAGS',
                'CARGO_ENCODED_RUSTFLAGS'):
        env.pop(key, None)
    env['CARGO_INCREMENTAL'] = '0'
    env['CARGO_BUILD_JOBS'] = '1'
    env['PATH'] = str(Path.home() / '.elan/bin') + os.pathsep + env.get('PATH', '')
    successful(supervisor.run('install-elan', ['bash', '-o', 'pipefail', '-c',
        'curl https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -sSf | bash -s -- -y --default-toolchain none'], root, env))
    for name, argv, cwd in (('rust-version', ['rustc', '-vV'], root),
                             ('cargo-version', ['cargo', '-V'], root),
                             ('lean-version', ['lake', 'env', 'lean', '--version'], root / 'formal')):
        successful(supervisor.run(name, argv, cwd, env))
    tool_files = {}
    for tool in ('cargo', 'rustc', 'lake', 'python3'):
        found = shutil.which(tool, path=env['PATH'])
        require(found is not None, '缺工具：' + tool)
        resolved = Path(found).resolve()
        tool_files[tool] = {'resolved': str(resolved), 'sha256': digest(resolved.read_bytes())}
    save(supervisor.output / 'toolchains.json', {
        'executables': tool_files, 'python': sys.version,
        'os_release': Path('/etc/os-release').read_text(),
        'lean_toolchain_sha256': digest((root / 'formal/lean-toolchain').read_bytes()),
        'versions': ['rust-version.stdout', 'cargo-version.stdout', 'lean-version.stdout'],
        'note': 'cargo/rustc/lake 可能是 rustup/elan 代理；真实版本以同次版本原件为准'})
    # 原 telemetry 的 du target 要求目录存在；仅在临时 runner 中创建空目录。
    (root / 'rust/target').mkdir(exist_ok=True)
    (root / 'formal/target').mkdir(exist_ok=True)
    (root / 'target').mkdir(exist_ok=True)
    for key, target in TARGETS.items():
        verify_source_names(root, target['cases'])
        target_dir = root / 'rust/target' / ('issue1393-' + key)
        target_dir.mkdir(exist_ok=False)
        env['CARGO_TARGET_DIR'] = str(target_dir)
        build_argv = ['cargo', 'test', '--locked', *target['args'], '--no-run',
                      '--jobs', '1', '--message-format=json']
        raw = telemetry_command(supervisor, root, key + '-build', build_argv, root / 'rust', env)
        binary = select_binary(raw, target, root, target_dir)
        identity = file_identity(binary)
        save(supervisor.output / (key + '-binary.json'), identity)
        require(file_identity(binary) == identity, 'list 前 binary 已变化')
        listing = telemetry_command(supervisor, root, key + '-list', [str(binary), '--list'], root / 'rust', env)
        verify_list(listing, target['cases'])
        save(supervisor.output / (key + '-selection.json'), {'features': target['features'],
                                                          'names': list(target['cases']), 'binary': identity})
        for index, name in enumerate(target['cases']):
            require(file_identity(binary) == identity, 'exact 前 binary 已变化')
            raw = telemetry_command(supervisor, root, key + '-exact-' + str(index),
                                    [str(binary), name, '--exact', '--nocapture', '--test-threads=1'], root / 'rust', env)
            verify_exact(raw, name)
            require(file_identity(binary) == identity, 'exact 后 binary 已变化')
    telemetry_command(supervisor, root, 'lean-single',
                      ['lake', 'env', 'lean', 'Origin/RawInclusionInitialization.lean'], root / 'formal', env)
    telemetry_command(supervisor, root, 'fixture', ['python3', 'scripts/check_fixture_drift.py'], root, env)


def package(output: Path, budget: Budget):
    """原件缺失/截止/ZIP 错误均抛错；成功摘要不把自身递归收入。"""
    require(budget.remaining() > 0, '封包已超过总截止')
    files = {}
    for path in sorted(output.iterdir()):
        if path.is_file() and path.name not in ('FILES.json', 'evidence.zip', 'PACKAGE.json'):
            require(budget.remaining() > 0, 'FILES 超时')
            files[path.name] = {'sha256': digest(path.read_bytes()), 'bytes': path.stat().st_size}
    require('result.json' in files and 'steps.json' in files, '封包缺结果或步骤原件')
    save(output / 'FILES.json', files)
    with zipfile.ZipFile(output / 'evidence.zip', 'x', compression=zipfile.ZIP_STORED) as archive:
        for name in [*files, 'FILES.json']:
            require(budget.remaining() > 0, 'ZIP 超时')
            archive.write(output / name, name)
    require(budget.remaining() > 0, 'ZIP 超过总截止')
    save(output / 'PACKAGE.json', {'status': 'PACKAGED', 'sha256': digest((output / 'evidence.zip').read_bytes()),
                                 'upload': 'NOT_RECORDED_HERE: 以 GitHub upload-artifact 步骤结论为准'})


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--contract', type=Path, required=True)
    parser.add_argument('--start', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(argv)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    supervisor = None
    budget = None
    code = 1
    result = {'status': 'REFUSED', 'real_execution': 'NOT_RUN', 'package_receipt': 'PACKAGE.json'}
    try:
        contract = json.loads(args.contract.read_text())
        validate_contract(os.environ.get('ISSUE1393_EXECUTE', 'False'), contract)
        require(os.environ.get('GITHUB_ACTIONS') == 'true' and sys.platform == 'linux', '仅具名远端 Linux CI')
        require(os.environ.get('GITHUB_EVENT_NAME') == 'workflow_dispatch'
                and os.environ.get('GITHUB_RUN_ATTEMPT') == contract['run_attempt'], '未知事件或重复执行')
        require(os.environ.get('GITHUB_JOB') == 'necessary-validation', '非具名任务')
        budget = Budget(json.loads(args.start.read_text()))
        save(output / 'contract.json', contract)
        save(output / 'start.json', json.loads(args.start.read_text()))
        supervisor = Supervisor(output, budget)
        for sig in (signal.SIGTERM, signal.SIGINT):
            signal.signal(sig, supervisor.handle_signal)
        root = Path(__file__).resolve().parents[2]
        require(not output.is_relative_to(root), '证据必须在 checkout 外，不能污染绑定')
        bind_source(root, contract, supervisor)
        result['real_execution'] = 'STARTED'
        execute_checks(root, supervisor)
        require(not supervisor.interrupted, '封包前收到信号')
        result['status'] = 'PASS'
        code = 0
    except Exception as exc:
        result['error'] = str(exc)
        if result['real_execution'] == 'STARTED':
            result['status'] = 'FAILED'
    finally:
        done = {r['name'] for r in supervisor.records} if supervisor else set()
        for name in planned_steps():
            if name not in done:
                if supervisor:
                    supervisor.skip(name, '未准入或前序未通过')
        if supervisor is None:
            save(output / 'steps.json', [{'name': n, 'status': 'NOT_RUN', 'exit': None} for n in planned_steps()])
        result['received_signal'] = supervisor.interrupted if supervisor else None
        save(output / 'result.json', result)
        if budget:
            try:
                package(output, budget)
                require(not supervisor or not supervisor.interrupted, '封包期间收到信号')
            except Exception as exc:
                result['status'] = 'FAILED'
                result['package'] = 'FAILED'
                result['package_error'] = str(exc)
                save(output / 'result.json', result)
                save(output / 'PACKAGE.json', {'status': 'FAILED', 'error': str(exc)})
                code = 1
        else:
            save(output / 'PACKAGE.json', {'status': 'NOT_RUN', 'reason': '未准入，没有执行预算'})
        print(json.dumps(result, ensure_ascii=False))
    return code


if __name__ == '__main__':
    sys.exit(main())
