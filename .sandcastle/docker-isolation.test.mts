import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import { assertHostPackagingSafe, hardenMounts, hardenProvider, parseObservation, type PathProbe } from "./docker-isolation.mts";

const wt = "/h/wt";
/** 假文件系统：present 为存在的路径，links 为符号链接及其真实目标。 */
const fakeProbe = (present: string[], links: Record<string, string> = {}): PathProbe => ({
  lstatSymlink: (p) => (p in links ? true : present.includes(p) ? false : null),
  realpath: (p) => { if (p in links) return links[p]!; if (p === wt || present.includes(p) || p.startsWith("/h/") || p === "/") return p; throw new Error("ENOENT"); },
});
const mounts = [
  { hostPath: wt, sandboxPath: "/home/agent/workspace" },
  { hostPath: `${wt}/.git`, sandboxPath: "/home/agent/workspace/.git" },
  { hostPath: "/h/repo/.git", sandboxPath: "/h/repo/.git" },
];

test("git 挂载只读，worktree 保持读写，已存在的 .agents/.codex 追加只读子挂载", () => {
  const out = hardenMounts(mounts, wt, fakeProbe([`${wt}/.agents`, `${wt}/.codex`]));
  assert.equal(out.find((m) => m.hostPath === wt)?.readonly, undefined);
  assert.equal(out.find((m) => m.hostPath === "/h/repo/.git")?.readonly, true);
  assert.equal(out.find((m) => m.sandboxPath === "/home/agent/workspace/.git")?.readonly, true);
  assert.deepEqual(out.filter((m) => m.hostPath !== wt && m.readonly).map((m) => m.sandboxPath).sort(),
    ["/h/repo/.git", "/home/agent/workspace/.agents", "/home/agent/workspace/.codex", "/home/agent/workspace/.git"]);
  assert.ok(!out.some((m) => m.sandboxPath.endsWith("/.aws")));
});

test("worktree 内 .git 目录（非链接 worktree）也被只读覆盖；不存在的受保护路径不新挂", () => {
  const out = hardenMounts([{ hostPath: wt, sandboxPath: "/home/agent/workspace" }], wt, fakeProbe([`${wt}/.git`]));
  assert.deepEqual(out.map((m) => [m.sandboxPath, m.readonly ?? false]), [["/home/agent/workspace", false], ["/home/agent/workspace/.git", true]]);
  assert.throws(() => hardenMounts([{ hostPath: "/other", sandboxPath: "/x" }], wt), /worktree 本体/);
});

test("受保护子路径是符号链接或真实路径逃出 worktree 时拒绝，而不是静默跳过或跟随", () => {
  const only = [{ hostPath: wt, sandboxPath: "/home/agent/workspace" }];
  for (const name of [".agents", ".codex", ".git"]) {
    assert.throws(() => hardenMounts(only, wt, fakeProbe([`${wt}/${name}`], { [`${wt}/${name}`]: "/h/secret" })), /符号链接/);
  }
  // 已被 Sandcastle 挂过的 .git 也不能是链接
  assert.throws(() => hardenMounts(mounts, wt, fakeProbe([`${wt}/.git`], { [`${wt}/.git`]: "/h/other/.git" })), /符号链接/);
  // 非链接但真实路径越界（父目录为链接）
  const escaping: PathProbe = { lstatSymlink: (p) => (p === `${wt}/.agents` ? false : null), realpath: (p) => (p === wt ? wt : "/h/elsewhere/.agents") };
  assert.throws(() => hardenMounts(only, wt, escaping), /逃出 worktree/);
});

test("hardenProvider 只改写 mounts，其余字段与 create 其他入参原样透传", async () => {
  let received: any;
  const base = { name: "docker", env: { A: "1" }, create: async (o: any) => { received = o; return "handle"; } };
  const wrapped = hardenProvider(base as any, fakeProbe([])) as any;
  assert.equal(wrapped.name, "docker");
  assert.equal(await wrapped.create({ worktreePath: wt, hostRepoPath: "/h/repo", env: { K: "v" }, mounts }), "handle");
  assert.equal(received.env.K, "v");
  assert.equal(received.hostRepoPath, "/h/repo");
  assert.equal(received.mounts.find((m: any) => m.hostPath === "/h/repo/.git").readonly, true);
});

test("Sandcastle docker() 启动参数仍不含 privileged/cap/security-opt（升级漂移守卫）", () => {
  const dir = new URL("../node_modules/@ai-hero/sandcastle/dist/", import.meta.url);
  const src = readFileSync(new URL("chunk-CP3TYXZA.js", dir), "utf8");
  assert.doesNotMatch(src, /--privileged|--cap-add|--cap-drop|--security-opt|--pid|--ipc|--userns/);
});

test("探针输出解析为观测值", () => {
  const o = parseObservation("apparmor=docker-default (enforce)\nseccomp=2\ncapEff=0000000000000000\ncapBnd=00000000a80425fb\nroWrite=touch: x: Read-only file system\nrwWrite=ok\n");
  assert.equal(o.apparmor, "docker-default (enforce)");
  assert.equal(o.rwWrite, "ok");
  assert.equal(o.capBnd, "00000000a80425fb");
});

function repo(config: string[]) {
  const dir = mkdtempSync(join(tmpdir(), "pkg-safe-"));
  const g = (...a: string[]) => execFileSync("git", ["-C", dir, ...a], { stdio: "pipe", env: { ...process.env, GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_SYSTEM: "/dev/null" } });
  g("init", "-q");
  for (const c of config) { const [k, v] = c.split("="); g("config", k!, v!); }
  writeFileSync(join(dir, "a.txt"), "x");
  writeFileSync(join(dir, ".gitattributes"), "a.txt filter=evil\n");
  g("add", "-A");
  return dir;
}

test("宿主封包：仓内属性引用宿主命令型驱动被拒；lfs 白名单与无驱动放行", () => {
  const env = { GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_SYSTEM: "/dev/null" };
  const orig = { ...process.env };
  Object.assign(process.env, env);
  try {
    assert.throws(() => assertHostPackagingSafe(repo(["filter.evil.clean=sh -c x"])), /命令型驱动 evil/);
    assertHostPackagingSafe(repo([]));
    assertHostPackagingSafe(repo(["filter.lfs.clean=git-lfs clean"]));
    const r = repo([]);
    assert.throws(() => assertHostPackagingSafe(r, (a) => a[0] === "config" ? `file:${r}/.git/config\tcore.hookspath ${r}/hooks\n` : ""), /hooksPath/);
  } finally { for (const k of Object.keys(env)) { if (orig[k] === undefined) delete process.env[k]; else process.env[k] = orig[k]; } }
});

import { mkdirSync as mk, symlinkSync, realpathSync as rp } from "node:fs";
import { hardenMounts as harden } from "./docker-isolation.mts";

test("真实文件系统：.agents 为指向 worktree 外的符号链接被拒；正常目录通过", () => {
  const dir = rp(mkdtempSync(join(tmpdir(), "harden-fs-")));
  const work = join(dir, "wt"); const outside = join(dir, "outside");
  mk(work); mk(outside); mk(join(work, ".codex"));
  symlinkSync(outside, join(work, ".agents"));
  const only = [{ hostPath: work, sandboxPath: "/home/agent/workspace" }];
  assert.throws(() => harden(only, work), /符号链接/);
  const ok = join(dir, "wt2"); mk(ok); mk(join(ok, ".codex"));
  assert.deepEqual(harden([{ hostPath: ok, sandboxPath: "/s" }], ok).map((m) => m.sandboxPath), ["/s", "/s/.codex"]);
});

test("hooksPath：相对路径、worktree 内、经符号链接回到 worktree 内均拒绝；.git/hooks 与 worktree 外放行", () => {
  const dir = rp(mkdtempSync(join(tmpdir(), "hooks-fs-")));
  const work = join(dir, "wt"); mk(join(work, ".git", "hooks"), { recursive: true }); mk(join(work, "hooks"));
  mk(join(dir, "ext-hooks"));
  symlinkSync(join(work, "hooks"), join(dir, "link-into-wt"));
  const cfg = (value: string) => (args: string[]) => (args[0] === "config" ? `file:/x\tcore.hookspath ${value}\n` : "");
  assert.throws(() => assertHostPackagingSafe(work, cfg("hooks")), /绝对路径/);
  assert.throws(() => assertHostPackagingSafe(work, cfg(join(work, "hooks"))), /worktree 内/);
  assert.throws(() => assertHostPackagingSafe(work, cfg(join(dir, "link-into-wt"))), /worktree 内/);
  assert.throws(() => assertHostPackagingSafe(work, cfg(join(work, "not-yet", "hooks"))), /worktree 内/);
  assertHostPackagingSafe(work, cfg(join(work, ".git", "hooks")));
  assertHostPackagingSafe(work, cfg(join(dir, "ext-hooks")));
});
