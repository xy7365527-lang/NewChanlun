"""K4 C 路径（六边同步分析）→ M1 背驰定位器（E）端到端回测。

与 M2 路径（``m2_e2e_backtest.py``，极性 S 压缩）的对照实验：

  aligned ES/GC/CL 1min（公共时间戳）
      │ 构造 6 条边：主边 ES/$,GC/$,CL/$ + 派生边 ES/GC,ES/CL,GC/CL（比值）
      │ 各边 60min 聚合 + RecursiveOrchestrator（level-0 σ + 最近买卖点）
      ▼
  6 × EdgeReading(date)   ←── 六边各自独立读数，不压缩成 S
      │ k4_path_c.closure_consensus  圈闭合多数表决纠错
      │ k4_path_c.joint_reading(EQUITY)  联合读数 → favored 方向 + score
      ▼
  C 路径逐日 equity favored 方向（无前视，滞后一日）
      │ favored==UP → 放行做多入场；否则回避（长多引擎对 DOWN/FLAT 的唯一投射）
      ▼
  E 版本（背驰定位器，run_swing_trading MODE_NONE，纯多头）
      │ E+C = 在 C 路径 equity favored≠UP 的日掩码入场触发
      ▼
  对比表：E（无方向输入） vs E+C（C 路径联合读数决定方向） vs Buy&Hold

═══ 认识论标注（formalization-validity-domain 规则）═══
  - 圈闭合纠错机制（closure_consensus）：L0（K4 图结构的纯代数推论，零信息增量）。
  - 单边 σ / 买卖点读数：L2（真实数据，缠论引擎产出）。
  - 管线连通性（本脚本跑通、各层对接）：L1。
  - E vs E+C 真实收益对比：L2（可产生否定性结果）。
  - C 路径**不走 ω 压缩**，直接读 ES-incident 边——是对 project_omega_regime_falsified
    （ω→美股方向被反向证伪）的结构性回应。若 E+C 仍逊于 E，缩小的是"六边联合读数对
    单股票择时有价值"这一假设的有效域，不得包装为 C 路径"有效"。

═══ 无前视保证 ═══
  date X 的盘中 bar 只使用 date < X（严格早于）的 C 路径 favored（滞后一日）。

═══ 边界条件 ═══
  - 6 条边建立在 ES/GC/CL 的**公共时间戳交集**上（736k 1min），保证圈闭合恒等式
    在价格层精确成立（σ 离散符号层的矛盾才是纠错信号）。
  - 派生边 OHLC 用比值包络：h=numH/denL, l=numL/denH（单 bar 内比值的严格上下界）。
  - OKLO 是动产（EQUITY 顶点）→ 联合读数 target=EQUITY，聚合 ES/$, ES/GC, ES/CL 三条
    ES-incident 边。
  - 门控仅作用于入场，不强制平仓（E 引擎自身 L2 趋势顶背驰离场）。

谱系引用：project_omega_regime_falsified、project_divergence_locator_entry_exit、
project_config_sigma_ontology（527号）、project_backtest_benchmark_falsifiability、
526号（a0 递归本体论区分）。
"""

from __future__ import annotations

import bisect
import json
import sys
import time
from dataclasses import dataclass, replace
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.orchestrator.recursive import (  # noqa: E402
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.strategy.k4_path_c import (  # noqa: E402
    ALL_EDGES,
    EDGE_ES_USD,
    CEdge,
    CVertex,
    EdgeReading,
    EdgeReadingLike,
    LeveledEdgeReading,
    joint_reading,
    rank_assets,
)
from newchan.topology.config_space import WalkDirection  # noqa: E402
from newchan.topology.k4_scanner import walk_direction_from_snapshot  # noqa: E402
from newchan.types import Bar  # noqa: E402

import fugue_alpha_diagnosis as ef  # noqa: E402,F401
from fugue_alpha_diagnosis import (  # noqa: E402
    MODE_NONE,
    BarSignal,
    compute_metrics,
    compute_signals,
    run_swing_trading,
)

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "m2_path_c_multilevel_results.md"
OUTPUT_JSON = DATA_DIR / "m2_path_c_multilevel_results.json"

# 三个资产顶点的连续期货（dated）。
ASSET_FILES = {
    CVertex.EQUITY: DATA_DIR / "es_1m_databento.json",
    CVertex.GOLD: DATA_DIR / "gc_1m_databento.json",
    CVertex.OIL: DATA_DIR / "cl_1m_databento.json",
}

# 交易标的（带时间戳 1min；bars[].ts 格式）。OKLO 是动产 → EQUITY 顶点。
SYMBOL_FILE = DATA_DIR / "oklo_1m_databento.json"
SYMBOL_NAME = "OKLO"
SYMBOL_TARGET = CVertex.EQUITY

# K4 边的工作周期（分钟）。**多级别递归必须用 5min**——实测（ES 1.75 年）：
#   TF=60min/30min：走势(move)层几乎不 settle（1.75 年仅 3 个 move settle），
#                    L2 PH 永不翻转（l2_direction 恒 0），L1 背驰买卖点≈0 → 多级别退化。
#   TF=15min：边际（l2≠0 占 23%，但底背驰买点仅 3 个）。
#   TF=5min：走势层富集（move settle 23/32，l2≠0 占 25%，底/顶背驰 17/19）→ 完整
#            L0/L1/L2 级别体系成立（任务 directive #2）。
# 单级别与多级别同 TF 跑（控制变量），隔离差异为"读数级别粒度"而非 TF。
TF_MINUTES = 5


# ════════════════════════════════════════════════════════════
# 1) 公共时间戳对齐 + 6 条边 60min OHLC 构造
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class _Series:
    """单标的 1min OHLC（按 dates 索引）。"""

    o: dict[str, float]
    h: dict[str, float]
    lo: dict[str, float]
    c: dict[str, float]


def _load_asset(path: Path) -> _Series:
    d = json.loads(path.read_text())
    dates = d["dates"]
    o = {ts: float(v) for ts, v in zip(dates, d["opens"])}
    h = {ts: float(v) for ts, v in zip(dates, d["highs"])}
    lo = {ts: float(v) for ts, v in zip(dates, d["lows"])}
    c = {ts: float(v) for ts, v in zip(dates, d["closes"])}
    return _Series(o=o, h=h, lo=lo, c=c)


# 60min bar：(day_str, o, h, l, c)
Bar60 = tuple[str, float, float, float, float]


def _edge_1min_ohlc(
    edge: CEdge, es: _Series, gc: _Series, cl: _Series, ts: str,
) -> tuple[float, float, float, float]:
    """单条边在时间戳 ts 的**完整 OHLC** K 线（非 close/close 单比值）。

    主边 = 对应资产原生 OHLC；派生边 = 比值包络：
        Open  = num_O / den_O
        High  = num_H / den_L   （比值上界：分子高 / 分母低）
        Low   = num_L / den_H   （比值下界：分子低 / 分母高）
        Close = num_C / den_C

    **为何必须构造完整 OHLC 而非只用 close/close**：缠论笔的端点是**分型**
    （顶分型=局部高点、底分型=局部低点），分型识别依赖 K 线的 high/low。只用
    close/close 比值会丢失单 bar 内的高低信息 → 笔/线段/走势全链无法正确分型。
    包络 (h=numH/denL, l=numL/denH) 是宽松估计（真实比值极值落在 [L, H] 内，因
    分子分母极值未必同时刻出现），但**保证不丢失高低信息**（H ≥ 实际比值极大、
    L ≤ 实际比值极小），故分型不会漏判。验证：H≥max(O,C) ∧ L≤min(O,C) 恒成立。
    """
    src = {CVertex.EQUITY: es, CVertex.GOLD: gc, CVertex.OIL: cl}
    if not edge.is_derived:
        s = src[edge.num]
        return s.o[ts], s.h[ts], s.lo[ts], s.c[ts]
    n = src[edge.num]
    dd = src[edge.den]
    return (
        n.o[ts] / dd.o[ts],
        n.h[ts] / dd.lo[ts],   # 比值上界：分子高 / 分母低
        n.lo[ts] / dd.h[ts],   # 比值下界：分子低 / 分母高
        n.c[ts] / dd.c[ts],
    )


def _bucket_key(ts: str) -> str:
    """TF_MINUTES 分钟桶键：'YYYY-MM-DD_<bucket>'（按日内分钟数整除 TF）。

    TF=60 时等价于按小时聚合（与旧行为一致）；TF=5 时每 5 分钟一桶。
    """
    hh = int(ts[11:13])
    mm = int(ts[14:16])
    return f"{ts[:10]}_{(hh * 60 + mm) // TF_MINUTES}"


def build_edge_bars() -> dict[CEdge, list[Bar60]]:
    """构造 6 条边的 TF_MINUTES-min bar 序列（公共时间戳交集，按时间升序）。"""
    t0 = time.time()
    es = _load_asset(ASSET_FILES[CVertex.EQUITY])
    gc = _load_asset(ASSET_FILES[CVertex.GOLD])
    cl = _load_asset(ASSET_FILES[CVertex.OIL])
    common = sorted(set(es.c) & set(gc.c) & set(cl.c))
    print(f"  公共时间戳 {len(common):,}（{common[0]} → {common[-1]}），"
          f"加载 {time.time() - t0:.1f}s")

    # 每条边：逐 1min 计算 OHLC，按 TF_MINUTES 分钟桶聚合
    acc: dict[CEdge, list[Bar60]] = {e: [] for e in ALL_EDGES}
    cur_key: str | None = None
    cur: dict[CEdge, list[float]] = {}
    cur_day = ""
    for ts in common:
        key = _bucket_key(ts)
        if key != cur_key:
            if cur:
                for e in ALL_EDGES:
                    v = cur[e]
                    acc[e].append((cur_day, v[0], v[1], v[2], v[3]))
            cur_key = key
            cur_day = ts[:10]
            cur = {}
            for e in ALL_EDGES:
                o, h, lo, c = _edge_1min_ohlc(e, es, gc, cl, ts)
                cur[e] = [o, h, lo, c]
        else:
            for e in ALL_EDGES:
                o, h, lo, c = _edge_1min_ohlc(e, es, gc, cl, ts)
                v = cur[e]
                v[1] = max(v[1], h)
                v[2] = min(v[2], lo)
                v[3] = c
    if cur:
        for e in ALL_EDGES:
            v = cur[e]
            acc[e].append((cur_day, v[0], v[1], v[2], v[3]))
    print(f"  6 边 {TF_MINUTES}min 聚合完成，每边 {len(acc[EDGE_ES_USD]):,} bar")
    return acc


# ════════════════════════════════════════════════════════════
# 2) 各边逐 bar 跑引擎 → 逐日 EdgeReading（σ + 最近买卖点，无前视）
# ════════════════════════════════════════════════════════════


def _latest_bsp(snap: RecursiveOrchestratorSnapshot) -> tuple:
    """从快照提取最近的买卖点 (side, kind, confirmed)。无则 (None, None, False)。

    取 seg_idx 最大（最近）的买卖点；confirmed 优先于 candidate（同 seg_idx 时）。
    """
    bsps = snap.bsp_snapshot.buysellpoints
    if not bsps:
        return (None, None, False)
    best = max(bsps, key=lambda b: (b.seg_idx, 1 if b.confirmed else 0))
    return (best.side, best.kind, best.confirmed)


def _edge_reading_by_date(
    edge: CEdge, bars: list[Bar60],
) -> dict[str, EdgeReading]:
    """流式跑引擎，返回 date_str → 该日收盘时的 EdgeReading（无前视）。"""
    orch = RecursiveOrchestrator(stream_id=f"c_{edge.label}", max_levels=2)
    base = datetime(2020, 1, 1)
    by_date: dict[str, EdgeReading] = {}
    for j, (day, oo, hh, ll, cc) in enumerate(bars):
        bar = Bar(ts=base + timedelta(hours=j),
                  open=oo, high=hh, low=ll, close=cc, volume=0.0)
        snap = orch.process_bar(bar)
        direction = walk_direction_from_snapshot(snap, level=0)
        side, kind, confirmed = _latest_bsp(snap)
        by_date[day] = EdgeReading(
            edge=edge, direction=direction,
            bsp_side=side, bsp_kind=kind, bsp_confirmed=confirmed,
        )
    return by_date


def _leveled_edge_reading_by_date(
    edge: CEdge, bars: list[Bar60],
) -> dict[str, LeveledEdgeReading]:
    """多级别读数：复用 E 版本 compute_signals 跑该边 60min OHLC → 按日聚合。

    级别对齐（任务要求）：
    - **方向取最高级别 L2**：``BarSignal.l2_direction`` = L2 PH 翻转累积态。
    - **买卖点取操作级别 L1**：L1 走势级背驰买卖点——底背驰
      （``down_move_settled ∧ entry_div_ok``）= buy；顶背驰
      （``up_move_settled ∧ exit_div_ok``）= sell。背驰确认（37 课 MACD 面积/价格）
      已内化在 entry/exit_div_ok。

    日内聚合：同一天的最后一个 60min bar 决定该日 EOD 读数；``l1_bsp_side``
    carry-forward 最近一个背驰买卖点（与单级别 ``_latest_bsp`` 取最近一致）。
    """
    o = [b[1] for b in bars]
    h = [b[2] for b in bars]
    lo = [b[3] for b in bars]
    c = [b[4] for b in bars]
    sigs = compute_signals(o, h, lo, c)

    by_date: dict[str, LeveledEdgeReading] = {}
    last_bsp: str | None = None  # carry-forward 最近 L1 背驰买卖点
    for j, b in enumerate(bars):
        sg = sigs[j]
        if sg.down_move_settled and sg.entry_div_ok:
            last_bsp = "buy"
        elif sg.up_move_settled and sg.exit_div_ok:
            last_bsp = "sell"
        l2 = sg.l2_direction
        direction = (
            WalkDirection.UP if l2 > 0
            else WalkDirection.DOWN if l2 < 0
            else WalkDirection.FLAT
        )
        by_date[b[0]] = LeveledEdgeReading(
            edge=edge, l2_direction=direction,
            l1_bsp_side=last_bsp, l1_div_confirmed=last_bsp is not None,
        )
    return by_date


Timeline = list[tuple[str, dict[CEdge, EdgeReadingLike]]]


def _build_timeline(
    edge_bars: dict[CEdge, list[Bar60]],
    reader_fn,
    default_factory,
    tag: str,
) -> Timeline:
    """通用逐日六边读数时间线构造（单/多级别共享）。

    每条边 carry-forward 对齐（缺日沿用最近已知读数）。
    """
    per_edge: dict[CEdge, dict[str, EdgeReadingLike]] = {}
    for e in ALL_EDGES:
        t0 = time.time()
        per_edge[e] = reader_fn(e, edge_bars[e])
        print(f"  [{tag}] {e.label:8s} 完成（{time.time() - t0:.1f}s）")

    all_dates = sorted(set().union(*[set(per_edge[e]) for e in ALL_EDGES]))
    carried: dict[CEdge, dict[str, EdgeReadingLike]] = {}
    for e in ALL_EDGES:
        out: dict[str, EdgeReadingLike] = {}
        last: EdgeReadingLike = default_factory(e)
        for dd in all_dates:
            if dd in per_edge[e]:
                last = per_edge[e][dd]
            out[dd] = last
        carried[e] = out
    return [(dd, {e: carried[e][dd] for e in ALL_EDGES}) for dd in all_dates]


def build_reading_timeline(edge_bars: dict[CEdge, list[Bar60]]) -> Timeline:
    """单级别时间线（level-0 走势方向 + 最近买卖点）。"""
    return _build_timeline(
        edge_bars, _edge_reading_by_date,
        lambda e: EdgeReading(edge=e, direction=WalkDirection.FLAT,
                              bsp_side=None, bsp_kind=None, bsp_confirmed=False),
        "单级别",
    )


def build_leveled_reading_timeline(edge_bars: dict[CEdge, list[Bar60]]) -> Timeline:
    """多级别时间线（L2 方向 + L1 操作级背驰买卖点，与 E 版本进出场同构）。"""
    return _build_timeline(
        edge_bars, _leveled_edge_reading_by_date,
        lambda e: LeveledEdgeReading(edge=e, l2_direction=WalkDirection.FLAT,
                                     l1_bsp_side=None, l1_div_confirmed=False),
        "多级别",
    )


# ════════════════════════════════════════════════════════════
# 3) 无前视 C 路径 equity favored 查询 + 逐日做多放行表
# ════════════════════════════════════════════════════════════


def allow_long_by_date(
    bar_dates: list[str],
    timeline: Timeline,
) -> dict[str, bool]:
    """逐唯一交易日：C 路径 equity 联合读数 favored==UP 时放行做多（无前视，滞后一日）。

    date X 使用 timeline 中 date < X 的最近读数。X 早于时间线起点 → 默认放行。
    """
    tl_dates = [d for d, _ in timeline]
    cache: dict[str, bool] = {}
    for dd in dict.fromkeys(bar_dates):
        idx = bisect.bisect_left(tl_dates, dd) - 1
        if idx < 0:
            cache[dd] = True
            continue
        readings = timeline[idx][1]
        jr = joint_reading(readings, SYMBOL_TARGET)
        cache[dd] = jr.favored is WalkDirection.UP
    return cache


# ════════════════════════════════════════════════════════════
# 4) OKLO 数据加载 + 信号磁带门控（复用 fugue E 引擎）
# ════════════════════════════════════════════════════════════


def load_symbol_dated(
    path: Path,
) -> tuple[list[float], list[float], list[float], list[float], list[str]]:
    d = json.loads(path.read_text())
    bars = d["bars"]
    opens = [float(b["open"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    closes = [float(b["close"]) for b in bars]
    bdates = [str(b["ts"])[:10] for b in bars]
    return opens, highs, lows, closes, bdates


def gate_signals(
    signals: list[BarSignal],
    bar_dates: list[str],
    allow_map: dict[str, bool],
) -> tuple[list[BarSignal], int]:
    """E+C 信号磁带：C 路径 equity favored≠UP 的日，掩码该 bar 入场触发。

    与 m2_e2e 同机制：MODE_NONE 下 down_move_settled 仅用于 FLAT→LONG 入场判定，
    掩码它精确等价于"该日不开新多仓"，不触碰离场/持仓逻辑。返回 (门控后磁带, 掩码数)。
    """
    masked = 0
    out: list[BarSignal] = []
    for i, sig in enumerate(signals):
        if not allow_map.get(bar_dates[i], True) and sig.down_move_settled:
            out.append(replace(sig, down_move_settled=False))
            masked += 1
        else:
            out.append(sig)
    return out, masked


def buy_and_hold_pct(closes: list[float]) -> float:
    if len(closes) < 2 or closes[0] <= 0:
        return 0.0
    return (closes[-1] / closes[0] - 1.0) * 100.0


# ════════════════════════════════════════════════════════════
# 5) 回测
# ════════════════════════════════════════════════════════════


def _run_gated_variant(
    signals: list[BarSignal],
    bdates: list[str],
    timeline: Timeline,
) -> tuple[dict, int, int]:
    """对一条时间线跑 E+C 门控变体，返回 (metrics, 拦截日数, 掩码入场数)。"""
    allow_map = allow_long_by_date(bdates, timeline)
    n_block_days = sum(1 for v in allow_map.values() if not v)
    gated, masked = gate_signals(signals, bdates, allow_map)
    trades, _ = run_swing_trading(gated, MODE_NONE)
    return compute_metrics(trades), n_block_days, masked


def backtest(
    timeline_single: Timeline,
    timeline_multi: Timeline,
) -> dict:
    """三栏对比：E（无方向）vs E+C 单级别 vs E+C 多级别递归。"""
    print(f"\n── {SYMBOL_NAME} ──")
    opens, highs, lows, closes, bdates = load_symbol_dated(SYMBOL_FILE)
    n = len(closes)
    print(f"  bars={n:,}  日期 {bdates[0]} → {bdates[-1]}")

    t0 = time.time()
    signals = compute_signals(opens, highs, lows, closes)
    print(f"  信号计算完成（{time.time() - t0:.1f}s）")

    # E：无方向输入，纯背驰定位器（基线）
    trades_e, _ = run_swing_trading(signals, MODE_NONE)
    m_e = compute_metrics(trades_e)

    # E+C 单级别（level-0 方向 + 最近买卖点）
    m_cs, block_s, masked_s = _run_gated_variant(signals, bdates, timeline_single)
    # E+C 多级别（L2 方向 + L1 操作级背驰买卖点）
    m_cm, block_m, masked_m = _run_gated_variant(signals, bdates, timeline_multi)

    n_days = len({d for d in bdates})
    bh = buy_and_hold_pct(closes)

    print(f"  交易日={n_days}")
    print(f"  E              : n={m_e['n']:3d}  收益={m_e['total_compound']:+9.2f}%  "
          f"胜率={m_e['win_rate']:.1f}%")
    print(f"  E+C 单级别      : n={m_cs['n']:3d}  收益={m_cs['total_compound']:+9.2f}%  "
          f"胜率={m_cs['win_rate']:.1f}%  拦截日={block_s} 掩码={masked_s}")
    print(f"  E+C 多级别递归  : n={m_cm['n']:3d}  收益={m_cm['total_compound']:+9.2f}%  "
          f"胜率={m_cm['win_rate']:.1f}%  拦截日={block_m} 掩码={masked_m}")
    print(f"  Buy&Hold: {bh:+.2f}%")

    return {
        "symbol": SYMBOL_NAME,
        "bars": n,
        "date_start": bdates[0],
        "date_end": bdates[-1],
        "unique_days": n_days,
        "buy_hold_pct": round(bh, 4),
        "E": m_e,
        "E_C_single": m_cs,
        "E_C_multi": m_cm,
        "blocked_days_single": block_s,
        "masked_entries_single": masked_s,
        "blocked_days_multi": block_m,
        "masked_entries_multi": masked_m,
    }


# ════════════════════════════════════════════════════════════
# 6) C 路径诊断（纠错量 + 排序分布）
# ════════════════════════════════════════════════════════════


def diagnose_timeline(
    timeline: Timeline,
) -> dict:
    """统计圈闭合纠错量与 equity favored / 资产排序分布（单/多级别通用）。"""
    from collections import Counter

    dissent_total = 0
    dissent_days = 0
    fav_dist: Counter = Counter()
    top_asset_dist: Counter = Counter()
    for _, readings in timeline:
        jr = joint_reading(readings, SYMBOL_TARGET)
        fav_dist[jr.favored.name] += 1
        if jr.total_dissent > 0:
            dissent_days += 1
            dissent_total += jr.total_dissent
        ranked = rank_assets(readings)
        top_asset_dist[ranked[0].target.value] += 1
    return {
        "days": len(timeline),
        "dissent_days": dissent_days,
        "dissent_total": dissent_total,
        "favored_dist": dict(fav_dist),
        "top_asset_dist": dict(top_asset_dist),
    }


# ════════════════════════════════════════════════════════════
# 7) 报告
# ════════════════════════════════════════════════════════════


def _diag_block(L: list[str], title: str, diag: dict) -> None:
    L.append(f"### {title}")
    L.append("")
    L.append(f"- 圈闭合纠错：{diag['dissent_days']}/{diag['days']} 天检出读数矛盾"
             f"（dissent>0），累计纠错估计数 {diag['dissent_total']}。")
    L.append(f"- equity favored 方向分布：{diag['favored_dist']}")
    L.append(f"- 最强做多资产排序首位分布：{diag['top_asset_dist']}")
    L.append("")


def write_report(result: dict, diag_single: dict, diag_multi: dict,
                 timeline_multi: Timeline) -> None:
    L: list[str] = []
    L.append("# K4 C 路径多级别递归 → E 背驰定位器端到端回测\n")
    L.append(f"工作周期：{TF_MINUTES}min 聚合期货。**单级别** = level-0 走势方向 + 最近"
             "买卖点；**多级别递归** = L2 PH 最高级别方向 + L1 操作级背驰买卖点"
             "（与 E 版本进出场同构）。")
    L.append(f"K4 时间线：{len(timeline_multi)} 天"
             f"（{timeline_multi[0][0]} → {timeline_multi[-1][0]}）。\n")

    L.append("## C 路径诊断（圈闭合纠错 + 排序分布）")
    L.append("")
    _diag_block(L, "单级别（level-0 σ）", diag_single)
    _diag_block(L, "多级别递归（L2 σ）", diag_multi)
    L.append("> dissent>0 = 某派生边直接读数被另外两条估计路径（过现金/过第三资产）"
             "否决——六边冗余相对三边代数推导的检错增益。多级别用 L2 σ 做圈闭合"
             "（任务要求级别对齐，不混级别）。")
    L.append("")

    L.append("## 对比表：E vs E+C(单级别) vs E+C(多级别递归)")
    L.append("")
    L.append("| 版本 | 交易数 | 复利收益% | 胜率% | Sharpe | MaxDD% | 相对 E 留存 |")
    L.append("|------|--------|-----------|-------|--------|--------|-------------|")
    e_ret = result["E"]["total_compound"]
    rows = (
        ("E（无方向输入，纯背驰基线）", "E"),
        ("E+C 单级别（level-0 方向+最近买卖点）", "E_C_single"),
        ("E+C 多级别递归（L2 方向+L1 背驰买卖点）", "E_C_multi"),
    )
    for ver, key in rows:
        m = result[key]
        retain = "100%" if key == "E" else f"{m['total_compound'] / e_ret * 100:.0f}%"
        L.append(
            f"| {ver} | {m['n']} | {m['total_compound']:+.2f} | {m['win_rate']:.1f} | "
            f"{m['sharpe']:.3f} | {m['max_dd']:.2f} | {retain} |"
        )
    L.append(
        f"| Buy&Hold | — | {result['buy_hold_pct']:+.2f} | — | — | — | — |"
    )
    L.append("")

    # 多级别 vs 单级别的核心判定
    cs = result["E_C_single"]["total_compound"]
    cm = result["E_C_multi"]["total_compound"]
    verdict = (
        "多级别递归 **优于** 单级别" if cm > cs
        else "多级别递归 **劣于** 单级别" if cm < cs
        else "多级别递归与单级别**持平**"
    )
    L.append(f"> **核心判定（L2）**：{verdict}"
             f"（多级别 {cm:+.2f}% vs 单级别 {cs:+.2f}%）。")
    L.append("")

    L.append("## 门控统计")
    L.append("")
    L.append("| 版本 | bars | 起 | 止 | 交易日 | 拦截日 | 掩码入场 |")
    L.append("|------|------|----|----|--------|--------|----------|")
    L.append(
        f"| 单级别 | {result['bars']} | {result['date_start']} | "
        f"{result['date_end']} | {result['unique_days']} | "
        f"{result['blocked_days_single']} | {result['masked_entries_single']} |"
    )
    L.append(
        f"| 多级别 | {result['bars']} | {result['date_start']} | "
        f"{result['date_end']} | {result['unique_days']} | "
        f"{result['blocked_days_multi']} | {result['masked_entries_multi']} |"
    )
    L.append("")

    L.append("## 结果包六要素")
    L.append("")
    L.append("**结论**：见上述对比表（E vs E+C 单级别 vs E+C 多级别递归）+ 双诊断。")
    L.append("")
    L.append("**定义依据**：")
    L.append("- 六条边 = 主边 ES/$,GC/$,CL/$（生成树）+ 派生边 ES/GC,ES/CL,GC/CL，"
             "各自独立跑引擎（compute-once 缓存，每边算一次）。")
    L.append("- **单级别** σ = level-0 settled-move 方向；买卖点 = type1/2/3。")
    L.append("- **多级别** σ = L2 PH 翻转累积态（最高级别走势方向，walk 取最高级）；"
             "L1 买卖点 = 操作级走势背驰（底背驰买 down_move_settled∧entry_div_ok / "
             "顶背驰卖 up_move_settled∧exit_div_ok，37 课 MACD 面积背驰确认内化）。")
    L.append("- 圈闭合纠错 = K4 每资产对 3 条估计路径多数表决"
             "（k4_path_c.closure_consensus，**同级别 σ 闭合，不混级别**——单级别用"
             "level-0 σ、多级别用 L2 σ，由 EdgeReadingLike.sigma 协议保证）。")
    L.append("- 联合读数 = ES-incident 边方向票+买卖点票之和"
             "（k4_path_c.joint_reading，target=EQUITY，OKLO 是动产）。")
    L.append("- E = run_swing_trading(MODE_NONE)：底背驰买入、L2 趋势顶背驰清仓，纯多头。")
    L.append("")
    L.append("**边界条件**：")
    L.append("- 六边建立在 ES/GC/CL 公共时间戳交集上，圈闭合恒等式价格层精确成立。")
    L.append("- 多级别方向票/买卖点票权重 W_DIRECTION : W_L1_DIVERGENCE_BSP（默认 1:1）"
             "是 [新缠论:选择] 可调常数——比值>1 买卖点主导（反转择时）、<1 方向主导"
             "（趋势跟随）。改比值移动 favored 临界 → 翻转门控结果。")
    L.append("- 门控仅作用于入场，不强制平仓（E 引擎自身 L2 顶背驰离场）。")
    L.append("- 无前视：date X 仅用 date<X 的 C 路径 favored（滞后一日）。")
    L.append("")
    L.append("**下游推论**（本次 OKLO 运行的**确定结论**，非条件假设）：")
    em = result["E"]["total_compound"]
    if cm < cs:
        L.append(f"- **多级别递归劣于单级别**（{cm:+.2f}% vs {cs:+.2f}%，留存"
                 f"{cm / em * 100:.0f}% vs {cs / em * 100:.0f}%）：级别细化（L2 方向"
                 f"+L1 背驰买卖点）在 OKLO 强上行单股票上**收缩了有效域**——L2 方向"
                 f"latch 更保守（多级别 favored FLAT {diag_multi['favored_dist'].get('FLAT', 0)} 天"
                 f" vs 单级别 {diag_single['favored_dist'].get('FLAT', 0)} 天），拦截更多"
                 f"（{result['blocked_days_multi']} vs {result['blocked_days_single']} 日）"
                 f"→ 入场更少（{result['E_C_multi']['n']} vs {result['E_C_single']['n']} 笔）"
                 f"→ 牛市削减暴露更狠。**不得包装为多级别'有效'**"
                 "（formalization-validity-domain）。")
    elif cm > cs:
        L.append(f"- **多级别递归优于单级别**（{cm:+.2f}% vs {cs:+.2f}%）：级别感知有"
                 "信息增益——但仍需 L3 多标的/多时段交叉验证才可声明鲁棒（单标的非充分）。")
    else:
        L.append(f"- 多级别与单级别持平（{cm:+.2f}%）：级别细化对门控择时中性。")
    if cs < em and cm < em:
        L.append(f"- **两路径均 < E 基线**（{em:+.2f}%）：方向门控本身在牛市削减暴露"
                 "——确认 project_divergence_gate_entry_exit_falsified（门控砍入场=减暴露"
                 "=强趋势中跑输），级别细化不改变该否定方向，只改变削减幅度。")
    L.append("- 多级别在 60min/30min TF 下退化为空有效域（走势层不 settle、L2 PH 不翻转）；"
             "本结论仅在 TF=5min（走势层富集）成立——级别体系的成立依赖足够细的工作周期。")
    L.append("")
    L.append("**谱系引用**：project_omega_regime_falsified（C 路径不走 ω 压缩）、"
             "project_divergence_locator_entry_exit（E 版本进出场，多级别与之同构）、"
             "527号（config σ 走势方向态）、project_backtest_benchmark_falsifiability、"
             "526号（a0 递归本体论）。")
    L.append("")
    L.append("**影响声明**：k4_path_c.py 增 EdgeReadingLike 协议 + LeveledEdgeReading"
             "（圈闭合/联合读数泛化为级别对齐，单级别零行为变更）；本回测脚本增多级别"
             "时间线 + 三栏对比。不修改引擎/k4_integration/config_space/fugue。")
    L.append("")
    L.append("**认识论等级**：圈闭合纠错机制 L0；单/多级别单边读数 L2；管线连通 L1；"
             "三栏收益对比 L2（单标的，可证伪；非 L3 交叉验证）。")
    L.append("")

    OUTPUT_MD.write_text("\n".join(L))
    print(f"\n报告 → {OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主入口
# ════════════════════════════════════════════════════════════


def main() -> None:
    print("═" * 64)
    print("  K4 C 路径多级别递归回测：六边同步分析 → E 背驰定位器")
    print("═" * 64)

    print("\n[1] 构造六边 60min bar（ES/GC/CL 公共时间戳，compute-once 共享）")
    edge_bars = build_edge_bars()

    print("\n[2] 单级别时间线（level-0 σ + 最近买卖点）")
    timeline_single = build_reading_timeline(edge_bars)
    print("\n[3] 多级别递归时间线（L2 σ + L1 操作级背驰买卖点）")
    timeline_multi = build_leveled_reading_timeline(edge_bars)

    print("\n[4] C 路径诊断（圈闭合纠错 + 排序分布）")
    diag_single = diagnose_timeline(timeline_single)
    diag_multi = diagnose_timeline(timeline_multi)
    print(f"  单级别：纠错日={diag_single['dissent_days']}/{diag_single['days']}  "
          f"favored={diag_single['favored_dist']}")
    print(f"  多级别：纠错日={diag_multi['dissent_days']}/{diag_multi['days']}  "
          f"favored={diag_multi['favored_dist']}")

    print("\n[5] OKLO 回测：E vs E+C(单级别) vs E+C(多级别递归)")
    result = backtest(timeline_single, timeline_multi)

    write_report(result, diag_single, diag_multi, timeline_multi)
    OUTPUT_JSON.write_text(json.dumps(
        {"result": result, "diag_single": diag_single, "diag_multi": diag_multi},
        indent=2, ensure_ascii=False))
    print(f"结果 JSON → {OUTPUT_JSON}")
    print("\n" + "═" * 64)
    print("  完成")
    print("═" * 64)


if __name__ == "__main__":
    main()
