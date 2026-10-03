// #1467：给既有R_W结构附具名原生状态坐标，不重分解、不填完成标记。
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {fileURLToPath} from 'node:url';
const [publishedArg,nativeArg,outArg]=process.argv.slice(2);
assert.equal(process.argv.length,5,'usage: node stage29_native_time.mjs PUBLISHED_ROOT NATIVE_ROOT NEW_OUTPUT_DIRECTORY');
const published=await fs.realpath(publishedArg),native=await fs.realpath(nativeArg),out=path.resolve(outArg);
const repo=await fs.realpath(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..'));
const parent=await fs.realpath(path.dirname(out));assert(parent!==repo&&!parent.startsWith(repo+path.sep));await fs.mkdir(out,{mode:0o700});
const sha=b=>createHash('sha256').update(b).digest('hex');
const read=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const lines=b=>{const a=b.toString('utf8').split('\n');assert.equal(a.pop(),'');return a;};
const ns=s=>{const m=/^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?Z$/.exec(s);assert(m,'invalid time encoding');const ms=Date.parse(m[1]+'Z');assert(Number.isSafeInteger(ms));return BigInt(ms)*1000000n+BigInt((m[2]??'').padEnd(9,'0'));};
function gcd(a,b){a=a<0n?-a:a;while(b){[a,b]=[b,a%b];}return a;}
function rational(n,d){assert(d>0n);const g=gcd(n,d);return {numerator:n/g+'',denominator:d/g+'',unit:'price_tick_per_nanosecond'};}
function bind(o,p,n){
  assert.equal(o.source_line,p.source_line);assert.equal(n.line,p.source_line);assert.equal(n.status,'ready');assert.equal(o.frontier,n.frontier,'frontier binding');assert.deepEqual(o.quote,n.current_quote,'L1 binding');assert.equal(o.capture,p.capture_time_lower_bound);assert.equal(o.capture,n.available_at_capture);
  const app=n.applications.filter(a=>a.type==='applied').at(-1);
  if(!app)return {status:'unknown',reason:'no_frontier_message_time',ns:null};
  assert.equal(app.native_sequence,o.frontier,'application frontier binding');assert.equal(app.available_line,p.source_line,'application availability');assert(app.captured_line<=p.source_line,'future application source');
  if(app.native_event_at===null)return {status:'unknown',reason:'missing_native_time',ns:null};
  const t=ns(app.native_event_at);
  return {status:'recorded',native_event_at:app.native_event_at,ns:t.toString(),native_type:app.native_type,state_sequence:app.native_sequence,original_capture_line:app.captured_line,available_line:app.available_line};
}
function point(e,p,label,cutoff){const o=e.observations[p.observation];assert(o,'endpoint outside current observation prefix');assert(p.source_line<=cutoff,'future endpoint');assert.deepEqual(p,{capture_time_lower_bound:o.capture,frontier:o.frontier,observation:o.index,source_line:o.source_line,value_ticks:o[label]},'endpoint source binding');return o;}
function rateSpan(e,start,end,label){
  const a=e.observations[start],b=e.observations[end];assert(a&&b);assert(start<=end);
  if(a.clock.ns===null||b.clock.ns===null)return {status:'undefined',reason:'missing_endpoint_time'};
  let prev=null;for(let i=start;i<=end;i++){const t=e.observations[i].clock.ns;if(t===null)return {status:'undefined',reason:'missing_interior_time'};const n=BigInt(t);if(prev!==null&&n<prev)return {status:'undefined',reason:'clock_reversal_within_span'};prev=n;}
  const dt=BigInt(b.clock.ns)-BigInt(a.clock.ns);if(dt===0n)return {status:'undefined',reason:'zero_native_duration'};if(dt<0n)return {status:'undefined',reason:'negative_native_duration'};
  const dp=BigInt(b[label])-BigInt(a[label]);return {status:'defined',...rational(dp,dt),price_delta_ticks:dp.toString(),duration_ns:dt.toString(),start_time:a.clock.native_event_at,end_time:b.clock.native_event_at};
}
function bump(counts,r){const k=r.status==='defined'?'defined':r.reason;counts[k]=(counts[k]??0)+1;}
const synthetic=(times,values)=>({observations:times.map((t,i)=>({clock:{ns:t===null?null:String(t)},weighted:String(values[i])}))});
const controls=[];
for(const [name,times,expected]of [['missing',[null,3],'missing_endpoint_time'],['same-time',[3,3],'zero_native_duration'],['backward',[3,2],'clock_reversal_within_span'],['interior-backward',[1,4,3,5],'clock_reversal_within_span'],['interior-missing',[1,null,5],'missing_interior_time']]){const r=rateSpan(synthetic(times,times.map((_,i)=>100+i)),0,times.length-1,'weighted');assert.equal(r.status,'undefined');assert.equal(r.reason,expected);controls.push({name,result:expected});}
assert.deepEqual(rateSpan(synthetic([1,4],[100,102]),0,1,'weighted').numerator,'2');assert.equal(rateSpan(synthetic([1,4],[100,102]),0,1,'weighted').denominator,'3');
const sourceControls=[];
async function inspect(name,pdir,ndir){
  const pb=await fs.readFile(path.join(pdir,'rows.jsonl')),pr=await read(path.join(pdir,'report.json'));
  assert.equal(sha(pb),pr.output_file_sha256);
  const ar=await read(path.join(ndir,'rows-archive.json')),nr=await read(path.join(ndir,'report.json')),packed=await fs.readFile(ar.archive_path),nb=gunzipSync(packed);assert.equal(sha(packed),ar.archive_sha256);assert.equal(sha(nb),ar.original_sha256);assert.equal(sha(nb),nr.output_file_sha256);
  const ps=lines(pb),nsrc=lines(nb);assert.equal(ps.length,nsrc.length);
  const epochs=new Map(),records=[],stats={observations:0,missing_times:0,tied_adjacent_times:0,reversed_adjacent_times:0,weighted:{confirmed:0,confirmed_rates:{},active_checks:0,active_rates:{}},midpoint:{confirmed:0,confirmed_rates:{},active_checks:0,active_rates:{}}};let controlsDone=false;
  for(let i=0;i<ps.length;i++){
    const p=JSON.parse(ps[i]),n=JSON.parse(nsrc[i]);assert.equal(p.source_line,i+1);assert.equal(n.line,i+1);
    if(p.ended_scope)records.push({kind:'domain_end',available_line:p.source_line,epoch:p.ended_scope.epoch,tail_status:p.ended_scope.tail_status});
    if(p.epoch===null)continue;
    if(!epochs.has(p.epoch))epochs.set(p.epoch,{observations:[],priorClock:null,weighted:{},midpoint:{}});
    const e=epochs.get(p.epoch),o=p.new_observation;
    if(o){assert.equal(o.index,e.observations.length);const clock=bind(o,p,n);const observation={...o,clock};e.observations.push(observation);stats.observations++;
      if(clock.ns===null)stats.missing_times++;else if(e.priorClock!==null){const t=BigInt(clock.ns);if(t===e.priorClock)stats.tied_adjacent_times++;if(t<e.priorClock)stats.reversed_adjacent_times++;}e.priorClock=clock.ns===null?null:BigInt(clock.ns);
      records.push({kind:'observation_clock',epoch:p.epoch,index:o.index,source_line:o.source_line,frontier:o.frontier,clock});
      if(!controlsDone&&name==='baseline'){
        const badFrontier=structuredClone(o);badFrontier.frontier='0';assert.throws(()=>bind(badFrontier,p,n),assert.AssertionError);sourceControls.push({name:'frontier-mismatch',rejected:true});
        const badQuote=structuredClone(o);badQuote.quote.bid_quantity='0';assert.throws(()=>bind(badQuote,p,n),assert.AssertionError);sourceControls.push({name:'quote-mismatch',rejected:true});
        const badNative=structuredClone(n);badNative.applications.filter(a=>a.type==='applied').at(-1).captured_line=p.source_line+1;assert.throws(()=>bind(o,p,badNative),assert.AssertionError);sourceControls.push({name:'future-source-line',rejected:true});controlsDone=true;
      }
    }
    for(const label of ['weighted','midpoint']){
      for(const u of p[label+'_confirmed']){
        assert.equal(u.epoch,p.epoch);assert(o);assert.equal(u.confirmed_at.observation,o.index);assert.equal(u.confirmed_at.source_line,p.source_line);assert.equal(u.confirmed_at.capture_time_lower_bound,p.capture_time_lower_bound);assert(u.start.observation<u.end.observation&&u.end.observation<u.confirmed_at.observation);
        point(e,u.start,label,p.source_line);point(e,u.end,label,p.source_line);const r=rateSpan(e,u.start.observation,u.end.observation,label);stats[label].confirmed++;bump(stats[label].confirmed_rates,r);
        if(r.status==='defined')assert(u.direction==='up'?BigInt(r.numerator)>0n:BigInt(r.numerator)<0n);
        records.push({kind:'confirmed_unit_rate',epoch:p.epoch,label,ordinal:u.ordinal,available_line:p.source_line,available_capture:p.capture_time_lower_bound,start_observation:u.start.observation,end_observation:u.end.observation,direction:u.direction,rate:r,scope:'numeric candidate rate only; not canonical Stroke or Move completion'});
      }
      const a=p[label+'_active'];assert.equal(a.provisional,true);stats[label].active_checks++;
      let result;
      if(a.phase==='undecided'){result={status:'undefined',reason:'direction_not_initialized'};}else{point(e,a.start,label,p.source_line);point(e,a.extreme,label,p.source_line);result=rateSpan(e,a.start.observation,a.extreme.observation,label);}
      bump(stats[label].active_rates,result);const signature=JSON.stringify({a,rate:result});if(signature!==e[label].lastActive){e[label].lastActive=signature;records.push({kind:'active_rate_update',epoch:p.epoch,label,available_line:p.source_line,provisional:true,phase:a.phase,start_observation:a.start?.observation??null,end_observation:a.extreme?.observation??null,rate:result});}
    }
  }
  assert.equal(stats.observations,pr.observations);assert.equal(stats.weighted.confirmed,pr.weighted_completed);assert.equal(stats.midpoint.confirmed,pr.midpoint_completed);assert.equal(epochs.size,pr.epochs_started);
  const output=records.map(r=>JSON.stringify(r)).join('\n')+'\n';assert(!/"(?:order_id|client_oid|maker_order_id|taker_order_id)"/.test(output));assert(Buffer.byteLength(output)<=16*1048576);
  const rowsPath=path.join(out,name+'-rates.jsonl');await fs.writeFile(rowsPath,output,{flag:'wx',mode:0o600});
  return {name,epochs:epochs.size,...stats,output:{path:rowsPath,bytes:Buffer.byteLength(output),sha256:sha(output),records:records.length},inputs:{published_rows:path.join(pdir,'rows.jsonl'),published_rows_sha256:sha(pb),published_report_sha256:sha(await fs.readFile(path.join(pdir,'report.json'))),native_archive:ar.archive_path,native_archive_sha256:sha(packed),native_rows_sha256:sha(nb),native_report_sha256:sha(await fs.readFile(path.join(ndir,'report.json')))},scope:'same original DC output, native state timestamps from admitted records, exact rational candidate rates'};
}
try{
  const baseline=await inspect('baseline',published,path.join(native,'baseline'));
  const gap=await inspect('long-gap',path.join(published,'verification-1/long-gap'),path.join(native,'verification-1/long-gap'));
  const summary={profile:'RW-DC-native-state-time/1',passed:true,baseline,gap,undefined_controls:controls,source_controls:sourceControls,checker_sha256:sha(await fs.readFile(fileURLToPath(import.meta.url))),semantic:'not_reviewed',limits:['one historical minute and one delayed capture perturbation','native message time is an explicit candidate coordinate, not a proved physical or canonical Chan clock','no new F1 decomposition or completion status','no canonical atom, full F2 or market value qualification']};
  await fs.writeFile(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n',{flag:'wx',mode:0o600});
  console.log(JSON.stringify({passed:true,baseline:{observations:baseline.observations,missing:baseline.missing_times,ties:baseline.tied_adjacent_times,reversals:baseline.reversed_adjacent_times,weighted:baseline.weighted.confirmed_rates,midpoint:baseline.midpoint.confirmed_rates},gap:{observations:gap.observations,weighted:gap.weighted.confirmed_rates,midpoint:gap.midpoint.confirmed_rates},summary:path.join(out,'summary.json')}));
}catch(e){await fs.writeFile(path.join(out,'failure.json'),JSON.stringify({passed:false,error:String(e)},null,2)+'\n',{flag:'wx'});throw e;}
