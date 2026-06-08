"""布伦特原油 BZ=F 上涨分量 settle 阶梯 —— 回答"涨到 $120 的上涨结束了没"。

存在论位置（与 brn_settle_ladder.py 的精确对偶）
------------------------------------------------
`brn_settle_ladder.py` 是**买点视角**：原 close 序列跑 sublevel merge tree，追踪
**下跌分量**（valley 诞生），读"反弹升破即 settle"的止跌确认价。

本脚本是**卖点视角**——追踪**上涨分量**，回答"涨到 $118 的这波涨势结束了没"。
要看上涨分量必须用 sublevel/superlevel 的精确对偶：喂 ``-close``（qqq_soxx_settle.py
的镜像树对偶，完全同一套引擎）。

  对偶字典（mirror = [-c for c in closes]）
  ----------------------------------------------------------------
  mirror 的下跌分量              ≡  close 的上涨分量
  mirror valley 诞生(birth)      =  close peak（上涨腿的顶）   → 真实顶价 = -birth_price
  mirror death 屏障(death)       =  close valley（回调低点）   → 真实跌破价 = -death_price
  mirror persistence             =  death-birth = peak-valley  = 上涨腿幅度（恒正）
  mirror「被更高峰合并而 settle」 =  close「回调跌破鞍点 valley 而 settle」= 上涨腿被杀
  mirror 全局最低 valley         =  close 全局最高峰           → 永不被回调 settle
  mirror gap_to_settle           =  当前价还需**下跌**多少才触发该上涨腿 settle（符号自洽）

  符号自洽推导：mirror_last=-last；mirror_settle=S_m；gap_m=S_m-mirror_last=S_m+last。
  真实跌破价 P=-S_m，当前价到跌破的下跌空间 = last-P = last+S_m = gap_m。∎

关键判读纠正（避免把 trivial 当信号，090号声明膨胀禁令）
-------------------------------------------------------
全局上涨分量（mirror settle_price=None）= 牛市主峰（本例 $58→$118 那条大腿），
**定义上永远 alive 直到序列终止**——它 alive 是 trivial 的，**不是**"涨势还在"的有效
信号。它只能被幅度超过 reversal_amplitude 的反向结构否定（对象否定对象，§17.3 规则3）。

真正可证伪的卖点信号是：现价下方一系列**可被回调 settle 的上涨腿**逐级跌破——逐级
跌破 = 涨势逐级瓦解，正是缠论"次级别卖点 → 本级别卖点 → 顶分型确认"的拓扑形态。

认识论等级（formalization-validity-domain）
-------------------------------------------
- 数据：真实（yfinance BZ=F 2y 日线 cache，L2 底材）。
- 镜像树 + settle 阶梯算法：**L0**（merge tree 确定性属性 + sublevel/superlevel 对偶，零信息增量）。
- "上涨腿 settle ↔ 缠论卖点"：**L1**（PH↔缠论同构，充分性需 MACD 动力学层确认，521号）。
- "BRN 当前涨势 intact / 见顶确认"：**L2**（单标的真实数据可否证）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
SYMBOL = "BZ=F"
PERIOD = "2y"


def load() -> dict:
    """复用 BRN_1d_2y.json cache；不存在则 yfinance 拉 BZ=F 2y 并剔 NaN。"""
    cache_file = CACHE / "BRN_1d_2y.json"
    if cache_file.exists():
        return json.load(open(cache_file))

    import math
    import yfinance as yf
    df = yf.download(SYMBOL, period=PERIOD, interval="1d",
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit(f"yfinance 返回空数据: {SYMBOL}")

    def col(name: str) -> list[float]:
        s = df[name] if (name in df.columns or (name,) in df.columns) \
            else df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    raw = {"closes": col("Close"), "highs": col("High"), "lows": col("Low"),
           "opens": col("Open"),
           "dates": [d.strftime("%Y-%m-%d") for d in df.index]}
    keep = [i for i in range(len(raw["closes"]))
            if not any(math.isnan(raw[k][i]) for k in ("closes", "highs", "lows", "opens"))]
    data = {"symbol": SYMBOL,
            "closes": [raw["closes"][i] for i in keep],
            "highs": [raw["highs"][i] for i in keep],
            "lows": [raw["lows"][i] for i in keep],
            "opens": [raw["opens"][i] for i in keep],
            "dates": [raw["dates"][i] for i in keep]}
    cache_file.write_text(json.dumps(data, ensure_ascii=False, indent=2))
    return data


def analyze_upleg(data: dict, tau: float) -> dict:
    """镜像树读出上涨分量：已 settled（被回调杀死）+ 仍 alive（含全局主峰）。"""
    closes, dates = data["closes"], data["dates"]
    n = len(closes)
    last, last_date = closes[-1], dates[-1]
    mirror = [-p for p in closes]

    def d(i: int) -> str:
        return dates[i] if 0 <= i < n else "?"

    # --- 已 settled 的上涨腿：镜像树 update 过程中被合并的分量 = 被回调杀死的涨腿 ---
    tree = OnlineMergeTree()
    settled: list[dict] = []
    for j, p in enumerate(mirror):
        for b in tree.update(p):
            if b.persistence <= 0:
                continue
            settled.append({
                "top_price": round(-b.birth_price, 2),   # 镜像 birth=-顶价 → 真实顶价
                "top_date": d(b.birth_idx),
                "break_price": round(-b.death_price, 2),  # 镜像 death=-回调低 → 真实跌破价
                "break_date": d(b.death_idx),
                "rise_amplitude": round(b.persistence, 2),  # death-birth=peak-valley=涨幅
                "span": b.span,
                "above_tau": b.persistence > tau,
            })
    settled.sort(key=lambda r: -r["rise_amplitude"])  # 幅度大→小（大级别在前）

    # --- 仍 alive 的上涨腿：树终态（不 finalize）读 settle 触发 ---
    trigs = settle_triggers(tree)
    cur_pers = {t.birth_idx: t.current_persistence for t in trigs}
    dom_idx = max(cur_pers, key=lambda k: cur_pers[k]) if cur_pers else None

    rungs: list[dict] = []   # 可被回调 settle 的 alive 上涨腿（settleable）
    globals_: list[dict] = []  # 全局主峰（settle_price=None，trivial alive）
    for t in trigs:
        top_price = -t.birth_price
        if t.settle_price is not None:
            break_price = -t.settle_price            # 跌破此价 → 该上涨腿 settle
            rungs.append({
                "top_price": round(top_price, 2),
                "top_date": d(t.birth_idx),
                "break_price": round(break_price, 2),
                "rise_amplitude": round(t.settle_price - t.birth_price, 2),  # 镜像 persistence=涨幅
                "gap_down_to_break": round(t.gap_to_settle, 2),  # 还需下跌多少触发
                "pct_to_break": round((break_price / last - 1.0) * 100.0, 1),
                "is_dominant": (t.birth_idx == dom_idx),
                "above_tau": (t.settle_price - t.birth_price) > tau,
            })
        else:
            # 全局主峰：reversal_amplitude = 反向(下跌)结构需超过此幅度才否定该涨势
            rev = t.reversal_amplitude
            origin_break = round(top_price - rev, 2)  # 跌回此价 = 涨腿被完全否定（回到起点 valley）
            globals_.append({
                "top_price": round(top_price, 2),
                "top_date": d(t.birth_idx),
                "rise_amplitude": round(t.current_persistence, 2),
                "reversal_amplitude": round(rev, 2),
                "full_negate_price": origin_break,       # 跌破此价 = 整条大腿被对象否定
                "pct_to_full_negate": round((origin_break / last - 1.0) * 100.0, 1),
                "drawdown_from_top_pct": round((last / top_price - 1.0) * 100.0, 1),
                "is_dominant": (t.birth_idx == dom_idx),
            })
    rungs.sort(key=lambda r: -r["break_price"])  # 真实跌破价高→低（近端小级别→深层大级别）

    return {
        "last": round(last, 2), "last_date": last_date, "n": n,
        "tau": round(tau, 2),
        "settled_uplegs": settled,
        "alive_rungs": rungs,
        "globals": globals_,
    }


def main() -> None:
    data = load()
    closes, highs, lows, dates = (data["closes"], data["highs"],
                                  data["lows"], data["dates"])
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)
    res = analyze_upleg(data, tau)

    peak = max(closes)
    peak_date = dates[closes.index(peak)]
    valley = min(closes)
    valley_date = dates[closes.index(valley)]
    res["meta"] = {
        "symbol": SYMBOL, "period": PERIOD,
        "date_start": dates[0], "date_end": dates[-1],
        "global_peak": round(peak, 2), "global_peak_date": peak_date,
        "global_valley": round(valley, 2), "global_valley_date": valley_date,
    }
    (CACHE / "brn_upleg_settle.json").write_text(
        json.dumps(res, ensure_ascii=False, indent=2))

    # --- 控制台摘要 ---
    L = res["last"]
    print("=" * 76)
    print(f"BRN (BZ=F) 上涨分量 settle 阶梯  ({dates[0]} → {dates[-1]}, n={len(closes)})")
    print("=" * 76)
    print(f"全局主峰 {peak:.2f} @ {peak_date}  |  全局谷底 {valley:.2f} @ {valley_date}")
    print(f"当前 {L:.2f} @ {res['last_date']}  |  距主峰 {(L/peak-1)*100:+.1f}%  |  τ(ATR)={res['tau']}")
    print()
    sig_set = [s for s in res["settled_uplegs"] if s["above_tau"]]
    print(f"--- 已 settled（被回调杀死）的上涨腿：{len(res['settled_uplegs'])} 条"
          f"（{len(sig_set)} 条 >τ）---")
    for s in sig_set[:12]:
        print(f"    顶 {s['top_price']:<7}({s['top_date']}) 涨幅{s['rise_amplitude']:<6} "
              f"→ 回调跌破 {s['break_price']:<7}({s['break_date']}) 杀死  span={s['span']}")
    print()
    print(f"--- 仍 alive 可被回调 settle 的上涨腿：{len(res['alive_rungs'])} 条 ---")
    print("    （现价下方逐级跌破线，近→深 = 小级别→大级别卖点）")
    for r in res["alive_rungs"]:
        if not r["above_tau"]:
            continue
        dom = " ★主导" if r["is_dominant"] else ""
        print(f"    顶 {r['top_price']:<7}({r['top_date']}) 涨幅{r['rise_amplitude']:<6} "
              f"→ 跌破 {r['break_price']:<7}({r['pct_to_break']:+.1f}%) 确认涨势结束 "
              f"还需跌 {r['gap_down_to_break']:.2f}{dom}")
    print()
    print(f"--- 主导上涨分量（全局主峰，trivial alive）---")
    for g in res["globals"]:
        print(f"    $58→$118 大腿: 顶 {g['top_price']} @ {g['top_date']}（涨幅 {g['rise_amplitude']}）")
        print(f"    现价 {L:.2f}，已自顶回撤 {g['drawdown_from_top_pct']:+.1f}%")
        print(f"    定义上永不被回调 settle（全局 elder, settle_price=None）")
        print(f"    只能被反向下跌结构否定：反向幅度需 > {g['reversal_amplitude']}"
              f" → 即跌破 {g['full_negate_price']}（{g['pct_to_full_negate']:+.1f}%）"
              f"才算整条大腿被对象否定")
    print()
    print("已存 analysis/data_cache/brn_upleg_settle.json")


if __name__ == "__main__":
    main()
