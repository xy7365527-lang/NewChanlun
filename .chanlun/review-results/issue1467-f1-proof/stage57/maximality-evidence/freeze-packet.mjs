import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const P=path.join(R,'.chanlun/review-results/issue1467-f1-proof');
const D=path.join(P,'stage57/maximality-evidence');
const E='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/maximality-author';
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const put=(name,x)=>fs.writeFileSync(path.join(D,name),JSON.stringify(x,null,2)+'\n');
const picks=[
 ['018',18,[12,24,26,28,40,42,44,46,48,54,64,68]],
 ['020',20,[12,14,16,18,20,22,24,30,32,40,44,48,52,54,56,58,60,94]],
 ['038',38,[12,14,18,22,24,26,28,42,44,290,292,294,296,298,300]],
 ['084',84,[12,50,52,54,56,60,68]],
];
const excerpts=picks.map(([n,num,lines])=>{
 const rel=`docs/chanlun/text/blog/${n}-第${num}课.md`;const file=path.join(R,rel);const a=fs.readFileSync(file,'utf8').split('\n');
 return {path:rel,sha256:hash(file),lines:lines.map(line=>({line,text:a[line-1]}))};
});
put('source-excerpts-v1.json',{scope:'仅核指定仓内转载的可见作者归属；全文行原样保留供独评。',exclusions:['018:24行内娇注、46/48注释不用','020:14/22行首注不用，24/56尾部注不用','038:14/42注不用，300只用等号后的作者答复','不得将raw腿直接授予020:52的Zn资格'],sources:excerpts});
const mf=path.join(E,'lean-exact/run-manifest.json');const m=JSON.parse(fs.readFileSync(mf,'utf8'));
const source=path.join(D,'RawMaximality.lean');const code=fs.readFileSync(source,'utf8');
const readback={source_path:source,source_sha256:hash(source),definitions:code.slice(0,code.indexOf('theorem raw_maximal')),declaration:m.target.declaration,actual_type:m.target.actual_type,universes:m.target.universes,binder_kinds:m.target.binder_kinds,axioms:m.target.axioms,dependencies:m.target.dependencies,definition_hashes:m.target.definition_hashes,environment:m.environment};
put('readback-packet-v1.json',readback);
put('verification-summary-v1.json',{machine:{run_id:m.run_id,report:mf,report_sha256:hash(mf),machine:m.machine,machine_verification_passed:m.machine_verification_passed,exact_root_passed:m.exact_root_passed,root_closure:m.root_closure,actual_type:m.target.actual_type,axioms:m.target.axioms,unexpected_axioms:m.target.unexpected_axioms,unsafe_dependencies:m.target.unsafe_dependencies,comparison:m.target.comparison.definitionally_equal,semantic:m.semantic},formal_scope:'泛型区间关系RawTerminal到最大性/唯一性/保留追加/时钟；D54提取和实数解释是普通证明；ER57未证。',real_attempt:{source:path.join(E,'RawMaximalityRealAttempt.lean'),sha256:hash(path.join(E,'RawMaximalityRealAttempt.lean')),contract:'lean-contract-v1.json',receipt:path.join(E,'lean-real-timeout.json'),status:'timeout_sigterm_no_compilation_claim',old_processes_checked_absent:[82505,82936,84157]},independent_review:'pending',existing_certificate:'certificate-v1.json',readback:'readback-packet-v1.json'});
const author=['stage57/extension-maximality.md',...fs.readdirSync(D).filter(n=>n!=='manifest-v1.json').map(n=>'stage57/maximality-evidence/'+n)];
const dependencies=[
 'stage57/Intent-v1.md',
 'stage56/initial-p-lifecycle.md','stage56/independent-lifecycle-review.md','stage56/ScopeClarifications-v1.md',
 'stage54/candidate-evidence/contract-v1.md','stage54/candidate-evidence/ordinary-proofs-v1.md',
 'stage54/initial-contract-audit.md','stage54/independent-initial-contract-review.md',
];
const external=['lean-exact/run-manifest.json','lean-exact-run.json','lean-exact-run.stdout.txt','lean-exact-run.stderr.txt','RawMaximalityRealAttempt.lean','lean-contract-real-attempt.json','lean-real-timeout.json'];
const entry=(file)=>({path:file,bytes:fs.statSync(file).size,sha256:hash(file)});
put('manifest-v1.json',{issue:1467,stage:57,scope:'固定K/C/s原始最大性作者包；不锁动态Progress',author:author.map(p=>entry(path.join(P,p))),dependencies:[...dependencies.map(p=>path.join(P,p)),...excerpts.map(e=>path.join(R,e.path))].map(entry),external:external.map(p=>entry(path.join(E,p))),review_status:'pending'});
console.log(JSON.stringify({author:author.length,dependencies:dependencies.length+excerpts.length,external:external.length,exact_root_passed:m.exact_root_passed,axioms:m.target.axioms}));
