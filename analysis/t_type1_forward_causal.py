#!/usr/bin/env python3
"""实时因果实证: type1 买卖点前向(向前看)收益 vs 随机对照.
L2 真实数据检验, 可证伪. raw_bar 是 closes 数组索引 = BSP 可见时点(因果对齐).
前向收益只用 raw_bar 之后的 close, 严禁未来信息泄漏.
"""
import csv
import json
import random
import statistics as st

random.seed(42)

CSV = "/Users/silencehan/Projects/NewChanlun/analysis/data_cache/t_bsp_dump_BTC.csv"
JSON = "/Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json"
HORIZONS = [60, 240, 1440, 10080, 43200]  # 1h 4h 1d 1w 1mo (1min bar)
H_NAMES = {60: "1h", 240: "4h", 1440: "1d", 10080: "1w", 43200: "1mo"}

closes = json.load(open(JSON))["closes"]
L = len(closes)
print(f"closes len = {L}")

# 读 type1 信号
buys, sells = [], []  # (raw_bar, level)
with open(CSV) as f:
    for row in csv.DictReader(f):
        rb = int(row["raw_bar"])
        lvl = int(row["level"])
        if row["kind"] == "type1_buy":
            buys.append((rb, lvl))
        elif row["kind"] == "type1_sell":
            sells.append((rb, lvl))
print(f"n_buy={len(buys)} n_sell={len(sells)}")

def fwd_ret(bar, N):
    """前向收益 close[bar+N]/close[bar]-1; 越界返回 None"""
    if bar + N >= L:
        return None
    base = closes[bar]
    if base == 0:
        return None
    return closes[bar + N] / base - 1.0

def stats_block(samples, N, is_buy):
    """samples: list of (bar,level); 返回 dict"""
    rets, dropped = [], 0
    for bar, _ in samples:
        r = fwd_ret(bar, N)
        if r is None:
            dropped += 1
        else:
            rets.append(r)
    if not rets:
        return {"n": 0, "dropped": dropped, "mean": None, "median": None, "win": None}
    if is_buy:
        win = sum(1 for r in rets if r > 0) / len(rets)
    else:
        win = sum(1 for r in rets if r < 0) / len(rets)
    return {"n": len(rets), "dropped": dropped,
            "mean": st.mean(rets), "median": st.median(rets), "win": win}

def pct(x):
    return f"{x*100:+.2f}%" if x is not None else "NA"

# ---------- 1. 前向收益 type1_buy / type1_sell ----------
print("\n==== 1. 前向收益 ====")
buy_res, sell_res = {}, {}
for N in HORIZONS:
    b = stats_block(buys, N, True)
    s = stats_block(sells, N, False)
    buy_res[N] = b
    sell_res[N] = s
    print(f"[BUY  N={N:>5}({H_NAMES[N]})] n={b['n']:>3} drop={b['dropped']:>3} "
          f"mean={pct(b['mean'])} median={pct(b['median'])} win={b['win']*100:.1f}%" if b['n'] else
          f"[BUY  N={N:>5}({H_NAMES[N]})] n=0 drop={b['dropped']}")
for N in HORIZONS:
    s = sell_res[N]
    print(f"[SELL N={N:>5}({H_NAMES[N]})] n={s['n']:>3} drop={s['dropped']:>3} "
          f"mean={pct(s['mean'])} median={pct(s['median'])} win(r<0)={s['win']*100:.1f}%" if s['n'] else
          f"[SELL N={N:>5}({H_NAMES[N]})] n=0 drop={s['dropped']}")

# ---------- 2. 随机对照 ----------
print("\n==== 2. 随机对照 (1000 时点, seed42) ====")
# 抽 1000 个有效起点(确保各 N 可比, 抽时点不限制 N, 越界则该 N 跳过)
rand_bars = random.sample(range(L), 1000)
rand_res = {}
for N in HORIZONS:
    rets = [fwd_ret(b, N) for b in rand_bars]
    rets = [r for r in rets if r is not None]
    rand_res[N] = {"n": len(rets),
                   "mean": st.mean(rets) if rets else None,
                   "median": st.median(rets) if rets else None,
                   "rets": rets}
    rr = rand_res[N]
    print(f"[RAND N={N:>5}({H_NAMES[N]})] n={rr['n']:>4} mean={pct(rr['mean'])} median={pct(rr['median'])}")

# buy vs random: 差值 + buy均值落在随机分布的百分位
print("\n==== buy 是否显著优于随机 ====")
buy_vs_rand = {}
for N in HORIZONS:
    bm = buy_res[N]["mean"]
    rm = rand_res[N]["mean"]
    rrets = rand_res[N]["rets"]
    if bm is None or not rrets:
        continue
    # buy均值在随机【单时点收益分布】中的百分位
    pctile = sum(1 for r in rrets if r < bm) / len(rrets) * 100
    diff = bm - rm
    buy_vs_rand[N] = {"buy_mean": bm, "rand_mean": rm, "diff": diff, "pctile": pctile}
    print(f"[N={N:>5}({H_NAMES[N]})] buy={pct(bm)} rand={pct(rm)} diff={pct(diff)} "
          f"buy均值落在随机分布第{pctile:.1f}百分位")

# ---------- 3. 按 level 分层 ----------
print("\n==== 3. 按 level 分层 ====")
by_level = {}
for lvl in [0, 1, 2, 3]:
    bl = [(b, l) for (b, l) in buys if l == lvl]
    sl = [(b, l) for (b, l) in sells if l == lvl]
    by_level[lvl] = {"n_buy": len(bl), "n_sell": len(sl), "buy": {}, "sell": {}}
    print(f"--- level {lvl}: n_buy={len(bl)} n_sell={len(sl)} ---")
    for N in HORIZONS:
        b = stats_block(bl, N, True)
        s = stats_block(sl, N, False)
        by_level[lvl]["buy"][N] = b
        by_level[lvl]["sell"][N] = s
        bstr = f"mean={pct(b['mean'])} med={pct(b['median'])} win={b['win']*100:.1f}%(n{b['n']})" if b['n'] else "n=0"
        sstr = f"mean={pct(s['mean'])} med={pct(s['median'])} win(r<0)={s['win']*100:.1f}%(n{s['n']})" if s['n'] else "n=0"
        print(f"  N={N:>5}({H_NAMES[N]}) BUY[{bstr}] SELL[{sstr}]")

# ---------- 4. MAE (type1_buy 后 1d 窗口最大不利偏移) ----------
print("\n==== 4. MAE type1_buy 后 N=1440(1d) ====")
N_MAE = 1440
maes, mae_dropped = [], 0
for bar, _ in buys:
    if bar + N_MAE >= L:
        mae_dropped += 1
        continue
    base = closes[bar]
    if base == 0:
        mae_dropped += 1
        continue
    window_min = min(closes[bar:bar + N_MAE + 1])
    maes.append(window_min / base - 1.0)
maes_sorted = sorted(maes)
def q(p):
    return maes_sorted[int(p * (len(maes_sorted) - 1))]
mae_mean = st.mean(maes)
mae_med = st.median(maes)
print(f"MAE n={len(maes)} dropped={mae_dropped} mean={pct(mae_mean)} median={pct(mae_med)}")
print(f"MAE 分布: p10={pct(q(0.10))} p25={pct(q(0.25))} p50={pct(q(0.50))} "
      f"p75={pct(q(0.75))} p90={pct(q(0.90))} min={pct(maes_sorted[0])} max={pct(maes_sorted[-1])}")
# 分桶
buckets = [(-1.0, -0.20), (-0.20, -0.10), (-0.10, -0.05), (-0.05, -0.02), (-0.02, 0.0)]
print("MAE 分桶:")
for lo, hi in buckets:
    c = sum(1 for m in maes if lo < m <= hi)
    print(f"  ({lo*100:.0f}%,{hi*100:.0f}%]: {c} ({c/len(maes)*100:.1f}%)")
mild = sum(1 for m in maes if m > -0.02)
print(f"  >-2%(几乎没回撤): {mild} ({mild/len(maes)*100:.1f}%)")

print("\n==== 5. 样本量 ====")
print(f"n_buy={len(buys)} n_sell={len(sells)}")
