// C64v3 finite-value codec. Input domain: finite, acyclic JSON data.
export class EncodingError extends Error {}
const fail = message => { throw new EncodingError(message); };
const integer = value => {
  if(typeof value!=='string'||value!==value.trim()||!/^[-]?[0-9]+$/.test(value))fail('invalid integer string');
  return BigInt(value);
};
const natural = value => {
  if(typeof value!=='string'||value!==value.trim()||!(/^[0-9]+$/).test(value))fail('invalid natural string');
  return BigInt(value);
};
const hex = value => {
  if(typeof value!=='string'||value!==value.trim()||value.length%2!==0||!(/^[0-9a-fA-F]*$/).test(value))fail('invalid byte hex');
  return value.toLowerCase();
};
const gcd = (a,b) => { while(b!==0n){const r=a%b;a=b;b=r;} return a; };
export function normalize(x) {
  if(!Array.isArray(x))fail('node must be an array');
  switch(x[0]) {
    case 'n': if(x.length!==2)fail('natural arity');return ['n',natural(x[1]).toString()];
    case 'z': if(x.length!==2)fail('integer arity');return ['z',integer(x[1]).toString()];
    case 'q': {
      if(x.length!==3)fail('rational arity');
      let a=integer(x[1]),b=integer(x[2]);if(b===0n)fail('zero denominator');
      if(b<0n){a=-a;b=-b;}const g=gcd(a<0n?-a:a,b);
      return ['q',(a/g).toString(),(b/g).toString()];
    }
    case 'b': if(x.length!==2)fail('bytes arity');return ['b',hex(x[1])];
    case 'l': {
      if(x.length!==2||!Array.isArray(x[1]))fail('list arity');
      return ['l',x[1].map(normalize)];
    }
    default: fail('unknown node tag');
  }
}
export const encode=x=>JSON.stringify(normalize(x));
const schemaHex=Buffer.from('C64v3-record','ascii').toString('hex');
export function recordValue(r) {
  if(r===null||typeof r!=='object'||Array.isArray(r))fail('record must be JSON object');
  const keys=Object.keys(r).sort();
  if(JSON.stringify(keys)!==JSON.stringify(['F','q','rho','s','u']))fail('record fields must be exact');
  return normalize(['l',[
    ['b',schemaHex],['b',r.rho],['n',r.q],['n',r.s],['n',r.u],r.F,
  ]]);
}
export const recordCode=r=>JSON.stringify(recordValue(r));
export function bucket(records) {
  if(!Array.isArray(records))return {kind:'EncodingError',message:'records must be finite array'};
  const codes=[];
  try {
    for(const r of records){const code=recordCode(r);if(!codes.some(c=>c===code))codes.push(code);}
  } catch(e) {
    if(e instanceof EncodingError)return {kind:'EncodingError',message:e.message};
    throw e; // Resource or runtime failures are not a mathematical encoding verdict.
  }
  return {kind:codes.length===0?'empty':codes.length===1?'single':'ValueConflict',codes};
}
export function active0(events,rho,q,sText) {
  if(!Array.isArray(events))fail('history must be finite event list');
  for(const e of events)normalize(e); // E1 example event values also use the finite carrier.
  const k=BigInt(events.length),s=natural(sText);if(s>k)fail('frontier outside history');
  const identity=['l',[['b',hex(rho)],['n',natural(q).toString()],['n',s.toString()]]];
  const status=s===k?'EmptyBuffer':'AwaitingEvidence';
  return normalize(['l',[identity,['n',k.toString()],['b',Buffer.from(status,'ascii').toString('hex')],
    ['n',s.toString()],['n',k.toString()],['n',(k-s).toString()]]]);
}
