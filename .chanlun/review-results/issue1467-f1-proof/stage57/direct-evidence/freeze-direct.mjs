import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=fileURLToPath(new URL('.',import.meta.url)),repo=path.resolve(here,'../../../../..');
const P=path.resolve(here,'../..'),E='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/direct-author';
const hash=p=>createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const receipt=E+'/witness-check/run-manifest.json',m=JSON.parse(fs.readFileSync(receipt));
assert.equal(m.exact_root_passed,true);assert.deepEqual(m.target.unexpected_axioms,[]);
assert.equal(hash(here+'DirectLifecycle.lean'),hash(E+'/project/DirectLifecycle.lean'));
const modules=['BookQuoteProof','BookQuoteSpec','DirectHullProof','DirectHullSpec','DirectOutputProof','DirectOutputSourcePathProof','DirectOutputSourcePathSpec','DirectOutputSpec','DirectQuoteProof','DirectQuoteSpec','LocatedProof','LocatedSpec','MovingQuoteProof','MovingQuoteSpec','MovingQuoteWireSpec'];
for(const x of modules)assert.equal(hash(P+'/'+x+'.lean'),hash(E+'/project/'+x+'.lean'));
const formal=Object.entries(m.target.import_artifacts).filter(([k])=>k.startsWith('Origin.')).map(([k,v])=>({module:k,current_source:{path:repo+'/formal/'+k.replaceAll('.','/')+'.lean',sha256:hash(repo+'/formal/'+k.replaceAll('.','/')+'.lean')},loaded_olean:v}));
const sourceFiles=[...modules.map(x=>P+'/'+x+'.lean'),P+'/Stage47OutputSourceAndRawTurns.md',P+'/stage47/independent-direct-output-comparison.md',P+'/stage54/candidate-evidence/contract-v1.md',P+'/stage54/candidate-evidence/ordinary-proofs-v1.md',P+'/stage54/candidate-evidence/relation-reference.mjs',P+'/stage53/candidate-evidence/relation-reference.mjs',P+'/stage56/initial-p-lifecycle.md',P+'/stage56/independent-lifecycle-review.md',P+'/stage56/ScopeClarifications-v1.md'];
const summary={status:'author-evidence-only',controls:2,originalEvents:15,prefixesIncludingInitial:17,marketSamples:0,
 ordinaryProof:'A57 source-aware primitive adaptation and D54/L56 transfer; not fully formalized',
 lean:{runId:m.run_id,exactRootPassed:m.exact_root_passed,declaration:m.target.declaration,expectedType:m.target.expected_type,actualType:m.target.actual_type,axioms:m.target.axioms,helper:{declaration:'Stage57Direct.primitive_strict',status:'compiled; printed transitive axioms',axioms:['propext','Classical.choice','Quot.sound']},semantic:m.semantic.status,receipt:{path:receipt,sha256:hash(receipt)},researchModulesFreshlyCompiled:16,formalSourceOleanBindings:formal,formalFreshCompile:false},
 scope:'Finite original Step/Reads/construct fields, author JS D54-direct calculations, and ordinary A57; no original P/B56/F2/production claim',
 failureHistory:[{path:E+'/attempt1.stdout.log',sha256:hash(E+'/attempt1.stdout.log'),reason:'Lean proof-writing: simp reduced reflexive equality to True; Nat namespace needed. Model and target unchanged.'}],
 sourceBindings:sourceFiles.map(p=>({path:path.relative(repo,p),sha256:hash(p)}))};
fs.writeFileSync(here+'verification-summary.json',JSON.stringify(summary,null,2)+'\n');
const outputs=['../direct-lifecycle.md',...fs.readdirSync(here).filter(n=>n!=='manifest-v1.json'&&fs.statSync(here+n).isFile())].sort().map(p=>({path:path.relative(repo,path.resolve(here,p)),sha256:hash(path.resolve(here,p))}));
fs.writeFileSync(here+'manifest-v1.json',JSON.stringify({schema:'stage57-direct-freeze-v1',sourceBindings:summary.sourceBindings,outputs,receipt:summary.lean.receipt,note:'Content binding only, no timestamp authority or independent review'},null,2)+'\n');
console.log(JSON.stringify({status:'frozen',files:outputs.length,runId:m.run_id,reportSha256:hash(path.resolve(here,'../direct-lifecycle.md'))}));
