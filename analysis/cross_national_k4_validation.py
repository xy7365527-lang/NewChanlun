#!/usr/bin/env python3
"""跨国 K4 管线验证 — 真实数据跑通三层结构，产出报告（L2）。

用法：
    PYTHONPATH=src python analysis/cross_national_k4_validation.py [window_bars]
  window_bars 默认 60000（控制递归耗时；每文件取最近 N 根）。传 0 = 全量。

概念溯源：254号三层结构、292号折叠区间套、528/529号折叠通道重构。
认识论等级：L2（真实 databento/TWS 1min/1h 数据）。

⚠ 分辨率说明（诚实标注）：US 用 1min（顶点齐全），EU/JP/CN 用 1h（databento 10年史）。
不同经济体的 bar 数/时间跨度见报告各边 date_range——跨经济体配置比较需注意此分辨率差异。
"""

from __future__ import annotations

import sys
import time
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.topology.cross_national_pipeline import (  # noqa: E402
    CrossNationalK4Result,
    run_cross_national_k4,
)
from newchan.topology.graph import Vertex  # noqa: E402

REPORT_PATH = ROOT / "analysis" / "cross_national_k4_report.md"


def _sigma_label(name: str) -> str:
    return {"UP": "↑", "DOWN": "↓", "FLAT": "─"}.get(name, "?")


def write_report(res: CrossNationalK4Result, window_bars: int, elapsed: float) -> None:
    L: list[str] = []
    L.append("# 跨国 K4 管线验证报告\n")
    L.append(f"生成时间：{datetime.now().strftime('%Y-%m-%d %H:%M')}")
    L.append(f"窗口：{'全量' if not window_bars else f'最近 {window_bars} bars/边'} | 总耗时：{elapsed:.1f}s")
    L.append("认识论等级：L2（真实数据）\n")

    # ── 层1：各经济体 K4 ──
    L.append("## 层1：各经济体 K4 配置 Γ=(σ_P, σ_C, σ_R)\n")
    L.append("| 经济体 | 本币 | σ_P | σ_C | σ_R | 完整配置 | 缺口顶点 |")
    L.append("|--------|------|-----|-----|-----|---------|---------|")
    for eco, r in res.economies.items():
        def sg(v: Vertex) -> str:
            return _sigma_label(r.partial_sigma[v].name) if v in r.partial_sigma else "·"
        cfg = r.config.label if r.config else "（部分）"
        miss = ",".join(v.name for v in r.missing) or "—"
        L.append(f"| {eco} | {r.currency} | {sg(Vertex.P)} | {sg(Vertex.C)} | {sg(Vertex.R)} | {cfg} | {miss} |")
    L.append("")
    # 各边详情
    for eco, r in res.economies.items():
        if not r.edges:
            continue
        L.append(f"### {eco} 独立边详情\n")
        L.append("| 边 | bars | 范围 | 最高级别走势 | σ | L |")
        L.append("|----|------|------|------------|---|---|")
        for v, e in r.edges.items():
            L.append(f"| {v.name}/M | {e.bar_count:,} | {e.date_range[0][:10]}→{e.date_range[1][:10]} "
                     f"| {e.move_kind}.{e.move_direction}{'(settled)' if e.move_settled else ''} "
                     f"| {_sigma_label(e.sigma.name)} | L{e.max_level} |")
        L.append("")

    # ── 层0：货币边 ──
    L.append("## 层0：结算尺空间 Σ（货币边 M_i/M_US）\n")
    L.append("| 经济体 | FX边 | bars | 最高级别走势 | σ | L |")
    L.append("|--------|------|------|------------|---|---|")
    for eco, fx in res.currency_layer.fx_readings.items():
        L.append(f"| {eco} | {fx.name} | {fx.bar_count:,} | {fx.move_kind}.{fx.move_direction}"
                 f"{'(settled)' if fx.move_settled else ''} | {_sigma_label(fx.sigma.name)} | L{fx.max_level} |")
    brk = res.currency_layer.synchronized_break_signal()
    L.append(f"\n**结算尺断裂候选信号**（254号定理2：≥2条货币边同向已结算趋势）：{'⚠ 是' if brk else '否'}")
    L.append("> 注：这是候选信号，非确认——单边事件无法区分分子/分母变化（254号 OQ5）。\n")

    # ── 跨国 C 路径 ──
    cp = res.c_path
    L.append("## 跨国 C 路径：折叠通道全局共享（292号）\n")
    L.append("| 折叠通道 | 标的 | bars | 走势 | σ |")
    L.append("|---------|------|------|------|---|")
    L.append(f"| Au (C↔M) | {cp.au_reading.name} | {cp.au_reading.bar_count:,} "
             f"| {cp.au_reading.move_kind}.{cp.au_reading.move_direction} | {_sigma_label(cp.au_reading.sigma.name)} |")
    L.append(f"| Oil (C→P) | {cp.oil_reading.name} | {cp.oil_reading.bar_count:,} "
             f"| {cp.oil_reading.move_kind}.{cp.oil_reading.move_direction} | {_sigma_label(cp.oil_reading.sigma.name)} |")
    L.append(f"\n**ω = 金价/油价 = {cp.omega_last:.2f}** | 折叠方向：{cp.omega_direction} → {cp.credit_signal}")
    L.append("> 292号：Au 全局共享 → 一条 GC 序列锚定所有经济体的 C↔M 折叠。")
    L.append("> 482号：ω 仅作长期方向指标，**不接实时入场门控**（Omega Regime 已 L2 证伪）。")
    L.append(f"> 各经济体本币 C/M 边：{list(cp.economy_c_edges.keys()) or '无（C 多为本币数据缺口）'}\n")

    # ── 结果包 ──
    L.append("## 结果包\n")
    L.append("**结论**：跨国 K4 三层结构（层0货币/层1各经济体/跨国C路径）在真实数据上跑通，"
             "各经济体配置与折叠通道读数见上表。\n")
    L.append("**定义依据**：254号三层递归结构；独立边 X/M=本币计价序列（M 基准，026号）；"
             "走势方向=最高级别 move 方向（a_move_v1.py）；Au 全局共享折叠（292号）。\n")
    L.append("**边界条件**：")
    L.append("- US 用 1min、EU/JP/CN 用 1h——分辨率差异，跨经济体配置比较需注意（见各边 bars/范围）。")
    L.append("- 中国本土 P/C/R（IF/SC/AU9999）在 IB/databento 均无源（已验证），CN 仅 FX 层。")
    L.append("- EU/JP 的 C（广义商品）/R（不动产）本币源缺口（254号 OQ2 不动产本地化），为部分配置。")
    L.append("- window_bars 截取最近 N 根——宏观 regime 判断应用大窗口/全量。\n")
    L.append("**下游推论**：层0 货币边同步模式可检测结算尺断裂（254号定理2）；"
             "Γ_i 历史轨迹揭示各经济体 regime 切换；ω 折叠通道跨经济体共享锚定信用环境。\n")
    L.append("**谱系引用**：254号（多经济体本体论）、292号（折叠区间套）、330号（资本循环）、"
             "527号（σ走势方向）、528/529号（折叠通道重构）、project_global_k4_data_sources（数据边界）。\n")
    L.append("**影响声明**：新建验证脚本+报告，不改引擎；消费 cross_national_pipeline.py（src）。\n")
    L.append("**认识论等级**：L2（真实数据，可证伪）。中国本土缺口为否定性结果（缩小有效域）。")

    REPORT_PATH.write_text("\n".join(L))
    print(f"\n报告已写入：{REPORT_PATH}")


def main() -> int:
    window = int(sys.argv[1]) if len(sys.argv) > 1 else 60000
    window_bars = window if window > 0 else None
    print("=" * 60)
    print("  跨国 K4 管线验证（真实数据 L2）")
    print(f"  窗口：{'全量' if not window_bars else f'最近 {window_bars} bars/边'}")
    print("=" * 60)
    t0 = time.time()
    res = run_cross_national_k4(window_bars=window_bars)
    elapsed = time.time() - t0

    print("\n层1 各经济体：")
    for eco, r in res.economies.items():
        cfg = r.config.label if r.config else f"部分{[v.name for v in r.partial_sigma]}"
        print(f"  {eco}({r.currency}): {cfg} | 缺口={[v.name for v in r.missing]}")
    print("\n层0 货币边：")
    for eco, fx in res.currency_layer.fx_readings.items():
        print(f"  {eco}: σ={fx.sigma.name} ({fx.move_kind}.{fx.move_direction})")
    print(f"  断裂候选信号: {res.currency_layer.synchronized_break_signal()}")
    cp = res.c_path
    print(f"\n跨国C路径: Au σ={cp.au_reading.sigma.name}, Oil σ={cp.oil_reading.sigma.name}, "
          f"ω={cp.omega_last:.2f} → {cp.credit_signal}")
    print(f"\n总耗时：{elapsed:.1f}s")

    write_report(res, window or 0, elapsed)
    return 0


if __name__ == "__main__":
    sys.exit(main())
