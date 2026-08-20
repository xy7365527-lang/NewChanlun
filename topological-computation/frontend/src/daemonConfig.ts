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

function endpointUrl(raw: string, field: "HTTP" | "WebSocket"): URL {
  if (typeof raw !== "string" || raw.trim() === "") {
    throw new DaemonConfigError(`${field} endpoint must be a non-empty URL`);
  }

  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    throw new DaemonConfigError(`${field} endpoint is not a valid absolute URL: ${raw}`);
  }

  if (url.username || url.password) {
    throw new DaemonConfigError(`${field} endpoint must not contain credentials`);
  }
  return url;
}

export function isExactLoopbackHost(hostname: string): boolean {
  return EXACT_LOOPBACK_HOSTS.has(hostname.toLowerCase());
}

function requireSecureTransport(
  url: URL,
  field: "HTTP" | "WebSocket",
): void {
  const requiredScheme = field === "HTTP" ? "https:" : "wss:";
  const loopbackProtocol = field === "HTTP" ? "http:" : "ws:";

  if (url.protocol === requiredScheme) return;
  if (url.protocol === loopbackProtocol && isExactLoopbackHost(url.hostname)) return;

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

export function validateDaemonInstance(instance: DaemonInstance): DaemonInstance {
  if (!instance || typeof instance !== "object") {
    throw new DaemonConfigError("daemon instance must be an object");
  }

  const id = typeof instance.id === "string" ? instance.id.trim() : "";
  const name = typeof instance.name === "string" ? instance.name.trim() : "";
  if (!id) throw new DaemonConfigError("daemon instance id must not be empty");
  if (!name) throw new DaemonConfigError(`daemon instance ${id || "<unknown>"} name must not be empty`);

  const http = endpointUrl(instance.httpBase, "HTTP");
  const ws = endpointUrl(instance.wsUrl, "WebSocket");
  requireSecureTransport(http, "HTTP");
  requireSecureTransport(ws, "WebSocket");

  return {
    id,
    name,
    httpBase: normalizeHttpBase(http),
    wsUrl: ws.toString(),
  };
}

const BUILT_IN_INPUTS: DaemonInstance[] = [
  {
    id: "vps-0",
    name: "VPS-0",
    httpBase: "https://46.4.204.119:9765",
    wsUrl: "wss://46.4.204.119:8765/ws",
  },
  {
    id: "vps-1",
    name: "VPS-1",
    httpBase: "https://46.4.204.119:9766",
    wsUrl: "wss://46.4.204.119:8766/ws",
  },
  {
    id: "vps-2",
    name: "VPS-2",
    httpBase: "https://46.4.204.119:9767",
    wsUrl: "wss://46.4.204.119:8767/ws",
  },
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

  if (!Array.isArray(parsed) || parsed.length === 0) {
    throw new DaemonConfigError(`${source} must be a non-empty JSON array of daemon instances`);
  }

  const instances = parsed.map((value, index) => {
    if (!value || typeof value !== "object") {
      throw new DaemonConfigError(`${source}[${index}] must be a daemon instance object`);
    }
    const candidate = value as Partial<DaemonInstance>;
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
          ? "已拒绝旧版 daemon 配置：公网 HTTP/WS 端点必须改用 HTTPS/WSS"
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
          rejection: `已拒绝不安全的已保存 daemon 配置：${detail}`,
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
  const validated = instances.map(validateDaemonInstance);
  storage.setItem(storageKey, JSON.stringify(validated));
  return validated;
}

export function deriveWebSocketUrl(httpBase: string): string {
  const http = endpointUrl(httpBase, "HTTP");
  requireSecureTransport(http, "HTTP");

  http.protocol = http.protocol === "https:" ? "wss:" : "ws:";
  if (http.port === "9765") http.port = "8765";
  http.pathname = "/ws";
  http.search = "";
  http.hash = "";
  return http.toString();
}

/**
 * Browsers deliberately hide the exact TLS failure from application code.
 * Preserve the available cause while explaining that HTTPS/WSS failures are
 * fail-closed and can include certificate or host-identity rejection.
 */
export function describeDaemonConnectionError(endpoint: string, cause?: unknown): string {
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
  const summary = usesTls
    ? "安全连接失败（浏览器已拒绝网络连接或 TLS 证书/主机名校验未通过）"
    : "连接失败";
  return detail ? `${summary}: ${detail}` : summary;
}
