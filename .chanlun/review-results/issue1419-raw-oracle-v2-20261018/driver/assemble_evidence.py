"""只封本轮已读的确切来源、合同、AC域矩阵；不读凭据/市场大缓存/其他工位。"""
from pathlib import Path
import json
import re
import shutil
from raw_oracle import digest,canonical
from prepare_candidate import ROOT,write,SESSION,NS

REPO=ROOT.parents[2]
PRIOR=Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/raw-run01-artifacts')
MANAGER=PRIOR.parent


def metadata(path):
    b=path.read_bytes();return {'bytes':len(b),'sha256':digest(b)}


def main():
    sources=[]
    def copy(source,destination,role):
        p=ROOT/destination;p.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,p)
        sources.append({'original':str(source),'copy':destination,'role':role,**metadata(p)})
    for f in ['bi.md','xianduan.md','zhongshu.md']:
        copy(REPO/'.chanlun/definitions'/f,'sources/definitions/'+f,'已裁正本；字节来自固定5cb')
    for lesson in ['020','062','067','071','077','078','081']:
        p=next((REPO/'docs/chanlun/text/blog').glob(lesson+'-*.md'))
        copy(p,'sources/blog/'+p.name,'作者归属按引用行逐项核；不引用编者注')
    paths=['s_session/browser/s-content-v3.js','s_session/browser/q3-consumer.js',
           'rust/src/bin/s_session_v3/projection.rs','rust/src/bin/s_session_v3/raw_input.rs',
           'rust/src/bin/s_session_v2/tb02a.rs','rust/src/bin/s_session_v2/tb02c.rs',
           'rust/src/theta_v0/parser/feature_seq.rs','s_session/README.md']
    for p in paths:copy(REPO/p,'sources/5cb/'+p,'只读接口/历史实现证据；不是定义oracle')
    for p in ['rust/src/theta_v0/config.rs','rust/src/bin/s_session_v3/generation.rs',
              'rust/src/bin/s_session_v2/tb02e.rs','rust/src/bin/s_session_v2/tb02f.rs']:
        copy(PRIOR/'raw-evidence/sources/634f62cd'/p,'sources/634f/'+p,'复用前轮封存donor原件；不执行')
    copy(MANAGER/'issue1419-live.json','sources/issue1419-live.json','本轮票完整原件')
    copy(MANAGER/'blocked-by.json','sources/blocked-by.json','本轮原生依赖原件')
    copy(PRIOR/'raw-evidence/RAW-INVENTORY.json','sources/MARKET600-INVENTORY.reference.json','仅原件清单复用；600市场raw不替换')
    copy(PRIOR/'raw-evidence/oracle/REFERENCE-CONDITIONAL-LEDGER.json','sources/MARKET600-CONDITIONAL.reference.json','6完成+1进行/1中心0对；非独立raw oracle')
    copy(PRIOR/'raw-evidence/validation/unit-tests.stdout','sources/PRIOR-23-CHECKS.reference.stdout','前轮原结果，只复用不重跑')
    copy(REPO/'s_session/catalog/signed-catalog.json','sources/signed-catalog.json','原目录字节；新源实际22轴绑定仍待核')
    citations={
      'RAW_NO_INCLUSION':{'definition':'.chanlun/definitions/bi.md:113-120,323-334','blog':[('062',28)],'claim':'包含闭区间判定；本例每对两端同向严格变化，无一次合并，组与根集恒单根。'},
      'FRACTAL':{'definition':'.chanlun/definitions/bi.md:288-319','blog':[('062',22),('062',32)],'claim':'high/low同时严格极值才分型；每个内部三K窗口完整分类。'},
      'BI':{'definition':'.chanlun/definitions/bi.md:101-120,297-334','blog':[('081',110),('077',54),('077',58)],'claim':'merged不共用；raw极值区至少3；交替、方向、末笔与确认分开。'},
      'SEGMENT_TERMINATION':{'definition':'.chanlun/definitions/xianduan.md:159-169,263-283,304-317,333-351','blog':[('067',18),('067',20),('067',28),('071',30),('071',32),('071',40),('071',42),('077',62),('077',64),('078',20),('078',22),('078',24)],'claim':'奇数≥3、首三笔非退化重叠；两类终结；首段最低起点；strict缺口封闭；无转折窗成员。'},
      'SECOND_SEQUENCE':{'definition':'.chanlun/definitions/xianduan.md:133-169,199-208','blog':[('067',38),('067',46),('078',52),('078',54)],'claim':'从b之后取原段同向笔；先包含/分型后严格越界；本例无包含并在越界之前或同笔先确认。'},
      'CENTER_SEED':{'definition':'.chanlun/definitions/zhongshu.md:71-83,115-134','blog':[('020',48),('020',54)],'claim':'三个完成连续同级交替段；ZD=max低，ZG=min高，严格小于。'},
      'CENTER_ROLES':{'definition':'.chanlun/definitions/zhongshu.md:194-216,222-247;§6.1','blog':[('020',52),('020',54),('020',56),('020',58)],'claim':'核心冻结；四角色只数Zn；全单元延伸及last+2连接端点条款；完整16格相邻关系。'}
    }
    for key,item in citations.items():
        quotes=[]
        for lesson,line in item.pop('blog'):
            p=next((REPO/'docs/chanlun/text/blog').glob(lesson+'-*.md'));lines=p.read_text().splitlines();text=lines[line-1]
            marker=next(i+1 for i,v in enumerate(lines) if '↑正文' in v)
            # 仅裁去已逐行目视确认的行内编注，避免将非作者句子带入依据。
            quote=re.split(r'（娇：|（娇注：|（注：|\(娇注：',text)[0]
            assert not re.match(r'^(?:（注：|娇注：|\(娇注：)',quote)
            quotes.append({'file':'docs/chanlun/text/blog/'+p.name,'line':line,'body_boundary_line':marker,
                           'author':'缠中说禅','attribution':'续排《忽闻台风可休市》正文，标题100行' if lesson=='081' and line==110 else '正文界前，无行首注',
                           'inline_editor_note_excluded':quote!=text,'quote':quote,'whole_file_sha256':metadata(p)['sha256']})
        item['verified_quotes']=quotes
    write('DEFINITION-REFERENCES.json',citations)
    spec_path='.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/inputs/SPEC-COVERAGE-INPUT.json'
    spec=json.loads((REPO/spec_path).read_bytes());issue=json.loads((MANAGER/'issue1419-live.json').read_bytes())
    ids=set(re.findall(r'A-(?:CC|LC|OB|ST)-\d+-\d+',issue['body']))
    found={}
    def walk(value,path='',parent=None):
        if isinstance(value,dict):
            if value.get('id') in ids and 'required_result' in value:found[value['id']]=(value,path,parent)
            for k,v in value.items():
                if k=='historical_generic_acceptance_not_effective':continue
                walk(v,path+'/'+k,value if 'id' in value else parent)
        elif isinstance(value,list):
            for i,v in enumerate(value):walk(v,path+'/'+str(i),parent)
    walk(spec)
    assert set(found)==ids,(ids-set(found))
    rows=[]
    for id in sorted(ids):
        a,p,parent=found[id]
        coverage='具名raw例的离线语义账；正式执行/公共观察/恢复未验'
        if '-OB-' in id:coverage='固定切面/闭包toy只证明离线工具；本项实际公共观察、历史、前端均NOT_RUN'
        elif id.endswith('-03') and '-CC-' in id:coverage='同raw定义重算与具名恢复价格expected已备；正式新进程/持久恢复NOT_RUN'
        elif id.endswith('-04') and '-CC-' in id:coverage='实际生产及浏览器可见链NOT_RUN；只有待比较expected'
        elif id=='A-ST-011-01':coverage='正常strict分支两例；相切/倒置反例并未由本raw覆盖，不销整项'
        elif id=='A-ST-011-02':coverage='仅两中心+连接及关系输入；上级构造#1429未覆盖'
        rows.append({'id':id,'source_pointer':p,'required_result':a['required_result'],
                     'allowed_input_domain':parent.get('quantified_domain',parent.get('applicable_domain',parent.get('domain','按原条款对象/观察合同；TestOnly只覆盖本票声明结构子域'))),
                     'candidate_coverage':coverage,'still_due_market_domain':'本候选无市场认证、无原量重放、无全图完整历史；真实市场域义务全部保留',
                     'product_status':'NOT_RUN'})
    ac=[x for x in spec['acceptance_plan'] if x['id'] in ('AC-02','AC-03')]
    write('AC-DOMAIN-MATRIX.json',{'source':spec_path,**metadata(REPO/spec_path),
         'leaf_ac':[{'id':'票面AC1','coverage':'两组真实raw定义推导的11111候选；正式核未运行'},
                    {'id':'票面AC2','coverage':'核心/角色/归属及非Zn修订expected完备；正式变化未运行'},
                    {'id':'票面AC3','coverage':'正式会话+公共查询+浏览器全NOT_RUN'},
                    {'id':'票面AC4','coverage':'独立定义oracle已备；产品双进程/持久恢复全NOT_RUN'}],
         'AC_02_03_original':ac,'all_named_acceptances':rows,
         'allowed_testonly_basis':'issue1419-live.json票面“TestOnly结构原始输入只证明声明受测域”；不把该许可扩大为市场或完整业务验收',
         'entire_ticket_complete':False})
    matrix=['# #1419 AC × 输入域 × 本候选覆盖','',
            '本表逐项保留票内全部细项A；原文、量词和JSON pointer见同名JSON。允许TestOnly只源于本票声明的结构受测域。整票、市场源认证、原量和全图历史均未验。','',
            '| AC | 允许域 | 本候选覆盖 | 仍欠真实市场域/实际执行 |','|---|---|---|---|']
    matrix.extend('| '+r['id']+' | 原条款对象域；TestOnly仅结构子义务 | '+r['candidate_coverage']+' | 市场认证/原量/完整历史未覆盖；产品NOT_RUN |' for r in rows)
    matrix+=['','AC-02：只覆盖此无包含/无同型相邻/非退化有限域与具名负控，其他正常/端点/退化/证据不足及跨级同输入同输出锁仍欠。',
             'AC-03：只提供最低构造级10完成段→2中枢→1连接/1相邻对的独立expected；高级递归、上级对象、下游真实消费及全部ST量词未验。',
             '票面四项：AC1/2的定义备料已完成；AC3全未运行；AC4只完成独立oracle与具名重放/恢复预期。没有把任一整项勾为通过。']
    (ROOT/'AC-DOMAIN-MATRIX.md').write_text('\n'.join(matrix)+'\n')
    write('sources/SOURCE-MANIFEST.json',sources)
    prior=json.loads((PRIOR/'raw-evidence/DRIVER-CONTRACT.json').read_bytes())
    contract={**prior,'schema':'issue1419-raw-driver-contract/2','identity':{'session_id':SESSION,'session_generation':'1','source_namespace':NS,'source_epoch':'1'},
              'execution_authorized':False,'authorization':False,'source_commit':'PENDING_SOURCE','source_tree':'PENDING_SOURCE',
              'source_manifest_sha256':'PENDING_SOURCE','binary_sha256':'PENDING_SOURCE','frozen_run_rule':'PENDING_SOURCE',
              'profile':{'path':'sources/ohlc_integer_tb02a_v1.json',**metadata(ROOT/'sources/ohlc_integer_tb02a_v1.json')},
              'signed_catalog':{'path':'sources/signed-catalog.json',**metadata(ROOT/'sources/signed-catalog.json')},
              'input_artifacts':[{'path':str(p.relative_to(ROOT)),**metadata(p)} for p in [ROOT/'RAW-INPUT.json',*sorted((ROOT/'raw').glob('*'))]],
              'independent_expected':[{'path':str(p.relative_to(ROOT)),**metadata(p)} for p in [ROOT/'INDEPENDENT-WITNESS-LEDGER.json',*sorted((ROOT/'expected').glob('*.json'))]],
              'cuts':{'input_counts':[39,63,87,111,123,135,139],'nonempty_pair_first_count':123,
                      'final_all_evidence_pens_confirmed_count':139,
                      'steps':['base','revise-non-zn-tail','withdraw-second-center','restore-original'],
                      'input_frontiers':[138,139,204,269],
                      'meaning':'generation/cut/profile/rule/catalog与回执同次绑定；批次修订续接用input_frontier，不用seq_range.to'},
              'blockers':['PENDING_SOURCE：合流源/树/二进制/规则/独审未冻结',
                          'PENDING_RESOURCE_ADMISSION：原帽不变；新input与监督实装需根准入',
                          'PENDING_FINAL_ADAPTER：634f字段离线适配toy已证，最终source同合同/22轴/原始来源闭包仍待核',
                          'PENDING_PRODUCT_EXECUTION：S3/Q/双跑/持久恢复/浏览器NOT_RUN',
                          'MARKET_DOMAIN_OUTSTANDING：市场认证/原量重放/全图完整历史未覆盖'],
              'quantifiers_not_proved':['正式S3/Q的非空成功','真实对象修订/撤回及AsKnown保留','真实新进程同输入重放','真实持久恢复',
                                       '浏览器核对','市场源认证/原量/全图历史','整票/父域/Voice/高级对象完成'],
              'toy_engineering_completed':['定义见证/固定expected','七字段qcut3/真实projection_ref/完整节点闭包','具名expected比较与634f wire适配',
                                           '固定Python子进程监督：成功/截止/TERM→KILL/日志限额/失联/取消/reap'],
              'volume_contract':'现有profile volume_unit=1；本受控raw量原件就是1。市场原量不因此获准转换；未改profile或载体。'}
    # argv保留原接口/feature；只替换本轮身份、确切输入/时钟和恢复位置。
    for name,argv in contract['commands'].items():
        replacements={'tb1419-btc600-priceonly-draft':SESSION,'sources/5cb7253d/s_session/catalog/signed-catalog.json':'sources/signed-catalog.json',
                      'sources/5cb7253d/s_session/profiles/ohlc_integer_tb02a_v1.json':'sources/ohlc_integer_tb02a_v1.json',
                      'raw/clock-plan.draft.json':'raw/clock-plan.json','recover-after-300':'recover-after-111',
                      'raw/advance-0300.draft.json':'raw/advance-0111.epoch2.json'}
        contract['commands'][name]=[replacements.get(v,v) for v in argv]
    adv=contract['commands']['advance'];adv[adv.index('--max-input-bytes')+1]=str((ROOT/'raw/advance-0111.epoch2.json').stat().st_size)
    contract['execution_order']=[
        '两臂fresh-a/fresh-b串行，分别从空的本次TestOnly数据库init；相同raw、时钟与控制顺序',
        '各臂先serve投递messages.base.jsonl第1..111行，逐条等待同次commit回执',
        '固cut111六族+内容闭包；TERM并reap S；recover-after-111到epoch2；重读同cut原件并比较',
        'advance使用advance-0111.epoch2.json；再serve送messages.recovery-epoch2.jsonl第2..28行',
        '在123/135/139及具名前缀按实际回执固定切面；末3行依次修订/撤回/恢复原价格，每步固cut、watch、AsKnown历史',
        '比较两臂完整内容/业务值/身份/时间/顺序/修订账；运行capture/进程身份差异保原件单列，不抹业务差异',
        '浏览器同切面核中心/角色/连接/尾部/修订证据；未通过不能套用旧浏览器延期豁免']
    contract['replay']['normal_recovery_after_input_count']=111
    contract['replay']['normal_recovery']='TERM并reap后同库recover epoch2；使用messages.recovery-epoch2.jsonl确定后缀；定义值与fresh-a/fresh-b相同，epoch/capture差异原件保留单列'
    contract['supervision']['implementation']='driver/toy_supervisor.py；限定固定toy，产品启动能力仍不开放'
    contract['hard_limits']['enforcement_claim']='逻辑预算及直接toy子进程监督已测；未资格化OS内存/磁盘/同步IO硬截止/EINTR/产品后代树'
    write('EXACT-RUN-PROPOSAL.json',contract)
    write('RAW-INVENTORY.json',{'controlled':contract['input_artifacts'],'input_order':'raw/ORDER.json',
         'independent_expected':contract['independent_expected'],
         'market_control':{'bundle':str(PRIOR),'inventory':'sources/MARKET600-INVENTORY.reference.json',
                           'reference_ledger':'sources/MARKET600-CONDITIONAL.reference.json','unchanged':True,
                           'full_cache_rehashed_this_round':False,'whole_history_satisfied':False},
         'new_market_window_search':False,'volume':'controlled unit quantity = 1；无市场量替换',
         'authorization':False})
    write('validation/source-prerequisites.json',{'baseline':'5cb7253d46d973669e7d10cd3331e12c9ef313b1',
         'blockers':[{'number':b['number'],'state':b['state']} for b in json.loads((MANAGER/'blocked-by.json').read_bytes())],
         'graph_tools':'当前工具表无search_graph等MCP接口；源码发现按不足结果回退rg',
         'xerj':'curl http://127.0.0.1:19200/health；exit7，connection refused；未重装重索引',
         'prior_tests':'sources/PRIOR-23-CHECKS.reference.stdout复用，未重跑原23项',
         'product_run':'NOT_RUN'})
    print(json.dumps({'sources':len(sources),'named_A_rows':len(rows),'authorization':False,
                      'input_files':len(contract['input_artifacts']),'product_run':'NOT_RUN'},ensure_ascii=False))


if __name__=='__main__':main()
