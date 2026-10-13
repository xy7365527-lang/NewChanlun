#!/usr/bin/env python3
"""#1414：实际持久库公共Q值对独立冻结账簿；不运行段判据。"""
import argparse
from contextlib import closing
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
import s_readonly_server as reader


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def wire(value):
    if type(value) is int:
        return str(value)
    if isinstance(value, list):
        return [wire(v) for v in value]
    if isinstance(value, dict):
        return {k: wire(v) for k,v in value.items()}
    return value


def check(snapshot, expected, case):
    """FIRST预期是首次check见证；当前完整原成员另保留，不能把后来c当首c。"""
    objects = snapshot['objects']
    def data(kind, entity):
        found = [o['payload']['data'] for o in objects
                 if o['kind'] == kind and o['payload']['data'].get('entity_id') == entity]
        require(len(found) == 1, f'{case}: unique {kind}/{entity}')
        return found[0]
    segment = data('CC-013.segment', 'segment:1:1')
    first = data('CC-012.feature_sequence', 'first:segment:1:1')
    seed = data('CC-011.segment_seed', 'seed:1:1')['construction']
    require(seed['vector'] == expected['seed_vector'], 'seed vector')
    require(seed['overlap'] == wire(expected['seed_overlap']), 'seed closed overlap')
    require(segment['result'] == expected['result'], f'{case}: result at {expected["input_count"]}')
    require(segment['trigger_stroke'] == wire(expected['trigger_stroke']), 'actual trigger frontier')
    members = [int(e['stroke_idx']) for e in first['facts']['raw_elements']]
    require(set(expected['first_members']) <= set(members), 'FIRST original members missing')
    if expected['strict'] is not None:
        strict = expected['strict']
        require(segment['strict'] == wire({k: strict[k] for k in segment['strict']}), 'FIRST strict frozen witness')
        require(any(c['elements'] == wire(expected['first_window']) and c['first_fractal'] and not c['provisional']
                    for c in first['facts']['checks']), 'FIRST first legal check missing')
    else:
        require(segment['strict'] is None, 'inapplicable strict is not null')
    facts = segment['fact_vector']
    for key in ('stroke_broken', 'segment_broken', 'new_segment_formed'):
        require(facts[key] == expected['fact_vector'][key], 'independent fact ' + key)
    witness = expected['fact_vector']['stroke_break_witness']
    require(facts['stroke_break_witness'] == (None if witness is None else wire({k:witness[k] for k in facts['stroke_break_witness']})), 'independent stroke break witness')
    if expected['second_applicable']:
        second = data('CC-012.feature_sequence', 'second:segment:1:1')
        require(second['second_sequence_status'] == expected['second_sequence_status'], 'SECOND applicable status')
        f = second['facts']
        for actual, wanted in (('raw_elements','second_raw_elements'),('standard','second_standard')):
            require(f[actual] == wire(expected[wanted]), 'SECOND ' + actual)
        require((None if f['matched'] is None else f['matched']['elements']) == wire(expected['second_match']), 'SECOND matched triple')
        if f['matched'] is not None:
            require(f['matched']['top_fractal'] == (expected['second_fractal_kind'] == 'TOP'), 'SECOND any top')
            require(f['matched']['bottom_fractal'] == (expected['second_fractal_kind'] == 'BOTTOM'), 'SECOND any bottom')
        for k in ('from_stroke','through_stroke','input_stroke_count','resource_truncated'):
            require(f[k] == wire(expected['second_scan'][k]), 'SECOND scan ' + k)
        require(len(f['steps']) == len(expected['second_steps']), 'SECOND steps length')
        for actual, wanted in zip(f['steps'], expected['second_steps']):
            for k in ('incoming','after','action','merge_direction','scope'):
                require(actual[k] == wire(wanted[k]), 'SECOND independent inclusion ' + k)
    else:
        require(segment['second_sequence_id'] is None, 'inapplicable SECOND identity')
    if expected['geometric_end'] is None:
        require(segment['geometric_end'] is None and segment['terminated_known_at'] is None and facts['termination_witness'] is None, 'premature termination')
    else:
        require(segment['geometric_end']['group_anchor'] == wire(expected['geometric_end']), 'geometric endpoint')
        require(segment['geometric_end']['stroke_index'] == wire(expected['end_stroke']), 'geometric stroke')
        require(segment['terminated_known_at']['input_frontier'] == wire(expected['termination_first_known_raw']), 'first known raw frontier')
        require(segment['terminated_known_at']['generation'] == wire(expected['termination_first_known_input_count']), 'first known cut')
        require(facts['termination_witness'] is not None and facts['termination_witness']['through_stroke'] == segment['trigger_stroke'], 'same-evaluation termination certificate')
        if expected['input_count'] == expected['termination_first_known_input_count']:
            require(any(o['kind'] == 'CC-055.segment_change' and o['payload']['data']['entity_id'] == segment['entity_id']
                        and o['payload']['data']['change'] == 'terminated'
                        and o['payload']['data']['after']['result'] == expected['result']
                        for o in objects), 'first known cut lacks terminated change')
    return segment['result']


def verify(database, case, node=None):
    oracle = json.loads((ROOT/'tests/fixtures/tb02d/hand-oracle.json').read_text())['cases'][case]
    raw = json.loads((ROOT/'tests/fixtures/tb02d/raw-ledger.json').read_text())['cases'][case]
    audit = reader._query_integrity()
    with closing(reader.open_readonly(database)) as conn, conn:
        conn.execute('BEGIN')
        proof = audit.verify(audit.capture(conn), vars(reader))
    require(proof['generation'] == len(raw), 'missing complete natural input')
    states = []
    results = {}
    for count, expected in oracle['cuts'].items():
        state = audit.project_state(proof, int(count), vars(reader))
        results[count] = check(state['snapshot'], expected, case)
        for cid in ('CC-012','CC-013','CC-014'):
            axis = next(x for x in state['catalog']['items'] if x['id'] == cid)
            require(axis['implementation_status'] == 'not_implemented', 'finite domain cannot close whole axis')
        states.append(state)
    if node:
        script = "const fs=require('node:fs'),A=require(process.argv[1]);for(const s of JSON.parse(fs.readFileSync(0,'utf8'))){A.validateTB02Catalog(s.catalog);A.validateTB02CutSources(s.snapshot);A.validateTB02Axes(s.snapshot.catalog_evidence.axes,s.snapshot.objects);}"
        run = subprocess.run([str(node),'-e',script,str(ROOT/'browser/tb01c-client.js')], input=json.dumps(states), text=True, capture_output=True, timeout=120)
        require(run.returncode == 0, run.stderr)
    return {'status':'passed','case':case,'cuts':proof['generation'],'oracle_cuts':results,
            'scope':'frozen named cuts of actual Q; full natural prefix traces separately captured and compared'}

if __name__ == '__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--database',type=Path,required=True)
    p.add_argument('--case',required=True)
    p.add_argument('--node',type=Path)
    print(json.dumps(verify(**vars(p.parse_args())),ensure_ascii=False))
