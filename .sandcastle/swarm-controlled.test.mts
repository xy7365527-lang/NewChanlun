import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ROW_ENV, issueReceipt, swarmBinding } from "./docker-acceptance.mts";
import { assertSwarmExecutionReady } from "./swarm-route.mts";
import { assertControlledBase } from "./swarm-controlled.mts";

const TICKET = 4242;
const tmp = (p: string) => mkdtempSync(join(tmpdir(), p));
function repo() {
  const dir = tmp("controlled-base-");
  const git = (...a: string[]) => execFileSync("git", ["-C", dir, ...a], { encoding: "utf8", stdio: "pipe" }).trim();
  git("init", "-q", "-b", "main"); git("config", "user.name", "t"); git("config", "user.email", "t@x");
  writeFileSync(join(dir, "a"), "1"); git("add", "."); git("commit", "-qm", "c1");
  const candidate = git("rev-parse", "HEAD");
  return { dir, git, candidate };
}
const admissionFor = (head: string) => ({ row: "13" as const, receipt: { head } as never, consume: () => undefined });
const good = {
  apparmor: "docker-default (enforce)", seccomp: "2", capEff: "0000000000000000", capBnd: "00000000a80425fb",
  roWrite: "Read-only file system", rwWrite: "ok", roChildInRw: "Read-only file system", roFileAppend: "Read-only file system",
  renameMountpoint: "busy", hardlinkAcross: "Invalid cross-device link", symlinkIntoRo: "Read-only file system",
};

test("受控第 13 行基线=receipt.head：HEAD 一致放行；引用不存在、checkout 漂移、验收票分支不基于候选一律在工蜂前拒绝", () => {
  const r = repo();
  assert.equal(assertControlledBase(admissionFor(r.candidate), TICKET, r.dir), r.candidate);
  assert.throws(() => assertControlledBase(admissionFor("1".repeat(40)), TICKET, r.dir), /候选 \w+ 在当前仓库中不存在/);
  assert.throws(() => assertControlledBase({ row: "14", consume: () => undefined } as never, TICKET, r.dir), /缺少 receipt 候选 head/);
  // checkout 前进（HEAD≠receipt.head）
  writeFileSync(join(r.dir, "a"), "2"); r.git("commit", "-qam", "c2");
  assert.throws(() => assertControlledBase(admissionFor(r.candidate), TICKET, r.dir), /当前 checkout HEAD .* 不是被验收候选/);
  // 回到候选，验收票分支已存在：基于候选可复用，不基于候选拒绝
  r.git("checkout", "-q", r.candidate);
  r.git("branch", `sandcastle/issue-${TICKET}`, "HEAD");
  assert.equal(assertControlledBase(admissionFor(r.candidate), TICKET, r.dir), r.candidate);
  r.git("checkout", "-q", "--orphan", "other"); r.git("commit", "-qm", "unrelated", "--allow-empty");
  r.git("branch", "-f", `sandcastle/issue-${TICKET}`, "HEAD");
  r.git("checkout", "-q", r.candidate);
  assert.throws(() => assertControlledBase(admissionFor(r.candidate), TICKET, r.dir), /不基于候选/);
});

test("入口行为：候选在当前仓库不存在时，在探针、占用验收行之前就 FAIL-loud（不烧验收行）", () => {
  const r = repo(); const acc = tmp("controlled-acc-");
  const identity = { head: "2".repeat(40), dirty: "" }; // receipt 绑定的候选不在 r.dir 里
  issueReceipt({ dir: acc, image: "img", approval: "ok", swarmTicket: TICKET, identity });
  let probed = false; let baseRef = "main";
  assert.throws(() => assertSwarmExecutionReady("img", {
    rows: ["13"], binding: swarmBinding(TICKET), acceptance: { dir: acc, env: { [ROW_ENV]: "13" }, identity },
    verify: (a) => { baseRef = assertControlledBase(a, TICKET, r.dir); },
    probe: () => { probed = true; return good; },
  }), /候选 \w+ 在当前仓库中不存在，拒绝启动工蜂/);
  assert.equal(probed, false);
  assert.equal(baseRef, "main");
  assert.deepEqual(readdirSync(acc).filter((f) => f.startsWith("used-")), [], "验收行未被占用");
  // 候选存在且 HEAD 一致：通过，并把基线交给调用方、占用第 13 行
  const ok = { head: r.candidate, dirty: "" };
  const acc2 = tmp("controlled-acc-");
  issueReceipt({ dir: acc2, image: "img", approval: "ok", swarmTicket: TICKET, identity: ok });
  assertSwarmExecutionReady("img", {
    rows: ["13"], binding: swarmBinding(TICKET), acceptance: { dir: acc2, env: { [ROW_ENV]: "13" }, identity: ok },
    verify: (a) => { baseRef = assertControlledBase(a, TICKET, r.dir); }, probe: () => good,
  });
  assert.equal(baseRef, r.candidate);
  assert.equal(readdirSync(acc2).filter((f) => f.startsWith("used-")).length, 1);
});

test("main-swarm 受控分支用 receipt.head 建工蜂 worktree；普通 swarm 仍是 main", () => {
  const src = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.match(src, /let baseRef = "main"/);
  assert.match(src, /baseRef = assertControlledBase\(admission, controlledTicket\)/);
  assert.match(src, /baseBranch: baseRef/);
  assert.equal(src.match(/baseBranch: "main"/g)?.length, 1, "仅 planner（只在普通分支）保留 main");
  assert.ok(existsSync(new URL("./swarm-controlled.mts", import.meta.url)));
});
