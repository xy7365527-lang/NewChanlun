"""秒级a0结构密度测试：CL 1个月 ohlcv-1s vs 同源聚合 1min。

判据（任务预注册）：秒级数据上 1 分钟窗口内的平均笔数 <3 → 线段级中枢
无法在 1 分钟内形成 → 秒级 a0 无递归增益。
"""
import json
import time
from pathlib import Path

import newchan_rust as R

DATA = Path(__file__).resolve().parent / "data_cache" / "cl_1s_databento_1mo.json"
MAX_LEVELS = 6


def run_engine(opens, highs, lows, closes, tag):
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    t0 = time.time()
    for i in range(len(closes)):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
    dt = time.time() - t0
    n = len(closes)
    strokes = orch.current_strokes()
    segs = orch.current_segments()
    moves = orch.current_moves()
    recursive = orch.current_recursive()
    print(f"\n== {tag}: {n} bars, {dt:.1f}s, {n/max(dt,1e-9)/1000:.1f}K bar/s ==")
    print(f"  strokes={len(strokes)} segments={len(segs)} L1_moves={len(moves)}")
    for (lid, zhs, mvs) in recursive:
        print(f"  recursive lid={lid}: zhongshus={len(zhs)} moves={len(mvs)}")
    return {
        "n_bars": n, "secs": dt, "n_strokes": len(strokes),
        "n_segments": len(segs), "n_l1_moves": len(moves),
        "recursive": [(lid, len(zhs), len(mvs)) for (lid, zhs, mvs) in recursive],
    }


raw = json.loads(DATA.read_text())
ts = raw["timestamps_ns"]
o1s, h1s, l1s, c1s = raw["opens"], raw["highs"], raw["lows"], raw["closes"]

# 同源聚合到 1min
agg = {}
order = []
for i, t in enumerate(ts):
    key = t // 60_000_000_000
    if key not in agg:
        agg[key] = [o1s[i], h1s[i], l1s[i], c1s[i]]
        order.append(key)
    else:
        b = agg[key]
        b[1] = max(b[1], h1s[i])
        b[2] = min(b[2], l1s[i])
        b[3] = c1s[i]
o1m = [agg[k][0] for k in order]
h1m = [agg[k][1] for k in order]
l1m = [agg[k][2] for k in order]
c1m = [agg[k][3] for k in order]
n_minutes = len(order)
print(f"1s bars={len(c1s)}  聚合后 1min bars={n_minutes}  "
      f"平均每分钟有成交秒数={len(c1s)/n_minutes:.1f}")

r1s = run_engine(o1s, h1s, l1s, c1s, "ohlcv-1s a0")
r1m = run_engine(o1m, h1m, l1m, c1m, "ohlcv-1m a0 (同源聚合)")

spm = r1s["n_strokes"] / n_minutes
print(f"\n== 密度判据 ==")
print(f"  秒级笔总数 {r1s['n_strokes']} / {n_minutes} 分钟 = "
      f"平均 {spm:.2f} 笔/分钟  (判据阈值 3)")
print(f"  笔数膨胀 {r1s['n_strokes']}/{r1m['n_strokes']} = "
      f"{r1s['n_strokes']/max(r1m['n_strokes'],1):.1f}×, "
      f"线段膨胀 {r1s['n_segments']}/{r1m['n_segments']} = "
      f"{r1s['n_segments']/max(r1m['n_segments'],1):.1f}×")

out = Path(__file__).resolve().parent / "data_cache" / "_sec_density_result.json"
out.write_text(json.dumps({"1s": r1s, "1m": r1m, "n_minutes": n_minutes,
                           "strokes_per_minute": spm}, indent=1))
print(f"结果写入 {out.name}")
