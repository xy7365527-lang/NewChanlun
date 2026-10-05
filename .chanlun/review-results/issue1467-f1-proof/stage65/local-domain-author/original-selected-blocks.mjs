const range=p=>[Math.min(p.start_price,p.end_price),Math.max(p.start_price,p.end_price)];
const ix=xs=>[Math.max(...xs.map(x=>x[0])),Math.min(...xs.map(x=>x[1]))];
const hull=xs=>[Math.min(...xs.map(x=>x[0])),Math.max(...xs.map(x=>x[1]))];
const contains=(a,b)=>a[0]<=b[0]&&b[1]<=a[1];

function certAt(s,cut){
 const st=ref.prefixes[cut].stable,begin=5*s,up=s%2===0,mem=st.slice(begin,begin+5);if(mem.length!==5)return null;
 const feats=st.slice(begin).map((p,j)=>({...p,pen:begin+j,range:range(p)})).filter(p=>p.up!==up);let tri=null;
 for(let i=1;i<feats.length;i++){const a=feats[i-1].range,b=feats[i].range;if(contains(a,b)||contains(b,a))return null;if(i<2)continue;const aa=feats[i-2].range;if(up?a[1]>aa[1]&&a[1]>b[1]:a[0]<aa[0]&&a[0]<b[0]){tri=feats.slice(i-2,i+1);break;}}
 if(!tri)return null;const[a,b,c]=tri.map(p=>p.range);if(!(up?b[0]>a[0]&&b[0]>c[0]:b[1]<a[1]&&b[1]<c[1]))return null;
 if(!(ix([a,b])[0]<ix([a,b])[1]))return null;const start=mem[0].start,end=mem.at(-1).end,whole=hull(mem.map(range)),ic=ix(mem.slice(0,3).map(range));if(!(ic[0]<ic[1]))return null;
 if(prices[end]!== (up?b[1]:b[0]))return null;if(whole[0]!==Math.min(prices[start],prices[end])||whole[1]!==Math.max(prices[start],prices[end]))return null;
 return {id:`S${s}`,up,start,end,own:[start+1,end],members:[begin,begin+5],range:whole,known_at:cut,first_selected:tri.map(p=>p.pen),selected_ranges:tri.map(p=>p.range),gap:false,initial_three_core:ic};
}

const force=s=>{let first=ref.strokes[s.members[0]],last=ref.strokes[s.members[1]-1];let v=p=>(p.end_price-p.start_price)/(observations[p.end].time-observations[p.start].time);return{first_pen:s.members[0],last_pen:s.members[1]-1,first_v:v(first),last_v:v(last),L:v(last)-v(first)};};
const cores=[],objects=[],attempts=[];let cursor=0;
while(cursor+4<segments.length){const startSeg=cursor,up=segments[cursor].up,ownedCores=[];let cidx=cursor+4,bidx=cursor,localFailure=null;
 while(cidx<segments.length){let mem=segments.slice(cidx-3,cidx),core=ix(mem.map(s=>s.range)),outer=hull(mem.map(s=>s.range));let K={id:`K${cores.length}`,grade:0,members:mem.map(s=>s.id),member_own:[mem[0].own[0],mem.at(-1).own[1]],core,initial_outer:outer,seed_happened_at:mem.at(-1).end,known_at:mem.at(-1).known_at,lifecycle:'seed_frozen; successor relation checked separately'};assert(core[0]<core[1]);
 if(ownedCores.length){let prev=ownedCores.at(-1);let separated=up?prev.initial_outer[1]<outer[0]:outer[1]<prev.initial_outer[0];if(!separated){localFailure={stage:'strict-initial-outer-order',previous:prev.id,next:K.id};break;}}
 cores.push(K);ownedCores.push(K);let b=segments[bidx],c=segments[cidx],bf=force(b),cf=force(c);let previousPrices=prices.slice(segments[startSeg].start,c.start+1);let priorExtreme=up?Math.max(...previousPrices):Math.min(...previousPrices);let extreme=up?prices[c.end]>priorExtreme:prices[c.end]<priorExtreme;
 let weak=cf.L<bf.L,holds=weak&&extreme&&b.up===c.up;let attempt={object_index:objects.length,cores:ownedCores.map(k=>k.id),b:b.id,c:c.id,b_force:bf,c_force:cf,strict_weakening:weak,strict_extreme:extreme,prior_extreme:priorExtreme,endpoint:prices[c.end],GeneralDiv64:holds,happened_at:c.end,known_at:c.known_at};attempts.push(attempt);
 if(holds){let ownSeg=segments.slice(startSeg,cidx+1),whole=hull(ownSeg.map(s=>s.range));let obj={id:`X${objects.length}`,grade:0,type:ownedCores.length===1?'P':up?'U':'D',technical_direction:up?'Up':'Down',start:ownSeg[0].start,end:c.end,own:[ownSeg[0].own[0],c.end],whole_range:whole,members:ownSeg.map(s=>s.id),cores:ownedCores.map(k=>k.id),general_div:attempt,completed:true,completion_status:'candidate interpretation only',original_completed_certified:false,source_next_start:c.end};objects.push(obj);cursor=cidx+1;break;}
 bidx=cidx;cidx+=4;
 }
 if(localFailure){put('scan-failure.json',localFailure);break;}if(cidx>=segments.length)break;
}
