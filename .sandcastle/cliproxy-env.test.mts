import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  CLIPROXY_BASE_URL,
  cliProxyEnv,
  extractFirstApiKey,
  readCliProxyAccessKey,
} from "./cliproxy-env.mts";
import { MAIN_MODEL, SUPPORTED_MAIN_MODELS } from "./main-model.mts";

test("主控模型只有单一配置值", () => {
  assert.equal(MAIN_MODEL, "gpt-6-astra");
  assert.deepEqual(SUPPORTED_MAIN_MODELS, [MAIN_MODEL]);
});

test("CLIProxy 配置只取 access.api-keys 首项", () => {
  const yaml = "server:\n  api-keys:\n    - wrong\naccess:\n  api-keys:\n    - \"first-key\"\n    - second\n";
  assert.equal(extractFirstApiKey(yaml), "first-key");
  assert.equal(extractFirstApiKey("access:\n  api-keys: [abc, def]\n"), "abc");
  assert.equal(extractFirstApiKey("access:\n  other: true\n"), null);
});

test("cliProxyEnv 从指定配置读取并只返回路由变量", () => {
  const dir = mkdtempSync(join(tmpdir(), "cliproxy-env-test-"));
  try {
    const config = join(dir, "config.yaml");
    writeFileSync(config, "access:\n  api-keys: [fixture-secret]\n");
    assert.equal(readCliProxyAccessKey(config), "fixture-secret");
    assert.deepEqual(cliProxyEnv(config), {
      ANTHROPIC_BASE_URL: CLIPROXY_BASE_URL,
      ANTHROPIC_AUTH_TOKEN: "fixture-secret",
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
