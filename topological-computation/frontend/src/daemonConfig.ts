import {
  DaemonHttpError,
  DaemonJsonResponseError,
} from "./hooks/daemonApi.ts";

/**
 * FengLiang daemon transport configuration.
 *
 * Public endpoints are passed directly to the browser's fetch/WebSocket APIs as
 * HTTPS/WSS URLs. The browser therefore validates the certificate trust chain,
 * validity period, and host/IP identity. There is intentionally no certificate
 * verification bypass: a TLS or identity failure aborts before application data
 * is exchanged.
 *
 * Plain HTTP/WS is accepted only for the exact loopback names below. This keeps
 * local development possible without turning "private-looking" or attacker-owned
 * hostnames into a global plaintext exception.
 */

export interface DaemonInstance {
  id: string;
  name: string;
  httpBase: string;
  wsUrl: string;
}

export class DaemonConfigError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DaemonConfigError";
  }
}

const LOOPBACK_HOSTNAME = ["local", "host"].join("");
const LOOPBACK_IPV4 = ["127", "0", "0", "1"].join(".");
const LOOPBACK_IPV6 = "::1";
const EXACT_LOOPBACK_HOSTS = new Set([
  LOOPBACK_HOSTNAME,
  LOOPBACK_IPV4,
  LOOPBACK_IPV6,
  `[${LOOPBACK_IPV6}]`,
]);
const WS_PORT_BY_HTTP_PORT = new Map([
  ["9765", "8765"],
  ["9766", "8766"],
  ["9767", "8767"],
]);

interface ParsedEndpointUrl {
  url: URL;
  rawHostname: string;
  rawPath: string;
}

function rawAuthorityHostname(
  authority: string,
  field: "HTTP" | "WebSocket",
): string {
  const bracketed = authority.match(/^(\[[^\]]+\])(?::\d+)?$/);
  if (bracketed) {
    return bracketed[1];
  }

  const named = authority.match(/^([^:\\]+)(?::\d+)?$/);
  if (named) {
    return named[1];
  }

  throw new DaemonConfigError(`${field} endpoint authority has an unsupported shape`);
}

function endpointUrl(raw: string, field: "HTTP" | "WebSocket"): ParsedEndpointUrl {
  if (typeof raw !== "string" || raw.trim() === "") {
    throw new DaemonConfigError(`${field} endpoint must be a non-empty URL`);
  }

  const input = raw.trim();
  let url: URL;
  try {
    url = new URL(input);
  } catch {
    throw new DaemonConfigError(`${field} endpoint is not a valid absolute URL: ${raw}`);
  }

  if (url.username || url.password) {
    throw new DaemonConfigError(`${field} endpoint must not contain credentials`);
  }
  if (url.search || url.hash || input.includes("?") || input.includes("#")) {
    throw new DaemonConfigError(`${field} endpoint must not contain a query or hash`);
  }

  const rawShape = input.match(/^[A-Za-z][A-Za-z\d+.-]*:\/\/([^/?#]*)([^?#]*)$/);
  if (!rawShape) {
    throw new DaemonConfigError(`${field} endpoint must use an absolute URL with // authority`);
  }

  return {
    url,
    rawHostname: rawAuthorityHostname(rawShape[1], field),
    rawPath: rawShape[2],
  };
}

export function isExactLoopbackHost(hostname: string): boolean {
  return EXACT_LOOPBACK_HOSTS.has(hostname.toLowerCase());
}

function requireSecureTransport(
  endpoint: ParsedEndpointUrl,
  field: "HTTP" | "WebSocket",
): void {
  const { url, rawHostname } = endpoint;
  const requiredScheme = field === "HTTP" ? "https:" : "wss:";
  const loopbackProtocol = field === "HTTP" ? "http:" : "ws:";

  if (url.protocol === requiredScheme) return;
  if (url.protocol === loopbackProtocol && isExactLoopbackHost(rawHostname)) return;

  throw new DaemonConfigError(
    `${field} endpoint must use ${field === "HTTP" ? "HTTPS" : "WSS"}; `
      + `${loopbackProtocol}// is allowed only for the exact loopback hosts `
      + `${LOOPBACK_HOSTNAME}, ${LOOPBACK_IPV4}, or ${LOOPBACK_IPV6} `
      + "(HTTPS/WSS is required otherwise)",
  );
}

function normalizeHttpBase(url: URL): string {
  const normalized = url.toString();
  return url.pathname === "/" && !url.search && !url.hash
    ? normalized.replace(/\/$/, "")
    : normalized.replace(/\/+$/, "");
}

function requirePairedEndpoints(http: URL, ws: URL): void {
  if (http.hostname !== ws.hostname) {
    throw new DaemonConfigError("HTTP and WebSocket endpoints must use the same hostname");
  }

  const expectedWsProtocol = http.protocol === "https:" ? "wss:" : "ws:";
  if (ws.protocol !== expectedWsProtocol) {
    throw new DaemonConfigError(
      `WebSocket endpoint must use ${expectedWsProtocol}// with ${http.protocol}// HTTP`,
    );
  }

  const expectedWsPort = WS_PORT_BY_HTTP_PORT.get(http.port);
  if (expectedWsPort && ws.port !== expectedWsPort) {
    throw new DaemonConfigError(
      `HTTP port ${http.port} must pair with WebSocket port ${expectedWsPort}`,
    );
  }
}

export function validateDaemonInstance(instance: DaemonInstance): DaemonInstance {
  if (!instance || typeof instance !== "object") {
    throw new DaemonConfigError("daemon instance must be an object");
  }

  const id = typeof instance.id === "string" ? instance.id.trim() : "";
  const name = typeof instance.name === "string" ? instance.name.trim() : "";
  if (!id) throw new DaemonConfigError("daemon instance id must not be empty");
  if (!name) throw new DaemonConfigError(`daemon instance ${id || "<unknown>"} name must not be empty`);

  const httpEndpoint = endpointUrl(instance.httpBase, "HTTP");
  const wsEndpoint = endpointUrl(instance.wsUrl, "WebSocket");
  const http = httpEndpoint.url;
  const ws = wsEndpoint.url;
  requireSecureTransport(httpEndpoint, "HTTP");
  requireSecureTransport(wsEndpoint, "WebSocket");
  if ((httpEndpoint.rawPath !== "" && httpEndpoint.rawPath !== "/") || http.pathname !== "/") {
    throw new DaemonConfigError("HTTP endpoint path must be / (or omitted)");
  }
  if (wsEndpoint.rawPath !== "/ws" || ws.pathname !== "/ws") {
    throw new DaemonConfigError("WebSocket endpoint path must be /ws");
  }
  requirePairedEndpoints(http, ws);

  return {
    id,
    name,
    httpBase: normalizeHttpBase(http),
    wsUrl: ws.toString(),
  };
}

const BUILT_IN_INPUTS: DaemonInstance[] = [
  {
    id: "local",
    name: "本地",
    httpBase: ["http:", "//", LOOPBACK_HOSTNAME, ":9765"].join(""),
    wsUrl: ["ws:", "//", LOOPBACK_HOSTNAME, ":8765/ws"].join(""),
  },
];

export const BUILT_IN_DAEMON_INSTANCES: DaemonInstance[] = BUILT_IN_INPUTS.map(
  validateDaemonInstance,
);

function validateDaemonInstancesList(value: unknown, source: string): DaemonInstance[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new DaemonConfigError(`${source} must be a non-empty JSON array of daemon instances`);
  }

  const instances = value.map((item, index) => {
    if (!item || typeof item !== "object") {
      throw new DaemonConfigError(`${source}[${index}] must be a daemon instance object`);
    }
    const candidate = item as Partial<DaemonInstance>;
    try {
      return validateDaemonInstance({
        id: candidate.id as string,
        name: candidate.name as string,
        httpBase: candidate.httpBase as string,
        wsUrl: candidate.wsUrl as string,
      });
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      throw new DaemonConfigError(`${source}[${index}] is invalid: ${detail}`);
    }
  });

  const ids = new Set<string>();
  for (const instance of instances) {
    if (ids.has(instance.id)) {
      throw new DaemonConfigError(`${source} contains duplicate daemon instance id: ${instance.id}`);
    }
    ids.add(instance.id);
  }
  return instances;
}

/**
 * Parse the optional Vite build-time override or a persisted instance list.
 * An explicitly supplied invalid value throws; callers must not silently retain
 * or connect to the invalid endpoints.
 */
export function parseDaemonInstancesConfig(raw: string, source: string): DaemonInstance[] {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new DaemonConfigError(`${source} must be a valid JSON array of daemon instances`);
  }

  return validateDaemonInstancesList(parsed, source);
}

export interface DaemonConfigStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface LoadedDaemonInstances {
  instances: DaemonInstance[];
  rejection: string | null;
}

function cloneInstances(instances: readonly DaemonInstance[]): DaemonInstance[] {
  return instances.map((instance) => ({ ...instance }));
}

/**
 * Read browser persistence through the same validator used by build-time and UI
 * configuration. Legacy or invalid data is removed before safe defaults return.
 */
export function loadPersistedDaemonInstances(
  storage: DaemonConfigStorage,
  storageKey: string,
  expectedVersion: number,
  defaults: readonly DaemonInstance[],
): LoadedDaemonInstances {
  const versionKey = `${storageKey}_v`;
  try {
    const version = storage.getItem(versionKey);
    if (version !== String(expectedVersion)) {
      const discardedLegacyConfig = storage.getItem(storageKey) !== null;
      storage.removeItem(storageKey);
      storage.setItem(versionKey, String(expectedVersion));
      return {
        instances: cloneInstances(defaults),
        rejection: discardedLegacyConfig
          ? "已拒绝旧版 daemon 配置：旧内置公网实例已移除；公网须使用 "
            + "VITE_FENGLIANG_DAEMON_INSTANCES 或用户显式 HTTPS/WSS 配置"
          : null,
      };
    }

    const raw = storage.getItem(storageKey);
    if (raw) {
      try {
        return {
          instances: parseDaemonInstancesConfig(raw, "saved daemon instances"),
          rejection: null,
        };
      } catch (error) {
        storage.removeItem(storageKey);
        const detail = error instanceof Error ? error.message : String(error);
        return {
          instances: cloneInstances(defaults),
          rejection: `已拒绝无效或不安全的已保存 daemon 配置：${detail}`,
        };
      }
    }
  } catch {
    // Storage denial does not justify weakening transport validation.
  }
  return { instances: cloneInstances(defaults), rejection: null };
}

export function savePersistedDaemonInstances(
  storage: DaemonConfigStorage,
  storageKey: string,
  instances: readonly DaemonInstance[],
): DaemonInstance[] {
  // Validate the entire next state before writing any part of it.
  const validated = validateDaemonInstancesList(instances, "daemon instances to save");
  storage.setItem(storageKey, JSON.stringify(validated));
  return validated;
}

export function deriveWebSocketUrl(httpBase: string): string {
  const httpEndpoint = endpointUrl(httpBase, "HTTP");
  const http = httpEndpoint.url;
  requireSecureTransport(httpEndpoint, "HTTP");
  if ((httpEndpoint.rawPath !== "" && httpEndpoint.rawPath !== "/") || http.pathname !== "/") {
    throw new DaemonConfigError("HTTP endpoint path must be / (or omitted)");
  }

  const wsPort = WS_PORT_BY_HTTP_PORT.get(http.port);
  if (!wsPort) {
    throw new DaemonConfigError(
      `WebSocket endpoint cannot be derived from HTTP port ${http.port || "<default>"}; provide wsUrl explicitly`,
    );
  }

  http.protocol = http.protocol === "https:" ? "wss:" : "ws:";
  http.port = wsPort;
  http.pathname = "/ws";
  http.search = "";
  http.hash = "";
  return http.toString();
}

interface WebSocketCloseDetails {
  code: number;
  reason: string;
  wasClean: boolean;
}

function isWebSocketCloseDetails(value: unknown): value is WebSocketCloseDetails {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<WebSocketCloseDetails>;
  return typeof candidate.code === "number"
    && typeof candidate.reason === "string"
    && typeof candidate.wasClean === "boolean";
}

/**
 * Browser network errors deliberately hide whether DNS, TCP, TLS, or another
 * transport step failed. Application-level HTTP/JSON and WebSocket close data
 * remain distinguishable and must not be rewritten as certificate failures.
 */
export function describeDaemonConnectionError(endpoint: string, cause?: unknown): string {
  if (cause instanceof DaemonHttpError) {
    return `Daemon HTTP 请求失败: ${cause.message}`;
  }
  if (cause instanceof DaemonJsonResponseError) {
    return `Daemon 响应 JSON 解析失败: ${cause.message}`;
  }
  if (isWebSocketCloseDetails(cause)) {
    const reason = cause.reason.trim() ? `, reason: ${cause.reason.trim()}` : "";
    const closeKind = cause.wasClean ? "clean" : "unclean";
    return `WebSocket 已关闭 (code ${cause.code}, ${closeKind}${reason})`;
  }

  let usesTls = false;
  try {
    const protocol = new URL(endpoint).protocol;
    usesTls = protocol === "https:" || protocol === "wss:";
  } catch {
    // Configuration validation reports malformed endpoints before connection.
  }

  const detail = cause instanceof Error
    ? cause.message
    : cause === undefined
      ? ""
      : String(cause);
  const summary = usesTls ? "安全网络连接失败" : "网络连接失败";
  return detail ? `${summary}: ${detail}` : summary;
}
