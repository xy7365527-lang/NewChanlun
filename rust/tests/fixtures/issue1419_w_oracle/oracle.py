#!/usr/bin/env python3
"""#1419 离线差分 oracle；expected 仅接收冻结 base 和原输入，候选只进入 compare。"""
import argparse, collections, copy, hashlib, json, pathlib, sys
BASE=pathlib.Path('/Volumes/AgentStorage/issue1419-finish29ad/export')
CAND=pathlib.Path('/Volumes/AgentStorage/issue1419-candidate-delta29ai/export')
GROUPS={'long':14,'revision':8,'realrule':3}
POLICY='raw-initialization-w/1'
JSON_FIELDS={'payload_json','input_refs_json','source_coords_json'}
def load(path): return json.loads(path.read_text())
def canon(v): return json.dumps(v,sort_keys=True,separators=(',',':'),ensure_ascii=False)
def coords(xs): return sorted(set(xs),key=int)
def raw(e): return {k:e[k] for k in ('open','high','low','close','source_coord')}
def ref(e):
 return {k:str(e[k]) for k in ('event_id','identity_key','input_revision','payload_hash','receipt_id','revision','seq','source_coord')}
def strict(a,b):
 d=[int(b[k])-int(a[k]) for k in ('high','low')]
 return 'UP' if min(d)>0 else 'DOWN' if max(d)<0 else None
def contained(a,b):
 ah,al,bh,bl=(int(x[k]) for x,k in ((a,'high'),(a,'low'),(b,'high'),(b,'low')))
 return (ah>=bh and al<=bl) or (bh>=ah and bl<=al)
def evidence(a,b,ss):
 return dict(direction=strict(a,b),established_at=b['source_coord'],incoming=copy.deepcopy(b),previous_acc=copy.deepcopy(a),source_coords=coords(ss))
def compare_endpoints(a,b):
 if a is None: return [],[]
 cmp=[dict(axis=k,acc=a[k],incoming=b[k],order='LT' if int(a[k])<int(b[k]) else 'GT' if int(a[k])>int(b[k]) else 'EQ') for k in ('high','low')]
 classes=collections.defaultdict(list)
 for label,v in [('acc',a),('incoming',b)]:
  for k in ('high','low'): classes[int(v[k])].append(label+'.'+k)
 return cmp,[sorted(classes[p]) for p in sorted(classes)]

class Codec:
 """公开 s-content/1 的独立实现；nodes 保存每个必需节点。"""
 def __init__(self): self.nodes={}
 def f(self,tag,body):
  d=hashlib.sha256(b'NewChanlun/SContent/1\0'+bytes([tag])+len(body).to_bytes(8,'big')+body).digest()
  self.nodes['sc1-'+d.hex()]={'tag':tag,'body_hex':body.hex()}
  return d
 def m(self,hs):
  n=len(hs)
  if not n:return self.f(0x11,b'')
  if n==1:return self.f(0x12,hs[0])
  k=1<<((n-1).bit_length()-1)
  return self.f(0x13,n.to_bytes(8,'big')+self.m(hs[:k])+self.m(hs[k:]))
 def blob(self,b): return self.m([self.f(0x10,b[i:i+4096]) for i in range(0,len(b),4096)])
 def value(self,v,stored=False):
  if v is None:return self.f(0,b'')
  if isinstance(v,bool):return self.f(2 if v else 1,b'')
  if isinstance(v,(str,int)):
   b=(v if isinstance(v,str) else str(v)).encode()
   return self.f(3 if isinstance(v,str) else 4,len(b).to_bytes(8,'big')+self.blob(b))
  if isinstance(v,list):
   return self.f(5,len(v).to_bytes(8,'big')+self.m([self.value(x,stored) for x in v]))
  if isinstance(v,dict):
   fields=[]
   for k in sorted(v,key=lambda x:x.encode()):
    if stored and k in JSON_FIELDS:
     inner=json.loads(v[k]); assert canon(inner)==v[k],('noncanonical json view',k)
     digest=self.f(7,self.value(inner,False))
    else:digest=self.value(v[k],stored)
    fields.append(self.f(0x14,self.value(k)+digest))
   return self.f(6,len(v).to_bytes(8,'big')+self.m(fields))
  raise TypeError(type(v))
 def object_id(self,obj,state):
  value={'kind':obj['kind'],'fact_key':obj['fact_key'],'payload':json.loads(obj['payload_json']),'input_refs':json.loads(obj['input_refs_json']),'profile_id':state['profile_id'],'rule_revision':state['rule_revision']}
  return 'obj3-'+self.f(0x20,self.value(value)).hex()

def fold(events):
 """直接按区间规则左折；审计 trace 是旁路输出，不调用任何生产模块。"""
 bars=[raw(e) for e in events]; init=None
 for j in range(1,len(bars)):
  if strict(bars[j-1],bars[j]) is not None:
   init=evidence(bars[j-1],bars[j],[v['source_coord'] for v in bars[:j+1]]); break
 if init is None:return None,[],[]
 groups=[];steps=[];ev=copy.deepcopy(init)
 def group(bar,ev):
  return {'bar':copy.deepcopy(bar),'members':[bar['source_coord']],'high_sources':[bar['source_coord']],'low_sources':[bar['source_coord']],'direction_evidence':copy.deepcopy(ev),'confirmation_evidence':None,'confirmed':False}
 def step(incoming,before,after,action,inc,ss,g):
  cmp,endpoints=compare_endpoints(before,incoming)
  payload=dict(action=action,incoming=copy.deepcopy(incoming),acc_before=copy.deepcopy(before),acc_after=copy.deepcopy(after),contains=inc,direction=ev['direction'],direction_evidence=copy.deepcopy(ev),comparisons=cmp,endpoint_order=endpoints,high_sources=g['high_sources'][:],low_sources=g['low_sources'][:],waiting_reasons=['equal_extreme_identity'] if max(len(g['high_sources']),len(g['low_sources']))>1 else [])
  steps.append((payload,coords(ss)))
 groups.append(group(bars[0],ev))
 step(bars[0],None,bars[0],'seed',None,init['source_coords'],groups[-1])
 for b in bars[1:]:
  g=groups[-1];a=copy.deepcopy(g['bar'])
  ss=coords(g['members']+ev['source_coords']+[b['source_coord']])
  inc=contained(a,b)
  if inc:
   after=copy.deepcopy(a); after['close']=b['close']
   for axis in ('high','low'):
    pick=max if ev['direction']=='UP' else min
    after[axis]=str(pick(int(a[axis]),int(b[axis])))
    if b[axis]==after[axis]:
     g[axis+'_sources']=(g[axis+'_sources'] if a[axis]==after[axis] else [])+[b['source_coord']]
   g['members'].append(b['source_coord']);g['bar']=after
  else:
   ev=evidence(a,b,ss); assert ev['direction'] is not None
   g['confirmed']=True;g['confirmation_evidence']=copy.deepcopy(ev)
   after=copy.deepcopy(b);g=group(after,ev);groups.append(g)
  step(b,a,after,'merge' if inc else 'new_group',inc,ss,g)
 output=[]
 for i,g in enumerate(groups):
  p={k:v for k,v in g['bar'].items() if k!='source_coord'}
  reasons=['equal_extreme_identity'] if max(len(g['high_sources']),len(g['low_sources']))>1 else []
  p.update(group_anchor=g['bar']['source_coord'],group_index=str(i),members=g['members'],high_sources=g['high_sources'],low_sources=g['low_sources'],confirmed=g['confirmed'],confirmation_evidence=g['confirmation_evidence'],direction=g['direction_evidence']['direction'],direction_evidence=g['direction_evidence'],waiting_reasons=reasons,initial_direction_evidence=init,initialization_policy_revision=POLICY)
  ss=g['members']+g['direction_evidence']['source_coords']
  if g['confirmation_evidence']:ss+=g['confirmation_evidence']['source_coords']
  output.append((p,coords(ss)))
 return init,steps,output

def request(slot,subject,reasons,ss):
 insufficient=any(r in ('fewer_than_three_merged_bars','no_subsequent_bar','direction_not_yet_established','initial_direction_unsettled') for r in reasons)
 return dict(request_id=canon(slot),subject_id=subject,reasons=reasons,source_coords=coords(ss),axes=dict(input_quality='INSUFFICIENT' if insufficient else 'SUFFICIENT',applicability='DOMAIN_PROOF_MISSING' if reasons else 'IN_DOMAIN',computation='INCOMPLETE' if reasons else 'COMPLETE',validity='CURRENT'))

def expected(base):
 """唯一 expected 入口：不接收 candidate，不读取候选路径。"""
 out=copy.deepcopy(base);codec=Codec()
 events=base['effective_events']; bycoord={e['source_coord']:e for e in events}
 init,steps,groups=fold(events);und=init is None
 # 本原25合同每代prior为空；发布生命周期不在本回归的输入内。
 gen=base['generation'];cut=base['structure_cut'];new=[];req=[]
 oldks=next(o for o in base['objects'] if o['kind']=='CC-054.knowledge_state')
 old_payload=json.loads(oldks['payload_json'])
 old_requests={q['request_id']:q for q in old_payload['requests']}
 def add(kind,slot,payload,ss):
  ss=coords(ss)
  o=dict(kind=kind,fact_key=canon(slot),payload_json=canon(payload),input_refs_json=canon([ref(bycoord[s]) for s in ss]),source_coords_json=canon(ss),first_known_cut=cut,first_known_generation=gen,published_generation=gen,object_revision=1)
  o['object_id']=codec.object_id(o,base);new.append(o);return o
 state='Undetermined' if und else 'Established'
 ss=list(bycoord) if und else init['source_coords']
 slot=['CC-004.initialization','W']
 w=add(slot[0],slot,dict(direction_state=state,first_raw_pair=init,initialization_policy_revision=POLICY),ss)
 req.append(request(slot,w['object_id'],['initial_direction_unsettled'] if und else [],ss))
 if und:
  req.append(request(['CC-005.inclusion_group','pending'],None,['initial_direction_unsettled'],list(bycoord)))
  for e in events:
   slot=['CC-004.undetermined_raw',e['source_coord']]
   u=add(slot[0],slot,dict(raw=raw(e),direction_state=state,initialization_policy_revision=POLICY,pending_since_cut=cut),[e['source_coord']])
   req.append(request(slot,u['object_id'],['initial_direction_unsettled'],[e['source_coord']]))
 else:
  for kind,entries,key in [('CC-004.inclusion_step',steps,'incoming'),('CC-005.inclusion_group',groups,'group_anchor')]:
   for p,ss in entries:
    coord=p[key]['source_coord'] if key=='incoming' else p[key]
    slot=[kind,coord];o=add(kind,slot,p,ss)
    # W只消除未建立方向；其他请求轴和原因保持原base合同，不能拿旧W历史实现覆盖。
    q=copy.deepcopy(old_requests[canon(slot)])
    q.update(subject_id=o['object_id'],source_coords=coords(ss))
    if 'direction_not_yet_established' in q['reasons']:
     q=request(slot,o['object_id'],[r for r in q['reasons'] if r!='direction_not_yet_established'],ss)
    req.append(q)
 oldks=next(o for o in base['objects'] if o['kind']=='CC-054.knowledge_state')
 p=json.loads(oldks['payload_json'])
 # 未改的后半请求逐项锁原base；不得排序，也不得按subject集合化。
 req.extend(copy.deepcopy(q) for q in p['requests'] if json.loads(q['request_id'])[0] not in ('CC-004.inclusion_step','CC-005.inclusion_group'))
 reasons=sorted(set(p['waiting_reasons'])|{'initial_direction_unsettled'}) if und else [r for r in p['waiting_reasons'] if r!='direction_not_yet_established']
 p.update(requests=req,waiting_reasons=reasons,domain='raw_adjacent_W_initialization',direction_state=state,initialization_policy_revision=POLICY)
 add('CC-054.knowledge_state',['CC-054.knowledge_state','TB-02-A'],p,list(bycoord))
 # 保持原生产总线序：shape → generic(kind,key,id排序) → TB02B线序。
 generic_kinds={'CC-004.inclusion_step','CC-005.inclusion_group','CC-007.fractal_description','CC-054.knowledge_state'}
 new.extend(copy.deepcopy(o) for o in base['objects'] if o['kind']=='CC-007.fractal_description')
 new.sort(key=lambda o:(o['kind'],o['fact_key'],o['object_id']))
 result=[];inserted=False
 for o in base['objects']:
  if o['kind'] in generic_kinds:
   if not inserted:result.extend(new);inserted=True
   continue
  o=copy.deepcopy(o)
  if o['kind']=='CC-056.version' and und:
   p=json.loads(o['payload_json']);p['data']['waiting_reasons']=['initial_direction_unsettled']
   o['payload_json']=canon(p);o['object_id']=codec.object_id(o,base)
  result.append(o)
 assert inserted
 out['objects']=result
 obs=[];inserted=False
 for o in base['observations']:
  if o.get('detail',{}).get('tb02a'):
   if not inserted:
    for reason in reasons:
     x=copy.deepcopy(o);x['reason']=reason;x['detail']['waiting_reasons']=reasons[:];obs.append(x)
    inserted=True
  else:obs.append(copy.deepcopy(o))
 assert inserted
 out['observations']=obs
 return out,dict(direction_state=state,steps=len(steps),groups=len(groups),objects=len(result),requests=len(req))

def diff(expected,actual,path=''):
 errors=[]
 if type(expected)!=type(actual):return [dict(path=path,expected=expected,actual=actual)]
 if isinstance(expected,dict):
  if expected.keys()!=actual.keys(): errors.append(dict(path=path+'/keys',expected=sorted(expected),actual=sorted(actual)))
  for k in [k for k in expected if k in actual]:errors.extend(diff(expected[k],actual[k],path+'/'+k))
 elif isinstance(expected,list):
  if len(expected)!=len(actual):errors.append(dict(path=path+'/length',expected=len(expected),actual=len(actual)))
  for i,(a,b) in enumerate(zip(expected,actual)):errors.extend(diff(a,b,path+'/'+str(i)))
 elif expected!=actual:
  if path.rsplit('/',1)[-1] in JSON_FIELDS and isinstance(expected,str):
   nested=diff(json.loads(expected),json.loads(actual),path+'#json')
   errors.extend(nested or [dict(path=path+'/canonical_bytes',expected=expected,actual=actual)])
  else:errors.append(dict(path=path,expected=expected,actual=actual))
 return errors

def leaf_count(x):
 if isinstance(x,dict):return 1+sum(leaf_count(v) for v in x.values())
 if isinstance(x,list):return 1+sum(leaf_count(v) for v in x)
 return 1

def check_nodes(codec,inventory):
 errors=[]
 for digest,body in codec.nodes.items():
  # 0x20 identity framing is not a stored content node.
  if body['tag']==0x20:continue
  if digest not in inventory:errors.append({'path':'nodes/'+digest,'expected':body,'actual':'MISSING'})
  elif inventory[digest]!=body:errors.append({'path':'nodes/'+digest,'expected':body,'actual':inventory[digest]})
 return errors

def negative_controls(expected_state,candidate):
 trials=[]
 def run(name,mut):
  x=copy.deepcopy(candidate);mut(x);errors=diff(expected_state,x)
  trials.append(dict(mutation=name,caught=bool(errors),paths=[e['path'] for e in errors[:5]]))
 def payload(kind,fn):
  def change(x):
   o=next(o for o in x['objects'] if o['kind']==kind);p=json.loads(o['payload_json']);fn(p);o['payload_json']=canon(p)
  return change
 run('raw-high',lambda x:x['raw_events'][0].update(high='-999'))
 run('W-state',payload('CC-004.initialization',lambda p:p.update(direction_state='BROKEN')))
 run('object-id',lambda x:x['objects'][0].update(object_id='obj3-bad'))
 def whitespace(x):
  o=next(o for o in x['objects'] if 'payload_json' in o);o['payload_json']+=' '
 run('json-text-whitespace',whitespace)
 run('input-reference',lambda x:x['objects'][0].update(input_refs_json='[]'))
 run('source-coordinate',lambda x:x['objects'][0].update(source_coords_json='[]'))
 run('generation-metadata',lambda x:x['objects'][0].update(first_known_generation=999))
 run('object-array-order',lambda x:x['objects'].reverse())
 run('missing-object',lambda x:x['objects'].pop())
 run('duplicate-object',lambda x:x['objects'].append(copy.deepcopy(x['objects'][0])))
 run('request-order',payload('CC-054.knowledge_state',lambda p:p['requests'].reverse()))
 run('request-subject',payload('CC-054.knowledge_state',lambda p:p['requests'][0].update(subject_id='foreign')))
 run('observation-reason',lambda x:x['observations'][0].update(reason='foreign'))
 run('CC056-waiting',payload('CC-056.version',lambda p:p['data'].update(waiting_reasons=['foreign'])))
 if any(o['kind']=='CC-004.undetermined_raw' for o in candidate['objects']):
  run('pending-cut',payload('CC-004.undetermined_raw',lambda p:p.update(pending_since_cut='cut-999')))
 else:
  run('seed-direction',payload('CC-004.inclusion_step',lambda p:p.update(direction='UP')))
  run('group-members',payload('CC-005.inclusion_group',lambda p:p['members'].append('999')))
  run('group-confirmation',payload('CC-005.inclusion_group',lambda p:p.update(confirmed=False)))
  def badmerge(x):
   o=next(o for o in x['objects'] if o['kind']=='CC-004.inclusion_step' and json.loads(o['payload_json'])['action']=='merge')
   p=json.loads(o['payload_json']);p['acc_after']['high']='-999';o['payload_json']=canon(p)
  if any(o['kind']=='CC-004.inclusion_step' and json.loads(o['payload_json'])['action']=='merge' for o in candidate['objects']):run('merge-high',badmerge)
 return trials

def main():
 parser=argparse.ArgumentParser();parser.add_argument('--base',type=pathlib.Path,default=BASE);parser.add_argument('--candidate',type=pathlib.Path,default=CAND);parser.add_argument('--out',type=pathlib.Path,default=pathlib.Path(__file__).parent);parser.add_argument('--emit',type=pathlib.Path);args=parser.parse_args()
 allerrors=[];rows=[];trials=[];total=0
 original_roots=load(pathlib.Path(__file__).with_name('original_base_roots.json'))
 for group,n in GROUPS.items():
  be=load(args.base/group/'events.json');ce=load(args.candidate/group/'events.json');assert be==ce,group+' events changed'
  cuts=load(args.base/group/'cuts.json');cc=load(args.candidate/group/'cuts.json');assert len(cuts)==len(cc)==n
  inventory={}
  for line in (args.candidate/group/'nodes.jsonl').read_text().splitlines():
   node=json.loads(line);digest=node.pop('digest')
   assert digest not in inventory or inventory[digest]==node
   inventory[digest]=node
  for cut,cutc in zip(cuts,cc):
   fields=['cut','gen','prefix','rule','profile_id','profile_hash','catalog_revision','scope','semantic_commit_ns']
   assert all(cut[k]==cutc[k] for k in fields),(group,cut['gen'],'cut mismatch')
   gen=cut['gen'];base=load(args.base/group/f'g{gen}.generated_state.json');cand=load(args.candidate/group/f'g{gen}.generated_state.json')
   assert base==load(args.base/group/f'g{gen}.state_without_axes.json')
   base_root='sc1-'+Codec().value(base,True).hex()
   assert base_root==original_roots[group][gen-1]==cut['generated_state']['root']==cut['state_without_axes']['root'],(group,gen,'original frozen base root mismatch')
   expected_state,stats=expected(base);errors=diff(expected_state,cand)
   errors+=diff(expected_state,load(args.candidate/group/f'g{gen}.state_without_axes.json'),'/state_without_axes')
   codec=Codec();root='sc1-'+codec.value(expected_state,True).hex()
   cref=dict(format='s-content/1',root=root,kind='object',extent=len(expected_state))
   errors+=diff(cref,cutc['generated_state'],'/generated_state_ref')
   errors+=diff(cref,cutc['state_without_axes'],'/state_without_axes_ref')
   # nodes.jsonl 是最终 store.iter()，不是写入时间日志；只能核可达闭包。
   cut_inventory=inventory
   errors+=check_nodes(codec,cut_inventory)
   assert len(codec.nodes)<=cutc['nodes_after_cut']<=len(inventory),(group,gen,'node count impossible')
   for error in errors:error.update(group=group,generation=gen)
   allerrors.extend(errors);total+=leaf_count(expected_state)*2
   if args.emit:
    (args.emit/group).mkdir(parents=True,exist_ok=True);(args.emit/group/f'g{gen}.expected_state.json').write_text(json.dumps(expected_state,ensure_ascii=False,sort_keys=True,separators=(',',':')))
   rows.append(dict(group=group,generation=gen,**stats,required_nodes=len(codec.nodes),errors=len(errors),root=root))
   if group=='long' and gen in (1,7):
    trials.extend(dict(group=group,generation=gen,**t) for t in negative_controls(expected_state,cand))
    if gen>1:
     stale=load(args.candidate/group/f'g{gen-1}.generated_state.json')
     trials.append(dict(group=group,generation=gen,mutation='stale-generation-replay',caught=bool(diff(expected_state,stale)),paths=['/generation','/objects']))
    for name,inv in [('missing-root-node',{d:v for d,v in cut_inventory.items() if d!=root}),('corrupt-root-node',{**cut_inventory,root:dict(tag=0,body_hex='')})]:
     node_errors=check_nodes(codec,inv)
     trials.append(dict(group=group,generation=gen,mutation=name,caught=bool(node_errors),paths=[e['path'] for e in node_errors[:5]]))
    badref=copy.deepcopy(cutc['generated_state']);badref['extent']+=1
    trials.append(dict(group=group,generation=gen,mutation='content-ref-extent',caught=bool(diff(cref,badref)),paths=['/generated_state_ref/extent']))
 result=dict(status='PASS' if not allerrors and all(t['caught'] for t in trials) else 'FAIL',cases=len(rows),structural_checks=total,errors=allerrors,negative_controls=trials,cases_detail=rows)
 args.out.mkdir(parents=True,exist_ok=True)
 (args.out/'results.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
 (args.out/'coverage.tsv').write_text('\t'.join(rows[0])+'\n'+'\n'.join('\t'.join(str(v) for v in row.values()) for row in rows)+'\n')
 print(json.dumps({k:v for k,v in result.items() if k not in ('errors','negative_controls','cases_detail')},ensure_ascii=False));print('errors:',len(allerrors),'negative controls:',len(trials));print(json.dumps(allerrors[:5],ensure_ascii=False,indent=2))
 return 0 if result['status']=='PASS' else 1
if __name__=='__main__':sys.exit(main())

