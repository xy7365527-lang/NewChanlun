import {
  BUILT_IN_DAEMON_INSTANCES,
  parseDaemonInstancesConfig,
} from "./daemonConfig";
import type { DaemonInstance } from "./daemonConfig";

export type { DaemonInstance } from "./daemonConfig";

// ── Design Tokens ──────────────────────────────────────────────
export const T = {
  bg: "#0a0a0f",
  bgPanel: "#0d0d14",
  bgCard: "#12121c",
  bgHover: "#1a1a28",
  border: "#1e1e2e",
  borderActive: "#2a2a44",
  text: "#e8e8f0",
  textDim: "#6a6a8a",
  textMuted: "#3a3a5a",
  accent: "#7c6cff",
  accentGlow: "rgba(124, 108, 255, 0.15)",
  // f-value thermal palette
  fLow: "#22d68a",     // fold zone — safe, verdant
  fMid: "#f0c040",     // gray zone — caution, amber
  fHigh: "#ff4466",    // negate zone — danger, crimson
  fZero: "#00ffcc",    // f=0 — perfect equivalence, cyan
  // settled ring
  settled: "#4488ff",
  settledGlow: "rgba(68, 136, 255, 0.3)",
  // traversal
  traversalPulse: "#ffffff",
  // status
  alive: "#22d68a",
  feeding: "#f0c040",
  crystallized: "#4488ff",
} as const;

export const FONT = {
  display: "'DM Mono', 'JetBrains Mono', monospace",
  body: "'DM Sans', 'Satoshi', sans-serif",
  mono: "'DM Mono', 'JetBrains Mono', monospace",
} as const;

// ── Config ─────────────────────────────────────────────────────
// Vite embeds this override at build time. Invalid explicit configuration throws
// during application startup rather than falling back to an unsafe endpoint.
const instanceOverride = import.meta.env.VITE_FENGLIANG_DAEMON_INSTANCES;
export const DEFAULT_INSTANCES: DaemonInstance[] = instanceOverride?.trim()
  ? parseDaemonInstancesConfig(instanceOverride, "VITE_FENGLIANG_DAEMON_INSTANCES")
  : BUILT_IN_DAEMON_INSTANCES.map((instance) => ({ ...instance }));

export const DAEMON_HTTP = DEFAULT_INSTANCES[0].httpBase;
export const DAEMON_WS = DEFAULT_INSTANCES[0].wsUrl;
export const WS_THROTTLE_MS = 100;
export const STATUS_POLL_MS = 1000;

// ── Multi-instance ─────────────────────────────────────────────
export const INSTANCE_STORAGE_KEY = "fl-daemon-instances";

export const INSTANCE_COLORS = [
  "#00ccff",  // cyan (self/first)
  "#ff4466",  // red
  "#22d68a",  // green
  "#f0c040",  // amber
  "#cc66ff",  // purple
  "#ff8c42",  // orange
  "#42c6ff",  // light cyan
] as const;

// ── f-value color mapping ───────────────────────────────────────
export function fToColor(f: number): string {
  if (f < 0) return T.textMuted;   // unknown / no terrain data
  if (f <= 0.5) return T.fZero;
  if (f < 5) return T.fLow;
  if (f < 12) return T.fMid;
  return T.fHigh;
}

export function importanceToOpacity(imp: number): number {
  return 0.4 + imp * 0.6;
}
