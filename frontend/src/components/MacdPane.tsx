import { useEffect, useRef } from "react";
import {
  createChart,
  HistogramSeries,
  type IChartApi,
  type ISeriesApi,
  type Time,
  ColorType,
} from "lightweight-charts";
import type { OverlayResponse } from "../types/overlay";

interface Props {
  overlay: OverlayResponse | null;
  /** 主图 chart 实例，用于同步时间轴 */
  mainChart: IChartApi | null;
}

export function MacdPane({ overlay, mainChart }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const histRef = useRef<ISeriesApi<"Histogram"> | null>(null);

  // 创建 MACD 子图
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const chart = createChart(el, {
      layout: {
        background: { type: ColorType.Solid, color: "#131722" },
        textColor: "#787b86",
      },
      grid: {
        vertLines: { color: "#1f2430" },
        horzLines: { color: "#1f2430" },
      },
      rightPriceScale: { borderColor: "#2a2e39" },
      timeScale: {
        borderColor: "#2a2e39",
        timeVisible: true,
        secondsVisible: false,
        visible: false,
      },
      crosshair: { mode: 0 },
      height: 140,
    });

    const hist = chart.addSeries(HistogramSeries, {
      priceFormat: { type: "price", precision: 4, minMove: 0.0001 },
      base: 0,
    });

    chartRef.current = chart;
    histRef.current = hist;

    // resize
    const ro = new ResizeObserver(() => {
      chart.applyOptions({ width: el.clientWidth });
    });
    ro.observe(el);

    return () => {
      ro.disconnect();
      chart.remove();
      chartRef.current = null;
      histRef.current = null;
    };
  }, []);

  // 同步时间轴
  useEffect(() => {
    if (!mainChart || !chartRef.current) return;

    function sync(range: any) {
      if (range && chartRef.current) {
        chartRef.current.timeScale().setVisibleLogicalRange(range);
      }
    }

    mainChart.timeScale().subscribeVisibleLogicalRangeChange(sync);
    return () => {
      mainChart.timeScale().unsubscribeVisibleLogicalRangeChange(sync);
    };
  }, [mainChart]);

  // 更新数据
  useEffect(() => {
    if (!histRef.current || !overlay?.macd?.series) return;

    const data = overlay.macd.series.map((p) => ({
      time: p.time as unknown as Time,
      value: p.hist,
      color:
        p.hist >= 0
          ? "rgba(38,166,154,0.6)"
          : "rgba(239,83,80,0.6)",
    }));

    histRef.current.setData(data);
  }, [overlay]);

  return (
    <div
      ref={containerRef}
      style={{
        height: 140,
        borderTop: "1px solid #2a2e39",
        flexShrink: 0,
        position: "relative",
      }}
    >
      <span
        style={{
          position: "absolute",
          top: 2,
          left: 6,
          fontSize: 10,
          color: "#787b86",
          zIndex: 2,
        }}
      >
        MACD (NewChan)
      </span>
    </div>
  );
}
