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

// #1617：路由连通不代表 Docker 工具可运行。
test("默认 namespace 失败时在 planner 和 claim 前阻塞，不永久禁用 swarm", async () => {
  const { assertSwarmExecutionReady } = await import("./swarm-route.mts");
  assert.throws(() => assertSwarmExecutionReady("fixture", () => { throw new Error("EPERM"); }), /namespace/);
  let seen = ""; assertSwarmExecutionReady("fixture", image => { seen = image; }); assert.equal(seen, "fixture");
  const source = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.ok(source.indexOf("assertSwarmExecutionReady(IMAGE)") < source.indexOf("for (let iteration"));
});
