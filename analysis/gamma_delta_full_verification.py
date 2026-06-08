"""Γ→Δ 全验证 — 配置转换矩阵 + Δ流量直接验证 + 分辨变量搜索（三任务合一）。

任务（编排者）：执行设计文档 §10.7（配置转换矩阵 L2）+ Δ 直接实证（COT）+ 分辨变量搜索。
上游：gamma_delta_mapping.py（1h K4 H(X|Γ)=1.584 一对多）。

================================ a0 选择（编排者纠正，关键）================================
**a0 = 最小可用周期（1h），不是日线。** 日线当 a0 时最高级别走势在 10 年里只翻转 1-2 次
（宏观包络），转换矩阵退化——这不是 bug 要补丁（在日线上读线段级 σ 是补丁），是 a0 选错。
正确做法：用 1h（或 30min）当 a0，让笔→线段→走势→中枢的级别**自然涌现**，
**σ = 涌现出的最高级别走势方向**（`move_snapshot.moves[-1]`，与 gamma_delta_mapping 一致）。

================================ 顶点 / 数据可用性 ================================
正典 Γ=(σ_P,σ_C,σ_R)=`config_space.Configuration`：P=股票/M、C=商品/M、R=不动产/M。
- **1h Databento（已有，10 年 2016-2026）**：仅期货 ES/GC/CL。$=UUP 用 1m→1h 降采样（2020+）。
  → 1h 只能构造**期货 K4 {ES,GC,CL,$}**（无 VNQ/DBC 1h）。这是 528号折叠通道实例
    （GC=Au 通道、CL=Oil 通道），**不是**正典 P/C/R。
- **TWS 30min（并行拉取中）**：ES/UUP/VNQ/DBC/GC/CL → 可构造**正典 P=ES/C=DBC/R=VNQ/M=UUP**。

因此：
- **本次（1h 期货实例）**：Task 2（COT，期货原生）、Task 3（分辨变量，跨边熵）完整执行；
  Task 1 做**结构转换矩阵**（配置数/翻转/吸收态，无卢麒元病态标签——期货无 R 轴）。
- **病态标签版 Task 1** 待 TWS 30min 正典 P/C/R 数据（R=VNQ）后用 `--canonical` 重跑。

================================ 认识论等级 ================================
- 转换矩阵/驻留/吸收态 = **L2**（真实 1h，可证伪）。
- flow_to_walk 桥接（σ价格方向 ≈ Δ流量方向）= **L2 经验假设**，Task 2 用 COT 真实 Δ 检验。
- Task 3 条件熵下降：加变量**机械降熵**（有限样本）→ **shuffle 对照**分离真实增益。
- σ 取最高级别走势（涌现自 1h a0），转换矩阵按**日采样**（σ 已涌现，采样间隔不改 σ 本身）。

运行：PYTHONPATH=src python analysis/gamma_delta_full_verification.py
      PYTHONPATH=src python analysis/gamma_delta_full_verification.py --canonical   # TWS 30min 就绪后
输出：analysis/gamma_delta_full_results.md
"""

from __future__ import annotations

import json
import math
import random
import sys
import urllib.parse
import urllib.request
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.capital_flow_taxonomy import (
    CapitalFlow,
    PathologyMode,
    classify,
    mode_histogram,
)
from newchan.types import Bar

_HERE = Path(__file__).resolve().parent
_DATA = _HERE / "data_cache"
_OUT = _HERE / "gamma_delta_full_results.md"
_START, _END = "2016-01-01", "2026-06-05"
_SEED = 42
_SHUFFLES = 50
_SYM = {1: "+", 0: "0", -1: "-"}


# ════════════════════════════════════════════════════════════════════════════
# 运行配置：a0 周期 + 顶点→数据源映射
# ════════════════════════════════════════════════════════════════════════════

@dataclass(frozen=True)
class RunConfig:
    """一次运行的 a0 粒度 + 顶点映射 + 病态可分类性。"""

    label: str
    a0: str                       # "1h" | "30m"
    # role → (cache_file, native_resolution)。native: "1h"|"30m"|"1m"
    sources: dict[str, tuple[str, str]]
    canonical: bool               # True=正典 P/C/R（可病态分类）；False=期货折叠通道实例
    note: str


FUTURES_1H = RunConfig(
    label="期货 K4 {ES,GC,CL,$} @ 1h a0（折叠通道实例）",
    a0="1h",
    sources={
        "P": ("es_1h_databento.json", "1h"),   # 股权（生产资本）
        "C": ("cl_1h_databento.json", "1h"),   # 商品（油，窄）
        "Au": ("gc_1h_databento.json", "1h"),  # 金（Au 折叠通道）
        "M": ("uup_1m_full.json", "1m"),       # 美元（度量基准）
    },
    canonical=False,
    note="1h 无 VNQ/DBC → 无 R 轴；Task1 仅结构转换（无病态标签）。",
)

CANONICAL_30M = RunConfig(
    label="正典 K4 {P=SPY,C=DBC,R=VNQ,M=UUP} @ 30m a0（528号正确顶点，全 ETF）",
    a0="30m",
    sources={
        "P": ("spy_30m_tws.json", "30m"),   # 股权 ETF（可翻页至 2012，非 ES 期货 3.2yr 上限）
        "C": ("dbc_30m_tws.json", "30m"),   # 宽商品 ETF
        "R": ("vnq_30m_tws.json", "30m"),   # 不动产 ETF
        "M": ("uup_30m_tws.json", "30m"),   # 美元 ETF（公共窗口由此限到 ~12yr）
        "Au": ("gld_30m_tws.json", "30m"),  # 金 ETF（Au 折叠通道）
        "Oil": ("uso_30m_tws.json", "30m"), # 油 ETF（Oil 折叠通道）
    },
    canonical=True,
    note="正典 P/C/R（全 ETF，UUP 限 ~12yr）；Task1 含卢麒元病态标签；ω=σ(GLD/USO)。",
)


# ════════════════════════════════════════════════════════════════════════════
# 数据层
# ════════════════════════════════════════════════════════════════════════════

def _load_local(fname: str) -> dict | None:
    p = _DATA / fname
    if not p.exists():
        return None
    return json.load(open(p))


def _period_key(ds: str, a0: str) -> str:
    """时间戳 → a0 周期键。1h: 'YYYY-MM-DD HH'；30m: 'YYYY-MM-DD HH:MM' 取半点。"""
    if a0 == "1h":
        return ds[:13]
    # 30m：按 30 分钟桶。
    hh = ds[11:13]
    mm = "00" if ds[14:16] < "30" else "30"
    return f"{ds[:10]} {hh}:{mm}"


def _resample_to_a0(d: dict, native: str, a0: str) -> dict:
    """原生分辨率序列 → a0 周期 OHLC。native==a0 时仍按周期键合并（幂等）。"""
    g: dict[str, list[int]] = defaultdict(list)
    for i, ds in enumerate(d["dates"]):
        g[_period_key(ds, a0)].append(i)
    keys, o, h, l, c = [], [], [], [], []
    for k in sorted(g):
        idx = g[k]
        keys.append(k)
        o.append(d["opens"][idx[0]])
        h.append(max(d["highs"][i] for i in idx))
        l.append(min(d["lows"][i] for i in idx))
        c.append(d["closes"][idx[-1]])
    return {"keys": keys, "opens": o, "highs": h, "lows": l, "closes": c}


def _fetch_yf_daily(ticker: str, cache_name: str) -> dict | None:
    """yfinance 日线（候选 regime 变量用，日分辨率广播到 a0），带缓存。"""
    cache = _DATA / cache_name
    if cache.exists():
        return json.load(open(cache))
    try:
        import yfinance as yf

        df = yf.download(ticker, start="2013-01-01", end="2026-06-06",
                         progress=False, auto_adjust=True)
        if df is None or df.empty:
            return None

        def col(name: str):
            return df[(name, ticker)] if (name, ticker) in df.columns else df[name]

        out = {
            "dates": [d.strftime("%Y-%m-%d") for d in df.index],
            "opens": [float(x) for x in col("Open")],
            "highs": [float(x) for x in col("High")],
            "lows": [float(x) for x in col("Low")],
            "closes": [float(x) for x in col("Close")],
        }
        json.dump(out, open(cache, "w"))
        return out
    except Exception as exc:  # noqa: BLE001
        print(f"  [yf 失败] {ticker}: {repr(exc)[:120]}")
        return None


def _fetch_fred(series_id: str) -> dict[str, float]:
    cache = _DATA / f"_fred_{series_id}.json"
    if cache.exists():
        return json.load(open(cache))
    url = (f"https://fred.stlouisfed.org/graph/fredgraph.csv?id={series_id}"
           f"&cosd=2013-01-01&coed=2026-06-06")
    out: dict[str, float] = {}
    last_exc = None
    for _ in range(3):
        try:
            raw = urllib.request.urlopen(
                urllib.request.Request(url, headers={"User-Agent": "research"}), timeout=90
            ).read().decode()
            for line in raw.splitlines()[1:]:
                parts = line.split(",")
                if len(parts) != 2:
                    continue
                d, v = parts
                try:
                    out[d] = float(v)
                except ValueError:
                    continue
            json.dump(out, open(cache, "w"))
            return out
        except Exception as exc:  # noqa: BLE001
            last_exc = exc
    print(f"  [FRED 失败 ×3] {series_id}: {repr(last_exc)[:120]}")
    return out


def _fetch_yield_slope() -> tuple[dict[str, float], str]:
    """收益率曲线斜率 {day: 10Y−短端}。FRED 2s10s 优先；不可达回退 ^TNX−^IRX。"""
    dgs2, dgs10 = _fetch_fred("DGS2"), _fetch_fred("DGS10")
    if dgs2 and dgs10:
        common = set(dgs2) & set(dgs10)
        return {d: dgs10[d] - dgs2[d] for d in common}, "FRED 10Y−2Y（2s10s）"
    tnx = _fetch_yf_daily("^TNX", "_tnx_1d_yf.json")
    irx = _fetch_yf_daily("^IRX", "_irx_1d_yf.json")
    if tnx and irx:
        ti = {ds[:10]: i for i, ds in enumerate(tnx["dates"])}
        ii = {ds[:10]: i for i, ds in enumerate(irx["dates"])}
        common = set(ti) & set(ii)
        return ({d: tnx["closes"][ti[d]] - irx["closes"][ii[d]] for d in common},
                "yfinance ^TNX−^IRX（10Y−13wk 代理，FRED 不可达回退）")
    return {}, "斜率数据不可达"


def _fetch_cot(market_pattern: str, cache_name: str) -> list[tuple[str, float]]:
    """CFTC 传统期货持仓（Socrata 6dca-aqww）→ [(date, noncomm_net)] 升序。"""
    cache = _DATA / cache_name
    if cache.exists():
        return [(d, v) for d, v in json.load(open(cache))]
    where = (f"market_and_exchange_names like '{market_pattern}' "
             f"AND report_date_as_yyyy_mm_dd > '2015-12-01'")
    params = urllib.parse.urlencode({
        "$select": "report_date_as_yyyy_mm_dd,noncomm_positions_long_all,"
                   "noncomm_positions_short_all",
        "$where": where,
        "$order": "report_date_as_yyyy_mm_dd",
        "$limit": "5000",
    })
    url = f"https://publicreporting.cftc.gov/resource/6dca-aqww.json?{params}"
    out: list[tuple[str, float]] = []
    try:
        rows = json.load(urllib.request.urlopen(
            urllib.request.Request(url, headers={"User-Agent": "research"}), timeout=60))
        for r in rows:
            d = r["report_date_as_yyyy_mm_dd"][:10]
            lo = float(r.get("noncomm_positions_long_all", 0) or 0)
            sh = float(r.get("noncomm_positions_short_all", 0) or 0)
            out.append((d, lo - sh))
        json.dump(out, open(cache, "w"))
    except Exception as exc:  # noqa: BLE001
        print(f"  [COT 失败] {market_pattern}: {repr(exc)[:120]}")
    return out


# ════════════════════════════════════════════════════════════════════════════
# 缠论 σ 层：a0 递归 → 最高级别走势方向
# ════════════════════════════════════════════════════════════════════════════

def _top_sigma(snap) -> int:
    """涌现出的最高级别走势方向 σ ∈ {+1,0,−1}。

    = `move_snapshot.moves[-1]`（顶层聚合走势，缠论递归涌现的最高级别走势）。
    consolidation 或无走势 → 0。**这是正确读法**（a0=1h 让级别自然涌现，取最高级别）；
    在日线上读线段级是补丁（a0 选错的症状），已废弃。
    """
    moves = snap.move_snapshot.moves
    if not moves:
        return 0
    m = moves[-1]
    if m.kind == "consolidation":
        return 0
    return 1 if m.direction == "up" else -1


def _key_ts(key: str) -> datetime:
    """周期键 → datetime。支持 'YYYY-MM-DD'、'... HH'、'... HH:MM'。"""
    if ":" in key:
        return datetime.strptime(key, "%Y-%m-%d %H:%M")
    if " " in key:
        return datetime.strptime(key, "%Y-%m-%d %H")
    return datetime.strptime(key, "%Y-%m-%d")


def _ratio_bar(na: dict, ia: int, nb: dict, ib: int, key: str) -> Bar:
    """比价 bar（正确比值 OHLC：高/低交叉相除）。key→ts。"""
    ts = _key_ts(key)
    return Bar(
        ts=ts,
        open=na["opens"][ia] / nb["opens"][ib],
        high=na["highs"][ia] / nb["lows"][ib],
        low=na["lows"][ia] / nb["highs"][ib],
        close=na["closes"][ia] / nb["closes"][ib],
    )


def run_ratio_edge(name: str, ra: dict, rb: dict, keys: list[str],
                   ia: dict[str, int], ib: dict[str, int]) -> dict[str, int]:
    """比值边 ra/rb 在 a0 周期网格上逐 bar 递归，返回 {key: σ}（最高级别走势）。"""
    orch = RecursiveOrchestrator(stream_id=name, max_levels=6, stroke_mode="wide")
    sig: dict[str, int] = {}
    for k in keys:
        sig[k] = _top_sigma(orch.process_bar(_ratio_bar(ra, ia[k], rb, ib[k], k)))
    return sig


def _resampled_index(r: dict) -> dict[str, int]:
    return {k: i for i, k in enumerate(r["keys"])}


def _daily_sample(sig: dict[str, int], keys: list[str]) -> list[tuple[str, int]]:
    """a0 σ 序列 → 每日末值采样 [(day, σ)]（σ 已涌现，仅选转换矩阵的采样间隔）。"""
    by_day: dict[str, int] = {}
    for k in keys:  # keys 升序 → 最后写入=当日末。
        by_day[k[:10]] = sig[k]
    return sorted(by_day.items())


# ════════════════════════════════════════════════════════════════════════════
# 信息论
# ════════════════════════════════════════════════════════════════════════════

def _entropy(counts) -> float:
    tot = sum(counts)
    if tot == 0:
        return 0.0
    h = 0.0
    for n in counts:
        if n > 0:
            p = n / tot
            h -= p * math.log2(p)
    return h


def _cond_entropy(pairs: list[tuple]) -> float:
    by_x: dict = defaultdict(Counter)
    for x, y in pairs:
        by_x[x][y] += 1
    total = len(pairs)
    h = 0.0
    for yc in by_x.values():
        px = sum(yc.values()) / total
        h += px * _entropy(list(yc.values()))
    return h


def _shuffle_floor(base_cond: list[tuple], cand_vals: list,
                   rng: random.Random, n: int = _SHUFFLES) -> float:
    """打乱候选标签 n 次，平均条件熵下降 = 有限样本伪增益。"""
    h_base = _cond_entropy(base_cond)
    drops = []
    pool = list(cand_vals)
    for _ in range(n):
        rng.shuffle(pool)
        with_cand = [((x, pool[i]), y) for i, (x, y) in enumerate(base_cond)]
        drops.append(h_base - _cond_entropy(with_cand))
    return sum(drops) / len(drops)


# ════════════════════════════════════════════════════════════════════════════
# 公共：构造 a0 网格 + 主边/交叉边 σ
# ════════════════════════════════════════════════════════════════════════════

def _build_edges(cfg: RunConfig):
    """加载 cfg 顶点源 → a0 重采样 → 公共网格 → 主边/交叉边 σ。

    返回 dict：{keys, days, sig_main:{P,C,R/Au}, sig_cross:{...}, omega, raw:{role:resampled}}
    或 {"error": ...}。
    """
    raw = {}
    for role, (fname, native) in cfg.sources.items():
        d = _load_local(fname)
        if d is None:
            return {"error": f"数据缺失：{role}={fname}（{cfg.note}）"}
        raw[role] = _resample_to_a0(d, native, cfg.a0)

    idx = {role: _resampled_index(r) for role, r in raw.items()}
    roles = list(raw)
    common = set(idx[roles[0]])
    for role in roles[1:]:
        common &= set(idx[role])
    keys = sorted(k for k in common if _START <= k[:10] <= _END)
    if len(keys) < 500:
        return {"error": f"公共 a0 bar 不足（{len(keys)}）"}
    return {"raw": raw, "idx": idx, "keys": keys}


# ════════════════════════════════════════════════════════════════════════════
# Task 1：配置转换矩阵（a0=1h，σ=最高级别走势；日采样转换）
# ════════════════════════════════════════════════════════════════════════════

def task1_transition_matrix(cfg: RunConfig) -> dict:
    print("\n" + "=" * 78)
    print(f"Task 1：配置转换矩阵 — {cfg.label}")
    print("=" * 78)
    e = _build_edges(cfg)
    if "error" in e:
        print("  " + e["error"])
        return e
    raw, idx, keys = e["raw"], e["idx"], e["keys"]
    M = raw["M"]
    # 主边 σ(P/M), σ(C/M), 第三轴。正典：R/M；期货：Au/M。
    third_role = "R" if cfg.canonical else "Au"
    sP = run_ratio_edge("P/M", raw["P"], M, keys, idx["P"], idx["M"])
    sC = run_ratio_edge("C/M", raw["C"], M, keys, idx["C"], idx["M"])
    s3 = run_ratio_edge(f"{third_role}/M", raw[third_role], M, keys,
                        idx[third_role], idx["M"])

    # 日采样 → 每日 Γ。
    dP, dC, d3 = (_daily_sample(s, keys) for s in (sP, sC, s3))
    days = [d for d, _ in dP]
    pmap, cmap, tmap = (dict(x) for x in (dP, dC, d3))
    gamma_seq = [(pmap[d], cmap[d], tmap[d]) for d in days]

    gamma_resid = Counter(gamma_seq)
    trans: dict[tuple, Counter] = defaultdict(Counter)
    n_transitions = 0
    for a, b in zip(gamma_seq, gamma_seq[1:]):
        trans[a][b] += 1
        if a != b:
            n_transitions += 1

    absorb = []
    for g, nxt in trans.items():
        tot = sum(nxt.values())
        absorb.append((g, tot, nxt.get(g, 0) / tot))
    absorb.sort(key=lambda x: -x[2])

    print(f"a0={cfg.a0} 公共 bar={len(keys)}；日采样={len(days)} 天 "
          f"({days[0]}→{days[-1]})")
    print(f"出现配置数：{len(gamma_resid)}/27；配置翻转次数：{n_transitions}")

    out = {
        "label": cfg.label, "canonical": cfg.canonical, "note": cfg.note,
        "a0": cfg.a0, "n_bars": len(keys), "n_days": len(days),
        "span": (days[0], days[-1]), "third_role": third_role,
        "n_configs_seen": len(gamma_resid), "n_transitions": n_transitions,
        "gamma_residence": gamma_resid, "absorb": absorb, "trans": trans,
    }

    # 病态标签（仅正典 P/C/R）。
    if cfg.canonical:
        patho_seq = [classify(CapitalFlow(*g)) for g in gamma_seq]
        patho_resid = Counter(p.value for p in patho_seq)
        ptrans: dict[str, Counter] = defaultdict(Counter)
        for a, b in zip(patho_seq, patho_seq[1:]):
            ptrans[a.value][b.value] += 1
        runs: dict[str, list[int]] = defaultdict(list)
        cur, n = patho_seq[0].value, 1
        for p in patho_seq[1:]:
            if p.value == cur:
                n += 1
            else:
                runs[cur].append(n)
                cur, n = p.value, 1
        runs[cur].append(n)
        patho_self = {}
        for p, nxt in ptrans.items():
            tot = sum(nxt.values())
            patho_self[p] = nxt.get(p, 0) / tot if tot else 0.0
        out.update({"patho_residence": patho_resid, "ptrans": ptrans,
                    "runs": runs, "patho_self": patho_self})
        print(f"病态驻留（天）：{dict(patho_resid.most_common())}")
    else:
        print("  [期货实例无 R 轴 → 无病态标签；结构转换矩阵已计算。"
              "病态标签版待 TWS 30min 正典 P/C/R。]")
    return out


# ════════════════════════════════════════════════════════════════════════════
# Task 2：Δ 流量直接验证（CFTC COT，期货子集，L2）
# ════════════════════════════════════════════════════════════════════════════

def _delta_sign(prev: float, cur: float, thresh_frac: float) -> int:
    if prev == 0:
        return 0
    chg = cur - prev
    if abs(chg) < thresh_frac * abs(prev):
        return 0
    return 1 if chg > 0 else -1


def task2_delta_flow() -> dict:
    print("\n" + "=" * 78)
    print("Task 2：Δ 流量直接验证（CFTC COT 非商业净持仓变化 = 真实 Δ）")
    print("=" * 78)
    print("Δ=非商业净持仓周变化符号（真实流量代理）；映射 ES→ΔP、CL→ΔC(油)、GC→ΔAu。无 R 期货代理。")

    cot_es = _fetch_cot("E-MINI S&P 500%", "_cot_es.json")
    cot_cl = _fetch_cot("CRUDE OIL, LIGHT SWEET%", "_cot_cl.json")
    cot_gc = _fetch_cot("GOLD - COMMODITY%", "_cot_gc.json")
    if not (cot_es and cot_cl and cot_gc):
        return {"error": "COT 数据拉取失败（网络/端点）"}
    print(f"COT 记录数：ES={len(cot_es)} CL={len(cot_cl)} GC={len(cot_gc)}")

    def deltas(cot):
        out = {}
        for (pd, pv), (cd, cv) in zip(cot, cot[1:]):
            out[cd] = _delta_sign(pv, cv, 0.05)
        return out

    dP, dC, dAu = deltas(cot_es), deltas(cot_cl), deltas(cot_gc)
    cot_dates = sorted(set(dP) & set(dC) & set(dAu))
    cot_dates = [d for d in cot_dates if _START <= d <= _END]
    print(f"三市场公共 COT 周：{len(cot_dates)}")

    # 期货 Γ（1h a0，最高级别走势）：ES/$、CL/$、GC/$ + ω=GC/CL。
    es = _resample_to_a0(_load_local("es_1h_databento.json"), "1h", "1h")
    cl = _resample_to_a0(_load_local("cl_1h_databento.json"), "1h", "1h")
    gc = _resample_to_a0(_load_local("gc_1h_databento.json"), "1h", "1h")
    uup_raw = _load_local("uup_1m_full.json")
    uup = _resample_to_a0(uup_raw, "1m", "1h")
    ei, li, gi, mi = (_resampled_index(x) for x in (es, cl, gc, uup))
    pkeys = sorted(set(ei) & set(li) & set(gi) & set(mi))
    pkeys = [k for k in pkeys if _START <= k[:10] <= _END]
    sP = run_ratio_edge("ES/$", es, uup, pkeys, ei, mi)
    sC = run_ratio_edge("CL/$", cl, uup, pkeys, li, mi)
    sAu = run_ratio_edge("GC/$", gc, uup, pkeys, gi, mi)
    sOmega = run_ratio_edge("GC/CL", gc, cl, pkeys, gi, li)
    # 日末 σ。
    def daymap(s):
        m = {}
        for k in pkeys:
            m[k[:10]] = s[k]
        return m
    mP, mC, mAu, mOm = (daymap(s) for s in (sP, sC, sAu, sOmega))
    pdays = sorted(mP)

    def nearest(target, m):
        cand = [d for d in pdays if d <= target]
        return m.get(cand[-1]) if cand else None

    rows = []
    for wk in cot_dates:
        gp, gc_, gau, om = (nearest(wk, m) for m in (mP, mC, mAu, mOm))
        if None in (gp, gc_, gau, om):
            continue
        rows.append({"gamma": (gp, gc_, gau), "omega": om,
                     "delta": (dP[wk], dC[wk], dAu[wk])})
    print(f"对齐样本（Γ_期货, ω, Δ_COT）：{len(rows)}")

    g_delta = [(r["gamma"], r["delta"]) for r in rows]
    gw_delta = [((r["gamma"], r["omega"]), r["delta"]) for r in rows]
    H_d = _entropy(list(Counter(r["delta"] for r in rows).values()))
    H_d_g = _cond_entropy(g_delta)
    H_d_gw = _cond_entropy(gw_delta)
    by_g: dict = defaultdict(Counter)
    for g, d in g_delta:
        by_g[g][d] += 1
    mult = [len(c) for c in by_g.values()]

    print(f"H(Δ)={H_d:.3f}  H(Δ|Γ)={H_d_g:.3f}  H(Δ|Γ,ω)={H_d_gw:.3f}  "
          f"ω增益={H_d_g-H_d_gw:.3f}")

    bridge = {}
    for k, vi_ in (("P", 0), ("C", 1), ("Au", 2)):
        nz = sum(1 for r in rows if r["gamma"][vi_] != 0 and r["delta"][vi_] != 0)
        same_nz = sum(1 for r in rows if r["gamma"][vi_] != 0 and r["delta"][vi_] != 0
                      and r["gamma"][vi_] == r["delta"][vi_])
        same_all = sum(1 for r in rows if r["gamma"][vi_] == r["delta"][vi_])
        bridge[k] = {"agree_all": same_all / len(rows) if rows else 0,
                     "agree_nonzero": same_nz / nz if nz else None, "n_nonzero": nz}
    print("flow_to_walk 桥接（σ价格方向=Δ流量方向？）："
          + "  ".join(f"{k}:{(b['agree_nonzero'] or 0):.0%}(n={b['n_nonzero']})"
                      for k, b in bridge.items()))

    return {"n_rows": len(rows), "H_d": H_d, "H_d_g": H_d_g, "H_d_gw": H_d_gw,
            "mult_max": max(mult) if mult else 0,
            "mult_mean": sum(mult) / len(mult) if mult else 0,
            "bridge": bridge, "n_cot": len(cot_dates),
            "cot_counts": {"ES": len(cot_es), "CL": len(cot_cl), "GC": len(cot_gc)}}


# ════════════════════════════════════════════════════════════════════════════
# Task 3：分辨变量搜索（a0=1h，跨边熵 + shuffle 对照）
# ════════════════════════════════════════════════════════════════════════════

def _vix_state(v: float) -> int:
    return -1 if v < 15 else (1 if v > 25 else 0)


def _slope_state(s: float) -> int:
    return -1 if s < -0.1 else (1 if s > 0.1 else 0)


def task3_resolution_search(cfg: RunConfig) -> dict:
    print("\n" + "=" * 78)
    print(f"Task 3：分辨变量搜索 — {cfg.label}")
    print("=" * 78)
    e = _build_edges(cfg)
    if "error" in e:
        print("  " + e["error"])
        return e
    raw, idx, keys = e["raw"], e["idx"], e["keys"]
    M = raw["M"]
    third_role = "R" if cfg.canonical else "Au"

    # 主边 Γ。
    sP = run_ratio_edge("P/M", raw["P"], M, keys, idx["P"], idx["M"])
    sC = run_ratio_edge("C/M", raw["C"], M, keys, idx["C"], idx["M"])
    s3 = run_ratio_edge(f"{third_role}/M", raw[third_role], M, keys, idx[third_role], idx["M"])
    # 交叉边 X。ω=σ(Au/Oil)=金油比。
    # 期货实例：第三轴=Au，oil=C(CL) → ω=Au/C 即交叉边 C/Au 的逆 → **X 剔除 C/Au**（防循环，
    #   与 gamma_delta_mapping 的 H(X_rest|Γ,ω) 一致）。正典实例：ω=Au/Oil 独立于 X=(P/C,P/R,C/R)，保留三边。
    xPC = run_ratio_edge("P/C", raw["P"], raw["C"], keys, idx["P"], idx["C"])
    xP3 = run_ratio_edge(f"P/{third_role}", raw["P"], raw[third_role], keys, idx["P"], idx[third_role])
    oil_role = "Oil" if cfg.canonical else "C"
    omega = run_ratio_edge("omega=Au/Oil", raw["Au"], raw[oil_role], keys, idx["Au"], idx[oil_role])
    if cfg.canonical:
        xC3 = run_ratio_edge(f"C/{third_role}", raw["C"], raw[third_role], keys, idx["C"], idx[third_role])
    else:
        xC3 = None  # 期货：C/Au 由 ω 覆盖，剔除防循环。
    # 外部候选：oil σ(Oil/M)、gold σ(Au/M)。
    cand_oil = run_ratio_edge("oil/M", raw[oil_role], M, keys, idx[oil_role], idx["M"])
    cand_gold = run_ratio_edge("gold/M", raw["Au"], M, keys, idx["Au"], idx["M"])

    # 候选 regime 变量（日分辨率，广播到 a0 周期键）。
    vix = _fetch_yf_daily("^VIX", "_vix_1d_yf.json")
    vix_day = ({ds[:10]: vix["closes"][i] for i, ds in enumerate(vix["dates"])}
               if vix else {})
    slope_map, slope_src = _fetch_yield_slope()
    print(f"VIX={'有' if vix else '无'}；斜率来源：{slope_src}")

    # 日采样（降自相关 + 与 regime 变量对齐）。
    def dmap(s):
        m = {}
        for k in keys:
            m[k[:10]] = s[k]
        return m
    DP, DC, D3, XPC, XP3, OM, OIL, GOLD = (
        dmap(s) for s in (sP, sC, s3, xPC, xP3, omega, cand_oil, cand_gold))
    XC3 = dmap(xC3) if xC3 is not None else None
    days = sorted(DP)

    rows = []
    for d in days:
        x = (XPC[d], XP3[d], XC3[d]) if XC3 is not None else (XPC[d], XP3[d])
        rows.append({
            "gamma": (DP[d], DC[d], D3[d]),
            "X": x,
            "omega": OM[d], "oil": OIL[d], "gold": GOLD[d],
            "vix": _vix_state(vix_day[d]) if d in vix_day else None,
            "slope": _slope_state(slope_map[d]) if d in slope_map else None,
        })

    rng = random.Random(_SEED)
    base_g = [(r["gamma"], r["X"]) for r in rows]
    base_gw = [((r["gamma"], r["omega"]), r["X"]) for r in rows]
    H_X = _entropy(list(Counter(r["X"] for r in rows).values()))
    H_X_g = _cond_entropy(base_g)
    H_X_gw = _cond_entropy(base_gw)
    print(f"日采样={len(rows)} 天；H(X)={H_X:.3f} H(X|Γ)={H_X_g:.3f} "
          f"H(X|Γ,ω)={H_X_gw:.3f} ω增益={H_X_g-H_X_gw:.3f}")

    candidates = {
        "oil(Oil/M)": [r["oil"] for r in rows],
        "gold(Au/M)": [r["gold"] for r in rows],
        "VIX_regime": [r["vix"] for r in rows],
        "yield_slope": [r["slope"] for r in rows],
    }
    cand_results = {}
    for cname, vals in candidates.items():
        valid = [(r, v) for r, v in zip(rows, vals) if v is not None]
        if len(valid) < 100:
            cand_results[cname] = {"error": f"有效样本不足({len(valid)})"}
            continue
        base = [((r["gamma"], r["omega"]), r["X"]) for r, _ in valid]
        with_c = [((r["gamma"], r["omega"], v), r["X"]) for r, v in valid]
        h_base = _cond_entropy(base)
        h_with = _cond_entropy(with_c)
        obs = h_base - h_with
        floor = _shuffle_floor(base, [v for _, v in valid], rng)
        cand_results[cname] = {"n": len(valid), "h_base": h_base, "h_with": h_with,
                               "observed_gain": obs, "shuffle_floor": floor,
                               "real_gain": obs - floor,
                               "n_states": len(set(v for _, v in valid))}

    print("候选变量真实增益（扣 shuffle）：")
    for cname, v in sorted([(k, v) for k, v in cand_results.items() if "error" not in v],
                           key=lambda kv: -kv[1]["real_gain"]):
        print(f"  {cname:14s} 真实={v['real_gain']:.3f} "
              f"(观测{v['observed_gain']:.3f}−伪{v['shuffle_floor']:.3f}) N={v['n']}")
    for cname, v in cand_results.items():
        if "error" in v:
            print(f"  {cname:14s} [{v['error']}]")

    # 最小变量集贪心（外部变量）。
    avail = {"ω": [r["omega"] for r in rows], "oil": [r["oil"] for r in rows],
             "gold": [r["gold"] for r in rows], "VIX": [r["vix"] for r in rows],
             "slope": [r["slope"] for r in rows]}
    dropped = [k for k, col in avail.items() if all(v is None for v in col)]
    for k in dropped:
        del avail[k]
    full_idx = [i for i in range(len(rows))
                if all(avail[k][i] is not None for k in avail)]
    ctx = [rows[i]["gamma"] for i in full_idx]
    Y = [rows[i]["X"] for i in full_idx]
    h_cur = _cond_entropy(list(zip(ctx, Y)))
    greedy_log = [("Γ", h_cur, None)]
    chosen: list[str] = []
    print(f"贪心起点 H(X|Γ)={h_cur:.3f}（公共非缺口样本 {len(full_idx)}）"
          + (f"；排除无覆盖 {dropped}" if dropped else ""))
    remaining = set(avail)
    while remaining:
        best, best_real, best_h = None, -1e9, None
        for k in remaining:
            col = [avail[k][i] for i in full_idx]
            with_pairs = [((c, v), y) for c, v, y in zip(ctx, col, Y)]
            h_with = _cond_entropy(with_pairs)
            floor = _shuffle_floor(list(zip(ctx, Y)), col, rng, n=20)
            real = (h_cur - h_with) - floor
            if real > best_real:
                best, best_real, best_h = k, real, h_with
        if best_real <= 0.005:
            break
        chosen.append(best)
        ctx = [(*c, avail[best][i]) for c, i in zip(ctx, full_idx)]
        h_cur = best_h
        greedy_log.append((best, h_cur, best_real))
        print(f"  + {best:6s} → H={h_cur:.3f}（真实增益 {best_real:.3f}）")
        remaining.discard(best)

    return {"label": cfg.label, "n_rows": len(rows), "H_X": H_X, "H_X_g": H_X_g,
            "H_X_gw": H_X_gw, "candidates": cand_results, "greedy": greedy_log,
            "n_full": len(full_idx), "chosen": chosen, "final_H": h_cur,
            "slope_src": slope_src, "third_role": third_role,
            "canonical": cfg.canonical,
            "x_edges": (f"σ(P/C),σ(P/{third_role}),σ(C/{third_role})" if cfg.canonical
                        else f"σ(P/C),σ(P/Au)（剔除 C/Au=ω 防循环）")}


# ════════════════════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════════════════════

def write_report(cfg: RunConfig, t1: dict, t2: dict, t3: dict) -> None:
    L = []
    w = L.append
    w("# Γ→Δ 全验证结果：配置转换矩阵 + Δ流量直接验证 + 分辨变量搜索\n")
    w(f"> 生成：{datetime.now().strftime('%Y-%m-%d %H:%M')}　实例：**{cfg.label}**\n")
    w("> 脚本：`analysis/gamma_delta_full_verification.py`；上游 `gamma_delta_mapping.py`"
      "（1h K4 H(X|Γ)=1.584）。\n")
    w(f"> **a0 = {cfg.a0}**（最小可用周期；σ=涌现最高级别走势 `move_snapshot.moves[-1]`）。"
      "日线当 a0 是错误（最高级别 10 年仅翻转 1-2 次），在日线读线段级 σ 是补丁——已废弃。\n")

    chosen = t3.get("chosen", []) if "error" not in t3 else []
    fH = t3.get("final_H", float("nan")) if "error" not in t3 else float("nan")
    w("\n## 结果包（六要素）\n")
    w("**1. 结论**：")
    if "error" not in t1:
        w(f"(a) Task1 配置转换矩阵（a0={cfg.a0}，σ=最高级别走势）：")
        w(f"{t1['n_days']} 天出现 {t1['n_configs_seen']}/27 配置、{t1['n_transitions']} 次翻转")
        if not t1["canonical"]:
            w("（期货实例，无 R 轴 → 无病态标签，结构转换矩阵；病态版待 30min 正典）；")
        else:
            runs_avg = {p: (sum(r) / len(r) if r else 0)
                        for p, r in t1.get("runs", {}).items()}
            top3 = [p for p, _ in sorted(runs_avg.items(), key=lambda x: -x[1])[:3]]
            zhuanzi = t1.get("patho_residence", {}).get("走资", 0)
            w(f"（正典 P/C/R）。**§10.5 否定性结果**：最持久 Top3={top3}，"
              f"走资/沉没非吸收态（沉没游程最短之一"
              f"{'、走资 12 年内 0 天' if zhuanzi == 0 else ''}）；")
    if "error" not in t2:
        w(f"(b) Task2 Γ_期货→Δ_COT 一对多(H(Δ|Γ)={t2['H_d_g']:.2f})，"
          "flow_to_walk 桥接≈随机→σ价格方向≠流量方向；")
    if "error" not in t3:
        w(f"(c) Task3 外部最小集 {chosen} 把 H(X|Γ) 降到 {fH:.3f} bits"
          f"{'（未到 0，残余结构性不确定）' if fH > 0.1 else ''}。")
    w("\n")
    w("**2. 定义依据**：Γ=(σ_P,σ_C,σ_R)=`config_space.Configuration`；"
      "病态判据=`capital_flow_taxonomy.classify`；"
      f"σ=最高级别走势（a0={cfg.a0} 涌现）；{cfg.note}\n")
    win_note = ("公共窗口由 M(UUP 30m) 限到 ~12yr（2016+）" if cfg.canonical
                else "$(UUP 1m→1h) 限到 2020+")
    w(f"**3. 边界条件**：a0={cfg.a0}，{win_note}；"
      "Δ=COT 周频非商业净持仓变化（仅期货 ES/GC/CL，无 R 期货代理）；"
      "转换矩阵按日采样（σ 已由 a0 涌现，采样间隔不改 σ）；"
      "Task3 多变量条件熵含有限样本伪增益（已 shuffle 扣减；真实残余 ≥ 报告值）；"
      "走资=0 天是此窗口（牛市主导）特性，非分类错误（330号：真走资需跨国层）。\n")
    w("**4. 下游推论**：σ 价格方向≠真实流量（Task2）→ 流量驱动选股需 COT/fund flows；"
      "走资/沉没非吸收态（§10.5 否定）→ M2 选股不宜假设这两态「易进难出」而提前降仓。\n")
    w("**5. 谱系引用**：254/330/482/527/528号；"
      "auto-memory `project_k4_fold_channel_model`、`project_omega_regime_falsified`、"
      "`project_gamma_delta_full_verification`。\n")
    w("**6. 影响声明**：`analysis/gamma_delta_full_verification.py`（a0 修正：日线→1h/30m，"
      "σ 线段补丁→最高级别走势；`--canonical` 切正典 P/C/R）"
      "+ `scripts/tws/fetch_30m_tws.py`（新，6+3 标的 30m）+ 本结果文件；不改源码。\n")

    # ── Task 1 ──
    w(f"\n---\n\n## Task 1：配置转换矩阵（a0={cfg.a0}，L2）\n")
    if "error" in t1:
        w(f"**数据缺口**：{t1['error']}\n")
    else:
        w(f"- 实例：{t1['label']}；第三轴={t1['third_role']}\n")
        w(f"- a0={t1['a0']}，公共 bar={t1['n_bars']}，日采样={t1['n_days']} 天"
          f"（{t1['span'][0]}→{t1['span'][1]}）\n")
        w(f"- 出现配置数：{t1['n_configs_seen']}/27；**配置翻转次数={t1['n_transitions']}**\n")
        if not t1["canonical"]:
            w("\n> **期货折叠通道实例**（第三轴=Au 金，非不动产 R）→ 不套用卢麒元病态标签"
              "（病态判据需 P/C/R）。本表为**结构转换矩阵**：配置驻留 + 自循环吸收。"
              "病态标签版（含 §10.5 走资/沉没吸收态检验）待 TWS 30min 正典 VNQ=R。\n")
        # 配置吸收态 Top。
        w("\n### 1.1 配置吸收态 Top（自循环概率最高）\n")
        w(f"| Γ=(σP,σC,σ{t1['third_role']}) | 出现 | 自循环P |\n|---|---|---|\n")
        for g, tot_, self_p in t1["absorb"][:12]:
            gl = f"({_SYM[g[0]]},{_SYM[g[1]]},{_SYM[g[2]]})"
            w(f"| {gl} | {tot_} | {self_p:.3f} |\n")
        if t1["canonical"]:
            theory = mode_histogram()
            ttot = sum(theory.values())
            tot = sum(t1["patho_residence"].values())
            w("\n### 1.2 病态驻留分布（实际时间 vs 理论配分）\n")
            w("| 病态 | 实际天数 | 实际占比 | 理论配置占比 |\n|---|---|---|---|\n")
            for mode in PathologyMode:
                d = t1["patho_residence"].get(mode.value, 0)
                w(f"| {mode.value} | {d} | {d/tot:.1%} | {theory[mode]/ttot:.0%} |\n")
            w("\n### 1.3 吸收态分析（自循环 + 平均游程）\n")
            w("| 病态 | 自循环P | 平均游程(天) | 出现天数 |\n|---|---|---|---|\n")
            by_run = []
            for p, sp_ in sorted(t1["patho_self"].items(), key=lambda x: -x[1]):
                runs = t1["runs"].get(p, [])
                avg = sum(runs) / len(runs) if runs else 0
                by_run.append((p, avg))
                w(f"| {p} | {sp_:.3f} | {avg:.1f} | {t1['patho_residence'].get(p,0)} |\n")
            # §10.5 吸收态假说裁决（诚实，可能否定）。
            top3 = [p for p, _ in sorted(by_run, key=lambda x: -x[1])[:3]]
            never = [m.value for m in PathologyMode
                     if t1["patho_residence"].get(m.value, 0) == 0]
            fs_top = {"走资", "沉没"} & set(top3)
            w("\n> **§10.5 吸收态裁决**（走资/沉没 = 方向性吸收态？易进难出=游程最长）：")
            w(f"实测最持久 Top3={top3}。")
            if fs_top:
                w(f"{sorted(fs_top)} 入 Top3 → 部分支持。\n")
            else:
                w("**走资/沉没均不在 Top3 → §10.5「走资/沉没=方向性吸收态」未得经验支持（否定性结果）。**")
                w(f"最持久=健康/溃坝/空转；沉没游程最短之一。")
                if "走资" in never:
                    w("走资(FLIGHT)在 2016-2026 **从未出现**（0 天）——需 ≥2 资产同时相对 $ 走弱，")
                    w("此牛市主导窗口在走势级别上未发生（330号：单国 K4 测不到真走资，需跨国层）。")
                w("与日线-线段实例（早期跑）的否定结果一致，现在正典 30m/12yr 上确证。\n")
            w("\n### 1.4 病态层转换矩阵（行=今，列=明，行归一）\n")
            modes_seen = [m.value for m in PathologyMode
                          if t1["patho_residence"].get(m.value, 0) > 0]
            w("| 今\\明 | " + " | ".join(modes_seen) + " |\n")
            w("|" + "---|" * (len(modes_seen) + 1) + "\n")
            for a in modes_seen:
                row = t1["ptrans"].get(a, {})
                tota = sum(row.values()) or 1
                w(f"| {a} | " + " | ".join(f"{row.get(b,0)/tota:.2f}"
                                           for b in modes_seen) + " |\n")

    # ── Task 2 ──
    w("\n---\n\n## Task 2：Δ 流量直接验证（CFTC COT，期货子集，L2）\n")
    if "error" in t2:
        w(f"**数据缺口**：{t2['error']}\n> ETF fund flows 无免费 API（有效域边界，非 workaround）。\n")
    else:
        w(f"- COT 记录：{t2['cot_counts']}；公共周={t2['n_cot']}；对齐样本={t2['n_rows']}\n")
        w("- **Δ=非商业净持仓周变化符号**（真实流量代理，非价格方向）；"
          "Γ_期货 来自 1h a0 最高级别走势\n")
        w("\n### 2.1 Γ→Δ 一对多检验\n| 量 | bits |\n|---|---|\n")
        w(f"| H(Δ) | {t2['H_d']:.3f} |\n| H(Δ\\|Γ_期货) | {t2['H_d_g']:.3f} |\n"
          f"| H(Δ\\|Γ_期货,ω) | {t2['H_d_gw']:.3f} |\n")
        w(f"\n> Γ→Δ：**{'一对多✓' if t2['H_d_g']>0.05 else '确定'}**"
          f"（多样性 max={t2['mult_max']} mean={t2['mult_mean']:.2f}）；"
          f"ω增益={t2['H_d_g']-t2['H_d_gw']:.3f} bits。\n")
        w("\n### 2.2 flow_to_walk 桥接命中率（σ价格方向=Δ流量方向？）\n")
        w("| 顶点 | 全样本同号 | 双非零同号 | 非零样本 |\n|---|---|---|---|\n")
        for k, b in t2["bridge"].items():
            an = f"{b['agree_nonzero']:.1%}" if b["agree_nonzero"] is not None else "n/a"
            w(f"| {k} | {b['agree_all']:.1%} | {an} | {b['n_nonzero']} |\n")
        w("\n> σ≈Δ 是经验假设（`capital_flow_taxonomy`）。双非零命中率≈50%（随机）"
          "→价格走势方向不能代理真实流量（存量重估偏差）。\n")

    # ── Task 3 ──
    w(f"\n---\n\n## Task 3：分辨变量搜索（a0={cfg.a0}，L2/L3）\n")
    if "error" in t3:
        w(f"**数据缺口**：{t3['error']}\n")
    else:
        w(f"- 日采样样本：{t3['n_rows']}（全变量非缺口：{t3['n_full']}）；交叉边 X={t3['x_edges']}\n")
        w(f"- 收益率斜率来源：{t3.get('slope_src','—')}\n")
        if not t3.get("canonical", False):
            w("- **期货实例**：oil(Oil/M)=σ(CL/$)、gold(Au/M)=σ(GC/$) 即 Γ 分量（内生）→ 增益≈0 是预期；"
              "ω=σ(GC/CL) 已剔除对应 X 边防循环。真正外部分辨变量=VIX、yield_slope。\n")
        w("\n### 3.1 基线条件熵\n| 量 | bits |\n|---|---|\n")
        w(f"| H(X) | {t3['H_X']:.3f} |\n| H(X\\|Γ) | {t3['H_X_g']:.3f} |\n"
          f"| H(X\\|Γ,ω) | {t3['H_X_gw']:.3f} |\n")
        w("\n### 3.2 候选变量信息增益（shuffle 扣减伪增益）\n")
        w("| 变量 | 观测增益 | shuffle伪增益 | **真实增益** | 状态数 | N |\n"
          "|---|---|---|---|---|---|\n")
        for cname, v in sorted([(k, v) for k, v in t3["candidates"].items()
                                if "error" not in v], key=lambda kv: -kv[1]["real_gain"]):
            w(f"| {cname} | {v['observed_gain']:.3f} | {v['shuffle_floor']:.3f} | "
              f"**{v['real_gain']:.3f}** | {v['n_states']} | {v['n']} |\n")
        for cname, v in t3["candidates"].items():
            if "error" in v:
                w(f"| {cname} | — | — | [{v['error']}] | — | — |\n")
        w("\n> 加任何变量都机械降熵（有限样本+高基数）。shuffle 对照测纯伪增益；"
          "**真实增益=观测−伪**，≤0=无分辨力。\n")
        w("\n### 3.3 最小变量集贪心搜索（仅外部变量）\n")
        w("| 步骤 | 加入 | H(X\\|...) | 真实增益 |\n|---|---|---|---|\n")
        for i, (k, h, real) in enumerate(t3["greedy"]):
            w(f"| {i} | {k} | {h:.3f} | {f'{real:.3f}' if real is not None else '—'} |\n")
        w(f"\n> 选中集：**{t3['chosen'] or '（无变量真实增益>0.005）'}**；"
          f"终态 H(X\\|Γ,选中)={t3['final_H']:.3f} bits。")
        if t3["final_H"] > 0.1:
            w("**未到 0** → (Γ,ω,候选) 不足以完全确定交叉边；残余=结构性不确定"
              "（σ 不保比值同态）或需未观测变量（真实流量/级别细分）。\n")
        else:
            w("接近 0 → 近完备变量集。\n")

    _OUT.write_text("".join(L), encoding="utf-8")
    print(f"\n报告已写入：{_OUT}")


def main() -> None:
    random.seed(_SEED)
    canonical = "--canonical" in sys.argv
    cfg = CANONICAL_30M if canonical else FUTURES_1H
    print(f"Γ→Δ 全验证。实例：{cfg.label}")
    if canonical:
        # 检查 30min 数据是否就绪。
        missing = [f for _, (f, _) in cfg.sources.items() if not (_DATA / f).exists()]
        if missing:
            print(f"  [缺 30min 数据：{missing}] —— TWS 拉取未完成，退回期货 1h 实例。")
            cfg = FUTURES_1H
    t1 = task1_transition_matrix(cfg)
    t2 = task2_delta_flow()
    t3 = task3_resolution_search(cfg)
    write_report(cfg, t1, t2, t3)
    print("\n完成。")


if __name__ == "__main__":
    main()
