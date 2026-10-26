import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { drainRequested, DrainFrozenError, withLegacyAdmission } from "./drain-control.mts";

const helper = fileURLToPath(new URL("./drain-control.py", import.meta.url));
const temp = () => mkdtempSync(join(tmpdir(), "nc-drain-"));
const cli = (root: string, command: string, ...args: string[]) => JSON.parse(execFileSync("python3", [helper, command, "--root", root, ...args], { encoding: "utf8" }));
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

function freeze(root: string): Promise<number | null> {
  const child = spawn("python3", [helper, "freeze", "--root", root], { stdio: "ignore" });
  return new Promise((resolve, reject) => { child.on("error", reject); child.on("exit", resolve); });
}

test("无标记时旧调用继续，重复freeze/resume保持同代次", async () => {
  const root = temp();
  assert.equal(drainRequested(root), false);
  assert.equal(await withLegacyAdmission(() => 7, root), 7);
  assert.deepEqual(cli(root, "freeze"), { v: 1, mode: "draining", generation: 1 });
  assert.equal(drainRequested(root), true);
  assert.equal(cli(root, "freeze").generation, 1);
  await assert.rejects(withLegacyAdmission(() => assert.fail("不能派发"), root), DrainFrozenError);
  assert.throws(() => cli(root, "resume", "--generation", "0"));
  assert.equal(drainRequested(root), true);
  assert.equal(cli(root, "resume", "--generation", "1").mode, "open");
  assert.equal(cli(root, "resume", "--generation", "1").generation, 1);
});

test("freeze和选择到首次副作用共用锁，已admit的工蜂不被终止", async () => {
  const root = temp();
  let begin!: () => void;
  const started = new Promise<void>((resolve) => { begin = resolve; });
  let finish!: () => void;
  const release = new Promise<void>((resolve) => { finish = resolve; });
  const events: string[] = [];
  const admitted = withLegacyAdmission(async () => {
    events.push("selected"); begin(); await release; events.push("claimed");
  }, root);
  await started;
  let frozen = false;
  const freezing = freeze(root).then((code) => { assert.equal(code, 0); frozen = true; });
  await delay(80);
  assert.equal(frozen, false);
  finish(); await admitted; await freezing;
  assert.deepEqual(events, ["selected", "claimed"]);
  await assert.rejects(withLegacyAdmission(() => events.push("second"), root), DrainFrozenError);
  events.push("admitted-worker-completed");
  assert.equal(events.at(-1), "admitted-worker-completed");
});

test("冻结后所有门禁副作用拒绝，包括heal/requeue和手工claim", async () => {
  const root = temp(); cli(root, "freeze");
  for (const effect of ["claim", "heal", "requeue", "release", "manual-review"]) {
    await assert.rejects(withLegacyAdmission(() => assert.fail(effect), root), DrainFrozenError);
  }
});

test("未知或半写状态失败关闭，恢复不能用旧代次重开新冻结", async () => {
  const root = temp(); writeFileSync(join(root, "state.json"), "{");
  assert.equal(drainRequested(root), true);
  await assert.rejects(withLegacyAdmission(() => assert.fail(), root), DrainFrozenError);
  writeFileSync(join(root, "state.json"), JSON.stringify({ v: 99, mode: "open", generation: 0 }));
  await assert.rejects(withLegacyAdmission(() => assert.fail(), root), DrainFrozenError);
  writeFileSync(join(root, "state.json"), JSON.stringify({ v: 1, mode: "open", generation: 1 }));
  cli(root, "freeze");
  assert.throws(() => cli(root, "resume", "--generation", "1"));
  assert.equal(JSON.parse(readFileSync(join(root, "state.json"), "utf8")).generation, 2);
});

test("控制进程被杀后fcntl锁自动释放，无PID猜测与人工删锁", async () => {
  const root = temp();
  const module = fileURLToPath(new URL("./drain-control.mts", import.meta.url));
  const script = `import { withLegacyAdmission } from ${JSON.stringify(module)}; await withLegacyAdmission(async()=>{console.log('holding');await new Promise(()=>{});},${JSON.stringify(root)});`;
  const child = spawn(process.execPath, ["--import", "tsx", "--input-type=module", "-e", script], { stdio: ["ignore", "pipe", "inherit"] });
  await new Promise<void>((resolve, reject) => { child.stdout.once("data", () => resolve()); child.once("error", reject); });
  child.kill("SIGKILL");
  assert.equal(await freeze(root), 0);
  assert.equal(drainRequested(root), true);
});
