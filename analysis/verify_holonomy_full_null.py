"""Part B 全历史 + 零结构 null 对照（回应合成确认偏差风险）。

formalization-validity-domain 规则警告"合成确认偏差"：递归是否对任何序列都报结构？
null 对照 = 把残差的逐 bar 增量随机重排（破坏时间序列结构，保留边际分布），
若 null 的中枢/趋势量显著低于真实残差 → 真实残差的结构非递归人工产物。
"""

from __future__ import annotations

import math

from analysis.verify_cross_national_closure import (
    _aligned_minute_residual,
    _resample_scalar_to_hourly_ohlc,
    _run_chanlun,
    _report,
    _EUR_WEIGHT,
)


def _shuffle_increments_null(bars):
    """对 close 增量做 i.i.d. surrogate（固定种子随机重排），重建 OHLC null 序列。

    随机重排破坏序列依赖（趋势/中枢的时间组织），保留增量边际分布。
    固定种子保证可复现。缠论结构近似时间反演协变，故不能用"反转"做 null
    （反转会找到相似结构，误导）；必须打散序列依赖。
    """
    import random

    rng = random.Random(517)  # 517号谱系编号作种子，可复现
    closes = [math.log(b.close) for b in bars]
    incs = [closes[i + 1] - closes[i] for i in range(len(closes) - 1)]
    rng.shuffle(incs)  # i.i.d. surrogate：破坏时间组织，保留边际分布
    from newchan.types import Bar

    out = [bars[0]]
    cur = closes[0]
    for i, inc in enumerate(incs):
        cur += inc
        out.append(Bar(ts=bars[i + 1].ts, open=math.exp(cur), high=math.exp(cur),
                       low=math.exp(cur), close=math.exp(cur)))
    return out


if __name__ == "__main__":
    print("Part B 全历史货币层和乐 + null 对照")
    ts, resid = _aligned_minute_residual("dx", "usd6e", 1.0, -_EUR_WEIGHT)
    bars = _resample_scalar_to_hourly_ohlc(ts, resid)
    print(f"全历史小时 bar = {len(bars):,}  ({bars[0].ts.isoformat()[:10]} → {bars[-1].ts.isoformat()[:10]})")

    print("\n[1] 真实残差（全历史）")
    _report(_run_chanlun("HOLO-FULL: 非欧元篮子", bars))

    print("\n[2] null 对照（增量反转重排，破坏时间组织）")
    _report(_run_chanlun("NULL: 增量反转", _shuffle_increments_null(bars)))
