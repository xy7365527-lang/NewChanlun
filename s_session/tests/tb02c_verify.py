#!/usr/bin/env python3
"""#1404：真实已发布 S 库 → 公共 Q → 冻结账簿；不替代 HTTP/GUI 验收。"""
import argparse
from contextlib import closing
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as reader


def _require(condition, detail):
    if not condition:
        raise ValueError(detail)


def check(snapshot, raw_prefix, case):
    _require(case in ('first_up', 'first_down'), '未知的独立账簿工作例')
    oracle = json.loads((ROOT / 'tests/fixtures/tb02c/hand-oracle.json').read_text())['cuts'][str(raw_prefix)]
    sign = 1 if case == 'first_up' else -1
    records = snapshot['objects']
    def data(kind, entity):
        found = [o['payload']['data'] for o in records if o['kind'] == kind and o['payload']['data']['entity_id'] == entity]
        _require(len(found) == 1, (kind, entity))
        return found[0]
    seed = data('CC-011.segment_seed', 'seed:1:1')['construction']
    segment = data('CC-013.segment', 'segment:1:1')
    sequence = data('CC-012.feature_sequence', 'first:segment:1:1')
    _require(seed['vector'] == oracle['seed_vector'], "验收条件不成立：seed['vector'] == oracle['seed_vector']")
    interval = oracle['seed_overlap'] if sign == 1 else [-oracle['seed_overlap'][1], -oracle['seed_overlap'][0]]
    _require(seed['overlap'] == list(map(str, interval)), "验收条件不成立：seed['overlap'] == list(map(str, interval))")
    _require(seed['direction'] == ('UP' if sign == 1 else 'DOWN'), "验收条件不成立：seed['direction'] == ('UP' if sign == 1 else 'DOWN')")
    refs = sequence['stroke_refs']
    _require([int(r['start_anchor']) for r in refs] + [int(refs[-1]['end_anchor'])] == oracle['stroke_anchors'], "验收条件不成立：[int(r['start_anchor']) for r in refs] + [int(refs[-1]['end_anchor'])] == oracle['stroke_anchors']")
    _require([int(r['start_price']) for r in refs] + [int(refs[-1]['end_price'])] == [v * sign for v in oracle['prices']], "验收条件不成立：[int(r['start_price']) for r in refs] + [int(refs[-1]['end_price'])] == [v * sign for v in oracle['prices']]")
    _require([int(e['stroke_idx']) for e in sequence['facts']['raw_elements']] == oracle['first_members'], "验收条件不成立：[int(e['stroke_idx']) for e in sequence['facts']['raw_elements']] == oracle['first_members']")
    _require(segment['result'] == oracle['result'], "验收条件不成立：segment['result'] == oracle['result']")
    if raw_prefix == 15:
        _require(segment['geometric_end'] is None and segment['terminated_known_at'] is None, "验收条件不成立：segment['geometric_end'] is None and segment['terminated_known_at'] is None")
        _require(oracle['reason'] in segment['waiting_reasons'], "验收条件不成立：oracle['reason'] in segment['waiting_reasons']")
    else:
        intervals = oracle['standard_intervals'] if sign == 1 else [[-hi, -lo] for lo, hi in oracle['standard_intervals']]
        _require([[int(e['low']), int(e['high'])] for e in sequence['facts']['standard']] == intervals, "验收条件不成立：[[int(e['low']), int(e['high'])] for e in sequence['facts']['standard']] == intervals")
        _require(sequence['facts']['checks'][-1]['gap'] is False, "验收条件不成立：sequence['facts']['checks'][-1]['gap'] is False")
        _require(segment['geometric_end']['group_anchor'] == str(oracle['geometric_end']), "验收条件不成立：segment['geometric_end']['group_anchor'] == str(oracle['geometric_end'])")
        _require(segment['geometric_end']['stroke_index'] == str(oracle['end_stroke']), "验收条件不成立：segment['geometric_end']['stroke_index'] == str(oracle['end_stroke'])")
        _require(segment['trigger_stroke'] == str(oracle['trigger_stroke']), "验收条件不成立：segment['trigger_stroke'] == str(oracle['trigger_stroke'])")
        # 单根自然到达序列下，末端 raw25 的右组 raw26 才能使第六笔成立。
        _require(int(segment['terminated_known_at']['input_frontier']) >= 26, "验收条件不成立：int(segment['terminated_known_at']['input_frontier']) >= 26")
        _require(data('CC-013.segment', 'segment:1:13')['result'] == 'NO_FIRST', "验收条件不成立：data('CC-013.segment', 'segment:1:13')['result'] == 'NO_FIRST'")
        _require(any(o['kind'] == 'CC-055.segment_relation' for o in records), "验收条件不成立：any(o['kind'] == 'CC-055.segment_relation' for o in records)")
    _require(all(o['payload']['data'].get('result') != 'CASE_TWO' for o in records if o['kind'].startswith('CC-013.')), "验收条件不成立：all(o['payload']['data'].get('result') != 'CASE_TWO' for o in records if o['kind'].startswith('CC-013.'))")


def verify(database, case, node=None):
    audit = reader._query_integrity()
    with closing(reader.open_readonly(database)) as conn, conn:
        conn.execute('BEGIN')
        proof = audit.verify(audit.capture(conn), vars(reader))
    # 本入口计划是一根输入一代；不把裁到15/27的最终列表充作自然前缀。
    states = [audit.project_state(proof, n, vars(reader)) for n in range(1, proof['generation'] + 1)]
    _require(len(states) == 28, '缺自然到达的完整28个cut')
    for state in states:
        for cid in ('CC-012', 'CC-013'):
            axis = next(item for item in state['catalog']['items'] if item['id'] == cid)
            _require(axis['implementation_status'] == 'not_implemented', '第一种不冒充整个两情况轴已交付')
        count = sum(o['kind'] == 'CC-010.bi' for o in state['snapshot']['objects'])
        if count < 3:
            _require(not any(o['kind'] in ('CC-011.segment_seed', 'CC-013.segment') for o in state['snapshot']['objects']), "验收条件不成立：not any(o['kind'] in ('CC-011.segment_seed', 'CC-013.segment') for o in state['snapshot']['objects'])")
    for prefix in (15, 27):
        check(states[prefix]['snapshot'], prefix, case)
    if node:
        script = """const fs=require('node:fs'), A=require(process.argv[1]);
for(const s of JSON.parse(fs.readFileSync(0,'utf8'))){A.validateTB02Catalog(s.catalog);A.validateTB02CutSources(s.snapshot);A.validateTB02Axes(s.snapshot.catalog_evidence.axes,s.snapshot.objects);}console.log('28 cuts');"""
        result = subprocess.run([str(node), '-e', script, str(ROOT / 'browser/tb01c-client.js')], input=json.dumps(states), text=True, capture_output=True, timeout=60)
        if result.returncode:
            raise ValueError(result.stderr)
    return {'status': 'passed', 'cuts': len(states), 'case': case, 'scope': '真实持久库公共Q/观察器合同，不替代HTTP与GUI'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--database', type=Path, required=True)
    parser.add_argument('--case', choices=('first_up', 'first_down'), required=True)
    parser.add_argument('--node', type=Path)
    print(json.dumps(verify(**vars(parser.parse_args())), ensure_ascii=False))
