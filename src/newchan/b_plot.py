"""B 系统 — matplotlib 时间序列绘图"""

from __future__ import annotations

import platform

import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import pandas as pd

# 中文字体支持
if platform.system() == "Windows":
    plt.rcParams["font.sans-serif"] = ["Microsoft YaHei", "SimHei", "sans-serif"]
else:
    plt.rcParams["font.sans-serif"] = ["PingFang SC", "Noto Sans CJK SC", "sans-serif"]
plt.rcParams["axes.unicode_minus"] = False


def plot_close(df: pd.DataFrame, title: str = "") -> None:
    """绘制收盘价时间序列折线图。

    严格遵循 DataCamp matplotlib time-series 教材套路：
      fig, ax = plt.subplots()
      ax.plot(df.index, df['close'])
      ax.xaxis.set_major_formatter(DateFormatter)

    Parameters
    ----------
    df : pd.DataFrame
        必须有 DatetimeIndex 以及 close 列。
    title : str
        图表标题。
    """
    fig, ax = plt.subplots(figsize=(14, 5))

    ax.plot(df.index, df["close"], linewidth=1.0)

    # x 轴日期格式 — 根据数据跨度自动选择
    span = df.index[-1] - df.index[0]
    if span.days > 365:
        ax.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m"))
        ax.xaxis.set_major_locator(mdates.MonthLocator(interval=3))
    elif span.days > 30:
        ax.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m-%d"))
        ax.xaxis.set_major_locator(mdates.WeekdayLocator(interval=1))
    else:
        ax.xaxis.set_major_formatter(mdates.DateFormatter("%m-%d %H:%M"))
        ax.xaxis.set_major_locator(mdates.AutoDateLocator())

    fig.autofmt_xdate()

    ax.set_xlabel("时间")
    ax.set_ylabel("收盘价")
    if title:
        ax.set_title(title)
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.show()
