import { test } from "node:test";
import assert from "node:assert/strict";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ACCEPTANCE_ROWS, MAX_RESUMES, ROW_ENV, acceptedFor, checkAdmission, contractDigest, issueReceipt, mergeBinding,
  recordAcceptance, retireReceipt, swarmBinding, type AcceptanceRow,
} from "./docker-acceptance.mts";
import { acceptanceRoot, assertMountsExcludeAcceptance, controlledDir } from "./acceptance-dir.mts";
import { controlledSwarmTicket, TICKET_ENV } from "./swarm-controlled.mts";

const HEAD = "a".repeat(40);
const IMAGE = "sandcastle:newchanlun";
const SWARM_TICKET = 4242;
const clean = { head: HEAD, dirty: "" };
const dir = () => mkdtempSync(join(tmpdir(), "docker-acceptance-")); // mkdtemp：0700、属于当前用户
const env = (row?: string): NodeJS.ProcessEnv => (row ? { [ROW_ENV]: row } : {});
const ctx = (d: string, row?: string, binding?: string, extra: Record<string, unknown> = {}) => ({ dir: d, env: env(row), identity: clean, binding, ...extra });
const issue = (d: string, extra: Record<string, unknown> = {}) =>
  issueReceipt({ dir: d, image: IMAGE, approval: "turn 01a17310 用户批准方案B", swarmTicket: SWARM_TICKET, identity: clean, ...extra });
const B13 = swarmBinding(SWARM_TICKET);
const B14 = mergeBinding({ runId: "run-a", base: "b".repeat(40), head: "c".repeat(40), incoming: "d".repeat(40) });
const B15 = mergeBinding({ runId: "run-b", base: "b".repeat(40), head: "c".repeat(40), incoming: "e".repeat(40) });
const BINDINGS: Record<AcceptanceRow, string> = { "13": B13, "14": B14, "15": B15 };

function review(d: string, model = "claude-opus-5-5", head = "f".repeat(40), verdict = "VERDICT: PASS") {
  const p = join(d, `review-${model.replace(/\W/g, "_")}.txt`);
  writeFileSync(p, `${verdict}\nHEAD: ${head}\nREVIEWER_MODEL: ${model}\n`);
  return p;
}
function evidence(d: string) {
  return Object.fromEntries(ACCEPTANCE_ROWS.map((r) => { const p = join(d, `ev-${r}.txt`); writeFileSync(p, r); return [r, p]; })) as Record<AcceptanceRow, string>;
}
function runAll(d: string) { for (const r of ACCEPTANCE_ROWS) checkAdmission(IMAGE, ctx(d, r, BINDINGS[r])).consume(); }
function accepted(d: string, extra: Record<string, unknown> = {}) {
  issue(d, extra); runAll(d);
  return recordAcceptance({ dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
}
const read = (d: string, f: string) => JSON.parse(readFileSync(join(d, f), "utf8"));
const write = (d: string, f: string, v: unknown) => { writeFileSync(join(d, f), JSON.stringify(v)); chmodSync(join(d, f), 0o600); };

test("无 receipt、无验收行、不精确代码一律 FAIL-loud", () => {
  const d = dir();
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 缺失/);
  issue(d);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, B13)), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "16", B13)), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /缺少运行绑定/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { identity: { head: HEAD, dirty: " M .sandcastle/x.mts" } })), /代码不精确/);
  assert.throws(() => issueReceipt({ dir: dir(), image: IMAGE, approval: "x", swarmTicket: 1, identity: { head: HEAD, dirty: "?? .sandcastle/new.mts" } }), /未提交改动/);
  assert.throws(() => issueReceipt({ dir: dir(), image: IMAGE, approval: " ", swarmTicket: 1, identity: clean }), /批准引用/);
  assert.throws(() => issueReceipt({ dir: dir(), image: IMAGE, approval: "x", identity: clean }), /专用验收票/);
  assert.throws(() => issue(dir(), { ttlHours: 100000 }), /ttlHours/);
});

test("receipt 绑定 head、合同摘要、镜像、批准范围、有效期与第 13 行的专用票", () => {
  const d = dir(); const r = issue(d, { rows: ["13", "14"] });
  assert.equal(r.contract, contractDigest());
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { identity: { head: "b".repeat(40), dirty: "" } })), /head 漂移/);
  assert.throws(() => checkAdmission("other:image", ctx(d, "13", B13)), /镜像不一致/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "15", B15)), /批准范围不含第 15 行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { now: new Date(Date.now() + 13 * 3_600_000) })), /已过期/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", swarmBinding(1237))), /只允许专用验收票 #4242/, "不能指向在飞业务票 #1237");
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { rows: ["14", "15"] })), /不能用于此入口/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", B14, { rows: ["13"] })), /不能用于此入口/);
  write(d, "receipt.json", { ...read(d, "receipt.json"), contract: "0".repeat(64) });
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /隔离合同已变化/);
});

test("非法 expiresAt / 半截 receipt 不会失败开放", () => {
  const d = dir(); const r = issue(d);
  write(d, "receipt.json", { ...r, expiresAt: "not-a-date" });
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 无效.*expiresAt/);
  writeFileSync(join(d, "receipt.json"), '{"schema":2,"tick'); chmodSync(join(d, "receipt.json"), 0o600);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 无效/);
  write(d, "receipt.json", { ...r, rows: ["13"], swarmTicket: undefined });
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /专用验收票/);
  write(d, "receipt.json", { ...r, head: "zz" });
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /40 位 SHA/);
});

test("验收目录只认宿主受控目录：符号链接、宽权限、与 worktree 重叠、权限宽的事实文件都拒绝", () => {
  const d = dir();
  const link = join(tmpdir(), `acc-link-${Date.now()}`); symlinkSync(d, link);
  assert.throws(() => controlledDir(link), /符号链接/);
  const wide = dir(); chmodSync(wide, 0o777);
  assert.throws(() => controlledDir(wide), /权限过宽/);
  assert.throws(() => checkAdmission(IMAGE, ctx(wide, "13", B13)), /验收目录不合格/);
  const inside = dir(); const nested = join(inside, "wt", ".sandcastle", "logs"); mkdirSync(nested, { recursive: true, mode: 0o700 });
  assert.throws(() => controlledDir(nested, { roots: [join(inside, "wt")] }), /与 worktree 重叠/);
  assert.throws(() => controlledDir(inside, { roots: [join(inside, "wt")] }), /与 worktree 重叠/, "目录包含 worktree 同样拒绝");
  issue(d);
  chmodSync(join(d, "receipt.json"), 0o666);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /权限过宽/);
});

test("生产路径不读环境变量：SANDCASTLE_DOCKER_ACCEPTANCE_DIR 指向的伪造目录无效", () => {
  const forged = dir();
  const home = dir();
  const prev = { HOME: process.env.HOME, FORGED: process.env.SANDCASTLE_DOCKER_ACCEPTANCE_DIR };
  process.env.HOME = home; process.env.SANDCASTLE_DOCKER_ACCEPTANCE_DIR = forged;
  try {
    // 在伪造目录里放一份“完整”记录。
    accepted(forged);
    assert.equal(acceptedFor(IMAGE, { identity: clean }), null);
    assert.throws(() => checkAdmission(IMAGE, { identity: clean, env: {} }), /验收目录不合格|未声明受控验收行/);
    assert.equal(acceptanceRoot(), join(home, ".sandcastle-host", "docker-acceptance-1742"));
  } finally {
    process.env.HOME = prev.HOME;
    if (prev.FORGED === undefined) delete process.env.SANDCASTLE_DOCKER_ACCEPTANCE_DIR; else process.env.SANDCASTLE_DOCKER_ACCEPTANCE_DIR = prev.FORGED;
  }
});

test("容器挂载不得暴露验收目录或其祖先（如整个 $HOME）", () => {
  const home = dir(); const root = acceptanceRoot(home);
  mkdirSync(root, { recursive: true });
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: home }], root), /暴露/);
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: root }], root), /暴露/);
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: join(root, "sub") }], root), /暴露/);
  assertMountsExcludeAcceptance([{ hostPath: join(home, "elsewhere") }], root);
});

test("伪造 accepted.json 无效：无 receipt / 无 used / nonce、批准、有效期被替换都放行失败", () => {
  const d = dir(); const r = issue(d);
  const forged = { schema: 2, ticket: 1742, nonce: r.nonce, head: HEAD, contract: contractDigest(), image: IMAGE, approval: r.approval, issuedAt: r.issuedAt, expiresAt: r.expiresAt, acceptedAt: r.issuedAt, evidence: evidence(d), crossFamily: { file: review(d), head: "f".repeat(40), model: "claude-opus-5-5" } };
  write(d, "accepted.json", forged);
  assert.equal(acceptedFor(IMAGE, ctx(d)), null, "没有 used 记录");
  assert.throws(() => checkAdmission(IMAGE, ctx(d)), /未声明受控验收行/);
  const d2 = dir(); write(d2, "accepted.json", forged);
  assert.equal(acceptedFor(IMAGE, ctx(d2)), null, "没有 receipt");
  const d3 = dir(); accepted(d3);
  assert.ok(acceptedFor(IMAGE, ctx(d3)));
  const good = read(d3, "accepted.json");
  for (const patch of [{ nonce: "other" }, { approval: "别的批准" }, { expiresAt: new Date(Date.now() + 9e9).toISOString() }, { head: "b".repeat(40) }, { image: "x:y" }, { contract: "0".repeat(64) }]) {
    write(d3, "accepted.json", { ...good, ...patch });
    assert.equal(acceptedFor(IMAGE, ctx(d3)), null, JSON.stringify(patch));
  }
});

test("登记验收：拒绝过期 receipt、批准引用被替换、缺行、缺证据；记录保存有效期，到期自动失效", () => {
  const d = dir(); const r = issue(d); runAll(d);
  const ev = evidence(d); const rv = review(d);
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, evidence: ev, crossFamilyReview: rv, identity: clean, now: new Date(Date.parse(r.expiresAt) + 1000) }), /已过期/);
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, approval: "different-approval", evidence: ev, crossFamilyReview: rv, identity: clean }), /批准引用与 receipt 不一致/);
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, evidence: { ...ev, "15": join(d, "missing") }, crossFamilyReview: rv, identity: clean }), /缺少证据文件/);
  const rec = recordAcceptance({ dir: d, image: IMAGE, approval: r.approval, evidence: ev, crossFamilyReview: rv, identity: clean });
  assert.equal(rec.expiresAt, r.expiresAt); assert.equal(rec.approval, r.approval);
  assert.ok(checkAdmission(IMAGE, ctx(d)).accepted);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, undefined, { now: new Date(Date.parse(r.expiresAt) + 1000) })), /未声明受控验收行/);
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, evidence: ev, crossFamilyReview: rv, identity: clean }), /EEXIST|file already exists/);
});

test("used 标记崩溃安全：半写/损坏/不一致的记录不算事实，登记与放行都拒绝", () => {
  const d = dir(); const r = issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  const f13 = `used-${r.nonce}-13.json`;
  assert.deepEqual(readdirSync(d).filter((f) => f.includes(".tmp-")), [], "发布后不留临时文件");
  const good = read(d, f13);
  // 崩溃在发布前只会留 .tmp：不影响占用
  writeFileSync(join(d, `used-${r.nonce}-14.json.tmp-123-x`), '{"sche');
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume();
  for (const bad of ['{"sche', "{}", JSON.stringify({ ...good, nonce: "other" }), JSON.stringify({ ...good, head: "b".repeat(40) }), JSON.stringify({ ...good, row: "14" }), JSON.stringify({ ...good, binding: "" }), JSON.stringify({ ...good, at: "nope" })]) {
    writeFileSync(join(d, f13), bad); chmodSync(join(d, f13), 0o600);
    assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean }), /used 记录无效/, bad.slice(0, 30));
    assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /used 记录无效/, bad.slice(0, 30));
  }
});

test("每个验收行一次性：占用后新运行/并发占用都拒绝；占用前核对不消耗；同一行不同绑定拒绝", () => {
  const d = dir(); issue(d);
  const a = checkAdmission(IMAGE, ctx(d, "14", B14)); const b = checkAdmission(IMAGE, ctx(d, "14", B14));
  a.consume();
  assert.throws(() => b.consume(), /并发占用/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", B15)), /另一运行\/检查点使用，新运行不得复用/);
  checkAdmission(IMAGE, ctx(d, "15", B15)); // 其他行不受影响
});

test("第 15 行：同一 run+检查点被 SIGKILL 后重入是恢复而非重复占用，且登记可完成；新运行仍被拒", () => {
  const d = dir(); issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume(); // 首个进程占用后被 SIGKILL
  const again = checkAdmission(IMAGE, ctx(d, "15", B15)); // 重入：同一 run、同一检查点
  assert.equal(again.resumed, true);
  again.consume();
  const resumes = readdirSync(d).filter((f) => f.startsWith("resume-"));
  assert.equal(resumes.length, 1);
  const rec = recordAcceptance({ dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  assert.ok(rec);
  const other = mergeBinding({ runId: "run-b", base: "b".repeat(40), head: "c".repeat(40), incoming: "9".repeat(40) });
  const d2 = dir(); issue(d2); checkAdmission(IMAGE, ctx(d2, "15", B15)).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", other)), /新运行不得复用/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", mergeBinding({ runId: "run-c", base: "b".repeat(40), head: "c".repeat(40), incoming: "e".repeat(40) }))), /新运行不得复用/);
  for (let i = 0; i < MAX_RESUMES; i++) checkAdmission(IMAGE, ctx(d2, "15", B15)).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", B15)), /恢复次数已达上限/);
});

test("同一 checkout 不允许重复签发，须先退役保留历史；退役同时作废 accepted", () => {
  const d = dir(); accepted(d);
  assert.throws(() => issue(d), /EEXIST|file already exists/);
  const moved = retireReceipt({ dir: d });
  assert.ok(moved && existsSync(moved) && !existsSync(join(d, "receipt.json")) && !existsSync(join(d, "accepted.json")));
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
  issue(d);
});

test("外部异家族独审：同家族、缺 PASS、缺 HEAD、缺模型均拒绝；通过后记录模型与 HEAD", () => {
  const d = dir(); issue(d); runAll(d); const ev = evidence(d);
  const attempt = (file: string) => recordAcceptance({ dir: d, image: IMAGE, evidence: ev, crossFamilyReview: file, identity: clean });
  assert.throws(() => attempt(review(d, "gpt-6.1-sol")), /同家族/);
  assert.throws(() => attempt(review(d, "mf/gpt-5.5")), /同家族/);
  assert.throws(() => attempt(review(d, "claude-opus-5-5", "f".repeat(40), "VERDICT: BLOCK")), /VERDICT: PASS/);
  assert.throws(() => attempt(review(d, "claude-opus-5-5", "short")), /HEAD/);
  assert.throws(() => attempt(join(d, "none.txt")), /缺少外部异家族独审报告/);
  const rec = attempt(review(d, "claude-opus-5-5"));
  assert.equal(rec.crossFamily.model, "claude-opus-5-5"); assert.equal(rec.crossFamily.head, "f".repeat(40));
});

test("受控第 13 行入口：必须显式专用验收票，不读线上队列", () => {
  assert.equal(controlledSwarmTicket({}), undefined);
  assert.equal(controlledSwarmTicket({ [ROW_ENV]: "13", [TICKET_ENV]: "4242" }), 4242);
  assert.throws(() => controlledSwarmTicket({ [ROW_ENV]: "13" }), /专用验收票/);
  assert.throws(() => controlledSwarmTicket({ [ROW_ENV]: "13", [TICKET_ENV]: "12x" }), /专用验收票/);
  assert.throws(() => controlledSwarmTicket({ [ROW_ENV]: "13", [TICKET_ENV]: "0" }), /专用验收票/);
  assert.throws(() => controlledSwarmTicket({ [ROW_ENV]: "14", [TICKET_ENV]: "4242" }), /14\/15 属于归并冲突入口/);
  const src = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.match(src, /controlledTicket\s*\n?\s*\? \[\{ id: String\(controlledTicket\)/, "受控入口的输入只有验收票");
  assert.ok(src.indexOf("await planAndClaim()") > src.indexOf("controlledTicket"), "planner/claim 只在非受控分支");
});
