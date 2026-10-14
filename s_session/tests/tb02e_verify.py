#!/usr/bin/env python3
"""#1419 S9（根裁 D-E (B)）：实际持久库公共 Q 值对独立冻结账簿 fixtures/tb02e；不运行中枢判据。

(B) 口径：正式运行只到第 43 条（2 个完成段 + 进行中段），核的是 TB-02-E 在正式入口与公共查询上的非空形状
（CC-022 单元归属、CC-056 版本游标与等待原因、CC-016/017 与中枢关系对象缺席），以及同一 cut 上 Q 合同与浏览器合同的判定。
"""
import argparse
from contextlib import closing
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as reader  # noqa: E402
import s_tb02_contract as contract  # noqa: E402

FIXTURE = ROOT / 'tests/fixtures/tb02e'


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def check(snapshot, expected, case):
    objects = snapshot['objects']
    data = lambda o: o['payload']['data']
    by_kind = lambda kind: [o for o in objects if o['kind'] == kind]
    n = expected['input_count']
    # 段层：完成段与进行中段的实体 id 与结果（单元投影的来源）
    segments = {data(o)['entity_id']: data(o) for o in by_kind('CC-013.segment')}
    done = sorted(e for e, d in segments.items() if d['result'] in ('CASE_ONE', 'CASE_TWO'))
    require(done == sorted(expected['completed_units']), f'{case}@{n}: 完成段 {done}')
    require(expected['in_progress_unit'] in segments and segments[expected['in_progress_unit']]['result'] not in ('CASE_ONE', 'CASE_TWO'),
            f'{case}@{n}: 进行中段')
    # 中枢类对象缺席（(B) 前缀不足三个完成单元）
    for kind in expected['center_kinds_absent']:
        require(not by_kind(kind), f'{case}@{n}: 不应出现 {kind}')
    # CC-022 单元归属：逐单元逐字段
    got = {data(o)['unit']['unit_id']: data(o) for o in by_kind('CC-022.unit_assignment')}
    require(sorted(got) == sorted(a['unit_id'] for a in expected['assignments']), f'{case}@{n}: 归属单元集 {sorted(got)}')
    for want in expected['assignments']:
        a = got[want['unit_id']]
        for k in ('status', 'reason', 'seed_from_self'):
            require(a[k] == want[k], f'{case}@{n}: {want["unit_id"]}.{k} = {a[k]}')
        require(a['center_id'] is None and a['left_center_id'] is None and a['right_center_id'] is None and a['right_pending'] is False,
                f'{case}@{n}: {want["unit_id"]} 不应挂中枢')
        if want['completed']:
            for k in ('direction', 'lo', 'hi'):
                require(a['unit'][k] == want[k], f'{case}@{n}: {want["unit_id"]}.unit.{k} = {a["unit"][k]}（成员笔方向极值）')
        else:
            require(a['unit'].get('completed') is False, f'{case}@{n}: 进行中单元形状')
    # CC-056 版本：恰一，游标与等待原因
    versions = [data(o) for o in by_kind('CC-056.center_version')]
    require(len(versions) == 1, f'{case}@{n}: center_version 个数 {len(versions)}')
    v = versions[0]
    require(v['input_frontier'] == expected['input_frontier'], f'{case}@{n}: input_frontier {v["input_frontier"]}')
    require(v['scan_cursor'] == expected['scan_cursor'], f'{case}@{n}: scan_cursor {v["scan_cursor"]}')
    require(sorted(v['waiting_reasons']) == expected['waiting_reasons'], f'{case}@{n}: waiting_reasons {v["waiting_reasons"]}')
    return {'completed': done, 'assignments': len(got), 'cursor': v['scan_cursor']['state']}


def verify(database, case, node=None):
    oracle = json.loads((FIXTURE / 'hand-oracle.json').read_text())['cases'][case]
    raw = json.loads((FIXTURE / 'raw-ledger.json').read_text())['cases'][case]
    audit = reader._query_integrity()
    with closing(reader.open_readonly(database)) as conn, conn:
        conn.execute('BEGIN')
        proof = audit.verify(audit.capture(conn), vars(reader))
    require(proof['generation'] == len(raw), 'missing complete natural input')
    states, results = [], {}
    for count, expected in oracle['cuts'].items():
        state = audit.project_state(proof, int(count), vars(reader))
        snapshot = state['snapshot']
        results[count] = check(snapshot, expected, case)
        axes = snapshot['catalog_evidence']['axes']
        for cid in ('CC-016', 'CC-017', 'CC-022'):
            axis = next(x for x in state['catalog']['items'] if x['id'] == cid)
            require(axis['implementation_status'] == 'not_implemented', 'finite domain cannot close whole axis')
        # 整 cut 中枢门只看带 payload 的对象（CC-006 形态对象无 payload，不属中枢合同）。
        records = [{'kind': o['kind'], 'object_id': o['object_id'], 'payload': o['payload']} for o in snapshot['objects'] if 'payload' in o]
        rule = snapshot['catalog_evidence']['rule_revision']
        require(rule == contract.CENTER_RULE, f'cut 规则 {rule}')
        contract.validate_center_cut(records, rule, axes)
        states.append({'state': state, 'records': records, 'axes': axes})
    if node:
        script = ("const fs=require('node:fs'),A=require(process.argv[1]);"
                  "for(const x of JSON.parse(fs.readFileSync(0,'utf8'))){const s=x.state;A.validateTB02Catalog(s.catalog);"
                  "A.validateTB02CutSources(s.snapshot);A.validateTB02Axes(s.snapshot.catalog_evidence.axes,s.snapshot.objects);"
                  "A.validateCenterCut(x.records,s.snapshot.catalog_evidence.rule_revision,x.axes);}")
        run = subprocess.run([str(node), '-e', script, str(ROOT / 'browser/tb01c-client.js')], input=json.dumps(states),
                             text=True, capture_output=True, timeout=120)
        require(run.returncode == 0, run.stderr)
    return {'status': 'passed', 'case': case, 'cuts': proof['generation'], 'oracle_cuts': results,
            'scope': 'frozen named cuts of actual Q（根裁 D-E (B)：无 seed/中枢的前缀）; full natural prefix traces separately captured and compared'}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database', type=Path, required=True)
    p.add_argument('--case', required=True)
    p.add_argument('--node', type=Path)
    print(json.dumps(verify(**vars(p.parse_args())), ensure_ascii=False))
