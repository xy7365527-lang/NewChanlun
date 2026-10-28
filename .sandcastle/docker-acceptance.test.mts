import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir, userInfo } from "node:os";
import { join } from "node:path";
import {
  ACCEPTANCE_ROWS, MAX_CHECKPOINTS, dockerImageId, parseImageInspect, MAX_RESUMES, ROW_ENV, acceptedFor, checkAdmission, contractDigest, issueReceipt, mergeBinding,
  mergeInstance, modelFamily, parseCrossFamilyReview, parseMergeBinding, recordAcceptance, retireReceipt, swarmBinding, type AcceptanceRow,
} from "./docker-acceptance.mts";
import { accountHome, acceptanceRoot, assertHomeNotRedirected, assertMountsExcludeAcceptance, controlledDir } from "./acceptance-dir.mts";
import { controlledSwarmTicket, TICKET_ENV } from "./swarm-controlled.mts";

const HEAD = "a".repeat(40);
const IMAGE = "sandcastle:newchanlun";
const IMG_ID = "sha256:" + "1".repeat(64);
const imageIdOf = () => IMG_ID;
const SWARM_TICKET = 4242;
const clean = { head: HEAD, dirty: "" };
const dir = () => mkdtempSync(join(tmpdir(), "docker-acceptance-")); // mkdtemp：0700、属于当前用户
const env = (row?: string): NodeJS.ProcessEnv => (row ? { [ROW_ENV]: row } : {});
const ctx = (d: string, row?: string, binding?: string, extra: Record<string, unknown> = {}) => ({ dir: d, env: env(row), identity: clean, binding, imageIdOf, ...extra });
const issue = (d: string, extra: Record<string, unknown> = {}) =>
  issueReceipt({ imageIdOf, dir: d, image: IMAGE, approval: "turn 01a17310 用户批准方案B", swarmTicket: SWARM_TICKET, identity: clean, ...extra });
const B13 = swarmBinding(SWARM_TICKET);
const cp = (o: Partial<{ runId: string; base: string; instance: string; head: string; incoming: string }> = {}) =>
  mergeBinding({ runId: "run-a", base: "b".repeat(40), instance: "inst-1", head: "c".repeat(40), incoming: "d".repeat(40), ...o });
const B14 = cp();
const B15 = cp({ runId: "run-b", incoming: "e".repeat(40) });
const BINDINGS: Record<AcceptanceRow, string> = { "13": B13, "14": B14, "15": B15 };

function review(d: string, model = "claude-opus-5-5", head = HEAD, first = "VERDICT: PASS") {
  const p = join(d, `review-${model.replace(/\W/g, "_")}-${head.slice(0, 4)}.txt`);
  writeFileSync(p, `${first}\nHEAD: ${head}\nREVIEWER_MODEL: ${model}\n\n正文可以自由书写。\n`);
  return p;
}
function evidence(d: string) {
  return Object.fromEntries(ACCEPTANCE_ROWS.map((r) => { const p = join(d, `ev-${r}.txt`); writeFileSync(p, r); return [r, p]; })) as Record<AcceptanceRow, string>;
}
function runAll(d: string) { for (const r of ACCEPTANCE_ROWS) checkAdmission(IMAGE, ctx(d, r, BINDINGS[r])).consume(); }
function accepted(d: string, extra: Record<string, unknown> = {}) {
  issue(d, extra); runAll(d);
  return recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
}
const read = (d: string, f: string) => JSON.parse(readFileSync(join(d, f), "utf8"));
const write = (d: string, f: string, v: unknown) => { writeFileSync(join(d, f), JSON.stringify(v)); chmodSync(join(d, f), 0o600); };
const usedFile = (d: string, nonce: string, row: string) => `used-${nonce}-${row}.json`;

test("无 receipt、无验收行、不精确代码一律 FAIL-loud", () => {
  const d = dir();
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 缺失/);
  issue(d);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, B13)), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "16", B13)), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /缺少运行绑定/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { identity: { head: HEAD, dirty: " M .sandcastle/x.mts" } })), /代码不精确/);
  assert.throws(() => issueReceipt({ imageIdOf, dir: dir(), image: IMAGE, approval: "x", swarmTicket: 1, identity: { head: HEAD, dirty: "?? .sandcastle/new.mts" } }), /未提交改动/);
  assert.throws(() => issueReceipt({ imageIdOf, dir: dir(), image: IMAGE, approval: " ", swarmTicket: 1, identity: clean }), /批准引用/);
  assert.throws(() => issueReceipt({ imageIdOf, dir: dir(), image: IMAGE, approval: "x", identity: clean }), /专用验收票/);
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
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", "merge-conflict:v1:garbage")), /binding/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", "任意非空字符串")), /binding/);
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

test("生产路径不读 $HOME：账号主目录取自 passwd；HOME 被重定向到伪造完整目录时 FAIL-loud，不放行", () => {
  const forgedHome = dir();
  const forgedDir = join(forgedHome, ".sandcastle-host", "docker-acceptance-1742");
  accepted(forgedDir); // 伪造目录里有完整记录（含伪造的批准引用）
  assert.equal(acceptanceRoot(), join(userInfo().homedir, ".sandcastle-host", "docker-acceptance-1742"));
  assert.equal(accountHome(), userInfo().homedir);
  assert.throws(() => assertHomeNotRedirected({ HOME: forgedHome }, accountHome()), /不一致/);
  assertHomeNotRedirected({ HOME: accountHome() }, accountHome());
  assertHomeNotRedirected({}, accountHome());
  const prev = process.env.HOME;
  process.env.HOME = forgedHome;
  try {
    assert.equal(acceptedFor(IMAGE, { identity: clean }), null);
    assert.throws(() => checkAdmission(IMAGE, { identity: clean, env: { [ROW_ENV]: "13" }, binding: B13 }), /尚无精确验收 receipt.*不一致/);
    assert.throws(() => checkAdmission(IMAGE, { identity: clean, env: {} }), /尚无精确验收 receipt/);
  } finally { if (prev === undefined) delete process.env.HOME; else process.env.HOME = prev; }
});

test("容器挂载不得暴露验收目录或其祖先（如整个主目录）", () => {
  const home = dir(); const root = acceptanceRoot(home);
  mkdirSync(root, { recursive: true });
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: home }], root), /暴露/);
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: root }], root), /暴露/);
  assert.throws(() => assertMountsExcludeAcceptance([{ hostPath: join(root, "sub") }], root), /暴露/);
  assertMountsExcludeAcceptance([{ hostPath: join(home, "elsewhere") }], root);
});

test("伪造 accepted.json 无效：无 receipt / 无 used / nonce、批准、有效期、快照被替换都放行失败", () => {
  const d = dir(); const r = issue(d);
  const forged = { schema: 2, ticket: 1742, nonce: r.nonce, head: HEAD, contract: contractDigest(), image: IMAGE, imageId: IMG_ID, approval: r.approval, issuedAt: r.issuedAt, expiresAt: r.expiresAt, acceptedAt: r.issuedAt, evidence: evidence(d), crossFamily: { file: review(d), head: HEAD, model: "claude-opus-5-5" }, bindings: { "13": [B13], "14": [B14], "15": [B15] } };
  write(d, "accepted.json", forged);
  assert.equal(acceptedFor(IMAGE, ctx(d)), null, "没有 used 记录");
  assert.throws(() => checkAdmission(IMAGE, ctx(d)), /未声明受控验收行/);
  const d2 = dir(); write(d2, "accepted.json", forged);
  assert.equal(acceptedFor(IMAGE, ctx(d2)), null, "没有 receipt");
  const d3 = dir(); accepted(d3);
  assert.ok(acceptedFor(IMAGE, ctx(d3)));
  const good = read(d3, "accepted.json");
  for (const patch of [{ nonce: "other" }, { approval: "别的批准" }, { expiresAt: new Date(Date.now() + 9e9).toISOString() }, { head: "b".repeat(40) }, { image: "x:y" }, { imageId: "sha256:" + "2".repeat(64) }, { contract: "0".repeat(64) }, { bindings: { ...good.bindings, "13": ["swarm-ticket:1237"] } }, { bindings: { ...good.bindings, "14": [cp({ runId: "forged" })] } }, { crossFamily: { ...good.crossFamily, head: "f".repeat(40) } }]) {
    write(d3, "accepted.json", { ...good, ...patch });
    assert.equal(acceptedFor(IMAGE, ctx(d3)), null, JSON.stringify(patch).slice(0, 80));
  }
});

test("登记验收：拒绝过期 receipt、批准引用被替换、缺行、缺证据；记录保存有效期，到期自动失效", () => {
  const d = dir(); const r = issue(d); runAll(d);
  const ev = evidence(d); const rv = review(d);
  assert.throws(() => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: ev, crossFamilyReview: rv, identity: clean, now: new Date(Date.parse(r.expiresAt) + 1000) }), /已过期/);
  assert.throws(() => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, approval: "different-approval", evidence: ev, crossFamilyReview: rv, identity: clean }), /批准引用与 receipt 不一致/);
  assert.throws(() => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: { ...ev, "15": join(d, "missing") }, crossFamilyReview: rv, identity: clean }), /缺少证据文件/);
  const rec = recordAcceptance({ imageIdOf, dir: d, image: IMAGE, approval: r.approval, evidence: ev, crossFamilyReview: rv, identity: clean });
  assert.equal(rec.expiresAt, r.expiresAt); assert.equal(rec.approval, r.approval);
  assert.ok(checkAdmission(IMAGE, ctx(d)).accepted);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, undefined, { now: new Date(Date.parse(r.expiresAt) + 1000) })), /未声明受控验收行/);
  assert.throws(() => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: ev, crossFamilyReview: rv, identity: clean }), /EEXIST|file already exists/);
});

test("used 标记崩溃安全且 binding 语义受校验：半写/损坏/不一致/非法 binding 不算事实", () => {
  const d = dir(); const r = issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  const f13 = usedFile(d, r.nonce, "13"); const f14 = usedFile(d, r.nonce, "14");
  assert.deepEqual(readdirSync(d).filter((f) => f.includes(".tmp-")), [], "发布后不留临时文件");
  const good13 = read(d, f13);
  writeFileSync(join(d, `used-${r.nonce}-14.json.tmp-123-x`), '{"sche'); // 崩溃在发布前只留 .tmp
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume();
  const good14 = read(d, f14);
  const attempt = () => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  const bad13: unknown[] = ['{"sche', "{}", { ...good13, nonce: "other" }, { ...good13, head: "b".repeat(40) }, { ...good13, row: "14" }, { ...good13, binding: "" }, { ...good13, at: "nope" },
    { ...good13, binding: "wrong-nonempty" }, { ...good13, binding: swarmBinding(1237) }];
  for (const bad of bad13) {
    writeFileSync(join(d, f13), typeof bad === "string" ? bad : JSON.stringify(bad)); chmodSync(join(d, f13), 0o600);
    assert.throws(attempt, /used 记录无效|binding/, String(JSON.stringify(bad)).slice(0, 40));
    assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /used 记录无效|binding/);
  }
  writeFileSync(join(d, f13), JSON.stringify(good13)); chmodSync(join(d, f13), 0o600);
  for (const binding of ["wrong-nonempty", "merge-conflict:v1:" + Buffer.from("{}").toString("base64url"), "swarm-ticket:4242"]) {
    write(d, f14, { ...good14, binding });
    assert.throws(attempt, /used 记录无效/, binding.slice(0, 30));
  }
  write(d, f14, good14);
  assert.ok(attempt());
});

test("used 绑定在登记后被改为另一合法 binding：与登记快照不符，放行失败", () => {
  const d = dir(); const r = issue(d); runAll(d);
  recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  assert.ok(acceptedFor(IMAGE, ctx(d)));
  const f14 = usedFile(d, r.nonce, "14");
  write(d, f14, { ...read(d, f14), binding: cp({ runId: "other-run" }) });
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
});

test("每个验收行一次性：占用后新运行/并发占用都拒绝；占用前核对不消耗", () => {
  const d = dir(); issue(d);
  const a = checkAdmission(IMAGE, ctx(d, "14", B14)); const b = checkAdmission(IMAGE, ctx(d, "14", B14));
  a.consume();
  assert.throws(() => b.consume(), /并发占用/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", B15)), /另一运行\/检查点使用，新运行不得复用/);
  checkAdmission(IMAGE, ctx(d, "15", B15)); // 其他行不受影响
});

test("第 15 行：同一 run 实例+检查点被 SIGKILL 后重入是恢复，登记可完成；别的 run、别的基线、同 runId 重置后的新实例都被拒", () => {
  const d = dir(); issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume(); // 首个进程占用后被 SIGKILL
  const again = checkAdmission(IMAGE, ctx(d, "15", B15)); // 重入：同一 run 实例、同一检查点
  assert.equal(again.resumed, true);
  again.consume();
  assert.equal(readdirSync(d).filter((f) => f.startsWith("resume-")).length, 1);
  assert.ok(recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean }));

  const d2 = dir(); issue(d2); checkAdmission(IMAGE, ctx(d2, "15", B15)).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", cp({ runId: "run-c", incoming: "e".repeat(40) }))), /新运行不得复用/, "runId 不同");
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", cp({ runId: "run-b", base: "9".repeat(40), incoming: "e".repeat(40) }))), /新运行不得复用/, "基线不同");
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", cp({ runId: "run-b", instance: "inst-2", incoming: "e".repeat(40) }))), /新运行不得复用/, "同 runId 重置后从头重跑=新实例，不能冒充恢复");
  for (let i = 0; i < MAX_RESUMES; i++) checkAdmission(IMAGE, ctx(d2, "15", B15)).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d2, "15", B15)), /恢复次数已达上限/);
});

test("同一归并 run 实例的第二个冲突检查点：单独通过门禁、单独绑定、单独留痕；别的 run 的检查点混入则登记无效", () => {
  const d = dir(); const r = issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume();
  const adm = checkAdmission(IMAGE, ctx(d, "14", B14)); adm.consume();
  const adm2 = checkAdmission(IMAGE, ctx(d, "14", cp({ head: "8".repeat(40), incoming: "7".repeat(40) })));
  assert.equal(adm2.resumed, undefined, "新检查点不是恢复");
  adm2.consume();
  assert.equal(readdirSync(d).filter((f) => f.startsWith(`cp-${r.nonce}-14-`)).length, 1);
  assert.equal(checkAdmission(IMAGE, ctx(d, "14", cp({ head: "8".repeat(40), incoming: "7".repeat(40) }))).resumed, true, "同一第二检查点重入=恢复");
  const rec = recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  assert.equal(rec.bindings["14"].length, 2);
  assert.ok(acceptedFor(IMAGE, ctx(d)));
  // 注入别的 run 的检查点记录 → 与行锚点不同 run，快照核对失败
  const foreign = cp({ runId: "foreign" });
  const marker = { ...read(d, usedFile(d, r.nonce, "14")), binding: foreign };
  const name = `cp-${r.nonce}-14-${hash16(foreign)}.json`;
  write(d, name, marker);
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
});
const hash16 = (binding: string) => createHash("sha256").update(binding).digest("hex").slice(0, 16);

test("检查点数量有上限；第 13 行一次性，不允许同票重跑冒充恢复", () => {
  const d = dir(); issue(d);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /一次性/);
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  for (let i = 0; i < MAX_CHECKPOINTS - 1; i++) checkAdmission(IMAGE, ctx(d, "14", cp({ incoming: i.toString(16).padStart(40, "1") }))).consume();
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14", cp({ incoming: "2".repeat(40) }))), /检查点数已达上限/);
});

test("mergeBinding 可解析、规范编码；mergeInstance 在宿主证据目录原子创建并绑定 run/基线", () => {
  const b = cp();
  assert.deepEqual(parseMergeBinding(b), { runId: "run-a", base: "b".repeat(40), instance: "inst-1", head: "c".repeat(40), incoming: "d".repeat(40) });
  assert.throws(() => parseMergeBinding(b + "x"), /binding/);
  assert.throws(() => parseMergeBinding("swarm-ticket:1"), /归并检查点/);
  const ev = dir();
  const id = mergeInstance(ev, "run-a", "b".repeat(40));
  assert.equal(mergeInstance(ev, "run-a", "b".repeat(40)), id, "重入读回同一实例");
  assert.throws(() => mergeInstance(ev, "run-z", "b".repeat(40)), /不符/);
  const ev2 = dir();
  assert.notEqual(mergeInstance(ev2, "run-a", "b".repeat(40)), id, "新证据目录（重置后重跑）得到新实例");
});

test("同一 checkout 不允许重复签发，须先退役保留历史；退役同时作废 accepted", () => {
  const d = dir(); accepted(d);
  assert.throws(() => issue(d), /EEXIST|file already exists/);
  const moved = retireReceipt({ dir: d });
  assert.ok(moved && existsSync(moved) && !existsSync(join(d, "receipt.json")) && !existsSync(join(d, "accepted.json")));
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
  issue(d);
});

test("外部异家族独审：首个非空行必须是 VERDICT: PASS，HEAD 绑定 receipt，重复行/正文 PASS/同家族/别名/未知名称都拒绝", () => {
  const d = dir(); issue(d); runAll(d); const ev = evidence(d);
  const attempt = (file: string) => recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: ev, crossFamilyReview: file, identity: clean });
  assert.throws(() => attempt(review(d, "claude-opus-5-5", "f".repeat(40))), /不是被验收的候选/, "HEAD 不等于 receipt.head");
  assert.throws(() => attempt(review(d, "claude-opus-5-5", "0".repeat(40))), /不是被验收的候选/);
  assert.throws(() => attempt(review(d, "gpt-6.1-sol")), /同属 openai/);
  assert.throws(() => attempt(review(d, "mf/gpt-5.5")), /同属 openai/);
  assert.throws(() => attempt(review(d, "codex-auto-review")), /同属 openai/);
  assert.throws(() => attempt(review(d, "GPT_6_astra")), /同属 openai/);
  assert.throws(() => attempt(review(d, "auto-free")), /路由别名/);
  assert.throws(() => attempt(review(d, "openrouter-free")), /路由别名/);
  assert.throws(() => attempt(review(d, "mystery-model-9")), /不在已知家族表/);
  assert.throws(() => attempt(review(d, "claude-opus-5-5", HEAD, "VERDICT: BLOCK")), /首个非空行/);
  assert.throws(() => attempt(join(d, "none.txt")), /缺少外部异家族独审报告/);
  const trick = join(d, "trick.txt");
  writeFileSync(trick, `BLOCK @ ${HEAD}\nVERDICT: BLOCK\n示例格式：\nVERDICT: PASS\nHEAD: ${HEAD}\nREVIEWER_MODEL: claude-opus-5-5\n`);
  assert.throws(() => attempt(trick), /首个非空行/, "首行 BLOCK、正文含 PASS 示例行不得登记");
  writeFileSync(trick, `VERDICT: PASS\nVERDICT: BLOCK\nHEAD: ${HEAD}\nREVIEWER_MODEL: claude-opus-5-5\n`);
  assert.throws(() => attempt(trick), /多个 VERDICT/);
  writeFileSync(trick, `VERDICT: PASS\nHEAD: ${HEAD}\nHEAD: ${"f".repeat(40)}\nREVIEWER_MODEL: claude-opus-5-5\n`);
  assert.throws(() => attempt(trick), /HEAD 必须恰好出现一次/);
  writeFileSync(trick, `VERDICT: PASS\nHEAD: ${HEAD}\nREVIEWER_MODEL: claude-opus-5-5\nREVIEWER_MODEL: gpt-6-astra\n`);
  assert.throws(() => attempt(trick), /REVIEWER_MODEL 必须恰好出现一次/);
  writeFileSync(trick, `\n\nVERDICT: PASS\nHEAD: ${HEAD}\nREVIEWER_MODEL: claude-opus-5-5\n`);
  const rec = attempt(trick);
  assert.equal(rec.crossFamily.model, "claude-opus-5-5"); assert.equal(rec.crossFamily.head, HEAD);
});

test("登记后每次放行都重读外部独审报告：篡改 HEAD/首行/模型即失效", () => {
  const d = dir(); accepted(d);
  const file = read(d, "accepted.json").crossFamily.file as string;
  assert.ok(acceptedFor(IMAGE, ctx(d)));
  const original = readFileSync(file, "utf8");
  for (const tampered of [original.replace(HEAD, "f".repeat(40)), original.replace("VERDICT: PASS", "VERDICT: BLOCK"), original.replace("claude-opus-5-5", "gpt-6-astra"), original.replace("claude-opus-5-5", "auto-free"), "VERDICT: PASS\n"]) {
    writeFileSync(file, tampered);
    assert.equal(acceptedFor(IMAGE, ctx(d)), null, tampered.slice(0, 40));
  }
  writeFileSync(file, original);
  assert.ok(acceptedFor(IMAGE, ctx(d)));
});

test("模型家族表", () => {
  for (const [name, fam] of [["gpt-6-astra", "openai"], ["codex-auto-review", "openai"], ["GPT_6_astra", "openai"], ["o3-mini", "openai"], ["mf/gpt-5.5", "openai"], ["claude-opus-5-5", "anthropic"], ["yd/claude-sonnet-5", "anthropic"], ["gemini-4-argon-high", "google"], ["kimi-k3", "moonshot"]] as const) {
    assert.equal(modelFamily(name), fam, name);
  }
  for (const bad of ["auto-free", "openrouter-free", "beam-free", "local-llm", "", "who-knows", "auto-review"]) {
    assert.throws(() => modelFamily(bad), /路由别名|不在已知家族表|为空/, bad);
  }
  assert.throws(() => parseCrossFamilyReview(`VERDICT: PASS\nHEAD: ${HEAD}\nREVIEWER_MODEL: gpt-5.5\n`, HEAD), /同属/);
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
  const merge = readFileSync(new URL("./swarm-merge.mts", import.meta.url), "utf8");
  assert.doesNotMatch(merge, /conflictExecutionReady/, "冲突门禁不再是 run 级布尔");
  assert.match(merge, /readiedCheckpoints/);
});

// —— 镜像 Id 绑定（D6 前凭据暴露闭合）——
test("receipt 绑定镜像 Id：tag 被重指/镜像重建后 Id 漂移，受控放行、登记、已登记放行都 FAIL-loud", () => {
  const d = dir(); const r = issue(d);
  assert.equal(r.imageId, IMG_ID); assert.equal(read(d, "receipt.json").imageId, IMG_ID);
  const drifted = () => "sha256:" + "9".repeat(64);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { imageIdOf: drifted })), /镜像 Id 漂移/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { imageIdOf: () => { throw new Error("docker 不可用"); } })), /镜像 Id 无法确认.*docker 不可用/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13, { imageIdOf: () => "latest" })), /Id 格式无效/);
  assert.deepEqual(readdirSync(d).filter((f) => f.startsWith("used-")), [], "漂移不烧验收行");
  runAll(d);
  assert.throws(() => recordAcceptance({ imageIdOf: drifted, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean }), /镜像 Id 不符/);
  recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  assert.ok(acceptedFor(IMAGE, ctx(d)));
  assert.equal(acceptedFor(IMAGE, ctx(d, undefined, undefined, { imageIdOf: drifted })), null, "已登记放行同样绑定 Id");
});

test("receipt/accepted 缺 imageId 或 Id 不是 sha256：解析失败", () => {
  const d = dir(); issue(d);
  const rec = read(d, "receipt.json"); delete rec.imageId; write(d, "receipt.json", rec);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 无效.*imageId/);
  write(d, "receipt.json", { ...rec, imageId: "sandcastle:newchanlun" });
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", B13)), /receipt 无效.*imageId/);
});

test("合同摘要含容器 env 白名单与 Id 绑定：改白名单即改摘要", async () => {
  const iso = await import("./docker-isolation.mts");
  assert.deepEqual([...iso.CONTAINER_ENV_ALLOW], ["CLIPROXY_BASE_URL", "CLIPROXY_LOCAL_KEY", "SANDCASTLE_MERGE_INSTANCE", "SANDCASTLE_MERGE_RUN"]);
  const canonical = (allow: readonly string[]) => createHash("sha256").update(JSON.stringify({
    id: "1742-docker-plan-b-v3", protectedSubpaths: iso.PROTECTED_SUBPATHS, innerSandbox: iso.DOCKER_INNER_SANDBOX,
    capMask: iso.DEFAULT_CAP_MASK.toString(16), securityOpts: [], containerEnvAllow: allow, imageIdBound: true, probe: iso.PROBE_SCRIPT,
  })).digest("hex");
  assert.equal(contractDigest(), canonical(iso.CONTAINER_ENV_ALLOW), "摘要正文含白名单与 Id 绑定");
  assert.notEqual(contractDigest(), canonical([...iso.CONTAINER_ENV_ALLOW, "KIMI_API_KEY"]), "白名单变化即摘要变化");
});

test("dockerImageId 默认解析：真实 docker inspect 的 Id；镜像不存在时抛错而不是回退 tag", { skip: process.env.SANDCASTLE_REAL_DOCKER_TESTS !== "1" ? "真实 docker 测试默认关闭（设 SANDCASTLE_REAL_DOCKER_TESTS=1 开启）" : realImage("alpine:3.22") ? false : "本机无 alpine:3.22" }, () => {
  assert.match(dockerImageId("alpine:3.22"), /^sha256:[0-9a-f]{64}$/);
  assert.throws(() => dockerImageId("sandcastle-no-such-image:never"), /无法解析镜像/);
});
function realImage(name: string): boolean { try { execFileSync("docker", ["image", "inspect", name, "--format", "{{.Id}}"], { stdio: "ignore", timeout: 15_000 }); return true; } catch { return false; } }

test("parseImageInspect：Id 必须是 sha256；镜像自带非空凭据类 ENV 拒绝，空占位键与普通键放行", () => {
  const id = "sha256:" + "a".repeat(64);
  assert.equal(parseImageInspect(`${id}\t${JSON.stringify(["PATH=/bin", "KIMI_API_KEY=", "GH_TOKEN=", "GH_REPO=a/b"])}`, "img"), id);
  assert.throws(() => parseImageInspect(`${id}\t${JSON.stringify(["DEEPSEEK_API_KEY=abc"])}`, "img"), (e: Error) => /DEEPSEEK_API_KEY/.test(e.message) && !/abc/.test(e.message));
  assert.throws(() => parseImageInspect(`latest\t[]`, "img"), /Id 格式无效/);
  assert.throws(() => parseImageInspect(`${id}\tnot-json`, "img"), /ENV 无法解析/);
});

test("Admission.imageId：受控首次/恢复/新检查点与已登记放行都返回准入核对过的 Id", () => {
  const d = dir(); issue(d);
  assert.equal(checkAdmission(IMAGE, ctx(d, "13", B13)).imageId, IMG_ID);
  checkAdmission(IMAGE, ctx(d, "13", B13)).consume();
  assert.equal(checkAdmission(IMAGE, ctx(d, "14", B14)).imageId, IMG_ID);
  checkAdmission(IMAGE, ctx(d, "14", B14)).consume();
  assert.equal(checkAdmission(IMAGE, ctx(d, "14", B14)).imageId, IMG_ID, "恢复");
  assert.equal(checkAdmission(IMAGE, ctx(d, "14", cp({ head: "8".repeat(40), incoming: "7".repeat(40) }))).imageId, IMG_ID, "新检查点");
  checkAdmission(IMAGE, ctx(d, "15", B15)).consume();
  recordAcceptance({ imageIdOf, dir: d, image: IMAGE, evidence: evidence(d), crossFamilyReview: review(d), identity: clean });
  const adm = checkAdmission(IMAGE, ctx(d)); assert.ok(adm.accepted); assert.equal(adm.imageId, IMG_ID);
});
