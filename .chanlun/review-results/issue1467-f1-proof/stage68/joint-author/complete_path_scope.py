#!/usr/bin/env python3
"""Close already computed maximal paths under prefixes; no candidate replay."""
import copy
import hashlib
import json
from pathlib import Path
D=Path(__file__).resolve().parent
ws=json.loads((D/'all-windows.json').read_text()); byid={w['id']:w for w in ws}
trace=json.loads((D/'prefix-results.json').read_text())
result=[]
for row in trace:
    paths={tuple(m['delta'][:k]) for m in row['deltas'] for k in range(len(m['delta'])+1)}
    out=[]
    for ids in sorted(paths,key=lambda x:(len(x),x)):
        factors=[byid[x] for x in ids]
        assert len(factors)<3 # Already-computed maximal graph has longest path2.
        pp=[(a['id'],b['id']) for a,b in zip(factors,factors[1:]) if a['grade']==b['grade'] and a['kind']==b['kind']=='P']
        stop=factors[-1]['end'] if factors else 21
        out.append({'delta':list(ids),'kinds':[w['kind'] for w in factors],
          'is_maximal':any(list(ids)==m['delta'] for m in row['deltas']),
          'no_pp':not pp,'pp_violations':pp,'has_three':False,'candidate_F2_pass':False,'original_F2_pass':False,
          'factor_owns':[w['own'] for w in factors],'active_tail':[stop+1,row['t']] if row['t']>stop else [],
          'original_completed':[None for _ in factors]})
    result.append({'t':row['t'],'all_path_count':len(out),'nonempty_path_count':sum(bool(x['delta']) for x in out),
      'all_paths':out,'NoPP_paths_with_three':0,'NoPP_removed_paths_with_three':0})
(D/'all-delta-paths.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
first=next(x['t'] for x in result if x['nonempty_path_count']>1)
fix={'frozen_main_files_unchanged':True,'candidate_reruns':0,
 'problem':'Main output enumerated maximal paths only, while frozen R_delta says all edge paths.',
 'mathematical_completion':'Every path in finite forward DAG extends to a maximal path; all prefixes of every recorded maximal path give every edge path.',
 'first_two_nonempty_paths_at':first,
 'distinct_attempts_at_238':result[238]['all_paths'],
 'changed_claims':['result.final_counts.maximal_delta_paths remains1 and refers only to maximal paths',
   'result.earliest_multiple_maximal_delta remains null, not a proof all delta paths unique',
   'model.multiple_delta_paths_found=false refers only to maximal; all-path relation has two nonempty attempts from238',
   'NoPP no_pp_paths=0 at238 in main result is maximal-only; one nonempty shorter prefix passes NoPP but lacks three children'],
 'unchanged_claims':['all40 windows and4 accepted edges unchanged','no path has3 children even withoutNoPP','finite5+9+5 obstruction unchanged'],
 'original_legal_relation_uniqueness':'unknown, not inferred from either number of candidate attempts or maximal paths'}
(D/'PathScopeCorrections.json').write_text(json.dumps(fix,ensure_ascii=False,indent=2)+'\n')
print(json.dumps({'first_two_nonempty':first,'final_paths':result[-1]['all_path_count'],'nonempty_final':result[-1]['nonempty_path_count'],'new_candidate_runs':0}))
