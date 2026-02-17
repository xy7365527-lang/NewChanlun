/**
 * 后端 API 封装
 * 开发环境下 Vite proxy 把 /api 转发到 Python 后端 :8765
 */

import type {
  OhlcvResponse,
  SearchResult,
  OverlayResponse,
  LiveStatus,
} from "../types/overlay";

const BASE = ""; // 同源，Vite proxy 处理

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json() as Promise<T>;
}

// ── OHLCV（分页） ──

export interface GetOhlcvParams {
  symbol: string;
  interval?: string; // 默认 "1min"
  tf?: string; // 默认 "1m"
  to?: number; // epoch seconds，返回 <= to
  countBack?: number; // 返回最近 N 条
  after?: number; // epoch seconds，返回 > after
}

export async function getOhlcv(p: GetOhlcvParams): Promise<OhlcvResponse> {
  const qs = new URLSearchParams();
  qs.set("symbol", p.symbol);
  qs.set("interval", p.interval ?? "1min");
  qs.set("tf", p.tf ?? "1m");
  if (p.to !== undefined) qs.set("to", String(p.to));
  if (p.countBack !== undefined) qs.set("countBack", String(p.countBack));
  if (p.after !== undefined) qs.set("after", String(p.after));
  return fetchJson<OhlcvResponse>(`${BASE}/api/ohlcv?${qs}`);
}

// ── 品种搜索 ──

export async function searchSymbols(q: string): Promise<SearchResult[]> {
  return fetchJson<SearchResult[]>(`${BASE}/api/search?q=${encodeURIComponent(q)}`);
}

// ── 缠论 overlay ──

export interface GetOverlayParams {
  symbol: string;
  interval?: string;
  tf?: string;
  detail?: "min" | "full";
  segment_algo?: "v0" | "v1";
  stroke_mode?: "wide" | "strict";
}

export async function getOverlay(p: GetOverlayParams): Promise<OverlayResponse> {
  const qs = new URLSearchParams();
  qs.set("symbol", p.symbol);
  qs.set("interval", p.interval ?? "1min");
  qs.set("tf", p.tf ?? "1m");
  qs.set("detail", p.detail ?? "full");
  qs.set("segment_algo", p.segment_algo ?? "v1");
  qs.set("stroke_mode", p.stroke_mode ?? "wide");
  qs.set("min_strict_sep", "5");
  qs.set("center_sustain_m", "2");
  return fetchJson<OverlayResponse>(`${BASE}/api/newchan/overlay?${qs}`);
}

// ── Live 状态 ──

export async function getLiveStatus(): Promise<LiveStatus> {
  return fetchJson<LiveStatus>(`${BASE}/api/live/status`);
}

// ── 品种列表（已缓存） ──

export async function getSymbols(): Promise<{ symbol: string; interval: string }[]> {
  return fetchJson<{ symbol: string; interval: string }[]>(`${BASE}/api/symbols`);
}
