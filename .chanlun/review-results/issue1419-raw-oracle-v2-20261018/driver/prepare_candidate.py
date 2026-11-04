"""生成声明TestOnly原件及定义见证。构造参数预先给定，绝不按产品结果选窗。"""
from pathlib import Path
from copy import deepcopy
from datetime import datetime, timezone, timedelta
import json
from raw_oracle import canonical,digest,derive,public_semantics,need

ROOT=Path(__file__).resolve().parents[1]
SESSION='tb1419-controlled-raw-v2'
NS='testonly.raw1419.definition.v2'


def write(path,value):
    p=ROOT/path
    p.parent.mkdir(parents=True,exist_ok=True)
    p.write_text(json.dumps(value,ensure_ascii=False,indent=2,allow_nan=False)+'\n')


def construct():
    # 仅fixture生成法：这些是raw价格折线控制点，不是输入到产品的SegUnit。
    macro=[100,200,140,240,180,340,300,400,340,440,260,500]
    points=[macro[0]]
    for a,b in zip(macro,macro[1:]):
        sign=1 if b>a else -1
        points.extend([a+20*sign,a+10*sign,b])
    points.append(480)
    mids=[8*p+(4 if j%2==0 else -4) for j,p in enumerate(points)]
    centers=[mids[0]+16,mids[0]]
    for a,b in zip(mids,mids[1:]):
        need((b-a)%4==0,'插值必须整数')
        centers.extend(a+(b-a)*j//4 for j in range(1,5))
    centers.append(mids[-1]+16)
    bars=[{'ordinal':i,'open':m,'high':m+4,'low':m-4,'close':m,'volume':1} for i,m in enumerate(centers)]
    return bars,{'macro_control_prices':macro,'turn_control_prices':points,'scale':8,
                 'half_candle_range':4,'vertex_first_ordinal':1,'vertex_spacing':4,
                 'meaning':'从价折线插值为OHLC的TestOnly构造配方；不是市场源，也不向产品注入段。'}


def event(b,revision,received_index):
    i=b['ordinal']
    return {'event_id':f'controlled-{i:04d}','seq':str(i),'revision':str(revision),
            'timestamp':str(946684800+60*i),'received_at':(datetime(2000,1,1,tzinfo=timezone.utc)+timedelta(seconds=10*received_index)).isoformat().replace('+00:00','Z'),
            'raw_text':canonical({'domain':'TestOnly','bar':b}).decode(),
            **{k:str(b[k]) for k in ('open','high','low','close','volume')}}


def envelope(events,name,epoch='1'):
    payload={'op':'ingest','target_session_id':SESSION,'target_session_generation':'1','writer_epoch':epoch,
             'clock_event_id':name,'raw_input':{'schema_revision':'s-ohlc/1','session_id':SESSION,
             'source_namespace':NS,'source_epoch':'1','instrument':'TESTONLY.DEFINITION.INTEGER',
             'profile':'ohlc_integer_tb02a_v1','events':events}}
    return {'schema_revision':'s-session/3','session_id':SESSION,'session_generation':'1',
            'source_namespace':NS,'source_epoch':'1','message_id':name,'producer_id':'raw1419-oracle-v2',
            'producer_epoch':'1','payload_hash':digest(canonical(payload)),'causal_refs':[],'payload':payload}


def main():
    bars,recipe=construct()
    write('RAW-INPUT.json',{'domain':'TestOnly','market_source_authenticated':False,
                         'volume_semantics':'受控单元量1，不是原市场volume','bars':bars})
    write('raw/CONSTRUCTION-RECIPE.json',recipe)
    base=derive(bars)
    revised=deepcopy(bars)
    for k in ('open','high','low','close'):revised[121][k]+=32
    rledger=derive(revised)
    withdrawn=deepcopy(revised)
    for i in range(74,len(bars)):
        m=bars[73]['close']+16*(i-73)
        withdrawn[i].update(open=m,high=m+4,low=m-4,close=m)
    wledger=derive(withdrawn)
    # 独立手算锚；不以演算器自身输出改期望。
    need(len(base['segments'])==10,'预先指定10完成段未证明')
    need([(c['ZD'],c['ZG']) for c in base['centers']]==[(1120,1600),(2720,3200)],'手算strict核心不符')
    need([c['roles'] for c in base['centers']]==[
        {'DD':800,'D':1440,'G':1600,'GG':2720},
        {'DD':2400,'D':2720,'G':3200,'GG':3520}], '手算Zn角色不符')
    need(base['pairs'][0]['leaf']=='UP_EXPANSION','完整相邻对不符')
    need(base['centers']==rledger['centers'],'非Zn末端小修不应改变中枢角色')
    need(len(wledger['segments'])==4 and len(wledger['centers'])==1,'撤回步骤非预期')
    variants={'base':bars,'revise-non-zn-tail':revised,'withdraw-second-center':withdrawn,'restore-original':bars}
    ledgers={'base':base,'revise-non-zn-tail':rledger,'withdraw-second-center':wledger,'restore-original':base}
    for name,value in variants.items():
        if name!='base':write(f'raw/{name}.json',{'domain':'TestOnly','bars':value})
        write(f'expected/{name}.json',public_semantics(ledgers[name]))
    prefix_counts=[39,63,87,111,123,135,139]
    prefixes={str(n):derive(bars[:n]) for n in prefix_counts}
    write('INDEPENDENT-WITNESS-LEDGER.json',{
        'schema':'issue1419-independent-witness/2','status':'INDEPENDENT_RAW_WITNESSES_FROZEN',
        'raw_sha256':digest((ROOT/'RAW-INPUT.json').read_bytes()),
        'domain':'TestOnly：无raw包含、无连续同型分型、无特征包含、无退化seed；并列极值未触发',
        'definitions':'DEFINITION-REFERENCES.json','base':base,
        'revision_ledgers':{'revise-non-zn-tail':rledger,'withdraw-second-center':wledger},
        'prefixes':{k:{'input_count':int(k),**public_semantics(v)} for k,v in prefixes.items()},
        'scope':'全部raw/分型/笔/完成段与进行中尾部。这里只证定义工作例；正式owner运行NOT_RUN。'})
    revisions=[{'name':'revise-non-zn-tail','events':[(revised[121],2)],
                'meaning':'同一raw身份121修订2，价格整体+32；u9低点2080→2112；非Zn故c6六变量不变'},
               {'name':'withdraw-second-center','events':[(withdrawn[i],3 if i==121 else 2) for i in range(74,len(bars))],
                'meaning':'单次原子raw_input修订74..138为递增尾部；不是raw删除。已失去依据的第二中心及其派生事实须显式撤回'},
               {'name':'restore-original','events':[(bars[i],4 if i==121 else 3) for i in range(74,len(bars))],
                'meaning':'更高raw revision恢复原始价格字节；领域值回base，新版本/获知时点不得冒用旧对象修订'}]
    messages=[envelope([event(b,1,i)],f'raw-{i:04d}') for i,b in enumerate(bars)]
    clock={}
    for i,msg in enumerate(messages):
        t=946684800000000000+10_000_000_000*i
        clock[msg['message_id']]={'accept_ns':str(t),'attempts':[{'begin_ns':str(t+100_000_000),'commit_ns':str(t+200_000_000)}]}
    (ROOT/'raw/messages.base.jsonl').write_bytes(b''.join(canonical(m)+b'\n' for m in messages))
    order=[{'name':m['message_id'],'file':'raw/messages.base.jsonl','line':i+1,'input_frontier':i} for i,m in enumerate(messages)]
    frontier=len(bars)-1
    for j,r in enumerate(revisions):
        m=envelope([event(b,revision,len(bars)+j) for b,revision in r['events']],r['name'])
        write(f"raw/{r['name']}.message.json",m)
        frontier+=len(r['events'])
        order.append({'name':r['name'],'file':f"raw/{r['name']}.message.json",'input_frontier':frontier,'meaning':r['meaning']})
        t=946684800000000000+10_000_000_000*(len(bars)+j)
        clock[r['name']]={'accept_ns':str(t),'attempts':[{'begin_ns':str(t+100_000_000),'commit_ns':str(t+200_000_000)}]}
    clock['recover-after-111']={'recover_ns':str(946684800000000000+10_000_000_000*110+500_000_000)}
    write('raw/clock-plan.json',{'schema_revision':'s-clock-plan/1','clock_plan_id':SESSION,
                              'origin_utc':'2000-01-01T00:00:00Z','unit':'ns','events':clock})
    # 独立恢复臂从111输入后recover，余下每条只改已声明的writer_epoch。
    recovery=[]
    for msg in messages[111:]:
        m=deepcopy(msg);m['payload']['writer_epoch']='2';m['payload_hash']=digest(canonical(m['payload']));recovery.append(m)
    for r in revisions:
        m=json.loads((ROOT/f"raw/{r['name']}.message.json").read_bytes())
        m['payload']['writer_epoch']='2';m['payload_hash']=digest(canonical(m['payload']));recovery.append(m)
    (ROOT/'raw/messages.recovery-epoch2.jsonl').write_bytes(b''.join(canonical(m)+b'\n' for m in recovery))
    (ROOT/'raw/advance-0111.epoch2.json').write_bytes(canonical(recovery[0])+b'\n')
    write('raw/ORDER.json',{'base_order':order,'recovery_after_base_input_count':111,
                          'recovery_suffix':'raw/messages.recovery-epoch2.jsonl',
                          'recovery_count':len(recovery),'generation_rule':'每条成功advance一代；input_frontier按接纳事件数，修订批次不等于generation'})
    write('expected/TRANSITIONS.json',{'steps':[
        {'name':'base','centers':['c0','c6'],'pair_count':1},
        {'name':'revise-non-zn-tail','changed_unit':'u9','low_before':2080,'low_after':2112,
         'unchanged_center_roles':['c0','c6'],'changed_pending':'u10'},
        {'name':'withdraw-second-center','retained_centers':['c0'],'withdrawn_centers':['c6'],
         'withdrawn_pair':['c0','c6'],'withdrawn_completed_units':['u4','u5','u6','u7','u8','u9'],
         'pending_unit':'u4','c0_members':['u0','u1','u2','u3'],
         'c0_roles':{'DD':800,'D':1120,'G':1600,'GG':1920}},
        {'name':'restore-original','domain_values_equal':'base','history':'旧修订/撤回必须保留AsKnown；业务重现不等于历史字节回退'}],
        'product_object_ids':'PENDING_SOURCE；以seed成员/组锚等语义键映射真实对象，不捏造obj3/refs',
        'product_run':'NOT_RUN'})
    print(json.dumps({'domain':'Python纯定义模型','bars':len(bars),'fractals':len(base['low_level']['fractals']),
                      'strokes':len(base['low_level']['strokes']),'completed_segments':len(base['segments']),
                      'case_two':[s['id'] for s in base['segments'] if s['result']=='CASE_TWO'],
                      'centers':base['centers'],'withdrawn_variant_centers':len(wledger['centers']),
                      'product_run':'NOT_RUN'},ensure_ascii=False,indent=2))


if __name__=='__main__':main()
