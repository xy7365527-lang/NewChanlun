"""从 STRUCTURAL.jsonl 抽逐代结构读数：笔（价/raw 锚/state）、段（端点/result/终止代）、种子、segment_version 等待理由、是否存在 center 族。"""
import json,sys
out=[]
for line in open(sys.argv[1]):
    d=json.loads(line); g=d['generation']; by={}
    for o in d['objects']: by.setdefault(o['kind'],[]).append(o['payload'].get('data', o['payload']))
    bis=sorted(((b['start']['raw_first'],b['start']['price'],b['end']['raw_first'],b['end']['price'],b.get('state')) for b in by.get('CC-010.bi',[])),key=lambda t:int(t[0]))
    segs=[]
    for s in by.get('CC-013.segment',[]):
        refs=s['stroke_refs']; segs.append({'id':s['entity_id'],'dir':s['construction']['direction'],'start':[refs[0]['start_anchor'],refs[0]['start_price']],
            'geometric_end':[s['geometric_end']['group_anchor'],s['geometric_end']['price']] if s.get('geometric_end') else None,
            'result':s.get('result'),'formed_g':(s.get('formed_known_at') or {}).get('generation'),'terminated_g':(s.get('terminated_known_at') or {}).get('generation'),
            'waiting':s.get('waiting_reasons')})
    sv=(by.get('CC-056.segment_version') or [{}])[0]
    out.append({'generation':g,'bi':[list(b) for b in bis],'segments':sorted(segs,key=lambda x:x['id']),
        'seeds':sorted(x['entity_id'] for x in by.get('CC-011.segment_seed',[])),
        'segment_version':{'delivered_scope':sv.get('delivered_scope'),'deferred_scope':sv.get('deferred_scope'),'waiting_reasons':sv.get('waiting_reasons')},
        'center_kinds':sorted(k for k in by if 'center' in k or k.startswith(('CC-02','CC-03')))})
for r in out: print(json.dumps(r,ensure_ascii=False,sort_keys=True))
