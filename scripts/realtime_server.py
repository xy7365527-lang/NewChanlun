"""缠论实时 WebSocket Server。

加载历史数据 → 引擎增量处理 → WebSocket 推送：
1. 连接时发送历史快照（OHLC + 所有缠论标注）
2. 然后逐 bar 流式推送（模拟实时 or TV MCP 拉取）

用法：
  PYTHONPATH=src python scripts/realtime_server.py --symbol HK700
  PYTHONPATH=src python scripts/realtime_server.py --symbol HK700 --speed 50
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
import time
from datetime import datetime, timezone, timedelta
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.a_macd import OnlineMacdState
from newchan.a_online_persistence import OnlineMergeTree
from newchan.types import Bar

DATA_DIR = PROJECT_ROOT / "analysis" / "data_cache"

DATASETS: dict[str, tuple[Path, str]] = {
    "QQQ": (DATA_DIR / "qqq_1m_databento_full.json", "databento"),
    "OKLO": (DATA_DIR / "oklo_1m_databento_full.json", "databento"),
    "HK700": (DATA_DIR / "hk700_1m_tws.json", "tws"),
}


def _parse_ts(s: str) -> datetime:
    for fmt in ("%Y-%m-%d %H:%M:%S%z", "%Y-%m-%d %H:%M:%S",
                "%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M:%S"):
        try:
            return datetime.strptime(s, fmt)
        except ValueError:
            continue
    return datetime(2024, 1, 1, tzinfo=timezone.utc)


def load_bars(path: Path, fmt: str) -> list[Bar]:
    with open(path) as f:
        raw = json.load(f)
    if fmt == "databento":
        opens, highs, lows, closes = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
        volumes = raw.get("volumes", [None] * len(opens))
        base = datetime(2024, 1, 1, 9, 30, tzinfo=timezone.utc)
        return [Bar(ts=base + timedelta(minutes=i), open=opens[i], high=highs[i],
                    low=lows[i], close=closes[i], volume=volumes[i]) for i in range(len(opens))]
    if fmt == "tws":
        return [Bar(ts=_parse_ts(b["date"]), open=b["open"], high=b["high"],
                    low=b["low"], close=b["close"], volume=b.get("volume")) for b in raw]
    raise ValueError(f"Unknown format: {fmt}")


class RealtimeEngine:
    def __init__(self, symbol: str):
        self.symbol = symbol
        self.orch = RecursiveOrchestrator(
            stream_id=f"{symbol}_rt", max_levels=8,
            stroke_mode="new", min_strict_sep=5, enable_macd_divergence=True,
        )
        self.macd = OnlineMacdState()
        self.merge_tree = OnlineMergeTree()
        self.bar_idx = 0
        self.snap = None

    def process_bar(self, bar: Bar) -> dict:
        self.snap = self.orch.process_bar(bar)
        m, s, h = self.macd.update(bar.close, bar.ts)
        self.merge_tree.update(bar.close)
        self.bar_idx += 1

        m2r = self.snap.bi_snapshot.merged_to_raw
        def raw_idx(merged):
            return m2r[merged][0] if merged < len(m2r) else merged

        strokes = self.snap.bi_snapshot.strokes
        segments = self.snap.seg_snapshot.segments
        zhongshus = self.snap.zs_snapshot.zhongshus
        moves = self.snap.move_snapshot.moves
        bsps = self.snap.bsp_snapshot.buysellpoints

        return {
            "type": "update",
            "bar": {"t": bar.ts.isoformat(), "o": bar.open, "h": bar.high,
                    "l": bar.low, "c": bar.close},
            "macd": {"m": round(m, 6) if m else 0, "s": round(s, 6) if s else 0,
                     "h": round(h, 6) if h else 0} if m is not None else None,
            "stats": {
                "bi": len(strokes), "seg": len(segments),
                "zs": len(zhongshus), "mv": len(moves), "bsp": len(bsps),
            },
            "strokes_tail": [
                {"t0": bar.ts.isoformat(), "p0": round(st.p0, 4),
                 "t1": bar.ts.isoformat(), "p1": round(st.p1, 4), "dir": st.direction}
                for st in strokes[-3:]
            ] if strokes else [],
            "events": [
                {"type": e.__class__.__name__, "data": str(e)}
                for e in self.snap.all_events[:5]
            ],
        }

    def get_snapshot(self) -> dict:
        if not self.snap:
            return {"type": "snapshot", "ohlc": [], "strokes": [], "segments": [],
                    "zhongshus": [], "bsp": [], "macd": [], "ph": []}

        m2r = self.snap.bi_snapshot.merged_to_raw
        def raw_idx(merged):
            return m2r[merged][0] if merged < len(m2r) else merged

        strokes = self.snap.bi_snapshot.strokes
        segments = self.snap.seg_snapshot.segments
        zhongshus = self.snap.zs_snapshot.zhongshus
        bsps = self.snap.bsp_snapshot.buysellpoints

        stroke_data = [{"i0": raw_idx(s.i0), "i1": raw_idx(s.i1), "dir": s.direction,
                        "p0": round(s.p0, 4), "p1": round(s.p1, 4)} for s in strokes]

        seg_data = []
        for sg in segments:
            si0 = min(sg.s0, len(strokes) - 1)
            si1 = min(sg.s1, len(strokes) - 1)
            seg_data.append({"bi0": raw_idx(strokes[si0].i0), "bi1": raw_idx(strokes[si1].i1),
                             "dir": sg.direction, "p0": round(sg.p0, 4), "p1": round(sg.p1, 4)})

        zs_data = []
        for z in zhongshus:
            if not segments:
                continue
            si0 = min(z.seg_start, len(segments) - 1)
            si1 = min(z.seg_end, len(segments) - 1)
            bi0 = raw_idx(strokes[min(segments[si0].s0, len(strokes) - 1)].i0)
            bi1 = raw_idx(strokes[min(segments[si1].s1, len(strokes) - 1)].i1)
            zs_data.append({"bi0": bi0, "bi1": bi1, "zg": round(z.zg, 4),
                            "zd": round(z.zd, 4), "settled": z.settled})

        bsp_data = [{"idx": raw_idx(b.bar_idx) if b.bar_idx < len(m2r) else b.bar_idx,
                      "kind": b.kind, "side": b.side, "price": round(b.price, 4),
                      "confirmed": b.confirmed, "settled": b.settled} for b in bsps]

        bc = self.merge_tree.current_barcode()
        ph = [{"bp": round(b.birth_price, 4),
               "dp": round(b.death_price, 4) if b.death_price is not None else None,
               "pers": round(b.persistence, 4), "settled": b.settled}
              for b in bc.all_bars[:50]]

        return {"type": "full_snapshot", "strokes": stroke_data, "segments": seg_data,
                "zhongshus": zs_data, "bsp": bsp_data, "ph": ph}


async def run_server(symbol: str, speed: int, port: int):
    try:
        import websockets
    except ImportError:
        print("Installing websockets...")
        import subprocess
        subprocess.check_call([sys.executable, "-m", "pip", "install", "websockets", "-q"])
        import websockets

    path, fmt = DATASETS[symbol]
    print(f"Loading {symbol} from {path}...")
    bars = load_bars(path, fmt)
    print(f"  {len(bars)} bars loaded")

    history_size = min(len(bars) - 1000, int(len(bars) * 0.95))
    history_bars = bars[:history_size]
    stream_bars = bars[history_size:]

    engine = RealtimeEngine(symbol)
    print(f"Processing {history_size} historical bars...")
    t0 = time.time()
    ohlc_history = []
    macd_history = []
    for i, bar in enumerate(history_bars):
        engine.process_bar(bar)
        ohlc_history.append({"t": bar.ts.isoformat(), "o": bar.open, "h": bar.high,
                             "l": bar.low, "c": bar.close})
        m, s, h = engine.macd._macd_hist[-1] if engine.macd._macd_hist else (0, 0, 0), \
                  engine.macd._signal_hist[-1] if engine.macd._signal_hist else (0, 0, 0), \
                  engine.macd._hist_hist[-1] if engine.macd._hist_hist else (0, 0, 0)
        if (i + 1) % 50000 == 0:
            print(f"  [{i+1}/{history_size}]", flush=True)

    elapsed = time.time() - t0
    print(f"  History processed in {elapsed:.1f}s")

    snapshot = engine.get_snapshot()
    snapshot["ohlc"] = ohlc_history
    snapshot["symbol"] = symbol
    snapshot["stream_count"] = len(stream_bars)

    clients: set = set()

    async def handler(websocket):
        clients.add(websocket)
        print(f"Client connected ({len(clients)} total)")
        try:
            await websocket.send(json.dumps(snapshot, default=str))
            async for msg in websocket:
                pass
        finally:
            clients.discard(websocket)
            print(f"Client disconnected ({len(clients)} total)")

    async def stream_bars_task():
        await asyncio.sleep(2)
        print(f"\nStreaming {len(stream_bars)} bars at {speed} bars/s...")
        for i, bar in enumerate(stream_bars):
            if not clients:
                await asyncio.sleep(0.1)
                continue
            update = engine.process_bar(bar)
            msg = json.dumps(update, default=str)
            dead = set()
            for ws in clients:
                try:
                    await ws.send(msg)
                except Exception:
                    dead.add(ws)
            clients -= dead
            if (i + 1) % 100 == 0:
                print(f"  Streamed {i+1}/{len(stream_bars)}", end="\r", flush=True)
            await asyncio.sleep(1.0 / speed)
        print(f"\nStream complete")

    async with websockets.serve(handler, "localhost", port):
        print(f"\nWebSocket server running at ws://localhost:{port}")
        print(f"Open analysis/chanlun_viewer.html in browser")
        await stream_bars_task()
        await asyncio.Future()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--symbol", choices=list(DATASETS.keys()), default="HK700")
    parser.add_argument("--speed", type=int, default=20, help="Bars per second")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    asyncio.run(run_server(args.symbol, args.speed, args.port))


if __name__ == "__main__":
    main()
