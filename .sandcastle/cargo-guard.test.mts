/**
 * #1588 ④ 本机 cargo 垫片测试。
 * 在 main.mts 同款工蜂环境构造（process.env + cargoGuardEnv）下跑 `cargo --version` 等，断言被拦截、
 * 指引指向 nc-remote；并静态核 nc-remote 本地段不调 cargo（cargo 只在 ssh 远端 heredoc 里出现）。
 * 跑法：npx tsx --test .sandcastle/cargo-guard.test.mts
 */
import { test, type TestContext } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import {
  CARGO_GUARD_DIR_NAME,
  cargoGuardEnv,
  defaultCargoGuardRoot,
  guardScript,
  GUARDED_TOOLS,
  installCargoGuard,
  LOCAL_CARGO_BLOCKED_EXIT,
} from "./cargo-guard.mts";

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

// ── N4：垫片目录复用固定路径，不再每次运行新建临时目录 ─────────────────────────────

function freshParent(t: TestContext): string {
  const parent = mkdtempSync(join(tmpdir(), "nc-cargo-guard-test-"));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  return parent;
}

test("N4：缺省垫片目录是固定路径，重复安装不在 $TMPDIR 新建目录", () => {
  const countGuardDirs = () => readdirSync(tmpdir()).filter((n) => n.startsWith(CARGO_GUARD_DIR_NAME)).length;
  const first = installCargoGuard();
  const before = countGuardDirs();
  const second = installCargoGuard();
  assert.equal(first.binDir, join(defaultCargoGuardRoot(), "bin"));
  assert.equal(defaultCargoGuardRoot(), join(tmpdir(), CARGO_GUARD_DIR_NAME));
  assert.deepEqual(second, first);
  assert.equal(countGuardDirs(), before);
});

test("N4：同一根目录反复安装原地覆盖，无残留临时文件", (t) => {
  const parent = freshParent(t);
  const root = join(parent, CARGO_GUARD_DIR_NAME);
  const a = installCargoGuard(root);
  const b = installCargoGuard(root);
  assert.deepEqual(a, b);
  assert.deepEqual(readdirSync(parent), [CARGO_GUARD_DIR_NAME]);
  assert.deepEqual(readdirSync(root).sort(), ["bin", "claude-env.sh"]);
  assert.deepEqual(readdirSync(a.binDir).sort(), [...GUARDED_TOOLS].sort());
});

test("N4：垫片被改坏后，下次安装恢复原样并继续拦截", (t) => {
  const root = join(freshParent(t), CARGO_GUARD_DIR_NAME);
  const guard = installCargoGuard(root);
  const cargo = join(guard.binDir, "cargo");
  writeFileSync(cargo, "#!/bin/sh\nexit 0\n");
  chmodSync(cargo, 0o644);
  installCargoGuard(root);
  assert.equal(readFileSync(cargo, "utf8"), guardScript("cargo"));
  const env = { ...process.env, ...cargoGuardEnv(guard) };
  assert.equal(spawnSync("sh", ["-c", "cargo --version"], { env }).status, LOCAL_CARGO_BLOCKED_EXIT);
});

test("N4：固定路径被软链预占 → FAIL-loud，拒绝使用", (t) => {
  const parent = freshParent(t);
  mkdirSync(join(parent, "elsewhere"));
  symlinkSync(join(parent, "elsewhere"), join(parent, CARGO_GUARD_DIR_NAME));
  assert.throws(() => installCargoGuard(join(parent, CARGO_GUARD_DIR_NAME)), /软链或属主不符/);
});

test("N4：其它进程并发重装时，运行中的工蜂调用 cargo 始终被拦截（原子替换，无半截垫片）", async (t) => {
  const root = join(freshParent(t), CARGO_GUARD_DIR_NAME);
  const guard = installCargoGuard(root);
  const mod = new URL("./cargo-guard.mts", import.meta.url).href;
  const code = `const { installCargoGuard } = await import(${JSON.stringify(mod)}); for (let i = 0; i < 40; i++) installCargoGuard(${JSON.stringify(root)});`;
  const children = Array.from({ length: 3 }, () => spawn(process.execPath, ["--import", "tsx", "--input-type=module", "-e", code], { stdio: "inherit" }));
  const exits = children.map((c) => new Promise<number | null>((res) => c.on("exit", res)));
  const env = { ...process.env, ...cargoGuardEnv(guard) };
  for (let i = 0; i < 30; i++) {
    const r = spawnSync("sh", ["-c", "cargo --version"], { env });
    assert.equal(r.status, LOCAL_CARGO_BLOCKED_EXIT, `第 ${i} 次调用未被拦截（exit ${r.status}）`);
  }
  assert.deepEqual(await Promise.all(exits), [0, 0, 0]);
  assert.deepEqual(readdirSync(guard.binDir).sort(), [...GUARDED_TOOLS].sort());
});
