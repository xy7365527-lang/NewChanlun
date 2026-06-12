"""临时诊断（subagent）：segment 级短差 卖出/买回 配对的结构分类。

复刻 fugue_version_i 的 segment-ladder 信号 + FSM 配对逻辑（always-long 简化），
但保留每个 confirmed BSP 的完整结构上下文（kind / center_seg_start / center_zd/zg /
bp.price / bp.bar_idx），对每对 (sell, buy) 分类：
  - 同中枢配对（buy.center_seg_start == sell.center_seg_start）
  - 跨中枢配对（buy 锚定更晚/更高的中枢）
  - kind 组合（type1/2/3 × type1/2/3）
并统计 diff>0/<0 在各类中的分布 + 被浪费的买点（无开放短差时 confirmed 的买点）。
"""
import json, sys, time
from pathlib import Path
from datetime import datetime, timedelta

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar
from per_level_bsp import confirmed_bsp_bi_zhongshu

N_SLICE = int(sys.argv[1]) if len(sys.argv) > 1 else 100_000
OUT = ROOT / "analysis" / "_pair_diag_out.json"

raw = json.loads((ROOT / "analysis/data_cache/oklo_1m_databento.json").read_text())
bars = raw["bars"][:N_SLICE]
n = len(bars)

base = datetime(2020, 1, 1)
t0 = time.time()
orch = RecursiveOrchestrator(stream_id="pairdiag2", max_levels=8)
seen = set(); last_stroke_n = 0
open_diff = None; pairs = []; wasted_buys = 0; ignored_sells = 0
new_sell_ctx = None
for i in range(n):
    b = bars[i]
    c = float(b["close"])
    snap = orch.process_bar(Bar(ts=base + timedelta(minutes=i), open=float(b["open"]),
                                high=float(b["high"]), low=float(b["low"]), close=c,
                                volume=0.0))
    strokes = snap.bi_snapshot.strokes
    new_buys = []; new_sells = []
    if len(strokes) > last_stroke_n:
        last_stroke_n = len(strokes)
        bsps = confirmed_bsp_bi_zhongshu(strokes)
        for bp in bsps:
            if not bp.confirmed:
                continue
            key = (bp.kind, bp.side, bp.seg_idx)
            if key in seen:
                continue
            seen.add(key)
            ctx = {"bar": i, "close": c, "kind": bp.kind, "side": bp.side,
                   "seg_idx": bp.seg_idx, "center": bp.center_seg_start,
                   "zd": bp.center_zd, "zg": bp.center_zg,
                   "bp_price": bp.price, "bp_bar_idx": bp.bar_idx}
            (new_buys if bp.side == "buy" else new_sells).append(ctx)
    # FSM 配对（与 run_version_i 主循环逐字同序：sell 优先 elif buy）
    sell_any = bool(new_sells); buy_any = bool(new_buys)
    if sell_any and open_diff is None:
        open_diff = new_sells[0] if len(new_sells) == 1 else new_sells[-1]
        open_diff = dict(open_diff); open_diff["n_same_bar_sells"] = len(new_sells)
        if buy_any:
            wasted_buys += len(new_buys)  # 同 bar 买点被 elif 吞掉
    elif buy_any and open_diff is not None:
        bctx = new_buys[0] if len(new_buys) == 1 else new_buys[-1]
        pairs.append({
            "sell": open_diff, "buy": dict(bctx),
            "diff": open_diff["close"] - c,
            "gap_bars": i - open_diff["bar"],
            "same_center": (bctx["center"] == open_diff["center"]),
            "buy_center_newer": (bctx["center"] is not None and open_diff["center"] is not None
                                  and bctx["center"] > open_diff["center"]),
        })
        open_diff = None
        if len(new_buys) > 1:
            wasted_buys += len(new_buys) - 1
        if sell_any:
            ignored_sells += len(new_sells)
    else:
        if buy_any:
            wasted_buys += len(new_buys)
        if sell_any and open_diff is not None:
            ignored_sells += len(new_sells)
    if i % 20000 == 0:
        print(f"  bar {i}/{n}  strokes={last_stroke_n} pairs={len(pairs)} "
              f"elapsed={time.time()-t0:.0f}s", flush=True)

# 统计
def stats(ps):
    if not ps:
        return {"n": 0}
    n_fly = sum(1 for p in ps if p["diff"] < 0)
    n_win = sum(1 for p in ps if p["diff"] > 0)
    return {"n": len(ps), "fly_rate": round(n_fly/len(ps)*100, 1),
            "win_rate": round(n_win/len(ps)*100, 1),
            "avg_diff": round(sum(p["diff"] for p in ps)/len(ps), 4),
            "avg_gap_bars": round(sum(p["gap_bars"] for p in ps)/len(ps), 1),
            "med_gap_bars": sorted(p["gap_bars"] for p in ps)[len(ps)//2]}

same = [p for p in pairs if p["same_center"]]
cross = [p for p in pairs if not p["same_center"]]
cross_newer = [p for p in pairs if p["buy_center_newer"]]
by_kind = {}
for p in pairs:
    k = f"{p['sell']['kind']}->{p['buy']['kind']}"
    by_kind.setdefault(k, []).append(p)

out = {
    "n_bars": n, "n_pairs": len(pairs),
    "wasted_buys": wasted_buys, "ignored_sells": ignored_sells,
    "all": stats(pairs),
    "same_center": stats(same),
    "cross_center": stats(cross),
    "cross_center_buy_newer": stats(cross_newer),
    "by_kind": {k: stats(v) for k, v in sorted(by_kind.items())},
    "sell_kind_dist": {},
    "buy_kind_dist": {},
    "worst8": sorted(pairs, key=lambda p: p["diff"])[:8],
}
for p in pairs:
    out["sell_kind_dist"][p["sell"]["kind"]] = out["sell_kind_dist"].get(p["sell"]["kind"], 0) + 1
    out["buy_kind_dist"][p["buy"]["kind"]] = out["buy_kind_dist"].get(p["buy"]["kind"], 0) + 1
OUT.write_text(json.dumps(out, indent=1, default=str))
print("done", time.time()-t0, "s; pairs:", len(pairs))
