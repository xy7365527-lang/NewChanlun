import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
const root='/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun';
const dest=process.argv[2];
if(!dest || !path.isAbsolute(dest))throw new Error('absolute destination required');
const specs=[
 ['docs/chanlun/text/blog/017-第17课.md',[[12,14],[36,58]],'作者正文；引用40的最低止点、44/46/52的类型约束，不要求初始材料无限递归'],
 ['docs/chanlun/text/blog/018-第18课.md',[[12,12],[24,44]],'作者正文；24只采用(娇：之前及其后作者句，不采用括号内注；46整行编注不引用'],
 ['docs/chanlun/text/blog/020-第20课.md',[[12,12],[30,32]],'作者正文；30是原义次级波动延伸约束，不能免角色前件'],
 ['docs/chanlun/text/blog/031-第31课.md',[[32,32],[376,386],[408,418]],'正文界之后作者答疑；382在==后、386在===后为所用作者答复；416在==后仅作者反问。问题文本不当作者结论；不采用390/420加粗重复作为独立来源'],
 ['docs/chanlun/text/blog/035-第35课.md',[[12,24]],'作者正文；16的笔是成交笔；本轮订单观察不冒充成交'],
 ['docs/chanlun/text/blog/038-第38课.md',[[12,12],[18,30],[44,44]],'作者正文；22/26保留同级读法许可和非同级NoPP的适用域'],
 ['docs/chanlun/text/blog/053-第53课.md',[[12,12],[18,34],[36,36]],'作者正文；24盘背用于中枢震荡；32震荡内买卖点级别不能偷换'],
 ['docs/chanlun/text/blog/057-第57课.md',[[12,12],[26,42],[48,48]],'作者正文；32保留真实次级角色要求；40只给局部分解比较与回抽'],
 ['docs/chanlun/text/blog/061-第61课.md',[[12,12],[26,28],[44,44],[56,56]],'作者正文；44明确前三成核、第四离开、第五回试'],
 ['docs/chanlun/text/blog/064-第64课.md',[[12,14],[32,36]],'作者正文；34的注意括号是作者连续论证，非注家标记，采用线段以下无中枢的句子'],
 ['docs/chanlun/text/blog/067-第67课.md',[[12,28],[62,62]],'作者正文；20在（娇：前、28在（娇注：前采用；后缀注排除'],
 ['docs/chanlun/text/blog/069-第69课.md',[[12,14],[30,34],[46,46]],'作者正文；不采用36/38的娇注'],
 ['docs/chanlun/text/blog/083-第83课.md',[[12,22],[36,36]],'作者正文；14承认可设计而判稳定性差，18解释特征确认；不改写为数学不可构造'],
 ['docs/chanlun/text/blog/084-第84课.md',[[12,12],[52,60],[68,68]],'作者正文；52唯一性、54初始以前不递归、56最低核与走势a1；并不直接给本候选合格证'],
 ['.chanlun/definitions/zhongshu.md',[[1,9],[132,184]],'已裁#812 S-5/S-2；是现行约束，和生产选型分开'],
 ['.chanlun/definitions/level_recursion.md',[[24,49],[91,112]],'24-49旧Move[0]接口只作对象区分，不当任意初始模型定理；91-112为#812明确撤销'],
 ['.chanlun/definitions/qushi.md',[[40,50],[74,90]],'已裁#815相应级别计数；方向引用回到031:386原答复'],
 ['.chanlun/definitions/beichi.md',[[261,268],[309,323],[336,357],[521,547]],'现行#814比较域/#865完成动力学/#873力度定义，非作者逐字公式'],
 ['docs/adr/0011-operation-decomposition-layer.md',[[1,17],[63,74],[88,95]],'现行架构正本；70明标按消费角色分为推断；NoPP及旁路不回灌仅约束所选消费角色'],
];
const result=specs.map(([file,spans,use])=>{
 const buf=fs.readFileSync(path.join(root,file)), lines=buf.toString('utf8').split('\n');
 const selected=[];for(const[a,b]of spans)for(let n=a;n<=b;n++)selected.push({line:n,text:lines[n-1]});
 return {path:file,bytes:buf.length,sha256:crypto.createHash('sha256').update(buf).digest('hex'),author_markers:lines.map((text,i)=>({line:i+1,text})).filter(x=>x.text.includes('作者：')||x.text.includes('↑正文')).slice(0,8),use,selected};
});
fs.mkdirSync(dest,{recursive:true});
fs.writeFileSync(path.join(dest,'source-excerpts.json'),JSON.stringify({scope:'Stage61 initial object interpretation only; 027:854/856 not used',files:result},null,2)+'\n');
console.log(JSON.stringify({files:result.length,lines:result.reduce((n,x)=>n+x.selected.length,0)}));
