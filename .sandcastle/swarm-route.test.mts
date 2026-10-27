import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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

test("swarm 四角色共用主控模型，provider 命令不显式换档", () => {
  const provider = swarmAgent();
  const command = provider.buildPrintCommand({ prompt: "hi", dangerouslySkipPermissions: true });
  assert.equal(MAIN_MODEL, "gpt-6-astra");
  assert.ok(command.command.includes(MAIN_MODEL));
  assert.ok(!command.command.includes("prime-agent"));
});

test("swarm Docker 使用宿主网关而非容器回环，密钥只存在运行时 env", () => {
  const env = { CLIPROXY_BASE_URL: DOCKER_CLIPROXY_BASE_URL, CLIPROXY_LOCAL_KEY: "fixture-secret" };
  const sandbox = swarmSandbox("fixture-image", env);
  assert.equal(sandbox.env.CLIPROXY_BASE_URL, "http://host.docker.internal:8318");
  assert.equal(sandbox.env.CLIPROXY_LOCAL_KEY, "fixture-secret");
  assert.ok(!DOCKER_CLIPROXY_BASE_URL.includes("127.0.0.1"));
});

test("main-swarm 不再实例化 prime-agent/k3，planner/单票沙盒统一走路由 helper", () => {
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(!source.includes("primeAgent("));
  assert.ok(!source.includes('"k3"'));
  assert.equal(source.match(/swarmSandbox\(IMAGE\)/g)?.length, 2);
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
  assert.ok(source.indexOf("assertDockerCliProxyReachable(IMAGE)") < source.indexOf("for (let iteration"));
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
  const admit = (image: string) => { events.push(`admit:${image}`); return { consume: () => { events.push("consume"); } }; };
  const probe = (obs: object) => (image: string) => { events.push(`probe:${image}`); return obs as typeof good; };
  assertSwarmExecutionReady("fixture", probe(good), admit);
  assert.deepEqual(events, ["admit:fixture", "probe:fixture", "consume"]);
  events.length = 0;
  assert.throws(() => assertSwarmExecutionReady("fixture", probe({ ...good, roWrite: "ok" }), admit), /EROFS/);
  assert.deepEqual(events, ["admit:fixture", "probe:fixture"], "合同不成立时不占用验收行");
  events.length = 0;
  assert.throws(() => assertSwarmExecutionReady("fixture", probe(good), () => { throw new Error("[FAIL-loud #1617] receipt 缺失"); }), /receipt 缺失/);
  assert.deepEqual(events, [], "无准入时不启动任何探针容器");
  assert.throws(() => assertSwarmExecutionReady("fixture", () => { throw new Error("EPERM"); }, admit), /FAIL-loud #1617/);
  assert.throws(() => assertSwarmExecutionReady("fixture", probe({ ...good, capBnd: "000001ffffffffff" }), admit), /CapBnd/);
  assert.throws(() => assertSwarmExecutionReady("fixture", probe({ ...good, seccomp: "0" }), admit), /Seccomp/);
  assert.deepEqual(violations({ ...good, capBnd: "0000000000000000" }), []);
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(source.indexOf("assertSwarmExecutionReady(IMAGE)") < source.indexOf("for (let iteration"));
});

test("默认入口不带 receipt 时即使观测全部成立也拒绝（不注入 admit）", async () => {
  const { assertSwarmExecutionReady } = await import("./swarm-route.mts");
  const previous = { ...process.env };
  process.env.SANDCASTLE_DOCKER_ACCEPTANCE_DIR = "/nonexistent-acceptance-dir";
  delete process.env.SANDCASTLE_DOCKER_ACCEPTANCE_ROW;
  try {
    let probed = false;
    assert.throws(() => assertSwarmExecutionReady("fixture", () => { probed = true; throw new Error("不应到达"); }), /尚无精确验收 receipt/);
    assert.equal(probed, false);
  } finally { process.env = previous; }
});
