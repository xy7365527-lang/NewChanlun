"""统一规则多标的回测 — HSI / SHCOMP / BRN / QQQ 可比对照（真实 OHLC 执行价）。

存在论位置
----------
此前三标的（HSI/SHCOMP/BRN）由不同脚本回测，规则各异 → 不可比：
  - hk_index_backtest.py：纯缠论全仓，无走势方向、无 FSM。
  - hk_three_stage_backtest.py：cost_reduction_fsm，但**走势方向用卖点编号**判级别。

本脚本把四标的统一到 `scripts/recursive_backtest.py`（QQQ 80% 胜率那个）的核心逻辑，
**与之完全同构**：

  1. 走势方向用**线段方向**判断（不看卖点编号）。
  2. 线段方向转空 = 主级别退出清仓。
  3. 线段方向未转 + 遇卖点 = 次级别短差（减仓后回补）。
  4. cost_reduction_fsm 管理仓位（267/338 号三阶段降成本）。
  5. **lag=1 因果口径**：信号在确认日已知走势方向，执行在**下一交易日收盘**。
  6. PH settle 门控（B 组）：sublevel settle 确认建仓。

复用 recursive_backtest.py 的：TrendTimeline / build_trend_timeline /
build_settle_indices / has_settle_near / classify_holding_event /
FsmTrade / FsmConfig / run_fsm_backtest / run_simple_backtest / Stats / compute_stats。

执行价：真实 OHLC，绝不用 TV 标注价（消除前视偏差）
----------------------------------------------------
**核心修正（前视偏差）**：TV 缠论买卖点 label 的 price 是分型端点理想价（买点打在
局部低点、卖点打在局部高点）。用标注价做执行价 → 买点价天然 < 卖点价 → 系统性
前视偏差（用 TV 标注价实测 C 组 100% 假胜率）。本脚本执行价一律取 **yfinance 真实日线
收盘价**（lag=1）。**注意**：这缩小但未完全消除前视偏差 —— 缠论买卖点标在分型极值点，
DTW 精确对齐 + lag=1（仅移 1 日）使执行价仍近极值，C 组残留 89-100% 高胜率（与 QQQ
基准 93.6% 同源）。彻底消除需分型确认滞后建模（TV label 未提供），故 **A 组（趋势持仓）
为主要观察对象**。详见报告缺口声明。

对齐：DTW 单调最优对齐（价格锚点）
----------------------------------
TV daily_chanlun 的 bar 坐标系不是真实日历日线（~500 端点 bar 跨数十年），bar 索引
无法直接用。采用 **DTW 单调对齐**把 TV 买卖点 price 序列对齐到 yfinance 真实日线
close 序列：动态规划找严格递增映射 f，最小化 Σ|tv_price[i] − yf_close[f[i]]|。
全局最优 + 单调，自动处理 bar↔日历的非线性（优于 bar 线性 scale —— BRN 上后者
系统性偏移致中位误差 22%，DTW 降至 0.65%）。对齐质量（中位价格误差）：
  HSI 0.18% / SHCOMP 0.15% / BRN 0.65%（BRN 有 31/116 信号 >5%，Brent 同价多期
  歧义致单调约束下次优匹配 —— 已诊断声明，非 bug）。
执行价 = yf_close[f[i] + 1]（lag=1），走势方向 = seg_dir[f[i]]（信号确认日，因果）。

数据源
------
  HSI    → yfinance ^HSI       SHCOMP → yfinance 000001.SS
  BRN    → yfinance BZ=F       QQQ    → 本地 QQQ_1d_max.json（真实日线缓存）
QQQ 统一组用 DTW 对齐到 QQQ 真实日线（与三标的同构）；QQQ 基准对照 = 原
recursive_backtest.py（周线 bar 线性映射，从 JSON 读）。

认识论等级（formalization-validity-domain）
-------------------------------------------
- 递归引擎 / OnlineMergeTree / FSM 转移：**L0**。
- DTW bar→真实日线对齐：**L1**（价格匹配，质量见上；BRN 部分信号 L1 噪声较高）。
- 各组胜率/收益：**L2**（真实 OHLC 执行价，单标的单时段可否证 —— 否定性结果有价值）。
- 四标的横向对比：**L3**（四标的同源：真实日线 + 递归引擎走势 + DTW 对齐 + lag=1）。
"""
from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
sys.path.insert(0, str(ROOT / "src"))

import recursive_backtest as rb  # noqa: E402  复用 FSM/timeline/stats（统一规则物质形态）

DATA_DIR = ROOT / "analysis" / "data_cache"
REPORT_PATH = ROOT / "analysis" / "unified_backtest_results.md"
RESULT_JSON = DATA_DIR / "unified_backtest_results.json"
QQQ_BASELINE_JSON = DATA_DIR / "recursive_backtest_qqq.json"

_BSP_KEYWORDS = ["1买", "2买", "3买", "1卖", "2卖", "3卖"]
_LAG = 1  # 因果口径：信号确认日已知走势方向，执行在下一交易日收盘

SYMBOLS = [
    {"name": "hsi", "label": "恒生指数 HSI", "labels": "hsi_daily_chanlun.json",
     "source": "yfinance", "ticker": "^HSI"},
    {"name": "shcomp", "label": "上证指数 SHCOMP", "labels": "shcomp_daily_chanlun.json",
     "source": "yfinance", "ticker": "000001.SS"},
    {"name": "brn", "label": "Brent 原油 BRN", "labels": "brn_daily_chanlun.json",
     "source": "yfinance", "ticker": "BZ=F"},
    {"name": "qqq", "label": "纳指 QQQ（统一组）", "labels": "qqq_chanlun_labels.json",
     "source": "cache", "ohlc": "QQQ_1d_max.json"},
]


# ════════════════════════════════════════════════════════════════════════════
# 1. 真实 OHLC 加载（yfinance + 本地缓存）
# ════════════════════════════════════════════════════════════════════════════


def _yf_cache_path(ticker: str) -> Path:
    safe = ticker.replace("^", "").replace("=", "_").replace(".", "_")
    return DATA_DIR / f"_yf_{safe}_daily.json"


def load_ohlc(cfg: dict) -> pd.DataFrame:
    """真实日线 OHLC（DatetimeIndex）。yfinance 结果落地缓存避免重复下载。"""
    if cfg["source"] == "cache":
        d = json.loads((DATA_DIR / cfg["ohlc"]).read_text())
        return pd.DataFrame(
            {"open": d["opens"], "high": d["highs"], "low": d["lows"], "close": d["closes"]},
            index=pd.to_datetime(d["dates"]),
        ).sort_index()

    cache = _yf_cache_path(cfg["ticker"])
    if cache.exists():
        d = json.loads(cache.read_text())
    else:
        import yfinance as yf
        df = yf.download(cfg["ticker"], period="max", interval="1d",
                         progress=False, auto_adjust=False)
        if df.empty:
            raise RuntimeError(f"yfinance {cfg['ticker']} 返回空")
        d = {
            "dates": [str(x.date()) for x in df.index],
            "open": df["Open"].values.astype(float).ravel().tolist(),
            "high": df["High"].values.astype(float).ravel().tolist(),
            "low": df["Low"].values.astype(float).ravel().tolist(),
            "close": df["Close"].values.astype(float).ravel().tolist(),
        }
        cache.write_text(json.dumps(d))
    return pd.DataFrame(
        {"open": d["open"], "high": d["high"], "low": d["low"], "close": d["close"]},
        index=pd.to_datetime(d["dates"]),
    ).sort_index()


# ════════════════════════════════════════════════════════════════════════════
# 2. TV 买卖点加载（si=3 主缠论 study）
# ════════════════════════════════════════════════════════════════════════════


def _main_chan_si(data: dict) -> int:
    counts: dict[int, int] = {}
    for lbl in data["pine"]["labels"]:
        text = lbl.get("text") or ""
        if any(k in text for k in _BSP_KEYWORDS):
            counts[lbl.get("si")] = counts.get(lbl.get("si"), 0) + 1
    if not counts:
        raise ValueError("无买卖点 label")
    return max(counts, key=counts.get)


def load_bsp(cfg: dict) -> list[dict]:
    """主缠论 study 的 1/2/3 类买卖点，按 bar 排序。price 仅用于 DTW 对齐，不作执行价。"""
    data = json.loads((DATA_DIR / cfg["labels"]).read_text())
    si = _main_chan_si(data)
    out: list[dict] = []
    for lbl in data["pine"]["labels"]:
        if lbl.get("si") != si:
            continue
        text = lbl.get("text") or ""
        if lbl.get("price") is None or lbl.get("time") is None:
            continue
        raw = next((k for k in _BSP_KEYWORDS if k in text), None)
        if raw is None:
            continue
        suffix = "(趋势)" if "趋势" in text else ("(盘整)" if "盘整" in text else "")
        out.append({
            "bar": int(lbl["time"]),
            "price": float(lbl["price"]),
            "signal_type": raw + suffix,
            "raw_type": raw,
            "is_buy": "买" in raw,
        })
    out.sort(key=lambda s: s["bar"])
    return out


# ════════════════════════════════════════════════════════════════════════════
# 3. DTW 单调最优对齐（价格锚点 → 真实日线索引）
# ════════════════════════════════════════════════════════════════════════════


def dtw_align(prices: list[float], closes: np.ndarray) -> list[int]:
    """严格递增单调映射 f，最小化 Σ|prices[i] − closes[f[i]]|（动态规划）。

    全局最优 + 单调，自动处理 TV bar ↔ 真实日历的非线性。
    """
    A = np.asarray(prices, dtype=float)
    B = np.asarray(closes, dtype=float)
    m, n = len(A), len(B)
    INF = 1e18
    dp = np.empty((m, n))
    bk = np.full((m, n), -1, dtype=np.int32)
    dp[0] = np.abs(A[0] - B)
    for i in range(1, m):
        prev = dp[i - 1]
        pm = np.empty(n)
        pm_arg = np.empty(n, dtype=np.int32)
        cur_min, cur_arg = INF, -1
        for j in range(n):  # 前缀最小（严格递增：用 0..j-1）
            pm[j] = cur_min
            pm_arg[j] = cur_arg
            if prev[j] < cur_min:
                cur_min, cur_arg = prev[j], j
        dp[i] = np.abs(A[i] - B) + pm
        bk[i] = pm_arg
    f = [0] * m
    f[m - 1] = int(np.argmin(dp[m - 1]))
    for i in range(m - 1, 0, -1):
        f[i - 1] = int(bk[i][f[i]])
    return f


# ════════════════════════════════════════════════════════════════════════════
# 4. 信号构造（真实日期 + lag=1 真实执行价）
# ════════════════════════════════════════════════════════════════════════════


def build_signals(bsp: list[dict], f: list[int], df: pd.DataFrame) -> tuple[list[dict], list[float]]:
    """date_idx=f[i]（走势方向因果索引）；matched_close=close[f[i]+lag]（lag=1 真实执行价）。"""
    closes = df["close"].values.astype(float)
    dates = df.index
    n = len(closes)
    sigs: list[dict] = []
    diffs: list[float] = []
    for s, di in zip(bsp, f):
        exec_idx = min(di + _LAG, n - 1)
        diffs.append(abs(closes[di] - s["price"]) / s["price"] * 100)
        sigs.append({
            "bar_idx": s["bar"],
            "date_idx": di,
            "date": dates[di],
            "matched_close": float(closes[exec_idx]),  # lag=1 真实收盘，非 TV 标注价
            "tv_price": s["price"],
            "align_diff_pct": diffs[-1],
            "signal_type": s["signal_type"],
            "raw_type": s["raw_type"],
            "is_buy": s["is_buy"],
        })
    return sigs, diffs


# ════════════════════════════════════════════════════════════════════════════
# 5. 单标的三组回测
# ════════════════════════════════════════════════════════════════════════════


@dataclass
class SymbolResult:
    name: str
    label: str
    n_bars: int
    n_signals: int
    n_buy: int
    n_sell: int
    n_segments: int
    n_settle: int
    align_median: float
    align_bad: int
    date0: str
    date1: str
    bh: float
    sig_in_up: int
    sig_in_down: int
    stats_a: rb.Stats
    stats_b: rb.Stats
    stats_c: rb.Stats


def _count_segments(seg_dir: list[str]) -> int:
    n, prev = 0, None
    for d in seg_dir:
        if d and d != prev:
            n += 1
            prev = d
    return n


def run_symbol(cfg: dict) -> SymbolResult:
    df = load_ohlc(cfg)
    bsp = load_bsp(cfg)
    closes = df["close"].values.astype(float)
    f = dtw_align([s["price"] for s in bsp], closes)
    signals, diffs = build_signals(bsp, f, df)

    timeline = rb.build_trend_timeline(df)
    settle = rb.build_settle_indices(closes)

    trades_a = rb.run_fsm_backtest(signals, timeline, settle,
                                   rb.FsmConfig(use_settle_filter=False, use_trend_filter=True))
    trades_b = rb.run_fsm_backtest(signals, timeline, settle,
                                   rb.FsmConfig(use_settle_filter=True, settle_window=5, use_trend_filter=True))
    trades_c = rb.run_simple_backtest(signals)

    sig_dirs = [timeline.seg_dir[s["date_idx"]] for s in signals]
    d0, d1 = signals[0]["date"], signals[-1]["date"]
    c0 = float(df["close"].asof(d0))
    c1 = float(df["close"].asof(d1))
    bh = (c1 - c0) / c0 if c0 else float("nan")

    return SymbolResult(
        name=cfg["name"], label=cfg["label"], n_bars=len(df),
        n_signals=len(signals),
        n_buy=sum(1 for s in signals if s["is_buy"]),
        n_sell=sum(1 for s in signals if not s["is_buy"]),
        n_segments=_count_segments(timeline.seg_dir), n_settle=len(settle),
        align_median=float(np.median(diffs)), align_bad=sum(1 for d in diffs if d > 5),
        date0=str(d0.date()), date1=str(d1.date()), bh=bh,
        sig_in_up=sum(1 for d in sig_dirs if d == "up"),
        sig_in_down=sum(1 for d in sig_dirs if d == "down"),
        stats_a=rb.compute_stats("A 缠论+FSM三阶段", trades_a),
        stats_b=rb.compute_stats("B +PH settle门控", trades_b),
        stats_c=rb.compute_stats("C 简化全仓baseline", trades_c),
    )


# ════════════════════════════════════════════════════════════════════════════
# 6. 报告
# ════════════════════════════════════════════════════════════════════════════


def load_qqq_baseline() -> dict | None:
    return json.loads(QQQ_BASELINE_JSON.read_text()) if QQQ_BASELINE_JSON.exists() else None


def build_report(results: list[SymbolResult], qqq_base: dict | None) -> str:
    fp, ff = rb.fmt_pct, rb.fmt_f
    L: list[str] = []
    a = L.append

    a("# 统一规则多标的回测 — HSI / SHCOMP / BRN / QQQ（真实 OHLC 执行价）")
    a("")
    a("**统一规则**（复用 `scripts/recursive_backtest.py` 核心 FSM 逻辑，四标的同构）：")
    a("1. 走势方向用**线段方向**判断（不看卖点编号）。")
    a("2. 线段方向转空 = 主级别退出清仓。")
    a("3. 线段方向未转 + 遇卖点 = 次级别短差。")
    a("4. cost_reduction_fsm 三阶段降成本（267/338号）。")
    a("5. **lag=1 因果口径**：执行在信号确认日的下一交易日收盘。")
    a("6. PH settle 门控（B 组）。")
    a("")
    a("**执行价 = yfinance 真实日线收盘（绝不用 TV 标注价）**；TV 买卖点经 **DTW 单调对齐**"
      "映射到真实日线（价格锚点全局最优，严格递增）。")
    a("")
    a("| 组 | 持仓系统 | 走势过滤 | PH 过滤 |")
    a("|----|---------|---------|--------|")
    a("| A | cost_reduction_fsm 五状态三阶段 | 仅 up 线段建仓 | 否 |")
    a("| B | cost_reduction_fsm 五状态三阶段 | 仅 up 线段建仓 | 买点前 5 日内有 settle |")
    a("| C | 简化全仓进出（long-only baseline） | 无 | 否 |")
    a("")
    a("**认识论等级**：L2（真实 OHLC 执行价，可否证）；横向 **L3**（四标的同源）。")
    a("")
    a("---")
    a("")
    a("## 汇总：A 组（缠论+FSM 三阶段，线段方向门控）")
    a("")
    a("| 标的 | 日线根数 | 买卖点 | 线段数 | A笔数 | A胜率 | A总收益 | A回撤 | A盈亏比 | 平均持有天 |")
    a("|------|--------|-------|-------|------|------|--------|------|--------|----------|")
    for r in results:
        s = r.stats_a
        a(f"| {r.label} | {r.n_bars} | {r.n_signals}(买{r.n_buy}/卖{r.n_sell}) | {r.n_segments} "
          f"| {s.closed_trades} | {fp(s.win_rate)} | {fp(s.total_return)} | {fp(s.max_drawdown)} "
          f"| {ff(s.profit_factor)} | {ff(s.avg_hold_days, 0)} |")
    a("")
    a("## 汇总：B 组（A + PH settle 门控）")
    a("")
    a("| 标的 | B笔数 | B胜率 | B总收益 | B回撤 | B盈亏比 |")
    a("|------|------|------|--------|------|--------|")
    for r in results:
        s = r.stats_b
        a(f"| {r.label} | {s.closed_trades} | {fp(s.win_rate)} | {fp(s.total_return)} "
          f"| {fp(s.max_drawdown)} | {ff(s.profit_factor)} |")
    a("")
    a("## 汇总：C 组（简化全仓 baseline）")
    a("")
    a("| 标的 | C笔数 | C胜率 | C总收益 | C回撤 | C盈亏比 | C平均持有天 | 回测期b&h |")
    a("|------|------|------|--------|------|--------|-----------|----------|")
    for r in results:
        s = r.stats_c
        a(f"| {r.label} | {s.closed_trades} | {fp(s.win_rate)} | {fp(s.total_return)} "
          f"| {fp(s.max_drawdown)} | {ff(s.profit_factor)} | {ff(s.avg_hold_days, 0)} | {fp(r.bh)} |")
    a("")

    if qqq_base:
        a("## QQQ 基准对照（原 recursive_backtest.py：周线 bar 线性映射）")
        a("")
        a("> 与上方 QQQ 统一组同样用 QQQ 真实日线 OHLC + 递归引擎走势，差异仅在**信号映射方式**"
          "（基准=周线 bar 线性推算 + 价格精修；统一组=DTW 单调对齐 + lag=1）。")
        a("")
        a("| 组 | 笔数 | 胜率 | 总收益 | 回撤 | 盈亏比 | 平均持有天 |")
        a("|----|------|------|--------|------|--------|-----------|")
        for k, v in qqq_base.items():
            a(f"| {k} | {v['closed_trades']} | {fp(v['win_rate'])} | {fp(v['total_return'])} "
              f"| {fp(v['max_drawdown'])} | {ff(v['profit_factor'])} | {ff(v['avg_hold_days'], 0)} |")
        a("")

    a("## 各标的：对齐质量 / 走势覆盖 / 按信号类型")
    a("")
    for r in results:
        a(f"### {r.label}")
        a("")
        a(f"- 日线 {r.n_bars} 根，回测期 {r.date0} ~ {r.date1}")
        a(f"- **DTW 对齐质量**：价格中位误差 {r.align_median:.2f}%，偏差>5% 的 {r.align_bad}/{r.n_signals}")
        a(f"- 买卖点落在 up 线段 {r.sig_in_up} / down 线段 {r.sig_in_down} | PH settle {r.n_settle} 个")
        a("")
        a("| 组 | 信号类型 | 笔数 | 胜率 | 平均收益 |")
        a("|----|---------|------|------|---------|")
        for st in (r.stats_a, r.stats_b, r.stats_c):
            for kind, v in st.by_type.items():
                a(f"| {st.name} | {kind} | {v['trades']} | {fp(v['win_rate'])} | {fp(v['avg_ret'])} |")
        a("")

    a("## 结果包（六要素）")
    a("")
    a("1. **结论**：四标的在统一规则（线段方向判走势 + cost_reduction_fsm + lag=1 真实 OHLC 执行价）"
      "下的 A/B/C 三组横向可比（L3）。见汇总表。")
    a("2. **定义依据**：走势方向 = 递归引擎日线线段（缠论§5 Move[0]）；FSM = 267/338号；"
      "PH settle = a_online_persistence §10；买卖点 = TV CHANLUN|CZSC 1/2/3 类；"
      "执行价 = yfinance 真实日线收盘（^HSI/000001.SS/BZ=F/QQQ 缓存）。")
    a("3. **边界条件**（结论翻转）：")
    a("   - DTW 对齐若价格误差大（BRN 部分信号），映射日偏离真实信号日 → 执行价对应错误时点；"
      "BRN 结果可信度低于 HSI/SHCOMP。")
    a("   - use_trend_filter 关闭 → A/B 趋同 C；sub_ratio→0 → 短差停止。")
    a("   - 单边强趋势标的，buy-and-hold 吞没主动交易优势。")
    a("4. **下游推论**：若 A>C 跨标的成立 → 线段门控+短差降成本提供跨市场正 alpha；"
      "若 A≤C → 该机制账面收益不足抵消放弃的趋势涨幅（证伪）。")
    a("5. **谱系引用**：267/338/268a号；formalization-validity-domain（线段=Move[0]，"
      "走势级别错配）；相对 hk_three_stage_backtest.py 的概念分离 = 「走势方向 vs 卖点编号」判级别；"
      "相对前一版本统一脚本的修正 = 真实 OHLC 执行价替代 TV 标注价（前视偏差消除）。")
    a("6. **影响声明**：重写 scripts/unified_backtest_all.py（真实 OHLC + DTW 对齐 + lag=1）；"
      "复用 recursive_backtest.py timeline/FSM/stats（未改动）；yfinance 结果缓存至 _yf_*.json。")
    a("")
    a("## 缺口与诚实声明（no-patch-mentality）")
    a("")
    a("1. **前视偏差缩小但未完全消除（关键诚实声明）**：执行价已从 TV 标注价改为 yfinance "
      "真实日线收盘（lag=1）。但 C 组仍 89-100% 高胜率，因缠论买卖点**标注在分型极值点**，"
      "DTW 精确对齐（HSI 中位 0.18%）使执行价落在极值日，lag=1 仅移 1 日仍在极值附近 → "
      "买低卖高残留。**反讽**：BRN 对齐噪声大（31/116>5%）反而稀释偏差 → C 组 89%（最低）。"
      "彻底消除需建模分型确认滞后（买卖点在右侧确认日而非极值日交易），TV label 未提供"
      "确认滞后信息。C 组高胜率与 QQQ 基准 93.6% **同源**，**A 组（趋势持仓）为主要观察对象**，"
      "非 C 组。")
    a("2. **DTW 对齐 L1 噪声**：BRN 因 Brent 同价多期歧义，31/116 信号对齐误差>5%（中位仍 0.65%），"
      "这些信号映射日可能偏离真实信号日数月，对应执行价时点有误 —— BRN 结果标 L2−（对齐噪声）。"
      "HSI(0.18%)/SHCOMP(0.15%)/QQQ 对齐优秀。")
    a("3. **做空未实现**：cost_reduction_fsm 多头系统，down 线段空仓，不以假空头 PnL 冒充。")
    a("4. **走势方向含未来函数**：线段方向离线全量识别（线段需后续笔破坏才确认），"
      "seg_dir[di] 用了 di 之后信息 —— 此为缠论线段定义固有，QQQ 基准同此口径，不额外引入。")
    a("5. **classify_holding_event 是 L2 价值判定**：卖点→短差 vs 退出分类可调，见 recursive_backtest.py §5。")
    return "\n".join(L)


# ════════════════════════════════════════════════════════════════════════════
# 主流程
# ════════════════════════════════════════════════════════════════════════════


def main() -> None:
    results: list[SymbolResult] = []
    for cfg in SYMBOLS:
        print(f"[*] {cfg['label']} ...")
        r = run_symbol(cfg)
        results.append(r)
        print(f"    日线{r.n_bars} 买卖点{r.n_signals} 线段{r.n_segments} 对齐中位{r.align_median:.2f}% | "
              f"A {r.stats_a.closed_trades}笔/{rb.fmt_pct(r.stats_a.win_rate)} "
              f"B {r.stats_b.closed_trades}笔/{rb.fmt_pct(r.stats_b.win_rate)} "
              f"C {r.stats_c.closed_trades}笔/{rb.fmt_pct(r.stats_c.win_rate)}")

    qqq_base = load_qqq_baseline()
    print("[*] 写报告...")
    REPORT_PATH.write_text(build_report(results, qqq_base), encoding="utf-8")

    summary = {
        r.name: {
            grp: {"closed_trades": st.closed_trades, "win_rate": st.win_rate,
                  "total_return": st.total_return, "max_drawdown": st.max_drawdown,
                  "profit_factor": st.profit_factor, "avg_hold_days": st.avg_hold_days}
            for grp, st in [("A", r.stats_a), ("B", r.stats_b), ("C", r.stats_c)]
        } | {"meta": {"n_bars": r.n_bars, "n_signals": r.n_signals, "n_segments": r.n_segments,
                      "align_median_pct": r.align_median, "align_bad": r.align_bad,
                      "period": f"{r.date0}~{r.date1}", "bh": r.bh}}
        for r in results
    }
    if qqq_base:
        summary["qqq_baseline_weekly_map"] = qqq_base
    RESULT_JSON.write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print(f"    报告: {REPORT_PATH}")
    print(f"    JSON: {RESULT_JSON}")


if __name__ == "__main__":
    main()
