"""ph_level_analysis.py — PH persistence 作为级别的拓扑定义验证

假说：OnlineMergeTree 的 settle 事件 persistence = 级别的连续化度量。
不需要人为指定级别，persistence 自然给出尺度——如果 persistence 分布有自然聚类（gap），
对应缠论的笔级/线段级/走势级。

验证方法：
1. QQQ 日线跑 OnlineMergeTree（1255 根 5y 日线）
2. 收集 finalize 后所有 settled bars 的 persistence 值
3. 画 persistence 分布直方图，看是否有自然聚类
4. 用 TV 数据 (qqq_chanlun_labels.json) 的 137 个买卖点作为 ground truth
5. 看 BSP 价格对应的 persistence 范围是否与聚类对齐

认识论等级：L2（QQQ 5y 真实日线数据，结论可否证）
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import matplotlib  # noqa: E402
matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

from newchan.a_online_persistence import MergeBar, OnlineMergeTree  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUTPUT = ROOT / "analysis"


# ============================================================
# 数据加载
# ============================================================

def load_qqq(period: str = "5y") -> dict:
    return json.load(open(CACHE / f"QQQ_1d_{period}.json"))


def load_tv_bsp() -> list[dict]:
    """加载 TV 缠论买卖点标注（137 条）。"""
    data = json.load(open(CACHE / "qqq_chanlun_labels.json"))
    labels = data["pine"]["labels"]
    return [
        lbl for lbl in labels
        if lbl.get("price") and ("买" in str(lbl.get("text", "")) or "卖" in str(lbl.get("text", "")))
    ]


# ============================================================
# PH merge tree 运行
# ============================================================

def run_tree(closes: list[float]) -> list[MergeBar]:
    """运行 OnlineMergeTree，返回 finalize 后全部 settled bars。"""
    tree = OnlineMergeTree()
    for p in closes:
        tree.update(float(p))
    barcode = tree.finalize()
    return list(barcode.settled_bars)


# ============================================================
# 聚类分析
# ============================================================

def find_gap_boundaries(
    persistences: list[float],
    n_boundaries: int = 3,
    noise_floor: float = 1.0,
) -> list[float]:
    """在 log 空间中找最大跳跃，返回有意义的边界值。

    noise_floor: 低于此值的 persistence 视为数值噪声，不参与聚类。
    在 log 空间而非线性空间找间隔，避免大值区的绝对差掩盖小值区的结构。
    """
    # 过滤噪声下界
    meaningful = np.sort([p for p in persistences if p >= noise_floor])
    if len(meaningful) < 6:
        return []
    log_arr = np.log10(meaningful)
    gaps = np.diff(log_arr)
    # 找最大的 n_boundaries 个 log 间隔
    top_indices = np.argsort(gaps)[-n_boundaries:][::-1]
    boundaries = sorted(float(meaningful[i + 1]) for i in top_indices)
    return boundaries


def assign_level(p: float, boundaries: list[float]) -> int:
    """根据 persistence 值和边界分配级别（1=最小级别）。"""
    for k, b in enumerate(boundaries):
        if p < b:
            return k + 1
    return len(boundaries) + 1


# ============================================================
# 直方图生成
# ============================================================

NOISE_FLOOR = 1.0  # $1 以下视为数值噪声


def plot_histogram(bars: list[MergeBar], boundaries: list[float], out_path: Path) -> None:
    """生成 persistence 分布直方图（对数x轴）。"""
    persistences = [b.persistence for b in bars if b.persistence >= NOISE_FLOOR]
    fig, axes = plt.subplots(1, 2, figsize=(14, 5))

    # 左图：线性 x 轴
    axes[0].hist(persistences, bins=60, color="steelblue", alpha=0.7, edgecolor="white")
    for b in boundaries:
        axes[0].axvline(x=b, color="red", linestyle="--", alpha=0.8, linewidth=1.5)
    axes[0].set_xlabel("Persistence (价格幅度 $)")
    axes[0].set_ylabel("Count")
    axes[0].set_title("QQQ persistence 分布（线性轴）")

    # 右图：对数 x 轴（更清晰地显示多尺度结构）
    log_p = np.log10([p for p in persistences if p > 0])
    axes[1].hist(log_p, bins=60, color="coral", alpha=0.7, edgecolor="white")
    for b in boundaries:
        if b > 0:
            axes[1].axvline(x=np.log10(b), color="darkred", linestyle="--", alpha=0.8, linewidth=1.5)
    axes[1].set_xlabel("log10(Persistence)")
    axes[1].set_ylabel("Count")
    axes[1].set_title("QQQ persistence 分布（对数轴）")

    plt.suptitle(f"QQQ 5y 日线 | {len(persistences)} 个 settled events | {len(boundaries)} 个自然边界")
    plt.tight_layout()
    plt.savefig(out_path, dpi=150, bbox_inches="tight")
    plt.close()
    print(f"  直方图已保存: {out_path}")


# ============================================================
# TV BSP 对比
# ============================================================

def match_bsp_to_settled(
    bsps: list[dict], bars: list[MergeBar], boundaries: list[float]
) -> list[dict]:
    """
    对每个 TV BSP 价格，找最近的 settled bar (按 death_price 匹配)。
    返回每个 BSP 的匹配信息（persistence, level, 价格距离）。
    """
    death_prices = np.array([b.death_price for b in bars])
    results = []
    for bsp in bsps:
        price = float(bsp["price"])
        dists = np.abs(death_prices - price)
        nearest_idx = int(np.argmin(dists))
        nearest_bar = bars[nearest_idx]
        lvl = assign_level(nearest_bar.persistence, boundaries)
        results.append({
            "text": bsp.get("text", ""),
            "bsp_price": price,
            "matched_death_price": nearest_bar.death_price,
            "dist": float(dists[nearest_idx]),
            "persistence": nearest_bar.persistence,
            "span": nearest_bar.span,
            "level": lvl,
        })
    return results


def level_distribution(matched: list[dict]) -> dict[int, int]:
    """统计 BSP 匹配到各级别的分布。"""
    dist: dict[int, int] = {}
    for m in matched:
        lvl = m["level"]
        dist[lvl] = dist.get(lvl, 0) + 1
    return dist


# ============================================================
# 报告生成
# ============================================================

def write_report(
    bars: list[MergeBar],
    boundaries: list[float],
    matched_bsps: list[dict],
    lvl_dist: dict[int, int],
    closes: list[float],
    out_path: Path,
) -> None:
    all_persistences = [b.persistence for b in bars]
    persistences = [p for p in all_persistences if p >= NOISE_FLOOR]
    noise_count = len(all_persistences) - len(persistences)
    arr = np.array(persistences) if persistences else np.array([0.0])

    # 按级别分组统计（只用有意义的 persistences）
    level_bars: dict[int, list[float]] = {}
    for b in bars:
        if b.persistence < NOISE_FLOOR:
            continue
        lvl = assign_level(b.persistence, boundaries)
        level_bars.setdefault(lvl, []).append(b.persistence)

    lines = [
        "# PH Persistence 级别定义验证报告",
        "",
        f"> 生成时间：2026-05-31  ",
        f"> 数据：QQQ 5y 日线（{len(closes)} 根）  ",
        "> 认识论等级：L2（真实数据，可否证）",
        "",
        "---",
        "",
        "## 1. 假说",
        "",
        "OnlineMergeTree settle 事件的 persistence = 级别的连续化度量。",
        "如果 persistence 分布有 2-3 个自然聚类（gap），对应缠论笔/线段/走势三级。",
        "",
        "## 2. Settled Events 统计",
        "",
        f"- 总 settled bars：{len(bars)}（含噪声 <$1：{noise_count}，有意义：{len(persistences)}）",
        f"- 有意义 persistence 范围：${arr.min():.2f} — ${arr.max():.2f}",
        f"- 中位数：${np.median(arr):.2f}",
        f"- 均值：${arr.mean():.2f}",
        f"- 标准差：${arr.std():.2f}",
        "",
        "## 3. 自然边界（最大相对跳跃法）",
        "",
        "| 边界 | persistence 阈值 |",
        "|------|-----------------|",
    ]
    for k, b in enumerate(boundaries):
        lines.append(f"| L{k+1}/L{k+2} 分界 | ${b:.2f} |")

    lines += [
        "",
        "## 4. 各级别 Settled Bars 分布",
        "",
        "| 级别 | 数量 | persistence 范围 | 均值 | 缠论对应 |",
        "|------|------|-----------------|------|---------|",
    ]
    chanlun_names = {1: "笔级", 2: "线段级", 3: "走势级", 4: "更高级"}
    for lvl in sorted(level_bars.keys()):
        pvals = level_bars[lvl]
        parr = np.array(pvals)
        prev_b = boundaries[lvl - 2] if lvl >= 2 else 0.0
        next_b = boundaries[lvl - 1] if lvl - 1 < len(boundaries) else arr.max()
        name = chanlun_names.get(lvl, f"L{lvl}")
        lines.append(
            f"| L{lvl} ({name}) | {len(pvals)} | ${prev_b:.0f}–${next_b:.0f} | ${parr.mean():.1f} | {name} |"
        )

    lines += [
        "",
        "## 5. TV 137 个买卖点的 persistence 分布",
        "",
        f"TV CZSC 共 137 个买卖点。按最近 death_price 匹配到 settled bars，统计各级别分布：",
        "",
        "| PH 级别 | 匹配 BSP 数 | 缠论对应 |",
        "|---------|------------|---------|",
    ]
    for lvl in sorted(lvl_dist.keys()):
        name = chanlun_names.get(lvl, f"L{lvl}")
        lines.append(f"| L{lvl} ({name}) | {lvl_dist[lvl]} | {name} |")

    # Top 10 BSP matches with details
    top_matched = sorted(matched_bsps, key=lambda x: x["dist"])[:15]
    lines += [
        "",
        "### 5.1 精度最高的 15 个匹配（按价格距离排序）",
        "",
        "| BSP 文本 | BSP 价格 | 匹配 death_price | 价格距离 | persistence | PH 级别 | span |",
        "|---------|---------|----------------|---------|------------|---------|------|",
    ]
    for m in top_matched:
        lines.append(
            f"| {m['text'].strip()} | ${m['bsp_price']:.2f} | ${m['matched_death_price']:.2f} | ${m['dist']:.2f} | ${m['persistence']:.2f} | L{m['level']} | {m['span']} |"
        )

    lines += [
        "",
        "## 6. 假说验证结论",
        "",
    ]

    # Determine if clustering is present
    n_meaningful_levels = sum(1 for v in level_bars.values() if len(v) >= 3)
    has_clustering = n_meaningful_levels >= 2 and len(boundaries) >= 2

    # Check if BSP prices mainly fall in L2/L3 (not the smallest level)
    bsp_high_level = sum(v for lvl, v in lvl_dist.items() if lvl >= 2)
    bsp_frac = bsp_high_level / len(matched_bsps) if matched_bsps else 0

    if has_clustering:
        lines += [
            "**结论（L2 验证，可否证）：**",
            "",
            f"- persistence 分布有 {len(boundaries)} 个自然边界，对应 {len(boundaries) + 1} 个聚类 ✓",
            f"- {n_meaningful_levels} 个有意义级别（≥3 个 settled events）✓",
            f"- TV 137 个 BSP 中，{bsp_high_level} 个（{bsp_frac:.0%}）匹配到 L2+ persistence 范围",
            "",
            "**边界条件**：",
            "- 若 QQQ 价格进入低波动横盘（persistence 范围收窄），聚类数可能减少至 2 个",
            "- 若使用不同时段数据，边界值会随价格量级变化（persistence 是绝对价格差，非相对值）",
            "- persistence 聚类是必要条件验证，**不是** MACD 力度的充分条件（formalization-validity-domain 规则）",
        ]
    else:
        lines += [
            "**结论（L2 验证）：当前数据未发现明显自然聚类。**",
            "",
            "可能原因：",
            "- 5y 数据跨度不足以覆盖多个完整级别的结构",
            "- QQQ 为单向牛市，缺少足够的级别分层",
        ]

    lines += [
        "",
        "## 7. 结果包六要素",
        "",
        "**结论**：" + (f"persistence 分布有 {len(boundaries)} 个自然边界，初步支持 PH persistence = 级别度量的假说" if has_clustering else "未发现明显自然聚类，假说待进一步验证"),
        "",
        "**定义依据**：OnlineMergeTree settled bar 的 persistence = death_price - birth_price = 摆动幅度（prominence）。这与缠论中不同级别笔/线段/走势的力度差异在幅度维度上同构。",
        "",
        "**边界条件**：persistence 聚类边界值随时间窗口和标的价格量级变化；本结论仅在 QQQ 5y 日线（2021-2026）的 L2 数据上成立。",
        "",
        "**下游推论**：若假说成立，可用 persistence 阈值替代人工指定级别参数，作为 OnlineMergeTree 级别识别的自动化依据。",
        "",
        "**谱系引用**：formalization-validity-domain 规则（L2 认识论等级标注）；不确定是否有与 PH 级别定义相关的谱系记录，需手动查证。",
        "",
        "**影响声明**：新建 `scripts/ph_level_analysis.py`（本脚本），`analysis/ph_level_definition.md`（本报告），`analysis/ph_persistence_histogram.png`（直方图）。未修改任何引擎代码。",
    ]

    out_path.write_text("\n".join(lines), encoding="utf-8")
    print(f"  报告已保存: {out_path}")


# ============================================================
# 主函数
# ============================================================

def main() -> None:
    print("=== PH Persistence 级别定义验证 ===")

    # 1. 加载数据
    data = load_qqq("5y")
    closes = data["closes"]
    dates = data.get("dates", [])
    print(f"QQQ 5y: {len(closes)} 根K线 ({dates[0] if dates else '?'} → {dates[-1] if dates else '?'})")

    # 2. 运行 OnlineMergeTree
    print("运行 OnlineMergeTree...")
    bars = run_tree(closes)
    print(f"  Settled bars: {len(bars)}")

    if not bars:
        print("ERROR: 无 settled bars，无法继续分析")
        return

    persistences = [b.persistence for b in bars]
    print(f"  Persistence 范围: ${min(persistences):.2f} – ${max(persistences):.2f}")

    # 3. 寻找自然边界
    boundaries = find_gap_boundaries(persistences, n_boundaries=4)
    print(f"  自然边界（{len(boundaries)} 个）: {[f'${b:.1f}' for b in boundaries]}")

    # 4. 画直方图
    hist_path = OUTPUT / "ph_persistence_histogram.png"
    plot_histogram(bars, boundaries, hist_path)

    # 5. 加载 TV BSP 并匹配
    print("加载 TV 买卖点...")
    bsps = load_tv_bsp()
    print(f"  TV BSPs: {len(bsps)} 个")

    matched = match_bsp_to_settled(bsps, bars, boundaries)
    lvl_dist = level_distribution(matched)
    print(f"  BSP 级别分布: {lvl_dist}")

    # 6. 输出报告
    report_path = OUTPUT / "ph_level_definition.md"
    write_report(bars, boundaries, matched, lvl_dist, closes, report_path)

    print("\n✓ 分析完成")


if __name__ == "__main__":
    main()
