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

test("polling remains compatible when AbortSignal.any and AbortSignal.timeout are unavailable", async (t) => {
  const originalFetch = globalThis.fetch;
  const anyDescriptor = Object.getOwnPropertyDescriptor(AbortSignal, "any");
  const timeoutDescriptor = Object.getOwnPropertyDescriptor(AbortSignal, "timeout");
  Object.defineProperty(AbortSignal, "any", { configurable: true, value: undefined });
  Object.defineProperty(AbortSignal, "timeout", { configurable: true, value: undefined });
  t.after(() => {
    globalThis.fetch = originalFetch;
    if (anyDescriptor) Object.defineProperty(AbortSignal, "any", anyDescriptor);
    else Reflect.deleteProperty(AbortSignal, "any");
    if (timeoutDescriptor) Object.defineProperty(AbortSignal, "timeout", timeoutDescriptor);
    else Reflect.deleteProperty(AbortSignal, "timeout");
  });

  let fetchCalls = 0;
  globalThis.fetch = async (_input, init) => {
    fetchCalls += 1;
    assert.ok(init?.signal);
    return response({ steps: fetchCalls });
  };

  const cycles: boolean[] = [];
  const stop = startStatusPolling({
    instances: instances.slice(0, 1),
    intervalMs: 1_000,
    timeoutMs: 1_000,
    onStatus: () => {},
    onError: () => {},
    onCycleComplete: (reachable) => cycles.push(reachable),
  });
  t.after(stop);

  await waitFor(() => cycles.length === 1);
  assert.equal(fetchCalls, 1, "the first request still reaches fetch without newer AbortSignal APIs");
  assert.deepEqual(cycles, [true]);
});

test("a transient polling error settles before the next non-overlapping cycle", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  let fetchCalls = 0;
  let activeFetches = 0;
  let maximumActiveFetches = 0;
  globalThis.fetch = async () => {
    fetchCalls += 1;
    activeFetches += 1;
    maximumActiveFetches = Math.max(maximumActiveFetches, activeFetches);
    try {
      if (fetchCalls === 1) throw new TypeError("temporary network failure");
      return response({ steps: fetchCalls });
    } finally {
      activeFetches -= 1;
    }
  };

  const errors: unknown[] = [];
  const cycles: boolean[] = [];
  const stop = startStatusPolling({
    instances: instances.slice(0, 1),
    intervalMs: 5,
    timeoutMs: 1_000,
    onStatus: () => {},
    onError: (_instance, error) => errors.push(error),
    onCycleComplete: (reachable) => cycles.push(reachable),
  });
  t.after(stop);

  await waitFor(() => fetchCalls >= 2 && cycles.length >= 2);
  assert.equal(errors.length, 1);
  assert.deepEqual(cycles.slice(0, 2), [false, true]);
  assert.equal(maximumActiveFetches, 1);
});

test("callback failures cannot overlap an unfinished peer request or stop later cycles", async (t) => {
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });

  const slowPeer = deferred<Response>();
  let fetchCalls = 0;
  globalThis.fetch = async () => {
    fetchCalls += 1;
    if (fetchCalls === 1) throw new Error("alpha failed");
    if (fetchCalls === 2) return slowPeer.promise;
    return response({ steps: fetchCalls });
  };

  const stop = startStatusPolling({
    instances,
    intervalMs: 5,
    timeoutMs: 1_000,
    onStatus: () => {},
    onError: () => { throw new Error("consumer failed"); },
    onCycleComplete: () => {},
  });
  t.after(stop);

  await wait(20);
  assert.equal(fetchCalls, 2, "the next cycle waits for every request despite callback errors");
  slowPeer.resolve(response({ steps: 2 }));
  await waitFor(() => fetchCalls >= 4);
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
