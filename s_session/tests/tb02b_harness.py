#!/usr/bin/env python3
"""#1392：冻结输入包装、双新进程、故障矩阵及完整语义对拍。只写指定仓外 runtime。"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
RESULT_ROOT = Path('/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1374-tb03a-implementation-20260913/evidence/issue1392-tb02b/runtime')
sys.path.insert(0, str(ROOT))
from s_service_control import canonical
from tb01c_compare import first_difference, read_records


def dump(path, value):
    with path.open('xb') as stream:
        stream.write(json.dumps(value, ensure_ascii=False, indent=2).encode() + b'\n')


def prepare(output, binary, python, node, playwright_module, port_base, result_root=None):
    output = output.resolve()
    root = RESULT_ROOT if result_root is None else Path(result_root).resolve()
    if not output.is_relative_to(root) or output == root:
        raise ValueError('输出必须是指定 runtime 下的全新具名子目录')
    output.mkdir(parents=True, exist_ok=False)
    ledger_path = ROOT / 'tests/fixtures/tb02b/raw-ledger.json'
    oracle_path = ROOT / 'tests/fixtures/tb02b/hand-oracle.json'
    ledger = json.loads(ledger_path.read_text())
    cases = {c['case_id']: c['raw_bars'] for c in ledger['cases']}
    planned = [(name, bars, name, None, None) for name, bars in cases.items()]
    revision = dict(cases['lifecycle'][5], high=18, close=18)
    for stage in ('after_begin', 'after_batch', 'before_commit', 'after_commit'):
        planned.append(('revision_' + stage, cases['lifecycle'] + [revision], None, 12, stage))
    resources = json.loads((ROOT / 'tests/fixtures/tb01c/runtime-plan.json').read_text())['s_resources']
    runs = []
    for sequence, (name, bars, oracle_case, fault, stage) in enumerate(planned):
        for arm in ('a', 'b'):
            run = output / (name + '-' + arm)
            run.mkdir()
            sid = 'tb02b-' + name
            namespace = 'testonly.ohlc.' + sid
            events, messages = {}, []
            for index, bar in enumerate(bars):
                op = 'op-' + str(index).zfill(4)
                base = 946684800000000000 + index * 10000000000
                attempts = [{'begin_ns': str(base + 100000000), 'commit_ns': str(base + 200000000)}]
                if index == fault:
                    attempts.append({'begin_ns': str(base + 1100000000), 'commit_ns': str(base + 1200000000)})
                    events['recover-001'] = {'recover_ns': str(base + 1000000000)}
                events[op] = {'accept_ns': str(base), 'attempts': attempts}
                raw = {'event_id': bar['raw_id'], 'revision': '2' if index == fault else '1', 'seq': str(bar['raw_index']),
                       'received_at': '2000-01-01T00:00:00Z', 'raw_text': canonical(bar).decode(),
                       'timestamp': str(bar['raw_index'] * 60), 'volume': '1',
                       **{k: str(bar[k]) for k in ('open', 'high', 'low', 'close')}}
                payload = {'op': 'ingest', 'target_session_id': sid, 'target_session_generation': '1',
                           'writer_epoch': '2' if fault is not None and index > fault else '1', 'clock_event_id': op,
                           'raw_input': {'schema_revision': 's-ohlc/1', 'session_id': sid, 'source_namespace': namespace,
                                         'source_epoch': '1', 'instrument': 'TEST-OHLC', 'profile': 'ohlc_integer_tb02a_v1', 'events': [raw]}}
                messages.append({'schema_revision': 's-session/2', 'session_id': sid, 'session_generation': '1',
                                 'source_namespace': namespace, 'source_epoch': '1', 'message_id': op,
                                 'producer_id': 'tb02b-input', 'producer_epoch': '1', 'causal_refs': [],
                                 'payload_hash': hashlib.sha256(canonical(payload)).hexdigest(), 'payload': payload})
            dump(run / 'clock.json', {'schema_revision': 's-clock-plan/1', 'clock_plan_id': sid,
                                     'origin_utc': '2000-01-01T00:00:00Z', 'unit': 'ns', 'events': events})
            (run / 'messages.jsonl').write_bytes(b''.join(canonical(m) + b'\n' for m in messages))
            config = {'schema_revision': 's-launcher/2', 'session_id': sid, 'session_generation': '1',
                      'source_namespace': namespace, 'source_epoch': '1', 'producer_id': 'tb02b-control',
                      'producer_epoch': '1', 'writer_epoch': '1', 'query_epoch': '1', 'delivery_retain_generations': '4',
                      'port': str(port_base + len(runs)), 'startup_timeout_ms': '10000', 'db': str(run / 'session.sqlite'),
                      'socket': '/private/tmp/tb1392-' + hashlib.sha256(str(run).encode()).hexdigest()[:16] + '.sock',
                      'binary': str(binary.resolve()), 'python': str(python.resolve()),
                      'catalog': str(ROOT / 'catalog/signed-catalog.json'), 'profile': str(ROOT / 'profiles/ohlc_integer_tb02a_v1.json'),
                      'clock_plan': str(run / 'clock.json'), 'browser': str(ROOT / 'browser/index.html'),
                      'query_resource_config': str(ROOT / 'tests/fixtures/tb01c/query-resources.json'), 's_resources': resources}
            dump(run / 'config.json', config)
            command = [str(python), str(ROOT / 'tests/tb02b_runtime.py'), '--config', str(run / 'config.json'),
                       '--messages', str(run / 'messages.jsonl'), '--output', str(run / 'evidence'), '--node', str(node),
                       '--playwright-module', str(playwright_module)]
            if fault is not None:
                command += ['--fault-index', str(fault), '--fault-stage', stage]
            if oracle_case:
                command += ['--oracle-case', oracle_case]
            if oracle_case in ('lifecycle', 'mirror_lifecycle') or fault is not None:
                command += ['--browser-lifecycle-asof', '7']
            runs.append({'case_id': name, 'arm': arm, 'directory': str(run), 'command': command, 'input_count': len(messages)})
    plan = {'schema': 'tb02b-runtime-plan/1', 'normalization': '无字段剔除、无身份重命名；诊断耗时/PID从采集起单列。语义核心逐字段硬比较。',
            'fixture_sha256': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in (ledger_path, oracle_path)},
            'expected_tables': sorted(re.findall(r'CREATE TABLE(?: IF NOT EXISTS)? (\w+)', __import__('s_query_integrity').LEGACY_SCHEMA + __import__('s_query_integrity').CONTROL_SCHEMA + __import__('s_query_integrity').tb02.SCHEMA)),
            'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(), 'rule': 'new-bi-dual-coordinate/1',
            'fault_boundaries': ['持久Begin后', '不可变批次后', '对象/关系/变化/索引提交前', '提交后回执前'], 'runs': runs}
    dump(output / 'RUN-PLAN.json', plan)
    return plan


def execute(plan, output):
    outcomes = []
    for run in plan['runs']:
        directory = Path(run['directory'])
        with (directory / 'driver.stdout').open('xb') as stdout, (directory / 'driver.stderr').open('xb') as stderr:
            try:
                process = subprocess.run(run['command'], stdout=stdout, stderr=stderr, timeout=1800)
            except subprocess.TimeoutExpired:
                # 常驻 S/Q 由监督器分别持有；驱动超时也须逐个停止并留下原回执。
                for service in ('q', 's'):
                    argv = [run['command'][0], str(ROOT / 's_service_control.py'), 'stop-' + service,
                            '--config', str(directory / 'config.json'), '--state-dir', str(directory / 'evidence/control')]
                    receipt = subprocess.run(argv, capture_output=True, timeout=30)
                    (directory / ('timeout-stop-' + service + '.stdout')).write_bytes(receipt.stdout)
                    (directory / ('timeout-stop-' + service + '.stderr')).write_bytes(receipt.stderr)
                raise
        outcomes.append({'case_id': run['case_id'], 'arm': run['arm'], 'exit': process.returncode})
        dump(directory / 'EXIT.json', outcomes[-1])
        if process.returncode:
            dump(output / 'INCOMPLETE.json', outcomes)
            raise ValueError('正式运行失败，原件已保留：' + str(directory))
    for left, right in zip(plan['runs'][::2], plan['runs'][1::2]):
        a = list(read_records(Path(left['directory']) / 'evidence/semantic-core.jsonl'))
        b = list(read_records(Path(right['directory']) / 'evidence/semantic-core.jsonl'))
        # 每个原始消息：输入结果+原身份最终收据+live/known/revisit三个完整公共候选。
        # 剩余项是事前固定协议的全部表、schema和lifecycle，不接受两份相同截断。
        required = 5 * left['input_count']
        if len(a) != required + len(plan['expected_tables']) + 2 or sorted(r['table'] for r in a if r.get('kind') == 'authoritative_table') != plan['expected_tables'] or not any(r.get('kind') == 'authoritative_schema' for r in a):
            raise ValueError('语义核心缺正式输入/公共候选/全部持久表尾部')
        difference = first_difference(a, b)
        dump(output / (left['case_id'] + '-COMPARE.json'), {'difference': difference, 'records': len(a)})
        if difference:
            raise ValueError('完整语义双跑不一致：' + left['case_id'])
    dump(output / 'RESULT.json', {'status': 'captured_and_compared', 'runs': outcomes,
                                  'independent_review': '未执行；交管理者另会话验收'})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('output', 'binary', 'python', 'node', 'playwright-module'):
        parser.add_argument('--' + name, type=Path, required=True)
    parser.add_argument('--port-base', type=int, default=18700)
    parser.add_argument('--execute', action='store_true')
    args = vars(parser.parse_args())
    run = args.pop('execute')
    plan = prepare(**args)
    if run:
        execute(plan, args['output'])
    print(json.dumps({'planned_runs': len(plan['runs']), 'executed': run}, ensure_ascii=False))


if __name__ == '__main__':
    main()
