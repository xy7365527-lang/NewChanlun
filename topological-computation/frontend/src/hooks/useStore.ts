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
import type { DaemonInstance } from "../tokens";
import { DEFAULT_INSTANCES, INSTANCE_STORAGE_KEY, INSTANCE_COLORS } from "../tokens";

interface Beta1Point {
  step: number;
  beta1: number;
}

// Per-instance connection/status state
export interface InstanceState {
  wsConnected: boolean;
  reachable: boolean;
  status: StatusResponse | null;
  currentPositionLabel: string;
  currentPositionId: string;
  beta1History: Beta1Point[];
}

// ── Filter state ──────────────────────────────────────────────────
export interface FilterState {
  /** Enabled domains (empty = show all) */
  enabledDomains: Set<string>;
  /** Minimum degree threshold for activity filter */
  minDegree: number;
  /** Settlement filter: "all" | "settled" | "unsettled" */
  settlementFilter: "all" | "settled" | "unsettled";
  /** Visible instance IDs (empty = show all) */
  visibleInstances: Set<string>;
}

const FILTER_STORAGE_KEY = "fl-topology-filters";

function loadFilters(): FilterState {
  try {
    const raw = localStorage.getItem(FILTER_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        enabledDomains: new Set(parsed.enabledDomains ?? []),
        minDegree: parsed.minDegree ?? 0,
        settlementFilter: parsed.settlementFilter ?? "all",
        visibleInstances: new Set(parsed.visibleInstances ?? []),
      };
    }
  } catch { /* ignore */ }
  return {
    enabledDomains: new Set(),
    minDegree: 0,
    settlementFilter: "all",
    visibleInstances: new Set(),
  };
}

function saveFilters(f: FilterState): void {
  localStorage.setItem(FILTER_STORAGE_KEY, JSON.stringify({
    enabledDomains: [...f.enabledDomains],
    minDegree: f.minDegree,
    settlementFilter: f.settlementFilter,
    visibleInstances: [...f.visibleInstances],
  }));
}

const INSTANCE_VERSION = 3; // bump to force reset cached instances

function loadInstances(): DaemonInstance[] {
  try {
    const ver = localStorage.getItem(INSTANCE_STORAGE_KEY + "_v");
    if (ver !== String(INSTANCE_VERSION)) {
      localStorage.removeItem(INSTANCE_STORAGE_KEY);
      localStorage.setItem(INSTANCE_STORAGE_KEY + "_v", String(INSTANCE_VERSION));
      return DEFAULT_INSTANCES;
    }
    const raw = localStorage.getItem(INSTANCE_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as DaemonInstance[];
      if (Array.isArray(parsed) && parsed.length > 0) {
        return parsed.map(({ id, name, httpBase, wsUrl }) => ({ id, name, httpBase, wsUrl }));
      }
    }
  } catch { /* ignore */ }
  return DEFAULT_INSTANCES;
}

function saveInstances(instances: DaemonInstance[]): void {
  localStorage.setItem(INSTANCE_STORAGE_KEY, JSON.stringify(instances));
}

function assignInstanceColor(index: number): string {
  return INSTANCE_COLORS[index % INSTANCE_COLORS.length];
}

interface DaemonStore {
  // ── Multi-instance config ──────────────────────────────────
  instances: DaemonInstance[];
  instanceStates: Record<string, InstanceState>;
  addInstance: (inst: DaemonInstance) => void;
  removeInstance: (id: string) => void;
  updateInstanceState: (id: string, partial: Partial<InstanceState>) => void;

  // ── Connection (any reachable instance) ─────────────────────
  wsConnected: boolean;
  daemonReachable: boolean;

  // Metrics (from first reachable instance — shared K_active)
  status: StatusResponse | null;

  // beta_1 history for the curve (capped at 500 points)
  beta1History: Beta1Point[];

  // Topology (shared K_active, fetched from any reachable instance)
  topology: TopologyResponse | null;

  // Focus concept (drives /query calls)
  focusConcept: string | null;
  queryResult: import("../types").QueryResponse | null;

  // Narrative stream (merged from all instances)
  narrative: NarrativeEvent[];

  // Gap queue (from shared K_active)
  gaps: GapEntry[];

  // Operations (from shared K_active)
  operations: OperationStats | null;

  // Chat messages
  messages: ChatMessage[];

  // Expression pressure: number of unreported high-I events
  expressionPressure: number;

  // Peer instance positions (merged from all instances)
  peersPositions: Record<string, { posLabel: string; color: string; step: number; online: boolean }>;

  // ── Filter state ────────────────────────────────────────────
  filters: FilterState;
  setFilterDomains: (domains: Set<string>) => void;
  setFilterMinDegree: (minDegree: number) => void;
  setFilterSettlement: (v: "all" | "settled" | "unsettled") => void;
  setFilterVisibleInstances: (ids: Set<string>) => void;
  toggleFilterDomain: (domain: string) => void;
  toggleFilterInstance: (instanceId: string) => void;

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
  /** Handle WS batch from any instance — all feed shared state */
  handleWsBatch: (instanceId: string, msgs: WsMessage[]) => void;
  updatePeerPosition: (instanceId: string, posLabel: string, step?: number) => void;

  // Computed helper: first reachable instance's HTTP base
  getReachableHttpBase: () => string;
}

export const useStore = create<DaemonStore>((set, get) => ({
  // ── Multi-instance ──────────────────────────────────────────
  instances: loadInstances(),
  instanceStates: {},

  addInstance: (inst) =>
    set((state) => {
      const updated = [...state.instances, inst];
      saveInstances(updated);
      return { instances: updated };
    }),

  removeInstance: (id) =>
    set((state) => {
      const updated = state.instances.filter((i) => i.id !== id);
      if (updated.length === 0) return state; // don't remove last
      const { [id]: _removed, ...restStates } = state.instanceStates;
      saveInstances(updated);
      return { instances: updated, instanceStates: restStates };
    }),

  updateInstanceState: (id, partial) =>
    set((state) => ({
      instanceStates: {
        ...state.instanceStates,
        [id]: { ...defaultInstanceState(), ...state.instanceStates[id], ...partial },
      },
    })),

  // ── Aggregated fields ─────────────────────────────────────────
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
  expressionPressure: 0,
  peersPositions: {},

  // ── Filters ──────────────────────────────────────────────────
  filters: loadFilters(),

  setFilterDomains: (domains) =>
    set((state) => {
      const f = { ...state.filters, enabledDomains: domains };
      saveFilters(f);
      return { filters: f };
    }),

  setFilterMinDegree: (minDegree) =>
    set((state) => {
      const f = { ...state.filters, minDegree };
      saveFilters(f);
      return { filters: f };
    }),

  setFilterSettlement: (v) =>
    set((state) => {
      const f = { ...state.filters, settlementFilter: v };
      saveFilters(f);
      return { filters: f };
    }),

  setFilterVisibleInstances: (ids) =>
    set((state) => {
      const f = { ...state.filters, visibleInstances: ids };
      saveFilters(f);
      return { filters: f };
    }),

  toggleFilterDomain: (domain) =>
    set((state) => {
      const current = new Set(state.filters.enabledDomains);
      if (current.has(domain)) current.delete(domain);
      else current.add(domain);
      const f = { ...state.filters, enabledDomains: current };
      saveFilters(f);
      return { filters: f };
    }),

  toggleFilterInstance: (instanceId) =>
    set((state) => {
      const current = new Set(state.filters.visibleInstances);
      if (current.has(instanceId)) current.delete(instanceId);
      else current.add(instanceId);
      const f = { ...state.filters, visibleInstances: current };
      saveFilters(f);
      return { filters: f };
    }),

  setWsConnected: (v) => set({ wsConnected: v }),
  setDaemonReachable: (v) => set({ daemonReachable: v }),

  setStatus: (s) =>
    set((state) => {
      const last = state.beta1History[state.beta1History.length - 1];
      const newPt: Beta1Point = { step: s.steps, beta1: s.beta_1 };
      const shouldAppend = !last || last.step !== s.steps;
      const history = shouldAppend
        ? [...state.beta1History, newPt].slice(-500)
        : state.beta1History;
      const pressure = s.expression_pressure !== undefined
        ? s.expression_pressure
        : state.expressionPressure;
      return { status: s, beta1History: history, daemonReachable: true, expressionPressure: pressure };
    }),

  setTopology: (t) => set({ topology: t }),
  setFocusConcept: (c) => set({ focusConcept: c }),
  setQueryResult: (r) => set({ queryResult: r }),
  setNarrative: (events) => set({ narrative: events }),
  setGaps: (gaps) => set({ gaps }),
  setOperations: (ops) => set({ operations: ops }),

  addMessage: (msg) =>
    set((state) => ({ messages: [...state.messages, msg] })),

  // All instances feed into the same handler.
  // Each instance is a traverser on the shared K_active.
  // Step messages update per-instance position in instanceStates.
  // Narrative/gap/feed/pressure events are merged into the shared stream.
  handleWsBatch: (instanceId, msgs) =>
    set((state) => {
      let history = [...state.beta1History];
      let expressionPressure = state.expressionPressure;
      const newNarrative: NarrativeEvent[] = [];
      const newGaps: GapEntry[] = [...state.gaps];
      const newMessages: ChatMessage[] = [];

      // Per-instance position tracking
      const prevInst = state.instanceStates[instanceId] || defaultInstanceState();
      let instPosLabel = prevInst.currentPositionLabel;
      let instPosId = prevInst.currentPositionId;
      let instHistory = [...prevInst.beta1History];

      // Peer position updates
      let peersUpdated = false;
      const updatedPeers = { ...state.peersPositions };

      for (const msg of msgs) {
        if (msg.type === "step") {
          // Update per-instance position
          instPosLabel = msg.position_label;
          instPosId = msg.position;
          const lastInst = instHistory[instHistory.length - 1];
          if (!lastInst || lastInst.step !== msg.step) {
            instHistory.push({ step: msg.step, beta1: msg.beta_1 });
            if (instHistory.length > 500) instHistory = instHistory.slice(-500);
          }

          // Update shared beta1 history
          const last = history[history.length - 1];
          if (!last || last.step !== msg.step) {
            history.push({ step: msg.step, beta1: msg.beta_1 });
            if (history.length > 500) history = history.slice(-500);
          }

          if (msg.delta_beta_1 !== 0 || msg.operation !== "walk") {
            const importance = Math.min(
              1.0,
              Math.abs(msg.delta_beta_1) * 0.3 + (msg.operation !== "walk" ? 0.3 : 0)
            );
            // Find instance name for narrative attribution
            const instName = state.instances.find((i) => i.id === instanceId)?.name ?? instanceId;
            newNarrative.push({
              time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
              level: importance > 0.8 ? 3 : importance > 0.4 ? 2 : 1,
              text: `[${instName} 步 ${msg.step}] ${msg.operation} @ ${msg.position_label} beta_1=${msg.beta_1} D=${msg.delta_beta_1 >= 0 ? "+" : ""}${msg.delta_beta_1}`,
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
          expressionPressure = msg.count;
          if (msg.text) {
            newMessages.push({
              role: "daemon",
              text: msg.text,
              timestamp: Date.now(),
            });
          }
        } else if (msg.type === "peer_position") {
          const existing = updatedPeers[msg.instance];
          updatedPeers[msg.instance] = {
            posLabel: msg.position_label,
            color: existing?.color || assignInstanceColor(Object.keys(updatedPeers).length),
            step: msg.step ?? 0,
            online: true,
          };
          peersUpdated = true;
        }
      }

      const merged = [...newNarrative.reverse(), ...state.narrative].slice(0, 200);

      return {
        beta1History: history,
        narrative: merged,
        gaps: newGaps.slice(0, 30),
        expressionPressure,
        peersPositions: peersUpdated ? updatedPeers : state.peersPositions,
        messages: newMessages.length > 0
          ? [...state.messages, ...newMessages]
          : state.messages,
        // Update this instance's state
        instanceStates: {
          ...state.instanceStates,
          [instanceId]: {
            ...prevInst,
            currentPositionLabel: instPosLabel,
            currentPositionId: instPosId,
            beta1History: instHistory,
          },
        },
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
            color: existing?.color || assignInstanceColor(Object.keys(state.peersPositions).length),
            step: step ?? existing?.step ?? 0,
            online: true,
          },
        },
      };
    }),

  // First reachable instance's HTTP base (they all share the same K_active)
  getReachableHttpBase: () => {
    const state = get();
    for (const inst of state.instances) {
      const s = state.instanceStates[inst.id];
      if (s?.reachable) return inst.httpBase;
    }
    // Fallback: first instance
    return state.instances[0]?.httpBase ?? "http://46.225.187.39:9765";
  },
}));

function defaultInstanceState(): InstanceState {
  return {
    wsConnected: false,
    reachable: false,
    status: null,
    currentPositionLabel: "",
    currentPositionId: "",
    beta1History: [],
  };
}
