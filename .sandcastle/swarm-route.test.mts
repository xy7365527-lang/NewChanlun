import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFile } from "node:child_process";
import { createServer } from "node:http";
import { once } from "node:events";
import { promisify } from "node:util";
import {
  DOCKER_CLIPROXY_BASE_URL,
  assertDockerCliProxyReachable,
  swarmAgent,
  swarmSandbox,
} from "./swarm-route.mts";
import { MAIN_MODEL } from "./main-model.mts";

const ID = "sha256:" + "3".repeat(64);

test("swarm 四角色共用主控模型，provider 命令不显式换档", () => {
  const provider = swarmAgent();
  const command = provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true });
  assert.equal(MAIN_MODEL, "gpt-6-astra");
  assert.ok(command.command.includes(MAIN_MODEL));
  assert.ok(!command.command.includes("prime-agent"));
});

test("swarm Docker 使用宿主网关而非容器回环，密钥只存在运行时 env", () => {
  const env = { CLIPROXY_BASE_URL: DOCKER_CLIPROXY_BASE_URL, CLIPROXY_LOCAL_KEY: "fixture-secret" };
  const sandbox = swarmSandbox(ID, env);
  assert.equal(sandbox.env.CLIPROXY_BASE_URL, "http://host.docker.internal:8318");
  assert.equal(sandbox.env.CLIPROXY_LOCAL_KEY, "fixture-secret");
  assert.ok(!DOCKER_CLIPROXY_BASE_URL.includes("127.0.0.1"));
});

test("main-swarm 不再实例化 prime-agent/k3，planner/单票沙盒统一走路由 helper", () => {
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(!source.includes("primeAgent("));
  assert.ok(!source.includes('"k3"'));
  assert.equal(source.match(/swarmSandbox\(IMAGE_ID_ADMITTED\)/g)?.length, 2, "容器以准入返回的 Id 启动，不用 tag");
  assert.doesNotMatch(source, /swarmSandbox\(IMAGE\)/);
  assert.equal(source.match(/swarmAgent\(\)/g)?.length, 3);
});

test("main-swarm 领票前验证 Docker 路由，失败时不回退旧模型", () => {
  let called = 0;
  const previousExitCode = process.exitCode;
  try {
    assertDockerCliProxyReachable("fixture-image", (image, probe) => {
      called += 1;
      assert.equal(image, "fixture-image");
      assert.ok(probe.includes("/v1/responses"));
    });
    assert.equal(called, 1);
    assert.throws(
      () => assertDockerCliProxyReachable("fixture-image", () => { throw new Error("ECONNREFUSED"); }),
      /不会回退 k3\/Claude/,
    );
  } finally {
    process.exitCode = previousExitCode;
  }
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(source.indexOf("assertDockerCliProxyReachable(IMAGE_ID_ADMITTED)") < source.indexOf("for (let iteration"));
});

test("main-swarm 路由门实际请求主控模型，而不是只测 TCP 连通", () => {
  assertDockerCliProxyReachable("fixture-image", (_image, probe) => {
    assert.ok(probe.includes("fetch("));
    assert.ok(probe.includes("/v1/responses"));
    assert.ok(probe.includes(MAIN_MODEL));
  });
});

test("main-swarm 探针发出 Responses 请求并验收主控模型响应", async () => {
  const requests: Array<{ url?: string; authorization?: string; model?: string }> = [];
  const server = createServer((request, response) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk) => { body += chunk; });
    request.on("end", () => {
      requests.push({
        url: request.url,
        authorization: request.headers.authorization,
        model: JSON.parse(body).model,
      });
      response.writeHead(200, { "content-type": "application/json" });
      response.end(JSON.stringify({ object: "response", status: "completed", model: MAIN_MODEL, output: [] }));
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  try {
    let probe = "";
    assertDockerCliProxyReachable("fixture-image", (_image, value) => { probe = value; });
    const address = server.address();
    assert.ok(address && typeof address === "object");
    const { stdout } = await promisify(execFile)(process.execPath, ["-e", probe], {
      env: {
        ...process.env,
        CLIPROXY_BASE_URL: `http://127.0.0.1:${address.port}`,
        CLIPROXY_LOCAL_KEY: "fixture-secret",
      },
    });
    assert.match(stdout, /MAIN_MODEL_HTTP_OK/);
    assert.deepEqual(requests, [{
      url: "/v1/responses",
      authorization: "Bearer fixture-secret",
      model: MAIN_MODEL,
    }]);
  } finally {
    server.close();
    await once(server, "close");
  }
});

test("main-swarm 路由门失败后保留非零退出码", () => {
  const previousExitCode = process.exitCode;
  try {
    assert.throws(
      () => assertDockerCliProxyReachable("fixture-image", () => { throw new Error("HTTP 503"); }),
      /main-swarm 不可用/,
    );
    assert.equal(process.exitCode, 1);
  } finally {
    process.exitCode = previousExitCode;
  }
});

// #1617/#1742：路由连通和容器观测都不代替精确验收 receipt。
test("准入先于探针、占用后于探针；观测不成立不烧掉验收行", async () => {
  const { assertSwarmExecutionReady } = await import("./swarm-route.mts");
  const { violations } = await import("./docker-isolation.mts");
  const good = {
    apparmor: "docker-default (enforce)", seccomp: "2", capEff: "0000000000000000", capBnd: "00000000a80425fb",
    roWrite: "touch: cannot touch '/probe/ro/x': Read-only file system", rwWrite: "ok",
    roChildInRw: "Read-only file system", roFileAppend: "Read-only file system",
    renameMountpoint: "mv: Device or resource busy", hardlinkAcross: "ln: Invalid cross-device link",
    symlinkIntoRo: "Read-only file system",
  };
  assert.deepEqual(violations(good), []);
  const events: string[] = [];
  const admit = (image: string) => { events.push(`admit:${image}`); return { imageId: ID, consume: () => { events.push("consume"); } }; };
  const probe = (obs: object) => (image: string) => { events.push(`probe:${image === ID ? "<admitted-id>" : image}`); return obs as typeof good; };
  assert.equal(assertSwarmExecutionReady("fixture", { probe: probe(good), admit }), ID, "返回准入核对过的 Id，供启动容器原样使用");
  assert.deepEqual(events, ["admit:fixture", "probe:<admitted-id>", "consume"], "探针也针对准入 Id，而不是 tag");
  let seenCtx: unknown;
  assertSwarmExecutionReady("fixture", { probe: probe(good), admit: (image, c) => { seenCtx = c; return { imageId: ID, consume: () => undefined }; }, binding: "swarm-ticket:4242", rows: ["13"] });
  assert.deepEqual(seenCtx, { binding: "swarm-ticket:4242", rows: ["13"] });
  events.length = 0;
  assert.throws(() => assertSwarmExecutionReady("fixture", { probe: probe({ ...good, roWrite: "ok" }), admit }), /EROFS/);
  assert.deepEqual(events, ["admit:fixture", "probe:<admitted-id>"], "合同不成立时不占用验收行");
  events.length = 0;
  assert.throws(() => assertSwarmExecutionReady("fixture", { probe: probe(good), admit: () => { throw new Error("[FAIL-loud #1617] receipt 缺失"); } }), /receipt 缺失/);
  assert.deepEqual(events, [], "无准入时不启动任何探针容器");
  assert.throws(() => assertSwarmExecutionReady("fixture", { probe: () => { throw new Error("EPERM"); }, admit }), /FAIL-loud #1617/);
  assert.throws(() => assertSwarmExecutionReady("fixture", { probe: probe({ ...good, capBnd: "000001ffffffffff" }), admit }), /CapBnd/);
  assert.throws(() => assertSwarmExecutionReady("fixture", { probe: probe({ ...good, seccomp: "0" }), admit }), /Seccomp/);
  assert.deepEqual(violations({ ...good, capBnd: "0000000000000000" }), []);
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(source.indexOf("assertSwarmExecutionReady(IMAGE,") < source.indexOf("for (let iteration"));
});

test("默认入口不带 receipt 时即使观测全部成立也拒绝（不注入 admit）", async () => {
  const { assertSwarmExecutionReady } = await import("./swarm-route.mts");
  const previous = { ...process.env };
  process.env.HOME = mkdtempSync(join(tmpdir(), "swarm-route-home-"));
  delete process.env.SANDCASTLE_DOCKER_ACCEPTANCE_ROW;
  try {
    let probed = false;
    assert.throws(() => assertSwarmExecutionReady("fixture", { probe: () => { probed = true; throw new Error("不应到达"); } }), /尚无精确验收 receipt/);
    assert.equal(probed, false);
  } finally { process.env = previous; }
});

test("swarmSandbox 只接受准入核对过的 sha256 镜像 Id：tag 一律 FAIL-loud", () => {
  assert.throws(() => swarmSandbox("sandcastle:newchanlun", { CLIPROXY_LOCAL_KEY: "k" }), /sha256 镜像 Id/);
  assert.throws(() => swarmSandbox("sha256:abc", { CLIPROXY_LOCAL_KEY: "k" }), /sha256 镜像 Id/);
  assert.doesNotThrow(() => swarmSandbox(ID, { CLIPROXY_LOCAL_KEY: "k" }));
});

test("CI 等价（无 CLIPROXY_LOCAL_KEY、launchctl 也取不到）：传 tag 仍先报 sha256 Id 错误；传 Id 才轮到缺密钥", () => {
  const saved = process.env.CLIPROXY_LOCAL_KEY; delete process.env.CLIPROXY_LOCAL_KEY;
  const noKey = () => undefined; let lookups = 0;
  try {
    assert.throws(() => swarmSandbox("sandcastle:newchanlun", undefined, () => { lookups++; return undefined; }), /sha256 镜像 Id/);
    assert.equal(lookups, 0, "Id 校验先于密钥求值，tag 路径不触碰 launchctl");
    assert.throws(() => swarmSandbox("sha256:abc", undefined, noKey), /sha256 镜像 Id/);
    assert.throws(() => swarmSandbox(ID, undefined, noKey), /缺少 CLIPROXY_LOCAL_KEY/);
    assert.doesNotThrow(() => swarmSandbox(ID, undefined, () => "from-launchctl"));
  } finally { if (saved === undefined) delete process.env.CLIPROXY_LOCAL_KEY; else process.env.CLIPROXY_LOCAL_KEY = saved; }
});

test("三条 Docker 入口都用准入返回的 Id 启动：main-swarm（受控行 / 已登记放行共用 assertSwarmExecutionReady）与冲突工蜂", () => {
  const merge = readFileSync(new URL("./swarm-merge.mts", import.meta.url), "utf8");
  assert.match(merge, /const admittedId = assertSwarmExecutionReady\(image,/);
  assert.match(merge, /swarmSandbox\(launchImage,/);
  assert.doesNotMatch(merge, /swarmSandbox\(image\b/);
  const route = readFileSync(new URL("./swarm-route.mts", import.meta.url), "utf8");
  assert.match(route, /assertContainerIsolationContract\(admission\.imageId,/);
  for (const f of ["main-swarm.mts", "swarm-merge.mts", "swarm-route.mts"]) assert.doesNotMatch(readFileSync(new URL(`./${f}`, import.meta.url), "utf8"), /imageName: image\b|imageName: IMAGE\b/, f);
});

// —— 模型预检排在占用之前：预检失败不烧受控行 ——
import { mkdtempSync as mk2, readdirSync as ls2, existsSync as ex2 } from "node:fs";
import { tmpdir as td2 } from "node:os";
import { join as j2 } from "node:path";
import { assertSwarmExecutionReady, } from "./swarm-route.mts";
import { ROW_ENV, checkAdmission, issueReceipt, mergeBinding, swarmBinding } from "./docker-acceptance.mts";

const OBS = {
  apparmor: "docker-default (enforce)", seccomp: "2", capEff: "0000000000000000", capBnd: "00000000a80425fb",
  roWrite: "Read-only file system", rwWrite: "ok", roChildInRw: "Read-only file system", roFileAppend: "Read-only file system",
  renameMountpoint: "Device or resource busy", hardlinkAcross: "Invalid cross-device link", symlinkIntoRo: "Read-only file system",
};
const HEAD40 = "a".repeat(40); const CLEAN = { head: HEAD40, dirty: "" }; const imageIdOf = () => ID;
const claimFiles = (d: string) => ls2(d).filter((f) => /^(used|resume|cp)-/.test(f));

test("受控第 13 行：模型预检（含 502 overloaded 等瞬时故障）失败时不写 used-13，同一 receipt 随后仍可占用；成功才占用", () => {
  const d = mk2(j2(td2(), "route-order-")); issueReceipt({ imageIdOf, dir: d, image: "img", approval: "ok", swarmTicket: 4242, identity: CLEAN });
  const events: string[] = [];
  const acceptance = { imageIdOf, dir: d, env: { [ROW_ENV]: "13" }, identity: CLEAN };
  const run = (modelProbe: (id: string) => void) => assertSwarmExecutionReady("img", { rows: ["13"], binding: swarmBinding(4242), acceptance, probe: () => { events.push("isolation"); return OBS; }, modelProbe });
  assert.throws(() => run(() => { events.push("model"); throw new Error("HTTP 502: overloaded"); }), /overloaded/);
  assert.deepEqual(claimFiles(d), [], "预检失败：没有 used-13");
  assert.deepEqual(events, ["isolation", "model"], "顺序：隔离探针 → 模型预检");
  assert.throws(() => run(() => { throw new Error("quota exceeded"); }), /quota/);
  assert.deepEqual(claimFiles(d), []);
  events.length = 0;
  assert.equal(run((id) => { events.push(`model:${id === ID ? "admitted-id" : id}`); }), ID);
  assert.deepEqual(events, ["isolation", "model:admitted-id"], "预检用准入 Id");
  assert.equal(claimFiles(d).filter((f) => f.startsWith("used-")).length, 1, "预检成功后才占用");
  assert.throws(() => run(() => undefined), /一次性/, "占用后第 13 行仍是一次性的");
});

test("冲突路径 14/15：模型预检失败不写 used / resume / cp，随后仍可占用；占用后同一检查点重入才记 resume", () => {
  const d = mk2(j2(td2(), "route-order-")); issueReceipt({ imageIdOf, dir: d, image: "img", approval: "ok", swarmTicket: 4242, identity: CLEAN });
  const cp = (o: Partial<{ head: string; incoming: string }> = {}) => mergeBinding({ runId: "run-a", base: "b".repeat(40), instance: "inst-1", head: "c".repeat(40), incoming: "d".repeat(40), ...o });
  for (const row of ["14", "15"] as const) {
    const acceptance = { imageIdOf, dir: d, env: { [ROW_ENV]: row }, identity: CLEAN };
    const run = (binding: string, modelProbe: (id: string) => void) => assertSwarmExecutionReady("img", { rows: ["14", "15"], binding, acceptance, probe: () => OBS, modelProbe });
    assert.throws(() => run(cp(), () => { throw new Error("HTTP 503"); }), /503/);
    assert.deepEqual(claimFiles(d).filter((f) => f.includes(`-${row}`)), [], `第 ${row} 行预检失败：无 used/cp`);
    run(cp(), () => undefined);
    assert.equal(claimFiles(d).filter((f) => f.startsWith("used-") && f.includes(`-${row}`)).length, 1);
    // 同一检查点重入（新进程语义）：预检失败不新增 resume；成功才记 resume
    const before = claimFiles(d).length;
    assert.throws(() => run(cp(), () => { throw new Error("HTTP 502"); }), /502/);
    assert.equal(claimFiles(d).length, before, "预检失败不新增 resume");
    run(cp(), () => undefined);
    assert.equal(claimFiles(d).filter((f) => f.startsWith("resume-") && f.includes(`-${row}-`)).length, 1);
    // 新检查点：预检失败不新增 cp-
    assert.throws(() => run(cp({ incoming: "e".repeat(40) }), () => { throw new Error("HTTP 502"); }), /502/);
    assert.equal(claimFiles(d).filter((f) => f.startsWith("cp-") && f.includes(`-${row}-`)).length, 0);
  }
  void checkAdmission; void ex2;
});

test("源码断言：三条入口都不再在 consume 之后做模型预检——预检经 modelProbe 在 assertSwarmExecutionReady 内排在 consume 之前", () => {
  const route = readFileSync(new URL("./swarm-route.mts", import.meta.url), "utf8");
  const iso = route.indexOf("assertContainerIsolationContract(admission.imageId"), model = route.indexOf("options.modelProbe?.(admission.imageId)"), consume = route.indexOf("admission.consume()");
  assert.ok(iso > 0 && iso < model && model < consume, "隔离探针 → 模型预检 → consume");
  const main = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.match(main, /modelProbe: \(id\) => assertDockerCliProxyReachable\(id\)/);
  assert.doesNotMatch(main, /^assertDockerCliProxyReachable\(/m, "main-swarm 不再在 assertSwarmExecutionReady 之后单独预检");
  const merge = readFileSync(new URL("./swarm-merge.mts", import.meta.url), "utf8");
  assert.match(merge, /modelProbe: routeChecked \? undefined : \(id\) => \{ assertDockerCliProxyReachable\(id\); routeChecked = true; \}/);
  assert.ok(merge.indexOf("assertSwarmExecutionReady(image") < merge.indexOf("readiedCheckpoints.set(binding"));
  assert.doesNotMatch(merge, /\n\s*if \(!routeChecked\) \{ assertDockerCliProxyReachable/, "冲突路径不再有 consume 之后的预检");
});
