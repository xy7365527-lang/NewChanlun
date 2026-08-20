import assert from "node:assert/strict";
import test from "node:test";

import { createDaemonAPI } from "../src/hooks/daemonApi.ts";

test("status forwards the caller's AbortSignal to the browser fetch boundary", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const baseUrl = `http:${"//"}localhost:9765`;
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
