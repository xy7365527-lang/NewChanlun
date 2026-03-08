import { create } from "zustand";
import type {
  StatusResponse,
  TopologyResponse,
  TopologyNode,
  NarrativeEvent,
  GapEntry,
  OperationStats,
  ChatMessage,
  WsMessage,
} from "../types";

interface Beta1Point {
  step: number;
  beta1: number;
}

// Color palette for peer instances (auto-assigned)
const PEER_COLORS = [
  "#ff4466",  // red
  "#22d68a",  // green
  "#f0c040",  // amber
  "#cc66ff",  // purple
  "#ff8c42",  // orange
  "#42c6ff",  // cyan
];

function _assignPeerColor(
  instanceId: string,
  existing: Record<string, { color: string }>,
): string {
  const usedColors = new Set(Object.values(existing).map((p) => p.color));
  for (const c of PEER_COLORS) {
    if (!usedColors.has(c)) return c;
  }
  // Fallback: hash-based color
  let hash = 0;
  for (let i = 0; i < instanceId.length; i++) {
    hash = (hash * 31 + instanceId.charCodeAt(i)) | 0;
  }
  return PEER_COLORS[Math.abs(hash) % PEER_COLORS.length];
}

interface DaemonStore {
  // Connection
  wsConnected: boolean;
  daemonReachable: boolean;

  // Metrics (from /status poll)
  status: StatusResponse | null;

  // beta_1 history for the curve (capped at 500 points)
  beta1History: Beta1Point[];

  // Topology
  topology: TopologyResponse | null;

  // Focus concept (drives /query calls)
  focusConcept: string | null;
  queryResult: import("../types").QueryResponse | null;

  // Narrative stream
  narrative: NarrativeEvent[];

  // Gap queue
  gaps: GapEntry[];

  // Operations
  operations: OperationStats | null;

  // Chat messages
  messages: ChatMessage[];

  // Current traversal position label (from WS steps)
  currentPositionLabel: string;
  currentPositionId: string;

  // Expression pressure: number of unreported high-I events
  expressionPressure: number;

  // Peer instance positions (from SharedLayer cross-instance sync)
  peersPositions: Record<string, { posLabel: string; color: string; step: number; online: boolean }>;

  // Actions
  setWsConnected: (v: boolean) => void;
  setDaemonReachable: (v: boolean) => void;
  setStatus: (s: StatusResponse) => void;
  setTopology: (t: TopologyResponse) => void;
  setFocusConcept: (c: string | null) => void;
  setQueryResult: (r: import("../types").QueryResponse | null) => void;
  setNarrative: (events: NarrativeEvent[]) => void;
  setGaps: (gaps: GapEntry[]) => void;
  setOperations: (ops: OperationStats) => void;
  addMessage: (msg: ChatMessage) => void;
  handleWsBatch: (msgs: WsMessage[]) => void;
  updatePeerPosition: (instanceId: string, posLabel: string, step?: number) => void;
}

export const useStore = create<DaemonStore>((set) => ({
  wsConnected: false,
  daemonReachable: false,
  status: null,
  beta1History: [],
  topology: null,
  focusConcept: null,
  queryResult: null,
  narrative: [],
  gaps: [],
  operations: null,
  messages: [],
  currentPositionLabel: "",
  currentPositionId: "",
  expressionPressure: 0,
  peersPositions: {},

  setWsConnected: (v) => set({ wsConnected: v }),
  setDaemonReachable: (v) => set({ daemonReachable: v }),

  setStatus: (s) =>
    set((state) => {
      // Append beta_1 point if steps advanced
      const last = state.beta1History[state.beta1History.length - 1];
      const newPt: Beta1Point = { step: s.steps, beta1: s.beta_1 };
      const shouldAppend = !last || last.step !== s.steps;
      const history = shouldAppend
        ? [...state.beta1History, newPt].slice(-500)
        : state.beta1History;
      // Sync expression_pressure from status if provided
      const pressure = s.expression_pressure !== undefined
        ? s.expression_pressure
        : state.expressionPressure;
      // Sync position from status poll (fallback when WS is unavailable)
      const posLabel = s.position_label || state.currentPositionLabel;
      const posId = s.position || state.currentPositionId;
      return { status: s, beta1History: history, daemonReachable: true, expressionPressure: pressure, currentPositionLabel: posLabel, currentPositionId: posId };
    }),

  setTopology: (t) => set({ topology: t }),
  setFocusConcept: (c) => set({ focusConcept: c }),
  setQueryResult: (r) => set({ queryResult: r }),
  setNarrative: (events) => set({ narrative: events }),
  setGaps: (gaps) => set({ gaps }),
  setOperations: (ops) => set({ operations: ops }),

  addMessage: (msg) =>
    set((state) => ({ messages: [...state.messages, msg] })),

  handleWsBatch: (msgs) =>
    set((state) => {
      let history = [...state.beta1History];
      let currentPositionLabel = state.currentPositionLabel;
      let currentPositionId = state.currentPositionId;
      let expressionPressure = state.expressionPressure;
      const newNarrative: NarrativeEvent[] = [];
      const newGaps: GapEntry[] = [...state.gaps];
      const newMessages: ChatMessage[] = [];
      let peersUpdated = false;
      const updatedPeers = { ...state.peersPositions };

      for (const msg of msgs) {
        if (msg.type === "step") {
          // Update beta_1 history
          const last = history[history.length - 1];
          if (!last || last.step !== msg.step) {
            history.push({ step: msg.step, beta1: msg.beta_1 });
            if (history.length > 500) history = history.slice(-500);
          }
          currentPositionLabel = msg.position_label;
          currentPositionId = msg.position;

          // Significant events -> narrative
          if (msg.delta_beta_1 !== 0 || msg.operation !== "walk") {
            const importance = Math.min(
              1.0,
              Math.abs(msg.delta_beta_1) * 0.3 + (msg.operation !== "walk" ? 0.3 : 0)
            );
            newNarrative.push({
              time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
              level: importance > 0.8 ? 3 : importance > 0.4 ? 2 : 1,
              text: `[步 ${msg.step}] ${msg.operation} @ ${msg.position_label} beta_1=${msg.beta_1} D=${msg.delta_beta_1 >= 0 ? "+" : ""}${msg.delta_beta_1}`,
              importance,
              type: msg.operation,
            });
          }
        } else if (msg.type === "gap") {
          newGaps.unshift({
            concept: msg.concept,
            degree: msg.degree,
            avg_neighbor_degree: msg.avg_degree,
            status: "searching",
          });
          newNarrative.push({
            time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
            level: 2,
            text: `gap detected: '${msg.concept}' (degree=${msg.degree}, avg_neighbor=${msg.avg_degree})`,
            importance: 0.62,
          });
        } else if (msg.type === "feed") {
          newNarrative.push({
            time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
            level: 2,
            text: `feed: ${msg.source} accepted=${msg.accepted}, verdict=${msg.verdict}, +${msg.new_vertices}V`,
            importance: 0.72,
          });
        } else if (msg.type === "expression_pressure") {
          // Update pressure indicator
          expressionPressure = msg.count;
          // If daemon is actively sharing a finding, push it as a daemon message
          if (msg.text) {
            newMessages.push({
              role: "daemon",
              text: msg.text,
              timestamp: Date.now(),
            });
          }
        } else if (msg.type === "peer_position") {
          // Update peer instance position from SharedLayer cross-instance sync
          const existing = updatedPeers[msg.instance];
          updatedPeers[msg.instance] = {
            posLabel: msg.position_label,
            color: existing?.color || _assignPeerColor(msg.instance, updatedPeers),
            step: msg.step ?? 0,
            online: true,
          };
          peersUpdated = true;
        }
      }

      // Merge narrative (newest first, cap at 200)
      const merged = [...newNarrative.reverse(), ...state.narrative].slice(0, 200);

      return {
        beta1History: history,
        currentPositionLabel,
        currentPositionId,
        narrative: merged,
        gaps: newGaps.slice(0, 30),
        expressionPressure,
        peersPositions: peersUpdated ? updatedPeers : state.peersPositions,
        messages: newMessages.length > 0
          ? [...state.messages, ...newMessages]
          : state.messages,
      };
    }),

  updatePeerPosition: (instanceId, posLabel, step) =>
    set((state) => {
      const existing = state.peersPositions[instanceId];
      return {
        peersPositions: {
          ...state.peersPositions,
          [instanceId]: {
            posLabel,
            color: existing?.color || _assignPeerColor(instanceId, state.peersPositions),
            step: step ?? existing?.step ?? 0,
            online: true,
          },
        },
      };
    }),
}));
