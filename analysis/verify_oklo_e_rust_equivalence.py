"""OKLO 333K E 版本 — Python compute_signals ↔ Rust compute_e_signals_rust 端到端等价验证。

## 目的（M1 收尾关键任务）

确认信号计算层从 Python `RecursiveOrchestrator`（O(N²)，~分钟级）切换到 Rust
`RecursiveOrchestrator`（逐位等价、快 ~23×）后，E 版本回测结果**不变**：
  OKLO full（333,613 bars）E 版本复利 = 455.81%（基线）→ Rust 驱动应逐位复现。

## 等价范围（有效域声明，formalization-validity-domain 规则）

E 版本 `run_swing_trading(MODE_NONE)` 只读 BarSignal 的 5 个字段
（close / down_move_settled / up_move_settled / entry_div_ok / exit_div_ok / l2_flip_short）。
`compute_e_signals_rust` 精确产出这些字段（其余 mode 的字段填默认）。故本验证证明的是
**E 字段等价**（有效域 = E 版本所消费字段），**非全 mode（A/B/D/current）字段 bit-exact**。
后者需额外重建 BSP/L0·L1 PH/new_up_moves 事件，超出 E 版本验证目标。

## 认识论等级

- 移植正确性：L1（管线等价，由 BarSignal E 字段逐位对比 + 回测指标完全相等保证）。
- 回测结论：L2（真实数据 OKLO 1min 333K）。
"""

from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402


def _metrics_line(tag: str, m: dict, bh: float, secs: float) -> str:
    return (
        f"  [{tag:8s}] 复利={m['total_compound']:+11.4f}% "
        f"BH={bh:+9.2f}% 超额={m['total_compound'] - bh:+11.4f}% | "
        f"交易={m['n']:4d} 胜率={m['win_rate']:5.1f}% 夏普={m['sharpe']:+.4f} "
        f"MDD={m['max_dd']:+7.2f}% | 信号 {secs:7.2f}s"
    )


def _e_field_diff(py_sigs: list, rs_sigs: list) -> tuple[int, list[str]]:
    """逐 bar 对比 E 版本所消费的 5 个 BarSignal 字段，返回 (分歧数, 前若干样例)。"""
    assert len(py_sigs) == len(rs_sigs), (
        f"信号磁带长度不等 py={len(py_sigs)} rust={len(rs_sigs)}"
    )
    fields = (
        "close", "down_move_settled", "up_move_settled",
        "entry_div_ok", "exit_div_ok", "l2_flip_short",
    )
    n_diff = 0
    samples: list[str] = []
    for i, (p, r) in enumerate(zip(py_sigs, rs_sigs)):
        for f in fields:
            pv, rv = getattr(p, f), getattr(r, f)
            if pv != rv:
                n_diff += 1
                if len(samples) < 10:
                    samples.append(f"    bar {i} .{f}: py={pv!r} rust={rv!r}")
                break
    return n_diff, samples


def main() -> None:
    print("OKLO 333K E 版本 — Python ↔ Rust 信号层等价验证\n")

    opens, highs, lows, closes = F.load_1min("OKLO")
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"标的 OKLO | {n:,} bars | BH={bh:+.2f}%\n")

    # ── Python 基线（O(N²) RecursiveOrchestrator） ──
    print("[1/2] Python compute_signals（O(N²) 引擎）...", flush=True)
    t0 = time.time()
    py_sigs = F.compute_signals(opens, highs, lows, closes)
    py_sig_s = time.time() - t0
    py_trades, _ = F.run_swing_trading(py_sigs, F.MODE_NONE)
    py_m = F.compute_metrics(py_trades)
    print(_metrics_line("Python", py_m, bh, py_sig_s), flush=True)

    # ── Rust 驱动 ──
    print("\n[2/2] Rust compute_e_signals_rust（O(N²) 但快 ~23×）...", flush=True)
    t1 = time.time()
    rs_sigs = RE.compute_e_signals_rust(opens, highs, lows, closes)
    rs_sig_s = time.time() - t1
    rs_trades, _ = F.run_swing_trading(rs_sigs, F.MODE_NONE)
    rs_m = F.compute_metrics(rs_trades)
    print(_metrics_line("Rust", rs_m, bh, rs_sig_s), flush=True)

    # ── 等价判定 ──
    print("\n" + "=" * 64)
    print("等价判定")
    print("=" * 64)

    n_diff, samples = _e_field_diff(py_sigs, rs_sigs)
    print(f"  BarSignal E 字段逐 bar 分歧：{n_diff} / {n:,} bars")
    for s in samples:
        print(s)

    # 回测指标完全相等（复利、交易笔数、胜率、夏普、MDD）
    metric_keys = ["total_compound", "n", "win_rate", "sharpe", "max_dd"]
    metric_ok = all(py_m[k] == rs_m[k] for k in metric_keys)
    print(f"\n  回测指标完全相等：{'PASS' if metric_ok else 'FAIL'}")
    for k in metric_keys:
        flag = "✓" if py_m[k] == rs_m[k] else "✗"
        print(f"    {flag} {k:16s} py={py_m[k]!r:>22}  rust={rs_m[k]!r}")

    # 速度对比
    speedup = py_sig_s / rs_sig_s if rs_sig_s > 0 else float("inf")
    print("\n  速度对比（信号层）：")
    print(f"    Python {py_sig_s:8.2f}s  →  Rust {rs_sig_s:8.2f}s  (加速 {speedup:.1f}×)")

    print("\n" + "=" * 64)
    verdict = (n_diff == 0) and metric_ok
    print(f"结论：{'✅ 等价 PASS' if verdict else '❌ 分歧 FAIL'}"
          f" | E 复利 Python={py_m['total_compound']:+.4f}% "
          f"Rust={rs_m['total_compound']:+.4f}%")
    print("=" * 64)

    sys.exit(0 if verdict else 1)


if __name__ == "__main__":
    main()
