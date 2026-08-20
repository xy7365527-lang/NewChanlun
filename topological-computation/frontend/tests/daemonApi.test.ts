import assert from "node:assert/strict";
import test from "node:test";

import {
  createDaemonAPI,
  DaemonHttpError,
  DaemonJsonResponseError,
} from "../src/hooks/daemonApi.ts";

test("status forwards the caller's AbortSignal to the browser fetch boundary", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const baseUrl = "http://localhost:9765";
  const controller = new AbortController();
  let callCount = 0;

  globalThis.fetch = async (input, init) => {
    callCount += 1;
    assert.equal(String(input), `${baseUrl}/status`);
    assert.equal(init?.signal, controller.signal);
    return {
      ok: true,
      json: async () => ({ steps: 7 }),
    } as Response;
  };

  const status = await createDaemonAPI(baseUrl).status(controller.signal);
  assert.equal(callCount, 1);
  assert.equal(status.steps, 7);
});

test("HTTP status failures preserve the real response status and path", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  globalThis.fetch = async () => ({
    ok: false,
    status: 503,
    json: async () => ({ error: "unavailable" }),
  }) as Response;

  await assert.rejects(
    createDaemonAPI("https://daemon.example.test").status(),
    (error: unknown) => error instanceof DaemonHttpError
      && error.status === 503
      && error.path === "/status",
  );
});

test("JSON parse failures retain response metadata instead of becoming network failures", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  globalThis.fetch = async () => ({
    ok: true,
    status: 200,
    json: async () => { throw new SyntaxError("Unexpected token"); },
  }) as Response;

  await assert.rejects(
    createDaemonAPI("https://daemon.example.test").status(),
    (error: unknown) => error instanceof DaemonJsonResponseError
      && error.status === 200
      && error.path === "/status"
      && /Unexpected token/.test(error.message),
  );
});
