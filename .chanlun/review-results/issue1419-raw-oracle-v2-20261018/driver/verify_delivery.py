"""交付的定向静态/原件检查；不重跑前轮23项或产品。"""
import ast
import json
from pathlib import Path
from raw_oracle import canonical,digest,need
from prepare_candidate import ROOT


def main():
    report={'scope':'Python AST/原件字节/schema/顺序/旧帽不变','product_run':'NOT_RUN'}
    for path in (ROOT/'driver').glob('*.py'):ast.parse(path.read_text(),filename=str(path))
    contract=json.loads((ROOT/'EXACT-RUN-PROPOSAL.json').read_bytes())
    need(contract['authorization'] is False and contract['execution_authorized'] is False,'授权不得变真')
    for key in ('source_commit','source_tree','source_manifest_sha256','binary_sha256','frozen_run_rule'):
        need(contract[key]=='PENDING_SOURCE','不得冒认冻结源')
    for item in contract['input_artifacts']+contract['independent_expected']:
        b=(ROOT/item['path']).read_bytes();need(len(b)==item['bytes'] and digest(b)==item['sha256'],'原件身份漂移:'+item['path'])
    old=json.loads((Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/raw-run01-artifacts/raw-evidence/DRIVER-CONTRACT.json')).read_bytes())
    need(contract['existing_caps_reference']==old['existing_caps_reference'],'原帽被修改')
    need(contract['proposed_aggregate_caps_not_authorized']==old['proposed_aggregate_caps_not_authorized'],'累计提案被扩帽')
    for file in ('S-RESOURCES.reference.json','Q-RESOURCES.reference.json'):
        need((ROOT/file).read_bytes()==(Path('/Volumes/AgentStorage/issue1419-sandcastle-manager-20261018/raw-run01-artifacts/raw-evidence')/file).read_bytes(),'原资源字节漂移')
    fields=set('event_id revision seq received_at raw_text open high low close timestamp volume'.split())
    base=[json.loads(line) for line in (ROOT/'raw/messages.base.jsonl').read_bytes().splitlines()]
    suffix=[json.loads(line) for line in (ROOT/'raw/messages.recovery-epoch2.jsonl').read_bytes().splitlines()]
    clock=json.loads((ROOT/'raw/clock-plan.json').read_bytes())['events']
    original=json.loads((ROOT/'RAW-INPUT.json').read_bytes())['bars']
    effective={};history=[];revisions={};frontier=-1
    all_messages=base[:111]+suffix
    max_frame=0
    for j,m in enumerate(all_messages):
        p=m['payload'];need(digest(canonical(p))==m['payload_hash'],'消息hash')
        need(p['writer_epoch']==('1' if j<111 else '2'),'恢复epoch顺序')
        need(p['clock_event_id'] in clock,'时钟无事件')
        max_frame=max(max_frame,len(canonical(m)))
        for e in p['raw_input']['events']:
            need(set(e)==fields and all(type(v) is str for v in e.values()),'OHLC exact字段')
            i=int(e['seq']);r=int(e['revision'])
            need(e['event_id']==f'controlled-{i:04d}' and r==revisions.get(i,0)+1,'身份或修订跳号')
            need(e['volume']=='1','受控原量须1')
            need(int(e['low'])<=min(int(e['open']),int(e['close']))<=max(int(e['open']),int(e['close']))<=int(e['high']),'OHLC包络')
            raw=json.loads(e['raw_text'])['bar']
            need(all(int(e[k])==raw[k] for k in ('open','high','low','close','volume')),'原文投影不一致')
            revisions[i]=r;effective[i]=raw;frontier+=1;history.append((i,r))
        if j>=139:
            name=m['message_id'];wanted=json.loads((ROOT/f'raw/{name}.json').read_bytes())['bars']
            need([effective[i] for i in sorted(effective)]==wanted,'修订步骤有效raw不符:'+name)
    need(frontier==269 and [effective[i] for i in sorted(effective)]==original,'恢复原价/最终输入前沿')
    need(max_frame<1048576,'输入突破原S frame帽')
    need('recover_ns' in clock['recover-after-111'],'恢复时钟键错误')
    need((ROOT/'raw/advance-0111.epoch2.json').read_bytes()==canonical(suffix[0])+b'\n','one-shot输入不一致')
    report.update(python_files=len(list((ROOT/'driver').glob('*.py'))),input_files=len(contract['input_artifacts']),
                  expected_files=len(contract['independent_expected']),messages_per_arm=len(all_messages),
                  accepted_events_per_arm=len(history),final_input_frontier=frontier,
                  maximum_input_envelope_bytes=max_frame,original_caps_byte_identical=True,
                  all_checks='PASS',authorization=False)
    print(json.dumps(report,ensure_ascii=False,indent=2))


if __name__=='__main__':main()
