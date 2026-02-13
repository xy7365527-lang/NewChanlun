"""Bar <-> DataFrame 转换工具"""

from __future__ import annotations

from dataclasses import asdict
from typing import Sequence

import pandas as pd

from newchan.types import Bar


def bars_to_df(bars: Sequence[Bar]) -> pd.DataFrame:
    """将 Bar 列表转为带 DatetimeIndex 的 DataFrame。

    严格遵循 DataCamp matplotlib time-series 教材套路：
      df['ts'] = pd.to_datetime(df['ts'])
      df = df.set_index('ts').sort_index()
    """
    df = pd.DataFrame([asdict(b) for b in bars])
    df["ts"] = pd.to_datetime(df["ts"])
    df = df.set_index("ts").sort_index()
    return df
