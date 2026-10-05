import assert from 'node:assert/strict';
import fs from 'node:fs';
const p=[0,6,2,8,3,7,0],t=[0,6,10,16,21,25,32];
const v=p.slice(1).map((x,i)=>(x-p[i])/(t[i+1]-t[i]));
assert.deepEqual(v,[1,-1,1,-1,1,-1]);
const arms=[];
for(let i=0;i<v.length;i++)for(let j=i;j<v.length;j++)
 if(v[i]===v[j])arms.push({i,j,direction:v[i],L:v[j]-v[i]});
const pairs=[];
for(const b of arms)for(const c of arms)if(b.j<c.i&&b.direction===c.direction){
 assert.equal(b.L,0);assert.equal(c.L,0);assert.equal(c.L<b.L,false);
 pairs.push({b:[b.i,b.j],c:[c.i,c.j],difference:c.L-b.L});
}
assert.ok(pairs.length>0);
const result={scope:'EQ64的固定时钟数值B控制；不是原义笔/核/结束实例',p,t,v,arms:arms.length,pairs:pairs.length,strictWeakeningPairs:0,allAssertionsPassed:true};
const out=process.argv[2];if(out)fs.writeFileSync(out,JSON.stringify(result,null,2)+'\n');
console.log(JSON.stringify(result,null,2));
