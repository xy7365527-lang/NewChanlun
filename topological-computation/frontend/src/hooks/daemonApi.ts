import type {
  StatusResponse,
  TopologyResponse,
  QueryResponse,
  NarrativeEvent,
  GapEntry,
  OperationStats,
  TraverseResponse,
  FeedResponse,
  PresentResponse,
  PersistenceResponse,
} from "../types";

export class DaemonHttpError extends Error {
  constructor(
    readonly status: number,
    readonly path: string,
  ) {
    super(`HTTP ${status} for ${path}`);
    this.name = "DaemonHttpError";
  }
}

export class DaemonJsonResponseError extends Error {
  constructor(
    readonly status: number,
    readonly path: string,
    cause: unknown,
  ) {
    const detail = cause instanceof Error ? cause.message : String(cause);
    super(`Invalid JSON response (HTTP ${status}) for ${path}: ${detail}`);
    this.name = "DaemonJsonResponseError";
  }
}

async function readJsonResponse<T>(response: Response, path: string): Promise<T> {
  try {
    return await response.json() as T;
  } catch (error) {
    throw new DaemonJsonResponseError(response.status, path, error);
  }
}

function makeGet(baseUrl: string) {
  return async function get<T>(path: string, signal?: AbortSignal): Promise<T> {
    const res = await fetch(`${baseUrl}${path}`, { signal });
    if (!res.ok) {
      throw new DaemonHttpError(res.status, path);
    }
    return readJsonResponse<T>(res, path);
  };
}

function makePost(baseUrl: string) {
  return async function post<T>(path: string, body: Record<string, unknown>): Promise<T> {
    const res = await fetch(`${baseUrl}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!res.ok) {
      throw new DaemonHttpError(res.status, path);
    }
    return readJsonResponse<T>(res, path);
  };
}

export interface DaemonAPI {
  status: (signal?: AbortSignal) => Promise<StatusResponse>;
  topology: (center?: string, radius?: number) => Promise<TopologyResponse>;
  query: (concept: string) => Promise<QueryResponse>;
  narrative: (n?: number) => Promise<NarrativeEvent[]>;
  gaps: () => Promise<GapEntry[]>;
  operations: () => Promise<OperationStats>;
  traverse: (start: string) => Promise<TraverseResponse>;
  feed: (text: string) => Promise<FeedResponse>;
  present: (text: string) => Promise<PresentResponse>;
  persistence: () => Promise<PersistenceResponse>;
}

export function createDaemonAPI(baseUrl: string): DaemonAPI {
  const get = makeGet(baseUrl);
  const post = makePost(baseUrl);

  return {
    status: (signal?: AbortSignal) => get<StatusResponse>("/status", signal),

    topology: (center?: string, radius = 2) => {
      const params = center
        ? `?center=${encodeURIComponent(center)}&radius=${radius}`
        : "?full=true";
      return get<TopologyResponse>(`/topology${params}`);
    },

    query: (concept: string) =>
      get<QueryResponse>(`/query?concept=${encodeURIComponent(concept)}`),

    narrative: (n = 20) =>
      get<NarrativeEvent[]>(`/narrative?n=${n}`),

    gaps: () => get<GapEntry[]>("/gaps"),

    operations: () => get<OperationStats>("/operations"),

    traverse: (start: string) =>
      post<TraverseResponse>("/traverse", { start }),

    feed: (text: string) =>
      post<FeedResponse>("/feed", { text }),

    present: (text: string) =>
      post<PresentResponse>("/present", { text }),

    persistence: () =>
      get<PersistenceResponse>("/persistence"),
  };
}
