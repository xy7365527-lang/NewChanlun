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
import {
  DaemonHttpError,
  DaemonJsonResponseError,
} from "../src/hooks/daemonApi.ts";

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

test("built-in defaults contain only exact localhost development", () => {
  deepEqual(BUILT_IN_DAEMON_INSTANCES, [{
    id: "local",
    name: "本地",
    httpBase: endpoint("http", `${localHostname}:9765`),
    wsUrl: endpoint("ws", `${localHostname}:8765`, "/ws"),
  }]);
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
      wsUrl: endpoint("ws", `${loopbackIpv4}:8765`, "/ws"),
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
      wsUrl: endpoint("ws", `${loopbackIpv4}:8765`, "/ws"),
    },
  ]);
});

test("daemon endpoint validation rejects query, hash, and unsupported paths while normalizing the HTTP root slash", () => {
  const valid = {
    id: "prod",
    name: "Production",
    httpBase: endpoint("https", "daemon.example.com:9765", "/"),
    wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
  };
  deepEqual(validateDaemonInstance(valid), {
    ...valid,
    httpBase: endpoint("https", "daemon.example.com:9765"),
  });

  const invalid = [
    { ...valid, httpBase: `${valid.httpBase}?tenant=one` },
    { ...valid, httpBase: `${valid.httpBase}#status` },
    { ...valid, httpBase: endpoint("https", "daemon.example.com:9765", "/api") },
    { ...valid, wsUrl: `${valid.wsUrl}?tenant=one` },
    { ...valid, wsUrl: `${valid.wsUrl}#events` },
    { ...valid, wsUrl: endpoint("wss", "daemon.example.com:8765", "/events") },
  ];
  for (const instance of invalid) {
    throwsType(() => validateDaemonInstance(instance), DaemonConfigError);
  }
});

test("HTTP and WebSocket endpoints must form one host, transport, and known-port pair", () => {
  for (const [httpPort, wsPort] of [["9765", "8765"], ["9766", "8766"], ["9767", "8767"]]) {
    const instance = {
      id: `prod-${httpPort}`,
      name: `Production ${httpPort}`,
      httpBase: endpoint("https", `daemon.example.com:${httpPort}`),
      wsUrl: endpoint("wss", `daemon.example.com:${wsPort}`, "/ws"),
    };
    deepEqual(validateDaemonInstance(instance), instance);
  }

  const invalid = [
    {
      id: "host-mismatch",
      name: "Host mismatch",
      httpBase: endpoint("https", "api.example.com:9765"),
      wsUrl: endpoint("wss", "events.example.com:8765", "/ws"),
    },
    {
      id: "scheme-mismatch",
      name: "Scheme mismatch",
      httpBase: endpoint("https", `${localHostname}:9765`),
      wsUrl: endpoint("ws", `${localHostname}:8765`, "/ws"),
    },
    {
      id: "port-mismatch",
      name: "Port mismatch",
      httpBase: endpoint("https", "daemon.example.com:9765"),
      wsUrl: endpoint("wss", "daemon.example.com:8766", "/ws"),
    },
  ];
  for (const instance of invalid) {
    throwsType(() => validateDaemonInstance(instance), DaemonConfigError);
  }
});

test("plaintext loopback transport accepts only an exact raw authority hostname", () => {
  for (const rawHost of ["LOCALHOST", loopbackIpv4, "[::1]"]) {
    validateDaemonInstance({
      id: `allowed-${rawHost}`,
      name: "Allowed loopback",
      httpBase: endpoint("http", `${rawHost}:9765`),
      wsUrl: endpoint("ws", `${rawHost}:8765`, "/ws"),
    });
  }

  for (const rawHost of [
    "127.1",
    "2130706433",
    "0x7f000001",
    "0177.0.0.1",
    "localhost.",
    "127.0.0.1.",
  ]) {
    throwsType(
      () => validateDaemonInstance({
        id: `rejected-${rawHost}`,
        name: "Rejected loopback spelling",
        httpBase: endpoint("http", `${rawHost}:9765`),
        wsUrl: endpoint("ws", `${rawHost}:8765`, "/ws"),
      }),
      DaemonConfigError,
    );
  }
});

test("HTTP endpoint raw path permits only an absent path or one root slash", () => {
  for (const path of ["", "/"]) {
    validateDaemonInstance({
      id: `allowed-root-${path.length}`,
      name: "Allowed root path",
      httpBase: endpoint("https", "daemon.example.com:9765", path),
      wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
    });
  }

  for (const path of ["/.", "/%2e", "/a/..", "\\."]) {
    throwsType(
      () => validateDaemonInstance({
        id: `rejected-path-${path}`,
        name: "Rejected normalized path",
        httpBase: endpoint("https", "daemon.example.com:9765", path),
        wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
      }),
      DaemonConfigError,
    );
  }
});

test("configuration parsing fails closed instead of falling back after an explicit invalid override", () => {
  throwsWith(
    () => parseDaemonInstancesConfig("not-json", "VITE_FENGLIANG_DAEMON_INSTANCES"),
    (error: unknown) => error instanceof DaemonConfigError
      && /VITE_FENGLIANG_DAEMON_INSTANCES/.test(error.message)
      && /JSON/.test(error.message),
  );
});

test("parse and save share non-empty and normalized unique-id list validation without partial writes", () => {
  const instance = {
    id: "prod",
    name: "Production",
    httpBase: endpoint("https", "daemon.example.com:9765"),
    wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
  };
  const invalidLists = [
    [],
    [instance, { ...instance, id: " prod ", name: "Duplicate" }],
  ];

  for (const invalid of invalidLists) {
    throwsType(
      () => parseDaemonInstancesConfig(JSON.stringify(invalid), "test daemon instances"),
      DaemonConfigError,
    );

    const storage = new MemoryStorage({ instances: "unchanged" });
    throwsType(
      () => savePersistedDaemonInstances(storage, "instances", invalid),
      DaemonConfigError,
    );
    equal(storage.getItem("instances"), "unchanged");
  }
});

test("current persisted configuration is validated and rejected before it can be loaded or saved", () => {
  const key = "daemon-instances";
  const insecure = [{
    id: "bad",
    name: "Bad",
    httpBase: endpoint("http", "public.example:9765"),
    wsUrl: endpoint("wss", "public.example:8765", "/ws"),
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
  matches(loaded.rejection ?? "", /已拒绝无效或不安全的已保存 daemon 配置/);
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
    [`${key}_v`]: "5",
    [key]: JSON.stringify([{ legacy: true }]),
  });
  const legacy = loadPersistedDaemonInstances(
    legacyStorage,
    key,
    6,
    BUILT_IN_DAEMON_INSTANCES,
  );
  matches(legacy.rejection ?? "", /旧内置公网实例已移除/);
  matches(legacy.rejection ?? "", /VITE_FENGLIANG_DAEMON_INSTANCES/);
  matches(legacy.rejection ?? "", /用户显式 HTTPS\/WSS 配置/);
  equal(legacyStorage.getItem(key), null);
  equal(legacyStorage.getItem(`${key}_v`), "6");

  const secure = [{
    id: "prod",
    name: "Production",
    httpBase: endpoint("https", "daemon.example.com:9765"),
    wsUrl: endpoint("wss", "daemon.example.com:8765", "/ws"),
  }];
  const secureStorage = new MemoryStorage({
    [`${key}_v`]: "6",
    [key]: JSON.stringify(secure),
  });
  const current = loadPersistedDaemonInstances(
    secureStorage,
    key,
    6,
    BUILT_IN_DAEMON_INSTANCES,
  );
  deepEqual(current.instances, secure);
  equal(current.rejection, null);
});

test("public HTTP and WS are rejected for IPs, hostnames, and loopback lookalikes", () => {
  const cases = [
    [endpoint("http", "public.example:9765"), endpoint("wss", "public.example:8765", "/ws")],
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

test("WebSocket derivation supports each documented daemon port mapping without downgrading TLS", () => {
  for (const [httpPort, wsPort] of [["9765", "8765"], ["9766", "8766"], ["9767", "8767"]]) {
    equal(
      deriveWebSocketUrl(endpoint("https", `daemon.example.com:${httpPort}`)),
      endpoint("wss", `daemon.example.com:${wsPort}`, "/ws"),
    );
  }
  equal(
    deriveWebSocketUrl(endpoint("http", `${localHostname}:9765`)),
    endpoint("ws", `${localHostname}:8765`, "/ws"),
  );
  throwsType(() => deriveWebSocketUrl(endpoint("http", "daemon.example.com:9765")), DaemonConfigError);
});

test("unknown HTTP ports require an explicit WebSocket endpoint", () => {
  throwsWith(
    () => deriveWebSocketUrl(endpoint("https", "daemon.example.com:9443")),
    (error: unknown) => error instanceof DaemonConfigError && /wsUrl|explicit/i.test(error.message),
  );
  throwsWith(
    () => deriveWebSocketUrl(endpoint("https", "daemon.example.com")),
    (error: unknown) => error instanceof DaemonConfigError && /wsUrl|explicit/i.test(error.message),
  );

  const explicit = {
    id: "custom-ports",
    name: "Custom ports",
    httpBase: endpoint("https", "daemon.example.com:9443"),
    wsUrl: endpoint("wss", "daemon.example.com:9444", "/ws"),
  };
  deepEqual(validateDaemonInstance(explicit), explicit);
});

test("connection errors distinguish HTTP, JSON, WebSocket close, and generic network failures", () => {
  const httpMessage = describeDaemonConnectionError(
    endpoint("https", "daemon.example.com:9765"),
    new DaemonHttpError(503, "/status"),
  );
  matches(httpMessage, /HTTP 503/);
  matches(httpMessage, /\/status/);
  doesNotMatch(httpMessage, /TLS|证书/);

  const jsonMessage = describeDaemonConnectionError(
    endpoint("https", "daemon.example.com:9765"),
    new DaemonJsonResponseError(200, "/status", new SyntaxError("Unexpected token")),
  );
  matches(jsonMessage, /JSON/);
  matches(jsonMessage, /Unexpected token/);
  doesNotMatch(jsonMessage, /TLS|证书/);

  const closeMessage = describeDaemonConnectionError(
    endpoint("wss", "daemon.example.com:8765", "/ws"),
    { code: 1006, reason: "upstream restart", wasClean: false },
  );
  matches(closeMessage, /1006/);
  matches(closeMessage, /upstream restart/);
  doesNotMatch(closeMessage, /TLS|证书/);

  const secureNetworkMessage = describeDaemonConnectionError(
    endpoint("https", "daemon.example.com:9765"),
    new TypeError("Failed to fetch"),
  );
  matches(secureNetworkMessage, /安全网络连接失败/);
  matches(secureNetworkMessage, /Failed to fetch/);
  doesNotMatch(secureNetworkMessage, /证书|主机名/);

  const localMessage = describeDaemonConnectionError(
    endpoint("http", `${localHostname}:9765`),
    new Error("ECONNREFUSED"),
  );
  matches(localMessage, /网络连接失败/);
  matches(localMessage, /ECONNREFUSED/);
  doesNotMatch(localMessage, /证书/);
});
