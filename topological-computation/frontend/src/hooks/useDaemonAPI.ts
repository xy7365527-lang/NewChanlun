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

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${DAEMON_HTTP}${path}`);
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} for ${path}`);
  }
  return res.json() as Promise<T>;
}

async function post<T>(path: string, body: Record<string, unknown>): Promise<T> {
  const res = await fetch(`${DAEMON_HTTP}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} for ${path}`);
  }
  return res.json() as Promise<T>;
}

export const daemonAPI = {
  status: (): Promise<StatusResponse> => get<StatusResponse>("/status"),

  topology: (center?: string, radius = 2, full = false): Promise<TopologyResponse> => {
    if (center) {
      return get<TopologyResponse>(`/topology?center=${encodeURIComponent(center)}&radius=${radius}`);
    }
    if (full) {
      return get<TopologyResponse>(`/topology?full=true`);
    }
    return get<TopologyResponse>(`/topology`);
  },

  query: (concept: string): Promise<QueryResponse> =>
    get<QueryResponse>(`/query?concept=${encodeURIComponent(concept)}`),

  narrative: (n = 20): Promise<NarrativeEvent[]> =>
    get<NarrativeEvent[]>(`/narrative?n=${n}`),

  gaps: (): Promise<GapEntry[]> => get<GapEntry[]>("/gaps"),

  operations: (): Promise<OperationStats> => get<OperationStats>("/operations"),

  traverse: (start: string): Promise<TraverseResponse> =>
    post<TraverseResponse>("/traverse", { start }),

  feed: (text: string): Promise<FeedResponse> =>
    post<FeedResponse>("/feed", { text }),

  /**
   * POST /present — user presence.
   *
   * Semantics: the user is present, not asking a question.
   * The system decides what (if anything) to share:
   * - Unreported high-importance events (expression pressure)
   * - Co-gaze traversal of user-named concepts
   * - Silence (nothing to say right now)
   *
   * text can be empty string (when user clicks the pressure indicator).
   */
  present: (text: string): Promise<PresentResponse> =>
    post<PresentResponse>("/present", { text }),

  /**
   * 获取持久同调数据（birth-death 对列表）。
   * 若 daemon 未实现此端点，调用方应降级处理（返回 null / 合成数据）。
   */
  persistence: (): Promise<PersistenceResponse> =>
    get<PersistenceResponse>("/persistence"),
};
