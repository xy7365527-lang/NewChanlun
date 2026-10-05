#!/usr/bin/env python3
"""L67-live-A plus one necessary R control; no RW parser, no history generator."""
from pathlib import Path
from fractions import Fraction
import hashlib, json, resource, sys, time

HERE = Path(__file__).resolve().parent
OUT = Path(sys.argv[1]).resolve() if len(sys.argv)>1 else HERE
OUT.mkdir(parents=True, exist_ok=True)
E = HERE.parents[1]
R = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
A = E / 'stage66/nested-author/run'
V = E / 'stage66/nested-review'
INPUTS = [A/'source.json', A/'reference.stdout', V/'independent-prefixes.json',
          V/'independent-pens.json', V/'independent-objects.json', V/'independent-parent.json',
          V/'independent-narrow-segments.json', V/'independent-S15.json',
          V/'source-line-audit.json', V/'review.md',
          E/'stage66/adoption/ScopeCorrections-v2.md',
          R/'.chanlun/definitions/beichi.md',
          *[R/'docs/chanlun/text/blog'/f'{n}-第{int(n)}课.md' for n in ['061','033','079']],
          HERE/'WorkCard.md']

def digest(path):
    h=hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda:f.read(65536),b''):h.update(chunk)
    return h.hexdigest()
def hashes():return [{'path':str(p),'bytes':p.stat().st_size,'sha256':digest(p)} for p in INPUTS]
def write(name,data):
    (OUT/name).write_text(json.dumps(data,ensure_ascii=False,indent=2)+'\n')
def read(path):return json.loads(path.read_text())
def q(x):
    if x is None:return None
    x=Fraction(x)
    return {'num':x.numerator,'den':x.denominator}
def val(x):return Fraction(x['num'],x['den']) if x is not None else None

t0=time.perf_counter(); before=hashes();write('inputhash-before.json',before)
source=read(A/'source.json'); prefixes=read(V/'independent-prefixes.json')
reference=read(A/'reference.stdout')
assert reference['prefixes']==prefixes and len(prefixes)==443
# Read only the frozen per-prefix states; reference final strokes is discarded.
del reference
assert [o['index'] for o in source['observations']]==list(range(443))
assert [o['time'] for o in source['observations']]==list(range(443))

# Exact quantity verification is source-only and incremental; no RW reconstruction.
ask=int(source['initial']['ask_quantity']); bid=int(source['initial']['bid_quantity'])
seen=[]; times=[]; certificates={}; narrow_failures={}; sealed={}; rows=[]; role_first={}; transitions=[]
firsts={n:{m:{'weak':None,'extreme':None,'complete_candidate':None} for m in ['A','R']} for n in ['P0','P1','P2','c0','parent']}
closed_candidates=[]; pen_first_stable={}; anchor_first={}; last_anchors={}; last_bools={}

def rng(start,end):
    assert 0<=start<=end<len(seen)
    return [min(seen[start:end+1]),max(seen[start:end+1])]
def ir(p):return sorted([p['start_price'],p['end_price']])
def inter(xs):return [max(x[0] for x in xs),min(x[1] for x in xs)]
def inc(a,b):return a[0]<=b[0] and b[1]<=a[1]
def speed(p):return Fraction(p['end_price']-p['start_price'],times[p['end']]-times[p['start']])
def closed_force(stable,start,end):
    ps=[p for p in stable if start<=p['start'] and p['end']<=end]
    assert ps and ps[0]['start']==start and ps[-1]['end']==end
    return {'first':ps[0], 'last':ps[-1], 'first_v':q(speed(ps[0])), 'last_v':q(speed(ps[-1])), 'L':q(speed(ps[-1])-speed(ps[0]))}
def narrow(stable,block):
    begin=5*block; members=stable[begin:begin+5]
    if len(members)!=5:return None,'insufficient-stable'
    up=members[0]['up']; fs=[(j,ir(p)) for j,p in enumerate(stable) if j>=begin and p['up']!=up]
    selected=None
    for j in range(1,len(fs)):
        if inc(fs[j-1][1],fs[j][1]) or inc(fs[j][1],fs[j-1][1]):return None,'containment'
        if j<2:continue
        x,y,z=[p[1] for p in fs[j-2:j+1]]
        if (y[1]>max(x[1],z[1]) if up else y[0]<min(x[0],z[0])):
            selected=fs[j-2:j+1];break
    if not selected:return None,'no-selected-fractal'
    x,y,z=[p[1] for p in selected];i=inter([ir(p) for p in members[:3]])
    start,end=members[0]['start'],members[-1]['end']
    if not ((y[0]>max(x[0],z[0]) if up else y[1]<min(x[1],z[1])) and inter([x,y])[0]<inter([x,y])[1] and i[0]<i[1] and rng(start,end)==sorted([seen[start],seen[end]]) and seen[end]==(y[1] if up else y[0])):return None,'geometry-false'
    return {'block':block,'up':up,'start':start,'end':end,'range':rng(start,end),'witness_pen_slots':[v[0] for v in selected],'evidence':'narrow'},None

def selected_s15(stable):
    # A separately named inherited source071 certificate; never called as fallback.
    if 14 not in certificates or len(stable)<83:return None
    old,new=stable[75:80],stable[80:83]
    if [p['up'] for p in old]!=[True,False,True,False,True] or [p['up'] for p in new]!=[False,True,False]:return None
    d=[old[0]['start_price'],old[1]['end_price'],old[3]['end_price']];g=[old[0]['end_price'],old[2]['end_price'],old[4]['end_price']]
    breaks=lambda ds:[(i+1,j+1) for i in range(len(g)) for j in range(len(ds)) if j>=i+2 and ds[j]<=g[i]]
    if breaks(d)!=[] or breaks(d+[new[0]['end_price']])!=[(2,4)]:return None
    io=inter([ir(p) for p in old[:3]]);inew=inter([ir(p) for p in new])
    if not(io[0]<io[1] and inew[0]<inew[1] and new[-1]['end_price']<new[0]['end_price'] and new[1]['end_price']<new[0]['start_price'] and certificates[14]['end']==old[0]['start']):return None
    return {'block':15,'up':True,'start':old[0]['start'],'end':old[-1]['end'],'range':rng(old[0]['start'],old[-1]['end']),'witness_pen_slots':list(range(75,83)),'evidence':'selected-source071'}

def note_role(name,key,t,detail):
    d=role_first.setdefault(name,{})
    if key not in d:d[key]={'first_known_at':t,**detail}

def live(prefix,start,mode):
    stable=prefix['stable']; first=next((p for p in stable if p['start']==start),None)
    if first is None:return {'L':None,'reason':'first-pen-not-stable'}
    if mode=='A':
        p=prefix['active'] or (stable[-1] if stable else None)
        if p is None:return {'L':None,'reason':'no-active-or-stable-pen'}
        anchor=p['start'];kind='real-active-start' if prefix['active'] else 'recent-stable-start'
    else:
        if not prefix['anchors']:return {'L':None,'reason':'no-known-anchor'}
        anchor=prefix['anchors'][-1]['raw'];kind='unconfirmed-residual'
    if anchor<start:return {'L':None,'reason':'selected-pen-anchor-precedes-c','anchor':anchor}
    duration=times[-1]-times[anchor]
    if duration<=0:return {'L':None,'reason':'zero-duration','anchor':anchor}
    v=Fraction(seen[-1]-seen[anchor],duration)
    return {'L':q(v-speed(first)),'v':q(v),'first_v':q(speed(first)),'first_pen':first,'anchor':anchor,'anchor_price':seen[anchor],'duration':duration,'kind':kind,'signed':True}

def emit(name,t,prefix,start,role_ready,bL,extreme,details,closed_now=False):
    result={'id':name,'cut':t,'price':seen[-1],'c_start':start,'local_role_gate':role_ready,'source_role_gate':None,'source_complete':None,'extreme':extreme,'lifecycle':'closed-local-certificate' if closed_now else 'candidate-open','eligible_now':bool(role_ready and not closed_now),**details}
    for mode in ['A','R']:
        v=({'L':None,'reason':'local-closed-certificate-no-live-domain'} if closed_now else (live(prefix,start,mode) if start is not None else {'L':None,'reason':'c-role-start-not-known'}))
        weak=val(v['L'])<bL if v['L'] is not None and bL is not None else None
        trigger=bool(role_ready and not closed_now and weak is True and extreme is True)
        result[mode]={**v,'weak':weak,'complete_candidate':trigger}
        for k,b in [('weak',role_ready and not closed_now and weak is True),('extreme',role_ready and not closed_now and extreme is True),('complete_candidate',trigger)]:
            if b and firsts[name][mode][k] is None:firsts[name][mode][k]=t
        if v.get('anchor') is not None:
            key=(name,mode);prev=last_anchors.get(key)
            if prev is not None and prev!=v['anchor']:transitions.append({'id':name,'mode':mode,'cut':t,'old_anchor':prev,'new_anchor':v['anchor'],'L':v['L']})
            last_anchors[key]=v['anchor']
        key=(name,mode)
        if key in last_bools and last_bools[key] and not trigger:result[mode]['candidate_withdrawn_at_this_prefix']=True
        last_bools[key]=trigger
        if result[mode].get('candidate_withdrawn_at_this_prefix'):result[mode]['withdrawal_reason']='local-certificate-closes-domain' if closed_now else 'predicate-no-longer-true'
    rows.append(result)

for t,prefix in enumerate(prefixes):
    o=source['observations'][t]
    if t:
        ev=source['events'][t-1]
        assert ev['id']==t and ev['time']==t and ev['side']=='bid' and ev['price']==source['bid'] and int(ev['quantity_before'])==bid
        amt=int(ev['amount']); assert amt>0
        bid+=amt if ev['action']=='add' else -amt
        assert bid>0 and bid==int(ev['quantity_after'])
    assert bid==int(o['bid_quantity']) and ask==int(o['ask_quantity'])
    raw=Fraction(source['ask']*bid+source['bid']*ask,bid+ask)
    assert raw==o['price'] and o['trade_volume']==0
    seen.append(o['price']);times.append(o['time'])
    assert prefix['cut']==t and prefix['stable_count']==len(prefix['stable'])
    for p in prefix['stable']+([prefix['active']] if prefix['active'] else []):
        assert p['start']<p['end']<=t and seen[p['start']]==p['start_price'] and seen[p['end']]==p['end_price']
    for an in prefix['anchors']:
        assert an['known_at']<=t and an['raw']<=t
        anchor_first.setdefault(an['raw'],t)
    for p in prefix['stable']:pen_first_stable.setdefault(p['start'],t)
    for block in range(21):
        cert,fail=narrow(prefix['stable'],block)
        if block==15:narrow_failures[t]=fail
        if cert and block not in certificates:certificates[block]={**cert,'known_at':t}
    special=selected_s15(prefix['stable'])
    if special and 15 not in certificates:certificates[15]={**special,'known_at':t}
    for k,name in enumerate(['P0','P1','P2','c0']):
        bslot=1+5*k; start_slot=5*bslot
        current_pens=prefix['stable']+([prefix['active']] if prefix['active'] else [])
        # Naming a start requires an actual observed anchor/pen at the frozen ordinal.
        if len(prefix['anchors'])>start_slot:
            a=prefix['anchors'][start_slot]['raw'];note_role(name,'start_anchor',t,{'at':a})
            if a in pen_first_stable:note_role(name,'b_first_pen_stable',t,{'at':a})
        if bslot-1 in certificates:
            note_role(name,'template_start_role',t,{'at':certificates[bslot-1]['end'],'basis':'preceding-certified-raw-block; finite template'})
        if bslot in certificates:note_role(name,'b_closed',t,{'start':certificates[bslot]['start'],'end':certificates[bslot]['end']})
        c_anchor_slot=5*(bslot+4)
        if len(prefix['anchors'])>c_anchor_slot:
            ca=prefix['anchors'][c_anchor_slot]['raw'];note_role(name,'c_anchor_observed',t,{'at':ca})
            if ca in pen_first_stable:note_role(name,'c_first_pen_stable',t,{'at':ca})
        cs=None;bL=None;core=None;ext=None;ready=False;extras={}
        if all(j in certificates for j in range(bslot,bslot+4)):
            b=certificates[bslot];ks=[certificates[j] for j in range(bslot+1,bslot+4)]
            core=inter([z['range'] for z in ks]);cs=ks[-1]['end']
            dirs=[certificates[j]['up'] for j in range(bslot,bslot+4)]
            ready=core[0]<core[1] and all(a!=b for a,b in zip(dirs,dirs[1:]))
            bforce=closed_force(prefix['stable'],b['start'],b['end']);bL=val(bforce['L'])
            prior=(max if b['up'] else min)(seen[b['start']:cs+1]);ext=seen[-1]>prior if b['up'] else seen[-1]<prior
            extras={'K':core,'b_L':q(bL),'prior_extreme':prior,'object_start':b['start'],'direction':'Up' if b['up'] else 'Down'}
            note_role(name,'K_and_c_role',t,{'c_start':cs,'K':core,'basis':'three current-prefix raw certificates, finite template'})
        emit(name,t,prefix,cs,ready,bL,ext,extras,closed_now=(bslot+4 in certificates))
        # Full frozen five-block object certificate is a separate diagnostic.
        if name not in sealed and all(j in certificates for j in range(bslot,bslot+5)):
            ss=[certificates[j] for j in range(bslot,bslot+5)];end=ss[-1]['end']
            cf=closed_force(prefix['stable'],ss[-1]['start'],end)
            endpoint_ext=seen[end]>prior if ss[0]['up'] else seen[end]<prior
            sealed[name]={'id':name,'start':ss[0]['start'],'end':end,'whole':rng(ss[0]['start'],end),'known_at':t,'core':core,'b_force':bforce,'c_force':cf,'endpoint_local':bool(val(cf['L'])<bL and endpoint_ext)}
            closed_candidates.append(sealed[name]);note_role(name,'whole_endpoint_certificate',t,{'end':end,'is_original':False})
    pstart=None;pready=False;pL=None;pext=None;pd={}
    if all(n in sealed for n in ['P0','P1','P2']) and 0 in certificates:
        members=[sealed[n] for n in ['P0','P1','P2']];pcore=inter([o['whole'] for o in members]);b=certificates[0]
        pstart=members[-1]['end'];pL=val(closed_force(prefix['stable'],b['start'],b['end'])['L']);pready=pcore[0]<pcore[1] and all(x['endpoint_local'] for x in members)
        prior=min(seen[b['start']:pstart+1]);pext=seen[-1]<prior
        pd={'K':pcore,'b_L':q(pL),'prior_extreme':prior,'object_start':b['start'],'direction':'Down'}
        note_role('parent','K_and_c_role',t,{'c_start':pstart,'K':pcore,'basis':'three whole local endpoint certificates; original qualification unknown'})
        if pstart in pen_first_stable:note_role('parent','c_first_pen_stable',t,{'at':pstart,'actual_first_stable':pen_first_stable[pstart]})
    emit('parent',t,prefix,pstart,pready,pL,pext,pd,closed_now=('c0' in sealed))

# Parent raw b and c primitive clocks, kept separate from later parent-role eligibility.
role_first['parent'].update({
 'start_anchor':{'first_known_at':anchor_first[certificates[0]['start']],'at':certificates[0]['start']},
 'b_first_pen_stable':{'first_known_at':pen_first_stable[certificates[0]['start']],'at':certificates[0]['start']},
 'b_raw_closed':{'first_known_at':certificates[0]['known_at'],'start':certificates[0]['start'],'end':certificates[0]['end']},
 'c_anchor_observed':{'first_known_at':anchor_first[sealed['c0']['start']],'at':sealed['c0']['start']},
 'c_first_pen_stable_primitive':{'first_known_at':pen_first_stable[sealed['c0']['start']],'at':sealed['c0']['start']},
 'whole_endpoint_certificate':{'first_known_at':sealed['c0']['known_at'],'end':sealed['c0']['end'],'is_original':False}})
# Seal online results before reading retrospective reference endpoint objects.
write('prefix-results.json',rows);write('role-first-known.json',role_first)
write('first-triggers.json',firsts);write('active-reanchoring.json',transitions)
write('online-raw-certificates.json',list(certificates.values()));write('whole-certificates.json',closed_candidates)
write('S15-narrow-status.json',[{'cut':t,'narrow':False,'reason':f} for t,f in narrow_failures.items() if f=='containment'])
online_hash=digest(OUT/'prefix-results.json'); write('online-seal.json',{'sha256':online_hash,'rows':len(rows),'prefixes':443,'final_objects_read':False,'final_pens_read':False})

expected=read(V/'independent-objects.json');parent=read(V/'independent-parent.json');pens=read(V/'independent-pens.json')
assert len(certificates)==21 and len(sealed)==4 and len(pens)==110
oldraw=read(V/'independent-narrow-segments.json')['certificates'];olds15=read(V/'independent-S15.json')
for old in oldraw:
    z=certificates[int(old['id'][1:])];assert (z['start'],z['end'],z['known_at'])==(old['start'],old['end'],old['known_at'])
assert certificates[15]['known_at']==olds15['known_at']==338
for old in expected:
    z=sealed[old['id']];assert (z['start'],z['end'],z['whole'],z['known_at'])==(old['start'],old['end'],old['whole'],old['local_known_at'])
    assert val(z['b_force']['L'])==old['b_force']['L'] and val(z['c_force']['L'])==old['c_force']['L']
rowmap={(z['id'],z['cut']):z for z in rows}
comparisons=[]
for old in expected+[{'id':'parent','end':parent['local_happened_at'],'local_known_at':parent['local_known_at'],'c_force':parent['c_force']}]:
    at=rowmap[(old['id'],old['end'])];known=rowmap[(old['id'],old['local_known_at'])]
    comparisons.append({'id':old['id'],'frozen_endpoint':old['end'],'frozen_local_known_at':old['local_known_at'],'frozen_L_c':old['c_force']['L'],'A_at_frozen_endpoint':at['A'],'R_at_frozen_endpoint':at['R'],'A_equal_endpoint':val(at['A']['L'])==old['c_force']['L'],'R_equal_endpoint':val(at['R']['L'])==old['c_force']['L'],'current_L_at_certificate_time':{'A':known['A']['L'],'R':known['R']['L']},'first_triggers':firsts[old['id']]})
write('endpoint-comparison.json',comparisons)
# Whole c0 vs raw b uses first/last true pens, never only final c0 subsegment.
pforce=closed_force(prefixes[-1]['stable'],sealed['c0']['start'],sealed['c0']['end']);assert val(pforce['L'])==-3500 and parent['b_force']['L']==-1000
write('parent-whole-closure.json',{'b_L':parent['b_force']['L'],'c_full_force':pforce,'c_last_subsegment_L':expected[-1]['c_force']['L'],'different':val(pforce['L'])!=expected[-1]['c_force']['L'],'original_completed':None})

# Source provenance: capture lines with body boundary, author and annotation checks.
audit=[]
for name,chosen in [('061',[26,28]),('033',[24,26]),('079',[52,58,60,64])]:
    path=R/'docs/chanlun/text/blog'/f'{name}-第{int(name)}课.md';lines=path.read_text().splitlines(); boundary=next(i+1 for i,s in enumerate(lines) if '↑正文' in s)
    for line in chosen:
        s=lines[line-1];inline=next((j for token in ['(娇注：','（娇注：','(注:','（注：'] if (j:=s.find(token))>=0),None)
        body=s if inline is None else s[:inline]
        audit.append({'file':str(path),'sha256':digest(path),'line':line,'body_boundary':boundary,'author_line':12,'author':lines[11],'within_body':line<boundary,'line_starts_note':s.startswith(('（注','(注','（娇注','(娇注')),'contains_inline_note':inline is not None,'used_excerpt':body,'excluded_inline_note':s[inline:] if inline is not None else None})
path=R/'.chanlun/definitions/beichi.md';bl=path.read_text().splitlines()
for line in [263,265,309,315,317,319,321,338,342,356,465,466,510,563,565,568,569]:audit.append({'file':str(path),'line':line,'sha256':digest(path),'text':bl[line-1],'attribution':'project-doctrine-not-original-author'})
write('source-audit.json',audit)
# Concrete semantic obligations and minimal observed discriminators, not false defaults.
write('semantic-obligations.json',{
 'source_role_gate':None,'completed_original':None,'fixed_F2_qualification':None,
 'missing':[{'variable':'StructuralPenNow(c,t)','minimal_witness':{'object':'P0','cut':121,'real_active':prefixes[121]['active'],'latest_known_anchor':prefixes[121]['anchors'][-1],'A':rowmap[('P0',121)]['A'],'R':rowmap[('P0',121)]['R']},'obligation':'prove whether current unconfirmed residual is the structure-given pen for #873; not supplied'},
 {'variable':'Owner/Next/OriginalCrossingRole(b,K,c,t)','minimal_witness':{'object':'P0','cut':118,'local_role':role_first['P0']['K_and_c_role'],'primitive_children':'Raw067Segment selected certificates, not original completed Movement'},'obligation':'finite five-block template and three interval overlap do not prove original same-center most-recent same-direction crossing roles'},
 {'variable':'OriginalLowRankBRole(parent,t)','minimal_witness':{'cut':338,'b_start':1,'b_end':21,'raw_certificate_known':38,'local_parent_K':role_first['parent']['K_and_c_role']},'obligation':'raw b_low original comparison qualification remains unestablished; no interface false interpreted as semantics false'},
 {'variable':'v_first(c,t) at start before stable first pen','minimal_witness':{'P0_c_start':101,'anchor_known':102,'first_pen_stable':110},'obligation':'chosen fixed closed first-pen initialization cannot supply numeric L at every t from start; null is missing initialization, not incomparable force'}],
 'candidate_is_not_original_generaldiv':True,'no_endpoint_backfill':True,'no_RW_rerun':True,'no_new_history':True})
after=hashes();assert before==after;write('inputhash-after.json',after)
peak=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
if sys.platform!='darwin':peak*=1024
assert peak<=96*1024*1024,(peak,'RSS budget exceeded')
result={'status':'finite-run-complete-independent-review-pending','prefixes':443,'rows':len(rows),'raw_certificates':len(certificates),'narrow_certificates':20,'selected_S15':1,'objects':4,'parents':1,'peak_rss_bytes':peak,'elapsed_seconds':time.perf_counter()-t0,'input_hash_unchanged':True,'A_endpoint_equal':[z['A_equal_endpoint'] for z in comparisons],'R_endpoint_equal':[z['R_equal_endpoint'] for z in comparisons],'first_triggers':firsts,'source_gate':None,'original_completed':None,'fixed_F2':None}
write('result.json',result);print(json.dumps(result,ensure_ascii=False))
