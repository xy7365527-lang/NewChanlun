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
const pending=new Map();
child.stdio[4].on("data",chunk=>{
  buffer=Buffer.concat([buffer,chunk]);
  while(buffer.includes(0)){
    const at=buffer.indexOf(0),raw=buffer.subarray(0,at).toString();buffer=buffer.subarray(at+1);
    const message=JSON.parse(raw);log.write(JSON.stringify({direction:"received",message})+"\n");
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
    if(args["--as-known"]){
      await evaluate("document.getElementById('known-at').value="+JSON.stringify(args["--as-known"])+";document.getElementById('as-known').click();true");
      await capture("prepared-as-known");
      await evaluate("document.getElementById('watch').click();true");
      const after=await capture("changes-to-current");
      for(const [domain,view] of Object.entries(first.domains))if(view.status==="Available" && JSON.stringify(view.image)!==JSON.stringify(after.domains[domain].image))throw new Error("变化与原当前切面不等 "+domain);
    }
    await evaluate("Array.from(document.querySelectorAll('button')).find(b=>b.textContent.startsWith('查询原件')).click();true");
    const end=Date.now()+15000;
    while(Date.now()<end && await evaluate("Array.from(document.querySelectorAll('button')).some(b=>b.textContent.startsWith('查询原件')&&b.disabled)"))await new Promise(r=>setTimeout(r,50));
    const original=await evaluate("Array.from(document.querySelectorAll('pre')).map(p=>p.textContent).find(t=>t.includes('EconomicOriginal'))");
    if(!original)throw new Error("浏览器原件按钮未取得真实回执");
    await capture("original-receipt");
    const decisions=Object.values(first.domains).filter(v=>v.status==="Available").flatMap(v=>v.image.finance?.decisions||[]);
    if(decisions.length<2||decisions.some(d=>d.state!=="Committed"))throw new Error("成功浏览器域缺两重真实 Committed");
    const result={status:"passed",actual_browser:true,decisions:decisions.map(d=>d.decision_id),url:url.href};
    fs.writeFileSync(path.join(output,"result.json"),JSON.stringify(result,null,2),{flag:"wx"});console.log(JSON.stringify(result));
  }catch(error){fs.writeFileSync(path.join(output,"failure.json"),JSON.stringify({status:"failed",error:String(error)},null,2),{flag:"wx"});process.exitCode=1;}
  finally{try{await send("Browser.close",{},null);}catch{} if(child.exitCode===null)child.kill("SIGKILL");await new Promise(resolve=>{if(child.exitCode!==null||child.signalCode!==null)resolve();else child.once("exit",resolve);});log.end();}
})();
