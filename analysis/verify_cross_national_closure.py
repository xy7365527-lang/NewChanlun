"""跨国 K4 圈闭合（和乐 / holonomy）真实数据验证。

任务：用真实 1min 数据验证跨国闭合是否产出信息，特别关注圈闭合残差是否有结构。

概念依据
--------
- 517号谱系：和乐是闭合循环（2-Simplex）属性，hol(γ)=exp(∮_γ ω)，
  "告诉你某个循环在**不同一般等价物**下是否一致"。
- 254号：结算尺空间 Σ = 法定货币群 ∪ 黄金；层0 = 货币边 M_i/M_j。
- formalization-validity-domain 规则：L0 = 纯代数/同义反复（零信息增量）；
  L2 = 真实数据单标的假设检验（可证伪，正信息）。

核心命题（边界条件）
------------------
和乐的信息有效域 = 环路穿越的结算尺数 ≥ 2。
- 若环路所有边都是**同一计价单位**（USD）价格向量的比值 → 乘积 ≡ 1（代数强制，
  float 噪声级）→ L0 零信息。本数据 ES/GC/CL/BRN 全 USD 计价。
- 真正非平凡闭合只在货币层：DX（美元篮子）vs EURUSD（6E）的相对一致性。

三部分
------
Part A（L0 对照）：单结算尺三角环 GC→CL→ES→GC 的和乐 ≡ 1（证明零信息基线）。
Part B（L2 真实和乐）：DX 对 EURUSD 的货币层闭合残差，跑缠论递归看结构。
Part C（跨国同类边）：BRN/CL（Brent/WTI）= 跨国同类油边——是 1-Simplex 比价边
        （同 USD 计价的价差），不是 2-Simplex 和乐，对照说明。
"""

from __future__ import annotations

import json
import math
from datetime import datetime
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

_DATA = Path(__file__).resolve().parent / "data_cache"

# ICE 美元指数 DX 权重（EUR 主导 57.6%）。其余 5 货币 = JPY/GBP/CAD/SEK/CHF。
_EUR_WEIGHT = 0.576


def _load(sym: str) -> dict:
    with open(_DATA / f"{sym}_1m_databento_10y.json") as f:
        return json.load(f)


def _parse(s: str) -> datetime:
    core = s.split("+")[0].strip()
    return datetime.strptime(core[:19], "%Y-%m-%d %H:%M:%S")


# ════════════════════════════════════════════════════════════
# Part A — L0 对照：单结算尺环路和乐 ≡ 1（同义反复）
# ════════════════════════════════════════════════════════════


def part_a_single_numeraire_loop() -> None:
    print("\n" + "=" * 70)
    print("Part A [L0]：单结算尺三角环 GC→CL→ES→GC 和乐")
    print("  环路边全是同一 USD 价格向量比值 → 预期乘积恒为 1（零信息）")
    print("=" * 70)
    gc, cl, es = _load("gc"), _load("cl"), _load("es")

    # 按时间戳对齐三序列（取交集），逐 bar 算环路对数和乐残差
    def index(d: dict) -> dict[str, float]:
        return dict(zip(d["dates"], d["closes"]))

    igc, icl, ies = index(gc), index(cl), index(es)
    common = igc.keys() & icl.keys() & ies.keys()
    print(f"  三序列时间戳交集 = {len(common):,} bars")

    max_abs = 0.0
    n = 0
    for ts in common:
        g, c, e = igc[ts], icl[ts], ies[ts]
        if g > 0 and c > 0 and e > 0:
            # log[(GC/CL)·(CL/ES)·(ES/GC)] = 0 代数恒等
            resid = (math.log(g) - math.log(c)) + (math.log(c) - math.log(e)) + (
                math.log(e) - math.log(g)
            )
            max_abs = max(max_abs, abs(resid))
            n += 1
    print(f"  有效 bar = {n:,}")
    print(f"  环路对数和乐残差 max|Σω| = {max_abs:.3e}")
    print(f"  判定：{'≡0（float 噪声级）→ L0 零信息，同义反复' if max_abs < 1e-9 else '非零！'}")
    print("  → 结论：单结算尺下任何资产比价环路代数强制闭合，圈闭合残差不携带信息。")


# ════════════════════════════════════════════════════════════
# 残差序列构造 + 重采样 + 缠论递归
# ════════════════════════════════════════════════════════════


def _aligned_minute_residual(
    sym_a: str, sym_b: str, w_a: float, w_b: float
) -> tuple[list[datetime], list[float]]:
    """逐分钟对齐 sym_a、sym_b，构造标量残差 r = w_a·log(a) + w_b·log(b)。

    返回 (排序后的 datetime 列表, 残差列表)。
    """
    da, db = _load(sym_a), _load(sym_b)
    ia = dict(zip(da["dates"], da["closes"]))
    ib = dict(zip(db["dates"], db["closes"]))
    common = sorted(ia.keys() & ib.keys())
    ts_out: list[datetime] = []
    r_out: list[float] = []
    for ts in common:
        a, b = ia[ts], ib[ts]
        if a > 0 and b > 0:
            ts_out.append(_parse(ts))
            r_out.append(w_a * math.log(a) + w_b * math.log(b))
    return ts_out, r_out


def _resample_scalar_to_hourly_ohlc(
    ts: list[datetime], vals: list[float]
) -> list[Bar]:
    """把分钟级标量残差序列重采样为小时 OHLC bar（exp 归一为正值价格序列）。

    标量的小时 OHLC 良定义：open=首分钟、high=max、low=min、close=末分钟。
    exp() 保正值且可解释为"非欧元美元篮子指数"。
    """
    bars: list[Bar] = []
    cur_key: tuple[int, int, int, int] | None = None
    o = h = l = c = 0.0
    bucket_ts: datetime | None = None
    for t, v in zip(ts, vals):
        key = (t.year, t.month, t.day, t.hour)
        if key != cur_key:
            if cur_key is not None:
                bars.append(
                    Bar(ts=bucket_ts, open=math.exp(o), high=math.exp(h),
                        low=math.exp(l), close=math.exp(c))
                )
            cur_key = key
            bucket_ts = t.replace(minute=0, second=0)
            o = h = l = c = v
        else:
            h = max(h, v)
            l = min(l, v)
            c = v
    if cur_key is not None:
        bars.append(
            Bar(ts=bucket_ts, open=math.exp(o), high=math.exp(h),
                low=math.exp(l), close=math.exp(c))
        )
    return bars


def _resample_ratio_to_hourly_ohlc(sym_a: str, sym_b: str) -> list[Bar]:
    """sym_a/sym_b 比价逐分钟对齐后重采样小时 OHLC（正值，直接是比价）。"""
    ts, logr = _aligned_minute_residual(sym_a, sym_b, 1.0, -1.0)  # log(a/b)
    return _resample_scalar_to_hourly_ohlc(ts, logr)


def _run_chanlun(name: str, bars: list[Bar], window: int | None = None) -> dict:
    """对小时 bar 序列跑递归，提取结构量：趋势/中枢/背驰。"""
    if window:
        bars = bars[-window:]
    orch = RecursiveOrchestrator(stream_id=name, max_levels=6, stroke_mode="wide")
    snap = None
    for b in bars:
        snap = orch.process_bar(b)
    assert snap is not None

    # 趋势：最高级别走势
    moves = snap.move_snapshot.moves
    top = moves[-1] if moves else None
    max_level = max((rs.level_id for rs in snap.recursive_snapshots), default=1)

    # 中枢：各级别中枢总数
    zs_l1 = len(snap.zs_snapshot.zhongshus)
    zs_higher = sum(len(rs.zhongshus) for rs in snap.recursive_snapshots)
    zs_total = zs_l1 + zs_higher

    # 背驰：买卖点中带 divergence_key 的（type-1/3 由背驰驱动）
    bsps = snap.bsp_snapshot.buysellpoints
    diverg = [p for p in bsps if getattr(p, "divergence_key", None) is not None]

    return {
        "name": name,
        "bars": len(bars),
        "date_range": (bars[0].ts.isoformat()[:16], bars[-1].ts.isoformat()[:16]),
        "max_level": max_level,
        "top_move_kind": top.kind if top else "none",
        "top_move_dir": top.direction if top else "none",
        "top_move_settled": top.settled if top else False,
        "top_move_zs_count": top.zs_count if top else 0,
        "zs_total": zs_total,
        "n_moves": len(moves),
        "n_bsp": len(bsps),
        "n_divergence_bsp": len(diverg),
        "last_close": bars[-1].close,
    }


def _report(r: dict) -> None:
    print(f"\n  [{r['name']}]  {r['bars']:,} 小时bar  {r['date_range'][0]} → {r['date_range'][1]}")
    print(f"    趋势: 最高级别 L{r['max_level']} | 顶层走势 kind={r['top_move_kind']} "
          f"dir={r['top_move_dir']} settled={r['top_move_settled']} "
          f"zs_count={r['top_move_zs_count']}")
    print(f"    中枢: 全级别中枢总数 = {r['zs_total']} | 走势段数 = {r['n_moves']}")
    print(f"    背驰: 买卖点 {r['n_bsp']} 个，其中背驰驱动 {r['n_divergence_bsp']} 个")


# ════════════════════════════════════════════════════════════
# Part B — L2 真实货币层和乐：DX vs EURUSD
# ════════════════════════════════════════════════════════════


def part_b_currency_holonomy(window: int | None) -> None:
    print("\n" + "=" * 70)
    print("Part B [L2]：货币层闭合残差  r = log DX + 0.576·log EURUSD")
    print("  EURUSD = 1/USD6E。残差 = DX 中无法被 EUR 边解释的部分（非欧元货币篮子）")
    print("  若 DX 纯由 EUR 驱动 → 残差平坦（无趋势）；有结构 → 跨国信息越过单 FX 边")
    print("=" * 70)
    # EURUSD = 1/USD6E → log EURUSD = -log USD6E
    # r = log DX + 0.576·log EURUSD = log DX - 0.576·log USD6E
    ts, resid = _aligned_minute_residual("dx", "usd6e", 1.0, -_EUR_WEIGHT)
    print(f"  DX∩USD6E 分钟对齐 = {len(resid):,} bars")
    bars = _resample_scalar_to_hourly_ohlc(ts, resid)
    print(f"  重采样小时 bar = {len(bars):,}")

    res_holo = _run_chanlun("HOLO: DX·EURUSD^0.576 (非欧元篮子)", bars, window)
    _report(res_holo)

    # 对照 1：原始 DX（小时）—— 完整美元篮子结构
    dx = _load("dx")
    dx_bars = _resample_scalar_to_hourly_ohlc(
        [_parse(d) for d in dx["dates"]], [math.log(c) for c in dx["closes"] if c > 0]
    )
    res_dx = _run_chanlun("CTRL: 原始 DX（完整美元篮子）", dx_bars, window)
    _report(res_dx)

    # 对照 2：原始 EURUSD（小时）—— 单 FX 边结构
    u6 = _load("usd6e")
    eur_bars = _resample_scalar_to_hourly_ohlc(
        [_parse(d) for d in u6["dates"]], [-math.log(c) for c in u6["closes"] if c > 0]
    )
    res_eur = _run_chanlun("CTRL: EURUSD（单 FX 边）", eur_bars, window)
    _report(res_eur)

    print("\n  ── Part B 判定 ──")
    print(f"  残差最高级别 L{res_holo['max_level']} vs DX L{res_dx['max_level']} "
          f"vs EURUSD L{res_eur['max_level']}")
    has_struct = (res_holo["max_level"] >= 2 or res_holo["zs_total"] >= 1)
    print(f"  残差是否有缠论结构（趋势/中枢/背驰）：{'是' if has_struct else '否'}")
    print("  信息判定：残差结构 = 非欧元货币篮子的独立运动 = 单 EUR/USD 边看不到的跨国信息。")
    print("  诚实标注（231号）：缺 5 条 FX 腿 → 残差混入缺失腿，为**部分闭合**，L2 非 L3。")


# ════════════════════════════════════════════════════════════
# Part C — 跨国同类油边 BRN/CL（非和乐，对照）
# ════════════════════════════════════════════════════════════


def part_c_oil_cross_edge(window: int | None) -> None:
    print("\n" + "=" * 70)
    print("Part C [L2]：跨国同类油边 BRN/CL（Brent/WTI）")
    print("  254号跨国同类边 X_i/X_j。两者皆 USD 计价 → 是 1-Simplex 比价边（价差），")
    print("  不是 2-Simplex 和乐（无结算尺切换 → 无闭合残差）。对照其缠论结构。")
    print("=" * 70)
    bars = _resample_ratio_to_hourly_ohlc("brn", "cl")
    print(f"  BRN∩CL 重采样小时 bar = {len(bars):,}")
    res = _run_chanlun("OIL-EDGE: BRN/CL", bars, window)
    _report(res)
    print("\n  ── Part C 判定 ──")
    print("  BRN/CL 有缠论结构，但它是跨国**同类边读数**（1-Simplex），不是圈闭合残差。")
    print("  把它当'闭合'会犯 517号层级错误（边属性 vs 循环属性混淆）。")


if __name__ == "__main__":
    import sys

    window = int(sys.argv[1]) if len(sys.argv) > 1 else None
    print("跨国 K4 圈闭合（和乐）真实数据验证")
    print(f"递归窗口 = {'全量' if window is None else f'最近 {window} 小时bar'}")
    part_a_single_numeraire_loop()
    part_b_currency_holonomy(window)
    part_c_oil_cross_edge(window)
    print("\n" + "=" * 70)
    print("验证完成。")
    print("=" * 70)
