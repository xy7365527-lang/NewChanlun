"""纯Python s-content/1封闭图验证及七字段projection重建。

接口来源：基线s-content-v3.js encode/verifyRange；q3-consumer.js:2196。
不导入JS/Rust、不访问数据库。处理完整已收集节点集，不声称每页独立证明。
资源为调用者显式给定的逻辑工作上限；不是OS硬帽。
"""
from dataclasses import dataclass
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
    def __init__(self):self.nodes={}

    def add(self,tag,body):
        root='sc1-'+frame(tag,body)
        self.nodes[root]={'root':root,'tag':f'{tag:02x}','body_hex':body.hex()}
        return root_bytes(root)

    def merkle(self,roots):
        n=len(roots)
        if not n:return self.add(0x11,b'')
        if n==1:return self.add(0x12,roots[0])
        k=1<<((n-1).bit_length()-1)
        return self.add(0x13,u64(n)+self.merkle(roots[:k])+self.merkle(roots[k:]))

    def encode(self,v):
        extent=None
        if isinstance(v,Edge):ref_check(v.ref);return v.ref.copy()
        if v is None:tag,body=0,b''
        elif type(v) is bool:tag,body=(2 if v else 1),b''
        elif isinstance(v,(str,Number)):
            tag=4 if isinstance(v,Number) else 3
            token=v.token if tag==4 else v
            if tag==4:require(re.fullmatch(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?',token),'number token')
            b=token.encode('utf8');extent=str(len(b))
            body=u64(len(b))+self.merkle([self.add(0x10,b[i:i+4096]) for i in range(0,len(b),4096)])
        elif type(v) is int:return self.encode(Number(str(v)))
        elif isinstance(v,JSONText):tag,body=7,root_bytes(self.encode(v.value)['root'])
        elif type(v) is list:
            tag=5;extent=str(len(v));body=u64(len(v))+self.merkle([root_bytes(self.encode(x)['root']) for x in v])
        elif type(v) is dict:
            require(all(type(k) is str for k in v),'object key')
            tag=6;extent=str(len(v))
            roots=[self.add(0x14,root_bytes(self.encode(k)['root'])+root_bytes(self.encode(v[k])['root'])) for k in sorted(v,key=lambda k:k.encode())]
            body=u64(len(v))+self.merkle(roots)
        else:raise ValueError('不支持类型；禁止浮点丢精度')
        root=self.add(tag,body)
        return {'format':'s-content/1','root':'sc1-'+root.hex(),'kind':KINDS[tag],'extent':extent}


class Graph:
    def __init__(self,nodes,*,max_nodes,max_bytes,max_work):
        self.pool={};self.work=0;self.max_work=max_work;self.memo={};total=0
        for n in nodes:
            require(set(n)=={'root','tag','body_hex'},'节点字段')
            require(re.fullmatch('[0-9a-f]{2}',n['tag']) and re.fullmatch('(?:[0-9a-f]{2})*',n['body_hex']),'节点hex')
            t=int(n['tag'],16);b=bytes.fromhex(n['body_hex'])
            require(n['root']=='sc1-'+frame(t,b),'节点hash不符')
            if n['root'] in self.pool:require(self.pool[n['root']]==(t,b),'节点冲突')
            else:total+=len(b);self.pool[n['root']]=(t,b)
            require(len(self.pool)<=max_nodes and total<=max_bytes,'节点集合超提案限额')
        self.bytes=total

    def node(self,root,depth=0):
        self.work+=1
        require(self.work<=self.max_work and depth<128,'闭包逻辑工作量/深度超限')
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
        for step in path:
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


def projection(snapshot):
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
    enc=Encoder();ref=enc.encode(value)
    qcut='qcut3-'+frame(0x22,root_bytes(ref['root']))
    require(qcut==snapshot['cut_projection_digest'],'七字段qcut3不符')
    return ref,list(enc.nodes.values())


def close_projection(snapshot,pages,*,max_nodes,max_bytes,max_work):
    ref,built_nodes=projection(snapshot)
    anchor={'kind':'projection','fixed_cut':snapshot['fixed_cut'],
            'cut_projection_digest':snapshot['cut_projection_digest'],'projection_ref':ref}
    nodes=built_nodes+[n for page in pages for n in page['nodes']]
    graph=Graph(nodes,max_nodes=max_nodes,max_bytes=max_bytes,max_work=max_work)
    for p in pages:
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
