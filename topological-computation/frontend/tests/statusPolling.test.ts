import assert from "node:assert/strict";
import test from "node:test";

import {
  startStatusPolling,
  type StatusPollingInstance,
} from "../src/hooks/statusPolling.ts";

interface Deferred<T> {
  promise: Promise<T>;
  resolve: (value: T) => void;
  reject: (reason?: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function response(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as Response;
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor(predicate: () => boolean, timeoutMs = 1_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) {
      assert.fail(`condition was not met within ${timeoutMs}ms`);
    }
    await wait(5);
  }
}

const instances: StatusPollingInstance[] = [
  { id: "alpha", httpBase: `http:${"//"}localhost:9765` },
  { id: "beta", httpBase: `http:${"//"}127.0.0.1:9766` },
];

test("status polling starts immediately, polls peers concurrently, and never overlaps cycles", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const requests: Array<Deferred<Response> & { signal: AbortSignal }> = [];
  globalThis.fetch = (_input, init) => {
    assert.ok(init?.signal);
    const request = deferred<Response>();
    const pending = { ...request, signal: init.signal };
    init.signal.addEventListener("abort", () => request.reject(init.signal?.reason), { once: true });
    requests.push(pending);
    return request.promise;
  };

  const statuses: Array<{ id: string; first: boolean }> = [];
  const cycleReachability: boolean[] = [];
  const stop = startStatusPolling({
    instances,
    intervalMs: 5,
    timeoutMs: 1_000,
    onStatus: (instance, _status, firstReachable) => {
      statuses.push({ id: instance.id, first: firstReachable });
    },
    onError: () => {},
    onCycleComplete: (reachable) => cycleReachability.push(reachable),
  });
  t.after(stop);

  assert.equal(requests.length, 2, "both peers start in the immediate first cycle");
  await wait(20);
  assert.equal(requests.length, 2, "an unresolved cycle does not overlap itself");

  requests[0].resolve(response({ steps: 1 }));
  requests[1].resolve(response({ steps: 2 }));
  await waitFor(() => (
    statuses.length === 2
    && cycleReachability.length === 1
    && requests.length === 4
  ));

  assert.equal(statuses.length, 2);
  assert.equal(statuses.filter((entry) => entry.first).length, 1);
  assert.deepEqual(cycleReachability, [true]);
  assert.equal(requests.length, 4, "the next cycle starts only after the first settles");
  stop();
  assert.equal(requests[2].signal.aborted, true);
  assert.equal(requests[3].signal.aborted, true);
});

test("a status cycle times out through AbortSignal and reports one failed cycle", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  let capturedSignal: AbortSignal | null = null;
  globalThis.fetch = (_input, init) => new Promise<Response>((_resolve, reject) => {
    assert.ok(init?.signal);
    capturedSignal = init.signal;
    init.signal.addEventListener("abort", () => reject(init.signal?.reason), { once: true });
  });

  const errors: unknown[] = [];
  const cycles: boolean[] = [];
  const stop = startStatusPolling({
    instances: instances.slice(0, 1),
    intervalMs: 1_000,
    timeoutMs: 10,
    onStatus: () => {},
    onError: (_instance, error) => errors.push(error),
    onCycleComplete: (reachable) => cycles.push(reachable),
  });
  t.after(stop);

  await waitFor(() => (
    capturedSignal?.aborted === true
    && errors.length === 1
    && cycles.length === 1
  ));
  assert.equal(capturedSignal?.aborted, true);
  assert.equal(errors.length, 1);
  assert.deepEqual(cycles, [false]);
});

test("dispose aborts the active signal and suppresses late results and future cycles", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const request = deferred<Response>();
  let capturedSignal: AbortSignal | null = null;
  let callCount = 0;
  globalThis.fetch = (_input, init) => {
    assert.ok(init?.signal);
    capturedSignal = init.signal;
    callCount += 1;
    return request.promise;
  };

  let callbackCount = 0;
  const stop = startStatusPolling({
    instances: instances.slice(0, 1),
    intervalMs: 5,
    timeoutMs: 1_000,
    onStatus: () => { callbackCount += 1; },
    onError: () => { callbackCount += 1; },
    onCycleComplete: () => { callbackCount += 1; },
  });

  stop();
  assert.equal(capturedSignal?.aborted, true);
  request.resolve(response({ steps: 99 }));
  await wait(20);
  assert.equal(callbackCount, 0);
  assert.equal(callCount, 1);
});
