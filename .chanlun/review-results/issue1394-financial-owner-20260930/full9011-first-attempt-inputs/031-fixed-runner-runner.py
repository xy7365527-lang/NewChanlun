#!/usr/bin/env python3
"""#1394 完整9011 A/B fresh S runner。默认纯离线；具名 Root 输入决定首试门。"""
import argparse
import hashlib
import importlib.util
import json
import os
import re
import signal
import stat
import subprocess
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from evidence import (BINARY_SHA, CLOCK_SHA, COUNT, HEAD, PATCH_SHA, SOURCE_SHA, SCOPE,
                      TREE, Rejected, canonical, completion, decode, exact, positive, require,
                      sha, table_rows, validate_contract, validate_material, validate_final,
                      validate_receipt, validate_snapshot)

HERE = Path(__file__).resolve().parent
CHECKOUT = HERE.parents[1]
ORIGINAL = Path('/Users/silencehan/Projects/NewChanlun-1394-content-v3')
INPUTS = ORIGINAL / '.scratch/issue1394-full-history-formal-prep-20261001/inputs'
REFERENCE = ORIGINAL / '.scratch/issue1394-formal-fullclock-prefix16-20261001/release1'
BUILD = Path('/Users/silencehan/Projects/NewChanlun-1394-same-material-20261001/.scratch/issue1394-same-material-20261001')
BINDING = BUILD / 'SOURCE-BINARY-BINDING.json'
BINARY = BUILD / 'target/release/s_structure_session'
INPUT_NAMES = ('envelopes.draft.jsonl', 'raw-inputs.draft.jsonl', 'envelope-index.draft.json',
               'schedule.full9011.draft.json', 'clock-plan.full9011.draft.json',
               'Controller-A.draft.json', 'Controller-B.draft.json', 'source-interface-binding.json')
CODE_NAMES = ('runner.py', 'evidence.py', 'runtime.py')
ROOT_INPUTS = ORIGINAL / '.scratch/issue1394-full9011-root-inputs-20261001'
ROOT_ACCEPTANCE = ROOT_INPUTS / 'ROOT-ACCEPTANCE.json'
ROOT_SHA = '2f1753a4aed09c4fa28163354107a7891241649294813b96bc6ba2d99b0a8de4'
CALLER_SHA = '145874d694753b248c18cd199da6b8d5aa3cff145d1344df7f090c22fbf79378'
S7_SHA = 'b3d8b67b723d881a98dd0f4116fdc9651cf05b207111b19dad26fa69c2c54e69'
Q21_SHA = '50ea5bf865de8a37d8187069d70bee85ca4caac9c759a60ad2e3c717743d4eb1'
RUNTIME_OUTPUT = ORIGINAL / '.scratch/issue1394-full9011-formal-runtime-20261001'
SOCKET_DIR = Path('/private/tmp/nc1394-f9011-20261001')
GAP = 'UNMEASURED：严格 RSS/heap/IO/内部阶段峰值未测；采样最大仅为下界，不是首试准入门'
WRITE_GUARD = None


def file_identity(path):
    path = Path(path)
    before = path.lstat()
    require(stat.S_ISREG(before.st_mode), '禁止 symlink/非普通文件：' + str(path))
    digest = hashlib.sha256()
    with path.open('rb') as source:
        for block in iter(lambda: source.read(1024 * 1024), b''):
            digest.update(block)
    after = path.lstat()
    require((before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns)
            == (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns), 'hash 期间原件变化')
    return dict(path=str(path.absolute()), sha256=digest.hexdigest(), bytes=after.st_size,
                mtime_ns=after.st_mtime_ns, inode=after.st_ino, device=after.st_dev,
                uid=after.st_uid, mode=oct(stat.S_IMODE(after.st_mode)))


def load(path, *, metadata=False):
    return decode(Path(path).read_bytes(), metadata=metadata)


def save(path, value):
    """不可覆盖、fsync 文件及目录；失败原件和 partial 不改名为 complete。"""
    raw_save(path, canonical(value) + b'\n')


def raw_save(path, raw):
    path = Path(path)
    if WRITE_GUARD is not None:
        WRITE_GUARD(path, len(raw))
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'wb') as target:
        target.write(raw)
        target.flush()
        os.fsync(target.fileno())
    directory = os.open(path.parent, os.O_RDONLY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)
    if WRITE_GUARD is not None:
        WRITE_GUARD(path, 0)


def git(*args):
    # 仅只读命令；不继承或输出认证环境、不执行 git apply 写入。
    return subprocess.run(['git', *args], cwd=CHECKOUT, stdin=subprocess.DEVNULL,
                          capture_output=True, check=True, timeout=30).stdout


def root_git(*args):
    return subprocess.run(['git', *args], cwd=ORIGINAL, stdin=subprocess.DEVNULL,
                          capture_output=True, check=True, timeout=30).stdout


def implementation():
    return {name: file_identity(HERE / name)['sha256'] for name in CODE_NAMES}


def check_record(record, path=None):
    actual = file_identity(path or record['path'])
    for key in ('sha256', 'bytes', 'mtime_ns', 'inode'):
        require(exact(actual[key], record[key]), '绑定原件不符：' + actual['path'] + ':' + key)
    return actual


def patch_result(base, section):
    """内存中按精确 unified hunk 验证 baseline blob → candidate；不写 Git/源文件。"""
    source = base.splitlines(keepends=True)
    lines = section.splitlines(keepends=True)
    output, cursor, i, hunks = [], 0, 0, 0
    while i < len(lines):
        line = lines[i]
        if not line.startswith(b'@@ '):
            require(line.startswith((b'diff --git ', b'index ', b'--- a/', b'+++ b/')), 'patch 出现未支持的元数据')
            i += 1
            continue
        match = re.match(rb'@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@', line)
        require(match is not None, '非法 patch hunk')
        old_start, old_count, new_start, new_count = [int(match.group(k) or b'1') for k in range(1, 5)]
        start = old_start - 1 if old_count else old_start
        require(cursor <= start <= len(source), 'patch hunk 重排/越界')
        output.extend(source[cursor:start])
        cursor = start
        require((new_start - 1 if new_count else new_start) == len(output), 'patch 新位置不符')
        changes = []
        i += 1
        while i < len(lines) and not lines[i].startswith(b'@@ '):
            if lines[i].startswith(b'\\ No newline at end of file'):
                require(bool(changes), '非法 no-newline 标记')
                op, value = changes[-1]
                changes[-1] = op, value.removesuffix(b'\n')
            else:
                require(lines[i][:1] in (b' ', b'+', b'-'), '非法 patch hunk 行')
                changes.append((lines[i][:1], lines[i][1:]))
            i += 1
        old_seen, new_seen = 0, 0
        for op, value in changes:
            if op in (b' ', b'-'):
                require(cursor < len(source) and source[cursor] == value, 'patch 不是 baseline 原字节')
                cursor += 1
                old_seen += 1
            if op in (b' ', b'+'):
                output.append(value)
                new_seen += 1
        require((old_seen, new_seen) == (old_count, new_count), 'patch hunk 行数错误')
        hunks += 1
    require(hunks > 0, 'patch 没有实际 hunk')
    output.extend(source[cursor:])
    return b''.join(output)


def binary_binding():
    helper_head = git('rev-parse', 'HEAD').decode().strip()
    require(root_git('rev-parse', 'HEAD').decode().strip() == HEAD, 'Root 生产 HEAD 不符')
    require(root_git('rev-parse', 'HEAD:rust').decode().strip() == TREE, 'Root 生产 Rust tree 不符')
    require(git('rev-parse', 'HEAD:rust').decode().strip() == TREE, '固定 Rust tree 不符')
    require(not git('diff', '--name-only', 'HEAD', '--', 'rust', 's_session').strip(), '受用代码有未提交改动')
    bound = load(BINDING)
    require(bound['rust_tree'] == TREE, 'binary Rust tree 不属于当前实施面')
    require(git('rev-parse', bound['head'] + ':rust').decode().strip() == TREE, '绑定 head 的 Rust tree 不符')
    require(bound['binary']['path'] == str(BINARY) and bound['binary']['sha256'] == BINARY_SHA, '不能凭标签/旧 ed1f184 binary 放行')
    binary = check_record(bound['binary'])
    require(os.access(BINARY, os.X_OK), '绑定 binary 不可执行')
    require(bound['patch']['path'] == str(BUILD / 'candidate.patch')
            and bound['patch']['sha256'] == PATCH_SHA, 'patch 身份不符')
    patch = check_record(bound['patch'])
    names = [item['path'] for item in bound['source']]
    require(len(names) == len(set(names)) and bool(names), 'source 绑定重复/缺失')
    patch_raw = Path(patch['path']).read_bytes()
    patch_names = re.findall(rb'^diff --git a/(\S+) b/(\S+)$', patch_raw, re.M)
    require(all(a == b for a, b in patch_names)
            and {a.decode() for a, _ in patch_names} == set(names)
            and len(patch_names) == len(names), 'patch/source 集合不符')
    sections = {pair[0].decode(): part for pair, part in zip(patch_names,
                [b'diff --git ' + part for part in patch_raw.split(b'diff --git ')[1:]])}
    originals = [file_identity(BINDING), binary, patch]
    for item in bound['source']:
        require(item['path'].startswith('rust/') and '..' not in Path(item['path']).parts, '非法 source 路径')
        # binding.head 已是 candidate 提交；baseline_git_blob 才是 patch 原基线，二者不混用。
        base = git('cat-file', 'blob', item['baseline_git_blob'])
        current = file_identity(CHECKOUT / item['path'])
        require(sha(base) == item['baseline_sha256'], 'baseline source blob/SHA 不符')
        require(current['sha256'] == item['candidate_sha256'] and current['bytes'] == item['bytes'], '当前 source 不是已构建 candidate')
        require(git('show', 'HEAD:' + item['path']) == (CHECKOUT / item['path']).read_bytes(), 'candidate 不属于固定 HEAD')
        require(patch_result(base, sections[item['path']]) == (CHECKOUT / item['path']).read_bytes(), 'patch 并未产生当前 candidate 原字节')
        originals.append(current)
    # --check --reverse 只验证 patch 在 candidate 上完整反向适用，不写 worktree/index。
    git('apply', '--check', '--reverse', str(BUILD / 'candidate.patch'))
    actual = load(BUILD / 'ACTUAL-NONTEST-RUSTC.json')
    command = actual['command']
    require(actual['test'] is False and actual['opt_level'] == 3
            and '--crate-name s_structure_session ' in command
            and '--crate-type bin ' in command and '-C opt-level=3 ' in command
            and re.search(r'(?:^|\s)--test(?:\s|$)', command) is None,
            '缺真实非test release rustc 证据')
    originals.append(file_identity(BUILD / 'ACTUAL-NONTEST-RUSTC.json'))
    return dict(head=HEAD, helper_head=helper_head, rust_tree=TREE, build_baseline_head=bound['head'],
                binary_sha256=BINARY_SHA, patch_sha256=PATCH_SHA,
                binding_sha256=sha(BINDING.read_bytes()), originals=originals)


def material_at(inputs):
    inputs = Path(inputs)
    env = (inputs / INPUT_NAMES[0]).read_bytes()
    clock = (inputs / 'clock-plan.full9011.draft.json').read_bytes()
    require(sha(clock) == CLOCK_SHA, '完整 frozen clock SHA 不符')
    result = validate_material(env, load(inputs / 'schedule.full9011.draft.json'), clock,
                               (inputs / 'raw-inputs.draft.jsonl').read_bytes(),
                               load(inputs / 'envelope-index.draft.json'))
    source_meta = load(inputs / 'source-interface-binding.json')['source']
    require(source_meta['sha256'] == SOURCE_SHA, 'source 元数据 SHA 不符')
    # 仅读输入目录冻结 JSON 的摘要；不打开 source 元数据指向的原历史文件。
    frozen = file_identity(inputs / 'source.bars.frozen.json')
    require(frozen['sha256'] == SOURCE_SHA and frozen['bytes'] == source_meta['bytes'], '输入冻结 source 原件不完整')
    # schedule SHA 使用文件原字节，而非通过改格式制造新身份。
    result['schedule_sha256'] = sha((inputs / 'schedule.full9011.draft.json').read_bytes())
    originals = [file_identity(inputs / name) for name in INPUT_NAMES] + [frozen]
    return result, originals


def directory_identity(path):
    path = Path(path)
    info = path.lstat()
    require(stat.S_ISDIR(info.st_mode) and info.st_uid in (0, os.getuid()), '目录非本 owner/含 symlink：' + str(path))
    return dict(path=str(path), device=info.st_dev, inode=info.st_ino, uid=info.st_uid,
                mode=stat.S_IMODE(info.st_mode))


def ancestors(path):
    """只读记录已有祖先的 owner/inode；不存在的 fresh 路径不创建。"""
    records = []
    cursor = Path(path).parent
    while cursor != cursor.parent:
        if os.path.lexists(cursor):
            records.append(directory_identity(cursor))
        cursor = cursor.parent
    return records


def own_destination(path, *, runtime=False):
    path = Path(path).absolute()
    require('..' not in path.parts, '输出路径含上跳')
    if runtime:
        require(path == RUNTIME_OUTPUT, '仅 Root 精确声明的正式 output')
    else:
        require(path.is_relative_to(HERE) and path != HERE, '离线 prepared 仅本 isolated scratch 子目录')
    ancestors(path)
    return path


def root_inputs():
    """原件 SHA 锁先于 JSON 解码；不读取数据库/凭据，不新造批准或容量文件。"""
    originals = []
    def frozen(path, digest, size=None):
        identity = file_identity(path)
        require(identity['sha256'] == digest and (size is None or identity['bytes'] == size), 'Root 冻结 SHA/bytes 不符：' + str(path))
        originals.append(identity)
        return load(path)
    root = frozen(ROOT_ACCEPTANCE, ROOT_SHA)
    require(type(root['ticket']) is int and root['ticket'] == 1394
            and root['root_current_HEAD'] == HEAD and root['rust_tree'] == TREE,
            'Root 绑定名分/生产 HEAD/Rust tree 不符')
    expected_paths = dict(caller_observation=ROOT_INPUTS / 'CALLER-OBSERVATION.json',
        S7=ROOT_INPUTS / 'TestOnly-frozen9011-fullclock-serial-first-attempt.s-resources.json',
        binary=BINARY, binary_source_binding=BINDING,
        source=INPUTS / 'source.bars.frozen.json', clock=INPUTS / 'clock-plan.full9011.draft.json',
        schedule=INPUTS / 'schedule.full9011.draft.json', envelopes=INPUTS / 'envelopes.draft.jsonl',
        raw_inputs=INPUTS / 'raw-inputs.draft.jsonl', fullinput_check=ROOT_INPUTS / 'FULL9011-INPUT-CHECK.json')
    values = {}
    for key, path in expected_paths.items():
        record = root[key]
        require(record['path'] == str(path), 'Root 文件路径错误：' + key)
        identity = file_identity(path)
        require(identity['sha256'] == record['sha256'] and identity['bytes'] == record['bytes'], 'Root 原件身份错误：' + key)
        originals.append(identity)
        if key in ('caller_observation', 'S7'):
            values[key] = load(path)
    require(root['caller_observation']['sha256'] == CALLER_SHA and root['S7']['sha256'] == S7_SHA,
            'Root caller/S7 SHA 错误')
    caller, resource = values['caller_observation'], values['S7']
    # SHA 锁已覆盖完整值；关键 owner 门再逐值明写，避免内部派生偷换。
    for key, value in dict(exchange_timeout_ms='30000', startup_timeout_ms='30000', cleanup_timeout_ms='30000',
             per_case_observation_deadline_ms='3600000', minimum_host_free_bytes_before_start='6442450944',
             max_owned_artifact_allocated_bytes='4294967296', sampling_period_ms='100', source_count='9011',
             retained_generation_window='4', case_order=['A', 'B'], no_retries=True).items():
        require(exact(caller[key], value), 'caller 逐值门错误：' + key)
    require(set(resource) == {'schema_revision', 's_resources'} and resource['schema_revision'] == 's-structure-resources/1', 'S7 文件形状错误')
    controllers, domains = {}, []
    for label in ('A', 'B'):
        record = root['controllers'][label]
        path = ROOT_INPUTS / f'Controller-{label}.json'
        require(record['path'] == str(path), 'Controller 不属于 Root 具名路径')
        config = frozen(path, record['sha256'], record['bytes'])
        require(config['db'] == str(RUNTIME_OUTPUT / label / 's.sqlite')
                and config['socket'] == str(SOCKET_DIR / (label + '.sock'))
                and config['startup_timeout_ms'] == caller['startup_timeout_ms']
                and config['delivery_retain_generations'] == caller['retained_generation_window']
                and exact(config['s_resources'], resource['s_resources'])
                and config['s_resources']['response_timeout_ms'] == caller['exchange_timeout_ms'], 'Controller 逐值门错误')
        require(config['binary'] == str(BINARY) and config['clock_plan'] == str(INPUTS / 'clock-plan.full9011.draft.json'), 'Controller binary/clock 错误')
        qpath = ROOT_INPUTS / 'query-resources.original-Q21.json'
        require(config['query_resource_config'] == str(qpath), 'Q21 必须为 Root 具名原件')
        qidentity = file_identity(qpath)
        require(qidentity['sha256'] == Q21_SHA, '原 Q21 SHA 错误')
        originals.append(qidentity)
        original = load(INPUTS / f'Controller-{label}.draft.json')
        require(file_identity(original['query_resource_config'])['sha256'] == Q21_SHA, '原 Controller Q21 已变化')
        originals.append(file_identity(original['query_resource_config']))
        allowed_changed = {'db', 'socket', 'binary', 's_resources', 'query_resource_config'}
        require(exact({k:v for k,v in config.items() if k not in allowed_changed},
                      {k:v for k,v in original.items() if k not in allowed_changed}), 'Controller 原语义偏移')
        controllers[label] = config
        domains.append(dict(label=label, db=config['db'], socket=config['socket'],
                            state_dir=str(Path(config['db']).parent / 'controller')))
    contract = dict(schema_revision='issue1394-root-bound-run/1', ticket=1394, scope=SCOPE,
        root_acceptance_sha256=ROOT_SHA, caller_observation_sha256=CALLER_SHA, resource_sha256=S7_SHA,
        s_resources=resource['s_resources'], caller_observation=caller, domains=domains,
        output_dir=str(RUNTIME_OUTPUT), socket_dir=str(SOCKET_DIR), q21_sha256=Q21_SHA,
        source_sha256=root['source']['sha256'], clock_sha256=root['clock']['sha256'],
        envelopes_sha256=root['envelopes']['sha256'], schedule_sha256=root['schedule']['sha256'],
        original_checkpoint_every='100')
    return contract, controllers, sorted({r['path']:r for r in originals}.values(), key=lambda r:r['path'])


def prepare(output):
    output = own_destination(output)
    require(not os.path.lexists(output), 'prepared 输出必须 fresh，禁止覆盖')
    contract, controllers, root_originals = root_inputs()
    material, originals = material_at(INPUTS)
    validate_contract(contract, material, contract)
    binding = binary_binding()
    originals.extend(binding['originals'] + root_originals)
    originals.extend(file_identity(REFERENCE / name) for name in ('harness.py', 'post_check.py', 'report.py', 'build.py'))
    originals.extend(file_identity(HERE / name) for name in CODE_NAMES)
    originals.extend(file_identity(CHECKOUT / 's_session' / name) for name in ('s_service_control.py', 's_socket_client.py'))
    for config in controllers.values():
        for key in ('profile', 'catalog', 'query_resource_config'):
            originals.append(file_identity(config[key]))
    originals = sorted({row['path']: row for row in originals}.values(), key=lambda row: row['path'])
    output.mkdir(parents=True, mode=0o700)
    save(output / 'PREPARED.json', dict(schema_revision='issue1394-prepared/2', status='S_Q_NOT_RUN',
         inputs=str(INPUTS), material=material, binding=binding, implementation=implementation(),
         contract=contract, originals=originals, strict_peaks='UNMEASURED', full_resource_capacity=False))
    save(output / 'ORIGINALS-BEFORE.json', originals)
    after = [file_identity(row['path']) for row in originals]
    save(output / 'ORIGINALS-AFTER.json', after)
    require(exact(after, originals), '离线准备首末原件变化')
    return dict(status='S_Q_NOT_RUN', offline_material='PASS_9011', root_inputs='PASS_EXACT_SHA_VALUES',
                originals_unchanged=True, prepared=str(output), strict_peaks='UNMEASURED',
                full_resource_capacity=False, runtime_review='供 Root 后续审定')


def validate_paths(contract):
    frozen, _, _ = root_inputs()
    require(exact(contract, frozen), '路径计划不是 Root 精确冻结计划')
    output = own_destination(contract['output_dir'], runtime=True)
    require(not os.path.lexists(output), '正式 output 必须 fresh')
    socket_dir = Path(contract['socket_dir'])
    require(socket_dir == SOCKET_DIR and not os.path.lexists(socket_dir), '仅 Root fresh socket 目录')
    records = ancestors(output) + ancestors(socket_dir)
    for domain in contract['domains']:
        for path in (domain['db'], domain['state_dir'], domain['socket']):
            require(not os.path.lexists(path), '不接管已有域：' + path)
        require(len(os.fsencode(domain['socket'])) <= 100, 'socket 路径过长')
        for suffix in ('-wal', '-shm', '.control.lock', '.control-owner.json'):
            require(not os.path.lexists(domain['db'] + suffix), '不能采用已有 DB sidecar')
    return output, sorted({r['path']:r for r in records}.values(), key=lambda r:r['path'])


def check_directories(records):
    for record in records:
        require(exact(directory_identity(record['path']), record), '祖先目录身份发生变化：' + record['path'])


def native_controller_paths(config):
    """read_config 的既有路径解析；保留原 JSON，并只准该原生确定性变换。"""
    result = dict(config)
    for key in ('db', 'socket', 'binary', 'python', 'catalog', 'profile', 'clock_plan', 'browser', 'query_resource_config'):
        result[key] = str(Path(config[key]).resolve())
    return result


def gate_files(prepared_dir):
    prepared = load(Path(prepared_dir) / 'PREPARED.json')
    require(prepared['schema_revision'] == 'issue1394-prepared/2' and prepared['status'] == 'S_Q_NOT_RUN', 'prepared 名分错误')
    require(exact(prepared['implementation'], implementation()), 'runner 变化须重新离线准备')
    for row in prepared['originals']:
        check_record(row)
    material, _ = material_at(prepared['inputs'])
    require(exact(material, prepared['material']), 'prepared 材料被换')
    require(exact(binary_binding(), prepared['binding']), 'binary/source/patch 身份变化')
    contract, _, _ = root_inputs()
    validate_contract(prepared['contract'], material, contract)
    output, directories = validate_paths(contract)
    for root in (output.parent, SOCKET_DIR.parent):
        filesystem = os.statvfs(root)
        require(filesystem.f_bavail * filesystem.f_frsize >= int(contract['caller_observation']['minimum_host_free_bytes_before_start']),
                '开始空闲不足冻结 6GiB owner 门')
    check_directories(directories)
    return prepared, contract, output, directories


def native_client():
    path = CHECKOUT / 's_session/s_socket_client.py'
    spec = importlib.util.spec_from_file_location('issue1394_native_socket_client', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def received_reply(transport, request_raw, response_raw, client):
    require(transport['transport'] == 'Received', 'DeliveryUnknown/不可达不能因持久 Committed 或部分 wire 而放行')
    require(response_raw.endswith(b'\n') and b'\n' not in response_raw[:-1], 'reply 截断/多帧')
    response = client.decode_frame(response_raw)
    require(response == transport['response'], 'exchange 解码不是实际 reply wire')
    return client.validate_reply(client.decode_frame(request_raw), response, producer='S', producer_epoch='1')


def verify_bundle(bundle):
    """只读输出 JSON/原 wire；从不打开 SQLite，也不 import Controller/runtime。"""
    bundle = Path(bundle)
    plan = load(bundle / 'RUN-PLAN.json')
    prepared = load(bundle / 'PREPARED.json')
    require(plan['inputs'] == prepared['inputs'] == str(INPUTS)
            and plan['contract_sha256'] == sha(canonical(prepared['contract']))
            and plan['resource_sha256'] == S7_SHA
            and sha((bundle / 'RESOURCE.json').read_bytes()) == S7_SHA
            and plan['retries'] is False, 'run 原件/顺序绑定错误')
    require(plan['scope'] == SCOPE and plan['count_per_domain'] == COUNT
            and plan['order'] == ['A', 'B'], 'run 范围不是完整9011串行')
    for row in prepared['originals']:
        check_record(row)
    material, _ = material_at(prepared['inputs'])
    require(exact(material, prepared['material']), '离线 verifier 材料身份不符')
    lines = (Path(prepared['inputs']) / 'envelopes.draft.jsonl').read_bytes().splitlines(keepends=True)
    clock = (Path(prepared['inputs']) / 'clock-plan.full9011.draft.json').read_bytes()
    require(exact(prepared['implementation'], implementation()), 'verifier 实现不属于准备时身份')
    client = native_client()
    cases, snapshots = [], {}
    wire_pairs_equal = True
    previous_case_end = None
    for label in ('A', 'B'):
        d = bundle / label
        result = load(d / 'CASE-RESULT.json')
        require(type(result['committed_count']) is int and result['label'] == label and result['status'] == 'VERIFIED'
                and result['committed_count'] == COUNT, 'partial 永远不能 complete')
        caller = prepared['contract']['caller_observation']
        require(all(type(result[k]) is int for k in ('case_begin_ns', 'cleanup_start_ns', 'cleanup_deadline_ns', 'case_end_observed_ns'))
                and result['case_begin_ns'] <= result['cleanup_start_ns'] <= result['case_end_observed_ns'] <= result['cleanup_deadline_ns']
                and result['cleanup_start_ns'] - result['case_begin_ns'] <= int(caller['per_case_observation_deadline_ms']) * 1_000_000
                and result['cleanup_deadline_ns'] - result['cleanup_start_ns'] == int(caller['cleanup_timeout_ms']) * 1_000_000,
                'case/cleanup 绝对时间门错误')
        require(previous_case_end is None or previous_case_end < result['case_begin_ns'], 'A 完整保全前已开始 B')
        previous_case_end = result['case_end_observed_ns']
        require((d / 'Controller.json').read_bytes() == (ROOT_INPUTS / f'Controller-{label}.json').read_bytes(), '运行 Controller 原件偏移')
        lifecycle = load(d / 'LIFECYCLE.json')
        require(lifecycle['launch_nonce'] == plan['domains'][label]['launch_nonce'], '域 launch nonce 不符')
        launches = lifecycle['processes']
        require([r['op'] for r in launches] == ['init', 'serve'], '缺少真实 init/serve Popen')
        require(len({r['pid'] for r in launches}) == 2, 'PID 复用或重复进程记录')
        identity = load(d / 'RUNNING-IDENTITY.json')
        serve = launches[1]
        resource = identity['resource_identity']
        frozen_resource = load(bundle / 'RESOURCE.json')
        require(identity['db'] == str(RUNTIME_OUTPUT / label / 's.sqlite')
                and identity['socket_identity']['path'] == str(SOCKET_DIR / (label + '.sock')), 'RUNNING 物理域错误')
        require(serve['pid'] == identity['pid'] and serve['argv'] == identity['command']
                and serve['native_nonce'] == identity['control_instance_id']
                and serve['resource_sha256'] == resource['sha256'] == sha(canonical(frozen_resource))
                and identity['state'] == 'running', 'PID/argv/nonce/resource SHA 没绑定')
        check_record(lifecycle['resource_original'])
        for process in launches:
            require(process['binary_sha256'] == BINARY_SHA and process['resource_contract_sha256'] == plan['contract_sha256']
                    and process['launch_nonce'] == lifecycle['launch_nonce'] and process['same_Popen_reaped'] is True,
                    '进程缺本域/资源/真实同 Popen reap 绑定')
            require('--protocol-revision' in process['argv']
                    and process['argv'][process['argv'].index('--protocol-revision') + 1] == 's-session/3', '不是正式 init/serve')
        require(plan['domains'][label]['expected_returncode'] == -int(signal.SIGTERM if label == 'A' else signal.SIGINT), '退出信号不是 A TERM/B INT')
        require(launches[0]['returncode'] == 0 and launches[1]['returncode'] == plan['domains'][label]['expected_returncode'], '真实进程终态失败')
        start = load(d / 'START-RESULT.json')
        require(start['state'] == 'ready' and start['initialization'] is not None
                and client.decode_frame((d / 'init.stdout.wire').read_bytes()) == start['initialization'], '缺少实际 init stdout 原件/对应初始化结果')
        stop_request = load(d / 'STOP-REQUEST.json')
        stop = load(d / 'STOP-RESULT.json')
        stopped_status = load(d / 'STOPPED-STATUS.json')
        expected_signal = 'SIGTERM' if label == 'A' else 'SIGINT'
        require(stop_request['action'] == 'Controller.stop' and stop_request['signal'] == expected_signal
                and stop_request['status']['pid'] == identity['pid']
                and stop_request['status']['control_instance_id'] == identity['control_instance_id']
                and stop['state'] == stopped_status['state'] == 'stopped'
                and stop['pid'] == stopped_status['pid'] == identity['pid']
                and stop['signal'] == expected_signal, 'STOP/status/stop 原证据门失败')
        ready_request = (d / lifecycle['ready_wire_stem']).with_suffix('.request.wire').read_bytes()
        ready_wire = (d / lifecycle['ready_wire_stem']).with_suffix('.reply.wire').read_bytes()
        require(ready_request.endswith(b'\n') and ready_wire.endswith(b'\n'), 'Ready wire 截断')
        payload = client.validate_reply(client.decode_frame(ready_request), client.decode_frame(ready_wire), producer='S', producer_epoch='1')
        require(payload['kind'] == 'Ready' and payload['control_instance_id'] == serve['native_nonce'], 'Ready 原 nonce 不符')
        request_files = {p.name for p in d.glob('input-*.request.wire')}
        require(request_files == {f'input-{n}-0.request.wire' for n in range(1, COUNT + 1)}, '请求 wire 有缺条/额外/重试')
        require({p.name for p in d.glob('input-*.persistent.json')}
                == {f'input-{n}.persistent.json' for n in range(1, COUNT + 1)}, '持久证据缺条/多条')
        progress = [decode(raw) for raw in (d / 'PROGRESS.jsonl').read_bytes().splitlines()]
        require(len(progress) == COUNT, '缺完整逐条已验证持久进度')
        previous_commit = 0
        for n, raw in enumerate(lines, 1):
            request = (d / f'input-{n}-0.request.wire').read_bytes()
            response = (d / f'input-{n}-0.reply.wire').read_bytes()
            wire = load(d / f'input-{n}-0.wire.json')
            require(request == raw and wire['send_completed'] is True
                    and wire['request_sha256'] == sha(request) and wire['reply_sha256'] == sha(response), 'wire 原件错误/未完整发送')
            progress_row = progress[n - 1]
            require(type(progress_row['ordinal']) is int and progress_row['ordinal'] == n
                    and progress_row['status'] == 'Committed'
                    and type(wire['begin_ns']) is int and type(wire['end_ns']) is int
                    and type(progress_row['committed_validation_finished_ns']) is int
                    and previous_commit < wire['begin_ns'] <= wire['end_ns'] <= progress_row['committed_validation_finished_ns']
                    and progress_row['committed_validation_finished_ns'] <= result['cleanup_start_ns'], '逐条 wire/持久验证错序')
            previous_commit = progress_row['committed_validation_finished_ns']
            exchange = load(d / f'input-{n}.exchange.json')
            require(exchange.get('error') is None, 'exchange 有异常，不得通过')
            payload = received_reply(exchange['result'], raw, response, client)
            require(exchange['physical_handle_ns'] == 'UNMEASURED'
                    and exchange['internal_Hash_audit_build_store_SQLCOMMIT_ns'] == 'UNMEASURED', '不得把整体 exchange 当内部物理耗时')
            receipt = load(d / f'input-{n}.persistent.json')
            validate_receipt(receipt, raw, n, payload, clock)
            require(progress_row['request_sha256'] == sha(raw[:-1])
                    and progress_row['reply_wire_sha256'] == sha(response)
                    and progress_row['persistent_sha256'] == sha(canonical(receipt)), '进度未绑定逐条原证据')
            if label == 'B':
                wire_pairs_equal &= response == (bundle / 'A' / f'input-{n}-0.reply.wire').read_bytes()
                wire_pairs_equal &= exact(receipt, load(bundle / 'A' / f'input-{n}.persistent.json'))
        snapshots[label] = {}
        for stage in ('initialized', 'final'):
            snapshot = load(d / (stage + '.fullDB.json'))
            validate_snapshot(snapshot)
            snapshots[label][stage] = snapshot
        validate_final(snapshots[label]['final'], lines, clock)
        cases.append(result)
    before = load(bundle / 'ORIGINALS-BEFORE.json')
    after = load(bundle / 'ORIGINALS-AFTER.json')
    require(exact(before, after), '首末原件 hash/mtime/inode 变化')
    for record in before:
        check_record(record)
    checkpoint_files = sorted(bundle.glob('ORIGINALS-CHECKPOINT-*.json'))
    require(len(checkpoint_files) >= 4, '缺中间原件保全')
    for path in checkpoint_files:
        require(exact(load(path)['originals'], before), '中间原件变化')
    labels = {load(path)['label'] for path in checkpoint_files}
    require({'A-ready', 'A-after-stop', 'B-ready', 'B-after-stop', 'final-or-abort'}.issubset(labels), '中间原件必要阶段缺失')
    require(load(bundle / 'SOCKET-DIRECTORY-CLEANUP.json')['removed_owned_directory'] is True, '全局清理门失败')
    require(exact(load(bundle / 'CONTRACT.json'), root_inputs()[0]), '运行计划偏离 Root 冻结输入')
    measurements = load(bundle / 'MEASUREMENT-GAPS.json')
    require(measurements['strict_RSS_heap_IO_stage_peaks'] == 'UNMEASURED', '未测严格峰值不得 PASS')
    comparisons = {stage: exact(snapshots['A'][stage], snapshots['B'][stage]) for stage in ('initialized', 'final')}
    comparisons['all_receipts_and_reply_wire'] = wire_pairs_equal
    outcome = completion(cases, comparisons, True)
    require(outcome['complete9011'], '对拍或清理门失败不得 PASS')
    return dict(**outcome, comparison=comparisons, scope=SCOPE)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    prep = commands.add_parser('prepare', help='纯离线材料与绑定预检；不会建立 Controller')
    prep.add_argument('--output', required=True, type=Path)
    for name in ('gate', 'run'):
        command = commands.add_parser(name, help='绑定具名 Root 输入；run 仅供 Root 后续显式执行')
        command.add_argument('--prepared', required=True, type=Path)
    verify = commands.add_parser('verify', help='只读 JSON/wire verifier，不打开 DB')
    verify.add_argument('--bundle', required=True, type=Path)
    args = parser.parse_args(argv)
    runtime_entered = False
    try:
        if args.command == 'prepare':
            result = prepare(args.output)
        elif args.command == 'verify':
            result = verify_bundle(args.bundle)
        else:
            gate_files(args.prepared)
            if args.command == 'gate':
                result = dict(status='GATE_PASSED', runtime='S_Q_NOT_RUN', run_allowed=True,
                              strict_peaks='UNMEASURED', full_resource_capacity=False, runtime_review='供 Root 后续审定')
            else:
                # 只有完整 gate 成立才 import runtime；runtime 入口会再次 gate，避免直接绕过。
                from runtime import run_ab
                runtime_entered = True
                result = run_ab(args.prepared)
        print(json.dumps(result, ensure_ascii=False))
        return 2 if result.get('status') == 'FAIL_OR_PARTIAL' else 0
    except (Rejected, KeyError, ValueError, OSError, subprocess.SubprocessError) as error:
        status = ('RUN_FAILED_OR_PARTIAL' if runtime_entered else
                  'VERIFICATION_FAILED' if args.command == 'verify' else 'S_Q_NOT_RUN')
        print(json.dumps(dict(status=status,
                              run_allowed=False, complete9011=False,
                              error=str(error)), ensure_ascii=False))
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
