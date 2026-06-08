"""标准化双向 settle 扫描工具 —— 同一棵 OnlineMergeTree 引擎，跑买/卖两个对偶视角。

存在论位置（sublevel / superlevel 精确对偶，符号翻转）
----------------------------------------------------------------
- **买点视角（下跌分量 / sublevel）**：喂原始 ``close`` 序列。merge tree 的分量诞生于
  valley（下跌腿的底），settle = 一个**反弹**升破鞍点 peak 把这条下跌腿吸收（止跌确认）。
- **卖点视角（上涨分量 / superlevel）**：喂 ``-close``（镜像树）。镜像下跌分量 ≡ 原序列
  上涨分量；分量诞生于镜像 valley = 原序列 peak（上涨腿的顶），settle = 镜像反弹 = 原序列
  **回调**跌破鞍点 valley 把这条上涨腿吸收（涨势瓦解 / 卖点确认）。

两侧用同一函数 ``analyze_side(closes, dates, sign)`` 实现：sign=+1 是买点侧，sign=-1 是
卖点侧。真实价 = ``sign * tree_price``（sign=±1 自逆），振幅 persistence 恒正。

关键诚实点（no-patch-mentality / 090号声明膨胀禁令）
----------------------------------------------------------------
1. **因果屏障定级别**：级别一律用 settle_persistence = |settle价 − pivot价|（"未来一个
   反向结构要多大才吸收这条腿"，纯局部纯因果），**绝不用 cap−pivot**（含未来 hindsight）。
2. **全局腿 trivial alive**：全局最深分量 settle_price=None，定义上永不被反弹/回调 settle，
   它 alive 是 trivial 的，**不是**"趋势还在"的有效信号。只能被幅度超过 reversal_amplitude
   的反向结构对象否定（§17.3 规则3）。报告中隔离显示，不混入可证伪阶梯。
3. **buypoint_score 五维是形态学层**：第5维振幅衰减 ≠ MACD 力度背驰（521号），只读形态。

认识论等级（formalization-validity-domain）
----------------------------------------------------------------
- 数据：真实 yfinance 日线（L2 底材）。
- merge tree + settle 阶梯 + sublevel/superlevel 对偶：**L0**（确定性属性，零信息增量）。
- "腿 settle ↔ 缠论买/卖点"：**L1**（PH↔缠论同构，充分性需 MACD 动力学层确认，521号）。
- 单标的"当前在什么位置 / 见底见顶确认"：**L2**（真实数据可否证）。
- 跨标的鲁棒性：**L3 未做**（本工具仅单标的逐个分析）。

用法
----
    .venv/bin/python scripts/dual_settle_scan.py BZ=F OKLO 0700.HK ...
    .venv/bin/python scripts/dual_settle_scan.py BZ=F --period 2y   # 窗口敏感性
"""
from __future__ import annotations

import argparse
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import (  # noqa: E402
    settle_triggers,
    nearest_rebound_settle,
    dominant_trigger,
)
from newchan.a_buypoint_score import score_buypoint, ScorePolicy  # noqa: E402
from newchan.a_ph_zhongshu import detect_zhongshu, ZhongshuPolicy  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

OUT_DIR = ROOT / "analysis" / "dual_settle_results"


# ====================================================================
# 数据拉取
# ====================================================================


def pull(symbol: str, period: str, interval: str = "1d") -> dict:
    """yfinance 拉日线，剔除任一 OHLC 字段为 NaN 的根（结算缺口 / 停牌）。"""
    import yfinance as yf

    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise RuntimeError(f"yfinance 返回空数据: {symbol}")

    def col(name: str) -> list[float]:
        if name in df.columns or (name,) in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    raw = {
        "closes": col("Close"), "highs": col("High"),
        "lows": col("Low"), "opens": col("Open"),
        "dates": [d.strftime("%Y-%m-%d") for d in df.index],
    }
    keep = [
        i for i in range(len(raw["closes"]))
        if not any(math.isnan(raw[k][i]) for k in ("closes", "highs", "lows", "opens"))
    ]
    return {
        "symbol": symbol,
        "closes": [raw["closes"][i] for i in keep],
        "highs": [raw["highs"][i] for i in keep],
        "lows": [raw["lows"][i] for i in keep],
        "opens": [raw["opens"][i] for i in keep],
        "dates": [raw["dates"][i] for i in keep],
    }


# ====================================================================
# 单侧分析（sign=+1 买点/下跌分量；sign=-1 卖点/上涨分量）
# ====================================================================


def analyze_side(data: dict, sign: int, tau: float) -> dict:
    """对 sign*close 序列跑 merge tree，读 settle 阶梯 + buypoint_score + 中枢。

    真实价 = sign * tree_price（sign=±1 自逆）。
    - sign=+1：pivot=下跌腿底(valley)，barrier=反弹止跌价(rebound)，触发=价格**上行**。
    - sign=-1：pivot=上涨腿顶(peak)，barrier=回调跌破价(pullback)，触发=价格**下行**。
    """
    closes, dates = data["closes"], data["dates"]
    highs, lows = data["highs"], data["lows"]
    n = len(closes)
    last = closes[-1]
    series = [sign * p for p in closes]

    def d(i: int) -> str:
        return dates[i] if 0 <= i < n else "?"

    # --- 已 settled 的腿：update 过程中被合并的分量 ---
    tree = OnlineMergeTree()
    settled: list[dict] = []
    for p in series:
        for b in tree.update(p):
            if b.persistence <= 0:
                continue
            settled.append({
                "pivot_price": round(sign * b.birth_price, 2),   # 下:valley底 / 上:peak顶
                "pivot_date": d(b.birth_idx),
                "barrier_price": round(sign * b.death_price, 2),  # 下:反弹价 / 上:跌破价
                "barrier_date": d(b.death_idx),
                "amplitude": round(b.persistence, 2),             # 腿幅度（恒正）
                "span": b.span,
                "above_tau": b.persistence > tau,
            })
    settled.sort(key=lambda r: -r["amplitude"])  # 大级别在前

    # --- 仍 alive 的腿：活树读 settle 触发（不 finalize）---
    trigs = settle_triggers(tree)
    near = nearest_rebound_settle(trigs)   # 近端：settle_price 最小者（最先被触及）
    dom = dominant_trigger(trigs)          # 主导：最深腿（通常全局 settle_price=None）

    alive_rungs: list[dict] = []  # 可被反向结构 settle 的 alive 腿（可证伪阶梯）
    globals_: list[dict] = []     # 全局腿（settle_price=None，trivial alive）
    for t in trigs:
        pivot_real = sign * t.birth_price
        if t.settle_price is not None:
            barrier_real = sign * t.settle_price
            # 触发距离：价格需朝 barrier 方向移动多少（恒正幅度）。
            # 下跌侧 settle 在上方(反弹)，上涨侧 settle 在下方(回调)，|gap| 统一。
            trigger_gap = abs(t.settle_price - (sign * last)) if t.gap_to_settle is not None \
                else abs(barrier_real - last)
            alive_rungs.append({
                "pivot_price": round(pivot_real, 2),
                "pivot_date": d(t.birth_idx),
                "barrier_price": round(barrier_real, 2),  # 触及此价 → 该腿 settle
                "amplitude": round(t.settle_persistence, 2),
                "trigger_gap": round(trigger_gap, 2),     # 还需移动多少触发
                "pct_to_barrier": round((barrier_real / last - 1.0) * 100.0, 1),
                "is_dominant": t.is_dominant,
                "is_nearest": (near is not None and t.birth_idx == near.birth_idx),
                "above_tau": t.settle_persistence > tau,
            })
        else:
            rev = t.reversal_amplitude  # 反向结构需超过此幅度才否定该腿
            # 整条腿被对象否定的价位 = pivot 反向移动 reversal_amplitude
            negate_real = pivot_real - sign * rev
            globals_.append({
                "pivot_price": round(pivot_real, 2),
                "pivot_date": d(t.birth_idx),
                "amplitude": round(t.current_persistence, 2),
                "reversal_amplitude": round(rev, 2),
                "negate_price": round(negate_real, 2),
                "pct_to_negate": round((negate_real / last - 1.0) * 100.0, 1),
                "drawdown_from_pivot_pct": round((last / pivot_real - 1.0) * 100.0, 1),
                "is_dominant": t.is_dominant,
            })
    # 近端→深层排序（nearest first）：价格朝触发方向移动时最先触及的 barrier 在前。
    # 买点侧(sign=+1)价格上行→最低 barrier 先触及→升序；卖点侧(sign=-1)价格下行→
    # 最高 barrier 先触及→降序。两侧统一为 key = sign*barrier 的升序排列。
    alive_rungs.sort(key=lambda r: sign * r["barrier_price"])

    # --- buypoint_score 五维（镜像侧 = sellpoint 质量）---
    score_tree = OnlineMergeTree()
    for p in series:
        score_tree.update(p)
    if sign == 1:
        s_highs, s_lows, s_closes = highs, lows, closes
    else:
        # 镜像 ATR：high↔-low、low↔-high，range=high-low 不变；close=-close
        s_highs = [-x for x in lows]
        s_lows = [-x for x in highs]
        s_closes = series
    bp = score_buypoint(score_tree, highs=s_highs, lows=s_lows,
                        closes=s_closes, policy=ScorePolicy())

    # --- detect_zhongshu（finalize 后全部 bars）---
    final = score_tree.finalize()
    zs = detect_zhongshu(list(final.all_bars), bars_per_day=1.0, noise_floor=tau,
                         policy=ZhongshuPolicy(min_members=3))
    zhongshus = [{
        "zd": round(sign * z.zd if sign == 1 else sign * z.zg, 2),  # 真实下沿
        "zg": round(sign * z.zg if sign == 1 else sign * z.zd, 2),  # 真实上沿
        "n_members": z.n_members,
        "level_persistence": round(z.level_persistence, 2),
        "period_label": z.period_label,
        "lo_date": d(z.lo), "hi_date": d(z.hi),
    } for z in zs]
    # 保证 zd<=zg
    for z in zhongshus:
        if z["zd"] > z["zg"]:
            z["zd"], z["zg"] = z["zg"], z["zd"]

    return {
        "settled": settled,
        "alive_rungs": alive_rungs,
        "globals": globals_,
        "score": {
            "structure_completion": round(bp.structure_completion, 3),
            "zhongshu_count": bp.zhongshu_count,
            "nesting_depth": bp.nesting_depth,
            "alive_cleanliness": round(bp.alive_cleanliness, 3),
            "amplitude_decay": round(bp.amplitude_decay, 3),
            "composite_weighted": bp.composite_weighted,
            "composite_product": bp.composite_product,
            "explanations": list(bp.explanations),
        },
        "zhongshus": zhongshus,
        "nearest": None if near is None else {
            "barrier_price": round(sign * near.settle_price, 2),
            "pivot_price": round(sign * near.birth_price, 2),
        },
        "dominant": None if dom is None else {
            "pivot_price": round(sign * dom.birth_price, 2),
            "pivot_date": d(dom.birth_idx),
            "settle_is_global": dom.settle_price is None,
            "amplitude": round(dom.current_persistence, 2),
        },
    }


def analyze_ticker(data: dict, tau: float) -> dict:
    closes, dates = data["closes"], data["dates"]
    return {
        "meta": {
            "symbol": data["symbol"], "n": len(closes),
            "date_start": dates[0], "date_end": dates[-1],
            "last": round(closes[-1], 2), "last_date": dates[-1],
            "peak": round(max(closes), 2),
            "peak_date": dates[closes.index(max(closes))],
            "valley": round(min(closes), 2),
            "valley_date": dates[closes.index(min(closes))],
            "tau_atr": round(tau, 3),
        },
        "buy": analyze_side(data, +1, tau),    # 下跌分量 / 买点视角
        "sell": analyze_side(data, -1, tau),   # 上涨分量 / 卖点视角
    }


# ====================================================================
# Markdown 渲染
# ====================================================================


def render_md(res: dict) -> str:
    m = res["meta"]
    L = m["last"]
    buy, sell = res["buy"], res["sell"]
    lines: list[str] = []
    A = lines.append

    A(f"## {m['symbol']} 双向 Settle 阶梯")
    A("")
    A(f"> 数据：yfinance 日线 {m['date_start']} → {m['date_end']}（n={m['n']}）｜"
      f"τ(ATR14)={m['tau_atr']}")
    A(f"> 当前 **{L}** @ {m['last_date']}｜全局峰 {m['peak']} @ {m['peak_date']}｜"
      f"全局谷 {m['valley']} @ {m['valley_date']}")
    A(f"> 引擎：`OnlineMergeTree + settle_triggers + score_buypoint + detect_zhongshu`，"
      f"sublevel/superlevel 符号对偶。认识论：阶梯算法 L0，腿↔买卖点 L1，单标的定位 L2。")
    A("")

    # ---------- 买点视角 ----------
    A("### 买点视角（下跌分量）")
    A("")
    rungs = [r for r in buy["alive_rungs"] if r["above_tau"]]
    if rungs:
        A("**alive 下跌腿阶梯**（settle=反弹止跌确认价，近端→深层）：")
        A("")
        A("| pivot底(日期) | 反弹settle价 | 腿幅 | 距现价 | 还需涨 | 标记 |")
        A("|---|---|---|---|---|---|")
        for r in rungs:
            tag = " ".join(t for t, c in
                           [("★主导", r["is_dominant"]), ("近端", r["is_nearest"])] if c) or "—"
            A(f"| {r['pivot_price']} ({r['pivot_date']}) | **{r['barrier_price']}** | "
              f"{r['amplitude']} | {r['pct_to_barrier']:+.1f}% | {r['trigger_gap']} | {tag} |")
    else:
        A("**alive 下跌腿阶梯**：无 >τ 的可反弹 settle 下跌腿（下跌腿多已被吸收或仅剩全局腿）。")
    A("")
    s = buy["score"]
    A(f"**买点评分**：综合(加权) **{s['composite_weighted']}** / 100 ｜"
      f"几何 {s['composite_product']} ｜中枢 {s['zhongshu_count']} ｜区间套深度 "
      f"{s['nesting_depth']} ｜结构完成度 {s['structure_completion']} ｜"
      f"alive干净度 {s['alive_cleanliness']} ｜振幅衰减 {s['amplitude_decay']}")
    if buy["nearest"]:
        A(f"> 引擎原语 nearest_rebound_settle：**{buy['nearest']['barrier_price']}**"
          f"（吸收 pivot {buy['nearest']['pivot_price']} 的最近下跌腿，**未过滤 τ**，"
          f"可能为噪声腿；可证伪关键价以上表 >τ 阶梯为准）")
    if buy["dominant"]:
        dd = buy["dominant"]
        gl = "全局腿（settle=None，下跌未完成）" if dd["settle_is_global"] else "可反弹 settle"
        A(f"> 主导（最深）下跌腿：pivot {dd['pivot_price']} @ {dd['pivot_date']}，"
          f"幅度 {dd['amplitude']}，{gl}")
    A("")

    # ---------- 卖点视角 ----------
    A("### 卖点视角（上涨分量）")
    A("")
    srungs = [r for r in sell["alive_rungs"] if r["above_tau"]]
    if srungs:
        A("**alive 上涨腿阶梯**（settle=回调跌破确认价，近端→深层）：")
        A("")
        A("| pivot顶(日期) | 跌破settle价 | 腿幅 | 距现价 | 还需跌 | 标记 |")
        A("|---|---|---|---|---|---|")
        for r in srungs:
            tag = " ".join(t for t, c in
                           [("★主导", r["is_dominant"]), ("近端", r["is_nearest"])] if c) or "—"
            A(f"| {r['pivot_price']} ({r['pivot_date']}) | **{r['barrier_price']}** | "
              f"{r['amplitude']} | {r['pct_to_barrier']:+.1f}% | {r['trigger_gap']} | {tag} |")
    else:
        A("**alive 上涨腿阶梯**：无 >τ 的可回调 settle 上涨腿（涨腿多已被吸收或仅剩全局主峰腿）。")
    A("")
    n_settled_up = len([x for x in sell["settled"] if x["above_tau"]])
    A(f"**已 settled 上涨腿数量**：{n_settled_up} 条 >τ（共 {len(sell['settled'])} 条）"
      f"= 历史上已被回调杀死的涨势。")
    ss = sell["score"]
    A(f"**卖点评分（镜像 buypoint）**：综合(加权) **{ss['composite_weighted']}** / 100 ｜"
      f"几何 {ss['composite_product']} ｜中枢 {ss['zhongshu_count']} ｜区间套深度 "
      f"{ss['nesting_depth']} ｜结构完成度 {ss['structure_completion']}")
    if sell["globals"]:
        for g in sell["globals"]:
            A(f"> 全局主峰腿：顶 {g['pivot_price']} @ {g['pivot_date']}（涨幅 {g['amplitude']}），"
              f"现价距顶 {g['drawdown_from_pivot_pct']:+.1f}%。定义上永不被回调 settle，"
              f"只能被反向下跌 > {g['reversal_amplitude']}（即跌破 {g['negate_price']}，"
              f"{g['pct_to_negate']:+.1f}%）对象否定。")
    A("")

    # ---------- 综合定位 ----------
    A("### 综合定位")
    A("")
    # 当前位置：相对全局峰谷
    rng = m["peak"] - m["valley"]
    pos_pct = (L - m["valley"]) / rng * 100.0 if rng > 0 else 0.0
    A(f"- **当前位置**：{L}，处于全局区间 [{m['valley']}, {m['peak']}] 的 "
      f"{pos_pct:.0f}% 分位（0%=全局谷，100%=全局峰）。")
    # 近端买/卖关键价：取 >τ 阶梯首行（已 nearest-first 排序），与上表自洽。
    # 不用 nearest_rebound_settle（不过滤 τ，可能落到噪声腿，与显示阶梯不一致）。
    buy_key = rungs[0]["barrier_price"] if rungs else None
    sell_key = srungs[0]["barrier_price"] if srungs else None
    if buy_key is not None:
        A(f"- **上行关键价**：反弹升破 **{buy_key}** → 最近一段(>τ)下跌腿 settle（止跌确认）。")
    if sell_key is not None:
        A(f"- **下行关键价**：回调跌破 **{sell_key}** → 最近一段(>τ)上涨腿 settle（涨势瓦解）。")
    # 主导腿状态
    if buy["dominant"] and buy["dominant"]["settle_is_global"]:
        A(f"- **主导下跌腿未完成**：最深下跌腿 settle=None（全局腿），任何次级别反弹的"
          f"区间套收敛大概率假收敛（§17.3 规则3）——下跌大方向未被证伪。")
    if sell["globals"]:
        g = sell["globals"][0]
        A(f"- **主导上涨腿状态**：全局主峰 {g['pivot_price']}，现价已回撤 "
          f"{g['drawdown_from_pivot_pct']:+.1f}%，整条大腿对象否定价 {g['negate_price']}。")
    A("- **关键价位汇总**：" + " ｜ ".join(filter(None, [
        f"近端止跌 {buy_key}" if buy_key else None,
        f"近端瓦解 {sell_key}" if sell_key else None,
        f"全局谷 {m['valley']}", f"全局峰 {m['peak']}",
    ])))
    A("")
    A("---")
    A("")
    return "\n".join(lines)


# ====================================================================
# 主流程
# ====================================================================


def run_ticker(symbol: str, period: str, suffix: str = "") -> dict:
    data = pull(symbol, period)
    tau = atr_noise_threshold(data["highs"], data["lows"], data["closes"],
                              period=14, multiple=1.0)
    res = analyze_ticker(data, tau)
    res["meta"]["period"] = period
    md = render_md(res)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    safe = symbol.replace("=", "_").replace(".", "_").replace("/", "_")
    fname = f"{safe}{suffix}.md"
    (OUT_DIR / fname).write_text(md, encoding="utf-8")
    print(f"[{symbol}] n={res['meta']['n']} "
          f"last={res['meta']['last']} "
          f"买点={res['buy']['score']['composite_weighted']} "
          f"卖点={res['sell']['score']['composite_weighted']} "
          f"→ {OUT_DIR / fname}")
    return res


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("tickers", nargs="+")
    ap.add_argument("--period", default="max")
    ap.add_argument("--suffix", default="")
    args = ap.parse_args()

    for sym in args.tickers:
        try:
            run_ticker(sym, args.period, args.suffix)
        except Exception as e:  # 单标的失败不阻塞全量
            print(f"[{sym}] 失败: {type(e).__name__}: {e}", file=sys.stderr)


if __name__ == "__main__":
    main()
