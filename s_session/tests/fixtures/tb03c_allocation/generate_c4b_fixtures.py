"""#1406 C4b 来源原件构造器，不计算分配或账簿期望。"""
import copy
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def generate():
    raw = json.loads((HERE / 'RAW.json').read_text())
    manifest = json.loads((HERE / 'FROZEN-MANIFEST.json').read_text())
    template = raw['events'][3]
    cases = [('late-fee-N1', 'fee_only', 0),
             ('bust-N3', 'bust', 2), ('refund-N3-commission', 'reverse_component', 2)]
    for name, kind, offset in cases:
        index = len(raw['events'])
        original = raw['events'][3 + offset]
        event = copy.deepcopy(template)
        event['event'] = name
        event['allocation_id'] = 'alloc-' + name
        event['source'].update(external_fact_id=name, source_cursor=str(index + 1),
                               source_event_time=f'source-event-{index + 1}',
                               received_time=f'source-received-{index + 1}')
        event['stable_point']['seq'] = str(100 + index)
        ref = {key: original['source'][key] for key in
               ['source_namespace', 'source_epoch', 'external_fact_id']}
        event['depends_on'] = dict(ref)
        ref.update(allocation_id=original['allocation_id'], manifest_hash=manifest['manifest_hash'],
                   fill_allocation_policy_id='frozen-manifest-proportional-cumulative/2',
                   policy_revision='we7-frozen-1')
        fill = dict(manifest_hash=manifest['manifest_hash'], symbol='TEST-N',
                    report_kind=kind, fill_ref=ref, income_changes=dict(Pi='0', A='0', W='0'))
        if kind == 'fee_only':
            fill.update(amount='5', fee_type='exchange', cause='fill')
        elif kind == 'bust':
            fill.update({'from': '7', 'to': '5', 'reported_quantity': '2', 'component': 'acquisition_cash'})
        else:
            fill.update(component='commission')
        event['netted_fill'] = fill
        for key in ['stable_point', 'depends_on', 'netted_fill']:
            name_ref = f'{name}:{key}'
            event[key]['source_ref'] = name_ref
            raw['evidence_index']['fragments'][name_ref] = [{'json_pointer': f'/events/{index}/{key}'}]
        raw['events'].append(event)
    return raw


if __name__ == '__main__':
    (HERE / 'C4B-RAW.json').write_text(json.dumps(generate(), ensure_ascii=False, sort_keys=True, indent=2) + '\n')
