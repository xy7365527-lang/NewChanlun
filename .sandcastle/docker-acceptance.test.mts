import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ACCEPTANCE_ROWS, DIR_ENV, ROW_ENV, acceptedFor, checkAdmission, checkControlledAdmission, contractDigest,
  issueReceipt, recordAcceptance, retireReceipt, type AcceptanceRow,
} from "./docker-acceptance.mts";

const HEAD = "a".repeat(40);
const IMAGE = "sandcastle:newchanlun";
const clean = { head: HEAD, dirty: "" };
const dir = () => mkdtempSync(join(tmpdir(), "docker-acceptance-"));
const env = (d: string, row?: string) => ({ [DIR_ENV]: d, ...(row ? { [ROW_ENV]: row } : {}) }) as NodeJS.ProcessEnv;
const ctx = (d: string, row?: string, identity = clean, now?: Date) => ({ env: env(d, row), dir: d, identity, now });
const issue = (d: string, extra: Partial<Parameters<typeof issueReceipt>[0]> = {}) =>
  issueReceipt({ dir: d, image: IMAGE, approval: "turn 01a17310 用户批准方案B", identity: clean, ...extra });

test("无 receipt、无验收行、不精确代码一律 FAIL-loud", () => {
  const d = dir();
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /receipt 缺失/);
  issue(d);
  assert.throws(() => checkAdmission(IMAGE, ctx(d)), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "16")), /未声明受控验收行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", { head: HEAD, dirty: " M .sandcastle/x.mts" })), /代码不精确/);
  assert.throws(() => issueReceipt({ dir: dir(), image: IMAGE, approval: "x", identity: { head: HEAD, dirty: "?? .sandcastle/new.mts" } }), /未提交改动/);
  assert.throws(() => issueReceipt({ dir: dir(), image: IMAGE, approval: " ", identity: clean }), /批准引用/);
});

test("receipt 绑定 head、合同摘要、镜像、批准范围与有效期，漂移即拒", () => {
  const d = dir(); const r = issue(d, { rows: ["13"] });
  assert.equal(r.contract, contractDigest());
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", { head: "b".repeat(40), dirty: "" })), /head 漂移/);
  assert.throws(() => checkAdmission("other:image", ctx(d, "13")), /镜像不一致/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "14")), /批准范围不含第 14 行/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13", clean, new Date(Date.now() + 13 * 3_600_000))), /已过期/);
  writeFileSync(join(d, "receipt.json"), JSON.stringify({ ...r, contract: "0".repeat(64) }));
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /隔离合同已变化/);
  writeFileSync(join(d, "receipt.json"), "{not json");
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /无法解析/);
});

test("每个验收行一次性：占用后重复、并发占用都拒绝；占用前核对不消耗", () => {
  const d = dir(); issue(d);
  const a = checkAdmission(IMAGE, ctx(d, "13")); const b = checkAdmission(IMAGE, ctx(d, "13"));
  a.consume();
  assert.throws(() => b.consume(), /并发占用/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, "13")), /第 13 行已使用/);
  checkAdmission(IMAGE, ctx(d, "14")); // 其他行不受影响
});

test("同一 checkout 不允许重复签发，须先退役保留历史", () => {
  const d = dir(); issue(d);
  assert.throws(() => issue(d), /EEXIST/);
  const moved = retireReceipt(d);
  assert.ok(moved && existsSync(moved) && !existsSync(join(d, "receipt.json")));
  issue(d);
});

test("登记验收：三行都真实占用且证据存在才成立；之后凭记录放行，drift 即失效", () => {
  const d = dir(); issue(d);
  const evidence = Object.fromEntries(ACCEPTANCE_ROWS.map((r) => { const p = join(d, `ev-${r}.txt`); writeFileSync(p, r); return [r, p]; })) as Record<AcceptanceRow, string>;
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, approval: "ok", evidence, identity: clean }), /第 13 行尚未经受控入口真实运行/);
  for (const r of ACCEPTANCE_ROWS) checkControlledAdmission(IMAGE, ctx(d, r)).consume();
  assert.throws(() => recordAcceptance({ dir: d, image: IMAGE, approval: "ok", evidence: { ...evidence, "15": join(d, "missing") }, identity: clean }), /缺少证据文件/);
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
  assert.throws(() => checkAdmission(IMAGE, ctx(d)), /未声明受控验收行/);
  const rec = recordAcceptance({ dir: d, image: IMAGE, approval: "ok", evidence, identity: clean });
  assert.equal(rec.head, HEAD);
  checkAdmission(IMAGE, ctx(d)).consume(); // 无需验收行，可重复放行
  checkAdmission(IMAGE, ctx(d));
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, { head: "c".repeat(40), dirty: "" })), /尚无精确验收 receipt/);
  assert.throws(() => checkAdmission("other:image", ctx(d)), /尚无精确验收 receipt/);
  assert.throws(() => checkAdmission(IMAGE, ctx(d, undefined, { head: HEAD, dirty: "?? .sandcastle/x" })), /尚无精确验收 receipt/);
  const saved = JSON.parse(readFileSync(join(d, "accepted.json"), "utf8"));
  writeFileSync(join(d, "accepted.json"), JSON.stringify({ ...saved, contract: "0".repeat(64) }));
  assert.equal(acceptedFor(IMAGE, ctx(d)), null);
  retireReceipt(d);
  assert.equal(existsSync(join(d, "accepted.json")), false);
});
