// #1467/#1468：恢复、前缀、真实长缺口和合成边界；通过后压缩可再生行输出。
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import { gzipSync, gunzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { runCase, sha } from './stage26_run.mjs';
import { inspect } from './stage26_inspect.mjs';

const [binaryArg,inputArg,gapArg,rootArg,outputArg]=process.argv.slice(2);
assert.equal(process.argv.length,7,'usage: node stage26_verify.mjs BINARY INPUT GAP_INPUT BASELINE_ROOT NEW_VERIFY_DIR');
const binary=await fs.realpath(binaryArg),input=await fs.realpath(inputArg),gapInput=await fs.realpath(gapArg),root=await fs.realpath(rootArg),output=path.resolve(outputArg);
const baseline=path.join(root,'baseline'),pkg=path.dirname(fileURLToPath(import.meta.url));
const receipts=[],inspections=[],archives=[];
await fs.mkdir(output,{mode:0o700});
const exists=async p=>{try{await fs.access(p);return true;}catch(e){if(e.code==='ENOENT')return false;throw e;}};
const rows=async p=>await exists(p)?fs.readFile(p):gunzipSync(await fs.readFile(p+'.gz'));
const split=b=>{const a=b.toString('utf8').split('\n');assert.equal(a.pop(),'');return a;};
async function bytes(d){let n=0;for(const e of await fs.readdir(d,{withFileTypes:true})){const p=path.join(d,e.name);n+=e.isDirectory()?await bytes(p):(await fs.stat(p)).size;}return n;}
async function budget(){const n=await bytes(root);assert(n<=384*1048576,'evidence exceeds 384MiB');return n;}
async function archiveRows(p){
  const original=await fs.readFile(p),compressed=gzipSync(original,{level:6});
  assert.equal(sha(gunzipSync(compressed)),sha(original));
  await fs.writeFile(p+'.gz',compressed,{flag:'wx',mode:0o600});
  const r={original_path:p,archive_path:p+'.gz',original_bytes:original.length,archive_bytes:compressed.length,
    original_sha256:sha(original),archive_sha256:sha(compressed),decompressed_verified:true};
  await fs.writeFile(path.join(path.dirname(p),'rows-archive.json'),JSON.stringify(r,null,2)+'\n',{flag:'wx',mode:0o600});
  await fs.unlink(p);archives.push(r);return r;
}
async function execute(name,{data=input,checkpoint,lines=[],stop}={}){
  const r=await runCase({binary,input:data,dir:path.join(output,name),checkpoint,lines,stop});receipts.push(r);
  assert.equal(r.timed_out,false);assert.equal(r.signal,null);return r;
}
async function success(name,opts={}){const r=await execute(name,opts);assert.equal(r.exit_code,0,`${name} failed: ${r.stderr}`);return r;}
async function reject(name,opts,reason){const r=await execute(name,opts);assert.equal(r.exit_code,1);assert(r.stderr.includes(reason));assert.equal(r.report,null);return r;}
async function pool(jobs){const all=[];for(let i=0;i<jobs.length;i+=2)all.push(...await Promise.allSettled(jobs.slice(i,i+2).map(f=>f())));for(const r of all)if(r.status==='rejected')throw r.reason;return all.map(r=>r.value);}
const quote=(p,q,id)=>[String(p),String(q),id];
function synthetic(){
  const all=[];let n=0;
  const stamp=()=>`2021-01-01T00:00:00.${String(++n*1000).padStart(9,'0')}Z`;
  const add=m=>{const t=stamp();all.push(t+' '+JSON.stringify({product_id:'BTC-USD',time:t,...m}));};
  const snap=(sequence,bids,asks)=>add({type:'full_snapshot',generated:true,sequence,bids,asks});
  const done=(sequence,id,side,price)=>add({type:'done',sequence,order_id:id,side,price:String(price),remaining_size:'1',reason:'canceled'});
  const open=(sequence,id,side,price)=>add({type:'open',sequence,order_id:id,side,price:String(price),remaining_size:'1'});
  const asks=[quote(103,1,'test-a2'),quote(104,1,'test-a1')];
  snap(100,[quote(100,1,'test-b0')],[quote(102,1,'test-a0'),quote(104,1,'test-a1')]);
  done(101,'test-a0','sell',102);open(102,'test-a2','sell',103);open(103,'test-b1','buy',101);
  done(104,'test-b1','buy',101);
  add({type:'match',sequence:105,trade_id:1,maker_order_id:'test-b0',side:'buy',price:'100',size:'1'});
  open(106,'test-b2','buy',100);
  snap(106,[quote(100,1,'test-b2')],asks);
  snap(108,[quote(100,1,'test-b2')],asks);
  open(109,'test-b3','buy',99);done(110,'test-b3','buy',99);
  add({type:'change',sequence:111,order_id:'test-b2',side:'buy',price:'100',old_size:'1',new_size:'2'});
  open(112,'test-a3','sell',105);
  add({type:'match',sequence:113,trade_id:2,maker_order_id:'test-b2',side:'buy',price:'100',size:'0.5'});
  add({type:'change',sequence:114,order_id:'test-b2',side:'buy',price:'100',old_size:'1.5',new_size:'1'});
  open(115,'test-b4','buy',99);
  all.push('');n++;
  snap(115,[quote(100,1,'test-b2'),quote(99,1,'test-b4')],[...asks,quote(105,1,'test-a3')]);
  open(116,'test-a4','sell',106);open(117,'test-b5','buy',98);
  return all;
}
try{
  const sourceBytes=await fs.readFile(input),sourceLines=split(sourceBytes);
  assert.equal(sha(sourceBytes),'8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd');
  const br=JSON.parse(await fs.readFile(path.join(baseline,'report.json'),'utf8'));
  const bi=JSON.parse(await fs.readFile(path.join(root,'baseline-inspection.json'),'utf8'));
  assert(bi.passed);assert.equal(bi.checker_sha256,sha(await fs.readFile(path.join(pkg,'stage26_inspect.mjs'))));
  assert.equal(br.executable_sha256,sha(await fs.readFile(binary)));
  assert.equal(br.state_sha256,'c17603a9b18a1d885d580663ff360560ebdcfc451f09700963af334687d2b1cf');
  const baselineBytes=await rows(path.join(baseline,'rows.jsonl')),baselineLines=split(baselineBytes);
  assert.equal(sha(baselineBytes),br.output_file_sha256);
  await budget();
  const recovery=await pool([339,340,10000].map(n=>async()=>{
    const r=await success(`resume-${n}`,{checkpoint:path.join(baseline,'checkpoints',`line-${n}.json`)});
    assert.equal(r.report.state_sha256,br.state_sha256);assert.equal(r.report.observer_sha256,br.observer_sha256);
    assert.equal(r.report.output_chain_sha256,br.output_chain_sha256);assert.equal(r.report.shape_chain_sha256,br.shape_chain_sha256);
    assert.equal((await rows(r.rows)).toString('utf8'),baselineLines.slice(n).join('\n')+'\n');
    await archiveRows(r.rows);await budget();console.log(JSON.stringify({passed:`resume-${n}`}));return n;
  }));
  const prefixes=[];
  for(const n of [340,10000]){
    const data=path.join(output,`prefix-${n}.ndjson`);await fs.writeFile(data,sourceLines.slice(0,n).join('\n')+'\n',{flag:'wx',mode:0o600});
    const r=await success(`prefix-${n}`,{data,stop:n});assert.equal((await rows(r.rows)).toString('utf8'),baselineLines.slice(0,n).join('\n')+'\n');
    await archiveRows(r.rows);prefixes.push(n);await budget();console.log(JSON.stringify({passed:`prefix-${n}`}));
  }
  const gap=await success('long-gap',{data:gapInput,lines:[10000]});
  const gapInspection=await inspect(gapInput,gap.rows,path.join(gap.dir,'report.json'));inspections.push(gapInspection);
  assert.equal(gap.report.shape_chain_sha256,br.shape_chain_sha256);assert.equal(gap.report.price_book_sha256,br.price_book_sha256);
  assert.equal(gap.report.completed_blocks,br.completed_blocks);assert.equal(gap.report.model_events,br.model_events);
  const gapLines=split(await rows(gap.rows)),gapRows=gapLines.map(JSON.parse);
  assert(gapRows.slice(1000,10004).every(r=>r.status==='sequence_gap'&&r.applications.length===0&&r.confirmed.length===0&&!r.active_update));
  const burst=gapRows[10004];assert.equal(burst.status,'ready');assert(burst.applications.length>=9005);
  assert(burst.confirmed.length>0&&burst.confirmed.every(b=>b.known_at_line===10005));
  assert(gapInspection.delayed_confirmations>bi.delayed_confirmations);
  const gapCp=JSON.parse(await fs.readFile(path.join(gap.dir,'checkpoints/line-10000.json'),'utf8'));
  assert.equal(Object.keys(gapCp.state.pending).length,9000);
  await archiveRows(gap.rows);await budget();
  const gr=await success('gap-resume',{data:gapInput,checkpoint:path.join(gap.dir,'checkpoints/line-10000.json')});
  assert.equal(gr.report.state_sha256,gap.report.state_sha256);assert.equal(gr.report.observer_sha256,gap.report.observer_sha256);
  assert.equal(gr.report.output_chain_sha256,gap.report.output_chain_sha256);
  assert.equal((await rows(gr.rows)).toString('utf8'),gapLines.slice(10000).join('\n')+'\n');await archiveRows(gr.rows);
  console.log(JSON.stringify({passed:'long-gap-and-resume',delayed_confirmations:gapInspection.delayed_confirmations,burst_confirmations:burst.confirmed.length}));
  const missingData=path.join(output,'missing-sequence.ndjson');
  await fs.writeFile(missingData,sourceLines.filter((_,i)=>i!==1000).join('\n')+'\n',{flag:'wx',mode:0o600});
  const missing=await success('missing-sequence',{data:missingData});
  inspections.push(await inspect(missingData,missing.rows,path.join(missing.dir,'report.json')));
  const missingRows=split(await rows(missing.rows)).map(JSON.parse);
  assert(missingRows.slice(1000).every(r=>r.status==='sequence_gap'&&r.applications.length===0&&r.confirmed.length===0&&!r.active_update));
  assert(missing.report.pending>0&&missing.report.active_at_end.provisional);
  await archiveRows(missing.rows);
  const synLines=synthetic(),synData=path.join(output,'synthetic.ndjson');
  await fs.writeFile(synData,synLines.join('\n')+'\n',{flag:'wx',mode:0o600});
  const syn=await success('synthetic',{data:synData,lines:[7]});
  const si=await inspect(synData,syn.rows,path.join(syn.dir,'report.json'));inspections.push(si);
  assert.equal(syn.report.epochs,4);assert.equal(syn.report.barriers,1);assert.equal(syn.report.unfinished_domain_exits,3);
  const sr=split(await rows(syn.rows)).map(JSON.parse);
  assert.equal(sr[5].status,'missing_quote');
  assert(sr[6].confirmed.some(b=>b.geometry===null&&b.known_at_line===7));
  assert.equal(sr[7].applications[0].type,'snapshot_reaffirmed');assert.equal(sr[7].active_update,false);
  assert(sr[8].applications.some(e=>e.type==='epoch_end'&&e.tail_status==='unfinished_on_domain_exit'));
  assert(sr[11].applications.some(e=>e.disposition==='model_barrier'));
  assert.equal(sr[16].status,'disconnected');assert(syn.report.active_at_end.provisional);
  const synResume=await success('synthetic-resume',{data:synData,checkpoint:path.join(syn.dir,'checkpoints/line-7.json')});
  assert.equal((await rows(synResume.rows)).toString('utf8'),sr.slice(7).map(x=>JSON.stringify(x)).join('\n')+'\n');
  assert.equal(synResume.report.observer_sha256,syn.report.observer_sha256);
  const cpText=await fs.readFile(path.join(baseline,'checkpoints/line-10000.json'),'utf8');
  for(const [name,replacement,reason] of [
    ['source',s=>s.replace(/"source_sha256":"[a-f0-9]{64}"/,'"source_sha256":"'+'0'.repeat(64)+'"'),'checkpoint profile/source/executable mismatch'],
    ['executable',s=>s.replace(/"executable_sha256":"[a-f0-9]{64}"/,'"executable_sha256":"'+'0'.repeat(64)+'"'),'checkpoint profile/source/executable mismatch'],
    ['state',s=>s.replace('"lines":10000','"lines":10001'),'checkpoint payload checksum mismatch'],
    ['observer',s=>s.replace(/"model_index":(\d+)/,(_,n)=>'"model_index":'+(Number(n)+1)),'checkpoint payload checksum mismatch'],
  ]){
    const tampered=replacement(cpText);assert.notEqual(tampered,cpText);
    const cp=path.join(output,`bad-${name}.json`);await fs.writeFile(cp,tampered,{flag:'wx',mode:0o600});
    const r=await reject(`reject-${name}`,{checkpoint:cp},reason);assert.equal(await exists(r.rows),false);
  }
  for(const [name,mutate,reason]of [
    ['snapshot-conflict',m=>{m.bids[0][1]='2';},'same-sequence snapshot contradicts'],
    ['snapshot-rewind',m=>{m.sequence=105;},'snapshot sequence rewinds'],
  ]){
    const lines=synLines.slice(0,8),i=lines[7].indexOf(' '),m=JSON.parse(lines[7].slice(i+1));mutate(m);lines[7]=lines[7].slice(0,i+1)+JSON.stringify(m);
    const data=path.join(output,`${name}.ndjson`);await fs.writeFile(data,lines.join('\n')+'\n',{flag:'wx',mode:0o600});await reject(name,{data},reason);
  }
  if(await exists(path.join(baseline,'rows.jsonl')))await archiveRows(path.join(baseline,'rows.jsonl'));
  const finalBytes=await budget();
  const summary={all_passed:true,baseline:bi,legacy_state_equal:true,recovery_lines:recovery,physical_prefixes:prefixes,
    gap:{inspection:gapInspection,shape_equal:true,price_book_equal:true,pending_checkpoint:9000,burst_line:10005,burst_applications:burst.applications.length,burst_confirmations:burst.confirmed.length,resume_equal:true},
    synthetic:si,missing_sequence_stays_unpublished:true,negative_controls:6,evidence_bytes:finalBytes,evidence_limit_bytes:384*1048576,
    runner_sha256:sha(await fs.readFile(fileURLToPath(import.meta.url))),inspector_sha256:sha(await fs.readFile(path.join(pkg,'stage26_inspect.mjs'))),
    receipts:receipts.map(r=>({dir:r.dir,exit_code:r.exit_code,timed_out:r.timed_out,elapsed_ms:r.elapsed_ms})),archives,inspections,
    scope:'finite native profile with independent author implementation; no full Rust refinement, semantic-agent review, F2 or value result'};
  await fs.writeFile(path.join(output,'summary.json'),JSON.stringify(summary,null,2)+'\n',{flag:'wx',mode:0o600});
  console.log(JSON.stringify({all_passed:true,model_events:br.model_events,completed:br.completed_blocks,gap_delayed_confirmations:gapInspection.delayed_confirmations,bytes:finalBytes,summary:path.join(output,'summary.json')}));
}catch(error){await fs.writeFile(path.join(output,'failure.json'),JSON.stringify({all_passed:false,error:String(error),receipts:receipts.map(r=>({dir:r.dir,exit_code:r.exit_code,timed_out:r.timed_out,stderr:r.stderr}))},null,2)+'\n',{flag:'wx',mode:0o600});throw error;}
