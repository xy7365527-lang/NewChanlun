#!/usr/bin/env python3
"""Seal and validate this evidence package; does not compute candidate windows."""
import ast
import datetime
import hashlib
import json
import re
import sys
from pathlib import Path
D=Path(__file__).resolve().parent
LIMIT=96*1024*1024
def sha(p):
    with p.open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def load(p):return json.loads(p.read_text())
def save(name,obj):
    p=D/name
    if p.exists():raise FileExistsError(p)
    p.write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')
for p in D.glob('*.py'):
    compile(p.read_text(),str(p),'exec');ast.parse(p.read_text())
inputs=load(D/'run/inputhash-before.json')
assert inputs==load(D/'run/inputhash-after.json')
checks=[]
for item in inputs:
    p=Path(item['path']);actual=sha(p)
    assert actual==item['sha256'] and p.stat().st_size==item['bytes']
    checks.append({'path':str(p),'sha256':actual,'bytes':p.stat().st_size})
save('inputhash-final.json',checks)
links=[]
for md in ['Report.md','WorkCard.md']:
    for target in re.findall(r'\]\(([^)]+)\)',(D/md).read_text()):
        if target.startswith(('http:','https:','#')):continue
        p=(D/target).resolve();assert p.exists(),str(p)
        links.append({'from':md,'target':target})
result=load(D/'run/result.json');trace=load(D/'run/prefix-results.json');windows=load(D/'run/all-windows.json')
assert len(trace)==583 and len(windows)==78
assert all(x['counts']['all_paths']==1 and x['counts']['max_factor_count']==0 for x in trace)
assert len([w for w in windows if w['JEnd68']])==5
assert len([w for w in windows if w['raw_start']==1])==6
assert not any(w['JEnd68'] for w in windows if w['raw_start']==1)
assert load(D/'internal-chain.json')['fixed_F2_necessary_conditions']['candidate_interface_pass']
assert not load(D/'internal-chain.json')['rooted_F2_admission']
assert sha(D/'WorkCard.md')==load(D/'WorkCard.freeze.json')['sha256']
assert load(D/'run/receipt.json')['max_rss_bytes']<=LIMIT
save('command-receipt.json',{'main_command':['python3',str(D/'check_joint.py'),'--out',str(D/'run')],
 'redirected_stdout':str(D/'run.stdout'),'redirected_stderr':str(D/'run.stderr'),
 'cwd':load(D/'run/receipt.json')['cwd'],'exit_code':0,'tool_chunk_id':'2dd199',
 'scope':'author record of the tool invocation; true process cwd and argv are independently recorded inside run/receipt.json',
 'main_runs':1,'stage68_equivalence_control_executions':1,'candidate_window_reruns_after_main':0,
 'preparation_read_stalls':'read-only calls eventually completed; no input mutation or system repair',
 'assembly':{'script':'assemble.py','exit_code':0,'candidate_window_evaluations':0}})
save('seal-receipt.json',{'cwd':str(Path.cwd()),'argv':sys.argv,'sealed_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'input_files_unchanged':len(inputs),'python_static_compile':'pass','local_links_checked':links,
 'run_counts_checked_from_sealed_outputs':True,'candidate_window_evaluations':0,'R_W_runs':0,
 'main_script_sha256':sha(D/'check_joint.py'),'python_review':'finite-domain Approve; not mathematics review',
 'independent_mathematics_review':'pending','exit_code':0})
files=[p for p in sorted(D.rglob('*')) if p.is_file() and p.name not in ['manifest.json','FINAL.json']]
entries=[{'path':str(p.relative_to(D)),'sha256':sha(p),'bytes':p.stat().st_size} for p in files]
total=sum(x['bytes'] for x in entries);assert total<=LIMIT
save('manifest.json',{'candidate':'Omega68-JointCut-v1 on existing H2','created_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),
 'entries':entries,'artifact_bytes_before_manifest':total,'output_root':str(D),'main_runs':1})
save('FINAL.json',{'status':'author_complete_finite_root_obstruction_independent_math_review_pending',
 'report':'Report.md','report_sha256':sha(D/'Report.md'),'manifest_sha256':sha(D/'manifest.json'),
 'workcard_sha256':sha(D/'WorkCard.md'),'main_script_sha256':sha(D/'check_joint.py'),
 'history':'existing Stage64 H2','observations':583,'events':582,'pens':144,'raws':28,
 'grammar_windows':78,'filtered_counts':[78,23,7,7,5],
 'root':'S1 after fixed S0 prelude','rooted_nonempty_paths':0,'rooted_three_child_paths':0,
 'internal_numerical_triple':{'path':[4,9,18,27],'types':['P','D','U'],'core':[50000,80000],
 'known_at':558,'rooted_admission':False,'missing_root_events':[22,81],'original_admission':False},
 'counterfactual_remove_NoPP_rooted_max_factors':0,'all_prefixes_checked':583,
 'stage68_control':{'windows_full_field_equal':40,'all_path_and_classes_prefixes_equal':443},
 'new_histories':0,'R_W_runs':0,'main_candidate_runs':1,'source_semantic_qualification':'unproved',
 'P1_P4_complete':False,'independent_math_review':'pending','python_code_review':'finite-domain Approve',
 'max_rss_bytes':load(D/'run/receipt.json')['max_rss_bytes'],'artifact_bytes_before_manifest':total,
 'input_hashes_unchanged':True,'old_sealed_evidence_preserved':True,
 'no_changes_to':['other authors','old frozen evidence','repository','production','formal','doctrine','tracker']})
print(json.dumps({'files':len(entries),'bytes':total,'manifest_sha256':sha(D/'manifest.json'),'report_sha256':sha(D/'Report.md'),'FINAL_sha256':sha(D/'FINAL.json')},ensure_ascii=False))
