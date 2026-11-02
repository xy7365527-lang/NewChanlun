#!/usr/bin/env node
/* #1394：真实 Chrome CDP pipe；不装依赖、不复用登录态，不渲染夹具 JSON。 */
"use strict";
const {spawn} = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const args = Object.fromEntries(process.argv.slice(2).reduce((a,x,i,v)=>i%2?a:[...a,[x,v[i+1]]],[]));
const output = path.resolve(args["--output"]);
fs.mkdirSync(output,{recursive:false});
const chrome = args["--chrome"];
const url = new URL(args["--url"]);
if(url.hostname!=="127.0.0.1" || url.protocol!=="http:")throw new Error("仅允许本机正式页面");
const flags=["--headless","--disable-background-networking","--disable-component-update","--disable-sync","--no-first-run","--no-default-browser-check","--remote-debugging-pipe","--user-data-dir="+path.join(output,"profile"),"about:blank"];
fs.writeFileSync(path.join(output,"command.json"),JSON.stringify({chrome,flags,url:url.href},null,2),{flag:"wx"});
const child=spawn(chrome,flags,{stdio:["ignore","pipe","pipe","pipe","pipe"],env:{PATH:process.env.PATH,TMPDIR:output}});
child.stdout.pipe(fs.createWriteStream(path.join(output,"chrome.stdout"),{flags:"wx"}));
child.stderr.pipe(fs.createWriteStream(path.join(output,"chrome.stderr"),{flags:"wx"}));
const log=fs.createWriteStream(path.join(output,"cdp.jsonl"),{flags:"wx"});
let sequence=0,buffer=Buffer.alloc(0),session;
const httpReplies=new Map(),httpFinished=new Set();
const pending=new Map();
child.stdio[4].on("data",chunk=>{
  buffer=Buffer.concat([buffer,chunk]);
  while(buffer.includes(0)){
    const at=buffer.indexOf(0),raw=buffer.subarray(0,at).toString();buffer=buffer.subarray(at+1);
    const message=JSON.parse(raw);log.write(JSON.stringify({direction:"received",message})+"\n");
    if(message.method==="Network.responseReceived" && message.params.response.url.includes("/api/economic/"))httpReplies.set(message.params.requestId,message.params.response);
    if(message.method==="Network.loadingFinished")httpFinished.add(message.params.requestId);
    const waiter=pending.get(message.id);if(waiter){pending.delete(message.id);clearTimeout(waiter.timer);message.error?waiter.reject(new Error(JSON.stringify(message.error))):waiter.resolve(message.result);}
  }
});
function failAll(error){for(const w of pending.values()){clearTimeout(w.timer);w.reject(error);}pending.clear();}
child.on("error",failAll);child.on("exit",(code,signal)=>failAll(new Error("Chrome 退出 "+code+" / "+signal)));
function send(method,params={},target=session){
  const id=++sequence,message={id,method,params,...(target?{sessionId:target}:{})};
  log.write(JSON.stringify({direction:"sent",message})+"\n");
  return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{pending.delete(id);reject(new Error("CDP 超时 "+method));},30000);pending.set(id,{resolve,reject,timer});child.stdio[3].write(JSON.stringify(message)+"\0");});
}
async function evaluate(expression){const r=await send("Runtime.evaluate",{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw new Error(JSON.stringify(r.exceptionDetails));return r.result.value;}
async function settle(){
  const end=Date.now()+30000;
  while(Date.now()<end){if(await evaluate("typeof client !== 'undefined' && client.state !== null && !busy"))return;await new Promise(r=>setTimeout(r,50));}
  throw new Error("正式浏览器未得到完整已核经济切面");
}
async function capture(name){
  await settle();const state=await evaluate("client.state"),text=await evaluate("document.body.innerText");
  fs.writeFileSync(path.join(output,name+".json"),JSON.stringify({state,text},null,2),{flag:"wx"});
  const screenshot=await send("Page.captureScreenshot",{format:"png",captureBeyondViewport:true});fs.writeFileSync(path.join(output,name+".png"),Buffer.from(screenshot.data,"base64"),{flag:"wx"});
  return state;
}
(async()=>{
  try {
    const {targetId}=await send("Target.createTarget",{url:"about:blank"},null);
    ({sessionId:session}=await send("Target.attachToTarget",{targetId,flatten:true},null));
    await send("Page.enable");await send("Runtime.enable");
    // 禁止页面走出本机，页面资源仍全经正式 HTTP/API。
    await send("Network.enable");await send("Network.setBlockedURLs",{urls:["https://*"]});
    await send("Page.navigate",{url:url.href});await settle();const first=await capture("current");
    const discovery=await evaluate("client.discovery");
    // #1716：/4 = /2 内容域 + 签署 cap 激活，同走内容域完整向量。
    const contentDomain=["tb03b-finance/2","tb03b-finance/4"];
    if(!contentDomain.includes(discovery.finance_revision) || first.completeness!=="CompleteVector")throw new Error("必须真实内容域（/2 或 /4）完整向量");
    const decisions=Object.entries(first.domains).filter(([d])=>d.startsWith("B:")).flatMap(([domain,v])=>v.image.finance.decisions.map(d=>({domain,decision:d})));
    if(new Set(decisions.map(d=>d.domain)).size<2 || decisions.some(({decision:d})=>d.state!=="Committed"))throw new Error("当前域缺双方真实 Committed");
    if(!args["--as-known"])throw new Error("必须指定可同时观察双方 Prepared 的 --as-known");
    await evaluate("document.getElementById('known-at').value="+JSON.stringify(args["--as-known"])+";document.getElementById('as-known').click();true");
    const prepared=await capture("prepared-as-known");
    if(prepared.mode!=="AsKnown" || prepared.as_known_ns!==args["--as-known"] || prepared.completeness!=="CompleteVector")throw new Error("未返回请求的完整历史模式");
    for(const {domain,decision:d} of decisions){
      const view=prepared.domains[domain],old=view.image.finance.decisions.find(x=>x.decision_id===d.decision_id);
      if(JSON.stringify(view.image.book)!==JSON.stringify(first.domains[domain].image.book) || !old || old.state!=="Prepared" || old.grant!==null || old.local_commit_receipt!==null || old.execution_eligibility!=="None" || JSON.stringify(old.prepared)!==JSON.stringify(d.prepared) || BigInt(view.cut.commit_seq)>=BigInt(first.domains[domain].cut.commit_seq) || BigInt(view.commit_ns)>BigInt(args["--as-known"]))throw new Error("双方 Prepared 历史不成立 "+domain);
    }
    await evaluate("document.getElementById('watch').click();true");
    const after=await capture("changes-to-current");
    if(after.completeness!=="CompleteVector")throw new Error("Watch 未取得完整末尾向量");
    for(const [domain,view] of Object.entries(first.domains)){
      const next=after.domains[domain];
      if(JSON.stringify(view.cut)!==JSON.stringify(next.cut) || JSON.stringify(view.image)!==JSON.stringify(next.image) || view.image_hash!==next.image_hash || view.commit_ns!==next.commit_ns)throw new Error("Watch 同源 cut/image/commit_ns 与当前不同 "+domain);
    }
    const buttons=await evaluate("Array.from(document.querySelectorAll('button[data-original-id]')).map(b=>({kind:b.dataset.originalKind,domain:b.dataset.originalDomain,id:b.dataset.originalId}))");
    const expected=[...decisions.map(({domain,decision:d})=>({kind:"decision",domain,id:d.decision_id})),...first.domains.E.image.finance.slots.flatMap(s=>s.grants.map(g=>({kind:"grant",domain:"E",id:g.grant_id})))];
    if(buttons.length!==expected.length || expected.some(e=>!buttons.some(b=>JSON.stringify(b)===JSON.stringify(e))))throw new Error("原件按钮未覆盖全部决定/grant");
    const originals=[];
    for(let i=0;i<buttons.length;i++){
      await evaluate("document.querySelectorAll('button[data-original-id]')["+i+"].click();true");
      const end=Date.now()+15000;
      while(Date.now()<end && await evaluate("document.querySelectorAll('button[data-original-id]')["+i+"].disabled"))await new Promise(r=>setTimeout(r,50));
      if(await evaluate("document.querySelectorAll('button[data-original-id]')["+i+"].disabled"))throw new Error("原件读取超时");
      const raw=await evaluate("document.querySelectorAll('button[data-original-id]')["+i+"].nextElementSibling.textContent");
      const reply=JSON.parse(raw);
      if(reply.kind!=="EconomicOriginal")throw new Error("原件未经过客户端验证");
      originals.push({request:buttons[i],reply});
    }
    fs.writeFileSync(path.join(output,"all-originals.json"),JSON.stringify(originals,null,2),{flag:"wx"});
    await capture("original-receipt");
    const http=[];
    for(const [requestId,response] of httpReplies){
      if(response.status!==200)throw new Error("真实 HTTP 状态非 200 "+response.url);
      if(!httpFinished.has(requestId))throw new Error("HTTP 回复未完成 "+response.url);
      const body=await send("Network.getResponseBody",{requestId});
      const raw=body.base64Encoded?Buffer.from(body.body,"base64").toString("utf8"):body.body;
      fs.writeFileSync(path.join(output,"http-"+http.length+".json"),raw,{flag:"wx"});
      const value=JSON.parse(raw);
      if(response.url.endsWith("/ready")){if(!["tb03b-finance/2","tb03b-finance/4"].includes(value.finance_revision))throw new Error("真实 discovery 版本不符");}
      else if(value.schema_revision!=="economic-session/2")throw new Error("真实 HTTP envelope 非 /2");
      http.push({requestId,url:response.url,status:response.status,body_file:"http-"+http.length+".json"});
    }
    fs.writeFileSync(path.join(output,"http-index.json"),JSON.stringify(http,null,2),{flag:"wx"});
    const result={status:"passed",actual_browser:true,finance_revision:discovery.finance_revision,completeness:first.completeness,
      prepared_as_known:args["--as-known"],watch_same_cut_image_commit_ns:true,originals:buttons,
      decisions:decisions.map(({decision:d})=>d.decision_id),url:url.href,
      verification_boundary:"经济 HTTP 不提供内容节点；本次不独立证明 FinancialBasis 节点闭包或七字段正文绑定，不重算无前根的顶层经济 root"};
    fs.writeFileSync(path.join(output,"result.json"),JSON.stringify(result,null,2),{flag:"wx"});console.log(JSON.stringify(result));
  }catch(error){fs.writeFileSync(path.join(output,"failure.json"),JSON.stringify({status:"failed",error:String(error)},null,2),{flag:"wx"});process.exitCode=1;}
  finally{try{await send("Browser.close",{},null);}catch{} if(child.exitCode===null)child.kill("SIGKILL");await new Promise(resolve=>{if(child.exitCode!==null||child.signalCode!==null)resolve();else child.once("exit",resolve);});log.end();}
})();
