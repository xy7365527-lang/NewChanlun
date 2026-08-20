import assert from "node:assert/strict";
import test from "node:test";

import {
  removeConfiguredInstanceState,
  updateConfiguredInstanceState,
  type ConnectionStateFields,
} from "../src/hooks/connectionState.ts";

interface TestInstanceState extends ConnectionStateFields {
  marker: string;
}

const instances = [
  { id: "alpha", name: "Alpha" },
  { id: "beta", name: "Beta" },
];

function emptyState(): TestInstanceState {
  return {
    wsConnected: false,
    httpError: null,
    wsError: null,
    connectionError: null,
    marker: "",
  };
}

test("connection aggregation is order independent and stays connected while any configured WebSocket is open", () => {
  let aggregate = updateConfiguredInstanceState(
    instances,
    {},
    "alpha",
    emptyState,
    { wsConnected: true },
  );
  assert.equal(aggregate.wsConnected, true);

  aggregate = updateConfiguredInstanceState(
    instances,
    aggregate.instanceStates,
    "beta",
    emptyState,
    { wsConnected: false, wsError: "beta websocket failed" },
  );
  assert.equal(aggregate.wsConnected, true);
  assert.match(aggregate.connectionError ?? "", /Beta.*beta websocket failed/);

  aggregate = updateConfiguredInstanceState(
    instances,
    aggregate.instanceStates,
    "alpha",
    emptyState,
    { wsConnected: false },
  );
  assert.equal(aggregate.wsConnected, false);
});

test("HTTP and WebSocket recovery clear only their own channel error", () => {
  let aggregate = updateConfiguredInstanceState(
    instances,
    {},
    "alpha",
    emptyState,
    { httpError: "status failed", wsError: "stream failed" },
  );
  assert.match(aggregate.instanceStates.alpha.connectionError ?? "", /status failed/);
  assert.match(aggregate.instanceStates.alpha.connectionError ?? "", /stream failed/);

  aggregate = updateConfiguredInstanceState(
    instances,
    aggregate.instanceStates,
    "alpha",
    emptyState,
    { httpError: null },
  );
  assert.equal(aggregate.instanceStates.alpha.connectionError, "stream failed");
  assert.match(aggregate.connectionError ?? "", /stream failed/);

  aggregate = updateConfiguredInstanceState(
    instances,
    aggregate.instanceStates,
    "alpha",
    emptyState,
    { wsError: null, wsConnected: true },
  );
  assert.equal(aggregate.instanceStates.alpha.connectionError, null);
  assert.equal(aggregate.connectionError, null);
});

test("removal recomputes aggregates and late updates cannot recreate a removed instance", () => {
  const connected = updateConfiguredInstanceState(
    instances,
    {},
    "alpha",
    emptyState,
    { wsConnected: true, wsError: "old stream error" },
  );
  const remainingInstances = instances.filter((instance) => instance.id !== "alpha");
  const removed = removeConfiguredInstanceState(
    remainingInstances,
    connected.instanceStates,
    "alpha",
  );

  assert.equal(removed.wsConnected, false);
  assert.equal(removed.connectionError, null);
  assert.equal("alpha" in removed.instanceStates, false);

  const late = updateConfiguredInstanceState(
    remainingInstances,
    removed.instanceStates,
    "alpha",
    emptyState,
    { wsConnected: false, wsError: "late close" },
  );
  assert.equal("alpha" in late.instanceStates, false);
  assert.equal(late.connectionError, null);
});
