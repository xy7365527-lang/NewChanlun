"""PH 导航区间套 —— 满血版：Python(ripser) 真实 persistence → TradingView 推送。

范式（pending-012 已结算架构）：PH = 缠论旁的【独立结构监视层】。本脚本是该监视层
的 TV 前端——不是 Pine 近似（降级版），而是 Python 端跑完整 ripser persistence
（H0 sublevel + H1 中枢 loop 几何），结果经 tradingview-mcp CLI 推到 TV 图表。

数据流：
  tv ohlcv (CLI/CDP 实时拉) → a_persistence_barcode (ripser 真实 PH) → tv draw shape (推图)

与纯 Pine 版的本质差异（pending-012②）：
  - H0 sublevel persistence：笔/趋势级 prominence（与 Pine 近似同，但此处精确算法）
  - H1 (Takens 嵌入 + Vietoris-Rips loop)：中枢的相空间几何持续性——Pine 无法计算
    （无 ripser），是 PH 真正超出振幅的信息。

诚实声明（239号）：H0 ≈ 振幅 ∈ ker(D) 时间盲；背驰判断 H0+MACD 双度量并存。

可重复调用：每次运行 = 拉最新 → 算 PH → 删除上轮本脚本标注(按 entity_id) → 推新标注。
不使用 draw clear（那会删用户手画图）——只精确删除 .ph_push_state.json 记录的 id。

认识论等级：PH 算法 L0；腾讯 700 实时数据读出 L2。
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import numpy as np  # noqa: E402

from newchan.a_persistence_barcode import barcode_from_prices, atr  # noqa: E402
from newchan.a_macd import compute_macd, macd_area_for_range  # noqa: E402
from tencent_recursive_persistence import (  # noqa: E402
    h0_positioned, zigzag_legs, classify_level_by_days, PBar,
)
import pandas as pd  # noqa: E402

TV_DIR = Path("/Users/silencehan/Projects/tradingview-mcp")
TV_CLI = TV_DIR / "src" / "cli" / "index.js"
STATE = Path(__file__).resolve().parent / ".ph_push_state.json"

LEVEL_CHAIN = ["月线级", "周线级", "日线级", "30分钟级", "5分钟级", "1分钟级"]


def sub_level(name: str) -> str:
    for i, lv in enumerate(LEVEL_CHAIN):
        if name.startswith(lv):
            return LEVEL_CHAIN[i + 1] if i + 1 < len(LEVEL_CHAIN) else lv + "(链尾)"
    return "?"


# ====================== TradingView CLI 桥 ======================
def tv(*args: str) -> dict:
    """调 tradingview-mcp CLI，返回解析的 JSON。"""
    r = subprocess.run(
        ["node", str(TV_CLI), *args],
        capture_output=True, text=True, cwd=str(TV_DIR), timeout=60,
    )
    out = r.stdout.strip()
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        return {"success": False, "error": f"non-json: {out[:200]} | stderr: {r.stderr[:200]}"}


def pull_ohlcv(n: int = 300) -> list[dict]:
    res = tv("ohlcv", "-n", str(n))
    if not res.get("success"):
        raise RuntimeError(f"ohlcv 失败: {res}")
    return res["bars"]


def draw_text(price: float, time: int, text: str, color: str | None = None) -> str | None:
    args = ["draw", "shape", "-t", "text", "-p", str(price), "--time", str(time), "--text", text]
    if color:
        args += ["--overrides", json.dumps({"color": color, "fontsize": 12})]
    return tv(*args).get("entity_id")


def draw_hline(price: float, time: int, color: str = "#2962FF") -> str | None:
    return tv("draw", "shape", "-t", "horizontal_line", "-p", str(price), "--time", str(time),
              "--overrides", json.dumps({"linecolor": color, "linewidth": 2})).get("entity_id")


def draw_trend(p1: float, t1: int, p2: float, t2: int, color: str = "#787B86") -> str | None:
    return tv("draw", "shape", "-t", "trend_line", "-p", str(p1), "--time", str(t1),
              "--price2", str(p2), "--time2", str(t2),
              "--overrides", json.dumps({"linecolor": color, "linewidth": 2})).get("entity_id")


def draw_rect(top: float, t1: int, bot: float, t2: int, color: str = "#FF9800") -> str | None:
    return tv("draw", "shape", "-t", "rectangle", "-p", str(top), "--time", str(t1),
              "--price2", str(bot), "--time2", str(t2),
              "--overrides", json.dumps({"color": color, "backgroundColor": color,
                                         "transparency": 85, "linewidth": 1})).get("entity_id")


def remove_prev() -> int:
    """删除上一轮本脚本创建的标注（按 entity_id，不动用户手画图）。"""
    if not STATE.exists():
        return 0
    ids = json.loads(STATE.read_text()).get("ids", [])
    removed = 0
    for eid in ids:
        if tv("draw", "remove", eid).get("success"):
            removed += 1
    return removed


def save_ids(ids: list[str]):
    STATE.write_text(json.dumps({"ids": [i for i in ids if i]}, ensure_ascii=False))


# ====================== PH 计算 ======================
def analyze(bars: list[dict], bars_per_day: float):
    closes = [float(b["close"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    times = [int(b["time"]) for b in bars]
    tau = atr(highs, lows, closes, period=14)
    h0 = h0_positioned(closes)
    actives = [b for b in h0 if b.persistence > tau]
    dom = actives[0] if actives else h0[0]
    dom_days = dom.span / bars_per_day
    dom_level = classify_level_by_days(dom_days)
    # H1（中枢 loop 几何）——满血版超出 Pine 的信号
    full = barcode_from_prices(closes, maxdim=1)
    h1 = full.by_dimension(1)
    legs = zigzag_legs(closes, threshold=max(tau, 1e-9))
    return {
        "closes": closes, "highs": highs, "lows": lows, "times": times, "tau": tau,
        "h0": h0, "dom": dom, "dom_days": dom_days, "dom_level": dom_level,
        "h1_count": len(h1), "h1_max": full.max_persistence(1), "legs": legs,
    }


def divergence(a: dict):
    """30min 确认级别次级别背驰：最近两个同向笔 H0 + MACD 双度量。"""
    closes, legs, tau = a["closes"], a["legs"], a["tau"]
    if len(legs) < 3:
        return None
    df = compute_macd(pd.DataFrame({"close": closes}))
    last = legs[-1]
    same = [lg for lg in legs[:-1] if (lg[2] > 0) == (last[2] > 0)]
    if not same:
        return None
    A, C = same[-1], last
    fa = barcode_from_prices(closes[A[0]:A[1] + 1], maxdim=0).total_persistence(0)
    fc = barcode_from_prices(closes[C[0]:C[1] + 1], maxdim=0).total_persistence(0)
    ma = macd_area_for_range(df, A[0], A[1])
    mc = macd_area_for_range(df, C[0], C[1])
    is_down = C[2] < 0
    macd_a = abs(ma["area_neg"]) if is_down else abs(ma["area_pos"])
    macd_c = abs(mc["area_neg"]) if is_down else abs(mc["area_pos"])
    return {
        "dir": "下跌" if is_down else "上涨", "fa": fa, "fc": fc,
        "macd_a": macd_a, "macd_c": macd_c,
        "h0_div": fc < fa, "macd_div": macd_c < macd_a,
    }


# ====================== 推送 ======================
def push(symbol: str = "HKEX:700"):
    print(f"# PH 导航区间套推送 → {symbol}")
    st = tv("status")
    if not st.get("success") or not st.get("cdp_connected"):
        raise RuntimeError(f"TV CDP 未连通: {st}")
    cur_sym = st.get("chart_symbol")
    cur_tf = str(st.get("chart_resolution"))
    print(f"  当前图表: {cur_sym} @ {cur_tf}  (CDP ok)")

    # 30min（操作确认级别 = 当前图）
    bars30 = pull_ohlcv(300)
    a30 = analyze(bars30, bars_per_day=11.0)
    div = divergence(a30)

    # 日线（导航定级别）——切 tf 拉取后恢复
    try:
        tv("timeframe", "1D")
        barsD = pull_ohlcv(300)
        aD = analyze(barsD, bars_per_day=1.0)
    finally:
        tv("timeframe", cur_tf)  # 恢复用户原周期

    dom_level = aD["dom_level"]
    op_level = sub_level(dom_level)
    cf_level = sub_level(op_level)
    print(f"  导航: 主导={dom_level}({aD['dom'].span}日,p={aD['dom'].persistence:.1f}) "
          f"→ 操作={op_level} → 确认={cf_level}")
    print(f"  H1中枢loop: 日线 {aD['h1_count']}个(max p={aD['h1_max']:.2f}) | "
          f"30min {a30['h1_count']}个(max p={a30['h1_max']:.2f})")

    # ---- 删除上轮标注 ----
    print(f"  删除上轮标注: {remove_prev()} 个")

    ids: list[str] = []
    t_last = a30["times"][-1]
    t_first = a30["times"][0]
    px_last = a30["closes"][-1]
    hi = max(a30["highs"])
    lo = min(a30["lows"])

    # ---- 导航信息标注（顶部）----
    nav_txt = (f"PH导航 | 主导:{dom_level} 操作:{op_level} 确认:{cf_level} | "
               f"日线H1中枢:{aD['h1_count']}")
    ids.append(draw_text(hi, t_last, nav_txt, color="#2962FF"))

    # ---- 背驰状态标注 ----
    if div:
        ca_h0 = div["fc"] / div["fa"] if div["fa"] else 0
        ca_md = div["macd_c"] / div["macd_a"] if div["macd_a"] else 0
        both = div["h0_div"] and div["macd_div"]
        flag = "★双背驰=买点确认" if both else ("⚠H0/MACD分歧" if div["h0_div"] != div["macd_div"] else "未背驰=待次级别衰竭")
        div_txt = f"30min{div['dir']}笔背驰: H0 C/A={ca_h0:.2f} MACD C/A={ca_md:.2f} → {flag}"
        ids.append(draw_text(lo, t_last, div_txt, color="#00C853" if both else "#FF6D00"))

    # ---- 关键价位 hline：30min 主导特征 birth/death ----
    dom30 = a30["dom"]
    bp = dom30.birth
    dp = dom30.death if not np.isinf(dom30.death) else hi
    ids.append(draw_hline(bp, t_first, color="#26A69A"))   # birth（低点）青
    ids.append(draw_hline(dp, t_first, color="#EF5350"))   # death（高点）红

    # ---- persistence 价格区间矩形：30min 最后一笔（C 笔）----
    legs30 = a30["legs"]
    if legs30:
        c0, c1, cd = legs30[-1]
        top = max(a30["closes"][c0], a30["closes"][c1])
        bot = min(a30["closes"][c0], a30["closes"][c1])
        ids.append(draw_rect(top, a30["times"][c0], bot, a30["times"][c1], color="#FF9800"))

    # ---- 最近 6 笔 zigzag + persistence 标注 ----
    for (i0, i1, d) in legs30[-6:]:
        p0, p1 = a30["closes"][i0], a30["closes"][i1]
        col = "#26A69A" if d > 0 else "#EF5350"
        ids.append(draw_trend(p0, a30["times"][i0], p1, a30["times"][i1], color=col))
        ids.append(draw_text(p1, a30["times"][i1], f"{abs(d):.1f}", color=col))

    save_ids(ids)
    print(f"  推送完成: {len([i for i in ids if i])} 个标注已写入 {symbol} 图表")
    print(f"  状态文件: {STATE}")


if __name__ == "__main__":
    sym = sys.argv[1] if len(sys.argv) > 1 else "HKEX:700"
    push(sym)
