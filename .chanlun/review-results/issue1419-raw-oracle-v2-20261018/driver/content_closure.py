"""纯Python s-content/1封闭图验证及七字段projection重建。

接口来源：基线s-content-v3.js encode/verifyRange；q3-consumer.js:2196。
不导入JS/Rust、不访问数据库。处理完整已收集节点集，不声称每页独立证明。
资源为调用者显式给定的逻辑工作上限；不是OS硬帽。
"""
from dataclasses import dataclass
from itertools import chain
import hashlib
import re
from fixed_cut_driver import require,canonical,FAMILIES,strict_load

DOMAIN=b'NewChanlun/SContent/1\0'
KINDS=['null','bool','bool','string','number','array','object','json_text','template_batch']


def u64(n):
    require(type(n) is int and 0<=n<2**63,'非i64长度')
    return n.to_bytes(8,'big')


def frame(tag,body):
    return hashlib.sha256(DOMAIN+bytes([tag])+u64(len(body))+body).hexdigest()


def root_bytes(root):
    require(isinstance(root,str) and re.fullmatch('sc1-[0-9a-f]{64}',root),'节点root格式')
    return bytes.fromhex(root[4:])


@dataclass(frozen=True)
class Edge:
    ref:dict


@dataclass(frozen=True)
class Number:
    token:str


@dataclass(frozen=True)
class JSONText:
    value:object


def ref_check(ref):
    require(set(ref)=={'format','root','kind','extent'} and ref['format']=='s-content/1','ContentRef字段')
    root_bytes(ref['root'])
    require(ref['kind'] in KINDS,'ContentRef种类')
    if ref['kind'] in ('string','number','array','object'):
        require(type(ref['extent']) is str and re.fullmatch('0|[1-9][0-9]*',ref['extent']),'extent非规范')
        u64(int(ref['extent']))
    else:require(ref['extent'] is None,'标量extent非空')


class Encoder:
    def __init__(self,work_budget=None):
        self.budget=work_budget
        self.charge()
        self.nodes={}

    def charge(self,count=1):
        if self.budget is not None:self.budget.charge(count)


    def add(self,tag,body):
        self.charge(1+(len(body)+4095)//4096)
        root='sc1-'+frame(tag,body)
        self.nodes[root]={'root':root,'tag':f'{tag:02x}','body_hex':body.hex()}
        return root_bytes(root)

    def merkle(self,roots):
        self.charge(1+len(roots))
        n=len(roots)
        if not n:return self.add(0x11,b'')
        if n==1:return self.add(0x12,roots[0])
        k=1<<((n-1).bit_length()-1)
        return self.add(0x13,u64(n)+self.merkle(roots[:k])+self.merkle(roots[k:]))

    def encode(self,v):
        self.charge()
        extent=None
        if isinstance(v,Edge):ref_check(v.ref);return v.ref.copy()
        if v is None:tag,body=0,b''
        elif type(v) is bool:tag,body=(2 if v else 1),b''
        elif isinstance(v,(str,Number)):
            tag=4 if isinstance(v,Number) else 3
            token=v.token if tag==4 else v
            self.charge(1+(len(token)+1023)//1024)
            if tag==4:require(re.fullmatch(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?',token),'number token')
            b=token.encode('utf8');extent=str(len(b))
            body=u64(len(b))+self.merkle([self.add(0x10,b[i:i+4096]) for i in range(0,len(b),4096)])
        elif type(v) is int:return self.encode(Number(str(v)))
        elif isinstance(v,JSONText):tag,body=7,root_bytes(self.encode(v.value)['root'])
        elif type(v) is list:
            self.charge(len(v))
            tag=5;extent=str(len(v));body=u64(len(v))+self.merkle([root_bytes(self.encode(x)['root']) for x in v])
        elif type(v) is dict:
            self.charge(1+len(v)*max(1,len(v).bit_length()))
            for key in v:
                self.charge(1+(len(key)+1023)//1024 if type(key) is str else 1)
            require(all(type(k) is str for k in v),'object key')
            tag=6;extent=str(len(v))
            roots=[self.add(0x14,root_bytes(self.encode(k)['root'])+root_bytes(self.encode(v[k])['root'])) for k in sorted(v,key=lambda k:k.encode())]
            body=u64(len(v))+self.merkle(roots)
        else:raise ValueError('不支持类型；禁止浮点丢精度')
        root=self.add(tag,body)
        return {'format':'s-content/1','root':'sc1-'+root.hex(),'kind':KINDS[tag],'extent':extent}


class WorkBudget:
    """闭包、领域转换共用，按访问计量；截止由调用者累计预算提供。"""
    def __init__(self,limit,check=lambda:None):
        require(type(limit) is int and limit>0,'逻辑工作预算无效')
        self.limit=limit;self.used=0;self.check=check

    def charge(self,count=1):
        # 先查绝对截止并预留；失败不消耗未获准的工作，不把帽向上挪。
        self.check()
        require(type(count) is int and count>=0,'逻辑工作计量无效')
        require(count<=self.limit-self.used,
                f'闭包逻辑工作量超限：used={self.used}, requested={count}, limit={self.limit}')
        self.used+=count

    def iter(self,values):
        # 连 next()/StopIteration 都收费，空输入和碎片输入不能零费绕过。
        self.charge()
        iterator=iter(values)
        while True:
            self.charge()
            try:value=next(iterator)
            except StopIteration:return
            yield value

    def scan(self,value):
        """在复制/序列化前有界遍历；字符串按最多4KiB UTF-8块预留。"""
        self.charge()
        stack=[iter((value,))]
        while stack:
            self.charge()
            try:item=next(stack[-1])
            except StopIteration:stack.pop();continue
            if isinstance(item,dict):
                self.charge();stack.append(iter(chain(item.keys(),item.values())))
            elif isinstance(item,(list,tuple)):
                self.charge();stack.append(iter(item))
            elif isinstance(item,(str,bytes)):
                unit=1024 if isinstance(item,str) else 4096
                self.charge(1+(len(item)+unit-1)//unit)
            require(len(stack)<128,'工作扫描深度超限')


def header_from_raw(tag,body):
    """s-content-v3.js headerFromRaw:648-681；包括未被引用的节点。"""
    n=len(body)
    if tag in (0,1,2,8,0x11):require(n==0,'节点头：空体种类有载荷')
    elif tag in (3,4,5,6):
        require(n==40,'节点头：值头长度')
        require(int.from_bytes(body[:8],'big')<2**63,'节点头：U64域')
    elif tag in (7,0x12):require(n==32,'节点头：单子长度')
    elif tag==0x10:require(1<=n<=4096,'节点头：字节块长度')
    elif tag==0x13:
        require(n==72,'节点头：分支长度')
        require(1<int.from_bytes(body[:8],'big')<2**63,'节点头：分支计数')
    elif tag==0x14:require(n==64,'节点头：field长度')
    else:raise ValueError('节点头：未知tag')


class Graph:
    def __init__(self,nodes,*,max_nodes,max_bytes,max_work,work_budget=None):
        self.budget=work_budget if work_budget is not None else WorkBudget(max_work)
        require(self.budget.limit==max_work,'工作预算帽不一致')
        self.budget.charge()
        self.pool={};self.memo={};total=0
        for n in self.budget.iter(nodes):
            self.budget.charge()
            require(isinstance(n,dict) and len(n)==3 and set(n)=={'root','tag','body_hex'},'节点字段')
            require(type(n['tag']) is str and len(n['tag'])==2 and
                    re.fullmatch('[0-9a-f]{2}',n['tag']),'节点hex')
            text=n['body_hex']
            require(type(text) is str and len(text)%2==0,'节点hex')
            size=len(text)//2
            # 在正则扫描、hex分配和hash之前分别预留；重复节点也完整收费。
            self.budget.charge(1+(len(text)+4095)//4096)
            require(re.fullmatch('(?:[0-9a-f]{2})*',text),'节点hex')
            self.budget.charge(1+(size+4095)//4096)
            require(size<=max_bytes,'节点集合超提案限额')
            root_bytes(n['root'])
            duplicate=n['root'] in self.pool
            require(duplicate or (len(self.pool)<max_nodes and size<=max_bytes-total),'节点集合超提案限额')
            t=int(n['tag'],16);b=bytes.fromhex(text)
            self.budget.charge(1+(size+4095)//4096)
            require(n['root']=='sc1-'+frame(t,b),'节点hash不符')
            self.budget.charge()
            header_from_raw(t,b)
            self.budget.charge()
            if duplicate:require(self.pool[n['root']]==(t,b),'节点冲突')
            else:total+=len(b);self.pool[n['root']]=(t,b)
        self.bytes=total

    @property
    def work(self):return self.budget.used

    def node(self,root,depth=0):
        self.budget.charge()
        require(depth<128,'闭包深度超限')
        require(root in self.pool,'缺失闭包节点:'+root)
        return self.pool[root]

    def ref(self,root):
        t,b=self.node(root)
        require(t<=7,'非公开值节点/未实例化template')
        if t<=2:require(not b,'标量体');extent=None
        elif t==7:require(len(b)==32,'json_text体');extent=None
        else:require(len(b)==40,'值头体');extent=str(int.from_bytes(b[:8],'big'));u64(int(extent))
        return {'format':'s-content/1','root':root,'kind':KINDS[t],'extent':extent}

    def checked(self,ref):
        self.budget.charge()
        ref_check(ref);require(self.ref(ref['root'])==ref,'引用种类/extent不符')
        return ref['root']

    @staticmethod
    def r(b):return 'sc1-'+b.hex()

    def leaves(self,root,n,depth=0):
        t,b=self.node(root,depth)
        if n==0:require(t==0x11 and b==b'','空Merkle');return []
        if n==1:require(t==0x12 and len(b)==32,'单叶Merkle');return [self.r(b)]
        require(t==0x13 and len(b)==72 and int.from_bytes(b[:8],'big')==n,'分支计数')
        k=1<<((n-1).bit_length()-1)
        return self.leaves(self.r(b[8:40]),k,depth+1)+self.leaves(self.r(b[40:]),n-k,depth+1)

    def entries(self,ref):
        root=self.checked(ref);t,b=self.node(root);n=int(ref['extent'])
        require(t in (5,6),'非集合值')
        roots=self.leaves(self.r(b[8:]),n)
        if t==5:return [self.ref(r) for r in roots]
        out=[];last=None
        for r in roots:
            tag,body=self.node(r);require(tag==0x14 and len(body)==64,'对象field')
            keyref=self.ref(self.r(body[:32]));require(keyref['kind']=='string','field key非string')
            key=self.decode(keyref)
            require(last is None or last.encode()<key.encode(),'对象键重复/无序')
            out.append({'key':key,'value_ref':self.ref(self.r(body[32:]))});last=key
        return out

    def byte_value(self,ref):
        root=self.checked(ref);t,b=self.node(root);n=int(ref['extent'])
        require(t in (3,4),'非字节值')
        out=[]
        for i,r in enumerate(self.leaves(self.r(b[8:]),(n+4095)//4096)):
            tag,body=self.node(r);require(tag==0x10 and len(body)==min(4096,n-i*4096),'字节块长度')
            out.append(body)
        return b''.join(out)

    def decode(self,ref,depth=0,ordinary=False):
        root=self.checked(ref)
        require(depth<128,'值嵌套过深')
        if ordinary:require(ref['kind']!='json_text','json_text内非普通JSON')
        cache_key=(root,ordinary)
        if cache_key in self.memo:return self.memo[cache_key]
        t,b=self.node(root,depth)
        if t==0:v=None
        elif t in (1,2):v=t==2
        elif t in (3,4):
            token=self.byte_value(ref).decode('utf8')
            if t==4:
                require(re.fullmatch(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?',token),'数值token非法')
                v=Number(token)
            else:v=token
        elif t==5:v=[self.decode(r,depth+1,ordinary) for r in self.entries(ref)]
        elif t==6:v={e['key']:self.decode(e['value_ref'],depth+1,ordinary) for e in self.entries(ref)}
        elif t==7:v=JSONText(self.decode(self.ref(self.r(b)),depth+1,True))
        else:raise ValueError('闭包中有未公开template')
        self.memo[cache_key]=v
        return v

    def resolve(self,ref,path):
        self.checked(ref)
        for step in self.budget.iter(path):
            require(type(step) is dict and len(step)==1,'路径格式')
            if 'field' in step:
                require(ref['kind']=='object','field路径非object')
                found=[x['value_ref'] for x in self.entries(ref) if x['key']==step['field']]
                require(len(found)==1,'field路径不存在');ref=found[0]
            elif 'index' in step:
                require(ref['kind']=='array' and type(step['index']) is str and re.fullmatch('0|[1-9][0-9]*',step['index']),'index路径')
                es=self.entries(ref);i=int(step['index']);require(i<len(es),'index越界');ref=es[i]
            elif step=={'json_text':True}:
                require(ref['kind']=='json_text','json_text路径');ref=self.ref(self.r(self.node(ref['root'])[1]))
            else:raise ValueError('未知路径')
        return ref


def projection(snapshot,work_budget=None):
    if work_budget is not None:work_budget.scan(snapshot)
    require(set(snapshot)=={'fixed_cut','catalog','rows','counts','scope','omissions','order_version','cut_projection_digest'},'projection八个传输字段')
    require(snapshot['omissions']==[] and snapshot['order_version']=='s-record-order/2','projection范围/序')
    counts={f:sum(r['family']==f for r in snapshot['rows']) for f in FAMILIES}
    require(snapshot['counts']=={**{f:str(n) for f,n in counts.items()},'total':str(len(snapshot['rows']))},'六族计数')
    require(all(r['family'] in FAMILIES for r in snapshot['rows']),'未知族')
    seen=set();prior=-1
    for r in snapshot['rows']:
        require(set(r)=={'family','key','record'},'row字段')
        key=canonical([r['family'],r['key']]);require(key not in seen,'重复row');seen.add(key)
        rank=FAMILIES.index(r['family']);require(rank>=prior,'混族页');prior=rank
    fixed=dict(snapshot['fixed_cut']);fixed['catalog_evidence']=Edge(fixed['catalog_evidence'])
    value={k:snapshot[k] for k in ('counts','scope','omissions','order_version')}
    value.update(fixed_cut=fixed,catalog=Edge(snapshot['catalog']),
                 rows=[{**r,'record':Edge(r['record'])} for r in snapshot['rows']])
    enc=Encoder(work_budget);ref=enc.encode(value)
    if work_budget is not None:work_budget.charge()
    qcut='qcut3-'+frame(0x22,root_bytes(ref['root']))
    require(qcut==snapshot['cut_projection_digest'],'七字段qcut3不符')
    if work_budget is not None:work_budget.charge(len(enc.nodes))
    return ref,list(enc.nodes.values())


def close_projection(snapshot,pages,*,max_nodes,max_bytes,max_work,work_budget=None):
    work_budget=work_budget if work_budget is not None else WorkBudget(max_work)
    require(work_budget.limit==max_work,'工作预算帽不一致')
    work_budget.charge()
    require(isinstance(pages,(list,tuple)),'内容页必须是可重遍历的固定集合')
    ref,built_nodes=projection(snapshot,work_budget)
    anchor={'kind':'projection','fixed_cut':snapshot['fixed_cut'],
            'cut_projection_digest':snapshot['cut_projection_digest'],'projection_ref':ref}
    def page_nodes():
        for page in work_budget.iter(pages):
            yield from work_budget.iter(page['nodes'])
    nodes=chain(built_nodes,page_nodes())
    graph=Graph(nodes,max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work,work_budget=work_budget)
    # Edge 的编码仅承诺 root；原始传输引用的其余元数据必须另核，不能重推后自证。
    graph.checked(snapshot['catalog'])
    graph.checked(snapshot['fixed_cut']['catalog_evidence'])
    for row in work_budget.iter(snapshot['rows']):graph.checked(row['record'])
    for p in work_budget.iter(pages):
        work_budget.scan(p)
        require(p['anchor']==anchor,'内容混切面或projection_ref')
        target=graph.resolve(ref,p['path']);require(target==p['target_ref'],'内容路径根不符')
        unit='items' if target['kind'] in ('array','object') else 'bytes' if target['kind'] in ('string','number') else 'value'
        require(p['unit']==unit,'内容单位不符')
        start,end,total=map(int,(p['offset'],p['end'],p['total']))
        require(0<=start<=end<=total and total==(1 if unit=='value' else int(target['extent'])),'内容范围')
        items=graph.entries(target)[start:end] if unit=='items' else graph.byte_value(target)[start:end].hex() if unit=='bytes' else ([target] if start==0 and end==1 else [])
        require(p['items']==items,'内容页内容/位置不符')
    decoded=graph.decode(ref)
    return {'closure_verified':True,'projection_ref':ref,'graph':graph,'decoded':decoded,
            'nodes':len(graph.pool),'node_body_bytes':graph.bytes,'work':graph.work,
            'qualification':'完整节点集合闭包+七字段固定切面；不代替领域oracle/真实运行'}
