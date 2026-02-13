import { create } from "zustand";
import type { ChanEvent, ReplayStatus } from "../types/events";

interface ReplayState {
  isReplaying: boolean;
  sessionId: string | null;
  status: ReplayStatus;
  events: ChanEvent[];
  maxEvents: number;

  setReplaying: (v: boolean) => void;
  setSessionId: (id: string | null) => void;
  updateStatus: (s: Partial<ReplayStatus>) => void;
  pushEvent: (ev: ChanEvent) => void;
  clearEvents: () => void;
}

const DEFAULT_STATUS: ReplayStatus = {
  mode: "idle",
  currentIdx: 0,
  totalBars: 0,
  speed: 1,
};

export const useReplayStore = create<ReplayState>((set, get) => ({
  isReplaying: false,
  sessionId: null,
  status: { ...DEFAULT_STATUS },
  events: [],
  maxEvents: 200,

  setReplaying: (v) => set({ isReplaying: v }),

  setSessionId: (id) => set({ sessionId: id }),

  updateStatus: (s) =>
    set((state) => ({ status: { ...state.status, ...s } })),

  pushEvent: (ev) =>
    set((state) => {
      const next = [...state.events, ev];
      if (next.length > state.maxEvents) {
        next.splice(0, next.length - state.maxEvents);
      }
      return { events: next };
    }),

  clearEvents: () => set({ events: [] }),
}));
