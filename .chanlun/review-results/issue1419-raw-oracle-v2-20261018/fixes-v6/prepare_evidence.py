"""#1419 v6静态来源/合同工作草稿；只读Git对象，无安装/索引/产品调用。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
REPO=ROOT.parents[2]
BASE='1cb46982baa0f6ca06cd0884d602bcaaa8395f2e'
MANAGER=Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018')


def sha(raw):return hashlib.sha256(raw).hexdigest()
def write(name,value): (HERE/name).write_text(json.dumps(value,ensure_ascii=False,indent=2)+'\n')
def git(*args):return subprocess.check_output(['git',*args],cwd=REPO)


def main():
    # 新证据可复跑，旧证据仅逐字节复制到本轮引用；不改任何历史产物。
    reads=[]
    for name in ('RAW-LIFECYCLE-CONTRACT.md','RAW-LIFECYCLE-ENTRYPOINTS-46A.json','RAW-V5-INDEPENDENT-REVIEW.json','issue1419-live.json'):
        raw=(MANAGER/name).read_bytes();(HERE/(name+'.reference')).write_bytes(raw)
        reads.append(dict(source=str(MANAGER/name),copy=name+'.reference',bytes=len(raw),sha256=sha(raw)))
    for name in ('MANIFEST.json','SHA256SUMS'):
        raw=git('show',BASE+':'+str(ROOT.relative_to(REPO))+'/'+name)
        (HERE/('V5-'+name+'.reference')).write_bytes(raw)
    for name in ('fixes-v4/REFERENCE-SEARCH.reference.md','fixes-v4/XERJ-ATTEMPTS.reference.json',
                 'fixes-v4/REFERENCE-HITS.reference.json','fixes-v5/STATIC-WIRE-SOURCES.json'):
        raw=(ROOT/name).read_bytes();reads.append(dict(source=name,bytes=len(raw),sha256=sha(raw)))
    write('READ-SOURCES.json',dict(graph_tools_available=False,new_xerj_query=False,index_rebuilt=False,
                                 qualification='复用既有响应，按授权回退到固定Git原件；未声称在线检索成功',reads=reads))
    sources=[]
    specs=[('s_session/browser/q3-consumer.js',[(42,59),(1951,1978),(2590,2662),(2728,2752),(2823,2848)]),
           ('rust/src/bin/s_session_v3/q_watch.rs',[(491,547),(618,667)]),
           ('rust/src/bin/s_session_v3/projection.rs',[(265,331)])]
    for path,ranges in specs:
        raw=git('show',BASE+':'+path)
        prior=git('show','46a11d1cbdeecab3444efbfd324be29a73ecb293:'+path)
        assert raw==prior
        lines=raw.splitlines(keepends=True);excerpts=[]
        for first,last in ranges:
            piece=b''.join(lines[first-1:last])
            excerpts.append(dict(first_line=first,last_line=last,sha256=sha(piece),text=piece.decode()))
        sources.append(dict(commit=BASE,path=path,bytes=len(raw),sha256=sha(raw),
                            blob=git('rev-parse',BASE+':'+path).decode().strip(),same_as_46a=True,excerpts=excerpts))
    write('STATIC-WIRE-SOURCES.json',dict(qualification='固定正式原件静态合同；不授予新产品source资格',sources=sources))
    # 三个受影响入口模块及固定入口图的全符号列表，为人工责任矩阵提供可重复枚举。
    modules=[]
    for path in ('driver/fixed_cut_driver.py','driver/driver_v2.py','driver/content_closure.py'):
        raw=(ROOT/path).read_bytes();tree=ast.parse(raw);entries=[]
        for node in tree.body:
            if isinstance(node,ast.FunctionDef):entries.append(dict(name=node.name,line=node.lineno))
            elif isinstance(node,ast.ClassDef):
                entries.extend(dict(name=node.name+'.'+n.name,line=n.lineno) for n in node.body if isinstance(n,ast.FunctionDef))
        modules.append(dict(path=path,sha256=sha(raw),entries=entries))
    write('ENTRYPOINTS.json',dict(method='完整AST枚举三个模块顶层函数、类方法；与固定46a入口图对照',modules=modules))
    proposal=json.loads((ROOT/'fixes-v5/EXACT-RUN-PROPOSAL.json').read_text())
    proposal.update(schema='issue1419-raw-driver-contract/6',baseline=BASE,
                    previous_candidate_baseline=proposal['baseline'],review_status='PENDING_INDEPENDENT_REVIEW_OF_V6_DIFF')
    proposal['blockers']=[x.replace('v5','v6').replace('V5','V6') for x in proposal['blockers']]
    proposal['artifact_path_base']='父目录 issue1419-raw-oracle-v2-20261018；继承路径不相对 fixes-v6'
    proposal['scope_status']='OFFLINE_COLLECTOR_SESSION_VALIDATED_PRODUCT_LIFECYCLE_UNVERIFIED'
    proposal['driver_revision'].update(report='fixes-v6/REPORT.md',matrix='fixes-v6/ENTRYPOINT-MATRIX.md',schema='fixes-v6/DRIVER-CONTRACT.schema.json',
        watch_requires='必须先真实closed_snapshot返回receipt，再bind_watch(receipt,cursor)；同Collector收据及完整cut绑定，拒绝dict/外来/旧失配收据；成功时原子提交共享cut/projection及watch状态',
        work_budget='从构造到请求/信封/完整闭包/原始类型/上下文复制/收据/提交，始终同一WorkBudget；失败不退款，不加帽、不重置',
        public_payload='保留records/compare_domain原始类型门；closed_snapshot发行收据前也真实调用records；0x07路径读取累计已收到节点，不要求重复全图')
    proposal['watch_contract']=dict(
        trusted_anchor='本Collector真实closed_snapshot→snapshot/content→call→close_projection→records全链成功后发行opaque receipt；bind_watch只认本实例最新收据，普通dict和外来收据拒绝',
        stable_fields=['session_id','session_generation','catalog_revision','profile_id','profile_hash','scope','rule_revision'],
        per_generation_fields=['structure_cut','cut_generation','index_frontier','input_frontier','seq_range','catalog_run_status','catalog_evidence'],
        read_view_fields=['history_mode','as_of_generation'],
        per_generation_rule='所有固定发布字段按代对齐；AsKnown(g)与RecomputedWithRevision(null)仅为读取视图，分别保存projection根；已知代际input_frontier单调，历史/新代可读取而不推进watch游标',
        envelope='observed_head须RecomputedWithRevision/null且完整同代身份一致；投递head相等，retain/first为正且first≤head+1，无Gap游标不得越过first-1；head≥末批，has_more严格等于head>末批。较高head也在成功后登记；全部值先核后提交',
        failure='closed_snapshot或整次watch任何失败不提交上下文/收据/切面/游标/去重账；先前成功状态保留；请求编号、工作与回复费用不回滚',
        caller_protocol_preconditions='accepted_commits的具名输入→实际接纳映射仍由离线轨迹提供；bind_named_submissions核原请求摘要/会话/事件数/重复代际，不证明S真实接纳。watch验证与完整delta字段及本Collector上下文的一致性，返回closure_verified=false；未声称delta完整内容或领域生命周期已验',
        lifecycle='UNVERIFIED',formal_wire_binding='PENDING_SOURCE')
    write('EXACT-RUN-PROPOSAL.json',proposal)
    schema=json.loads((ROOT/'fixes-v5/DRIVER-CONTRACT.schema.json').read_text())
    schema['title']='#1419 v6候选与本Collector验证收据合同'
    schema['properties']['schema']['const']=proposal['schema']
    schema['properties']['watch_contract']['const']=proposal['watch_contract']
    cut=schema['$defs']['accepted_binding']['properties']['accepted_cut']
    cut['properties']['as_of_generation']['type']=['string','null']
    cut['allOf']=[
        {'if':{'properties':{'history_mode':{'const':'AsKnown'}}},'then':{'properties':{'as_of_generation':{'type':'string'}}},
         'else':{'properties':{'history_mode':{'const':'RecomputedWithRevision'},'as_of_generation':{'type':'null'}}}}]
    schema['$defs']['snapshot_receipt']={'description':'进程内对象身份，不可JSON序列化或从schema构造；只由本Collector完整闭包+公共原始类型验证发行。普通对象不是有效收据。','not':{}}
    schema['$defs']['watch_context']['description']='运行时Collector保存共享完整cut/projection及watch状态；本schema不能发行收据。session_state仅提供收费的观察副本；模式值按AsKnown(g)/Recomputed(null)核对。'
    write('DRIVER-CONTRACT.schema.json',schema)
    # 复用F1/F5的精确源码面：仅移动收集链；未变的函数不重跑几何/全前缀。
    reused=[]
    for path,names in [('driver/content_closure.py',None),('driver/driver_v2.py',('records','validate_public_payload','compare_domain','plain'))]:
        old=git('show',BASE+':'+str(ROOT.relative_to(REPO))+'/'+path).decode();new=(ROOT/path).read_text()
        if names is None:
            assert old==new;reused.append(dict(path=path,sha256=sha(new.encode()),unchanged='完整文件'))
        else:
            for name in names:
                def segment(text):return ast.get_source_segment(text,next(n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name==name))
                assert segment(old)==segment(new)
                reused.append(dict(path=path,symbol=name,sha256=sha(segment(new).encode()),unchanged=True))
    write('REUSED-EVIDENCE.json',dict(baseline=BASE,prior_review='RAW-V5-INDEPENDENT-REVIEW.json.reference',source=reused,
        scope='复用F5头/Graph/引用元数据锁、既有raw几何与定义证据；F1原公共门未改，新增收据接线另经真实链测试。原帽完整保留。'))


if __name__=='__main__':main()
