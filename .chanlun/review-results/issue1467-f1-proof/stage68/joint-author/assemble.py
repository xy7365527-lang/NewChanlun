#!/usr/bin/env python3
"""Package already computed finite evidence; do not rerun the candidate."""
import hashlib
import json
from pathlib import Path
D=Path(__file__).resolve().parent
R=Path('/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun')
def write(name,obj):
    (D/name).write_text(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')
ws=json.loads((D/'all-windows.json').read_text())
trace=json.loads((D/'prefix-results.json').read_text())
raw=json.loads((D/'raw-roots.json').read_text())
byid={w['id']:w for w in ws}
required=[byid[f'W68:S{a}-S{b}'] for a,b in [(1,5),(6,14),(15,19)]]
proof={
 'statement':'Within frozen JointCut grammar from S1 with at most 20 raw segments, no nonempty F2 input of >=3 completed candidate factors satisfies NoPP.',
 'assumptions':['each factor uses 4*k+1 raw segments, k>=1','k=1 iff candidate kind P',
   'factors are adjacent nonempty disjoint whole intervals starting at S1',
   'at most 20 raw segments S1..S20','NoPP on independent same-grade factors',
   'JEnd68 requires whole Extreme and signed force weakening'],
 'length_proof':['n>=5 requires >=25 segments, impossible',
   'n=4 all-single-core factors are P/P/P/P and violate NoPP; any multi-core factor raises length to >=24, impossible',
   'n=3 gives 4*sum(k)+3<=20, hence sum(k)<=4; sum(k)=3 gives P/P/P',
   'sum(k)=4 gives exactly one k=2 factor; NoPP forces it to the middle',
   'therefore unique length vector is (5,9,5); source-contiguity fixes S1..5 | S6..14 | S15..19'],
 'unique_required_assignment':required,
 'decisive_middle':{
   'id':required[1]['id'],'known_at':318,'candidate_kind':'D','own':[122,301],
   'cores':[[66000,68800],[62000,64000]],'reference_arm':'S10','exit_arm':'S14',
   'L_b':-1200,'L_c':-300,'weakening':False,'prior_whole_min':60000,'endpoint_price':60800,
   'whole_extreme':False,'raw_end':301,
   'assertion':'Both required dynamic conjuncts fail independently. No RootArm or initial Outer admission can reverse these numeric facts without changing this candidate.'},
 'not_claimed':['not all source-valid F1 grammars','not original GeneralDiv=false','not market generality','not every raw cut forbidden'],
 'source_raw_validity_unchanged':True}
assert required[1]['checks']['whole_extreme'] is False
assert required[1]['checks']['signed_force_weakening'] is False
assert [w['raw_stop']-w['raw_start'] for w in required]==[5,9,5]
write('finite-obstruction.json',proof)
relations=[]
for w in ws:
    relations.append({'id':w['id'],'raw_input':w['raws'],'core_ids':[c['id'] for c in w['cores']],
      'Own':w['own'],'grade':0,'structural_type':w['kind'],
      'raw_support_certified':True,'ComparisonReady68_for_this_core':w['role_pass'],
      'CycleDiv68':w['CycleDiv68'],'Completed68':w['JEnd68'],
      'Completed68_means':'explicit candidate interpretation from CycleDiv68 and raw c closure; no source sufficiency proof',
      'GeneralDiv_original':None,'MovementCompleted_original':None,
      'raw_occurrence':w['end'],'raw_evidence_complete':w['candidate_known'],
      'original_occurrence':None,'original_confirmation':None,'original_publication':None,
      'evidence':{'file':'all-windows.json','id':w['id']},'rejections':w['rejections']})
source_matrix=[
 {'id':'084-initial','class':'original text','source':'084:52/54/56','premise':'new f1 defines lowest centers and movements; f2 retained',
  'actual':'one fully explicit finite grammar with no parent-to-child feedback','met':True,'limit':'permission to construct, not semantic sufficiency'},
 {'id':'057-view','class':'original text','source':'057:26/28/30/32','premise':'viewpoint uniformity; restore actual structure for operational roles',
  'actual':'same raw grammar and numeric predicates for every window; raw b not promoted to q0 child','met':True,
  'limit':'source permission of exact RootArm68 role remains unknown; no same-view promotion of b/P0/P1, so earlier triple not silently excluded'},
 {'id':'061-role','class':'original text and project inference','source':'061:26; beichi:265','premise':'same center; nearest prior same-direction external crossing arm',
  'actual':'fixed grammar selects immediately preceding raw arm, checks entry/exit direction, excludes core members','met':True,
  'limit':'geometric candidate role only; no original completed-entry inference'},
 {'id':'061-live','class':'original text','source':'061:26','premise':'live weakness is provisional and can be negated',
  'actual':'no 118-type weak event is latched; candidate ends require complete raw c evidence','met':True,
  'limit':'delayed raw confirmation does not prove whole completion'},
 {'id':'865-arrow','class':'project inference','source':'beichi:313-323; cached #865','premise':'Movement completion needs dynamics, cannot derive from containing a core',
  'actual':'JEnd68 uses force+extreme+roles, not core existence alone; raw c closure is separately typed','met':None,
  'limit':'local CycleDiv68 + raw c closure => whole original completion is unproved'},
 {'id':'873-force','class':'project inference','source':'beichi:335-343; cached #873','premise':'velocity net increment on actual pens, common time axis',
  'actual':'each window uses source pen endpoints and event-duration slopes; same signed inequality throughout','met':True,
  'limit':'finite arithmetic convention, not equivalence to production inclusive-bar formula or proof of every time totality'},
 {'id':'037-trend','class':'original text','source':'037:16-22','premise':'trend c has sublevel/third-point/internal structure qualification',
  'actual':'new k>=2 windows are enumerated but raw c does not have these qualifications','met':None,
  'limit':'no original TrendDiv asserted; selected middle already numerically fails'},
 {'id':'043-whole','class':'original text','source':'043:34/38/48; qushi:50','premise':'local divergence need not end entire enclosing movement',
  'actual':'new whole ownership is explicit, but no source theorem equates JEnd68 to final whole completion','met':None,
  'limit':'four positive candidate edges are not certified original completed movements'},
 {'id':'C9-outer','class':'new initial choice under prior project inference','source':'Stage58 C9; 020:52-58; qushi:124-144','premise':'if Outer0 is original DD/GG it must realize actual Zn extrema',
  'actual':'Outer0=core is explicit new definition, original Zn/DD/GG role not assigned','met':None,
  'limit':'cannot use core separation as already proven original trend separation'},
 {'id':'018-F2','class':'original text','source':'018:24; fixed F2','premise':'three completed same-grade continuous whole ranges',
  'actual':'actual maximal rooted delta has two candidate factors; original completion all unknown','met':False,
  'limit':'finite interface not satisfied, no q1 output'},
 {'id':'827-NoPP','class':'project inference','source':'Stage66 scope review; #827 current policy','premise':'NoPP on actual independent constructive delta factors',
  'actual':'delta explicitly owns intervals and submitted to F2; first two candidate factors P/P at238','met':False,
  'limit':'candidate mapping diagnostic; finite obstruction also covers all released grammar alternatives'},
]
write('model.json',{
 'name':'Omega68-JointCut-v1','classification':'finite explicit joint candidate with rejected fixed-F2 interface',
 'status':'author computation complete; independent review pending',
 'one_candidate':True,'new_histories':0,'input':'Stage66 N66-low-prefix-nested-v1, 443 observations',
 'initial_level':0,'grammar':'b K1 (bridge K)* c, each K is exactly three raw segments; length=4k+1',
 'raw_root':{'id':'S0','identity':'Raw067Segment','raw_rank':-1,'own':[2,21],'end':21,'raw_known':38,
   'RawClosed':True,'force':-1000,'MovementCompleted_original':None,'GeneralDiv_original':None,
   'original_q0_member':False,'q0_member_false_scope':'this explicit mapping only',
   'ComparisonReady68':'only in actual external context under new RootArm68; not all core contexts',
   'source_role_compatibility':None},
 'root_hypothesis':'RawClosed plus stable real pens and actual external crossing geometry may supply fixed comparison L without asserting same-class Movement completion. Candidate assumption, not proved semantic bridge.',
 'initial_outer_choice':'Outer0=core; neither original Zn nor original DD/GG assigned',
 'object_table':relations,'prefix_contract':'local raw certificate known_at governs edge availability, full physical input preloading disclosed',
 'Owner':{'candidate_edge':'Own=(actual start,actual end]','delta':'disjoint continuous edge path from raw index1',
    'source_accounting':'E1 prelude, S0 raw root, delta factors, unresolved source remainder',
    'raw_ledger':'all old local records permanently retained; only actual delta factors participate in Adj'},
 'Next':{'delta':'actual successor in selected rooted path','MemberNext':'separate field of potential F2 member table',
   'original_Next':None,'multiple_delta_paths_found':False,'uniqueness_claim':'only finite maximal candidate path; no full legal relation uniqueness proof'},
 'consumer':'f2_contract in check_joint.py: executable necessary fixed-interface predicates, not production or complete F2 implementation',
 'selected_prefixes':[r for r in trace if r['t'] in [138,238,338,438]],
 'positive_fragment':{'nonempty_candidate_wholes':[w['id'] for w in ws if w['JEnd68']],
   'new_shifted_whole':'W68:S12-S16, own E242..341, evidence by358',
   'source_whole_completion_certified':False,'original_F2_nonempty_witness':False},
 'decisive_obstruction':'finite-obstruction.json','source_constraints':source_matrix,
 'P1':'this finite joint candidate fails requested nonempty fixed F2 and has unresolved original completion; all F1 not disproved',
 'P2':'finite candidate dependencies and explicit delayed clocks only; no full parser/recovery/latency proof',
 'P3':'not run','P4':'no adoption claim',
 'R_W':'this local branch attempted','R_D':'unchanged/open'})
# Supplement decisive lines missing from first capture; this is source packaging, not candidate replay.
supp=[]
for rel,lo,hi in [('.chanlun/definitions/qushi.md',124,144),('.chanlun/definitions/beichi.md',429,433)]:
 p=R/rel;lines=p.read_text().splitlines();supp.append({'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),
   'lines':[{'line':i,'text':lines[i-1]} for i in range(lo,hi+1)]})
write('source-supplement.json',supp)
write('InvocationCorrections.json',{
 'original_receipt_retained':True,'incorrect_field':'receipt.json/cwd','original_value':str(D),
 'adopted_actual_cwd':str(R),
 'actual_command':'python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage68/joint-author/check_joint.py > /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage68/joint-author/run.stdout 2> /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage68/joint-author/run.stderr',
 'why':'receipt stored HERE, which is output directory, not actual subprocess working directory; tool invocation used R',
 'execution_count':1,'result_effect':'none; all paths in candidate script are absolute',
 'known_early_prefix_coverage_field':'prefix ownership coverage_once is computed only at t>=21; early raw prelude is developing, not a completed q0 factor'})
print('packaged already computed relation, finite obstruction and source matrix')
