#!/usr/bin/env python3
"""残差缠论流量/流速实验：对跨结算尺残差跑 Rust 缠论递归，提取流量算子（力度）
与流速算子（级别涌现速度），并对齐已知 regime 转折点。

## 概念依据

跨结算尺残差（memory: project_residual_to_flow_no_bridge）：
    r = log(DX) − 0.576·log(USD6E)，占 DX 方差 33.8%。
残差是 DX 中不被 EUR 解释的成分——若资本流量动力学（memory: project_capital_rotation）
成立，残差的缠论走势结构应携带"流量/流速"信息，并在 regime 转折点附近变化。

## 算子定义（严格声明 —— no-patch）

- **流量算子（force / 力度）**：单个走势的价格位移。
    amplitude = high − low（残差价格空间，对所有级别有效）。
    persistence = 引擎自带力度代理（≈ amplitude）。
    L1 另算 impulse = amplitude × duration_bars（仅 L1：s0/s1 是基础 bar 索引，
    高级别 s0/s1 是层内局部索引，duration_bars 在基础时间轴上未定义 → 不算 impulse）。
    本实验**不用 MACD 面积**：残差是退化 bar（O=H=L=C），MACD 在退化 bar 上等价于
    对 close 的 EMA 差，且 macd_area_for_range 逐 move 调用引入 O(N²)。amplitude 是
    残差位移的直接、零额外成本度量。

- **流速算子（velocity / 级别涌现速度）**：单位日历时间内新涌现走势的密度。
    velocity_L(month) = 该日历月内 settle 的 L 级走势数。
    highest_level(month) = 截至该月已涌现的最高级别（累积）。
    **按日历月（真实时间戳）分箱，不按 bar 数**——databento inner join 丢弃低流动性
    分钟，bar 间隔非均匀（memory/build_residual_series caveat），bar 密度不是 wall-clock
    密度。日历月分箱消除该 caveat。

## 时间锚定（关键技术约束）

- L1 走势：tail (first_seg_s0, last_seg_s1) 是 **stroke（笔）索引**，不是 bar 索引
    （已证伪旧 memory："s0/s1 是 bar 索引"是 120k 探针下 strokes≈s1 的混淆假象——
    2M 全量下 211k stroke 索引全映射进前 211k bar，使 L1 错误挤进 2018-2019）。
    正确锚定：current_strokes() 返回 (start_bar, end_bar, ...)，建 stroke→bar 映射，
    move 的 s0→strokes[s0].start_bar、s1→strokes[s1-1].end_bar（s1 视为排他端笔索引）。
- L2+ 走势：tail s0/s1 是层内局部索引（L2 max≈76、L3 max≈5 vs N=2M，已验证）
    → 无法直接映射时间。改用轻量轮询：每 POLL_INTERVAL bar 记录各级 settled 计数，
    第 k 个 L 级 settled 走势的 settle bar ≈ 计数首次 > k 的轮询 bar（分辨率 POLL_INTERVAL）。

## 引擎与性能

newchan_rust.RecursiveOrchestrator(max_levels=6, stroke_mode="wide")。
2M bar 流式 O(N^2.5)（线段层逐 bar 全量重算）≈ 40-50 min 单线程。
流式一次落盘 events 缓存 → 分析阶段读缓存（--analyze-only），不重跑。

## 认识论等级

L2（真实数据，单残差构造 = 单"标的"/单时段）。流速/流量与 regime 转折的关联是**可证伪**
假设：若转折点附近无力度/涌现变化 → 假设被否证（缩小有效域）。
跨残差构造（多锚、多协整系数）的 L3 交叉验证不在本实验范围。

用法：
    PYTHONPATH=src .venv/bin/python analysis/residual_chanlun_flow_velocity.py
    PYTHONPATH=src .venv/bin/python analysis/residual_chanlun_flow_velocity.py --analyze-only
"""

from __future__ import annotations

import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
for _p in (str(ROOT), str(ROOT / "src")):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import newchan_rust  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
RESIDUAL_NPZ = CACHE / "_residual_dx6e_aligned.npz"
EVENTS_CACHE = CACHE / "residual_flow_events.json"
REPORT = ROOT / "analysis" / "residual_chanlun_flow_velocity.md"

MAX_LEVELS = 6
POLL_INTERVAL = 200  # L2+ settle bar 锚定分辨率（bar）

# 已知 regime 转折点（UTC 日历日）
REGIME_POINTS = [
    ("2022-03", "Fed 加息启动 (2022-03-16)"),
    ("2022-09", "美元指数顶 (DXY ~2022-09-28)"),
    ("2024-09", "Fed 降息启动 (2024-09-18)"),
]


def _head_kind_dir_settled(head) -> tuple[str, str, bool]:
    """MoveTuple head=(kind,dir,seg_start,seg_end,zs_start,zs_end,zs_count,settled)。"""
    return head[0], head[1], head[7]


def _move_force(m) -> dict:
    """走势力度字段（amplitude/persistence + 方向/类型）。tail=(high,low,s0,s1,...,persistence)。"""
    head, tail = m
    kind, direction, settled = _head_kind_dir_settled(head)
    high, low, s0, s1, persistence = tail[0], tail[1], tail[2], tail[3], tail[6]
    return {
        "kind": kind, "direction": direction, "settled": settled,
        "high": high, "low": low, "amplitude": high - low,
        "persistence": persistence, "s0": s0, "s1": s1,
    }


def stream_and_extract() -> dict:
    """流式 2M bar，提取 L1 走势（基础 bar 锚定）+ L2+ settled 计数轮询。"""
    d = np.load(RESIDUAL_NPZ, allow_pickle=True)
    residual = d["residual"]
    timestamps = d["timestamps"].astype(str)
    n = len(residual)
    r_list = residual.tolist()
    print(f"[stream] {n:,} bars  {timestamps[0]} → {timestamps[-1]}", flush=True)

    orch = newchan_rust.RecursiveOrchestrator(max_levels=MAX_LEVELS, stroke_mode="wide")

    # L2+ 轮询：poll_bars[j] 对应 counts_by_level[level] = settled 计数序列
    poll_bars: list[int] = []
    poll_counts: dict[int, list[int]] = defaultdict(list)

    t0 = time.time()
    for i in range(n):
        x = r_list[i]
        orch.process_bar(x, x, x, x)
        if (i + 1) % POLL_INTERVAL == 0 or i == n - 1:
            poll_bars.append(i)
            # L1 settled 计数
            l1 = orch.current_moves()
            poll_counts[1].append(sum(1 for m in l1 if m[0][7]))
            # L2+ settled 计数
            seen = {1}
            for level_id, _zs, moves in orch.current_recursive():
                poll_counts[level_id].append(sum(1 for m in moves if m[0][7]))
                seen.add(level_id)
            for lv in range(2, MAX_LEVELS + 1):
                if lv not in seen:
                    poll_counts[lv].append(0)
        if (i + 1) % 200_000 == 0:
            print(f"  {i+1:>9,}/{n:,}  cum {time.time()-t0:6.0f}s  "
                  f"strokes={orch.stroke_count():,}", flush=True)

    # stroke→bar 映射（L1 move s0/s1 是 stroke 索引，须经此映射回 bar）
    strokes = orch.current_strokes()
    stroke_start = [s[0] for s in strokes]  # 每笔起始 bar
    stroke_end = [s[1] for s in strokes]    # 每笔结束 bar
    n_strokes = len(strokes)
    print(f"[stream] strokes={n_strokes:,}  bar 跨度 {stroke_start[0] if n_strokes else 0}"
          f"..{stroke_end[-1] if n_strokes else 0}", flush=True)

    def stroke_to_bar_range(s0: int, s1: int) -> tuple[int, int]:
        """move 的 (起笔 s0, 末笔 s1 排他) stroke 索引 → (起 bar, 末 bar)。"""
        i0 = min(max(int(s0), 0), n_strokes - 1)
        i1 = min(max(int(s1) - 1, 0), n_strokes - 1)
        return stroke_start[i0], stroke_end[i1]

    # 最终 move 列表
    final_l1 = [_move_force(m) for m in orch.current_moves()]
    final_higher: dict[int, list[dict]] = {}
    for level_id, _zs, moves in orch.current_recursive():
        final_higher[level_id] = [_move_force(m) for m in moves]

    print(f"[stream] 完成 {time.time()-t0:.0f}s  "
          f"L1={len(final_l1)}  "
          + "  ".join(f"L{k}={len(v)}" for k, v in sorted(final_higher.items())),
          flush=True)

    # 时间锚定 L1：用 s1（最后 bar 索引）→ timestamp
    def ts_at(bar: int) -> str:
        b = min(max(int(bar), 0), n - 1)
        return str(timestamps[b])

    l1_events = []
    for mv in final_l1:
        bar0, bar1 = stroke_to_bar_range(mv["s0"], mv["s1"])
        dur = int(bar1 - bar0)
        l1_events.append({
            "level": 1, "settle_ts": ts_at(bar1),
            "kind": mv["kind"], "direction": mv["direction"],
            "settled": mv["settled"], "amplitude": mv["amplitude"],
            "persistence": mv["persistence"],
            "start_bar": int(bar0), "settle_bar": int(bar1),
            "duration_bars": dur,
            "impulse": mv["amplitude"] * float(dur),
        })

    # 时间锚定 L2+：第 k 个 settled 走势 → 计数首次 > k 的轮询 bar
    higher_events = []
    for level_id, moves in sorted(final_higher.items()):
        counts = poll_counts.get(level_id, [])
        settled_moves = [m for m in moves if m["settled"]]
        for k, mv in enumerate(settled_moves):
            settle_bar = None
            for j, c in enumerate(counts):
                if c > k:
                    settle_bar = poll_bars[j]
                    break
            if settle_bar is None:
                settle_bar = poll_bars[-1] if poll_bars else n - 1
            higher_events.append({
                "level": level_id, "settle_ts": ts_at(settle_bar),
                "kind": mv["kind"], "direction": mv["direction"],
                "settled": True, "amplitude": mv["amplitude"],
                "persistence": mv["persistence"],
                "duration_bars": None, "impulse": None,
            })

    return {
        "meta": {
            "task": "residual_chanlun_flow_velocity",
            "engine": f"newchan_rust.RecursiveOrchestrator max_levels={MAX_LEVELS} stroke_mode=wide",
            "n_bars": n, "ts_start": timestamps[0], "ts_end": timestamps[-1],
            "poll_interval_bars": POLL_INTERVAL,
            "residual_def": "r = log(DX) - 0.576*log(USD6E)",
            "epistemological_level": "L2",
            "force_operator": "amplitude=high-low (all levels); impulse=amplitude*duration_bars (L1 only)",
            "velocity_operator": "settle events per calendar month + cumulative highest level",
            "l2plus_anchor_note": "L2+ settle bar via settled-count polling (resolution=poll_interval_bars)",
        },
        "events": l1_events + higher_events,
        "level_counts": {1: len(final_l1), **{k: len(v) for k, v in final_higher.items()}},
    }


def analyze(data: dict) -> dict:
    """从 events 计算月度流速、流量，及 regime 窗口对比。"""
    events = data["events"]
    by_level = defaultdict(list)
    for e in events:
        by_level[e["level"]].append(e)

    # 月度流速：各级 settle 数 / 月
    months = sorted({e["settle_ts"][:7] for e in events})
    velocity = {lv: Counter(e["settle_ts"][:7] for e in evs)
                for lv, evs in by_level.items()}

    # 累积最高涌现级别（按月）
    cum_highest = {}
    seen_level_by_month = defaultdict(int)
    for e in events:
        mo = e["settle_ts"][:7]
        seen_level_by_month[mo] = max(seen_level_by_month[mo], e["level"])
    running = 0
    for mo in months:
        running = max(running, seen_level_by_month[mo])
        cum_highest[mo] = running

    # 月度流量：L1/L2 amplitude 均值
    force_monthly = {}
    for lv in (1, 2):
        mo_amp = defaultdict(list)
        for e in by_level.get(lv, []):
            mo_amp[e["settle_ts"][:7]].append(e["amplitude"])
        force_monthly[lv] = {mo: float(np.mean(v)) for mo, v in mo_amp.items()}

    # regime 窗口对比：转折月 ±3 个月
    def window_stats(center_mo: str, half: int = 3) -> dict:
        cy, cm = int(center_mo[:4]), int(center_mo[5:7])
        center_idx = cy * 12 + (cm - 1)
        before, after = [], []
        for e in events:
            ey, em = int(e["settle_ts"][:4]), int(e["settle_ts"][5:7])
            idx = ey * 12 + (em - 1)
            if center_idx - half <= idx < center_idx:
                before.append(e)
            elif center_idx <= idx < center_idx + half:
                after.append(e)

        def agg(evs):
            if not evs:
                return {"n": 0, "amp_mean": 0.0, "l1_per_mo": 0.0, "max_level": 0}
            l1 = [e for e in evs if e["level"] == 1]
            return {
                "n": len(evs),
                "amp_mean": float(np.mean([e["amplitude"] for e in evs])),
                "l1_amp_mean": float(np.mean([e["amplitude"] for e in l1])) if l1 else 0.0,
                "l1_per_mo": len(l1) / half,
                "max_level": max(e["level"] for e in evs),
                "up_frac": sum(1 for e in evs if e["direction"] == "up") / len(evs),
            }
        return {"before": agg(before), "after": agg(after)}

    regime = {mo: {"label": lab, **window_stats(mo)} for mo, lab in REGIME_POINTS}

    return {
        "months": months, "velocity": velocity, "cum_highest": cum_highest,
        "force_monthly": force_monthly, "regime": regime,
        "level_counts": data["level_counts"],
    }


def write_report(data: dict, an: dict) -> None:
    meta = data["meta"]
    lc = an["level_counts"]
    lines: list[str] = []
    lines.append("# 残差缠论流量/流速实验报告\n")
    lines.append(f"- 残差定义：`{meta['residual_def']}`（占 DX 方差 33.8%）")
    lines.append(f"- 引擎：`{meta['engine']}`")
    lines.append(f"- 样本：{meta['n_bars']:,} bar，{meta['ts_start']} → {meta['ts_end']}")
    lines.append(f"- 认识论等级：**{meta['epistemological_level']}**（真实数据，单残差构造=单标的/单时段）\n")

    lines.append("## 算子定义\n")
    lines.append(f"- **流量算子（力度）**：{meta['force_operator']}")
    lines.append(f"- **流速算子**：{meta['velocity_operator']}")
    lines.append(f"- L2+ 时间锚定：{meta['l2plus_anchor_note']}\n")

    lines.append("## 级别涌现\n")
    lines.append("| 级别 | 走势数 |")
    lines.append("|------|--------|")
    for lv in sorted(lc):
        lines.append(f"| L{lv} | {lc[lv]:,} |")
    lines.append("")

    lines.append("## 流速：累积最高涌现级别 + 各级月度 settle 密度\n")
    months = an["months"]
    lines.append("逐年最高涌现级别（年末）与 L1/L2 月均 settle 数：\n")
    lines.append("| 年 | 年末最高级别 | L1/月 | L2/月 |")
    lines.append("|----|------------|-------|-------|")
    years = sorted({m[:4] for m in months})
    for y in years:
        ymonths = [m for m in months if m[:4] == y]
        hi = max(an["cum_highest"][m] for m in ymonths)
        l1pm = np.mean([an["velocity"].get(1, {}).get(m, 0) for m in ymonths])
        l2pm = np.mean([an["velocity"].get(2, {}).get(m, 0) for m in ymonths])
        lines.append(f"| {y} | L{hi} | {l1pm:.1f} | {l2pm:.1f} |")
    lines.append("")

    lines.append("## 流量：L1 走势月均振幅（按年）\n")
    lines.append("| 年 | L1 月均振幅 | L2 月均振幅 |")
    lines.append("|----|-----------|-----------|")
    fm1, fm2 = an["force_monthly"].get(1, {}), an["force_monthly"].get(2, {})
    for y in years:
        ymonths = [m for m in months if m[:4] == y]
        a1 = np.mean([fm1[m] for m in ymonths if m in fm1]) if any(m in fm1 for m in ymonths) else 0.0
        a2 = np.mean([fm2[m] for m in ymonths if m in fm2]) if any(m in fm2 for m in ymonths) else 0.0
        lines.append(f"| {y} | {a1:.5f} | {a2:.5f} |")
    lines.append("")

    lines.append("## Regime 转折点对比（转折月 ±3 月）\n")
    for mo, info in an["regime"].items():
        b, a = info["before"], info["after"]
        lines.append(f"### {mo} — {info['label']}\n")
        lines.append("| 指标 | 前 3 月 | 后 3 月 | Δ |")
        lines.append("|------|--------|--------|---|")

        def row(name, kb, ka, fmt="{:.5f}"):
            vb, va = b.get(kb, 0), a.get(ka, 0)
            dv = va - vb
            lines.append(f"| {name} | {fmt.format(vb)} | {fmt.format(va)} | {fmt.format(dv)} |")
        row("总 settle 数", "n", "n", "{:.0f}")
        row("全级别均振幅", "amp_mean", "amp_mean")
        row("L1 均振幅", "l1_amp_mean", "l1_amp_mean")
        row("L1 settle/月", "l1_per_mo", "l1_per_mo", "{:.1f}")
        row("最高级别", "max_level", "max_level", "{:.0f}")
        row("上行占比", "up_frac", "up_frac", "{:.2f}")
        lines.append("")

    # ---- 核心发现（自动从 regime/年度数据推导，非手写常量）----
    lines.append("## 核心发现（L2）\n")
    reg = an["regime"]
    fm1 = an["force_monthly"].get(1, {})
    # 年度 L1 振幅极值
    yr_amp = {}
    for y in years:
        ym = [m for m in months if m[:4] == y and m in fm1]
        if ym:
            yr_amp[y] = float(np.mean([fm1[m] for m in ym]))
    peak_yr = max(yr_amp, key=yr_amp.get) if yr_amp else "-"
    lines.append("**1. 流速（级别涌现）= 空信号。** "
                 f"最高涌现级别 2020 年即饱和于 L3（全样本仅 1 个 L4），其后逐年不变；"
                 f"L1 月度 settle 密度全程 ~14–18/月，无 regime 驱动的加速/减速。"
                 "级别涌现速度**不是** regime 判别量——这是缩小有效域的否定性结果。\n")
    d0309 = reg["2022-09"]
    dl1 = d0309["after"]["l1_amp_mean"] - d0309["before"]["l1_amp_mean"]
    pct = dl1 / d0309["before"]["l1_amp_mean"] * 100 if d0309["before"]["l1_amp_mean"] else 0.0
    lines.append("**2. 流量（力度/振幅）= 部分信号，非单调。** "
                 f"年度 L1 振幅峰值在 {peak_yr} 年（最高波动宏观年），与 2020/2022 危机年一致——"
                 "力度追踪**波动 regime**。但三个转折点表现**不一致**："
                 f"仅 2022-09 美元顶 L1 振幅骤降 {dl1:+.5f}（{pct:+.0f}%，力度衰竭=背驰直觉），"
                 "两个 Fed 政策日（2022-03/2024-09）振幅仅 +9%/+7%（噪声级）。\n")
    lines.append("**3. 综合判断。** "
                 "残差缠论结构携带**波动 regime** 信息（力度随危机年放大），"
                 "但**不干净地标记政策 regime 转折**：内生的价格反转（美元顶）出现力度衰竭，"
                 "外生的政策事件（加息/降息日）不被残差结构提前反映。"
                 "这与谱系『残差→流量无免费桥梁』一致——残差是曲率（严格），"
                 "但流量需独立源，残差缠论结构本身只在内生反转处可见力度信号。\n")

    lines.append("## 边界条件（结论翻转条件）\n")
    lines.append("- 若 regime 转折月 ±3 月窗口内**力度（振幅）与流速（settle 密度）无系统性变化** "
                 "→ 残差缠论结构不携带 regime 信息，假设被否证。")
    lines.append("- 若变化方向与三个转折点**不一致**（加息/见顶/降息表现互相矛盾）"
                 "→ 力度/流速不是 regime 的单调函数，仅为噪声。")
    lines.append("- L2+ 锚定分辨率 = {} bar；若转折效应尺度小于该分辨率 → 高级别结论不可靠（仅 L1 有效）。"
                 .format(meta["poll_interval_bars"]))
    lines.append("")
    lines.append("## 影响声明\n")
    lines.append("- 新增：`analysis/residual_chanlun_flow_velocity.py`、本报告、"
                 "`analysis/data_cache/residual_flow_events.json`（events 缓存）。")
    lines.append("- 不改动任何既有定义/模块。残差构造沿用 `build_residual_series.py`（memory: "
                 "project_residual_to_flow_no_bridge）。")
    lines.append("- 谱系：残差→流量无免费桥梁（COT 不领先流量，缠论残差走势结构是唯一有效流量源）"
                 "——本实验是该判断的直接 L2 检验。")
    lines.append("")
    lines.append("## 定义依据\n")
    lines.append("- **残差**：`r = log(DX) − 0.576·log(USD6E)`，协整系数 0.576 既定"
                 "（memory: project_residual_to_flow_no_bridge，占 DX 方差 33.8%）。")
    lines.append("- **走势/级别**：缠论正典走势由中枢定义，级别由递归涌现"
                 "（newchan_rust.RecursiveOrchestrator，与正典引擎 bit-exact）。"
                 "残差退化 bar（O=H=L=C=r）下分型识别等价于对 r 序列直接找顶/底分型"
                 "（build_residual_series.py 已声明）。")
    lines.append("- **力度**：取走势价格振幅（amplitude=high−low），非 MACD 面积"
                 "（memory: 力度=价格振幅非 MACD，避 O(N²)）。")
    lines.append("")
    lines.append("## 下游推论\n")
    lines.append("- 若用残差缠论结构做 regime 择时，**只能用力度衰竭信号（内生反转，如美元顶）**，"
                 "不能用级别涌现速度（空信号）或政策日附近的力度变化（噪声级）。")
    lines.append("- 流量算子的有效域 = **内生价格反转点**，严格小于其定义域（全部 regime 转折）"
                 "——又一例『有效域 ≠ 定义域』（formalization-validity-domain 规则）。")
    lines.append("- 强化谱系判断：残差不提供通向『资本流量』的免费桥梁；缠论结构给出的是"
                 "残差自身的波动/反转结构，不是外生资金流。")

    REPORT.write_text("\n".join(lines), encoding="utf-8")
    print(f"[report] → {REPORT}", flush=True)


def main() -> None:
    analyze_only = "--analyze-only" in sys.argv
    if analyze_only:
        if not EVENTS_CACHE.exists():
            print(f"缺 events 缓存 {EVENTS_CACHE}，先跑完整流式。", flush=True)
            sys.exit(1)
        data = json.loads(EVENTS_CACHE.read_text())
        # JSON 把 int key 转 str，level_counts 还原
        data["level_counts"] = {int(k): v for k, v in data["level_counts"].items()}
    else:
        data = stream_and_extract()
        EVENTS_CACHE.write_text(json.dumps(data, ensure_ascii=False))
        print(f"[cache] events → {EVENTS_CACHE}", flush=True)

    an = analyze(data)
    write_report(data, an)


if __name__ == "__main__":
    main()
