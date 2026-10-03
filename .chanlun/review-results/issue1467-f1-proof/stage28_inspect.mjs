// #1467：只重读已验证行输出；不回放、不输出订单身份。
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {fileURLToPath} from 'node:url';
const [rootArg,outArg]=process.argv.slice(2);
assert.equal(process.argv.length,4,'usage: node stage28_inspect.mjs STAGE26_ROOT NEW_OUTPUT_DIRECTORY');
const root=await fs.realpath(rootArg),out=path.resolve(outArg);
const repo=await fs.realpath(path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../..'));
const parent=await fs.realpath(path.dirname(out));
assert(parent!==repo&&!parent.startsWith(repo+path.sep),'evidence must stay outside repository');
await fs.mkdir(out,{mode:0o700});
const sha=b=>createHash('sha256').update(b).digest('hex');
const json=async p=>JSON.parse(await fs.readFile(p,'utf8'));
const stamp=s=>{const m=/^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?Z$/.exec(s);assert(m);const ms=Date.parse(m[1]+'Z');assert(Number.isSafeInteger(ms));return BigInt(ms)*1000000n+BigInt((m[2]??'').padEnd(9,'0'));};
const key=q=>`${q.epoch}:${q.model_state}`;
const prices=q=>[BigInt(q.value.bid_price),BigInt(q.value.ask_price)];
const l1=q=>[q.value.bid_price,q.value.ask_price,q.value.bid_quantity,q.value.ask_quantity].join(':');
async function inspect(name,directory){
  const archive=await json(path.join(directory,'rows-archive.json'));
  const report=await json(path.join(directory,'report.json'));
  const packed=await fs.readFile(archive.archive_path),raw=gunzipSync(packed);
  assert.equal(sha(packed),archive.archive_sha256);assert.equal(sha(raw),archive.original_sha256);assert.equal(sha(raw),report.output_file_sha256);
  const lines=raw.toString('utf8').split('\n');assert.equal(lines.pop(),'');
  const quotes=new Map(),blocks=[];
  function put(q){if(!q)return;const k=key(q),old=quotes.get(k);if(old)assert.deepEqual(old,q);else quotes.set(k,q);}
  for(const line of lines){const r=JSON.parse(line);for(const a of r.applications){if(a.type==='anchor')put(a.quote);if(a.disposition==='mutation'){put(a.before_model_quote);put(a.after_model_quote);}}for(const b of r.confirmed){assert.equal(b.provisional,false);assert.equal(b.known_at_line,r.line);blocks.push(b);}}
  assert.equal(blocks.length,report.completed_blocks);
  let geometry=0,zeroLines=0,zeroNs=0,fixedPrices=0,fixedL1=0,edge2=0n,mid2=0n,spreadSum=0n;const examples=[];const logical=[];
  for(const b of blocks){assert.equal(b.finish-b.start,b.event_count);assert(b.finish>b.start);assert(b.known_at_line>=b.end_quote.available_line);assert.deepEqual(quotes.get(key(b.start_quote)),b.start_quote);assert.deepEqual(quotes.get(key(b.end_quote)),b.end_quote);
    const dl=b.end_quote.available_line-b.start_quote.available_line,dt=stamp(b.end_quote.available_capture)-stamp(b.start_quote.available_capture);assert(dl>=0);assert(dt>=0n);if(dl===0)zeroLines++;if(dt===0n)zeroNs++;
    logical.push({epoch:b.epoch,ordinal:b.ordinal,positive:b.positive,start:b.start,finish:b.finish,geometry:b.geometry,available_duration_lines:dl,available_duration_ns:dt.toString()});
    if(!b.geometry)continue;geometry++;
    const [bid0,ask0]=prices(b.start_quote),[bid1,ask1]=prices(b.end_quote);assert(bid0<ask0&&bid1<ask1);
    const start=BigInt(b.geometry.start_price),finish=BigInt(b.geometry.end_price),sgn=b.positive?1n:-1n;
    assert.equal(start,b.positive?bid0:ask0);assert.equal(finish,b.positive?ask1:bid1);
    const e=2n*sgn*(finish-start),m=sgn*(bid1+ask1-bid0-ask0),s=ask0-bid0+ask1-bid1;assert.equal(e,m+s);assert(e>0n&&m>=0n&&s>0n);edge2+=e;mid2+=m;spreadSum+=s;
    let allPrices=true,allL1=true;for(let i=b.start;i<=b.finish;i++){const q=quotes.get(`${b.epoch}:${i}`);assert(q?.value);const [bi,ai]=prices(q);allPrices&&=bi===bid0&&ai===ask0;allL1&&=l1(q)===l1(b.start_quote);}if(allPrices)fixedPrices++;if(allL1)fixedL1++;
    if(dt===0n&&examples.length<2)examples.push({epoch:b.epoch,ordinal:b.ordinal,start:b.start,finish:b.finish,available_line:b.end_quote.available_line,available_capture:b.end_quote.available_capture,start_price:start.toString(),end_price:finish.toString(),all_prices_fixed:allPrices});
  }
  assert.equal(edge2,mid2+spreadSum);
  return {name,input:{archive_path:archive.archive_path,archive_sha256:archive.archive_sha256,raw_sha256:archive.original_sha256,report_path:path.join(directory,'report.json'),report_sha256:sha(await fs.readFile(path.join(directory,'report.json'))),shape_chain_sha256:report.shape_chain_sha256},rows:lines.length,confirmed:blocks.length,geometry,zero_available_line_duration:zeroLines,zero_available_ns_duration:zeroNs,all_best_prices_fixed:fixedPrices,all_l1_fixed:fixedL1,selected_absolute_change_times_two:edge2.toString(),directed_midpoint_change_times_two:mid2.toString(),endpoint_spreads_sum:spreadSum.toString(),examples,logical};
}
const baseline=await inspect('baseline',path.join(root,'baseline'));
const gap=await inspect('long-gap',path.join(root,'verification-1/long-gap'));
assert.equal(baseline.input.shape_chain_sha256,gap.input.shape_chain_sha256);assert.equal(baseline.confirmed,gap.confirmed);
let changed=0;for(let i=0;i<baseline.logical.length;i++){const a=baseline.logical[i],b=gap.logical[i];const {available_duration_lines:al,available_duration_ns:an,...aa}=a,{available_duration_lines:bl,available_duration_ns:bn,...bb}=b;assert.deepEqual(aa,bb);if(al!==bl||an!==bn)changed++;}
for(const k of ['all_best_prices_fixed','all_l1_fixed','selected_absolute_change_times_two','directed_midpoint_change_times_two','endpoint_spreads_sum'])assert.equal(baseline[k],gap[k]);
delete baseline.logical;delete gap.logical;
const result={profile:'ORDER-CLOCK-EDGE-v1',passed:true,baseline,gap,logical_blocks_equal:true,changed_available_durations:changed,checker_sha256:sha(await fs.readFile(fileURLToPath(import.meta.url))),scope:'descriptive statistics over one already inspected minute and a delay perturbation; no complete Move, market value or semantic-review qualification'};
await fs.writeFile(path.join(out,'summary.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});
console.log(JSON.stringify({passed:true,confirmed:baseline.confirmed,fixed_best_prices:baseline.all_best_prices_fixed,fixed_l1:baseline.all_l1_fixed,baseline_zero_duration:baseline.zero_available_ns_duration,gap_zero_duration:gap.zero_available_ns_duration,changed_available_durations:changed,summary:path.join(out,'summary.json')}));
