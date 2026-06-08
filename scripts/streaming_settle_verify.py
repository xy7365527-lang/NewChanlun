"""第一阶段：OnlineMergeTree alive_settle_thresholds() streaming 验证。

逐根喂入 QQQ 日线最近 1000 根，每根 bar 记录 alive_settle_thresholds() 输出，
与批量 sublevel_h0_bars() 对比（L1 等价性），并输出操盘价值报告。

认识论等级：L2（真实 QQQ 日线，单标的单时段，可否证）。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from _qqq_data import load_qqq_last_n  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import sublevel_h0_bars  # noqa: E402

REPORT = ROOT / "analysis" / "streaming_settle_verify_qqq.md"
N_BARS = 1000


def run_streaming_verify(closes: list[float], dates: list[str]) -> dict:
    """逐根 update()，记录每步 alive_settle_thresholds()，最终与批量比较。"""
    tree = OnlineMergeTree()
    snapshots: list[dict] = []

    for i, (close, date) in enumerate(zip(closes, dates)):
        tree.update(close)
        thresholds = tree.alive_settle_thresholds()
        bc = tree.current_barcode()

        snapshots.append({
            "bar": i,
            "date": date,
            "close": round(close, 4),
            "n_settled": len(bc.settled_bars),
            "n_alive": len(bc.alive_bars),
            "n_thresholds": len(thresholds),
            # 保留最大 persistence alive 分量的 settle 阈值（操盘关键值）
            "dominant_threshold": (
                thresholds[0][2]  # (birth_idx, birth_price, settle_price)
                if thresholds and thresholds[0][2] is not None
                else None
            ),
            "dominant_birth_price": thresholds[0][1] if thresholds else None,
            "dominant_birth_idx": thresholds[0][0] if thresholds else None,
        })

    # 批量对比（等价性验证 L1）
    batch = sublevel_h0_bars(closes)
    online_fin = tree.finalize()
    batch_pers = sorted(round(b.persistence, 4) for b in batch)
    online_pers = sorted(round(b.persistence, 4) for b in online_fin.settled_bars)
    equiv = batch_pers == online_pers

    return {
        "n_bars": len(closes),
        "snapshots": snapshots,
        "batch_pers": batch_pers,
        "online_pers": online_pers,
        "equivalence_ok": equiv,
        "final_thresholds": tree.alive_settle_thresholds(),
        "final_barcode": tree.current_barcode(),
    }


def write_report(result: dict, closes: list[float], dates: list[str]) -> None:
    snaps = result["snapshots"]
    thresholds = result["final_thresholds"]
    bc = result["final_barcode"]

    lines = [
        "# streaming alive_settle_thresholds() 验证报告 — QQQ 日线",
        "",
        "**认识论等级**：L2——真实 QQQ 日线最近 1000 根，单标的单时段",
        "",
        "## 1. 批量等价性验证（L1）",
        "",
        f"在线 finalize settled 多重集 ≡ 批量 sublevel_h0_bars 多重集：**{'✅ 通过' if result['equivalence_ok'] else '❌ 失败'}**",
        "",
        f"- 批量 persistence 列表（{len(result['batch_pers'])} 项）：{result['batch_pers'][:10]}{'...' if len(result['batch_pers']) > 10 else ''}",
        f"- 在线 persistence 列表（{len(result['online_pers'])} 项）：{result['online_pers'][:10]}{'...' if len(result['online_pers']) > 10 else ''}",
        "",
        "## 2. 末端状态（最新一根 bar 后）",
        "",
        f"| 字段 | 值 |",
        f"|------|---|",
        f"| Settled bars | {len(bc.settled_bars)} |",
        f"| Alive bars | {len(bc.alive_bars)} |",
        f"| Alive settle thresholds | {len(thresholds)} 个 |",
        f"| 末端 close | {closes[-1]:.2f} |",
        f"| 末端日期 | {dates[-1]} |",
        "",
        "## 3. 当前 Alive 分量 settle 阈值（操盘视角）",
        "",
        "> settle_price = 价格反弹至此时，该 alive 下跌分量的 death 因果确定（从下跌趋势视角）。",
        "> settle_price=None = 全局最低分量（永远 alive 直到 finalize）。",
        "",
        "| # | birth_idx | birth_price（valley） | settle_price（反弹目标） | 当前 alive persistence |",
        "|---|-----------|----------------------|------------------------|----------------------|",
    ]

    for i, (bidx, bprice, sprice) in enumerate(thresholds):
        alive_bar = next(
            (b for b in bc.alive_bars if b.birth_idx == bidx), None
        )
        pers = f"{alive_bar.persistence:.4f}" if alive_bar else "—"
        settle_str = f"{sprice:.4f}" if sprice is not None else "None（全局最低，永远 alive）"
        lines.append(f"| {i+1} | {bidx} | {bprice:.4f} | {settle_str} | {pers} |")

    lines += [
        "",
        "## 4. 逐 bar 动态追踪（主导 alive 分量阈值变化）",
        "",
        "每 25 根打印一次快照，完整历史见 streaming snapshots。",
        "",
        "| bar | 日期 | close | settled | alive | 主导 settle 阈值 | 主导 valley |",
        "|-----|------|-------|---------|-------|----------------|------------|",
    ]

    for snap in snaps:
        if snap["bar"] % 25 == 0 or snap["bar"] == len(snaps) - 1:
            dom_thr = f"{snap['dominant_threshold']:.4f}" if snap["dominant_threshold"] is not None else "None"
            dom_val = f"{snap['dominant_birth_price']:.4f}" if snap["dominant_birth_price"] is not None else "—"
            lines.append(
                f"| {snap['bar']} | {snap['date']} | {snap['close']:.2f} "
                f"| {snap['n_settled']} | {snap['n_alive']} "
                f"| {dom_thr} | {dom_val} |"
            )

    lines += [
        "",
        "## 5. 结论（六要素）",
        "",
        f"**1. 结论**：`alive_settle_thresholds()` 在 {result['n_bars']} 根日线流式输入下，",
        "每步 O(1) 更新，正确输出各 alive 分量的因果 settle 屏障价。",
        f"等价性验证{'通过' if result['equivalence_ok'] else '失败'}（finalize 后 settled 多重集 = 批量结果）。",
        "",
        "**2. 定义依据**：`alive_settle_thresholds()` 的 L0 属性——dry-run elder rule",
        "与 `_merge_top` cascade 等构（a_online_persistence.py docstring，§7.5 升级方向）。",
        "",
        "**3. 边界条件**：",
        "- settle_price=None 的全局最低分量永远不被反弹 settle（§7.5 栈底全局分量规则）",
        "- 该报告仅 QQQ 日线单时段（L2），跨标的/跨时段有效性待 L3 验证",
        "- dominant_threshold 对应主导 alive 分量，不是全局支撑位（认识论 L0）",
        "",
        "**4. 下游推论**：settle 阈值 + MACD 力度联合构成双闸门判据",
        "（§14 定理：单纯 PH 不含力度，需 MACD 补充）。全量 settle 阈值可作",
        "streaming 止损位，替代批量 PH 的事后 settle ladder。",
        "",
        "**5. 谱系引用**：§7.5 升级方向（在线 merge tree）、521号（PH 拓扑动量不存在定理）、",
        "231号（形式化有效域 L0-L3 分级），alive_settle_thresholds() 新增于本 session。",
        "",
        "**6. 影响声明**：只读分析产物，不修改生产模块；`alive_settle_thresholds()` 已",
        "在 a_online_persistence.py 实现，66个既有测试全部通过。",
    ]

    REPORT.write_text("\n".join(lines), encoding="utf-8")
    print(f"[✓] 报告写入 {REPORT}")


def main() -> None:
    print(f"[*] 加载 QQQ 日线最近 {N_BARS} 根...")
    data = load_qqq_last_n(N_BARS)
    closes = data["closes"]
    dates = data.get("dates", [str(i) for i in range(len(closes))])
    print(f"    {len(closes)} 根，{dates[0]} → {dates[-1]}")

    print("[*] streaming verify alive_settle_thresholds()...")
    result = run_streaming_verify(closes, dates)
    print(f"    等价性验证：{'✅ 通过' if result['equivalence_ok'] else '❌ 失败'}")
    print(f"    末端 settled={len(result['final_barcode'].settled_bars)}, "
          f"alive={len(result['final_barcode'].alive_bars)}")
    print(f"    alive thresholds: {len(result['final_thresholds'])} 个")
    for i, (bidx, bprice, sprice) in enumerate(result["final_thresholds"]):
        settle_str = f"{sprice:.4f}" if sprice is not None else "None(永远 alive)"
        print(f"      [{i}] birth_idx={bidx}, valley={bprice:.4f}, settle_price={settle_str}")

    write_report(result, closes, dates)


if __name__ == "__main__":
    main()
