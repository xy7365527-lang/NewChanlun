/**
 * #1915 N1/N2：旧票迁移与 Deep Agents 持有生命周期。真实切流模块与票级 flock 原样运行，
 * 只把执行器状态根与排空根指到临时目录；假 gh 拒绝所有未声明调用，不接触 GitHub。
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { specBodyDigest, specRulingBody } from "./wayfinder_engine.mts";

const repo = new URL("../", import.meta.url);
type Issue = { number: number; title: string; body: string; labels: string[]; state?: "OPEN" | "CLOSED" };
type Write = { n: number; args: string; before: string[] };
type Claim = { state: string; run_id: string; generation: number };

const FAKE_GH = `#!${process.execPath}
const fs=require('node:fs'), root=process.env.FIXTURE, p=root+'/state.json', s=JSON.parse(fs.readFileSync(p)), a=process.argv.slice(2), cmd=a.join(' ');
const val=k=>a[a.indexOf(k)+1], save=()=>fs.writeFileSync(p,JSON.stringify(s));
const out=x=>{save();console.log(JSON.stringify(x));};
const spec={number:2,title:'SPEC: fixture',body:s.body,state:'OPEN',labels:[]};
const raw=i=>({...i,id:i.number,labels:i.labels.map(name=>({name})),assignees:[],state:i.state??'OPEN',issue_dependencies_summary:{blocked_by:s.edges.filter(e=>e[0]===i.number).length},blockedBy:{nodes:s.edges.filter(e=>e[0]===i.number).map(()=>({state:'OPEN'}))}});
if(a[0]==='issue'&&a[1]==='list'){out(cmd.includes('wayfinder:map')?[{number:1,title:'map',body:'',state:'OPEN',labels:[]}]:s.issues.filter(i=>i.labels.includes(val('--label'))&&(i.state??'OPEN')==='OPEN').map(raw));}
else if(a[0]==='issue'&&a[1]==='view'){out(a[2]==='2'?spec:raw(s.issues.find(i=>i.number===+a[2])));}
else if(a[0]==='issue'&&a[1]==='create'){const i={number:101+s.issues.length,title:val('--title'),body:fs.readFileSync(val('--body-file'),'utf8'),labels:[]};s.issues.push(i);save();console.log('https://github.com/fixture/repo/issues/'+i.number);}
else if(a[0]==='issue'&&a[1]==='edit'){const i=s.issues.find(i=>i.number===+a[2]);s.writes.push({n:+a[2],args:a.slice(3).join(' '),before:[...i.labels]});if(a.includes('--body'))i.body=val('--body');for(let k=0;k<a.length;k++){if(a[k]==='--add-label') i.labels=[...new Set([...i.labels,...a[k+1].split(',')])];if(a[k]==='--remove-label')i.labels=i.labels.filter(l=>!a[k+1].split(',').includes(l));}out({});}
else if(a[0]==='api') {
 const ep=a.find(x=>x.startsWith('repos/'));
 if(cmd.includes('--method POST')){if(ep.endsWith('/sub_issues')){const n=+a.find(x=>x.startsWith('sub_issue_id=')).split('=')[1];s.links=[...new Set([...s.links,n])];}
 else if(ep.endsWith('/dependencies/blocked_by')){const n=+ep.split('/').at(-3), d=+a.find(x=>x.startsWith('issue_id=')).split('=')[1];s.edges.push([n,d]);}else process.exit(74);out({});}
 else if(ep.endsWith('/1/sub_issues'))out([spec,...s.issues.filter(i=>s.links.includes(i.number)).map(raw)]);
 else if(ep.endsWith('/2/comments'))out([s.pass]);
 else if(ep.includes('/dependencies/blocked_by'))out([s.edges.filter(e=>e[0]===+ep.split('/').at(-3)).map(e=>({id:e[1],number:e[1],state:'OPEN'}))]);
 else if(/issues\\?/.test(ep)){
  if(s.onScan&&!s.scanned){s.scanned=true;const t=s.issues.find(x=>x.number===101);
   if(s.onScan==='mirror')t.labels=[...new Set([...t.labels,'deepagents'])];
   if(s.onScan==='intent'){fs.mkdirSync(root+'/exec/claims',{recursive:true});fs.writeFileSync(root+'/exec/claims/101.json',JSON.stringify({state:'intent',run_id:'t101-1',generation:1}));}}
  out([[],s.issues.map(raw)]);}
 else if(/issues\\/\\d+$/.test(ep)){save();console.log(+ep.split('/').pop());}
 else {console.error('unexpected '+cmd);process.exit(74);}
} else {console.error('unexpected '+cmd);process.exit(74);}
`;

function fixture(issues: Issue[], o: { claim?: Claim; onScan?: "mirror" | "intent" } = {}) {
  const root = mkdtempSync(join(tmpdir(), "wfe-hold-"));
  const drain = join(root, "drain"), exec = join(root, "exec");
  for (const dir of [join(root, "scripts"), join(root, ".sandcastle"), drain, join(exec, "claims")]) mkdirSync(dir, { recursive: true });
  writeFileSync(join(drain, "state.json"), JSON.stringify({ v: 1, mode: "open", generation: 1 }));
  for (const file of ["scripts/wayfinder_engine.mts", ".sandcastle/cliproxy-env.mts", ".sandcastle/main-model.mts",
    ".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.json", ".sandcastle/drain-control.mts",
    ".sandcastle/drain-control.py", ".sandcastle/deepagents-cutover.mts", ".sandcastle/queue-policy.json"]) {
    let text = readFileSync(new URL(file, repo), "utf8");
    if (file.endsWith("drain-control.mts")) text = text.replace('"/Volumes/AgentStorage/newchanlun-legacy-drain"', JSON.stringify(drain));
    if (file.endsWith("deepagents-cutover.mts")) text = text.replace('"/Volumes/AgentStorage/deepagents-exec"', JSON.stringify(exec));
    if (file === "scripts/wayfinder_engine.mts" && process.env.HOLD_ENGINE_SOURCE) text = readFileSync(process.env.HOLD_ENGINE_SOURCE, "utf8");
    writeFileSync(join(root, file), text);
  }
  if (o.claim) writeFileSync(join(exec, "claims", "101.json"), JSON.stringify(o.claim));
  const body = "<!-- wayfinder-split -->\n```json\n" + JSON.stringify({ tickets: [
    { id: "S1", title: "first", depends_on: [] }, { id: "S2", title: "second", depends_on: ["S1"] }] }) + "\n```";
  const pass = { user: { login: "fixture" }, created_at: "2026-11-07T01:00:00Z",
    body: specRulingBody({ specNumber: 2, verdict: "PASS", author: "draft", reviewer: "review", digest: specBodyDigest(body), evidence: "/fake/review" }) };
  writeFileSync(join(root, "state.json"), JSON.stringify({ body, pass, issues, links: issues.map(i => i.number),
    edges: [[102, 101]], writes: [], onScan: o.onScan ?? null }));
  writeFileSync(join(root, "gh"), FAKE_GH, { mode: 0o755 });
  const run = () => spawnSync(process.execPath, ["--experimental-strip-types", "scripts/wayfinder_engine.mts", "--live", "--once"], {
    cwd: root, encoding: "utf8", timeout: 30_000,
    env: { ...process.env, PATH: `${root}:${process.env.PATH}`, FIXTURE: root, GH_REPO: "fixture/repo",
      WAYFINDER_ORCHESTRATOR: "fixture", WAYFINDER_GH_MAX_ATTEMPTS: "1" } });
  const read = () => JSON.parse(readFileSync(join(root, "state.json"), "utf8")) as { issues: Issue[]; writes: Write[] };
  const edit = (f: (s: { issues: Issue[] }) => void) => { const s = read(); f(s); writeFileSync(join(root, "state.json"), JSON.stringify(s)); };
  return { root, exec, run, read, edit, done: () => rmSync(root, { recursive: true, force: true }) };
}

const legacy = (labels: string[], state: "OPEN" | "CLOSED" = "OPEN"): Issue[] => [
  { number: 101, title: "[impl] S1 first", body: "原实施票的额外验收条件", labels, state },
  { number: 102, title: "[impl] S2 second", body: "", labels: [] },
];
const published = (i: Issue) => /wayfinder-published:/.test(i.body);

test("N1 已关旧票：持有记录与镜像都还在，迁移照常完成整份 SPEC 的发布", () => {
  const f = fixture(legacy(["deepagents", "deepagents:queue"], "CLOSED"), { claim: { state: "active", run_id: "t101-1", generation: 1 } });
  try {
    for (let n = 0; n < 3; n++) {
      const r = f.run();
      assert.equal(r.status, 0, r.stderr);
      assert.doesNotMatch(r.stderr, /执行失败|unexpected/, r.stderr);
    }
    const [s1, s2] = f.read().issues;
    assert.ok(published(s1!) && published(s2!), "两张计划票都补齐发布凭据");
    assert.ok(!s1!.labels.includes("ready-for-agent"), "已关票不被放行");
    assert.ok(s2!.labels.includes("ready-for-agent"), "S2 发布");
  } finally { f.done(); }
});

test("N1 开着的旧票：释放排空前镜像在、每轮等待且零写入；释放摘镜像后迁移完成", () => {
  const f = fixture(legacy(["deepagents", "deepagents:queue"]),
    { claim: { state: "released", run_id: "t101-1", generation: 1 } });
  try {
    for (let n = 0; n < 2; n++) {
      const r = f.run();
      assert.equal(r.status, 0, r.stderr);
      assert.match(r.stderr, /#101 票面仍有 deepagents 镜像.*重跑 executor release/);
    }
    assert.deepEqual(f.read().writes.filter(w => w.n === 101), [], "持有未结束时不碰旧票");
    f.edit(s => { s.issues[0]!.labels = s.issues[0]!.labels.filter(l => l !== "deepagents"); });
    for (let n = 0; n < 3; n++) assert.doesNotMatch(f.run().stderr, /执行失败/);
    const s = f.read();
    assert.equal(s.issues.length, 2, "不重复建票");
    assert.ok(published(s.issues[0]!) && published(s.issues[1]!));
    assert.match(s.issues[0]!.body, /原实施票的额外验收条件/);
  } finally { f.done(); }
});

for (const onScan of ["mirror", "intent"] as const) {
  test(`N2 分页后才出现的持有（${onScan === "mirror" ? "镜像标签" : "本机 intent 记录、镜像未挂"}）：锁内重读后不摘放行标签、不改正文`, () => {
    const f = fixture(legacy(["deepagents:queue"]), { onScan });
    try {
      const r = f.run();
      assert.equal(r.status, 0, r.stderr);
      const s = f.read();
      assert.deepEqual(s.writes.filter(w => w.n === 101), [], JSON.stringify(s.writes));
      assert.ok(s.issues[0]!.labels.includes("deepagents:queue"));
      assert.equal(s.issues[0]!.body, "原实施票的额外验收条件");
      assert.match(r.stderr, /未完整发布的 #101 (票面仍有 deepagents 镜像|正由 t101-1 持有（intent）)/);
    } finally { f.done(); }
  });
}

test("N2 执行器正持票级锁（领取或释放进行中）：迁移不写，下一轮重试", async () => {
  const f = fixture(legacy(["deepagents:queue"]));
  const holder = spawn("python3", ["-c", "import fcntl,os,sys\nfd=os.open(sys.argv[1],os.O_RDWR|os.O_CREAT,0o600)\nfcntl.flock(fd,fcntl.LOCK_EX)\nprint('locked',flush=True)\nsys.stdin.read()",
    join(f.exec, "claims", "101.lock")], { stdio: ["pipe", "pipe", "inherit"] });
  try {
    await new Promise<void>((resolve, reject) => {
      holder.stdout.once("data", () => resolve());
      holder.once("exit", () => reject(new Error("lock holder exited")));
    });
    const r = f.run();
    assert.equal(r.status, 0, r.stderr);
    assert.deepEqual(f.read().writes.filter(w => w.n === 101), []);
    assert.match(r.stderr, /#101 的票级动作锁正被执行器持有/);
  } finally {
    holder.stdin.end();
    await new Promise(resolve => holder.once("exit", resolve));
    f.done();
  }
});
