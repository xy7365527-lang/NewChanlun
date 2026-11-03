import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, renameSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import * as sandcastle from "@ai-hero/sandcastle";
import { createBindMountSandboxProvider, type SandboxProvider } from "@ai-hero/sandcastle";
import {
  ROUND_FILES, acceptedFor, bindMerges, checkControlledAdmission, inspectFrozenRound, issueFrozenReceipt, issueReceipt, loadMergePlan, publishMergeAdmission,
  mergeBinding, recordAcceptance, swarmBinding, ROW_ENV, type AcceptanceRow, type DeploymentSelection,
} from "./docker-acceptance.mts";
import { runDeploymentAcceptance, parseCli, deploymentStatus, type DeploymentDeps, type DeploymentOutcome } from "./deployment-acceptance.mts";
import { DrainFrozenError, LockBusyError, withFileLock } from "./drain-control.mts";
import { noSandbox } from "@ai-hero/sandcastle/sandboxes/no-sandbox";
import { createSwarmSandbox, preserveFailedInit, repoInitLock } from "./worktree-init.mts";
import { runDeploymentSwarmMerge } from "./swarm-merge.mts";

const HELPER = fileURLToPath(new URL("./drain-control.py", import.meta.url));
const ACCEPTANCE_CLI = fileURLToPath(new URL("./docker-acceptance.mts", import.meta.url));
const CHILD = fileURLToPath(new URL("./fixtures/issue1867/merge-child.mts", import.meta.url));
const IMG_ID = "sha256:" + "1".repeat(64);
const IMAGE = "sandcastle:newchanlun";
const TICKET = 4321;
const PARTNER_ISSUE = 1840;
const PROBE = "docs/agents/sandcastle-docker-acceptance-1742.md";
const OBS = {
  apparmor: "docker-default (enforce)", seccomp: "2", capEff: "0000000000000000", capBnd: "00000000a80425fb",
  roWrite: "Read-only file system", rwWrite: "ok", roChildInRw: "Read-only file system", roFileAppend: "Read-only file system",
  renameMountpoint: "Device or resource busy", hardlinkAcross: "Invalid cross-device link", symlinkIntoRo: "Read-only file system",
};
const report = (files: string[], status = "ready") => `<sandcastle-result>${JSON.stringify({ status, files, verification: [] })}</sandcastle-result>`;
const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));
async function untilFree(path: string) {
  for (let i = 0; i < 50; i++) {
    try { await withFileLock(path, () => undefined, { nonblock: true }); return; }
    catch (e) { if (!(e instanceof LockBusyError)) throw e; await delay(100); }
  }
  throw new Error(`${path} 未释放`);
}
const drain = (root: string, ...args: string[]) => execFileSync("python3", [HELPER, ...args, "--root", root], { encoding: "utf8" });

function barrier() {
  let open!: () => void; let hit!: () => void;
  const released = new Promise<void>((r) => { open = r; });
  const reached = new Promise<void>((r) => { hit = r; });
  return { reached, open, wait: async () => { hit(); await released; } };
}
function spawnTracked(cmd: string, args: string[]) {
  const child = spawn(cmd, args, { stdio: ["ignore", "pipe", "pipe"] });
  let out = ""; child.stdout.on("data", (d) => { out += String(d); }); child.stderr.on("data", (d) => { out += String(d); });
  let exited = false;
  const done = new Promise<number | null>((r) => child.on("exit", (code) => { exited = true; r(code); }));
  return { child, done, exited: () => exited, output: () => out };
}

function world(t: { after: (fn: () => void) => void }, o: { candidatePath?: string } = {}) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "nc-1867-")));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo"); mkdirSync(repo);
  const git = (...args: string[]) => execFileSync("git", args, { cwd: repo, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-q", "-b", "main"); git("config", "user.name", "Test"); git("config", "user.email", "test@example.invalid");
  writeFileSync(join(repo, "README.md"), "base\n"); writeFileSync(join(repo, ".gitignore"), ".sandcastle/worktrees/\n"); git("add", "."); git("commit", "-qm", "old base");
  const oldBase = git("rev-parse", "HEAD");
  git("checkout", "-q", "-b", `sandcastle/issue-${PARTNER_ISSUE}`);
  mkdirSync(join(repo, "docs/agents"), { recursive: true });
  writeFileSync(join(repo, PROBE), "# Sandcastle Docker 验收探针（#1742）\n本文件由宿主冲突伙伴写入。\n写入方：宿主冲突伙伴\n"); git("add", "."); git("commit", "-qm", "partner fixture");
  const partnerSha = git("rev-parse", "HEAD");
  git("checkout", "-q", "main");
  writeFileSync(join(repo, "deploy.txt"), "deployment\n"); git("add", "."); git("commit", "-qm", "deployment head");
  const head = git("rev-parse", "HEAD");
  const partnerReview = join(root, "partner-review.md"); writeFileSync(partnerReview, `VERDICT: PASS\n伙伴 ${partnerSha} 只新增探针文档三行。\n`);
  const partner = { issue: PARTNER_ISSUE, branch: `sandcastle/issue-${PARTNER_ISSUE}`, sha: partnerSha, parent: oldBase, review: partnerReview, approval: "https://example.invalid/approval#partner" };
  const dir = mkdtempSync(join(root, "accept-")); const drainRoot = mkdtempSync(join(root, "drain-"));
  drain(drainRoot, "freeze");
  const identity = { head, dirty: "" };
  const ctx = { dir, repo, identity, imageIdOf: () => IMG_ID, roots: [] as string[] };
  const creates: string[] = []; const gh: string[][] = []; const events: string[] = [];
  const onCreate: Array<(o: { worktreePath: string }) => Promise<void> | void> = [];
  const provider = () => createBindMountSandboxProvider({ name: "fixture", create: async (opts) => {
    creates.push(opts.worktreePath); for (const f of onCreate) await f(opts);
    return { worktreePath: "/home/agent/workspace", exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }), copyFileIn: async () => undefined, copyFileOut: async () => undefined, close: async () => undefined };
  } }) as unknown as SandboxProvider;
  const candidateText = "# Sandcastle Docker 验收探针（#1742）\n本文件由本轮第 13 行作者写入。\n写入方：作者\n";
  const deps = (extra: Partial<DeploymentDeps> = {}): DeploymentDeps => ({
    ctx, drainRoot,
    probe: () => { events.push("isolation"); return OBS; },
    modelProbe: () => { events.push("model"); },
    provider, mergeProvider: provider,
    gh: (args) => { gh.push(args); return JSON.stringify({ number: TICKET, title: "冻结验收探针", body: "只新增探针文档三行", labels: [], comments: [] }); },
    fetchIssue: () => "冻结验收探针",
    runAgent: async (sb, role) => {
      events.push(role);
      if (role === "implementer") {
        mkdirSync(join(sb.worktreePath, dirname(o.candidatePath ?? PROBE)), { recursive: true });
        writeFileSync(join(sb.worktreePath, o.candidatePath ?? PROBE), candidateText);
        return { stdout: report([o.candidatePath ?? PROBE]), iterations: [] };
      }
      return { stdout: report([]), iterations: [{ sessionId: "review-1" }] };
    },
    docker: (args) => (args[0] === "ps" ? "" : "[]"),
    runConflictAgent: async ({ cwd, sandbox }) => {
      const h = await (sandbox as unknown as { create: (x: unknown) => Promise<{ close: () => Promise<void> }> }).create({ worktreePath: cwd, hostRepoPath: repo, mounts: [{ hostPath: cwd, sandboxPath: "/home/agent/workspace" }], env: {} });
      writeFileSync(join(cwd, PROBE), "# Sandcastle Docker 验收探针（#1742）\n冲突已由工蜂解决。\n写入方：作者与宿主冲突伙伴\n");
      await h.close();
      return report([PROBE]);
    },
    ...extra,
  });
  const issue = (extra: Record<string, unknown> = {}) => issueFrozenReceipt({ image: IMAGE, approval: "https://example.invalid/1867#issuecomment-6603636406", swarmTicket: TICKET, generation: 1, partner, ...ctx, drainRoot, ...extra });
  const claims = () => readdirSync(dir).filter((f) => /^(used|resume|cp)-/.test(f)).sort();
  const branchOf = (b: string) => { try { return git("rev-parse", "--verify", "-q", `refs/heads/${b}`); } catch { return ""; } };
  const sel = (nonce: string, row: AcceptanceRow): DeploymentSelection => ({ ticket: TICKET, head, nonce, row });
  const reviewFile = (sha: string, model = "claude-opus-5-5") => { const p = join(root, `review-${sha.slice(0, 8)}-${model}.md`); writeFileSync(p, `VERDICT: PASS\nHEAD: ${sha}\nREVIEWER_MODEL: ${model}\n`); return p; };
  const admissionFile = (sha: string, nonce: string) => { const p = join(root, `admit-${sha.slice(0, 8)}-${nonce.slice(0, 8)}.md`); writeFileSync(p, `VERDICT: ADMIT\nHEAD: ${sha}\nNONCE: ${nonce}\n`); return p; };
  const bind = (nonce: string, cand: string, extra: Record<string, unknown> = {}) => bindMerges({ ...ctx, ticket: TICKET, head, nonce, candidateReview: reviewFile(cand), controllerAdmission: admissionFile(cand, nonce), ...extra });
  const noSideEffects = (why: string) => {
    assert.deepEqual(claims(), [], `${why}：无 used/resume/cp`);
    assert.deepEqual(creates, [], `${why}：没有容器启动`);
    assert.equal(branchOf(`sandcastle/issue-${TICKET}`), "", `${why}：没有专票分支`);
    assert.equal(existsSync(join(repo, ".sandcastle/worktrees", `sandcastle-issue-${TICKET}`)), false, `${why}：没有任务工位`);
  };
  return { root, repo, git, head, oldBase, partner, partnerSha, dir, drainRoot, ctx, identity, creates, gh, events, onCreate, deps, issue, claims, branchOf, sel, bind, reviewFile, admissionFile, noSideEffects };
}

test("完整冻结验收轮：row13 候选 pending → bind 封定 → rows14/15 真实冲突完成 → official accept；终态拒绝重放，accepted 跨 resume 保留", async (t) => {
  const w = world(t);
  const r = await w.issue();
  assert.equal(r.purpose.kind, "frozen-deployment");
  const out13 = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  assert.equal(out13.state, "candidate_pending");
  assert.deepEqual(w.events, ["isolation", "model", "implementer", "reviewer"], "锁内隔离→模型预检→消费→启动；作者与独审在锁外");
  assert.equal(w.creates.length, 1);
  assert.equal(w.git("rev-parse", `${out13.head}^`), w.head, "候选唯一父＝本轮部署 head");
  assert.equal(w.git("diff", "--name-only", w.head, out13.head), PROBE);
  assert.ok(w.gh.every((a) => a.slice(0, 3).join(" ") === `issue view ${TICKET}`), "只读取专票，无 gh 写入");
  assert.equal(existsSync(join(w.repo, ".sandcastle/logs/workers.jsonl")), false, "不写业务 workers.jsonl（harvest 不可见）");
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  assert.match(readFileSync(join(round.roundDir, ROUND_FILES.log), "utf8"), /"status":"consumed"/);

  const plan = await w.bind(r.nonce, out13.head);
  assert.deepEqual(plan.runs["14"].manifest.inputs.map((i) => i.sha), [out13.head, w.partnerSha], "固定顺序：本轮候选、固定伙伴");
  assert.notEqual(plan.runs["14"].manifest.runId, plan.runs["15"].manifest.runId);
  assert.notEqual(plan.runs["14"].cwd, plan.runs["15"].cwd);
  assert.notEqual(plan.runs["14"].instance, plan.runs["15"].instance);
  const again = await w.bind(r.nonce, out13.head);
  assert.deepEqual(again, plan, "相同输入重复登记读回原计划，不生成新实例");

  for (const row of ["14", "15"] as const) {
    const out: DeploymentOutcome = await runDeploymentAcceptance(w.sel(r.nonce, row), w.deps());
    assert.equal(out.state, "merge_done");
    const log: string = w.git("log", "--format=%B", out.head);
    const runId: string = plan.runs[row].manifest.runId;
    const inputs: string[] = [out13.head, w.partnerSha];
    for (const sha of inputs) assert.equal(log.split("\n").filter((l) => l === `Sandcastle-Merge: ${runId}/${sha}`).length, 1, `第 ${row} 行每个输入唯一 trailer`);
  }
  assert.equal(w.creates.length, 3, "row13 一次 + 两行各一次冲突工蜂");
  assert.deepEqual(w.claims().filter((f) => f.startsWith("used-")).length, 3);

  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), /完整完成结果，拒绝终态重放/);
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /已有本轮候选结果/);
  assert.equal(w.creates.length, 3);

  const evidence = Object.fromEntries((["13", "14", "15"] as const).map((row) => { const p = join(w.root, `ev-${row}`); writeFileSync(p, row); return [row, p]; })) as Record<AcceptanceRow, string>;
  const rec = recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) });
  assert.equal(rec.round?.candidate, out13.head);
  assert.ok(acceptedFor(IMAGE, w.ctx));
  drain(w.drainRoot, "resume", "--generation", "1");
  assert.ok(acceptedFor(IMAGE, w.ctx), "按批准 resume 后 accepted 仍有效（历史 scope，不依赖 drain）");
  assert.deepEqual(deploymentStatus({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx).merges?.["15"].phase, "done");
});

test("official accept 关联本轮结果：完成结果被换、或缺行结果都拒绝；used 不能单独证明完成", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  await w.bind(r.nonce, c.head);
  await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps());
  const evidence = Object.fromEntries((["13", "14", "15"] as const).map((row) => { const p = join(w.root, `ev-${row}`); writeFileSync(p, row); return [row, p]; })) as Record<AcceptanceRow, string>;
  assert.throws(() => recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) }), /第 15 行尚未经受控入口真实运行/);
  await runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps());
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  const resultPath = join(round.roundDir, ROUND_FILES.result("15"));
  const good = readFileSync(resultPath, "utf8");
  writeFileSync(resultPath, good.replace(/"instance": "[^"]+"/, '"instance": "forged"'));
  assert.throws(() => recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) }), /第 15 行没有本 run 的完成结果/);
  writeFileSync(resultPath, good);
  recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) });
  writeFileSync(resultPath, good.replace(c.head, "f".repeat(40)));
  assert.equal(acceptedFor(IMAGE, w.ctx), null, "登记后结果事实被改，acceptedFor 失效");
});

test("身份与授权负控：错 nonce/专票/head、脏代码、过期、仓库或镜像漂移、普通 receipt、合法普通 accepted 都在副作用前拒绝", async (t) => {
  const w = world(t); const r = await w.issue();
  const attempts: Array<[string, DeploymentSelection, Partial<DeploymentDeps>, RegExp]> = [
    ["错 nonce", { ...w.sel(r.nonce, "13"), nonce: "00000000-0000-4000-8000-000000000000" }, {}, /不是当前冻结验收轮/],
    ["错专票", { ...w.sel(r.nonce, "13"), ticket: 1237 }, {}, /不是本轮绑定/],
    ["同树不同 SHA", { ...w.sel(r.nonce, "13"), head: "e".repeat(40) }, {}, /不是本轮 receipt\.head/],
    ["脏代码", w.sel(r.nonce, "13"), { ctx: { ...w.ctx, identity: { head: w.head, dirty: " M .sandcastle/x.mts" } } }, /代码不精确/],
    ["执行代码不是部署 head", w.sel(r.nonce, "13"), { ctx: { ...w.ctx, identity: { head: w.oldBase, dirty: "" } } }, /不是本轮部署 head/],
    ["过期", w.sel(r.nonce, "13"), { ctx: { ...w.ctx, now: new Date(Date.parse(r.expiresAt) + 1000) } }, /已过期/],
    ["仓库漂移", w.sel(r.nonce, "13"), { ctx: { ...w.ctx, repo: w.root } }, /不是本轮绑定仓库/],
    ["镜像漂移", w.sel(r.nonce, "13"), { ctx: { ...w.ctx, imageIdOf: () => "sha256:" + "2".repeat(64) } }, /镜像 Id 漂移/],
  ];
  for (const [why, sel, extra, re] of attempts) {
    await assert.rejects(runDeploymentAcceptance(sel, w.deps(extra)), re, why);
    w.noSideEffects(why);
  }
  assert.throws(() => parseCli(["run", "--ticket", String(TICKET), "--head", w.head, "--nonce", r.nonce, "--row", "16"]), /验收行非法/);
  assert.throws(() => parseCli(["run", "--ticket", String(TICKET), "--head", w.head, "--nonce", r.nonce, "--row", "13", "--root", "/tmp"]), /不接受的参数 --root/);
  assert.throws(() => parseCli(["run", "--ticket", String(TICKET), "--head", w.head, "--nonce", r.nonce, "--row", "13", "--manifest", "x"]), /不接受的参数 --manifest/);
  assert.throws(() => checkControlledAdmission(IMAGE, { ...w.ctx, env: { [ROW_ENV]: "13" }, binding: swarmBinding(TICKET) }), /冻结部署验收用途，只能经 deployment-acceptance/);
  w.noSideEffects("普通受控入口");
});

test("accepted 不能短路专用身份：普通 controlled receipt 及其合法 accepted 存在时专用入口仍拒绝", async (t) => {
  const w = world(t);
  issueReceipt({ image: IMAGE, approval: "普通受控", swarmTicket: TICKET, ...w.ctx });
  const cp = (runId: string) => mergeBinding({ runId, base: "b".repeat(40), instance: "i", head: "c".repeat(40), incoming: "d".repeat(40) });
  for (const [row, binding] of [["13", swarmBinding(TICKET)], ["14", cp("r14")], ["15", cp("r15")]] as const) checkControlledAdmission(IMAGE, { ...w.ctx, env: { [ROW_ENV]: row }, binding }).consume();
  const evidence = Object.fromEntries((["13", "14", "15"] as const).map((row) => { const p = join(w.root, `ev-${row}`); writeFileSync(p, row); return [row, p]; })) as Record<AcceptanceRow, string>;
  recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) });
  assert.ok(acceptedFor(IMAGE, w.ctx), "普通 accepted 合法有效");
  const before = w.claims();
  await assert.rejects(runDeploymentAcceptance({ ticket: TICKET, head: w.head, nonce: JSON.parse(readFileSync(join(w.dir, "receipt.json"), "utf8")).nonce, row: "13" }, w.deps()), /不是冻结部署验收用途/);
  assert.deepEqual(w.claims(), before); assert.deepEqual(w.creates, []);
});

test("drain 负控：签发与运行只认 draining 且代次一致；open 或新代次都 DrainFrozenError，零消费零启动", async (t) => {
  const w = world(t);
  drain(w.drainRoot, "resume", "--generation", "1");
  await assert.rejects(w.issue(), DrainFrozenError, "open 时不能签发冻结验收 receipt");
  assert.equal(existsSync(join(w.dir, "receipt.json")), false);
  drain(w.drainRoot, "freeze");
  await assert.rejects(w.issue(), DrainFrozenError, "代次不符");
  const r = await w.issue({ generation: 2 });
  drain(w.drainRoot, "resume", "--generation", "2");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), DrainFrozenError);
  w.noSideEffects("open");
  drain(w.drainRoot, "freeze");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), DrainFrozenError);
  w.noSideEffects("代次漂移");
  assert.deepEqual(w.events, [], "冻结拒绝在探针之前");
  assert.match(drain(w.drainRoot, "status"), /"generation": 3/, "验收命令从不改写 mode/generation");
});

test("签发负控：固定伙伴唯一父不符、独审缺失或不指向 SHA、伙伴即专票、专票分支已存在都拒绝；伙伴父不要求等于部署 head", async (t) => {
  const w = world(t);
  assert.notEqual(w.partner.parent, w.head);
  await assert.rejects(w.issue({ partner: { ...w.partner, parent: w.head } }), /唯一父与批准输入不符/);
  await assert.rejects(w.issue({ partner: { ...w.partner, review: join(w.root, "missing.md") } }), /独审报告不存在/);
  const wrong = join(w.root, "wrong-review.md"); writeFileSync(wrong, "VERDICT: PASS\n别的提交\n");
  await assert.rejects(w.issue({ partner: { ...w.partner, review: wrong } }), /不是指向该 SHA 的 PASS/);
  await assert.rejects(w.issue({ partner: { ...w.partner, extra: 1 } }), /未知字段/);
  await assert.rejects(w.issue({ swarmTicket: PARTNER_ISSUE }), /不能是本轮专票/);
  w.git("branch", `sandcastle/issue-${TICKET}`, w.oldBase);
  await assert.rejects(w.issue(), /已存在，不能作为本轮新候选/);
  assert.equal(existsSync(join(w.dir, "receipt.json")), false);
});

test("可执行性负控：隔离/模型预检失败、未登记孤儿目录、专票分支被他人占用、工位目录为符号链接，均不写 used 不启动；孤儿 sentinel 保留", async (t) => {
  const w = world(t); const r = await w.issue();
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps({ probe: () => ({ ...OBS, capBnd: "000001ffffffffff" }) })), /隔离|cap/i);
  w.noSideEffects("隔离失败");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps({ modelProbe: () => { throw new Error("HTTP 502 overloaded"); } })), /502/);
  w.noSideEffects("模型失败");
  const orphan = join(w.repo, ".sandcastle/worktrees/someone-else"); mkdirSync(orphan, { recursive: true }); writeFileSync(join(orphan, "WIP.txt"), "他人未知工作");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /未登记目录/);
  w.noSideEffects("孤儿目录");
  assert.equal(readFileSync(join(orphan, "WIP.txt"), "utf8"), "他人未知工作");
  rmSync(orphan, { recursive: true });
  rmSync(join(w.repo, ".sandcastle/worktrees"), { recursive: true });
  symlinkSync(w.root, join(w.repo, ".sandcastle/worktrees"));
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /符号链接/);
  w.noSideEffects("预绑定路径符号链接");
  rmSync(join(w.repo, ".sandcastle/worktrees"));
  w.git("branch", `sandcastle/issue-${TICKET}`, w.head);
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /已存在，不是本轮新候选/);
  assert.deepEqual(w.claims(), []); assert.deepEqual(w.creates, []);
});

test("row13 一次性：消费后 provider 初始化失败且容器写入了工位 → used-13 保留、工位被 worktree lock 保留未删；同 nonce 不能重跑", async (t) => {
  const w = world(t); const r = await w.issue();
  w.onCreate.push((o) => { writeFileSync(join(o.worktreePath, "sentinel.txt"), "容器启动中写入"); throw new Error("容器初始化失败"); });
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /容器初始化失败/);
  assert.equal(w.claims().filter((f) => f.startsWith("used-")).length, 1, "消费后启动失败不撤回 used-13");
  const cwd = join(w.repo, ".sandcastle/worktrees", `sandcastle-issue-${TICKET}`);
  assert.equal(readFileSync(join(cwd, "sentinel.txt"), "utf8"), "容器启动中写入", "Sandcastle 的 remove --force 未清掉带未知写入的现场");
  assert.match(w.git("worktree", "list", "--porcelain"), /locked "?sandcastle-1867-preserved: provider init failed/);
  w.onCreate.length = 0;
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /一次性的，已被占用/);
});

test("并发：两个进程内调用竞争同一 row13，只一次消费、一次启动", async (t) => {
  const w = world(t); const r = await w.issue();
  const results = await Promise.allSettled([runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps())]);
  assert.equal(results.filter((x) => x.status === "fulfilled").length, 1);
  const rejected = results.find((x) => x.status === "rejected") as PromiseRejectedResult;
  assert.ok(rejected.reason instanceof LockBusyError || /一次性|已有本轮候选/.test(String(rejected.reason)), String(rejected.reason));
  assert.equal(w.claims().filter((f) => f.startsWith("used-")).length, 1);
  assert.equal(w.creates.length, 1);
});

test("resume 竞争：专用准入持锁时 resume 等到消费并取得容器 handle 后才返回；handle 返回后全局锁已释放，agent 不占锁", async (t) => {
  const w = world(t); const r = await w.issue();
  const b = barrier();
  let statusWhileAgent = "";
  const running = runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps({
    barrier: (p) => (p === "row13-before-consume" ? b.wait() : undefined),
    runAgent: async (sb, role) => {
      if (role === "implementer") {
        statusWhileAgent = drain(w.drainRoot, "status");
        writeFileSync(join(sb.worktreePath, PROBE.split("/").pop()!), "x\n");
        return { stdout: report([PROBE.split("/").pop()!]), iterations: [] };
      }
      return { stdout: report([]), iterations: [] };
    },
  }));
  await b.reached;
  const resume = spawnTracked("python3", [HELPER, "resume", "--root", w.drainRoot, "--generation", "1"]);
  await delay(400);
  assert.equal(resume.exited(), false, "resume 必须等专用准入完成");
  assert.deepEqual(w.claims(), []);
  b.open();
  const out = await running;
  assert.equal(await resume.done, 0);
  assert.equal(out.state, "candidate_pending", "先拿到共享锁的准入视为已准入，可完成");
  assert.match(statusWhileAgent, /"mode"/, "agent 运行期间 status（排他锁）可返回：全局锁已在容器 handle 返回后释放");
  assert.equal(w.creates.length, 1);
});

for (const window of ["row13-before-consume", "inside-provider-create"] as const) {
  test(`retire 竞争窗口（${window}）：控制锁使退役等到“消费→容器 handle 返回”完成；之后的新准入读不到该 receipt`, async (t) => {
    const w = world(t); const r = await w.issue();
    const b = barrier();
    if (window === "inside-provider-create") w.onCreate.push(() => b.wait());
    const running = runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps({ barrier: (p) => (p === window ? b.wait() : undefined) }));
    await b.reached;
    const retire = spawnTracked(process.execPath, ["--import", "tsx", ACCEPTANCE_CLI, "retire", "--dir", w.dir]);
    await delay(500);
    assert.equal(retire.exited(), false, "退役不能插进最后重核与启动之间");
    assert.ok(existsSync(join(w.dir, "receipt.json")));
    b.open();
    const out = await running;
    assert.equal(out.state, "candidate_pending", "已启动的工蜂保留完成权");
    assert.equal(await retire.done, 0, retire.output());
    assert.equal(existsSync(join(w.dir, "receipt.json")), false);
    assert.equal(w.claims().filter((f) => f.startsWith("used-")).length, 1);
    await assert.rejects(w.bind(r.nonce, out.head), /receipt 缺失/, "退役后不能再封定或准入");
  });
}

test("退役在先：随后的准入在锁内重核读不到 receipt，零消费零启动", async (t) => {
  const w = world(t); const r = await w.issue();
  const retire = spawnTracked(process.execPath, ["--import", "tsx", ACCEPTANCE_CLI, "retire", "--dir", w.dir]);
  assert.equal(await retire.done, 0, retire.output());
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps()), /receipt 缺失/);
  w.noSideEffects("已退役");
});

test("bind-merges 来源负控：无候选、独审 head 不符或同家族、主控准入错 nonce、输入改变、候选与伙伴不冲突都拒绝", async (t) => {
  const w = world(t); const r = await w.issue();
  await assert.rejects(w.bind(r.nonce, "a".repeat(40)), /尚无已发布候选/);
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  await assert.rejects(w.bind(r.nonce, c.head, { candidateReview: w.reviewFile(w.head) }), /不是被验收的候选/);
  await assert.rejects(w.bind(r.nonce, c.head, { candidateReview: w.reviewFile(c.head, "gpt-6.1-sol") }), /同属 openai 家族/);
  await assert.rejects(w.bind(r.nonce, c.head, { controllerAdmission: w.admissionFile(c.head, "00000000-0000-4000-8000-000000000000") }), /NONCE 不是本轮/);
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), /尚未封定/);
  const plan = await w.bind(r.nonce, c.head);
  const otherAdmission = join(w.root, "admit-other.md"); writeFileSync(otherAdmission, `VERDICT: ADMIT\nHEAD: ${c.head}\nNONCE: ${r.nonce}\n`);
  await assert.rejects(w.bind(r.nonce, c.head, { controllerAdmission: otherAdmission }), /已封定且与本次输入不同/);
  assert.deepEqual(loadMergePlan(inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx)), plan);

  const v = world(t, { candidatePath: "docs/agents/other.md" }); const rv = await v.issue();
  const cv = await runDeploymentAcceptance(v.sel(rv.nonce, "13"), v.deps());
  await assert.rejects(v.bind(rv.nonce, cv.head), /不冲突/);
});

test("归并恢复边界：准入事实已写而工位未建可续建；无准入事实却有他人分支拒绝；已消费后删检查点不能重建；done 未发布只补发布不再启动", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  w.git("branch", plan.runs["15"].manifest.branch, w.head);
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps()), /没有 run 准入事实却已有/);
  w.git("branch", "-D", plan.runs["15"].manifest.branch);
  publishMergeAdmission(round, plan.runs["14"]);
  const creates = w.creates.length;
  const done = await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps());
  assert.equal(done.state, "merge_done");
  assert.equal(w.creates.length, creates + 1);
  const resultPath = join(round.roundDir, ROUND_FILES.result("14"));
  renameSync(resultPath, `${resultPath}.lost`);
  const claims = w.claims();
  const again = await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps());
  assert.equal(again.head, done.head);
  assert.equal(w.creates.length, creates + 1, "补发布不启动工蜂");
  assert.deepEqual(w.claims(), claims, "补发布不新写行消费记录");
  rmSync(resultPath);
  rmSync(plan.runs["14"].checkpoint);
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), /原检查点缺失：不能从基线重建/);
});

test("row15 真实中断：冲突工蜂写 WIP 时 controller 被 SIGKILL，同命令恢复原 run：锁接管、孤儿 WIP 归档还原、resume 留痕、每输入唯一 trailer", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const marker = join(w.root, "child-at-conflict");
  const child = spawnTracked(process.execPath, ["--import", "tsx", CHILD, JSON.stringify({ sel: w.sel(r.nonce, "15"), dir: w.dir, repo: w.repo, head: w.head, imageId: IMG_ID, drainRoot: w.drainRoot, marker, probe: PROBE })]);
  for (let i = 0; i < 300 && !existsSync(marker); i++) await delay(100);
  assert.ok(existsSync(marker), child.output());
  const claimsWhileLive = w.claims();
  const startedAt = Date.now();
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps()), LockBusyError, "活持有者在时立即拒绝");
  assert.ok(Date.now() - startedAt < 5000);
  assert.deepEqual(w.claims(), claimsWhileLive); assert.equal(w.creates.length, 1);
  child.child.kill("SIGKILL"); await child.done;
  const run = plan.runs["15"];
  await untilFree(join(inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx).roundDir, ROUND_FILES.runLock("15")));
  assert.equal(JSON.parse(readFileSync(run.checkpoint, "utf8")).phase, "conflict");
  assert.ok(existsSync(`${run.checkpoint}.lock`), "SIGKILL 留下持有者已死的锁");
  assert.equal(w.claims().filter((f) => f.startsWith("used-") && f.endsWith("-15.json")).length, 1);
  const admission = readFileSync(plan.candidate.controllerAdmission, "utf8");
  writeFileSync(plan.candidate.controllerAdmission, admission.replace(r.nonce, "00000000-0000-4000-8000-000000000000"));
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps()), /候选来源证据当前无效.*NONCE/, "恢复时新冲突工蜂须重核当前主控准入");
  assert.equal(w.claims().filter((f) => f.startsWith("resume-")).length, 0); assert.equal(w.creates.length, 1);
  writeFileSync(plan.candidate.controllerAdmission, admission);
  const out = await runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps());
  assert.equal(out.state, "merge_done");
  assert.equal(w.claims().filter((f) => f.startsWith("resume-") && f.includes("-15-")).length, 1, "同一检查点恢复留痕");
  assert.equal(w.claims().filter((f) => f.startsWith("cp-")).length, 0, "没有新冲突绑定冒充新 run");
  assert.ok(readdirSync(run.evidenceDir).some((f) => f.startsWith("orphan-wip-")), "孤儿 WIP 已归档");
  assert.ok(readdirSync(dirname(run.checkpoint)).some((f) => f.startsWith("checkpoint.json.lock.stale-")), "死锁已归档后接管");
  const log = w.git("log", "--format=%B", out.head);
  for (const sha of [c.head, w.partnerSha]) assert.equal(log.split("\n").filter((l) => l === `Sandcastle-Merge: ${run.manifest.runId}/${sha}`).length, 1);
});

test("Sandcastle 依赖行为：库本身会删除未登记孤儿目录、会 force remove 初始化失败工位；共用初始化路径拒绝孤儿并锁定带写入的失败现场", async (t) => {
  const w = world(t);
  const orphan = join(w.repo, ".sandcastle/worktrees/orphan"); mkdirSync(orphan, { recursive: true }); writeFileSync(join(orphan, "WIP"), "x");
  const failing = (write: boolean) => createBindMountSandboxProvider({ name: "fixture", create: async (o) => { if (write) writeFileSync(join(o.worktreePath, "sentinel"), "y"); throw new Error("init failed"); } }) as unknown as SandboxProvider;
  await assert.rejects(createSwarmSandbox({ repo: w.repo, branch: "sandcastle/probe-a", baseBranch: w.head, provider: failing(false) }), /未登记目录/);
  assert.ok(existsSync(join(orphan, "WIP")), "共用路径保留孤儿");
  await assert.rejects(sandcastle.createSandbox({ cwd: w.repo, branch: "sandcastle/probe-raw", baseBranch: w.head, sandbox: failing(true) }), /init failed/);
  assert.equal(existsSync(orphan), false, "负控：库的 pruneStale 删掉了未登记目录");
  assert.equal(existsSync(join(w.repo, ".sandcastle/worktrees/sandcastle-probe-raw")), false, "负控：库 force remove 了带写入的失败工位");
  await assert.rejects(sandcastle.createSandbox({ cwd: w.repo, branch: "sandcastle/probe-kept", baseBranch: w.head, sandbox: preserveFailedInit(failing(true)) }), /init failed/);
  assert.equal(readFileSync(join(w.repo, ".sandcastle/worktrees/sandcastle-probe-kept/sentinel"), "utf8"), "y", "worktree lock 后单个 --force 不再移除");
  await assert.rejects(sandcastle.createSandbox({ cwd: w.repo, branch: "sandcastle/probe-clean", baseBranch: w.head, sandbox: preserveFailedInit(failing(false)) }), /init failed/);
  assert.equal(existsSync(join(w.repo, ".sandcastle/worktrees/sandcastle-probe-clean")), false, "干净的新工位照常清理");
});

test("rows14 活持锁重入：首个停在消费前时第二个立即 LockBusyError，无消费无 provider；首个照常完成", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  await w.bind(r.nonce, c.head);
  const b = barrier();
  const first = runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps({ barrier: (p) => (p === "merge-before-consume" ? b.wait() : undefined) }));
  await b.reached;
  const claims = w.claims(); const creates = w.creates.length;
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), LockBusyError);
  assert.deepEqual(w.claims(), claims); assert.equal(w.creates.length, creates);
  b.open();
  assert.equal((await first).state, "merge_done");
});

test("来源证据当前有效性：候选独审撤回则新归并拒绝且零副作用，恢复后同一封定计划可执行；登记与 acceptedFor 随证据失效", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const review = readFileSync(plan.candidate.review.file, "utf8");
  writeFileSync(plan.candidate.review.file, review.replace("VERDICT: PASS", "VERDICT: FAIL"));
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), /候选来源证据当前无效/);
  assert.equal(w.creates.length, 1); assert.deepEqual(w.claims().filter((f) => f.endsWith("-14.json")), []);
  assert.equal(existsSync(plan.runs["14"].cwd), false, "未建归并工位");
  writeFileSync(plan.candidate.review.file, review);
  await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps());
  await runDeploymentAcceptance(w.sel(r.nonce, "15"), w.deps());
  assert.deepEqual(loadMergePlan(inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx)), plan, "封定计划未被改写");
  const evidence = Object.fromEntries((["13", "14", "15"] as const).map((row) => { const p = join(w.root, `ev-${row}`); writeFileSync(p, row); return [row, p]; })) as Record<AcceptanceRow, string>;
  const accept = () => recordAcceptance({ ...w.ctx, image: IMAGE, evidence, crossFamilyReview: w.reviewFile(w.head) });
  renameSync(plan.candidate.controllerAdmission, `${plan.candidate.controllerAdmission}.gone`);
  assert.throws(accept, /候选来源证据当前无效.*缺少主控准入记录/);
  renameSync(`${plan.candidate.controllerAdmission}.gone`, plan.candidate.controllerAdmission);
  accept();
  assert.ok(acceptedFor(IMAGE, w.ctx));
  writeFileSync(plan.candidate.review.file, review.replace(c.head, w.head));
  assert.equal(acceptedFor(IMAGE, w.ctx), null, "独审 HEAD 漂移后 accepted 失效");
  writeFileSync(plan.candidate.review.file, review);
  assert.ok(acceptedFor(IMAGE, w.ctx), "证据恢复有效后重新成立");
});

test("main.mts 两处建工位都走共用初始化锁；none provider 初始化失败保留带写入现场，isolated 原样；既有分支（REVIEW_ONLY）可复用", async (t) => {
  const src = readFileSync(new URL("./main.mts", import.meta.url), "utf8");
  assert.doesNotMatch(src, /sandcastle\.createSandbox\(/);
  assert.equal(src.match(/createSwarmSandbox\(\{\n\s+repo: REPO,/g)?.length, 2);
  assert.match(src, /withLegacyAdmission\(\(\) => createSwarmSandbox\(\{/, "REVIEW_ONLY 仍在 drain 准入内建工位");
  assert.match(src, /return await createSwarmSandbox\(\{[\s\S]{0,200}\} catch \(e\) \{\n\s+if \(attempt < WORKTREE_TIMEOUT_MAX_ATTEMPTS && isWorktreeTimeoutError\(e\)\)/, "超时重试仍包在外层");
  const w = world(t);
  const none = (write: boolean) => ({ tag: "none", name: "fixture", env: {}, create: async (o: { worktreePath: string }) => { if (write) writeFileSync(join(o.worktreePath, "sentinel"), "y"); throw new Error("init failed"); } }) as unknown as SandboxProvider;
  await assert.rejects(createSwarmSandbox({ repo: w.repo, branch: "sandcastle/issue-77", baseBranch: "main", provider: none(true) }), /init failed/);
  assert.equal(readFileSync(join(w.repo, ".sandcastle/worktrees/sandcastle-issue-77/sentinel"), "utf8"), "y");
  await assert.rejects(createSwarmSandbox({ repo: w.repo, branch: "sandcastle/issue-78", baseBranch: "main", provider: none(false) }), /init failed/);
  assert.equal(existsSync(join(w.repo, ".sandcastle/worktrees/sandcastle-issue-78")), false);
  const isolated = { tag: "isolated", name: "fixture", env: {}, create: async () => { throw new Error("x"); } } as unknown as SandboxProvider;
  assert.equal(preserveFailedInit(isolated), isolated);
  w.git("branch", "sandcastle/issue-79", w.head);
  const sb = await createSwarmSandbox({ repo: w.repo, branch: "sandcastle/issue-79", baseBranch: "main", provider: noSandbox() });
  assert.equal(execFileSync("git", ["-C", sb.worktreePath, "rev-parse", "HEAD"], { encoding: "utf8" }).trim(), w.head, "既有分支复用，不按 baseBranch 重建");
  await sb.close();
  const orphan = join(w.repo, ".sandcastle/worktrees/other"); mkdirSync(orphan); writeFileSync(join(orphan, "WIP"), "z");
  await assert.rejects(createSwarmSandbox({ repo: w.repo, branch: "sandcastle/issue-80", baseBranch: "main", provider: noSandbox() }), /未登记目录/);
  assert.equal(readFileSync(join(orphan, "WIP"), "utf8"), "z");
});

test("来源最后重核在阻塞检查之后：冲突工蜂最后一次镜像解析期间撤回独审 → 不消费、不启动", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const review = readFileSync(plan.candidate.review.file, "utf8");
  let armed = false;
  w.ctx.imageIdOf = () => { if (armed) writeFileSync(plan.candidate.review.file, review.replace("VERDICT: PASS", "VERDICT: FAIL")); return IMG_ID; };
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps({ barrier: (p) => { if (p === "merge-before-consume") armed = true; } })), /候选来源证据当前无效/);
  assert.equal(w.creates.length, 1);
  assert.deepEqual(w.claims().filter((f) => f.endsWith("-14.json") || f.includes("-14-")), []);
});

test("首次 run 准入：模型预检期间撤回独审 → 不发布准入、不建工位、不消费", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const review = readFileSync(plan.candidate.review.file, "utf8");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps({
    modelProbe: () => { writeFileSync(plan.candidate.review.file, review.replace("VERDICT: PASS", "VERDICT: FAIL")); },
  })), /候选来源证据当前无效/);
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  assert.equal(existsSync(join(round.roundDir, ROUND_FILES.admission("14"))), false);
  assert.equal(existsSync(plan.runs["14"].cwd), false);
  assert.equal(w.branchOf(plan.runs["14"].manifest.branch), "");
  assert.deepEqual(w.claims().filter((f) => f.includes("-14")), []);
  assert.equal(w.creates.length, 1);
});

test("首次 run 准入：等待仓库初始化锁期间撤回主控准入 → 不发布准入、不建工位、不消费；恢复后同命令完成", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const admission = readFileSync(plan.candidate.controllerAdmission, "utf8");
  let release!: () => void; const held = new Promise<void>((res) => { release = res; });
  let locked!: () => void; const holding = new Promise<void>((res) => { locked = res; });
  const holder = withFileLock(repoInitLock(w.repo), async () => { locked(); await held; });
  await holding;
  let probed = false;
  const running = runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps({ modelProbe: () => { probed = true; } }));
  for (let i = 0; i < 100 && !probed; i++) await delay(50);
  assert.equal(probed, true);
  await delay(300);
  writeFileSync(plan.candidate.controllerAdmission, admission.replace(r.nonce, "00000000-0000-4000-8000-000000000000"));
  release(); await holder;
  await assert.rejects(running, /候选来源证据当前无效.*NONCE/);
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  assert.equal(existsSync(join(round.roundDir, ROUND_FILES.admission("14"))), false);
  assert.equal(existsSync(plan.runs["14"].cwd), false);
  assert.deepEqual(w.claims().filter((f) => f.includes("-14")), []);
  writeFileSync(plan.candidate.controllerAdmission, admission);
  assert.equal((await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps())).state, "merge_done");
});

/** row14 真实冲突解决后停在 prepared 检查点（宿主尚未提交），进程正常退出；之后的恢复只有宿主提交，不再建 provider。 */
async function stopRow14AtPrepared(t: { after: (fn: () => void) => void }) {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  const plan = await w.bind(r.nonce, c.head);
  const d = w.deps();
  await assert.rejects(runDeploymentSwarmMerge({ ...w.sel(r.nonce, "14"), row: "14" }, {
    ctx: d.ctx, drainRoot: d.drainRoot, probe: d.probe, modelProbe: d.modelProbe, provider: d.mergeProvider, runConflictAgent: d.runConflictAgent, docker: d.docker,
    logWorker: (e) => { if (e.status === "prepared" && e.position === 1) throw new Error("stop-at-prepared"); },
  }), /stop-at-prepared/);
  const run = plan.runs["14"];
  assert.equal(JSON.parse(readFileSync(run.checkpoint, "utf8")).phase, "prepared");
  return { w, r, run, head: w.git("rev-parse", run.manifest.branch), creates: w.creates.length };
}

test("专用 run 恢复（prepared，无新 provider）：同代次 draining 正常完成宿主提交", async (t) => {
  const { w, r, run, head, creates } = await stopRow14AtPrepared(t);
  assert.equal((await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps())).state, "merge_done");
  assert.notEqual(w.git("rev-parse", run.manifest.branch), head);
  assert.equal(w.creates.length, creates, "恢复不建新 provider");
});

test("专用 run 恢复：旧入口已 resume 或进入新冻结代次时 DrainFrozenError，在宿主提交与补发布结果之前拒绝", async (t) => {
  const { w, r, run, head, creates } = await stopRow14AtPrepared(t);
  drain(w.drainRoot, "resume", "--generation", "1");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), DrainFrozenError);
  assert.equal(w.git("rev-parse", run.manifest.branch), head, "open：无宿主提交");
  assert.equal(JSON.parse(readFileSync(run.checkpoint, "utf8")).phase, "prepared");
  drain(w.drainRoot, "freeze");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), DrainFrozenError);
  assert.equal(w.git("rev-parse", run.manifest.branch), head, "新代次：无宿主提交");
  assert.equal(w.creates.length, creates);
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  assert.equal(existsSync(join(round.roundDir, ROUND_FILES.result("14"))), false, "不补发布完成结果");
});

test("专用 run 恢复：done 未发布结果时，代次已变则不补发布", async (t) => {
  const w = world(t); const r = await w.issue();
  const c = await runDeploymentAcceptance(w.sel(r.nonce, "13"), w.deps());
  await w.bind(r.nonce, c.head);
  await runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps());
  const round = inspectFrozenRound({ ticket: TICKET, head: w.head, nonce: r.nonce }, w.ctx);
  const resultPath = join(round.roundDir, ROUND_FILES.result("14"));
  renameSync(resultPath, `${resultPath}.lost`);
  drain(w.drainRoot, "resume", "--generation", "1");
  drain(w.drainRoot, "freeze");
  await assert.rejects(runDeploymentAcceptance(w.sel(r.nonce, "14"), w.deps()), DrainFrozenError);
  assert.equal(existsSync(resultPath), false);
});
