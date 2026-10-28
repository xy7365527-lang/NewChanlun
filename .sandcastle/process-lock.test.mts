import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { hostname, tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { acquireLock, psStart, type LockEnv } from "./process-lock.mts";
import { runManagedMerge, type MergeCheck, type MergeManifest, type MergeOptions } from "./managed-merge.mts";

const loader = import.meta.resolve("tsx");
const here = fileURLToPath(new URL(".", import.meta.url));
const dir = () => mkdtempSync(join(tmpdir(), "plock-"));
const env = (o: Partial<LockEnv> = {}): LockEnv => ({ pid: process.pid, host: hostname(), startOf: psStart, now: () => new Date(), ...o });
const write = (lock: string, o: Record<string, unknown>) => writeFileSync(lock, JSON.stringify(o));
const stale = (d: string) => readdirSync(d).filter((f) => f.includes(".stale-"));

function deadPid(): number {
  const child = spawn(process.execPath, ["-e", "0"]); const pid = child.pid!;
  child.kill("SIGKILL");
  return pid;
}
async function waitDead(pid: number) { for (let i = 0; i < 100 && psStart(pid); i++) await new Promise((r) => setTimeout(r, 30)); }

test("持有者已死（pid 不存在）：原子归档旧锁并接管，旧锁留作证据", async () => {
  const d = dir(); const lock = join(d, "c.json.lock");
  const pid = deadPid(); await waitDead(pid);
  write(lock, { pid, start: "Mon Jan  1 00:00:00 2024", host: hostname(), token: "old", cwd: "/x" });
  const held = acquireLock(lock, "/y", env());
  assert.equal(stale(d).length, 1); assert.equal(held.archived, join(d, stale(d)[0]!));
  assert.equal(JSON.parse(readFileSync(held.archived!, "utf8")).token, "old");
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, held.owner.token);
  held.release(); assert.equal(existsSync(lock), false);
});

test("pid 被复用（pid 存在但启动时间不同）：接管", () => {
  const d = dir(); const lock = join(d, "c.lock");
  write(lock, { pid: process.pid, start: "Mon Jan  1 00:00:00 2024", host: hostname(), token: "old", cwd: "/x" });
  const held = acquireLock(lock, "/y", env());
  assert.equal(stale(d).length, 1); held.release();
});

test("持有者存活且身份一致 / 锁损坏 / 旧格式 / 主机不同 / 启动时间无法判定：一律拒绝且锁文件不变", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const mine = psStart(process.pid)!;
  const cases: Array<[string, () => void, RegExp, LockEnv?]> = [
    ["存活", () => write(lock, { pid: process.pid, start: mine, host: hostname(), token: "t", cwd: "/x" }), /仍在运行/],
    ["损坏", () => writeFileSync(lock, '{"pid":12'), /无法解析/],
    ["旧格式", () => write(lock, { pid: 84311, cwd: "/x" }), /缺少持有者身份/],
    ["主机不同", () => write(lock, { pid: 1, start: "x", host: "other-host", token: "t", cwd: "/x" }), /其他主机/],
    ["无法判定", () => write(lock, { pid: 77, start: "x", host: hostname(), token: "t", cwd: "/x" }), /boom/, env({ startOf: (pid) => { if (pid === 77) throw new Error("boom"); return psStart(pid); } })],
  ];
  for (const [name, setup, re, e] of cases) {
    setup(); const before = readFileSync(lock, "utf8");
    assert.throws(() => acquireLock(lock, "/y", e ?? env()), re, name);
    assert.equal(readFileSync(lock, "utf8"), before, `${name}：现场不动`); assert.deepEqual(stale(d), [], name);
  }
});

test("release 只删自己的锁：已被他人接管则不误删", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const held = acquireLock(lock, "/y", env());
  write(lock, { pid: 1, start: "x", host: hostname(), token: "someone-else", cwd: "/z" });
  held.release(); assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, "someone-else");
});

test("真实子进程拿锁后被 SIGKILL：同一把锁自动接管", async () => {
  const d = dir(); const lock = join(d, "c.lock");
  const script = join(d, "hold.mts");
  writeFileSync(script, `import { acquireLock } from ${JSON.stringify(here + "process-lock.mts")}; acquireLock(${JSON.stringify(lock)}, "/c"); console.log("LOCKED"); setInterval(() => {}, 1000);`);
  const child = spawn(process.execPath, ["--import", loader, script], { stdio: ["ignore", "pipe", "inherit"] });
  await new Promise<void>((res, rej) => { child.stdout.on("data", (b) => String(b).includes("LOCKED") && res()); child.on("exit", () => rej(new Error("子进程提前退出"))); });
  assert.throws(() => acquireLock(lock, "/y", env()), /仍在运行/, "持有者存活时拒绝");
  child.kill("SIGKILL"); await new Promise((r) => child.on("exit", r)); await waitDead(child.pid!);
  assert.ok(existsSync(lock), "SIGKILL 残留锁");
  const held = acquireLock(lock, "/y", env()); assert.equal(stale(d).length, 1); held.release();
});

test("两个并发重入争同一把残留锁：只有一个成功，另一个拒绝；不误搬对方的新锁", async () => {
  const d = dir(); const lock = join(d, "c.lock");
  const pid = deadPid(); await waitDead(pid);
  const script = join(d, "contend.mts");
  writeFileSync(script, `import { acquireLock } from ${JSON.stringify(here + "process-lock.mts")};
try { const h = acquireLock(${JSON.stringify(lock)}, "/c"); console.log("WON"); setTimeout(() => { h.release(); }, 1500); } catch (e) { console.log("LOST " + (e instanceof Error ? e.message.slice(0, 80) : e)); }`);
  for (let round = 0; round < 3; round++) {
    write(lock, { pid, start: "Mon Jan  1 00:00:00 2024", host: hostname(), token: "old" + round, cwd: "/x" });
    const run = () => new Promise<string>((res) => { let out = ""; const c = spawn(process.execPath, ["--import", loader, script], { stdio: ["ignore", "pipe", "inherit"] }); c.stdout.on("data", (b) => (out += b)); c.on("exit", () => res(out)); });
    const results = await Promise.all([run(), run(), run()]);
    assert.equal(results.filter((r) => r.startsWith("WON")).length, 1, JSON.stringify(results));
    assert.equal(results.filter((r) => r.startsWith("LOST")).length, 2, JSON.stringify(results));
    assert.ok(!existsSync(`${lock}.takeover`), "守卫已释放");
    assert.equal(existsSync(lock), false, "赢家退出时释放自己的锁");
  }
});

// —— 通过 runManagedMerge 的端到端：宿主归并（无冲突）与冲突（Docker 门禁所在回调）两条路径 ——
const report = (files: string[]) => `<sandcastle-result>${JSON.stringify({ status: "ready", files, verification: [] })}</sandcastle-result>`;
function fixture(conflict: boolean) {
  const cwd = dir(); const evidence = dir();
  const git = (...a: string[]) => execFileSync("git", a, { cwd, encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-b", "main"); git("config", "user.name", "T"); git("config", "user.email", "t@x");
  writeFileSync(join(cwd, "a.txt"), "base\n"); writeFileSync(join(cwd, "b.txt"), "base\n"); git("add", "."); git("commit", "-m", "base");
  const base = git("rev-parse", "HEAD"); const shas: string[] = [];
  for (const n of [1, 2]) { git("checkout", "-b", `sandcastle/issue-${n}`, base); writeFileSync(join(cwd, n === 1 || conflict ? "a.txt" : "b.txt"), `i${n}\n`); git("add", "."); git("commit", "-m", `i${n}`); shas.push(git("rev-parse", "HEAD")); }
  const manifest: MergeManifest = { runId: "lock", branch: "sandcastle/merge-lock", base, inputs: shas.map((sha, i) => ({ issue: i + 1, branch: `sandcastle/issue-${i + 1}`, sha, acceptance: { head: sha, status: "accepted", reviewEvidence: "r", verificationEvidence: "v" } })) };
  git("checkout", "-b", manifest.branch, base);
  return { cwd, evidence, git, base, shas, manifest, checkpoint: join(evidence, "checkpoint.json") };
}
function childRun(f: ReturnType<typeof fixture>, killAt: "conflict" | "verify") {
  const script = join(f.evidence, "run.mts");
  writeFileSync(script, `import { runManagedMerge } from ${JSON.stringify(here + "managed-merge.mts")};
const f = ${JSON.stringify({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest })};
await runManagedMerge({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
  verify: async (head) => { ${killAt === "verify" ? 'process.kill(process.pid, "SIGKILL"); await new Promise(() => {});' : ""} return { head, checkId: "v1", status: "passed", evidence: "ok" }; },
  resolveConflict: async () => { ${killAt === "conflict" ? 'process.kill(process.pid, "SIGKILL"); await new Promise(() => {});' : "throw new Error('x')"} } });`);
  return spawnSync(process.execPath, ["--import", loader, script], { encoding: "utf8" });
}
for (const [name, conflict, killAt] of [["宿主归并路径（无冲突）", false, "verify"], ["冲突工蜂路径（Docker 门禁回调内被 SIGKILL）", true, "conflict"]] as const) {
  test(`${name}：SIGKILL 残留锁后同清单自动恢复收敛到 done，不重复提交`, async () => {
    const f = fixture(conflict);
    const r = childRun(f, killAt); assert.equal(r.signal, "SIGKILL");
    assert.ok(existsSync(`${f.checkpoint}.lock`), "残留锁"); const holderPid = JSON.parse(readFileSync(`${f.checkpoint}.lock`, "utf8")).pid;
    await waitDead(holderPid);
    const verified: string[] = []; let conflictCalls = 0;
    const opts: MergeOptions = { cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
      verify: async (head): Promise<MergeCheck> => { verified.push(head); return { head, checkId: "v1", status: "passed", evidence: "ok" }; },
      resolveConflict: async () => { conflictCalls++; writeFileSync(join(f.cwd, "a.txt"), "both\n"); return report(["a.txt"]); } };
    const done = await runManagedMerge(opts);
    assert.equal(done.phase, "done"); assert.equal(done.entries.length, 2);
    assert.equal(stale(f.evidence).length, 1, "旧锁已归档"); assert.equal(existsSync(`${f.checkpoint}.lock`), false);
    const parents = f.git("rev-list", "--parents", "--merges", `${f.base}..HEAD`).split("\n").filter(Boolean);
    assert.equal(parents.length, 2, "两个归并提交，无重复");
    if (conflict) assert.equal(conflictCalls, 1);
    const again = await runManagedMerge(opts); assert.equal(again.phase, "done");
    const headAfter = f.git("rev-parse", "HEAD"); await runManagedMerge(opts); assert.equal(f.git("rev-parse", "HEAD"), headAfter, "重入不新增提交");
    assert.equal(f.git("rev-list", "--parents", "--merges", `${f.base}..HEAD`).split("\n").filter(Boolean).length, 2);
  });
}

test("runManagedMerge：持有者存活 / 锁损坏 / 主机不同时拒绝，检查点与现场不变", async () => {
  const f = fixture(false); const lock = `${f.checkpoint}.lock`;
  const opts: MergeOptions = { cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
    verify: async (head) => ({ head, checkId: "v1", status: "passed", evidence: "ok" }), resolveConflict: async () => { throw new Error("x"); } };
  const head = f.git("rev-parse", "HEAD");
  for (const [setup, re] of [
    [() => write(lock, { pid: process.pid, start: psStart(process.pid), host: hostname(), token: "t", cwd: f.cwd }), /仍在运行/],
    [() => writeFileSync(lock, "garbage"), /无法解析/],
    [() => write(lock, { pid: 1, start: "x", host: "other", token: "t", cwd: f.cwd }), /其他主机/],
  ] as const) {
    setup(); const before = readFileSync(lock, "utf8");
    await assert.rejects(runManagedMerge(opts), re);
    assert.equal(readFileSync(lock, "utf8"), before); assert.equal(f.git("rev-parse", "HEAD"), head); assert.equal(existsSync(f.checkpoint), false);
    rmSync(lock);
  }
});
