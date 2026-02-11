/** NewChan overlay API 响应类型（对应 schema_version: "newchan_overlay_v2"） */

export interface OhlcvBar {
  time: string; // "YYYY-MM-DD HH:MM:SS"
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

export interface OhlcvResponse {
  data: OhlcvBar[];
  count: number;
  error?: string;
}

export interface SearchResult {
  symbol: string;
  secType: string;
  exchange: string;
  currency: string;
  description: string;
  source: string;
}

export interface OverlayStroke {
  id: number;
  t0: number;
  t1: number;
  dir: "up" | "down";
  confirmed: boolean;
  high: number;
  low: number;
  p0: number;
  p1: number;
  macd_area_total: number;
  macd_area_pos: number;
  macd_area_neg: number;
  macd_n_bars: number;
}

export interface OverlaySegment {
  id: number;
  t0: number;
  t1: number;
  s0: number;
  s1: number;
  dir: "up" | "down";
  confirmed: boolean;
  high: number;
  low: number;
  p0: number;
  p1: number;
  ep0: {
    merged_i: number;
    time: number;
    price: number;
    type: "top" | "bottom";
  };
  ep1: {
    merged_i: number;
    time: number;
    price: number;
    type: "top" | "bottom";
  };
  stroke_points: Array<{ time: number; value: number }>;
  macd_area_total: number;
}

export interface OverlayCenter {
  id: number;
  t0: number;
  t1: number;
  ZD: number;
  ZG: number;
  kind: "candidate" | "settled";
  confirmed: boolean;
  sustain: number;
  macd_area_total: number;
}

export interface OverlayTrend {
  id: number;
  t0: number;
  t1: number;
  kind: "trend" | "consolidation";
  dir: "up" | "down";
  confirmed: boolean;
  high: number;
  low: number;
  p0: number | null;
  p1: number | null;
  macd_area_total: number;
}

export interface MacdPoint {
  time: number;
  macd: number;
  signal: number;
  hist: number;
}

export interface OverlayAnchors {
  settle_core_low: number;
  settle_core_high: number;
  run_exit_idx: number | null;
  run_exit_side: string | null;
  run_exit_extreme: number | null;
  event_seen_pullback: boolean;
  event_pullback_settled: boolean;
}

export interface OverlayLStar {
  level: number;
  center_id: number;
  regime: string;
  is_alive: boolean;
  death_reason: string | null;
  anchors: OverlayAnchors | null;
}

// ── 多级别结构 ──

export interface OverlayLevelCenter {
  id: number;
  t0: number;
  t1: number;
  ZD: number;
  ZG: number;
  kind: "candidate" | "settled";
  confirmed: boolean;
  sustain: number;
}

export interface OverlayLevelTrend {
  id: number;
  t0: number;
  t1: number;
  kind: "trend" | "consolidation";
  dir: "up" | "down";
  confirmed: boolean;
  high: number;
  low: number;
}

export interface OverlayLevel {
  level: number;
  n_moves: number;
  centers: OverlayLevelCenter[];
  trends: OverlayLevelTrend[];
}

export interface OverlayResponse {
  schema_version: string;
  symbol: string;
  tf: string;
  detail: string;
  lstar: OverlayLStar | null;
  strokes: OverlayStroke[];
  segments: OverlaySegment[];
  centers: OverlayCenter[];
  trends: OverlayTrend[];
  levels: OverlayLevel[];
  macd: {
    fast: number;
    slow: number;
    signal: number;
    series: MacdPoint[];
  };
}

export interface LiveStatus {
  running: boolean;
  symbols: string[];
  bar_count: number;
  last_error: string | null;
}
