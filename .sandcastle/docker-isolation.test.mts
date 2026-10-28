import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import { createSandbox } from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { homedir } from "node:os";
import { rmSync } from "node:fs";
import { CONTAINER_ENV_ALLOW, assertHostPackagingSafe, containerEnvLeaks, declaredContainerEnv, hardenMounts, hardenProvider, imageEnvSecrets, parseObservation, type PathProbe } from "./docker-isolation.mts";
import { MERGE_INSTANCE_ENV, MERGE_RUN_ENV } from "./orphan-container.mts";
import { swarmSandbox } from "./swarm-route.mts";

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
  const base = { name: "docker", env: { CLIPROXY_BASE_URL: "http://h:1" }, create: async (o: any) => { received = o; return "handle"; } };
  const wrapped = hardenProvider(base as any, fakeProbe([])) as any;
  assert.equal(wrapped.name, "docker");
  assert.equal(await wrapped.create({ worktreePath: wt, hostRepoPath: "/h/repo", env: { K: "v" }, mounts }), "handle");
  assert.deepEqual(received.env, { CLIPROXY_BASE_URL: "http://h:1" }, "运行期 env 里的 K 不进入容器");
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

// —— D6 前凭据暴露闭合：容器 env 白名单 ——
const LEAK = ["KIMI_API_KEY", "DEEPSEEK_API_KEY", "ZAI_API_KEY", "MOONSHOT_API_KEY", "GH_TOKEN", "OPENAI_API_KEY", "EXTRA_X"];
const dotEnvText = [...LEAK.map((k) => `${k}=LEAKSENTINEL-${k}`), "CLIPROXY_LOCAL_KEY=LEAKSENTINEL-ENVFILE", "CLIPROXY_BASE_URL=LEAKSENTINEL-URL", "SANDCASTLE_MERGE_RUN=LEAKSENTINEL-RUN"].join("\n") + "\n";

test("白名单与身份键名：只有 CLIProxy 路由键与归并身份键，键名与 orphan-container 逐字一致", () => {
  assert.deepEqual([...CONTAINER_ENV_ALLOW].sort(), ["CLIPROXY_BASE_URL", "CLIPROXY_LOCAL_KEY", MERGE_INSTANCE_ENV, MERGE_RUN_ENV].sort());
});

test("hardenProvider：满值 .env 并入的 env 被剥离——白名单键的值只取 provider 声明，.env/运行期同名键也不采信", async () => {
  let received: any;
  const declared = { CLIPROXY_BASE_URL: "http://host.docker.internal:8318", CLIPROXY_LOCAL_KEY: "declared", [MERGE_RUN_ENV]: "run-1", [MERGE_INSTANCE_ENV]: "inst-1" };
  const base = { name: "docker", env: declared, create: async (o: any) => { received = o; return "h"; } };
  const merged = Object.fromEntries(dotEnvText.trim().split("\n").map((l) => l.split("=") as [string, string]));
  await (hardenProvider(base as any, fakeProbe([])) as any).create({ worktreePath: wt, mounts, env: { ...merged, ...declared, EXTRA_RUN: "x" } });
  assert.deepEqual(received.env, declared);
  assert.ok(!JSON.stringify(received).includes("LEAKSENTINEL"));
});

test("declaredContainerEnv：白名单之外的声明键 FAIL-loud（只报键名）；空值/undefined 丢弃", () => {
  assert.throws(() => declaredContainerEnv({ CLIPROXY_LOCAL_KEY: "s3cret-value", KIMI_API_KEY: "s3cret-value" }), (e: Error) => /KIMI_API_KEY/.test(e.message) && !/s3cret-value/.test(e.message));
  assert.deepEqual(declaredContainerEnv({ CLIPROXY_LOCAL_KEY: undefined, CLIPROXY_BASE_URL: "" }), {});
  assert.deepEqual(declaredContainerEnv(undefined), {});
  assert.throws(() => hardenProvider({ env: { GH_TOKEN: "x" }, create: async () => 0 } as any, fakeProbe([])), /GH_TOKEN/);
});

test("imageEnvSecrets / containerEnvLeaks：镜像自带非空凭据、容器里多出的键、被覆盖成非空值的凭据键都能抓到", () => {
  const image = ["PATH=/bin", "KIMI_API_KEY=", "GH_TOKEN=", "GH_REPO=a/b", "NODE_VERSION=22"];
  assert.deepEqual(imageEnvSecrets(image), []);
  assert.deepEqual(imageEnvSecrets([...image, "DEEPSEEK_API_KEY=abc"]), ["DEEPSEEK_API_KEY"]);
  const ok = [...image, "HOME=/home/agent", "CLIPROXY_LOCAL_KEY=k", "CLIPROXY_BASE_URL=u", `${MERGE_RUN_ENV}=r`];
  assert.deepEqual(containerEnvLeaks(ok, image), []);
  assert.deepEqual(containerEnvLeaks([...ok, "EXTRA_X=1"], image), ["EXTRA_X"]);
  assert.deepEqual(containerEnvLeaks(ok.map((e) => (e === "KIMI_API_KEY=" ? "KIMI_API_KEY=real" : e)), image), ["KIMI_API_KEY"]);
});

test("Sandcastle 升级漂移守卫：容器 env 只来自 create 的 env + HOME，不引入 --env-file；resolveEnv 仍读 .sandcastle/.env（这正是白名单存在的理由）", () => {
  const dir = new URL("../node_modules/@ai-hero/sandcastle/dist/", import.meta.url);
  const dockerSrc = readFileSync(new URL("chunk-CP3TYXZA.js", dir), "utf8");
  assert.doesNotMatch(dockerSrc, /--env-file|--env\b/);
  assert.match(dockerSrc, /\.\.\.createOptions\.env,\s*HOME: "\/home\/agent"/);
  const index = readFileSync(new URL("index.js", dir), "utf8");
  assert.match(index, /join\(repoDir, "\.sandcastle", "\.env"\)/);
  const route = readFileSync(new URL("./swarm-route.mts", import.meta.url), "utf8");
  assert.match(route, /hardenProvider\(docker\(\{ imageName: imageId, env \}\)\)/, "Docker 路径唯一入口经 hardenProvider");
  for (const f of ["main-swarm.mts", "swarm-merge.mts"]) assert.doesNotMatch(readFileSync(new URL(`./${f}`, import.meta.url), "utf8"), /\bdocker\(\{/, `${f} 不得绕过 swarmSandbox`);
});

// 真实容器：走 Sandcastle 真实的 createSandbox（真实读 .sandcastle/.env、真实 docker run），只起 sleep infinity 的无工蜂容器，docker inspect 看 Config.Env。
const REAL_IMAGE = process.env.SANDCASTLE_ENV_TEST_IMAGE ?? "sandcastle:newchanlun";
const hasImage = (() => { try { execFileSync("docker", ["image", "inspect", REAL_IMAGE, "--format", "{{.Id}}"], { stdio: "ignore", timeout: 15_000 }); return true; } catch { return false; } })();
const idOf = (image: string) => execFileSync("docker", ["image", "inspect", image, "--format", "{{.Id}}"], { encoding: "utf8" }).trim();
async function inspectEnvOf(provider: ReturnType<typeof swarmSandbox>): Promise<{ env: string[]; imageEnv: string[]; image: string }> {
  const base = join(homedir(), ".cache", "sandcastle-env-probe"); mkdirSync(base, { recursive: true });
  const repo = mkdtempSync(join(base, "repo-"));
  const g = (...a: string[]) => execFileSync("git", a, { cwd: repo, encoding: "utf8" }).trim();
  try {
    g("init", "-b", "main"); g("config", "user.name", "T"); g("config", "user.email", "t@x");
    mkdirSync(join(repo, ".sandcastle")); writeFileSync(join(repo, ".sandcastle", ".env"), dotEnvText);
    writeFileSync(join(repo, ".gitignore"), ".sandcastle/\n"); writeFileSync(join(repo, "a.txt"), "a"); g("add", "."); g("commit", "-m", "i");
    const sb = await createSandbox({ branch: "env-probe", sandbox: provider, cwd: repo });
    try {
      const hit = execFileSync("docker", ["ps", "-q", "--no-trunc"], { encoding: "utf8" }).trim().split("\n").filter(Boolean)
        .map((id) => JSON.parse(execFileSync("docker", ["inspect", id], { encoding: "utf8" }))[0]).filter((c) => JSON.stringify(c.Mounts).includes(repo));
      assert.equal(hit.length, 1, "恰好找到本夹具的容器");
      const img = JSON.parse(execFileSync("docker", ["image", "inspect", REAL_IMAGE], { encoding: "utf8" }))[0];
      return { env: hit[0].Config.Env as string[], imageEnv: img.Config.Env as string[], image: hit[0].Image as string };
    } finally { await sb.close(); }
  } finally { rmSync(repo, { recursive: true, force: true }); }
}
const declaredRoute = { CLIPROXY_BASE_URL: "http://host.docker.internal:8318", CLIPROXY_LOCAL_KEY: "declared-no-network", [MERGE_RUN_ENV]: "run-env-test", [MERGE_INSTANCE_ENV]: "inst-env-test" };

test("真实容器 EnvKeys：满值 .env 夹具仓经 swarmSandbox 起无凭据临时容器，docker inspect 的 Config.Env 只含镜像自带键＋白名单＋HOME，且无任何泄漏值", { skip: hasImage ? false : `本机无镜像 ${REAL_IMAGE}`, timeout: 180_000 }, async () => {
  const { env, imageEnv } = await inspectEnvOf(swarmSandbox(idOf(REAL_IMAGE), declaredRoute));
  const text = env.join("\n");
  assert.ok(!text.includes("LEAKSENTINEL"), "任何 .env 值都不得进入容器");
  assert.deepEqual(containerEnvLeaks(env, imageEnv), []);
  const keys = new Set(env.map((e) => e.split("=")[0]));
  for (const k of CONTAINER_ENV_ALLOW) assert.ok(keys.has(k), `白名单键 ${k} 已注入`);
  assert.ok(env.includes("CLIPROXY_LOCAL_KEY=declared-no-network") && env.includes(`${MERGE_RUN_ENV}=run-env-test`), "值来自 provider 声明");
  assert.ok(!keys.has("EXTRA_X") && !keys.has("OPENAI_API_KEY"), ".env 里镜像没有的键不在容器里");
  for (const k of ["KIMI_API_KEY", "DEEPSEEK_API_KEY", "ZAI_API_KEY", "MOONSHOT_API_KEY", "GH_TOKEN"]) assert.ok(!env.some((e) => e.startsWith(`${k}=`) && e !== `${k}=`), `${k} 不得带值（镜像自带的空占位除外）`);
  assert.deepEqual(imageEnvSecrets(imageEnv), [], "镜像自带 ENV 无非空凭据");
});

test("对照（证明上面的断言有牙齿）：同一夹具、不经 hardenProvider 的裸 docker() 容器会带上 .env 的值", { skip: hasImage ? false : `本机无镜像 ${REAL_IMAGE}`, timeout: 180_000 }, async () => {
  const raw = docker({ imageName: REAL_IMAGE, env: declaredRoute }) as unknown as ReturnType<typeof swarmSandbox>;
  const { env, imageEnv } = await inspectEnvOf(raw);
  assert.ok(env.join("\n").includes("LEAKSENTINEL"));
  assert.ok(containerEnvLeaks(env, imageEnv).includes("KIMI_API_KEY") && containerEnvLeaks(env, imageEnv).includes("EXTRA_X"));
});

// —— tag/Id 窗口：准入解析出的 Id 原样用于启动容器 ——
test("真实容器：tag 在准入之后被重指，容器仍以已准入 Id 启动（inspect .Image 核对）；按 tag 启动的对照会落到被重指的镜像；已准入镜像被回收时 FAIL-loud 而不是换镜像", { skip: hasImage ? false : `本机无镜像 ${REAL_IMAGE}`, timeout: 300_000 }, async () => {
  const stamp = Date.now();
  const tag = `sandcastle-idtest:${stamp}`, keep = `sandcastle-idtest-keep:${stamp}`;
  const build = (label: string) => execFileSync("docker", ["build", "-q", "-t", tag, "-"], { input: `FROM ${REAL_IMAGE}\nLABEL sandcastle.idtest=${label}\n`, encoding: "utf8" }).trim();
  try {
    build("admitted");
    const admitted = idOf(tag); // 模拟 assertSwarmExecutionReady 返回的准入 Id
    execFileSync("docker", ["tag", tag, keep]); // 另一个名字保住已准入镜像（containerd 镜像库里无名镜像会被回收）
    build("repointed"); // 准入之后 tag 被重指到另一个镜像
    const repointed = idOf(tag);
    assert.notEqual(admitted, repointed, "夹具：tag 确已指向另一个 Id");
    const viaId = await inspectEnvOf(swarmSandbox(admitted, declaredRoute));
    assert.equal(viaId.image, admitted, "容器以准入 Id 启动");
    const raw = await inspectEnvOf(docker({ imageName: tag, env: declaredRoute }) as unknown as ReturnType<typeof swarmSandbox>);
    assert.equal(raw.image, repointed, "对照：按 tag 启动会落到被重指的镜像（窗口真实存在）");
    // 已准入镜像不复存在：按 Id 启动失败，而不是静默换成别的镜像
    execFileSync("docker", ["rmi", keep]);
    await assert.rejects(inspectEnvOf(swarmSandbox(admitted, declaredRoute)), /not found locally|No such image|create failed/);
  } finally {
    for (const t of [tag, keep]) { try { execFileSync("docker", ["rmi", t], { stdio: "ignore" }); } catch { /* 已不存在 */ } }
  }
});
