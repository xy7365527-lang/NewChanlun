import assert from "node:assert/strict";
import test, { type TestContext } from "node:test";

import {
  connectDaemonWebSocket,
  resolveDaemonWebSocketTarget,
} from "../src/hooks/daemonWebSocket.ts";

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

class FakeWebSocket {
  static instances: FakeWebSocket[] = [];

  onopen: ((event: Event) => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: Event) => void) | null = null;
  closeCalls = 0;
  readonly url: string;

  constructor(url: string) {
    this.url = url;
    FakeWebSocket.instances.push(this);
  }

  close(): void {
    this.closeCalls += 1;
  }

  emitOpen(): void {
    this.onopen?.(new Event("open"));
  }

  emitMessage(data: string): void {
    this.onmessage?.({ data });
  }

  emitError(): void {
    this.onerror?.(new Event("error"));
  }

  emitClose(): void {
    this.onclose?.(new Event("close"));
  }
}

function installFakeWebSocket(t: TestContext): void {
  const original = globalThis.WebSocket;
  FakeWebSocket.instances = [];
  globalThis.WebSocket = FakeWebSocket as unknown as typeof WebSocket;
  t.after(() => {
    globalThis.WebSocket = original;
  });
}

const wsUrl = `ws:${"//"}localhost:8765/ws`;

test("WebSocket target defaults to the first configured instance and preserves explicit overrides", () => {
  const defaults = [{ id: "local", wsUrl }];
  assert.deepEqual(resolveDaemonWebSocketTarget(defaults), {
    instanceId: "local",
    wsUrl,
  });

  const explicitWsUrl = `wss:${"//"}daemon.example.test/ws`;
  assert.deepEqual(resolveDaemonWebSocketTarget(defaults, {
    instanceId: "explicit",
    wsUrl: explicitWsUrl,
  }), {
    instanceId: "explicit",
    wsUrl: explicitWsUrl,
  });
});

test("WebSocket messages are throttled into one attributed batch", async (t) => {
  installFakeWebSocket(t);
  const batches: Array<{ id: string; messages: unknown[] }> = [];
  const states: Array<{ wsConnected: boolean; wsError: string | null }> = [];
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 50,
    describeError: () => "stream failed",
    onBatch: (id, messages) => batches.push({ id, messages }),
    onState: (_id, state) => states.push(state),
  });
  t.after(dispose);

  const socket = FakeWebSocket.instances[0];
  socket.emitOpen();
  socket.emitMessage(JSON.stringify({ type: "first", value: 1 }));
  socket.emitMessage("not-json");
  socket.emitMessage(JSON.stringify({ type: "second", value: 2 }));
  await waitFor(() => batches.length === 1);

  assert.deepEqual(states, [{ wsConnected: true, wsError: null }]);
  assert.equal(batches.length, 1);
  assert.equal(batches[0].id, "alpha");
  assert.deepEqual(batches[0].messages, [
    { type: "first", value: 1 },
    { type: "second", value: 2 },
  ]);
});

test("error and a later close share exactly one reconnect timer", async (t) => {
  installFakeWebSocket(t);
  const states: Array<{ wsConnected: boolean; wsError: string | null }> = [];
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 5,
    describeError: () => "stream failed",
    onBatch: () => {},
    onState: (_id, state) => states.push(state),
  });
  t.after(dispose);

  const first = FakeWebSocket.instances[0];
  first.emitError();
  first.emitClose();
  await waitFor(() => FakeWebSocket.instances.length === 2);

  assert.equal(first.closeCalls, 1);
  assert.equal(FakeWebSocket.instances.length, 2);
  assert.deepEqual(states, [{ wsConnected: false, wsError: "stream failed" }]);
});

test("dispose cancels timers, drops buffered messages, closes once, and ignores late events", async (t) => {
  installFakeWebSocket(t);
  let callbackCount = 0;
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 5,
    describeError: () => "stream failed",
    onBatch: () => { callbackCount += 1; },
    onState: () => { callbackCount += 1; },
  });

  const socket = FakeWebSocket.instances[0];
  socket.emitMessage(JSON.stringify({ type: "pending" }));
  dispose();
  dispose();
  assert.equal(socket.closeCalls, 1);

  socket.emitOpen();
  socket.emitMessage(JSON.stringify({ type: "late" }));
  socket.emitError();
  socket.emitClose();
  await wait(20);

  assert.equal(callbackCount, 0);
  assert.equal(FakeWebSocket.instances.length, 1);
});

test("dispose cancels a reconnect that was already scheduled by close", async (t) => {
  installFakeWebSocket(t);
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 5,
    describeError: () => "stream failed",
    onBatch: () => {},
    onState: () => {},
  });

  FakeWebSocket.instances[0].emitClose();
  dispose();
  await wait(20);
  assert.equal(FakeWebSocket.instances.length, 1);
});
