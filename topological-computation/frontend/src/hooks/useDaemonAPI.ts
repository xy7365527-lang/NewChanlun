import { DAEMON_HTTP } from "../tokens";
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

function makeGet(baseUrl: string) {
  return async function get<T>(path: string): Promise<T> {
    const res = await fetch(`${baseUrl}${path}`);
    if (!res.ok) {
      throw new Error(`HTTP ${res.status} for ${path}`);
    }
    return res.json() as Promise<T>;
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
      throw new Error(`HTTP ${res.status} for ${path}`);
    }
    return res.json() as Promise<T>;
  };
}

export interface DaemonAPI {
  status: () => Promise<StatusResponse>;
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
    status: () => get<StatusResponse>("/status"),

    topology: (center?: string, radius = 2) => {
      const params = center
        ? `?center=${encodeURIComponent(center)}&radius=${radius}`
        : "";
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

/** Backward-compatible default instance using DAEMON_HTTP from tokens */
export const daemonAPI = createDaemonAPI(DAEMON_HTTP);
