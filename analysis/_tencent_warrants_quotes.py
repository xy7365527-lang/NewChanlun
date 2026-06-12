"""腾讯 700.HK call 涡轮：批量报价 + greeks → 杠杆/有效杠杆/情景凸性排名。

只查询展示，不下任何单。
gearing = spot * multiplier / mid          （multiplier = 每份涡轮兑的股数）
eff_gearing = delta * gearing
凸性情景 = 用市场 IV 做 BS 即时重估，正股 +10%/+20% 时涡轮价格倍数
"""
import json
import math
from datetime import date

from ib_insync import IB, Contract, Stock

BATCH = 60
WAIT_S = 8.0
R = 0.04  # HKD 无风险利率近似
Q = 0.005  # 股息率近似


def norm_cdf(x: float) -> float:
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def bs_call(s: float, k: float, t: float, vol: float, r: float = R, q: float = Q) -> float:
    if t <= 0 or vol <= 0:
        return max(s - k, 0.0)
    d1 = (math.log(s / k) + (r - q + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    d2 = d1 - vol * math.sqrt(t)
    return s * math.exp(-q * t) * norm_cdf(d1) - k * math.exp(-r * t) * norm_cdf(d2)


def implied_vol(price: float, s: float, k: float, t: float) -> float | None:
    """二分法求 IV（per-share 口径）。"""
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


def bs_delta_gamma(s: float, k: float, t: float, vol: float) -> tuple[float, float]:
    d1 = (math.log(s / k) + (R - Q + 0.5 * vol * vol) * t) / (vol * math.sqrt(t))
    delta = math.exp(-Q * t) * norm_cdf(d1)
    gamma = math.exp(-Q * t) * math.exp(-0.5 * d1 * d1) / (s * vol * math.sqrt(t) * math.sqrt(2 * math.pi))
    return delta, gamma


def good(v) -> bool:
    return v is not None and not math.isnan(v) and v > 0


with open("analysis/_tencent_warrants_contracts.json") as f:
    rows = json.load(f)

ib = IB()
ib.connect("127.0.0.1", 7496, clientId=74, timeout=15)
ib.reqMarketDataType(1)

# 正股现价
stk = Stock("700", "SEHK", "HKD")
ib.qualifyContracts(stk)
t_stk = ib.reqMktData(stk, "", False, False)
ib.sleep(3)
spot = next((v for v in (t_stk.last, (t_stk.bid + t_stk.ask) / 2 if good(t_stk.bid) and good(t_stk.ask) else None, t_stk.close) if good(v)), None)
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
        bid = tk.bid if good(tk.bid) else None
        ask = tk.ask if good(tk.ask) else None
        last = tk.last if good(tk.last) else None
        close = tk.close if good(tk.close) else None
        mg = tk.modelGreeks
        rec = dict(r)
        rec.update(
            bid=bid,
            ask=ask,
            last=last,
            close=close,
            iv_ib=(mg.impliedVol if mg and good(mg.impliedVol) else None),
            delta_ib=(mg.delta if mg and mg.delta is not None and not math.isnan(mg.delta) else None),
            gamma_ib=(mg.gamma if mg and mg.gamma is not None and not math.isnan(mg.gamma) else None),
        )
        out.append(rec)
    for r, tk in tickers:
        ib.cancelMktData(tk.contract)
    print(f"batch {i // BATCH + 1}: {len(out)}/{len(rows)}", flush=True)

ib.disconnect()

# ── 计算杠杆与凸性 ──
results = []
for r in out:
    m = float(r["multiplier"])  # 每份涡轮兑股数
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
    per_share_price = mid / m  # 折算到每股权利金
    gearing = spot * m / mid

    iv = r["iv_ib"]
    delta, gamma = r["delta_ib"], r["gamma_ib"]
    iv_src = "IB"
    if iv is None:
        iv = implied_vol(per_share_price, spot, k, t)
        iv_src = "BS"
    if iv is not None and (delta is None or gamma is None):
        delta, gamma = bs_delta_gamma(spot, k, t, iv)

    eff = delta * gearing if delta is not None else None
    x10 = x20 = None
    if iv is not None:
        p10 = bs_call(spot * 1.10, k, t, iv)
        p20 = bs_call(spot * 1.20, k, t, iv)
        x10 = p10 / per_share_price
        x20 = p20 / per_share_price
    results.append(
        dict(
            code=r["localSymbol"],
            strike=k,
            expiry=e,
            dte=dte,
            ratio=round(1 / m),
            bid=r["bid"],
            ask=r["ask"],
            mid=mid,
            spread_pct=((r["ask"] - r["bid"]) / mid * 100 if r["bid"] and r["ask"] else None),
            gearing=gearing,
            delta=delta,
            gamma=gamma,
            iv=iv,
            iv_src=iv_src,
            eff_gearing=eff,
            x10=x10,
            x20=x20,
        )
    )

with open("analysis/_tencent_warrants_results.json", "w") as f:
    json.dump(dict(spot=spot, asof=str(today), results=results), f, ensure_ascii=False, indent=1)

print(f"\nspot={spot}  有报价可计算: {len(results)}/{len(out)}")


def fmt(v, nd=2, w=8):
    return f"{v:>{w}.{nd}f}" if isinstance(v, (int, float)) else " " * (w - 2) + "--"


def show(title, rows_, key):
    print(f"\n=== {title} ===")
    print(f"{'code':>6} {'K':>7} {'expiry':>9} {'dte':>4} {'ratio':>5} {'bid':>7} {'ask':>7} "
          f"{'gear':>7} {'delta':>6} {'IV':>5} {'effG':>7} {'x@+10%':>7} {'x@+20%':>7} {'sprd%':>6}")
    for x in rows_:
        print(f"{x['code']:>6} {x['strike']:>7.2f} {x['expiry']:>9} {x['dte']:>4} {x['ratio']:>5} "
              f"{fmt(x['bid'], 3, 7)} {fmt(x['ask'], 3, 7)} {fmt(x['gearing'], 1, 7)} "
              f"{fmt(x['delta'], 2, 6)} {fmt(x['iv'], 2, 5)} {fmt(x['eff_gearing'], 1, 7)} "
              f"{fmt(x['x10'], 1, 7)} {fmt(x['x20'], 1, 7)} {fmt(x['spread_pct'], 1, 6)}")


tradeable = [x for x in results if x["bid"] and x["ask"]]
by_eff = sorted((x for x in tradeable if x["eff_gearing"]), key=lambda x: -x["eff_gearing"])
show("有效杠杆 Top 15（有双边报价）", by_eff[:15], "eff_gearing")

by_x20 = sorted((x for x in tradeable if x["x20"]), key=lambda x: -x["x20"])
show("凸性 Top 15：正股+20% 即时重估倍数", by_x20[:15], "x20")

zone = sorted((x for x in tradeable if 420 <= x["strike"] <= 500), key=lambda x: (x["expiry"], x["strike"]))
show(f"行权价 420-500 区间（现价 {spot:.1f}）", zone, "strike")
