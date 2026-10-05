// Stage69 #1467: finite-feature fixed learner; no filesystem or evaluator access.
import {createHash} from 'node:crypto';
export const VERSION='stage69-softmax-finite-features/1';
export const CLASSES=['up','down','none'];
export const SETTINGS=Object.freeze({steps:200,learning_rate:0.05,l2:0.001});
export const A=['last_trade_price','seen_trade_count','since_last_trade_ns','observed_trade_count_1s','observed_trade_quantity_1s'];
export const B=[...A,'bid_price','ask_price','bid_quantity','ask_quantity','spread','queue_imbalance','published_l1_changes_1s','published_mid2_delta_1s'];
export const need=(x,m)=>{if(!x)throw Error(m);};
export const hash=x=>createHash('sha256').update(typeof x==='string'?x:JSON.stringify(x)).digest('hex');
export function exact(o,keys,where){need(o!==null&&typeof o==='object'&&!Array.isArray(o),where+':object');need(Object.keys(o).sort().join('|')===[...keys].sort().join('|'),where+':keys');}
export function integer(x){need(typeof x==='string'&&/^-?(0|[1-9][0-9]*)$/.test(x),'integer:string');return BigInt(x);}
const finite=(x,m)=>{need(Number.isFinite(x),m+':nonfinite');return x;};
export function featureNames(arm){need(arm==='A'||arm==='B','arm');return arm==='A'?A:B;}
export function convert(features,arm){const fields=featureNames(arm);exact(features,fields,'features');return fields.map(k=>{
 const v=features[k];if(v===null)return null;
 if(k==='queue_imbalance'){exact(v,['numerator','denominator'],'imbalance');const n=integer(v.numerator),d=integer(v.denominator);need(d>0n&&n>=-d&&n<=d,'imbalance:range');return finite(finite(Number(n),'numerator')/finite(Number(d),'denominator'),'imbalance');}
 const b=integer(v);if(!['published_mid2_delta_1s'].includes(k))need(b>=0n,'feature:negative');
 const scale=k==='since_last_trade_ns'?1e9:k==='published_mid2_delta_1s'?2e8:['seen_trade_count','observed_trade_count_1s','published_l1_changes_1s'].includes(k)?1:1e8;
 return finite(finite(Number(b),k)/scale,k);
 });}
export function preprocess(xs,fields){need(xs.length>0,'preprocess:empty');return fields.map((name,j)=>{const vals=xs.map(x=>x[j]).filter(x=>x!==null);const mean=vals.length?finite(vals.reduce((s,x)=>finite(s+x,'mean_sum'),0)/vals.length,'mean'):0;
 const variance=finite(xs.reduce((s,x)=>finite(s+(x[j]===null?0:(x[j]-mean)**2),'variance_sum'),0)/xs.length,'variance');const raw_std=finite(Math.sqrt(variance),'std');return {name,observed:vals.length,missing:xs.length-vals.length,mean,variance,raw_std,scale:raw_std===0?1:raw_std};});}
export function transform(x,prep){return [1,...x.map((v,j)=>finite(((v===null?prep[j].mean:v)-prep[j].mean)/prep[j].scale,'transform')),...x.map(v=>v===null?1:0)];}
export function softmax(z){z.forEach(x=>finite(x,'logit'));const m=Math.max(...z),e=z.map(x=>Math.exp(x-m)),s=e.reduce((a,b)=>a+b,0),p=e.map(x=>finite(x/s,'probability'));need(p.every(x=>x>=0&&x<=1)&&Math.abs(p.reduce((a,b)=>a+b,0)-1)<1e-12,'probability:range');return p;}
export function probs(weights,x){return softmax(weights.map(w=>w.reduce((s,v,j)=>finite(s+v*x[j],'dot'),0)));}
export function gradient(weights,xs,ys,l2){const g=weights.map(w=>w.map(()=>0));xs.forEach((x,i)=>{const p=probs(weights,x);p.forEach((q,k)=>x.forEach((v,j)=>{g[k][j]+=((q-(ys[i]===k?1:0))*v)/xs.length;}));});return g.map((r,k)=>r.map((v,j)=>finite(v+(j===0?0:l2*weights[k][j]),'gradient')));}
export function identity(row){need(row.product_id==='BTC-USD','product');need(integer(row.decision_ns)>0n,'decision');}
export function fit(request){exact(request,['op','version','arm','fit_cutoff_ns','rows'],'fit');need(request.op==='fit'&&request.version===VERSION,'fit:protocol');const fields=featureNames(request.arm),cut=integer(request.fit_cutoff_ns);need(Array.isArray(request.rows)&&request.rows.length>0&&request.rows.length<=572,'fit:rows');let prior=null;
 const xs=[],ys=[];for(const r of request.rows){exact(r,['product_id','decision_ns','features','label'],'training-row');identity(r);const t=integer(r.decision_ns);need(t<cut&&(prior===null||t>prior),'fit:decision-before-cutoff-and-ordered');prior=t;need(CLASSES.includes(r.label),'fit:label');xs.push(convert(r.features,request.arm));ys.push(CLASSES.indexOf(r.label));}
 const prep=preprocess(xs,fields),design=xs.map(x=>transform(x,prep));let w=CLASSES.map(()=>Array(design[0].length).fill(0));
 for(let i=0;i<SETTINGS.steps;i++){const g=gradient(w,design,ys,SETTINGS.l2);w=w.map((r,k)=>r.map((v,j)=>finite(v-SETTINGS.learning_rate*g[k][j],'weight')));}
 const model={version:VERSION,arm:request.arm,fit_cutoff_ns:request.fit_cutoff_ns,classes:CLASSES,settings:SETTINGS,feature_names:fields,preprocessing:prep,weights:w,train_rows:xs.length,class_counts:CLASSES.map((_,k)=>ys.filter(y=>y===k).length),training_request_sha256:hash(request)};
 return {model,model_sha256:hash(model)};
}
export function predict(request,state){exact(request,['op','version','product_id','decision_ns','features'],'predict');need(request.op==='predict'&&request.version===VERSION,'predict:protocol');need(state,'predict:not-fitted');identity(request);need(integer(request.decision_ns)>=integer(state.model.fit_cutoff_ns),'predict:before-fit-cutoff');const x=transform(convert(request.features,state.model.arm),state.model.preprocessing),p=probs(state.model.weights,x);let k=0;for(let j=1;j<p.length;j++)if(p[j]>p[k])k=j;return {status:'predicted',version:VERSION,arm:state.model.arm,product_id:request.product_id,decision_ns:request.decision_ns,model_sha256:state.model_sha256,classes:CLASSES,probabilities:p,prediction:CLASSES[k]};}
