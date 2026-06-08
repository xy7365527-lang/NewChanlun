"""多标的全量递归引擎运行 + JSON 结果导出。

支持三种数据格式：
- Databento columnar: {opens, highs, lows, closes, volumes}
- TWS row: [{date, open, high, low, close, volume}, ...]
- Polygon row: {bars: [{ts, open, high, low, close, volume}, ...]}

用法：
  PYTHONPATH=src python analysis/run_databento_recursive.py
  PYTHONPATH=src python analysis/run_databento_recursive.py --symbol QQQ
  PYTHONPATH=src python analysis/run_databento_recursive.py --symbol HK700
"""

from __future__ import annotations

import argparse
import json
import time
from datetime import datetime, timezone
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

DATA_DIR = Path(__file__).resolve().parent / "data_cache"
OUT_DIR = Path(__file__).resolve().parent / "data_cache"

DATASETS: dict[str, tuple[Path, str]] = {
    "QQQ": (DATA_DIR / "qqq_1m_databento_full.json", "databento"),
    "OKLO": (DATA_DIR / "oklo_1m_databento_full.json", "databento"),
    "HK700": (DATA_DIR / "hk700_1m_tws.json", "tws"),
}


def _parse_ts(s: str) -> datetime:
    for fmt in ("%Y-%m-%d %H:%M:%S%z", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(s, fmt)
        except ValueError:
            continue
    return datetime(2024, 1, 1, tzinfo=timezone.utc)


def load_bars(path: Path, fmt: str) -> list[Bar]:
    with open(path) as f:
        raw = json.load(f)

    if fmt == "databento":
        from datetime import timedelta
        opens = raw["opens"]
        highs = raw["highs"]
        lows = raw["lows"]
        closes = raw["closes"]
        volumes = raw.get("volumes", [None] * len(opens))
        base_ts = datetime(2024, 1, 1, 9, 30, tzinfo=timezone.utc)
        return [
            Bar(
                ts=base_ts + timedelta(minutes=i),
                open=opens[i], high=highs[i], low=lows[i], close=closes[i],
                volume=volumes[i],
            )
            for i in range(len(opens))
        ]

    if fmt == "tws":
        return [
            Bar(
                ts=_parse_ts(b["date"]),
                open=b["open"], high=b["high"], low=b["low"], close=b["close"],
                volume=b.get("volume"),
            )
            for b in raw
        ]

    if fmt == "polygon":
        return [
            Bar(
                ts=_parse_ts(b["ts"]) if isinstance(b["ts"], str) else datetime(2024, 1, 1, tzinfo=timezone.utc),
                open=b["open"], high=b["high"], low=b["low"], close=b["close"],
                volume=b.get("volume"),
            )
            for b in raw["bars"]
        ]

    raise ValueError(f"Unknown format: {fmt}")


def run_recursive(symbol: str, bars: list[Bar]) -> dict:
    from newchan.a_macd import OnlineMacdState
    from newchan.a_persistence_barcode import sublevel_h0_bars

    print(f"\n{'='*70}")
    print(f"  {symbol}: {len(bars)} bars")
    print(f"{'='*70}")

    orch = RecursiveOrchestrator(
        stream_id=f"{symbol}_1min",
        max_levels=8,
        stroke_mode="new",
        min_strict_sep=5,
    )
    macd_state = OnlineMacdState()

    t0 = time.time()
    snap = None
    macd_values: list[tuple[float, float, float]] = []
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        m, s, h = macd_state.update(bar.close, bar.ts)
        macd_values.append((m, s, h))
        if (i + 1) % 50000 == 0:
            e = time.time() - t0
            bi = len(snap.bi_snapshot.strokes)
            sg = len(snap.seg_snapshot.segments)
            zs = len(snap.zs_snapshot.zhongshus)
            mv = len(snap.move_snapshot.moves)
            rc = len(snap.recursive_snapshots)
            print(
                f"  [{i+1:>7d}/{len(bars)}] {e:>6.1f}s | "
                f"bi={bi:>5d} seg={sg:>4d} zs={zs:>4d} mv={mv:>3d} rec_levels={rc}",
                flush=True,
            )
    elapsed = time.time() - t0
    print(f"\n  Done in {elapsed:.1f}s ({len(bars)/elapsed:.0f} bars/s)")

    print("  Computing PH barcode (batch)...", flush=True)
    prices = [bar.close for bar in bars]
    ph_bars = sublevel_h0_bars(prices)
    print(f"  PH: {len(ph_bars)} bars")

    result = extract_result(snap, bars, symbol, elapsed, macd_values, ph_bars)
    print_summary(result, symbol)
    return result


def extract_result(snap, bars: list[Bar], symbol: str, elapsed: float,
                   macd_values=None, barcode=None) -> dict:
    strokes = snap.bi_snapshot.strokes
    segments = snap.seg_snapshot.segments
    zhongshus = snap.zs_snapshot.zhongshus
    moves = snap.move_snapshot.moves
    rec_snaps = snap.recursive_snapshots
    bsps = snap.bsp_snapshot.buysellpoints
    m2r = snap.bi_snapshot.merged_to_raw

    def raw_idx(merged: int) -> int:
        if merged < len(m2r):
            return m2r[merged][0]
        return min(merged, len(bars) - 1)

    def bar_ts(idx: int) -> str:
        return bars[min(idx, len(bars) - 1)].ts.isoformat()

    def merged_ts(merged: int) -> str:
        return bar_ts(raw_idx(merged))

    ohlc = [
        {"t": bar_ts(i), "o": b.open, "h": b.high, "l": b.low, "c": b.close}
        for i, b in enumerate(bars)
    ]

    stroke_data = [
        {
            "i0": s.i0, "i1": s.i1, "dir": s.direction,
            "p0": round(s.p0, 4), "p1": round(s.p1, 4),
            "t0": merged_ts(s.i0), "t1": merged_ts(s.i1),
        }
        for s in strokes
    ]

    def _seg_raw_i0(sg):
        si = min(sg.s0, len(strokes) - 1)
        return raw_idx(strokes[si].i0)

    def _seg_raw_i1(sg):
        si = min(sg.s1, len(strokes) - 1)
        return raw_idx(strokes[si].i1)

    seg_data = [
        {
            "s0": sg.s0, "s1": sg.s1, "dir": sg.direction,
            "p0": round(sg.p0, 4), "p1": round(sg.p1, 4),
            "h": round(sg.high, 4), "l": round(sg.low, 4),
            "bi0": _seg_raw_i0(sg),
            "bi1": _seg_raw_i1(sg),
            "t0": bar_ts(_seg_raw_i0(sg)),
            "t1": bar_ts(_seg_raw_i1(sg)),
        }
        for sg in segments
    ]

    def _zs_raw_bounds(z):
        if not segments:
            return 0, 0
        si0 = min(z.seg_start, len(segments) - 1)
        si1 = min(z.seg_end, len(segments) - 1)
        bi0 = raw_idx(strokes[min(segments[si0].s0, len(strokes) - 1)].i0)
        bi1 = raw_idx(strokes[min(segments[si1].s1, len(strokes) - 1)].i1)
        return bi0, bi1

    zs_data = []
    for z in zhongshus:
        bi0, bi1 = _zs_raw_bounds(z)
        zs_data.append({
            "zg": round(z.zg, 4), "zd": round(z.zd, 4),
            "gg": round(z.gg, 4), "dd": round(z.dd, 4),
            "seg_s": z.seg_start, "seg_e": z.seg_end,
            "cnt": z.seg_count, "settled": z.settled,
            "bi0": bi0, "bi1": bi1,
            "t0": bar_ts(bi0), "t1": bar_ts(bi1),
        })

    mv_data = [
        {
            "kind": m.kind, "dir": m.direction,
            "zs_cnt": m.zs_count, "settled": m.settled,
            "h": round(m.high, 4), "l": round(m.low, 4),
            "seg_s": m.seg_start, "seg_e": m.seg_end,
            "pers": round(m.persistence, 4),
        }
        for m in moves
    ]

    bsp_data = [
        {
            "kind": b.kind, "side": b.side,
            "price": round(b.price, 4), "bar_idx": b.bar_idx,
            "ts": bar_ts(b.bar_idx),
            "confirmed": b.confirmed, "settled": b.settled,
        }
        for b in bsps
    ]

    rec_data = {}
    for rs in rec_snaps:
        lv = rs.level_id
        zs_list = [
            {
                "zg": round(z.zg, 4), "zd": round(z.zd, 4),
                "gg": round(z.gg, 4), "dd": round(z.dd, 4),
                "comp_s": z.comp_start, "comp_e": z.comp_end,
                "cnt": z.comp_count, "settled": z.settled,
            }
            for z in rs.zhongshus
        ]
        mv_list = [
            {
                "kind": m.kind, "dir": m.direction,
                "zs_cnt": m.zs_count, "settled": m.settled,
                "h": round(m.high, 4), "l": round(m.low, 4),
                "seg_s": m.seg_start, "seg_e": m.seg_end,
                "pers": round(m.persistence, 4),
            }
            for m in rs.moves
        ]
        rec_data[f"L{lv}"] = {"zhongshus": zs_list, "moves": mv_list}

    macd_data = []
    if macd_values:
        step = max(1, len(macd_values) // 80000)
        for i in range(0, len(macd_values), step):
            m, s, h = macd_values[i]
            if m is not None:
                macd_data.append({
                    "t": bar_ts(i), "m": round(m, 6), "s": round(s, 6), "h": round(h, 6),
                })

    ph_data = []
    if barcode:
        for b in barcode:
            ph_data.append({
                "bp": round(b.birth, 4),
                "dp": round(b.death, 4) if b.death != float('inf') else None,
                "pers": round(b.persistence, 4),
                "dim": b.dimension,
                "settled": True,
            })

    return {
        "meta": {
            "symbol": symbol, "bars": len(bars), "elapsed_s": round(elapsed, 1),
        },
        "ohlc": ohlc,
        "L1": {
            "strokes": stroke_data, "segments": seg_data,
            "zhongshus": zs_data, "moves": mv_data, "bsp": bsp_data,
        },
        "recursive": rec_data,
        "macd": macd_data,
        "ph": ph_data,
    }


def print_summary(result: dict, symbol: str) -> None:
    l1 = result["L1"]
    print(f"\n  --- {symbol} L1 ---")
    print(f"  Strokes:   {len(l1['strokes']):>6d}")
    print(f"  Segments:  {len(l1['segments']):>6d}")
    print(f"  Zhongshus: {len(l1['zhongshus']):>6d}")
    print(f"  Moves:     {len(l1['moves']):>6d}")
    print(f"  BSPs:      {len(l1['bsp']):>6d}")

    for rs_key, rs_val in result["recursive"].items():
        print(f"\n  --- {symbol} {rs_key} ---")
        print(f"  Zhongshus: {len(rs_val['zhongshus']):>4d}")
        print(f"  Moves:     {len(rs_val['moves']):>4d}")

    levels = list(result["recursive"].keys())
    max_lv = levels[-1] if levels else "L1"
    print(f"\n  Recursive depth: L1 -> {max_lv}" if levels else "\n  No recursive levels")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbol", choices=list(DATASETS.keys()) + ["ALL"], default="ALL")
    args = parser.parse_args()

    symbols = list(DATASETS.keys()) if args.symbol == "ALL" else [args.symbol]

    results = {}
    for sym in symbols:
        path, fmt = DATASETS[sym]
        if not path.exists():
            print(f"SKIP {sym}: {path} not found")
            continue
        bars = load_bars(path, fmt)
        results[sym] = run_recursive(sym, bars)

        out_path = OUT_DIR / f"{sym.lower()}_recursive_result.json"
        with open(out_path, "w") as f:
            json.dump(results[sym], f, separators=(",", ":"), default=str)
        print(f"  Saved: {out_path}")

    return results


if __name__ == "__main__":
    main()
