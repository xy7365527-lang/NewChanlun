// ── Domain types matching daemon_api.py response shapes ────────

export interface StatusResponse {
  beta_1: number;
  vertices: number;
  edges: number;
  settled: number;
  steps: number;
  crystallization_count: number;
  encounter_density: number;
  status: "traversing" | "feeding" | "crystallized";
  expression_pressure?: number;
}

export interface TopologyNode {
  id: string;
  label: string;
  f_avg: number;
  degree: number;
  settled: boolean;
  type: "text" | "code" | "system";
  // runtime fields added by D3
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
  fx?: number | null;
  fy?: number | null;
  isTraversal?: boolean;
}

export interface TopologyLink {
  source: string | TopologyNode;
  target: string | TopologyNode;
  type: string;
  f?: number;
}

export interface TopologyMeta {
  total_vertices: number;
  total_edges: number;
  shown_vertices: number;
  shown_edges: number;
}

export interface TopologyResponse {
  nodes: TopologyNode[];
  links: TopologyLink[];
  meta: TopologyMeta;
}

export interface NeighborInfo {
  id: string;
  label: string;
  f: number;
  edge_type: string;
}

export interface QueryResponse {
  found: boolean;
  vertex: {
    id: string;
    label: string;
    f_avg: number;
    degree: number;
    settled: boolean;
    rings: number;
    settled_rings: number;
  } | null;
  neighbors: NeighborInfo[];
  f_terrain: {
    fold_zone: string[];
    gray_zone: string[];
    negate_zone: string[];
  };
  narrative: string;
}

export interface NarrativeEvent {
  time: string;
  level: 1 | 2 | 3;
  text: string;
  importance: number;
  type?: string;
}

export interface GapEntry {
  concept: string;
  degree: number;
  avg_neighbor_degree: number;
  status: string;
}

export interface OperationStats {
  fold: { count: number; blocked: number };
  negate: { count: number; blocked: number };
  sublate: { count: number; blocked: number };
  walk: { count: number; blocked: number };
}

export interface TraverseResponse {
  steps: number;
  events: Array<{ step: number; type: string; text: string; importance: number }>;
  narrative: string;
  crystallized: boolean;
  error?: string;
}

export interface FeedResponse {
  accepted: boolean;
  new_vertices: number;
  new_edges: number;
  match_rate: number;
  verdict: string;
  delta_beta_1?: number;
  error?: string;
}

// ── /present response ────────────────────────────────────────────

export interface PresentPart {
  source: "unreported" | "co-gaze" | "language_organ";
  text: string;
}

export interface PresentResponse {
  type: "sharing" | "co-gaze" | "silence" | "dialogue";
  parts: PresentPart[];
  injected: boolean;
  concepts_found: string[];
  expression_pressure: number;
  llm_used?: boolean;
}

// ── WebSocket message types ─────────────────────────────────────

export interface WsStepMessage {
  type: "step";
  step: number;
  position: string;
  position_label: string;
  beta_1: number;
  delta_beta_1: number;
  operation: string;
  f_value: number;
  crystallized: boolean;
}

export interface WsGapMessage {
  type: "gap";
  concept: string;
  degree: number;
  avg_degree: number;
}

export interface WsFeedMessage {
  type: "feed";
  accepted: boolean;
  source: string;
  verdict: string;
  match_rate: number;
  new_vertices: number;
}

/** Daemon proactively shares expression pressure updates. */
export interface WsExpressionMessage {
  type: "expression_pressure";
  count: number;
  text?: string;        // narrative of the triggering event
  importance?: number;
}

export type WsMessage =
  | WsStepMessage
  | WsGapMessage
  | WsFeedMessage
  | WsExpressionMessage;

// ── Chat message ────────────────────────────────────────────────

export interface ChatMessage {
  role: "user" | "daemon";
  text: string;
  timestamp: number;
}

// ── Persistence / TDA types ─────────────────────────────────────

export interface PersistencePair {
  birth: number;
  death: number | null;   // null = essential（无死亡）
  dim: 0 | 1;             // 0 = β₀ 连通分量，1 = β₁ 环
  settled?: boolean;      // 是否为 settled 环
  label?: string;         // 可选标注
}

export interface PersistenceResponse {
  pairs: PersistencePair[];
  max_filtration: number;
}
