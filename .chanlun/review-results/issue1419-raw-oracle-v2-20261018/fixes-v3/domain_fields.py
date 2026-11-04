"""#1419 已冻结有限域的字段映射；无产品调用，不产生产品 known_at。

预期来自旧独立账簿。数值转 wire 文本仅发生在本映射边界，布尔不当整数。
生产 adapter 的字段存在性另由静态来源核对；并非生产运行证明。
"""
from fixed_cut_driver import require,canonical


def wire(v):
    if type(v) is int:return str(v)
    if isinstance(v,list):return [wire(x) for x in v]
    if isinstance(v,dict):return {k:wire(x) for k,x in v.items()}
    return v


def equal(actual,expected,label):
    require(canonical(actual)==canonical(wire(expected)),label+'不符')


def fields(actual,expected,label):
    require(isinstance(actual,dict),label+'非对象')
    for key,value in expected.items():
        require(key in actual,label+'缺字段:'+key)
        equal(actual[key],value,label+'.'+key)


def endpoint(f):
    return dict(kind=f['kind'],merged_index=f['anchor'],group_anchor=f['anchor'],
                price=f['price'],extreme_roots=f['extreme_roots'],
                raw_first=min(f['extreme_roots']),raw_last=max(f['extreme_roots']))


def formation(s):
    return dict(merged_gap=s['merged_gap'],raw_between_actual_extrema=s['raw_between'],
                top_price=s['high'],bottom_price=s['low'],vector=[True]*3,
                failed_conditions=[],waiting_reasons=[])


def construction(strokes,start,end):
    xs=strokes[start:end+1];n=len(xs)
    overlap=[max(x['low'] for x in xs[:3]),min(x['high'] for x in xs[:3])] if n>=3 else None
    vector=[all(a['direction']!=b['direction'] for a,b in zip(xs,xs[1:])),overlap[0]<=overlap[1],
            xs[-1]['end_price']>xs[0]['start_price'] if xs[0]['direction']=='UP' else xs[-1]['end_price']<xs[0]['start_price'],
            xs[0]['direction']==xs[-1]['direction']] if n>=3 and n%2 else None
    names=['direction_alternation','closed_seed_overlap','top_gt_bottom','same_end_direction']
    return dict(start_stroke=start,stroke_indices=list(range(start,end+1)),
                length='odd_ge3' if n>=3 and n%2 else 'even_ge3' if n>=3 else f'n={n}',
                vector=vector,overlap=overlap,
                overlap_relation=None if overlap is None else 'OVERLAP' if overlap[0]<overlap[1] else 'TOUCH' if overlap[0]==overlap[1] else 'DISJOINT',
                direction=xs[0]['direction'] if vector==[True]*4 else None,
                failed_conditions=[name for name,ok in zip(names,vector) if not ok] if vector is not None else ['fewer_than_three_strokes' if n<3 else 'even_stroke_count'])


def element(e):
    return dict(high=e['high'],low=e['low'],stroke_idx=e['index'],stroke_indices=[e['index']])


def segment_facts(e,strokes):
    completed='result' in e
    through=e.get('observed_through_stroke',len(strokes)-1)
    built=construction(strokes,e['start_stroke'],e.get('end_stroke',through))
    proof=e.get('proof') or (e.get('candidates') or [None])[-1]
    strict=None;second=None
    if proof:
        a,b,c=proof['abc'];up=e['direction']=='UP'
        strict=dict(elements=[element(x) for x in (a,b,c)],raw_gap=proof['initial_gap'],
                    gap_closed_by_c=proof['gap_closed_by_c'],
                    effective_case='CASE_TWO' if proof['initial_gap'] and not proof['gap_closed_by_c'] else 'CASE_ONE',
                    closure_operator='<=' if up else '>=',closure_left=c['low' if up else 'high'],closure_right=a['high' if up else 'low'])
        if 'second_sequence' in proof and proof['second_sequence']['state']=='CONFIRMED':
            match=proof['second_sequence']['checks'][-1]
            second=dict(elements=[element(x) for x in match['elements']],top_fractal=match['top'],
                        bottom_fractal=match['bottom'],through_stroke=match['through_stroke'])
    witness=None
    for i in range(e['start_stroke'],through+1,2):
        for j in range(i+3,through+1,2):
            a=strokes[i]['end_price'];b=strokes[j]['end_price']
            if (b<=a if e['direction']=='UP' else b>=a):
                witness=dict(earlier_stroke=i,later_stroke=j,earlier_price=a,later_price=b,
                             comparison='<=' if e['direction']=='UP' else '>=')
                break
        if witness:break
    cert=dict(kind=e['result'],strict=strict,second_match=second,construction=built,
              minimum_count_met=True,through_stroke=through) if completed else None
    vector=dict(stroke_broken=witness is not None,new_segment_formed=completed,segment_broken=completed,
                stroke_break_witness=witness,scan_from_stroke=e['start_stroke'],scan_through_stroke=through,
                scan_complete=True,termination_witness=cert)
    geo=None
    if completed:
        b=strokes[e['end_stroke']]
        geo=dict(stroke_index=e['end_stroke'],group_anchor=b['end_raw'],price=b['end_price'],extreme_roots=[b['end_raw']])
    return dict(construction=built,strict=strict,fact_vector=vector,geometric_end=geo,
                result=e.get('result','NO_FIRST'),trigger_stroke=through if completed else None)


def mapped(value,mapping,label):
    require(isinstance(value,str) and value in mapping,label+'悬空/非法引用')
    return mapping[value]


def nullable(data,key,mapping,expected):
    require(key in data,'引用缺字段:'+key)
    if expected is None:require(data[key] is None,'预期wire null:'+key)
    else:require(mapped(data[key],mapping,key)==expected,'引用目标不符:'+key)
