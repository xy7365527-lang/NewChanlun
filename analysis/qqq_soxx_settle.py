"""QQQ / SOXX 大级别卖点分析 —— 上涨分量 settle 阶梯（卖点视角，OKLO 模板的对偶）。

存在论位置（与 OKLO settle_ladder 的对偶）
------------------------------------------
OKLO settle_ladder 是**买点视角**：原价格序列跑 sublevel-set H0 merge tree，追踪
**下跌分量**（valley 诞生），看下跌腿何时被**反弹**逐级 settle（次级别买点）。

本脚本是**卖点视角**——判断大级别卖点、牛市上涨结构是否 intact。要看「上涨分量」
必须用 sublevel/superlevel 的**精确对偶**：喂 ``-price``。

  对偶字典（mirror = [-p for p in closes]）
  ----------------------------------------------------------------
  mirror 的下跌分量            ≡  price 的上涨分量
  mirror valley (诞生)         =  -price peak（上涨腿的顶）          → 真实顶价 = -birth_price
  mirror 屏障 peak (settle价)  =  -price valley 屏障                → 真实跌破价 = -settle_price
  mirror「反弹 settle」         =  price「回调跌破鞍点 valley 而 settle」
  mirror 全局最低 valley       =  price 全局最高 peak（牛市主峰）    → 永不被回调 settle
  mirror gap_to_settle          =  当前价还需**下跌**多少才触发该上涨腿 settle（符号自洽）

  符号自洽推导：mirror_last = -last；mirror_settle = S_m；
  gap_m = S_m - mirror_last = S_m + last。真实跌破价 P = -S_m，
  当前价到跌破的下跌空间 = last - P = last + S_m = gap_m。∎

**关键判读纠正**（避免把 trivial 当信号）
-------------------------------------------
全局上涨分量（mirror settle_price=None）= 牛市主峰，**定义上永远 alive 直到序列终止**
→ 它 alive 是 trivial 的，**不是**「牛市还在」的有效信号。

真正的大级别卖点信号是：**最深的可被回调证伪的上涨腿**（mirror settle_price 最大、
真实跌破价最低的 settleable 分量）是否被回调 settle。
- 它仍 alive → 大级别上涨结构 intact，回调是买点机会。
- 它被 settle（价格跌破其真实跌破价）→ 大级别卖点确认，牛市该级别结束。

上涨 settle 阶梯（现价下方一系列跌破线，近→深 = 小级别→大级别）逐级跌破 =
牛市逐级瓦解，正是缠论「次级别卖点 → 本级别卖点 → 大级别顶分型确认」的拓扑形态。

认识论等级（formalization-validity-domain）
-------------------------------------------
- 镜像树 + settle 阶梯算法：**L0**（merge tree 确定性属性 + sublevel/superlevel 对偶，零信息增量）。
- 「上涨腿 settle ↔ 缠论卖点」：**L1**（PH↔缠论同构，需 MACD 动力学层确认充分性，521号）。
- 「QQQ/SOXX 当前牛市 intact / 卖点确认」：**L2**（单标的真实数据可否证，已用 5y 真实日线）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf  # noqa: E402

from newchan.a_online_persistence import OnlineMergeTree, MergeBar  # noqa: E402
from newchan.a_settle_trigger import settle_triggers  # noqa: E402
from newchan.a_buypoint_score import score_buypoint, ScorePolicy  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402
from newchan.a_level_detection import adaptive_gap_boundaries  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


# ====================================================================
# 数据获取（cache 复用）
# ====================================================================
def pull(symbol: str, period: str = "5y", interval: str = "1d") -> dict:
    """拉 yfinance 日线并 cache 到 {SYM}_1d_{period}.json（已存在则复用）。"""
    cache_file = CACHE / f"{symbol}_1d_{period}.json"
    if cache_file.exists():
        return json.load(open(cache_file))

    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit(f"yfinance 返回空数据: {symbol}")

    def col(name: str) -> list[float]:
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    data = {
        "symbol": symbol,
        "closes": col("Close"),
        "highs": col("High"),
        "lows": col("Low"),
        "opens": col("Open"),
        "dates": [d.strftime("%Y-%m-%d") for d in df.index],
    }
    cache_file.write_text(json.dumps(data, ensure_ascii=False, indent=2))
    return data


# ====================================================================
# 级别命名（自适应 log-gap 带，复用 OKLO 口径）
# ====================================================================
# band 0 = 最粗（主升浪/年线季线级），向细递增。5y 日线 ETF 量级。
LEVEL_NAMES = ["主升浪(年/季线级)", "大波段(月线级)", "中波段(周线级)",
               "日线级(大)", "日线级(中)", "日线级(小)", "60分级或更小"]


def level_namer(all_persistences: list[float]):
    pos = [p for p in all_persistences if p > 0]
    if not pos:
        return (lambda p: 0), 1, []
    boundaries = sorted(adaptive_gap_boundaries(pos, n_mad=3.0), reverse=True)
    edges = [float("inf"), *boundaries, 0.0]

    def band_of(p: float) -> int:
        for k in range(len(edges) - 1):
            hi, lo = edges[k], edges[k + 1]
            if (p < hi or k == 0) and p >= lo:
                return k
        return len(edges) - 2

    return band_of, len(edges) - 1, boundaries


def lvl_name(band: int) -> str:
    return LEVEL_NAMES[band] if band < len(LEVEL_NAMES) else f"L{band}(更细)"


# ====================================================================
# 上涨分量 settle 阶梯（镜像树，卖点视角 = 本任务主体）
# ====================================================================
def analyze_uptrend(closes: list[float], dates: list[str]) -> dict:
    """镜像树读出上涨分量 settle 阶梯。返回真实价格语义的结构。"""
    n = len(closes)
    last = closes[-1]
    last_date = dates[-1]
    mirror = [-p for p in closes]

    tree = OnlineMergeTree()
    for p in mirror:
        tree.update(p)
    trigs = settle_triggers(tree)

    # 还原真实价格语义
    rungs = []  # 可被回调 settle 的上涨腿（settleable）
    globals_ = []  # 全局上涨主峰（settle_price=None）
    for t in trigs:
        top_price = -t.birth_price                       # 上涨腿的顶价
        if t.settle_price is not None:
            break_price = -t.settle_price                # 跌破此价 → 该上涨腿 settle
            settle_pers = t.settle_price - t.birth_price  # mirror persistence = 上涨腿幅度
            gap_down = t.gap_to_settle                     # 当前价还需下跌多少触发 settle
            rungs.append({
                "top_price": top_price, "top_date": dates[t.birth_idx] if 0 <= t.birth_idx < n else "?",
                "break_price": break_price, "rise_amplitude": settle_pers,
                "gap_down_to_break": gap_down, "is_dominant": t.is_dominant,
                "pct_to_break": (break_price / last - 1.0) * 100.0,
            })
        else:
            rise_amp = t.reversal_amplitude  # 否定牛市主轴所需幅度 = 主升浪全幅
            globals_.append({
                "start_price": top_price,  # 镜像全局 valley = 真实全局最高峰；但 birth=最高峰顶
                "start_date": dates[t.birth_idx] if 0 <= t.birth_idx < n else "?",
                "rise_amplitude": rise_amp, "is_dominant": t.is_dominant,
            })
    # 真实跌破价从高到低（近端小级别 → 深层大级别）= mirror settle_price 升序
    rungs.sort(key=lambda r: -r["break_price"])
    return {
        "last": last, "last_date": last_date, "n": n,
        "rungs": rungs, "globals": globals_,
    }


# ====================================================================
# 下跌分量 settle 阶梯（原树，近端回调买点位置）
# ====================================================================
def analyze_downtrend(closes: list[float], dates: list[str]) -> dict:
    """原树读出近端回调的 settle 阶梯（次级别买点位置 = 反弹升破即 settle）。"""
    n = len(closes)
    last = closes[-1]
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(p)
    trigs = settle_triggers(tree)
    settleable = []
    for t in trigs:
        if t.settle_price is not None:
            settleable.append({
                "valley_price": t.birth_price,
                "valley_date": dates[t.birth_idx] if 0 <= t.birth_idx < n else "?",
                "settle_price": t.settle_price,  # 反弹升破此价 → 下跌腿 settle（次级别买点确认）
                "gap_up_to_settle": t.gap_to_settle,
                "fall_amplitude": t.settle_price - t.birth_price,
                "pct_to_settle": (t.settle_price / last - 1.0) * 100.0,
            })
    settleable.sort(key=lambda r: r["settle_price"])  # 近端（最易反弹触及）在前
    return {"last": last, "settleable": settleable, "n": n}


# ====================================================================
# 中枢（原树真实价格 finalize bars → detect_zhongshu）
# ====================================================================
def analyze_zhongshu(closes: list[float], highs: list[float],
                     lows: list[float], dates: list[str]) -> list[dict]:
    bc = OnlineMergeTree.from_prices(closes)
    bars = bc.settled_bars
    noise = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)
    zs = detect_zhongshu(bars, noise_floor=noise, policy=ZhongshuPolicy())
    n = len(closes)

    def d(i: int) -> str:
        return dates[i] if 0 <= i < n else "?"

    return [{
        "level": z.period_label, "zd": z.zd, "zg": z.zg, "width": z.width,
        "envelope_low": z.envelope_low, "envelope_high": z.envelope_high,
        "n_members": z.n_members, "span": z.span,
        "start": d(z.lo), "end": d(z.hi),
    } for z in zs]


# ====================================================================
# 双视角评分（原树=买点/下跌结构；镜像树=卖点/上涨结构完成度）
# ====================================================================
def score_both(closes: list[float], highs: list[float], lows: list[float]) -> dict:
    pol = ScorePolicy()
    down_tree = OnlineMergeTree()
    for p in closes:
        down_tree.update(p)
    buy = score_buypoint(down_tree, highs=highs, lows=lows, closes=closes, policy=pol)

    mirror = [-p for p in closes]
    up_tree = OnlineMergeTree()
    for p in mirror:
        up_tree.update(p)
    # 镜像树的 high/low 也需取负并交换（noise 阈仅看幅度，量级不变，用原 high/low 即可近似）
    sell = score_buypoint(up_tree, highs=[-l for l in lows], lows=[-h for h in highs],
                          closes=mirror, policy=pol)
    return {
        "buy_structure_completion": buy.structure_completion,
        "buy_composite": buy.composite_weighted,
        "sell_structure_completion": sell.structure_completion,
        "sell_composite": sell.composite_weighted,
        "sell_amplitude_decay": sell.amplitude_decay,  # 顶背驰形态前提（上涨腿幅度收窄）
    }


# ====================================================================
# 单标的完整分析
# ====================================================================
# 回撤百分档操盘锚点（人为标尺——见有效域标注；区别于自适应 gap 算法带）
RETRACE_BANDS = [(0.0, 3.0, "小级别(日线小/噪声)"), (3.0, 10.0, "日线级"),
                 (10.0, 20.0, "周线级"), (20.0, 35.0, "月线级(大波段)"),
                 (35.0, 1e9, "主升浪/全程")]


def aggregate_uptrend_bands(rungs: list[dict], tau: float) -> list[dict]:
    """τ 过滤噪声腿后，按回撤百分档聚合上涨 settle 阶梯（操盘卖点线）。

    每档给：腿数、最近端防线（跌破价最高=最先触发）、该档最深跌破价。
    这是绕开 adaptive_gap 在宽幅度谱失效（见模块/报告有效域标注）的稳健呈现。
    """
    sig = [r for r in rungs if r["rise_amplitude"] > tau]
    out: list[dict] = []
    for lo, hi, name in RETRACE_BANDS:
        xs = [r for r in sig if lo <= -r["pct_to_break"] < hi]
        if not xs:
            continue
        near = max(xs, key=lambda r: r["break_price"])  # 最先触发
        deep = min(xs, key=lambda r: r["break_price"])  # 该档最深
        out.append({
            "band": name, "n_legs": len(xs),
            "near_break": near["break_price"], "near_pct": near["pct_to_break"],
            "near_amp": near["rise_amplitude"],
            "deep_break": deep["break_price"], "deep_pct": deep["pct_to_break"],
        })
    return out


def analyze_symbol(symbol: str, period: str = "5y") -> dict:
    data = pull(symbol, period=period)
    closes, highs, lows, dates = (data["closes"], data["highs"],
                                  data["lows"], data["dates"])
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)
    up = analyze_uptrend(closes, dates)
    down = analyze_downtrend(closes, dates)
    zs = analyze_zhongshu(closes, highs, lows, dates)
    sc = score_both(closes, highs, lows)
    up_bands = aggregate_uptrend_bands(up["rungs"], tau)

    # 级别带（用上涨腿幅度 + 全局幅度定带）
    band_pers = [r["rise_amplitude"] for r in up["rungs"]]
    band_pers += [g["rise_amplitude"] for g in up["globals"]]
    band_of, _, boundaries = level_namer(band_pers)
    for r in up["rungs"]:
        r["level"] = lvl_name(band_of(r["rise_amplitude"]))
    for g in up["globals"]:
        g["level"] = "主升浪(全局最高峰)"

    # 大级别卖点线 = 最深 settleable 上涨腿（真实跌破价最低）
    deepest = max(up["rungs"], key=lambda r: r["rise_amplitude"]) if up["rungs"] else None
    nearest = up["rungs"][0] if up["rungs"] else None  # 真实跌破价最高 = 近端小级别

    # 全局主峰日期（牛市主升浪是否新鲜的有效信号）
    peak_idx = closes.index(max(closes))
    return {
        "symbol": symbol, "n": up["n"], "last": up["last"], "last_date": up["last_date"],
        "period_first_date": dates[0], "period_low": min(closes), "period_high": max(closes),
        "peak_date": dates[peak_idx], "tau": tau,
        "up": up, "down": down, "zhongshu": zs, "scores": sc,
        "up_bands": up_bands,
        "level_boundaries": [round(b, 2) for b in boundaries],
        "deepest_uprung": deepest, "nearest_uprung": nearest,
    }


def pct(x: float) -> str:
    return f"{x:+.1f}%"


def render_symbol(r: dict) -> list[str]:
    s = r["symbol"]
    L: list[str] = []
    L.append(f"### {s}（当前 {r['last']:.2f} @ {r['last_date']}）")
    L.append("")
    L.append(f"- 数据：{r['period_first_date']} ~ {r['last_date']}，n={r['n']} 日线，"
             f"区间 [{r['period_low']:.2f}, {r['period_high']:.2f}]，τ(ATR噪声阈)={r['tau']:.2f}")
    peak_fresh = r["peak_date"] == r["last_date"]
    dist_to_peak = (r["last"] / r["period_high"] - 1) * 100
    peak_note = ("主峰就是今天，主升浪 alive 且新鲜" if peak_fresh
                 else f"主峰已在 {r['peak_date']}，当前距主峰 {pct(dist_to_peak)}")
    L.append(f"- **全局上涨主峰**：{r['period_high']:.2f} @ {r['peak_date']}"
             f"（主升幅 {r['up']['globals'][0]['rise_amplitude']:.2f}，永不被回调 settle）"
             f" → {peak_note}")
    L.append("")
    # (a)(b) 上涨 settle 阶梯（卖点线）
    L.append("**【a/b】上涨分量 settle 阶梯（牛市结束线，现价下方逐级跌破，τ 过滤后按回撤档）**")
    L.append("")
    if r["up_bands"]:
        L.append("| 级别档(回撤%) | 腿数 | 最近端防线(最先触发) | 该档最深 |")
        L.append("|---|---|---|---|")
        for b in r["up_bands"]:
            L.append(f"| {b['band']} | {b['n_legs']} | "
                     f"{b['near_break']:.2f} ({pct(b['near_pct'])}, 幅{b['near_amp']:.1f}) | "
                     f"{b['deep_break']:.2f} ({pct(b['deep_pct'])}) |")
    else:
        L.append("（τ 过滤后无显著上涨腿——见下方判读）")
    L.append("")
    # (c) 近端回调买点阶梯
    st = r["down"]["settleable"]
    L.append(f"**【c】近端回调买点阶梯（下跌腿反弹升破即 settle = 次级别买点确认，共 {len(st)} 个）**")
    L.append("")
    if st:
        L.append("| 谷价(日期) | 反弹升破→settle | 距现价 | 需涨 | 跌幅 |")
        L.append("|---|---|---|---|---|")
        for x in st[:8]:
            L.append(f"| {x['valley_price']:.2f} ({x['valley_date']}) | {x['settle_price']:.2f} | "
                     f"{pct(x['pct_to_settle'])} | {x['gap_up_to_settle']:.2f} | {x['fall_amplitude']:.2f} |")
        if len(st) > 8:
            L.append(f"| …共 {len(st)} 个 | | | | |")
    else:
        L.append("**0 个可 settle 下跌腿** —— 当前价处历史新高区，无 alive 下跌腿待反弹确认"
                 "（所有下跌腿已被 settle）= 纯多头、无回调待修复。")
    L.append("")
    # (d) 中枢
    zs = r["zhongshu"]
    L.append(f"**【d】中枢结构（共 {len(zs)} 个，缠论 ≥2 同向中枢=趋势）**")
    L.append("")
    if zs:
        L.append("| 级别 | [zd, zg] | 宽 | 成员 | span | 时段 |")
        L.append("|---|---|---|---|---|---|")
        for z in zs:
            L.append(f"| {z['level']} | [{z['zd']:.1f}, {z['zg']:.1f}] | {z['width']:.1f} | "
                     f"{z['n_members']} | {z['span']} | {z['start']}~{z['end']} |")
    L.append("")
    sc = r["scores"]
    L.append(f"**评分**：买点结构完成度 {sc['buy_structure_completion']:.3f}（买点综合 {sc['buy_composite']:.1f}）"
             f"｜卖点结构完成度 {sc['sell_structure_completion']:.3f}（卖点综合 {sc['sell_composite']:.1f}）"
             f"｜顶背驰前提(振幅衰减) {sc['sell_amplitude_decay']:.3f}")
    L.append("")
    return L


def main() -> None:
    symbols = ["QQQ", "SOXX", "MOS"]
    results = {s: analyze_symbol(s) for s in symbols}
    (CACHE / "qqq_soxx_settle.json").write_text(
        json.dumps(results, ensure_ascii=False, indent=2, default=str))
    # 控制台摘要
    for s in symbols:
        r = results[s]
        print(f"{s}: 主峰 {r['period_high']:.2f}@{r['peak_date']} | "
              f"上涨档 {len(r['up_bands'])} | 中枢 {len(r['zhongshu'])} | "
              f"近端买点腿 {len(r['down']['settleable'])} | "
              f"卖点结构完成度 {r['scores']['sell_structure_completion']:.3f}")
    print("已存 analysis/data_cache/qqq_soxx_settle.json")
    # 渲染各标的 markdown 片段供报告引用
    body: list[str] = []
    for s in symbols:
        body += render_symbol(results[s])
    (CACHE / "_qqq_soxx_sections.md").write_text("\n".join(body))
    print("已存 analysis/data_cache/_qqq_soxx_sections.md（报告片段）")


if __name__ == "__main__":
    main()
