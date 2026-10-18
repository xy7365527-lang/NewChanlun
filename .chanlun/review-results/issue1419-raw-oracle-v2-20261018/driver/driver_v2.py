"""复用前轮离线分页/游标，新增完整内容闭包及定义值比较。无live启动入口。"""
from collections import deque
import json
from content_closure import Encoder,Graph,Edge,JSONText,Number,projection,close_projection
from fixed_cut_driver import Collector,require,canonical


class CollectorV2(Collector):
    def closed_snapshot(self,fixed,page_size,*,max_nodes,max_bytes,max_work):
        snapshot=self.snapshot(fixed,page_size)
        root,_=projection(snapshot)
        anchor={'kind':'projection','fixed_cut':fixed,
                'cut_projection_digest':snapshot['cut_projection_digest'],'projection_ref':root}
        queue=deque([(root,[])]);seen=set();pages=[]
        while queue:
            self.budget.check()
            target,path=queue.popleft()
            if target['root'] in seen:continue
            seen.add(target['root']);require(len(seen)<=max_nodes,'内容队列超限')
            kind=target['kind'];unit='items' if kind in ('array','object') else 'bytes' if kind in ('string','number') else 'value'
            result=self.content(anchor,path,unit,1 if unit=='value' else page_size)
            for p in result['pages']:
                require(p['target_ref']==target,'内容请求root不符')
                pages.append(p)
                if kind=='object':
                    queue.extend((e['value_ref'],path+[{'field':e['key']}]) for e in p['items'])
                elif kind=='array':
                    queue.extend((r,path+[{'index':str(int(p['offset'])+i)}]) for i,r in enumerate(p['items']))
                elif kind=='json_text':
                    graph=Graph(p['nodes'],max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work)
                    child=graph.resolve(target,[{'json_text':True}])
                    queue.append((child,path+[{'json_text':True}]))
            require(sum(len(canonical(p)) for p in pages)<=max_bytes,'内容暂存超提案限额')
        verified=close_projection(snapshot,pages,max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work)
        return snapshot,pages,verified


def plain(v):
    if isinstance(v,JSONText):return plain(v.value)
    if isinstance(v,Number):
        require(v.token.lstrip('-').isdigit(),'领域投影不接受非整数JSON数值')
        return int(v.token)
    if isinstance(v,list):return [plain(x) for x in v]
    if isinstance(v,dict):return {k:plain(x) for k,x in v.items()}
    return v


def records(verified,family='objects'):
    # verified必须是close_projection的返回；重核已验证图，不能仅相信调用者布尔值。
    require(isinstance(verified.get('graph'),Graph),'缺实际闭包图')
    value=plain(verified['graph'].decode(verified['projection_ref']))
    out=[]
    for row in value['rows']:
        if row['family']!=family:continue
        r=row['record'];payload=r.get('payload_json')
        if isinstance(payload,str):payload=json.loads(payload)
        if payload is not None:out.append((r,payload['data']))
    return out


def compare_domain(verified,ledger,*,require_pair=True):
    """按634f wire字段适配；最终source同名合同仍须审查绑定，不能自称产品验收。

    比较的是同一已验证projection里的对象值。语义键来自实际成员笔start_stroke，
    并逐笔核价格、组锚、根集；不会拿任意注入unit映射替代低级桥。
    """
    objects=records(verified)
    bykind={}
    for r,d in objects:bykind.setdefault(r['kind'],[]).append(d)
    actual=bykind.get('CC-013.segment',[])
    expected=ledger['segments'];pending=ledger['pending_segment']
    require(len(actual)==len(expected)+(pending is not None),'完成/进行段总数不符')
    expected_by_start={s['start_stroke']:s for s in expected+([pending] if pending else [])}
    units={};seen=set();stroke_map={s['index']:s for s in ledger['low_level']['strokes']}
    for d in actual:
        start=int(d['construction']['start_stroke'])
        require(start in expected_by_start and start not in seen,'段起点重复/域外');seen.add(start)
        e=expected_by_start[start];units[d['entity_id']]=e['id']
        refs={int(s['stroke_index']):s for s in d['stroke_refs']}
        if 'result' in e:
            require(d['result']==e['result'] and int(d['geometric_end']['stroke_index'])==e['end_stroke'],'段终结/边界不符')
            ids=range(e['start_stroke'],e['observed_through_stroke']+1)
        else:
            require(d['result'] not in ('CASE_ONE','CASE_TWO'),'未完成尾部被伪发布')
            ids=range(start,len(stroke_map))
        for i in ids:
            require(i in refs,'缺成员/终结见证笔');a=refs[i];s=stroke_map[i]
            for k in ('start_price','end_price'):require(int(a[k])==s[k],'笔价格不符')
            for akey,skey in (('start_anchor','start_raw'),('end_anchor','end_raw')):require(int(a[akey])==s[skey],'组锚不符')
            for akey,skey in (('start_roots','start_raw'),('end_roots','end_raw')):
                # 当前wire根集是source_coord十进制文本列表；新源若改schema则硬失败后显式适配。
                require([int(x) for x in a[akey]]==[s[skey]],'极值根集不符')
    centers=bykind.get('CC-016.center',[]);roles=bykind.get('CC-017.center_roles',[])
    require(len(centers)==len(ledger['centers'])==len(roles),'中枢/角色总数不符')
    center_map={};expected_centers={tuple(c['seed']):c for c in ledger['centers']}
    for d in centers:
        seed=tuple(units[x] for x in d['seed_units']);require(seed in expected_centers,'seed身份不符')
        e=expected_centers[seed];require(e['id'] not in center_map.values(),'重复seed中枢')
        center_map[d['entity_id']]=e['id']
        require([units[x] for x in d['members']]==e['members'] and [units[x] for x in d['zn_members']]==e['Zn'],'成员/Zn集合或顺序不符')
        rr=[r for r in roles if r['center_id']==d['entity_id']];require(len(rr)==1,'角色绑定不唯一')
        for group,names in [('CORE',['ZD','ZG']),('OUTER',['DD','GG']),('INNER_EXTREMA',['D','G'])]:
            for k in names:
                value=rr[0]['roles'][group][k]
                require(int(value['value'])==(e[k] if k in ('ZD','ZG') else e['roles'][k]),'角色值不符:'+k)
                require(value['frozen']==(group=='CORE'),'冻结属性不符')
                if k in e['contributors']:
                    require([units[x] for x in value['contributors']]==e['contributors'][k],'贡献成员不符')
    assignments=bykind.get('CC-022.unit_assignment',[])
    require(len(assignments)==len(ledger['assignments']),'归属集合缺项')
    for e in ledger['assignments']:
        found=[d for d in assignments if units[d['unit']['unit_id']]==e['unit']]
        require(len(found)==1 and found[0]['status']==e['status'],'归属状态/唯一性')
        d=found[0]
        for wire,local in [('center_id','center'),('left_center_id','left_center'),('right_center_id','right_center')]:
            require(center_map.get(d.get(wire))==e.get(local),'归属两端不符')
    pairs=bykind.get('CC-020.center_pair',[])
    require(len(pairs)==len(ledger['pairs']),'相邻对完整性失败；撤回后不得残留旧对')
    if require_pair:
        require(len(pairs)>0,'相邻对非空义务失败')
    inverse={'LT':'>','EQ':'=','GT':'<'}
    for e in ledger['pairs']:
        found=[d for d in pairs if center_map[d['a_center_id']]==e['left'] and center_map[d['b_center_id']]==e['right']]
        require(len(found)==1,'相邻对身份');d=found[0]
        require(d['leaf']==e['leaf'],'相邻对分支')
        expected_matrix={'B.'+y+'?A.'+x:inverse[v] for xy,v in e['matrix16'].items() for x,y in [xy.split(':')]}
        require(d['matrix']==expected_matrix,'相邻对完整16格不符')
    return {'status':'OFFLINE_CLOSED_PROJECTION_MATCH','segments':len(expected),'centers':len(centers),
            'pairs_compared':len(pairs) if require_pair else 0,'product_run':'NOT_RUN',
            'limitations':'原始事件血缘/所有低级证据schema/历史修订/真实资源须冻结源执行验收；本比较不自行销项'}


def compare_named_runs(actual,expected):
    """具名步骤全量领域账簿逐值比较，不删时间/关系等字段来凑相等。"""
    require(set(actual)==set(expected) and set(expected)=={'base','revise-non-zn-tail','withdraw-second-center','restore-original'},'具名步骤不闭合')
    for name in expected:require(canonical(actual[name])==canonical(expected[name]),'具名expected不符:'+name)
    return {'status':'OFFLINE_NAMED_EXPECTED_MATCH','product_run':'NOT_RUN'}


def reject_live():raise RuntimeError('authorization=false；PENDING_SOURCE；本driver不启动产品')
