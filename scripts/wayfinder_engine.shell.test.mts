/**
 * wayfinder_engine 壳层测试（#1084 追加验收）：
 *  - 注入一次 GitHub API timeout → 有限重试 + 指数退避后自动恢复并继续（进程不永久停摆）；
 *  - 空队列 → 正常走完一轮（不退出即由常驻 for(;;) 收口，本测试只验 --once 一轮）。
 * 跑法：npx tsx --test scripts/wayfinder_engine.shell.test.mts
 *
 * 做法：临时目录里放一个假 gh（首 N 次调用 exit 1 模拟 timeout，之后对 issue list 返回 []），
 * 把该目录前置进 PATH，子进程跑 --once --dry-run 一轮，断言重试日志与一轮收尾都出现。
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

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
    ]) copyFileSync(join(repoRoot, source), join(dir, target));
    const result = spawnSync(process.execPath, [
      "--experimental-strip-types", "--input-type=module", "--eval",
      "const m = await import('./scripts/wayfinder_engine.mts'); " +
      "if (m.DEFAULT_SPEC_MODEL !== 'kimi-k3.1-pro') throw new Error('wrong model'); " +
      "console.log(m.DEFAULT_SPEC_MODEL);",
    ], { cwd: dir, encoding: "utf8", timeout: 30_000 });
    assert.equal(result.status, 0, `stdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
    assert.equal(result.stdout.trim(), "kimi-k3.1-pro");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
