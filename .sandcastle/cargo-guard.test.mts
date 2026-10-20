/**
 * #1588 ④ 本机 cargo 垫片测试。
 * 在 main.mts 同款工蜂环境构造（process.env + cargoGuardEnv）下跑 `cargo --version` 等，断言被拦截、
 * 指引指向 nc-remote；并静态核 nc-remote 本地段不调 cargo（cargo 只在 ssh 远端 heredoc 里出现）。
 * 跑法：npx tsx --test .sandcastle/cargo-guard.test.mts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { cargoGuardEnv, GUARDED_TOOLS, installCargoGuard, LOCAL_CARGO_BLOCKED_EXIT } from "./cargo-guard.mts";

function workerEnv() {
  const guard = installCargoGuard();
  return { guard, env: { ...process.env, ...cargoGuardEnv(guard) } };
}

test("工蜂环境下 `cargo --version` 被拦截：非零退出 + 指引指向 nc-remote", () => {
  const { guard, env } = workerEnv();
  const r = spawnSync("sh", ["-c", "cargo --version"], { env, encoding: "utf8" });
  assert.equal(r.status, LOCAL_CARGO_BLOCKED_EXIT);
  assert.equal(r.stdout, "");
  assert.match(r.stderr, /nc-local-cargo-guard #1588/);
  assert.match(r.stderr, /~\/\.local\/bin\/nc-remote <已提交的 ref/);
  assert.match(r.stderr, /NC_TARGET=/);
  assert.match(r.stderr, /推分支开 PR 走 CI/);
  const which = spawnSync("sh", ["-c", "command -v cargo"], { env, encoding: "utf8" });
  assert.equal(which.stdout.trim(), join(guard.binDir, "cargo"));
});

test("所有垫片工具（cargo/rustc/rustup/cargo-* …）都拦截，且可执行", () => {
  const { guard, env } = workerEnv();
  for (const tool of GUARDED_TOOLS) {
    assert.ok(statSync(join(guard.binDir, tool)).mode & 0o111, `${tool} 不可执行`);
    const r = spawnSync("sh", ["-c", `${tool} build --release`], { env, encoding: "utf8" });
    assert.equal(r.status, LOCAL_CARGO_BLOCKED_EXIT, `${tool} 未被拦截`);
    assert.ok(r.stderr.includes(`禁止运行 ${tool}`));
  }
});

test("login shell 重排 PATH 后（垫片被挤到后面），source CLAUDE_ENV_FILE 仍把垫片顶回最前", () => {
  const { guard } = workerEnv();
  // 模拟 path_helper：/opt/homebrew/bin 等系统路径排到垫片前面。
  const env = { ...process.env, PATH: `/opt/homebrew/bin:/usr/bin:/bin:${guard.binDir}`, CLAUDE_ENV_FILE: guard.envFile };
  const r = spawnSync("bash", ["-c", 'source "$CLAUDE_ENV_FILE"; command -v cargo; cargo --version'], { env, encoding: "utf8" });
  assert.equal(r.stdout.trim(), join(guard.binDir, "cargo"));
  assert.equal(r.status, LOCAL_CARGO_BLOCKED_EXIT);
});

test("垫片不影响非 Rust 工具（git / node 照常）", () => {
  const { env } = workerEnv();
  assert.equal(spawnSync("sh", ["-c", "git --version"], { env }).status, 0);
  assert.equal(spawnSync("sh", ["-c", "node --version"], { env }).status, 0);
});

/** nc-remote 本地段 = ssh heredoc（<<SH … SH）之外的行；cargo 只允许出现在 heredoc（远端 WSL 执行）里。 */
function localSegment(script: string): string {
  const out: string[] = [];
  let inRemote = false;
  for (const line of script.split("\n")) {
    if (inRemote) {
      if (line.trim() === "SH") inRemote = false;
      continue;
    }
    if (/<<-?\s*'?SH'?\s*$/.test(line)) inRemote = true;
    if (/^\s*#/.test(line)) continue;
    out.push(line);
  }
  return out.join("\n");
}

test("localSegment：只剥 heredoc 与注释", () => {
  const s = "# cargo 注释\necho a\nssh host bash -s <<SH\ncargo build\nSH\necho b";
  assert.equal(localSegment(s), "echo a\nssh host bash -s <<SH\necho b");
});

const NC_REMOTE = join(homedir(), ".local/bin/nc-remote");

test("nc-remote 本地段不调用 cargo / rustc / rustup（垫片不影响远端执行）", (t) => {
  if (!existsSync(NC_REMOTE)) {
    t.skip(`${NC_REMOTE} 不在本机（CI runner 无 nc-remote；本机演练见 REPORT.md）`);
    return;
  }
  const script = readFileSync(NC_REMOTE, "utf8");
  const local = localSegment(script);
  assert.ok(!/\b(cargo|rustc|rustup)\b/.test(local), `nc-remote 本地段出现 cargo：\n${local}`);
  // 远端段确有 cargo 环境（source ~/.cargo/env 在 heredoc 里），说明 Rust 命令在 WSL 侧解析。
  assert.match(script, /<<SH[\s\S]*source ~\/\.cargo\/env[\s\S]*\nSH\n/);
});
