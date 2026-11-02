import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** 真实容器测试（build / rmi / 起容器）必须显式开启 SANDCASTLE_REAL_DOCKER_TESTS=1；默认套件一律 skip 且不调用 docker。
 * 单独成文件：子进程运行含真实容器测试的文件，不能把本测试放进它们自己里面（会递归）。 */
test("真实容器测试默认关闭：不设 SANDCASTLE_REAL_DOCKER_TESTS 时整体运行相关测试文件也不调用 docker", () => {
  const bin = mkdtempSync(join(tmpdir(), "no-docker-")); const calls = join(bin, "calls");
  writeFileSync(join(bin, "docker"), `#!/bin/sh\necho "$*" >> "${calls}"\nexit 99\n`, { mode: 0o755 });
  const env: NodeJS.ProcessEnv = { ...process.env, PATH: `${bin}:${process.env.PATH}` }; delete env.SANDCASTLE_REAL_DOCKER_TESTS; delete env.NODE_TEST_CONTEXT;
  const here = fileURLToPath(new URL(".", import.meta.url));
  const out = execFileSync(process.execPath, ["--import", import.meta.resolve("tsx"), "--test", join(here, "docker-isolation.test.mts"), join(here, "docker-acceptance.test.mts")], { env, encoding: "utf8", timeout: 120_000, stdio: ["ignore", "pipe", "pipe"] });
  assert.match(out, /# fail 0/); assert.match(out, /# skipped [4-9]/, "docker-isolation 的 3 项与 docker-acceptance 的 1 项真实 docker 测试被 skip");
  assert.ok(!existsSync(calls), existsSync(calls) ? readFileSync(calls, "utf8") : "");
});
