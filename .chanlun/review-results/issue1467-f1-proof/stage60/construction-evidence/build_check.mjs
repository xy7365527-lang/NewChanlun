import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const R=path.resolve(here,'../../../../..');
const E=process.argv[2];
if(!E) throw new Error('Usage: node build_check.mjs <new absolute output directory>');
if(!path.isAbsolute(E)) throw new Error('output must be absolute');
fs.mkdirSync(E,{recursive:true});
const put=(name,x)=>fs.writeFileSync(path.join(E,name),JSON.stringify(x,null,2)+'\n');
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const run=(name,cmd,args)=>{
 const r=spawnSync(cmd,args,{encoding:'utf8',maxBuffer:16*1024*1024});
 fs.writeFileSync(path.join(E,name+'.stdout'),r.stdout??'');fs.writeFileSync(path.join(E,name+'.stderr'),r.stderr??'');
 put(name+'.receipt.json',{command:[cmd,...args],status:r.status,signal:r.signal,error:r.error?.message??null});
 assert.equal(r.status,0,`${name}: ${r.stderr}`);return r.stdout;
};
let A=25n; const gcd=(a,b)=>b?gcd(b,a%b):a;let lcm=1n;
for(let d=4n;d<=324n;d++) lcm=lcm/gcd(lcm,d)*d; A*=lcm;
const templates=[
{name:'T60-main',rows:[[100,120,110,150,130,160],[160,145,155,130,140,120],[120,135,125,145,130,150],[150,140,148,130,140,125],[125,135,130,178,175,180],[180,176,179,170,175,140]]},
{name:'N60-equal-force',rows:[[100,120,110,150,130,160],[160,145,155,130,140,120],[120,135,125,145,130,150],[150,140,148,130,140,125],[125,135,130,178,160,180],[180,176,179,170,175,140]]}
];
const sourcePaths=[
'.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs',
'rust/src/theta_v0/types.rs',
'formal/Origin/SegmentFeatureSeq.lean','formal/Origin/SegmentFeatureComplete.lean','formal/Origin/SegmentAutoConstruct.lean',
'formal/Origin/CenterComplete.lean',
'.chanlun/definitions/beichi.md','.chanlun/definitions/zhongshu.md',
'docs/chanlun/text/blog/053-第53课.md','docs/chanlun/text/blog/057-第57课.md','docs/chanlun/text/blog/061-第61课.md','docs/chanlun/text/blog/067-第67课.md','docs/chanlun/text/blog/078-第78课.md','docs/chanlun/text/blog/081-第81课.md','docs/chanlun/text/blog/084-第84课.md'
];
const sources=sourcePaths.map(p=>({path:p,sha256:hash(path.join(R,p))}));
put('source-identities.json',sources);
const lib='/tmp/nc1467-cargo-target/debug/deps';
const newchan=path.join(lib,'libnewchan_rust.rlib');
const serde=path.join(lib,'libserde_json-d8aa09aa6d50fe2e.rlib');
put('runtime-identities.json',{node:process.version,dependencies:[newchan,serde].map(p=>({path:p,sha256:hash(p)})),plan_sha256:hash(path.join(here,'template-v1.md'))});
const binary=path.join(E,'reference-probe');
const pythonLib='/Users/silencehan/.local/share/uv/python/cpython-3.12.13-macos-aarch64-none/lib';
run('compile','rustc',[path.join(here,'reference_probe.rs'),'--edition=2021','-L',`dependency=${lib}`,'-L',`native=${pythonLib}`,'-C',`link-arg=-Wl,-rpath,${pythonLib}`,'--extern',`newchan_rust=${newchan}`,'--extern',`serde_json=${serde}`,'-o',binary]);
const range=s=>[Math.min(s.start_price,s.end_price),Math.max(s.start_price,s.end_price)];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];
const intersectsStrict=(a,b)=>Math.max(a[0],b[0])<Math.min(a[1],b[1]);
const segmentCertificate=(x,ref,s,cut)=>{
 const startPen=5*s,endPen=startPen+5; const ss=ref.prefixes[cut].stable;
 const members=ss.slice(startPen,endPen); if(members.length!==5)return null;
 const up=s%2===0; const features=ss.slice(startPen).map((p,j)=>({...p,pen:startPen+j,range:range(p)})).filter(p=>p.up!==up);
 for(let j=1;j<features.length;j++){
  if(contains(features[j-1].range,features[j].range)||contains(features[j].range,features[j-1].range))return null;
 }
 let tri=null;
 for(let j=1;j+1<features.length;j++){
  const [a,b,c]=features.slice(j-1,j+2).map(e=>e.range);
  const formalFractal=up?b[1]>a[1]&&b[1]>c[1]:b[0]<a[0]&&b[0]<c[0];
  if(formalFractal){tri=features.slice(j-1,j+2);break;}
 }
 if(!tri)return null;
 const [a,b,c]=tri.map(t=>t.range);
 const strong=up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1];
 if(!strong||!intersectsStrict(a,b))return null;
 const start=members[0].start,end=members.at(-1).end;
 const start_price=x.observations[start].price,end_price=x.observations[end].price;
 const bounds=[Math.min(start_price,end_price),Math.max(start_price,end_price)];
 if(end_price!==(up?b[1]:b[0]))return null;
 if(!(up?end_price>start_price:end_price<start_price))return null;
 const initial=members.slice(0,3).map(range);
 const initialCore=[Math.max(...initial.map(t=>t[0])),Math.min(...initial.map(t=>t[1]))];
 if(!(initialCore[0]<initialCore[1]))return null;
 if(!members.every(p=>range(p)[0]>=bounds[0]&&range(p)[1]<=bounds[1]))return null;
 return {id:`${x.name}/S${s}`,up,members:[startPen,endPen],source:{observations:[start,end],owned_events:[start+1,end],shared_initial_state:start},start,end,start_price,end_price,range:bounds,known_at:cut,
 feature_strokes:features.map(t=>t.pen),features:features.map(t=>t.range),first_selected:tri.map(t=>t.pen),selected_ranges:tri.map(t=>t.range),gap:false,strong_fractal:true,initial_three_core:initialCore,
 evidence_events:[end+1,cut],source_rule:'067:18,20(author part),22,28(author part); actual stable R_W-v1 strokes; no adjacent containment'};
};
const outputs=[];
for(const template of templates){
 const pivots=[template.rows[0][0]*100];
 for(const row of template.rows){assert.equal(row[0]*100,pivots.at(-1));pivots.push(...row.slice(1).map(p=>p*100));}
 const prices=[10050,pivots[0]];
 for(let j=0;j+1<pivots.length;j++)for(let k=1;k<=4;k++) prices.push(pivots[j]+(pivots[j+1]-pivots[j])*k/4);
 prices.push(14025); assert.equal(prices.length,123);
 const quantities=prices.map(p=>{assert(Number.isInteger(p)&&p>=10000&&p<=18000&&p%25===0);const n=A*BigInt(p-9900),d=BigInt(18100-p);assert.equal(n%d,0n);return n/d;});
 const observations=prices.map((p,i)=>({index:i,time:i,price:p,bid_quantity:quantities[i].toString(),ask_quantity:A.toString(),trade_volume:0,untradable:true}));
 const events=quantities.slice(1).map((q,j)=>{const d=q-quantities[j];assert.notEqual(d,0n);return {id:j+1,time:j+1,side:'bid',price:9900,action:d>0n?'add':'cancel',amount:(d>0n?d:-d).toString(),quantity_before:quantities[j].toString(),quantity_after:q.toString()};});
 const x={name:template.name,profile:'Omega60-PB-v1',clock:'t_i=i; shared unit sampling clock; not event-count clock inferred after observation',bid:9900,ask:18100,ask_quantity:A.toString(),initial:{time:0,bid_quantity:quantities[0].toString(),ask_quantity:A.toString()},pivots,observations,events};
 // Replay actual order events, deriving each observation independently of stored p.
 let q=quantities[0];for(let i=0;i<observations.length;i++){
  if(i){const e=events[i-1],amount=BigInt(e.amount);assert.equal(q.toString(),e.quantity_before);if(e.action==='cancel')assert(amount<q);q+=e.action==='add'?amount:-amount;assert.equal(q.toString(),e.quantity_after);}
  assert(q>0n); const num=18100n*q+9900n*A,den=q+A;assert.equal(num%den,0n);assert.equal(Number(num/den),prices[i]);
 }
 put(template.name+'.source.json',x);
 const ref=JSON.parse(run(template.name+'.reference',binary,[path.join(E,template.name+'.source.json')]));
 assert.equal(ref.groups,123);assert.equal(ref.strokes.length,30);
 ref.strokes.forEach((s,j)=>{assert.equal(s.start,1+4*j);assert.equal(s.end,5+4*j);assert.equal(s.start_price,prices[s.start]);assert.equal(s.end_price,prices[s.end]);assert.equal(s.up,j%2===0);assert.equal(s.end-s.start-1,3);assert(s.up?s.start_price<s.end_price:s.start_price>s.end_price);});
 // Each stable prefix must literally match the full concrete result; no future endpoint substituted.
 for(const p of ref.prefixes)assert.deepEqual(p.stable,ref.strokes.slice(0,p.stable_count));
 const cuts=[38,58,78,98,118]; const certs=cuts.map((cut,s)=>segmentCertificate(x,ref,s,cut));assert(certs.every(Boolean));
 certs.forEach((s,j)=>{assert.equal(s.first_selected.join(','),[5*j+3,5*j+5,5*j+7].join(','));if(j){assert.equal(certs[j-1].end,s.start);assert.equal(certs[j-1].end_price,s.start_price);}});
 const K=[Math.max(...certs.slice(1,4).map(s=>s.range[0])),Math.min(...certs.slice(1,4).map(s=>s.range[1]))];assert.deepEqual(K,[12500,15000]);
 const crossedUpper=s=>s.start_price<=K[1]&&s.end_price>K[1]&&s.up;
 const preceding=certs.slice(0,4).filter(crossedUpper);assert.equal(preceding.at(-1).id,certs[0].id);assert(crossedUpper(certs[4]));
 const core={id:template.name+'/K',view:'057 minimum segment viewpoint with seed S1,S2,S3; declared analysis grouping, not fixed F2 tower',members:[1,2,3],core:K,whole:[12000,16000],happened_by:81,known_by:98,earlier_overlapping_seed:{members:[0,1,2],core:[12000,15000],claimed_distinct_same_level_center:false}};
 const force=s=>{const first=ref.strokes[s.members[0]],last=ref.strokes[s.members[1]-1];const atom=a=>({stroke:ref.strokes.indexOf(a),start:a.start,end:a.end,dp:a.end_price-a.start_price,dt:observations[a.end].time-observations[a.start].time,v:(a.end_price-a.start_price)/(observations[a.end].time-observations[a.start].time)});const a=atom(first),z=atom(last);assert.equal(a.dt,4);assert.equal(z.dt,4);return {first:a,last:z,L:z.v-a.v};};
 const b=force(certs[0]),c=force(certs[4]);assert.equal(b.L,250);assert.equal(c.L,template.name==='T60-main'?-125:250);
 const returnObs=observations.find(o=>o.index>101&&o.price>=K[0]&&o.price<=K[1]);
 const short=segmentCertificate(x,ref,4,114);assert.equal(short,null);
 const cert={template:template.name,observations:observations.length,events:events.length,reference_strokes:ref.strokes.length,at_eof:{stable:ref.prefixes.at(-1).stable_count,active:ref.prefixes.at(-1).active,unconfirmed_segment:'S5'},segments:certs,core,
 pair:{b:'S0',c:'S4',direction:'up',K:core.id,ordered_nonoverlap:true,preceding_same_direction_crosses:preceding.map(s=>s.id),internal_same_direction:{id:'S2',max:certs[2].end_price,upper_boundary:K[1],strictly_crosses:false},extreme:{previous_high:16000,new_high:18000,pass:true},force:{b,c,difference:c.L-b.L,strict_weakening:c.L<b.L}},
 endings:{certified_object:'S4',happened_at:101,known_at:118,via:'067 first no-gap case; not derived from EOF or comparison',return_to_K:{observation:returnObs.index,price:returnObs.price},whole_P:{candidate_source:[1,101],claim:'not certified',first_missing_rule:'PanDiv in the declared minimum viewpoint implies exact completion of this whole b+K+c: not supplied by 053:24/057:40 or 067 segment-end rule'}},
 negative_prefix:{cut:114,stable_strokes:ref.prefixes[114].stable_count,needed_feature:27,feature_is_stable:ref.prefixes[114].stable_count>27,segment_certificate:short}};
 put(template.name+'.certificate.json',cert);outputs.push(cert);
}
assert(outputs[0].pair.force.strict_weakening);assert(!outputs[1].pair.force.strict_weakening);
assert.deepEqual(outputs[0].segments.map(s=>({start:s.start,end:s.end,known:s.known_at,first:s.first_selected})),outputs[1].segments.map(s=>({start:s.start,end:s.end,known:s.known_at,first:s.first_selected})));
put('summary.json',{status:'finite-source-segments-and-local-pan-comparison-passed',completed_whole_P:false,templates:outputs.map(x=>({name:x.template,counts:[x.observations,x.events,x.reference_strokes,x.segments.length],core:x.core.core,Lb:x.pair.force.b.L,Lc:x.pair.force.c.L,weakening:x.pair.force.strict_weakening,end:x.endings,negative_prefix:x.negative_prefix}))});
const artifacts=fs.readdirSync(E).filter(n=>n!=='run-manifest.json').sort().map(n=>({path:n,bytes:fs.statSync(path.join(E,n)).size,sha256:hash(path.join(E,n))}));
const inputs=['template-v1.md','reference_probe.rs','build_check.mjs'].map(n=>({path:path.join(here,n),sha256:hash(path.join(here,n))}));
const total=artifacts.reduce((s,a)=>s+a.bytes,0);assert(total<128*1024*1024);
put('run-manifest.json',{inputs,sources,artifacts,total_bytes:total,budget_bytes:128*1024*1024});
console.log(JSON.stringify({status:'passed',artifact_bytes:total,templates:outputs.map(x=>({name:x.template,Lb:x.pair.force.b.L,Lc:x.pair.force.c.L,completed_whole_P:false}))}));
