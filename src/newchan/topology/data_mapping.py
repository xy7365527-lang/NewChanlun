"""K4 顶点 / 折叠通道 → 数据源映射（单一真相源，528号）。

概念溯源：254号代理变量、259号 C 顶点代理需结算属性、330号资本循环、
          528号四套冲突映射的统一、形式化有效域规则（231号：诚实标注数据缺口）。

## 单一真相源（528号要求）

528号失效模式：回测（金=C）、实时监控（金=$）、正典（金∈C）三处映射冲突。
本模块是**唯一**的顶点→标的映射真相源，消除四套并存冲突。下游（backtest /
k4_monitor / scanner）必须从此处读取，不得各自硬编码。

## 顶点代理（折叠通道模型下的重新分工）

折叠通道模型澄清了 259号"C 顶点代理需结算属性（黄金）"的旧争论：
  - 旧单顶点模型：C 必须选一个代理（金 or 油 or DBC），故 259号要求选有结算属性的金。
  - 折叠通道模型：C 顶点 = 纯商品资本（广义商品）；金的**货币/结算属性**归入 Au
    折叠通道（C↔M），不再是 C 顶点代理。故 C 顶点代理 = 广义商品指数（DBC）。

  金、油作为**折叠通道观测量**（fold_channel.py），不占顶点。

## 数据缺口诚实标注（231号 / no-patch）

美国四顶点 M(UUP)/P(ES)/C(DBC)/R(VNQ) 的 1min 数据均已齐备（C/R 经 TWS 拉取）。
跨经济体（EU/JP/CN）的 C（广义商品）/R（不动产）本币源仍多为缺口（254号 OQ2：
不动产高度本地化）；中国本土 P(IF)/C(SC)/AU9999 在 IB/databento 均无源（已验证）。
这些缺口按形式化有效域规则诚实标注，**不伪造替代**。跨国数据规格见
cross_national_pipeline.py 的 ECONOMY_SPECS。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from newchan.topology.fold_channel import AU, OIL, FoldChannel
from newchan.topology.graph import Vertex


class DataAvailability(Enum):
    """数据源可用状态。"""

    AVAILABLE = "available"            # 已缓存，1min 级别可用
    DAILY_ONLY = "daily_only"          # 仅日线可用，无分钟级
    NEEDS_ACQUISITION = "needs_acquisition"  # 无数据源，需采集（诚实标注，不伪造）


@dataclass(frozen=True, slots=True)
class DataSource:
    """一个标的的数据源描述。

    Attributes
    ----------
    symbol : str
        代理标的代码。
    description : str
        含义说明。
    cache_file : str
        analysis/data_cache/ 下的缓存文件名（NEEDS_ACQUISITION 时为空）。
    availability : DataAvailability
        可用状态。
    """

    symbol: str
    description: str
    cache_file: str
    availability: DataAvailability


# ── 顶点 → 数据源（单一真相源）──────────────────────────────────

VERTEX_DATA_SOURCES: dict[Vertex, DataSource] = {
    Vertex.M: DataSource(
        symbol="UUP",
        description="美元（货币资本 / 度量基准）",
        cache_file="uup_1m_full.json",
        availability=DataAvailability.AVAILABLE,
    ),
    Vertex.P: DataSource(
        symbol="ES",
        description="标普期货（生产资本金融化 / 股票）",
        cache_file="es_1m_databento.json",
        availability=DataAvailability.AVAILABLE,
    ),
    Vertex.C: DataSource(
        symbol="DBC",
        description="广义商品指数 ETF（商品资本）。折叠通道模型下 C = 纯商品资本，"
                    "金的结算属性归 Au 折叠通道、不再是 C 代理。1min 经 TWS 拉取"
                    "（scripts/tws/fetch_1m_tws.py）。",
        cache_file="dbc_1m_tws.json",
        availability=DataAvailability.AVAILABLE,
    ),
    Vertex.R: DataSource(
        symbol="VNQ",
        description="房地产 ETF（不动产代理）。折叠模型下 R=不动产（330号固定资本容器），"
                    "正典代理用房地产 ETF（VNQ）而非债券——消解研究§6『TLT 既当 R 又属 "
                    "CASH』的冲突（债券久期属 M）。254号 OQ2：不动产高度本地化，跨经济体需"
                    "逐截面校准，VNQ 仅美国截面。1min 经 TWS 拉取（scripts/tws/fetch_1m_tws.py）。",
        cache_file="vnq_1m_tws.json",
        availability=DataAvailability.AVAILABLE,
    ),
}


# ── 折叠通道 → 观测标的 ──────────────────────────────────────────

FOLD_CHANNEL_DATA_SOURCES: dict[str, DataSource] = {
    AU.name: DataSource(
        symbol="GC",
        description="黄金（Au 折叠通道 C↔M 的 $ 相位观测量）",
        cache_file="gc_1m_databento.json",
        availability=DataAvailability.AVAILABLE,
    ),
    OIL.name: DataSource(
        symbol="CL",
        description="原油（Oil 通道 C→P 的 $ 相位观测量）",
        cache_file="cl_1m_databento.json",
        availability=DataAvailability.AVAILABLE,
    ),
}


def vertex_source(vertex: Vertex) -> DataSource:
    """返回顶点的数据源（单一真相源）。"""
    return VERTEX_DATA_SOURCES[vertex]


def fold_channel_source(channel: FoldChannel) -> DataSource:
    """返回折叠通道观测量的数据源。"""
    return FOLD_CHANNEL_DATA_SOURCES[channel.name]


def data_gaps() -> tuple[Vertex, ...]:
    """返回 1min 级别无完整数据源的顶点（诚实缺口清单，231号）。

    Returns
    -------
    tuple[Vertex, ...]
        availability 非 AVAILABLE 的顶点（当前：C 需采集、R 仅日线）。
    """
    return tuple(
        v for v, src in VERTEX_DATA_SOURCES.items()
        if src.availability is not DataAvailability.AVAILABLE
    )
