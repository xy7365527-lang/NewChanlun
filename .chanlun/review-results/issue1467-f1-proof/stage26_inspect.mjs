// #1467：独立BigInt价量簿、原生应用日程及最大符号块检查；订单ID仅在内存使用。
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { sha, PROFILE } from './stage26_run.mjs';

const dec = s => {
  assert.equal(typeof s, 'string'); assert(/^\d+(\.\d{0,8})?$/.test(s));
  const [a,b=''] = s.split('.'); return BigInt(a)*100000000n+BigInt(b.padEnd(8,'0') || '0');
};
const seq = m => { if (typeof m.sequence === 'number') assert(Number.isSafeInteger(m.sequence)); return BigInt(m.sequence); };
function canonical(x) {
  if (Array.isArray(x)) return x.map(canonical);
  if (x && typeof x === 'object') return Object.fromEntries(Object.keys(x).sort().map(k=>[k,canonical(x[k])]));
  if (typeof x === 'number') assert(Number.isSafeInteger(x), 'reference numeric domain');
  return x;
}
class Heap {
  constructor(max) { this.a=[]; this.max=max; }
  better(a,b) { return this.max ? a>b : a<b; }
  push(v) { const a=this.a; a.push(v); let i=a.length-1; while(i){const p=(i-1)>>1;if(!this.better(a[i],a[p]))break;[a[i],a[p]]=[a[p],a[i]];i=p;} }
  pop() { const a=this.a,top=a[0],v=a.pop();if(a.length){a[0]=v;let i=0;for(;;){let j=i,l=i*2+1,r=l+1;if(l<a.length&&this.better(a[l],a[j]))j=l;if(r<a.length&&this.better(a[r],a[j]))j=r;if(j===i)break;[a[i],a[j]]=[a[j],a[i]];i=j;}}return top; }
  best(levels) { while(this.a.length && !levels.has(String(this.a[0])))this.pop();return this.a[0]; }
}
class Book {
  constructor(){this.orders=new Map();this.bids=new Map();this.asks=new Map();this.bh=new Heap(true);this.ah=new Heap(false);}
  level(buy,p){return (buy?this.bids:this.asks).get(String(p))??0n;}
  delta(buy,p,d){const map=buy?this.bids:this.asks,heap=buy?this.bh:this.ah,key=String(p),old=map.get(key)??0n,n=old+d;assert(n>=0n);if(n===0n)map.delete(key);else{map.set(key,n);if(old===0n)heap.push(p);}}
  add(id,o){assert(!this.orders.has(id),'duplicate reference order');assert(o.price>0n&&o.size>0n);this.orders.set(id,o);this.delta(o.buy,o.price,o.size);}
  snapshot(m){for(const [field,buy] of [['bids',true],['asks',false]])for(const [p,q,id]of m[field])this.add(id,{buy,price:dec(p),size:dec(q)});}
  quote(){const b=this.bh.best(this.bids),a=this.ah.best(this.asks);if(b===undefined||a===undefined)return {value:null,status:b===undefined?(a===undefined?'empty_both':'empty_bid'):'empty_ask'};if(b>=a)return {value:null,status:'crossed_or_locked'};return {value:{bid_price:String(b),bid_quantity:String(this.level(true,b)),ask_price:String(a),ask_quantity:String(this.level(false,a)),scale:100000000},status:'valid'};}
  digest(){let s='';for(const [tag,map]of [['B',this.bids],['A',this.asks]])for(const p of [...map.keys()].sort((a,b)=>BigInt(a)<BigInt(b)?-1:1))s+=`${tag},${p},${map.get(p)}\n`;return sha(Buffer.from(s));}
  orderDigest(){let s='';for(const id of [...this.orders.keys()].sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b)))){const o=this.orders.get(id);s+=`${id}\t${o.buy}\t${o.price}\t${o.size}\n`;}return sha(Buffer.from(s));}
  apply(m){
    const kind=m.type,id=kind==='match'?m.maker_order_id:m.order_id,old=this.orders.get(id);
    if(kind==='received')return {noop:'received_no_book_effect'};
    if((kind==='done'||kind==='change')&&!old)return {noop:kind==='done'?'nonresting_done':'nonresting_change'};
    if(kind==='open'){
      const o={buy:m.side==='buy',price:dec(m.price),size:dec(m.remaining_size)},before=this.level(o.buy,o.price);
      this.add(id,o);return {id,buy:o.buy,price:o.price,delta:o.size,action:'add',reason:'resting_open',barrier:false,before,after:this.level(o.buy,o.price)};
    }
    assert(old,'reference missing resting order');assert.equal(m.side,old.buy?'buy':'sell');assert.equal(dec(m.price),old.price);
    let next,action,reason,barrier=false;
    if(kind==='match'){const qty=dec(m.size);assert(qty>0n&&qty<=old.size);next=old.size-qty;action='execute';reason='resting_match';}
    else if(kind==='done'){assert.equal(dec(m.remaining_size),old.size);next=0n;action='cancel';barrier=m.reason!=='canceled';reason=barrier?'resting_done_unmapped_reason':'resting_canceled_done';}
    else if(kind==='change'){
      assert(!Object.hasOwn(m,'new_price'));assert.equal(dec(m.old_size),old.size);next=dec(m.new_size);
      if(next===old.size)return {noop:'resting_size_unchanged'};
      barrier=next>old.size;action=barrier?'add':'cancel';reason=barrier?'resting_size_increase_model_barrier':'resting_size_decrease';
    }else throw Error('unsupported reference native type');
    const delta=next-old.size,before=this.level(old.buy,old.price);
    this.delta(old.buy,old.price,delta);if(next===0n)this.orders.delete(id);else this.orders.set(id,{...old,size:next});
    return {id,buy:old.buy,price:old.price,delta,action,reason,barrier,before,after:this.level(old.buy,old.price)};
  }
}
function parseLines(bytes){const s=bytes.toString('utf8').split('\n');assert.equal(s.pop(),'');return s;}

export async function inspect(inputPath, rowsPath, reportPath) {
  const inputBytes=await fs.readFile(inputPath),rowBytes=await fs.readFile(rowsPath);
  const raw=parseLines(inputBytes),texts=parseLines(rowBytes),rows=texts.map(JSON.parse);
  const report=JSON.parse(await fs.readFile(reportPath,'utf8'));
  assert.equal(report.source_sha256,sha(inputBytes));assert.equal(report.output_file_sha256,sha(rowBytes));
  assert.equal(raw.length,rows.length);assert.equal(report.profile,PROFILE);
  let book=new Book(),frontier=null,pending=new Map(),seen=new Map(),epochs=[],current=null,fragments=new Map(),nextRank=-1;
  let modelEvents=0,completed=0,unpriced=0,ended=0,barriers=0,applied=0,delayed=0,knownLateBlocks=0;
  let outChain=sha(Buffer.from(PROFILE));
  const nativeEffects={},normalizations={},noops={};const increment=(m,k)=>{m[k]=(m[k]??0)+1;};
  function block(start,finish,ordinal,flip=null,full=false){
    const ep=current,events=ep.events.slice(start,finish),positive=events[0].model.positive;
    assert(events.length>0&&events.every(e=>e.model.positive===positive));
    const qs=start===0?ep.anchor:ep.events[start-1].after,qe=events.at(-1).after;
    const valid=!!qs.value&&events.every(e=>!!e.after.value);
    let geometry=null;
    if(valid){const a=positive?qs.value.bid_price:qs.value.ask_price,b=positive?qe.value.ask_price:qe.value.bid_price;
      const x=BigInt(a),y=BigInt(b);assert(positive?x<y:x>y);
      geometry={direction:positive?'up':'down',start_model_state:start,end_model_state:finish,start_price:a,end_price:b,low:String(x<y?x:y),high:String(x>y?x:y),scale:100000000,value_kind:'selected_quote_edges_not_trade_prices'};
    }
    const first=events[0].model;
    const v={epoch:ep.id,ordinal,positive,start,finish,first_known_model:start+1,first_known_line:first.available_line,
      first_known_capture:first.available_capture,start_quote:qs,end_quote:qe,all_quotes_valid:valid,geometry,event_count:events.length,provisional:!flip};
    if(full||flip)v.events=events.map(e=>e.model);
    if(flip){v.known_at_model=finish+1;v.known_at_line=flip.available_line;v.known_at_capture=flip.available_capture;v.confirmation_event=flip;}
    return v;
  }
  function end(reason,line,local,out){
    if(current?.events.length){const ordinal=current.starts.length-1,start=current.starts[ordinal];out.push({type:'epoch_end',reason,available_line:line,available_capture:local,tail_status:'unfinished_on_domain_exit',tail:block(start,current.events.length,ordinal,null,true)});ended++;}
    current=null;fragments.clear();
  }
  function anchor(nativeSeq,line,local,reason,out){
    end(reason,line,local,out);const id=epochs.length,q=book.quote();
    const a={epoch:id,model_state:0,native_sequence:String(nativeSeq),available_line:line,available_capture:local,origin:reason,...q};
    current={id,anchor:a,events:[],starts:[]};epochs.push(current);
    fragments=new Map([...book.orders.keys()].sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))).map((id,i)=>[id,i]));nextRank=-1;
    out.push({type:'anchor',reason,quote:a});
  }
  for(let ri=0;ri<raw.length;ri++){
    const line=ri+1,actual=rows[ri],split=raw[ri].indexOf(' '),local=raw[ri]?raw[ri].slice(0,split):null;
    const message=raw[ri]?JSON.parse(raw[ri].slice(split+1)):null;
    const applications=[],confirms=[];let activeUpdate=false;
    assert.equal(actual.line,line);assert.equal(actual.available_at_capture,local);
    if(!message){end('disconnect',line,null,applications);book=new Book();frontier=null;pending.clear();seen.clear();activeUpdate=true;}
    else if(message.type==='full_snapshot'){
      const s=seq(message),candidate=new Book();candidate.snapshot(message);assert(message.generated);
      assert(frontier===null||s>=frontier);
      if(frontier===s){assert.equal(candidate.orderDigest(),book.orderDigest(),'same-sequence snapshot book');applications.push({type:'snapshot_reaffirmed',native_sequence:String(s),available_line:line,available_capture:local});}
      else{book=candidate;anchor(s,line,local,'snapshot',applications);activeUpdate=true;}
      frontier=s;for(const key of pending.keys())if(key<=s)pending.delete(key);
    }else{
      const s=seq(message),signature=sha(Buffer.from(JSON.stringify(canonical(message))));
      if(seen.has(s))assert.equal(seen.get(s),signature,'duplicate native payload');
      else{seen.set(s,signature);if(frontier===null||s>frontier)pending.set(s,{message,line,local});}
    }
    if(message&&frontier!==null){
      while(pending.has(frontier+1n)){
        const s=frontier+1n,r=pending.get(s);pending.delete(s);frontier=s;
        const before=book.quote(),effect=book.apply(r.message),after=book.quote();applied++;if(r.line<line)delayed++;
        const item={type:'applied',native_sequence:String(s),native_type:r.message.type,captured_line:r.line,captured_at:r.local,
          available_line:line,available_capture:local,native_event_at:typeof r.message.time==='string'?r.message.time:null};
        if(effect.noop){assert.deepEqual(before,after);item.disposition='noop';item.normalization=effect.noop;increment(noops,effect.noop);applications.push(item);continue;}
        assert.equal(effect.after-effect.before,effect.delta);
        Object.assign(item,{before_quote:before.value,before_status:before.status,after_quote:after.value,after_status:after.status,buy:effect.buy,
          price:String(effect.price),signed_delta:String(effect.delta),level_before:String(effect.before),level_after:String(effect.after),normalization:effect.reason});
        if(effect.barrier){item.disposition='model_barrier';applications.push(item);barriers++;anchor(s,line,local,effect.reason,applications);activeUpdate=true;continue;}
        assert(current,'mutation without anchor');let rank;
        if(effect.action==='add'){assert(!fragments.has(effect.id));rank=nextRank--;fragments.set(effect.id,rank);}
        else{assert(fragments.has(effect.id));rank=fragments.get(effect.id);if(!book.orders.has(effect.id))fragments.delete(effect.id);}
        const positive=effect.buy?effect.action==='add':effect.action!=='add';
        if(before.value&&after.value){for(const k of ['bid_price','ask_price'])assert(positive?BigInt(after.value[k])>=BigInt(before.value[k]):BigInt(after.value[k])<=BigInt(before.value[k]));}
        const beforeRef=current.events.length?current.events.at(-1).after:current.anchor;
        assert.deepEqual(beforeRef.value,before.value);assert.equal(beforeRef.status,before.status);
        const index=current.events.length;
        const model={epoch:current.id,index,native_sequence:String(s),native_type:r.message.type,native_event_at:item.native_event_at,
          captured_line:r.line,captured_at:r.local,available_line:line,available_capture:local,model_action:effect.action,
          normalization:effect.reason,buy:effect.buy,price:String(effect.price),amount:String(effect.delta<0n?-effect.delta:effect.delta),positive,fragment:String(rank)};
        const afterRef={epoch:current.id,model_state:index+1,native_sequence:String(s),available_line:line,available_capture:local,origin:'applied_mutation',...after};
        if(index===0)current.starts.push(0);
        else if(current.events.at(-1).model.positive!==positive){
          const ordinal=current.starts.length-1,start=current.starts[ordinal],b=block(start,index,ordinal,model,true);
          confirms.push(b);completed++;if(!b.geometry)unpriced++;if(model.captured_line<line)knownLateBlocks++;
          current.starts.push(index);
        }
        current.events.push({model,after:afterRef});modelEvents++;increment(nativeEffects,model.native_type);increment(normalizations,model.normalization);
        Object.assign(item,{disposition:'mutation',model_event:model,before_model_quote:beforeRef,after_model_quote:afterRef});applications.push(item);activeUpdate=true;
      }
    }
    const q=book.quote();
    const status=!message?'disconnected':frontier===null?'awaiting_snapshot':pending.size?'sequence_gap':q.status==='valid'?'ready':q.status==='crossed_or_locked'?'crossed_or_locked_quote':'missing_quote';
    assert.equal(actual.status,status);assert.equal(actual.frontier,frontier===null?null:String(frontier));assert.equal(actual.pending,pending.size);
    assert.deepEqual(actual.current_quote,status==='ready'?q.value:null);
    assert.deepEqual(actual.applications,applications,`application mismatch at capture ${line}`);
    assert.deepEqual(actual.confirmed,confirms,`confirmation mismatch at capture ${line}`);
    assert.equal(actual.active_update,activeUpdate);
    const active=activeUpdate&&current?.events.length?block(current.starts.at(-1),current.events.length,current.starts.length-1):null;
    assert.deepEqual(actual.active,active,`active mismatch at capture ${line}`);
    outChain=sha(Buffer.concat([Buffer.from(outChain),Buffer.from(texts[ri]+'\n')]));
  }
  const endActive=current?.events.length?block(current.starts.at(-1),current.events.length,current.starts.length-1,null,true):null;
  assert.deepEqual(report.active_at_end,endActive);assert.equal(report.eof_confirms_tail,false);
  assert.equal(report.price_book_sha256,book.digest());assert.equal(report.output_chain_sha256,outChain);
  for(const [k,v]of Object.entries({model_events:modelEvents,completed_blocks:completed,completed_without_geometry:unpriced,unfinished_domain_exits:ended,barriers,applied_records:applied,delayed_records:delayed,epochs:epochs.length}))assert.equal(report[k],v,k);
  assert.deepEqual(report.native_effects,nativeEffects);assert.deepEqual(report.normalizations,normalizations);assert.deepEqual(report.noops,noops);
  return {passed:true,lines:raw.length,model_events:modelEvents,completed,unpriced,epochs:epochs.length,barriers,applied,delayed,
    delayed_confirmations:knownLateBlocks,price_book_sha256:book.digest(),report_path:reportPath,
    scope:'independent author implementation over the named finite profile, not independent semantic-agent review'};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const [input,rows,report,output]=process.argv.slice(2);if(!output)throw Error('usage: node stage26_inspect.mjs INPUT ROWS REPORT NEW_SUMMARY');
  const result=await inspect(input,rows,report);result.checker_sha256=sha(await fs.readFile(fileURLToPath(import.meta.url)));
  await fs.writeFile(output,JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});console.log(JSON.stringify(result));
}
