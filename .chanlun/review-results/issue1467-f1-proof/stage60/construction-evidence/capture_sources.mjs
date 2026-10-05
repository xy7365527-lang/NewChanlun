import fs from'node:fs';import path from'node:path';import crypto from'node:crypto';import{fileURLToPath}from'node:url';
const here=path.dirname(fileURLToPath(import.meta.url)),R=path.resolve(here,'../../../../..'),output=process.argv[2];
const declarations=[
['docs/chanlun/text/blog/053-第53课.md',[12,18,20,24],36,{}],
['docs/chanlun/text/blog/057-第57课.md',[12,26,28,30,32,40],48,{}],
['docs/chanlun/text/blog/061-第61课.md',[12,26],56,{}],
['docs/chanlun/text/blog/067-第67课.md',[12,18,20,22,28],62,{20:'adopt text before （娇：; exclude the remainder',28:'adopt text before （娇注：; exclude the remainder'}],
['docs/chanlun/text/blog/078-第78课.md',[12,18,20,22,24],64,{}],
['docs/chanlun/text/blog/081-第81课.md',[100,102,108,110,112,114],78,{100:'a separate appended author blog begins here; not the first article body',102:'authorship of appended blog',110:'adopted new-stroke rule in that appended author text; no inline editorial mark'}],
['docs/chanlun/text/blog/084-第84课.md',[12,52,54,56,60],68,{}],
['.chanlun/definitions/beichi.md',[263,264,265,266,267,268,313,314,315,316,317,318,319,320,321,322,323,336,337,338,342,348,356,357,365,370],null,{265:'D-3: exclude center construction members, not all same-direction boundary crossing geometric intervals',338:'#873 is repository inference, not verbatim Chan author definition'}],
['.chanlun/definitions/zhongshu.md',[132,134,136,138,140,142,144,146],null,{}],
['formal/Origin/ForceVelocity.lean',[21,22,30,31,32,53,54,55,60,61,62,116,117],null,{}]
];
const out=declarations.map(([p,ns,boundary,notes])=>{const text=fs.readFileSync(path.join(R,p),'utf8'),lines=text.split('\n');return{path:p,sha256:crypto.createHash('sha256').update(text).digest('hex'),first_body_boundary:boundary,lines:ns.map(n=>({line:n,text:lines[n-1],scope:notes[n]??(boundary?(n<boundary?'first author body/header; line-start and inline marks visually checked':'see separate author provenance'):'canonical definition or Lean source')}))};});
fs.writeFileSync(output,JSON.stringify(out,null,2)+'\n',{flag:'wx'});
