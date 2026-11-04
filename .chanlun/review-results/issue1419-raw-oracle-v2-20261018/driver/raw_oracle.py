"""#1419 受控无包含域的定义演算；绝不调用产品核或读产品输出。

这是有显式域门的证明检查器，不是全域笔段实现。域外直接失败；不会
退回另一判据。每条raw、每个三K窗口、每笔、每个终结的证据全部输出。
引用键在 DEFINITION-REFERENCES.json 中，产品身份/版本须另由冻结源绑定。
"""
from copy import deepcopy
import hashlib
import json


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode()


def digest(value):
    return hashlib.sha256(value).hexdigest()


def need(value, reason):
    if not value:
        raise ValueError(reason)


def contains(a, b):
    return a['low'] <= b['low'] and a['high'] >= b['high']


def compare(a, b):
    return 'LT' if a < b else 'GT' if a > b else 'EQ'


def raw_to_strokes(bars):
    groups, adjacency, windows, fractals = [], [], [], []
    for i, b in enumerate(bars):
        need(b['ordinal'] == i, 'raw顺序不稠密')
        need(all(type(b[k]) is int for k in ('open','high','low','close','volume')), 'raw非整数')
        need(b['low'] <= min(b['open'], b['close']) <= max(b['open'], b['close']) <= b['high'], 'OHLC包络')
        need(b['volume'] == 1 and b['low'] < b['high'], '受控量/非退化K域')
        groups.append({'merged_index': i, 'group_anchor': i, 'raw_members': [i],
                       'high': b['high'], 'low': b['low'], 'high_roots': [i],
                       'low_roots': [i], 'reference': 'RAW_NO_INCLUSION'})
        if i:
            a = bars[i-1]
            inc = contains(a,b) or contains(b,a)
            adjacency.append({'left': i-1, 'right': i,
                              'left_contains_right': contains(a,b),
                              'right_contains_left': contains(b,a),
                              'high_order': compare(a['high'],b['high']),
                              'low_order': compare(a['low'],b['low'])})
            need(not inc, '域外：raw有包含；不能沿用单根极值证明')
    for i in range(1,len(bars)-1):
        a,b,c = bars[i-1:i+2]
        top = b['high'] > max(a['high'],c['high']) and b['low'] > max(a['low'],c['low'])
        bottom = b['high'] < min(a['high'],c['high']) and b['low'] < min(a['low'],c['low'])
        rising = a['high'] < b['high'] < c['high'] and a['low'] < b['low'] < c['low']
        falling = a['high'] > b['high'] > c['high'] and a['low'] > b['low'] > c['low']
        need(sum((top,bottom,rising,falling)) == 1,'域外：三K不属唯一严格分类')
        kind = 'TOP' if top else 'BOTTOM' if bottom else 'RISING' if rising else 'FALLING'
        windows.append({'middle':i,'members':[i-1,i,i+1], 'kind':kind,
                        'highs':[x['high'] for x in (a,b,c)],'lows':[x['low'] for x in (a,b,c)],
                        'first_known_input_count':i+2,'reference':'FRACTAL'})
        if top or bottom:
            fractals.append({'id':f'f{i}','kind':kind,'anchor':i,
                             'price': b['high'] if top else b['low'],
                             'extreme_roots':[i],'first_known_input_count':i+2})
    strokes=[]
    for j,(a,b) in enumerate(zip(fractals,fractals[1:])):
        need(a['kind'] != b['kind'],'域外：存在同类相邻分型；须先做#1343取舍')
        gap=b['anchor']-a['anchor']
        between=min(b['extreme_roots'])-max(a['extreme_roots'])-1
        support_a=list(range(a['anchor']-1,a['anchor']+2))
        support_b=list(range(b['anchor']-1,b['anchor']+2))
        up=a['kind']=='BOTTOM'
        need(gap>=3 and between>=3 and not set(support_a)&set(support_b),'新笔双坐标间隔失败')
        need((b['price']>a['price']) if up else (b['price']<a['price']),'笔方向失败')
        strokes.append({'id':f'b{j}','index':j,'direction':'UP' if up else 'DOWN',
                        'start_raw':a['anchor'],'end_raw':b['anchor'],
                        'start_price':a['price'],'end_price':b['price'],
                        'low':min(a['price'],b['price']),'high':max(a['price'],b['price']),
                        'endpoint_fractals':[a['id'],b['id']], 'endpoint_root_sets':[a['extreme_roots'],b['extreme_roots']],
                        'supports':[support_a,support_b],'merged_gap':gap,'raw_between':between,
                        'first_known_input_count':b['first_known_input_count'],
                        'confirmed_input_count':fractals[j+2]['first_known_input_count'] if j+2<len(fractals) else None,
                        'reference':'BI'})
    return {'groups':groups,'adjacency':adjacency,'all_three_bar_windows':windows,
            'fractals':fractals,'strokes':strokes,
            'edge_windows':{'missing_left':[0], 'missing_right':[len(bars)-1]},
            'same_price':{'adjacent_same_kind_fractals':0,'multiple_extreme_roots':0,
                          'policy':'same-price/keep-earlier+extreme-region/1',
                          'evidence':'逐组根集均为单根；逐对分型均异型，非相邻的同价不能触发取舍。'}}


def feature(b):
    return {k:b[k] for k in ('index','direction','low','high')}


def seq_proof(elements):
    steps=[]
    for i,e in enumerate(elements):
        prior=elements[i-1] if i else None
        inc=prior is not None and (contains(prior,e) or contains(e,prior))
        need(not inc,'域外：特征序列有包含，当前无包含证明不能使用')
        steps.append({'incoming':feature(e),'previous':feature(prior) if prior else None,
                      'contains':inc,'action':'APPEND','roots':[e['index']]})
    return steps


def second_proof(strokes, b_index, direction, peak):
    selected=[]; steps=[]; checks=[]; stream=[]
    for i in range(b_index+1,len(strokes)):
        s=strokes[i]
        matched=False
        if s['direction']==direction:
            selected.append(s)
            steps=seq_proof(selected)
            if len(selected)>=3:
                a,b,c=selected[-3:]
                top=b['high']>a['high'] and b['high']>c['high']
                bottom=b['low']<a['low'] and b['low']<c['low']
                matched=top or bottom
                checks.append({'elements':[feature(x) for x in (a,b,c)],'top':top,'bottom':bottom,
                               'through_stroke':i})
        crossed=s['high']>peak if direction=='UP' else s['low']<peak
        stream.append({'stroke':i,'selected_same_direction':s['direction']==direction,
                       'fractal_first':matched,'strict_crossing_after_fractal_check':crossed,
                       'state':'CONFIRMED' if matched else 'KILLED_BY_NEW_EXTREME' if crossed else 'OPEN_AT_PREFIX_END'})
        if matched or crossed:
            return {'steps':steps,'checks':checks,'per_stroke_order':stream,
                    'state':stream[-1]['state'],'through_stroke':i,'reference':'SECOND_SEQUENCE'}
    return {'steps':steps,'checks':checks,'per_stroke_order':stream,
            'state':'OPEN_AT_PREFIX_END','through_stroke':len(strokes)-1,'reference':'SECOND_SEQUENCE'}


def derive_segments(strokes):
    segments=[]; start=0
    while start<len(strokes):
        direction=strokes[start]['direction']; up=direction=='UP'
        opposite=[b for b in strokes[start:] if b['direction']!=direction]
        hit=None; candidates=[]
        # 按推进顺序先过域门，包括不足三元素及不成 strict 的 pending 前缀。
        # 终结之后的元素属于下一段，不借未来元素扩大当前检查域。
        advanced=[]
        for element in opposite[:2]:
            advanced.append(element);seq_proof(advanced)
        for j in range(1,len(opposite)-1):
            a,b,c=opposite[j-1:j+2]
            advanced.append(c)
            steps=seq_proof(advanced)
            strict=(b['high']>max(a['high'],c['high']) and b['low']>max(a['low'],c['low'])) if up else (b['high']<min(a['high'],c['high']) and b['low']<min(a['low'],c['low']))
            if not strict:
                continue
            # 本夹具所有候选均不互含，不触发#1490窗/并列；不实现替代宽档。
            gap=b['low']>a['high'] if up else a['low']>b['high']
            closed=c['low']<=a['high'] if up else c['high']>=a['low']
            proof={'abc':[feature(x) for x in (a,b,c)],'standardization':steps,
                   'strict_fractal':strict,'initial_gap':gap,'gap_closed_by_c':closed if gap else None,
                   'turn_window_members':[], 'reference':'SEGMENT_TERMINATION'}
            second=None
            if gap and not closed:
                second=second_proof(strokes,b['index'],direction,b['high'] if up else b['low'])
                proof['second_sequence']=second
                if second['state']!='CONFIRMED':
                    candidates.append(proof)
                    continue
            through=max(c['index'],second['through_stroke'] if second else c['index'])
            end=b['index']-1; members=strokes[start:end+1]
            need(len(members)>=3 and len(members)%2==1,'段笔数/奇数性失败')
            seed_lo=max(x['low'] for x in members[:3]); seed_hi=min(x['high'] for x in members[:3])
            need(seed_lo<seed_hi,'本夹具前三笔非退化重叠失败')
            lo=min(x['low'] for x in members); hi=max(x['high'] for x in members)
            need(members[0]['start_price']==(lo if up else hi),'段起点非反向极值')
            need(members[-1]['end_price']==(hi if up else lo),'段终点非方向极值')
            hit={'id':f'u{len(segments)}','direction':direction,'start_stroke':start,'end_stroke':end,
                 'member_strokes':[x['id'] for x in members],'member_raw_interval':[members[0]['start_raw'],members[-1]['end_raw']],
                 'low':lo,'high':hi,'first_three_overlap':[seed_lo,seed_hi],
                 'first_segment_start_roots':[x['start_raw'] for x in members if x['direction']==direction and x['start_price']==(lo if up else hi)] if not segments else None,
                 'result':'CASE_TWO' if second else 'CASE_ONE', 'proof':proof,
                 'observed_through_stroke':through,
                 'first_known_input_count':strokes[through]['first_known_input_count'],
                 'all_evidence_strokes_confirmed_input_count':strokes[through]['confirmed_input_count']}
            break
        if hit is None:
            return segments, {'id':f'u{len(segments)}','direction':direction,'start_stroke':start,
                              'member_strokes':[b['id'] for b in strokes[start:]],
                              'feature_elements':[feature(x) for x in opposite],
                              'candidates':candidates,'status':'OPEN_AT_PREFIX_END',
                              'completed':False,'reason':'尚无满足全部终结见证的下一段；不得送入strict seed'}
        segments.append(hit);start=hit['end_stroke']+1
    return segments,None


def center_facts(units, pending):
    centers=[];attempts=[];assignments=[];i=0
    while i+2<len(units):
        seed=units[i:i+3];lo=max(u['low'] for u in seed);hi=min(u['high'] for u in seed)
        vector=[True,True,True,seed[0]['direction']!=seed[1]['direction']!=seed[2]['direction'],lo<hi]
        attempts.append({'start':i,'seed':[x['id'] for x in seed], 'condition_vector':vector,
                         'ZD':lo,'ZG':hi,'core_order':compare(lo,hi),'reference':'CENTER_SEED'})
        if not all(vector):i+=1;continue
        j=i+3
        while j<len(units) and units[j]['low']<=hi and units[j]['high']>=lo:j+=1
        members=units[i:j];zn=members[::2]
        need(len(members)<9,'域外：≥9须TB02H')
        roles={'DD':min(u['low'] for u in zn),'D':max(u['low'] for u in zn),
               'G':min(u['high'] for u in zn),'GG':max(u['high'] for u in zn)}
        cid=f'c{i}'
        contributors={k:[u['id'] for u in zn if u['low' if k in ('DD','D') else 'high']==v] for k,v in roles.items()}
        centers.append({'id':cid,'seed':[x['id'] for x in seed],'ZD':lo,'ZG':hi,'roles':roles,
                        'members':[u['id'] for u in members],'Zn':[u['id'] for u in zn],
                        'contributors':contributors,'member_intersections':[{'unit':u['id'],'interval':[max(lo,u['low']),min(hi,u['high'])]} for u in members],
                        'first_known_input_count':max(u['first_known_input_count'] for u in seed),
                        'last_member':j-1,'resume':j+1,'reference':'CENTER_ROLES'})
        assignments.extend({'unit':u['id'],'status':'CENTER_MEMBER','center':cid} for u in members)
        if j<len(units):
            assignments.append({'unit':units[j]['id'],'status':'CONNECTOR','left_center':cid,'right_center':None,
                                'exclusion':{'low':units[j]['low'],'high':units[j]['high'],'ZD':lo,'ZG':hi}})
        i=j+1
    assigned={x['unit'] for x in assignments}
    assignments.extend({'unit':u['id'],'status':'NOT_YET_ASSIGNED','completed':True} for u in units if u['id'] not in assigned)
    if pending:assignments.append({'unit':pending['id'],'status':'NOT_YET_ASSIGNED','completed':False})
    pairs=[]
    for left,right in zip(centers,centers[1:]):
        for a in assignments:
            if a.get('left_center')==left['id']:a['right_center']=right['id']
        keys=['DD','ZD','ZG','GG'];l={**left['roles'],'ZD':left['ZD'],'ZG':left['ZG']};r={**right['roles'],'ZD':right['ZD'],'ZG':right['ZG']}
        matrix={x+':'+y:compare(l[x],r[y]) for x in keys for y in keys}
        leaf=('UP_NEWBIRTH' if l['GG']<r['DD'] else 'UP_EXPANSION') if l['ZG']<r['ZD'] else ('DOWN_NEWBIRTH' if r['GG']<l['DD'] else 'DOWN_EXPANSION') if r['ZG']<l['ZD'] else 'CORE_NOT_STRICTLY_SEPARATED'
        pairs.append({'left':left['id'],'right':right['id'],'endpoints_left':{k:l[k] for k in keys},
                      'endpoints_right':{k:r[k] for k in keys},'matrix16':matrix,'leaf':leaf,
                      'upper_object':'NOT_CONSTRUCTED_DEFERRED_1429','product_pair':'NOT_RUN'})
    return {'centers':centers,'attempts':attempts,'assignments':assignments,'pairs':pairs}


def derive(bars):
    low=raw_to_strokes(bars)
    segments,pending=derive_segments(low['strokes'])
    return {'qualification':'INDEPENDENT_TESTONLY_RAW_DEFINITION_EXAMPLE',
            'input_count':len(bars),'low_level':low,'segments':segments,'pending_segment':pending,
            **center_facts(segments,pending),'product_run':'NOT_RUN'}


def public_semantics(ledger):
    return {k:deepcopy(ledger[k]) for k in ('segments','pending_segment','centers','attempts','assignments','pairs')}
