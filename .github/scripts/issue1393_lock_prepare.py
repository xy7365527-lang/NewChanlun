#!/usr/bin/env python3
"""#1393 独立锁解析准备。唯一生产动作是固定版本 generate-lockfile；默认拒绝。"""
from __future__ import annotations

import difflib
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import sys
import time
import tomllib
import types

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = '.github/workflows/issue1393-lock-prepare.yml'
RUNNER = '.github/scripts/issue1393_lock_prepare.py'
TEST_FILE = '.github/scripts/test_issue1393_lock_prepare.py'
REPORT = '.chanlun/review-results/issue1393-lock-prepare-20261018/'
HELPER = '.github/scripts/issue1393_validate.py'
HELPER_SHA256 = 'a98e7e6f54dacef35476f245a1f938df6cd0a952191fd7ea67938dd7185b1830'
SOURCE_COMMIT = '8e50d40933c2a6a25f882481f3c46aa885ce6407'
SOURCE_TREE = '0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0'
TELEMETRY = '.github/scripts/run-cargo-with-telemetry.sh'
TOOLCHAIN = '1.99.0-x86_64-unknown-linux-gnu'
RUSTC_COMMIT = 'b940084d7eb6a299eb4bfeb8e34901bc051e7ac4'
RUSTC_FIRST = 'rustc 1.99.0 (b940084d7 2026-09-28)'
CARGO_VERSION = 'cargo 1.99.0 (5f94df478 2026-08-27)'
REGISTRY = 'registry+https://github.com/rust-lang/crates.io-index'
TOTAL_SECONDS = 900
FINAL_RESERVE = 90
DIGEST_CHUNK_BYTES = 1024 * 1024
REPOSITORY = 'xy7365527-lang/NewChanlun'
PUSH_REF = 'refs/heads/codex/1393-lock-once-20261018'
GATE_FALSE = b"  ISSUE1393_LOCK_EXECUTE: 'False'\n"
GATE_TRUE = b"  ISSUE1393_LOCK_EXECUTE: 'True'\n"
FIXED = {
    'schema': 'issue1393-lock-prepare/1', 'status': 'BOUND_FOR_ROOT_REVIEW',
    'repository': REPOSITORY, 'ref': PUSH_REF, 'event': 'push',
    'workflow_path': WORKFLOW, 'runner_path': RUNNER, 'job': 'lock-prepare',
    'runner_os': 'Linux', 'runner_label': 'ubuntu-24.04', 'run_attempt': '1',
    'prepared_ref_state': 'E_PREPUBLISHED_FALSE_ON_SAME_REF',
    'activation': 'WORKFLOW_FALSE_TO_TRUE_ONLY',
    'authorization_boundary': 'ROOT_SEPARATE_E_PUBLICATION_AND_F_SINGLE_PUSH',
    'total_seconds': TOTAL_SECONDS, 'jobs': 1, 'initial_disk_bytes': 2147483648,
    'telemetry_seconds': 10, 'toolchain': TOOLCHAIN, 'rustc_commit': RUSTC_COMMIT,
    'cargo_version': CARGO_VERSION, 'command': 'GENERATE_LOCKFILE_ONLY',
    'registry': REGISTRY, 'output': 'ARTIFACT_ONLY', 'rerun': 'FORBIDDEN',
    'formal_validation': 'NOT_RUN',
}
EXTRA_HASHES = ('helper_sha256', 'manifest_sha256', 'old_lock_sha256',
                'validation_workflow_sha256', 'toolchain_evidence_sha256')


def load_fixed_helper():
    # 只读固定路径；先验固定 SHA256，再执行同一份已验字节。合同不能选模块。
    raw = (ROOT / HELPER).read_bytes()
    if hashlib.sha256(raw).hexdigest() != HELPER_SHA256:
        raise RuntimeError('固定 helper 源码摘要不符，拒绝加载')
    module = types.ModuleType('_issue1393_lock_fixed_helper')
    module.__file__ = str(ROOT / HELPER)
    exec(compile(raw, module.__file__, 'exec'), module.__dict__)
    return module


v = load_fixed_helper()
# 独立私有模块实例的固定适配，不修改磁盘旧 runner，也不执行其 main/产品检查。
v.WORKFLOW, v.RUNNER, v.TEST_FILE, v.REPORT = WORKFLOW, RUNNER, TEST_FILE, REPORT
v.GATE_FALSE, v.GATE_TRUE = GATE_FALSE, GATE_TRUE
v.TOTAL_SECONDS, v.FINAL_RESERVE = TOTAL_SECONDS, FINAL_RESERVE
v.PUSH_REF = PUSH_REF
v.MESSAGE_BEGIN = 'ISSUE1393-LOCK-CONTRACT-BEGIN\n'
v.MESSAGE_END = '\nISSUE1393-LOCK-CONTRACT-END'
v.CONTRACT_FIXED = FIXED
v.HASH_FIELDS += EXTRA_HASHES
v.GIT_COMMANDS['workflow-prepared'] = ['show', 'HEAD^:' + WORKFLOW]
# 比旧验证入口更窄：ci.yml、旧验证入口及所有其它路径均属于不可变源。
v.is_product_path = lambda p: p not in (WORKFLOW, RUNNER, TEST_FILE) and not p.startswith(REPORT)
require, save, digest = v.require, v.save, v.digest


def validate_push(event, contract, head):
    """复用 #1488/#1393 identity 判据，仅换固定 ref/job，禁止新分支/强推/重跑。"""
    require(os.environ.get('GITHUB_ACTIONS') == 'true' and sys.platform == 'linux', '仅 GitHub Linux')
    expected = {'RUNNER_OS': 'Linux', 'GITHUB_REPOSITORY': REPOSITORY, 'GITHUB_REF': PUSH_REF,
                'GITHUB_EVENT_NAME': 'push', 'GITHUB_RUN_ATTEMPT': '1', 'GITHUB_JOB': 'lock-prepare',
                'GITHUB_SHA': head, 'GITHUB_WORKFLOW_SHA': head,
                'GITHUB_WORKFLOW_REF': REPOSITORY + '/' + WORKFLOW + '@' + PUSH_REF}
    require(all(os.environ.get(k) == value for k, value in expected.items()), 'GitHub 身份不符')
    require(type(event) is dict and type(event.get('repository')) is dict
            and event['repository'].get('full_name') == REPOSITORY and event.get('ref') == PUSH_REF,
            'push repo/ref 不符')
    require(all(event.get(k) is False for k in ('created', 'deleted', 'forced')), '只允许已存在分支普通 push')
    require(event.get('before') == contract['prepared_commit'], 'before 必须为预发布 E')
    require(type(head) is str and re.fullmatch(r'[0-9a-f]{40}', head)
            and len(set(head)) > 1 and head != contract['prepared_commit'], 'head 非法')
    require(event.get('after') == head and type(event.get('head_commit')) is dict
            and event['head_commit'].get('id') == head, 'head 绑定不符')


v.validate_push = validate_push


def bind_source(root, contract, supervisor, event):
    # 本具名解析合同固定 S；先拒绝伪换源，再由旧全树 identity 核实际 Git 对象。
    require(contract['source_commit'] == SOURCE_COMMIT and contract['source_tree'] == SOURCE_TREE,
            '锁解析 source 必须是固定 commit/tree')
    require(contract['prepared_commit'] != SOURCE_COMMIT, '准备 E 不得等于 source S')
    # 原实现核完整 S/E/F、单父、message、唯一门、全部 Git 对象及固定 runner/CI/telemetry。
    v.bind_source(root, contract, supervisor, event)
    for path, key in ((HELPER, 'helper_sha256'), ('rust/Cargo.toml', 'manifest_sha256'),
                      ('rust/Cargo.lock', 'old_lock_sha256'),
                      ('.github/workflows/issue1393-validation.yml', 'validation_workflow_sha256'),
                      (REPORT + 'RUST-TOOLCHAIN-39091453686.txt', 'toolchain_evidence_sha256')):
        require(digest((root / path).read_bytes()) == contract[key], path + ' 摘要不符')
    require(contract['helper_sha256'] == HELPER_SHA256, 'helper 必须是固定已审版本')
    evidence = (root / (REPORT + 'RUST-TOOLCHAIN-39091453686.txt')).read_bytes()
    require(all(s.encode() in evidence for s in (RUSTC_FIRST, RUSTC_COMMIT, CARGO_VERSION)), '工具链基准原件不符')
    (supervisor.output / 'toolchain-reference.txt').write_bytes(evidence)


def tree_files(raw):
    result = {}
    for entry in raw.decode().split('\0'):
        if entry:
            meta, path = entry.split('\t', 1)
            mode, kind, oid = meta.split()
            result[path] = (mode, kind, oid)
    return result


def verified_bytes(root, relative, entries):
    root = root.resolve()
    path = root / relative
    require(relative in entries and entries[relative][:2] == ('100644', 'blob'), '源必须是普通 Git 文件：' + relative)
    require(path.resolve().is_relative_to(root)
            and not any(p.is_symlink() for p in (path, *path.parents) if p.is_relative_to(root))
            and path.is_file(), '禁止源 symlink')
    raw = path.read_bytes()
    oid = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
    require(oid == entries[relative][2], '源字节与 Git blob 不符：' + relative)
    return raw


def target_sources(manifest, entries):
    require(not any(k in manifest for k in ('workspace', 'patch', 'replace')), '不支持 workspace/替换来源')
    def dependencies(table):
        for group in ('dependencies', 'dev-dependencies', 'build-dependencies'):
            for dep in table.get(group, {}).values():
                if isinstance(dep, dict):
                    require(not any(k in dep for k in ('git', 'path', 'registry', 'workspace')), '依赖必须默认 crates.io')
    dependencies(manifest)
    for table in manifest.get('target', {}).values():
        dependencies(table)
    selected = {'rust/Cargo.toml', 'rust/Cargo.lock'}
    for p in ('rust/src/lib.rs', 'rust/src/main.rs', 'rust/build.rs'):
        if p in entries:
            selected.add(p)
    for kind in ('lib', 'bin', 'test', 'bench', 'example'):
        rows = [manifest[kind]] if kind == 'lib' and kind in manifest else manifest.get(kind, [])
        for row in rows:
            if 'path' in row:
                p = PurePosixPath(row['path'])
                require(not p.is_absolute() and '..' not in p.parts and p.suffix == '.rs', '目标源码路径非法')
                selected.add('rust/' + str(p))
    build = manifest.get('package', {}).get('build')
    require(build in (None, True, False, 'build.rs'), '未知 build 源路径')
    # Cargo 自动发现的入口；只运输真实入口字节，不复制模块树/fixture/历史/数据。
    for p in entries:
        if re.fullmatch(r'rust/(src/bin|tests|examples|benches)/[^/]+\.rs', p) or re.fullmatch(
                r'rust/(src/bin|tests|examples|benches)/[^/]+/main\.rs', p):
            selected.add(p)
    return sorted(selected)


def copy_resolution_source(root, work, output, budget):
    entries = tree_files((output / 'execution-ls-tree.stdout').read_bytes())
    manifest_raw = verified_bytes(root, 'rust/Cargo.toml', entries)
    manifest = tomllib.loads(manifest_raw.decode())
    require(manifest.get('dev-dependencies', {}).get('tempfile') == '3', '已批准 tempfile 依赖不得更改')
    # 先保存输入，随后任何运输失败仍有两份一手原件。
    (output / 'Cargo.toml').write_bytes(manifest_raw)
    (output / 'Cargo.lock.old').write_bytes(verified_bytes(root, 'rust/Cargo.lock', entries))
    inventory = []
    for relative in target_sources(manifest, entries):
        require(budget.remaining(FINAL_RESERVE) > 0, '运输超过共享截止')
        raw = verified_bytes(root, relative, entries)
        target = work / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(raw)
        inventory.append({'source': relative, 'git_blob': entries[relative][2], 'sha256': digest(raw), 'bytes': len(raw)})
    save(output / 'transported-source.json', inventory)
    return inventory


def isolated_environment(work):
    # 不读取/复制宿主 .cargo、registry 凭据或 GitHub token；从空白 allowlist 构造。
    for directory in ('home', 'cargo-home', 'rustup-home', 'bin', 'tmp', 'xdg', 'rust/target'):
        (work / directory).mkdir(parents=True, exist_ok=False)
    for parent in (work / 'rust', work, *work.parents):
        require(not any((parent / '.cargo' / name).exists() for name in ('config', 'config.toml')),
                '临时工作目录祖先存在 Cargo 配置，拒绝读取')
    return {'HOME': str(work / 'home'), 'CARGO_HOME': str(work / 'cargo-home'),
            'RUSTUP_HOME': str(work / 'rustup-home'), 'XDG_CONFIG_HOME': str(work / 'xdg'),
            'TMPDIR': str(work / 'tmp'), 'PATH': str(work / 'bin') + ':/usr/bin:/bin',
            'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8', 'CARGO_BUILD_JOBS': '1',
            'CARGO_INCREMENTAL': '0', 'CARGO_NET_RETRY': '0', 'RUSTUP_MAX_RETRIES': '0',
            'RUSTUP_AUTO_INSTALL': '0', 'RUSTUP_DIST_SERVER': 'https://static.rust-lang.org',
            'RUSTUP_UPDATE_ROOT': 'https://static.rust-lang.org/rustup',
            'CARGO_REGISTRIES_CRATES_IO_PROTOCOL': 'sparse',
            'CARGO_REGISTRIES_CRATES_IO_INDEX': 'sparse+https://index.crates.io/',
            'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null', 'GIT_TERMINAL_PROMPT': '0'}


def resolution_argv(work):
    return ['cargo', '+' + TOOLCHAIN, 'generate-lockfile', '--manifest-path', str(work / 'rust/Cargo.toml')]


def verify_toolchain(rustc, cargo):
    lines = rustc.decode().splitlines()
    require(lines and lines[0] == RUSTC_FIRST, 'rustc 版本不符')
    fields = dict(line.split(': ', 1) for line in lines[1:] if ': ' in line)
    require(fields.get('commit-hash') == RUSTC_COMMIT and fields.get('release') == '1.99.0'
            and fields.get('host') == 'x86_64-unknown-linux-gnu', '完整 rustc 身份不符')
    require(cargo.decode().strip() == CARGO_VERSION, 'cargo 版本不符')


def install_toolchain(root, work, supervisor, env):
    rustup = shutil.which('rustup')
    require(rustup, 'runner 缺 rustup 可执行程序；不自动下载安装脚本')
    proxy = Path(rustup).resolve()
    raw = proxy.read_bytes()
    save(supervisor.output / 'toolchain-fetch.json', {
        'proxy_source': str(proxy), 'proxy_sha256': digest(raw), 'toolchain': TOOLCHAIN,
        'dist_server': env['RUSTUP_DIST_SERVER'], 'update_root': env['RUSTUP_UPDATE_ROOT'],
        'expected_channel_manifest': 'https://static.rust-lang.org/dist/channel-rust-1.99.0.toml',
        'cache': '本次独占空 RUSTUP_HOME；没有导入宿主缓存', 'network': '仅固定官方来源；非网络抓包'})
    for name in ('rustup', 'rustc', 'cargo'):
        target = work / 'bin' / name
        target.write_bytes(raw)
        target.chmod(0o700)
    v.telemetry_command(supervisor, root, 'install-toolchain',
                        ['rustup', 'toolchain', 'install', TOOLCHAIN, '--profile', 'minimal', '--no-self-update'],
                        work / 'rust', env)
    channel = work / 'rustup-home/toolchains' / TOOLCHAIN / 'lib/rustlib/multirust-channel-manifest.toml'
    require(channel.is_file(), '缺 rustup 安装的 channel manifest 原件')
    (supervisor.output / 'rustup-channel-manifest.toml').write_bytes(channel.read_bytes())
    rustc = v.telemetry_command(supervisor, root, 'rust-version', ['rustc', '+' + TOOLCHAIN, '-vV'], work / 'rust', env)
    cargo = v.telemetry_command(supervisor, root, 'cargo-version', ['cargo', '+' + TOOLCHAIN, '-V'], work / 'rust', env)
    verify_toolchain(rustc, cargo)
    binaries = {}
    for name in ('cargo', 'rustc'):
        p = work / 'rustup-home/toolchains' / TOOLCHAIN / 'bin' / name
        binaries[name] = {'sha256': digest(p.read_bytes()), 'path': str(p)}
    save(supervisor.output / 'toolchain-verified.json', binaries)


def packages(raw):
    rows = tomllib.loads(raw.decode()).get('package', [])
    require(type(rows) is list and rows, 'lock 缺 package')
    result = {}
    for p in rows:
        require(type(p.get('name')) is str and type(p.get('version')) is str, 'package 身份缺失')
        key = (p['name'], p['version'], p.get('source', ''))
        require(key not in result, '重复 package 身份')
        result[key] = p
    return result


def lock_changes(old, new):
    before, after = packages(old), packages(new)
    def edges(rows):
        return sorted([list(k) + [d] for k, p in rows.items() for d in p.get('dependencies', [])])
    old_edges, new_edges = edges(before), edges(after)
    return {'packages_before': list(before.values()), 'packages_after': list(after.values()),
            'added': [after[k] for k in sorted(after.keys() - before.keys())],
            'removed': [before[k] for k in sorted(before.keys() - after.keys())],
            'updated_same_identity': [{'before': before[k], 'after': after[k]} for k in sorted(before.keys() & after.keys()) if before[k] != after[k]],
            'version_or_source_changes': [{'name': n, 'before': [list(k) for k in before if k[0] == n],
                                            'after': [list(k) for k in after if k[0] == n]}
                                           for n in sorted({k[0] for k in before} & {k[0] for k in after})
                                           if {k for k in before if k[0] == n} != {k for k in after if k[0] == n}],
            'edges_before': old_edges, 'edges_after': new_edges,
            'edges_added': [x for x in new_edges if x not in old_edges],
            'edges_removed': [x for x in old_edges if x not in new_edges],
            'note': '完整重解析差异；生成成功不等于正式 --locked 验证成功'}


def snapshot_registry(work, output, budget):
    rows = []
    base = work / 'cargo-home/registry/index'
    if base.exists():
        for p in sorted(base.rglob('*')):
            require(budget.remaining() > 0, 'registry 存证超过共享截止')
            require(not p.is_symlink(), 'registry 中禁止 symlink')
            if p.is_file():
                raw = p.read_bytes()
                row = {'path': str(p.relative_to(base)), 'sha256': digest(raw), 'bytes': len(raw)}
                if p.name == 'config.json':
                    row['raw_utf8'] = raw.decode()
                    config = v.strict_json(raw)
                    row['official'] = config.get('dl') == 'https://static.crates.io/crates' and config.get('api') == 'https://crates.io'
                rows.append(row)
    save(output / 'registry.json', {'index': 'sparse+https://index.crates.io/', 'files': rows,
                                   'credentials': '未读取、未导入', 'source': REGISTRY,
                                   'network_observation': '固定环境、registry 原件摘要、工具 stdout/stderr；非逐请求抓包'})
    require(all(r.get('official', True) for r in rows), '实际 registry config 非官方')


def preserve_resolution(work, output, budget):
    candidate = work / 'rust/Cargo.lock'
    errors = []
    try:
        if candidate.is_file():
            raw = candidate.read_bytes()
            (output / 'Cargo.lock').write_bytes(raw)
            old = (output / 'Cargo.lock.old').read_bytes()
            patch = ''.join(difflib.unified_diff(old.decode().splitlines(True), raw.decode().splitlines(True),
                                               fromfile='a/rust/Cargo.lock', tofile='b/rust/Cargo.lock'))
            (output / 'Cargo.lock.patch').write_text(patch)
            save(output / 'lock-changes.json', lock_changes(old, raw))
    except Exception as exc:
        errors.append(str(exc))
    try:
        snapshot_registry(work, output, budget)
    except Exception as exc:
        errors.append(str(exc))
    require(not errors, '；'.join(errors))


def validate_generated_lock(raw):
    rows = packages(raw)
    roots = [p for p in rows.values() if 'source' not in p]
    require(len(roots) == 1 and roots[0]['name'] == 'newchan_rust', 'lock 含未知本地包')
    for p in rows.values():
        if 'source' in p:
            require(p['source'] == REGISTRY and re.fullmatch(r'[0-9a-f]{64}', p.get('checksum', '')), '非官方包或缺 checksum')
    require(any(k[0] == 'tempfile' for k in rows) and any(d.split()[0] == 'tempfile' for d in roots[0].get('dependencies', [])),
            '生成 lock 缺 tempfile 包或根依赖边')


PLANNED = ('source-copy', 'install-toolchain', 'rust-version', 'cargo-version', 'generate-lockfile', 'preserve-resolution')


def check_budget(budget, supervisor=None):
    require(not supervisor or not supervisor.interrupted, '收到信号，禁止成功收尾')
    require(budget.remaining() > 0, '共享截止已到')


def file_digest(path, budget, supervisor=None):
    """读取、更新和最终摘要前后核同一个 900 秒窗口；每块不超过 1 MiB。"""
    check_budget(budget, supervisor)
    value = hashlib.sha256()
    with path.open('rb') as stream:
        while True:
            check_budget(budget, supervisor)
            chunk = stream.read(DIGEST_CHUNK_BYTES)
            check_budget(budget, supervisor)
            if not chunk:
                break
            value.update(chunk)
            check_budget(budget, supervisor)
    check_budget(budget, supervisor)
    result = value.hexdigest()
    check_budget(budget, supervisor)
    return result


def package(output, budget, supervisor=None):
    """仅覆盖固定 helper 私有实例的封包边界；失败保留原件，由 main 写失败散件。"""
    check_budget(budget, supervisor)
    files = {}
    for path in sorted(output.iterdir()):
        if path.is_file() and path.name not in ('FILES.json', 'evidence.zip', 'PACKAGE.json'):
            files[path.name] = {'sha256': file_digest(path, budget, supervisor), 'bytes': path.stat().st_size}
    require('result.json' in files and 'steps.json' in files, '封包缺结果或步骤原件')
    check_budget(budget, supervisor)
    save(output / 'FILES.json', files)
    check_budget(budget, supervisor)
    with v.zipfile.ZipFile(output / 'evidence.zip', 'x', compression=v.zipfile.ZIP_STORED) as archive:
        for name in [*files, 'FILES.json']:
            check_budget(budget, supervisor)
            archive.write(output / name, name)
            check_budget(budget, supervisor)
    archive_hash = file_digest(output / 'evidence.zip', budget, supervisor)
    check_budget(budget, supervisor)
    save(output / 'PACKAGE.json', {'status': 'PACKAGED', 'sha256': archive_hash,
                                 'upload': 'NOT_RECORDED_HERE: 以 GitHub upload-artifact 步骤结论为准'})
    check_budget(budget, supervisor)


v.package = package


def failed_package(output, result, exc, supervisor):
    result.update(status='FAILED', package_error=str(exc),
                  received_signal=supervisor.interrupted if supervisor else None)
    save(output / 'PACKAGE.json', {'status': 'FAILED', 'error': str(exc)})
    save(output / 'result.json', result)


def main():
    # False 在事件读取、临时目录、helper 子进程、checkout 之后入口处再次拒绝。
    if os.environ.get('ISSUE1393_LOCK_EXECUTE', 'False') != 'True':
        print('PREPARED_NOT_AUTHORIZED：全部真实命令 NOT_RUN')
        return 1
    output = Path(os.environ['RUNNER_TEMP']).resolve() / 'issue1393-lock-evidence'
    work = output.parent / 'issue1393-lock-work'
    output.mkdir(exist_ok=False)
    budget = supervisor = None
    inventory = None
    result = {'status': 'REFUSED', 'resolution': 'NOT_RUN', 'formal_validation': 'NOT_RUN',
              'authorization': '合同摘要仅溯源；实际根权限必须在 E 预发布和 F push 前分别核验'}
    code = 1
    try:
        require(not output.is_relative_to(ROOT), '证据必须在 checkout 外')
        start = v.strict_json((output.parent / 'issue1393-lock-start.json').read_bytes())
        budget = v.Budget(start)
        save(output / 'start.json', start)
        event, contract = v.read_push_contract(output)
        save(output / 'contract.json', contract)
        supervisor = v.Supervisor(output, budget)
        for sig in (signal.SIGTERM, signal.SIGINT):
            signal.signal(sig, supervisor.handle_signal)
        bind_source(ROOT, contract, supervisor, event)
        require(budget.remaining(FINAL_RESERVE) > 0, 'checkout/身份核验已耗尽解析窗口')
        require(shutil.disk_usage(output.parent).free >= 2147483648, '2 GiB 起始磁盘门不满足')
        work.mkdir(exist_ok=False)
        copying = {'name': 'source-copy', 'status': 'RUNNING', 'exit': None}
        supervisor.records.append(copying)
        save(output / 'steps.json', supervisor.records)
        try:
            inventory = copy_resolution_source(ROOT, work, output, budget)
            copying['status'] = 'PASS'
        except Exception as exc:
            copying.update(status='FAILED', error=str(exc))
            raise
        env = isolated_environment(work)
        save(output / 'resolver-environment.json', env)
        install_toolchain(ROOT, work, supervisor, env)
        result['resolution'] = 'STARTED'
        v.telemetry_command(supervisor, ROOT, 'generate-lockfile', resolution_argv(work), work / 'rust', env)
        validate_generated_lock((work / 'rust/Cargo.lock').read_bytes())
        result['resolution'] = 'GENERATED_PENDING_ROOT_REVIEW'
        result['status'] = 'PASS'
        code = 0
    except Exception as exc:
        result.update(status='FAILED' if result['resolution'] != 'NOT_RUN' else 'REFUSED', error=str(exc))
    finally:
        if (output / 'Cargo.lock.old').is_file() and work.is_dir():
            try:
                preserve_resolution(work, output, budget)
                for item in inventory or []:
                    require(digest((ROOT / item['source']).read_bytes()) == item['sha256'], 'checkout 源被修改')
                    if item['source'] != 'rust/Cargo.lock':
                        require(digest((work / item['source']).read_bytes()) == item['sha256'], '临时源被修改')
                supervisor.records.append({'name': 'preserve-resolution', 'status': 'PASS', 'exit': None})
            except Exception as exc:
                result.update(status='FAILED', preservation_error=str(exc))
                supervisor.records.append({'name': 'preserve-resolution', 'status': 'FAILED', 'exit': None, 'error': str(exc)})
                code = 1
        done = {r['name'] for r in supervisor.records} if supervisor else set()
        rows = supervisor.records if supervisor else []
        rows.extend({'name': n, 'status': 'NOT_RUN', 'exit': None, 'reason': '未准入/前步失败，禁止重跑'} for n in PLANNED if n not in done)
        if supervisor and supervisor.interrupted:
            result.update(status='FAILED', received_signal=supervisor.interrupted)
            code = 1
        save(output / 'steps.json', rows)
        save(output / 'result.json', result)
        try:
            require(budget is not None, '没有有效预算')
            v.package(output, budget, supervisor)
            check_budget(budget, supervisor)
        except Exception as exc:
            failed_package(output, result, exc, supervisor)
            code = 1
    try:
        summary = json.dumps(result, ensure_ascii=False)
        if budget is not None:
            check_budget(budget, supervisor)
        print(summary)
        # 包括收据及输出耗时；最后成功出口仍须在原窗口内，ZIP 里的旧快照不能作准。
        if budget is not None:
            check_budget(budget, supervisor)
    except Exception as exc:
        failed_package(output, result, exc, supervisor)
        code = 1
        print(json.dumps(result, ensure_ascii=False))
    return code


if __name__ == '__main__':
    sys.exit(main())
