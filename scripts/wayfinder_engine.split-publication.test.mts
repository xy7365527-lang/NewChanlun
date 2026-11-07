import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { specBodyDigest, specRulingBody } from './wayfinder_engine.mts';

const repo = new URL('../', import.meta.url);
for (const label of ['sandcastle', 'deepagents:queue']) {
  for (const fault of ['create', 'link', 'link-ack', 'edge', 'edge-ack', 'block', 'receipt', 'publish', 'title-only', 'title-only-edge', 'title-only-held', 'unknown-dependency', 'cycle', 'none']) {
    test(`${label} 拆票事务：${fault} 中断后恢复，依赖完整才发布且不重复创建`, () => {
      const root = mkdtempSync(join(tmpdir(), 'split-publication-'));
      try {
        mkdirSync(join(root, 'scripts')); mkdirSync(join(root, '.sandcastle'));
        // 只隔离准入设施；生产拆票、裁定与发布代码原样运行。
        writeFileSync(join(root, '.sandcastle/deepagents-cutover.mts'), `export const HOLD_LABELS=['needs-info']; export const QUEUE_LABEL='deepagents:queue'; export const releaseLabel=()=>${JSON.stringify(label)}; export const graphAdmissionFrozen=()=>false; export const withGraphAdmission=async f=>f(); export const MIRROR_LABEL='deepagents'; export const withTicketHold=async (n,f)=>f(null);`);
        for (const f of ['scripts/wayfinder_engine.mts', '.sandcastle/cliproxy-env.mts', '.sandcastle/main-model.mts', '.sandcastle/scheduler-config.mts', '.sandcastle/scheduler-config.json']) writeFileSync(join(root, f), readFileSync(f === 'scripts/wayfinder_engine.mts' && process.env.SPLIT_ENGINE_SOURCE ? process.env.SPLIT_ENGINE_SOURCE : new URL(f, repo)));
        const body = '<!-- wayfinder-split -->\n```json\n'+JSON.stringify({tickets:[{id:'S1',title:'first',depends_on:fault==='cycle'?['S2']:[]},{id:'S2',title:'second',depends_on:[fault==='unknown-dependency'?'missing':'S1']}]})+'\n```';
        const ruling = (verdict: 'PASS'|'BLOCK') => ({user:{login:'fixture'}, body:specRulingBody({specNumber:2,verdict,author:'draft',reviewer:'review',digest:specBodyDigest(body),evidence:'/fake/review'}),created_at:verdict==='PASS'?'2026-11-07T01:00:00Z':'2026-11-07T02:00:00Z'});
        writeFileSync(join(root, 'state.json'), JSON.stringify({body,pass:ruling('PASS'),block:ruling('BLOCK'),issues:fault.startsWith('title-only')?[{number:101,title:'[impl] S1 first',body:'原实施票的额外验收条件',labels:fault==='title-only-held'?['deepagents','deepagents:queue']:[]},{number:102,title:'[impl] S2 second',body:'',labels:[]}]:[],links:fault.startsWith('title-only')?[101,102]:[],edges:fault==='title-only-held'?[[102,101]]:[],fault:fault==='title-only-edge'?'edge':fault,used:false,blocked:false,violations:[]}));
        writeFileSync(join(root, 'gh'), `#!${process.execPath}
const fs=require('node:fs'), p=process.env.FIXTURE+'/state.json', s=JSON.parse(fs.readFileSync(p)), a=process.argv.slice(2), cmd=a.join(' ');
const val=k=>a[a.indexOf(k)+1], save=()=>fs.writeFileSync(p,JSON.stringify(s));
const out=x=>{save();console.log(JSON.stringify(x));};
const spec={number:2,title:'SPEC: fixture',body:s.body,state:'OPEN',labels:[]};
const raw=i=>({...i,id:i.number,labels:i.labels.map(name=>({name})),assignees:[],state:'OPEN',issue_dependencies_summary:{blocked_by:s.edges.filter(e=>e[0]===i.number).length},blockedBy:{nodes:s.edges.filter(e=>e[0]===i.number).map(()=>({state:'OPEN'}))}});
const check=()=>{for(const i of s.issues) if(i.labels.some(l=>['sandcastle','deepagents:queue','ready-for-agent'].includes(l)) && !s.edges.some(e=>e[0]===102&&e[1]===101)) s.violations.push(cmd);};
const cut=k=>{if(s.fault===k&&!s.used){s.used=true;save();process.exit(73);}};
if(a[0]==='issue'&&a[1]==='list'){out(cmd.includes('wayfinder:map')?[{number:1,title:'map',body:'',state:'OPEN',labels:[]}]:s.issues.filter(i=>i.labels.includes(val('--label'))).map(raw));}
else if(a[0]==='issue'&&a[1]==='view'){out(a[2]==='2'?spec:raw(s.issues.find(i=>i.number===+a[2])));}
else if(a[0]==='issue'&&a[1]==='create'){const i={number:101+s.issues.length,title:val('--title'),body:fs.readFileSync(val('--body-file'),'utf8'),labels:a.flatMap((x,k)=>x==='--label'?[a[k+1]]:[])};s.issues.push(i);check();cut('create');save();console.log('https://github.com/fixture/repo/issues/'+i.number);}
else if(a[0]==='issue'&&a[1]==='edit'){const i=s.issues.find(i=>i.number===+a[2]);if(a.includes('--body'))i.body=val('--body');if(a.includes('--body-file'))i.body=fs.readFileSync(val('--body-file'),'utf8');for(let k=0;k<a.length;k++){if(a[k]==='--add-label') i.labels=[...new Set([...i.labels,...a[k+1].split(',')])];if(a[k]==='--remove-label')i.labels=i.labels.filter(l=>!a[k+1].split(',').includes(l));}check();if(a.includes('--body')&&i.body.includes('wayfinder-published:'))cut('receipt');if(a.includes('--add-label'))cut('publish');out({});}
else if(a[0]==='api') {
 const ep=a.find(x=>x.startsWith('repos/'));
 if(cmd.includes('--method POST')){if(ep.endsWith('/sub_issues')){cut('link');const n=+a.find(x=>x.startsWith('sub_issue_id=')).split('=')[1];s.links=[...new Set([...s.links,n])];cut('link-ack');if(s.links.length===2&&s.fault==='block'&&!s.used){s.used=true;s.blocked=true;}}
 else if(ep.endsWith('/dependencies/blocked_by')){cut('edge');const n=+ep.split('/').at(-3), d=+a.find(x=>x.startsWith('issue_id=')).split('=')[1];s.edges.push([n,d]);cut('edge-ack');}else process.exit(74);check();out({});}
 else if(ep.endsWith('/1/sub_issues'))out([spec,...s.issues.filter(i=>s.links.includes(i.number)).map(raw)]);
 else if(ep.endsWith('/2/comments'))out([s.pass,...(s.blocked?[s.block]:[])]);
 else if(ep.includes('/dependencies/blocked_by'))out([s.edges.filter(e=>e[0]===+ep.split('/').at(-3)).map(e=>({id:e[1],number:e[1],state:'OPEN'}))]);
 else if(/issues\\?/.test(ep))out([[],s.issues.map(raw)]);
 else if(/issues\\/\\d+$/.test(ep)){save();console.log(+ep.split('/').pop());}
 else {console.error('unexpected '+cmd);process.exit(74);}
} else {console.error('unexpected '+cmd);process.exit(74);}
`, {mode:0o755});
        const run = () => spawnSync(process.execPath, ['--experimental-strip-types','scripts/wayfinder_engine.mts','--live','--once'],{cwd:root,env:{...process.env,PATH:`${root}:${process.env.PATH}`,FIXTURE:root,GH_REPO:'fixture/repo',WAYFINDER_ORCHESTRATOR:'fixture',WAYFINDER_GH_MAX_ATTEMPTS:'1'},encoding:'utf8',timeout:30000});
        const read = () => JSON.parse(readFileSync(join(root,'state.json'),'utf8'));
        const first=run(); assert.equal(first.status,0,first.stderr);
        assert.deepEqual(read().violations,[], '依赖不完整期间已经可消费');
        if (fault==='title-only-held') {
          for (let n=0;n<2;n++) {
            const held=read().issues[0];
            assert.deepEqual(held.labels,['deepagents','deepagents:queue'],'迁移不得摘除在途持有的放行标签');
            assert.equal(held.body,'原实施票的额外验收条件','持有期间不改实施合同');
            if(n===0) run();
          }
          // 生产释放（executor release，排空后）只摘镜像；放行标签与票面其他部分不动。
          const released=read(); released.issues[0].labels=released.issues[0].labels.filter((l: string)=>l!=='deepagents'); writeFileSync(join(root,'state.json'),JSON.stringify(released));
          for(let n=0;n<3;n++) run();
          assert.equal(read().issues.length,2,'释放后恢复迁移，不重复建票');
          assert.match(read().issues[0].body,/wayfinder-published:/,'释放后完成发布');
          return;
        }

        if (fault==='unknown-dependency' || fault==='cycle') {
          assert.equal(read().issues.length,0,'无效依赖计划不得部分发布');
          assert.match(first.stderr,/拆票计划/);
          return;
        }
        if (fault==='block') assert.ok(read().issues.every((i: {labels:string[]}) => i.labels.length===0),'当前 BLOCK 不得发布');
        const s=read(); s.blocked=false; writeFileSync(join(root,'state.json'),JSON.stringify(s));
        for(let n=0;n<3;n++){const r=run();assert.equal(r.status,0,r.stderr);assert.doesNotMatch(r.stderr,/unexpected|执行失败|释放失败|拉取 tracker 状态失败/);}
        const done=read(); assert.equal(done.issues.length,2,'创建后失联不能重复建票');
        if (fault.startsWith('title-only')) assert.match(done.issues[0].body,/原实施票的额外验收条件/);
        assert.deepEqual(done.links.sort(),[101,102]);
        assert.deepEqual(done.edges,[[102,101]],'恢复补边且不重复加边');
        assert.deepEqual(done.violations,[]);
        assert.ok(done.issues.every((i: { labels: string[] })=>i.labels.includes('ready-for-agent')));
        assert.ok(done.issues[0].labels.includes(label));
        assert.ok(!done.issues[1].labels.includes(label),'依赖未关不得派工 S2');
      } finally {rmSync(root,{recursive:true,force:true});}
    });
  }
}
