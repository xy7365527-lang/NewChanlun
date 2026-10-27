"""复用前轮离线分页/游标，新增完整内容闭包及定义值比较。无live启动入口。"""
from collections import deque
import json
from pathlib import Path
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/"fixes-v3"))
from domain_fields import fields,equal,endpoint,formation,segment_facts,mapped,nullable
from content_closure import Encoder,Graph,Edge,JSONText,Number,WorkBudget,projection,close_projection
from fixed_cut_driver import Collector,require,canonical,strict_load


class CollectorV2(Collector):
    def closed_snapshot(self,fixed,page_size,*,max_nodes,max_bytes,max_work):
        if not hasattr(self,'work_budget'):
            self.work_budget=WorkBudget(max_work,self.budget.check)
        work_budget=self.work_budget
        require(work_budget.limit==max_work,'工作预算帽不一致；禁止重置/加帽')
        work_budget.charge()
        snapshot=self.snapshot(fixed,page_size)
        root,_=projection(snapshot,work_budget)
        work_budget.charge()
        anchor={'kind':'projection','fixed_cut':fixed,
                'cut_projection_digest':snapshot['cut_projection_digest'],'projection_ref':root}
        queue=deque([(root,[])]);seen=set();pages=[];stored_bytes=0
        while queue:
            work_budget.charge()
            target,path=queue.popleft()
            if target['root'] in seen:continue
            work_budget.charge()
            require(len(seen)<max_nodes,'内容队列超限');seen.add(target['root'])
            kind=target['kind'];unit='items' if kind in ('array','object') else 'bytes' if kind in ('string','number') else 'value'
            work_budget.charge()
            result=self.content(anchor,path,unit,1 if unit=='value' else page_size)
            for p in work_budget.iter(result['pages']):
                work_budget.scan(p)
                require(p['target_ref']==target,'内容请求root不符')
                size=len(canonical(p))
                require(size<=max_bytes-stored_bytes,'内容暂存超提案限额')
                work_budget.charge();pages.append(p);stored_bytes+=size
                def enqueue(child,step_kind,step_value):
                    # path复制和队列槽位都在分配前收费；同根重复项也不能免费排队。
                    work_budget.charge(2+len(path))
                    require(len(queue)<max_nodes,'内容队列超限')
                    queue.append((child,path+[{step_kind:step_value}]))
                if kind=='object':
                    for e in work_budget.iter(p['items']):enqueue(e['value_ref'],'field',e['key'])
                elif kind=='array':
                    for i,r in enumerate(work_budget.iter(p['items'])):enqueue(r,'index',str(int(p['offset'])+i))
                elif kind=='json_text':
                    graph=Graph(p['nodes'],max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work,work_budget=work_budget)
                    child=graph.resolve(target,[{'json_text':True}])
                    enqueue(child,'json_text',True)
        verified=close_projection(snapshot,pages,max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work,work_budget=work_budget)
        return snapshot,pages,verified


def plain(v,graph):
    # 每一次访问（含共享子图的重复访问）先扣同一预算，之后才分配结果。
    graph.budget.charge()
    if isinstance(v,JSONText):return plain(v.value,graph)
    if isinstance(v,Number):
        require(v.token.lstrip('-').isdigit(),'领域投影不接受非整数JSON数值')
        return int(v.token)
    if isinstance(v,list):return [plain(x,graph) for x in v]
    if isinstance(v,dict):return {k:plain(x,graph) for k,x in v.items()}
    return v


TESTED_KINDS={'CC-008.endpoint','CC-010.bi','CC-013.segment','CC-016.center_seed_attempt',
              'CC-016.center','CC-017.center_roles','CC-022.unit_assignment','CC-020.center_pair'}


def validate_public_payload(value, budget):
    """固定 object_records.rs:38：公共 payload 任意深度都不是 0x07 获准位置。

    必须在 plain 擦除类型之前遍历原值。每次 next、栈/迭代器分配均先收费；
    不只检查参与几何比较的字段，也不因共享子树而跳过重复访问。
    """
    budget.charge()
    stack = [budget.iter((value,))]
    while stack:
        budget.charge()
        try:
            item = next(stack[-1])
        except StopIteration:
            stack.pop()
            continue
        require(not isinstance(item, JSONText), '公共payload禁止JSONText/0x07')
        if isinstance(item, (dict, list)):
            budget.charge()
            require(len(stack) < 128, '公共payload检查深度超限')
            stack.append(budget.iter(item.values() if isinstance(item, dict) else item))


def records(verified,family='objects'):
    # verified必须是close_projection的返回；重核已验证图，不能仅相信调用者布尔值。
    require(isinstance(verified.get('graph'),Graph),'缺实际闭包图')
    graph=verified['graph'];value=graph.decode(verified['projection_ref'])
    out=[];selected=[]
    # 先枚举原始对象全集；受测 kind 不得因 payload 缺失而消失。
    for row in value['rows']:
        graph.budget.charge()
        if row['family']!=family:continue
        r=row['record']
        require(isinstance(r,dict) and isinstance(r.get('kind'),str),'对象kind缺失/非法')
        if 'payload' in r:
            validate_public_payload(r['payload'], graph.budget)
        if r['kind'] not in TESTED_KINDS:continue
        require('payload_json' not in r,'公共对象禁止内部字段payload_json：'+r['kind'])
        require('payload' in r and isinstance(r['payload'],dict),
                '受测kind缺失/null/非法payload：'+r['kind'])
        require(isinstance(r['payload'].get('data'),dict),'受测kind非法payload/data：'+r['kind'])
        graph.budget.charge()
        selected.append(r)
    for r in selected:
        payload=r['payload']
        payload=plain(payload,graph)
        require(isinstance(payload,dict) and isinstance(payload.get('data'),dict),'受测kind非法payload/data：'+r['kind'])
        graph.budget.charge()
        out.append((r,payload['data']))
    return out


def compare_domain(verified,ledger,*,require_pair=True):
    """按634f wire字段适配；最终source同名合同仍须审查绑定，不能自称产品验收。

    比较的是同一已验证projection里的对象值。语义键来自实际成员笔start_stroke，
    并逐笔核价格、组锚、根集；不会拿任意注入unit映射替代低级桥。
    """
    objects=records(verified)
    bykind={}
    for r,d in objects:bykind.setdefault(r['kind'],[]).append(d)
    low=ledger['low_level'];stroke_map={s['index']:s for s in low['strokes']}
    fractal_map={f['id']:f for f in low['fractals']}
    endpoints=bykind.get('CC-008.endpoint',[])
    require(len(endpoints)==len(low['fractals']),'低级端点数量不符')
    ep_by_anchor={}
    for d in endpoints:
        key=d.get('group_anchor');require(isinstance(key,str) and key not in ep_by_anchor,'低级端点组锚重复/非法')
        ep_by_anchor[key]=d
    for f in low['fractals']:
        require(str(f['anchor']) in ep_by_anchor,'缺低级端点')
        fields(ep_by_anchor[str(f['anchor'])],endpoint(f),'低级端点')
    bis=bykind.get('CC-010.bi',[])
    require(len(bis)==len(stroke_map),'低级笔数量不符')
    bi_by_start={};bi_ids=set()
    for d in bis:
        key=d['start']['group_anchor'];eid=d['entity_id']
        require(key not in bi_by_start and isinstance(eid,str) and eid not in bi_ids,'低级笔身份重复')
        bi_by_start[key]=d;bi_ids.add(eid)
    for s in low['strokes']:
        require(str(s['start_raw']) in bi_by_start,'缺低级笔')
        d=bi_by_start[str(s['start_raw'])]
        fields(d,{'identity_anchor':s['start_raw'],'formation':formation(s),'formed_evidence':formation(s)},'低级笔')
        for key,fid in zip(('start','end'),s['endpoint_fractals']):fields(d[key],endpoint(fractal_map[fid]),'笔分型绑定.'+key)
    actual=bykind.get('CC-013.segment',[])
    expected=ledger['segments'];pending=ledger['pending_segment']
    require(len(actual)==len(expected)+(pending is not None),'完成/进行段总数不符')
    all_units=expected+([pending] if pending else [])
    expected_by_start={str(s['start_stroke']):s for s in all_units}
    units={};seen=set();object_ids={};unit_expected={s['id']:s for s in all_units}
    object_id_set=set()
    for r,d in objects:
        if r['kind']=='CC-013.segment':
            oid=r.get('object_id')
            require(isinstance(oid,str) and oid and oid not in object_id_set,'段object_id缺失/重复')
            object_ids[d['entity_id']]=oid;object_id_set.add(oid)
    for d in actual:
        start=d['construction']['start_stroke']
        require(start in expected_by_start and start not in seen,'段起点重复/域外');seen.add(start)
        e=expected_by_start[start]
        require(isinstance(d.get('entity_id'),str) and d['entity_id'] not in units,'段实体身份缺失/重复')
        units[d['entity_id']]=e['id']
        fields(d,segment_facts(e,low['strokes']),'段')
        end=e.get('observed_through_stroke',len(stroke_map)-1)
        refs=d['stroke_refs'];ids=list(range(e['start_stroke'],end+1))
        equal([r['stroke_index'] for r in refs],ids,'成员/终结见证笔完整顺序')
        for a,i in zip(refs,ids):
            s=stroke_map[i];bi=bi_by_start[str(s['start_raw'])]
            fields(a,dict(stroke_index=i,entity_id=bi['entity_id'],start_anchor=s['start_raw'],end_anchor=s['end_raw'],
                          start_price=s['start_price'],end_price=s['end_price'],start_roots=[s['start_raw']],end_roots=[s['end_raw']]),'笔引用绑定')
    def unit_ref(d):
        uid=mapped(d['unit_id'],units,'单元');e=unit_expected[uid]
        want={'unit_id':d['unit_id'],'unit_object_id':object_ids[d['unit_id']]}
        if 'result' in e:want.update(direction=e['direction'],lo=e['low'],hi=e['high'])
        else:want['completed']=False
        equal(d,want,'单元完整投影')
        return uid
    centers=bykind.get('CC-016.center',[]);roles=bykind.get('CC-017.center_roles',[])
    require(len(centers)==len(ledger['centers'])==len(roles),'中枢/角色总数不符')
    center_map={};expected_centers={tuple(c['seed']):c for c in ledger['centers']}
    for d in centers:
        seed=tuple(mapped(x,units,'seed') for x in d['seed_units']);require(seed in expected_centers,'seed身份不符')
        e=expected_centers[seed];require(e['id'] not in center_map.values(),'重复seed中枢')
        require(isinstance(d.get('entity_id'),str) and d['entity_id'] not in center_map,'中枢实体身份重复/非法')
        center_map[d['entity_id']]=e['id']
        require([mapped(x,units,'中枢成员') for x in d['members']]==e['members'] and [mapped(x,units,'Zn成员') for x in d['zn_members']]==e['Zn'],'成员/Zn集合或顺序不符')
        rr=[r for r in roles if r['center_id']==d['entity_id']];require(len(rr)==1,'角色绑定不唯一')
        for group,names in [('CORE',['ZD','ZG']),('OUTER',['DD','GG']),('INNER_EXTREMA',['D','G'])]:
            for k in names:
                value=rr[0]['roles'][group][k]
                equal(value['value'],e[k] if k in ('ZD','ZG') else e['roles'][k],'角色值:'+k)
                require(value['frozen'] is (group=='CORE'),'冻结属性不符')
                contributors=e['contributors'].get(k)
                if contributors is None:
                    contributors=[uid for uid in e['seed'] if unit_expected[uid]['low' if k=='ZD' else 'high']==e[k]]
                require([mapped(x,units,'贡献成员') for x in value['contributors']]==contributors,'贡献成员不符')
    attempts=bykind.get('CC-016.center_seed_attempt',[])
    require(len(attempts)==len(ledger['attempts']),'seed尝试总数不符')
    attempts_by_seed={};attempt_ids=set()
    for d in attempts:
        seed=tuple(unit_ref(u) for u in d['units'])
        require(seed not in attempts_by_seed and isinstance(d.get('entity_id'),str) and d['entity_id'] not in attempt_ids,'seed尝试身份重复/非法')
        attempts_by_seed[seed]=d;attempt_ids.add(d['entity_id'])
    for e in ledger['attempts']:
        require(tuple(e['seed']) in attempts_by_seed,'缺seed尝试')
        d=attempts_by_seed[tuple(e['seed'])]
        equal(d['vector'],dict(zip(('completed','consecutive','same_level','alternating','strict'),e['condition_vector'])),'seed五条件')
        require(all(e['condition_vector']),'域外：本冻结账簿未提供失败seed排除原因的独立见证')
        fields(d,dict(core_order=e['core_order'],result='FORMED',exclusion_reason=None),'seed结论')
        target=expected_centers.get(tuple(e['seed']))
        nullable(d,'center_id',center_map,target['id'] if target else None)
        for k in ('ZD','ZG'):
            contributors=[uid for uid in e['seed'] if unit_expected[uid]['low' if k=='ZD' else 'high']==e[k]]
            equal(d[k]['value'],e[k],'seed核心.'+k)
            require([mapped(x,units,'seed贡献') for x in d[k]['contributors']]==contributors,'seed核心贡献不符')
        if target:
            center=next(c for c in centers if center_map[c['entity_id']]==target['id'])
            equal(center['seed_attempt_id'],d['entity_id'],'中枢seed见证绑定')
    assignments=bykind.get('CC-022.unit_assignment',[])
    require(len(assignments)==len(ledger['assignments']),'归属集合缺项')
    for e in ledger['assignments']:
        found=[d for d in assignments if unit_ref(d['unit'])==e['unit']]
        require(len(found)==1 and found[0]['status']==e['status'],'归属状态/唯一性')
        d=found[0]
        for wire,local in [('center_id','center'),('left_center_id','left_center'),('right_center_id','right_center')]:
            nullable(d,wire,center_map,e.get(local))
    pairs=bykind.get('CC-020.center_pair',[])
    require(len(pairs)==len(ledger['pairs']),'相邻对完整性失败；撤回后不得残留旧对')
    if require_pair:
        require(len(pairs)>0,'相邻对非空义务失败')
    inverse={'LT':'>','EQ':'=','GT':'<'}
    for e in ledger['pairs']:
        found=[d for d in pairs if mapped(d['a_center_id'],center_map,'pair左端')==e['left'] and mapped(d['b_center_id'],center_map,'pair右端')==e['right']]
        require(len(found)==1,'相邻对身份');d=found[0]
        require(d['leaf']==e['leaf'],'相邻对分支')
        connectors=[a['unit'] for a in ledger['assignments'] if a.get('left_center')==e['left'] and a.get('right_center')==e['right']]
        require([mapped(x,units,'pair连接') for x in d['connector_units']]==connectors,'pair连接成员不符')
        expected_matrix={'B.'+y+'?A.'+x:inverse[v] for xy,v in e['matrix16'].items() for x,y in [xy.split(':')]}
        require(d['matrix']==expected_matrix,'相邻对完整16格不符')
    return {'status':'SCOPED_STATIC_FIELDS_VERIFIED_LIFECYCLE_UNVERIFIED','segments':len(expected),'centers':len(centers),
            'pairs_compared':len(pairs),'product_run':'NOT_RUN',
            'verified_subquantifiers':['有限域低级端点几何/笔formation及引用绑定','段端点/construction/strict/fact_vector/终结证书',
                'seed五条件/核心/成员/Zn贡献/归属null及实体引用','相邻对连接成员及16格'],
            'unverified':{k:'UNVERIFIED' for k in ('first_visibility','known_at','AsKnown','entity_revision','raw_history','withdrawn_objects',
                'lower_source_coords_and_sealing','lower_confirmation','feature_sequence_objects','full_catalog_axes','product_source_binding')},
            'logical_work_used':verified['graph'].work,
            'limitations':'只声明逐项静态子量词；真实前缀轨迹和生命周期输出未接入，禁止据此销项'}



def compare_named_runs(actual,expected):
    """具名步骤全量领域账簿逐值比较，不删时间/关系等字段来凑相等。"""
    require(set(actual)==set(expected) and set(expected)=={'base','revise-non-zn-tail','withdraw-second-center','restore-original'},'具名步骤不闭合')
    for name in expected:require(canonical(actual[name])==canonical(expected[name]),'具名expected不符:'+name)
    return {'status':'OFFLINE_NAMED_EXPECTED_MATCH','product_run':'NOT_RUN'}


def reject_live():raise RuntimeError('authorization=false；PENDING_SOURCE；本driver不启动产品')


def compare_first_visibility(traces,ledger,bars,*,prefix_bindings):
    """未来正式输出接点：全前缀闭包 + 实际cut绑定；绝不写产品 known_at。

    traces: 按输入条数排序的 close_projection 返回值。
    prefix_bindings: 每条实际接纳后的 session_id/session_generation/cut_generation/
    input_frontier/input_prefix_sha256。只支持本轮无修订 base；修订历史另验。
    返回也不认证 known_at、AsKnown 或接纳操作真实性。
    """
    import hashlib
    require(len(traces)==len(bars)==ledger['input_count']==len(prefix_bindings),'首次可见全前缀集合不完整')
    first={};last_generation=-1;identity=None
    for count,(verified,binding) in enumerate(zip(traces,prefix_bindings),1):
        from fixed_cut_driver import natural
        fixed=plain(verified['graph'].decode(verified['projection_ref'])['fixed_cut'],verified['graph'])
        for key in ('session_id','session_generation','cut_generation','input_frontier'):
            require(binding[key]==fixed[key],'首次可见切面绑定不符:'+key)
        sid=(fixed['session_id'],fixed['session_generation'])
        require(identity is None or sid==identity,'首次可见混会话');identity=sid
        generation=natural(fixed['cut_generation'])
        require(generation>last_generation,'首次可见代际逆序');last_generation=generation
        require(natural(fixed['input_frontier'])==count-1,'首次可见输入前沿不符')
        require(binding['input_prefix_sha256']==hashlib.sha256(canonical(bars[:count])).hexdigest(),'首次可见raw前缀绑定不符')
        objs=records(verified);segment_keys={}
        for r,d in objs:
            if r['kind']=='CC-008.endpoint':key=('endpoint',d['group_anchor'])
            elif r['kind']=='CC-010.bi':key=('bi',d['start']['group_anchor'])
            elif r['kind']=='CC-013.segment' and d['result'] in ('CASE_ONE','CASE_TWO'):
                key=('segment',d['construction']['start_stroke']);segment_keys[d['entity_id']]=key[1]
            else:continue
            first.setdefault(key,count)
        for r,d in objs:
            if r['kind']=='CC-016.center':
                seed=tuple(mapped(x,segment_keys,'首次可见seed') for x in d['seed_units'])
                first.setdefault(('center',seed),count)
    expected={('endpoint',str(f['anchor'])):f['first_known_input_count'] for f in ledger['low_level']['fractals']}
    expected.update({('bi',str(s['start_raw'])):s['first_known_input_count'] for s in ledger['low_level']['strokes']})
    expected.update({('segment',str(s['start_stroke'])):s['first_known_input_count'] for s in ledger['segments']})
    starts={s['id']:str(s['start_stroke']) for s in ledger['segments']}
    expected.update({('center',tuple(starts[u] for u in c['seed'])):c['first_known_input_count'] for c in ledger['centers']})
    require(first==expected,'首次可见定义前沿不符；保留差异，不能挪expected')
    return {'status':'DEFINITION_FIRST_VISIBILITY_ONLY','prefixes':len(traces),'known_at':'UNVERIFIED',
            'AsKnown':'UNVERIFIED','product_run':'NOT_RUN'}
