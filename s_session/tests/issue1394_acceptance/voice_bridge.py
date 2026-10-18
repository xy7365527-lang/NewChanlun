"""同域候选编组：只读实际来源与独立γ证明，不生成γ、不选择候选、不签独评。

尚未冻结的生产terminal/K格式由根显式字段映射接入，不提供猜测默认值。
输出不是voice-binding-authority/1；必须经真实独评后另行冻结正式bundle。
"""
import copy
from pathlib import Path
import driver as d
from captured_cut import decimal, fingerprint, MIB

TERMINAL_FIELDS = {'profile','execution_level','confirmation_level','direction','anchor','start','end','confirmed_at','labels','source_ids'}
CARRIER_FIELDS = {'profile','level','direction','anchor','start','end','children','source_ids'}
SOURCE_FIELDS = {'profile','start','end'}


def pointer(obj, path):
    d.require(isinstance(path,str) and path.startswith('/'),'缺生产字段JSON pointer')
    for key in path[1:].split('/'):
        key=key.replace('~1','/').replace('~0','~')
        obj=obj[int(key)] if isinstance(obj,list) else obj[key]
    return obj


def artifacts(raw_by_name, entries, budget):
    d.require(type(entries) is list and entries,'缺完整原件清单')
    total=0;names=set()
    for entry in entries:
        d.exact(entry,{'name','path','sha256','bytes'},'authority artifact')
        name=entry['name'];d.require(name not in names,'原件重名');names.add(name)
        size=decimal(entry['bytes']);total+=size
        d.require(0<size<=64*MIB and total<=128*MIB,'权威单件64MiB/集合128MiB帽')
        d.require(Path(entry['path']).is_absolute(),'原件路径必须绝对')
    d.require(names==set(raw_by_name),'原件全集多余/遗漏')
    d.require({'input','controlled_gammas','carrier_objects','instrument_declaration','opening_raw',
               'terminal_objects','source_objects','gamma_proof','clock_mapping'}<=names,'缺terminal/K/source/独立γ原件')
    for entry in entries:
        raw=raw_by_name[entry['name']];budget.charge((raw,))
        d.require(len(raw)==decimal(entry['bytes']) and d.sha(raw)==entry['sha256'],'权威原件字节不同')


def assemble(facts, contract_raw, expected_sha, raw_by_name, budget):
    """捕获facts及全部raw须由管理者冻结。返回候选全集或明确缺项，不授予APPROVE。"""
    budget.charge((contract_raw,));d.require(d.sha(contract_raw)==expected_sha,'桥合同pin不同')
    contract=d.strict(contract_raw)
    d.require(contract['schema_revision']=='issue1394-voice-bridge/1' and contract['execution_authorized'] is False,'不是禁执行桥合同')
    d.require(facts['schema_revision']=='issue1394-captured-facts/1' and facts['execution_authorized'] is False
              and facts['status']=='UNAUTHENTICATED_FACTS' and fingerprint(facts)==contract['facts_sha256'],'固定cut事实不同')
    mapping=contract['mapping']
    d.exact(mapping,{'terminal_kinds','carrier_kinds','terminal','carrier','source'},'生产字段映射')
    d.exact(mapping['terminal'],TERMINAL_FIELDS,'terminal字段映射');d.exact(mapping['carrier'],CARRIER_FIELDS,'K字段映射')
    d.exact(mapping['source'],SOURCE_FIELDS,'来源区间字段映射')
    d.require(mapping['terminal_kinds'] and mapping['carrier_kinds'] and
              not set(mapping['terminal_kinds']) & set(mapping['carrier_kinds']),'缺独立terminal/K实际kind')
    objects=facts['snapshot']['objects'];by_id={o['object_id']:o for o in objects}
    d.require(len(by_id)==len(objects),'来源对象重复')
    terminals=[o for o in objects if o['kind'] in mapping['terminal_kinds']]
    carriers=[o for o in objects if o['kind'] in mapping['carrier_kinds']]
    missing=[]
    if not terminals:missing.append('实际terminal BSP全标签对象')
    if not carriers:missing.append('完整K森林对象')
    if 'gamma_proof' not in raw_by_name or 'controlled_gammas' not in raw_by_name:missing.append('独立γ证明原字节')
    if missing:
        budget.check()
        return {'status':'MISSING_DEPENDENCY','missing':missing,'candidates':[],
                'execution_authorized':False,'product_status':'NOT_RUN'}
    artifacts(raw_by_name,contract['artifacts'],budget)
    d.require(d.sha(raw_by_name['input'])==facts['source']['input_sha256'],'不是本域完整输入')
    d.require(d.strict(raw_by_name['source_objects'])==objects and d.strict(raw_by_name['terminal_objects'])==terminals
              and d.strict(raw_by_name['carrier_objects'])==carriers,'全对象/terminal/K原件删减或换源')
    instrument=d.strict(raw_by_name['instrument_declaration'])
    d.require(instrument==contract['instrument_declaration'] and instrument['input_sha256']==facts['source']['input_sha256']
              and instrument['profile_id']==facts['fixed_cut']['profile_id'],'instrument声明错域')
    clock=d.strict(raw_by_name['clock_mapping'])
    d.exact(clock,{'kind','source_origin','nanosecond_origin','nanoseconds_per_ordinal'},'TestOnly时钟映射')
    d.require(clock==contract['clock_mapping'] and clock['kind']=='TestOnlyAffineOrdinalToNs','缺冻结TestOnly映射')
    origin,ns_origin,step=(decimal(clock[k]) for k in ('source_origin','nanosecond_origin','nanoseconds_per_ordinal'))
    d.require(step>0,'时钟步长非正')
    opening=contract['opening_source']
    d.exact(opening,{'fact_key','fact_hash','allocation_id','raw_sha256','json_pointer','quantity','units'},'opening_source')
    d.require(opening['raw_sha256']==d.sha(raw_by_name['opening_raw']),'开局原字节不同')
    doc=pointer(d.strict(raw_by_name['opening_raw']),opening['json_pointer'])
    d.require(doc['quantity']==opening['quantity'] and doc['units']==opening['units'] and decimal(opening['quantity'])>0,'开局原量/单位不同')
    for k in ('fact_key','fact_hash'):d.require(isinstance(opening[k],str) and len(opening[k])==64,'开局身份未冻')
    gamma_doc=d.strict(raw_by_name['controlled_gammas']);proof=d.strict(raw_by_name['gamma_proof'])
    d.require(proof==contract['independent_gamma_proof'] and proof['input_sha256']==facts['source']['input_sha256']
              and proof['fixed_cut']==facts['fixed_cut'] and proof['profile_id']==facts['fixed_cut']['profile_id'],
              '独立γ证明未绑定输入/profile/完整cut')
    d.require(proof['author_id']!=contract['bundle_author_id'] and proof['proof_id'] and proof['method'],
              'γ证明缺独立作者/具名方法；仍须真实独评')
    d.require(proof['gamma_ids']==[g['gamma_id'] for g in gamma_doc] and len(set(proof['gamma_ids']))==len(gamma_doc),'γ证明未覆盖候选全集')
    profile=facts['fixed_cut']['profile_id'];frontier=int(facts['fixed_cut']['input_frontier'])
    def normalize(obj,kind):
        values={k:pointer(obj,v) for k,v in mapping[kind].items()}
        d.require(values['profile']==profile,'对象profile错域')
        start,end=decimal(values['start']),decimal(values['end'])
        d.require(start<=end<=frontier and values['direction'] in ('Long','Short'),'方向/整个来源区间不符')
        d.require(isinstance(values['anchor'],str) and values['anchor'],'缺锚')
        ids=values['source_ids']
        d.require(type(ids) is list and ids and len(set(ids))==len(ids) and all(i in by_id for i in ids),'来源对象缺失/重复')
        intervals=[]
        for ident in ids:
            src={k:pointer(by_id[ident],p) for k,p in mapping['source'].items()}
            d.require(src['profile']==profile,'source对象profile错域')
            a,z=decimal(src['start']),decimal(src['end'])
            d.require(start<=a<=z<=end,'source对象超出声明整个区间')
            intervals.append((a,z))
        cursor=start
        for a,z in sorted(intervals):
            d.require(a<=cursor,'整个来源区间有缺口')
            cursor=max(cursor,z+1)
        d.require(cursor==end+1,'整个来源区间未覆盖末端')
        return values
    ts={o['object_id']:normalize(o,'terminal') for o in terminals}
    ks={o['object_id']:normalize(o,'carrier') for o in carriers}
    parents={};visited=set()
    for ident,k in ks.items():
        d.require(type(k['children']) is list and len(set(k['children']))==len(k['children']),'K孩子重复')
        for child in k['children']:
            d.require(child in ks and child not in parents,'K子树缺件或多父')
            parents[child]=ident;sub=ks[child]
            d.require(decimal(sub['start'])>=decimal(k['start']) and decimal(sub['end'])<=decimal(k['end'])
                      and decimal(sub['level'])<decimal(k['level']),'K子树区间/级别不符')
    roots=[i for i in ks if i not in parents];stack=roots[:]
    while stack:
        budget.check();ident=stack.pop();d.require(ident not in visited,'K森林环/共享子树')
        visited.add(ident);stack.extend(ks[ident]['children'])
    d.require(visited==set(ks),'K森林不完整/环')
    candidates=[];gamma_terminals=set()
    for gamma in gamma_doc:
        budget.check();tid=gamma['terminal_id'];d.require(tid in ts,'γ没有实际terminal')
        gamma_terminals.add(tid);terminal=ts[tid]
        d.require(gamma['terminal_sha256']==fingerprint(by_id[tid]) and gamma['profile_id']==profile,'γ/terminal来源hash不同')
        for field in ('direction','anchor','start','end','confirmed_at','source_ids','execution_level','confirmation_level'):
            d.require(gamma[field]==terminal[field],'γ/terminal不匹配：'+field)
        labels=terminal['labels']
        d.require(type(labels) is list and labels and len({fingerprint(v) for v in labels})==len(labels)
                  and gamma['terminal_labels']==labels,'terminal同端同侧全标签缺漏')
        confirmed=decimal(terminal['confirmed_at']);execution=decimal(terminal['execution_level']);confirmation=decimal(terminal['confirmation_level'])
        d.require(decimal(terminal['end'])<=confirmed<=frontier and confirmed>=origin
                  and confirmation>=execution,'确认前缀/层级不符')
        ns=ns_origin+(confirmed-origin)*step;d.require(ns<2**63,'TestOnly时钟溢出')
        hosts=[ident for ident,k in ks.items() if decimal(k['level'])==execution
               and k['direction']==terminal['direction'] and k['anchor']==terminal['anchor']
               and decimal(k['start'])<=decimal(terminal['start']) and decimal(k['end'])>=decimal(terminal['end'])
               and set(terminal['source_ids'])<=set(k['source_ids'])]
        d.require(len(hosts)==1,'同级host必须唯一；零或多host均拒绝')
        host=hosts[0]
        d.require(gamma['host_id']==host and gamma['host_sha256']==fingerprint(by_id[host]),'γ host来源不同')
        candidates.append({'gamma_id':gamma['gamma_id'],'terminal_id':tid,'terminal_labels':copy.deepcopy(labels),
            'carrier_id':host,'carrier_sha256':fingerprint(by_id[host]),'source_ids':terminal['source_ids'],
            'profile_id':profile,'execution_level':str(execution),'confirmation_level':str(confirmation),
            'nest_depth':str(confirmation-execution),'sigma':terminal['direction'],
            'anchor':terminal['anchor'],'source_interval':[terminal['start'],terminal['end']],
            'first_confirming_observation_source':terminal['confirmed_at'],'observation_ns':str(ns),
            'instrument':instrument['instrument'],'opening_source':copy.deepcopy(opening)})
    d.require(gamma_doc and gamma_terminals==set(ts),'未覆盖全部terminal候选，禁止自行筛选')
    result={'schema_revision':'issue1394-voice-candidates/1','status':'UNAUTHENTICATED_CANDIDATES',
            'candidates':candidates,'K_roots':roots,'artifacts':contract['artifacts'],
            'missing':['真实独评receipt','根统一producer/consumer产物及精确运行准入','本B CaptureExternal/ApplyAllocation持久原事实e_cut'],
            'execution_authorized':False,'product_status':'NOT_RUN'}
    budget.check();return result


def execute_bind_voice(*args,**kwargs):
    raise d.Rejected('本模块不自动执行BindVoice；未认证候选不能进入正式经济入口')
