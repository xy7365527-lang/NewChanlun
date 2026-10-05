#!/usr/bin/env python3
"""One frozen finite JointCut relation, not an R_W parser or production F2."""
from __future__ import annotations
import collections
import datetime
import hashlib
import json
import platform
import resource
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
R = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
E = Path('/Users/silencehan/Documents/Codex/research-evidence/issue1467')
P = R / '.chanlun/review-results/issue1467-f1-proof'
LIMIT = 96 * 1024 * 1024
DATA = {
    'source': E/'stage66/nested-author/run/source.json',
    'pens': E/'stage66/nested-review/independent-pens.json',
    'narrow': E/'stage66/nested-review/independent-narrow-segments.json',
    'S15': E/'stage66/nested-review/independent-S15.json',
    'raw-prefixes': E/'stage66/nested-review/independent-prefixes.json',
    'source-reading': E/'stage67/base-role-review/source-direct-reading.json',
}
READINGS = [
    P/'GoalReframe-v4.md', P/'Stage58InitialModelAndReflectionAudit.md',
    P/'Stage63CertificateTransport.md', P/'Stage65DomainAndLifecycleObstructions.md',
    P/'Stage66NestedExpressionAndConsumerScope.md', P/'Stage67LiveForceAndInitialRoots.md',
    P/'stage67/adoption/LiveForceCorrections-v1.md',
    P/'stage67/adoption/BaseRoleCorrections-v1.md',
    P/'stage67/base-role-review/review.md',
    HERE/'WorkCard-v2.md', HERE/'WorkCard-v2.freeze.json', Path(__file__).resolve(),
]
SOURCE_RANGES = {
    'docs/chanlun/text/blog/018-第18课.md': [(24,32),(40,52)],
    'docs/chanlun/text/blog/020-第20课.md': [(30,30),(40,44),(52,60)],
    'docs/chanlun/text/blog/035-第35课.md': [(14,24)],
    'docs/chanlun/text/blog/037-第37课.md': [(16,22)],
    'docs/chanlun/text/blog/043-第43课.md': [(18,48)],
    'docs/chanlun/text/blog/057-第57课.md': [(26,32)],
    'docs/chanlun/text/blog/061-第61课.md': [(26,28)],
    'docs/chanlun/text/blog/084-第84课.md': [(52,56)],
    '.chanlun/definitions/beichi.md': [(263,268),(313,343)],
    '.chanlun/definitions/qushi.md': [(40,70),(88,111)],
    '.chanlun/definitions/zhongshu.md': [(24,37),(132,170)],
}

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def save(name, obj):
    (HERE/name).write_text(json.dumps(obj, ensure_ascii=False, indent=2) + '\n')

def hashes(paths):
    return [{'path':str(p), 'sha256':sha(p), 'bytes':p.stat().st_size} for p in paths]

def force(seg, pens):
    a,z=seg['members']; first,last=pens[a],pens[z-1]
    fv=(first['end_price']-first['start_price'])/(first['end']-first['start'])
    lv=(last['end_price']-last['start_price'])/(last['end']-last['start'])
    return {'first_pen':a,'last_pen':z-1,'first_v':fv,'last_v':lv,'L':lv-fv}

def meet(ranges):
    return [max(x[0] for x in ranges), min(x[1] for x in ranges)]

def paths_from(start, edges):
    """All maximal candidate paths; no tie-break or empty-success convention."""
    bystart=collections.defaultdict(list)
    for e in edges: bystart[e['raw_start']].append(e)
    def visit(i, path):
        if not bystart[i]:
            yield path
        else:
            for edge in bystart[i]:
                yield from visit(edge['raw_stop'],path+[edge])
    return list(visit(start,[]))

def f2_contract(path, allow_pp):
    """Frozen necessary F2 interface relation, no production implementation claim."""
    pp=[(a['id'],b['id']) for a,b in zip(path,path[1:])
        if a['grade']==b['grade'] and a['kind']==b['kind']=='P']
    row={'delta':[x['id'] for x in path], 'kinds':[x['kind'] for x in path],
         'no_pp':not pp, 'pp_violations':pp, 'has_three':len(path)>=3,
         'counterfactual_no_pp_removed':allow_pp,
         'candidate_member_table':[], 'candidate_center':None,
         'candidate_interface_pass':False, 'original_interface_pass':False,
         'original_completion_status':'unknown', 'reasons':[]}
    if not path: row['reasons'].append('no_nonempty_candidate_delta')
    if not allow_pp and pp: row['reasons'].append('NoPP')
    if len(path)<3: row['reasons'].append('fewer_than_three_completed_candidate_children')
    else:
        first=path[:3]; core=meet([x['whole'] for x in first])
        continuous=all(a['end']==b['start'] for a,b in zip(first,first[1:]))
        alternating=first[0]['technical_up']!=first[1]['technical_up'] and first[1]['technical_up']!=first[2]['technical_up']
        samegrade=len({x['grade'] for x in first})==1
        row['candidate_member_table']=[{'position':i,'id':x['id'],'whole':x['whole'],
            'own':x['own'],'grade':x['grade'],'candidate_completed':x['JEnd68'],
            'original_completed':None,'MemberNext':first[i+1]['id'] if i<2 else None}
            for i,x in enumerate(first)]
        row['candidate_center']={'core':core,'strict':core[0]<core[1],
            'continuous':continuous,'directions_alternate':alternating,'same_grade':samegrade,
            'original_qualified':None}
        for name,ok in [('positive_core',core[0]<core[1]),('continuous',continuous),
                        ('direction_alternation',alternating),('same_grade',samegrade)]:
            if not ok: row['reasons'].append(name)
        row['candidate_interface_pass']=not row['reasons']
    row['original_reasons']=['OriginalCompleted(child) is unknown',
        'RootArm68 source compatibility unknown',
        'Outer0=core interpretation as original outer unknown',
        'JEnd68 => whole MovementCompleted not proved']
    return row

started=time.monotonic()
start_utc=datetime.datetime.now(datetime.timezone.utc).isoformat()
reading=json.loads(DATA['source-reading'].read_text())
tickets=[t for t in reading['tickets'] if t['issue'] in [865,873]]
ticket_paths=list(dict.fromkeys(Path(t['cache_path']) for t in tickets))
inputs=list(dict.fromkeys(list(DATA.values())+READINGS+[R/p for p in SOURCE_RANGES]+ticket_paths))
before=hashes(inputs); save('inputhash-before.json',before)
freeze=json.loads((HERE/'WorkCard-v2.freeze.json').read_text())
assert sha(HERE/'WorkCard-v2.md')==freeze['sha256']
source=json.loads(DATA['source'].read_text()); obs=source['observations']
pens=json.loads(DATA['pens'].read_text())
certs=json.loads(DATA['narrow'].read_text())['certificates']
certs.append(json.loads(DATA['S15'].read_text()))
segments=sorted(certs,key=lambda s:s['start'])
assert len(obs)==443 and len(segments)==21
assert [s['id'] for s in segments]==[f'S{i}' for i in range(21)]
for i,s in enumerate(segments):
    s['force']=force(s,pens)
    s['own']=[s['start']+1,s['end']]
    actual=[min(o['price'] for o in obs[s['start']:s['end']+1]),max(o['price'] for o in obs[s['start']:s['end']+1])]
    assert s['range']==actual
    s['start_price']=obs[s['start']]['price'];s['end_price']=obs[s['end']]['price']
    s['RawClosed']=True;s['ComparisonReady68']=True
    s['ComparisonReady_original']=None;s['MovementCompleted_original']=None
    s['ComparisonReady68_reason']='raw stable certificate plus explicit RootArm68 candidate role; not source proof'
    if i: assert segments[i-1]['end']==s['start']

windows=[]
for s in range(1,21):
    k=1
    while s+4*k<21:
        z=s+4*k+1; parts=segments[s:z]; c=parts[-1]; bref=parts[-5]
        cores=[]
        for j in range(k):
            members=parts[4*j+1:4*j+4]; core=meet([x['range'] for x in members])
            dirs=[x['up'] for x in members]
            cores.append({'id':f'IC68:{s}:{j}', 'members':[x['id'] for x in members],
                'core':core,'Outer0':core,'Outer0_original_DD_GG':None,
                'own':[members[0]['start']+1,members[-1]['end']],
                'strict':core[0]<core[1],
                'direction_alternates':dirs[0]!=dirs[1] and dirs[1]!=dirs[2],
                'happened':members[-1]['end'], 'raw_known':max(x['known_at'] for x in members)})
        up_all=all(b['core'][0]>a['core'][1] for a,b in zip(cores,cores[1:]))
        down_all=all(b['core'][1]<a['core'][0] for a,b in zip(cores,cores[1:]))
        kind='P' if k==1 else ('U' if up_all else 'D' if down_all else None)
        typed=kind is not None and (k==1 or (kind=='U')==c['up'])
        core=cores[-1]['core']
        if c['up']:
            entry_cross=bref['start_price']<core[0]<=bref['end_price']
            exit_cross=c['start_price']<=core[1]<c['end_price']
        else:
            entry_cross=bref['start_price']>core[1]>=bref['end_price']
            exit_cross=c['start_price']>=core[0]>c['end_price']
        past=obs[parts[0]['start']:c['start']+1]
        extreme=max(o['price'] for o in past) if c['up'] else min(o['price'] for o in past)
        is_extreme=c['end_price']>extreme if c['up'] else c['end_price']<extreme
        weak=c['force']['L']<bref['force']['L']
        structural=all(x['strict'] and x['direction_alternates'] for x in cores) and typed
        roles=bref['up']==c['up'] and entry_cross and exit_cross
        relation_checks={'strict_cores':all(x['strict'] for x in cores),
            'member_direction_alternates':all(x['direction_alternates'] for x in cores),
            'coherent_type':typed,'same_direction_comparison_arms':bref['up']==c['up'],
            'entry_cross':entry_cross,'exit_cross':exit_cross,'whole_extreme':is_extreme,
            'signed_force_weakening':weak}
        accepted=structural and roles and is_extreme and weak
        owned=list(range(parts[0]['start']+1,c['end']+1))
        union=[n for x in parts for n in range(x['start']+1,x['end']+1)]
        assert union==owned and len(union)==len(set(union))
        record={'id':f'W68:S{s}-S{z-1}', 'raw_start':s,'raw_stop':z,'k':k,'grade':0,
            'kind':kind,'conceptual_direction':None if kind=='P' else kind,
            'technical_up':c['up'],'raws':[x['id'] for x in parts], 'cores':cores,
            'start':parts[0]['start'],'end':c['end'],'own':[owned[0],owned[-1]],
            'whole':[min(o['price'] for o in obs[parts[0]['start']:c['end']+1]),max(o['price'] for o in obs[parts[0]['start']:c['end']+1])],
            'b_ref':bref['id'],'c':c['id'],'b_force':bref['force'],'c_force':c['force'],
            'prior_whole_extreme':extreme,'end_price':c['end_price'],
            'candidate_happened':c['end'],'candidate_known':max(x['known_at'] for x in parts),
            'structural_pass':structural,'role_pass':roles,'CycleDiv68':structural and roles and is_extreme and weak,
            'RawEnd68':True,'JEnd68':accepted,
            'OriginalGeneralDiv':None,'OriginalMovementCompleted':None,
            'original_happened':None,'original_known':None,'original_published':None,
            'checks':relation_checks,'rejections':[name for name,ok in relation_checks.items() if not ok],
            'owns_each_event_once':True}
        windows.append(record);k+=1
assert len(windows)==40
save('raw-roots.json',segments)
save('all-windows.json',windows)

trace=[]; maximal_examples=[]; witness=None; cf_witness=None
for t in range(443):
    available=[w for w in windows if w['candidate_known']<=t]
    edges=[w for w in available if w['JEnd68']]
    paths=paths_from(1,edges)
    rows=[f2_contract(path,False) for path in paths]
    cf=[f2_contract(path,True) for path in paths]
    if witness is None:
        witness=next(({'t':t,'result':row} for row in rows if row['candidate_interface_pass']),None)
    if cf_witness is None:
        cf_witness=next(({'t':t,'result':row} for row in cf if row['candidate_interface_pass']),None)
    for path,row,control in zip(paths,rows,cf):
        stop=path[-1]['end'] if path else 21
        owns=[list(range(w['start']+1,w['end']+1)) for w in path]
        flat=[e for group in owns for e in group]
        assert len(flat)==len(set(flat))
        if t>=21:
            all_owned=[1]+list(range(2,22))+flat+list(range(stop+1,t+1))
            assert sorted(all_owned)==list(range(1,t+1)) and len(all_owned)==len(set(all_owned))
        row['ownership']={'E1':'leading-observation','raw_prelude':[2,min(t,21)] if t>=2 else [],
            'factors':[{ 'id':x['id'],'own':x['own']} for x in path],
            'active_remainder':[stop+1,t] if t>stop else [],
            'coverage_once':t>=21,'confirmed_original_factors':[]}
        control['ownership']=row['ownership']
    trace.append({'t':t,'raw_known':[s['id'] for s in segments if s['known_at']<=t],
        'counts':{'available_grammar_windows':len(available),
           'structural_pass':sum(w['structural_pass'] for w in available),
           'roles_pass_after_structure':sum(w['structural_pass'] and w['role_pass'] for w in available),
           'whole_extreme_after_structure_roles':sum(w['structural_pass'] and w['role_pass'] and w['checks']['whole_extreme'] for w in available),
           'candidate_completion_edges':len(edges),'maximal_delta_paths':len(paths),
           'no_pp_paths':sum(bool(row['delta']) and row['no_pp'] for row in rows),
           'three_child_paths':sum(row['has_three'] for row in rows),
           'candidate_f2_pass':sum(row['candidate_interface_pass'] for row in rows),'original_f2_pass':0},
        'deltas':rows,'no_pp_removed_control':cf})
save('prefix-results.json',trace)
save('selected-prefixes.json',[trace[t] for t in [38,118,138,218,238,318,338,418,438,442]])
rejection_counts=collections.Counter(r for w in windows for r in w['rejections'])
summary={'history':source['name'],'observations':443,'events':442,'raw_segments':21,
    'candidate_count':1,'unexecuted_superseded_drafts':1,'new_histories':0,'parser_reruns':0,
    'grammar_windows':len(windows),'window_lengths':dict(collections.Counter(4*w['k']+1 for w in windows)),
    'rejection_counts_over_all_windows':dict(rejection_counts),'final_counts':trace[-1]['counts'],
    'candidate_edges':[w['id'] for w in windows if w['JEnd68']],
    'first_candidate_F2_witness':witness,'first_NoPP_removed_F2_witness':cf_witness,
    'earliest_nonempty_delta':next((r['t'] for r in trace if any(q['delta'] for q in r['deltas'])),None),
    'earliest_multiple_maximal_delta':next((r['t'] for r in trace if len(r['deltas'])>1),None),
    'source_semantic_admission':False, 'GeneralDiv_original':'unknown',
    'scope':'all 40 windows and all candidate-edge partitions of this fixed grammar from S1; not all F1 models'}
save('result.json',summary)
source_rows=[]
for rel,ranges in SOURCE_RANGES.items():
    p=R/rel; lines=p.read_text().splitlines(); body=next((i+1 for i,l in enumerate(lines) if '↑正文' in l),None)
    row={'path':str(p),'sha256':sha(p),'body_end':body,'lines':[]}
    for lo,hi in ranges:
        for n in range(lo,hi+1):
            line=lines[n-1]
            row['lines'].append({'line':n,'text':line,'before_body_end':None if body is None else n<body,
                'editor_note_flag':any(x in line for x in ['(娇','（娇','(注：','（注：']),
                'adoption':'author prose only; flagged editor text is excluded from Report arguments'})
    source_rows.append(row)
# Read cached original comments directly, not the inherited excerpt as sole evidence.
cached_tickets=[]
for p in ticket_paths:
    x=json.loads(p.read_text())
    cache_comments=x.get('comments',[])
    for want in [t for t in tickets if Path(t['cache_path'])==p]:
        match=[c for c in cache_comments if c.get('url')==want['url']]
        assert len(match)==1 and match[0]['body']==want['body']
        cached_tickets.append({'cache_path':str(p),'sha256':sha(p),'issue':want['issue'],
            'live':False,'url':want['url'],'comment':match[0]})
save('sources.json',{'direct_sources':source_rows,'cached_original_comments':cached_tickets,
    'cautions':['original cached comments contain superseded dimensionless claims; current adopted beichi clauses prevail',
    'source provenance is repository canonical text, no new web historical authentication',
    'RootArm68 and Outer0=core are new candidate choices, not quotations']})
after=hashes(inputs); assert before==after
save('inputhash-after.json',after)
rss=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
if platform.system()!='Darwin':rss*=1024
assert rss<LIMIT
save('receipt.json',{'command':'python3 check_joint.py','cwd':str(HERE),'started_at':start_utc,
    'elapsed_seconds':time.monotonic()-started,'max_rss_bytes':rss,'rss_limit_bytes':LIMIT,
    'input_hashes_unchanged':True,'input_count':len(inputs),'exit_code':0,
    'full_source_and_reference_arrays_loaded':True,'claim_physical_future_read_isolation':False,
    'candidate_dependency':'an edge becomes available only after all constituent raw known_at; its price/pen support ends no later than known_at',
    'new_market_data':0,'rw_parser_executed':False,'production_F2_executed':False})
print(json.dumps(summary,ensure_ascii=False,indent=2))
