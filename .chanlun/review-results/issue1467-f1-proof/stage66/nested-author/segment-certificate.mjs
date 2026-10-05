// Exact source blocks extracted by extract.mjs; scoped finite probe only.
import assert from 'node:assert/strict';
const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const ix=xs=>[Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const hull=xs=>[Math.min(...xs.map(x=>x[0])),Math.max(...xs.map(x=>x[1]))];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];

export function segmentCertificate(ref,prices,s,cut){
function certAt(s,cut){
 const st=ref.prefixes[cut].stable,begin=5*s,mem=st.slice(begin,begin+5);if(mem.length!==5)return null;const up=mem[0].up;
 const feats=st.slice(begin).map((p,j)=>({...p,pen:begin+j,range:range(p)})).filter(p=>p.up!==up);let tri=null;
 for(let i=1;i<feats.length;i++){const a=feats[i-1].range,b=feats[i].range;if(contains(a,b)||contains(b,a))return null;if(i<2)continue;const aa=feats[i-2].range;if(up?a[1]>aa[1]&&a[1]>b[1]:a[0]<aa[0]&&a[0]<b[0]){tri=feats.slice(i-2,i+1);break;}}
 if(!tri)return null;const[a,b,c]=tri.map(p=>p.range);if(!(up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1]))return null;
 if(!(ix([a,b])[0]<ix([a,b])[1]))return null;const start=mem[0].start,end=mem.at(-1).end,whole=hull(mem.map(range)),ic=ix(mem.slice(0,3).map(range));if(!(ic[0]<ic[1]))return null;
 if(prices[end]!== (up?b[1]:b[0]))return null;if(whole[0]!==Math.min(prices[start],prices[end])||whole[1]!==Math.max(prices[start],prices[end]))return null;
 return {id:`S${s}`,up,start,end,own:[start+1,end],members:[begin,begin+5],range:whole,known_at:cut,first_selected:tri.map(p=>p.pen),selected_ranges:tri.map(p=>p.range),gap:false,initial_three_core:ic};
}

return certAt(s,cut);
}
