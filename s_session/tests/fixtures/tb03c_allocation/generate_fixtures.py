"""#1406 C4a 独立来源夹具；只构造原件，不计算逐重分配。"""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':'))


def generate():
    participants = [dict(chong_id=f'chong-N{i+1}', command_id=f'cmd-N{i+1}',
                         family_id=f'fam-N{i+1}', revision='1', requested_q=str(q))
                    for i, q in enumerate([1,3,3])]
    manifest = dict(projection_kind='netted', symbol='TEST-N', participants=participants,
                    tie_break_order=[p['chong_id'] for p in participants])
    manifest['manifest_hash'] = hashlib.sha256(canonical(manifest).encode()).hexdigest()
    template = json.loads((HERE.parent/'tb03c/raw/chong-R1.json').read_text())
    raw = dict(format=template['format'], units=template['units'], events=[],
               evidence_index={'fragments': {}})
    def ref(name, path):
        raw['evidence_index']['fragments'][name] = [{'json_pointer': path}]
        return name
    def source(identity, cursor):
        return dict(source_namespace='testonly-tb1406', source_epoch='fixture-epoch-1',
                    external_fact_id=identity, source_cursor=str(cursor),
                    source_event_time=f'source-event-{cursor}', received_time=f'source-received-{cursor}',
                    claimed_origin='TestOnly C4a independent fixture')
    for i, quantity in enumerate([1,2,2]):
        identity = f'N{i+1}-open'
        base = f'/events/{i}'
        opening = dict(units=str(quantity), Pi='0', A='0', W='0')
        opening['witness_refs'] = {k: ref(f'{identity}:{k}',f'{base}/observed_existing_book/{k}')
                                   for k in ['units','Pi','A','W']}
        raw['events'].append(dict(event=identity, allocation_id=f'alloc-{identity}',family_id=f'fam-N{i+1}',
            source=source(identity,i+1),book_identity=dict(chong_id=f'chong-N{i+1}',symbol='TEST-N',op_level=str(i+1),fixed_q=str([1,3,3][i]),side='long'),
            observed_existing_book=opening,known_cost_components=[dict(kind='acquisition_cash', amount_text=str(100*quantity),canonical_amount=str(100*quantity),currency='TEST-CASH',quantum='1',source_ref=ref(f'{identity}:cost',f'{base}/known_cost_components/0'))],
            stable_point=dict(profile='TestOnly',seq='100',source_ref=ref(f'{identity}:sp',f'{base}/stable_point'))))
    for i,(before,after,price,commission) in enumerate([(0,3,100,3),(3,4,101,1),(4,7,102,5)]):
        identity=f'net-fill-{i+1}';base=f'/events/{i+3}'
        raw['events'].append(dict(event=identity,allocation_id=f'alloc-{identity}',source=source(identity,i+4),
            stable_point=dict(profile='TestOnly',seq=str(101+i),source_ref=ref(f'{identity}:sp',f'{base}/stable_point')),
            netted_fill=dict(manifest_hash=manifest['manifest_hash'],symbol='TEST-N',report_kind='incremental',reported_quantity=str(after-before),
                **{'from':str(before),'to':str(after)},unit_price=str(price),commission=str(commission),cash_kind='acquisition_cash',
                income_changes=dict(Pi='0',A='0',W='0'),source_ref=ref(f'{identity}:fill',f'{base}/netted_fill'))))
    return manifest,raw

if __name__ == '__main__':
    for name,value in zip(['FROZEN-MANIFEST.json','RAW.json'],generate()):
        (HERE/name).write_text(json.dumps(value,ensure_ascii=False,indent=2,sort_keys=True)+'\n')
