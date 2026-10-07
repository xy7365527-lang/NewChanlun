"""#1404：完整 JSONL 流式验真和逐字节双跑比较。

本采集器的确定序列化允许用字节相同证明全部字段相同。字节不同只记
NOT_VERIFIED，不把对象键次序差异误报为语义差异。内存随最大标量和当前
对象键集合增长，不载入整条记录/整份轨迹；这不是 S/Q 资源资格证明。
"""
import hashlib
import os
from pathlib import Path

import ijson

from tb01c_compare import InvalidTrace
from tb01c_runtime import TABLES


CHUNK_BYTES = 65536
# 冻结的 LEGACY_SCHEMA、CONTROL_SCHEMA、tb02.SCHEMA；从源码导出，非结果反推。
AUTHORITATIVE_TABLES = tuple(sorted((*TABLES, 'raw_ohlc', 'structure_facts')))
IDENTITY_PATHS = {('kind',), ('operation',), ('table',), ('generation',),
                  *(('command', key) for key in ('id', 'client', 'op', 'mode', 'generation'))}
RECORD_KEYS = {
    'actual_ingest_result': {'kind', 'operation', 'result'},
    'final_receipt': {'kind', 'operation', 'request', 'response'},
    'public_candidate': {'kind', 'command', 'candidate'},
    'authoritative_table': {'kind', 'table', 'columns', 'rows'},
    'authoritative_schema': {'kind', 'rows'},
    'lifecycle_semantics': {'kind', 'value'},
    'normal_recovery_cut': {'kind', 'generation', 'before', 'after'},
}


def expected_layout(inputs):
    if type(inputs) is not int or inputs < 28:
        raise ValueError('TB-02-C 计划须覆盖恢复 cut 16/28')
    records = [{'kind': kind, 'operation': str(i)}
               for kind in ('actual_ingest_result', 'final_receipt') for i in range(inputs)]
    for i in range(inputs):
        records.extend([
            {'kind': 'public_candidate', 'command.id': f'live-{i}',
             'command.client': 'live', 'command.op': 'load' if i == 0 else 'watch'},
            {'kind': 'public_candidate', 'command.id': f'known-{i}',
             'command.client': 'history', 'command.op': 'load',
             'command.mode': 'AsKnown', 'command.generation': str(i + 1)},
        ])
    records.extend({'kind': 'public_candidate', 'command.id': f'revisit-{i}',
                    'command.client': 'history', 'command.op': 'load',
                    'command.mode': 'AsKnown', 'command.generation': str(i + 1)}
                   for i in range(inputs))
    records.extend({'kind': 'authoritative_table', 'table': table} for table in AUTHORITATIVE_TABLES)
    records.extend([{'kind': 'authoritative_schema'}, {'kind': 'lifecycle_semantics'},
                    {'kind': 'normal_recovery_cut', 'generation': '16'},
                    {'kind': 'normal_recovery_cut', 'generation': '28'}])
    return records


class _PairedLine:
    """给解析器一行的有限块，同时硬比两侧所有原始字节。"""
    def __init__(self, left, right, hashes):
        self.left, self.right, self.hashes = left, right, hashes
        self.finished = False
        self.bytes = 0

    def read(self, size):
        if size == 0 or self.finished:
            return b''
        size = CHUNK_BYTES if size < 0 else min(size, CHUNK_BYTES)
        raw = self.left.readline(size)
        other = self.right.read(len(raw))
        if raw != other:
            raise InvalidTrace('双跑字节不同；尚未证明语义相同')
        for digest, data in zip(self.hashes, (raw, other)):
            digest.update(data)
        self.bytes += len(raw)
        self.finished = not raw or raw.endswith(b'\n')
        return raw


def _record_identity(stream):
    stack, identity = [], {}
    complete = False
    for event, value in ijson.basic_parse(stream, buf_size=CHUNK_BYTES):
        if not stack:
            if event != 'start_map' or complete:
                raise InvalidTrace('每行须为一个非空对象')
            path = ()
        else:
            parent = stack[-1]
            path = parent['path'] + ((parent['key'],) if parent['keys'] is not None else (None,))
        if event in ('start_map', 'start_array'):
            if path in IDENTITY_PATHS:
                raise InvalidTrace('记录身份字段必须是字符串')
            stack.append({'keys': set() if event == 'start_map' else None, 'key': None, 'path': path})
        elif event == 'map_key':
            frame = stack[-1]
            if value in frame['keys']:
                raise InvalidTrace('重复 JSON 对象键：' + value[:120])
            frame['keys'].add(value)
            frame['key'] = value
        elif event in ('end_map', 'end_array'):
            frame = stack.pop()
            if not stack:
                if not frame['keys']:
                    raise InvalidTrace('记录对象为空')
                required = RECORD_KEYS.get(identity.get('kind'))
                if required is not None and frame['keys'] != required:
                    raise InvalidTrace('记录顶层字段缺失或多余')
                complete = True
        else:
            if event == 'number' and type(value) is not int:
                raise InvalidTrace('轨迹含浮点或非有限数')
            if path in IDENTITY_PATHS:
                if event != 'string':
                    raise InvalidTrace('记录身份字段必须是字符串')
                identity['.'.join(path)] = value
    if not complete or stack:
        raise InvalidTrace('JSON 记录不完整')
    return identity


def _stamp(stream):
    stat = os.fstat(stream.fileno())
    return stat.st_dev, stat.st_ino, stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns


def compare_exact_traces(left, right, layout):
    """layout 来自事前输入与冻结采集协议，不从待验轨迹取长度或身份。"""
    if not layout:
        raise ValueError('缺事前完整记录布局')
    result = {'status': 'NOT_VERIFIED', 'expected_records': len(layout), 'compared_records': 0,
              'compared_bytes': 0, 'method': 'exact_bytes_and_strict_jsonl',
              'excluded_fields': [], 'identity_renaming': []}
    hashes = [hashlib.sha256(), hashlib.sha256()]
    try:
        with Path(left).open('rb') as a, Path(right).open('rb') as b:
            before = (_stamp(a), _stamp(b))
            for index, expected in enumerate(layout):
                if not a.peek(1):
                    raise InvalidTrace('两侧共同前缀未覆盖事前完整记录布局')
                line = _PairedLine(a, b, hashes)
                actual = _record_identity(line)
                if actual != expected:
                    raise InvalidTrace(f'第 {index + 1} 行身份/次序不符：{actual!r} != {expected!r}')
                result['compared_records'] += 1
                result['compared_bytes'] += line.bytes
            if a.read(1) or b.read(1):
                raise InvalidTrace('计划末尾还有额外记录/字节')
            if before != (_stamp(a), _stamp(b)):
                raise InvalidTrace('读取期间原件发生变化')
    except (InvalidTrace, ijson.JSONError, OSError, UnicodeError, ValueError) as exc:
        result['detail'] = str(exc)[:1024]
        return result
    result.update(status='PASS', sha256=[digest.hexdigest() for digest in hashes])
    return result
