// #1467：合成存在性见证。预设价位/路径不是在线预测器。
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const [dirArg]=process.argv.slice(2);assert.equal(process.argv.length,3,'usage: node stage33_fixture.mjs NEW_DIRECTORY');const dir=path.resolve(dirArg);await fs.mkdir(dir,{mode:0o700});
const bidGrid=[10020,10024,10028,10030,10036,10042,10044,10050,10052],askGrid=[10028,10030,10032,10040,10048,10050,10054,10056,10060];
const targets=[[10020,10028],[10028,10032],[10024,10030],[10044,10050],[10042,10048],[10052,10060],[10036,10050]];
let book={bids:bidGrid.filter(p=>p<=10030).reverse(),asks:askGrid.filter(p=>p>=10040)};
const books=[structuredClone(book)],events=[],ops=[];
function emit(op,price){
  const buy=op.endsWith('Bid'),add=op.startsWith('add');const side=buy?'bids':'asks';
  if(add){assert(!book[side].includes(price));book[side].unshift(price);}else{assert.equal(book[side][0],price);book[side].shift();}
  assert(book.bids.length&&book.asks.length);assert(book.bids[0]<book.asks[0]);assert.deepEqual([...book.bids].sort((a,b)=>b-a),book.bids);assert.deepEqual([...book.asks].sort((a,b)=>a-b),book.asks);
  events.push({buy,action:add?'add':'cancel',price,amount:1});ops.push(op);books.push(structuredClone(book));
}
for(let i=0;i<targets.length;i++){
  const [bid,ask]=targets[i],positive=i%2===1;
  if(positive){while(book.asks[0]<ask)emit('cancelAsk',book.asks[0]);for(const p of bidGrid.filter(p=>p<=bid&&!book.bids.includes(p)))emit('addBid',p);}
  else{while(book.bids[0]>bid)emit('cancelBid',book.bids[0]);for(const p of askGrid.filter(p=>p>=ask&&!book.asks.includes(p)).reverse())emit('addAsk',p);}
  assert.equal(book.bids[0],bid);assert.equal(book.asks[0],ask);
}
emit('cancelAsk',10050);
assert.equal(events.length,38);assert.equal(books.length,39);
const positive=e=>e.buy?e.action==='add':e.action!=='add';const groups=[];
for(let i=0;i<events.length;i++){const e=events[i];if(!groups.length||groups.at(-1).positive!==positive(e))groups.push({positive:positive(e),start:i,finish:i+1});else groups.at(-1).finish=i+1;}
assert.deepEqual(groups.map(g=>g.finish-g.start),[6,4,2,9,2,7,7,1]);
const legs=groups.map(g=>{const a=books[g.start],b=books[g.finish];return {direction:g.positive?'up':'down',start:g.start,finish:g.finish,startPrice:g.positive?a.bids[0]:a.asks[0],endPrice:g.positive?b.asks[0]:b.bids[0]};});
const list=a=>'['+a.join(',')+']';
const lean=`import MovingQuoteProof\n\nnamespace DirectReachFixture\n\nopen MovingQuote\n\ndef initial : Book := ⟨${list(books[0].bids)},${list(books[0].asks)}⟩\ndef events : List Event := [\n${events.map(e=>`  ⟨⟨${e.buy},.${e.action},1⟩,${e.price}⟩`).join(',\n')}\n]\ndef books : List Book := [\n${books.map(b=>`  ⟨${list(b.bids)},${list(b.asks)}⟩`).join(',\n')}\n]\ndef quotes : List Quote := [\n${books.map(b=>`  ⟨${b.bids[0]},${b.asks[0]}⟩`).join(',\n')}\n]\ndef expectedLegs : List NewChanlun.Origin.Segment := [\n${legs.map(s=>`  ⟨.${s.direction},${s.start},${s.finish},${s.startPrice},${s.endPrice}⟩`).join(',\n')}\n]\n\nend DirectReachFixture\n`;
const scaffold=`  unfold DirectReachFixture.books DirectReachFixture.events\n${ops.map(op=>`  apply Steps.cons\n  · exact ${op}_step _ _ _`).join('\n')}\n  exact Steps.nil _\n`;
const sha=b=>createHash('sha256').update(b).digest('hex');
await fs.writeFile(path.join(dir,'DirectReachFixture.lean'),lean,{flag:'wx'});await fs.writeFile(path.join(dir,'trajectory-proof.txt'),scaffold,{flag:'wx'});
const result={profile:'DIRECT-FLOW-REACHABILITY-v1',events,books,groups,legs,fixture_sha256:sha(lean),proof_scaffold_sha256:sha(scaffold),generator_sha256:sha(await fs.readFile(fileURLToPath(import.meta.url))),scope:'synthetic model data; all Step/Flow and source claims still require Lean verification'};
await fs.writeFile(path.join(dir,'fixture.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({events:events.length,states:books.length,lengths:groups.map(g=>g.finish-g.start),known_at:groups.map((g,i)=>i+1<groups.length?g.finish+1:null),fixture_sha256:result.fixture_sha256,output:dir}));
