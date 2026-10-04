import assert from 'node:assert/strict';
const gcd=(a,b)=>b?gcd(b,a%b):a;
export function observation(q){const a=1000n+102n*BigInt(q),b=10n+BigInt(q),g=gcd(a,b);return `${a/g}/${b/g}`;}

// Batch dictionary replay. Only initial and raw events are read as inputs.
export function replay(source,count=source.events.length){
  const book=new Map();
  for(const side of ['bid','ask']){const o=source.initial[side];assert.ok(!book.has(o.id));book.set(o.id,{...o,side});}
  const states=[],snapshots=[];
  function observe(event){
    const orders=[...book.values()].filter(o=>o.quantity>0);
    for(const o of orders)assert.ok(Number.isSafeInteger(o.quantity)&&o.quantity>0);
    const bs=orders.filter(o=>o.side==='bid'),as=orders.filter(o=>o.side==='ask');
    const bid=Math.max(...bs.map(o=>o.price)),ask=Math.min(...as.map(o=>o.price));
    assert.equal(bid,100);assert.equal(ask,102);
    const q=bs.filter(o=>o.price===bid).reduce((n,o)=>n+o.quantity,0);
    const aq=as.filter(o=>o.price===ask).reduce((n,o)=>n+o.quantity,0);assert.equal(aq,10);
    states.push({event,q,w:observation(q)});
    snapshots.push({event,orders:orders.map(o=>({id:o.id,side:o.side,price:o.price,quantity:o.quantity})),bestBid:bid,bestAsk:ask,bestBidQuantity:q,bestAskQuantity:aq});
  }
  observe(0);
  for(let i=0;i<count;i++){
    const e=source.events[i];assert.equal(e.seq,i+1);assert.ok(Number.isSafeInteger(e.quantity)&&e.quantity>0);
    if(e.kind==='add'){assert.ok(!book.has(e.id));assert.ok(['bid','ask'].includes(e.side));book.set(e.id,{...e});}
    else{assert.equal(e.kind,'reduce');const o=book.get(e.id);assert.ok(o&&o.quantity>=e.quantity);o.quantity-=e.quantity;if(!o.quantity)book.delete(e.id);}
    observe(i+1);
  }
  return {states,snapshots};
}

// Separate persistent order ledger for the independent streaming leg parser.
export class NativeStream{
  constructor(source){this.ledger=Object.create(null);for(const side of ['bid','ask']){const o=source.initial[side];this.ledger[o.id]={price:o.price,amount:o.quantity,side};}this.seq=0;}
  quote(){
    let bestBid=-Infinity,bestAsk=Infinity,q=0,askQ=0;
    for(const o of Object.values(this.ledger)){if(o.side==='bid')bestBid=Math.max(bestBid,o.price);else bestAsk=Math.min(bestAsk,o.price);}
    for(const o of Object.values(this.ledger)){if(o.side==='bid'&&o.price===bestBid)q+=o.amount;if(o.side==='ask'&&o.price===bestAsk)askQ+=o.amount;}
    assert.equal(bestBid,100);assert.equal(bestAsk,102);assert.equal(askQ,10);return q;
  }
  apply(e){
    assert.equal(e.seq,++this.seq);
    if(e.kind==='reduce'){assert.ok(this.ledger[e.id]);this.ledger[e.id].amount-=e.quantity;assert.ok(this.ledger[e.id].amount>=0);if(!this.ledger[e.id].amount)delete this.ledger[e.id];}
    else{assert.equal(e.kind,'add');assert.ok(!this.ledger[e.id]);this.ledger[e.id]={amount:e.quantity,price:e.price,side:e.side};}
    return this.quote();
  }
}
