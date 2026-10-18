#!/usr/bin/env python3
"""#1393 保旧锁解析准备。固定 update --workspace；默认拒绝，缺证据拒绝。"""
from __future__ import annotations

import difflib
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import stat
import sys
import time
import tomllib
import types

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = '.github/workflows/issue1393-lock-prepare.yml'
RUNNER = '.github/scripts/issue1393_lock_prepare.py'
TEST_FILE = '.github/scripts/test_issue1393_lock_prepare.py'
REPORT = '.chanlun/review-results/issue1393-lock-preserve-20261018/'
LEGACY_REPORT = '.chanlun/review-results/issue1393-lock-prepare-20261018/'
PREPARE_BASE = '8542c00cb1b1013e8adc0ffdddc54e4d42f894a4'
HELPER = '.github/scripts/issue1393_validate.py'
HELPER_SHA256 = 'a98e7e6f54dacef35476f245a1f938df6cd0a952191fd7ea67938dd7185b1830'
SOURCE_COMMIT = '8e50d40933c2a6a25f882481f3c46aa885ce6407'
SOURCE_TREE = '0a2e04a4ce22a6b8732ad3b55f57446ee3f066f0'
MANIFEST_SHA256 = 'f129440150dd1eb148ddb07ca5828e14a13c41122e018920c05efa1bea2c8727'
OLD_LOCK_SHA256 = 'c591858344c1ff7c62335e10c0e2f1f5cf9e7447a8caf81cd92e556bac248b62'
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
PUSH_REF = 'refs/heads/codex/1393-lock-preserve-once-20261018'
OPERATION = 'UPDATE_WORKSPACE_PRESERVE_LOCK_ONLY'
# 固定基线未给这些数值；不得把 2 GiB 启动门误作输出帽或凭空扩大权限。
# 管理者须提供既有帽并冻结常量；UNBOUND 时合同/取证均拒绝。测试仅注入 toy 帽。
RESOURCE_CAPS = {'single_bytes': 'UNBOUND', 'total_bytes': 'UNBOUND', 'files': 'UNBOUND',
                 'items': 'UNBOUND', 'upload_bytes': 'UNBOUND'}
SPARSE_CACHE = 'cargo-home/registry/index/index.crates.io-1949cf8c6b5b557f'
GATE_FALSE = b"  ISSUE1393_LOCK_EXECUTE: 'False'\n"
GATE_TRUE = b"  ISSUE1393_LOCK_EXECUTE: 'True'\n"
FIXED = {
    'schema': 'issue1393-lock-preserve/1', 'status': 'BOUND_FOR_ROOT_REVIEW',
    'repository': REPOSITORY, 'ref': PUSH_REF, 'event': 'push',
    'workflow_path': WORKFLOW, 'runner_path': RUNNER, 'job': 'lock-prepare',
    'runner_os': 'Linux', 'runner_label': 'ubuntu-24.04', 'run_attempt': '1',
    'prepared_ref_state': 'E_PREPUBLISHED_FALSE_ON_SAME_REF',
    'activation': 'WORKFLOW_FALSE_TO_TRUE_ONLY',
    'authorization_boundary': 'ROOT_SEPARATE_E_PUBLICATION_AND_F_SINGLE_PUSH',
    'total_seconds': TOTAL_SECONDS, 'jobs': 1, 'initial_disk_bytes': 2147483648,
    'telemetry_seconds': 10, 'toolchain': TOOLCHAIN, 'rustc_commit': RUSTC_COMMIT,
    'cargo_version': CARGO_VERSION, 'command': OPERATION, 'operation': OPERATION,
    'registry': REGISTRY, 'output': 'ARTIFACT_ONLY', 'rerun': 'FORBIDDEN',
    'registry_evidence': 'EXCLUSIVE_SPARSE_CACHE_ONLY_MISSING_IS_PROOF_GAP',
    'registry_network_fallback': 'FORBIDDEN', 'resource_caps': RESOURCE_CAPS,
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
v.MESSAGE_BEGIN = 'ISSUE1393-LOCK-PRESERVE-CONTRACT-BEGIN\n'
v.MESSAGE_END = '\nISSUE1393-LOCK-PRESERVE-CONTRACT-END'
v.CONTRACT_FIXED = FIXED
v.HASH_FIELDS += EXTRA_HASHES
v.GIT_COMMANDS['workflow-prepared'] = ['show', 'HEAD^:' + WORKFLOW]
v.GIT_COMMANDS['legacy-ls-tree'] = ['ls-tree', '-rz', PREPARE_BASE, '--', LEGACY_REPORT]
# 比旧验证入口更窄：ci.yml、旧验证入口及所有其它路径均属于不可变源。
v.is_product_path = lambda p: p not in (WORKFLOW, RUNNER, TEST_FILE) and not p.startswith((REPORT, LEGACY_REPORT))
require, save, digest = v.require, v.save, v.digest


def contract_scope(contract):
    return digest(v.canonical({k: value for k, value in contract.items()
                              if k not in ('approval_text', 'approval_sha256', 'review_text', 'review_sha256')}))


_validate_contract = v.validate_contract


def validate_contract(gate, contract):
    _validate_contract(gate, contract)
    require(all(type(value) is int and value > 0 for value in RESOURCE_CAPS.values()), '既有资源帽尚未绑定')
    require(all(type(value) is int for value in contract['resource_caps'].values()), '资源帽必须精确整数')
    require(contract['manifest_sha256'] == MANIFEST_SHA256
            and contract['old_lock_sha256'] == OLD_LOCK_SHA256, '必须原 manifest 和原旧锁；拒绝广泛生成锁')
    for kind in ('approval', 'review'):
        receipt = v.strict_json(contract[kind + '_text'])
        require(type(receipt) is dict and set(receipt) == {'schema', 'scope_sha256', 'text'}
                and receipt['schema'] == 'issue1393-lock-preserve-' + kind + '/1'
                and receipt['scope_sha256'] == contract_scope(contract)
                and type(receipt['text']) is str and receipt['text'].strip()
                and receipt['text'] != 'UNBOUND', '旧批准/独审或未绑定新合同：' + kind)


v.validate_contract = validate_contract


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
    require(contract['manifest_sha256'] == MANIFEST_SHA256
            and contract['old_lock_sha256'] == OLD_LOCK_SHA256, '必须原 manifest/旧锁摘要')
    # 原实现核完整 S/E/F、单父、message、唯一门、全部 Git 对象及固定 runner/CI/telemetry。
    v.bind_source(root, contract, supervisor, event)
    legacy = tree_files(v.git_read(supervisor, root, 'legacy-ls-tree'))
    current = tree_files((supervisor.output / 'execution-ls-tree.stdout').read_bytes())
    require(legacy and legacy == {p: value for p, value in current.items() if p.startswith(LEGACY_REPORT)},
            '第一次完整原件历史改变')
    for path, key in ((HELPER, 'helper_sha256'), ('rust/Cargo.toml', 'manifest_sha256'),
                      ('rust/Cargo.lock', 'old_lock_sha256'),
                      ('.github/workflows/issue1393-validation.yml', 'validation_workflow_sha256'),
                      (LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt', 'toolchain_evidence_sha256')):
        require(digest((root / path).read_bytes()) == contract[key], path + ' 摘要不符')
    require(contract['helper_sha256'] == HELPER_SHA256, 'helper 必须是固定已审版本')
    evidence = (root / (LEGACY_REPORT + 'RUST-TOOLCHAIN-39091453686.txt')).read_bytes()
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
    old_raw = verified_bytes(root, 'rust/Cargo.lock', entries)
    require(digest(manifest_raw) == MANIFEST_SHA256 and digest(old_raw) == OLD_LOCK_SHA256,
            '运输输入不是原 manifest/旧锁')
    manifest = tomllib.loads(manifest_raw.decode())
    require(manifest.get('dev-dependencies', {}).get('tempfile') == '3', '已批准 tempfile 依赖不得更改')
    # 先保存输入，随后任何运输失败仍有两份一手原件。
    (output / 'Cargo.toml').write_bytes(manifest_raw)
    (output / 'Cargo.lock.old').write_bytes(old_raw)
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
    return ['cargo', '+' + TOOLCHAIN, 'update', '--workspace', '--manifest-path', str(work / 'rust/Cargo.toml')]


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


class EvidenceIO:
    """同一截止内，先核现有输出帽，再读/解析/写/摘要；不截断原件冒充完整。"""
    def __init__(self, output, budget, supervisor=None):
        self.output, self.budget, self.supervisor = output, budget, supervisor
        self.read_bytes = 0
        self.read_items = 0
        self.items = 0
        self.check()

    def check(self):
        check_budget(self.budget, self.supervisor)
        require(all(type(n) is int and n > 0 for n in RESOURCE_CAPS.values()), '既有资源帽尚未绑定')
        files, size = 0, 0
        for path in self.output.iterdir():
            check_budget(self.budget, self.supervisor)
            info = path.lstat()
            require(stat.S_ISREG(info.st_mode), '输出目录只准普通原件')
            files += 1
            size += info.st_size
            require(info.st_size <= RESOURCE_CAPS['single_bytes'], '单原件字节帽')
            require(files <= RESOURCE_CAPS['files'] and size <= RESOURCE_CAPS['total_bytes'], '输出累计字节/数量帽')
        check_budget(self.budget, self.supervisor)
        return files, size

    def run(self, fn, *args):
        self.check()
        value = fn(*args)
        self.check()
        return value

    def take_items(self, count):
        self.check()
        self.items += count
        require(self.items <= RESOURCE_CAPS['items'], '取证累计项数帽')

    def read(self, path, root):
        self.check()
        logical_root, path = root.absolute(), path.absolute()
        require(path.is_relative_to(logical_root), '原件路径逃逸')
        root, path = logical_root.resolve(), logical_root.resolve() / path.relative_to(logical_root)
        require(not any(p.is_symlink() for p in (path, *path.parents)
                                                      if p.is_relative_to(root)), '原件路径逃逸或 symlink')
        info = path.lstat()
        require(stat.S_ISREG(info.st_mode) and info.st_size <= RESOURCE_CAPS['single_bytes'], '原件非普通文件或超单响应帽')
        require(self.read_items + 1 <= RESOURCE_CAPS['items']
                and self.read_bytes + info.st_size <= RESOURCE_CAPS['total_bytes'], '输入累计字节/项数帽')
        self.read_items += 1
        self.read_bytes += info.st_size
        self.check()
        with path.open('rb') as stream:
            raw = stream.read(info.st_size + 1)
        self.check()
        require(len(raw) == info.st_size and path.stat().st_size == info.st_size, '原件读取时变化')
        return raw

    def write(self, name, raw):
        files, size = self.check()
        require(Path(name).name == name and type(raw) is bytes, '固定输出文件名/字节')
        require(len(raw) <= RESOURCE_CAPS['single_bytes'] and size + len(raw) <= RESOURCE_CAPS['total_bytes']
                and files + 1 <= RESOURCE_CAPS['files'], '写入超过输出字节/数量帽')
        with (self.output / name).open('xb') as stream:
            stream.write(raw)
        self.check()

    def json(self, name, value):
        self.check()
        chunks = bytearray()
        for chunk in json.JSONEncoder(ensure_ascii=False, sort_keys=True, indent=2).iterencode(value):
            check_budget(self.budget, self.supervisor)
            part = chunk.encode()
            require(len(chunks) + len(part) + 1 <= RESOURCE_CAPS['single_bytes'], 'JSON 原件超过单响应帽')
            chunks.extend(part)
        chunks.extend(b'\n')
        self.check()
        self.write(name, bytes(chunks))


def packages(raw):
    require(len(raw) <= RESOURCE_CAPS['single_bytes'], 'lock 字节帽')
    rows = tomllib.loads(raw.decode()).get('package', [])
    require(type(rows) is list and 0 < len(rows) <= RESOURCE_CAPS['items'], 'lock 缺 package 或超项数帽')
    result = {}
    for p in rows:
        require(type(p) is dict and type(p.get('name')) is str
                and re.fullmatch(r'[A-Za-z0-9_-]+', p['name'])
                and type(p.get('version')) is str
                and re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.+-]+)?', p['version']), 'package 身份缺失/非法')
        key = (p['name'], p['version'], p.get('source', ''))
        require(key not in result, '重复 package 身份')
        require(key[2] == REGISTRY or (key[2] == '' and p['name'] == 'newchan_rust'), '未知 package source/本地包')
        if key[2]:
            require(type(p.get('checksum')) is str and re.fullmatch(r'[0-9a-f]{64}', p['checksum']), 'package 缺 checksum')
        deps = p.get('dependencies', [])
        require(type(deps) is list and all(type(d) is str for d in deps)
                and len(deps) == len(set(deps)), 'dependencies 类型或重复边')
        result[key] = p
    require(sum(not key[2] for key in result) == 1, '必须唯一根包')
    require(sum(len(p.get('dependencies', [])) for p in rows) <= RESOURCE_CAPS['items'], 'lock 边项数帽')
    return result


def resolved_edges(rows):
    result = []
    by_name = {}
    for key in rows:
        by_name.setdefault(key[0], []).append(key)
    for parent, row in sorted(rows.items()):
        for dep in row.get('dependencies', []):
            match = re.fullmatch(r'([A-Za-z0-9_-]+)(?: ([^ ()]+))?(?: \(([^()]+)\))?', dep)
            require(match, '非法 lock 依赖字符串')
            name, version, source = match.groups()
            require(source in (None, REGISTRY), '依赖字符串含未知 source')
            candidates = [k for k in by_name.get(name, [])
                          if (version is None or k[1] == version) and (source is None or k[2] == source)]
            require(len(candidates) == 1, '依赖目标歧义/悬空：' + dep)
            result.append({'parent': list(parent), 'dependency': dep, 'target': list(candidates[0])})
    return result


def lock_changes(old, new):
    before, after = packages(old), packages(new)
    def edges(rows):
        return sorted([list(k) + [d] for k, p in rows.items() for d in p.get('dependencies', [])])
    old_edges, new_edges = edges(before), edges(after)
    resolved_before, resolved_after = resolved_edges(before), resolved_edges(after)
    def indexed(rows):
        return {(tuple(e['parent']), e['dependency']): e for e in rows}
    bi, ai = indexed(resolved_before), indexed(resolved_after)
    retargeted = [{'parent': list(key[0]), 'dependency': key[1], 'before': bi[key]['target'], 'after': ai[key]['target']}
                  for key in sorted(bi.keys() & ai.keys()) if bi[key]['target'] != ai[key]['target']]
    removed_edges = [e for e in resolved_before if e not in resolved_after]
    added_edges = [e for e in resolved_after if e not in resolved_before]
    changed = before.keys() ^ after.keys()
    changed |= {k for k in before.keys() & after.keys() if before[k] != after[k]}
    for edge in removed_edges + added_edges:
        changed.update((tuple(edge['parent']), tuple(edge['target'])))
    names = sorted({k[0] for k in changed})
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
            'resolved_edges_before': resolved_before, 'resolved_edges_after': resolved_after,
            'resolved_edges_added': added_edges, 'resolved_edges_removed': removed_edges,
            'text_unchanged_target_changed': retargeted,
            'changed_packages': [{'name': n, 'before': [list(k) for k in sorted(before) if k[0] == n],
                                  'after': [list(k) for k in sorted(after) if k[0] == n]} for n in names],
            'classification': {'necessary': ['tempfile="3" 的包存在性和 newchan_rust 根边'],
                               'avoidable': '未求解，等待真实约束独审；不套用旧广泛锁矩阵',
                               'pending_constraints': names, 'minimality': '不承诺零升级或数学最小'},
            'note': '文本边和精确目标边分别完整枚举；仅为待独审候选，正式验证 NOT_RUN'}


def sparse_path(name):
    require(re.fullmatch(r'[A-Za-z0-9_-]+', name), '非法 registry 包名')
    name = name.lower()
    if len(name) < 3:
        return str(len(name)) + '/' + name
    if len(name) == 3:
        return '3/' + name[0] + '/' + name
    return name[:2] + '/' + name[2:4] + '/' + name


def sparse_records(raw, name):
    # Cargo sparse cache v3 / index schema 2；不猜未知缓存格式，不执行内容。
    require(len(raw) <= RESOURCE_CAPS['single_bytes'], 'registry 单响应字节帽')
    require(raw[:5] == b'\x03\x02\0\0\0', '未知 sparse cache 格式')
    parts = raw[5:].split(b'\0')
    require(len(parts) >= 4 and parts[-1] == b'' and (len(parts) - 2) % 2 == 0,
            'sparse cache 截断/结构错误')
    require(parts[0].startswith((b'etag:', b'last-modified:')), '缺 cache validator 原件')
    require((len(parts) - 2) // 2 <= RESOURCE_CAPS['items'], 'registry 版本项数帽')
    records = {}
    for i in range(1, len(parts) - 1, 2):
        version, line = parts[i].decode(), parts[i + 1]
        record = v.strict_json(line)
        require(type(record) is dict and record.get('name') == name
                and record.get('vers') == version and version not in records, 'registry 名/版本/重复不符')
        records[version] = (record, line)
    return records


def validate_registry_record(record, package):
    require(record.get('name') == package['name'] and record.get('vers') == package['version']
            and record.get('cksum') == package['checksum'], 'registry 精确版本/checksum 不符')
    require(type(record.get('deps')) is list and len(record['deps']) <= RESOURCE_CAPS['items']
            and type(record.get('features')) is dict and type(record.get('yanked')) is bool, 'registry 缺 deps/features/yanked')
    require(record.get('rust_version') is None or type(record['rust_version']) is str, 'registry rust_version 类型')
    require('features2' not in record or type(record['features2']) is dict, 'registry features2 类型')
    for dep in record['deps']:
        require(type(dep) is dict and type(dep.get('name')) is str and type(dep.get('req')) is str
                and bool(dep['req']) and type(dep.get('features')) is list
                and all(type(f) is str for f in dep['features'])
                and type(dep.get('optional')) is bool and type(dep.get('default_features')) is bool
                and dep.get('kind') in (None, 'normal', 'build', 'dev')
                and (dep.get('target') is None or type(dep['target']) is str)
                and (dep.get('package') is None or type(dep['package']) is str), 'registry 依赖字段缺失/非法')
        require(dep.get('registry') in (None, 'https://github.com/rust-lang/crates.io-index'), 'registry 约束含未知 source')


def root_constraints(manifest, name):
    rows = []
    for target, table in [(None, manifest), *manifest.get('target', {}).items()]:
        for group in ('dependencies', 'dev-dependencies', 'build-dependencies'):
            for alias, dep in table.get(group, {}).items():
                value = {'version': dep} if type(dep) is str else dep
                if value.get('package', alias) == name:
                    rows.append({'alias': alias, 'group': group, 'target': target, 'declaration': value})
    return rows


def snapshot_registry(work, output, budget, changes, io):
    summary = {'status': 'INCOMPLETE', 'complete': False, 'source': REGISTRY,
               'index': 'sparse+https://index.crates.io/', 'acquisition': 'EXCLUSIVE_CACHE_ONLY',
               'network_fallback': 'FORBIDDEN', 'files': [], 'records': [], 'constraint_parents': [],
               'proof_gap': [], 'semver_solution': 'NOT_SOLVED_PENDING_INDEPENDENT_REVIEW'}
    try:
        io.check()
        require(changes is not None, '缺完整新旧依赖图')
        base = work / SPARSE_CACHE
        config_raw = io.read(base / 'config.json', work)
        io.write('registry-config.raw', config_raw)
        config = io.run(v.strict_json, config_raw)
        summary['config'] = {'artifact': 'registry-config.raw', 'sha256': io.run(digest, config_raw)}
        require(type(config) is dict and config.get('dl') == 'https://static.crates.io/crates'
                and config.get('api') == 'https://crates.io' and not config.get('auth-required', False), '非公共官方 registry 配置')
        manifest_raw = io.read(output / 'Cargo.toml', output)
        manifest = io.run(tomllib.loads, manifest_raw.decode())
        require(manifest.get('dev-dependencies', {}).get('tempfile') == '3', '根 tempfile 声明变化')
        needed, parents = {}, []
        for side in ('before', 'after'):
            rows = {(p['name'], p['version'], p.get('source', '')): p for p in changes['packages_' + side]}
            targets = {tuple(k) for p in changes['changed_packages'] for k in p[side]}
            for key in targets:
                require(key not in needed or needed[key].get('checksum') == rows[key].get('checksum'),
                        '同身份前后 checksum 冲突')
                needed[key] = rows[key]
            for edge in changes['resolved_edges_' + side]:
                if tuple(edge['target']) in targets:
                    parent = tuple(edge['parent'])
                    needed[parent] = rows[parent]
                    parents.append(dict(edge, graph=side))
        io.take_items(len(needed) + len(parents))
        selected = {}
        names = sorted({key[0] for key in needed if key[2]})
        parsed_items = 0
        for index, name in enumerate(names):
            io.check()
            try:
                relative = sparse_path(name)
                raw = io.read(base / '.cache' / relative, work)
                filename = 'registry-cache-' + str(index) + '.raw'
                io.write(filename, raw)
                checksum = io.run(digest, raw)
                summary['files'].append({'name': name, 'path': str(base.relative_to(work) / '.cache' / relative),
                                         'public_index_path': 'https://index.crates.io/' + relative,
                                         'artifact': filename, 'bytes': len(raw), 'sha256': checksum})
                records = io.run(sparse_records, raw, name)
                io.take_items(len(records))
                parsed_items += len(records)
                require(parsed_items <= RESOURCE_CAPS['items'], 'registry 累计版本项数帽')
                for key in sorted(k for k in needed if k[0] == name):
                    try:
                        require(key[1] in records, 'cache 缺精确版本：' + key[1])
                        record, line = records[key[1]]
                        io.run(validate_registry_record, record, needed[key])
                        selected[key] = record
                        summary['records'].append({'identity': list(key), 'source': REGISTRY,
                                                   'cache_artifact': filename, 'cache_sha256': checksum,
                                                   'raw_json': line.decode(), 'raw_json_sha256': io.run(digest, line),
                                                   'record': record})
                    except Exception as exc:
                        summary['proof_gap'].append({'identity': list(key), 'reason': str(exc)[:500]})
                        io.check()
            except Exception as exc:
                summary['proof_gap'].append({'name': name, 'reason': str(exc)[:500]})
                io.check()
        for edge in parents:
            io.check()
            key, target = tuple(edge['parent']), edge['target']
            if not key[2]:
                constraints = root_constraints(manifest, target[0])
                provenance = {'artifact': 'Cargo.toml', 'sha256': io.run(digest, manifest_raw)}
            else:
                record = selected.get(key)
                constraints = [d for d in record['deps'] if (d.get('package') or d['name']) == target[0]] if record else []
                provenance = {'registry_identity': list(key)}
            if not constraints:
                summary['proof_gap'].append({'edge': edge, 'reason': '缺约束父精确版本或对应依赖声明'})
            summary['constraint_parents'].append(dict(edge, constraints=constraints, provenance=provenance,
                                                      version_satisfaction='PENDING_REVIEW'))
        require(not summary['proof_gap'], 'registry 原件/版本/约束父存在 proof_gap')
        summary.update(status='COMPLETE_PENDING_REVIEW', complete=True)
        io.json('registry.json', summary)
        io.check()
        return summary
    except Exception as exc:
        # 紧急失败记录不再读取/解析/散列原件；完整成功只可能来自上面的有界写入。
        failure = {'status': 'FAILED', 'complete': False,
                   'proof_gap': [{'reason': str(exc)[:500]}], 'evidence': '已写原件保留；registry.json 若存在仅为未完成快照'}
        if not summary['proof_gap']:
            summary['proof_gap'] = failure['proof_gap']
        save(output / 'registry-failure.json', failure)
        if (output / 'registry.json').exists():
            save(output / 'registry.json', failure)
        else:
            try:
                summary.update(status='FAILED', complete=False)
                io.json('registry.json', summary)
            except Exception:
                pass
        raise


def preserve_resolution(work, output, budget, supervisor=None):
    errors, changes = [], None
    io = EvidenceIO(output, budget, supervisor)
    try:
        raw = io.read(work / 'rust/Cargo.lock', work)
        io.write('Cargo.lock', raw)
        old = io.read(output / 'Cargo.lock.old', output)
        patch = io.run(lambda: ''.join(difflib.unified_diff(old.decode().splitlines(True), raw.decode().splitlines(True),
                                                         fromfile='a/rust/Cargo.lock', tofile='b/rust/Cargo.lock')))
        io.write('Cargo.lock.patch', patch.encode())
        changes = io.run(lock_changes, old, raw)
        io.take_items(sum(len(changes[k]) for k in ('packages_before', 'packages_after',
                                                   'resolved_edges_before', 'resolved_edges_after')))
        io.json('lock-changes.json', changes)
    except Exception as exc:
        errors.append(str(exc))
    try:
        snapshot_registry(work, output, budget, changes, io)
    except Exception as exc:
        errors.append(str(exc))
    require(not errors, '；'.join(errors))
    io.check()


def validate_generated_lock(raw):
    rows = packages(raw)
    edges = resolved_edges(rows)
    require(any(e['parent'][0] == 'newchan_rust' and e['target'][0] == 'tempfile'
                and e['target'][2] == REGISTRY for e in edges), '生成 lock 缺 tempfile 包或精确根依赖边')


PLANNED = ('source-copy', 'install-toolchain', 'rust-version', 'cargo-version', 'update-workspace', 'preserve-resolution')


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
    io = EvidenceIO(output, budget, supervisor)
    count, size = io.check()
    # ZIP_STORED 及清单/收据也占原输出与上传帽；不因封包复制而扩大帽。
    overhead = 4096 + count * 1024
    require(size + overhead <= RESOURCE_CAPS['single_bytes']
            and size * 2 + overhead <= RESOURCE_CAPS['total_bytes']
            and size * 2 + overhead <= RESOURCE_CAPS['upload_bytes']
            and count + 3 <= RESOURCE_CAPS['files'], '封包/上传字节或数量帽')
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
    io.check()
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
        EvidenceIO(output, budget, supervisor)
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
        # Cargo update 缺 lock 会内部转 generate；调用前强制验证旧锁存在和原字节。
        io = EvidenceIO(output, budget, supervisor)
        require(io.run(digest, io.read(work / 'rust/Cargo.lock', work)) == OLD_LOCK_SHA256,
                '解析前旧锁缺失或改变，禁止调用 Cargo')
        require(io.run(digest, io.read(work / 'rust/Cargo.toml', work)) == MANIFEST_SHA256,
                '解析前 manifest 改变')
        result['resolution'] = 'STARTED'
        v.telemetry_command(supervisor, ROOT, 'update-workspace', resolution_argv(work), work / 'rust', env)
        io.run(validate_generated_lock, io.read(work / 'rust/Cargo.lock', work))
        result['resolution'] = 'UPDATED_PENDING_ROOT_REVIEW'
        result['status'] = 'PASS'
        code = 0
    except Exception as exc:
        result.update(status='FAILED' if result['resolution'] != 'NOT_RUN' else 'REFUSED', error=str(exc))
    finally:
        if (output / 'Cargo.lock.old').is_file() and work.is_dir():
            try:
                preserve_resolution(work, output, budget, supervisor)
                result['registry_evidence'] = 'COMPLETE_PENDING_REVIEW'
                for item in inventory or []:
                    require(digest((ROOT / item['source']).read_bytes()) == item['sha256'], 'checkout 源被修改')
                    if item['source'] != 'rust/Cargo.lock':
                        require(digest((work / item['source']).read_bytes()) == item['sha256'], '临时源被修改')
                supervisor.records.append({'name': 'preserve-resolution', 'status': 'PASS', 'exit': None})
            except Exception as exc:
                result.update(status='FAILED', registry_evidence='FAILED_PROOF_GAP', preservation_error=str(exc))
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
