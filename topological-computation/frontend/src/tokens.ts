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
export const DAEMON_HTTP = "http://localhost:9090";
export const DAEMON_WS = "ws://localhost:8765/ws";
export const WS_THROTTLE_MS = 100;
export const STATUS_POLL_MS = 1000;

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
