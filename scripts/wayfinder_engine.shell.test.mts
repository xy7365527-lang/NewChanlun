/**
 * wayfinder_engine 壳层测试（#1084 追加验收）：
 *  - 注入一次 GitHub API timeout → 有限重试 + 指数退避后自动恢复并继续（进程不永久停摆）；
 * 跑法：npx tsx --test scripts/wayfinder_engine.shell.test.mts
 *
 * 做法：临时目录里放一个假 gh（首 N 次调用 exit 1 模拟 timeout，之后对 issue list 返回 []），
 * 把该目录前置进 PATH，子进程跑 --once --dry-run 一轮，断言重试日志与一轮收尾都出现。
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { specRulingBody, specBodyDigest, isSpecApproved } from "./wayfinder_engine.mts";
import { MAIN_MODEL } from "../.sandcastle/main-model.mts";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

function fakeGhScript(stateFile: string, failCount: number): string {
  return `#!/usr/bin/env bash
n=$(cat "${stateFile}" 2>/dev/null || echo 0)
n=$((n+1))
echo "$n" > "${stateFile}"
if [ "$n" -le ${failCount} ]; then
  echo "Post \\"https://api.github.com/graphql\\": dial tcp 198.18.8.176:443: i/o timeout" >&2
  exit 1
fi
case "$*" in
  *"issue list"*) echo "[]" ;;
  *) echo "[]" ;;
esac
exit 0
`;
}

function runEngine(failCount: number, maxAttempts: number): string {
  const dir = mkdtempSync(join(tmpdir(), "wfe-shell-test-"));
  const stateFile = join(dir, "state");
  const ghPath = join(dir, "gh");
  writeFileSync(ghPath, fakeGhScript(stateFile, failCount), { mode: 0o755 });
  const env = {
    ...process.env,
    PATH: `${dir}:${process.env.PATH ?? ""}`,
    NO_COLOR: "1",
    CLICOLOR: "0",
    FORCE_COLOR: "0",
    CLICOLOR_FORCE: "0",
    WAYFINDER_GH_MAX_ATTEMPTS: String(maxAttempts),
    WAYFINDER_GH_BASE_BACKOFF_MS: "50",
  };
  try {
    // 重试日志走 console.error（stderr），一轮收尾走 stdout，两路合并后一起断言。
    const r = spawnSync(
      "npx",
      ["tsx", "scripts/wayfinder_engine.mts", "--once", "--dry-run"],
      { cwd: repoRoot, env, encoding: "utf8", timeout: 30_000 },
    );
    if (r.status !== 0) {
      throw new Error(
        `engine --once --dry-run 退出码 ${r.status}\nstdout:\n${r.stdout}\nstderr:\n${r.stderr}`,
      );
    }
    return (r.stdout ?? "") + "\n" + (r.stderr ?? "");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("壳：注入一次 gh timeout → 重试后自动恢复并走完一轮", () => {
  const out = runEngine(1, 3);
  assert.equal(out.includes("gh 查询失败（第 1/3 次"), true);
  assert.equal(out.includes("一轮结束"), true);
  assert.equal(out.includes("open 的 wayfinder:map 共 0 张"), true);
});

test("壳：空队列无 timeout → 正常走完一轮（不退出）", () => {
  const out = runEngine(0, 3);
  assert.equal(out.includes("gh 查询失败"), false);
  assert.equal(out.includes("扫描集（ready-for-agent ∪ wayfinder:impl）0 张"), true);
  assert.equal(out.includes("一轮结束"), true);
});

test("无 node_modules：Node 原生类型剥离可 import engine 与主控模型配置", () => {
  const dir = mkdtempSync(join(tmpdir(), "wfe-zero-deps-"));
  try {
    mkdirSync(join(dir, "scripts"), { recursive: true });
    mkdirSync(join(dir, ".sandcastle"), { recursive: true });
    for (const [source, target] of [
      ["scripts/wayfinder_engine.mts", "scripts/wayfinder_engine.mts"],
      [".sandcastle/cliproxy-env.mts", ".sandcastle/cliproxy-env.mts"],
      [".sandcastle/main-model.mts", ".sandcastle/main-model.mts"],
      [".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.mts"],
      [".sandcastle/scheduler-config.json", ".sandcastle/scheduler-config.json"],
      // #1742 冻结准入门同样零依赖：engine 的 import 闭包含 drain-control，运行时由它拉起同目录的 .py 助手。
      [".sandcastle/drain-control.mts", ".sandcastle/drain-control.mts"],
      [".sandcastle/drain-control.py", ".sandcastle/drain-control.py"],
      [".sandcastle/deepagents-cutover.mts", ".sandcastle/deepagents-cutover.mts"],
      [".sandcastle/queue-policy.json", ".sandcastle/queue-policy.json"],
    ]) copyFileSync(join(repoRoot, source), join(dir, target));
    const result = spawnSync(process.execPath, [
      "--experimental-strip-types", "--input-type=module", "--eval",
      "const m = await import('./scripts/wayfinder_engine.mts'); " +
      `if (m.DEFAULT_SPEC_MODEL !== '${MAIN_MODEL}') throw new Error('wrong model'); ` +
      "console.log(m.DEFAULT_SPEC_MODEL);",
    ], { cwd: dir, encoding: "utf8", timeout: 30_000 });
    assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
    assert.equal(result.stdout.trim(), MAIN_MODEL);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("切流壳：draining 下引擎向假 gh 发出新队列标签，不经过旧准入", () => {
  const root = mkdtempSync(join(tmpdir(), "wfe-cutover-shell-"));
  try {
    const state = join(root, "exec"), drain = join(root, "drain"), log = join(root, "writes");
    mkdirSync(state); mkdirSync(drain);
    writeFileSync(join(drain, "state.json"), JSON.stringify({ v: 1, mode: "draining", generation: 1 }));
    writeFileSync(join(state, "cutover.json"), JSON.stringify({ v: 1, drain_generation: 1, frozen_at: "fixture",
      legacy_snapshot: [], workers_log_length: 0, workers_log_path: "/fixture/workers.jsonl", open_claims: [], remote_jobs_idle: true, remote_freeze_var: "1",
      variable_source: "user-token-readonly", release_label: "deepagents:queue", engine_head: "a".repeat(40),
      legacy_services_unloaded: ["com.newchanlun.sandcastle-claimer", "com.newchanlun.sandcastle-harvest"] }));
    writeFileSync(join(root, "gh"), `#!/usr/bin/env bash
if [ "$1 $2" = "issue view" ]; then
  echo '{"state":"OPEN","labels":[],"assignees":[]}'
elif [ "$1 $2" = "issue edit" ]; then
  printf '%s\\n' "$*" >> "$D6_FIXTURE_LOG"
else
  exit 71
fi
`, { mode: 0o755 });
    const script = `import {execFileSync} from 'node:child_process';
      const m = await import(process.argv[1]);
      const roots = {stateRoot: process.argv[2], drainRoot: process.argv[3]};
      const io = {read: args => execFileSync('gh', args.split(' '), {encoding:'utf8'}),
                  write: args => execFileSync('gh', args.split(' '), {encoding:'utf8'})};
      await m.withEngineAdmission(() => m.releaseTicket(99, io, m.releaseLabel(roots.stateRoot, roots.drainRoot)), roots);`;
    const result = spawnSync(process.execPath, ["--experimental-strip-types", "--input-type=module", "-e", script,
      new URL("./wayfinder_engine.mts", import.meta.url).href, state, drain], {
      cwd: repoRoot, env: { ...process.env, PATH: `${root}:${process.env.PATH}`, D6_FIXTURE_LOG: log },
      encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(readFileSync(log, "utf8").trim(), "issue edit 99 --add-label deepagents:queue");
  } finally { rmSync(root, { recursive: true, force: true }); }
});

async function runLiveLoop(record: "valid" | "missing" | "malformed" | "stale", once = false) {
  const root = mkdtempSync(join(tmpdir(), "wfe-live-loop-"));
  const state = join(root, "exec"), drain = join(root, "drain"), log = join(root, "writes");
  try {
    for (const path of [state, drain, join(root, "scripts"), join(root, ".sandcastle/deepagents-pilot/pilot1742")])
      mkdirSync(path, { recursive: true });
    for (const file of ["scripts/wayfinder_engine.mts", ".sandcastle/cliproxy-env.mts",
      ".sandcastle/main-model.mts", ".sandcastle/scheduler-config.mts", ".sandcastle/scheduler-config.json",
      ".sandcastle/drain-control.mts", ".sandcastle/drain-control.py", ".sandcastle/deepagents-cutover.mts",
      ".sandcastle/queue-policy.json", ".sandcastle/deepagents-pilot/pilot1742/cutover.py"]) {
      const source = readFileSync(join(repoRoot, file), "utf8");
      const content = file === ".sandcastle/drain-control.mts"
        ? source.replace('"/Volumes/AgentStorage/newchanlun-legacy-drain"', JSON.stringify(drain))
        : file === ".sandcastle/deepagents-cutover.mts"
        ? source.replace('"/Volumes/AgentStorage/deepagents-exec"', JSON.stringify(state)) : source;
      writeFileSync(join(root, file), content);
    }
    const freeze = (generation: number) => writeFileSync(join(drain, "state.json"),
      JSON.stringify({ v: 1, mode: "draining", generation }));
    freeze(record === "stale" ? 2 : 1);
    const cutover = { v: 1, drain_generation: 1, frozen_at: "fixture", legacy_snapshot: [],
      workers_log_length: 0, workers_log_path: "/fixture/workers.jsonl", open_claims: [],
      remote_jobs_idle: true, remote_freeze_var: "1", variable_source: "user-token-readonly",
      release_label: "deepagents:queue", engine_head: "a".repeat(40),
      legacy_services_unloaded: ["com.newchanlun.sandcastle-claimer", "com.newchanlun.sandcastle-harvest"] };
    if (record !== "missing") writeFileSync(join(state, "cutover.json"),
      record === "malformed" ? "{" : JSON.stringify(cutover));
    const ticket = { number: 99, title: "[impl] shell fixture", state: "OPEN",
      labels: [{ name: "ready-for-agent" }], assignees: [] };
    for (const [name, value] of Object.entries({
      maps: [{ number: 1, title: "fixture map", state: "OPEN", labels: [] }],
      children: [{ number: 2, title: "SPEC: fixture", state: "OPEN" }, ticket],
      comments: [{ user: { login: "fixture-orchestrator" }, body: specRulingBody({ specNumber: 2, verdict: "PASS", author: "fixture-draft", reviewer: "fixture-review", digest: specBodyDigest(""), evidence: "/fixtures/review.json" }), created_at: "2026-01-01" }],
      tickets: [ticket],
    })) writeFileSync(join(root, name), JSON.stringify(value));
    writeFileSync(log, "");
    writeFileSync(join(root, "calls"), "");
    writeFileSync(join(root, "gh"), `#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$WFE_FIXTURE/calls"
case "$1 $2" in
  "issue list")
    case "$*" in
      *"--label wayfinder:map"*) cat "$WFE_FIXTURE/maps" ;;
      *) cat "$WFE_FIXTURE/tickets" ;;
    esac ;;
  "api --paginate")
    case "$3" in
      */issues/1/sub_issues) cat "$WFE_FIXTURE/children" ;;
      */issues/2/comments) cat "$WFE_FIXTURE/comments" ;;
      *) exit 71 ;;
    esac ;;
  "issue view")
    if [ -s "$WFE_FIXTURE/writes" ]; then
      echo '{"state":"OPEN","labels":[{"name":"deepagents:queue"}],"assignees":[]}'
    else echo '{"state":"OPEN","labels":[],"assignees":[]}'; fi ;;
  "issue edit") printf '%s\\n' "$*" >> "$WFE_FIXTURE/writes" ;;
  *) exit 71 ;;
esac
`, { mode: 0o755 });
    const child = spawn(process.execPath, ["--experimental-strip-types", "scripts/wayfinder_engine.mts",
      "--live", "--poll-seconds", "0.05", ...(once ? ["--once"] : [])], {
      cwd: root, env: { ...process.env, PATH: `${root}:${process.env.PATH}`, WFE_FIXTURE: root,
        WAYFINDER_ORCHESTRATOR: "fixture-orchestrator", WAYFINDER_GH_MAX_ATTEMPTS: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let out = "", err = "", changed = false, timedOut = false;
    const status = await new Promise<number | null>((resolve, reject) => {
      const timer = setTimeout(() => { timedOut = true; child.kill("SIGKILL"); }, 10_000);
      child.stdout.on("data", data => {
        out += String(data);
        if (!once && !changed && (out.match(/一轮结束/g) ?? []).length === 2) {
          changed = true;
          freeze(2);
        }
      });
      child.stderr.on("data", data => { err += String(data); });
      child.once("error", error => { clearTimeout(timer); reject(error); });
      child.once("close", code => { clearTimeout(timer); resolve(code); });
    });
    assert.equal(timedOut, false, `常驻入口未退出\n${out}\n${err}`);
    assert.equal(status, 0, `${out}\n${err}`);
    return { out: out + err, writes: readFileSync(log, "utf8"), calls: readFileSync(join(root, "calls"), "utf8") };
  } finally { rmSync(root, { recursive: true, force: true }); }
}

test("常驻入口：有效切流记录放行新队列并持续轮询，冻结代次变化后退出", async () => {
  const result = await runLiveLoop("valid");
  assert.equal(result.writes, "issue edit 99 --add-label deepagents:queue\n", result.out);
  assert.equal((result.out.match(/一轮结束/g) ?? []).length, 2);
  assert.match(result.out, /旧入口冻结/);
  assert.equal((result.calls.match(/issue list --label wayfinder:map/g) ?? []).length, 2);
});

for (const record of ["missing", "malformed", "stale"] as const) {
  test(`常驻入口：${record} 切流记录不读 tracker、不放行并退出`, async () => {
    const result = await runLiveLoop(record);
    assert.equal(result.calls, "");
    assert.equal(result.writes, "");
    assert.match(result.out, /旧入口冻结/);
  });
}

test("单轮入口：有效切流记录仍放行新队列且只运行一轮", async () => {
  const result = await runLiveLoop("valid", true);
  assert.equal(result.writes, "issue edit 99 --add-label deepagents:queue\n", result.out);
  assert.equal((result.out.match(/一轮结束/g) ?? []).length, 1);
});


test("SPEC CLI：真实读取审查文件，只比较所审摘要，不改写绑定或调用模型", () => {
  const root = mkdtempSync(join(tmpdir(), "wfe-spec-review-"));
  try {
    const body = "SPEC v1\n";
    const reviewPath = join(root, "review.json");
    const review = { specNumber: 7, verdict: "PASS", author: "draft-session", reviewer: "review-session", digest: specBodyDigest(body) };
    writeFileSync(reviewPath, JSON.stringify(review));
    writeFileSync(join(root, "body.json"), JSON.stringify({ body }));
    writeFileSync(join(root, "gh"), `#!/bin/sh
if [ "$1 $2 $3" != "issue view 7" ]; then exit 71; fi
cat "$WFE_FIXTURE/body.json"
`, { mode: 0o755 });
    const run = () => spawnSync(process.execPath, ["--experimental-strip-types", "scripts/wayfinder_engine.mts",
      "--spec-ruling", "7", "--review-file", reviewPath], {
      cwd: repoRoot, env: { ...process.env, PATH: `${root}:${process.env.PATH}`, WFE_FIXTURE: root },
      encoding: "utf8", timeout: 10_000,
    });
    const pass = run();
    assert.equal(pass.status, 0, pass.stderr);
    assert.match(pass.stdout, /SPEC 独审裁定：PASS/);
    assert.ok(pass.stdout.includes(`证据：${reviewPath}`));
    assert.equal(isSpecApproved({ number: 7, title: "SPEC: test", body, state: "OPEN", labels: [], assignees: [], blockedBy: 0,
      comments: [{ author: "fixture", body: pass.stdout, createdAt: "2026-11-07T00:00:00Z" }] }, "fixture"), true);
    writeFileSync(join(root, "body.json"), JSON.stringify({ body: body + "v2" }));
    const stale = run();
    assert.notEqual(stale.status, 0);
    assert.equal(stale.stdout, "");
    assert.match(stale.stderr, /正文已在独审后改动/);
    assert.deepEqual(JSON.parse(readFileSync(reviewPath, "utf8")), review);
    writeFileSync(join(root, "body.json"), JSON.stringify({ body }));
    writeFileSync(reviewPath, JSON.stringify({ ...review, verdict: "BLOCK" }));
    const block = run();
    assert.equal(block.status, 0, block.stderr);
    assert.match(block.stdout, /SPEC 独审裁定：BLOCK/);
    rmSync(reviewPath);
    const missing = run();
    assert.notEqual(missing.status, 0);
    assert.equal(missing.stdout, "");
    assert.match(missing.stderr, /ENOENT/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
