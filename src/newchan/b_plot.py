"""B 系统 — matplotlib 时间序列绘图"""

from __future__ import annotations

import platform
from datetime import datetime, timezone

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


# ── BSP 标签映射 ──

_BSP_LABEL = {
    ("type1", "buy"): "1B",
    ("type1", "sell"): "1S",
    ("type2", "buy"): "2B",
    ("type2", "sell"): "2S",
    ("type3", "buy"): "3B",
    ("type3", "sell"): "3S",
}


def _plot_bsp(ax: plt.Axes, bsp_list: list[dict]) -> None:
    """在 ax 上叠加买卖点标注。

    Parameters
    ----------
    ax : matplotlib Axes
    bsp_list : list[dict]
        来自 build_overlay_newchan() 返回的 ``overlay["bsp"]``。
        每个 dict 至少包含 time(epoch秒), price, kind, side。
    """
    if not bsp_list:
        return

    for bp in bsp_list:
        t = datetime.fromtimestamp(bp["time"], tz=timezone.utc)
        price = bp["price"]
        side = bp["side"]
        label = _BSP_LABEL.get((bp["kind"], side), "?")

        if side == "buy":
            marker, color, va, y_offset = "^", "#00AA00", "bottom", -1
        else:
            marker, color, va, y_offset = "v", "#CC0000", "top", 1

        ax.plot(t, price, marker=marker, color=color,
                markersize=8, markeredgewidth=0.5, markeredgecolor="black",
                zorder=5)
        ax.annotate(label, xy=(t, price), fontsize=7, fontweight="bold",
                    color=color, ha="center", va=va,
                    xytext=(0, 6 * y_offset), textcoords="offset points",
                    zorder=5)


def plot_overlay(
    df: pd.DataFrame,
    overlay: dict,
    title: str = "",
) -> None:
    """绘制收盘价 + 买卖点叠加图。

    Parameters
    ----------
    df : pd.DataFrame
        必须有 DatetimeIndex 以及 close 列。
    overlay : dict
        build_overlay_newchan() 的返回值，至少包含 ``bsp`` 键。
    title : str
        图表标题。
    """
    fig, ax = plt.subplots(figsize=(14, 5))

    ax.plot(df.index, df["close"], linewidth=1.0, color="#4488CC", label="收盘价")

    _plot_bsp(ax, overlay.get("bsp", []))

    # x 轴日期格式
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
