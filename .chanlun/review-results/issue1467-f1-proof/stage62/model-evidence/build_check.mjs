// #1467 Stage62: one predeclared history, finite author computation.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const R=path.resolve(here,'../../../../..');
const E=process.argv[2];
assert(E&&path.isAbsolute(E)&&!fs.existsSync(E),'new absolute output directory required');
fs.mkdirSync(E,{recursive:true});
const put=(n,x)=>fs.writeFileSync(path.join(E,n),JSON.stringify(x,null,2)+'\n',{flag:'wx'});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const run=(n,cmd,args)=>{const t=Date.now(),r=spawnSync(cmd,args,{encoding:'utf8',timeout:60000,maxBuffer:32*1024*1024});
 fs.writeFileSync(path.join(E,n+'.stdout'),r.stdout??'',{flag:'wx'});fs.writeFileSync(path.join(E,n+'.stderr'),r.stderr??'',{flag:'wx'});
 put(n+'.receipt.json',{argv:[cmd,...args],status:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:Date.now()-t});
 assert.equal(r.status,0,`${n}: ${r.stderr}`);return r.stdout;};
const card=fs.readFileSync(path.join(here,'template-v1.md'),'utf8');
const rows=JSON.parse(card.split('```json\n')[1].split('```')[0]);
assert.equal(rows.length,17);
const pivots=[rows[0][0]*200];
for(const row of rows){assert.equal(row[0]*200,pivots.at(-1));pivots.push(...row.slice(1).map(p=>p*200));}
const prices=[20100,pivots[0]];
for(let j=0;j+1<pivots.length;j++)for(let k=1;k<=4;k++)prices.push(pivots[j]+(pivots[j+1]-pivots[j])*k/4);
prices.push(36050);assert.equal(prices.length,343);
const gcd=(a,b)=>b?gcd(b,a%b):a;let A=1n;
for(let d=1n;d<=672n;d++)A=A/gcd(A,d)*d;
const qs=prices.map(p=>{assert(Number.isInteger(p)&&p%25===0&&p>19800&&p<36600);const n=A*BigInt((p-19800)/25),d=BigInt((36600-p)/25);assert.equal(n%d,0n);return n/d;});
const observations=prices.map((p,i)=>({index:i,time:i,price:p,bid_quantity:String(qs[i]),ask_quantity:String(A),trade_volume:0,untradable:true}));
const events=qs.slice(1).map((q,j)=>{const d=q-qs[j];assert.notEqual(d,0n);return {id:j+1,time:j+1,side:'bid',price:19800,action:d>0n?'add':'cancel',amount:String(d>0n?d:-d),quantity_before:String(qs[j]),quantity_after:String(q)};});
const source={name:'T62-joint',profile:'Omega62-PB-v2',clock:'t_i=i fixed before execution',bid:19800,ask:36600,ask_quantity:String(A),initial:{time:0,bid_quantity:String(qs[0]),ask_quantity:String(A)},pivots,observations,events};
let q=qs[0];for(let i=0;i<observations.length;i++){
 if(i){const e=events[i-1],amount=BigInt(e.amount);assert.equal(String(q),e.quantity_before);if(e.action==='cancel')assert(amount<q);q+=e.action==='add'?amount:-amount;assert.equal(String(q),e.quantity_after);}
 assert(q>0n);const n=36600n*q+19800n*A,d=q+A;assert.equal(n%d,0n);assert.equal(Number(n/d),prices[i]);}
put('T62-joint.source.json',source);
const lib='/tmp/nc1467-cargo-target/debug/deps',pythonLib='/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib';
const deps=[path.join(lib,'libnewchan_rust.rlib'),path.join(lib,'libserde_json-d8aa09aa6d50fe2e.rlib')];
put('runtime-identities.json',{node:process.version,dependencies:deps.map(p=>({path:p,sha256:hash(p)})),templates:['template-v1.md','template-v2.md'].map(n=>({path:n,sha256:hash(path.join(here,n))}))});
const binary=path.join(E,'reference-probe');
run('compile','rustc',[path.join(here,'reference_probe.rs'),'--edition=2021','-L',`dependency=${lib}`,'-L',`native=${pythonLib}`,'-C',`link-arg=-Wl,-rpath,${pythonLib}`,'--extern',`newchan_rust=${deps[0]}`,'--extern',`serde_json=${deps[1]}`,'-o',binary]);
const ref=JSON.parse(run('T62-joint.reference',binary,[path.join(E,'T62-joint.source.json')]));
assert.equal(ref.groups,343);
// Do not seal the final upward leg at EOF: the declared right support is higher.
assert.equal(ref.strokes.length,84);
for(const [j,s] of ref.strokes.entries()){
 assert.equal(s.start,1+4*j);assert.equal(s.end,5+4*j);assert.equal(s.start_price,prices[s.start]);assert.equal(s.end_price,prices[s.end]);assert.equal(s.up,j%2===0);}
for(const p of ref.prefixes)assert.deepEqual(p.stable,ref.strokes.slice(0,p.stable_count));
const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const intersect=xs=>[Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const hull=xs=>[Math.min(...xs.map(x=>x[0])),Math.max(...xs.map(x=>x[1]))];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];
function certAt(s,cut){
 const st=ref.prefixes[cut].stable,begin=5*s,up=s%2===0,mem=st.slice(begin,begin+5);
 if(mem.length!==5)return null;
 const feats=st.slice(begin).map((p,j)=>({...p,pen:begin+j,range:range(p)})).filter(p=>p.up!==up);
 let tri=null;
 for(let i=1;i<feats.length;i++){
  const a=feats[i-1].range,b=feats[i].range;
  if(contains(a,b)||contains(b,a))return null;
  if(i<2)continue;
  const aa=feats[i-2].range;
  if(up?a[1]>aa[1]&&a[1]>b[1]:a[0]<aa[0]&&a[0]<b[0]){tri=feats.slice(i-2,i+1);break;}
 }
 if(!tri)return null;
 const [a,b,c]=tri.map(p=>p.range);
 if(!(up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1]))return null;
 if(!(intersect([a,b])[0]<intersect([a,b])[1]))return null;
 const start=mem[0].start,end=mem.at(-1).end,whole=hull(mem.map(range)),ic=intersect(mem.slice(0,3).map(range));
 if(!(ic[0]<ic[1]))return null;
 if(prices[end]!==(up?b[1]:b[0]))return null;
 if(whole[0]!==Math.min(prices[start],prices[end])||whole[1]!==Math.max(prices[start],prices[end]))return null;
 return {id:`S${s}`,up,start,end,owned_events:[start+1,end],members:[begin,begin+5],range:whole,known_at:cut,first_selected:tri.map(p=>p.pen),selected_ranges:tri.map(p=>p.range),gap:false,initial_three_core:ic};
}
const segments=[];
for(let s=0;s<17;s++){
 let first=null;for(let k=0;k<prices.length;k++){const c=certAt(s,k);if(c&&!first)first=c;if(first){assert(c);const stablePart=x=>({...x,known_at:null});assert.deepEqual(stablePart(c),stablePart(first));}}
 if(first){assert.equal(first.known_at,38+20*s);assert.deepEqual(first.first_selected,[5*s+3,5*s+5,5*s+7]);segments.push(first);}
}
assert.equal(segments.length,16);
put('segments.json',segments);
const atom=j=>{const a=ref.strokes[j];return {stroke:j,start:a.start,end:a.end,dp:a.end_price-a.start_price,dt:observations[a.end].time-observations[a.start].time,v:(a.end_price-a.start_price)/(observations[a.end].time-observations[a.start].time)};};
const force=s=>{const first=atom(s.members[0]),last=atom(s.members[1]-1);return {first,last,L:last.v-first.v};};
const periods=[];
for(let p=0;p<3;p++){
 const off=p*5,b=segments[off],c=segments[off+4],members=segments.slice(off+1,off+4),K=intersect(members.map(s=>s.range)),Kouter=hull(members.map(s=>s.range));assert(K[0]<K[1]);
 const own=segments.slice(off,off+5),whole=hull(own.map(s=>s.range)),rawRange=[Math.min(...prices.slice(b.start,c.end+1)),Math.max(...prices.slice(b.start,c.end+1))];assert.deepEqual(rawRange,whole);
 const bf=force(b),cf=force(c),weak=cf.L<bf.L,extreme=b.up?prices[c.end]>prices[b.end]:prices[c.end]<prices[b.end];
 const roleRows=own.map((s,j)=>({id:s.id,core_member:j>0&&j<4,up:s.up,crosses_lower:s.range[0]<K[0]&&s.range[1]>=K[0],crosses_upper:s.range[0]<=K[1]&&s.range[1]>K[1]}));
 const eligible=roleRows.slice(0,4).filter(s=>!s.core_member&&s.up===c.up&&(s.crosses_lower||s.crosses_upper));assert.deepEqual(eligible.map(s=>s.id),[b.id]);
 const returnSegment=segments[off+5],nr=c.up?returnSegment.range[0]>K[1]:returnSegment.range[1]<K[0];assert(nr);assert(weak&&extreme);
 const geometryOnly={departure:c.id,return_segment:returnSegment.id,return_range:returnSegment.range,not_return:nr,happened_at:returnSegment.end,known_at:returnSegment.known_at,true_sublevel_roles:false,original_center_break:'unproved'};
 periods.push({id:`X${p}`,start:b.start,end:c.end,owned_events:[b.start+1,c.end],whole_range:whole,technical_direction:c.up?'Up':'Down',candidate_type:'P',core:{id:`K${p}`,members:members.map(s=>s.id),core:K,member_outer:Kouter,known_at:members.at(-1).known_at},comparison:{b:b.id,c:c.id,roles:roleRows,b_force:bf,c_force:cf,strict_weakening:weak,extreme},local_end:{happened_at:c.end,known_at:c.known_at,whole_general_divergence:'unproved'},no_return:geometryOnly,delayed_local_policy_known_at:Math.max(c.known_at,returnSegment.known_at)});
}
assert.deepEqual(periods.map(p=>p.core.core),[[25000,30000],[33000,34400],[30400,32000]]);
assert.deepEqual(periods.map(p=>p.whole_range),[[20000,36000],[31000,36000],[30000,36400]]);
assert.deepEqual(periods.map(p=>[p.comparison.b_force.L,p.comparison.c_force.L]),[[500,-250],[-550,-600],[50,-100]]);
const parentCore=intersect(periods.map(p=>p.whole_range));assert.deepEqual(parentCore,[31000,36000]);
// Explicitly preserve the same-level complete-expression obstruction.
const relations={src_next:[['X0','X1'],['X1','X2']],candidate_F2_member_next:[['X0','X1'],['X1','X2']],delta_op0:['X0','X1','X2'],delta_c0_if_identity_preserving:['X0','X1','X2'],delta_c0_adj:[['X0','X1'],['X1','X2']],NoPP_if_three_completed_P:false,canonical_complete_expression:null,active_parent:'Y1-candidate',completed_parent:null,F2_semantic_eligibility:'unproved',conditional_parent_core:parentCore};
put('periods.json',periods);put('relations.json',relations);
const timeline=ref.prefixes.map(p=>({cut:p.cut,segments:segments.filter(s=>s.known_at<=p.cut).map(s=>s.id),local_PB_ends:periods.filter(x=>x.local_end.known_at<=p.cut).map(x=>({id:x.id,event:x.end,published_at:x.local_end.known_at})),delayed_no_return_ends:periods.filter(x=>x.delayed_local_policy_known_at<=p.cut).map(x=>({id:x.id,event:x.end,published_at:x.delayed_local_policy_known_at})),certified_original_P:[],certified_original_center_break:[]}));
put('timeline.json',timeline);
const summary={status:'finite-source-local-comparisons-and-no-return-geometry-verified',counts:{observations:prices.length,events:events.length,template_legs:85,actual_reference_strokes:ref.strokes.length,stable_at_eof:ref.prefixes.at(-1).stable_count,certified_segments:segments.length,checked_prefixes:ref.prefixes.length},periods:periods.map(p=>({id:p.id,range:p.whole_range,core:p.core.core,Lb:p.comparison.b_force.L,Lc:p.comparison.c_force.L,event_end:p.end,local_known:p.local_end.known_at,no_return_known:p.no_return.known_at})),conditional_parent_core:parentCore,completed_original_P:0,completed_parent:0,full_joint_model:false,known_obligations:['GeneralDiv and whole identity','true sublevel roles for S-3','core catalogue and lifecycle beyond named window','full decomposition and NoPP scope','fixed F2 semantic input eligibility'],boundary:{external_left_support:[0,1],candidate_owned_events:[2,301],active_tail_events:[302,342]}};
put('summary.json',summary);
const sourcePaths=['.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs','rust/src/theta_v0/types.rs','.chanlun/definitions/beichi.md','.chanlun/definitions/zhongshu.md','docs/chanlun/text/blog/067-第67课.md','docs/chanlun/text/blog/078-第78课.md','docs/chanlun/text/blog/081-第81课.md'];
const artifacts=fs.readdirSync(E).sort().map(n=>({path:n,bytes:fs.statSync(path.join(E,n)).size,sha256:hash(path.join(E,n))}));const total=artifacts.reduce((a,b)=>a+b.bytes,0);assert(total<96*1024*1024);
put('run-manifest.json',{inputs:['template-v1.md','template-v2.md','build_check.mjs','reference_probe.rs'].map(n=>({path:path.join(here,n),sha256:hash(path.join(here,n))})),sources:sourcePaths.map(n=>({path:n,sha256:hash(path.join(R,n))})),artifacts,total_bytes:total});
console.log(JSON.stringify(summary));
