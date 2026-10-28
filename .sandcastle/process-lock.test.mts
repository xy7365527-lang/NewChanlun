import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
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
function childRun(f: ReturnType<typeof fixture>, killAt: "conflict" | "verify", beforeKill = "") {
  const script = join(f.evidence, "run.mts");
  writeFileSync(script, `import fs from "node:fs"; import * as cp from "node:child_process"; import { runManagedMerge } from ${JSON.stringify(here + "managed-merge.mts")};
const f = ${JSON.stringify({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest })};
await runManagedMerge({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
  verify: async (head) => { ${killAt === "verify" ? 'process.kill(process.pid, "SIGKILL"); await new Promise(() => {});' : ""} return { head, checkId: "v1", status: "passed", evidence: "ok" }; },
  resolveConflict: async () => { ${killAt === "conflict" ? beforeKill + 'process.kill(process.pid, "SIGKILL"); await new Promise(() => {});' : "throw new Error('x')"} } });`);
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

// —— TZ 回归、psStart 退出码映射、确定性交错 ——
import { psStart as ps2, type PsResult } from "./process-lock.mts";

test("启动时间与 TZ 无关：同一 pid 在不同 TZ 下取到同一字符串；持有者在一个 TZ 下存活，取锁方在另一个 TZ 下必须拒绝", async () => {
  const savedTz = process.env.TZ;
  try {
    const seen = new Set<string>();
    for (const tz of ["UTC", "Asia/Shanghai", "America/Los_Angeles", "Pacific/Auckland"]) { process.env.TZ = tz; seen.add(psStart(process.pid)!); }
    assert.equal(seen.size, 1, JSON.stringify([...seen]));
  } finally { if (savedTz === undefined) delete process.env.TZ; else process.env.TZ = savedTz; }
  const d = dir(); const lock = join(d, "c.lock"); const script = join(d, "hold.mts");
  writeFileSync(script, `import { acquireLock } from ${JSON.stringify(here + "process-lock.mts")}; acquireLock(${JSON.stringify(lock)}, "/c"); console.log("LOCKED"); setInterval(() => {}, 1000);`);
  const child = spawn(process.execPath, ["--import", loader, script], { stdio: ["ignore", "pipe", "inherit"], env: { ...process.env, TZ: "America/New_York" } });
  await new Promise<void>((res, rej) => { child.stdout.on("data", (b) => String(b).includes("LOCKED") && res()); child.on("exit", () => rej(new Error("子进程提前退出"))); });
  try {
    const saved = process.env.TZ;
    for (const tz of ["Asia/Shanghai", "UTC", undefined]) {
      if (tz === undefined) delete process.env.TZ; else process.env.TZ = tz;
      assert.throws(() => acquireLock(lock, "/y", env()), /仍在运行/, `取锁方 TZ=${tz}`);
    }
    if (saved === undefined) delete process.env.TZ; else process.env.TZ = saved;
    assert.deepEqual(stale(d), [], "活锁没有被接管");
  } finally { child.kill("SIGKILL"); }
});

test("psStart 退出码映射：只有 status=1 且 stdout/stderr 皆空表示已死；其余一律无法判定并抛错", () => {
  const r = (x: Partial<PsResult>): (() => PsResult) => () => ({ status: 0, stdout: "", stderr: "", ...x });
  assert.equal(ps2(5, r({ status: 1 })), null);
  assert.equal(ps2(5, r({ status: 0, stdout: " Mon Oct 27 09:29:54 2026\n" })), "Mon Oct 27 09:29:54 2026");
  for (const [name, x] of [
    ["1 但 stderr 有内容", { status: 1, stderr: "ps: Operation not permitted" }], ["1 但 stdout 有内容", { status: 1, stdout: "x" }],
    ["2", { status: 2 }], ["0 却无输出", { status: 0 }], ["0 但 stderr 有内容", { status: 0, stdout: "x", stderr: "warn" }],
    ["被信号杀", { status: null }], ["ps 不存在", { status: null, error: new Error("ENOENT") }],
  ] as const) assert.throws(() => ps2(5, r(x)), /ps/, name);
  // 无法判定 → acquireLock 拒绝并保持现场
  const d = dir(); const lock = join(d, "c.lock");
  write(lock, { pid: 77, start: "x", host: hostname(), token: "t", cwd: "/x" });
  assert.throws(() => acquireLock(lock, "/y", env({ startOf: (pid) => (pid === 77 ? ps2(77, r({ status: 1, stderr: "denied" })) : psStart(pid)) })), /无法判定/);
  assert.deepEqual(stale(d), []);
});

// 确定性交错：用注入的 startOf/afterArchive 在固定位置插入“另一个进程”的动作，不靠时序。
const fake = (alive: Map<number, string>, pid: number, host = "h", extra: Partial<LockEnv> = {}): LockEnv => ({
  pid, host, startOf: (p) => alive.get(p) ?? null, now: () => new Date("2026-10-28T00:00:00Z"), ...extra,
});

test("交错 1：B 判定旧持有者已死之后、进入接管前，A 已完成接管——B 重读发现 token 变了，不得搬走 A 的活锁（去掉 .takeover 守卫内的重读会出现两个持有者）", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[111, "A-start"], [222, "B-start"]]); // 99 已死
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  let a: ReturnType<typeof acquireLock> | undefined;
  const envB = fake(alive, 222, "h", {});
  const startOfB = envB.startOf;
  envB.startOf = (p) => { const r = startOfB(p); if (p === 99 && !a) a = acquireLock(lock, "/a", fake(alive, 111)); return r; }; // B 查完 99 已死的瞬间，A 抢先接管
  assert.throws(() => acquireLock(lock, "/b", envB), /仍在运行/);
  assert.ok(a);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, a!.owner.token, "A 的活锁仍在，没有被 B 搬走");
  assert.equal(stale(d).length, 1, "只归档了一次旧锁（A 的）");
  assert.ok(!existsSync(`${lock}.takeover`));
});

test("交错 2（P3）：旧锁归档之后、新锁替换之前，锁路径始终存在且仍是旧持有者；此刻来的正常取锁者被守卫挡住，不会拿到锁", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[333, "C-start"], [222, "B-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  let seen = ""; let cResult = "未执行";
  const envB = fake(alive, 222, "h", { afterArchive: () => {
    seen = readFileSync(lock, "utf8");
    try { acquireLock(lock, "/c", fake(alive, 333, "h", { guardTries: 2 })); cResult = "C 取到了锁"; } catch (e) { cResult = (e as Error).message; }
  } });
  const b = acquireLock(lock, "/b", envB);
  assert.equal(JSON.parse(seen).token, "old", "归档之后锁路径仍在，内容仍是旧持有者");
  assert.match(cResult, /长时间被占用/, cResult);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, b.owner.token, "只有 B 持锁");
  assert.equal(stale(d).length, 1); assert.equal(JSON.parse(readFileSync(b.archived!, "utf8")).token, "old", "归档是旧锁的内容");
  assert.ok(!existsSync(`${lock}.takeover`));
});

test("交错 2b（P3）：归档之后、替换之前接管者被杀——锁仍在（持有者已死），下一次重入照常接管，archived 非空（N-1 还原不会被跳过）", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[222, "B-start"], [444, "D-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  const before = readFileSync(lock, "utf8");
  assert.throws(() => acquireLock(lock, "/b", fake(alive, 222, "h", { afterArchive: () => { throw new Error("KILLED"); } })), /KILLED/);
  assert.equal(readFileSync(lock, "utf8"), before, "锁路径没有丢失"); assert.equal(stale(d).length, 1);
  const again = acquireLock(lock, "/d", fake(alive, 444));
  assert.ok(again.archived, "第二次接管同样带归档"); assert.equal(stale(d).length, 2);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, again.owner.token);
});

test("接管前清理（beforeTakeover）抛错：锁、现场、守卫都不动；仅在持有者确认已死后才被调用", () => {
  const d = dir(); const lock = join(d, "c.lock"); const alive = new Map<number, string>([[222, "B-start"], [55, "live"]]);
  write(lock, { pid: 55, start: "live", host: "h", token: "t", cwd: "/x" });
  let called = 0;
  assert.throws(() => acquireLock(lock, "/b", fake(alive, 222), () => { called++; }), /仍在运行/);
  assert.equal(called, 0, "持有者存活：不回收");
  write(lock, { pid: 99, start: "dead", host: "h", token: "t", cwd: "/x" });
  const before = readFileSync(lock, "utf8");
  assert.throws(() => acquireLock(lock, "/b", fake(alive, 222), () => { called++; throw new Error("清理失败"); }), /清理失败/);
  assert.equal(called, 1); assert.equal(readFileSync(lock, "utf8"), before); assert.deepEqual(stale(d), []); assert.ok(!existsSync(`${lock}.takeover`));
});

test("runManagedMerge：仅 phase=conflict 且持有者已死时才回收孤儿容器；失败拒绝并保留锁与现场；宿主路由（未提供回收）直接通过", async () => {
  const f = fixture(true);
  const killed = childRun(f, "conflict"); assert.equal(killed.signal, "SIGKILL");
  const holderPid = JSON.parse(readFileSync(`${f.checkpoint}.lock`, "utf8")).pid; await waitDead(holderPid);
  const lockBefore = readFileSync(`${f.checkpoint}.lock`, "utf8"); const headBefore = f.git("rev-parse", "HEAD");
  const resolver = (cwd: string) => async () => { writeFileSync(join(cwd, "a.txt"), "both\n"); return report(["a.txt"]); };
  const base: MergeOptions = { cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
    verify: async (head) => ({ head, checkId: "v1", status: "passed", evidence: "ok" }), resolveConflict: resolver(f.cwd) };
  await assert.rejects(runManagedMerge({ ...base, reclaimOrphans: () => { throw new Error("[FAIL-loud #1742] 孤儿容器不可回收"); } }), /孤儿容器不可回收/);
  assert.equal(readFileSync(`${f.checkpoint}.lock`, "utf8"), lockBefore, "锁未动"); assert.equal(f.git("rev-parse", "HEAD"), headBefore);
  assert.ok(existsSync(join(f.cwd, ".git")) && f.git("rev-parse", "MERGE_HEAD"), "MERGE_HEAD 保留");
  let reclaimed = 0;
  const done = await runManagedMerge({ ...base, reclaimOrphans: () => { reclaimed++; } });
  assert.equal(done.phase, "done"); assert.equal(reclaimed, 1);
  // 未提供回收（宿主路由）：同样的残留锁直接接管
  const g = fixture(true); const k2 = childRun(g, "conflict"); assert.equal(k2.signal, "SIGKILL");
  await waitDead(JSON.parse(readFileSync(`${g.checkpoint}.lock`, "utf8")).pid);
  const done2 = await runManagedMerge({ ...base, cwd: g.cwd, checkpoint: g.checkpoint, manifest: g.manifest, resolveConflict: resolver(g.cwd) }); assert.equal(done2.phase, "done");
  // 非冲突阶段被杀：即使提供了回收，也不调用
  const h = fixture(false); const k3 = childRun(h, "verify"); assert.equal(k3.signal, "SIGKILL");
  await waitDead(JSON.parse(readFileSync(`${h.checkpoint}.lock`, "utf8")).pid);
  let n = 0;
  const done3 = await runManagedMerge({ ...base, cwd: h.cwd, checkpoint: h.checkpoint, manifest: h.manifest, reclaimOrphans: () => { n++; } }); assert.equal(done3.phase, "done"); assert.equal(n, 0);
});


// —— P2 TOCTOU：回收在 .takeover 守卫内、重读 token 之后 ——
test("交错 3（P2）：A 判定旧持有者已死之后，B 已接管并开始自己的回收——A 重读发现 token 变了，不得回收（不会停掉 B 的活容器）", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[111, "A-start"], [222, "B-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  let reclaimsB = 0, reclaimsA = 0; let b: ReturnType<typeof acquireLock> | undefined;
  const envA = fake(alive, 222); const startOfA = envA.startOf;
  envA.startOf = (p) => { const r = startOfA(p); if (p === 99 && !b) b = acquireLock(lock, "/b", fake(alive, 111), () => { reclaimsB++; }); return r; };
  assert.throws(() => acquireLock(lock, "/a", envA, () => { reclaimsA++; }), /仍在运行/);
  assert.equal(reclaimsB, 1); assert.equal(reclaimsA, 0, "A 在 B 接管之后不得回收");
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, b!.owner.token);
});

test("交错 4（M2）：B 在守卫内（now() 调用点）时，第二个接管者必须被守卫挡住；去掉 .takeover 守卫会出现两个持有者", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[333, "C-start"], [222, "B-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  let cResult: unknown = "未执行"; let cReclaim = 0;
  const envB = fake(alive, 222, "h", { guardTries: 40 });
  envB.now = () => { // 恰在 B 持有守卫、尚未归档旧锁时，C 也来接管
    if (cResult === "未执行") { try { acquireLock(lock, "/c", fake(alive, 333, "h", { guardTries: 2 }), () => { cReclaim++; }); cResult = "C 取到了锁"; } catch (e) { cResult = (e as Error).message; } }
    return new Date("2026-10-28T00:00:00Z");
  };
  const b = acquireLock(lock, "/b", envB);
  assert.match(String(cResult), /takeover.*拒绝|长时间被占用/, String(cResult));
  assert.equal(cReclaim, 0);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, b.owner.token, "只有 B 持锁");
  assert.equal(stale(d).length, 1);
});

// —— N-1：孤儿在回收前写了冻结文件 ——
import { chmodSync as chmod2, symlinkSync as sym2 } from "node:fs";
const resolver = (cwd: string) => async () => { writeFileSync(join(cwd, "a.txt"), "both\n"); return report(["a.txt"]); };
const optsOf = (f: ReturnType<typeof fixture>, extra: Partial<MergeOptions> = {}): MergeOptions => ({
  cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
  verify: async (head) => ({ head, checkId: "v1", status: "passed", evidence: "ok" }), resolveConflict: resolver(f.cwd), ...extra });
async function killedWithOrphanWrite(beforeKill: string) {
  const f = fixture(true); const k = childRun(f, "conflict", beforeKill); assert.equal(k.signal, "SIGKILL");
  await waitDead(JSON.parse(readFileSync(`${f.checkpoint}.lock`, "utf8")).pid);
  return f;
}
const orphanArchives = (f: ReturnType<typeof fixture>) => readdirSync(f.evidence).filter((n) => n.startsWith("orphan-wip-"));

test("N-1：孤儿写了 allowed 冻结文件后被回收——自动归档、按检查点还原、收敛到 done，不重复提交", async () => {
  const f = await killedWithOrphanWrite(`fs.writeFileSync(f.cwd + "/a.txt", "ORPHAN WROTE THIS\\n");`);
  assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "ORPHAN WROTE THIS\n");
  let reclaimed = 0; let calls = 0;
  const done = await runManagedMerge(optsOf(f, { reclaimOrphans: () => { reclaimed++; }, resolveConflict: async () => { calls++; return resolver(f.cwd)(); } }));
  assert.equal(done.phase, "done"); assert.equal(reclaimed, 1); assert.equal(calls, 1, "按原流程重新启动冲突工蜂");
  const archives = orphanArchives(f); assert.equal(archives.length, 1);
  const arch = JSON.parse(readFileSync(join(f.evidence, archives[0]!), "utf8"));
  assert.ok(Buffer.from(arch.files["a.txt"].split(":")[1], "base64").toString().includes("ORPHAN WROTE THIS"), "孤儿写入的内容留在证据目录");
  assert.equal(f.git("rev-list", "--parents", "--merges", `${f.base}..HEAD`).split("\n").filter(Boolean).length, 2);
  const head = f.git("rev-parse", "HEAD"); await runManagedMerge(optsOf(f)); assert.equal(f.git("rev-parse", "HEAD"), head);
});

test("N-1：孤儿把 allowed 文件删掉/换成符号链接也能按检查点还原", async () => {
  for (const code of [`fs.rmSync(f.cwd + "/a.txt");`, `fs.rmSync(f.cwd + "/a.txt"); fs.symlinkSync("/etc/hosts", f.cwd + "/a.txt");`]) {
    const f = await killedWithOrphanWrite(code);
    const done = await runManagedMerge(optsOf(f, { reclaimOrphans: () => undefined })); assert.equal(done.phase, "done");
    assert.equal(orphanArchives(f).length, 1);
  }
});

test("N-1：孤儿还动了 allowed 之外的文件 / 未跟踪文件 / index / HEAD / MERGE_HEAD——FAIL-loud，现场不动（连 allowed 也不还原）", async () => {
  const cases: Array<[string, string]> = [
    ["改 allowed 之外的跟踪文件", `fs.writeFileSync(f.cwd + "/b.txt", "EVIL\\n");`],
    ["新增未跟踪文件", `fs.writeFileSync(f.cwd + "/new.txt", "x");`],
    ["暂存 allowed 文件（index 变）", `cp.execFileSync("git", ["add", "a.txt"], { cwd: f.cwd });`],
    ["MERGE_HEAD 被动", `cp.execFileSync("git", ["merge", "--abort"], { cwd: f.cwd });`],
    ["HEAD 被动", `cp.execFileSync("git", ["update-ref", "HEAD", "sandcastle/issue-1"], { cwd: f.cwd });`],
  ];
  for (const [name, code] of cases) {
    const f = fixture(true);
    const pre = `fs.writeFileSync(f.cwd + "/a.txt", "ORPHAN\\n"); ${code}`;
    const k = childRun(f, "conflict", pre); assert.equal(k.signal, "SIGKILL", `${name}: ${k.stderr.slice(0, 300)}`);
    await waitDead(JSON.parse(readFileSync(`${f.checkpoint}.lock`, "utf8")).pid);
    let calls = 0;
    await assert.rejects(runManagedMerge(optsOf(f, { reclaimOrphans: () => undefined, resolveConflict: async () => { calls++; return report(["a.txt"]); } })), /现场与检查点不一致/, name);
    assert.equal(calls, 0, name);
    assert.equal(orphanArchives(f).length, 0, `${name}：不归档、不还原`);
    if (name !== "MERGE_HEAD 被动" && name !== "HEAD 被动" && name !== "暂存 allowed 文件（index 变）") assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "ORPHAN\n", `${name}：allowed 文件也没被还原`);
  }
});

test("N-1：没有发生锁接管 / 未提供回收（宿主路由）时，snapshot 不一致保持原来的 FAIL-loud", async () => {
  // 无接管：先把残留锁人工移走，再手改 allowed 文件
  const f = fixture(true); const k = childRun(f, "conflict"); assert.equal(k.signal, "SIGKILL");
  await waitDead(JSON.parse(readFileSync(`${f.checkpoint}.lock`, "utf8")).pid);
  rmSync(`${f.checkpoint}.lock`); writeFileSync(join(f.cwd, "a.txt"), "HAND EDIT\n");
  await assert.rejects(runManagedMerge(optsOf(f, { reclaimOrphans: () => undefined })), /现场与检查点不一致/);
  assert.equal(orphanArchives(f).length, 0); assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "HAND EDIT\n");
  // 接管但没有回收钩子：无法确认没有存活孤儿 → 不还原
  const g = await killedWithOrphanWrite(`fs.writeFileSync(f.cwd + "/a.txt", "ORPHAN\\n");`);
  await assert.rejects(runManagedMerge(optsOf(g)), /现场与检查点不一致/);
  assert.equal(orphanArchives(g).length, 0);
});

test("P3：检查点无法解析时不跳过回收（FAIL-loud）；只有 ENOENT 才跳过", async () => {
  const f = await killedWithOrphanWrite("");
  writeFileSync(f.checkpoint, '{"phase":"conf'); let n = 0;
  await assert.rejects(runManagedMerge(optsOf(f, { reclaimOrphans: () => { n++; } })), /检查点无法解析/);
  assert.equal(n, 0); assert.ok(existsSync(`${f.checkpoint}.lock`), "锁未动");
  const g = fixture(true); // 无检查点 + 残留锁：ENOENT → 跳过回收，正常取锁
  writeFileSync(`${g.checkpoint}.lock`, JSON.stringify({ pid: 99, start: "dead", host: hostname(), token: "t", cwd: g.cwd }));
  let m = 0; const done = await runManagedMerge(optsOf(g, { reclaimOrphans: () => { m++; } })); assert.equal(done.phase, "done"); assert.equal(m, 0);
});

// —— G-1：接管守卫持有者被 SIGKILL 后的守卫残留 ——
const guardOf = (lock: string) => `${lock}.takeover`;
const guardStale = (d: string) => readdirSync(d).filter((f) => f.includes(".takeover.stale-"));
const putGuard = (lock: string, o: Record<string, unknown>) => writeFileSync(guardOf(lock), JSON.stringify(o));

test("G-1（d）：守卫持有者已死（pid 不存在 / pid 被复用）——第二层守卫搬走残留守卫，接管照常完成，beforeTakeover 恰好一次", () => {
  for (const [name, gp, gs] of [["pid 不存在", 98, "G-start"], ["pid 被复用", 555, "OLD-start"]] as const) {
    const d = dir(); const lock = join(d, "c.lock");
    const alive = new Map<number, string>([[222, "B-start"], [555, "NEW-start"]]);
    write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
    putGuard(lock, { pid: gp, start: gs, host: "h", token: "g-old", cwd: "/x" });
    let calls = 0; const b = acquireLock(lock, "/b", fake(alive, 222), () => { calls++; });
    assert.equal(calls, 1, name); assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, b.owner.token, name);
    assert.equal(guardStale(d).length, 1, name); assert.equal(JSON.parse(readFileSync(join(d, guardStale(d)[0]!), "utf8")).token, "g-old", `${name}：残留守卫留作证据`);
    assert.ok(!existsSync(guardOf(lock)) && !existsSync(`${guardOf(lock)}.g2`), `${name}：守卫与 g2 都已释放`);
  }
});

test("G-1（b）：守卫持有者存活——不得搬走，守卫原样，拒绝", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[222, "B-start"], [777, "G-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  putGuard(lock, { pid: 777, start: "G-start", host: "h", token: "g-live", cwd: "/x" });
  const gBefore = readFileSync(guardOf(lock), "utf8"); const lBefore = readFileSync(lock, "utf8"); let calls = 0;
  assert.throws(() => acquireLock(lock, "/b", fake(alive, 222, "h", { guardTries: 3 }), () => { calls++; }), /长时间被占用/);
  assert.equal(readFileSync(guardOf(lock), "utf8"), gBefore); assert.equal(readFileSync(lock, "utf8"), lBefore);
  assert.deepEqual(guardStale(d), []); assert.ok(!existsSync(`${guardOf(lock)}.g2`)); assert.equal(calls, 0);
});

test("G-1（c）：守卫无法解析 / 主机不同 / ps 无法判定——FAIL-loud，守卫、锁、g2 都不动", () => {
  const alive = new Map<number, string>([[222, "B-start"]]);
  const cases: Array<[string, () => string, LockEnv]> = [
    ["无法解析", () => "{bad", fake(alive, 222, "h", { guardTries: 2 })],
    ["缺身份", () => JSON.stringify({ pid: 98 }), fake(alive, 222, "h", { guardTries: 2 })],
    ["主机不同", () => JSON.stringify({ pid: 98, start: "x", host: "other", token: "g", cwd: "/x" }), fake(alive, 222, "h", { guardTries: 2 })],
    ["ps 无法判定", () => JSON.stringify({ pid: 98, start: "x", host: "h", token: "g", cwd: "/x" }),
      fake(alive, 222, "h", { guardTries: 2, startOf: (p) => { if (p === 98) throw new Error("ps boom"); return alive.get(p) ?? null; } })],
  ];
  for (const [name, content, e] of cases) {
    const d = dir(); const lock = join(d, "c.lock");
    write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
    writeFileSync(guardOf(lock), content()); const gBefore = readFileSync(guardOf(lock), "utf8"); const lBefore = readFileSync(lock, "utf8");
    assert.throws(() => acquireLock(lock, "/b", e), /长时间被占用/, name);
    assert.equal(readFileSync(guardOf(lock), "utf8"), gBefore, name); assert.equal(readFileSync(lock, "utf8"), lBefore, name);
    assert.deepEqual(guardStale(d), [], name); assert.ok(!existsSync(`${guardOf(lock)}.g2`), name);
  }
});

test("G-1（d 交错 A）：两个竞争者都判定守卫已死——先完成者搬走旧守卫并接管，后者在 g2 内重读到 ENOENT，不动任何东西，始终只有一个持锁者", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[222, "X-start"], [333, "Y-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  putGuard(lock, { pid: 98, start: "G-start", host: "h", token: "g-old", cwd: "/x" });
  let y: ReturnType<typeof acquireLock> | undefined; let xReclaims = 0;
  const envX = fake(alive, 222, "h", { guardTries: 2 }); const startOfX = envX.startOf;
  envX.startOf = (p) => { const r = startOfX(p); if (p === 98 && !y) y = acquireLock(lock, "/y", fake(alive, 333)); return r; }; // X 判定旧守卫已死的瞬间，Y 抢先完成整套接管
  assert.throws(() => acquireLock(lock, "/x", envX, () => { xReclaims++; }), /仍在运行/);
  assert.ok(y); assert.equal(xReclaims, 0, "X 没有进入接管前清理");
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, y!.owner.token, "Y 的锁没被动");
  assert.equal(guardStale(d).length, 1, "旧守卫只被搬走一次"); assert.equal(stale(d).filter((f) => !f.includes(".takeover.")).length, 1, "旧锁只归档一次");
  assert.ok(!existsSync(guardOf(lock)) && !existsSync(`${guardOf(lock)}.g2`));
});

test("G-1（d 交错 B）：X 判定旧守卫已死之后，旧守卫被换成另一个存活者的新守卫——X 在 g2 内重读到不同 token，不得搬走活守卫（去掉重读会误搬）", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[222, "X-start"], [777, "G-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  putGuard(lock, { pid: 98, start: "dead-start", host: "h", token: "g-old", cwd: "/x" });
  let swapped = false; const envX = fake(alive, 222, "h", { guardTries: 2 }); const startOfX = envX.startOf;
  envX.startOf = (p) => {
    const r = startOfX(p);
    if (p === 98 && !swapped) { swapped = true; renameSync(guardOf(lock), `${guardOf(lock)}.manual`); putGuard(lock, { pid: 777, start: "G-start", host: "h", token: "g-new", cwd: "/y" }); }
    return r;
  };
  let calls = 0;
  assert.throws(() => acquireLock(lock, "/x", envX, () => { calls++; }), /长时间被占用/);
  assert.equal(JSON.parse(readFileSync(guardOf(lock), "utf8")).token, "g-new", "活守卫仍在原位"); assert.deepEqual(guardStale(d), []);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, "old"); assert.equal(calls, 0); assert.ok(!existsSync(`${guardOf(lock)}.g2`));
});

test("G-1（e）：g2 自身残留（毫秒窗口内的双重故障）——文档化限制：FAIL-loud，不碰守卫和 g2；按 runbook 只 rename g2 后重入收敛，已死的守卫被自动归档", () => {
  const d = dir(); const lock = join(d, "c.lock");
  const alive = new Map<number, string>([[222, "B-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  putGuard(lock, { pid: 98, start: "G-start", host: "h", token: "g-old", cwd: "/x" });
  writeFileSync(`${guardOf(lock)}.g2`, JSON.stringify({ pid: 97, start: "G2-start", host: "h", token: "g2-old", cwd: "/x" }));
  const gBefore = readFileSync(guardOf(lock), "utf8"); const g2Before = readFileSync(`${guardOf(lock)}.g2`, "utf8");
  assert.throws(() => acquireLock(lock, "/b", fake(alive, 222, "h", { guardTries: 2 })), /长时间被占用.*\.g2.*TZ=UTC LC_ALL=C ps -p/);
  assert.equal(readFileSync(guardOf(lock), "utf8"), gBefore); assert.equal(readFileSync(`${guardOf(lock)}.g2`, "utf8"), g2Before);
  // runbook：只 rename g2（不碰 .takeover）
  renameSync(`${guardOf(lock)}.g2`, `${guardOf(lock)}.g2.manual-x`);
  const held = acquireLock(lock, "/b", fake(alive, 222)); assert.ok(held.archived);
  assert.equal(guardStale(d).length, 1, "已死的守卫由重入自动归档"); assert.equal(JSON.parse(readFileSync(join(d, guardStale(d)[0]!), "utf8")).token, "g-old");
  assert.ok(!existsSync(guardOf(lock)) && !existsSync(`${guardOf(lock)}.g2`));
});

test("runbook 口径锁：README 与报错文字都要求 `TZ=UTC LC_ALL=C ps -p`，且不让人手动处理 .takeover", () => {
  const readme = readFileSync(new URL("./README.md", import.meta.url), "utf8");
  assert.ok(readme.includes("TZ=UTC LC_ALL=C ps -p <pid> -o lstart="), "README 命令带 TZ/LC_ALL");
  assert.match(readme, /不要手动处理 `\*\.takeover`/);
  assert.ok(!/ps -p <pid> -o lstart=/.test(readme.replace(/TZ=UTC LC_ALL=C ps -p <pid> -o lstart=/g, "")), "README 里没有不带 TZ 的 ps 命令");
  const src = readFileSync(new URL("./process-lock.mts", import.meta.url), "utf8");
  assert.ok(src.includes("TZ=UTC LC_ALL=C ps -p <pid> -o lstart=") && src.includes("不要手动处理"));
});

test("P3-c：守卫释放带 token 校验——守卫在持有期间被换成别人的，退出时不得删掉", () => {
  const d = dir(); const lock = join(d, "c.lock"); const alive = new Map<number, string>([[222, "B-start"], [777, "G-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  const b = acquireLock(lock, "/b", fake(alive, 222), () => {
    renameSync(guardOf(lock), `${guardOf(lock)}.manual`); putGuard(lock, { pid: 777, start: "G-start", host: "h", token: "other", cwd: "/y" });
  });
  assert.ok(b.archived); assert.equal(JSON.parse(readFileSync(guardOf(lock), "utf8")).token, "other", "别人的守卫仍在");
});

test("P3-b：新守卫复用了已死持有者的 pid、但 token 不同——g2 内必须按 token 复核，不得搬走这个活守卫（改用 pid 复核会误搬）", () => {
  const d = dir(); const lock = join(d, "c.lock"); const alive = new Map<number, string>([[222, "X-start"]]);
  write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
  putGuard(lock, { pid: 98, start: "OLD-start", host: "h", token: "g-old", cwd: "/x" });
  let swapped = false; const envX = fake(alive, 222, "h", { guardTries: 2 });
  envX.startOf = (p) => {
    if (p === 98) {
      if (!swapped) { // X 判定“98 的启动时间不同＝已死”的瞬间：旧守卫被换成复用 pid 98 的新守卫，pid 98 此后以 NEW-start 存活
        swapped = true; renameSync(guardOf(lock), `${guardOf(lock)}.manual`); putGuard(lock, { pid: 98, start: "NEW-start", host: "h", token: "g-new", cwd: "/y" });
        alive.set(98, "NEW-start"); return "REUSED-start";
      }
      return alive.get(98) ?? null;
    }
    return alive.get(p) ?? null;
  };
  assert.throws(() => acquireLock(lock, "/x", envX), /长时间被占用/);
  assert.equal(JSON.parse(readFileSync(guardOf(lock), "utf8")).token, "g-new", "同 pid 的活守卫没被搬走"); assert.deepEqual(guardStale(d), []);
  assert.equal(JSON.parse(readFileSync(lock, "utf8")).token, "old"); assert.ok(!existsSync(`${guardOf(lock)}.g2`));
});

test("P3-a：持有守卫的重入顺手归档已死的 g2；g2 持有者存活 / 无法解析 / 主机不同 / ps 无法判定时不动", () => {
  const g2 = (lock: string) => `${guardOf(lock)}.g2`; const g2Stale = (d: string) => readdirSync(d).filter((f) => f.includes(".g2.stale-"));
  { // 已死：归档
    const d = dir(); const lock = join(d, "c.lock"); const alive = new Map<number, string>([[222, "B-start"]]);
    write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
    writeFileSync(g2(lock), JSON.stringify({ pid: 97, start: "G2-start", host: "h", token: "g2-dead", cwd: "/x" }));
    const b = acquireLock(lock, "/b", fake(alive, 222)); assert.ok(b.archived);
    assert.equal(g2Stale(d).length, 1); assert.equal(JSON.parse(readFileSync(join(d, g2Stale(d)[0]!), "utf8")).token, "g2-dead"); assert.ok(!existsSync(g2(lock)) && !existsSync(guardOf(lock)));
  }
  const keep: Array<[string, string, LockEnv]> = [
    ["存活（另一条回收路径正在 g2 内）", JSON.stringify({ pid: 97, start: "G2-start", host: "h", token: "g2-live", cwd: "/x" }), fake(new Map([[222, "B-start"], [97, "G2-start"]]), 222)],
    ["无法解析", "{bad", fake(new Map([[222, "B-start"]]), 222)],
    ["主机不同", JSON.stringify({ pid: 97, start: "x", host: "other", token: "g2", cwd: "/x" }), fake(new Map([[222, "B-start"]]), 222)],
    ["ps 无法判定", JSON.stringify({ pid: 97, start: "x", host: "h", token: "g2", cwd: "/x" }),
      fake(new Map([[222, "B-start"]]), 222, "h", { startOf: (p) => { if (p === 97) throw new Error("ps boom"); return p === 222 ? "B-start" : null; } })],
  ];
  for (const [name, content, e] of keep) {
    const d = dir(); const lock = join(d, "c.lock"); write(lock, { pid: 99, start: "dead", host: "h", token: "old", cwd: "/x" });
    writeFileSync(g2(lock), content);
    acquireLock(lock, "/b", e); assert.equal(readFileSync(g2(lock), "utf8"), content, `${name}：g2 原样`); assert.deepEqual(g2Stale(d), [], name);
  }
});

// —— G-1（a）：真实进程在守卫内被 SIGKILL，宿主重入收敛 ——
function childReenter(f: ReturnType<typeof fixture>, hook: "reclaim" | "afterArchive" | "guardRename") {
  const script = join(f.evidence, `reenter-${hook}.mts`);
  const extra = hook === "reclaim" ? "reclaimOrphans: () => { kill(); }" : hook === "afterArchive" ? "reclaimOrphans: () => {}, lockEnv: { ...realEnv(), afterArchive: kill }" : "reclaimOrphans: () => {}, lockEnv: { ...realEnv(), afterGuardReclaim: kill }";
  writeFileSync(script, `import { runManagedMerge } from ${JSON.stringify(here + "managed-merge.mts")}; import { realEnv } from ${JSON.stringify(here + "process-lock.mts")};
const f = ${JSON.stringify({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest })};
const kill = () => process.kill(process.pid, "SIGKILL");
await runManagedMerge({ cwd: f.cwd, checkpoint: f.checkpoint, manifest: f.manifest, checkId: "v1",
  verify: async (head) => ({ head, checkId: "v1", status: "passed", evidence: "ok" }), resolveConflict: async () => { throw new Error("x"); }, ${extra} });`);
  return spawnSync(process.execPath, ["--import", loader, script], { encoding: "utf8" });
}
const ORPHAN_WRITE = `fs.writeFileSync(f.cwd + "/a.txt", "ORPHAN WROTE THIS\\n");`;
for (const [name, orphanCode, hook] of [
  ["回收钩子内被杀（孤儿未写）", "", "reclaim"],
  ["回收钩子内被杀（孤儿已写）", ORPHAN_WRITE, "reclaim"],
  ["P3：归档之后、替换之前被杀（孤儿已写）", ORPHAN_WRITE, "afterArchive"],
] as const) {
  test(`G-1（a）：${name}——宿主重入自动搬走残留守卫并收敛到 done，不重复提交`, async () => {
    const f = await killedWithOrphanWrite(orphanCode); const lock = `${f.checkpoint}.lock`;
    const k = childReenter(f, hook); assert.equal(k.signal, "SIGKILL", k.stderr.slice(0, 300));
    const guard = guardOf(lock); assert.ok(existsSync(guard), "守卫残留");
    await waitDead(JSON.parse(readFileSync(guard, "utf8")).pid); await waitDead(k.pid);
    assert.ok(existsSync(lock), "锁路径始终存在");
    if (hook === "afterArchive") assert.equal(stale(f.evidence).filter((n) => !n.includes(".takeover.")).length, 1, "被杀前已归档一次");
    let reclaimed = 0;
    const done = await runManagedMerge(optsOf(f, { reclaimOrphans: () => { reclaimed++; } }));
    assert.equal(done.phase, "done"); assert.equal(reclaimed, 1);
    assert.equal(guardStale(f.evidence).length, 1, "残留守卫已归档"); assert.ok(!existsSync(guard) && !existsSync(`${guard}.g2`) && !existsSync(lock));
    assert.equal(orphanArchives(f).length, orphanCode ? 1 : 0, "孤儿已写时留下 orphan-wip 归档");
    assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), "both\n");
    assert.equal(f.git("rev-list", "--parents", "--merges", `${f.base}..HEAD`).split("\n").filter(Boolean).length, 2, "两个归并提交，无重复");
    const head = f.git("rev-parse", "HEAD"); await runManagedMerge(optsOf(f)); assert.equal(f.git("rev-parse", "HEAD"), head);
  });
}

// —— R-7：还原后重算快照必须与检查点逐字相等 ——
test("R-7：还原写回的字节相同但映像与检查点不相等时，FAIL-loud 并带归档路径（删掉还原后的逐字比较，这条报错就变成别的）", async () => {
  const f = await killedWithOrphanWrite("");
  const cp = JSON.parse(readFileSync(f.checkpoint, "utf8"));
  const img: string = cp.snapshot.files["a.txt"]; const i = img.indexOf(":"); const b64 = img.slice(i + 1); assert.ok(b64.length > 8);
  cp.snapshot.files["a.txt"] = `${img.slice(0, i)}:${b64.slice(0, 4)}\n${b64.slice(4)}`; // 同样的字节，非规范 base64：还原后重算得到规范编码，必然与检查点不等
  writeFileSync(f.checkpoint, JSON.stringify(cp, null, 2));
  const before = readFileSync(join(f.cwd, "a.txt"), "utf8"); let calls = 0;
  await assert.rejects(runManagedMerge(optsOf(f, { reclaimOrphans: () => undefined, resolveConflict: async () => { calls++; return report(["a.txt"]); } })), /孤儿写入已归档.*仍与检查点不一致/s);
  assert.equal(calls, 0); assert.equal(orphanArchives(f).length, 1, "先归档"); assert.equal(readFileSync(join(f.cwd, "a.txt"), "utf8"), before);
});

test("P3-a（真实进程）：g2 内、搬走旧守卫之后被杀——g2 残留，宿主重入收敛到 done，死 g2 被归档为 .g2.stale-*，结束时守卫与 g2 都不残留", async () => {
  const f = await killedWithOrphanWrite(ORPHAN_WRITE); const lock = `${f.checkpoint}.lock`; const guard = guardOf(lock);
  const k2 = childReenter(f, "reclaim"); assert.equal(k2.signal, "SIGKILL", k2.stderr.slice(0, 300)); await waitDead(k2.pid!);
  assert.ok(existsSync(guard), "第二个重入在守卫内被杀，守卫残留");
  const k3 = childReenter(f, "guardRename"); assert.equal(k3.signal, "SIGKILL", k3.stderr.slice(0, 300)); await waitDead(k3.pid!);
  assert.ok(existsSync(`${guard}.g2`), "g2 残留"); assert.ok(!existsSync(guard), "旧守卫已被搬走");
  const done = await runManagedMerge(optsOf(f, { reclaimOrphans: () => undefined }));
  assert.equal(done.phase, "done");
  assert.ok(!existsSync(guard) && !existsSync(`${guard}.g2`) && !existsSync(lock));
  assert.equal(readdirSync(f.evidence).filter((n) => n.includes(".g2.stale-")).length, 1, "死 g2 已归档"); assert.equal(guardStale(f.evidence).length, 1, "旧守卫已归档");
  assert.equal(orphanArchives(f).length, 1); assert.equal(f.git("rev-list", "--parents", "--merges", `${f.base}..HEAD`).split("\n").filter(Boolean).length, 2);
});
