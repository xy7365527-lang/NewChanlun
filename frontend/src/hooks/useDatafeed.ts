import { useEffect, useRef } from "react";
import type { IChartApi, ISeriesApi, Time, CandlestickData, HistogramData } from "lightweight-charts";
import { getOhlcv } from "../api/client";
import type { OhlcvBar } from "../types/overlay";

/** 将后端 bar 转为 LW Charts 格式 */
function convertBar(b: OhlcvBar): CandlestickData<Time> {
  return {
    time: (Math.floor(new Date(b.time + "Z").getTime() / 1000)) as unknown as Time,
    open: b.open,
    high: b.high,
    low: b.low,
    close: b.close,
  };
}

function convertVolume(b: OhlcvBar): HistogramData<Time> {
  return {
    time: (Math.floor(new Date(b.time + "Z").getTime() / 1000)) as unknown as Time,
    value: b.volume ?? 0,
    color: b.close >= b.open ? "rgba(38,166,154,0.3)" : "rgba(239,83,80,0.3)",
  };
}

interface DatafeedOptions {
  chart: IChartApi | null;
  candleSeries: ISeriesApi<"Candlestick"> | null;
  volumeSeries: ISeriesApi<"Histogram"> | null;
  symbol: string;
  interval: string;
  tf: string;
}

/**
 * 按需加载 + 实时更新
 *
 * - 初始加载最近 500 条
 * - 用户往前翻时自动加载更早数据
 * - 每 15 秒轮询增量更新
 */
export function useDatafeed({
  chart,
  candleSeries,
  volumeSeries,
  symbol,
  interval,
  tf,
}: DatafeedOptions) {
  const loadingMore = useRef(false);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // 初始加载
  useEffect(() => {
    if (!chart || !candleSeries || !volumeSeries || !symbol) return;

    let cancelled = false;

    async function load() {
      try {
        const res = await getOhlcv({ symbol, interval, tf, countBack: 500 });
        if (cancelled || !res.data?.length) return;

        const candles = res.data.map(convertBar);
        const volumes = res.data.map(convertVolume);

        candleSeries!.setData(candles);
        volumeSeries!.setData(volumes);
        chart!.timeScale().fitContent();
      } catch (e) {
        console.error("[datafeed] initial load failed:", e);
      }
    }

    load();

    return () => { cancelled = true; };
  }, [chart, candleSeries, volumeSeries, symbol, interval, tf]);

  // 往前翻加载更早数据
  useEffect(() => {
    if (!chart || !candleSeries || !volumeSeries) return;

    function onVisibleRangeChanged() {
      if (loadingMore.current) return;

      const range = chart!.timeScale().getVisibleLogicalRange();
      if (!range) return;

      const barsInfo = candleSeries!.barsInLogicalRange(range);
      if (barsInfo && barsInfo.barsBefore !== null && barsInfo.barsBefore < 50) {
        loadingMore.current = true;

        const allData = candleSeries!.data() as CandlestickData<Time>[];
        if (!allData.length) { loadingMore.current = false; return; }

        const earliest = allData[0].time as unknown as number;

        getOhlcv({ symbol, interval, tf, to: earliest - 1, countBack: 300 })
          .then((res) => {
            if (!res.data?.length) return;
            const olderCandles = res.data.map(convertBar);
            const olderVolumes = res.data.map(convertVolume);
            const currentCandles = candleSeries!.data() as CandlestickData<Time>[];
            const currentVolumes = volumeSeries!.data() as HistogramData<Time>[];

            candleSeries!.setData([...olderCandles, ...currentCandles]);
            volumeSeries!.setData([...olderVolumes, ...currentVolumes]);
          })
          .catch((e) => console.error("[datafeed] load more failed:", e))
          .finally(() => { loadingMore.current = false; });
      }
    }

    chart.timeScale().subscribeVisibleLogicalRangeChange(onVisibleRangeChanged);
    return () => {
      chart.timeScale().unsubscribeVisibleLogicalRangeChange(onVisibleRangeChanged);
    };
  }, [chart, candleSeries, volumeSeries, symbol, interval, tf]);

  // 实时轮询增量
  useEffect(() => {
    if (!candleSeries || !volumeSeries || !symbol) return;

    pollingRef.current = setInterval(async () => {
      try {
        const allData = candleSeries.data() as CandlestickData<Time>[];
        if (!allData.length) return;

        const latest = allData[allData.length - 1].time as unknown as number;
        const res = await getOhlcv({ symbol, interval, tf, after: latest });
        if (!res.data?.length) return;

        for (const bar of res.data) {
          candleSeries.update(convertBar(bar));
          volumeSeries.update(convertVolume(bar));
        }
      } catch (e) {
        // 静默失败，下次轮询重试
      }
    }, 15_000);

    return () => {
      if (pollingRef.current) clearInterval(pollingRef.current);
    };
  }, [candleSeries, volumeSeries, symbol, interval, tf]);
}
