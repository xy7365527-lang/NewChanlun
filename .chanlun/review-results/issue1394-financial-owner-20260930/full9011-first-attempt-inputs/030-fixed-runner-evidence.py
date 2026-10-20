"""#1394：纯字节/JSON 验证；本模块不导入 SQLite、Controller 或 socket。"""
import hashlib
import json
import re


COUNT = 9011
SOURCE_SHA = '76472995cbf0825ceeeaa94e89bd60d1b1416d6c7e0a7ec3a1b0ec5a90c60087'
CLOCK_SHA = 'b02ac2444161e1e7e327fcf2a2bb79b294356a3dd95a59a6404c5b92deb04f01'
HEAD = '3e5a5ba31dbe5c59beeee03de227fd8408eaac55'
TREE = '7196a77fef56cf87cbe91dc4b135326a142e2395'
BINARY_SHA = 'c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17'
PATCH_SHA = '677164b5a0ca4e43ab325502853f3ea72d4ad01313fb43a5fe8152124f19068e'
RESOURCE_FIELDS = {'max_frame_bytes', 'read_timeout_ms', 'write_timeout_ms',
                   'response_timeout_ms', 'audit_cache_source_bytes',
                   'queue_capacity', 'max_connections'}
SCOPE = 'formal-S-full9011-AB-serial-fresh'


class Rejected(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Rejected(message)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(',', ':'), allow_nan=False).encode('utf-8')


def unique(pairs):
    value = {}
    for key, item in pairs:
        require(key not in value, '重复 JSON 状态/字段：' + key)
        value[key] = item
    return value


def reject_number(value):
    raise Rejected('协议禁止浮点/非有限数值：' + value)


def decode(raw, *, metadata=False):
    return json.loads(raw, object_pairs_hook=unique,
                      parse_float=float if metadata else reject_number,
                      parse_constant=reject_number)


def positive(value, field):
    require(type(value) is str and re.fullmatch(r'[1-9][0-9]*', value) is not None,
            field + ' 必须显式为正整数文本，不填默认值')
    number = int(value)
    require(number <= 2**63 - 1, field + ' 超过 i64')
    return number


def frames(raw, count, label):
    require(raw.endswith(b'\n'), label + ' 截断或缺末尾 LF')
    lines = raw.splitlines(keepends=True)
    require(len(lines) == count, label + ' 条数不完整/有多余帧')
    for line in lines:
        require(line.endswith(b'\n') and b'\n' not in line[:-1], label + ' 帧边界错误')
        require(canonical(decode(line)) + b'\n' == line, label + ' 不是冻结规范原字节')
    return lines


def validate_material(envelopes, schedule, clock_raw, raws, index, *, count=COUNT):
    """count 参数只供小型反例；正式 CLI 固定 COUNT，不接受缩短范围参数。"""
    lines = frames(envelopes, count, 'envelopes')
    inputs = frames(raws, count, 'raw-inputs')
    rows = schedule['ordinal_order']
    require(len(rows) == len(index) == count, 'schedule/index 不是完整条数')
    require(schedule['clock_sha256'] == sha(clock_raw), 'schedule 不属于原 clock')
    clock = decode(clock_raw)
    require(clock['schema_revision'] == 's-clock-plan/1' and clock['unit'] == 'ns', 'clock 版本/量纲错误')
    require(set(clock['events']) == {f'input-{n}' for n in range(1, count + 1)}, 'clock 事件集合错误')
    identities = None
    time_frontier = -1
    for n, (line, raw_line, row, entry) in enumerate(zip(lines, inputs, rows, index), 1):
        env = decode(line)
        payload = env['payload']
        raw = payload['raw_input']
        require(type(row['ordinal']) is int and type(entry['ordinal']) is int and row['ordinal'] == entry['ordinal'] == n, '请求重排/ordinal 不连续')
        require(row['message_id'] == entry['message_id'] == env['message_id'] == f'message-{n}', '请求 identity 错序')
        require(row['clock_event_id'] == payload['clock_event_id'] == f'input-{n}', '同 clock 不同时序')
        require(row['expected_generation'] == str(n), 'generation 错序')
        require(row['action'] == 'exchange_once_then_validate_committed_before_next'
                and row['retries'] == 'none', 'schedule 放宽准入或重试')
        require(row['request_sha256'] == entry['envelope_sha256'] == sha(line[:-1]), '请求 hash 错误')
        require(type(entry['body_bytes']) is int and type(entry['lf_frame_bytes']) is int and entry['body_bytes'] == len(line) - 1 and entry['lf_frame_bytes'] == len(line), '帧字节数错误')
        require(env['schema_revision'] == 's-session/3' and payload['op'] == 'ingest', '必须为正式 S3 ingest')
        require(env['payload_hash'] == entry['payload_sha256'] == sha(canonical(payload)), 'payload 原摘要错误')
        require(canonical(raw) + b'\n' == raw_line and entry['raw_input_sha256'] == sha(raw_line[:-1]), 'raw input 原件不符')
        require(raw['schema_revision'] == 's-ohlc/1' and len(raw['events']) == 1, 'raw input 形状不符')
        event = raw['events'][0]
        require(event['seq'] == entry['source_index'] == str(n - 1), 'source_index 不连续')
        require(entry['raw_text_sha256'] == sha(event['raw_text'].encode()), 'raw_text hash 错误')
        identity = {key: env[key] for key in ('schema_revision', 'session_id', 'session_generation',
                    'source_namespace', 'source_epoch', 'producer_id', 'producer_epoch')}
        if identities is None:
            identities = identity
        require(identity == identities, '同域中途换身份')
        require(payload['target_session_id'] == env['session_id'] == raw['session_id']
                and payload['target_session_generation'] == env['session_generation'] == '1'
                and raw['source_namespace'] == env['source_namespace']
                and raw['source_epoch'] == env['source_epoch']
                and payload['writer_epoch'] == '1', 'source/session 绑定错误')
        tick = clock['events'][payload['clock_event_id']]
        require(len(tick['attempts']) == 1, '正式 schedule 必须单次 attempt')
        values = [positive(tick['accept_ns'], 'accept_ns'),
                  positive(tick['attempts'][0]['begin_ns'], 'begin_ns'),
                  positive(tick['attempts'][0]['commit_ns'], 'commit_ns')]
        require(time_frontier < values[0] < values[1] < values[2], 'clock 时序不单调')
        time_frontier = values[2]
    return dict(count=count, source_sha256=SOURCE_SHA, clock_sha256=sha(clock_raw),
                envelopes_sha256=sha(envelopes), schedule_sha256=sha(canonical(schedule)),
                max_lf_frame_bytes=max(map(len, lines)), identities=identities,
                ordered_request_hashes=[sha(line[:-1]) for line in lines])


def exact(left, right):
    """避免 Python True == 1 == 1.0；JSON/SQLite 元数据按原类型比较。"""
    if type(left) is not type(right):
        return False
    if type(left) is dict:
        return left.keys() == right.keys() and all(exact(left[k], right[k]) for k in left)
    if type(left) is list:
        return len(left) == len(right) and all(exact(a, b) for a, b in zip(left, right))
    return left == right


def validate_contract(contract, material, frozen):
    """只接受从具名 Root 原件逐值派生的执行计划，不新增容量/批准门。"""
    require(exact(contract, frozen), '执行计划偏离 Root 冻结逐值输入')
    require(contract['ticket'] == 1394 and contract['scope'] == SCOPE, 'Root 计划名分错误')
    for key in ('source_sha256', 'clock_sha256', 'envelopes_sha256', 'schedule_sha256'):
        require(contract[key] == material[key], 'Root 材料身份不符：' + key)
    resources = contract['s_resources']
    require(type(resources) is dict and set(resources) == RESOURCE_FIELDS, '资源缺失/多余；必须原七字段')
    for key, value in resources.items():
        positive(value, 's_resources.' + key)
    require(int(resources['max_frame_bytes']) >= material['max_lf_frame_bytes'], '原帧门不足')
    domains = contract['domains']
    require(type(domains) is list and [d['label'] for d in domains] == ['A', 'B'], '重复/缺失域或 A/B 重排')
    for key in ('db', 'state_dir', 'socket'):
        require(len({d[key] for d in domains}) == 2, '重复状态域：' + key)
    return contract


def cell(value):
    """SQLite 原类型和值：TEXT 不解析，BLOB 不重新编码，REAL 保存 IEEE 原精度。"""
    if value is None:
        return {'type': 'null', 'value': None}
    if type(value) is bytes:
        return {'type': 'blob', 'value': value.hex()}
    if type(value) is str:
        return {'type': 'text', 'value': value}
    if type(value) is int:
        return {'type': 'integer', 'value': str(value)}
    if type(value) is float:
        return {'type': 'real', 'value': value.hex()}
    raise Rejected('未知 SQLite 单元类型')


def uncell(item):
    require(type(item) is dict and set(item) == {'type', 'value'}, '原类型单元形状错误')
    kind, value = item['type'], item['value']
    if kind == 'blob':
        require(type(value) is str and re.fullmatch(r'(?:[0-9a-f]{2})*', value) is not None, 'BLOB 不是原小写 hex')
        return bytes.fromhex(value)
    if kind == 'integer':
        require(type(value) is str and re.fullmatch(r'0|-?[1-9][0-9]*', value) is not None, 'INTEGER 非规范')
        return int(value)
    if kind == 'real':
        require(type(value) is str, 'REAL 必须原 hex 文本')
        number = float.fromhex(value)
        require(number.hex() == value, 'REAL 原精度格式错误')
        return number
    require((kind == 'null' and value is None) or (kind == 'text' and type(value) is str), '单元原类型错误')
    return value


def table_rows(table):
    names = table['names']
    require(type(names) is list and all(type(n) is str for n in names), '表列名称原类型错误')
    require(type(table['rows']) is list and type(table['row_count']) is int and table['row_count'] >= 0, '表行数必须 INTEGER，禁止 bool/float')
    require(len(names) == len(set(names)), '重复表列')
    require(table['row_count'] == len(table['rows']) and table['sha256'] == sha(canonical(table['rows'])), '表行数/hash 错误')
    values = []
    for row in table['rows']:
        require(type(row) is list and len(row) == len(names), '表行少列')
        values.append(dict(zip(names, map(uncell, row))))
    return values


def native_frame(tag, body):
    require(type(tag) is int and 0 <= tag <= 255 and type(body) is bytes, 'native 节点原类型错误')
    return hashlib.sha256(b'NewChanlun/SContent/1\x00' + bytes([tag])
                          + len(body).to_bytes(8, 'big') + body).digest()


def validate_snapshot(snapshot):
    require(exact(snapshot['integrity_check'], [['ok']]) and exact(snapshot['foreign_key_check'], []), 'SQLite 一致性门失败')
    schema = table_rows(snapshot['schema'])
    require(snapshot['schema']['names'] == ['type', 'name', 'tbl_name', 'rootpage', 'sql'], 'schema 缺完整原列')
    schema_names = set()
    for row in schema:
        require(all(type(row[k]) is str for k in ('type', 'name', 'tbl_name'))
                and type(row['rootpage']) is int and (row['sql'] is None or type(row['sql']) is str), 'schema 原类型错误')
        require((row['type'], row['name']) not in schema_names, '重复 schema 对象')
        schema_names.add((row['type'], row['name']))
    tables = snapshot['tables']
    require(set(tables) == {r['name'] for r in schema if r['type'] == 'table'}, 'schema/table 完整集合不符')
    for value in tables.values():
        table_rows(value)
        require(exact(value['rows'], sorted(value['rows'], key=canonical)), '表 rows 顺序未冻结')
        columns = value['columns']
        require(type(columns) is list and len(columns) == len(value['names']), '每表列清单缺失')
        for cid, col in enumerate(columns):
            require(type(col) is list and len(col) == 7, 'table_xinfo 必须完整七元信息')
            require(type(col[0]) is int and col[0] == cid and type(col[1]) is str
                    and col[1] == value['names'][cid] and type(col[2]) is str
                    and type(col[3]) is int and col[3] in (0, 1)
                    and (col[4] is None or type(col[4]) is str)
                    and type(col[5]) is int and col[5] >= 0
                    and type(col[6]) is int and col[6] in (0, 1, 2, 3), 'table_xinfo 元信息原类型/次序错误')
    properties = snapshot['db_properties']
    require(set(properties) == {'user_version', 'application_id', 'encoding', 'page_size', 'auto_vacuum'}
            and type(properties['encoding']) is str
            and all(type(v) is int for k, v in properties.items() if k != 'encoding'), 'DB properties 原类型错误')
    require(exact(table_rows(tables['s_content_meta']), [{'singleton': 1, 'content_format': 's-content/1'}]), '内容格式不符')
    nodes = table_rows(tables['s_content_nodes'])
    digests = set()
    for node in nodes:
        require(type(node['digest']) is bytes and node['digest'] == native_frame(node['tag'], node['body']), 'native hash 不符原 BLOB token')
        require(node['digest'] not in digests, '重复 native 节点')
        digests.add(node['digest'])
    for publication in table_rows(tables['s_content_publications']):
        for key in ('manifest_root', 'snapshot_root', 'delta_root'):
            require(type(publication[key]) is bytes and publication[key] in digests, 'publication 悬空/非 BLOB native root')
    return True


def validate_receipt(saved, raw, ordinal, reply_payload, clock_raw, *, frontier_ordinal=None):
    env = decode(raw)
    receipts = table_rows(saved['receipt'])
    publications = table_rows(saved['publication'])
    deltas = table_rows(saved['delta'])
    ticks = table_rows(saved['clock_events'])
    protocol = table_rows(saved['protocol'])
    require(len(receipts) == len(publications) == len(deltas) == len(protocol) == 1, '缺少唯一持久收据/publication/delta/protocol')
    receipt, publication, delta, meta = receipts[0], publications[0], deltas[0], protocol[0]
    # native SCHEMA_V2 / record_accept_context / record_begin_context / record_commit。
    for row, fields in ((receipt, ('accepted_seq', 'attempt_count', 'first_published_generation')),
                        (publication, ('generation',)), (delta, ('generation', 'input_frontier')),
                        (meta, ('next_attempt_ordinal',))):
        require(all(type(row.get(k)) is int for k in fields), '持久 INTEGER 列混为 bool/REAL/TEXT')
    require(receipt['accepted_seq'] == ordinal - 1 and receipt['attempt_count'] == 1, '持久 accepted_seq/attempt_count 不符单次序列')
    frontier_ordinal = ordinal if frontier_ordinal is None else frontier_ordinal
    clock = decode(clock_raw)
    require(meta['next_attempt_ordinal'] == frontier_ordinal + 1
            and type(meta['logical_phase_frontier']) is str
            and meta['logical_phase_frontier'] == clock['events'][f'input-{frontier_ordinal}']['attempts'][0]['commit_ns'],
            '持久 next_attempt_ordinal/logical_phase_frontier 偏离原单次序列')
    for key in ('source_namespace', 'source_epoch', 'message_id', 'payload_hash'):
        require(receipt[key] == env[key], '持久 receipt 原因果身份错误：' + key)
    require(receipt['canonical_envelope'].encode() + b'\n' == raw, '持久 receipt 不是原请求字节')
    require(receipt['status'] == reply_payload['kind'] == 'Committed'
            and reply_payload['ok'] is True
            and reply_payload['original_message_id'] == env['message_id']
            and receipt['receipt_id'] == reply_payload['receipt_id']
            and receipt['first_published_generation'] == ordinal, '未 Committed 或收据/代号不符')
    require(receipt['clock_event_id'] == env['payload']['clock_event_id'], 'receipt clock 错序')
    public = reply_payload['publication']
    require(public['generation'] == str(ordinal) and publication['generation'] == delta['generation'] == ordinal
            and public['index_frontier'] == publication['batch_id'] == delta['index_frontier'], 'native publication 与同代 Delta 不一致')
    require(public['structure_cut'] == delta['next_cut']
            and public['catalog_revision'] == delta['catalog_revision']
            and public['input_frontier'] == str(delta['input_frontier'])
            and reply_payload['accepted_seq'] == str(receipt['accepted_seq']) == str(ordinal - 1)
            and receipt['attempt_count'] == 1, 'reply/receipt/cut/input frontier 不一致或发生重试')
    require(meta['protocol_revision'] == 's-session/3'
            and meta['session_generation'] == env['session_generation']
            and meta['clock_plan_hash'] == sha(clock_raw)
            and meta['clock_plan_json'].encode() == clock_raw, '持久完整 clock 上下文错误')
    require(len(ticks) == 3 and {r['phase'] for r in ticks} == {'accept', 'begin', 'commit'}, 'clock 持久 phase 缺失/重复')
    tick = clock['events'][env['payload']['clock_event_id']]
    message_key = canonical([env[k] for k in ('source_namespace', 'source_epoch', 'message_id')]).decode()
    for row in ticks:
        require(row['clock_event_id'] == env['payload']['clock_event_id'], 'clock 事件换序')
        expected = tick['accept_ns'] if row['phase'] == 'accept' else tick['attempts'][0][row['phase'] + '_ns']
        require(type(row['semantic_ns']) is str and row['semantic_ns'] == expected, '同 clock 不同持久时序/原类型')
        accept = row['phase'] == 'accept'
        require(type(row['attempt_index']) is int and row['attempt_index'] == (-1 if accept else 0), 'clock attempt_index 不符 native 单次序列')
        require(type(row['message_key']) is str and row['message_key'] == message_key, 'clock message_key 未关联原 header')
        require((row['attempt_ordinal'] is None if accept else
                 type(row['attempt_ordinal']) is int and row['attempt_ordinal'] == ordinal), 'clock attempt_ordinal 不符 native 序列')
        require(type(row['generation']) is int and row['generation'] == (ordinal - 1 if accept else ordinal), 'clock generation 不符 native 序列')
    return True


def validate_final(snapshot, lines, clock_raw, *, count=COUNT):
    """验证最终同 cut 域未丢持久原件；不把 A/B 同错当身份通过。"""
    require(len(lines) == count, '最终输入范围不完整')
    tables = snapshot['tables']
    receipts = table_rows(tables['s_input_messages'])
    publications = table_rows(tables['s_content_publications'])
    deltas = table_rows(tables['structure_deltas'])
    ticks = table_rows(tables['s_clock_events'])
    protocol = table_rows(tables['s_protocol_meta'])
    require(len(receipts) == len(publications) == len(deltas) == count
            and len(ticks) == 3 * count and len(protocol) == 1, '最终 receipt/publication/Delta/clock 缺条')
    require(all(type(r['generation']) is int for r in publications + deltas), '最终 generation 必须 INTEGER')
    require(sorted(row['generation'] for row in publications) == list(range(1, count + 1)), '最终 publication 缺代')
    require(sorted(row['generation'] for row in deltas) == list(range(1, count + 1)), '最终 Delta 缺代')
    meta = protocol[0]
    require(type(meta['next_attempt_ordinal']) is int and meta['next_attempt_ordinal'] == count + 1, '最终 attempt ordinal 不连续/有额外尝试')
    by_id = {row['message_id']: row for row in receipts}
    by_pub = {row['generation']: row for row in publications}
    by_delta = {row['generation']: row for row in deltas}
    require(len(by_id) == count, '最终重复消息 identity')
    clocks = {}
    for row in ticks:
        clocks.setdefault(row['clock_event_id'], []).append(row)
    for n, raw in enumerate(lines, 1):
        receipt = by_id[f'message-{n}']
        public, delta = by_pub[n], by_delta[n]
        # 用最终原持久域再次核同一语义，wire 原响应另由 verifier 原生校验。
        payload = dict(kind='Committed', ok=True, original_message_id=f'message-{n}',
                       receipt_id=receipt['receipt_id'], accepted_seq=str(receipt['accepted_seq']),
                       publication=dict(generation=str(n), index_frontier=public['batch_id'],
                                        structure_cut=delta['next_cut'], catalog_revision=delta['catalog_revision'],
                                        input_frontier=str(delta['input_frontier'])))
        saved = {'receipt': from_rows([receipt]), 'publication': from_rows([public]),
                 'delta': from_rows([delta]), 'clock_events': from_rows(clocks[f'input-{n}']),
                 'protocol': from_rows(protocol)}
        validate_receipt(saved, raw, n, payload, clock_raw, frontier_ordinal=count)
    raw_events = table_rows(tables['raw_events'])
    prices = table_rows(tables['raw_ohlc'])
    require(len(raw_events) == len(prices) == count
            and sorted(row['seq'] for row in raw_events) == list(range(count)), '最终 raw history 缺条/错序')
    require(all(type(r['seq']) is int and type(r['revision']) is int for r in raw_events)
            and all(type(r['revision']) is int for r in prices), 'raw seq/revision 原类型错误')
    price_index = {(row['identity_key'], row['revision']): row for row in prices}
    require(len(price_index) == count, 'raw_ohlc 重复 identity')
    for row in raw_events:
        raw_input = decode(lines[row['seq']])['payload']['raw_input']
        event = raw_input['events'][0]
        identity = '|'.join(raw_input[key] for key in ('source_namespace', 'source_epoch', 'instrument')) + '|' + event['event_id']
        native_event = {key: event[key] for key in ('event_id', 'revision', 'seq', 'received_at',
                        'raw_text', 'open', 'high', 'low', 'close', 'timestamp', 'volume')}
        event_hash = sha(canonical(native_event))
        raw_expected = dict(identity_key=identity, revision=positive(event['revision'], 'revision'),
            input_revision=event['revision'], payload_hash=event_hash,
            receipt_id='rcpt-' + sha((identity + '|' + event_hash).encode())[:16],
            seq=int(event['seq']), source_namespace=raw_input['source_namespace'],
            source_epoch=raw_input['source_epoch'], instrument=raw_input['instrument'],
            event_id=event['event_id'], received_at=event['received_at'],
            raw_text=event['raw_text'], price=event['close'], ts=event['timestamp'],
            volume=event['volume'], source_coord=event['seq'], supersedes_revision=None)
        for key, value in raw_expected.items():
            require(exact(row[key], value), '最终 raw history 原字段/类型不符：' + key)
        price = price_index[(row['identity_key'], row['revision'])]
        price_expected = dict(identity_key=identity, revision=raw_expected['revision'],
                              schema_revision='s-ohlc/1',
                              **{key: event[key] for key in ('open', 'high', 'low', 'close')})
        for key, value in price_expected.items():
            require(exact(price[key], value), '最终 OHLC 原字段/类型不符：' + key)
    return True


def from_rows(rows):
    require(bool(rows), '必需持久表没有行')
    names = list(rows[0])
    encoded = sorted([[cell(row[key]) for key in names] for row in rows], key=canonical)
    return dict(names=names, rows=encoded, row_count=len(rows), sha256=sha(canonical(encoded)))


def completion(cases, comparisons, originals_unchanged, *, count=COUNT):
    """任何 partial、清理失败或缺观测均不可升级 PASS/complete。"""
    complete = (len(cases) == 2 and [c.get('label') for c in cases] == ['A', 'B']
                and all(c.get('status') == 'VERIFIED' and c.get('committed_count') == count
                        and type(c.get('committed_count')) is int and exact(c.get('cleanup'), {'stop_requested': True, 'status_stopped': True,
                                                'stop_confirmed': True, 'same_handles_reaped': True,
                                                'socket_removed': True, 'controller_closed': True})
                        and not c.get('errors') for c in cases)
                and comparisons.get('initialized') is True and comparisons.get('final') is True
                and comparisons.get('all_receipts_and_reply_wire') is True
                and originals_unchanged is True)
    return dict(status='PASS' if complete else 'FAIL_OR_PARTIAL', complete9011=complete,
                full_resource_capacity=False, strict_RSS_heap_IO_stage_peaks='UNMEASURED',
                crosswalk_complete=False, Client_complete=False, C_complete=False, wholegraph_complete=False)
