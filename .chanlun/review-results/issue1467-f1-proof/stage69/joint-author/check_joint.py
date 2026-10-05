#!/usr/bin/env python3
"""Same frozen JointCut predicates; complete path relation; never runs R_W."""
from __future__ import annotations
import argparse
import ast
import collections
import datetime
import hashlib
import itertools
import json
import platform
import resource
import sys
import time
from pathlib import Path
HERE = Path(__file__).resolve().parent
E = Path('/Users/silencehan/Documents/Codex/research-evidence/issue1467')
R = Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
P = R / '.chanlun/review-results/issue1467-f1-proof'
LIMIT = 96*1024*1024

def sha(p):
    with p.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def load(p):
    return json.loads(p.read_text())

def save(d,name,obj):
    (d/name).write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')

def hashes(paths):
    return [{'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size} for p in paths]

def force(seg, pens):
    a,z=seg['members']; first,last=pens[a],pens[z-1]
    fv=(first['end_price']-first['start_price'])/(first['end']-first['start'])
    lv=(last['end_price']-last['start_price'])/(last['end']-last['start'])
    return {'first_pen':a,'last_pen':z-1,'first_v':fv,'last_v':lv,'L':lv-fv}

def meet(ranges):
    return [max(x[0] for x in ranges), min(x[1] for x in ranges)]

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

def enumerate_windows(obs,segments):
    windows=[]
    for s in range(1,len(segments)):
        k=1
        while s+4*k<len(segments):
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
    return windows

def paths_from(start, edges):
    """All rooted edge paths, including empty, with no selection or tie-break."""
    bystart=collections.defaultdict(list)
    for edge in edges:
        bystart[edge['raw_start']].append(edge)
    def visit(i,path):
        yield path
        for edge in bystart[i]:
            assert edge['raw_stop']>i
            yield from visit(edge['raw_stop'],path+[edge])
    return sorted(visit(start,[]),key=lambda path:(len(path),tuple(x['id'] for x in path)))


def path_outputs(t,windows,root_end=21):
    available=[w for w in windows if w['candidate_known']<=t]
    edges=[w for w in available if w['JEnd68']]
    paths=paths_from(1,edges)
    allrows=[]; classes=[]; consumers=[]
    for path in paths:
        checked=f2_contract(path,False)
        control=f2_contract(path,True)
        stop=path[-1]['end'] if path else root_end
        raw_stop=path[-1]['raw_stop'] if path else 1
        extendable=any(w['raw_start']==raw_stop for w in edges)
        row={'delta':[w['id'] for w in path],'kinds':[w['kind'] for w in path],
             'is_maximal':not extendable,'no_pp':checked['no_pp'],
             'pp_violations':[list(x) for x in checked['pp_violations']],
             'has_three':len(path)>=3,'candidate_F2_pass':checked['candidate_interface_pass'],
             'original_F2_pass':False,'factor_owns':[w['own'] for w in path],
             'active_tail':[stop+1,t] if t>stop else [],'original_completed':[None for w in path]}
        allrows.append(row)
        cls=dict(row,extendable_now=extendable,
                 candidate_windows_inside_tail=[w['id'] for w in edges if w['start']>=stop],
                 complete_original_decomposition=None,
                 source_partition_only='raw root + factor intervals + unresolved tail; coverage is not source-certified completeness')
        classes.append(cls)
        flat=[i for w in path for i in range(w['start']+1,w['end']+1)]
        assert len(flat)==len(set(flat))
        if t>=root_end:
            ledger=[1]+list(range(2,root_end+1))+flat+list(range(stop+1,t+1))
            assert sorted(ledger)==list(range(1,t+1)) and len(ledger)==len(set(ledger))
        consumers.append({'delta':row['delta'],'fixed_F2_necessary_conditions':checked,
                          'NoPP_removed_same_graph':control,
                          'candidate_clock':[{'id':w['id'],'happened':w['candidate_happened'],
                            'known':w['candidate_known']} for w in path],
                          'ledger_coverage_once':t>=root_end,
                          'original_F2_status':'unproved, false flag means not admitted, not disproof'})
    allout={'t':t,'all_path_count':len(allrows),'nonempty_path_count':sum(bool(x['delta']) for x in allrows),
            'all_paths':allrows,'NoPP_paths_with_three':sum(x['has_three'] and x['no_pp'] for x in allrows),
            'NoPP_removed_paths_with_three':sum(x['has_three'] for x in allrows)}
    classout={'t':t,'all_edge_paths':classes,
              'no_pp_paths_including_empty':[x['delta'] for x in classes if x['no_pp']],
              'no_pp_nonempty_paths':[x['delta'] for x in classes if x['no_pp'] and x['delta']],
              'three_child_paths':[x['delta'] for x in classes if x['has_three']],
              'no_pp_and_three_child_paths':[x['delta'] for x in classes if x['has_three'] and x['no_pp']],
              'counts':{'all':len(classes),'nonempty':sum(bool(x['delta']) for x in classes),
                        'no_pp_nonempty':sum(x['no_pp'] and bool(x['delta']) for x in classes),
                        'three_children':sum(x['has_three'] for x in classes),
                        'NoPP_and_three':sum(x['has_three'] and x['no_pp'] for x in classes)}}
    counts={'available_grammar_windows':len(available),'structural_pass':sum(w['structural_pass'] for w in available),
            'roles_pass_after_structure':sum(w['structural_pass'] and w['role_pass'] for w in available),
            'whole_extreme_after_structure_roles':sum(w['structural_pass'] and w['role_pass'] and w['checks']['whole_extreme'] for w in available),
            'candidate_completion_edges':len(edges),'all_paths':len(classes),
            'maximal_paths':sum(x['is_maximal'] for x in classes),
            'maximal_nonempty_paths':sum(x['is_maximal'] and bool(x['delta']) for x in classes),
            'maximal_no_pp_nonempty_paths':sum(x['is_maximal'] and x['no_pp'] and bool(x['delta']) for x in classes),
            'no_pp_nonempty_paths':classout['counts']['no_pp_nonempty'],
            'three_child_paths':classout['counts']['three_children'],
            'candidate_F2_pass':sum(x['candidate_F2_pass'] for x in classes),
            'NoPP_removed_candidate_F2_pass':sum(x['NoPP_removed_same_graph']['candidate_interface_pass'] for x in consumers),
            'max_factor_count':max(map(len,paths)), 'original_F2_admitted':0}
    return allout,classout,{'t':t,'counts':counts,'consumer_checks':consumers}


def rule_binding():
    old=E/'stage68/joint-author/check_joint.py'
    original=old.read_text();new=Path(__file__).read_text()
    o={n.name:ast.get_source_segment(original,n) for n in ast.parse(original).body if isinstance(n,ast.FunctionDef)}
    n={v.name:ast.get_source_segment(new,v) for v in ast.parse(new).body if isinstance(v,ast.FunctionDef)}
    exact={name:o[name]==n[name] for name in ['force','meet','f2_contract']}
    assert all(exact.values())
    block=original[original.index('windows=[]\n'):original.index('assert len(windows)==40')]
    expect=block.replace('range(1,21)','range(1,len(segments))').replace('while s+4*k<21:','while s+4*k<len(segments):')
    import textwrap
    actual=textwrap.dedent(n['enumerate_windows'].split('\n',1)[1]).removesuffix('return windows')
    assert actual==expect
    return {'old_script':str(old),'old_sha256':sha(old),'new_script':str(Path(__file__).resolve()),
            'new_sha256':sha(Path(__file__).resolve()),'functions_verbatim_equal':exact,
            'window_block_equal_after_only_upper_bound_substitutions':True,
            'substitutions':{'range(1,21)':'range(1,len(segments))','while s+4*k<21:':'while s+4*k<len(segments):'},
            'semantic_changes':[],
            'implementation_repairs':['direct all paths including empty; derived maximal/NoPP/three classes',
                                      'Path.cwd and sys.argv recorded','main guard and exclusive output directory'],
            'identifiers':'W68 and IC68 retain original rule-family names also on H2; not old-window identity'}


def stage68_control(out):
    old=E/'stage68/joint-author'
    obs=load(E/'stage66/nested-author/run/source.json')['observations']
    raws=load(old/'raw-roots.json')
    pens=load(E/'stage66/nested-review/independent-pens.json')
    assert all(force(raw,pens)==raw['force'] for raw in raws)
    actual=enumerate_windows(obs,raws)
    expected=load(old/'all-windows.json')
    assert actual==expected and len(actual)==40
    oldpaths=load(old/'all-delta-paths.json');oldclasses=load(old/'path-classes.json')
    for t in range(443):
        paths,classes,_=path_outputs(t,actual)
        assert paths==oldpaths[t],('all_paths',t)
        assert classes==oldclasses[t],('path_classes',t)
    result={'all_pass':True,'windows_full_field_equality':40,'raw_force_equality':21,
            'all_path_full_field_equality_prefixes':443,'path_classes_full_field_equality_prefixes':443,
            'maximal_count_at_final':sum(x['is_maximal'] for x in oldclasses[-1]['all_edge_paths']),
            'candidate_R_W_executions':0,'original_script_imported_or_executed':False,
            'scope':'known Stage68 sealed outputs and function/field equivalence, not new history'}
    save(out,'stage68-control.json',result)
    return result


def budget_paths(windows,raw_budget):
    bycoords={(w['raw_start'],w['raw_stop']):w for w in windows}
    plans=[]
    for n in range(3,raw_budget//5+1):
        for ks in itertools.product(range(1,raw_budget//4+1),repeat=n):
            lengths=[4*k+1 for k in ks]
            if sum(lengths)>raw_budget:continue
            if any(a==b==1 for a,b in zip(ks,ks[1:])):continue
            start=1;factors=[]
            for length in lengths:
                factors.append(bycoords[start,start+length]);start+=length
            plans.append({'ks':ks,'lengths':lengths,'raw_used':sum(lengths),
                          'windows':[w['id'] for w in factors],
                          'all_JEnd68':all(w['JEnd68'] for w in factors),
                          'factor_failures':[{'id':w['id'],'rejections':w['rejections'],
                            'kind':w['kind'],'own':w['own'],'whole':w['whole'],
                            'cores':[c['core'] for c in w['cores']],
                            'b_ref':w['b_ref'],'c':w['c'],'L_b':w['b_force']['L'],'L_c':w['c_force']['L'],
                            'prior_extreme':w['prior_whole_extreme'],'end_price':w['end_price'],
                            'happened':w['candidate_happened'],'known':w['candidate_known']} for w in factors]})
    return {'raw_budget':raw_budget,'no_pp_length_bound':'5*n+4*floor(n/2)',
            'four_factors_minimum':28,'plans_with_at_least_three_and_structural_NoPP':plans,
            'scope':'length admissibility only; candidate type and all dynamic checks are still required'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out',type=Path,required=True,help='new nonexisting output directory under stage69/joint-author')
    args=parser.parse_args();out=args.out.resolve()
    assert out.is_relative_to(HERE) and out!=HERE
    out.mkdir(parents=False,exist_ok=False)
    started=time.monotonic();utc=datetime.datetime.now(datetime.timezone.utc).isoformat()
    base=E/'stage64/dynamic-author/v2/run';review=E/'stage64/dynamic-review';old=E/'stage68/joint-author'
    context=[P/'Stage68PenTransitionsAndJointCuts.md',P/'Stage64DynamicCandidatesAndCausalFrontier.md',
             E/'stage65/lifecycle-author/Report.md',E/'stage65/lifecycle-review/review.md',
             E/'stage64/dynamic-author/Report.md',review/'review.md',
             E/'stage68/joint-review/review.md',old/'WorkCard-v2.md',old/'ScopeCorrections.md',
             old/'PathScopeCorrections.json',old/'InvocationCorrections.json',old/'manifest.json',old/'FINAL.json']
    inputs=list(dict.fromkeys([base/'source.json',base/'segments.json',base/'reference.stdout',base/'manifest.json',
            review/'input-hashes.json',review/'H2-prefix-audit.json',HERE/'WorkCard.md',HERE/'WorkCard.freeze.json',
            Path(__file__).resolve(),HERE/'original-rule-extract.py',HERE/'rule-extraction.diff',
            old/'check_joint.py',old/'all-windows.json',old/'all-delta-paths.json',old/'path-classes.json',old/'raw-roots.json',
            E/'stage66/nested-author/run/source.json',E/'stage66/nested-review/independent-pens.json']+context))
    before=hashes(inputs);save(out,'inputhash-before.json',before)
    assert sha(HERE/'WorkCard.md')==load(HERE/'WorkCard.freeze.json')['sha256']
    save(out,'rule-binding.json',rule_binding())
    author_hash={x['path']:x['sha256'] for x in load(base/'manifest.json')['artifacts']}
    review_hash={x['path']:x['sha256'] for x in load(review/'input-hashes.json')['files']}
    bindings=[]
    for name in ['source.json','segments.json','reference.stdout']:
        got=sha(base/name)
        assert got==author_hash[name]==review_hash['v2/run/'+name]
        bindings.append({'path':str(base/name),'sha256':got,'author_manifest_match':True,'review_capture_match':True})
    save(out,'h2-input-binding.json',bindings)
    stage68_control(out)
    source=load(base/'source.json');obs=source['observations'];raws=load(base/'segments.json');reference=load(base/'reference.stdout')
    pens=reference['strokes'];prefixes=reference['prefixes'];audit=load(review/'H2-prefix-audit.json')
    assert source['name']=='T64-H2-dynamic-repair'
    assert len(obs)==583 and len(source['events'])==582 and len(pens)==144 and len(raws)==28
    assert len(obs)<=600 and len(raws)<=32 and len(prefixes)==583
    assert raws==audit['certificates'] and audit['prefix_count']==583
    assert all(o['index']==o['time']==t for t,o in enumerate(obs))
    stable_checks=0
    for i,s in enumerate(raws):
        assert s['id']==f'S{i}'
        if i:assert raws[i-1]['end']==s['start']
        a,z=s['members'];memberpens=pens[a:z]
        assert memberpens[0]['start']==s['start'] and memberpens[-1]['end']==s['end']
        assert s['range']==[min(x['price'] for x in obs[s['start']:s['end']+1]),max(x['price'] for x in obs[s['start']:s['end']+1])]
        for t in range(s['known_at'],len(obs)):
            assert prefixes[t]['stable'][a:z]==memberpens
            stable_checks+=1
        s['force']=force(s,pens);s['own']=[s['start']+1,s['end']]
        s['start_price']=obs[s['start']]['price'];s['end_price']=obs[s['end']]['price']
        s['RawClosed']=True;s['ComparisonReady68']=True;s['ComparisonReady_original']=None;s['MovementCompleted_original']=None
    save(out,'raw-roots.json',raws)
    # Exactly one new-history evaluation; the old Stage68 equality control precedes this call.
    windows=enumerate_windows(obs,raws)
    assert len(windows)==78
    save(out,'all-windows.json',windows)
    allpaths=[];classes=[];trace=[]
    for t in range(len(obs)):
        a,c,p=path_outputs(t,windows,raws[0]['end']);allpaths.append(a);classes.append(c);trace.append(p)
        p['raw_known']=[s['id'] for s in raws if s['known_at']<=t]
        assert p['raw_known']==audit['certificate_ids_by_prefix'][t]
        for w in windows:
            if w['candidate_known']<=t:
                assert w['end']<=w['candidate_known']<=t
                assert all(raws[j]['known_at']<=t for j in range(w['raw_start'],w['raw_stop']))
    save(out,'all-delta-paths.json',allpaths);save(out,'path-classes.json',classes);save(out,'prefix-results.json',trace)
    save(out,'budget-obstruction.json',budget_paths(windows,len(raws)-1))
    accepted=[w for w in windows if w['JEnd68']]
    stages=[('structural',lambda w:w['structural_pass']),('roles',lambda w:w['role_pass']),
            ('whole_extreme',lambda w:w['checks']['whole_extreme']),('signed_force',lambda w:w['checks']['signed_force_weakening'])]
    pool=windows;layers=[]
    for name,predicate in stages:
        reject=[w['id'] for w in pool if not predicate(w)];pool=[w for w in pool if predicate(w)]
        layers.append({'stage':name,'rejected_here':reject,'remaining_count':len(pool)})
    root_attempts=[w for w in windows if w['raw_start']==1]
    save(out,'graph.json',{'root_raw_index':1,'raw_prelude':'S0','root_end':raws[0]['end'],
          'edges':accepted,'root_attempts':root_attempts,'all_nodes':list(range(1,len(raws)+1)),
          'maximal_final_paths':[x for x in classes[-1]['all_edge_paths'] if x['is_maximal']],
          'original_Owner_Next':'unproved','LocalDyn64_or_A64_global_imported':False})
    save(out,'rejection-layers.json',{'layers':layers,'overlapping_reason_counts':dict(collections.Counter(r for w in windows for r in w['rejections']))})
    keytimes=sorted(set([0,21,38,58,118,138,198,218,238,278,318,338,378,438,458,478,538,558,578,582]+[w['candidate_known'] for w in accepted]))
    save(out,'selected-prefixes.json',[{'trace':trace[t],'paths':classes[t]} for t in keytimes])
    first=lambda key:next((x['t'] for x in trace if x['counts'][key]),None)
    summary={'history':source['name'],'new_histories':0,'R_W_runs':0,'main_candidate_runs':1,
          'observations':len(obs),'events':len(source['events']),'pens':len(pens),'raw_segments':len(raws),
          'rooted_input_segments':len(raws)-1,'root_raw_start':1,'candidate_rule':'Omega68-JointCut-v1 unchanged',
          'grammar_windows':len(windows),'window_lengths':dict(collections.Counter(4*w['k']+1 for w in windows)),
          'final_counts':trace[-1]['counts'],'candidate_edges':[w['id'] for w in accepted],
          'first_nonempty':first('maximal_nonempty_paths'),'first_three_children':first('three_child_paths'),
          'first_candidate_F2':first('candidate_F2_pass'),'first_NoPP_removed_F2':first('NoPP_removed_candidate_F2_pass'),
          'stable_input_identity_checks':stable_checks,'source_semantics':'unproved; candidate nonadmission does not invalidate H2',
          'general_F1_or_R_W_disproved':False,'physical_read_isolation_claimed':False}
    save(out,'result.json',summary)
    after=hashes(inputs);assert before==after;save(out,'inputhash-after.json',after)
    rss=resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    if platform.system()!='Darwin':rss*=1024
    assert rss<=LIMIT
    total=sum(p.stat().st_size for p in HERE.rglob('*') if p.is_file())
    assert total<=LIMIT
    save(out,'receipt.json',{'cwd':str(Path.cwd()),'argv':sys.argv,'python_executable':sys.executable,
            'started_at':utc,'elapsed_seconds':time.monotonic()-started,'max_rss_bytes':rss,
            'rss_limit_bytes':LIMIT,'artifact_bytes_at_receipt':total,'input_hashes_unchanged':True,
            'exit_code':0,'Stage68_field_equivalence_control':True,'H2_main_runs':1,
            'full_reference_arrays_loaded':True,'physical_future_read_isolation':False,
            'dependency_scope':'raw known_at gates, known-time stable input identities, price support ends at candidate end; no raw parser rerun'})
    print(json.dumps(summary,ensure_ascii=False,indent=2))


if __name__=='__main__':
    main()
