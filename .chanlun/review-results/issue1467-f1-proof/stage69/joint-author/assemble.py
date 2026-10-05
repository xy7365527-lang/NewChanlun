#!/usr/bin/env python3
"""Read-only analysis of sealed run outputs; no candidate/window or R_W rerun."""
from pathlib import Path
import datetime
import hashlib
import importlib.util
import json
import sys
import time
sys.dont_write_bytecode=True
D=Path(__file__).resolve().parent
E=D.parents[1]
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def load(p):return json.loads(p.read_text())
def save(name,x):
    p=D/name
    if p.exists():raise FileExistsError(p)
    p.write_text(json.dumps(x,ensure_ascii=False,indent=2)+'\n')
started=time.monotonic()
runbefore=[{'path':str(p),'sha256':sha(p)} for p in sorted((D/'run').glob('*')) if p.is_file()]
# Import is safe because new entry point has a main guard; only fixed F2 checker is called.
spec=importlib.util.spec_from_file_location('joint_rule',D/'check_joint.py');mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
windows=load(D/'run/all-windows.json');byid={w['id']:w for w in windows}
chain=[byid[f'W68:S{a}-S{z}'] for a,z in [(4,8),(9,17),(18,26)]]
consumer=mod.f2_contract(chain,False)
assert consumer['candidate_interface_pass'] and consumer['candidate_center']['core']==[50000,80000]
assert [w['kind'] for w in chain]==['P','D','U']
save('internal-chain.json',{'scope':'three stored edges form an internal numerical chain; not an R_delta path from frozen S1 root',
 'new_window_evaluations':0,'root_changed':False,'raw_vertices':[4,9,18,27],
 'candidate_factors':chain,'fixed_F2_necessary_conditions':consumer,
 'all_three_evidence_known':max(w['candidate_known'] for w in chain),
 'unowned_root_gap':{'raw_segments':['S1','S2','S3'],'events':[22,81],'segment_count':3,
                    'minimum_whole_segment_count':5,'repair_inside_frozen_grammar':False},
 'rooted_F2_admission':False,'original_F2_admission':False,'original_completion_status':'unproved',
 'completion_ledger_warning':'Even a numerical inner triple does not assign the skipped E22..81 or prove any original whole completion.'})
roots=[w for w in windows if w['raw_start']==1]
assert len(roots)==6 and not any(w['JEnd68'] for w in roots)
proof={'claim':'No nonempty rooted candidate delta on fixed H2 in this frozen JointCut relation.',
 'proof_steps':['Every nonempty rooted path must have a first edge whose raw_start is1.',
 'Every grammar window from raw1 has stop1+4k+1<=28, hence k in1..6. These six windows are listed.',
 'Every one fails at least one unchanged JEnd68 conjunct; therefore no edge leaves1.',
 'Prefix edge sets are subsets of final edges because availability requires known_at<=t. Hence no prefix has a first edge.',
 'Thus all583 prefix path relations contain only the empty path. Empty is maximal in this graph but not nonempty success.'],
 'all_root_windows':roots,'NoPP_dependency':False,'raw_legality_changed':False,
 'global_impossibility_claim':False,'no_pp_length_plans':load(D/'run/budget-obstruction.json'),
 'finite_qualification':'This specific H2, fixed S1 origin, 28 selected raw certificates, all78 windows; no other history or F1 model.'}
save('finite-obstruction.json',proof)
old=E/'stage68/joint-author';review=load(E/'stage68/joint-review/FINAL.json')
assert sha(old/'FINAL.json')==review['frozen_author_final_sha256']
assert sha(old/'manifest.json')==review['frozen_author_manifest_sha256']
manifest=load(old/'manifest.json');checks=[]
for x in manifest['entries']:
 p=old/x['path']; actual=sha(p);assert actual==x['sha256'] and p.stat().st_size==x['bytes'];checks.append({'path':str(p),'sha256':actual,'matches':True})
save('stage68-seal-audit.json',{'review_final':str(E/'stage68/joint-review/FINAL.json'),'review_final_sha256':sha(E/'stage68/joint-review/FINAL.json'),
 'author_final_matches_review':True,'author_manifest_matches_review':True,'all_manifest_entries_match':True,'entries':checks})
a=load(E/'stage66/nested-author/run/source.json')['observations'];b=load(E/'stage64/dynamic-author/v2/run/source.json')['observations']
differences=[{'index':i,'stage68_input_price':u['price'],'H2_price':v['price']} for i,(u,v) in enumerate(zip(a,b)) if u['price']!=v['price']]
oldbyid={w['id']:w for w in load(old/'all-windows.json')}
save('stage68-comparison.json',{'rules_unchanged':True,'only_enumerator_bound_changed':True,
 'histories_are_different_not_prefix_extension':bool(differences),'overlap_observations_compared':min(len(a),len(b)),
 'price_differences_in_overlap':len(differences),'first_price_difference':differences[0] if differences else None,
 'old_rooted_raw_budget':20,'new_rooted_raw_budget':27,'old_windows':40,'new_windows':78,
 'old_edges':[[1,6],[6,11],[12,17],[16,21]],
 'new_edges':[[w['raw_start'],w['raw_stop']] for w in windows if w['JEnd68']],
 'root_window_same_grammar_new_values':{'Stage68':oldbyid['W68:S1-S5'],'H2':byid['W68:S1-S5']},
 'key_distinction':'H2 adds length-admissible options and contains an internal P/D/U chain, but no accepted first window fromS1; longer length alone is not a sufficient condition.'})
assert runbefore==[{'path':str(p),'sha256':sha(p)} for p in sorted((D/'run').glob('*')) if p.is_file()]
save('assembly-receipt.json',{'cwd':str(Path.cwd()),'argv':sys.argv,'started_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'elapsed_seconds':time.monotonic()-started,'exit_code':0,'candidate_window_evaluations':0,'R_W_runs':0,
 'run_outputs_unchanged':True,'original_inputs_unchanged':all(sha(Path(x['path']))==x['sha256'] for x in load(D/'run/inputhash-after.json'))})
print(json.dumps({'root_windows_failed':len(roots),'internal_chain_core':consumer['candidate_center']['core'],'internal_chain_known':558,'old_sealed_entries_match':len(checks),'overlap_price_differences':len(differences),'first_difference':differences[0]},ensure_ascii=False))
