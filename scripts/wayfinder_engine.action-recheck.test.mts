/** #1915：完整单轮入口的扫描后变更反例；假 gh 拒绝所有未声明调用，不接触 GitHub。 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { specBodyDigest, specRulingBody } from "./wayfinder_engine.mts";

const repo = new URL("../", import.meta.url);
function run(path: "split" | "queue", change: "body" | "block" | "closed" | "valid" | "second-create", nested = false) {
  const root = mkdtempSync(join(tmpdir(), "wfe-recheck-"));
  try {
    mkdirSync(join(root, "scripts")); mkdirSync(join(root, ".sandcastle"));
    const drain = join(root, "drain"), state = join(root, "exec");
    mkdirSync(drain); mkdirSync(state);
    writeFileSync(join(drain, "state.json"), JSON.stringify({ v: 1, mode: "open", generation: 1 }));
    for (const file of ["scripts/wayfinder_engine.mts", ".sandcastle/cliproxy-env.mts", ".sandcastle/main-model.mts",
      ".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.json", ".sandcastle/drain-control.mts",
      ".sandcastle/drain-control.py", ".sandcastle/deepagents-cutover.mts", ".sandcastle/queue-policy.json"]) {
      let text = readFileSync(new URL(file, repo), "utf8");
      if (file.endsWith("drain-control.mts")) text = text.replace('"/Volumes/AgentStorage/newchanlun-legacy-drain"', JSON.stringify(drain));
      if (file.endsWith("deepagents-cutover.mts")) text = text.replace('"/Volumes/AgentStorage/deepagents-exec"', JSON.stringify(state));
      writeFileSync(join(root, file), text);
    }
    const body = path === "split" ? '<!-- wayfinder-split -->\n```json\n' + JSON.stringify({ tickets: [
      { id: "S1", title: "first", summary: "reviewed A", depends_on: [], rulings: [8] },
      { id: "S2", title: "second", summary: "reviewed A", depends_on: ["S1"], rulings: [8] },
    ] }) + '\n```' : "reviewed A";
    const ruling = (spec: number, verdict: "PASS" | "BLOCK") => ({ user: { login: "fixture" },
      body: specRulingBody({ specNumber: spec, verdict, author: "draft", reviewer: "review", digest: specBodyDigest(body), evidence: "/fixture/review.json" }),
      created_at: verdict === "PASS" ? "2026-11-07T01:00:00Z" : "2026-11-07T02:00:00Z" });
    writeFileSync(join(root, "fixture.json"), JSON.stringify({ path, change, nested, body,
      pass: ruling(2, "PASS"), block: ruling(2, "BLOCK"), innerPass: ruling(3, "PASS") }));
    writeFileSync(join(root, "writes"), "");
    writeFileSync(join(root, "gh"), `#!${process.execPath}
const fs = require('node:fs');
const root = process.env.WFE_FIXTURE, f = JSON.parse(fs.readFileSync(root+'/fixture.json'));
const args = process.argv.slice(2), cmd = args.join(' ');
const read = p => fs.existsSync(root+'/'+p) ? fs.readFileSync(root+'/'+p, 'utf8') : '';
const write = () => fs.appendFileSync(root+'/writes', JSON.stringify(args)+'\\n');
const out = x => console.log(JSON.stringify(x));
const ticket = {number:99,title:'[impl] existing',state:'OPEN',labels:[{name:'ready-for-agent'}],assignees:[]};
const spec = n => ({number:n,title:'SPEC: fixture',state:'OPEN',body:f.body,sub_issues_summary:{total:f.nested ? 1 : 0,completed:0}});
const changed = read('scanned') && (f.change !== 'second-create' || read('created'));
if(args[0]==='issue' && args[1]==='list') {
  if(cmd.includes('wayfinder:map')) out([{number:1,title:'map',body:'',state:'OPEN',labels:[]}]);
  else { fs.writeFileSync(root+'/scanned','yes'); out(f.path==='queue' ? [ticket] : []); }
} else if(args[0]==='api' && args[1]==='--paginate') {
  const endpoint=args[2];
  if(endpoint.endsWith('/1/sub_issues')) out(f.nested ? [spec(2)] : [spec(2), ...(f.path==='queue'?[ticket]:[])]);
  else if(endpoint.includes('/issues?') || endpoint.endsWith('/dependencies/blocked_by')) out([]);
  else if(endpoint.endsWith('/2/sub_issues')) out([spec(3)]);
  else if(endpoint.endsWith('/3/sub_issues')) out([ticket]);
  else if(endpoint.endsWith('/2/comments')) out([f.pass,...(changed && f.change==='block'?[f.block]:[])]);
  else if(endpoint.endsWith('/3/comments')) out([f.innerPass]);
  else process.exit(71);
} else if(args[0]==='issue' && args[1]==='view') {
  if(args[2]==='2' || args[2]==='3') out({...spec(+args[2]),
    state:changed && f.change==='closed' && args[2]==='2'?'CLOSED':'OPEN',
    body:changed && ['body','second-create'].includes(f.change) && args[2]==='2'?f.body.replaceAll('reviewed A','unreviewed B'):f.body});
  else out(ticket);
} else if(args[0]==='issue' && args[1]==='create') {
  write(); const n=read('created')?102:101; fs.writeFileSync(root+'/created','yes'); console.log('https://github.com/fixture/repo/issues/'+n);
} else if(args[0]==='issue' && args[1]==='edit') {write();}
else if(args[0]==='api' && cmd.includes('--method POST')) {write();out({});}
else if(args[0]==='api' && /issues\\/(101|102)$/.test(args[1])) {console.log(+args[1].split('/').pop());}
else {console.error('Unexpected fake gh: '+cmd);process.exit(71);}
`, { mode: 0o755 });
    const result = spawnSync(process.execPath, ["--experimental-strip-types", "scripts/wayfinder_engine.mts", "--live", "--once"], {
      cwd: root, env: { ...process.env, PATH: `${root}:${process.env.PATH}`, WFE_FIXTURE: root,
        GH_REPO: "fixture/repo", WAYFINDER_ORCHESTRATOR: "fixture", WAYFINDER_GH_MAX_ATTEMPTS: "1" },
      encoding: "utf8", timeout: 20_000,
    });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /一轮结束/);
    assert.match(result.stdout, path === "split" ? /spec 已裁定 PASS/ : /待释放 1/);
    assert.doesNotMatch(result.stderr, /Unexpected fake gh|执行失败|释放失败|拉取 tracker 状态失败/);
    return { writes: readFileSync(join(root, "writes"), "utf8").trim().split("\n").filter(Boolean).map(s => JSON.parse(s) as string[]), out: result.stdout + result.stderr };
  } finally { rmSync(root, { recursive: true, force: true }); }
}

for (const path of ["split", "queue"] as const) {
  for (const change of ["body", "block", "closed"] as const) test(`${path}：扫描 PASS 后 ${change}，副作用前重新裁定并拒绝`, () => {
    const result = run(path, change);
    assert.deepEqual(result.writes, [], result.out);
  });
  test(`${path}：当前合法 PASS 仍可执行`, () => {
    const result = run(path, "valid");
    const actions = result.writes.filter(a => a[0] === "issue" && (path !== "split" || a[1] === "create"));
    assert.equal(actions.length, path === "split" ? 2 : 1, result.out);
    assert.equal(actions[0]![1], path === "split" ? "create" : "edit");
  });
}
test("嵌套 SPEC：内层 PASS 不能覆盖扫描后外层 BLOCK", () => {
  assert.deepEqual(run("queue", "block", true).writes, []);
});
test("拆票中途正文变更：不继续按旧计划建第二票", () => {
  const result = run("split", "second-create");
  assert.equal(result.writes.filter(a => a[0] === "issue" && a[1] === "create").length, 1, result.out);
});

test("嵌套 SPEC：外层与内层当前均 PASS 才挂队列", () => {
  assert.equal(run("queue", "valid", true).writes.filter(a => a[1] === "edit").length, 1);
});
