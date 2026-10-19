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
# 根 2026-10-18 本次新定的更窄上限（不是「既有帽」）：管理目录 ROOT-REGISTRY-CAPS-20261018.json，
# 留痕 https://github.com/xy7365527-lang/NewChanlun/issues/1393#issuecomment-6246643501 。
# single/total/files/items 只计 registry 证据正文与版本记录；upload_bytes 计全部 artifact 容器外层合计。
RESOURCE_CAPS = {'single_bytes': 65536, 'total_bytes': 2097152, 'files': 128, 'items': 512,
                 'upload_bytes': 33554432}
CAPS_SOURCE = 'ROOT_NEW_NARROWER_CAPS_20261018'
REGISTRY_EVIDENCE_MODE = 'EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY'
# 官方 actions/upload-artifact 固定 commit（非可变 v4）；容器上界依据其实际 bundle 与锁定依赖。
UPLOAD_ACTION = 'actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02'
CAPS_CONTRACT = {
    'source': CAPS_SOURCE, 'decision': 'PREPARATION_ONLY_NEW_NARROWER_CAPS',
    'registry_evidence': {'mode': REGISTRY_EVIDENCE_MODE, 'additional_network_for_evidence': False,
                          'single_bytes': 65536, 'total_bytes': 2097152, 'files': 128, 'items': 512,
                          'counts': 'REGISTRY_EVIDENCE_BODY_AND_VERSION_RECORDS_ONLY'},
    'upload_bytes': 33554432,
    'upload_counting': 'ALL_ARTIFACT_CONTAINERS_COMBINED_INCLUDING_MAIN_AND_RECEIPT_OUTER_OVERHEAD',
    'upload_bound': 'PRE_UPLOAD_ZIP_STORE_BOUND_98_PLUS_SUM_SIZE_128_2NAME', 'upload_action': UPLOAD_ACTION,
    'compression_level': 0,
    'existing_preserved': {'MAX_MESSAGE_BYTES': 32768, 'MAX_EVENT_BYTES': 1048576, 'total_seconds': 900,
                           'final_reserve_seconds': 90, 'jobs': 1, 'telemetry_seconds': 10,
                           'initial_disk_bytes': 2147483648},
}
ZIP_ARCHIVE_OVERHEAD = 98
ZIP_ENTRY_OVERHEAD = 128
ZIP_MAX_ENTRIES = 65535
ZIP_MAX_BYTES = 0xFFFFFFFF
RECEIPT_REQUEST = 'issue1393-lock-UPLOAD-REQUEST.json'
RECEIPT_FROZEN = 'issue1393-lock-UPLOAD-FROZEN.json'
RECEIPT_ACTUAL = 'issue1393-lock-UPLOAD-ACTUAL.json'
# 收据容器内三件的实现内分配上限（属 32 MiB 合计帽内部切分，不是新增根帽）。
RECEIPT_LIMITS = {RECEIPT_REQUEST: 4096, RECEIPT_FROZEN: 65536, RECEIPT_ACTUAL: 4096}
PACKAGE_JSON_MAX = 4096
REGISTRY_PREFIX = 'registry-'
SCAN_CHUNK_BYTES = 65536
VERSION_SEGMENT_MAX = 256
VALIDATOR_MAX = 4096
SPARSE_CACHE = 'cargo-home/registry/index/index.crates.io-1949cf8c6b5b557f'
GATE_FALSE = b"  ISSUE1393_LOCK_EXECUTE: 'False'\n"
GATE_TRUE = b"  ISSUE1393_LOCK_EXECUTE: 'True'\n"
FIXED = {
    'schema': 'issue1393-lock-preserve/2', 'status': 'BOUND_FOR_ROOT_REVIEW',
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
    'registry_evidence': 'EXISTING_EXCLUSIVE_CARGO_CACHE_ONLY_NO_EXTRA_EVIDENCE_NETWORK_MISSING_IS_PROOF_GAP',
    'registry_network_fallback': 'FORBIDDEN', 'resource_caps': CAPS_CONTRACT,
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


def strict_equal(left, right):
    """类型逐层相同的相等：拒绝 0/False、1/True、1.0/1 等宽松相等冒充根具名数值。"""
    if type(left) is not type(right):
        return False
    if type(left) is dict:
        return left.keys() == right.keys() and all(strict_equal(left[k], right[k]) for k in left)
    if type(left) is list:
        return len(left) == len(right) and all(strict_equal(a, b) for a, b in zip(left, right))
    return left == right


def validate_contract(gate, contract):
    _validate_contract(gate, contract)
    caps_bound()
    for key, expected in FIXED.items():
        require(strict_equal(contract[key], expected), '合同固定值类型或数值不符：' + key)
    require(contract['manifest_sha256'] == MANIFEST_SHA256
            and contract['old_lock_sha256'] == OLD_LOCK_SHA256, '必须原 manifest 和原旧锁；拒绝广泛生成锁')
    for kind in ('approval', 'review'):
        receipt = v.strict_json(contract[kind + '_text'])
        require(type(receipt) is dict and set(receipt) == {'schema', 'scope_sha256', 'text'}
                and receipt['schema'] == 'issue1393-lock-preserve-' + kind + '/2'
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


def packages(raw):
    # lock 不是 registry 证据：字节由读取处的上传帽余量约束，不挪用 registry 四帽。
    rows = tomllib.loads(raw.decode()).get('package', [])
    require(type(rows) is list and 0 < len(rows), 'lock 缺 package')
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


def is_registry_evidence(name):
    """registry 证据正文的唯一命名口径：四个 registry 帽只计这些文件。"""
    return name == 'registry.json' or name.startswith(REGISTRY_PREFIX)


def upload_name(name):
    """upload-artifact 固定 commit 的 validateFilePath 禁字符；名字必须可严格 UTF-8 编码。"""
    try:
        encoded = ('/' + name).encode('utf-8', 'strict')
    except UnicodeError:
        raise v.Refused('上传项名不是合法 UTF-8：' + repr(name)) from None
    require(name and not any(ch in name for ch in '":<>|*?\r\n') and '/' not in name and '\\' not in name,
            '上传项名含禁字符或路径分隔：' + repr(name))
    return encoded


def container_bound(entries):
    """固定 action + compression-level 0 的单个上传容器保守上界（ZIP store，非 ZIP64）。

    每项 = 原文字节 + 128 + 2*UTF8('/'+名)；每容器 + 98。依据 compress-commons 6.0.2：
    LFH 30、CFH 46、数据描述 ≤24（ZIP64 形式）、ZIP64 central extra 28；comment/extra 默认空；
    EOCD 22 + ZIP64 EOCD 56 + locator 20。超过 4 GiB 或项数达 ZIP64 门即拒绝，不外推。
    名字含前导 '/'（destinationPath 原样），比 archiver 去前导斜杠后的实名多 1B，保守。
    """
    rows = list(entries)
    require(len(rows) < ZIP_MAX_ENTRIES, '容器项数达到 ZIP64 门，拒绝')
    total = ZIP_ARCHIVE_OVERHEAD
    for name, size in rows:
        require(type(name) is str and type(size) is int and size >= 0, '容器项非法')
        encoded = upload_name(name)
        require(len(encoded) <= 0xFFFF and size < ZIP_MAX_BYTES, '容器项名/字节超 ZIP 非 ZIP64 范围')
        total += size + ZIP_ENTRY_OVERHEAD + 2 * len(encoded)
    require(total < ZIP_MAX_BYTES, '容器达 4 GiB ZIP64 门，拒绝')
    return total


# 收据容器三件的事前有限上界；实际写入超过各自上限即拒绝上传，不扩大。
RECEIPT_RESERVE = container_bound(RECEIPT_LIMITS.items())


def output_entries(output, budget=None, supervisor=None, hashes=False):
    rows = []
    for path in sorted(output.iterdir(), key=lambda p: p.name):
        if budget is not None:
            check_budget(budget, supervisor)
        info = path.lstat()
        require(stat.S_ISREG(info.st_mode), '上传目录只准普通文件（无目录/symlink/特殊文件）')
        upload_name(path.name)
        row = {'name': path.name, 'bytes': info.st_size}
        if hashes:
            row['sha256'] = file_digest(path, budget, supervisor)
            require(path.lstat().st_size == info.st_size, '上传原件摘要期间变化')
        rows.append(row)
    return rows


def caps_bound():
    require(all(type(n) is int and n > 0 for n in RESOURCE_CAPS.values())
            and set(RESOURCE_CAPS) == {'single_bytes', 'total_bytes', 'files', 'items', 'upload_bytes'},
            '根资源帽尚未绑定')


class EvidenceIO:
    """同一截止内先核帽再读/解析/写/摘要；不截断原件冒充完整。

    两套计数分开：registry 证据正文（registry.json / registry-*）按根四帽；
    整个输出目录按上传容器上界 + 收据容器预留 ≤ upload_bytes。工具链日志、旧源等不计入 2 MiB。
    """
    def __init__(self, output, budget, supervisor=None):
        self.output, self.budget, self.supervisor = output, budget, supervisor
        self.items = 0
        self.check()

    def check(self):
        check_budget(self.budget, self.supervisor)
        caps_bound()
        rows = output_entries(self.output, self.budget, self.supervisor)
        registry = [r for r in rows if is_registry_evidence(r['name'])]
        state = {'files': len(rows), 'bytes': sum(r['bytes'] for r in rows),
                 'registry_files': len(registry), 'registry_bytes': sum(r['bytes'] for r in registry),
                 'main_bound': container_bound((r['name'], r['bytes']) for r in rows)}
        require(state['registry_files'] <= RESOURCE_CAPS['files']
                and state['registry_bytes'] <= RESOURCE_CAPS['total_bytes'], 'registry 证据正文累计字节/文件帽')
        require(state['main_bound'] + RECEIPT_RESERVE <= RESOURCE_CAPS['upload_bytes'], '全部 artifact 容器上界超上传帽')
        check_budget(self.budget, self.supervisor)
        return state

    def headroom(self, name, state=None):
        """再加一个名为 name 的文件时还能放下的最大字节数（全部容器合计帽）。"""
        state = state or self.check()
        return (RESOURCE_CAPS['upload_bytes'] - RECEIPT_RESERVE - state['main_bound']
                - ZIP_ENTRY_OVERHEAD - 2 * len(upload_name(name)))

    def run(self, fn, *args):
        self.check()
        value = fn(*args)
        self.check()
        return value

    def take_items(self, count):
        """registry 版本记录累计 ≤ items；扫描中每保留一条即计。"""
        self.check()
        require(type(count) is int and count >= 0, '项数非法')
        require(self.items + count <= RESOURCE_CAPS['items'], 'registry 版本记录累计项数帽')
        self.items += count

    def open_checked(self, path, root):
        self.check()
        logical_root, path = root.absolute(), path.absolute()
        require(path.is_relative_to(logical_root), '原件路径逃逸')
        root, path = logical_root.resolve(), logical_root.resolve() / path.relative_to(logical_root)
        require(not any(p.is_symlink() for p in (path, *path.parents)
                        if p.is_relative_to(root)), '原件路径逃逸或 symlink')
        info = path.lstat()
        require(stat.S_ISREG(info.st_mode), '原件非普通文件')
        return path, info

    def read(self, path, root, limit):
        """有界读：limit 先于打开核定；读后核大小未变。"""
        path, info = self.open_checked(path, root)
        require(type(limit) is int and 0 <= info.st_size <= limit, '原件超过读取上限')
        self.check()
        with path.open('rb') as stream:
            raw = stream.read(info.st_size + 1)
        self.check()
        require(len(raw) == info.st_size and path.stat().st_size == info.st_size, '原件读取时变化')
        return raw

    def write(self, name, raw):
        state = self.check()
        require(Path(name).name == name and type(raw) is bytes, '固定输出文件名/字节')
        if is_registry_evidence(name):
            require(state['registry_files'] + 1 <= RESOURCE_CAPS['files']
                    and state['registry_bytes'] + len(raw) <= RESOURCE_CAPS['total_bytes'],
                    'registry 证据正文累计字节/文件帽')
        require(len(raw) <= self.headroom(name, state), '写入后全部 artifact 容器上界超上传帽')
        with (self.output / name).open('xb') as stream:
            stream.write(raw)
        self.check()

    def json(self, name, value):
        state = self.check()
        limit = self.headroom(name, state)
        if is_registry_evidence(name):
            limit = min(limit, RESOURCE_CAPS['total_bytes'] - state['registry_bytes'])
        chunks = bytearray()
        for chunk in json.JSONEncoder(ensure_ascii=False, sort_keys=True, indent=2).iterencode(value):
            check_budget(self.budget, self.supervisor)
            part = chunk.encode()
            require(len(chunks) + len(part) + 1 <= limit, 'JSON 原件超过剩余帽：' + name)
            chunks.extend(part)
        chunks.extend(b'\n')
        self.check()
        self.write(name, bytes(chunks))


SPARSE_MAGIC = b'\x03\x02\0\0\0'


def scan_sparse_cache(io, path, root, name, wanted):
    """流式扫描独占 Cargo sparse cache（v3 / index schema 2），只保留 wanted 版本原文。

    不读整文件进内存：只缓冲当前所需段（≤single_bytes）与短 version/validator 段；
    每块前后核同一截止/信号；保留一条即计 items。不联网、不补取。
    返回 (meta, kept, gaps)：kept[version] = (原 cache 偏移, 原文 bytes, 解析记录)。
    """
    path, info = io.open_checked(path, root)
    hasher = hashlib.sha256()
    kept, gaps, seen = {}, [], set()
    magic = bytearray()
    phase, current, validator = 'magic', None, None
    seg, seg_start, seg_len, overflow = bytearray(), 5, 0, False
    offset, scanned = 0, 0

    def limit():
        if phase == 'validator':
            return VALIDATOR_MAX
        if phase == 'version':
            return VERSION_SEGMENT_MAX
        return RESOURCE_CAPS['single_bytes'] if current in wanted else 0

    def feed(piece):
        nonlocal seg_len, overflow
        seg_len += len(piece)
        if overflow:
            return
        if seg_len > limit():
            overflow = True
            seg.clear()
            return
        seg.extend(piece)

    def finish(end):
        nonlocal phase, current, validator, seg_start, seg_len, overflow, scanned
        if phase == 'validator':
            require(not overflow and bytes(seg).startswith((b'etag:', b'last-modified:')), '缺/超长 cache validator 原件')
            validator = bytes(seg)
            phase = 'version'
        elif phase == 'version':
            require(not overflow and seg_len > 0, 'cache 版本段超长或为空')
            current = bytes(seg).decode('utf-8', 'strict')
            require(current not in seen, 'registry 重复版本：' + current)
            seen.add(current)
            phase = 'json'
        else:
            scanned += 1
            if current in wanted:
                if overflow:
                    gaps.append({'version': current, 'reason': '单条版本原文超 single_bytes；不截断冒充完整',
                                 'raw_bytes': seg_len, 'cache_offset': seg_start})
                else:
                    raw = bytes(seg)
                    record = v.strict_json(raw)
                    require(type(record) is dict and record.get('name') == name and record.get('vers') == current,
                            'registry 名/版本不符')
                    io.take_items(1)
                    kept[current] = (seg_start, raw, record)
            phase, current = 'version', None
        seg.clear()
        seg_start, seg_len, overflow = end + 1, 0, False

    with path.open('rb') as stream:
        while True:
            check_budget(io.budget, io.supervisor)
            chunk = stream.read(SCAN_CHUNK_BYTES)
            check_budget(io.budget, io.supervisor)
            if not chunk:
                break
            hasher.update(chunk)
            i = 0
            if phase == 'magic':
                take = chunk[:5 - len(magic)]
                magic.extend(take)
                i = len(take)
                if len(magic) == 5:
                    require(bytes(magic) == SPARSE_MAGIC, '未知 sparse cache 格式')
                    phase = 'validator'
            while i < len(chunk):
                j = chunk.find(b'\0', i)
                feed(chunk[i:] if j < 0 else chunk[i:j])
                if j < 0:
                    break
                finish(offset + j)
                i = j + 1
            offset += len(chunk)
    check_budget(io.budget, io.supervisor)
    require(offset == info.st_size and path.lstat().st_size == info.st_size, 'cache 扫描期间变化')
    require(phase == 'version' and seg_len == 0 and validator is not None, 'sparse cache 截断/结构错误')
    meta = {'path': str(path.relative_to(root.resolve())), 'bytes': offset, 'sha256': hasher.hexdigest(),
            'validator': validator.decode('utf-8', 'strict'), 'entries_scanned': scanned,
            'scope': 'WHOLE_CACHE_FILE_NOT_ARCHIVED_ONLY_IDENTITY_AND_SELECTED_RECORD_RAW'}
    return meta, kept, gaps


def sparse_records(raw, name):
    """内存版仅供 toy 对拍；生产取证一律 scan_sparse_cache 流式路径。"""
    require(raw[:5] == SPARSE_MAGIC, '未知 sparse cache 格式')
    parts = raw[5:].split(b'\0')
    require(len(parts) >= 4 and parts[-1] == b'' and (len(parts) - 2) % 2 == 0, 'sparse cache 截断/结构错误')
    require(parts[0].startswith((b'etag:', b'last-modified:')), '缺 cache validator 原件')
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
    require(type(record.get('deps')) is list and type(record.get('features')) is dict
            and type(record.get('yanked')) is bool, 'registry 缺 deps/features/yanked')
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
               'index': 'sparse+https://index.crates.io/', 'acquisition': REGISTRY_EVIDENCE_MODE,
               'additional_network_for_evidence': False, 'network_fallback': 'FORBIDDEN',
               'caps_source': CAPS_SOURCE,
               'caps': {k: RESOURCE_CAPS[k] for k in ('single_bytes', 'total_bytes', 'files', 'items')},
               'scope': {'records': 'EACH_SELECTED_RECORD_RAW_JSON_COMPLETE_OR_PROOF_GAP',
                         'cache': 'WHOLE_CACHE_FILE_NOT_ARCHIVED; PATH/BYTES/SHA256/VALIDATOR ONLY'},
               'files': [], 'records': [], 'constraint_parents': [],
               'proof_gap': [], 'semver_solution': 'NOT_SOLVED_PENDING_INDEPENDENT_REVIEW'}
    try:
        io.check()
        require(changes is not None, '缺完整新旧依赖图')
        base = work / SPARSE_CACHE
        config_raw = io.read(base / 'config.json', work, RESOURCE_CAPS['single_bytes'])
        io.write('registry-config.raw', config_raw)
        config = io.run(v.strict_json, config_raw)
        summary['config'] = {'artifact': 'registry-config.raw', 'sha256': io.run(digest, config_raw)}
        require(type(config) is dict and config.get('dl') == 'https://static.crates.io/crates'
                and config.get('api') == 'https://crates.io' and not config.get('auth-required', False), '非公共官方 registry 配置')
        manifest_raw = io.read(output / 'Cargo.toml', output, io.headroom('Cargo.toml'))
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
        selected = {}
        names = sorted({key[0] for key in needed if key[2]})
        for index, name in enumerate(names):
            io.check()
            wanted = {key[1]: key for key in needed if key[0] == name and key[2]}
            try:
                relative = sparse_path(name)
                meta, kept, gaps = scan_sparse_cache(io, base / '.cache' / relative, work, name, set(wanted))
                filename = REGISTRY_PREFIX + 'cache-' + str(index) + '.raw'
                extract = bytearray(SPARSE_MAGIC + meta['validator'].encode() + b'\0')
                positions = {}
                for version in sorted(kept, key=lambda ver: kept[ver][0]):
                    check_budget(budget, io.supervisor)
                    extract.extend(version.encode() + b'\0')
                    positions[version] = len(extract)
                    extract.extend(kept[version][1] + b'\0')
                io.write(filename, bytes(extract))
                summary['files'].append(dict(meta, name=name, public_index_path='https://index.crates.io/' + relative,
                                             evidence_artifact=filename, evidence_sha256=io.run(digest, bytes(extract)),
                                             evidence_bytes=len(extract)))
                for gap in gaps:
                    summary['proof_gap'].append(dict(gap, identity=list(wanted[gap['version']])))
                for version, key in sorted(wanted.items()):
                    if any(g['version'] == version for g in gaps):
                        continue
                    try:
                        require(version in kept, 'cache 缺精确版本：' + version)
                        cache_offset, raw, record = kept[version]
                        io.run(validate_registry_record, record, needed[key])
                        selected[key] = record
                        summary['records'].append({'identity': list(key), 'source': REGISTRY, 'raw_complete': True,
                                                   'cache_path': meta['path'], 'cache_offset': cache_offset,
                                                   'raw_json_bytes': len(raw), 'raw_json_sha256': io.run(digest, raw),
                                                   'evidence_artifact': filename, 'evidence_offset': positions[version]})
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
        summary['counts'] = {'records': io.items}
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
                save(output / 'registry.json', failure)
        raise


def preserve_resolution(work, output, budget, supervisor=None):
    errors, changes = [], None
    io = EvidenceIO(output, budget, supervisor)
    try:
        raw = io.read(work / 'rust/Cargo.lock', work, io.headroom('Cargo.lock'))
        io.write('Cargo.lock', raw)
        old = io.read(output / 'Cargo.lock.old', output, io.headroom('Cargo.lock.old'))
        patch = io.run(lambda: ''.join(difflib.unified_diff(old.decode().splitlines(True), raw.decode().splitlines(True),
                                                         fromfile='a/rust/Cargo.lock', tofile='b/rust/Cargo.lock')))
        io.write('Cargo.lock.patch', patch.encode())
        changes = io.run(lock_changes, old, raw)
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


PACKAGE_TAIL = ('FILES.json', 'evidence.zip', 'PACKAGE.json')


def package(output, budget, supervisor=None):
    """封包前先按全部容器上界核定：现有散件 + FILES + evidence.zip 上界 + PACKAGE 上限 + 收据预留。"""
    check_budget(budget, supervisor)
    io = EvidenceIO(output, budget, supervisor)
    rows = [r for r in output_entries(output, budget, supervisor) if r['name'] not in PACKAGE_TAIL]
    files = {}
    for row in rows:
        files[row['name']] = {'sha256': file_digest(output / row['name'], budget, supervisor), 'bytes': row['bytes']}
    require('result.json' in files and 'steps.json' in files, '封包缺结果或步骤原件')
    files_raw = v.canonical(files)
    inner = [(name, item['bytes']) for name, item in files.items()] + [('FILES.json', len(files_raw))]
    # Python zipfile ZIP_STORED（可寻址文件、无 ZIP64）每项 30+46+2*名，≤ 同一上界公式。
    zip_bound = container_bound(inner)
    projected = inner + [('evidence.zip', zip_bound), ('PACKAGE.json', PACKAGE_JSON_MAX)]
    require(container_bound(projected) + RECEIPT_RESERVE <= RESOURCE_CAPS['upload_bytes'],
            '封包后全部 artifact 容器上界超上传帽')
    check_budget(budget, supervisor)
    (output / 'FILES.json').write_bytes(files_raw)
    check_budget(budget, supervisor)
    with v.zipfile.ZipFile(output / 'evidence.zip', 'x', compression=v.zipfile.ZIP_STORED) as archive:
        for name in [*files, 'FILES.json']:
            check_budget(budget, supervisor)
            archive.write(output / name, name)
            check_budget(budget, supervisor)
    require((output / 'evidence.zip').lstat().st_size <= zip_bound, '内层 ZIP 超出事前上界')
    archive_hash = file_digest(output / 'evidence.zip', budget, supervisor)
    io.check()
    check_budget(budget, supervisor)
    receipt = v.canonical({'status': 'PACKAGED', 'sha256': archive_hash,
                           'upload': 'NOT_RECORDED_HERE: 以 GitHub upload-artifact 步骤结论为准'})
    require(len(receipt) <= PACKAGE_JSON_MAX, 'PACKAGE 收据超上限')
    save(output / 'PACKAGE.json', {'status': 'PACKAGED', 'sha256': archive_hash,
                                 'upload': 'NOT_RECORDED_HERE: 以 GitHub upload-artifact 步骤结论为准'})
    io.check()
    check_budget(budget, supervisor)


v.package = package


def write_output(name, value):
    path = os.environ.get('GITHUB_OUTPUT')
    if path:
        with open(path, 'a') as stream:
            stream.write(name + '=' + value + '\n')


def gate(mode):
    """上传前门：在任何真实上传之前核定全部容器合计上界；超限/超时/变化即不上传，不删原件。"""
    allowed, minutes, status = False, 0, {'mode': mode, 'status': 'REFUSED'}
    temp = None
    try:
        require(os.environ.get('ISSUE1393_LOCK_EXECUTE') == 'True', 'False 门：上传 NOT_RUN')
        temp = Path(os.environ['RUNNER_TEMP']).resolve()
        out = temp / 'issue1393-lock-evidence'
        start_path = temp / 'issue1393-lock-start.json'
        require(start_path.lstat().st_size <= 4096, '起点原件超界')
        budget = v.Budget(v.strict_json(start_path.read_bytes()))
        caps_bound()
        if mode == 'upload-gate':
            rows = output_entries(out, budget, None, hashes=True)
            main_bound = container_bound((r['name'], r['bytes']) for r in rows)
            frozen = v.canonical({'schema': 'issue1393-lock-upload-frozen/1', 'directory': out.name,
                                  'entries': rows, 'main_bound': main_bound, 'receipt_reserve': RECEIPT_RESERVE,
                                  'upload_bytes': RESOURCE_CAPS['upload_bytes'], 'action': UPLOAD_ACTION})
            require(len(frozen) <= RECEIPT_LIMITS[RECEIPT_FROZEN], '冻结清单超上限')
            require(main_bound + RECEIPT_RESERVE <= RESOURCE_CAPS['upload_bytes'], '主包+收据容器上界超上传帽')
            with (temp / RECEIPT_FROZEN).open('xb') as stream:
                stream.write(frozen)
            status.update(main_bound=main_bound, receipt_reserve=RECEIPT_RESERVE, entries=len(rows),
                          frozen_sha256=digest(frozen))
            reserve = 10
        else:
            require(mode == 'receipt-gate', '未知门')
            outcome = os.environ.get('PAYLOAD_OUTCOME')
            frozen_path = temp / RECEIPT_FROZEN
            if frozen_path.exists():
                require(frozen_path.lstat().st_size <= RECEIPT_LIMITS[RECEIPT_FROZEN], '冻结清单超上限')
                frozen = v.strict_json(frozen_path.read_bytes())
                rows = output_entries(out, budget, None, hashes=True)
                require(rows == frozen['entries'], '上传目录在门后变化；上界失效，拒绝收据上传')
                main_bound = frozen['main_bound']
            else:
                require(outcome != 'success', '主包已上传却无冻结清单')
                main_bound = 0
            sizes = []
            for name, cap in RECEIPT_LIMITS.items():
                path = temp / name
                if path.exists():
                    info = path.lstat()
                    require(stat.S_ISREG(info.st_mode) and info.st_size <= cap, name + ' 超上限或非普通文件')
                    sizes.append((name, info.st_size))
            require(any(n == RECEIPT_ACTUAL for n, _ in sizes), '缺实际上传收据')
            receipt_bound = container_bound(sizes)
            require(main_bound + receipt_bound <= RESOURCE_CAPS['upload_bytes'], '主包+收据容器合计超上传帽')
            status.update(main_bound=main_bound, receipt_bound=receipt_bound)
            reserve = 5
        minutes = max(0, int((budget.remaining() - reserve) // 60))
        require(minutes > 0, '共享截止剩余不足以上传')
        allowed = True
        status['status'] = 'ALLOWED'
    except Exception as exc:
        status['error'] = str(exc)[:500]
    if mode == 'upload-gate' and temp is not None:
        request = v.canonical(dict(status, authority='后续步骤实际 outcome/conclusion；请求不等于成功'))
        if len(request) > RECEIPT_LIMITS[RECEIPT_REQUEST]:
            request = v.canonical({'mode': mode, 'status': 'REFUSED', 'error': 'REQUEST 超上限'})
            allowed = False
        try:
            with (temp / RECEIPT_REQUEST).open('xb') as stream:
                stream.write(request)
        except Exception:
            allowed = False
    print(json.dumps(status, ensure_ascii=False))
    write_output('allowed', str(allowed).lower())
    write_output('minutes', str(max(1, minutes)))
    return 0


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
        require(io.run(digest, io.read(work / 'rust/Cargo.lock', work, io.headroom('Cargo.lock'))) == OLD_LOCK_SHA256,
                '解析前旧锁缺失或改变，禁止调用 Cargo')
        require(io.run(digest, io.read(work / 'rust/Cargo.toml', work, io.headroom('Cargo.toml'))) == MANIFEST_SHA256,
                '解析前 manifest 改变')
        result['resolution'] = 'STARTED'
        v.telemetry_command(supervisor, ROOT, 'update-workspace', resolution_argv(work), work / 'rust', env)
        io.run(validate_generated_lock, io.read(work / 'rust/Cargo.lock', work, io.headroom('Cargo.lock')))
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
    if sys.argv[1:] in (['upload-gate'], ['receipt-gate']):
        sys.exit(gate(sys.argv[1]))
    require(sys.argv[1:] == [], '未知参数')
    sys.exit(main())
