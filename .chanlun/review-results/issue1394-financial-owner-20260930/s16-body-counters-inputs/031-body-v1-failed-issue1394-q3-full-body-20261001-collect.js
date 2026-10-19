#!/usr/bin/env node
// Private #1394 S16 Content witness. The Python owner supplies the actual Q port.
"use strict";
const fs = require("fs"), net = require("net"), crypto = require("crypto"), zlib = require("zlib"), path = require("path");
const {pipeline} = require("stream/promises");
const root = "/Users/silencehan/Projects/NewChanlun-1394-content-v3";
const V = require(path.join(root, "s_session/browser/q3-values.js"));
const C = require(path.join(root, "s_session/browser/s-content-v3.js"));
const out = process.argv[2], port = Number(process.argv[3]);
if (!out || !Number.isSafeInteger(port) || port < 1 || port > 65535) throw Error("owner args");
const cfg = JSON.parse(fs.readFileSync(path.join(out, "Controller.json")));
const limits = JSON.parse(fs.readFileSync(cfg.query_resource_config)).limits;
const canonical = v => JSON.stringify(v, (_k, x) => x && typeof x === "object" && !Array.isArray(x) ? Object.fromEntries(Object.keys(x).sort().map(k => [k,x[k]])) : x);
const sha = b => crypto.createHash("sha256").update(b).digest("hex");
const put = (n,v) => fs.writeFileSync(path.join(out,n), canonical(v)+"\n", {flag:"wx"});
const raw = fs.createWriteStream(path.join(out,"raw-http.jsonl.gz"), {flags:"wx"});
const gz = zlib.createGzip({level:9});
const pipeDone = pipeline(gz,raw).then(()=>null,e=>e);
const timing = fs.createWriteStream(path.join(out,"http-metrics.jsonl"), {flags:"wx"});
let timingError=null;timing.on("error",e=>{timingError=e;});
let serial=0, wireBytes=0, responseBytes=0, contentRequests=0, largestReply=0, slowestMs=0;
const maxReply=Number(limits.max_reply_bytes);
const session=cfg.session_id, generation=cfg.session_generation, queryEpoch=cfg.query_epoch;
function requestHttp(op,payload) {
  const label="fullbody-"+(++serial);
  const req={schema_revision:"s-session/3",session_id:session,session_generation:generation,
    source_namespace:"formal-fullbody-s16",source_epoch:"1",message_id:label,
    producer_id:"controller-acceptance",producer_epoch:"1",payload_hash:sha(Buffer.from(canonical(payload))),
    causal_refs:[],payload};
  const body=Buffer.from(canonical(req));
  const wire=Buffer.concat([Buffer.from(`POST /api/v3/${op} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: ${body.length}\r\nConnection: close\r\n\r\n`),body]);
  const started=process.hrtime.bigint();
  return new Promise((resolve,reject)=>{
    const chunks=[];let bytes=0,finished=false;
    async function partial(reason){
      if(finished)return;finished=true;
      const response=Buffer.concat(chunks),log={serial,label,op,complete:false,error:String(reason),
        request_bytes:wire.length,response_wire_bytes:response.length,request_sha256:sha(wire),
        response_wire_sha256:sha(response),elapsed_ms:Number(process.hrtime.bigint()-started)/1e6,
        request_base64:wire.toString("base64"),response_base64:response.toString("base64")};
      try{
        await Promise.all([
          new Promise((r,j)=>gz.write(canonical(log)+"\n",e=>e?j(e):r())),
          new Promise((r,j)=>timing.write(canonical(Object.fromEntries(Object.entries(log).filter(([k])=>!k.endsWith("base64"))))+"\n",e=>e?j(e):r()))
        ]);
      }catch(writeError){
        const failures={raw_stream:String(writeError)};
        try{fs.writeFileSync(path.join(out,`partial-${serial}.response.bin`),response,{flag:"wx"});}catch(e){failures.response_fallback=String(e);}
        try{fs.writeFileSync(path.join(out,`partial-${serial}.request.bin`),wire,{flag:"wx"});}catch(e){failures.request_fallback=String(e);}
        try{fs.writeFileSync(path.join(out,`partial-${serial}.write-error.json`),canonical(failures)+"\n",{flag:"wx"});}catch(e){console.error("partial failure receipt write",e);}
      }
      reject(reason);
    }
    const sock=net.createConnection({host:"127.0.0.1",port});
    sock.setTimeout(8000);
    sock.on("connect",()=>sock.end(wire));
    sock.on("data",b=>{bytes+=b.length;if(bytes>maxReply+16384){sock.destroy(Error("response bound exceeded"));return;}chunks.push(b);});
    sock.on("timeout",()=>sock.destroy(Error("socket observation timeout")));
    sock.on("error",e=>{void partial(e);});
    sock.on("close",()=>{if(!finished)void partial(Error("socket closed before complete end"));});
    sock.on("end",async()=>{
      if(finished)return;finished=true;
      try {
        const response=Buffer.concat(chunks), sep=response.indexOf("\r\n\r\n");
        if(sep<0 || sep>16384)throw Error("HTTP header boundary");
        const head=response.subarray(0,sep).toString("latin1"),b=response.subarray(sep+4);
        const code=Number(head.split("\r\n")[0].split(" ")[1]);
        const len=/\r\ncontent-length:\s*(\d+)/i.exec("\r\n"+head);
        if(len && Number(len[1])!==b.length)throw Error("Content-Length");
        if(b.length>maxReply)throw Error("body exceeds original Q21 max_reply_bytes");
        const ms=Number(process.hrtime.bigint()-started)/1e6;
        const log={serial,label,op,code,request_bytes:wire.length,response_wire_bytes:response.length,
          response_body_bytes:b.length,request_sha256:sha(wire),response_wire_sha256:sha(response),
          response_body_sha256:sha(b),elapsed_ms:ms,
          request_base64:wire.toString("base64"),response_base64:response.toString("base64")};
        await Promise.all([
          new Promise((r,j)=>gz.write(canonical(log)+"\n",e=>e?j(e):r())),
          new Promise((r,j)=>timing.write(canonical(Object.fromEntries(Object.entries(log).filter(([k])=>!k.endsWith("base64"))))+"\n",e=>e?j(e):r()))
        ]);
        wireBytes+=response.length;responseBytes+=b.length;largestReply=Math.max(largestReply,b.length);slowestMs=Math.max(slowestMs,ms);
        const frame=JSON.parse(b.toString("utf8"));
        if(code!==200 || frame.payload_hash!==sha(Buffer.from(canonical(frame.payload))))throw Error("HTTP/envelope payload: "+code);
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
  const reader=new V.Reader({anchor,limit:Number(limits.max_page_size),check:()=>{},
    request:async req=>{contentRequests++;const r=await requestHttp("content",req);if(r.ok!==true)throw Error("Content refused: "+canonical(r));return r;},
    accept:async n=>{const old=nodes.get(n.root);if(old && canonical(old)!==canonical(n))nodeConflicts.push(n.root);},
    store:async n=>{nodes.set(n.root,n);}});
  const projection=reader.value(snapshot.projection_ref);
  await reader.complete(projection);
  const graph=await reader.finish();
  if(nodeConflicts.length)throw Error("node root collisions");
  const all=await digestChunks(graph.renderChunks(snapshot.projection_ref));
  const rows=fs.createWriteStream(path.join(out,"row-digests.jsonl"),{flags:"wx"});
  const families={};
  for(let i=0;i<snapshot.rows.length;i++){
    const r=snapshot.rows[i];
    await graph.verifyComplete(r.record);
    const body=await digestChunks(graph.renderChunks(r.record));
    const record={ordinal:i,family:r.family,row_key:r.key,record_ref:r.record,body_sha256:body.sha256,body_bytes:body.bytes};
    rows.write(canonical(record)+"\n");families[r.family]=(families[r.family]||0)+1;
  }
  await new Promise((resolve,reject)=>{rows.once("finish",resolve);rows.once("error",reject);rows.end();});
  if(canonical(families)!==canonical({objects:65,withdrawn_objects:82,witnesses:81,relations:199,observations:29,raw_history:16}))throw Error("six-family cardinality");
  put("body-summary.json",{status:"PASS_JS_CONTENT_CLOSURE",projection_ref:snapshot.projection_ref,projection_body_sha256:all.sha256,projection_body_bytes:all.bytes,
    catalog_evidence_ref:snapshot.fixed_cut.catalog_evidence,rows:472,families,unique_nodes:graph.metrics.uniqueNodes,
    framed_bytes:graph.metrics.framedBytes,content_requests:contentRequests,response_wire_bytes:wireBytes,response_body_bytes:responseBytes,
    largest_reply_body_bytes:largestReply,max_reply_bytes:maxReply,slowest_http_ms:slowestMs,
    JS_rss_bytes:process.memoryUsage().rss,JS_heap_used_bytes:process.memoryUsage().heapUsed,
    original_Q21_sha256:sha(fs.readFileSync(cfg.query_resource_config)),client_atomic_install:"NOT_ATTEMPTED",client_physical_allocation:"UNMEASURED"});
  put("content-node-roots.json",graph.nodes().map(n=>n.root));
}
(async()=>{
  let failed=null;
  try{await main();}
  catch(e){failed=e;fs.writeFileSync(path.join(out,"collector-error.json"),canonical({status:"FAIL",message:String(e),stack:e.stack,serial,contentRequests})+"\n");console.error(e.stack);}
  const timingDone=new Promise(r=>{
    if(timingError){r(timingError);return;}
    timing.once("finish",()=>r(null));timing.once("error",r);
    try{timing.end();}catch(e){r(e);}
  });
  gz.end();
  const settled=await Promise.allSettled([pipeDone,timingDone]);
  if(settled[0].status==="rejected" || settled[0].value || settled[1].status==="rejected" || settled[1].value){
    const reason=settled.map(x=>x.status==="rejected"?String(x.reason):String(x.value)).join("; ");
    try{fs.writeFileSync(path.join(out,"collector-stream-error.json"),canonical({status:"FAIL_STREAM_FLUSH",reason})+"\n");}catch(_){}
    failed=failed||Error(reason);
  }
  if(failed)process.exitCode=1;
})().catch(e=>{console.error(e.stack);process.exitCode=1;});
