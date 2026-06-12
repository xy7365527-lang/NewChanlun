"""腾讯 700.HK call 涡轮：2026-09~11 到期，按有效杠杆降序输出前 20。

只查询展示，不下任何单。
实际杠杆 gearing = spot * multiplier / mid   （multiplier = 每份涡轮兑的股数）
有效杠杆 eff_gearing = delta * gearing
"""
import json
import math
from datetime import date

from ib_insync import IB, Contract, Stock

BATCH = 60
WAIT_S = 8.0
R = 0.04   # HKD 无风险利率近似
Q = 0.005  # 股息率近似
EXP_LO, EXP_HI = "202609", "202611"


def norm_cdf(x: float) -> float:
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def bs_call(s: float, k: float, t: float, vol: float) -> float:
    if t <= 0 or vol <= 0:
        return max(s - k, 0.0)
    d1 = (math.log(s / k) + (R - Q + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    d2 = d1 - vol * math.sqrt(t)
    return s * math.exp(-Q * t) * norm_cdf(d1) - k * math.exp(-R * t) * norm_cdf(d2)


def implied_vol(price: float, s: float, k: float, t: float) -> float | None:
    lo, hi = 0.01, 4.0
    if not (bs_call(s, k, t, lo) <= price <= bs_call(s, k, t, hi)):
        return None
    for _ in range(80):
        mid = (lo + hi) / 2
        if bs_call(s, k, t, mid) < price:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


def bs_delta(s: float, k: float, t: float, vol: float) -> float:
    d1 = (math.log(s / k) + (R - Q + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    return math.exp(-Q * t) * norm_cdf(d1)


def good(v) -> bool:
    return v is not None and not math.isnan(v) and v > 0


with open("analysis/_tencent_warrants_contracts.json") as f:
    rows = json.load(f)
rows = [r for r in rows if EXP_LO <= r["expiry"][:6] <= EXP_HI and r["right"] == "C"]
print(f"2026-09~11 call warrants: {len(rows)}", flush=True)

ib = IB()
ib.connect("127.0.0.1", 7496, clientId=75, timeout=15)
ib.reqMarketDataType(1)  # live：frozen(2) 对港股涡轮不回 bid/ask 且 volume 为 nan

# 正股现价
stk = Stock("700", "SEHK", "HKD")
ib.qualifyContracts(stk)
t_stk = ib.reqMktData(stk, "", False, False)
ib.sleep(3)
spot = next(
    (v for v in (
        t_stk.last,
        (t_stk.bid + t_stk.ask) / 2 if good(t_stk.bid) and good(t_stk.ask) else None,
        t_stk.close,
    ) if good(v)),
    None,
)
ib.cancelMktData(stk)
print(f"spot 700.HK = {spot}", flush=True)
if not good(spot):
    raise SystemExit("无法获取正股现价")

today = date.today()
out = []
for i in range(0, len(rows), BATCH):
    chunk = rows[i : i + BATCH]
    tickers = []
    for r in chunk:
        wc = Contract(conId=r["conId"], exchange="SEHK")
        tickers.append((r, ib.reqMktData(wc, "", False, False)))
    ib.sleep(WAIT_S)
    for r, tk in tickers:
        mg = tk.modelGreeks
        rec = dict(r)
        rec.update(
            bid=tk.bid if good(tk.bid) else None,
            ask=tk.ask if good(tk.ask) else None,
            last=tk.last if good(tk.last) else None,
            close=tk.close if good(tk.close) else None,
            volume=tk.volume if good(tk.volume) else 0,
            iv_ib=(mg.impliedVol if mg and good(mg.impliedVol) else None),
            delta_ib=(mg.delta if mg and mg.delta is not None and not math.isnan(mg.delta) else None),
        )
        out.append(rec)
    for r, tk in tickers:
        ib.cancelMktData(tk.contract)
    print(f"batch {i // BATCH + 1}: {len(out)}/{len(rows)}", flush=True)

ib.disconnect()

# ── 计算杠杆 ──
results = []
for r in out:
    m = float(r["multiplier"])
    mid = None
    if r["bid"] and r["ask"]:
        mid = (r["bid"] + r["ask"]) / 2
    elif r["last"]:
        mid = r["last"]
    elif r["close"]:
        mid = r["close"]
    if not mid or mid <= 0:
        continue
    k = r["strike"]
    e = r["expiry"]
    dte = (date(int(e[:4]), int(e[4:6]), int(e[6:8])) - today).days
    if dte <= 0:
        continue
    t = dte / 365.0
    gearing = spot * m / mid

    iv, delta, iv_src = r["iv_ib"], r["delta_ib"], "IB"
    if delta is None:
        if iv is None:
            iv = implied_vol(mid / m, spot, k, t)
            iv_src = "BS"
        if iv is not None:
            delta = bs_delta(spot, k, t, iv)

    results.append(
        dict(
            code=r["localSymbol"],
            strike=k,
            expiry=e,
            dte=dte,
            bid=r["bid"],
            ask=r["ask"],
            mid=mid,
            volume=r["volume"],
            spread_pct=((r["ask"] - r["bid"]) / mid * 100 if r["bid"] and r["ask"] else None),
            gearing=gearing,
            delta=delta,
            iv_src=iv_src,
            eff_gearing=(delta * gearing if delta is not None else None),
        )
    )

with open("analysis/_tw_q4_eff_gearing_results.json", "w") as f:
    json.dump(dict(spot=spot, asof=str(today), results=results), f, ensure_ascii=False, indent=1)

tradeable = [x for x in results if x["bid"] and x["ask"] and x["eff_gearing"]]
top = sorted(tradeable, key=lambda x: -x["eff_gearing"])[:20]

print(f"\nspot={spot}  可计算: {len(results)}/{len(out)}  有双边报价+delta: {len(tradeable)}")
print(f"\n=== 有效杠杆 Top 20（到期 2026-09 ~ 2026-11）===")
print(f"{'code':>6} {'K':>7} {'expiry':>9} {'dte':>4} {'bid':>7} {'ask':>7} "
      f"{'gear':>7} {'effG':>7} {'delta':>6} {'sprd%':>6} {'vol':>10} {'src':>4}")


def fmt(v, nd=2, w=8):
    return f"{v:>{w}.{nd}f}" if isinstance(v, (int, float)) else " " * (w - 2) + "--"


for x in top:
    print(f"{x['code']:>6} {x['strike']:>7.2f} {x['expiry']:>9} {x['dte']:>4} "
          f"{fmt(x['bid'], 3, 7)} {fmt(x['ask'], 3, 7)} {fmt(x['gearing'], 1, 7)} "
          f"{fmt(x['eff_gearing'], 1, 7)} {fmt(x['delta'], 2, 6)} {fmt(x['spread_pct'], 1, 6)} "
          f"{fmt(x['volume'], 0, 10)} {x['iv_src']:>4}")
