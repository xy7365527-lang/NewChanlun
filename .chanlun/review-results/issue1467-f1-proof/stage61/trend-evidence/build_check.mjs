import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const R=path.resolve(here,'../../../../..');
const E=process.argv[2];
assert(E&&path.isAbsolute(E)&&!fs.existsSync(E),'supply a NEW absolute output directory');
fs.mkdirSync(E,{recursive:true});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const put=(n,x)=>fs.writeFileSync(path.join(E,n),JSON.stringify(x,null,2)+'\n');
const run=(n,cmd,args)=>{
 const t=Date.now(),r=spawnSync(cmd,args,{encoding:'utf8',maxBuffer:32*1024*1024});
 fs.writeFileSync(path.join(E,n+'.stdout'),r.stdout??'');fs.writeFileSync(path.join(E,n+'.stderr'),r.stderr??'');
 put(n+'.receipt.json',{argv:[cmd,...args],status:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:Date.now()-t});
 assert.equal(r.status,0,`${n}: ${r.stderr}`);return r.stdout;
};
const rows=[[100,220,140,240,180,300],[300,250,280,230,270,200],[200,240,220,260,230,280],
 [280,250,270,230,260,220],[220,350,270,380,310,500],[500,450,480,430,470,400],
 [400,440,420,460,430,480],[480,450,470,430,460,420],[420,520,490,550,510,650],
 [650,590,630,580,620,560],[560,610,580,630,590,750],[750,650,700,500,620,450]];
const vertices=[rows[0][0]*100];
for(const row of rows){assert.equal(row[0]*100,vertices.at(-1));vertices.push(...row.slice(1).map(x=>x*100));}
vertices.push(60000);assert.equal(vertices.length,62);
const gcd=(a,b)=>b?gcd(b,a%b):a;let lcm=1n;
for(let d=4n;d<=2804n;d++)lcm=lcm/gcd(lcm,d)*d;
const A=25n*lcm;
const lib='/tmp/nc1467-cargo-target/debug/deps',pythonLib='/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib';
const deps=[path.join(lib,'libnewchan_rust.rlib'),path.join(lib,'libserde_json-d8aa09aa6d50fe2e.rlib')];
const binary=path.join(E,'reference-probe');
run('compile','rustc',[path.join(here,'reference_probe.rs'),'--edition=2021','-L',`dependency=${lib}`,'-L',`native=${pythonLib}`,
 '-C',`link-arg=-Wl,-rpath,${pythonLib}`,'--extern',`newchan_rust=${deps[0]}`,'--extern',`serde_json=${deps[1]}`,'-o',binary]);
const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];
const intersection=xs=>[Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const hull=xs=>[Math.min(...xs.map(x=>x[0])),Math.max(...xs.map(x=>x[1]))];
const intersects=(a,b)=>Math.max(a[0],b[0])<Math.min(a[1],b[1]);
const results=[];
for(const name of ['T61-lift','N61-equal-force']) {
 const pivots=[vertices[0]];
 for(let s=0;s<61;s++){
  const lo=vertices[s],hi=vertices[s+1],d=Math.sign(hi-lo);
  let first=400,near=200,preHigh=300,last=800;
  if(s===39)last=1200;if(s===40)first=800;
  if(s===54){preHigh=100;last=name==='T61-lift'?300:1200;}
  if(s===55){first=200;near=100;}
  const p=[lo,lo+d*first,lo+d*near,hi-d*preHigh,hi-d*last,hi];
  assert.equal(p[0],pivots.at(-1));pivots.push(...p.slice(1));
 }
 const prices=[10050,pivots[0]];
 for(let i=0;i+1<pivots.length;i++)for(let k=1;k<=4;k++)prices.push(pivots[i]+(pivots[i+1]-pivots[i])*k/4);
 prices.push(60025);assert.equal(prices.length,1223);
 const q=prices.map(p=>{assert(Number.isInteger(p)&&p>=10000&&p<=80000&&p%25===0);const n=A*BigInt(p-9900),d=BigInt(80100-p);assert.equal(n%d,0n);return n/d;});
 const observations=prices.map((p,i)=>({index:i,time:i,price:p,bid_quantity:q[i].toString(),ask_quantity:A.toString(),trade_volume:0,untradable:true}));
 const events=q.slice(1).map((v,i)=>{const d=v-q[i];assert.notEqual(d,0n);return {id:i+1,time:i+1,side:'bid',price:9900,
  action:d>0n?'add':'cancel',amount:(d>0n?d:-d).toString(),quantity_before:q[i].toString(),quantity_after:v.toString()};});
 const source={name,clock:'t_i=i; declared before execution',bid:9900,ask:80100,ask_quantity:A.toString(),
  initial:{time:0,bid_quantity:q[0].toString(),ask_quantity:A.toString()},pivots,observations,events};
 let current=q[0];
 for(let i=0;i<observations.length;i++){
  if(i){const e=events[i-1],amount=BigInt(e.amount);assert.equal(current.toString(),e.quantity_before);if(e.action==='cancel')assert(amount<current);
   current+=e.action==='add'?amount:-amount;assert.equal(current.toString(),e.quantity_after);}
  assert(current>0n);const n=80100n*current+9900n*A,d=current+A;assert.equal(n%d,0n);assert.equal(Number(n/d),prices[i]);
 }
 put(name+'.source.json',source);
 const ref=JSON.parse(run(name+'.reference',binary,[path.join(E,name+'.source.json')]));
 // The frozen final support is 60025 > 60000: the last upward leg has no
 // completed top fractal, so only 304 reference strokes exist, not 305.
 // Preserve that actual activity; do not repair the input or seal at EOF.
 assert.equal(ref.groups,1223);assert.equal(ref.strokes.length,304);
 ref.strokes.forEach((p,j)=>{assert.equal(p.start,1+4*j);assert.equal(p.end,5+4*j);assert.equal(p.start_price,prices[p.start]);assert.equal(p.end_price,prices[p.end]);
  assert.equal(p.up,j%2===0);assert.equal(p.end-p.start-1,3);
  for(const k of [p.start,p.end])assert(k>0&&k+1<prices.length);
  assert(p.up?prices[p.start]<prices[p.start-1]&&prices[p.start]<prices[p.start+1]&&prices[p.end]>prices[p.end-1]&&prices[p.end]>prices[p.end+1]:
   prices[p.start]>prices[p.start-1]&&prices[p.start]>prices[p.start+1]&&prices[p.end]<prices[p.end-1]&&prices[p.end]<prices[p.end+1]);
 });
 for(const p of ref.prefixes)assert(p.stable_equals_final_prefix);
 const segments=[];
 for(let s=0;s<60;s++){
  const startPen=5*s,cut=38+20*s,up=s%2===0,cp=ref.certificate_prefixes.find(p=>p.cut===cut);
  assert(cp);assert.deepEqual(cp.stable,ref.strokes.slice(0,ref.prefixes[cut].stable_count));
  const members=cp.stable.slice(startPen,startPen+5);assert.equal(members.length,5);
  const features=cp.stable.slice(startPen).map((p,j)=>({...p,pen:startPen+j,range:range(p)})).filter(p=>p.up!==up);
  for(let j=1;j<features.length;j++)assert(!contains(features[j-1].range,features[j].range)&&!contains(features[j].range,features[j-1].range));
  let tri;
  for(let j=1;j+1<features.length;j++){
   const [a,b,c]=features.slice(j-1,j+2).map(e=>e.range);
   if(up?b[1]>a[1]&&b[1]>c[1]:b[0]<a[0]&&b[0]<c[0]){tri=features.slice(j-1,j+2);break;}
  }
  assert(tri);assert.deepEqual(tri.map(p=>p.pen),[startPen+3,startPen+5,startPen+7]);
  const [a,b,c]=tri.map(p=>p.range);assert(up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1]);assert(intersects(a,b));
  const start=members[0].start,end=members.at(-1).end,start_price=prices[start],end_price=prices[end],whole=hull(members.map(range));
  assert.equal(end_price,up?b[1]:b[0]);assert.deepEqual(whole,[Math.min(start_price,end_price),Math.max(start_price,end_price)]);
  const init=intersection(members.slice(0,3).map(range));assert(init[0]<init[1]);
  const x={id:`S${s}`,up,start,end,start_price,end_price,range:whole,members:[startPen,startPen+5],owned_events:[start+1,end],
   evidence_events:[end+1,cut],known_at:cut,initial_three_core:init,feature_strokes:features.map(p=>p.pen),features:features.map(p=>p.range),
   first_selected:tri.map(p=>p.pen),selected_ranges:tri.map(p=>p.range),gap:false,strong_fractal:true,
   rule:'067 no-containment/no-gap first case; exact stable source prefix'};
  if(s){assert.equal(segments.at(-1).end,start);assert.equal(segments.at(-1).end_price,start_price);}segments.push(x);
 }
 const atom=p=>({stroke:ref.strokes.indexOf(p),start:p.start,end:p.end,dp:p.end_price-p.start_price,dt:p.end-p.start,v:(p.end_price-p.start_price)/(p.end-p.start)});
 const force=(first,last)=>{const a=atom(ref.strokes[first]),z=atom(ref.strokes[last]);return {first:a,last:z,L:z.v-a.v};};
 const chunks=[];
 for(let j=0;j<12;j++){
  const members=segments.slice(5*j,5*j+5),start=members[0].start,end=members.at(-1).end;
  const selected=members.slice(1,4),kernel=intersection(selected.map(s=>s.range));
  // P11 is retained successor source, not an admitted center/movement. Its
  // frozen interior window is empty; record that failure instead of changing it.
  if(j<11)assert(kernel[0]<kernel[1]);else assert.deepEqual(kernel,[65000,62000]);
  const whole=[Math.min(...prices.slice(start,end+1)),Math.max(...prices.slice(start,end+1))];
  assert.deepEqual(whole,hull(members.map(s=>s.range)));
  const windows=[0,1,2].map(off=>({members:members.slice(off,off+3).map(s=>s.id),core:intersection(members.slice(off,off+3).map(s=>s.range)),whole:hull(members.slice(off,off+3).map(s=>s.range))}));
  const fb=force(25*j,25*j+4),fc=force(25*j+20,25*j+24);
  chunks.push({id:`P${j}`,proposed_level:0,up:j%2===0,start,end,start_price:prices[start],end_price:prices[end],range:whole,
   members:members.map(s=>s.id),owned_events:[start+1,end],segment_support_known:members.at(-1).known_at,
   complete:false,complete_status:'not certified as a whole P; only member segments certified',
   selected_window:{members:selected.map(s=>s.id),core:kernel,positive_width:kernel[0]<kernel[1],whole:hull(selected.map(s=>s.range)),object_identity:'declared interior window, not fixed-F2 instance'},
   all_three_windows:windows,entry_exit_force:{entry:fb,exit:fc,strict_weakening:fc.L<fb.L,
    extreme:j%2===0?members[4].end_price>members[0].end_price:members[4].end_price<members[0].end_price}});
 }
 const makeParent=(id,js)=>{const ps=js.map(j=>chunks[j]);return {id,proposed_level:1,members:ps.map(p=>p.id),member_sources:ps.map(p=>p.owned_events),
  core:intersection(ps.map(p=>p.range)),whole:hull(ps.map(p=>p.range)),start:ps[0].start,end:ps.at(-1).end,
  geometry_known:Math.max(...ps.map(p=>p.segment_support_known)),completed_input_count:ps.filter(p=>p.complete).length,qualified:false};};
 const parentA=makeParent('A',[1,2,3]),parentB=makeParent('B',[5,6,7]);
 assert.deepEqual(parentA.core,[22000,28000]);assert.deepEqual(parentA.whole,[20000,30000]);
 assert.deepEqual(parentB.core,[42000,48000]);assert.deepEqual(parentB.whole,[40000,50000]);assert(parentA.whole[1]<parentB.whole[0]);
 const b=force(100,124),c=force(200,274);assert.equal(b.L,100);assert.equal(c.L,name==='T61-lift'?-125:100);
 assert(chunks[9].range[0]>parentB.core[1]);assert(chunks[10].end_price>chunks[4].end_price);
 const lower=chunks.slice(8,11).map(p=>({parent:p.id,...p.selected_window}));
 assert(lower[0].whole[1]<lower[1].whole[0]);assert(intersects(lower[1].core,lower[2].core));
 const firstFailure={object:chunks[1],requested_use:'first input P1 for q=1 center A',
  last_member:segments[9],last_member_ends:201,last_member_known:218,
  attempt:'inherit Completed(P1,201) from 067(S9) and a one-core local comparison',
  concrete_failure:'entry S5 and exit S9 both have L=-100; strict weakening false although Extreme and segment endings pass',
  not_a_global_counterexample:'does not exclude another lawful decomposition or initial completion rule; no applicable P1 completion has been generated'};
 assert.equal(chunks[1].entry_exit_force.entry.L,-100);assert.equal(chunks[1].entry_exit_force.exit.L,-100);
 assert(!chunks[1].entry_exit_force.strict_weakening);assert(chunks[1].entry_exit_force.extreme);
 const certificate={name,counts:{observations:prices.length,events:events.length,strokes:ref.strokes.length,certified_segments:segments.length,proposed_segments:61},
  segments,chunks,firstFailure,parent:{q:1,a:'P0',A:parentA,b:'P4',B:parentB,c:['P8','P9','P10'],owned_events:[2,1101],end:1101,
   source_support_known:1118,force:{b,c,strict_weakening:c.L<b.L},extreme:{b:50000,c:75000,pass:true},
   departure_retest:{departure:'P8',retest:'P9',retest_range:chunks[9].range,B_upper:48000,geometric_no_return:true,
    departure_complete:false,retest_complete:false,third_buy_certified:false},
   c_internal_windows:lower,distinct_F2_center_instances_certified:0,Div_1:false,NE_TU_applied:false,
   successor:{object:'P11',owned_events:[1102,1201],source_support_known:1218,qualified_q1_successor:false}},
  eof:{stable_strokes:ref.prefixes.at(-1).stable_count,active_stroke:ref.prefixes.at(-1).active,uncertified_segment:'S60',
   proposed_strokes:305,actual_strokes:304,last_upward_leg_lacks_top_right_support:true}};
 put(name+'.certificate.json',certificate);results.push(certificate);
}
assert.deepEqual(results[0].segments.map(s=>[s.start,s.end,s.known_at,s.first_selected]),results[1].segments.map(s=>[s.start,s.end,s.known_at,s.first_selected]));
assert(results[0].parent.force.strict_weakening);assert(!results[1].parent.force.strict_weakening);
put('summary.json',{status:'actual-two-level-lift-scaffold; first-P0-completion-fails; no-standard-trend-certified',
 templates:results.map(r=>({name:r.name,counts:r.counts,Lb:r.parent.force.b.L,Lc:r.parent.force.c.L,
   first_failure:{object:'P1',source:[102,201],last_segment:'S9',segment_known:218,entryL:-100,exitL:-100,strict:false},Div_1:false,NE_TU_applied:false}))});
const sources=['.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs','rust/src/theta_v0/types.rs',
 '.chanlun/definitions/beichi.md','.chanlun/definitions/zhongshu.md',
 ...['018','035','037','057','067','078','081','084'].map(n=>`docs/chanlun/text/blog/${n}-第${Number(n)}课.md`)];
const inputs=['template-v1.md','reference_probe.rs','build_check.mjs'].map(n=>({path:path.join(here,n),sha256:hash(path.join(here,n))}));
const artifacts=fs.readdirSync(E).filter(n=>n!=='run-manifest.json').sort().map(n=>({path:n,bytes:fs.statSync(path.join(E,n)).size,sha256:hash(path.join(E,n))}));
const total=artifacts.reduce((s,a)=>s+a.bytes,0);assert(total<128*1024*1024);
put('run-manifest.json',{inputs,sources:sources.map(p=>({path:path.join(R,p),sha256:hash(path.join(R,p))})),
 runtime:{node:process.version,dependencies:deps.map(p=>({path:p,sha256:hash(p)}))},artifacts,total_bytes:total,budget_bytes:128*1024*1024});
console.log(JSON.stringify({status:'checks-passed-with-semantic-failure-preserved',bytes:total,templates:results.map(r=>({name:r.name,counts:r.counts,Lb:r.parent.force.b.L,Lc:r.parent.force.c.L,first_failure:'P1 completion'}))}));
