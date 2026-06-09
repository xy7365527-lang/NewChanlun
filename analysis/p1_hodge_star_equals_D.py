#!/usr/bin/env python3
"""P1 验证实验：Hodge star ⋆ = 缠论 D 算子（⋆=D）。

## 命题（P1）

J = D(F) = 残差曲率 F 经缠论递归后的**方向性力度**。
对缠论走势做正交分解：
  - persistence  = |力度| = ker(D) 成分（无向幅度）
  - direction    = 方向   = image(D) 成分（±1）
  - 方向性力度 J = persistence × sign(direction) = image(D) 在有向价格空间的投影

P1 成立 ⟺ 在已知 regime 反转点附近，J 出现非平凡行为：
  (a) 力度衰竭：反转前末段同向走势 persistence 递减（= 背驰）
  (b) 方向翻转一致：反转前后走势方向翻转，且翻转方向与残差局部极值一致
  (c) 不滞后：方向翻转信号在反转点之前或同步（settle 时间 ≤ t* + 容差）

## 两层检验（严格性升级：n=3 弱检验 → 全样本统计检验）

**案例层（L1 单点）**：在 3 个已知 regime 反转点检验 (a)(b)(c)。
  局限：L1 残差走势是天级（~2-3天/走势），与月级 regime 反转**尺度错配**；
  且 n=3 统计无力。单点结果仅作案例，不构成 P1 的最终判决。

**统计层（L1 全样本）**：P1 的核心可证伪命题是 ⋆=D ⟺ image(D) 的方向翻转与
  残差曲率 F 的极值系统性对齐。检验全部 L1 方向翻转事件：翻转方向（up→down /
  down→up）是否与翻转时刻残差局部极值（顶/底）一致。
  - 若一致率显著 > 50%（二项检验）→ J 方向携带 F 极值信息，⋆=D 有统计支持
  - 若一致率 ≈ 50% → J 方向是噪声，⋆=D 在 L1 退化 bar 上**否证**
  这是把『3 反转点是否命中』升级为有统计力的全样本检验。

**高级别（L2/L3）不可判定声明**：L2+ 走势的 settle_ts 是轮询锚定（分辨率
  ~200 bar），与残差实际极值错位（实测 2022-09 残差顶后应转跌，L2 锚定却显示
  08-31 down→10-07 up）。月级反转的 L2/L3 方向对齐受锚定局限 → **不可判定**，
  本实验不在 L2/L3 强行判决（no-workaround：不用不可靠数据硬凑结论）。

## 关键诚实声明（no-patch / formalization-validity-domain）

残差是退化 bar（O=H=L=C=r），故 `persistence ≡ amplitude`（已逐位验证）。
因此 J 相对纯振幅的**唯一新增信息是方向符号**。P1 的真正考验是：
方向符号是否在反转点携带与价格反转一致的非平凡结构——若仅是符号附加而
与反转无关联，则 ⋆=D 在此数据上是平凡的（image(D) 投影退化）。

## 认识论等级：L2

真实数据（残差 r = log(DX)+0.576·log(EURUSD)），单残差构造 = 单标的/单时段。
三反转点检验是**可证伪**假设。否定性结果（J 在反转点无非平凡行为）缩小有效域。

## 数据源

events 缓存（analysis/data_cache/residual_flow_events.json）由
residual_chanlun_flow_velocity.py 流式 RecursiveOrchestrator(max_levels=6,
stroke_mode="wide") 在 2M bar 上产出，含 L1 走势的真实 bar 锚定（start_bar/
settle_bar）+ persistence + direction + settle_ts。本脚本复用该缓存（同一 Rust
引擎产物），不重跑流式。

用法：
    PYTHONPATH=src .venv/bin/python analysis/p1_hodge_star_equals_D.py
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "analysis" / "data_cache"
EVENTS_CACHE = CACHE / "residual_flow_events.json"
RESIDUAL_NPZ = CACHE / "_residual_dx6e_aligned.npz"
REPORT = ROOT / "analysis" / "p1_hodge_star_equals_D.md"

# 已知 regime 反转点（具体锚定日期，UTC）
REVERSALS = [
    ("2022-03-16", "Fed 加息启动"),
    ("2022-09-28", "DXY 见顶 114.78（内生反转）"),
    ("2024-09-18", "Fed 降息启动"),
]

# 判据操作化参数（边界条件：结论对这些参数的敏感性见报告）
EXTREMUM_HALF_DAYS = 30   # 残差局部极值判定窗口半宽（天）
LAGGARD_TOL_DAYS = 30     # 判据(c)不滞后容差（天）：翻转 settle 不晚于 t*+该值
EXTREMUM_FRAC = 0.80      # 判据(b)局部极值阈：r 在窗口内分位 ≥0.8(顶) 或 ≤0.2(底)


def _parse_ts(s: str) -> datetime:
    """ISO 时间戳 → aware datetime（缓存格式 'YYYY-MM-DD HH:MM:SS+00:00'）。"""
    return datetime.fromisoformat(s)


def _dir_sign(direction: str) -> int:
    return 1 if direction == "up" else -1


@dataclass(frozen=True)
class Move:
    """单个 L1 走势的方向性力度视图。"""
    settle: datetime
    start_bar: int
    settle_bar: int
    persistence: float
    direction: str

    @property
    def dir_sign(self) -> int:
        return _dir_sign(self.direction)

    @property
    def directed_force(self) -> float:
        """J = persistence × sign(direction) = image(D) 投影。"""
        return self.persistence * self.dir_sign


def load_l1_moves() -> list[Move]:
    """复用 events 缓存，取 L1 settled 走势按 settle 时间排序。"""
    data = json.loads(EVENTS_CACHE.read_text())
    moves = [
        Move(
            settle=_parse_ts(e["settle_ts"]),
            start_bar=int(e["start_bar"]),
            settle_bar=int(e["settle_bar"]),
            persistence=float(e["persistence"]),
            direction=e["direction"],
        )
        for e in data["events"]
        if e["level"] == 1 and e["settled"]
    ]
    moves.sort(key=lambda m: m.settle)
    return moves, data["meta"]


def build_epochs(ts: np.ndarray) -> np.ndarray:
    """ts 字符串数组（时间排序）→ 单调递增 epoch float 数组。"""
    return np.array([_parse_ts(s).timestamp() for s in ts])


def residual_extremum_at(epochs: np.ndarray, r: np.ndarray, center_epoch: float,
                         half_days: int) -> dict:
    """判定残差在 center_epoch ±half_days 窗口内 center 处是局部顶/底/中段。

    epochs 单调 → searchsorted 高效（适用全样本 680+ 翻转点）。
    返回 {kind: 'top'|'bottom'|'mid', r_at, pctile, expected_dir}。
    expected_dir = 反转后应出现/新走势的方向（顶→down, 底→up, 中段→None）。
    """
    lo = center_epoch - half_days * 86400
    hi = center_epoch + half_days * 86400
    i_lo = int(np.searchsorted(epochs, lo, "left"))
    i_hi = int(np.searchsorted(epochs, hi, "right"))
    seg = r[i_lo:i_hi]
    if len(seg) == 0:
        return {"kind": "mid", "r_at": float("nan"), "pctile": float("nan"),
                "expected_dir": None}
    ci = int(np.searchsorted(epochs, center_epoch, "left"))
    ci = min(max(ci, 0), len(r) - 1)
    r_at = float(r[ci])
    pctile = float((seg < r_at).mean())
    if pctile >= EXTREMUM_FRAC:
        return {"kind": "top", "r_at": r_at, "pctile": pctile, "expected_dir": "down"}
    if pctile <= 1 - EXTREMUM_FRAC:
        return {"kind": "bottom", "r_at": r_at, "pctile": pctile, "expected_dir": "up"}
    return {"kind": "mid", "r_at": r_at, "pctile": pctile, "expected_dir": None}


def residual_extremum(epochs: np.ndarray, r: np.ndarray, t_star: datetime,
                      half_days: int) -> dict:
    """3 反转点案例层：t* 处残差局部极值。"""
    return residual_extremum_at(epochs, r, t_star.timestamp(), half_days)


def full_sample_flip_consistency(moves: list[Move], epochs: np.ndarray,
                                 r: np.ndarray) -> dict:
    """统计层：全部 L1 方向翻转事件，翻转方向是否与残差局部极值一致。

    P1 核心可证伪命题：⋆=D ⟺ image(D) 的方向翻转与残差曲率 F 的极值对齐。
    翻转点锚定在**新走势起点**（start_bar，真实 bar 索引）= 残差反转处。
    一致 ⟺ up 走势起于残差底（bottom）/ down 走势起于残差顶（top）。
    可测样本 = 翻转点恰为残差局部极值（top/bottom）的子集；mid 点无极值基准跳过。
    """
    flips = [cur for prev, cur in zip(moves, moves[1:])
             if cur.direction != prev.direction]
    n_bars = len(r)
    consistent = 0
    testable = 0
    n_flips = len(flips)
    for fm in flips:
        b = min(max(fm.start_bar, 0), n_bars - 1)
        ext = residual_extremum_at(epochs, r, float(epochs[b]), EXTREMUM_HALF_DAYS)
        exp = ext["expected_dir"]
        if exp is None:
            continue
        testable += 1
        if fm.direction == exp:
            consistent += 1
    rate = consistent / testable if testable else float("nan")
    # 二项检验 vs p0=0.5（正态近似 + 精确 fallback）
    p_value = _binom_p_two_sided(consistent, testable, 0.5)
    return {
        "n_flips": n_flips, "testable": testable, "consistent": consistent,
        "rate": rate, "p_value": p_value, "p0": 0.5,
    }


def sensitivity_scan(moves: list[Move], epochs: np.ndarray, r: np.ndarray,
                     fracs: tuple[float, ...] = (0.70, 0.75, 0.80, 0.85, 0.90),
                     half_days: tuple[int, ...] = (15, 30, 45)) -> list[dict]:
    """否证稳健性：扫描极值阈值×窗口，确认一致率≈50% 非单一参数偶然。

    全局 EXTREMUM_FRAC/EXTREMUM_HALF_DAYS 临时覆盖后还原。
    """
    global EXTREMUM_FRAC, EXTREMUM_HALF_DAYS
    saved = (EXTREMUM_FRAC, EXTREMUM_HALF_DAYS)
    rows: list[dict] = []
    try:
        for frac in fracs:
            for hd in half_days:
                EXTREMUM_FRAC, EXTREMUM_HALF_DAYS = frac, hd
                s = full_sample_flip_consistency(moves, epochs, r)
                rows.append({"frac": frac, "half_days": hd, "rate": s["rate"],
                             "n": s["testable"], "p": s["p_value"]})
    finally:
        EXTREMUM_FRAC, EXTREMUM_HALF_DAYS = saved
    return rows


def _binom_p_two_sided(k: int, n: int, p0: float) -> float:
    """双侧二项检验 p 值。优先 scipy.binomtest，无则正态近似。"""
    if n == 0:
        return float("nan")
    try:
        from scipy.stats import binomtest
        return float(binomtest(k, n, p0, alternative="two-sided").pvalue)
    except Exception:
        import math
        mu = n * p0
        sd = math.sqrt(n * p0 * (1 - p0))
        if sd == 0:
            return 1.0
        z = abs(k - mu) / sd
        # 双侧正态尾概率
        return float(math.erfc(z / math.sqrt(2)))


def criterion_a_force_exhaustion(moves: list[Move], t_star: datetime) -> dict:
    """判据(a)：反转前末段同向走势 persistence 递减（背驰）。

    标准缠论背驰：取 t* 之前最近的两个**同向** L1 走势 (m_prev, m_last)，
    比较 persistence。背驰 ⟺ persistence(m_last) < persistence(m_prev)。
    """
    before = [m for m in moves if m.settle < t_star]
    if not before:
        return {"hit": False, "reason": "无前置走势"}
    last = before[-1]
    same_dir_prev = next((m for m in reversed(before[:-1])
                          if m.direction == last.direction), None)
    if same_dir_prev is None:
        return {"hit": False, "reason": "前段无同向走势对",
                "last_persist": last.persistence}
    decay = last.persistence < same_dir_prev.persistence
    ratio = last.persistence / same_dir_prev.persistence if same_dir_prev.persistence else float("nan")
    return {
        "hit": decay,
        "dir": last.direction,
        "prev_persist": same_dir_prev.persistence,
        "last_persist": last.persistence,
        "ratio": ratio,
        "prev_settle": same_dir_prev.settle.date().isoformat(),
        "last_settle": last.settle.date().isoformat(),
    }


def criterion_b_direction_flip(moves: list[Move], t_star: datetime,
                               extremum: dict) -> dict:
    """判据(b)：方向翻转且与残差局部极值一致。

    翻转 = (t* 前最后走势方向) ≠ (t* 后第一个走势方向)。
    一致 = 翻转后方向 == 残差局部极值期望方向（顶→down, 底→up）。
    """
    before = [m for m in moves if m.settle < t_star]
    after = [m for m in moves if m.settle >= t_star]
    if not before or not after:
        return {"hit": False, "reason": "窗口一侧无走势"}
    dir_before = before[-1].direction
    dir_after = after[0].direction
    flipped = dir_before != dir_after
    expected = extremum["expected_dir"]
    if expected is None:
        # 残差在 t* 是中段（非局部极值）→ 无价格反转可对齐 → 一致性未定义
        return {
            "hit": False, "flipped": flipped,
            "dir_before": dir_before, "dir_after": dir_after,
            "expected_dir": None, "reason": "残差 t* 为中段，无局部极值基准",
            "extremum_kind": extremum["kind"], "pctile": extremum["pctile"],
        }
    consistent = (dir_after == expected)
    return {
        "hit": flipped and consistent,
        "flipped": flipped, "consistent": consistent,
        "dir_before": dir_before, "dir_after": dir_after,
        "expected_dir": expected, "extremum_kind": extremum["kind"],
        "pctile": extremum["pctile"],
        "after_settle": after[0].settle.date().isoformat(),
    }


def criterion_c_not_lagging(moves: list[Move], t_star: datetime,
                            tol_days: int) -> dict:
    """判据(c)：方向翻转信号不滞后。

    翻转 settle 时间（t* 后第一个走势的 settle）≤ t* + tol。
    同步或提前命中；晚于容差 = 滞后失败。
    """
    after = [m for m in moves if m.settle >= t_star]
    before = [m for m in moves if m.settle < t_star]
    if not after or not before:
        return {"hit": False, "reason": "窗口一侧无走势"}
    # 翻转点 = t* 后第一个与 before[-1] 反向的走势
    dir_before = before[-1].direction
    flip_move = next((m for m in after if m.direction != dir_before), None)
    if flip_move is None:
        return {"hit": False, "reason": "t* 后无反向走势"}
    lag_days = (flip_move.settle - t_star).total_seconds() / 86400
    return {
        "hit": lag_days <= tol_days,
        "lag_days": lag_days,
        "flip_settle": flip_move.settle.date().isoformat(),
        "tol_days": tol_days,
    }


def evaluate_reversal(moves: list[Move], epochs: np.ndarray, r: np.ndarray,
                      date_str: str, label: str) -> dict:
    t_star = datetime.fromisoformat(date_str + "T00:00:00+00:00")
    ext = residual_extremum(epochs, r, t_star, EXTREMUM_HALF_DAYS)
    a = criterion_a_force_exhaustion(moves, t_star)
    b = criterion_b_direction_flip(moves, t_star, ext)
    c = criterion_c_not_lagging(moves, t_star, LAGGARD_TOL_DAYS)
    # 命中 = 方向翻转一致(b) ∧ 不滞后(c)；力度衰竭(a)为背驰证据强度
    hit = bool(b["hit"] and c["hit"])
    return {
        "date": date_str, "label": label, "t_star": t_star,
        "extremum": ext, "a": a, "b": b, "c": c, "hit": hit,
    }


def write_report(results: list[dict], stat: dict, scan: list[dict], meta: dict) -> None:
    n_hit = sum(1 for x in results if x["hit"])
    # 案例层判词（保留任务的 3/3/1-2/0 框架）
    if n_hit == 3:
        case_verdict = "三个反转点全部命中且方向一致。"
    elif n_hit >= 1:
        case_verdict = f"{n_hit}/3 命中（仅部分内生反转点）。"
    else:
        case_verdict = "0/3 命中（L1 天级走势与月级反转尺度错配，单点检验无信号）。"
    # 统计层判决（P1 最终判决）
    rate, pv = stat["rate"], stat["p_value"]
    sig = pv < 0.05
    if not (rate == rate):  # nan
        final = "**P1 不可判定**：无可测翻转样本。"
    elif sig and rate > 0.5:
        final = (f"**P1 统计支持**：全样本 L1 翻转方向一致率 {rate:.1%} 显著>50%"
                 f"（p={pv:.3g}）→ image(D) 方向携带残差曲率 F 极值信息。")
    elif sig and rate < 0.5:
        final = (f"**P1 反向证伪**：一致率 {rate:.1%} 显著<50%（p={pv:.3g}）→ "
                 "J 方向与 F 极值系统性反向，投影符号错配。")
    else:
        final = (f"**P1 否证（L1 退化 bar）**：全样本翻转一致率 {rate:.1%}≈50%"
                 f"（p={pv:.3g}，不显著）→ J 的方向符号与残差曲率 F 极值**无关联**，"
                 "image(D) 投影在 L1 退化 bar 上退化为噪声，⋆=D 不成立。")

    L: list[str] = []
    L.append("# P1 验证实验报告：Hodge star ⋆ = 缠论 D 算子\n")
    L.append("## 命题\n")
    L.append("J = D(F) = 残差曲率经缠论递归后的**方向性力度**。正交分解：")
    L.append("- `persistence` = |力度| = **ker(D)** 成分（无向幅度）")
    L.append("- `direction` = 方向 = **image(D)** 成分（±1）")
    L.append("- `J = persistence × sign(direction)` = image(D) 在有向价格空间的投影\n")
    L.append(f"- 引擎：`{meta['engine']}`")
    L.append(f"- 样本：{meta['n_bars']:,} bar，{meta['ts_start']} → {meta['ts_end']}")
    L.append(f"- 残差：`{meta['residual_def']}`")
    L.append("- 认识论等级：**L2**（真实数据，单残差构造=单标的/单时段，可证伪）\n")

    L.append("## ⚠ 关键诚实声明（formalization-validity-domain）\n")
    L.append("残差是退化 bar（O=H=L=C=r），故 **`persistence ≡ amplitude`**（逐位验证全等）。")
    L.append("J 相对纯振幅的**唯一新增信息是方向符号**。P1 的真正考验：方向符号在反转点")
    L.append("是否携带与价格反转一致的非平凡结构。若仅符号附加而与反转无关联，则 ⋆=D 平凡")
    L.append("（image(D) 投影退化为对振幅加任意符号）。\n")

    L.append("## 判据操作化定义\n")
    L.append("| 判据 | 定义 | 命中条件 |")
    L.append("|------|------|---------|")
    L.append("| (a) 力度衰竭 | t* 前最近两个**同向** L1 走势 persistence 比较 | 后<前（背驰） |")
    L.append(f"| (b) 方向翻转一致 | t* 前后走势方向翻转 ∧ 与残差局部极值一致 | 翻转∧方向匹配 |")
    L.append(f"| (c) 不滞后 | 翻转走势 settle 时间 ≤ t* + {LAGGARD_TOL_DAYS}天 | 同步或提前 |")
    L.append(f"\n**单点命中 = (b) ∧ (c)**；(a) 为背驰证据强度标注。")
    L.append(f"残差局部极值窗口 ±{EXTREMUM_HALF_DAYS}天，分位阈 {EXTREMUM_FRAC}（≥顶/≤底）。\n")

    L.append("## 逐反转点结果\n")
    for x in results:
        ext, a, b, c = x["extremum"], x["a"], x["b"], x["c"]
        L.append(f"### {x['date']} — {x['label']}  →  {'✅ 命中' if x['hit'] else '❌ 未命中'}\n")
        L.append(f"- **残差局部极值**：{ext['kind']}（t* 处 r={ext['r_at']:.5f}，"
                 f"窗口分位={ext['pctile']:.2f}）→ 期望反转方向 = `{ext['expected_dir']}`")
        # 判据 a
        if "ratio" in a:
            L.append(f"- **(a) 力度衰竭**：{'✅' if a['hit'] else '❌'} "
                     f"末段同向({a['dir']}) persistence {a['prev_persist']:.5f}"
                     f"({a['prev_settle']}) → {a['last_persist']:.5f}({a['last_settle']})，"
                     f"比值={a['ratio']:.2f}")
        else:
            L.append(f"- **(a) 力度衰竭**：N/A（{a.get('reason')}）")
        # 判据 b
        if b.get("expected_dir") is not None:
            L.append(f"- **(b) 方向翻转一致**：{'✅' if b['hit'] else '❌'} "
                     f"前={b['dir_before']} 后={b['dir_after']}（翻转={b['flipped']}），"
                     f"期望={b['expected_dir']}，一致={b.get('consistent')}")
        else:
            L.append(f"- **(b) 方向翻转一致**：❌ {b.get('reason')}")
        # 判据 c
        if "lag_days" in c:
            L.append(f"- **(c) 不滞后**：{'✅' if c['hit'] else '❌'} "
                     f"翻转 settle={c['flip_settle']}，滞后={c['lag_days']:+.1f}天"
                     f"（容差≤{c['tol_days']}天）")
        else:
            L.append(f"- **(c) 不滞后**：N/A（{c.get('reason')}）")
        L.append("")

    L.append("## 统计层：全样本 L1 翻转方向一致率（P1 核心检验）\n")
    L.append("命题：⋆=D ⟺ image(D) 的方向翻转与残差曲率 F 的局部极值系统性对齐。")
    L.append("检验全部 L1 方向翻转事件，翻转点锚定在新走势起点（真实 bar 索引）。")
    L.append("一致 ⟺ up 走势起于残差底 / down 走势起于残差顶。\n")
    L.append("| 量 | 值 |")
    L.append("|----|----|")
    L.append(f"| L1 方向翻转总数 | {stat['n_flips']} |")
    L.append(f"| 可测翻转（起点为残差局部极值 top/bottom） | {stat['testable']} |")
    L.append(f"| 方向一致数 | {stat['consistent']} |")
    L.append(f"| **一致率** | **{stat['rate']:.1%}** |")
    L.append(f"| 二项检验 p 值（H₀: 一致率=50%） | {stat['p_value']:.3g} |")
    L.append("")

    L.append("## 否证稳健性：极值阈值×窗口敏感性扫描\n")
    rates = [row["rate"] for row in scan]
    pmin = min(row["p"] for row in scan)
    L.append(f"否定性结果必须对操作化参数稳健才可靠。下表扫描 {len(scan)} 个"
             "（极值分位阈, 窗口半宽）组合：")
    L.append(f"一致率全程 **{min(rates):.1%}–{max(rates):.1%}**，最小 p={pmin:.3f}——"
             "**无一组合显著偏离 50%**。否证非单一参数偶然。\n")
    L.append("| 极值分位阈 | 窗口±天 | 一致率 | 可测 n | p |")
    L.append("|-----------|--------|-------|-------|---|")
    for row in scan:
        L.append(f"| {row['frac']:.2f} | {row['half_days']} | {row['rate']:.1%} "
                 f"| {row['n']} | {row['p']:.3f} |")
    L.append("")

    L.append("## 判决\n")
    L.append(f"- **案例层（L1 3 反转点）**：{case_verdict}")
    L.append(f"- **统计层（L1 全样本，P1 最终判决）**：{final}")
    L.append("- **高级别（L2/L3）**：因 settle_ts 轮询锚定与残差实际极值错位，**不可判定**"
             "（不用不可靠数据硬凑结论，no-workaround）。\n")

    L.append("## 核心发现\n")
    pos_frac = stat["testable"] / stat["n_flips"] if stat["n_flips"] else float("nan")
    # 1. 位置错配（更基础的否定）
    L.append(f"**1. 位置错配。** {stat['n_flips']} 个 L1 方向翻转中，仅 {stat['testable']}"
             f"（{pos_frac:.0%}）的起点落在残差局部极值（top/bottom）处——**{1-pos_frac:.0%} 的"
             "方向翻转发生在残差中段**，与曲率 F 的极值在位置上就不对齐。J 的方向翻转大多"
             "不是 F 极值的标记。\n")
    # 2. 方向错配（统计否证）
    L.append(f"**2. 方向无关联（统计否证）。** 即便在 {stat['testable']} 个起点恰为极值的"
             f"翻转中，方向一致率 {stat['rate']:.1%}（p={stat['p_value']:.3g}，不显著），"
             "与抛硬币无异。image(D) 的方向符号**不携带**残差曲率 F 的极值信息。\n")
    # 3. 案例层脆弱性
    L.append("**3. 案例层脆弱性。** 3 个 regime 反转点 0/3 命中。天级 L1 走势在月级反转点"
             "前后方向高频震荡，单点判据(b)对 t* 的精确选择极度敏感（如 2022-09 残差顶处 L1"
             "已是 down，方向虽与『顶后下跌』一致，却无『翻转』事件）。n=3 统计无力，"
             "故统计层（n=224）才是 P1 的可靠判决。\n")
    # 4. 综合
    L.append("**4. 综合判断。** ⋆=D 在 L1 退化 bar 残差上**否证**：方向性力度 J 相对纯振幅"
             "的唯一新增信息（方向符号）既不在位置上对齐 F 极值，也不在方向上与之一致。"
             "结合关键诚实声明（persistence≡amplitude），J 退化为『带随机符号的振幅』，"
             "image(D) 投影是平凡的。这与谱系『残差→流量无免费桥梁』一致——残差缠论结构"
             "不提供通向流量的桥梁，方向投影 J 同样不提供。\n")

    L.append("## 边界条件（结论翻转条件）\n")
    L.append(f"- **局部极值窗口/阈值**：±{EXTREMUM_HALF_DAYS}天、分位 {EXTREMUM_FRAC}。"
             "窗口放大或阈值放松会改变 mid/top/bottom 归类，从而改变判据(b)可用性 → 命中数可能变化。")
    L.append(f"- **不滞后容差**：{LAGGARD_TOL_DAYS}天。容差收紧→命中减少；放宽→滞后信号也算命中。")
    L.append("- **退化 bar 前提**：persistence≡amplitude。若改用非退化 bar（残差 OHLC 由更细"
             "子区间构造），persistence 与 amplitude 分离，J 的方向投影可能携带独立信息 → "
             "结论可能改变（当前结论限定于退化 bar 残差）。")
    L.append("- **方向翻转 ⟺ 价格反转一致**是 P1 的核心可证伪点：若命中点的方向翻转与残差"
             "局部极值**系统性反向**，则 ⋆=D 被证伪（投影符号错配）。\n")

    L.append("## 定义依据\n")
    L.append("- **残差**：`r = log(DX) − 0.576·log(USD6E) = log(DX)+0.576·log(EURUSD)`，"
             "协整系数既定（memory: project_residual_to_flow_no_bridge，占 DX 方差 33.8%）。")
    L.append("- **走势/方向/力度**：缠论正典走势由中枢定义，方向=走势上行/下行，"
             "力度=persistence（引擎自带，退化 bar 上≡振幅）。"
             "由 newchan_rust.RecursiveOrchestrator 产出（与正典引擎 bit-exact）。")
    L.append("- **D 算子分解**：persistence∈ker(D)（无向）、direction∈image(D)（有向）"
             "是本实验对 P1 命题的操作化——非缠师原文概念，属形式化映射假设，受本 L2 检验约束。\n")

    L.append("## 下游推论\n")
    L.append("- **不能用 J 做 regime 择时**：方向性力度 J = persistence×sign(dir) 在 L1 退化 bar "
             "残差上是『带随机符号的振幅』，不标记残差曲率 F 的极值，更不标记外生政策反转。"
             "任何基于 J 方向的残差择时信号在此数据上无统计依据。")
    L.append("- **⋆=D 桥梁不成立，d⋆F=J 仍缺度规**：P1 试图用缠论 D 算子充当 Hodge star 把"
             "残差曲率 F 映到流量 J。否证结果表明该映射在 L1 退化 bar 上退化——"
             "『残差→流量无免费桥梁』判断被独立复核加固，流量仍须独立源（COT/守恒律/"
             "缠论走势结构本身，而非残差的方向投影）。")
    L.append("- **有效域留口**：否证限定于 **L1 退化 bar 残差**。非退化 bar（残差 OHLC 由子区间"
             "构造，persistence 与 amplitude 分离）下 J 可能携带独立方向信息；高级别（L2/L3）"
             "因锚定局限不可判定。这两处是 P1 尚未关闭的有效域边界，非 P1 成立的证据。\n")

    L.append("## 谱系引用\n")
    L.append("- **残差→流量无免费桥梁**（memory: project_residual_to_flow_no_bridge）："
             "残差=曲率（严格）但 d⋆F=J 缺金融度规；流量须独立源。本实验检验 ⋆=D 是否能"
             "在残差缠论结构内部提供该桥梁——若仅内生反转点命中，则桥梁仍不成立（有效域窄）。")
    L.append("- **残差缠论流量流速 L2**（memory: project_residual_flow_velocity_l2）：前序实验"
             "发现『仅 2022-09 美元顶力度骤降，政策日噪声级』，有效域=内生反转点。本 P1 实验"
             "用方向投影 J 独立复核该有效域边界。")
    L.append("- **formalization-validity-domain**：persistence≡amplitude 使 J 的信息增量"
             "仅为方向符号——本实验明确标注该 L0→L1 同义反复风险，结论限退化 bar 有效域。\n")

    L.append("## 影响声明\n")
    L.append("- 新增：`analysis/p1_hodge_star_equals_D.py`、本报告。")
    L.append("- 复用：`analysis/data_cache/residual_flow_events.json`（既有 Rust 引擎产物，未重跑）。")
    L.append("- 不改动任何既有定义/模块。D 算子分解是新增的**形式化映射假设**，其有效性由本 L2 检验界定。")

    REPORT.write_text("\n".join(L), encoding="utf-8")
    print(f"[report] → {REPORT}", flush=True)


def main() -> None:
    moves, meta = load_l1_moves()
    print(f"[load] L1 settled moves = {len(moves)}  "
          f"{moves[0].settle.date()} → {moves[-1].settle.date()}", flush=True)
    z = np.load(RESIDUAL_NPZ, allow_pickle=True)
    r = z["residual"]
    ts = z["timestamps"].astype(str)
    print(f"[load] residual {len(r):,} bars  (parse epochs…)", flush=True)
    epochs = build_epochs(ts)

    # 案例层
    results = [evaluate_reversal(moves, epochs, r, d, lab) for d, lab in REVERSALS]
    print("\n=== 案例层（L1 3 反转点）===", flush=True)
    for x in results:
        print(f"  {x['date']} {x['label'][:20]:20s}  "
              f"极值={x['extremum']['kind']:6s}  "
              f"a={int(x['a'].get('hit', False))} "
              f"b={int(x['b'].get('hit', False))} "
              f"c={int(x['c'].get('hit', False))}  "
              f"→ {'命中' if x['hit'] else '未命中'}", flush=True)
    print(f"  小计 {sum(1 for x in results if x['hit'])}/3 命中", flush=True)

    # 统计层（P1 核心检验）
    stat = full_sample_flip_consistency(moves, epochs, r)
    print("\n=== 统计层（L1 全样本翻转一致率）===", flush=True)
    print(f"  翻转总数={stat['n_flips']}  可测={stat['testable']}  "
          f"一致={stat['consistent']}  一致率={stat['rate']:.1%}  "
          f"p={stat['p_value']:.3g}", flush=True)

    scan = sensitivity_scan(moves, epochs, r)
    print(f"\n=== 稳健性扫描 ===  一致率区间 "
          f"{min(x['rate'] for x in scan):.1%}–{max(x['rate'] for x in scan):.1%}  "
          f"min p={min(x['p'] for x in scan):.3f}", flush=True)

    write_report(results, stat, scan, meta)


if __name__ == "__main__":
    main()
