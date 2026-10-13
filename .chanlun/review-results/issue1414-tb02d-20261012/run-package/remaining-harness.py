#!/usr/bin/env python3
"""#1414：冻结计划与双新进程重放入口。运行输出须选已有可写外盘目录。"""
import argparse
import hashlib
import json
import os
import signal
import time
from pathlib import Path
import subprocess
import sys

ROOT = Path('/Users/silencehan/.codex/worktrees/goal1323-tb1414/NewChanlun/s_session')
sys.path.insert(0, str(ROOT))
from s_service_control import canonical
from tb02c_compare import compare_exact_traces, expected_layout as old_layout


def expected_layout(inputs):
    layout = old_layout(inputs)
    layout[-2]['generation'] = '27'
    layout[-1]['generation'] = str(inputs)
    return layout


def dump(path, value):
    with path.open('x') as f:
        json.dump(value, f, ensure_ascii=False, indent=2)
        f.write('\n')


def save_comparison(path, value):
    """重复复验核对既有收据；结果变化时保全原证据并明确失败。"""
    if path.exists():
        if json.loads(path.read_text()) != value:
            raise ValueError('本次复验与已有比较收据不同，原收据保留：' + str(path))
    else:
        dump(path, value)


def prepare(output, binary, python, node, playwright_module, port_base):
    output.mkdir(parents=True, exist_ok=False)
    fixture = ROOT / 'tests/fixtures/tb02d/raw-ledger.json'
    cases = {k:v for k,v in json.loads(fixture.read_text())['cases'].items() if k in ('second_open_up','second_open_down')}
    resources = json.loads((ROOT / 'tests/fixtures/tb01c/runtime-plan.json').read_text())['s_resources']
    frozen_sources = {str(p.resolve()): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in ROOT.rglob('*') if p.is_file()
                      and p.suffix in {'.py','.js','.cjs','.html','.sh','.json','.jsonl'}
                      and '__pycache__' not in p.parts}
    frozen_sources[str(binary.resolve())] = hashlib.sha256(binary.read_bytes()).hexdigest()
    runs = []
    for case, bars in cases.items():
        for arm in ('a', 'b'):
            if case == 'second_open_up' and arm == 'a':
                continue
            directory = output / f'{case}-{arm}'
            directory.mkdir()
            sid = 'tb02d-' + case
            namespace = 'testonly.ohlc.' + sid
            clocks, messages = {}, []
            for i, bar in enumerate(bars):
                operation = f'op-{i:04}'
                ns = 946684800000000000 + i * 10000000000
                clocks[operation] = {'accept_ns': str(ns), 'attempts': [{'begin_ns': str(ns+100000000), 'commit_ns': str(ns+200000000)}]}
                raw = {'event_id': bar['raw_id'], 'revision': '1', 'seq': str(i), 'received_at': '2000-01-01T00:00:00Z',
                       'raw_text': canonical(bar).decode(), 'timestamp': str(i*60), 'volume': '1',
                       **{k: str(bar[k]) for k in ('open','high','low','close')}}
                payload = {'op':'ingest','target_session_id':sid,'target_session_generation':'1','writer_epoch':'1','clock_event_id':operation,
                           'raw_input':{'schema_revision':'s-ohlc/1','session_id':sid,'source_namespace':namespace,'source_epoch':'1',
                                        'instrument':'TEST-OHLC','profile':'ohlc_integer_tb02a_v1','events':[raw]}}
                messages.append({'schema_revision':'s-session/2','session_id':sid,'session_generation':'1','source_namespace':namespace,
                                 'source_epoch':'1','message_id':operation,'producer_id':'tb02d-input','producer_epoch':'1','causal_refs':[],
                                 'payload_hash':hashlib.sha256(canonical(payload)).hexdigest(),'payload':payload})
            clocks['recover-normal'] = {'recover_ns': str(946684800000000000 + (len(bars)+1)*10000000000)}
            dump(directory/'clock.json', {'schema_revision':'s-clock-plan/1','clock_plan_id':sid,'origin_utc':'2000-01-01T00:00:00Z','unit':'ns','events':clocks})
            (directory/'messages.jsonl').write_bytes(b''.join(canonical(m)+b'\n' for m in messages))
            config = {'schema_revision':'s-launcher/2','session_id':sid,'session_generation':'1','source_namespace':namespace,'source_epoch':'1',
                      'producer_id':'tb02d-control','producer_epoch':'1','writer_epoch':'1','query_epoch':'1','delivery_retain_generations':'4',
                      'port':str(port_base+len(runs)),'startup_timeout_ms':'10000','db':str(directory/'session.sqlite'),
                      'socket':'/private/tmp/tb1414-'+hashlib.sha256(str(directory).encode()).hexdigest()[:16]+'.sock',
                      'binary':str(binary.resolve()),'python':str(python.resolve()),'catalog':str(ROOT/'catalog/signed-catalog.json'),
                      'profile':str(ROOT/'profiles/ohlc_integer_tb02a_v1.json'),'clock_plan':str(directory/'clock.json'),
                      'browser':str(ROOT/'browser/index.html'),'query_resource_config':str(ROOT/'tests/fixtures/tb01c/query-resources.json'),
                      's_resources':resources}
            dump(directory/'config.json', config)
            command = [str(python),str(ROOT/'tests/tb02d_runtime.py'),'--config',str(directory/'config.json'),'--messages',str(directory/'messages.jsonl'),
                       '--output',str(directory/'evidence'),'--node',str(node),'--playwright-module',str(playwright_module),'--case',case]
            runs.append({'case':case,'arm':arm,'directory':str(directory),'command':command,
                         'prepared_sha256':{str(p.resolve()):hashlib.sha256(p.read_bytes()).hexdigest()
                             for p in (directory/'config.json',directory/'messages.jsonl',directory/'clock.json')}})
    plan = {'schema':'tb02d-runtime-plan/1','fixture_sha256':hashlib.sha256(fixture.read_bytes()).hexdigest(),
            'oracle_sha256':hashlib.sha256((ROOT/'tests/fixtures/tb02d/hand-oracle.json').read_bytes()).hexdigest(),
            'source_sha256':frozen_sources,
            'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'normal_recovery':True,'fault_injection':False,'operation_deadline_seconds':30,'run_deadline_seconds':900,'offline_deadlines_seconds':{'collect_core':120,'frozen_oracle':180,'oracle_node':120},'scope':'two named finite OHLC cases; default resource limits unchanged','runs':runs}
    dump(output/'RUN-PLAN.json', plan)
    return plan


def execute(plan, output):
    for run in plan['runs']:
        directory = Path(run['directory'])
        with (directory/'stdout').open('x') as out, (directory/'stderr').open('x') as err:
            child = subprocess.Popen(run['command'], stdout=out, stderr=err, start_new_session=True)
            try:
                child.wait(timeout=900)
                dump(directory/'EXIT.json', {'exit':child.returncode})
                if child.returncode:
                    raise ValueError('正式验证失败，保留原件：'+str(directory))
            except BaseException as cause:
                # 超时、用户中断或其他等待错误都必须回收本run，再保留原始异常。
                cleanup = []
                reap_deadline = time.monotonic() + 5
                for sig in (signal.SIGTERM, signal.SIGKILL):
                    try:
                        os.killpg(child.pid, sig)
                    except ProcessLookupError:
                        pass
                    except BaseException as exc:
                        cleanup.append({'action':'runtime-signal-error','error':repr(exc)})
                    try:
                        remaining = max(0, reap_deadline - time.monotonic())
                        child.wait(timeout=min(1, remaining) if sig == signal.SIGTERM else remaining)
                    except subprocess.TimeoutExpired:
                        continue
                    except BaseException as exc:
                        cleanup.append({'action':'runtime-reap-error','error':repr(exc)})
                cleanup.append({'action':'runtime-reaped','returncode':child.returncode})
                # S/Q可能独立会话启动，仍按本run实际控制句柄逐项停，任一失败不跳另一项。
                for service in ('q', 's'):
                    try:
                        stopped = subprocess.run([run['command'][0], str(ROOT/'s_service_control.py'), 'stop-'+service,
                            '--config', str(directory/'config.json'), '--state-dir', str(directory/'evidence/control')], capture_output=True, timeout=30)
                        (directory/('interrupted-stop-'+service+'.stdout')).write_bytes(stopped.stdout)
                        (directory/('interrupted-stop-'+service+'.stderr')).write_bytes(stopped.stderr)
                        cleanup.append({'action':'stop-'+service,'returncode':stopped.returncode})
                    except BaseException as exc:
                        cleanup.append({'action':'stop-'+service,'error':repr(exc)})
                dump(directory/'INTERRUPTED.json', {'cause':repr(cause),'cleanup':cleanup})
                raise
    # Three fresh captures only. Composite comparison is a separate explicit gate.


def validate_source_binding(plan, run):
    """逐条核实际源码/输入/构建快照，不接受只存在manifest或当前二进制标签。"""
    directory = Path(run['directory'])
    snapshot_dir = directory/'evidence/source-before-run'
    entries = json.loads((snapshot_dir/'MANIFEST.json').read_text())
    seen = {}
    for entry in entries:
        name = entry['snapshot']
        if Path(name).name != name or entry['source'] in seen:
            raise ValueError('来源快照路径/身份不唯一')
        raw = (snapshot_dir/name).read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if len(raw) != entry['bytes'] or digest != entry['sha256']:
            raise ValueError('实际来源快照损坏：'+entry['source'])
        seen[entry['source']] = digest
    expected = {**plan['source_sha256'], **run['prepared_sha256']}
    if seen != expected:
        raise ValueError('实际来源快照未逐项绑定事前完整源码/输入/构建清单')
    config = json.loads((directory/'config.json').read_text())
    if seen.get(config['binary']) != plan['binary_sha256']:
        raise ValueError('运行二进制不匹配计划构建SHA')
    for source, digest in run['prepared_sha256'].items():
        if hashlib.sha256(Path(source).read_bytes()).hexdigest() != digest:
            raise ValueError('事前消息/配置/时钟计划已改变：'+source)

