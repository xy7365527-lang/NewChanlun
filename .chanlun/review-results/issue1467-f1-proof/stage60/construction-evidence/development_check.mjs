// Added after the frozen static template run: a named causal observation protocol.
// It changes neither source events nor clocks nor the #873 formula.
import fs from 'node:fs';import path from 'node:path';import assert from 'node:assert/strict';
const [run,output]=process.argv.slice(2);if(!run||!output)throw Error('run directory and new output file required');
const result={protocol:'DevExtreme60-v1',status:'author exact finite computation',introduced:'after static run-v2; not preregistered before the template',rule:{anchor:'S3 certified endpoint, hence start=81 is usable from k=98',left:'b=S0 has been certified since 38',right:'among stable up-strokes whose start is at or after anchor, choose earliest endpoint attaining maximum end price; all stroke members from anchor through that stroke form the candidate c_k',revisability:'a later mature higher endpoint replaces the candidate; no claim that its time is the final q-Move endpoint',seal:'structural S4 confirmation at 118 ends development of this reference Segment only',clock:'unchanged t_i=i'},templates:[]};
for(const name of ['T60-main','N60-equal-force']){
 const x=JSON.parse(fs.readFileSync(path.join(run,name+'.source.json'))),r=JSON.parse(fs.readFileSync(path.join(run,name+'.reference.stdout'))),cert=JSON.parse(fs.readFileSync(path.join(run,name+'.certificate.json')));
 const maturity=j=>r.prefixes.find(p=>p.stable_count>j)?.cut??null;
 const atom=j=>{const a=r.strokes[j];return {id:j,start:a.start,end:a.end,dp:a.end_price-a.start_price,dt:x.observations[a.end].time-x.observations[a.start].time,fractal_known_at:r.prefixes.at(-1).anchors.find(n=>n.raw===a.end).known_at,stable_at:maturity(j)};};
 const rows=[];
 for(const p of r.prefixes){
  if(p.cut<cert.core.known_by){rows.push({cut:p.cut,ready:false,why:'the declared K/source boundary is not yet certified'});continue;}
  const cs=p.stable.map((s,j)=>({...s,j})).filter(s=>s.up&&s.start>=81);
  const best=cs.reduce((a,s)=>!a||s.end_price>a.end_price?s:a,null);
  assert(best);const first=p.stable.find(s=>s.start===81);assert(first&&first.up);
  const velocity=s=>(s.end_price-s.start_price)/(x.observations[s.end].time-x.observations[s.start].time);
  const L=velocity(best)-velocity(first);
  const exactMembers=p.stable.filter(s=>s.start>=81&&s.end<=best.end);
  assert.equal(exactMembers[0].start,81);assert.equal(exactMembers.at(-1).end,best.end);
  assert(exactMembers.every(s=>s.start_price>=12500&&s.end_price>=12500&&s.start_price<=best.end_price&&s.end_price<=best.end_price));
  rows.push({cut:p.cut,ready:true,current_candidate:{start:81,end:best.end,end_price:best.end_price,first_atom:20,last_atom:best.j,members:exactMembers.length},L,known_b_L:250,strict_weakening:L<250,extreme:best.end_price>16000,developing:p.cut<118,structural_end_certified:p.cut>=118});
 }
 const earliest=rows.find(p=>p.ready&&p.strict_weakening);
 assert.equal(earliest?.cut??null,name==='T60-main'?106:null);
 const row98=rows.find(p=>p.cut===98),row105=rows.find(p=>p.cut===105),row106=rows.find(p=>p.cut===106);
 assert.equal(row98.L,950);assert.equal(row105.L,950);assert.equal(row106.L,name==='T60-main'?-125:250);assert(row106.developing);
 result.templates.push({name,atom_knowledge:[0,4,20,22,24].map(atom),earliest_strict_weakening:earliest?.cut??null,selected_prefixes:[98,102,105,106,110,114,117,118].map(k=>rows[k]),all_prefixes:rows});
}
fs.writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(result.templates.map(x=>({name:x.name,earliest:x.earliest_strict_weakening,atoms:x.atom_knowledge}))));
