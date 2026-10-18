"""有限静态 wire 正控：只读 raw 和独审已核见证索引，不读 ledger/derive/比较器。

这不是第二次独立评审，也不是实际 S/Q 输出。宏观索引复用外部独审报告，
端点和区间从 RAW-INPUT 的三 K 重新取得；不重造输入或修改 expected。
"""
import json
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]


def build():
    raw=json.loads((ROOT/'RAW-INPUT.json').read_text())['bars']
    ep=[]
    for i in range(1,len(raw)-1):
        a,b,c=raw[i-1:i+2]
        if b['high']>max(a['high'],c['high']) and b['low']>max(a['low'],c['low']):kind,price='TOP',b['high']
        elif b['high']<min(a['high'],c['high']) and b['low']<min(a['low'],c['low']):kind,price='BOTTOM',b['low']
        else:continue
        ep.append(dict(kind=kind,price=price,merged_index=i,group_anchor=i,raw_first=i,raw_last=i,extreme_roots=[i]))
    assert len(ep)==35
    bs=[]
    for a,b in zip(ep,ep[1:]):
        bs.append(dict(a=a,b=b,lo=min(a['price'],b['price']),hi=max(a['price'],b['price'])))
    out=[]
    def number_text(v):
        if type(v)==int:return str(v)
        if type(v)==list:return list(map(number_text,v))
        if type(v)==dict:return {k:number_text(x) for k,x in v.items()}
        return v
    def add(kind,data):
        oid='fixture-object-'+str(len(out))
        out.append(dict(kind=kind,object_id=oid,payload_json={'data':number_text(data)}))
        return oid
    for p in ep:add('CC-008.endpoint',p)
    for i,b in enumerate(bs):
        a,z=b['a'],b['b']
        f=dict(merged_gap=z['merged_index']-a['merged_index'],raw_between_actual_extrema=z['raw_first']-a['raw_last']-1,
               top_price=b['hi'],bottom_price=b['lo'],vector=[True,True,True],failed_conditions=[],waiting_reasons=[])
        add('CC-010.bi',dict(entity_id=f'raw-bi-{i}',identity_anchor=a['group_anchor'],start=a,end=z,formation=f,formed_evidence=f))
    def feat(i):return dict(high=bs[i]['hi'],low=bs[i]['lo'],stroke_idx=i,stroke_indices=[i])
    units={};objects={}
    # 来源：RAW-V2-INDEPENDENT-REVIEW.result.validation 的全前缀/段表，非产品代际。
    through=[8,8,14,14,20,20,26,26,29,32,33]
    for j in range(11):
        start=3*j;end=start+2 if j<10 else 33;up=j%2==0
        lo=max(b['lo'] for b in bs[start:start+3]);hi=min(b['hi'] for b in bs[start:start+3])
        c=dict(start_stroke=start,stroke_indices=list(range(start,end+1)),length='odd_ge3' if j<10 else 'even_ge3',
               vector=[True]*4 if j<10 else None,overlap=[lo,hi],overlap_relation='OVERLAP',
               direction=('UP' if up else 'DOWN') if j<10 else None,failed_conditions=[] if j<10 else ['even_stroke_count'])
        strict=None;second=None;terminal=None;geo=None
        result='CASE_TWO' if j in (0,2,4,6) else 'CASE_ONE' if j<10 else 'NO_FIRST'
        if j<10:
            a,b,z=map(feat,[start+1,start+3,start+5])
            gap=b['low']>a['high'] if up else a['low']>b['high']
            closes=z['low']<=a['high'] if up else z['high']>=a['low']
            strict=dict(elements=[a,b,z],raw_gap=gap,gap_closed_by_c=closes if gap else None,effective_case=result,
                        closure_operator='<=' if up else '>=',closure_left=z['low' if up else 'high'],closure_right=a['high' if up else 'low'])
            if result=='CASE_TWO':second=dict(elements=list(map(feat,[start+4,start+6,start+8])),top_fractal=False,bottom_fractal=True,through_stroke=through[j])
            terminal=dict(kind=result,strict=strict,second_match=second,construction=c,minimum_count_met=True,through_stroke=through[j])
            geo=dict(stroke_index=end,group_anchor=bs[end]['b']['group_anchor'],price=bs[end]['b']['price'],extreme_roots=bs[end]['b']['extreme_roots'])
        breaks=[(i,k) for i in range(start,through[j]+1,2) for k in range(i+3,through[j]+1,2)
                if (bs[k]['b']['price']<=bs[i]['b']['price'] if up else bs[k]['b']['price']>=bs[i]['b']['price'])]
        w=None
        if breaks:
            i,k=breaks[0];w=dict(earlier_stroke=i,later_stroke=k,earlier_price=bs[i]['b']['price'],later_price=bs[k]['b']['price'],comparison='<=' if up else '>=')
        vector=dict(stroke_broken=bool(breaks),new_segment_formed=j<10,segment_broken=j<10,stroke_break_witness=w,
                    scan_from_stroke=start,scan_through_stroke=through[j],scan_complete=True,termination_witness=terminal)
        refs=[dict(stroke_index=i,entity_id=f'raw-bi-{i}',start_anchor=bs[i]['a']['group_anchor'],end_anchor=bs[i]['b']['group_anchor'],
                   start_price=bs[i]['a']['price'],end_price=bs[i]['b']['price'],start_roots=bs[i]['a']['extreme_roots'],end_roots=bs[i]['b']['extreme_roots']) for i in range(start,through[j]+1)]
        uid=f'unit-{j}'
        objects[uid]=add('CC-013.segment',dict(entity_id=uid,construction=c,strict=strict,fact_vector=vector,result=result,
                            trigger_stroke=through[j] if j<10 else None,geometric_end=geo,stroke_refs=refs))
        units[uid]=dict(unit_id=uid,unit_object_id=objects[uid],direction='UP' if up else 'DOWN',
                        lo=min(x['lo'] for x in bs[start:end+1]),hi=max(x['hi'] for x in bs[start:end+1])) if j<10 else dict(unit_id=uid,unit_object_id=objects[uid],completed=False)
    for first,last in [(0,4),(6,9)]:
        seed=[f'unit-{i}' for i in range(first,first+3)];members=[f'unit-{i}' for i in range(first,last+1)]
        zn=members[::2];cid=f'center-{first}';aid=f'attempt-{first}'
        values=dict(ZD=max(units[u]['lo'] for u in seed),ZG=min(units[u]['hi'] for u in seed),
                    DD=min(units[u]['lo'] for u in zn),D=max(units[u]['lo'] for u in zn),
                    G=min(units[u]['hi'] for u in zn),GG=max(units[u]['hi'] for u in zn))
        def extremum(k):
            pool=seed if k in ('ZD','ZG') else zn
            return dict(value=values[k],contributors=[u for u in pool if units[u]['lo' if k in ('ZD','DD','D') else 'hi']==values[k]])
        add('CC-016.center_seed_attempt',dict(entity_id=aid,units=[units[u] for u in seed],
            vector=dict(completed=True,consecutive=True,same_level=True,alternating=True,strict=True),
            ZD=extremum('ZD'),ZG=extremum('ZG'),core_order='LT',result='FORMED',exclusion_reason=None,center_id=cid))
        add('CC-016.center',dict(entity_id=cid,seed_attempt_id=aid,seed_units=seed,members=members,zn_members=zn))
        roles={group:{k:{**extremum(k),'frozen':group=='CORE'} for k in keys}
               for group,keys in [('CORE',['ZD','ZG']),('OUTER',['DD','GG']),('INNER_EXTREMA',['D','G'])]}
        add('CC-017.center_roles',dict(center_id=cid,roles=roles))
    for i in range(11):
        cid='center-0' if i<5 else 'center-6' if 6<=i<=9 else None
        add('CC-022.unit_assignment',dict(unit=units[f'unit-{i}'],status='CENTER_MEMBER' if cid else 'CONNECTOR' if i==5 else 'NOT_YET_ASSIGNED',
                    center_id=cid,left_center_id='center-0' if i==5 else None,right_center_id='center-6' if i==5 else None))
    matrix={f'B.{y}?A.{x}':('>' if b>a else '<' if b<a else '=')
            for x,a in zip(('DD','ZD','ZG','GG'),(800,1120,1600,2720))
            for y,b in zip(('DD','ZD','ZG','GG'),(2400,2720,3200,3520))}
    add('CC-020.center_pair',dict(a_center_id='center-0',b_center_id='center-6',connector_units=['unit-5'],leaf='UP_EXPANSION',matrix=matrix))
    return out
