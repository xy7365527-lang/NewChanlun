"""#1406 C6a 离线语义候选。只读取；不启动产品、不签最终 PASS。

可整体嵌入发行 runner 的独立 namespace，入口 verify_runtime。
"""
from __future__ import annotations

import ast
import base64
import re
import builtins
import copy
import io
import importlib.machinery
import pathlib
import hashlib
import importlib.util
import json
import locale
import math
from pathlib import Path
import signal
import sys
import types
from unittest.mock import patch


def require(value, message):
    if not value:
        raise ValueError(message)


def equal(actual, expected, message):
    require(actual == expected, message)


def finite(value):
    return type(value) in (int, float) and math.isfinite(value)


def strict_log_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, '日志重复键')
            result[key] = value
        return result
    def number(value):
        parsed = float(value)
        require(math.isfinite(parsed), '日志非有限数')
        return parsed
    def constant(value):
        raise ValueError('日志非法数值 ' + value)
    return json.loads(raw, object_pairs_hook=pairs, parse_float=number, parse_constant=constant)


def digest_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_driver(checkout, source_hashes):
    """先核全部执行/数据字节，再在不碰全局 import 缓存的私有模块图执行。"""
    checkout = Path(checkout).resolve()
    buffers = {}
    def freeze(relative, wanted):
        require(type(relative) is str and not Path(relative).is_absolute() and
                '..' not in Path(relative).parts, '源码路径越界')
        path = (checkout / relative).resolve()
        require(path.is_relative_to(checkout), '源码 symlink 越界')
        raw = path.read_bytes()
        equal(hashlib.sha256(raw).hexdigest(), wanted, '执行前冻结字节摘要不同 ' + relative)
        buffers[path] = raw
        return raw
    for relative, wanted in source_hashes.items():
        freeze(relative, wanted)
    driver_path = checkout / 's_session/tests/tb03c_query_runtime.py'
    require(driver_path in buffers, '缺 driver 源码锚')
    tree = ast.parse(buffers[driver_path], filename=str(driver_path))
    constants = {}
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id in ('SOURCE_FILES', 'INPUT_HASHES'):
                    constants[target.id] = ast.literal_eval(node.value)
    equal(set(source_hashes), set(constants['SOURCE_FILES']), '精确 11 源码集合')
    require(len(source_hashes) == 11, '源码数量非 11')
    for relative, wanted in constants['INPUT_HASHES'].items():
        freeze('s_session/tests/' + relative, wanted)
    paths = {'c6a_bound_driver': driver_path,
             's_socket_client': checkout / 's_session/s_socket_client.py',
             'tb03c_oracle': checkout / 's_session/tests/tb03c_oracle.py'}
    # 所有冻结可执行 oracle 一并先核、先审 import，含 importlib 动态加载者。
    executable = {driver_path, paths['s_socket_client']} | {
        checkout / 's_session/tests' / relative
        for relative in constants['INPUT_HASHES'] if relative.endswith('.py')}
    standard = sys.stdlib_module_names
    for path in executable:
        require(path in buffers, '递归执行依赖未冻结')
        parsed = ast.parse(buffers[path], filename=str(path))
        for node in ast.walk(parsed):
            if isinstance(node, ast.Import):
                names = [alias.name for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                require(node.level == 0, '未绑定相对递归 import')
                names = [node.module]
            else:
                continue
            for name in names:
                require(name.split('.')[0] in standard or name in paths,
                        '未绑定递归执行依赖 ' + name)
    private_modules = {}
    private_sys = types.ModuleType('sys')
    private_sys.__dict__.update(vars(sys))
    private_sys.path = list(sys.path)
    private_sys.modules = dict(sys.modules)

    class FrozenPath(type(Path())):
        def open(self, mode='r', buffering=-1, encoding=None, errors=None, newline=None):
            raw = buffers.get(Path(self).resolve())
            if raw is not None and mode in ('r', 'rb', 'rt'):
                if 'b' in mode:
                    return io.BytesIO(raw)
                return io.StringIO(raw.decode(locale.getencoding() if encoding in (None, 'locale') else encoding, errors or 'strict'), newline=newline)
            return super().open(mode, buffering, encoding, errors, newline)
    private_pathlib = types.ModuleType('pathlib')
    private_pathlib.__dict__.update(vars(pathlib))
    private_pathlib.Path = FrozenPath

    def execute(module, path):
        require(path in executable, '动态执行路径未冻结')
        module.__file__, module.__package__ = str(path), ''
        module.__dict__['__builtins__'] = {**vars(builtins), '__import__': isolated_import}
        exec(compile(buffers[path], str(path), 'exec'), module.__dict__)
        return module

    class FrozenLoader:
        def __init__(self, path):
            self.path = path
        def create_module(self, spec):
            return None
        def exec_module(self, module):
            execute(module, self.path)
    def frozen_spec(name, location, *args, **kwargs):
        require(not args and not kwargs, '动态加载参数未绑定')
        path = Path(location).resolve()
        require(path in executable, '动态执行依赖未冻结')
        return importlib.machinery.ModuleSpec(name, FrozenLoader(path), origin=str(path))
    private_util = types.ModuleType('importlib.util')
    private_util.__dict__.update(vars(importlib.util))
    private_util.spec_from_file_location = frozen_spec
    private_importlib = types.ModuleType('importlib')
    private_importlib.__dict__.update(vars(importlib))
    private_importlib.util = private_util

    def isolated_import(name, globals=None, locals=None, fromlist=(), level=0):
        require(level == 0, '隔离模块不允许未绑定相对 import')
        if name in paths:
            if name not in private_modules:
                module = types.ModuleType(name)
                private_modules[name] = module
                execute(module, paths[name])
            return private_modules[name]
        if name == 'sys':
            return private_sys
        if name == 'pathlib':
            return private_pathlib
        if name in ('importlib', 'importlib.util'):
            return private_util if name.endswith('.util') and fromlist else private_importlib
        require(name.split('.')[0] in standard, '执行期未绑定递归依赖 ' + name)
        return builtins.__import__(name, globals, locals, fromlist, level)
    return isolated_import('c6a_bound_driver')


def binder_for(driver):
    namespace = {'driver': driver}
    exec(BINDER_CODE, namespace)
    return types.SimpleNamespace(**namespace)


def expected_steps(driver, seed, plan, oracle):
    """完全从冻结原件枚举 20+21 个主写请求，不从运行回执取期望。"""
    rows = {'we5': [], 'we7': []}
    recorded = {}
    for index, step in enumerate(plan['steps']):
        domain, payload = driver.we5_payload(step, recorded)
        rows['we5'].append({'message': f'step-{index:03d}', 'domain': domain,
                            'payload': payload, 'outcome': step['outcome']})
        if step['op'] == 'capture' and step['outcome'] == 'Recorded':
            event = driver.event_at(payload['raw_utf8'], payload['json_pointer'])
            recorded[driver.fact_key(event)] = {'fact_hash': driver.fact_hash(payload['raw_utf8'], payload['json_pointer'])}
    ledger = oracle.IndependentLedger()
    raw = (driver.HERE / 'fixtures/tb03c_allocation/RAW.json').read_text()
    def add(domain, payload, outcome):
        rows['we7'].append({'message': f"we7-{len(rows['we7']):03d}", 'domain': domain,
                            'payload': payload, 'outcome': outcome})
    for index, event in enumerate(ledger.raw['events']):
        pointer, key = f'/events/{index}', driver.fact_key(event)
        fact_hash = driver.fact_hash(raw, pointer)
        external = event['source']['external_fact_id']
        add('E', {'op': 'CaptureExternal', 'raw_utf8': raw, 'json_pointer': pointer,
                  'token': 't-' + external, 'source_event_time': event['source']['source_event_time']}, 'Recorded')
        if index < 3:
            add('B:' + event['book_identity']['chong_id'], {'op': 'ApplyAllocation', 'fact_key': key,
                'allocation_id': event['allocation_id'], 'expected_fact_hash': fact_hash}, 'Applied')
        else:
            add('E', {'op': 'ResolveEvidence', 'evidence_id': 'resolve-' + key,
                     'covered': [{'fact_key': key, 'fact_hash': fact_hash, 'token': 't-' + external}],
                     'frontier': {'source_namespace': driver.SOURCE['source_namespace'],
                                  'source_epoch': driver.SOURCE['source_epoch'],
                                  'cursor': event['source']['source_cursor']},
                     'dependency_version': 'testonly-deps/1'}, 'Resolved')
            for participant in ledger.manifest['participants']:
                allocation = 'netted:' + driver.digest([key, event['allocation_id'], ledger.manifest['manifest_hash'], participant])
                add('B:' + participant['chong_id'], {'op': 'ApplyAllocation', 'fact_key': key,
                    'allocation_id': allocation, 'expected_fact_hash': fact_hash}, 'Applied')
    equal([len(rows[k]) for k in ('we5', 'we7')], [20, 21], '固定完整 41 步')
    return rows


class ProcessLedger:
    def __init__(self, root, binder, provenance, allowed):
        self.root, self.binder, self.allowed = root, binder, allowed
        path = binder.regular_file(root, 'processes.jsonl')
        self.allowed.add('processes.jsonl')
        self.rows = [strict_log_json(line) for line in path.read_bytes().splitlines()]
        binder.verify_processes(self.rows)
        self.start, self.end = provenance['driver_started_monotonic'], provenance['driver_finished_monotonic']
        require(finite(self.start) and finite(self.end) and self.start <= self.end, '外层 driver 区间非法')
        for row in self.rows:
            require(self.start <= row['monotonic'] <= self.end, '子进程事件越出外层 driver 区间')
        self.spawns = [row for row in self.rows if row['event'] == 'spawn']
        self.position = 0
        self.active = {}
        for row in self.rows:
            if row['event'] == 'spawn':
                self.active[row['pid']] = row
            elif row['event'] == 'reaped':
                original = self.active.pop(row['pid'])
                for key in ('stdout', 'stderr'):
                    equal(row[key], original[key], '收割原件归属')
                original['reaped'] = row
        require(not self.active, '未回收子进程')

    def consume(self, command, label):
        require(self.position < len(self.spawns), '缺具名进程')
        row = self.spawns[self.position]
        self.position += 1
        equal(row['command'], command, '具名进程命令顺序 ' + label)
        for stream in ('stdout', 'stderr'):
            expected = f'{self.position:04d}-{label}.{stream}'
            equal(row[stream], expected, '具名进程原件 ' + label)
            self.binder.regular_file(self.root, expected)
            self.allowed.add(expected)
        equal(row['reaped']['label'], label, '收割 label')
        return row, self.binder.regular_file(self.root, row['stdout']).read_bytes()


def page_uint(value, maximum=4194304):
    require(type(value) is str and re.fullmatch(r'0|[1-9][0-9]*', value) is not None,
            '分页整数必须是规范文本')
    require(len(value) <= 20 and int(value) <= maximum, '分页整数超界')
    return int(value)


def page_document(document, descriptor, query, epochs, manifest, digest):
    """独立核对重建文档和固定切面；不调用生产分页验证器。"""
    kind = document.get('kind')
    equal(kind, descriptor['document_kind'], '分页逻辑类型')
    fixed = {}
    def cut(value, domain):
        require(type(value) is dict, 'cut 类型')
        equal(set(value), {'domain_id', 'commit_seq', 'root_hash'}, 'cut 精确字段')
        equal(value['domain_id'], domain, 'cut 域')
        require(type(value['root_hash']) is str and re.fullmatch('[0-9a-f]{64}', value['root_hash']),
                'cut root_hash 必须为小写64hex字符串')
        return page_uint(value['commit_seq'], 2**64 - 1)
    def image(row, domain):
        seq = cut(row['cut'], domain)
        equal(row['image_hash'], digest(row['image']), '重建图像摘要')
        stamp = page_uint(row['commit_ns'], 2**64 - 1)
        return seq, stamp
    def changes(rows, base, end, domain):
        require(type(rows) is list and len(rows) <= 32, '变化数上限')
        previous = -1 if base is None else cut(base, domain)
        stamp = -1
        for row in rows:
            seq, now = image(row, domain)
            equal(seq, previous + 1, '变化连续序号')
            require(now >= stamp, '变化时间倒退')
            previous, stamp = seq, now
        equal(cut(end, domain), previous, '变化终点')
        if rows:
            equal(rows[-1]['cut'], end, '变化末cut')
        else:
            require(base is not None, '无创世变化')
            equal(end, base, '空变化固定cut')
    if kind in ('EconomicView', 'EconomicChanges'):
        domain = document['domain_id']
        equal(set(epochs), {domain}, 'owner 域集合')
        equal(document['writer_epoch'], epochs[domain], 'owner writer epoch')
        fixed[domain] = document['cut']
        if kind == 'EconomicView':
            equal(query['op'], 'ReadView', 'owner query类型')
            _, stamp = image(document, domain)
            if query['cut'] is not None:
                equal(document['cut'], query['cut'], '指定历史cut')
            if query['as_known_ns'] is not None:
                require(stamp <= page_uint(query['as_known_ns'], 2**64-1), '获知时间越界')
        else:
            equal(query['op'], 'Watch', 'owner watch类型')
            changes(document['changes'], query['after_cut'], document['cut'], domain)
    else:
        require(kind in ('EconomicSnapshot', 'EconomicWatch'), '未知文档类型')
        equal((document['session_id'], document['session_generation'], document['manifest_hash']),
              (manifest['session_id'], manifest['session_generation'], digest(manifest)), '公共身份')
        equal(set(document['domains']), set(epochs), '公共域集合')
        for domain, row in document['domains'].items():
            require(row['status'] in ('Available', 'Unavailable'), '公共可用状态')
            fixed[domain] = None
            if row['status'] == 'Unavailable':
                continue
            if kind == 'EconomicSnapshot':
                image(row, domain)
                fixed[domain] = row['cut']
                if query['cuts'] is not None:
                    equal(row['cut'], query['cuts'][domain], '公共指定cut')
                if query['snapshot_token'] is None and query['as_known_ns'] is not None:
                    require(page_uint(row['commit_ns'], 2**64-1) <= page_uint(query['as_known_ns'], 2**64-1),
                            '公共获知时间越界')
            else:
                equal(row['base_cut'], query['cuts'][domain], 'watch base')
                changes(row['changes'], row['base_cut'], row['next_cut'], domain)
                fixed[domain] = row['next_cut']
        if kind == 'EconomicSnapshot':
            equal(query['op'], 'snapshot', 'snapshot query')
            equal(document['completeness'], 'CompleteVector' if all(v is not None for v in fixed.values())
                  else 'PartialVector', '向量完整度')
            body = {k: v for k, v in document.items() if k != 'snapshot_token'}
            equal(document['snapshot_token'], 'economic-snapshot-' + digest(body), 'snapshot token摘要')
            if query['snapshot_token'] is not None:
                equal(document['snapshot_token'], query['snapshot_token'], '原token重读')
            else:
                equal(document['mode'], 'AsKnown' if query['cuts'] is not None or query['as_known_ns'] is not None
                      else 'Current', 'snapshot mode')
                equal(document['as_known_ns'], query['as_known_ns'], 'snapshot as known')
        else:
            equal(query['op'], 'watch', 'watch query')
    require(type(descriptor['fixed_cuts']) is dict, '描述符fixed_cuts类型')
    equal(set(descriptor['fixed_cuts']), set(epochs), '描述符fixed_cuts完整域集合')
    for domain, value in descriptor['fixed_cuts'].items():
        if value is not None:
            cut(value, domain)
    if query.get('cuts') is not None:
        equal(set(query['cuts']), set(epochs), '请求cuts完整域集合')
        for domain, value in query['cuts'].items():
            cut(value, domain)
    equal(descriptor['fixed_cuts'], fixed, '描述符固定切面投影')


def verify_internal_document(driver, records, domain, selector, manifest, epochs):
    """消费一组 owner Open/Page/Close；独立重组，不调用生产分页器。"""
    require(records, 'Q内部缺owner逻辑读')
    request, opened = records.pop(0)
    payload = request['payload']
    equal(set(payload), {'schema_revision', 'op', 'query', 'requested_remaining_ms'}, '内部Open字段')
    equal((payload['schema_revision'], payload['op'], payload['query']),
          ('economic-read-pages/1', 'EconomicReadOpen', selector), '内部Open完整选择器')
    remaining = page_uint(payload['requested_remaining_ms'], 30000)
    require(remaining > 0, '内部Open剩余预算')
    equal(set(opened), {'schema_revision', 'kind', 'descriptor', 'descriptor_sha256'}, '内部Open响应字段')
    equal((opened['schema_revision'], opened['kind']),
          ('economic-read-pages/1', 'EconomicReadOpened'), '内部Open成功')
    desc, dh = opened['descriptor'], opened['descriptor_sha256']
    equal(set(desc), {'schema_revision', 'transfer_id', 'session_id', 'session_generation',
        'manifest_hash', 'producer_id', 'producer_epoch', 'query_sha256', 'document_kind',
        'fixed_cuts', 'owner_epochs', 'document_bytes', 'document_sha256', 'chunk_bytes', 'page_count'},
        '内部描述符字段')
    for key, value in {'schema_revision': 'economic-read-pages/1', 'session_id': manifest['session_id'],
        'session_generation': manifest['session_generation'], 'manifest_hash': driver.digest(manifest),
        'producer_id': domain + ':' + manifest['session_id'], 'producer_epoch': epochs[domain],
        'query_sha256': driver.digest(selector), 'owner_epochs': {domain: epochs[domain]},
        'chunk_bytes': '262144'}.items():
        equal(desc[key], value, '内部描述符绑定 ' + key)
    require(type(desc['transfer_id']) is str and re.fullmatch('[0-9a-f]{32}', desc['transfer_id']),
            '内部transfer身份')
    equal(dh, driver.digest(desc), '内部描述符hash')
    total = page_uint(desc['document_bytes'])
    require(total > 0, '内部正文为空')
    count = (total + 262143) // 262144
    equal(page_uint(desc['page_count'], 16), count, '内部页数')
    raw = bytearray()
    for _ in range(count):
        require(records, 'Q内部缺Page')
        req, reply = records.pop(0)
        p = req['payload']
        ms = page_uint(p.get('requested_remaining_ms'), 30000)
        require(0 < ms <= remaining, '内部预算复活')
        remaining = ms
        equal(p, {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadPage',
            'transfer_id': desc['transfer_id'], 'descriptor_sha256': dh,
            'offset': str(len(raw)), 'requested_remaining_ms': str(ms)}, '内部Page完整请求')
        equal(set(reply), {'schema_revision', 'kind', 'transfer_id', 'descriptor_sha256', 'offset',
            'raw_byte_count', 'chunk_sha256', 'data_base64', 'next_offset', 'done'}, '内部Page字段')
        for key, value in {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadPage',
            'transfer_id': desc['transfer_id'], 'descriptor_sha256': dh, 'offset': str(len(raw))}.items():
            equal(reply[key], value, '内部Page绑定')
        fragment = base64.b64decode(reply['data_base64'], validate=True)
        equal(base64.b64encode(fragment).decode('ascii'), reply['data_base64'], '内部base64规范编码')
        equal(len(fragment), min(262144, total - len(raw)), '内部片长度')
        equal(page_uint(reply['raw_byte_count']), len(fragment), '内部片原长度')
        equal(reply['chunk_sha256'], hashlib.sha256(fragment).hexdigest(), '内部片hash')
        raw.extend(fragment)
        require(type(reply['done']) is bool, '内部done类型')
        equal(reply['done'], len(raw) == total, '内部done')
        equal(reply['next_offset'], None if len(raw) == total else str(len(raw)), '内部next offset')
    require(records, 'Q内部缺Close')
    req, reply = records.pop(0)
    equal(req['payload'], {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadClose',
        'transfer_id': desc['transfer_id'], 'descriptor_sha256': dh}, '内部Close完整请求')
    equal(reply, {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed',
        'transfer_id': desc['transfer_id'], 'descriptor_sha256': dh}, '内部Close成功')
    equal(hashlib.sha256(raw).hexdigest(), desc['document_sha256'], '内部完整文档hash')
    document = driver.decode_frame(raw)
    equal(driver.canonical(document), bytes(raw), '内部canonical完整字节')
    page_document(document, desc, selector, {domain: epochs[domain]}, manifest, driver.digest)
    return document


def replay_case_class(driver, binder):
    class ReplayCase(driver.Case):
        def __init__(self, name, output, input_manifest, process, provenance, schedule, allowed):
            require('case_startup_anchors' in provenance and name in provenance['case_startup_anchors'],
                    '缺独立 Q startup 锚 ' + name)
            self.clock_anchors = provenance['case_startup_anchors'][name]
            equal(set(self.clock_anchors), {'started_at', 'case_deadline', 'work_deadline'}, '独立 Q startup 锚字段')
            require(all(finite(v) for v in self.clock_anchors.values()), '独立 Q startup 锚时钟非法')
            self.name, self.output, self.work = name, output / name, output / name / 'state'
            self.root, self.process, self.provenance, self.schedule, self.allowed = output, process, provenance, schedule, allowed
            # *** 关键修复：直接读取文件中的 manifest，不从 input_manifest 构造 ***
            # 文件中的 manifest 包含 ProducerCase 在 super().__init__() 之前修改的所有字段：
            # - 'session_id': 'tb1406-c6a-' + name
            # - 'e': {}
            # - 每个 node 的 'db' 和 'socket'
            # - 追加的 authorized_sources（完整版，包含 EconomicRead* operations）
            self.manifest = self.read_json('state/manifest.json')
            # 保存原件，用于后续验证
            file_manifest = copy.deepcopy(self.manifest)
            self.nodes = {'E': self.manifest['e'], **{'B:' + n['chong_id']: n for n in self.manifest['chongs']}}
            for index, node in enumerate(self.nodes.values()):
                self.read_bytes(f'state/{index}.db')
            # 验证：文件中的 manifest 必须与我们期望的一致
            # 期望的 manifest = input_manifest + ProducerCase 的修改
            expected_manifest = copy.deepcopy(input_manifest)
            expected_manifest['session_id'] = 'tb1406-c6a-' + name
            expected_manifest['e'] = {}
            expected_nodes = {'E': expected_manifest['e'], **{'B:' + n['chong_id']: n for n in expected_manifest['chongs']}}
            for index, node in enumerate(expected_nodes.values()):
                node.update(db=str(self.work / f'{index}.db'), socket=str(self.work / f'{index}.sock'))
            expected_manifest['authorized_sources'].append({**driver.QUERY_SOURCE, 'operations': ['ReadView', 'Watch',
                'EconomicReadOpen', 'EconomicReadPage', 'EconomicReadClose']})
            equal(file_manifest, expected_manifest, '独立启动 manifest')
            self.binaries = Path(provenance['binaries_dir'])
            self.epochs, self.requests = {}, {}
            # 初始化父类 d.Case.paged 需要的序列计数器（参考 tb03c_query_runtime.py:600）
            self.sequence, self.query_sequence, self.logical_sequence = 0, 0, 0
            self.business, self.last_views, self.pending = [], None, None
            self.startup = False
            # 分页由下面独立的离线核验实现；禁止调用生产组装 helper 或伪时钟。
            self.budget = types.SimpleNamespace(until=lambda end: (_ for _ in ()).throw(
                ValueError('离线分页不可借用生产截止判据')))
            self.last_logical_time = process.start
            self.server_processes = []
            self.internal_messages = set()
            self.q_attempt = 0

        def read_bytes(self, relative):
            path = binder.regular_file(self.output, relative)
            self.allowed.add(f'{self.name}/{relative}')
            return path.read_bytes()

        def read_json(self, relative):
            self.read_bytes(relative)
            return binder.load(binder.regular_file(self.output, relative))

        def wire(self, address, request, *, http=False, limit=None, deadline=None, startup_ready=None):
            require(startup_ready is None, '离线 wire 不得另开启动循环')
            self.sequence += 1
            label = f'{self.sequence:04d}'
            transport = self.read_json(label + '-transport.json')
            equal(set(transport), {'request', 'response', 'request_sha256', 'response_sha256',
                                   'response_bytes', 'http', 'address'}, '业务 transport 精确字段')
            require(transport['http'] is http, '逐序 transport 角色')
            equal(transport['address'], list(address) if http else address, '逐序 transport 地址')
            prefix = 'http' if http else 'owner'
            for kind in ('request', 'response'):
                equal(transport[kind], f'{label}.{prefix}-{kind}.bin', '逐序原件名')
            sent, received = self.read_bytes(transport['request']), self.read_bytes(transport['response'])
            equal(sent, request, '具名请求原字节')
            for kind, raw in (('request', sent), ('response', received)):
                equal(transport[kind + '_sha256'], hashlib.sha256(raw).hexdigest(), '逐序 wire hash')
            require(type(transport['response_bytes']) is int, '响应长度类型')
            equal(transport['response_bytes'], len(received), '响应长度')
            maximum = driver.FRAME_BYTES + 16384 if http else driver.FRAME_BYTES
            require(len(sent) <= driver.FRAME_BYTES and len(received) <= maximum, '原 frame 字节界')
            if http:
                body = received.partition(b'\r\n\r\n')[2]
                require(len(body) <= int(driver.QUERY_RESOURCES['max_reply_bytes']), 'Q 原响应容量')
            return received

        def q_owner_scope(self, public, physical):
            self.q_attempt += 1
            name = f'q-owner-wire/attempt-{self.q_attempt:06d}/'
            equal(self.read_bytes(name + 'public-request.json'), driver.canonical(public), 'Q内部公共请求绑定')
            begin, end = self.read_json(name + 'begin.json'), self.read_json(name + 'end.json')
            equal(set(begin), {'schema_revision', 'attempt', 'public_request_sha256', 'started_at', 'deadline'}, 'Q内部scope开始字段')
            equal(set(end), {'attempt', 'rpc_count', 'wire_bytes', 'error_class', 'observed_at'}, 'Q内部scope结束字段')
            require(type(begin['attempt']) is int and type(end['attempt']) is int, 'Q内部attempt类型')
            equal((begin['attempt'], end['attempt']), (self.q_attempt, self.q_attempt), 'Q内部逐次attempt绑定')
            equal((begin['schema_revision'], begin['public_request_sha256']),
                  ('c6a-q-owner-wire/1', driver.digest(public)), 'Q内部scope身份')
            require(all(finite(v) for v in (begin['started_at'], begin['deadline'], end['observed_at'])),
                    'Q内部scope时钟')
            require(physical['started_at'] <= begin['started_at'] <= end['observed_at'] <= physical['observed_at']
                    and end['observed_at'] < min(begin['deadline'], physical['deadline']), 'Q内部scope迟到或越界')
            require(begin['deadline'] <= begin['started_at'] + 30, 'Q内部scope不得另开长窗')
            # 由冻结资源合同推出结构上界，不新增产品诊断拒绝帽。
            maximum_document = min(4194304, int(driver.QUERY_RESOURCES['max_cached_bytes']))
            pages_per_document = (maximum_document + 262143) // 262144
            maximum_rpc = int(driver.QUERY_RESOURCES['max_domains']) * (
                2 + int(driver.QUERY_RESOURCES['max_changes'])) * (2 + pages_per_document)
            require(type(end['rpc_count']) is int and 0 <= end['rpc_count'] <= maximum_rpc, 'Q内部rpc计数')
            equal(end['error_class'], None, 'Q内部scope失败')
            grouped = {domain: [] for domain in self.nodes}
            previous = begin['started_at']
            last_domain = dict.fromkeys(self.nodes, previous)
            total, logical_deadline = 0, None
            for index in range(1, end['rpc_count'] + 1):
                prefix = name + f'rpc-{index:04d}'
                start, finish = self.read_json(prefix + '-begin.json'), self.read_json(prefix + '-end.json')
                equal(set(start), {'index', 'address', 'timeout_ms', 'max_frame_bytes', 'started_at',
                    'deadline', 'logical_deadline', 'request_sha256', 'request_bytes'}, 'Q内部RPC开始字段')
                equal(set(finish), {'transport', 'error_class', 'started_send', 'observed_at',
                    'response_bytes', 'response_sha256'}, 'Q内部RPC结束字段')
                equal(start['index'], index, 'Q内部连续序号')
                equal(start['max_frame_bytes'], 1048576, 'Q内部原帧帽')
                require(type(start['timeout_ms']) is int and 0 < start['timeout_ms'] <= 30000, 'Q内部输送timeout')
                require(all(finite(v) for v in (start['started_at'], start['deadline'],
                        start['logical_deadline'], finish['observed_at'])), 'Q内部RPC时钟类型')
                require(previous <= start['started_at'] <= finish['observed_at'] <= end['observed_at']
                        and finish['observed_at'] < start['deadline'] <= start['logical_deadline']
                        <= begin['deadline'], 'Q内部RPC迟到或期限不属于scope')
                require(start['deadline'] <= start['started_at'] + start['timeout_ms']/1000,
                        'Q内部单次输送期限变长')
                previous = start['started_at']
                if logical_deadline is None:
                    logical_deadline = start['logical_deadline']
                equal(start['logical_deadline'], logical_deadline, 'Q内部逻辑读共同截止不可续窗')
                sent, received = self.read_bytes(prefix + '-request.bin'), self.read_bytes(prefix + '-response.bin')
                for side, raw, meta in (('request', sent, start), ('response', received, finish)):
                    require(0 < len(raw) <= 1048576, 'Q内部原帧大小')
                    equal(meta[side + '_bytes'], len(raw), 'Q内部原帧长度')
                    equal(meta[side + '_sha256'], hashlib.sha256(raw).hexdigest(), 'Q内部原帧摘要')
                    total += len(raw)
                equal((finish['transport'], finish['error_class'], finish['started_send']),
                      ('Received', None, True), 'Q内部失败输送不得算成功')
                req, rep = driver.decode_frame(sent), driver.decode_frame(received)
                equal(sent, driver.canonical(req) + b'\n', 'Q内部请求canonical原字节')
                require(received.endswith(b'\n') and received.count(b'\n') == 1, 'Q内部单响应帧')
                equal(set(req), {'schema_revision', 'session_id', 'session_generation', 'source_namespace',
                    'source_epoch', 'producer_id', 'producer_epoch', 'message_id', 'payload_hash',
                    'causal_refs', 'payload'}, 'Q内部请求公共头')
                for key, expected in {**driver.QUERY_SOURCE, 'schema_revision': public['schema_revision'],
                    'session_id': self.manifest['session_id'], 'session_generation': self.manifest['session_generation'],
                    'causal_refs': []}.items():
                    equal(req[key], expected, 'Q内部请求身份 ' + key)
                require(type(req['message_id']) is str and re.fullmatch('query-[0-9a-f]{32}', req['message_id']),
                        'Q内部随机请求身份格式')
                require(req['message_id'] not in self.internal_messages, 'Q内部请求身份重复')
                self.internal_messages.add(req['message_id'])
                equal(req['payload_hash'], driver.digest(req['payload']), 'Q内部请求内容绑定')
                domains = [d for d, node in self.nodes.items() if node['socket'] == start['address']]
                equal(len(domains), 1, 'Q内部具名owner地址')
                domain = domains[0]
                if req['payload'].get('op') != 'EconomicReadClose':
                    ms = page_uint(req['payload'].get('requested_remaining_ms'), 30000)
                    require(0 < ms <= int((logical_deadline - last_domain[domain]) * 1000),
                            'Q内部wire预算超过原共同截止')
                require(start['started_at'] >= last_domain[domain], '同域RPC顺序重叠')
                last_domain[domain] = finish['observed_at']
                value = driver.validate_reply(req, rep, producer=domain, producer_epoch=self.epochs[domain])
                grouped[domain].append((req, value))
            equal(end['wire_bytes'], total, 'Q内部scope字节账')
            require(total <= end['rpc_count'] * 2 * driver.FRAME_BYTES, 'Q内部scope逐帧字节账')
            return grouped

        def q_owner_projection(self, query, document, grouped):
            if query['op'] == 'snapshot' and query['snapshot_token'] is not None:
                require(all(not rows for rows in grouped.values()), '固定token不得新增owner读取')
                return
            for domain, rows in grouped.items():
                actual = document['domains'][domain]
                equal(actual['status'], 'Available', '正常轨Q域必须完整可用')
                if query['op'] == 'snapshot':
                    selector = {'op': 'ReadView', 'cut': None if query['cuts'] is None else query['cuts'][domain],
                                'as_known_ns': query['as_known_ns']}
                    view = verify_internal_document(driver, rows, domain, selector, self.manifest, self.epochs)
                    equal(actual, {'status': 'Available', **{k: view[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}},
                          'Q公共snapshot必须逐字来自内部owner文档')
                else:
                    selector = {'op': 'ReadView', 'cut': query['cuts'][domain], 'as_known_ns': None}
                    base = verify_internal_document(driver, rows, domain, selector, self.manifest, self.epochs)
                    changes = verify_internal_document(driver, rows, domain,
                        {'op': 'Watch', 'after_cut': query['cuts'][domain]}, self.manifest, self.epochs)
                    equal(actual, {'status': 'Available', 'base_cut': query['cuts'][domain],
                                   'next_cut': changes['cut'], 'changes': changes['changes']},
                          'Q公共watch必须逐字来自内部owner文档')
                    stamp = page_uint(base['commit_ns'], 2**64-1)
                    for change in changes['changes']:
                        view = verify_internal_document(driver, rows, domain,
                            {'op': 'ReadView', 'cut': change['cut'], 'as_known_ns': None},
                            self.manifest, self.epochs)
                        equal({k: view[k] for k in ('cut', 'commit_ns', 'image', 'image_hash')}, change,
                              'Q内部变化必须经过同cut全图回查')
                        now = page_uint(change['commit_ns'], 2**64-1)
                        require(now >= stamp, 'Q内部变化早于基点')
                        stamp = now
                require(not rows, '多余Q内部owner RPC')

        def paged(self, query, *, domain, message=None, deadline=None):
            self.logical_sequence += 1
            logical_id = f'logical-{self.logical_sequence:04d}'
            proof = self.read_json(logical_id + '-pagination.json')
            expected_fields = {'schema_revision', 'logical_id', 'domain', 'logical_message', 'selector',
                'query_sha256', 'session_id', 'session_generation', 'manifest_hash', 'producer_id',
                'producer_epoch', 'owner_epochs', 'started_at', 'deadline', 'case_deadline',
                'work_deadline', 'physical', 'status', 'payload', 'document_bytes', 'document_sha256',
                'observed_at'}
            equal(set(proof), expected_fields, '分页证明完整字段')
            epochs = dict(self.epochs) if domain == 'Q' else {domain: self.epochs[domain]}
            epoch = '1' if domain == 'Q' else self.epochs[domain]
            identity = {'schema_revision': 'c6a-logical-pages/1', 'logical_id': logical_id,
                'domain': domain, 'logical_message': message, 'selector': query,
                'query_sha256': driver.digest(query), 'session_id': self.manifest['session_id'],
                'session_generation': self.manifest['session_generation'], 'manifest_hash': driver.digest(self.manifest),
                'producer_id': domain + ':' + self.manifest['session_id'], 'producer_epoch': epoch,
                'owner_epochs': epochs, 'status': 'COMPLETE'}
            for key, value in identity.items():
                equal(proof[key], value, '分页证明绑定 ' + key)
            start, end, observed = proof['started_at'], proof['deadline'], proof['observed_at']
            require(all(finite(v) for v in (start, end, observed)), '分页逻辑时钟类型')
            equal(proof['case_deadline'], self.clock_anchors['case_deadline'], '分页case原截止')
            equal(proof['work_deadline'], self.clock_anchors['work_deadline'], '分页work原截止')
            expected_end = min(start + 30, proof['case_deadline'], proof['work_deadline'],
                               deadline if deadline is not None else proof['work_deadline'])
            equal(end, expected_end, '分页30秒唯一截止')
            require(self.last_logical_time <= start <= observed < end and observed <= self.process.end,
                    '分页迟到或时钟逆序')
            require(self.process.spawns and self.process.spawns[self.process.position-1]['monotonic'] <= start,
                    '分页早于所属服务启动')
            physical = proof['physical']
            require(type(physical) is list and 3 <= len(physical) <= 18, '分页物理帧数')
            previous, remaining, descriptor, descriptor_hash = start, 30000, None, None
            assembled = bytearray()
            public_scopes = []
            for index, row in enumerate(physical, 1):
                equal(set(row), {'index', 'sequence', 'op', 'remaining_ms', 'started_at', 'deadline',
                    'observed_at', 'transport', 'request_envelope_sha256', 'request_identity',
                    'response_envelope_sha256', 'response_identity'}, '分页物理索引字段')
                require(type(row['index']) is int and type(row['sequence']) is int, '分页索引类型')
                equal((row['index'], row['sequence'], row['transport']),
                      (index, self.sequence + 1, f'{self.sequence+1:04d}-transport.json'), '分页连续物理序列')
                require(finite(row['started_at']) and finite(row['observed_at']) and
                        previous <= row['started_at'] <= row['observed_at'] <= observed and row['observed_at'] < end,
                        '分页物理时钟或晚到')
                equal(row['deadline'], end, '分页不可续窗')
                ms = row['remaining_ms']
                require(type(ms) is int and 1 <= ms <= remaining and ms <= int((end-previous)*1000),
                        '分页剩余预算不可复活')
                remaining = ms
                # helper 先计算wire预算，再计算fetch超时；两者取时点不同。
                # 只从原帧读取该非确定值，并在原截止和前页观察时刻内限制。
                transport = self.read_json(row['transport'])
                sent = self.read_bytes(transport['request'])
                encoded = sent.partition(b'\r\n\r\n')[2] if domain == 'Q' else sent
                original = driver.decode_frame(encoded)
                wire_ms = None
                if index != len(physical):
                    wire_ms = page_uint(original['payload']['requested_remaining_ms'], 30000)
                    require(ms <= wire_ms <= int((end-previous)*1000), 'wire剩余预算越界')
                if index == 1:
                    payload = {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadOpen',
                               'query': query, 'requested_remaining_ms': str(wire_ms)}
                elif index == len(physical):
                    payload = {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadClose',
                               'transfer_id': descriptor['transfer_id'], 'descriptor_sha256': descriptor_hash}
                else:
                    payload = {'schema_revision': 'economic-read-pages/1', 'op': 'EconomicReadPage',
                        'transfer_id': descriptor['transfer_id'], 'descriptor_sha256': descriptor_hash,
                        'offset': str(len(assembled)), 'requested_remaining_ms': str(wire_ms)}
                equal(row['op'], payload['op'], '分页op顺序')
                evidence = {}
                if domain == 'Q':
                    response = self.http_frame('/api/economic/read', payload, evidence=evidence)
                    public_scopes.append((original, row))
                else:
                    response = self.owner_frame(domain, payload, logical_id + '-' + str(index), evidence=evidence)
                for key, value in evidence.items():
                    equal(row[key], value, '分页原信封索引 ' + key)
                equal(response.get('schema_revision'), 'economic-read-pages/1', '分页响应版本')
                if index == 1:
                    equal(set(response), {'schema_revision', 'kind', 'descriptor', 'descriptor_sha256'}, 'Open字段')
                    equal(response['kind'], 'EconomicReadOpened', 'Open成功')
                    descriptor, descriptor_hash = response['descriptor'], response['descriptor_sha256']
                    equal(set(descriptor), {'schema_revision', 'transfer_id', 'session_id', 'session_generation',
                        'manifest_hash', 'producer_id', 'producer_epoch', 'query_sha256', 'document_kind',
                        'fixed_cuts', 'owner_epochs', 'document_bytes', 'document_sha256', 'chunk_bytes', 'page_count'},
                        '描述符字段')
                    for key in ('session_id', 'session_generation', 'manifest_hash', 'producer_id',
                                'producer_epoch', 'query_sha256', 'owner_epochs'):
                        equal(descriptor[key], identity[key], '描述符可信绑定 ' + key)
                    equal(descriptor['schema_revision'], 'economic-read-pages/1', '描述符版本')
                    require(type(descriptor['transfer_id']) is str and
                            re.fullmatch('[0-9a-f]{32}', descriptor['transfer_id']), '随机transfer格式')
                    require(type(descriptor['document_sha256']) is str and
                            re.fullmatch('[0-9a-f]{64}', descriptor['document_sha256']), '正文hash格式')
                    equal(descriptor_hash, driver.digest(descriptor), '描述符hash')
                    total = page_uint(descriptor['document_bytes'])
                    require(total > 0, '正文为空')
                    equal(descriptor['chunk_bytes'], '262144', '固定stride')
                    equal(page_uint(descriptor['page_count'], 16), (total + 262143)//262144, '页数')
                    equal(len(physical), (total + 262143)//262144 + 2, '物理帧完整性')
                elif index == len(physical):
                    equal(response, {'schema_revision': 'economic-read-pages/1', 'kind': 'EconomicReadClosed',
                        'transfer_id': descriptor['transfer_id'], 'descriptor_sha256': descriptor_hash}, 'Close必须成功')
                else:
                    equal(set(response), {'schema_revision', 'kind', 'transfer_id', 'descriptor_sha256', 'offset',
                        'raw_byte_count', 'chunk_sha256', 'data_base64', 'next_offset', 'done'}, 'Page字段')
                    for key in ('transfer_id', 'descriptor_sha256', 'offset'):
                        equal(response[key], payload[key], 'Page绑定 ' + key)
                    equal(response['kind'], 'EconomicReadPage', 'Page成功')
                    count = min(262144, total - len(assembled))
                    equal(page_uint(response['raw_byte_count']), count, '页原字节数')
                    require(type(response['data_base64']) is str, 'base64类型')
                    fragment = base64.b64decode(response['data_base64'], validate=True)
                    equal(base64.b64encode(fragment).decode('ascii'), response['data_base64'], 'base64规范编码')
                    equal(len(fragment), count, '片长度')
                    equal(hashlib.sha256(fragment).hexdigest(), response['chunk_sha256'], '片hash')
                    assembled.extend(fragment)
                    require(type(response['done']) is bool, 'done类型')
                    equal(response['done'], len(assembled) == total, 'done精确边界')
                    equal(response['next_offset'], None if len(assembled) == total else str(len(assembled)),
                          'next_offset边界')
                previous = row['observed_at']
            equal(len(assembled), total, '完整文档长度')
            equal(hashlib.sha256(assembled).hexdigest(), descriptor['document_sha256'], '完整文档hash')
            document = driver.decode_frame(bytes(assembled))
            equal(driver.canonical(document), bytes(assembled), '完整canonical字节')
            page_document(document, descriptor, query, epochs, self.manifest, driver.digest)
            if domain == 'Q':
                for index, (public, row) in enumerate(public_scopes):
                    internal = self.q_owner_scope(public, row)
                    if index == 0:
                        self.q_owner_projection(query, document, internal)
                    else:
                        require(all(not frames for frames in internal.values()), 'Q Page/Close不得新增owner RPC')
            equal(proof['payload'], logical_id + '-payload.json', '逻辑文档原件名')
            equal(self.read_bytes(proof['payload']), bytes(assembled), '保存原件与独立重组逐字相等')
            require(type(proof['document_bytes']) is int, '证明长度类型')
            equal(proof['document_bytes'], total, '证明长度')
            equal(proof['document_sha256'], descriptor['document_sha256'], '证明hash')
            self.last_logical_time = observed
            return document

        def http(self, path, payload=None, *, deadline=None, startup_ready=None):
            reply = super().http(path, payload, deadline=deadline, startup_ready=startup_ready)
            equal((reply['session_id'], reply['session_generation'], reply['manifest_hash']),
                  (self.manifest['session_id'], self.manifest['session_generation'], driver.digest(self.manifest)),
                  '公共响应 manifest/session 身份')
            if path.endswith('/snapshot'):
                fields = {'kind', 'session_id', 'session_generation', 'manifest_hash', 'mode',
                          'as_known_ns', 'domains', 'completeness', 'snapshot_token'}
                equal(set(reply), fields, '公共 snapshot 完整字段')
                body = {k: v for k, v in reply.items() if k != 'snapshot_token'}
                equal(reply['snapshot_token'], 'economic-snapshot-' + driver.digest(body), '公共固定 token 身份')
                if payload['snapshot_token'] is None:
                    equal(reply['mode'], 'AsKnown' if payload['cuts'] is not None or payload['as_known_ns'] is not None else 'Current', '公共模式')
                    equal(reply['as_known_ns'], payload['as_known_ns'], '公共获知时刻')
            elif path.endswith('/watch'):
                equal(set(reply), {'kind', 'session_id', 'session_generation', 'manifest_hash', 'domains'}, '公共 watch 完整字段')
            return reply

        def owner(self, domain, payload, message, *, deadline=None):
            first = message not in self.requests
            if message.startswith(('step-', 'we7-')) and first:
                require(len(self.business) < len(self.schedule), '多余业务步骤')
                expected = self.schedule[len(self.business)]
                equal((domain, payload, message), (expected['domain'], expected['payload'], expected['message']), '独立 41 步请求顺序')
                self.business.append(message)
            reply = super().owner(domain, payload, message, deadline=deadline)
            if message.startswith(('step-', 'we7-')) and first:
                equal(driver.outcome(reply), self.schedule[len(self.business)-1]['outcome'], '独立步骤 outcome')
                if reply['kind'] in ('Recorded', 'Resolved', 'Applied'):
                    self.pending = (domain, copy.deepcopy(payload), copy.deepcopy(reply), copy.deepcopy(self.last_views))
            return reply

        def observe(self):
            result = super().observe()
            if self.pending:
                domain, payload, receipt, previous = self.pending
                require(previous is not None, '写前 owner 缺失')
                equal(receipt['cut'], result[domain]['cut'], '成功回执 cut 对 after owner')
                if payload['op'] == 'CaptureExternal':
                    binder.verify_capture(payload, receipt, previous['E'], result['E'])
                self.pending = None
            self.last_views = copy.deepcopy(result)
            return result

        def start(self):
            first_init = None
            for domain in self.nodes:
                init, raw = self.process.consume(self.args(domain, 'init'), self.name + '-init')
                if first_init is None:
                    first_init = init
                reply = driver.decode_frame(raw)
                equal((reply['kind'], reply['domain_id'], reply['writer_epoch']), ('Initialized', domain, '1'), '正常轨 init 身份')
                self.epochs[domain] = reply['writer_epoch']
                serve, _ = self.process.consume(self.args(domain, 'serve'), self.name + '-serve')
                require(init['reaped']['monotonic'] <= serve['monotonic'], 'init 未退出即 serve')
                self.server_processes.append(serve)
                equal(self.owner(domain, {'op': 'ReadView', 'cut': None, 'as_known_ns': None}, 'ready-' + domain)['kind'],
                      'EconomicView', 'owner 启动完整就绪')
            config = {'schema_revision': 'economic-query/1', 'manifest': 'manifest.json',
                      'manifest_hash': driver.digest(self.manifest), 'query_source': driver.QUERY_SOURCE,
                      'producer_epoch': '1', 'expected_epochs': self.epochs, 'resources': driver.QUERY_RESOURCES}
            equal(self.read_json('state/query.json'), config, 'Q 原配置')
            row = self.process.spawns[self.process.position]
            command = row['command']
            require(len(command) == 19, 'Q argv 长度')
            require(command[9] == '--port', 'Q port 位置')
            self.port = int(command[10])
            self.structural_db = self.work / 'UNUSED-STRUCTURE.db'
            expected_command = [self.provenance['python_executable'], '-B', str(driver.ROOT / 's_readonly_server.py'),
                '--db', str(self.structural_db), '--browser', str(self.work / 'UNUSED-BROWSER.html'),
                '--host', '127.0.0.1', '--port', str(self.port), '--resource-config',
                str(driver.HERE / 'fixtures/tb01c/query-resources.json'), '--producer-epoch', '1',
                '--economic-config', str(self.work / 'query.json'),
                '--economic-wire-evidence', str(self.output / 'q-owner-wire')]
            q, _ = self.process.consume(expected_command, self.name + '-q')
            self.server_processes.append(q)
            first = self.sequence + 1
            shared = None
            while True:
                self.sequence += 1
                transport = self.read_json(f'{self.sequence:04d}-transport.json')
                attempt = transport['startup_ready_attempt']
                for kind in ('request', 'response'):
                    raw = self.read_bytes(transport[kind])
                    maximum = driver.FRAME_BYTES + 16384 if kind == 'response' else driver.FRAME_BYTES
                    require(len(raw) <= maximum, 'Q startup 原 frame 字节界')
                if shared is None:
                    shared = attempt
                if attempt['phase'] == 'complete':
                    break
                require(attempt['phase'] == 'connect', 'Q startup 未成功且非连接拒绝')
            clock_anchors = self.clock_anchors
            work_start = clock_anchors['work_deadline'] - driver.WORK_SECONDS
            self.work_start = work_start
            require(self.process.start <= work_start <= self.process.spawns[0]['monotonic'], 'driver t0 原来源区间')
            require(self.server_processes[-2]['monotonic'] <= clock_anchors['started_at'] <= q['monotonic'] <= shared['attempt_started_at'], 'Q 启动同源时间次序')
            require(clock_anchors['case_deadline'] <= min(clock_anchors['work_deadline'], first_init['monotonic'] + driver.CASE_SECONDS), 'case 期限续租')
            require(clock_anchors['case_deadline'] > first_init['monotonic'], 'case 初始期限已过')
            self.read_bytes('Q-STARTUP.json')
            self.read_bytes('port.json')
            binder.verify_q_startup(self.output, manifest=self.manifest, port=self.port,
                started_at=clock_anchors['started_at'], case_deadline=clock_anchors['case_deadline'], work_deadline=clock_anchors['work_deadline'],
                first_sequence=first, success_sequence=self.sequence)
            proof = self.read_json('Q-STARTUP.json')
            require(proof['validated_at'] <= self.process.end, 'Q 核验越出 driver 生命期')
            for serve in self.server_processes:
                require(serve['reaped']['monotonic'] >= proof['validated_at'], 'Q 就绪前 owner 已退出')
            require(not self.structural_db.exists(), '经济路由产生结构 DB')
            self.startup = True

        def finish(self):
            equal(self.business, [row['message'] for row in self.schedule], '完整业务步骤')
            require(self.pending is None and self.startup, '步骤未闭合')
            for serve in self.server_processes:
                require(serve['reaped']['returncode'] in (0, -signal.SIGTERM), '服务异常退出')
            require(not self.structural_db.exists(), '经济路由产生结构 DB')
    return ReplayCase


def verify_runtime(output: Path, build_evidence: dict, provenance: dict) -> dict:
    """离线重放完整 41 步，成功只返回 VERIFIED_C6A_SEMANTICS。

    provenance 由外层持有；函数不将 driver RESULT 自述提升为外部事实。
    本函数所有 I/O 仍必须在外层硬截止/资源监督 helper 中运行。
    """
    required = {'checkout_root', 'binaries_dir', 'python_executable', 'source_sha', 'source_tree',
                'source_files', 'binaries_before', 'binaries_after', 'driver_started_monotonic',
                'driver_finished_monotonic', 'artifact_manifest_sha256', 'case_startup_anchors'}
    require(required <= set(provenance), '缺独立外层 provenance')
    output = Path(output).resolve()
    driver = load_driver(provenance['checkout_root'], provenance['source_files'])
    binder = binder_for(driver)
    driver.verify_build(build_evidence, provenance['source_sha'], provenance['source_tree'],
                        provenance['source_files'], provenance['binaries_before'])
    equal(provenance['binaries_before'], provenance['binaries_after'], '运行期间二进制改变')
    inventory = binder.verify_inventory(output, output / 'ARTIFACTS.json', provenance['artifact_manifest_sha256'])
    actual = set()
    for path in output.rglob('*'):
        require(not path.is_symlink(), '原件树禁止符号链接')
        if path.is_file():
            actual.add(path.relative_to(output).as_posix())
    equal(actual - {'ARTIFACTS.json', 'RESULT.json'}, inventory, 'ARTIFACTS 全普通文件精确覆盖')
    for marker in ('CANCELLED-AFTER-SEAL.json', 'FINALIZATION-REJECTED.json', 'FINALIZATION-FAILED.json'):
        require(marker not in actual, '取消/迟到/封口失败原件')
    receipt = binder.load(binder.regular_file(output, 'RESULT.json'))
    equal(set(receipt), {'status', 'scope', 'results', 'primary_error', 'cleanup_error', 'cancellation',
                        'candidate_only', 'outer_acceptance_requires', 'remaining_children', 'work_seconds',
                        'final_seconds', 'elapsed', 'artifact_manifest_sha256', 'not_proven'}, 'driver RESULT 完整字段')
    equal(receipt['scope'], 'TestOnly WE5/WE7 public Q normal', 'driver scope')
    equal(receipt['outer_acceptance_requires'], ['exit_zero', 'no_cancellation', 'no_failure_markers',
          'complete_verified_originals', 'group_reaped_with_owned_identity'], '外层责任不可删除')
    equal(receipt['not_proven'], ['complete C6', 'FIN/2', 'real S/gamma', 'per_leg success',
          'C7 recovery', 'frontend', '1394 readiness'], '未证实边界不可删除')
    equal(receipt['status'], 'CANDIDATE_C6A_NORMAL_ONLY', 'driver 仅候选正常终态')
    require(receipt['candidate_only'] is True and receipt['primary_error'] is None and
            receipt['cleanup_error'] is None and receipt['cancellation'] is None and
            receipt['remaining_children'] == [], 'driver 候选含失败')
    equal(receipt['artifact_manifest_sha256'], provenance['artifact_manifest_sha256'], '候选清单身份')
    equal((receipt['work_seconds'], receipt['final_seconds']), (600, 630), 'driver 原时限')
    require(finite(receipt['elapsed']) and 0 <= receipt['elapsed'] < 630, 'driver 晚候选')
    seed, plan, oracle = driver.load_inputs()
    schedule = expected_steps(driver, seed, plan, oracle)
    source = binder.load(binder.regular_file(output, 'SOURCE.json'))
    equal(source, {'base': driver.BASE, 'base_tree': driver.BASE_TREE, 'build': build_evidence,
        'inputs': driver.INPUT_HASHES, 'q_resource_sha256': driver.sha(driver.HERE / 'fixtures/tb01c/query-resources.json'),
        'limits': {'work': 600, 'per_case': 300, 'final': 630, 'startup': 10, 'exchange': 30, 'cli': 30,
                   'term_grace': 3, 'kill_grace': 3}, 'hard_io_deadline': False}, '完整 SOURCE 原合同')
    allowed = {'SOURCE.json'}
    processes = ProcessLedger(output, binder, provenance, allowed)
    for command, label, expected in ((['git', 'rev-parse', 'HEAD'], 'source-head', provenance['source_sha']),
            (['git', 'rev-parse', 'HEAD^{tree}'], 'source-tree', provenance['source_tree']),
            (['git', 'status', '--porcelain', '--untracked-files=all'], 'source-clean', '')):
        row, raw = processes.consume(command, label)
        equal(raw, (expected + '\n').encode() if expected else b'', '源身份 CLI 原件 ' + label)
    ReplayCase = replay_case_class(driver, binder)
    def check_saved(path, value):
        path = Path(path)
        relative = path.relative_to(output).as_posix()
        actual_value = binder.load(binder.regular_file(output, relative))
        equal(driver.canonical(actual_value), driver.canonical(value), '公共/独立 oracle 保存原件 ' + relative)
        allowed.add(relative)
    results, traces = [], {}
    clock_origin = None
    with patch.object(driver, 'save', check_saved):
        for name in ('we5', 'we7'):
            manifest = seed['manifest'] if name == 'we5' else driver.we7_manifest(seed, oracle)
            case = ReplayCase(name, output, manifest, processes, provenance, schedule[name], allowed)
            result = driver.execute_we5(case, plan) if name == 'we5' else driver.execute_we7(case, oracle)
            results.append(result)
            if clock_origin is None:
                clock_origin = case.work_start
            equal(case.work_start, clock_origin, '两 case 必须同一 driver t0')
            traces[name] = {'steps': len(case.business), 'transports': case.sequence}
            for i in range(len(case.nodes)):
                for suffix in ('.db', '.db-wal', '.db-shm'):
                    allowed.add(f'{name}/state/{i}{suffix}')
    require(processes.rows[-1]['monotonic'] <= clock_origin + receipt['elapsed'] <= processes.end, '候选 elapsed 与进程账同源')
    equal(processes.position, len(processes.spawns), '多余子进程')
    equal(receipt['results'], results, '内部候选必须对应外层完整重放结果')
    require(inventory <= allowed, '未消费或非合同原件 ' + repr(sorted(inventory - allowed)))
    return {'status': 'VERIFIED_C6A_SEMANTICS', 'candidate_only': True, 'steps': 41,
            'cases': traces, 'results': results, 'artifact_manifest_sha256': provenance['artifact_manifest_sha256'],
            'startup_clock_basis': '独立外层 case_startup_anchors；缺锚拒绝，禁止 proof/transport 反取',
            'not_proven': ['outer guardian/resource/deadline/cancellation acceptance', 'complete C6', 'full #1323 acceptance']}


BINDER_CODE = 'import hashlib, json, math, errno, signal\nfrom pathlib import Path, PurePosixPath\nINNER_LIMITS={"startup":10}\ndef require(condition, message):\n    if not condition:\n        raise ValueError(message)\n\ndef equal(actual, expected, message):\n    driver.equal(actual, expected, message)\n\ndef load(path):\n    # 日志含 monotonic 小数；仍拒绝重复键及 NaN/Infinity。\n    def unique(pairs):\n        result = {}\n        for key, value in pairs:\n            require(key not in result, "原件 JSON 重复键")\n            result[key] = value\n        return result\n    def bad_number(value):\n        raise ValueError("原件 JSON 非有限数值：" + value)\n    def finite_float(value):\n        number = float(value)\n        require(math.isfinite(number), "原件 JSON 非有限数值：" + value)\n        return number\n    return json.loads(Path(path).read_bytes(), object_pairs_hook=unique,\n                      parse_constant=bad_number, parse_float=finite_float)\n\ndef regular_file(root, name):\n    require(type(name) is str, "原件路径须为字符串")\n    root, relative = Path(root).resolve(), PurePosixPath(name)\n    require(name == relative.as_posix(), "原件路径不是规范 POSIX 相对路径")\n    require(not relative.is_absolute() and relative.parts and\n            all(p not in ("..", ".") for p in relative.parts), "原件路径越界")\n    path = root\n    for component in relative.parts:\n        path = path / component\n        require(not path.is_symlink(), "原件禁止符号链接")\n    require(path.is_file() and path.resolve().is_relative_to(root), "原件不是普通文件")\n    return path\n\ndef verify_inventory(root, manifest, expected_manifest_sha):\n    """验证已保存清单，不重新生成清单给伪造文件背书。调用方另核覆盖范围。"""\n    equal(driver.sha(manifest), expected_manifest_sha, "原清单身份")\n    rows = load(manifest)["files"]\n    require(type(rows) is list and rows, "缺原件清单")\n    seen = set()\n    for row in rows:\n        equal(set(row), {"path", "bytes", "sha256"}, "原件条目字段")\n        path = regular_file(root, row["path"])\n        require(row["path"] not in seen, "原件清单重复路径")\n        seen.add(row["path"])\n        require(type(row["bytes"]) is int and row["bytes"] >= 0, "原件长度非法")\n        equal(path.stat().st_size, row["bytes"], "原件长度")\n        equal(driver.sha(path), row["sha256"], "原件摘要")\n    return seen\n\ndef verify_capture(payload, receipt, before, after):\n    """H3：Capture 原件/事实及 Recorded cut 必须指向同一次 after owner。\n\n    只处理本轨首次成功 Capture。精确重送由外层独立比较原回执与不变图像。\n    不实现金额 reducer，不从回执或 owner 反向生成原事实预期。\n    """\n    equal(payload["op"], "CaptureExternal", "Capture 操作")\n    equal(receipt["kind"], "Recorded", "Capture 回执类型")\n    equal(receipt["domain_id"], "E", "Capture 回执角色")\n    event = driver.event_at(payload["raw_utf8"], payload["json_pointer"])\n    key = driver.fact_key(event)\n    expected = {"fact_key": key,\n                "fact_hash": driver.fact_hash(payload["raw_utf8"], payload["json_pointer"]),\n                "raw_utf8": payload["raw_utf8"], "json_pointer": payload["json_pointer"],\n                "raw_sha256": hashlib.sha256(payload["raw_utf8"].encode()).hexdigest(),\n                "event": event, "token": payload["token"],\n                "source_event_time": payload["source_event_time"]}\n    for owner in (before, after):\n        equal(owner["cut"]["domain_id"], "E", "Capture owner 域")\n        equal(owner["image_hash"], driver.digest(owner["image"]), "owner 图像摘要")\n    equal(receipt["fact_key"], key, "Capture 回执事实")\n    equal(receipt["fact_hash"], expected["fact_hash"], "Capture 回执事实内容")\n    equal(receipt["token"], payload["token"], "Capture 回执 token")\n    equal(receipt["cut"], after["cut"], "Recorded cut 未绑定 after owner")\n    equal(int(after["cut"]["commit_seq"]), int(before["cut"]["commit_seq"]) + 1,\n          "首次 Capture 单次提交")\n    require(not any(row["fact_key"] == key for row in before["image"]["recorded"]),\n            "首次 Capture 已有原事实")\n    records = [row for row in after["image"]["recorded"] if row["fact_key"] == key]\n    equal(len(records), 1, "Capture after 缺事实或重复")\n    for field, value in expected.items():\n        equal(records[0][field], value, "Capture after 原件/事实 " + field)\n\ndef verify_q_startup(root, *, manifest, port, started_at, case_deadline,\n                     work_deadline, first_sequence, success_sequence):\n    """核真实 driver 的 Q-STARTUP/transport/wire 原件，不执行网络操作。\n\n    参数必须来自调用者可信的 manifest、端口分配、同一单调时钟和完整序列账本，\n    不得从待验 proof/transport 反取。外层仍须证明来源、清单覆盖与监督真实性。\n    """\n    for value in (started_at, case_deadline, work_deadline):\n        require(type(value) in (int, float) and math.isfinite(value), "Q startup 可信时钟非法")\n    require(type(port) is int and 0 < port <= 65535, "Q startup 可信端口非法")\n    require(type(first_sequence) is int and type(success_sequence) is int and\n            0 < first_sequence <= success_sequence, "Q startup 可信序列非法")\n    deadline = min(started_at + INNER_LIMITS["startup"], case_deadline, work_deadline)\n    require(started_at < deadline, "Q startup 可信期限已过")\n    shared = {"schema_revision": "c6a-q-startup/1", "started_at": started_at,\n              "deadline": deadline, "case_deadline": case_deadline, "work_deadline": work_deadline}\n    proof = load(regular_file(root, "Q-STARTUP.json"))\n    equal(set(proof), set(shared) | {"first_sequence", "success_sequence", "validated_at",\n                                    "success_response_sha256"}, "Q startup proof 字段")\n    for key, expected in shared.items():\n        if key != "schema_revision":\n            require(type(proof[key]) in (int, float) and math.isfinite(proof[key]),\n                    "Q startup proof 时钟类型")\n        equal(proof[key], expected, "Q startup 可信锚 " + key)\n    for key, expected in (("first_sequence", first_sequence), ("success_sequence", success_sequence)):\n        require(type(proof[key]) is int, "Q startup proof 序列类型")\n        equal(proof[key], expected, "Q startup 首末序列")\n    validated = proof["validated_at"]\n    require(type(validated) in (int, float) and math.isfinite(validated) and\n            started_at <= validated < deadline, "Q startup 晚到或非法核验时间")\n    port_record = load(regular_file(root, "port.json"))\n    equal(port_record, {"host": "127.0.0.1", "port": port, "released_before_child_bind": True,\n                        "collision_policy": "FAIL without touching occupant"}, "Q startup 端口原件")\n    request = (f"GET /api/economic/ready HTTP/1.0\\r\\nHost: 127.0.0.1:{port}\\r\\n"\n               "Content-Type: application/json\\r\\nContent-Length: 0\\r\\nConnection: close\\r\\n\\r\\n").encode()\n    previous = started_at\n    ready = None\n    for sequence in range(first_sequence, success_sequence + 1):\n        label = f"{sequence:04d}"\n        transport = load(regular_file(root, label + "-transport.json"))\n        equal(set(transport), {"request", "response", "request_sha256", "response_sha256",\n                               "response_bytes", "http", "address", "startup_ready_attempt"},\n              "Q startup transport 字段")\n        require(transport["http"] is True, "Q startup 必须 HTTP")\n        equal(transport["address"], ["127.0.0.1", port], "Q startup host/port")\n        for kind in ("request", "response"):\n            equal(transport[kind], label + f".http-{kind}.bin", "Q startup 具名原件")\n        raw_request = regular_file(root, transport["request"]).read_bytes()\n        raw_response = regular_file(root, transport["response"]).read_bytes()\n        for kind, raw in (("request", raw_request), ("response", raw_response)):\n            equal(transport[kind + "_sha256"], hashlib.sha256(raw).hexdigest(), "Q startup 原件摘要")\n        equal(raw_request, request, "Q startup 显式零 body ready GET")\n        require(type(transport["response_bytes"]) is int, "Q startup 响应长度类型")\n        equal(transport["response_bytes"], len(raw_response), "Q startup 响应长度")\n        attempt = transport["startup_ready_attempt"]\n        equal(set(attempt), set(shared) | {"sequence", "attempt_started_at", "observed_at", "phase",\n                                           "exception_class", "exception_errno", "connect_completed", "send_entered"},\n              "Q startup attempt 字段")\n        for key, expected in shared.items():\n            if key != "schema_revision":\n                require(type(attempt[key]) in (int, float) and math.isfinite(attempt[key]),\n                        "Q startup attempt 时钟类型")\n            equal(attempt[key], expected, "Q startup attempt 原期限 " + key)\n        require(type(attempt["sequence"]) is int, "Q startup attempt 序列类型")\n        equal(attempt["sequence"], sequence, "Q startup 连续序列")\n        begin, observed = attempt["attempt_started_at"], attempt["observed_at"]\n        require(all(type(v) in (int, float) and math.isfinite(v) for v in (begin, observed)),\n                "Q startup attempt 时钟非法")\n        require(previous <= begin <= observed <= validated and observed < deadline,\n                "Q startup attempt 逆序或晚到")\n        previous = observed\n        if sequence < success_sequence:\n            equal((attempt["phase"], attempt["exception_class"], attempt["exception_errno"]),\n                  ("connect", "ConnectionRefusedError", errno.ECONNREFUSED), "Q startup 非连接拒绝不可重试")\n            require(attempt["connect_completed"] is False and attempt["send_entered"] is False,\n                    "Q startup 重试已连接或已发送")\n            equal(raw_response, b"", "Q startup 重试必须零响应")\n        else:\n            equal((attempt["phase"], attempt["exception_class"], attempt["exception_errno"]),\n                  ("complete", None, None), "Q startup 成功阶段")\n            require(attempt["connect_completed"] is True and attempt["send_entered"] is True,\n                    "Q startup 成功未完成连接发送")\n            equal(proof["success_response_sha256"], transport["response_sha256"], "Q startup 成功原件绑定")\n            ready = driver.parse_http(raw_response)\n    equal(ready["manifest_hash"], driver.digest(manifest), "Q startup manifest 身份")\n    equal(ready["book_revision"], "tb03c-book/1", "Q startup book 合同")\n    equal((ready["session_id"], ready["session_generation"], ready["producer_epoch"]),\n          (manifest["session_id"], manifest["session_generation"], "1"), "Q startup 会话/epoch 身份")\n    domains = ["E"] + ["B:" + node["chong_id"] for node in manifest["chongs"]]\n    require(type(ready["domains"]) is list and all(type(d) is str for d in ready["domains"]),\n            "Q startup domains 类型")\n    equal(sorted(ready["domains"]), sorted(domains), "Q startup owner 集合")\n    return ready\n\ndef verify_processes(events):\n    """按 spawn 实例闭合，拒绝没有 TERM 原件的 -15；CLI 永须 exit0。"""\n    active = {}\n    spawned = 0\n    previous = -math.inf\n    for event in events:\n        now = event["monotonic"]\n        require(type(now) in (int, float) and math.isfinite(now) and now >= previous,\n                "进程原件时序非法")\n        previous = now\n        pid, kind = event["pid"], event["event"]\n        require(type(pid) is int and pid > 0, "进程身份非法")\n        if kind == "spawn":\n            require(pid not in active, "进程实例重叠")\n            require(event["direct_child"] is True and event["inherits_driver_group"] is True,\n                    "进程未绑定 driver 组")\n            command = event["command"]\n            require(type(command) is list and command and all(type(x) is str for x in command),\n                    "缺进程命令")\n            active[pid] = {"command": command, "signals": []}\n            spawned += 1\n        else:\n            require(pid in active, "未知或已回收进程")\n            child = active[pid]\n            if kind == "signal":\n                require(event["returncode_before"] is None, "已退出进程不得发信号")\n                child["signals"].append(event["signal"])\n            elif kind == "reaped":\n                code, command = event["returncode"], child["command"]\n                require(type(code) is int, "退出码非法")\n                service = (len(command) > 1 and command[1] == "serve") or any(\n                    Path(arg).name == "s_readonly_server.py" for arg in command)\n                require(code == 0 or (service and code == -signal.SIGTERM and\n                                     child["signals"] == [signal.SIGTERM]),\n                        "非零 CLI 或缺 TERM 原件的退出")\n                require(signal.SIGKILL not in child["signals"], "正常轨出现 KILL 升级")\n                del active[pid]\n            else:\n                require(kind in ("wait", "wait-timeout", "signal-esrch"), "未知进程事件")\n    require(spawned > 0 and not active, "进程清单为空或未全部回收")'
