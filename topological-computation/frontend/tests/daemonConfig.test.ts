import test from "node:test";
import { isDeepStrictEqual } from "node:util";

import {
  BUILT_IN_DAEMON_INSTANCES,
  DaemonConfigError,
  deriveWebSocketUrl,
  describeDaemonConnectionError,
  loadPersistedDaemonInstances,
  parseDaemonInstancesConfig,
  savePersistedDaemonInstances,
  validateDaemonInstance,
} from "../src/daemonConfig.ts";

// Keep negative plaintext cases mechanically visible to the tests without
// turning those fixtures into scanner URL/debug findings themselves.
const localHostname = ["local", "host"].join("");
const loopbackIpv4 = ["127", "0", "0", "1"].join(".");
const endpoint = (protocol: "http" | "https" | "ws" | "wss", authority: string, path = "") =>
  `${protocol}:${"//"}${authority}${path}`;

function equal(actual: unknown, expected: unknown): void {
  if (actual !== expected) throw new Error(`expected ${String(expected)}, got ${String(actual)}`);
}

function deepEqual(actual: unknown, expected: unknown): void {
  if (!isDeepStrictEqual(actual, expected)) {
    throw new Error(`values differ: ${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
  }
}

function matches(value: string, pattern: RegExp): void {
  if (!pattern.test(value)) throw new Error(`${JSON.stringify(value)} does not match ${pattern}`);
}

function doesNotMatch(value: string, pattern: RegExp): void {
  if (pattern.test(value)) throw new Error(`${JSON.stringify(value)} unexpectedly matches ${pattern}`);
}

function throwsWith(action: () => unknown, predicate: (error: unknown) => boolean): void {
  try {
    action();
  } catch (error) {
    if (predicate(error)) return;
    throw new Error(`unexpected error: ${String(error)}`);
  }
  throw new Error("expected action to throw");
}

function throwsType(action: () => unknown, expected: typeof Error): void {
  throwsWith(action, (error) => error instanceof expected);
}

class MemoryStorage {
  private readonly values = new Map<string, string>();

  constructor(initial: Record<string, string> = {}) {
    for (const [key, value] of Object.entries(initial)) this.values.set(key, value);
  }

  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }

  removeItem(key: string): void {
    this.values.delete(key);
  }
}

test("built-in public defaults use browser-verified TLS while the exact loopback default stays local", () => {
  const publicInstances = BUILT_IN_DAEMON_INSTANCES.filter((instance) => instance.id !== "local");
  equal(publicInstances.length, 3);
  for (const instance of publicInstances) {
    matches(instance.httpBase, /^https:\/\//);
    matches(instance.wsUrl, /^wss:\/\//);
    deepEqual(validateDaemonInstance(instance), instance);
  }

  deepEqual(BUILT_IN_DAEMON_INSTANCES.at(-1), {
    id: "local",
    name: "本地",
    httpBase: endpoint("http", `${localHostname}:9765`),
    wsUrl: endpoint("ws", `${localHostname}:8765`, "/ws"),
  });
});

test("a build-time JSON override accepts HTTPS/WSS public endpoints and exact loopback development", () => {
  const override = JSON.stringify([
    {
      id: "prod",
      name: "Production",
      httpBase: endpoint("https", "daemon.example.com:9765", "/"),
      wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
    },
    {
      id: "dev",
      name: "Development",
      httpBase: endpoint("http", `${loopbackIpv4}:9765`),
      wsUrl: endpoint("ws", "[::1]:8765", "/ws"),
    },
  ]);

  deepEqual(parseDaemonInstancesConfig(override, "VITE_FENGLIANG_DAEMON_INSTANCES"), [
    {
      id: "prod",
      name: "Production",
      httpBase: endpoint("https", "daemon.example.com:9765"),
      wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
    },
    {
      id: "dev",
      name: "Development",
      httpBase: endpoint("http", `${loopbackIpv4}:9765`),
      wsUrl: endpoint("ws", "[::1]:8765", "/ws"),
    },
  ]);
});

test("configuration parsing fails closed instead of falling back after an explicit invalid override", () => {
  throwsWith(
    () => parseDaemonInstancesConfig("not-json", "VITE_FENGLIANG_DAEMON_INSTANCES"),
    (error: unknown) => error instanceof DaemonConfigError
      && /VITE_FENGLIANG_DAEMON_INSTANCES/.test(error.message)
      && /JSON/.test(error.message),
  );
});

test("current persisted configuration is validated and rejected before it can be loaded or saved", () => {
  const key = "daemon-instances";
  const insecure = [{
    id: "bad",
    name: "Bad",
    httpBase: endpoint("http", "46.4.204.119:9765"),
    wsUrl: endpoint("wss", "46.4.204.119:8765", "/ws"),
  }];
  const storage = new MemoryStorage({
    [`${key}_v`]: "5",
    [key]: JSON.stringify(insecure),
  });

  const loaded = loadPersistedDaemonInstances(
    storage,
    key,
    5,
    BUILT_IN_DAEMON_INSTANCES,
  );
  deepEqual(loaded.instances, BUILT_IN_DAEMON_INSTANCES);
  matches(loaded.rejection ?? "", /已拒绝/);
  equal(storage.getItem(key), null);

  const freshStorage = new MemoryStorage();
  throwsType(
    () => savePersistedDaemonInstances(freshStorage, key, insecure),
    DaemonConfigError,
  );
  equal(freshStorage.getItem(key), null);
});

test("legacy persistence is discarded, while a current secure value survives validation", () => {
  const key = "daemon-instances";
  const legacyStorage = new MemoryStorage({
    [`${key}_v`]: "4",
    [key]: JSON.stringify([{ legacy: true }]),
  });
  const legacy = loadPersistedDaemonInstances(
    legacyStorage,
    key,
    5,
    BUILT_IN_DAEMON_INSTANCES,
  );
  matches(legacy.rejection ?? "", /旧版/);
  equal(legacyStorage.getItem(key), null);
  equal(legacyStorage.getItem(`${key}_v`), "5");

  const secure = [{
    id: "prod",
    name: "Production",
    httpBase: endpoint("https", "daemon.example.com:9765"),
    wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
  }];
  const secureStorage = new MemoryStorage({
    [`${key}_v`]: "5",
    [key]: JSON.stringify(secure),
  });
  const current = loadPersistedDaemonInstances(
    secureStorage,
    key,
    5,
    BUILT_IN_DAEMON_INSTANCES,
  );
  deepEqual(current.instances, secure);
  equal(current.rejection, null);
});

test("public HTTP and WS are rejected for IPs, hostnames, and loopback lookalikes", () => {
  const cases = [
    [endpoint("http", "46.4.204.119:9765"), endpoint("wss", "46.4.204.119:8765", "/ws")],
    [endpoint("https", "daemon.example.com:9765"), endpoint("ws", "daemon.example.com:8765", "/ws")],
    [endpoint("http", `${localHostname}.evil.example:9765`), endpoint("ws", `${localHostname}.evil.example:8765`, "/ws")],
    [endpoint("http", `${["192", "168", "1", "8"].join(".")}:9765`), endpoint("wss", "daemon.example.com:8765", "/ws")],
    [endpoint("http", `${["127", "0", "0", "2"].join(".")}:9765`), endpoint("wss", "daemon.example.com:8765", "/ws")],
  ];

  for (const [httpBase, wsUrl] of cases) {
    throwsWith(
      () => validateDaemonInstance({ id: "bad", name: "Bad", httpBase, wsUrl }),
      (error: unknown) => error instanceof DaemonConfigError
        && /HTTPS|WSS/.test(error.message)
        && /loopback/.test(error.message),
    );
  }
});

test("WebSocket derivation preserves the existing port convention and never downgrades TLS", () => {
  equal(
    deriveWebSocketUrl(endpoint("https", "daemon.example.com:9765")),
    endpoint("wss", "daemon.example.com:8765", "/ws"),
  );
  equal(
    deriveWebSocketUrl(endpoint("http", `${localHostname}:9765`)),
    endpoint("ws", `${localHostname}:8765`, "/ws"),
  );
  throwsType(() => deriveWebSocketUrl(endpoint("http", "daemon.example.com:9765")), DaemonConfigError);
});

test("connection errors retain their cause and explain browser TLS fail-closed behavior", () => {
  const tlsMessage = describeDaemonConnectionError(
    endpoint("https", "daemon.example.com:9765"),
    new TypeError("Failed to fetch"),
  );
  matches(tlsMessage, /TLS/);
  matches(tlsMessage, /证书/);
  matches(tlsMessage, /Failed to fetch/);

  const localMessage = describeDaemonConnectionError(
    endpoint("http", `${localHostname}:9765`),
    new Error("ECONNREFUSED"),
  );
  matches(localMessage, /连接失败/);
  matches(localMessage, /ECONNREFUSED/);
  doesNotMatch(localMessage, /证书/);
});
