import { create } from "zustand";

interface AppState {
  symbol: string;
  interval: string; // 缓存 key 用的周期，如 "1min"
  tf: string; // 显示周期，如 "1m", "5m", "1h", "1d"
  detail: "min" | "full";

  setSymbol: (s: string) => void;
  setTf: (tf: string) => void;
}

// tf → interval 映射：前端显示周期 → 后端缓存周期
// 所有 tf 都从 1min 缓存 resample，所以 interval 总是 "1min"
const TF_TO_INTERVAL: Record<string, string> = {
  "1m": "1min",
  "5m": "1min",
  "15m": "1min",
  "30m": "1min",
  "1h": "1min",
  "4h": "1min",
  "1d": "1min",
  "1w": "1min",
};

export const SUPPORTED_TFS = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w"];

export const useAppStore = create<AppState>((set) => ({
  symbol: "BZ",
  interval: "1min",
  tf: "1m",
  detail: "full",

  setSymbol: (symbol) => set({ symbol: symbol.toUpperCase() }),
  setTf: (tf) => set({ tf, interval: TF_TO_INTERVAL[tf] ?? "1min" }),
}));
