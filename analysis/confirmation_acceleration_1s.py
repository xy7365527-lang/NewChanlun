"""1s a0 区间套加速确认测量 — candidate→confirmed 滞后（1s vs 1min）。

任务（编排者下达）
==================
确认滞后是终极瓶颈。1s a0 的假设价值不在 1s 操作（摩擦致死，CL 秒级判决在册），
而在给区间套提供更细层级以加速确认。本脚本测量：

  Q1. 同一标的同一两周窗口下，1s 塔与 1min 塔各自的 candidate→confirmed
      滞后（bar 数 + 真实时间）。
  Q2. 外生参照系（zigzag 回调摆动）下，确认完成时回调已走百分比
      （consumed ratio）——1s 是否显著更早。

方法
====
- 数据：BTC 1s 两周（满秒 1.21M bar）；ES 1s 两周（Databento ES.v.0，
  有成交秒，时段非连续）。
- 1min 对照 = 同一 1s 序列按墙钟分钟桶聚合（同窗同标的，干净对照）。
- 两塔各跑 compute_organic_signals（Rust 引擎 delta 事件接口），收集
  bsp_events 逐 bar 事件流：(kind, side, seg_idx, confirmed, ...)。
- 配对键 (ladder, kind, side, seg_idx)：candidate(confirmed=False) 入流 bar
  → confirmed(True) 入流 bar 之差 = 滞后。
- zigzag 摆动（close 序列，多阈值）：每个下跌摆动窗口内找各塔最早
  confirmed sell 事件（ladder≥2），consumed = (peak−close@event)/(peak−trough)。
  1min 塔事件 bar 映射回该分钟桶最后一根 1s 索引。

口径声明
========
- 两塔的同号 ladder **不是**同一结构尺度（1s 塔整体下移，14 天 1s 塔高
  ≈1min 数月）。per-ladder 滞后表描述各塔内部确认机制的时间常数；
  跨塔可比的唯一参照系是外生 zigzag 摆动（Q2）。
- ES 真实时间滞后含闭市时间（与"ES 确认滞后半天"的经验口径一致）。
- 认识论等级：L2（单标的×2、单时段真实数据；两周窗口 n=1，
  方向性观测非有效域结算）。

用法：PYTHONPATH=src .venv/bin/python analysis/confirmation_acceleration_1s.py [BTC|ES]
输出：data_cache/confirm_accel_{sym}.json
"""

from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from organic_signals import compute_organic_signals  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
ZIGZAG_THRESHOLDS = (0.005, 0.01, 0.02, 0.03)   # 0.5% / 1% / 2% / 3%
FIRST_BSP_LADDER = 2   # ladder≥2 才有 BSP 事件流（笔中枢承载层起）


# ════════════════════════════════════════════════════════════
# 数据加载（两种格式统一为 o/h/l/c + ts_sec）
# ════════════════════════════════════════════════════════════

def load_btc() -> dict:
    raw = json.loads((CACHE / "btc_1s_2week.json").read_text())
    t0 = int(datetime.fromisoformat(raw["dates"][0])
             .replace(tzinfo=timezone.utc).timestamp())
    t_last = int(datetime.fromisoformat(raw["dates"][-1])
                 .replace(tzinfo=timezone.utc).timestamp())
    n = len(raw["closes"])
    if t_last - t0 != n - 1:
        raise RuntimeError(f"BTC 1s 非满秒：span={t_last-t0} n={n}")
    return {"symbol": "BTC", "opens": raw["opens"], "highs": raw["highs"],
            "lows": raw["lows"], "closes": raw["closes"],
            "ts": [t0 + i for i in range(n)]}


def load_es() -> dict:
    raw = json.loads((CACHE / "es_1s_2week.json").read_text())
    return {"symbol": "ES", "opens": raw["opens"], "highs": raw["highs"],
            "lows": raw["lows"], "closes": raw["closes"],
            "ts": [t // 1_000_000_000 for t in raw["timestamps_ns"]]}


def aggregate_1min(d: dict) -> dict:
    """1s → 1min 墙钟分钟桶聚合。last1s[j] = 该分钟桶最后一根 1s 的索引。"""
    opens, highs, lows, closes, ts = (d["opens"], d["highs"], d["lows"],
                                      d["closes"], d["ts"])
    mo, mh, ml, mc, mts, last1s = [], [], [], [], [], []
    cur = -1
    for i in range(len(ts)):
        b = ts[i] // 60
        if b != cur:
            cur = b
            mo.append(opens[i]); mh.append(highs[i])
            ml.append(lows[i]); mc.append(closes[i])
            mts.append(b * 60); last1s.append(i)
        else:
            mh[-1] = max(mh[-1], highs[i])
            ml[-1] = min(ml[-1], lows[i])
            mc[-1] = closes[i]
            last1s[-1] = i
    return {"symbol": d["symbol"] + "-1min", "opens": mo, "highs": mh,
            "lows": ml, "closes": mc, "ts": mts, "last1s": last1s}


# ════════════════════════════════════════════════════════════
# 事件收集 + candidate→confirmed 配对
# ════════════════════════════════════════════════════════════

def collect_events(d: dict) -> list[tuple]:
    """跑信号层，扁平化 BSP 事件：(bar, ladder, kind, side, seg_idx, confirmed)。"""
    t0 = time.time()
    tape = compute_organic_signals(d["opens"], d["highs"], d["lows"], d["closes"])
    rows: list[tuple] = []
    for bar, s in enumerate(tape):
        if not s.bsp_events:
            continue
        for ladder, evs in enumerate(s.bsp_events):
            for (kind, side, seg_idx, confirmed, *_rest) in evs:
                rows.append((bar, ladder, kind, side, seg_idx, bool(confirmed)))
    print(f"  [{d['symbol']}] 信号层 {time.time()-t0:.1f}s "
          f"bars={len(tape):,} events={len(rows):,}", flush=True)
    return rows


def pair_lags(rows: list[tuple], obs_ts: list[int]) -> dict:
    """配对 candidate→confirmed，按 ladder 汇总滞后分布。

    obs_ts[bar] = 该 bar 的观测墙钟秒（1s: ts+1；1min: bucket+60）。
    """
    cand: dict[tuple, int] = {}
    conf: dict[tuple, int] = {}
    for (bar, ladder, kind, side, seg_idx, confirmed) in rows:
        key = (ladder, kind, side, seg_idx)
        if confirmed:
            conf.setdefault(key, bar)
        else:
            cand.setdefault(key, bar)

    per_ladder: dict[int, dict] = {}
    for key, cbar in conf.items():
        ladder = key[0]
        st = per_ladder.setdefault(ladder, {
            "n_confirmed": 0, "n_paired": 0, "n_direct": 0,
            "lag_bars": [], "lag_secs": []})
        st["n_confirmed"] += 1
        if key in cand and cand[key] <= cbar:
            st["n_paired"] += 1
            st["lag_bars"].append(cbar - cand[key])
            st["lag_secs"].append(obs_ts[cbar] - obs_ts[cand[key]])
        else:
            st["n_direct"] += 1
    for key, bar in cand.items():
        st = per_ladder.setdefault(key[0], {
            "n_confirmed": 0, "n_paired": 0, "n_direct": 0,
            "lag_bars": [], "lag_secs": []})
        st["n_candidates"] = st.get("n_candidates", 0) + 1

    def q(xs: list, p: float) -> float:
        if not xs:
            return float("nan")
        xs = sorted(xs)
        k = min(len(xs) - 1, int(p * len(xs)))
        return float(xs[k])

    out = {}
    for ladder, st in sorted(per_ladder.items()):
        lb, ls = st["lag_bars"], st["lag_secs"]
        out[ladder] = {
            "n_candidates": st.get("n_candidates", 0),
            "n_confirmed": st["n_confirmed"],
            "n_paired": st["n_paired"],
            "n_direct_confirmed": st["n_direct"],
            "lag_bars": {"p25": q(lb, .25), "p50": q(lb, .50),
                         "p75": q(lb, .75), "p90": q(lb, .90),
                         "mean": (sum(lb) / len(lb)) if lb else float("nan")},
            "lag_secs": {"p25": q(ls, .25), "p50": q(ls, .50),
                         "p75": q(ls, .75), "p90": q(ls, .90),
                         "mean": (sum(ls) / len(ls)) if ls else float("nan")},
        }
    return out


# ════════════════════════════════════════════════════════════
# zigzag 参照系 + consumed ratio
# ════════════════════════════════════════════════════════════

def zigzag_swings(closes: list[float], threshold: float) -> list[tuple]:
    """close 序列 zigzag：返回 [(direction, start_i, end_i)]，

    direction="down" 段 = 回调（peak→trough）。标准摆动检测：
    反向移动超过 threshold 比例时翻转。
    """
    swings: list[tuple] = []
    n = len(closes)
    if n < 2:
        return swings
    ext_i, ext_p = 0, closes[0]      # 当前极值
    piv_i = 0                        # 上一枢轴
    dirn = None                      # 当前摆动方向
    for i in range(1, n):
        c = closes[i]
        if dirn in (None, "up"):
            if c > ext_p:
                ext_i, ext_p = i, c
            if c <= ext_p * (1 - threshold):
                if dirn == "up":
                    swings.append(("up", piv_i, ext_i))
                piv_i = ext_i
                dirn = "down"
                ext_i, ext_p = i, c
                continue
        if dirn == "down":
            if c < ext_p:
                ext_i, ext_p = i, c
            if c >= ext_p * (1 + threshold):
                swings.append(("down", piv_i, ext_i))
                piv_i = ext_i
                dirn = "up"
                ext_i, ext_p = i, c
    return swings


def consumed_analysis(swings: list[tuple], closes_1s: list[float],
                      events_1s_idx: list[tuple]) -> dict:
    """每个下跌摆动内找最早 sell 事件（candidate / confirmed 分开），算 consumed。

    events_1s_idx: [(idx_1s, side, confirmed)]，已映射到 1s 索引域、已按 idx 排序。
    consumed = (peak − close@event) / (peak − trough) ∈ [0,1]（>1 截断不会出现，
    因 close@event 在窗口内 ≥ trough）。
    """
    import bisect
    ev_idx = [e[0] for e in events_1s_idx]
    res = {"n_swings": 0, "conf_hit": 0, "cand_hit": 0,
           "conf_consumed": [], "cand_consumed": [],
           "depth_pcts": []}
    for (dirn, s, e) in swings:
        if dirn != "down" or e <= s:
            continue
        peak, trough = closes_1s[s], closes_1s[e]
        if peak <= trough:
            continue
        res["n_swings"] += 1
        res["depth_pcts"].append((peak - trough) / peak * 100)
        lo = bisect.bisect_right(ev_idx, s)
        first_conf = first_cand = None
        for j in range(lo, len(ev_idx)):
            idx, side, confirmed = events_1s_idx[j]
            if idx > e:
                break
            if side != "sell":
                continue
            if confirmed and first_conf is None:
                first_conf = idx
            if not confirmed and first_cand is None:
                first_cand = idx
            if first_conf is not None and first_cand is not None:
                break
        denom = peak - trough
        if first_conf is not None:
            res["conf_hit"] += 1
            res["conf_consumed"].append((peak - closes_1s[first_conf]) / denom)
        if first_cand is not None:
            res["cand_hit"] += 1
            res["cand_consumed"].append((peak - closes_1s[first_cand]) / denom)
    return res


def summarize_consumed(r: dict) -> dict:
    def q(xs, p):
        if not xs:
            return float("nan")
        xs = sorted(xs)
        return float(xs[min(len(xs) - 1, int(p * len(xs)))])
    out = {"n_swings": r["n_swings"],
           "depth_pct_median": q(r["depth_pcts"], .5)}
    for tag in ("conf", "cand"):
        xs = r[f"{tag}_consumed"]
        out[f"{tag}_coverage"] = (r[f"{tag}_hit"] / r["n_swings"]
                                  if r["n_swings"] else float("nan"))
        out[f"{tag}_consumed_p50"] = q(xs, .5)
        out[f"{tag}_consumed_mean"] = (sum(xs) / len(xs)) if xs else float("nan")
    return out


# ════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════

def run_symbol(sym: str) -> None:
    d1s = load_btc() if sym == "BTC" else load_es()
    d1m = aggregate_1min(d1s)
    n1, nm = len(d1s["closes"]), len(d1m["closes"])
    print(f"[{sym}] 1s bars={n1:,}  1min bars={nm:,}", flush=True)

    rows_1s = collect_events(d1s)
    rows_1m = collect_events(d1m)

    obs_1s = [t + 1 for t in d1s["ts"]]
    obs_1m = [t + 60 for t in d1m["ts"]]
    lags_1s = pair_lags(rows_1s, obs_1s)
    lags_1m = pair_lags(rows_1m, obs_1m)

    # zigzag consumed：事件映射到 1s 索引域（仅 ladder≥2 的 BSP 事件）
    ev1s = sorted((bar, side, conf) for (bar, lad, _k, side, _sg, conf)
                  in rows_1s if lad >= FIRST_BSP_LADDER)
    last1s = d1m["last1s"]
    ev1m = sorted((last1s[bar], side, conf) for (bar, lad, _k, side, _sg, conf)
                  in rows_1m if lad >= FIRST_BSP_LADDER)

    zz = {}
    for th in ZIGZAG_THRESHOLDS:
        t0 = time.time()
        swings = zigzag_swings(d1s["closes"], th)
        r1s = summarize_consumed(consumed_analysis(swings, d1s["closes"], ev1s))
        r1m = summarize_consumed(consumed_analysis(swings, d1s["closes"], ev1m))
        zz[f"{th*100:g}%"] = {"tower_1s": r1s, "tower_1min": r1m}
        print(f"  [zigzag {th*100:g}%] swings={r1s['n_swings']} "
              f"depth_med={r1s['depth_pct_median']:.2f}% "
              f"conf_consumed 1s={r1s['conf_consumed_p50']:.3f} "
              f"1min={r1m['conf_consumed_p50']:.3f} "
              f"cover 1s={r1s['conf_coverage']:.2f}/1min={r1m['conf_coverage']:.2f} "
              f"({time.time()-t0:.0f}s)", flush=True)

    out = {"symbol": sym, "n_bars_1s": n1, "n_bars_1min": nm,
           "window": [d1s["ts"][0], d1s["ts"][-1]],
           "lags_1s_tower": lags_1s, "lags_1min_tower": lags_1m,
           "zigzag_consumed": zz}
    path = CACHE / f"confirm_accel_{sym.lower()}.json"
    path.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"✓ 落盘 {path}", flush=True)


if __name__ == "__main__":
    for sym in (sys.argv[1:] or ["BTC"]):
        run_symbol(sym)
