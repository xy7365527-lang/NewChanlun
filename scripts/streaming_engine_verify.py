"""第一阶段：修复后引擎的 streaming 验证（天然零前视）。

逐根喂入 RecursiveOrchestrator（reset_dir_on_fractal=True + MACD 背驰），
记录每根 bar 的信号状态，统计笔/线段/买卖点/confirmed，对比 TV 137。
附带 gap≥3 vs gap≥4 对比（问题4）。

5 项修复在 streaming 路径的落点：
  1. 尺度错位  → 截 1000 根日线窗口（本脚本控制）
  2. 包含方向  → RecursiveOrchestrator(reset_dir_on_fractal=True)
  3. 背驰MACD  → RecursiveOrchestrator(enable_macd_divergence=True)
  4. 笔 gap    → new_raw_gap_min 参数（3 vs 4 对比）
  5. 递归层    → RecursiveStack 已实现（max_levels）

认识论等级：L2（真实 QQQ 日线，结论可否证）。
"""
from __future__ import annotations

import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from _qqq_data import load_qqq_last_n, load_tv_bsp, to_bars  # noqa: E402
from newchan.events import (  # noqa: E402
    BuySellPointConfirmV1,
    BuySellPointInvalidateV1,
)
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402

OUTPUT = ROOT / "analysis"


def classify_bar_signal(events: list) -> str:
    """把一根 bar 的事件分类为信号状态标签。"""
    has_confirm = any(isinstance(e, BuySellPointConfirmV1) for e in events)
    has_negate = any(isinstance(e, BuySellPointInvalidateV1) for e in events)
    if has_confirm and has_negate:
        return "confirmed+negated"
    if has_confirm:
        return "confirmed"
    if has_negate:
        return "negated"
    return "none"


def run_streaming(bars: list, *, new_raw_gap_min: int, enable_macd: bool) -> dict:
    """逐根喂入，返回统计 + 每 bar 信号日志。"""
    orch = RecursiveOrchestrator(
        stream_id="verify",
        max_levels=6,
        stroke_mode="new",
        reset_dir_on_fractal=True,
        new_raw_gap_min=new_raw_gap_min,
        enable_macd_divergence=enable_macd,
    )

    bar_signals: list[dict] = []
    confirmed_ids: dict[int, dict] = {}   # bsp_id → 末次 confirm 详情（去重）
    confirmed_buy = set()
    confirmed_sell = set()
    signal_counts: dict[str, int] = defaultdict(int)

    last_snap = None
    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        last_snap = snap
        label = classify_bar_signal(snap.all_events)
        signal_counts[label] += 1

        for e in snap.all_events:
            if isinstance(e, BuySellPointConfirmV1):
                confirmed_ids[e.bsp_id] = {
                    "bar": i, "kind": e.kind, "side": e.side,
                    "price": e.price, "level": e.level_id,
                }
                if e.side == "buy":
                    confirmed_buy.add(e.bsp_id)
                else:
                    confirmed_sell.add(e.bsp_id)

        if label != "none":
            bar_signals.append({
                "bar": i, "date": bars[i].ts.date().isoformat(),
                "close": round(bar.close, 2), "label": label,
            })

    # 末态快照统计
    bsps = last_snap.bsp_snapshot.buysellpoints if last_snap else []
    n_levels = len(last_snap.recursive_snapshots) if last_snap else 0
    strokes = last_snap.bi_snapshot.strokes if last_snap else []
    segs = last_snap.seg_snapshot.segments if last_snap else []
    zhongshus = last_snap.zs_snapshot.zhongshus if last_snap else []
    moves = last_snap.move_snapshot.moves if last_snap else []

    return {
        "new_raw_gap_min": new_raw_gap_min,
        "enable_macd": enable_macd,
        "n_strokes": len(strokes),
        "n_strokes_confirmed": sum(1 for s in strokes if s.confirmed),
        "n_segments": len(segs),
        "n_segments_confirmed": sum(1 for s in segs if getattr(s, "confirmed", False)),
        "n_zhongshus": len(zhongshus),
        "n_moves": len(moves),
        "n_recursive_levels": n_levels,
        "n_bsp_final": len(bsps),
        "n_bsp_final_confirmed": sum(1 for b in bsps if b.confirmed),
        "n_confirmed_unique": len(confirmed_ids),
        "n_confirmed_buy": len(confirmed_buy),
        "n_confirmed_sell": len(confirmed_sell),
        "signal_counts": dict(signal_counts),
        "bar_signals": bar_signals,
        "confirmed_detail": list(confirmed_ids.values()),
    }


def _fmt_pct(part: int, whole: int) -> str:
    return f"{part}/{whole} ({100*part/whole:.1f}%)" if whole else f"{part}/0"


def write_report(results: list[dict], tv_n: int, n_bars: int, out: Path) -> None:
    base = next(r for r in results if r["new_raw_gap_min"] == 3 and r["enable_macd"])
    lines = [
        "# 第一阶段：修复后引擎 streaming 验证",
        "",
        f"> 数据：QQQ 日线最近 {n_bars} 根（yfinance，零前视逐根喂入）  ",
        "> 认识论等级：L2（真实数据，可否证）",
        "",
        "## 0. 配置对比总表",
        "",
        "| 配置 | 笔 | 线段 | 中枢 | 走势 | 递归层 | BSP末态 | confirmed(唯一) | 买 | 卖 |",
        "|------|---|------|------|------|--------|---------|----------------|----|----|",
    ]
    for r in results:
        cfg = f"gap≥{r['new_raw_gap_min']}{'+MACD' if r['enable_macd'] else ''}"
        lines.append(
            f"| {cfg} | {r['n_strokes']} | {r['n_segments']} | {r['n_zhongshus']} | "
            f"{r['n_moves']} | {r['n_recursive_levels']} | {r['n_bsp_final']} | "
            f"{r['n_confirmed_unique']} | {r['n_confirmed_buy']} | {r['n_confirmed_sell']} |"
        )

    lines += [
        "",
        "## 1. 对比 TV 137 个买卖点",
        "",
        f"| 维度 | 引擎(gap≥3+MACD) | TV 基准 | 量级 |",
        "|------|-----------------|---------|------|",
        f"| 买卖点(confirmed 唯一) | {base['n_confirmed_unique']} | {tv_n} | "
        f"{base['n_confirmed_unique']/tv_n:.2f}x |",
        f"| BSP 末态(含未确认) | {base['n_bsp_final']} | {tv_n} | — |",
        "",
        "## 2. 信号状态分布（gap≥3+MACD）",
        "",
        "| 状态 | bar 数 |",
        "|------|--------|",
    ]
    for label, cnt in sorted(base["signal_counts"].items()):
        lines.append(f"| {label} | {cnt} |")

    lines += [
        "",
        "## 3. confirmed 买卖点明细（gap≥3+MACD，时间序）",
        "",
        "| # | bar | 日期 | 类型 | 方向 | 价格 | 级别 |",
        "|---|-----|------|------|------|------|------|",
    ]
    bars_dates = base.get("_bars_dates", {})
    for i, c in enumerate(base["confirmed_detail"], 1):
        d = bars_dates.get(c["bar"], "")
        lines.append(
            f"| {i} | {c['bar']} | {d} | {c['kind']} | {c['side']} | "
            f"{c['price']:.2f} | L{c['level']} |"
        )

    lines += [
        "",
        "## 4. gap≥3 vs gap≥4 笔层影响（问题4）",
        "",
    ]
    g3 = next(r for r in results if r["new_raw_gap_min"] == 3 and r["enable_macd"])
    g4 = next((r for r in results if r["new_raw_gap_min"] == 4 and r["enable_macd"]), None)
    if g4:
        lines += [
            f"- gap≥3：{g3['n_strokes']} 笔 → {g3['n_segments']} 线段 → "
            f"{g3['n_confirmed_unique']} confirmed BSP",
            f"- gap≥4：{g4['n_strokes']} 笔 → {g4['n_segments']} 线段 → "
            f"{g4['n_confirmed_unique']} confirmed BSP",
            f"- 收紧 gap 过滤了 {g3['n_strokes']-g4['n_strokes']} 笔"
            f"（{_fmt_pct(g3['n_strokes']-g4['n_strokes'], g3['n_strokes'])}）",
        ]

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"报告已保存: {out}")


def main() -> None:
    print("=== 第一阶段：streaming 引擎验证 ===")
    data = load_qqq_last_n(1000)
    bars = to_bars(data)
    tv_bsp = load_tv_bsp()
    print(f"数据: {data['source']} {len(bars)} 根 "
          f"{data['dates'][0]}→{data['dates'][-1]}  TV BSP={len(tv_bsp)}")

    bars_dates = {i: b.ts.date().isoformat() for i, b in enumerate(bars)}

    configs = [
        (3, True),   # 主配置：gap≥3 + MACD
        (4, True),   # 对比：gap≥4 + MACD
        (3, False),  # 对比：gap≥3 无 MACD（背驰退化）
    ]
    results = []
    for gap, macd in configs:
        print(f"\n--- 运行 gap≥{gap} macd={macd} ---")
        r = run_streaming(bars, new_raw_gap_min=gap, enable_macd=macd)
        r["_bars_dates"] = bars_dates
        results.append(r)
        print(f"  笔={r['n_strokes']} 线段={r['n_segments']} 中枢={r['n_zhongshus']} "
              f"走势={r['n_moves']} 递归层={r['n_recursive_levels']}")
        print(f"  BSP末态={r['n_bsp_final']} confirmed唯一={r['n_confirmed_unique']} "
              f"(买{r['n_confirmed_buy']}/卖{r['n_confirmed_sell']})")

    write_report(results, len(tv_bsp), len(bars), OUTPUT / "streaming_engine_verify_qqq.md")
    print("\n✓ 第一阶段验证完成")


if __name__ == "__main__":
    main()
