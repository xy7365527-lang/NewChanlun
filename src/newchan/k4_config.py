"""K4 配置编码器 — 将 K4 参数编码为可序列化配置。

四矩阵拓扑（K4 完全图）的配置管理：
  - 四个顶点（Equity/RealEstate/Commodity/Cash）的标的映射
  - 六条边的比价关系参数
  - 等价对验证阈值
  - eff.dim 监控参数

所有配置类为 frozen dataclass（不可变）。
序列化格式：JSON / YAML。

谱系引用：329号(K4定义), 337号(工程化下游), 254号(K4本体论)
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from newchan.matrix_topology import ALL_EDGES, AssetVertex


# ── 顶点配置 ──────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class VertexConfig:
    """单个顶点的标的映射配置。

    Attributes
    ----------
    vertex : AssetVertex
        四矩阵顶点。
    symbol : str
        代表性标的代码（如 "^GSPC"）。
    display_name : str
        显示名称（如 "S&P 500"）。
    data_source : str
        数据源标识（如 "yfinance", "fred"）。
    """

    vertex: AssetVertex
    symbol: str
    display_name: str
    data_source: str = "yfinance"


# ── 边配置 ────────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class EdgeConfig:
    """单条比价边的配置。

    Attributes
    ----------
    vertex_a : AssetVertex
        边的第一个顶点。
    vertex_b : AssetVertex
        边的第二个顶点。
    sym_a : str
        vertex_a 的代表性标的代码。
    sym_b : str
        vertex_b 的代表性标的代码。
    description : str
        资本流转含义描述（ratio_relation_v1.md §3.2）。
    """

    vertex_a: AssetVertex
    vertex_b: AssetVertex
    sym_a: str
    sym_b: str
    description: str = ""


# ── 等价对验证阈值 ────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class ValidationThresholds:
    """等价对验证三层阈值（equivalence.py §1.2）。

    Attributes
    ----------
    t_cv : float
        Layer 1: CV 预筛阈值。低于此值判定为退化。
    t_stroke_pct : float
        Layer 2: 笔力度均值阈值。
    t_dynamics_pct : float
        Layer 3: 归一化 MACD |hist| 均值阈值。
    t_liquidity : float
        C-3: 零成交量占比阈值。
    min_overlap : int
        C-1: 最小重叠K线数。
    """

    t_cv: float = 0.01
    t_stroke_pct: float = 0.005
    t_dynamics_pct: float = 0.0001
    t_liquidity: float = 0.5
    min_overlap: int = 5


# ── eff.dim 监控参数 ──────────────────────────────────────


@dataclass(frozen=True, slots=True)
class EffDimParams:
    """eff.dim 监控参数。

    Attributes
    ----------
    window : int
        滚动窗口大小（交易日数）。
    percentile_threshold : float
        同步压缩百分位阈值。
    """

    window: int = 252
    percentile_threshold: float = 5.0


# ── Basis 监控参数 ────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class BasisParams:
    """Basis 信号监控参数。

    Attributes
    ----------
    rolling_window : int
        滚动窗口大小（交易日数）。
    z_threshold : float
        z-score 预警阈值。
    gld_to_oz : float
        GLD ETF → 盎司换算系数。
    """

    rolling_window: int = 20
    z_threshold: float = 2.0
    gld_to_oz: float = 10.0


# ── K4 总配置 ─────────────────────────────────────────────


@dataclass(frozen=True, slots=True)
class K4Config:
    """K4 四矩阵拓扑完整配置。

    不可变，所有修改通过工厂函数返回新实例。

    Attributes
    ----------
    region : str
        经济体/区域标识。
    vertices : tuple[VertexConfig, ...]
        四个顶点的标的映射。
    edges : tuple[EdgeConfig, ...]
        六条比价边的配置。
    validation : ValidationThresholds
        等价对验证阈值。
    effdim : EffDimParams
        eff.dim 监控参数。
    basis : BasisParams
        Basis 信号监控参数。
    data_start : str
        数据起始日期（YYYY-MM-DD）。
    """

    region: str
    vertices: tuple[VertexConfig, ...]
    edges: tuple[EdgeConfig, ...]
    validation: ValidationThresholds = field(default_factory=ValidationThresholds)
    effdim: EffDimParams = field(default_factory=EffDimParams)
    basis: BasisParams = field(default_factory=BasisParams)
    data_start: str = "2000-01-01"


# ── 验证 ──────────────────────────────────────────────────


def validate_config(config: K4Config) -> list[str]:
    """验证 K4 配置的完整性和一致性。

    检查项：
      1. 恰好 4 个顶点，覆盖全部 AssetVertex
      2. 恰好 6 条边，覆盖全部 C(4,2) 组合
      3. 边的 sym_a/sym_b 与顶点配置中的 symbol 一致
      4. 无重复顶点/边

    Returns
    -------
    list[str]
        错误列表。空列表 = 验证通过。
    """
    errors: list[str] = []

    # 顶点检查
    vertex_set = {v.vertex for v in config.vertices}
    expected_vertices = set(AssetVertex)

    if len(config.vertices) != 4:
        errors.append(f"需要恰好 4 个顶点，实际 {len(config.vertices)} 个")
    if vertex_set != expected_vertices:
        missing = expected_vertices - vertex_set
        extra = vertex_set - expected_vertices
        if missing:
            errors.append(f"缺少顶点：{[v.value for v in missing]}")
        if extra:
            errors.append(f"多余顶点：{[v.value for v in extra]}")
    if len(vertex_set) != len(config.vertices):
        errors.append("存在重复顶点")

    # 边检查
    expected_edge_set = {frozenset(pair) for pair in ALL_EDGES}
    actual_edge_set: set[frozenset[AssetVertex]] = set()

    for edge in config.edges:
        key = frozenset([edge.vertex_a, edge.vertex_b])
        if edge.vertex_a == edge.vertex_b:
            errors.append(f"自环边：{edge.vertex_a.value}")
            continue
        if key in actual_edge_set:
            errors.append(
                f"重复边：{edge.vertex_a.value}/{edge.vertex_b.value}"
            )
        actual_edge_set.add(key)

    if len(config.edges) != 6:
        errors.append(f"需要恰好 6 条边，实际 {len(config.edges)} 条")
    missing_edges = expected_edge_set - actual_edge_set
    extra_edges = actual_edge_set - expected_edge_set
    if missing_edges:
        for me in missing_edges:
            pair = list(me)
            errors.append(f"缺少边：{pair[0].value}/{pair[1].value}")
    if extra_edges:
        for ee in extra_edges:
            pair = list(ee)
            errors.append(f"多余边：{pair[0].value}/{pair[1].value}")

    # 边的 symbol 与顶点一致性
    vertex_symbol_map = {v.vertex: v.symbol for v in config.vertices}
    for edge in config.edges:
        expected_a = vertex_symbol_map.get(edge.vertex_a)
        expected_b = vertex_symbol_map.get(edge.vertex_b)
        if expected_a is not None and edge.sym_a != expected_a:
            errors.append(
                f"边 {edge.vertex_a.value}/{edge.vertex_b.value} 的 sym_a "
                f"'{edge.sym_a}' 与顶点 {edge.vertex_a.value} 的 symbol "
                f"'{expected_a}' 不一致"
            )
        if expected_b is not None and edge.sym_b != expected_b:
            errors.append(
                f"边 {edge.vertex_a.value}/{edge.vertex_b.value} 的 sym_b "
                f"'{edge.sym_b}' 与顶点 {edge.vertex_b.value} 的 symbol "
                f"'{expected_b}' 不一致"
            )

    return errors


# ── 序列化 ────────────────────────────────────────────────

_VERTEX_FROM_STR = {v.name: v for v in AssetVertex}


def _vertex_to_str(v: AssetVertex) -> str:
    return v.name


def _vertex_from_str(s: str) -> AssetVertex:
    if s not in _VERTEX_FROM_STR:
        raise ValueError(f"未知顶点：{s!r}，合法值：{list(_VERTEX_FROM_STR)}")
    return _VERTEX_FROM_STR[s]


def config_to_dict(config: K4Config) -> dict[str, Any]:
    """将 K4Config 序列化为纯字典（可直接 json.dumps）。"""
    return {
        "region": config.region,
        "data_start": config.data_start,
        "vertices": [
            {
                "vertex": _vertex_to_str(v.vertex),
                "symbol": v.symbol,
                "display_name": v.display_name,
                "data_source": v.data_source,
            }
            for v in config.vertices
        ],
        "edges": [
            {
                "vertex_a": _vertex_to_str(e.vertex_a),
                "vertex_b": _vertex_to_str(e.vertex_b),
                "sym_a": e.sym_a,
                "sym_b": e.sym_b,
                "description": e.description,
            }
            for e in config.edges
        ],
        "validation": {
            "t_cv": config.validation.t_cv,
            "t_stroke_pct": config.validation.t_stroke_pct,
            "t_dynamics_pct": config.validation.t_dynamics_pct,
            "t_liquidity": config.validation.t_liquidity,
            "min_overlap": config.validation.min_overlap,
        },
        "effdim": {
            "window": config.effdim.window,
            "percentile_threshold": config.effdim.percentile_threshold,
        },
        "basis": {
            "rolling_window": config.basis.rolling_window,
            "z_threshold": config.basis.z_threshold,
            "gld_to_oz": config.basis.gld_to_oz,
        },
    }


def config_from_dict(d: dict[str, Any]) -> K4Config:
    """从字典反序列化为 K4Config。"""
    vertices = tuple(
        VertexConfig(
            vertex=_vertex_from_str(v["vertex"]),
            symbol=v["symbol"],
            display_name=v["display_name"],
            data_source=v.get("data_source", "yfinance"),
        )
        for v in d["vertices"]
    )
    edges = tuple(
        EdgeConfig(
            vertex_a=_vertex_from_str(e["vertex_a"]),
            vertex_b=_vertex_from_str(e["vertex_b"]),
            sym_a=e["sym_a"],
            sym_b=e["sym_b"],
            description=e.get("description", ""),
        )
        for e in d["edges"]
    )
    val = d.get("validation", {})
    effdim = d.get("effdim", {})
    basis = d.get("basis", {})

    return K4Config(
        region=d["region"],
        data_start=d.get("data_start", "2000-01-01"),
        vertices=vertices,
        edges=edges,
        validation=ValidationThresholds(
            t_cv=val.get("t_cv", 0.01),
            t_stroke_pct=val.get("t_stroke_pct", 0.005),
            t_dynamics_pct=val.get("t_dynamics_pct", 0.0001),
            t_liquidity=val.get("t_liquidity", 0.5),
            min_overlap=val.get("min_overlap", 5),
        ),
        effdim=EffDimParams(
            window=effdim.get("window", 252),
            percentile_threshold=effdim.get("percentile_threshold", 5.0),
        ),
        basis=BasisParams(
            rolling_window=basis.get("rolling_window", 20),
            z_threshold=basis.get("z_threshold", 2.0),
            gld_to_oz=basis.get("gld_to_oz", 10.0),
        ),
    )


def config_to_json(config: K4Config, indent: int = 2) -> str:
    """序列化为 JSON 字符串。"""
    return json.dumps(
        config_to_dict(config), indent=indent, ensure_ascii=False,
    )


def config_from_json(s: str) -> K4Config:
    """从 JSON 字符串反序列化。"""
    return config_from_dict(json.loads(s))


def save_config(config: K4Config, path: str | Path) -> None:
    """保存配置到 JSON 文件。"""
    p = Path(path)
    p.write_text(config_to_json(config), encoding="utf-8")


def load_config(path: str | Path) -> K4Config:
    """从 JSON 文件加载配置。"""
    p = Path(path)
    return config_from_json(p.read_text(encoding="utf-8"))


# ── YAML 序列化（可选依赖）────────────────────────────────


def config_to_yaml(config: K4Config) -> str:
    """序列化为 YAML 字符串。需要 pyyaml。"""
    try:
        import yaml
    except ImportError as exc:
        raise ImportError(
            "YAML 序列化需要 pyyaml：pip install pyyaml"
        ) from exc
    return yaml.dump(
        config_to_dict(config),
        default_flow_style=False,
        allow_unicode=True,
        sort_keys=False,
    )


def config_from_yaml(s: str) -> K4Config:
    """从 YAML 字符串反序列化。需要 pyyaml。"""
    try:
        import yaml
    except ImportError as exc:
        raise ImportError(
            "YAML 反序列化需要 pyyaml：pip install pyyaml"
        ) from exc
    return config_from_dict(yaml.safe_load(s))


# ── 默认配置（k4_monitor.py 当前参数）────────────────────


# ratio_relation_v1.md §3.2 描述
_DEFAULT_EDGE_DESCRIPTIONS: dict[frozenset[str], str] = {
    frozenset(["EQUITY", "CASH"]): "股市整体涨跌",
    frozenset(["REAL_ESTATE", "CASH"]): "实际房价变化",
    frozenset(["COMMODITY", "CASH"]): "实物通胀",
    frozenset(["EQUITY", "REAL_ESTATE"]): "金融资产 vs 实物资产偏好",
    frozenset(["EQUITY", "COMMODITY"]): "金融资产 vs 避险资产",
    frozenset(["COMMODITY", "REAL_ESTATE"]): "实物之间的配置偏好",
}

# 顶点→标的映射（来自 k4_monitor.py NODES）
_DEFAULT_VERTEX_SYMBOLS: dict[AssetVertex, tuple[str, str, str]] = {
    # (symbol, display_name, data_source)
    AssetVertex.EQUITY: ("^GSPC", "S&P 500", "yfinance"),
    AssetVertex.REAL_ESTATE: ("DX-Y.NYB", "US Dollar Index", "yfinance"),
    AssetVertex.COMMODITY: ("DCOILWTICO", "WTI Crude Oil", "fred"),
    AssetVertex.CASH: ("GC=F", "Gold Futures", "yfinance"),
}


def make_default_config(region: str = "US") -> K4Config:
    """创建默认 K4 配置（与 k4_monitor.py 参数一致）。

    顶点映射：
      EQUITY     → ^GSPC (S&P 500)
      REAL_ESTATE → DX-Y.NYB (US Dollar Index)
      COMMODITY  → DCOILWTICO (WTI Crude Oil, FRED)
      CASH       → GC=F (Gold Futures)

    注意：k4_monitor.py 中 NODES 的命名（Au/USD/Equity/Commodity）与
    AssetVertex 枚举（EQUITY/REAL_ESTATE/COMMODITY/CASH）的映射关系：
      Au        → CASH（黄金是现金等价物）
      USD       → REAL_ESTATE（美元指数代理不动产/主权信用）
      Equity    → EQUITY
      Commodity → COMMODITY
    """
    vertices = tuple(
        VertexConfig(
            vertex=v,
            symbol=sym,
            display_name=name,
            data_source=src,
        )
        for v, (sym, name, src) in _DEFAULT_VERTEX_SYMBOLS.items()
    )

    vertex_sym = {v: sym for v, (sym, _, _) in _DEFAULT_VERTEX_SYMBOLS.items()}
    edges = tuple(
        EdgeConfig(
            vertex_a=a,
            vertex_b=b,
            sym_a=vertex_sym[a],
            sym_b=vertex_sym[b],
            description=_DEFAULT_EDGE_DESCRIPTIONS.get(
                frozenset([a.name, b.name]), ""
            ),
        )
        for a, b in ALL_EDGES
    )

    return K4Config(
        region=region,
        vertices=vertices,
        edges=edges,
    )
