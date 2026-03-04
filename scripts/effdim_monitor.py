"""
K4 eff.dim 实时监控工具
========================

谱系引用：329号（K4折叠内在性检验最终结算）、330号（K4拓扑与资本循环映射）
认识论等级：L2（真实数据，可否证）

四节点：Au (GC=F), $ (DX-Y.NYB), Equity (^GSPC), Commodity (CL=F)
每节点3条边 = log(price ratio) return 的 rolling 252d covariance → eigenvalue → Shannon entropy → eff.dim
二元信号：eff.dim < 5th percentile → 不可折叠相 / eff.dim >= 5th percentile → 可折叠相
边冻结识别：同步压缩时哪个节点的边在暴动（vol z-score > 3σ）
可监控信号（330号）：Equity/Commodity 长期趋势、Au/$ 长期趋势
"""

from __future__ import annotations

import sys
import warnings
from dataclasses import dataclass
from typing import Tuple

import numpy as np
import pandas as pd

warnings.filterwarnings("ignore")


# ============================================================
# 数据类（immutable）
# ============================================================

@dataclass(frozen=True)
class NodeEffDim:
    """单节点 eff.dim 计算结果"""
    name: str
    effdim: float
    percentile: float  # 在 rolling 历史中的 percentile
    phase: str         # "不可折叠" | "可折叠"
    edge_names: Tuple[str, ...]
    edge_vol_zscores: Tuple[float, ...]  # 当前各边 vol 的 z-score


@dataclass(frozen=True)
class EdgeFreezeStatus:
    """边冻结状态"""
    is_sync_compressed: bool         # 四节点是否同步处于底部 5%
    riot_node: str                   # 暴动节点（如果有）
    riot_edges: Tuple[str, ...]      # 暴动边
    frozen_edges: Tuple[str, ...]    # 冻结边
    detail: str


@dataclass(frozen=True)
class TrendSignal:
    """长期趋势可监控信号（330号）"""
    name: str
    current_ratio: float
    ma252: float
    direction: str  # "上升" | "下降" | "中性"
    interpretation: str


@dataclass(frozen=True)
class K4Status:
    """K4 完整状态快照"""
    date: str
    nodes: Tuple[NodeEffDim, ...]
    edge_freeze: EdgeFreezeStatus
    trend_signals: Tuple[TrendSignal, ...]
    data_range: str
    n_obs: int


# ============================================================
# 纯函数：计算层
# ============================================================

NODES = {
    "Au": {"ticker": "GC=F", "peers": ["DX-Y.NYB", "^GSPC", "CL=F"],
            "edge_names": ("Au/$", "Au/Equity", "Au/Commodity")},
    "$": {"ticker": "DX-Y.NYB", "peers": ["GC=F", "^GSPC", "CL=F"],
           "edge_names": ("$/Au", "$/Equity", "$/Commodity")},
    "Equity": {"ticker": "^GSPC", "peers": ["DX-Y.NYB", "GC=F", "CL=F"],
                "edge_names": ("Equity/$", "Equity/Au", "Equity/Commodity")},
    "Commodity": {"ticker": "CL=F", "peers": ["DX-Y.NYB", "GC=F", "^GSPC"],
                   "edge_names": ("Commodity/$", "Commodity/Au", "Commodity/Equity")},
}

EFFDIM_WINDOW = 252
VOL_WINDOW = 63  # 约3个月，用于边波动率 z-score
PHASE_THRESHOLD_PERCENTILE = 5.0  # 全样本 5th percentile 作为初始阈值


def download_prices() -> pd.DataFrame:
    """下载四个 ticker 的收盘价，返回对齐后的 DataFrame"""
    import yfinance as yf

    tickers = ["GC=F", "DX-Y.NYB", "^GSPC", "CL=F"]
    raw = yf.download(tickers, start="2000-01-01", progress=False)["Close"]
    if isinstance(raw, pd.Series):
        raise ValueError("yfinance 返回 Series 而非 DataFrame，数据异常")
    df = raw.dropna()
    df.columns = [str(c) if not isinstance(c, str) else c for c in df.columns]
    return df


def compute_log_ratio_returns(prices: pd.DataFrame, node_ticker: str,
                              peer_tickers: list[str]) -> pd.DataFrame:
    """计算节点与三个 peer 的 log(price ratio) return"""
    node_price = prices[node_ticker]
    returns = {}
    for i, peer in enumerate(peer_tickers):
        ratio = node_price / prices[peer]
        log_ret = np.log(ratio).diff()
        returns[f"edge_{i}"] = log_ret
    return pd.DataFrame(returns).dropna()


def compute_effdim_series(returns: pd.DataFrame, window: int = EFFDIM_WINDOW) -> pd.Series:
    """rolling 窗口计算 eff.dim 时间序列"""
    values = []
    dates = []
    arr = returns.values

    for i in range(window, len(arr)):
        w = arr[i - window: i]
        cov = np.cov(w.T)
        eigvals = np.linalg.eigvalsh(cov)
        eigvals = eigvals[eigvals > 1e-15]
        if len(eigvals) == 0:
            values.append(1.0)
        else:
            p = eigvals / eigvals.sum()
            entropy = -np.sum(p * np.log(p))
            values.append(np.exp(entropy))
        dates.append(returns.index[i])

    return pd.Series(values, index=dates)


def compute_edge_vol_zscores(returns: pd.DataFrame, window: int = VOL_WINDOW) -> np.ndarray:
    """计算最近一天各边 return 的 rolling vol z-score"""
    if len(returns) < window + 1:
        return np.zeros(returns.shape[1])

    recent = returns.iloc[-(window + 1):]
    rolling_vol = recent.rolling(window).std().iloc[-1].values
    full_vol_mean = returns.rolling(window).std().mean().values
    full_vol_std = returns.rolling(window).std().std().values

    zscores = np.where(
        full_vol_std > 1e-15,
        (rolling_vol - full_vol_mean) / full_vol_std,
        0.0,
    )
    return zscores


def classify_phase(effdim_series: pd.Series, current_effdim: float) -> Tuple[str, float]:
    """二元相态分类，返回 (phase, percentile)"""
    from scipy.stats import percentileofscore

    pct = percentileofscore(effdim_series.values, current_effdim, kind="rank")
    phase = "不可折叠" if pct < PHASE_THRESHOLD_PERCENTILE else "可折叠"
    return phase, pct


def compute_node_effdim(prices: pd.DataFrame, node_name: str) -> NodeEffDim:
    """计算单节点的完整 eff.dim 状态"""
    cfg = NODES[node_name]
    returns = compute_log_ratio_returns(prices, cfg["ticker"], cfg["peers"])
    effdim_series = compute_effdim_series(returns)

    if len(effdim_series) == 0:
        return NodeEffDim(
            name=node_name, effdim=float("nan"), percentile=float("nan"),
            phase="数据不足", edge_names=cfg["edge_names"],
            edge_vol_zscores=(0.0, 0.0, 0.0),
        )

    current = effdim_series.iloc[-1]
    phase, pct = classify_phase(effdim_series, current)
    vol_z = compute_edge_vol_zscores(returns)

    return NodeEffDim(
        name=node_name,
        effdim=float(current),
        percentile=float(pct),
        phase=phase,
        edge_names=cfg["edge_names"],
        edge_vol_zscores=tuple(float(z) for z in vol_z),
    )


def detect_edge_freeze(nodes: Tuple[NodeEffDim, ...]) -> EdgeFreezeStatus:
    """
    边冻结识别（329号）：
    当四节点同步处于底部 5% 时，检查哪个节点的边在暴动
    暴动 = 含该节点的3条边 vol z-score > 3σ
    冻结 = 不含该节点的3条边 vol z-score < 1σ
    """
    all_below_5 = all(n.percentile < PHASE_THRESHOLD_PERCENTILE for n in nodes)

    if not all_below_5:
        return EdgeFreezeStatus(
            is_sync_compressed=False,
            riot_node="",
            riot_edges=(),
            frozen_edges=(),
            detail="未处于同步压缩态（非全部节点 < 5th percentile）",
        )

    # 找暴动节点：哪个节点的边 vol z-score 最大
    max_mean_z = -1.0
    riot_node = ""
    for node in nodes:
        mean_z = np.mean(np.abs(node.edge_vol_zscores))
        if mean_z > max_mean_z:
            max_mean_z = mean_z
            riot_node = node.name

    riot_node_obj = next(n for n in nodes if n.name == riot_node)
    other_nodes = [n for n in nodes if n.name != riot_node]

    riot_edges = tuple(
        name for name, z in zip(riot_node_obj.edge_names, riot_node_obj.edge_vol_zscores)
        if abs(z) > 3.0
    )
    frozen_edges = tuple(
        name for other in other_nodes
        for name, z in zip(other.edge_names, other.edge_vol_zscores)
        if abs(z) < 1.0
    )

    if len(riot_edges) > 0:
        detail = f"暴动节点: {riot_node}（mean |z|={max_mean_z:.2f}）"
    else:
        detail = f"同步压缩但无明显暴动节点（最高 mean |z|={max_mean_z:.2f} < 3σ）"

    return EdgeFreezeStatus(
        is_sync_compressed=True,
        riot_node=riot_node,
        riot_edges=riot_edges,
        frozen_edges=frozen_edges,
        detail=detail,
    )


def compute_trend_signals(prices: pd.DataFrame) -> Tuple[TrendSignal, ...]:
    """
    可监控信号（330号）：
    1. Equity/Commodity (SPX/OIL)：持续上升 = 金融循环膨胀
    2. Au/$ (GC=F/DXY)：持续上升 = $ 度量功能被质疑
    """
    signals = []

    # Equity/Commodity
    spx_oil = prices["^GSPC"] / prices["CL=F"]
    spx_oil = spx_oil.dropna()
    if len(spx_oil) > EFFDIM_WINDOW:
        current = float(spx_oil.iloc[-1])
        ma = float(spx_oil.rolling(EFFDIM_WINDOW).mean().iloc[-1])
        direction = "上升" if current > ma else ("下降" if current < ma * 0.95 else "中性")
        signals.append(TrendSignal(
            name="Equity/Commodity (SPX/OIL)",
            current_ratio=current,
            ma252=ma,
            direction=direction,
            interpretation="上升 = 金融循环膨胀（堰塞湖价格投影扩大）" if direction == "上升"
            else "下降 = 金融循环向产业循环回归信号" if direction == "下降"
            else "无明显方向",
        ))

    # Au/$
    au_dxy = prices["GC=F"] / prices["DX-Y.NYB"]
    au_dxy = au_dxy.dropna()
    if len(au_dxy) > EFFDIM_WINDOW:
        current = float(au_dxy.iloc[-1])
        ma = float(au_dxy.rolling(EFFDIM_WINDOW).mean().iloc[-1])
        direction = "上升" if current > ma else ("下降" if current < ma * 0.95 else "中性")
        signals.append(TrendSignal(
            name="Au/$ (GC=F/DXY)",
            current_ratio=current,
            ma252=ma,
            direction=direction,
            interpretation="上升 = $ 度量功能健康度下降" if direction == "上升"
            else "下降 = $ 度量功能稳定" if direction == "下降"
            else "无明显方向",
        ))

    return tuple(signals)


def compute_k4_status(prices: pd.DataFrame) -> K4Status:
    """计算完整 K4 状态"""
    nodes = tuple(compute_node_effdim(prices, name) for name in NODES)
    edge_freeze = detect_edge_freeze(nodes)
    trend_signals = compute_trend_signals(prices)

    return K4Status(
        date=str(prices.index[-1].date()),
        nodes=nodes,
        edge_freeze=edge_freeze,
        trend_signals=trend_signals,
        data_range=f"{prices.index[0].date()} to {prices.index[-1].date()}",
        n_obs=len(prices),
    )


# ============================================================
# 输出层
# ============================================================

def format_status(status: K4Status) -> str:
    """格式化 K4 状态为终端输出"""
    lines = []
    lines.append("=" * 64)
    lines.append(f"  K4 eff.dim 监控  |  {status.date}")
    lines.append(f"  数据: {status.data_range} ({status.n_obs} obs)")
    lines.append("=" * 64)
    lines.append("")

    # 四节点 eff.dim
    lines.append("  节点 eff.dim")
    lines.append("  " + "-" * 56)
    for node in status.nodes:
        phase_marker = "■" if node.phase == "不可折叠" else "□"
        lines.append(
            f"  {phase_marker} {node.name:<12s}  "
            f"eff.dim={node.effdim:.4f}  "
            f"pct={node.percentile:5.1f}%  "
            f"[{node.phase}]"
        )
    lines.append("")

    # 边 vol z-score
    lines.append("  边波动率 z-score")
    lines.append("  " + "-" * 56)
    for node in status.nodes:
        edge_strs = []
        for name, z in zip(node.edge_names, node.edge_vol_zscores):
            flag = "!" if abs(z) > 3.0 else " "
            edge_strs.append(f"{name}={z:+.2f}{flag}")
        lines.append(f"  {node.name:<12s}  {' | '.join(edge_strs)}")
    lines.append("")

    # 边冻结
    lines.append("  边冻结状态")
    lines.append("  " + "-" * 56)
    if status.edge_freeze.is_sync_compressed:
        lines.append(f"  *** 同步压缩 ***  {status.edge_freeze.detail}")
        if status.edge_freeze.riot_edges:
            lines.append(f"  暴动边: {', '.join(status.edge_freeze.riot_edges)}")
        if status.edge_freeze.frozen_edges:
            lines.append(f"  冻结边: {', '.join(status.edge_freeze.frozen_edges)}")
    else:
        lines.append(f"  {status.edge_freeze.detail}")
    lines.append("")

    # 可监控信号
    lines.append("  可监控信号 (330号)")
    lines.append("  " + "-" * 56)
    for sig in status.trend_signals:
        arrow = "↑" if sig.direction == "上升" else ("↓" if sig.direction == "下降" else "→")
        lines.append(
            f"  {arrow} {sig.name:<28s}  "
            f"当前={sig.current_ratio:.2f}  MA252={sig.ma252:.2f}"
        )
        lines.append(f"    {sig.interpretation}")
    lines.append("")

    # 阈值因果提醒
    lines.append("  注释（329号结算）")
    lines.append("  " + "-" * 56)
    lines.append("  - 阈值因果：跨过阈值后 P(fold) ~1% 恒定，不随 eff.dim 增加")
    lines.append("  - 当前阈值：全样本 5th percentile（简化；实际制度相关）")
    lines.append("  - QE 期地板 ~1.4 pct，非 QE 期 3.9-22.4%")
    lines.append("  - 折叠是 K4 全局事件（all-or-nothing），不存在局部折叠")
    lines.append("=" * 64)

    return "\n".join(lines)


# ============================================================
# 入口
# ============================================================

def main() -> int:
    print("正在从 yfinance 下载数据...")
    try:
        prices = download_prices()
    except Exception as e:
        print(f"数据下载失败: {e}", file=sys.stderr)
        print("请检查网络连接和 yfinance 版本。", file=sys.stderr)
        return 1

    if len(prices) < EFFDIM_WINDOW + 10:
        print(f"数据不足: 需要至少 {EFFDIM_WINDOW + 10} 天, 实际 {len(prices)} 天",
              file=sys.stderr)
        return 1

    print("正在计算 K4 状态...")
    status = compute_k4_status(prices)
    print()
    print(format_status(status))
    return 0


if __name__ == "__main__":
    sys.exit(main())
