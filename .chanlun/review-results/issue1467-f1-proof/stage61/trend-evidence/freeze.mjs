import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const E='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage61/trend';
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const read=p=>JSON.parse(fs.readFileSync(p));
const checks={kind:'author postcheck, not independent review',same_source_across_retries:[],template_checks:[]};
const hs=['run-v1','run-v2','run-v3'].map(n=>({run:n,sha256:hash(path.join(E,n,'T61-lift.source.json'))}));
assert(hs.every(x=>x.sha256===hs[0].sha256));checks.same_source_across_retries=hs;
for(const name of ['T61-lift','N61-equal-force']){
 const dir=path.join(E,'run-v3'),x=read(path.join(dir,name+'.certificate.json')),
  raw=read(path.join(dir,name+'.source.json')),ref=read(path.join(dir,name+'.reference.stdout'));
 assert.deepEqual(x.counts,{observations:1223,events:1222,strokes:304,certified_segments:60,proposed_segments:61});
 assert.equal(raw.observations.length,1223);assert.equal(raw.events.length,1222);
 assert.equal(ref.prefixes.length,1223);assert.equal(ref.certificate_prefixes.length,60);
 const p=x.chunks[1],s=x.segments[9];assert.deepEqual(p.owned_events,[102,201]);assert.deepEqual(s.owned_events,[182,201]);
 assert.equal(s.owned_events[0]-p.owned_events[0],80);
 assert.deepEqual(p.members,['S5','S6','S7','S8','S9']);assert.equal(p.proposed_level,0);
 assert.equal(p.entry_exit_force.entry.L,-100);assert.equal(p.entry_exit_force.exit.L,-100);
 assert(p.entry_exit_force.extreme);assert(!p.entry_exit_force.strict_weakening);
 assert.equal(x.parent.force.b.L,100);assert.equal(x.parent.force.c.L,name==='T61-lift'?-125:100);
 assert.equal(x.parent.end,1101);assert.equal(ref.prefixes.find(p=>p.stable_count>274).cut,1106);
 assert.equal(x.segments[54].known_at,1118);assert.equal(x.chunks[8].end,901);assert.equal(x.chunks[9].end,1001);
 assert(x.chunks[9].end>x.chunks[8].end&&x.chunks[9].end<x.parent.end);
 assert.deepEqual(x.chunks[11].selected_window.core,[65000,62000]);assert(!x.chunks[11].selected_window.positive_width);
 assert(!x.parent.Div_1&&!x.parent.NE_TU_applied);
 checks.template_checks.push({name,P1_local_strict_weakening:false,P1_semantic_noncompletion_proved:false,
  P1_minus_S9_owned_event_count:80,outer_Lb:100,outer_Lc:x.parent.force.c.L,
  end_occurrence:1101,last_stroke_stable:1106,last_segment_known:1118,
  standard_trend_certified:false,NE_TU_applied:false});
}
fs.writeFileSync(path.join(E,'postcheck-v1.json'),JSON.stringify(checks,null,2)+'\n');
const run=read(path.join(E,'run-v3/run-manifest.json'));
for(const i of run.inputs)assert.equal(hash(i.path),i.sha256);
for(const s of run.sources)assert.equal(hash(s.path),s.sha256);
for(const a of run.artifacts)assert.equal(hash(path.join(E,'run-v3',a.path)),a.sha256);
const walk=d=>fs.readdirSync(d,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(path.join(d,e.name)):[path.join(d,e.name)]);
const files=[path.join(here,'../trend-completion.md'),...walk(here).filter(p=>path.basename(p)!=='manifest-v1.json'),...walk(E)];
const artifacts=files.sort().map(p=>({path:p,bytes:fs.statSync(p).size,sha256:hash(p)}));
const total=artifacts.reduce((s,a)=>s+a.bytes,0);assert(total<128*1024*1024);
const manifest={frozen_at:new Date().toISOString(),author_status:'finite source construction and specific candidate failure; independent review pending',
 ownership:['stage61/trend-completion.md','stage61/trend-evidence/',E],
 templates:['T61-lift','N61-equal-force'],adopted_run:path.join(E,'run-v3'),
 new_lean_root:null,old_roots_rerun:false,total_artifact_bytes:total,budget_bytes:128*1024*1024,
 meaningful_negative:'P1 designated entry/exit strict inequality is false; this does not prove semantic noncompletion',
 artifacts};
const output=path.join(here,'manifest-v1.json');fs.writeFileSync(output,JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({status:'frozen',manifest:output,sha256:hash(output),artifact_count:artifacts.length,total_artifact_bytes:total}));
