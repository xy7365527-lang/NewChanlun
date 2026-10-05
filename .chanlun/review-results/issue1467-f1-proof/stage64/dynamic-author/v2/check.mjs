// Scope-specific D64-035-conservative-v2; no production admission interface.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const H=path.dirname(fileURLToPath(import.meta.url));
const R='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const E=process.argv[2];assert(E&&path.isAbsolute(E)&&!fs.existsSync(E));fs.mkdirSync(E,{recursive:true});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const put=(n,x)=>fs.writeFileSync(path.join(E,n),JSON.stringify(x,null,2)+'\n',{flag:'wx'});
const run=(n,cmd,args)=>{let begin=Date.now();let r=spawnSync(cmd,args,{encoding:'utf8',timeout:60000,maxBuffer:40*1024*1024});fs.writeFileSync(path.join(E,n+'.stdout'),r.stdout??'',{flag:'wx'});fs.writeFileSync(path.join(E,n+'.stderr'),r.stderr??'',{flag:'wx'});put(n+'.receipt.json',{argv:[cmd,...args],status:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:Date.now()-begin});assert.equal(r.status,0);return r.stdout;};
const ends=[100,160,120,150,125,180,170,178,172,200,190,198,192,197,145,155,148,153,95,115,100,110,102,170,160,168,162,205,185,200];
const settings=new Map([[4,[1,3]],[22,[1,3]],[8,[1,1.5]],[26,[1,1.5]],[12,[1,4]],[13,[3,1.5]],[17,[1,4]]]);
const rows=ends.slice(0,-1).map((a,s)=>{let z=ends[s+1],d=Math.sign(z-a),[f,l]=settings.get(s)??[1,2];return [a,a+d*f,a+d*f/2,z-d*.5,z-d*l,z].map(x=>x*400);});
const pivots=[rows[0][0]];for(const row of rows){assert.equal(pivots.at(-1),row[0]);pivots.push(...row.slice(1));}
const prices=[40200,pivots[0]];for(let j=0;j+1<pivots.length;j++)for(let k=1;k<=4;k++)prices.push(pivots[j]+(pivots[j+1]-pivots[j])*k/4);prices.push(80100);
assert.equal(prices.length,583);assert(prices.every(p=>Number.isInteger(p)&&p%50===0));
const gcd=(a,b)=>b?gcd(b,a%b):a;let A=1n;for(let i=1n;i<=960n;i++)A=A/gcd(A,i)*i;
const qs=prices.map(p=>{let n=A*BigInt((p-36000)/50),d=BigInt((84000-p)/50);assert(n>0n&&d>0n&&n%d===0n);return n/d;});
const observations=prices.map((p,i)=>({index:i,time:i,price:p,bid_quantity:String(qs[i]),ask_quantity:String(A),trade_volume:0,untradable:true}));
const events=qs.slice(1).map((q,j)=>{let d=q-qs[j];assert(d!==0n);return {id:j+1,time:j+1,side:'bid',price:36000,action:d>0n?'add':'cancel',amount:String(d>0n?d:-d),quantity_before:String(qs[j]),quantity_after:String(q)};});
let q=qs[0];for(let i=0;i<prices.length;i++){if(i){let e=events[i-1],a=BigInt(e.amount);assert.equal(String(q),e.quantity_before);assert(e.action!=='cancel'||a<q);q+=e.action==='add'?a:-a;assert.equal(String(q),e.quantity_after);}let n=84000n*q+36000n*A,d=q+A;assert.equal(n%d,0n);assert.equal(Number(n/d),prices[i]);}
const source={name:'T64-H2-dynamic-repair',profile:'D64-035-conservative-H2',bid:36000,ask:84000,ask_quantity:String(A),initial:{time:0,bid_quantity:String(qs[0]),ask_quantity:String(A)},rows,pivots,observations,events};put('source.json',source);
const lib='/tmp/nc1467-cargo-target/debug/deps',pylib='/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib';
const deps=[path.join(lib,'libnewchan_rust.rlib'),path.join(lib,'libserde_json-d8aa09aa6d50fe2e.rlib')];put('runtime.json',{node:process.version,dependencies:deps.map(p=>({path:p,sha256:hash(p)}))});
const binary=path.join(E,'reference-probe');run('compile','rustc',[path.join(H,'reference_probe.rs'),'--edition=2021','-L',`dependency=${lib}`,'-L',`native=${pylib}`,'-C',`link-arg=-Wl,-rpath,${pylib}`,'--extern',`newchan_rust=${deps[0]}`,'--extern',`serde_json=${deps[1]}`,'-o',binary]);
const ref=JSON.parse(run('reference',binary,[path.join(E,'source.json')]));assert.equal(ref.groups,583);assert.equal(ref.strokes.length,144);
for(const [j,s]of ref.strokes.entries()){assert.equal(s.start,1+4*j);assert.equal(s.end,5+4*j);assert.equal(s.start_price,prices[s.start]);assert.equal(s.end_price,prices[s.end]);assert.equal(s.up,j%2===0);}for(const p of ref.prefixes)assert.deepEqual(p.stable,ref.strokes.slice(0,p.stable_count));
const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const ix=xs=>[Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const hull=xs=>[Math.min(...xs.map(x=>x[0])),Math.max(...xs.map(x=>x[1]))];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];
function certAt(s,cut){
 const st=ref.prefixes[cut].stable,begin=5*s,up=s%2===0,mem=st.slice(begin,begin+5);if(mem.length!==5)return null;
 const feats=st.slice(begin).map((p,j)=>({...p,pen:begin+j,range:range(p)})).filter(p=>p.up!==up);let tri=null;
 for(let i=1;i<feats.length;i++){const a=feats[i-1].range,b=feats[i].range;if(contains(a,b)||contains(b,a))return null;if(i<2)continue;const aa=feats[i-2].range;if(up?a[1]>aa[1]&&a[1]>b[1]:a[0]<aa[0]&&a[0]<b[0]){tri=feats.slice(i-2,i+1);break;}}
 if(!tri)return null;const[a,b,c]=tri.map(p=>p.range);if(!(up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1]))return null;
 if(!(ix([a,b])[0]<ix([a,b])[1]))return null;const start=mem[0].start,end=mem.at(-1).end,whole=hull(mem.map(range)),ic=ix(mem.slice(0,3).map(range));if(!(ic[0]<ic[1]))return null;
 if(prices[end]!== (up?b[1]:b[0]))return null;if(whole[0]!==Math.min(prices[start],prices[end])||whole[1]!==Math.max(prices[start],prices[end]))return null;
 return {id:`S${s}`,up,start,end,own:[start+1,end],members:[begin,begin+5],range:whole,known_at:cut,first_selected:tri.map(p=>p.pen),selected_ranges:tri.map(p=>p.range),gap:false,initial_three_core:ic};
}
const segments=[],segmentFailures=[];for(let s=0;s<29;s++){let first=null;for(let cut=0;cut<prices.length;cut++){let c=certAt(s,cut);if(c&&!first)first=c;if(first){assert(c);let strip=x=>({...x,known_at:null});assert.deepEqual(strip(c),strip(first));}}if(first)segments.push(first);else segmentFailures.push({segment:s,eof_pending:s===28,reason:'no declared strict no-gap certificate'});}
put('segments.json',segments);put('segment-failures.json',segmentFailures);
if(segments.length!==28){put('first-failure.json',{stage:'initial-segment-construction',expected:28,actual:segments.length,failures:segmentFailures});console.log(JSON.stringify({status:'initial-segment-construction-failed',segments:segments.length,failures:segmentFailures}));process.exit(0);}
for(const [s,c]of segments.entries()){assert.equal(c.id,`S${s}`);assert.equal(c.known_at,38+20*s);}
const force=s=>{let first=ref.strokes[s.members[0]],last=ref.strokes[s.members[1]-1];let v=p=>(p.end_price-p.start_price)/(observations[p.end].time-observations[p.start].time);return{first_pen:s.members[0],last_pen:s.members[1]-1,first_v:v(first),last_v:v(last),L:v(last)-v(first)};};
const cores=[],objects=[],attempts=[];let cursor=0;
while(cursor+4<segments.length){const startSeg=cursor,up=segments[cursor].up,ownedCores=[];let cidx=cursor+4,bidx=cursor,localFailure=null;
 while(cidx<segments.length){let mem=segments.slice(cidx-3,cidx),core=ix(mem.map(s=>s.range)),outer=hull(mem.map(s=>s.range));let K={id:`K${cores.length}`,grade:0,members:mem.map(s=>s.id),member_own:[mem[0].own[0],mem.at(-1).own[1]],core,initial_outer:outer,seed_happened_at:mem.at(-1).end,known_at:mem.at(-1).known_at,lifecycle:'seed_frozen; successor relation checked separately'};assert(core[0]<core[1]);
 if(ownedCores.length){let prev=ownedCores.at(-1);let separated=up?prev.initial_outer[1]<outer[0]:outer[1]<prev.initial_outer[0];if(!separated){localFailure={stage:'strict-initial-outer-order',previous:prev.id,next:K.id};break;}}
 cores.push(K);ownedCores.push(K);let b=segments[bidx],c=segments[cidx],bf=force(b),cf=force(c);let previousPrices=prices.slice(segments[startSeg].start,c.start+1);let priorExtreme=up?Math.max(...previousPrices):Math.min(...previousPrices);let extreme=up?prices[c.end]>priorExtreme:prices[c.end]<priorExtreme;
 let weak=cf.L<bf.L,holds=weak&&extreme&&b.up===c.up;let attempt={object_index:objects.length,cores:ownedCores.map(k=>k.id),b:b.id,c:c.id,b_force:bf,c_force:cf,strict_weakening:weak,strict_extreme:extreme,prior_extreme:priorExtreme,endpoint:prices[c.end],GeneralDiv64:holds,happened_at:c.end,known_at:c.known_at};attempts.push(attempt);
 if(holds){let ownSeg=segments.slice(startSeg,cidx+1),whole=hull(ownSeg.map(s=>s.range));let obj={id:`X${objects.length}`,grade:0,type:ownedCores.length===1?'P':up?'U':'D',technical_direction:up?'Up':'Down',start:ownSeg[0].start,end:c.end,own:[ownSeg[0].own[0],c.end],whole_range:whole,members:ownSeg.map(s=>s.id),cores:ownedCores.map(k=>k.id),general_div:attempt,completed:true,completion_status:'candidate interpretation only',original_completed_certified:false,source_next_start:c.end};objects.push(obj);cursor=cidx+1;break;}
 bidx=cidx;cidx+=4;
 }
 if(localFailure){put('scan-failure.json',localFailure);break;}if(cidx>=segments.length)break;
}
assert.deepEqual(objects.map(x=>x.type),['U','D','U']);assert.deepEqual(objects.map(x=>x.cores.length),[2,2,2]);
for(let i=0;i<objects.length;i++){let x=objects[i];assert.equal(x.start,1+180*i);assert.equal(x.end,181+180*i);assert.deepEqual(x.own,[2+180*i,181+180*i]);assert.equal(x.general_div.known_at,198+180*i);}
const source_next=objects.slice(0,-1).map((x,i)=>[x.id,objects[i+1].id]);
const violations=[];for(let i=0;i+1<objects.length;i++){let x=objects[i],last=cores.find(k=>k.id===x.cores.at(-1)),next=cores.find(k=>k.id===objects[i+1].cores[0]);let same=x.type==='U'?next.initial_outer[0]>last.initial_outer[1]:next.initial_outer[1]<last.initial_outer[0];if(same)violations.push({id:`V${violations.length}`,clause:'A64-035: completed trend iff next nucleus no longer same direction',completed_object:x.id,ended_at:x.end,end_known_at:x.general_div.known_at,last_core:last.id,last_outer:last.initial_outer,next_core:next.id,next_outer:next.initial_outer,next_core_known_at:next.known_at,continues_same_direction:true,contradiction_first_known_at:Math.max(x.general_div.known_at,next.known_at)});}
const parentCore=ix(objects.map(x=>x.whole_range));const parent={id:'Y0-conditional',grade:1,children:objects.map(x=>x.id),whole_ranges:objects.map(x=>x.whole_range),core:parentCore,strict_core:parentCore[0]<parentCore[1],source_contiguous:objects.every((x,i)=>i===0||x.start===objects[i-1].end),alternating_direction:true,candidate_children_completed:true,original_children_completed_certified:false,conditional_core_known_at:Math.max(...objects.map(x=>x.general_div.known_at)),completed:false,status:'F2 input blocked by failed common interpretation; range intersection is conditional only'};
const noPP=objects.every((x,i)=>i===0||x.type!=='P'||objects[i-1].type!=='P');assert(noPP);assert(parent.strict_core);assert.equal(violations.length,2);
// L(c,t) exists before c is confirmed. This diagnostic series never admits partial c.
const dynamics=attempts.map(a=>{const c=segments.find(s=>s.id===a.c);const first=ref.strokes[c.members[0]];return {c:c.id,points:ref.strokes.slice(c.members[0],c.members[1]).map(p=>({event:p.end,L:(p.end_price-p.start_price)/(p.end-p.start)-(first.end_price-first.start_price)/(first.end-first.start),segment_confirmed:p.end>=c.known_at,admitted:false}))};});
const timeline=observations.map(o=>({cut:o.index,segments:segments.filter(s=>s.known_at<=o.index).map(s=>s.id),candidate_completed:objects.filter(x=>x.general_div.known_at<=o.index).map(x=>x.id),nuclei:cores.filter(k=>k.known_at<=o.index).map(k=>k.id),violations:violations.filter(v=>v.contradiction_first_known_at<=o.index).map(v=>v.id),original_completed:[],original_F2_inputs:[]}));
const result={candidate:'D64-035-conservative-H2',counts:{observations:observations.length,events:events.length,actual_strokes:ref.strokes.length,stable_at_eof:ref.prefixes.at(-1).stable_count,segments:segments.length,candidate_completed_trends:objects.length,candidate_P:0,original_completed_certified:0,conditional_F2_triples:1,certified_F2_triples:0},initial_construction:{cores,objects,source_next,delta_c0:objects.map(x=>x.id),noPP},attempts,parent,violations,first_failure:violations[0],ownership:{left_support_events:[1,1],completed_candidate_window:[2,541],active_tail_events:[542,582],state_zero:'initial support; not event',confirmation_support_owned_by_successor:true},scope:{status:'refuted on one declared history relative to explicit A64-035 inheritance',not_refuted:['all initial F1 definitions','084 alternative lifecycle choices','fixed F2 semantics','original #873 force with full structural roles'],open:['original dynamic roles and whole semantics','P domain global compatibility and liveness','semantic uniqueness of raw decomposition','fixed F2 parent lifecycle and recursive closure','P1-P4']}};
put('result.json',result);put('dynamics.json',dynamics);put('timeline.json',timeline);put('first-failure.json',violations[0]);
const inputs=['WorkCard.md','WorkCard-v2.md','check.mjs','reference_probe.rs'].map(n=>({path:path.join(H,n),sha256:hash(path.join(H,n))}));const sourcePaths=['.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs','rust/src/theta_v0/types.rs','.chanlun/definitions/beichi.md','.chanlun/definitions/zhongshu.md','docs/chanlun/text/blog/035-第35课.md','docs/chanlun/text/blog/084-第84课.md','docs/chanlun/text/blog/018-第18课.md','docs/chanlun/text/blog/037-第37课.md','docs/chanlun/text/blog/033-第33课.md','docs/chanlun/text/blog/061-第61课.md'];
const artifacts=fs.readdirSync(E).map(n=>({path:n,bytes:fs.statSync(path.join(E,n)).size,sha256:hash(path.join(E,n))}));assert(artifacts.reduce((a,b)=>a+b.bytes,0)<96*1024*1024);put('manifest.json',{inputs,sources:sourcePaths.map(n=>({path:path.join(R,n),sha256:hash(path.join(R,n))})),artifacts});
console.log(JSON.stringify({counts:result.counts,first_failure:result.first_failure,objects:objects.map(x=>({id:x.id,type:x.type,own:x.own,range:x.whole_range,Lb:x.general_div.b_force.L,Lc:x.general_div.c_force.L,known:x.general_div.known_at})),parent}));
