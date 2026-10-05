import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const D=path.dirname(fileURLToPath(import.meta.url));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const read=p=>JSON.parse(fs.readFileSync(p,'utf8'));
const write=(p,v)=>fs.writeFileSync(path.join(D,p),JSON.stringify(v,null,2)+'\n');
const before=read(D+'/inputhash-before.json').files;
const after=before.map(f=>({path:f.path,bytes:fs.statSync(f.path).size,sha256:sha(f.path)}));
assert.deepEqual(before,after);
write('inputhash-after.json',{phase:'after independent arithmetic, source reading, report and immediately before seal',files:after,identical_to_before:true});
const findings={
 verdict:'accept finite raw/geometry mapping; require one machine-representation correction; source semantic role remains unresolved',
 accepted:['Frozen package identity and all 29 bounded checks','ClosedRaw(b,21;38) with inherited Stage66 certificate scope','b whole [40000,76000], own E2..21, L=-1000','c0 whole force -3500 and selected external comparison geometry','Earlier conditional b/P0/P1 triple ending 221; 238 only old local aggregate','Original semantic roles and times stay null','Movement-only completion obligation is not universally imposed on raw roots'],
 must_revise:[{id:'M1',severity:'representation/scope',locations:['base-role-author/relation/model-or-obstruction.json:86','base-role-author/relation/model-or-obstruction.json:93'],details:['Replace is not a Completed_original Movement with explicitly unasserted/null wording consistent with Report.md.','Include candidate-scoped external-entry geometry in the obstruction antecedent, not RawEnd067 alone.'],not_required:['new history','new doctrine','user approval gate','infinite recursive completion']}],
 unresolved:['RootArm67 source-compatible completed-entry role','Original source qualification of K1 and members','Full F1/source model existence or nonexistence'],
 novelty:{new_semantic_bridge_evidence:false,reused_raw_evidence:true,contribution:'explicit partial role map, typed gap and source/viewpoint scope analysis; does not fill prior H_b semantic gap'},
 source_status:{live:false,cached_original_issues:[812,865,873],canonical_blog_read_directly:true},
 semantic_admission_pass:false,semantic_nonexistence_proven:false,original_completed_b:null,
};
write('findings.json',findings);
const receipt=read(D+'/receipt.json');
receipt.review_status='complete_with_required_representation_fix';
receipt.required_revision_count=1;
receipt.input_hashes_unchanged_at_seal=true;
receipt.report='review.md';
write('receipt.json',receipt);
const files=fs.readdirSync(D,{withFileTypes:true}).filter(e=>e.isFile()&&!['manifest.json','FINAL.json'].includes(e.name)).map(e=>({path:e.name,bytes:fs.statSync(path.join(D,e.name)).size,sha256:sha(path.join(D,e.name))})).sort((a,b)=>a.path.localeCompare(b.path));
const total=files.reduce((n,f)=>n+f.bytes,0);
assert(total<96*1024*1024);
write('manifest.json',{format:1,task:'issue1467 Stage67 independent base-role review',frozen_author_manifest_sha256:'678f261effaf7f23b45e97ccbb1fe5944b368f28993261dff39536ceda980ffd',files,total_manifested_bytes:total,budget_bytes:96*1024*1024,input_files_unchanged_at_seal:true});
const result={status:'independent_review_complete_with_required_representation_fix',verdict:findings.verdict,independent_review:true,author_self_checks_recomputed:29,checks_passed:29,required_revision_count:1,semantic_admission_pass:false,semantic_nonexistence_proven:false,original_completed_b:null,RootArm67_source_proven:false,history_count:1,new_history_count:0,candidate_mapping_count:1,report:'review.md',findings:'findings.json',receipt:'receipt.json',manifest_sha256:sha(D+'/manifest.json'),report_sha256:sha(D+'/review.md'),independent_checks_sha256:sha(D+'/independent-checks.json'),total_manifested_bytes:total};
write('FINAL.json',result);
console.log(JSON.stringify(result,null,2));
