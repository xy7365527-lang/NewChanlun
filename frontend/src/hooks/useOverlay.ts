import { useEffect, useRef, useState } from "react";
import type { ISeriesApi } from "lightweight-charts";
import { getOverlay } from "../api/client";
import type { OverlayResponse, OverlayLStar } from "../types/overlay";
import { ChanTheoryPrimitive } from "../primitives/ChanTheoryPrimitive";

interface OverlayOptions {
  candleSeries: ISeriesApi<"Candlestick"> | null;
  symbol: string;
  interval: string;
  tf: string;
}

/**
 * 缠论 overlay：
 * - ChanTheoryPrimitive 绘制笔线/段线/中枢矩形框
 * - L* 状态供 StatusBadge 显示
 * - 每 60 秒刷新
 */
export function useOverlay({ candleSeries, symbol, interval, tf }: OverlayOptions) {
  const [lstar, setLstar] = useState<OverlayLStar | null>(null);
  const [overlay, setOverlay] = useState<OverlayResponse | null>(null);
  const primitiveRef = useRef<ChanTheoryPrimitive | null>(null);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const reqSeqRef = useRef(0);
  const latestKeyRef = useRef("");

  useEffect(() => {
    latestKeyRef.current = `${symbol}|${interval}|${tf}`;
  }, [symbol, interval, tf]);

  // 创建 / 销毁 primitive
  useEffect(() => {
    if (!candleSeries) return;

    const primitive = new ChanTheoryPrimitive();
    candleSeries.attachPrimitive(primitive);
    primitiveRef.current = primitive;

    return () => {
      try {
        candleSeries.detachPrimitive(primitive);
      } catch { /* already detached */ }
      primitiveRef.current = null;
    };
  }, [candleSeries]);

  // 加载 overlay 数据
  async function loadOverlay() {
    ++reqSeqRef.current;
    if (!symbol || !primitiveRef.current) return;
    try {
      const res = await getOverlay({ symbol, interval, tf });
      if (!res.schema_version?.startsWith("newchan_overlay_v")) {
        return;
      }
      setOverlay(res);
      setLstar(res.lstar);
      primitiveRef.current.setData(res);
    } catch (e) {
      console.error("[overlay] load failed:", e);
    }
  }

  // 首次加载 + symbol/tf 切换
  useEffect(() => {
    if (!candleSeries || !symbol) return;

    // 延迟加载（等 K线 setData 完成）
    const timer = setTimeout(() => loadOverlay(), 800);

    // 每 60 秒刷新
    pollingRef.current = setInterval(() => loadOverlay(), 60_000);

    return () => {
      clearTimeout(timer);
      if (pollingRef.current) clearInterval(pollingRef.current);
    };
  }, [candleSeries, symbol, interval, tf]);

  return { lstar, overlay };
}
