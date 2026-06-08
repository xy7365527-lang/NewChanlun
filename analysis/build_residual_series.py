"""构造跨结算尺残差序列 r = log(DX) − 0.576·log(USD6E)，缓存为紧凑 npz。

## 残差定义（概念依据）

跨结算尺残差（memory: project_residual_to_flow_no_bridge）：
    r = log(DX) + 0.576·log(EURUSD)，占 DX 方差 33.8%。

数据事实裁决（L2 真实数据验证）：
    usd6e_1m_databento_10y.json 的 closes ∈ [0.670, 1.042]，mean 0.853。
    EURUSD 现货同期 ∈ [0.95, 1.16]（2018-2026）。USD6E 区间与 1/EURUSD 吻合：
      - 2022-09 EURUSD 平价下破 ~0.96 → 1/0.96 ≈ 1.04 = USD6E max ✓
      - 2010 EURUSD 高点 ~1.49      → 1/1.49 ≈ 0.67 = USD6E min ✓
    故 USD6E = 1/EURUSD ⟹ log(EURUSD) = −log(USD6E)，残差为：
        r = log(DX) − 0.576·log(USD6E)。

## 时间戳对齐

databento 1min 缺失低流动性分钟（两文件时间网格不同）→ 按 UTC 时间戳 inner join。
DX 范围 2018-12-26 → 2026-06-05（2.05M bar），USD6E 2010-06-07 起（5.5M bar）。
交集从 2018-12-26 起。缠论引擎不依赖时间戳（process_bar 仅传 OHLC），故 inner join
后时间戳不连续**不影响缠论结构递归**——但影响"流速=每 bar 数"的物理解释（caveat）。

## 残差 OHLC 构造

残差是 **close 的函数**（r = f(DX_close, 6E_close)）。喂缠论引擎需 OHLC，此处取
退化 bar O=H=L=C=r：残差无独立 intrabar 极值（DX/6E 分钟内不同步，log 线性组合的
high ≠ 组合的极值），强行用 DX/6E 的 high/low 构造残差 high/low 不严格。退化 bar 下
缠论分型识别等价于对 r 序列直接找顶/底分型——这是残差 close 序列的真实缠论结构。

认识论等级：L1（管线正确性——对齐+残差构造，由 inner join 计数 + 残差统计自证）。
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / "analysis" / "data_cache"

BETA = 0.576  # 协整系数（既定，memory: project_residual_to_flow_no_bridge）


def _load_symbol(path: Path) -> tuple[list[str], list[float]]:
    """加载 databento json，返回 (dates, closes)。仅保留对齐所需两列。"""
    with open(path) as f:
        d = json.load(f)
    return d["dates"], d["closes"]


def main() -> None:
    dx_path = CACHE / "dx_1m_databento_10y.json"
    e6_path = CACHE / "usd6e_1m_databento_10y.json"

    print("加载 DX ...", flush=True)
    dx_dates, dx_close = _load_symbol(dx_path)
    print(f"  DX: {len(dx_dates):,} bars  {dx_dates[0]} → {dx_dates[-1]}", flush=True)
    dx_map = {ts: c for ts, c in zip(dx_dates, dx_close)}
    del dx_dates, dx_close

    print("加载 USD6E ...", flush=True)
    e6_dates, e6_close = _load_symbol(e6_path)
    print(f"  USD6E: {len(e6_dates):,} bars  {e6_dates[0]} → {e6_dates[-1]}", flush=True)

    # 按 USD6E 时间序遍历，inner join DX（保持时间升序）。
    ts_out: list[str] = []
    r_out: list[float] = []
    n_miss_dx = 0
    n_nonpos = 0
    n_nan = 0
    for ts, c6 in zip(e6_dates, e6_close):
        c_dx = dx_map.get(ts)
        if c_dx is None:
            n_miss_dx += 1
            continue
        # NaN 必删（memory: 数据 nan 必删）——nan 比较恒 False，须显式 isnan。
        if c_dx is None or c6 is None or math.isnan(c_dx) or math.isnan(c6):
            n_nan += 1
            continue
        if c_dx <= 0.0 or c6 <= 0.0:
            n_nonpos += 1
            continue
        r = math.log(c_dx) - BETA * math.log(c6)
        ts_out.append(ts)
        r_out.append(r)
    del e6_dates, e6_close, dx_map

    r_arr = np.asarray(r_out, dtype=np.float64)
    n = len(r_arr)
    print(f"\ninner join 命中: {n:,} bars", flush=True)
    print(f"  USD6E 有但 DX 无（join miss）: {n_miss_dx:,}", flush=True)
    print(f"  NaN（剔除）: {n_nan:,}", flush=True)
    print(f"  非正价（剔除）: {n_nonpos:,}", flush=True)
    print(f"  残差范围: {r_arr.min():.5f} .. {r_arr.max():.5f}  mean={r_arr.mean():.5f}  std={r_arr.std():.5f}", flush=True)
    print(f"  时间范围: {ts_out[0]} → {ts_out[-1]}", flush=True)

    out = CACHE / "_residual_dx6e_aligned.npz"
    # 时间戳存为定长字符串数组（np 高效），残差存 float64。
    np.savez_compressed(
        out,
        residual=r_arr,
        timestamps=np.asarray(ts_out, dtype="U25"),
        beta=np.float64(BETA),
    )
    print(f"\n缓存 → {out}  ({out.stat().st_size / 1e6:.1f} MB)", flush=True)


if __name__ == "__main__":
    sys.exit(main())
