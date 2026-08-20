import assert from "node:assert/strict";
import test, { type TestContext } from "node:test";

import {
  connectDaemonWebSocket,
  disposeDaemonWebSocketConnections,
  reconcileDaemonWebSocketConnections,
  resolveDaemonWebSocketTarget,
  type DaemonWebSocketConnections,
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
  onclose: ((event: CloseEvent) => void) | null = null;
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

  emitClose(
    code = 1006,
    reason = "",
    wasClean = false,
  ): void {
    this.onclose?.({ code, reason, wasClean } as CloseEvent);
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

const wsUrl = "ws://localhost:8765/ws";

test("WebSocket target defaults to the first configured instance and preserves explicit overrides", () => {
  const defaults = [{ id: "local", wsUrl }];
  assert.deepEqual(resolveDaemonWebSocketTarget(defaults), {
    instanceId: "local",
    wsUrl,
  });

  const explicitWsUrl = "wss://daemon.example.test/ws";
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

test("dispose flushes buffered messages before close, closes once, and ignores late events", async (t) => {
  installFakeWebSocket(t);
  const batches: unknown[][] = [];
  let stateCallbackCount = 0;
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 5,
    describeError: () => "stream failed",
    onBatch: (_id, messages) => { batches.push(messages); },
    onState: () => { stateCallbackCount += 1; },
  });

  const socket = FakeWebSocket.instances[0];
  socket.emitMessage(JSON.stringify({ type: "pending" }));
  dispose();
  dispose();
  assert.deepEqual(batches, [[{ type: "pending" }]]);
  assert.equal(socket.closeCalls, 1);

  socket.emitOpen();
  socket.emitMessage(JSON.stringify({ type: "late" }));
  socket.emitError();
  socket.emitClose();
  await wait(20);

  assert.equal(stateCallbackCount, 0);
  assert.deepEqual(batches, [[{ type: "pending" }]]);
  assert.equal(FakeWebSocket.instances.length, 1);
});

test("a normal WebSocket close clears connected state without reporting an error", async (t) => {
  installFakeWebSocket(t);
  const states: Array<{ wsConnected: boolean; wsError: string | null }> = [];
  let describeCalls = 0;
  const dispose = connectDaemonWebSocket({
    wsUrl,
    instanceId: "alpha",
    throttleMs: 5,
    reconnectMs: 50,
    describeError: () => {
      describeCalls += 1;
      return "stream failed";
    },
    onBatch: () => {},
    onState: (_id, state) => states.push(state),
  });
  t.after(dispose);

  const socket = FakeWebSocket.instances[0];
  socket.emitOpen();
  socket.emitClose(1000, "normal shutdown", true);

  assert.deepEqual(states, [
    { wsConnected: true, wsError: null },
    { wsConnected: false, wsError: null },
  ]);
  assert.equal(describeCalls, 0);
});

test("instance churn retains stable sockets and their throttled messages", async (t) => {
  installFakeWebSocket(t);
  const connections: DaemonWebSocketConnections = new Map();
  const batches: Array<{ id: string; messages: unknown[] }> = [];
  const alphaUrl = wsUrl;
  const betaUrl = "ws://localhost:8766/ws";
  const replacementAlphaUrl = "ws://localhost:8767/ws";

  const connect = (target: { id: string; wsUrl: string }) => connectDaemonWebSocket({
    wsUrl: target.wsUrl,
    instanceId: target.id,
    throttleMs: 10,
    reconnectMs: 1_000,
    describeError: () => "stream failed",
    onBatch: (id, messages) => batches.push({ id, messages }),
    onState: () => {},
  });

  reconcileDaemonWebSocketConnections(
    connections,
    [{ id: "alpha", wsUrl: alphaUrl }],
    connect,
  );
  const originalAlpha = FakeWebSocket.instances[0];
  originalAlpha.emitMessage(JSON.stringify({ type: "pending-alpha" }));

  reconcileDaemonWebSocketConnections(
    connections,
    [
      { id: "alpha", wsUrl: alphaUrl },
      { id: "beta", wsUrl: betaUrl },
    ],
    connect,
  );
  const beta = FakeWebSocket.instances[1];
  assert.equal(originalAlpha.closeCalls, 0, "adding a peer keeps the stable alpha socket");
  assert.equal(FakeWebSocket.instances.length, 2);
  await waitFor(() => batches.length === 1);
  assert.deepEqual(batches, [{
    id: "alpha",
    messages: [{ type: "pending-alpha" }],
  }]);

  reconcileDaemonWebSocketConnections(
    connections,
    [
      { id: "alpha", wsUrl: replacementAlphaUrl },
      { id: "beta", wsUrl: betaUrl },
    ],
    connect,
  );
  assert.equal(originalAlpha.closeCalls, 1, "a changed endpoint replaces its old socket");
  assert.equal(beta.closeCalls, 0, "an unchanged peer remains connected");
  assert.equal(FakeWebSocket.instances.length, 3);

  reconcileDaemonWebSocketConnections(
    connections,
    [{ id: "beta", wsUrl: betaUrl }],
    connect,
  );
  assert.equal(FakeWebSocket.instances[2].closeCalls, 1, "a removed socket is closed");
  assert.equal(beta.closeCalls, 0);

  disposeDaemonWebSocketConnections(connections);
  assert.equal(beta.closeCalls, 1);
  assert.equal(connections.size, 0);
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
