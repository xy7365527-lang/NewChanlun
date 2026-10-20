#!/usr/bin/env node
// Private #1394 S16 Content witness. The Python owner supplies the actual Q port.
"use strict";
const fs = require("fs"), net = require("net"), crypto = require("crypto"), path = require("path");
const {spawnSync} = require("child_process");
const root = "/Users/silencehan/Projects/NewChanlun-1394-content-v3";
const V = require(path.join(root, "s_session/browser/q3-values.js"));
const C = require(path.join(root, "s_session/browser/s-content-v3.js"));
const selftest=process.argv[2]==="--archive-selftest";
const out = selftest?process.argv[4]:process.argv[2]; let port = selftest?0:Number(process.argv[3]);
if (!out || (!selftest && (!Number.isSafeInteger(port) || port < 1 || port > 65535))) throw Error("owner args");
if(selftest)fs.mkdirSync(out);
const cfg = selftest?{session_id:"archive-selftest",session_generation:"1",query_epoch:"1"}:JSON.parse(fs.readFileSync(path.join(out, "Controller.json")));
const limits = selftest?{max_reply_bytes:"8388608",max_page_size:"31"}:JSON.parse(fs.readFileSync(cfg.query_resource_config)).limits;
const canonical = v => JSON.stringify(v, (_k, x) => x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.keys(x).sort().map(k => [k,x[k]])) : x);
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const put = (n,v) => fs.writeFileSync(path.join(out,n), canonical(v)+"\n", {flag:"wx"});
const SHA = b => crypto.createHash("sha256").update(b).digest();
const CRC_TABLE = Array.from({length:256},(_,i)=>{let c=i;for(let j=0;j<8;j++)c=(c&1)?0xedb88320^(c>>>1):c>>>1;return c>>>0;});
function crc32(a,b){let c=0xffffffff;for(const x of [a,b])for(const v of x)c=CRC_TABLE[(c^v)&255]^(c>>>8);return (c^0xffffffff)>>>0;}
function writeAll(fd,data){const b=Buffer.isBuffer(data)?data:Buffer.from(data);let pos=0;while(pos<b.length){const n=fs.writeSync(fd,b,pos,b.length-pos);if(n<=0)throw Error("fs.writeSync made no progress");pos+=n;}}
function allocatedBytes(dir){let total=0;for(const entry of fs.readdirSync(dir,{withFileTypes:true})){const p=path.join(dir,entry.name);if(entry.isDirectory())total+=allocatedBytes(p);else if(entry.isFile())total+=fs.statSync(p).blocks*512;}return total;}
class Archive {
  constructor(dir){this.dir=dir;this.parts=[];this.bytes=0;this.count=0;this.serial=0;this.chunk=0;this.compressed=0;this.closed=false;
    this.manifest=fs.openSync(path.join(dir,"archive-manifest.jsonl"),"wx");}
  append(meta,request,response){
    if(this.closed)throw Error("archive closed");
    const m=Buffer.from(canonical(meta));
    if(m.length>=65536 || request.length+response.length>10*1024*1024)throw Error("Q3R2 record bound");
    const h=Buffer.alloc(84);h.write("Q3R2",0,"ascii");h.writeUInt32BE(m.length,4);h.writeUInt32BE(request.length,8);h.writeUInt32BE(response.length,12);h.writeUInt32BE(crc32(request,response),16);SHA(request).copy(h,20);SHA(response).copy(h,52);
    const rec=Buffer.concat([h,m,request,response]);
    if(this.count && (this.count>=256 || this.bytes+rec.length>16*1024*1024))this.flush();
    if(meta.serial!==this.serial+1)throw Error("archive serial gap");
    this.parts.push(rec);this.bytes+=rec.length;this.count++;this.serial++;
    if(this.count>=256 || this.bytes>=16*1024*1024)this.flush();
  }
  reserveFor(requestBytes,maxResponseBytes){
    const worst=84+65535+requestBytes+maxResponseBytes;
    if(worst>10*1024*1024)throw Error("Q3R2 worst-case request/response exceeds record bound");
    if(this.count && this.bytes+worst>16*1024*1024)this.flush();
    // No HTTP request is sent unless the current pending frame and one whole
    // worst-case response can both be committed within the private 50 MiB cap.
    if(allocatedBytes(this.dir)+this.bytes+worst+2*1024*1024>50*1024*1024){
      if(this.count)this.flush();
      if(allocatedBytes(this.dir)+worst+2*1024*1024>50*1024*1024)
        throw Error("private 50 MiB worst-case wire/archive reservation unavailable before HTTP");
    }
  }
  flush(){
    if(!this.count)return;
    const raw=Buffer.concat(this.parts),n=this.chunk+1,name=`archive-chunk-${String(n).padStart(6,"0")}.zst`;
    const enc=spawnSync("zstd",["-q","-c","-3","--long=27","--check"],{input:raw,maxBuffer:32*1024*1024,timeout:30000});
    if(enc.error || enc.status!==0)throw Error("zstd encode: "+String(enc.error||enc.stderr));
    const dec=spawnSync("zstd",["-q","-dc"],{input:enc.stdout,maxBuffer:32*1024*1024,timeout:30000});
    if(dec.error || dec.status!==0 || !dec.stdout.equals(raw))throw Error("zstd immediate roundtrip");
    if(allocatedBytes(this.dir)+enc.stdout.length+2*1024*1024>50*1024*1024)throw Error("private archive 50 MiB preflush reservation");
    const tmp=path.join(this.dir,`archive-chunk-${String(n).padStart(6,"0")}.part`),target=path.join(this.dir,name);
    let fd=fs.openSync(tmp,"wx");
    try{writeAll(fd,enc.stdout);fs.fsyncSync(fd);}finally{fs.closeSync(fd);}
    fs.renameSync(tmp,target);
    const dfd=fs.openSync(this.dir,"r");try{fs.fsyncSync(dfd);}finally{fs.closeSync(dfd);}
    const receipt={chunk:n,file:name,format:"Q3R2/zstd-independent-256-or-16MiB/1",first_serial:this.serial-this.count+1,last_serial:this.serial,
      records:this.count,uncompressed_bytes:raw.length,uncompressed_sha256:sha(raw),compressed_bytes:enc.stdout.length,compressed_sha256:sha(enc.stdout)};
    writeAll(this.manifest,canonical(receipt)+"\n");fs.fsyncSync(this.manifest);
    this.chunk=n;this.compressed+=enc.stdout.length;this.parts=[];this.bytes=0;this.count=0;
  }
  close(){if(this.closed)return;try{this.flush();}finally{fs.fsyncSync(this.manifest);fs.closeSync(this.manifest);this.closed=true;}}
}
const archive=new Archive(out);
const timing=fs.openSync(path.join(out,"http-metrics.jsonl"),"wx");
let shutdownReason=null,activeSocket=null;
process.on("SIGTERM",()=>{shutdownReason="SIGTERM";if(activeSocket)activeSocket.destroy(Error("SIGTERM: preserve partial HTTP wire"));});
process.on("SIGINT",()=>{shutdownReason="SIGINT";if(activeSocket)activeSocket.destroy(Error("SIGINT: preserve partial HTTP wire"));});
let serial=0, wireBytes=0, responseBytes=0, contentRequests=0, largestReply=0, slowestMs=0;
const maxReply=Number(limits.max_reply_bytes);
const session=cfg.session_id, generation=cfg.session_generation, queryEpoch=cfg.query_epoch;
function requestHttp(op,payload) {
  if(shutdownReason)return Promise.reject(Error("collector shutdown: "+shutdownReason));
  const label="fullbody-"+(++serial);
  const req={schema_revision:"s-session/3",session_id:session,session_generation:generation,
    source_namespace:"formal-fullbody-s16",source_epoch:"1",message_id:label,
    producer_id:"controller-acceptance",producer_epoch:"1",payload_hash:sha(Buffer.from(canonical(payload))),
    causal_refs:[],payload};
  const body=Buffer.from(canonical(req));
  const wire=Buffer.concat([Buffer.from(`POST /api/v3/${op} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: ${body.length}\r\nConnection: close\r\n\r\n`),body]);
  archive.reserveFor(wire.length,maxReply+16384);
  const started=process.hrtime.bigint();
  return new Promise((resolve,reject)=>{
    const chunks=[];let bytes=0,finished=false;
    function partial(reason){
      if(finished)return;finished=true;
      activeSocket=null;
      const response=Buffer.concat(chunks),meta={serial,complete:false,error:String(reason),op,code:null};
      try{
        archive.append(meta,wire,response);
        writeAll(timing,canonical({serial,label,op,complete:false,error:String(reason),request_bytes:wire.length,response_wire_bytes:response.length,
          request_sha256:sha(wire),response_wire_sha256:sha(response),elapsed_ms:Number(process.hrtime.bigint()-started)/1e6})+"\n");
      }catch(writeError){
        try{fs.writeFileSync(path.join(out,`archive-failure-${serial}.json`),canonical({error:String(writeError),serial,request_sha256:sha(wire),response_wire_sha256:sha(response)})+"\n",{flag:"wx"});}catch(_){}
        reject(writeError);return;
      }
      reject(reason);
    }
    const sock=net.createConnection({host:"127.0.0.1",port});
    activeSocket=sock;
    sock.setTimeout(8000);
    sock.on("connect",()=>sock.end(wire));
    sock.on("data",b=>{bytes+=b.length;chunks.push(b);if(bytes>maxReply+16384)sock.destroy(Error("response bound exceeded"));});
    sock.on("timeout",()=>sock.destroy(Error("socket observation timeout")));
    sock.on("error",e=>{partial(e);});
    sock.on("close",()=>{if(!finished)partial(Error("socket closed before complete end"));});
    sock.on("end",()=>{
      if(finished)return;finished=true;
      activeSocket=null;
      try {
        const response=Buffer.concat(chunks), sep=response.indexOf("\r\n\r\n");
        let headerCode=null;
        if(sep>=0 && sep<=16384)headerCode=Number(response.subarray(0,sep).toString("latin1").split("\r\n")[0].split(" ")[1]);
        const validStatus=Number.isInteger(headerCode) && headerCode>=100 && headerCode<=599;
        archive.append({serial,complete:validStatus,error:validStatus?null:"invalid HTTP status line",op,code:validStatus?headerCode:null},wire,response);
        if(sep<0 || sep>16384)throw Error("HTTP header boundary");
        const head=response.subarray(0,sep).toString("latin1"),b=response.subarray(sep+4);
        const code=Number(head.split("\r\n")[0].split(" ")[1]);
        const len=/\r\ncontent-length:\s*(\d+)/i.exec("\r\n"+head);
        if(len && Number(len[1])!==b.length)throw Error("Content-Length");
        if(b.length>maxReply)throw Error("body exceeds original Q21 max_reply_bytes");
        const ms=Number(process.hrtime.bigint()-started)/1e6;
        const log={serial,label,op,code,complete:true,request_bytes:wire.length,response_wire_bytes:response.length,
          response_body_bytes:b.length,request_sha256:sha(wire),response_wire_sha256:sha(response),
          response_body_sha256:sha(b),elapsed_ms:ms};
        writeAll(timing,canonical(log)+"\n");
        wireBytes+=response.length;responseBytes+=b.length;largestReply=Math.max(largestReply,b.length);slowestMs=Math.max(slowestMs,ms);
        if(code!==200)throw Error("HTTP status "+code);
        const frame=JSON.parse(b.toString("utf8"));
        if(frame.payload_hash!==sha(Buffer.from(canonical(frame.payload))))throw Error("HTTP/envelope payload: "+code);
        const expected="reply-"+sha(Buffer.from(canonical([req.source_namespace,req.source_epoch,req.message_id,frame.payload_hash])));
        if(canonical(Object.keys(frame).sort())!==canonical(["schema_revision","session_id","session_generation","source_namespace","source_epoch","message_id","producer_id","producer_epoch","payload_hash","causal_refs","payload"].sort()))throw Error("reply envelope fields");
        if(frame.schema_revision!=="s-session/3" || frame.source_namespace!=="s-observe/replies" || frame.source_epoch!==queryEpoch || frame.message_id!==expected || frame.producer_id!=="Q:"+session || frame.producer_epoch!==queryEpoch || frame.session_id!==session || frame.session_generation!==generation)throw Error("reply identity");
        if(canonical(frame.causal_refs)!==canonical([{source_namespace:req.source_namespace,source_epoch:req.source_epoch,message_id:req.message_id,payload_hash:req.payload_hash}]))throw Error("causal identity");
        resolve(frame.payload);
      }catch(e){reject(e);}
    });
  });
}
function digestChunks(iter) {return (async()=>{const h=crypto.createHash("sha256");let bytes=0;for await(const chunk of iter){const b=Buffer.from(chunk,"utf8");h.update(b);bytes+=b.length;}return {sha256:h.digest("hex"),bytes};})();}
async function main(){
  const snapshot=JSON.parse(fs.readFileSync(path.join(out,"snapshot.complete.json")));
  if(snapshot.rows.length!==472 || snapshot.pages!==16)throw Error("S16 descriptor count");
  const anchor={kind:"projection",fixed_cut:snapshot.fixed_cut,cut_projection_digest:snapshot.cut_projection_digest,projection_ref:snapshot.projection_ref};
  const nodes=new Map(), nodeConflicts=[];
  const reader=new V.Reader({anchor,limit:Number(limits.max_page_size),check:()=>{if(shutdownReason)throw Error("collector shutdown: "+shutdownReason);},
    request:async req=>{contentRequests++;const r=await requestHttp("content",req);if(r.ok!==true)throw Error("Content refused: "+canonical(r));return r;},
    accept:async n=>{const old=nodes.get(n.root);if(old && canonical(old)!==canonical(n))nodeConflicts.push(n.root);},
    store:async n=>{nodes.set(n.root,n);}});
  const projection=reader.value(snapshot.projection_ref);
  await reader.complete(projection);
  const graph=await reader.finish();
  if(nodeConflicts.length)throw Error("node root collisions");
  const all=await digestChunks(graph.renderChunks(snapshot.projection_ref));
  const rows=fs.openSync(path.join(out,"row-digests.jsonl"),"wx");
  const families={};
  try{
    for(let i=0;i<snapshot.rows.length;i++){
      const r=snapshot.rows[i];
      await graph.verifyComplete(r.record);
      const body=await digestChunks(graph.renderChunks(r.record));
      const record={ordinal:i,family:r.family,row_key:r.key,record_ref:r.record,body_sha256:body.sha256,body_bytes:body.bytes};
      writeAll(rows,canonical(record)+"\n");families[r.family]=(families[r.family]||0)+1;
    }
    fs.fsyncSync(rows);
  }finally{fs.closeSync(rows);}
  if(canonical(families)!==canonical({objects:65,withdrawn_objects:82,witnesses:81,relations:199,observations:29,raw_history:16}))throw Error("six-family cardinality");
  put("body-summary.json",{status:"PASS_JS_CONTENT_CLOSURE",projection_ref:snapshot.projection_ref,projection_body_sha256:all.sha256,projection_body_bytes:all.bytes,
    catalog_evidence_ref:snapshot.fixed_cut.catalog_evidence,rows:472,families,unique_nodes:graph.metrics.uniqueNodes,
    framed_bytes:graph.metrics.framedBytes,content_requests:contentRequests,response_wire_bytes:wireBytes,response_body_bytes:responseBytes,
    largest_reply_body_bytes:largestReply,max_reply_bytes:maxReply,slowest_http_ms:slowestMs,
    JS_rss_bytes:process.memoryUsage().rss,JS_heap_used_bytes:process.memoryUsage().heapUsed,
    original_Q21_sha256:sha(fs.readFileSync(cfg.query_resource_config)),client_atomic_install:"NOT_ATTEMPTED",client_physical_allocation:"UNMEASURED"});
  put("content-node-roots.json",graph.nodes().map(n=>n.root));
}
async function archiveSelftest(){
  const mode=process.argv[3];if(!["normal","term","hold","shortwrite","rowerror","http503","budget"].includes(mode))throw Error("selftest mode");
  let expectedRowError=false,shortWriteCalls=0,budget=null;
  if(mode==="budget"){
    const originalAllocated=allocatedBytes;
    try{
      let flushed=false;
      const probe=Object.create(Archive.prototype);
      probe.count=1;probe.bytes=16*1024*1024-1024;probe.flush=()=>{flushed=true;probe.count=0;probe.bytes=0;};
      allocatedBytes=()=>flushed?46*1024*1024:30*1024*1024;
      let refused=false;
      try{probe.reserveFor(2048,8388608+16384);}catch(e){refused=String(e).includes("reservation unavailable before HTTP");}
      if(!flushed || !refused)throw Error("pending 16 MiB unsafe budget was not refused before HTTP");
      flushed=false;probe.count=1;probe.bytes=16*1024*1024-1024;
      allocatedBytes=()=>flushed?36*1024*1024:20*1024*1024;
      probe.reserveFor(2048,8388608+16384);
      if(!flushed)throw Error("safe pending frame was not flushed first");
      budget={unsafe_refused_before_http:true,safe_pending_flush_first:true,private_cap_bytes:50*1024*1024};
    }finally{allocatedBytes=originalAllocated;}
  }
  else if(["normal","shortwrite","rowerror"].includes(mode)){
    archive.append({serial:1,complete:true,error:null,op:"test",code:200},Buffer.from("GET /test HTTP/1.0\r\n\r\n"),Buffer.from("HTTP/1.0 200 OK\r\nContent-Length: 2\r\n\r\nOK"));
    if(mode==="shortwrite"){
      const original=fs.writeSync;
      fs.writeSync=(fd,data,offset,length)=>{shortWriteCalls++;return original(fd,data,offset,Math.min(length,7));};
      try{archive.close();}finally{fs.writeSync=original;}
      if(shortWriteCalls<2)throw Error("short write simulation did not exercise loops");
    }
    if(mode==="rowerror"){
      const fd=fs.openSync(path.join(out,"row-digests.jsonl"),"wx"),original=fs.writeSync;
      fs.writeSync=(target,...args)=>{if(target===fd)throw Object.assign(Error("injected row EIO"),{code:"EIO"});return original(target,...args);};
      try{writeAll(fd,'{"ordinal":0}\n');}catch(e){expectedRowError=e.code==="EIO";}
      finally{fs.writeSync=original;fs.closeSync(fd);}
      if(!expectedRowError)throw Error("row write EIO not detected");
    }
  }else{
    let accepted=null;
    const server=net.createServer({allowHalfOpen:true},s=>{
      accepted=s;
      s.on("data",()=>{
        if(mode==="http503"){
          s.end("HTTP/1.0 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n");return;
        }
        s.write("HTTP/1.0 200 OK\r\nContent-Length: 100\r\n\r\nabc");
        if(mode==="term")setTimeout(()=>process.kill(process.pid,"SIGTERM"),20);
        else put("selftest-active.json",{pid:process.pid,mode,partial_response_sent:true});
      });
    });
    await new Promise(r=>server.listen(0,"127.0.0.1",r));port=server.address().port;
    try{await requestHttp("content",{op:"content",anchor:{},path:[],unit:"items",offset:"0",limit:"1"});throw Error("term test unexpectedly completed");}
    catch(e){if(mode==="http503"?archive.serial!==1:shutdownReason!=="SIGTERM")throw e;}
    if(accepted)accepted.destroy();server.close();
  }
  archive.close();fs.fsyncSync(timing);fs.closeSync(timing);
  put("selftest-result.json",{mode,archive_records:archive.serial,archive_chunks:archive.chunk,archive_closed:archive.closed,
    shutdownReason,expectedRowError,shortWriteCalls,budget,business_success:false});
}
if(selftest){archiveSelftest().catch(e=>{console.error(e.stack);try{archive.close();}catch(_){}try{fs.closeSync(timing);}catch(_){}process.exitCode=1;});}
else (async()=>{
  let failed=null;
  try{await main();}catch(e){failed=e;console.error(e.stack);}
  try{archive.close();}catch(e){failed=failed||e;console.error("archive close",e.stack);}
  try{fs.fsyncSync(timing);fs.closeSync(timing);}catch(e){failed=failed||e;console.error("timing close",e.stack);}
  if(failed || shutdownReason){
    try{put("collector-error.json",{status:"FAIL",message:String(failed||shutdownReason),stack:failed?.stack||null,
      serial,contentRequests,shutdown_reason:shutdownReason,archive_records:archive.serial,archive_chunks:archive.chunk,
      archive_closed:archive.closed});}catch(e){console.error("collector error receipt",e);}
    process.exitCode=1;
  }else{
    try{put("collector-terminal.json",{status:"PASS_ARCHIVE_FLUSHED",archive_records:archive.serial,archive_chunks:archive.chunk,
      archive_closed:archive.closed,compressed_bytes:archive.compressed});}catch(e){console.error(e);process.exitCode=1;}
  }
})().catch(e=>{console.error(e.stack);process.exitCode=1;});
