import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { MERGE_INSTANCE_ENV, MERGE_RUN_ENV, WORKSPACE_DEST, reclaimOrphanContainers, redactedInspect } from "./orphan-container.mts";

const SECRET = "sk-super-secret-cliproxy-key-0123456789";
const IMAGE = "sandcastle:newchanlun";
const tmp = () => realpathSync(mkdtempSync(join(tmpdir(), "orphan-")));
const GIT = "/Users/x/Projects/NewChanlun/.git";
function container(cwd: string, over: Record<string, unknown> = {}, id = "a".repeat(64)) {
  return {
    Id: id, Name: `/sandcastle-${id.slice(0, 8)}`, Created: "2026-10-27T23:00:00Z", State: { Running: true, Pid: 123 },
    Config: { Image: IMAGE, Env: [`CLIPROXY_LOCAL_KEY=${SECRET}`, `${MERGE_RUN_ENV}=run-1`, `${MERGE_INSTANCE_ENV}=inst-1`, "HOME=/home/agent"] },
    // 形态取自 row15-orphan-mounts.txt：工位 rw=true；工位 .git/.agents 与父仓 .git rw=false
    Mounts: [
      { Type: "bind", Source: GIT, Destination: GIT, RW: false },
      { Type: "bind", Source: `${cwd}/.git`, Destination: `${WORKSPACE_DEST}/.git`, RW: false },
      { Type: "bind", Source: `${cwd}/.agents`, Destination: `${WORKSPACE_DEST}/.agents`, RW: false },
      { Type: "bind", Source: cwd, Destination: WORKSPACE_DEST, RW: true },
    ],
    ...over,
  };
}
function fakeDocker(containers: unknown[], o: { psFail?: boolean; stopFail?: boolean; rmFail?: boolean } = {}) {
  const calls: string[][] = [];
  const live = new Set(containers.map((c) => (c as { Id: string }).Id));
  const docker = (args: string[]): string => {
    calls.push(args);
    if (o.psFail) throw new Error("Cannot connect to the Docker daemon");
    // 真实 docker ps 的行为：不带 --filter 就列出全部运行中容器；带 name= 过滤才缩小
    if (args[0] === "ps" && args.includes("-q") && !args.some((a) => a.startsWith("id="))) {
      const i = args.indexOf("--filter"); const re = i >= 0 && args[i + 1]!.startsWith("name=") ? new RegExp(args[i + 1]!.slice(5)) : null;
      return containers.filter((c) => live.has((c as { Id: string }).Id)).filter((c) => !re || re.test(((c as { Name: string }).Name).replace(/^\//, ""))).map((c) => (c as { Id: string }).Id).join("\n");
    }
    if (args[0] === "ps") return args.some((a) => a.startsWith("id=")) && live.has(args.find((a) => a.startsWith("id="))!.slice(3)) ? "x\n" : "";
    if (args[0] === "inspect") return JSON.stringify(containers.filter((c) => args.includes((c as { Id: string }).Id)));
    if (args[0] === "stop") { if (o.stopFail) throw new Error("stop failed"); return ""; }
    if (args[0] === "rm") { if (o.rmFail) throw new Error("rm failed"); live.delete(args[1]!); return ""; }
    throw new Error(`unexpected ${args.join(" ")}`);
  };
  return { docker, calls, live };
}
const opts = (cwd: string, ev: string) => ({ cwd, image: IMAGE, runId: "run-1", instance: "inst-1", evidenceDir: ev, now: () => new Date("2026-10-28T00:00:00Z") });
const verbs = (calls: string[][]) => calls.map((c) => c[0]);

test("唯一匹配：先归档脱敏 inspect，再 stop、rm；证据里没有密钥值", () => {
  const cwd = tmp(); const ev = tmp(); const f = fakeDocker([container(cwd)]);
  const r = reclaimOrphanContainers({ ...opts(cwd, ev), docker: f.docker });
  assert.equal(r.length, 1); assert.deepEqual(verbs(f.calls).filter((v) => v === "stop" || v === "rm"), ["stop", "rm"]);
  assert.ok(f.calls.findIndex((c) => c[0] === "stop") > -1 && f.live.size === 0);
  const files = readdirSync(ev).filter((n) => n.startsWith("orphan-container-"));
  assert.equal(files.length, 1); const text = readFileSync(join(ev, files[0]!), "utf8");
  assert.ok(!text.includes(SECRET), "证据里不能出现密钥值"); assert.ok(!/sk-super|CLIPROXY_LOCAL_KEY=/.test(text));
  const j = JSON.parse(text);
  assert.deepEqual(j.EnvKeys, ["CLIPROXY_LOCAL_KEY", "HOME", MERGE_INSTANCE_ENV, MERGE_RUN_ENV].sort());
  assert.equal(j.Image, IMAGE); assert.equal(j.Mounts.length, 4); assert.equal(j.Id, "a".repeat(64));
  assert.ok(!("Config" in j) && !("Env" in j));
  assert.deepEqual(Object.keys(redactedInspect(container(cwd))).sort(), ["Created", "EnvKeys", "Id", "Image", "Mounts", "Name", "State"]);
});

test("无 sandcastle 容器：直接通过，不 inspect", () => {
  const cwd = tmp(); const f = fakeDocker([]);
  assert.deepEqual(reclaimOrphanContainers({ ...opts(cwd, tmp()), docker: f.docker }), []);
  assert.deepEqual(verbs(f.calls), ["ps"]);
});

test("与本工位无关的容器不碰：别的工位、别的 run、别的实例", () => {
  const cwd = tmp(); const other = tmp();
  const f = fakeDocker([container(other, {}, "b".repeat(64)),
    container(other, { Config: { Image: IMAGE, Env: [`${MERGE_RUN_ENV}=run-2`, `${MERGE_INSTANCE_ENV}=inst-1`] } }, "c".repeat(64)),
    container(other, { Config: { Image: IMAGE, Env: [`${MERGE_RUN_ENV}=run-1`, `${MERGE_INSTANCE_ENV}=inst-OTHER`] } }, "d".repeat(64))]);
  // 第一个容器身份 env 对得上 run-1/inst-1 但挂载在别的工位 → 属于“挂载来源不同”，应拒绝；先单独验证后两个无关容器
  const unrelated = fakeDocker([container(other, { Config: { Image: IMAGE, Env: [`${MERGE_RUN_ENV}=run-2`, `${MERGE_INSTANCE_ENV}=inst-1`] } }, "c".repeat(64)),
    container(other, { Config: { Image: IMAGE, Env: ["HOME=/home/agent"] } }, "e".repeat(64))]);
  assert.deepEqual(reclaimOrphanContainers({ ...opts(cwd, tmp()), docker: unrelated.docker }), []);
  assert.equal(verbs(unrelated.calls).filter((v) => v === "stop" || v === "rm").length, 0);
  assert.throws(() => reclaimOrphanContainers({ ...opts(cwd, tmp()), docker: f.docker }), /挂载来源与本归并工位不同/);
  assert.equal(f.live.size, 3, "拒绝时一个容器都没动");
});

test("拒绝且不动：挂载来源不同、两个匹配、身份 env 不符、镜像不符、非 Sandcastle 名字、docker 不可用、停删失败", () => {
  const cwd = tmp(); const other = tmp();
  const run = (cs: unknown[], o: Parameters<typeof fakeDocker>[1] = {}) => { const f = fakeDocker(cs, o); return { f, go: () => reclaimOrphanContainers({ ...opts(cwd, tmp()), docker: f.docker }) }; };
  const noMutation = (f: ReturnType<typeof fakeDocker>) => assert.equal(f.calls.filter((c) => c[0] === "stop" || c[0] === "rm").length, 0);
  let t = run([container(other)]); assert.throws(t.go, /挂载来源与本归并工位不同/); noMutation(t.f);
  t = run([container(cwd, {}, "a".repeat(64)), container(cwd, {}, "b".repeat(64))]); assert.throws(t.go, /匹配到 2 个容器/); noMutation(t.f);
  t = run([container(cwd, { Config: { Image: IMAGE, Env: [`${MERGE_RUN_ENV}=run-1`, `${MERGE_INSTANCE_ENV}=inst-9`] } })]); assert.throws(t.go, /身份 env 与本归并 run\/实例不符/); noMutation(t.f);
  t = run([container(cwd, { Config: { Image: IMAGE, Env: ["HOME=/x"] } })]); assert.throws(t.go, /身份 env/); noMutation(t.f);
  t = run([container(cwd, { Config: { Image: "evil:latest", Env: [`${MERGE_RUN_ENV}=run-1`, `${MERGE_INSTANCE_ENV}=inst-1`] } })]); assert.throws(t.go, /镜像 evil:latest/); noMutation(t.f);
  t = run([container(cwd, { Name: "/not-ours" })]); assert.throws(t.go, /不是 Sandcastle 容器/); noMutation(t.f);
  assert.ok(t.f.calls.every((c) => !c.includes("--filter")), "不在 daemon 侧按名字过滤，否则这条分支在真实 docker 上走不到");
  // 非 sandcastle 名字、既无身份也不挂本工位：不碰
  const bystander = fakeDocker([container(other, { Name: "/postgres", Config: { Image: "postgres", Env: ["A=1"] } }, "f".repeat(64))]);
  assert.deepEqual(reclaimOrphanContainers({ ...opts(cwd, tmp()), docker: bystander.docker }), []);
  t = run([], { psFail: true }); assert.throws(t.go, /docker 不可用或输出无法判定/);
  t = run([container(cwd)], { stopFail: true }); assert.throws(t.go, /docker stop .* 失败/);
  t = run([container(cwd)], { rmFail: true }); assert.throws(t.go, /docker rm .* 失败/);
  // 工位挂载缺失（身份对得上但没有 /home/agent/workspace 挂载）
  t = run([container(cwd, { Mounts: [{ Type: "bind", Source: cwd, Destination: "/elsewhere", RW: true }] })]); assert.throws(t.go, /工位挂载来源/); noMutation(t.f);
});

test("归档失败（证据文件已存在）则不动容器；归档文件名含容器 id 与时间", () => {
  const cwd = tmp(); const ev = tmp();
  const f1 = fakeDocker([container(cwd)]);
  const r = reclaimOrphanContainers({ ...opts(cwd, ev), docker: f1.docker });
  assert.match(r[0]!.archive, /orphan-container-aaaaaaaaaaaa-2026-10-28T00-00-00-000Z\.json$/);
  const f2 = fakeDocker([container(cwd)]);
  assert.throws(() => reclaimOrphanContainers({ ...opts(cwd, ev), docker: f2.docker }), /无法归档/);
  assert.equal(f2.calls.filter((c) => c[0] === "stop" || c[0] === "rm").length, 0);
});

test("并发重入已回收：stop/rm 失败但容器已不存在，视为成功", () => {
  const cwd = tmp(); const ev = tmp();
  const c = container(cwd); const gone = new Set<string>();
  const docker = (args: string[]): string => {
    if (args[0] === "ps" && args.includes("-q") && !args.some((a) => a.startsWith("id="))) return c.Id;
    if (args[0] === "ps") return "";
    if (args[0] === "inspect") return JSON.stringify([c]);
    gone.add(args[0]!); throw new Error("No such container");
  };
  const r = reclaimOrphanContainers({ ...opts(cwd, ev), docker });
  assert.equal(r.length, 1); assert.deepEqual([...gone].sort(), ["rm", "stop"]);
});

test("swarm-merge 接线：冲突工蜂容器带身份 env（无密钥）；回收只在锁持有者已死且停在冲突阶段后、接管前发生", () => {
  const src = readFileSync(new URL("./swarm-merge.mts", import.meta.url), "utf8");
  assert.match(src, /\[MERGE_RUN_ENV\]: manifest\.runId, \[MERGE_INSTANCE_ENV\]: mergeInstance\(/);
  assert.match(src, /reclaimOrphans: \(\) => \{ reclaimOrphanContainers\(/);
  const mm = readFileSync(new URL("./managed-merge.mts", import.meta.url), "utf8");
  assert.match(mm, /acquireLock\(lock, o\.cwd, o\.lockEnv, \(\) => \{/);
  assert.match(mm, /phase === "conflict"\) \{ o\.reclaimOrphans\(\); orphansSettled = true; \}/);
  const lockSrc = readFileSync(new URL("./process-lock.mts", import.meta.url), "utf8");
  assert.ok(lockSrc.indexOf("beforeTakeover?.(holder)") > lockSrc.indexOf("仍在运行") && lockSrc.indexOf("beforeTakeover?.(holder)") < lockSrc.indexOf("renameSync(lock, target)"));
  assert.ok(existsSync(new URL("./orphan-container.mts", import.meta.url)));
});
