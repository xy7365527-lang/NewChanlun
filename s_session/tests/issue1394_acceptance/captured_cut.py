"""#1394 已捕获原字节的固定cut导出。无网络/启动/认证入口。

只支持已核协议：Q snapshot/2 + financial-basis/3 + 全basis SessionClosure。
模板未接出生表则拒绝；不从六族输出制造G/terminal/K。合同pin须由根提供。
"""
from dataclasses import dataclass
import math
import re
import driver as d
import closure as c

FAMILIES = ('objects', 'withdrawn_objects', 'witnesses', 'relations', 'observations', 'raw_history')
FIXED = set('session_id session_generation structure_cut cut_generation catalog_revision index_frontier profile_id profile_hash input_frontier seq_range catalog_run_status catalog_evidence scope rule_revision history_mode as_of_generation'.split())
ANCHOR = set('session_id session_generation writer_epoch structure_cut index_frontier commit_ns basis_ref'.split())
SNAPSHOT = set('session_id generation structure_cut catalog_revision scope index_frontier profile_id profile_hash history_mode as_of_generation objects withdrawn_objects witnesses relations observations raw_history input_frontier seq_range catalog_run_status catalog_evidence'.split())
TOKEN_FIXED = 'session_id session_generation cut_generation scope history_mode structure_cut catalog_revision index_frontier profile_hash rule_revision'.split()
MIB = 1024 * 1024
TRANSPORT = set('max_frame_bytes read_timeout_ms write_timeout_ms response_timeout_ms queue_capacity max_connections'.split())
Q_LIMITS = set('max_frame_bytes max_pending_connections socket_read_timeout_ms socket_write_timeout_ms max_capture_bytes capture_deadline_ms verify_deadline_ms max_query_workers max_query_queue max_reply_bytes max_cached_images max_cache_bytes poll_interval_ms max_client_leases client_lease_ms max_client_pending_batches max_page_size max_watch_batches max_atomic_delta_node_bytes max_atomic_delta_nodes max_atomic_delta_records'.split())


def resources(value, budget, closure_budget):
    """只核原冻结对象，不产生部署默认值或OS资源承诺。"""
    d.exact(value,{'session','reader','q','s_writer','economic_domains','deadlines','legacy_reader'},'逐对象资源')
    d.validate_resources(value['session'],value['reader'])
    d.exact(value['q'],{'schema_revision','limits'},'Q资源外层')
    d.require(value['q']['schema_revision']=='s-query-resources/3','Q资源版本')
    d.exact(value['q']['limits'],Q_LIMITS,'Q原18+3Delta资源')
    d.exact(value['s_writer'],{'schema_revision','s_resources'},'S资源外层')
    d.require(value['s_writer']['schema_revision']=='s-structure-resources/1','S资源版本')
    d.exact(value['s_writer']['s_resources'],TRANSPORT|{'audit_cache_source_bytes'},'S七资源')
    domains=value['economic_domains']
    d.require('E' in domains and any(k.startswith('B:') for k in domains),'E/B逐对象资源缺失')
    for caps in domains.values():d.exact(caps,TRANSPORT,'E/B输送资源')
    for caps in [value['q']['limits'],value['s_writer']['s_resources'],*domains.values()]:
        for number in caps.values():d.positive(number)
    q=value['q']['limits']
    d.require(d.positive(q['max_cached_images'])>=d.positive(q['max_query_workers']),'Q缓存不足worker')
    d.exact(value['legacy_reader'],{'max_reply_bytes','timeout_ms'},'旧读取资源')
    d.require(d.positive(value['legacy_reader']['max_reply_bytes'])<=16*MIB
              and d.positive(value['legacy_reader']['timeout_ms'])<=10000,'旧16MiB/10秒不可放宽')
    deadlines=value['deadlines']
    d.exact(deadlines,{'work','cleanup','seal','stages'},'固定绝对截止')
    d.require(deadlines['stages'] and all(type(n) in (int,float) and math.isfinite(n) and n>0
        for n in [deadlines['work'],deadlines['cleanup'],deadlines['seal'],*deadlines['stages'].values()]),'缺完整阶段/整次/回收封口截止')
    d.require(max(deadlines['stages'].values())<=deadlines['work']<deadlines['cleanup']<deadlines['seal'], '截止/预留顺序')
    d.require(closure_budget.deadline<=budget.deadline<=min(deadlines['stages'].values()),'读取预算延长最早冻结截止')


def decimal(value):
    d.require(isinstance(value, str) and re.fullmatch(r'0|[1-9][0-9]*', value), '非规范非负整数')
    number = int(value)
    d.require(number < 2**63, '整数超界')
    return number


def fingerprint(value):
    return d.sha(d.canonical(value))


@dataclass(frozen=True)
class Capture:
    route: str
    request: bytes
    reply: bytes

    def identity(self):
        return {'route': self.route, 'request_sha256': d.sha(self.request), 'reply_sha256': d.sha(self.reply)}


@dataclass(frozen=True)
class Edge:
    ref: dict


def content_ref(value, budget, sink=None):
    """独立确定性内容编码，仅用来复核已捕获JSON/引用的根；不是生产语义生成器。"""
    def node(tag, body):
        budget.check()
        root = c.node_root(tag, body)
        if sink is not None:sink[root] = c.Node(root, tag, body)
        return bytes.fromhex(root[4:])
    def sequence(items):
        if not items:return node(0x11, b'')
        if len(items)==1:return node(0x12, items[0])
        split=1 << ((len(items)-1).bit_length()-1)
        return node(0x13,len(items).to_bytes(8,'big')+sequence(items[:split])+sequence(items[split:]))
    def encode(v):
        budget.check()
        if isinstance(v,Edge):
            d.exact(v.ref,{'format','root','kind','extent'},'ContentRef')
            d.require(v.ref['format']=='s-content/1','内容格式不同')
            return c.root_bytes(v.ref['root']),v.ref['kind'],v.ref['extent']
        if v is None:tag,kind,body,extent=0,'null',b'',None
        elif type(v) is bool:tag,kind,body,extent=(2 if v else 1),'bool',b'',None
        elif isinstance(v,str) or type(v) is int:
            raw=v.encode() if isinstance(v,str) else str(v).encode()
            tag,kind=(3,'string') if isinstance(v,str) else (4,'number')
            extent=str(len(raw));body=len(raw).to_bytes(8,'big')+sequence([node(0x10,raw[i:i+4096]) for i in range(0,len(raw),4096)])
        elif type(v) is list:
            tag,kind,extent=5,'array',str(len(v));body=len(v).to_bytes(8,'big')+sequence([encode(x)[0] for x in v])
        elif type(v) is dict:
            tag,kind,extent=6,'object',str(len(v))
            body=len(v).to_bytes(8,'big')+sequence([node(0x14,encode(k)[0]+encode(v[k])[0]) for k in sorted(v)])
        else:raise d.Rejected('未接非整数数字/扩展内容编码，不猜填')
        return node(tag,body),kind,extent
    root,kind,extent=encode(value)
    budget.check()
    return {'format':'s-content/1','root':'sc1-'+root.hex(),'kind':kind,'extent':extent}


def decode_verified(target, nodes, budget):
    """调用前必须已经通过closure.verify；沿验证后的DAG显式栈解码。"""
    cache={};stack=[(target['root'],False)]
    def ref(raw):return 'sc1-'+raw.hex()
    while stack:
        root,leaving=stack.pop();budget.check()
        if root in cache:continue
        n=nodes[root];tag,body=n.tag,n.body
        if tag in (3,4,5,6):children=[ref(body[8:])]
        elif tag in (7,0x12):children=[ref(body)]
        elif tag==0x13:children=[ref(body[8:40]),ref(body[40:])]
        elif tag==0x14:children=[ref(body[:32]),ref(body[32:])]
        else:children=[]
        if not leaving:
            stack.append((root,True));stack.extend((x,False) for x in reversed(children));continue
        values=[cache[x] for x in children]
        if tag==0:result=None
        elif tag in (1,2):result=tag==2
        elif tag==0x10:result=body
        elif tag==0x11:result=[]
        elif tag==0x12:result=[values[0]]
        elif tag==0x13:result=values[0]+values[1]
        elif tag==0x14:result=(values[0],values[1])
        elif tag in (3,4):
            raw=b''.join(values[0]);result=raw.decode() if tag==3 else d.strict(raw)
        elif tag==5:result=values[0]
        elif tag==6:result=dict(values[0])
        elif tag==7:raise d.Rejected('json_text语义解码尚未接线')
        else:raise d.Rejected('模板不能充当已发布对象')
        cache[root]=result
    budget.check()
    return cache[target['root']]


def row_identity(family, record):
    if family=='objects':
        key=[record['object_id']]
        order=(1,record['kind'],record['fact_key'],*key,decimal(record['object_revision'])) if 'fact_key' in record else (0,*(decimal(record[x]) for x in ('window_start','window_mid','window_end')),*key,decimal(record['object_revision']))
    elif family=='withdrawn_objects':key=[record['object_id']];order=(decimal(record['withdrawn_generation']),*key,decimal(record['object_revision']))
    elif family=='witnesses':key=[record['witness_id']];order=(record['object_id'],decimal(record['slot']),*key)
    elif family=='relations':key=[record[k] for k in ('subject','relation_type','object')];order=tuple(key)
    elif family=='observations':key=[record['observation_id']];order=(record['kind'],-1 if record['window_start'] is None else decimal(record['window_start']),*key)
    else:key=[record['identity_key'],record['revision']];order=(decimal(record['seq']),key[0],decimal(key[1]))
    return key,order


def envelope(capture, fixed):
    request,reply=d.strict(capture.request),d.strict(capture.reply)
    for value in (request,reply):
        d.require(value['schema_revision']=='s-session/3' and
                  all(value[k]==fixed[k] for k in ('session_id','session_generation')),
                  'Q外层会话/版本不同')
        d.require(value['payload_hash']==fingerprint(value['payload']),'Q payload摘要不同')
    causal={k:request[k] for k in ('source_namespace','source_epoch','message_id','payload_hash')}
    d.require(reply['causal_refs']==[causal],'Q回复错绑请求')
    d.require(reply['message_id']=='reply-'+fingerprint([request['source_namespace'],request['source_epoch'],request['message_id'],reply['payload_hash']]),'Q回复身份不同')
    return request['payload'],reply['payload']


def export_captured(contract_raw, expected_sha, captures, source, budget, closure_budget, *, input_raw):
    """source为独立持久节点读取器。只返回未认证事实；root pin不是执行准入。"""
    budget.charge((contract_raw,));d.require(d.sha(contract_raw)==expected_sha,'导出合同原字节pin不符')
    contract=d.strict(contract_raw)
    d.require(contract['schema_revision']=='issue1394-captured-cut/1' and contract['execution_authorized'] is False,'导出不接受执行授权')
    resources(contract['resources'],budget,closure_budget)
    identity=contract['source']
    for key,size in (('commit',40),('tree',40),('binary_sha256',64),('input_sha256',64),('artifact_manifest_sha256',64)):
        d.require(isinstance(identity.get(key),str) and re.fullmatch('[0-9a-f]{'+str(size)+'}',identity[key]),'缺根冻结来源身份：'+key)
    budget.charge((input_raw,))
    d.require(d.sha(input_raw)==identity['input_sha256'],'完整输入原字节不同')
    fixed,anchor=contract['fixed_cut'],contract['anchor']
    d.exact(fixed,FIXED,'fixed_cut');d.exact(anchor,ANCHOR,'financial anchor')
    for key in ('session_id','session_generation','structure_cut','index_frontier'):
        d.require(anchor[key]==fixed[key],'Q/financial不是同cut')
    d.require(fixed['profile_hash']==identity['profile_hash'] and fixed['rule_revision']==identity['rule_revision']
              and fixed['catalog_revision']==identity['catalog_revision'],'profile/rule/catalog错绑')
    d.require(contract['full_scope']==fixed['scope'] and fixed['history_mode']=='AsKnown'
              and fixed['as_of_generation']==fixed['cut_generation'],'只接受冻结AsKnown完整切面')
    d.require(closure_budget.deadline<=budget.deadline,'闭包不能延长最早截止')
    d.require([x.identity() for x in captures]==contract['captures'],'捕获请求/回复有序原件全集不同')
    total=0;pages=[];closure_pages=[];basis_pair=None
    for capture_index,capture in enumerate(captures):
        budget.charge((capture.request,capture.reply));total+=len(capture.request)+len(capture.reply)
        d.require(max(len(capture.request),len(capture.reply))<=64*MIB and total<=128*MIB,'捕获原件超过64/128MiB帽')
        job=contract['resources']['reader']['job_limits']
        d.require(total<=d.positive(job['max_total_wire_bytes'])
                  and capture_index+1<=d.positive(job['max_total_requests']), '捕获累计wire/请求超过Job帽')
        caps=contract['resources']['q']['limits'] if capture.route=='snapshot' else contract['resources']['reader']
        request_cap=caps['max_frame_bytes'] if capture.route=='snapshot' else caps['max_request_bytes']
        d.require(len(capture.request)<=d.positive(request_cap) and len(capture.reply)<=d.positive(caps['max_reply_bytes']), '捕获帧超过冻结对象帽')
        if capture.route=='snapshot':pages.append(envelope(capture,fixed))
        elif capture.route=='financial-basis':
            d.require(basis_pair is None,'重复basis');basis_pair=(d.strict(capture.request),d.strict(capture.reply))
        elif capture.route=='financial-closure':closure_pages.append((d.strict(capture.request),d.strict(capture.reply)))
        else:raise d.Rejected('未接线捕获route；不猜wire格式')
    d.require(pages and basis_pair and closure_pages,'缺snapshot/basis/全闭包原件')
    rows=[];metadata=None;next_token=None;issued=None;done=False
    for index,(request,page) in enumerate(pages):
        budget.check();d.require(not done,'完成后有多余页')
        d.require(page['ok'] is True and page['fixed_cut']==fixed and page['omissions']==[],'混cut或截断投影')
        current={k:page[k] for k in ('fixed_cut','catalog','counts','scope','omissions','order_version','cut_projection_digest')}
        if metadata is None:metadata=current
        d.require(current==metadata and page['scope']==fixed['scope'] and page['order_version']=='s-record-order/2','页元数据漂移')
        if index==0:
            d.require(request=={'op':'snapshot','session_id':fixed['session_id'],'session_generation':fixed['session_generation'],
                'scope':fixed['scope'],'history_mode':fixed['history_mode'],'as_of_generation':fixed['cut_generation'],
                'page_size':contract['page_size'],'order_version':'s-record-order/2'},'首请求未冻结完整cut')
            issued=page['validated_capture_digest']
        else:d.require(request=={'op':'snapshot','snapshot_token':next_token},'漏页/换token/重排')
        d.require(re.fullmatch('[0-9a-f]{64}',page['validated_capture_digest']) is not None,'capture摘要格式')
        d.require(decimal(page['offset'])==len(rows),'分页offset缺漏')
        total_rows=decimal(page['counts']['total']);size=decimal(contract['page_size'])
        d.require(size>0 and len(page['rows'])==min(size,total_rows-len(rows)),'分页数量不符')
        rows.extend(page['rows']);done=len(rows)==total_rows
        d.require(page['done'] is done,'完成标记提前/滞后')
        next_token=page['next_token']
        expected=None
        if not done:
            expected={k:fixed[k] for k in TOKEN_FIXED}
            expected.update(token_schema='s-snapshot-token/2',order_version='s-record-order/2',page_size=contract['page_size'],
                            offset=str(len(rows)),cut_projection_digest=page['cut_projection_digest'],issued_capture_digest=issued)
            expected['token_checksum']=fingerprint(expected)
        d.require(next_token==expected,'分页token字段/checksum/来源漂移')
    d.require(done,'缺末页')
    nodes=[];after=None;finished=False;node_total=None
    for request,reply in closure_pages:
        budget.check();d.require(not finished,'闭包完成后多页')
        d.exact(request,{'schema_revision','anchor','path','after','max_reply_bytes'},'closure请求')
        d.require(request['schema_revision']=='s-financial-closure/1' and request['anchor']==anchor
                  and request['path']==[] and request['after']==after,'闭包不是固定全basis/分页断裂')
        d.require(len(d.canonical(reply))+1<=decimal(request['max_reply_bytes']),'闭包回复超过请求帽')
        d.require(reply['ok'] is True and reply['anchor']==anchor and reply['path']==[] and reply['after']==after
                  and reply['target_ref']==anchor['basis_ref'],'闭包回复错绑')
        if node_total is None:node_total=decimal(reply['total'])
        d.require(decimal(reply['total'])==node_total and reply['nodes'],'闭包数量漂移/空页')
        for raw in reply['nodes']:
            d.exact(raw,{'root','tag','body_hex'},'proof node')
            d.require(re.fullmatch('[0-9a-f]{2}',raw['tag']) is not None and re.fullmatch('(?:[0-9a-f]{2})*',raw['body_hex']) is not None,'节点非规范十六进制')
            n=c.Node(raw['root'],int(raw['tag'],16),bytes.fromhex(raw['body_hex']))
            d.require(not nodes or n.root>nodes[-1].root,'闭包重复/漏序')
            nodes.append(n)
        after=nodes[-1].root;finished=len(nodes)==node_total
        d.require(reply['next']==after and reply['done'] is finished,'闭包尾游标/完成标记错误')
    d.require(finished,'闭包漏页')
    closure_result=c.verify(anchor['basis_ref'],nodes,source,closure_budget)
    d.require(len(nodes)<=d.positive(contract['resources']['session']['max_closure_nodes']), '闭包超过会话冻结节点帽')
    d.require(len(nodes)<=d.positive(job['max_total_nodes']) and
              sum(len(n.body)+len(c.DOMAIN)+9 for n in nodes)<=d.positive(job['max_total_node_bytes']),
              '闭包累计节点/字节超过Job帽')
    by_root={n.root:n for n in nodes}
    basis=decode_verified(anchor['basis_ref'],by_root,budget)
    d.require(basis['kind']=='FinancialBasis' and all(basis[k]==anchor[k] for k in ('session_id','session_generation','writer_epoch','commit_ns')),'basis身份不同')
    d.require(basis['pending_revision'] is False,'basis仍有待决修订')
    snapshot=basis['snapshot']
    d.exact(snapshot,SNAPSHOT,'完整二十字段snapshot')
    for key in ('scope','seq_range','catalog_run_status'):
        d.require(snapshot[key]==fixed[key],'快照元数据不同：'+key)
    d.require(content_ref(snapshot['catalog_evidence'],budget)==fixed['catalog_evidence'],'catalog证据不是该快照原件')
    for k,sk in (('session_id','session_id'),('cut_generation','generation'),('structure_cut','structure_cut'),('index_frontier','index_frontier'),('profile_hash','profile_hash'),('profile_id','profile_id'),('catalog_revision','catalog_revision'),('input_frontier','input_frontier')):
        d.require(snapshot[sk]==fixed[k],'basis快照不是Q固定cut：'+k)
    expected_rows=[];object_ids=set()
    for family in FAMILIES:
        records=sorted(snapshot[family],key=lambda r:row_identity(family,r)[1]);seen=set()
        for record in records:
            key,_=row_identity(family,record);encoded_key=d.canonical(key)
            d.require(encoded_key not in seen,'重复公开键');seen.add(encoded_key)
            if family in ('objects','withdrawn_objects'):
                d.require(key[0] not in object_ids,'active/withdrawn重复对象');object_ids.add(key[0])
            expected_rows.append({'family':family,'key':key,'record':content_ref(record,budget)})
        d.require(metadata['counts'][family]==str(len(records)),'记录族数量不符')
    d.require(rows==expected_rows and set(metadata['counts'])==set(FAMILIES)|{'total'},'对象/来源/关系/见证/历史有序全集不同')
    d.require(len(rows)<=d.positive(job['max_total_records']), '投影累计记录超过Job帽')
    qfixed=dict(fixed);qfixed['catalog_evidence']=Edge(fixed['catalog_evidence'])
    projection={'fixed_cut':qfixed,'catalog':Edge(metadata['catalog']),
                'rows':[dict(row,record=Edge(row['record'])) for row in rows],
                **{k:metadata[k] for k in ('counts','scope','omissions','order_version')}}
    projection_ref=content_ref(projection,budget)
    qdigest='qcut3-'+c.root_bytes(c.node_root(0x22,c.root_bytes(projection_ref['root']))).hex()
    d.require(qdigest==metadata['cut_projection_digest']==contract['cut_projection_digest'],'Q完整投影根不符')
    request,reply=basis_pair
    wanted=[r['object_id'] for r in snapshot['objects']]
    d.require(request=={'schema_revision':'s-financial-query/1','session_id':anchor['session_id'],
        'session_generation':anchor['session_generation'],'writer_epoch':anchor['writer_epoch'],'object_ids':wanted},'财务点查未覆盖全对象')
    d.require(reply['schema_revision']=='s-financial-basis/3' and reply['kind']=='FinancialBasisRef'
              and reply['content_format']=='s-content/1' and reply['pending_revision'] is False
              and reply['basis_ref']==anchor['basis_ref'],'财务basis未固定')
    d.require(all(reply[k]==anchor[k] for k in ('session_id','session_generation','writer_epoch','commit_ns')),'财务会话身份不同')
    d.require(reply['snapshot_ref']==content_ref(snapshot,budget),'财务快照ref不符')
    selected=[]
    for index,record in enumerate(snapshot['objects']):
        ref=content_ref(record,budget)
        selected.append({'object_id':record['object_id'],'snapshot_index':str(index),'object_ref':ref,
            'object_hash':c.root_bytes(c.node_root(0x23,c.root_bytes(ref['root']))).hex()})
    d.require(reply['selected_objects']==selected,'点查对象/顺序/来源hash不符')
    for raw in reply['proof_nodes']:
        d.require(raw['root'] in by_root and c.Node(raw['root'],int(raw['tag'],16),bytes.fromhex(raw['body_hex']))==by_root[raw['root']], '财务proof不在原件闭包')
    result={'schema_revision':'issue1394-captured-facts/1','source':identity,'fixed_cut':fixed,'anchor':anchor,
            'resources':contract['resources'],
            'snapshot':snapshot,'captures':[x.identity() for x in captures],'closure':closure_result,
            'status':'UNAUTHENTICATED_FACTS','execution_authorized':False,'product_status':'NOT_RUN',
            'missing':['正式terminal全标签/完整K森林/独立γ语义认证','真实独评与根精确运行授权']}
    budget.check();return result


def execute(*args,**kwargs):
    raise d.Rejected('execution_authorized=false；本模块无网络/启动/BindVoice能力')
