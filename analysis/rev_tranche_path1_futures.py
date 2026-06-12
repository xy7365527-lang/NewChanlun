"""路径1：期货 10y 1min 重跑有机赋格 v2 Rust 交易层 — REV tranche 定义域检验。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-10）
═══════════════════════════════════════════════════════════════════════
stock 1min（OKLO/QQQ/BRN）上 V1r 的 n_rev_tranche_adds 全为 0——递归涌现
级别太少（entry≤3 时 voices 域 [floor, entry) 只剩 home 一层，tranche 无
升级空间，空定义域）。假设：期货 10-16y 数据递归涌现级别更多（L4/L5），
tranche 才有非空定义域。

管线完全照搬 analysis/organic_fugue_rust_backtest.process_symbol：
  信号层 compute_organic_signals(+dir_flips) → pack_tape → Rust
  run_organic_rust(O0/V1f/V1r) + Python run_version_i(P5) 基线。
新增观测量（任务核心）：
  - V1r n_rev_tranche_adds / n_rev_tranche_closes（tranche 定义域判决）
  - entry ladder 分布（diag header[5]）+ rev_opens_by_ladder
  - tape 涌现级别证据：max(s.max_ladder) + dir_flips 按 ladder 计数
  - Δ(V1r) vs Δ(V1f)（预注册判据：tranche 激活且 Δ(V1r)>Δ(V1f) = 正贡献）

O0≡P5 守卫：期货数据首跑。不一致时如实记录（o0_guard 字段），不硬改。

数据：databento parallel-array（opens/...，含 dates 时间戳），nan/≤0 bar
整行删除（fugue_v2_full_backtest.load_ohlc 逐字复用）。years = 逐 bar 年
份列表（真实时间戳），span_years 由首尾日期导出。

增量落盘：每标的完成即写 data_cache/_rev_tranche_p1_{sym}.json，主进程
merge 至 data_cache/rev_tranche_path1_futures.json。重跑跳过已完成标的。

认识论等级：O0≡P5 守卫 L1（管线等价）；单标的 tranche 判决 L2；
七标的交叉 L3。

用法：PYTHONPATH=src .venv/bin/python analysis/rev_tranche_path1_futures.py
  env：BT_SYMBOLS=DX,BRN,...（选标的） BT_FORCE=1（强制重跑）
       BT_WORKERS=3（并行 worker 数，默认 3）
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import Counter
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

DATA_DIR = ROOT / "analysis" / "data_cache"
# BT_MAX_BARS（冒烟截断）时输出走 _smoke 后缀，绝不污染正式结果。
_SMOKE = "_smoke" if os.environ.get("BT_MAX_BARS", "0") != "0" else ""
OUT_JSON = DATA_DIR / f"rev_tranche_path1_futures{_SMOKE}.json"

# 7 期货，按 bar 数升序（小的先出结果；6E 最大 5.5M 放最后）。
SYMBOL_ORDER = ["DX", "BRN", "CL", "ZN", "6E", "GC", "ES"]
REV_VARIANTS = ["O0", "V1f", "V1r"]


def _part_path(sym: str) -> Path:
    return DATA_DIR / f"_rev_tranche_p1_{sym}{_SMOKE}.json"


def process_symbol(symbol: str) -> dict:
    """单标的：load → 信号层(+D3) → pack → O0/V1f/V1r + P5 基线 → 落盘。"""
    # worker 进程内 import（spawn 安全 + 各 worker 独立 Rust 实例）
    import newchan_rust as nr
    from fugue_alpha_diagnosis import CompletedTrade
    from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc
    from fugue_version_i import (
        LADDER_SEG,
        PAIRING_VARIANTS,
        extended_metrics,
        run_version_i,
    )
    from organic_fugue_rust_check import pack_tape
    from organic_signals import compute_organic_signals

    floor = LADDER_SEG
    print(f"\n{'=' * 64}\n  {symbol} — 路径1 期货 tranche 定义域检验\n{'=' * 64}",
          flush=True)
    opens, highs, lows, closes, years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    span_years = (years[-1] - years[0] + 1) if years else None
    print(f"  bars={n:,}  BH={bh:+.2f}%  span≈{span_years}y", flush=True)

    # ── 信号层（单 pass：磁带 + D3 方向行收集） ──
    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    flip_ladders = Counter(lad for _b, lad, _d in dir_flips)
    tape_max_ladder = max(s.max_ladder for s in tape)
    print(f"  信号层 {sig_s:.1f}s  D3 翻转行 {len(dir_flips):,}"
          f"  涌现 max_ladder={tape_max_ladder}"
          f"  flip按ladder={dict(sorted(flip_ladders.items()))}", flush=True)

    rtape = pack_tape(tape, dir_flips=dir_flips)

    # ── P5 基线（Python oracle） ──
    t0 = time.time()
    trades_p5, _ = run_version_i(
        tape, floor_ladder=floor, pairing=PAIRING_VARIANTS["P5"])
    m_p5 = extended_metrics(trades_p5, years)
    p5c = m_p5["total_compound"]
    print(f"  [P5 ] 交易={m_p5['n']} 复利={p5c:+.2f}%"
          f" [{time.time() - t0:.1f}s]", flush=True)

    out: dict = {
        "n_bars": n, "bh": round(bh, 2), "span_years": span_years,
        "sig_elapsed": round(sig_s, 1),
        "n_dir_flips": len(dir_flips),
        "dir_flips_by_ladder": {str(k): v
                                for k, v in sorted(flip_ladders.items())},
        "tape_max_ladder": tape_max_ladder,
        "P5": {"metrics": {k: m_p5[k] for k in
                           ("n", "win_rate", "total_compound",
                            "sharpe", "max_dd")},
               "by_year": m_p5["by_year"]},
        "variants": {},
    }

    # ── O0 / V1f / V1r ──
    for name in REV_VARIANTS:
        t0 = time.time()
        res = nr.run_organic_rust(rtape, name, floor_ladder=floor,
                                  stop_mode="none", diag=True)
        el = time.time() - t0
        trades = [CompletedTrade(entry_bar=r[0], entry_price=r[1],
                                 exit_bar=r[2], exit_price=r[3], pnl_pct=r[4],
                                 exit_reason=r[5], n_short_diffs=r[6],
                                 cost_basis_at_exit=r[7])
                  for r in res["trades"]]
        m = extended_metrics(trades, years)

        # 腿聚合（口径与 organic_fugue_rust_backtest._leg_stats_rust 一致）
        # + rev 腿 ladder 分布 + entry ladder 分布（diag header[5]）
        by_leg: dict[str, list] = {"main": [], "osc": [], "rev": []}
        rev_leg_ladders: Counter = Counter()
        entry_ladders: Counter = Counter()
        for header, diffs in res["diag"]:
            entry_ladders[header[5]] += 1
            for (key, leg, sb, sp, bb, bp), (sh, df, pf, *_rest) in diffs:
                by_leg[leg].append((sp, df, pf))
                if leg == "rev":
                    rev_leg_ladders[key] += 1
        legs = {}
        for leg, rows in by_leg.items():
            k = len(rows)
            legs[leg] = ({"n_pairs": 0, "win_rate": None,
                          "avg_diff_pct": None, "net_cash": 0.0} if k == 0
                         else {
                "n_pairs": k,
                "win_rate": round(
                    sum(1 for _sp, df, _pf in rows if df > 0) / k * 100, 1),
                "avg_diff_pct": round(
                    sum(df / sp for sp, df, _pf in rows) / k * 100, 4),
                "net_cash": round(sum(pf for _sp, _df, pf in rows), 1),
            })

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
            "counters": {k: c[k] for k in
                         ("n_rev_attempts", "n_rev_sub_anchor_rejects",
                          "n_rev_frozen_rejects", "n_rev_open",
                          "n_rev_close_t5", "n_rev_close_t6",
                          "n_rev_close_t7", "n_rev_struct_close",
                          "n_rev_tranche_adds", "n_rev_tranche_closes",
                          "n_open_rejects_zero")},
            "t6_close_share_pct": t6_share,
            "entry_ladder_dist": {str(k): v for k, v
                                  in sorted(entry_ladders.items())},
            "rev_leg_pairs_by_ladder": {str(k): v for k, v
                                        in sorted(rev_leg_ladders.items())},
            "rev_attempts_by_ladder": list(res["rev_attempts_by_ladder"]),
            "rev_opens_by_ladder": list(res["rev_opens_by_ladder"]),
            "elapsed_s": round(el, 3),
        }
        out["variants"][name] = pack
        print(f"  [{name:3s}] 交易={m['n']:5d} 复利={m['total_compound']:+10.2f}%"
              f" Δ(vs P5)={pack['delta_vs_p5']:+8.1f}pp"
              f" | rev开={c['n_rev_open']:5d}"
              f" tranche加/关={c['n_rev_tranche_adds']}/{c['n_rev_tranche_closes']}"
              f" entry层={pack['entry_ladder_dist']} [{el:.2f}s]", flush=True)

    # ── O0≡P5 守卫（期货首跑；不一致如实记录，不硬改） ──
    res_o0 = nr.run_organic_rust(rtape, "O0", floor_ladder=floor,
                                 stop_mode="none", diag=False)
    rust_o0 = res_o0["trades"]
    guard = "PASS"
    if len(trades_p5) != len(rust_o0):
        guard = f"FAIL 笔数 P5={len(trades_p5)} RustO0={len(rust_o0)}"
    else:
        for ti, (tp, tr) in enumerate(zip(trades_p5, rust_o0)):
            if (tp.entry_bar, tp.entry_price, tp.exit_bar, tp.exit_price,
                    tp.pnl_pct, tp.exit_reason, tp.n_short_diffs,
                    tp.cost_basis_at_exit) != tuple(tr):
                guard = (f"FAIL trade#{ti}: P5={tp} RustO0={tr}")
                break
    out["o0_guard"] = guard
    print(f"  [O0≡P5] {guard}", flush=True)

    _part_path(symbol).write_text(
        json.dumps(out, indent=2, ensure_ascii=False))
    print(f"  [{symbol}] 已落盘 {_part_path(symbol).name}", flush=True)
    return out


def main() -> None:
    symbols = [s.strip().upper() for s in
               os.environ.get("BT_SYMBOLS", ",".join(SYMBOL_ORDER)).split(",")]
    force = os.environ.get("BT_FORCE", "0") == "1"
    workers = int(os.environ.get("BT_WORKERS", "3"))

    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())

    todo = []
    for sym in symbols:
        part = _part_path(sym)
        if not force and sym in results:
            print(f"[skip] {sym} 已在汇总 JSON", flush=True)
            continue
        if not force and part.exists():
            try:
                results[sym] = json.loads(part.read_text())
                print(f"[merge] {sym} 从分片恢复", flush=True)
                continue
            except (json.JSONDecodeError, OSError):
                pass
        todo.append(sym)
    if results:
        OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))

    if todo:
        with ProcessPoolExecutor(max_workers=workers) as ex:
            futs = {ex.submit(process_symbol, s): s for s in todo}
            for fut in as_completed(futs):
                sym = futs[fut]
                results[sym] = fut.result()
                OUT_JSON.write_text(
                    json.dumps(results, indent=2, ensure_ascii=False))
                print(f"[merged] {sym} → {OUT_JSON.name}"
                      f"（{len(results)} 标的）", flush=True)

    # 汇总打印
    print(f"\n{'=' * 64}\n  汇总（{len(results)} 标的）\n{'=' * 64}", flush=True)
    for s in SYMBOL_ORDER:
        if s not in results:
            continue
        r = results[s]
        p5 = r["P5"]["metrics"]["total_compound"]
        v = r["variants"]
        adds = v["V1r"]["counters"]["n_rev_tranche_adds"]
        print(f"  {s:4s} P5={p5:+9.1f}% "
              f"Δ(V1f)={v['V1f']['delta_vs_p5']:+8.1f} "
              f"Δ(V1r)={v['V1r']['delta_vs_p5']:+8.1f} "
              f"tranche加={adds} max_ladder={r['tape_max_ladder']} "
              f"guard={r['o0_guard'][:30]}", flush=True)


if __name__ == "__main__":
    main()
