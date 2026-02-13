"""overlay 全链路断言 — 集成测试

目的：在开启 NEWCHAN_ASSERT 时，A 系统全链路断言应当通过，
以此把 Spec/公理落成“可执行的不变量”，降低语义漂移。

注意：这里不追求输出对象的具体数量（那属于实现细节），
而是确保管线在一段结构性数据上能跑出非空结构且不触发断言。
"""

from __future__ import annotations

import pandas as pd

from newchan.ab_bridge_newchan import build_overlay_newchan


def _make_structured_df(n_blocks: int = 10) -> pd.DataFrame:
    """构造可产生分型→笔→段→(可能)中枢的结构性数据。

    采用 5-bar block：
    - 偶数 block：中间形成底分型
    - 奇数 block：中间形成顶分型

    block 间避免包含关系，确保包含处理不会把序列“吞掉”。
    """
    bottom_block = [
        (20.0, 10.0),
        (18.0, 8.0),
        (15.0, 5.0),   # bottom fractal center
        (18.0, 8.0),
        (20.0, 10.0),
    ]
    top_block = [
        (15.0, 5.0),
        (18.0, 8.0),
        (22.0, 12.0),  # top fractal center
        (18.0, 8.0),
        (15.0, 5.0),
    ]

    highs: list[float] = []
    lows: list[float] = []
    for b in range(n_blocks):
        block = bottom_block if b % 2 == 0 else top_block
        for h, l in block:
            highs.append(h)
            lows.append(l)

    idx = pd.date_range("2025-01-01", periods=len(highs), freq="h")
    opens = [(h + l) / 2 for h, l in zip(highs, lows)]
    closes = [(h + l) / 2 for h, l in zip(highs, lows)]
    return pd.DataFrame(
        {"open": opens, "high": highs, "low": lows, "close": closes},
        index=idx,
    )


def test_overlay_strict_assertions_pass(monkeypatch):
    monkeypatch.setenv("NEWCHAN_ASSERT", "1")

    df = _make_structured_df(n_blocks=12)
    out = build_overlay_newchan(df, symbol="TEST", tf="1h", detail="min")

    assert out["schema_version"] == "newchan_overlay_v2"
    # 确保确实跑出一定结构（避免“全空也算通过”）
    assert len(out["strokes"]) > 0
    assert len(out["segments"]) > 0
