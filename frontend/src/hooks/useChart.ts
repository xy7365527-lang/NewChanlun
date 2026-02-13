import { useEffect, useState } from "react";
import {
  createChart,
  CandlestickSeries,
  HistogramSeries,
  type IChartApi,
  type ISeriesApi,
  ColorType,
} from "lightweight-charts";

export interface ChartRefs {
  chart: IChartApi;
  candleSeries: ISeriesApi<"Candlestick">;
  volumeSeries: ISeriesApi<"Histogram">;
}

/**
 * 创建并管理 Lightweight Charts 实例。
 * 返回 state（触发重渲染让子 hook 能拿到 chart 实例）。
 */
export function useChart(containerRef: React.RefObject<HTMLDivElement | null>) {
  const [refs, setRefs] = useState<ChartRefs | null>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const chart = createChart(container, {
      layout: {
        background: { type: ColorType.Solid, color: "#131722" },
        textColor: "#d1d4dc",
      },
      grid: {
        vertLines: { color: "#1e222d" },
        horzLines: { color: "#1e222d" },
      },
      crosshair: { mode: 0 },
      rightPriceScale: { borderColor: "#2a2e39" },
      timeScale: {
        borderColor: "#2a2e39",
        timeVisible: true,
        secondsVisible: false,
      },
    });

    const candleSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#26a69a",
      downColor: "#ef5350",
      borderUpColor: "#26a69a",
      borderDownColor: "#ef5350",
      wickUpColor: "#26a69a",
      wickDownColor: "#ef5350",
    });

    const volumeSeries = chart.addSeries(HistogramSeries, {
      priceFormat: { type: "volume" },
      priceScaleId: "vol",
    });
    chart.priceScale("vol").applyOptions({
      scaleMargins: { top: 0.82, bottom: 0 },
    });

    setRefs({ chart, candleSeries, volumeSeries });

    // 响应式 resize
    const ro = new ResizeObserver(() => {
      chart.applyOptions({
        width: container.clientWidth,
        height: container.clientHeight,
      });
    });
    ro.observe(container);

    return () => {
      ro.disconnect();
      chart.remove();
      setRefs(null);
    };
  }, [containerRef]);

  return refs;
}
