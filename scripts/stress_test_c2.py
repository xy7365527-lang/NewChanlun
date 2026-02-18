"""C-2 三层退化连锁检测压力测试（024号谱系 #1）。

用已有缓存的 5 个标的（GLD, IAU, SLV, SPY, TLT）生成全部 C(5,2)=10 对，
跑 validate_pair 三层检测，输出诊断报告。

预期：
  - 正例（不同资产类别的 pair）：valid=True, 三层诊断字段完整
  - 反例（近常数 pair 如 GLD/IAU）：valid=False, CV 极低
  - 边界（同类但不同标的如 SLV/IAU）：可能揭示新的边界情况
"""

from __future__ import annotations

import itertools
from pathlib import Path

import pandas as pd

from newchan.equivalence import validate_pair

CACHE_DIR = Path(__file__).resolve().parent.parent / ".cache"
SYMBOLS = ["GLD", "IAU", "SLV", "SPY", "TLT"]


def _load(sym: str) -> pd.DataFrame:
    path = CACHE_DIR / f"{sym}_1day_raw.parquet"
    if not path.exists():
        raise FileNotFoundError(f"缓存不存在: {path}")
    return pd.read_parquet(path)


def main() -> None:
    data = {sym: _load(sym) for sym in SYMBOLS}

    print(f"{'Pair':<12} {'Valid':>5} {'CV':>10} {'StrokePct':>10} "
          f"{'MACDNorm':>10} {'#Strokes':>8}  Reason")
    print("-" * 85)

    results = []
    for a, b in itertools.combinations(SYMBOLS, 2):
        vr = validate_pair(data[a], data[b])
        cv_s = f"{vr.cv:.6f}" if vr.cv is not None else "-"
        sp_s = f"{vr.stroke_mean_pct:.4%}" if vr.stroke_mean_pct is not None else "-"
        mn_s = f"{vr.macd_norm_hist:.6f}" if vr.macd_norm_hist is not None else "-"
        ns_s = str(vr.n_strokes) if vr.n_strokes is not None else "-"
        reason_s = vr.reason[:30] if vr.reason else ""

        print(f"{a}/{b:<7} {str(vr.valid):>5} {cv_s:>10} {sp_s:>10} "
              f"{mn_s:>10} {ns_s:>8}  {reason_s}")

        results.append({
            "pair": f"{a}/{b}", "valid": vr.valid,
            "cv": vr.cv, "stroke_mean_pct": vr.stroke_mean_pct,
            "macd_norm_hist": vr.macd_norm_hist, "n_strokes": vr.n_strokes,
            "reason": vr.reason,
        })

    # 汇总
    n_valid = sum(1 for r in results if r["valid"])
    n_invalid = sum(1 for r in results if not r["valid"])
    print(f"\n汇总: {n_valid} valid, {n_invalid} invalid, {len(results)} total")

    # 阈值边界分析
    valid_cvs = [r["cv"] for r in results if r["valid"] and r["cv"] is not None]
    invalid_cvs = [r["cv"] for r in results if not r["valid"] and r["cv"] is not None]
    if valid_cvs and invalid_cvs:
        print(f"正例 CV 范围: [{min(valid_cvs):.6f}, {max(valid_cvs):.6f}]")
        print(f"反例 CV 范围: [{min(invalid_cvs):.6f}, {max(invalid_cvs):.6f}]")
        gap = min(valid_cvs) / max(invalid_cvs) if max(invalid_cvs) > 0 else float("inf")
        print(f"CV 间隔倍数: {gap:.1f}x")


if __name__ == "__main__":
    main()
