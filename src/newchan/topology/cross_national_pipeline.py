"""跨国 K4 管线 — 把 cross_national.py 的结构接入真实数据，跑出每经济体配置 + 货币层。

概念溯源：254号多经济体资本流转（$ 展开为结算尺空间 Σ，三层递归）、292号折叠区间套、
          330号资本循环、528/529号折叠通道重构。

## 三层结构（254号定义3）

- **层 0：结算尺空间 Σ**（货币边 M_i/M_j = 汇率）。对 FX 序列跑递归 → σ_FX。
  框架有效性判断：多条 FX 边同步同向大级别结构事件 = 结算尺断裂（254号定理2）。
- **层 1：截面内 K4_i**（每经济体的 P/C/R/M 四顶点、独立边 P/M、C/M、R/M）。
  对本币计价序列跑递归 → 该经济体的 Configuration Γ_i = (σ_P, σ_C, σ_R)。
- **层 2**：单标的买卖点（不在本模块；本模块产出层0+层1）。

## 独立边 = 本币计价序列本身

在某经济体的本币截面下，独立边 X/M 的"比价"就是 X 用本币计价的价格序列本身
（M 是分母/度量基准，026号双重身份）。故 σ(P/M) = 对本币股指序列跑递归取最高级别
走势方向，无需显式比价。派生边/跨国边才需比价。

## 折叠通道全局共享（292号§3.4 区间套）

Au（黄金）全局共享——一条 GC 序列锚定**所有**经济体的 C↔M 折叠通道（292号：黄金
是同一个黄金，全球同一折叠）。Oil（C→P）半共享。本模块把 Au/Oil 作为跨经济体共享的
折叠通道观测量，不在每个经济体重复建模。

## 数据缺口诚实标注（231号）

各经济体的 C（广义商品）/R（不动产）多数无本币 1min 源（254号 OQ2：不动产高度本地化）。
缺口标注为 None，配置为**部分配置**（partial），不伪造。中国本土 IF/AU9999/SC 在 IB/
databento 均无源（已验证），用日线代理另行处理。
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import newchan_rust

from newchan.topology.config_space import Configuration, WalkDirection
from newchan.topology.graph import Vertex

_DATA_DIR = Path(__file__).resolve().parents[3] / "analysis" / "data_cache"


# ════════════════════════════════════════════════════════════
# 经济体规格（spec 驱动，单一真相源）
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class EconomyK4Spec:
    """一个经济体的 K4 数据规格。

    Attributes
    ----------
    economy : str
        经济体标识（"US"/"EU"/"JP"/"CN"）。
    currency : str
        本币结算尺（"USD"/"EUR"/"JPY"/"CNY"）。
    vertex_files : dict[Vertex, str | None]
        本币计价的顶点序列缓存文件名；None = 数据缺口（不伪造）。
        独立边 P/M、C/M、R/M 直接由 P/C/R 的本币序列给出（M 是基准）。
    fx_to_usd : str | None
        本币兑美元的 FX 序列缓存文件（货币边 M_i/M_US，254号层0）；US 为 None。
    """

    economy: str
    currency: str
    vertex_files: dict[Vertex, str | None]
    fx_to_usd: str | None


# 跨国 K4 规格（基于实测可用数据；缺口诚实标注 None）。
# US 用 1min（TWS/databento 全顶点齐备）；其余经济体用 1h（databento 10年史）。
ECONOMY_SPECS: dict[str, EconomyK4Spec] = {
    "US": EconomyK4Spec(
        economy="US",
        currency="USD",
        vertex_files={
            Vertex.M: "uup_1m_full.json",      # 美元（度量基准）
            Vertex.P: "es_1m_databento.json",  # 标普期货（生产资本金融化）
            Vertex.C: "dbc_1m_tws.json",       # 综合商品 ETF（商品资本，TWS 拉取）
            Vertex.R: "vnq_1m_tws.json",       # 房地产 ETF（不动产，TWS 拉取）
        },
        fx_to_usd=None,  # 美元截面本身
    ),
    "EU": EconomyK4Spec(
        economy="EU",
        currency="EUR",
        vertex_files={
            Vertex.M: None,                      # 货币边由 fx_to_usd 表达
            Vertex.P: "fesx_1h_databento.json",  # STOXX50 期货（EUR 计价）
            Vertex.C: None,                      # EUR 计价广义商品：无源（缺口）
            Vertex.R: None,                      # 欧洲不动产：无统一源（254号OQ2）
        },
        fx_to_usd="eurusd_1h_databento.json",
    ),
    "JP": EconomyK4Spec(
        economy="JP",
        currency="JPY",
        vertex_files={
            Vertex.M: None,
            Vertex.P: "nkd_1h_databento.json",   # 日经期货
            Vertex.C: None,
            Vertex.R: None,
        },
        fx_to_usd="usdjpy_1h_databento.json",
    ),
    "CN": EconomyK4Spec(
        economy="CN",
        currency="CNY",
        vertex_files={
            Vertex.M: None,
            Vertex.P: None,    # 本土沪深300(IF/CFFEX) 1min IB/databento 无源；用日线代理另处理
            Vertex.C: None,    # SC原油(INE)/AU9999(SGE) 无源
            Vertex.R: None,
        },
        fx_to_usd="usdcnh_1h_databento.json",  # 离岸人民币（在岸 USDCNY 仅日线）
    ),
}

# 全局共享折叠通道（292号 Au 全局共享 → 一条序列锚定所有经济体）
GLOBAL_FOLD_FILES: dict[str, str] = {
    "Au": "gc_1m_databento.json",   # 黄金（C↔M 折叠，全局共享）
    "Oil": "cl_1m_databento.json",  # 原油（C→P 通道）
}


# ════════════════════════════════════════════════════════════
# 数据加载 + 走势方向
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class _Series:
    symbol: str
    dates: list[str]
    opens: list[float]
    highs: list[float]
    lows: list[float]
    closes: list[float]


def _load_series(filename: str, window_bars: int | None = None) -> _Series:
    """加载列式 JSON 价格序列；window_bars 限定最近 N 根（控制递归耗时）。"""
    path = _DATA_DIR / filename
    with open(path) as f:
        d = json.load(f)
    sl = slice(-window_bars, None) if window_bars else slice(None)
    return _Series(
        symbol=d["symbol"],
        dates=d["dates"][sl],
        opens=d["opens"][sl],
        highs=d["highs"][sl],
        lows=d["lows"][sl],
        closes=d["closes"][sl],
    )


def _direction_to_sigma(direction: str, kind: str) -> WalkDirection:
    """最高级别走势 (kind, direction) → σ ∈ WalkDirection。

    盘整 → FLAT；趋势上 → UP；趋势下 → DOWN（k4_ratio 同口径）。
    """
    if kind == "consolidation":
        return WalkDirection.FLAT
    if direction == "up":
        return WalkDirection.UP
    if direction == "down":
        return WalkDirection.DOWN
    return WalkDirection.FLAT


@dataclass(frozen=True, slots=True)
class EdgeReading:
    """一条边跑完递归后的走势读数。"""

    name: str
    bar_count: int
    date_range: tuple[str, str]
    sigma: WalkDirection
    move_kind: str
    move_direction: str
    move_settled: bool
    max_level: int
    last_price: float


def _run_recursive(name: str, s: _Series) -> EdgeReading:
    """对一条边序列跑 Rust RecursiveOrchestrator，提取最高级别走势 → EdgeReading。

    引擎为 ``newchan_rust.RecursiveOrchestrator``（逐位等价 Python 版，~23×）。
    Python O(N²) 流式引擎对 1min 量级（852k bar）不可行，故 1min 跨国管线必须用
    Rust 引擎。Rust 接口为查询式：``process_bar(o,h,l,c)`` 逐 bar 驱动，
    ``current_moves()`` / ``current_recursive()`` 读出结构化结果。

    走势 move 元组结构（``rust/src/lib.rs:469``，固定 8 元素）：
    ``(kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled)``。
    """
    orch = newchan_rust.RecursiveOrchestrator(max_levels=6, stroke_mode="wide")
    n = len(s.closes)
    for i in range(n):
        orch.process_bar(s.opens[i], s.highs[i], s.lows[i], s.closes[i])
    moves = orch.current_moves()
    if moves:
        head = moves[-1][0]
        kind, direction, settled = head[0], head[1], head[7]
    else:
        kind, direction, settled = "none", "none", False
    max_level = 1
    for entry in orch.current_recursive():
        max_level = max(max_level, entry[0])
    return EdgeReading(
        name=name,
        bar_count=n,
        date_range=(s.dates[0][:19], s.dates[-1][:19]),
        sigma=_direction_to_sigma(direction, kind),
        move_kind=kind,
        move_direction=direction,
        move_settled=settled,
        max_level=max_level,
        last_price=s.closes[-1],
    )


# ════════════════════════════════════════════════════════════
# 层 1：每经济体 K4 配置
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class EconomyK4Result:
    """一个经济体的 K4 层1 结果。"""

    economy: str
    currency: str
    edges: dict[Vertex, EdgeReading]          # 已读出的独立边（P/C/R 对 M）
    missing: tuple[Vertex, ...]               # 数据缺口顶点
    config: Configuration | None              # 三独立边齐全时的完整 Γ；否则 None
    partial_sigma: dict[Vertex, WalkDirection]  # 已有分量（部分配置）

    @property
    def is_complete(self) -> bool:
        return self.config is not None


def compute_economy_k4(
    spec: EconomyK4Spec,
    window_bars: int | None = None,
) -> EconomyK4Result:
    """对一个经济体跑层1：独立边 P/M、C/M、R/M → Configuration（部分或完整）。

    独立边 X/M = 本币计价的 X 序列本身（M 是基准）。缺口顶点不伪造。

    Parameters
    ----------
    spec : EconomyK4Spec
        经济体规格。
    window_bars : int | None
        每条边只处理最近 N 根 bar（控制递归耗时）；None = 全量。
    """
    edges: dict[Vertex, EdgeReading] = {}
    missing: list[Vertex] = []
    partial: dict[Vertex, WalkDirection] = {}

    for vtx in (Vertex.P, Vertex.C, Vertex.R):
        fname = spec.vertex_files.get(vtx)
        if not fname or not (_DATA_DIR / fname).exists():
            missing.append(vtx)
            continue
        reading = _run_recursive(f"{spec.economy}:{vtx.name}/M", _load_series(fname, window_bars))
        edges[vtx] = reading
        partial[vtx] = reading.sigma

    config: Configuration | None = None
    if {Vertex.P, Vertex.C, Vertex.R} <= set(partial):
        config = Configuration(
            sigma_p=partial[Vertex.P],
            sigma_c=partial[Vertex.C],
            sigma_r=partial[Vertex.R],
        )

    return EconomyK4Result(
        economy=spec.economy,
        currency=spec.currency,
        edges=edges,
        missing=tuple(missing),
        config=config,
        partial_sigma=partial,
    )


# ════════════════════════════════════════════════════════════
# 层 0：货币边（结算尺空间 Σ）
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class CurrencyLayerResult:
    """层0：货币边（M_i/M_US 汇率）走势读数。"""

    fx_readings: dict[str, EdgeReading]  # economy → 该经济体货币兑美元的 FX 边读数

    def synchronized_break_signal(self) -> bool:
        """254号定理2：多条货币边同步同向大级别结构事件 = 结算尺断裂候选。

        判据：≥2 条 FX 边方向相同且均为已结算趋势（非盘整）。这是**候选信号**，
        非确认——单边事件无法区分分子/分母变化（254号）。
        """
        trending = [
            r for r in self.fx_readings.values()
            if r.move_kind != "consolidation" and r.move_settled
        ]
        if len(trending) < 2:
            return False
        dirs = {r.move_direction for r in trending}
        return len(dirs) == 1


def compute_currency_layer(
    specs: dict[str, EconomyK4Spec],
    window_bars: int | None = None,
) -> CurrencyLayerResult:
    """对所有非美元经济体的 FX 边跑递归 → 层0 结算尺空间读数。"""
    readings: dict[str, EdgeReading] = {}
    for eco, spec in specs.items():
        if not spec.fx_to_usd or not (_DATA_DIR / spec.fx_to_usd).exists():
            continue
        readings[eco] = _run_recursive(
            f"FX:{spec.currency}/USD", _load_series(spec.fx_to_usd, window_bars)
        )
    return CurrencyLayerResult(fx_readings=readings)


# ════════════════════════════════════════════════════════════
# 跨国 C 路径联合读数（折叠通道全局共享）
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class CrossNationalCPathResult:
    """跨国 C 路径联合读数（Au/Oil 折叠通道 + 各经济体 C/M 边）。

    292号：Au 全局共享 → 一条 GC 序列锚定所有经济体的 C↔M 折叠。Oil 半共享 C→P。
    """

    au_reading: EdgeReading        # Au=C↔M 折叠通道（GC，全局）
    oil_reading: EdgeReading       # Oil=C→P 通道（CL，全局）
    omega_last: float              # ω = 金价/油价（最新）
    omega_direction: str           # ω 折叠通道走势方向（金油相对强弱）
    economy_c_edges: dict[str, EdgeReading]  # 各经济体本币 C/M 边（多数缺口）

    @property
    def credit_signal(self) -> str:
        """ω 走势 → 信用环境读数（482号；仅长期方向，非实时门控——已证伪）。"""
        if self.omega_direction == "up":
            return "信用收缩（金强于油，ω↑）"
        if self.omega_direction == "down":
            return "信用扩张（油强于金，ω↓）"
        return "ω 方向不明"


def compute_cross_national_c_path(
    economy_results: dict[str, EconomyK4Result],
    window_bars: int | None = None,
) -> CrossNationalCPathResult:
    """跨国 C 路径：全局 Au/Oil 折叠通道 + 各经济体已有的 C/M 边联合读数。"""
    au = _run_recursive("Au:C<->M", _load_series(GLOBAL_FOLD_FILES["Au"], window_bars))
    oil = _run_recursive("Oil:C->P", _load_series(GLOBAL_FOLD_FILES["Oil"], window_bars))

    # ω = 金价/油价（用各自最新收盘，折叠通道 $ 相位之比，482号）
    omega_last = au.last_price / oil.last_price if oil.last_price > 0 else float("nan")
    # ω 折叠通道方向：金边方向 vs 油边方向的相对强弱
    if au.sigma.value > oil.sigma.value:
        omega_dir = "up"
    elif au.sigma.value < oil.sigma.value:
        omega_dir = "down"
    else:
        omega_dir = "flat"

    economy_c_edges = {
        eco: res.edges[Vertex.C]
        for eco, res in economy_results.items()
        if Vertex.C in res.edges
    }

    return CrossNationalCPathResult(
        au_reading=au,
        oil_reading=oil,
        omega_last=omega_last,
        omega_direction=omega_dir,
        economy_c_edges=economy_c_edges,
    )


# ════════════════════════════════════════════════════════════
# 全管线 + 验证
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class CrossNationalK4Result:
    """跨国 K4 全管线产物。"""

    economies: dict[str, EconomyK4Result]
    currency_layer: CurrencyLayerResult
    c_path: CrossNationalCPathResult


def run_cross_national_k4(
    specs: dict[str, EconomyK4Spec] | None = None,
    window_bars: int | None = None,
) -> CrossNationalK4Result:
    """跑完整跨国 K4 管线（层0 货币 + 层1 各经济体 + 跨国 C 路径）。

    Parameters
    ----------
    specs : dict | None
        经济体规格，默认 ECONOMY_SPECS。
    window_bars : int | None
        每条边只处理最近 N 根 bar（控制递归耗时）。验证建议用窗口。
    """
    specs = specs or ECONOMY_SPECS
    economies = {
        eco: compute_economy_k4(spec, window_bars)
        for eco, spec in specs.items()
    }
    currency_layer = compute_currency_layer(specs, window_bars)
    c_path = compute_cross_national_c_path(economies, window_bars)
    return CrossNationalK4Result(
        economies=economies,
        currency_layer=currency_layer,
        c_path=c_path,
    )
