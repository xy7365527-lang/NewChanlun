"""路径2：更大 base 周期（5min/30min）下有机赋格 v2 Rust 交易层重跑。

假设：更大 base 周期 → 同样行情结构在更少 bar 内展开 → 递归涌现到更高
ladder → REV tranche（C4/C5）获得非空定义域（1min 三标的 n_rev_tranche_adds
全 0：entry≤LADDER_SEG 无升级空间）。
反向力量：bar 数变为 1/k，结构事件总数下降——涌现级别集合作为直接证据记录。

聚合口径（用户指定）：纯顺序分块，每 k 根 1min 合成 1 根（open=首根 open /
high=max / low=min / close=末根 close），尾部不足 k 根丢弃，不做时间戳对齐。
years 同步分块（取块首 bar 的日历年）——日历跨度不变，by_year 口径一致。

管线照搬 organic_fugue_rust_backtest.process_symbol：
  compute_organic_signals(+dir_flips) → pack_tape → nr.run_organic_rust
  （O0 / V1 / V1f / V1r）+ P5 Python 基线（run_version_i）。
O0≡P5 守卫：Rust O0 trades 与 Python P5 trades 全 8 字段逐笔对账——
聚合数据上不一致则如实记录为矛盾（不修实现），变体仍照跑并标注。

BT_TF=1 为涌现观测专用模式：只跑信号层 + 涌现级别观测（1min 交易数字
直接采用在册 organic_fugue_rust_backtest.json，不重复跑交易层）。

认识论等级：L2（三标的真实数据，单一聚合口径；可产生否定性结果）。

用法：
  PYTHONPATH=src .venv/bin/python analysis/rev_tranche_path2_coarse.py
  env：BT_SYMBOLS=OKLO,QQQ,BRN  BT_TF=5,30（可含 1=观测模式）  BT_FORCE=1
输出：
  analysis/data_cache/rev_tranche_path2_coarse.json（增量落盘，键 SYM@TF）
  analysis/_rev_tranche_path2_section.md（报告节）
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_alpha_diagnosis import CompletedTrade  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import (  # noqa: E402
    LADDER_SEG,
    MAX_LADDER,
    PAIRING_VARIANTS,
    extended_metrics,
    run_version_i,
)
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "rev_tranche_path2_coarse.json"
OUT_MD = ROOT / "analysis" / "_rev_tranche_path2_section.md"
REF_1MIN_JSON = DATA_DIR / "organic_fugue_rust_backtest.json"  # 1min 在册对照

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
TFS = [int(t) for t in os.environ.get("BT_TF", "5,30").split(",")]
REV_VARIANTS = ["V1", "V1f", "V1r"]
FLOOR = LADDER_SEG


# ════════════════════════════════════════════════════════════
# 聚合（纯顺序分块，用户指定口径）
# ════════════════════════════════════════════════════════════

def aggregate_bars(opens: list[float], highs: list[float], lows: list[float],
                   closes: list[float], years: list[int] | None, k: int):
    """每 k 根合成 1 根；尾部不足 k 根丢弃；years 取块首 bar 年份。"""
    if k == 1:
        return opens, highs, lows, closes, years
    n_chunks = len(closes) // k
    o_out: list[float] = []
    h_out: list[float] = []
    l_out: list[float] = []
    c_out: list[float] = []
    y_out: list[int] = []
    for j in range(n_chunks):
        a, b = j * k, (j + 1) * k
        o_out.append(opens[a])
        h_out.append(max(highs[a:b]))
        l_out.append(min(lows[a:b]))
        c_out.append(closes[b - 1])
        if years is not None:
            y_out.append(years[a])
    return o_out, h_out, l_out, c_out, (y_out if years is not None else None)


# ════════════════════════════════════════════════════════════
# 涌现级别观测（关键观测量：tape 中 ladder 键集合 + dir_flips 行）
# ════════════════════════════════════════════════════════════

def emergence_obs(tape, dir_flips: list) -> dict:
    bsp_c = [0] * MAX_LADDER
    div_c = [0] * MAX_LADDER
    settled_c = [0] * MAX_LADDER
    buy1_c = [0] * MAX_LADDER
    sell1_c = [0] * MAX_LADDER
    mx = 0
    for s in tape:
        if s.max_ladder > mx:
            mx = s.max_ladder
        if s.bsp_events:
            for lad in range(MAX_LADDER):
                if s.bsp_events[lad]:
                    bsp_c[lad] += len(s.bsp_events[lad])
        if s.div_events:
            for lad in range(MAX_LADDER):
                if s.div_events[lad]:
                    div_c[lad] += len(s.div_events[lad])
        if s.up_move_settled:
            for lad in range(MAX_LADDER):
                if s.up_move_settled[lad]:
                    settled_c[lad] += 1
        for lad in range(MAX_LADDER):
            if s.buy1[lad]:
                buy1_c[lad] += 1
            if s.sell1[lad]:
                sell1_c[lad] += 1
    flip_c = Counter(lad for _bar, lad, _d in dir_flips)
    return {
        "final_max_ladder": mx,
        "bsp_ladders": {str(l): c for l, c in enumerate(bsp_c) if c},
        "div_ladders": {str(l): c for l, c in enumerate(div_c) if c},
        "up_settled_ladders": {str(l): c for l, c in enumerate(settled_c) if c},
        "buy1_ladders": {str(l): c for l, c in enumerate(buy1_c) if c},
        "sell1_ladders": {str(l): c for l, c in enumerate(sell1_c) if c},
        "dir_flip_ladders": {str(l): flip_c[l] for l in sorted(flip_c)},
        "n_dir_flips": len(dir_flips),
    }


# ════════════════════════════════════════════════════════════
# Rust 交易层（口径与 organic_fugue_rust_backtest 逐字一致）
# ════════════════════════════════════════════════════════════

def _trades_from_rust(rows: list) -> list[CompletedTrade]:
    return [CompletedTrade(entry_bar=r[0], entry_price=r[1], exit_bar=r[2],
                           exit_price=r[3], pnl_pct=r[4], exit_reason=r[5],
                           n_short_diffs=r[6], cost_basis_at_exit=r[7])
            for r in rows]


def _leg_stats_rust(diag: list) -> dict:
    """Rust diag → 按腿类型聚合（口径与 organic_fugue_backtest._leg_stats 一致）。"""
    by_leg: dict[str, list] = {"main": [], "osc": [], "rev": []}
    for _header, diffs in diag:
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1) in diffs:
            by_leg[leg].append((sp, df, pf))
    out = {}
    for leg, rows in by_leg.items():
        n = len(rows)
        if n == 0:
            out[leg] = {"n_pairs": 0, "win_rate": None, "avg_diff_pct": None,
                        "net_cash": 0.0}
            continue
        out[leg] = {
            "n_pairs": n,
            "win_rate": round(sum(1 for sp, df, pf in rows if df > 0) / n * 100, 1),
            "avg_diff_pct": round(
                sum(df / sp for sp, df, pf in rows) / n * 100, 4),
            "net_cash": round(sum(pf for _sp, _df, pf in rows), 1),
        }
    return out


def _o0_vs_p5_guard(trades_p5: list[CompletedTrade], rtape) -> dict:
    """Rust O0 trades 与 Python P5 trades 全 8 字段对账。

    聚合数据上不一致 → 如实记录为矛盾（status=FAIL + 首个分歧），不修实现。
    """
    res = nr.run_organic_rust(rtape, "O0", floor_ladder=FLOOR,
                              stop_mode="none", diag=False)
    rust_trades = res["trades"]
    if len(trades_p5) != len(rust_trades):
        return {"status": "FAIL",
                "detail": f"笔数不等 P5={len(trades_p5)} RustO0={len(rust_trades)}"}
    fields = ("entry_bar", "entry_price", "exit_bar", "exit_price", "pnl_pct",
              "exit_reason", "n_short_diffs", "cost_basis_at_exit")
    for ti, (tp, tr) in enumerate(zip(trades_p5, rust_trades)):
        py_tuple = (tp.entry_bar, tp.entry_price, tp.exit_bar, tp.exit_price,
                    tp.pnl_pct, tp.exit_reason, tp.n_short_diffs,
                    tp.cost_basis_at_exit)
        if py_tuple != tuple(tr):
            bad = [f"{f}: P5={a!r} RustO0={b!r}"
                   for f, a, b in zip(fields, py_tuple, tuple(tr)) if a != b]
            return {"status": "FAIL",
                    "detail": f"trade#{ti} 字段分歧 " + "; ".join(bad)}
    return {"status": "PASS", "n_trades": len(trades_p5)}


COUNTER_KEYS = ("n_rev_attempts", "n_rev_sub_anchor_rejects",
                "n_rev_frozen_rejects", "n_rev_open",
                "n_rev_close_t5", "n_rev_close_t6", "n_rev_close_t7",
                "n_rev_struct_close", "n_rev_tranche_adds",
                "n_rev_tranche_closes", "n_open_rejects_zero")


def process(symbol: str, tf: int) -> dict:
    print(f"\n{'=' * 64}\n  {symbol} @ {tf}min — 路径2 聚合重跑\n{'=' * 64}",
          flush=True)
    o1, h1, l1, c1, y1 = load_ohlc(SYMBOL_FILES[symbol])
    opens, highs, lows, closes, years = aggregate_bars(o1, h1, l1, c1, y1, tf)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  1min bars={len(c1):,} → {tf}min bars={n:,}  BH={bh:+.2f}%",
          flush=True)

    # ── 信号层（单 pass：磁带 + D3 方向行）──
    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    obs = emergence_obs(tape, dir_flips)
    print(f"  信号层 {sig_s:.1f}s  D3 翻转 {len(dir_flips):,}"
          f"  max_ladder={obs['final_max_ladder']}"
          f"  flip_ladders={obs['dir_flip_ladders']}", flush=True)

    out: dict = {"symbol": symbol, "tf_min": tf, "n_bars": n,
                 "bh": round(bh, 2), "sig_elapsed": round(sig_s, 1),
                 "emergence": obs}

    if tf == 1:  # 涌现观测专用模式：交易数字采用在册 1min 结果
        print("  [TF=1 观测模式] 跳过交易层（在册数字见 "
              "organic_fugue_rust_backtest.json）", flush=True)
        return out

    rtape = pack_tape(tape, dir_flips=dir_flips)

    # ── P5 基线（Python oracle）──
    t0 = time.time()
    trades_p5, _ = run_version_i(tape, floor_ladder=FLOOR,
                                 pairing=PAIRING_VARIANTS["P5"])
    m_p5 = extended_metrics(trades_p5, years)
    p5c = m_p5["total_compound"]
    print(f"  [P5 ] 交易={m_p5['n']:4d} 复利={p5c:+10.2f}%"
          f" [{time.time() - t0:.1f}s]", flush=True)
    out["P5"] = {"metrics": {k: m_p5[k] for k in
                             ("n", "win_rate", "total_compound",
                              "sharpe", "max_dd")}}

    # ── O0≡P5 守卫（不一致 → 记录矛盾，变体照跑并标注）──
    guard = _o0_vs_p5_guard(trades_p5, rtape)
    out["o0_guard"] = guard
    print(f"  [O0≡P5] {guard['status']}"
          + (f"  {guard.get('detail', '')}" if guard["status"] != "PASS" else
             f"  {guard['n_trades']} 笔"), flush=True)

    # ── REV 变体矩阵 ──
    out["variants"] = {}
    for name in REV_VARIANTS:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=FLOOR,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = _trades_from_rust(res["trades"])
        m = extended_metrics(trades, years)
        legs = _leg_stats_rust(res["diag"])
        c = res["counters"]
        n_close = (c["n_rev_close_t5"] + c["n_rev_close_t6"]
                   + c["n_rev_close_t7"] + c["n_rev_struct_close"])
        t6_share = (round(c["n_rev_close_t6"] / n_close * 100, 1)
                    if n_close else None)
        pack = {
            "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                          "sharpe", "max_dd")},
            "delta_vs_p5": round(m["total_compound"] - p5c, 1),
            "leg_stats": legs,
            "counters": {k: c[k] for k in COUNTER_KEYS},
            "t6_close_share_pct": t6_share,
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        print(f"  [{name:3s}] 交易={m['n']:4d} 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:5d} G1拒={c['n_rev_sub_anchor_rejects']:5d}"
              f" tranche加={c['n_rev_tranche_adds']:4d}"
              f" t6占比={t6_share}% rev净现金={legs['rev']['net_cash']:+.0f}"
              f" [{el:.3f}s]", flush=True)
    return out


# ════════════════════════════════════════════════════════════
# 报告节
# ════════════════════════════════════════════════════════════

# 1min 在册对照（organic_fugue_rust_backtest.md，base 周期不同不可比绝对收益）
REF_1MIN = {
    "OKLO": {"P5": 740.9, "V1f_delta": 607.5, "V1r_delta": 607.5},
    "QQQ": {"P5": 108.2, "V1f_delta": -9.3, "V1r_delta": -9.3},
    "BRN": {"P5": 488.4, "V1f_delta": -125.6, "V1r_delta": -125.6},
}


def _ladder_set(d: dict) -> str:
    return "{" + ",".join(sorted(d, key=int)) + "}" if d else "∅"


def write_report(results: dict) -> None:
    ref = {}
    if REF_1MIN_JSON.exists():
        ref = json.loads(REF_1MIN_JSON.read_text())
    L: list[str] = []
    L.append("## 路径2：更大 base 周期（5min/30min）\n")
    L.append("> 假设：更大 base 周期 → 递归涌现到更高 ladder → REV tranche"
             "（C4/C5 递归建仓）获得非空定义域（1min 三标的 tranche加全 0）。"
             "聚合口径 = 纯顺序分块（每 k 根 1min 合 1 根，尾部丢弃，无时间戳"
             "对齐）。**绝对收益不可与 1min 直接比较（base 周期变了）——"
             "重点是 Δ(vs P5) 符号与 tranche 激活。**\n")
    L.append("> 认识论等级：**L2**（OKLO/QQQ/BRN 三标的真实数据，单一聚合口径"
             "5min/30min 两点采样）。\n")

    # ── 主结果表 ──
    L.append("### 结果表（每 (标的,周期) × 变体）\n")
    L.append("| 标的@TF | bars | O0≡P5 | BH% | P5复利% | 变体 | 复利% | "
             "**Δ(vs P5) pp** | 1min Δ对照 pp | rev开 | G1拒 | **tranche加/关** | "
             "t6占比% | rev对数 | rev净现金 |")
    L.append("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    for key, r in sorted(results.items()):
        if r["tf_min"] == 1 or "variants" not in r:
            continue
        sym = r["symbol"]
        p5c = r["P5"]["metrics"]["total_compound"]
        g = r["o0_guard"]["status"]
        first = True
        for v, p in r["variants"].items():
            c = p["counters"]
            rev = p["leg_stats"]["rev"]
            ref_d = REF_1MIN.get(sym, {}).get(f"{v}_delta", "—") \
                if v in ("V1f", "V1r") else "—"
            head = (f"| {key} | {r['n_bars']:,} | {g} | {r['bh']:+.1f} | "
                    f"{p5c:+.1f} " if first else "| | | | | ")
            L.append(
                head + f"| {v} | {p['metrics']['total_compound']:+.1f} | "
                f"**{p['delta_vs_p5']:+.1f}** | {ref_d} | {c['n_rev_open']} | "
                f"{c['n_rev_sub_anchor_rejects']} | "
                f"**{c['n_rev_tranche_adds']}/{c['n_rev_tranche_closes']}** | "
                f"{p['t6_close_share_pct']} | {rev['n_pairs']} | "
                f"{rev['net_cash']:+.0f} |")
            first = False
    L.append("")

    # ── 涌现级别对照 ──
    L.append("### 涌现级别对照（直接证据：bar 数 ↓ vs 结构变大的角力）\n")
    L.append("| 标的@TF | bars | max_ladder | BSP事件 ladder集 | 背驰 ladder集 | "
             "settled ladder集 | D3翻转 ladder分布 | D3总行数 |")
    L.append("|---|---|---|---|---|---|---|---|")
    for key, r in sorted(results.items()):
        e = r["emergence"]
        flips = ", ".join(f"L{l}:{c}" for l, c in e["dir_flip_ladders"].items())
        L.append(f"| {key} | {r['n_bars']:,} | {e['final_max_ladder']} | "
                 f"{_ladder_set(e['bsp_ladders'])} | "
                 f"{_ladder_set(e['div_ladders'])} | "
                 f"{_ladder_set(e['up_settled_ladders'])} | {flips} | "
                 f"{e['n_dir_flips']:,} |")
    # 1min 在册 D3 行数（per-ladder 分布须 TF=1 观测模式补采）
    for sym in ("OKLO", "QQQ", "BRN"):
        if f"{sym}@1" not in results and sym in ref:
            L.append(f"| {sym}@1（在册） | {ref[sym]['n_bars']:,} | — | — | — | — "
                     f"| — | {ref[sym]['n_dir_flips']:,} |")
    L.append("")

    # ── 判决（数据已封闭后由本函数静态产出；数字以上表为准）──
    L.append("""### tranche 判决：假设被否证（否定性结果）

**n_rev_tranche_adds = 0，全部 6 个 (标的,周期) 组合。** 更大 base 周期没有
给 REV tranche 提供定义域。

**机制级根因**（`rust/src/trading/runner.rs:330-345` + `level_operating_unit.rs:410`）：
tranche 目标区间 = `(rev home k, entry_ladder−1]`，其中 `entry_ladder` = 进场
bar 上最高的 confirmed type1 buy（buy1）ladder，voices 域 = `[floor=2, entry)`。
三标的 × 三周期（含 1min 观测 pass）的 buy1 ladder 集合**恒为 {2,3}**——
confirmed type1 买点从未浮出到 ladder≥4（recursive 层有 bsp/div 事件，如
OKLO@1 L4 有 58 个 bsp 事件，但其中无一以 confirmed type1 buy 形态进入 buy1
掩码）。entry≤3 → voices={2} → cap=entry−1≤2 → 区间 (2,2]=∅。**tranche 定义域
的空性是 buy1≤3 的定理，与 base 周期无关**——本实验把"1min 特异"升级为
"1/5/30min 三点上的不变事实"。

### 涌现级别判决：聚合没有抬高涌现，反而单调不增

max_ladder（1min→5min→30min）：OKLO 5→5→4；QQQ 5→5→4；BRN 6→6→5。
D3 翻转行数同比例缩水（OKLO 24,417→4,943→759）；每-bar 翻转密度近似守恒
（OKLO ≈5.5%/5.5%/5.1%）——顺序分块聚合大致保持每-bar 结构密度，因此
bar 数 ÷k 直接把绝对结构事件数 ÷k。"行情结构更大"的正向力量未观测到：
**反向力量（bar 数下降）完全主导**。BRN@1 D3=117,575 与在册逐数一致
（观测 pass 与在册管线自洽）。

### Δ(vs P5) 符号对照：V1f 正 Δ 的有效域进一步收缩

| 标的 | 1min Δ(V1f) | 5min Δ(V1f) | 30min Δ(V1f) |
|------|------------|------------|-------------|
| OKLO | **+607.5** | −306.2 | −12.9 |
| QQQ  | −9.3 | −61.2 | −8.3 |
| BRN  | −125.6 | −14.6 | −27.8 |

6/6 个聚合组合 Δ(V1f) 全负。OKLO 1min 的唯一正例不随 base 周期迁移——
V1f 的正 Δ 是 (标的×分辨率) 特异的，不是 REV 框架的鲁棒性质。
V1r ≡ V1f 处处成立（tranche 从未激活，C4/C5 代码路径空转）。

### 守卫与边界条件

- **O0≡P5**：6/6 PASS（Rust O0 trades 与 Python P5 trades 全 8 字段逐笔
  一致）——聚合数据上无矛盾。
- **聚合口径边界**：纯顺序分块（用户指定），无时间戳对齐——跨日/跨 session
  的 bar 会混入同块。若改用日历对齐聚合，K 线形态会变，结论原则上可翻转；
  但涌现层级由结构（分型/笔/段）驱动，预期对此不敏感（未验证）。
- **TF 采样边界**：仅 5/30 两点 + 1min 在册。不能外推"不存在任何 TF 使
  buy1≥4"；判决翻转条件 = 某 TF/某历史段出现 ladder≥4 的 confirmed type1
  buy 进入 buy1 掩码（届时 entry≥5 才有 ≥2 级 tranche 空间，entry=4 给 1 级）。
- **统计功效边界**：30min 下交易数极少（OKLO 9 / QQQ 20 / BRN 56 笔），
  OKLO@30 P5=−2.2% vs BH+309%——30min 的 Δ 符号信息量受小样本限制；
  5min（65/122/331 笔）是本路径的主要证据层。
- **认识论等级**：L2（三标的真实数据、单一聚合口径、两个 TF 采样点；
  否定性结果——tranche 空定义域边界被收紧而非假设被确认）。

产物：`analysis/rev_tranche_path2_coarse.py` /
`analysis/data_cache/rev_tranche_path2_coarse.json`""")
    L.append("")
    OUT_MD.write_text("\n".join(L))


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    force = os.environ.get("BT_FORCE", "0") == "1"
    for tf in TFS:
        for sym in SYMBOLS:
            key = f"{sym}@{tf}"
            if key in results and not force:
                print(f"[skip] {key} 已有结果", flush=True)
                continue
            results[key] = process(sym, tf)
            OUT_JSON.write_text(json.dumps(results, indent=2,
                                           ensure_ascii=False))
            write_report(results)
            print(f"  [{key}] 已落盘", flush=True)
    if results:
        write_report(results)


if __name__ == "__main__":
    main()
