import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import vm from 'node:vm';
const here=path.dirname(new URL(import.meta.url).pathname);
const base='/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage65/local-domain-author';
const out=path.join(here,'lift-control');
fs.mkdirSync(out,{recursive:true});
const hash=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const write=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2)+'\n');
let assertions=0;
function check(x,label){assertions++;if(!x)throw Error(label);}
check(hash(path.join(base,'Report.md'))==='6818337bf2adcc02f7bf5843ddeeac5b79afd585d6f60077aba5ab79d2bdcf85','frozen other-author Report');
check(hash(path.join(base,'manifest.json'))==='3a8b909f0f75b0d8e116b020751c97e59c013429101b6fa08cafe93da13de4ff','frozen other-author manifest');
const names=['source.json','segments.json','scan.json','reference.stdout'];
const inputBindings=names.map(name=>({path:path.join(base,'run',name),sha256:hash(path.join(base,'run',name))}));
const [source,segments,scan,reference]=names.map(n=>JSON.parse(fs.readFileSync(path.join(base,'run',n),'utf8')));
check(source.observations.length===343 && segments.length===16 && reference.strokes.length===84,'frozen lift counts');
const S=Object.fromEntries(segments.map(s=>[s.id,s]));
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const segIndex=id=>Number(id.slice(1));
// Reconstruct all polarity inputs from the frozen nucleus member indices.
const kernels=scan.cores.map(k=>{
  const m=k.members.map(id=>S[id]);
  check(m.length===3 && m[0].up!==m[1].up && m[1].up!==m[2].up,'triple alternation');
  const core=[Math.max(...m.map(x=>x.range[0])),Math.min(...m.map(x=>x.range[1]))];
  const outer=[Math.min(...m.map(x=>x.range[0])),Math.max(...m.map(x=>x.range[1]))];
  check(core[0]<core[1] && same(core,k.core) && same(outer,k.initial_outer),'geometry');
  return {id:k.id,members:k.members,memberOwn:[m[0].start+1,m[2].end],core,outer,
    seedHappenedAt:m[2].end,knownAt:Math.max(...m.map(x=>x.known_at)),
    polarity:m[0].up?'D':'U',pattern:m.map(x=>x.up?'U':'D').join('')};
});
const K=Object.fromEntries(kernels.map(k=>[k.id,k]));
const edgeDirection=(a,b)=>b.outer[0]>a.outer[1]?'U':b.outer[1]<a.outer[0]?'D':'O';
// Execute the EXACT same function body, rather than a second implementation.
const original=fs.readFileSync(path.join(here,'check.mjs'),'utf8');
const begin=original.indexOf('function polarityMap(cut) {');
const end=original.indexOf('\nconst timeline=[];',begin);
check(begin>=0 && end>begin,'fixed function extraction');
const functionText=original.slice(begin,end).trim();
const polarityMap=vm.runInNewContext(functionText+'\npolarityMap;',{kernels,segments,K,segIndex,edgeDirection,check});
const timeline=[];let previous=[];let firstPP=null;
for(let cut=0;cut<source.observations.length;cut++){
  const m=polarityMap(cut);check(same(m.closed.slice(0,previous.length),previous),'closed stability');
  if(cut>=1){
    const own=m.closed.flatMap(x=>Array.from({length:x.end-x.start},(_,j)=>x.start+j+1));
    const tail=Array.from({length:Math.max(0,cut-m.buffer[0]+1)},(_,j)=>m.buffer[0]+j);
    check(same([...own,...tail],Array.from({length:cut-1},(_,j)=>j+2)),'Own plus buffer');
  }
  for(let i=1;i<m.closed.length;i++){
    const a=m.closed[i-1],b=m.closed[i];
    if(a.type==='P' && b.type==='P' && !firstPP)
      firstPP={knownAt:cut,left:a,right:b,ownContiguous:a.end===b.start,
        expression:'delta-pol-independent-closed=[Y0,Y1,...]',preserveIndependentItems:true};
  }
  previous=m.closed;timeline.push(m);
}
const final=timeline.at(-1);
check(same(kernels.map(k=>k.pattern),['DUD','UDU','DUD']),'three alternating seed polarities');
check(same(final.closed.map(x=>[x.end,x.knownAt,x.type]),[[101,198,'P'],[201,298,'P']]),'two closed P');
check(final.blocks.at(-1).type==='P' && final.blocks.at(-1).end===null,'third P pending');
check(firstPP?.knownAt===298 && firstPP.ownContiguous,'actual first NoPP failure');
check(inputBindings.every(x=>hash(x.path)===x.sha256),'inputs unchanged');
write('input-bindings.json',inputBindings);
write('function-binding.json',{source:path.join(here,'check.mjs'),sourceHash:hash(path.join(here,'check.mjs')),
  exactFunctionSha256:crypto.createHash('sha256').update(functionText).digest('hex'),functionText});
write('timeline.json',timeline);
write('result.json',{observations:343,prefixes:343,segments:16,kernels:kernels.length,assertions,
  ruleChanged:false,newHistoryGenerated:false,parserExecuted:false,kernelInputs:kernels,final,
  sourceBindingsAreConditional:true,firstNoPPFailure:firstPP,
  scope:'NoPP refutes this unchanged M65-pol rule plus independent closed-item expression on the already frozen lift; not all F1 or all P/P combinations'});
console.log(JSON.stringify({assertions,closed:final.closed.map(x=>[x.end,x.knownAt,x.type]),
  firstNoPPFailure:firstPP.knownAt,third:final.blocks.at(-1).type+'-open',ruleChanged:false}));
