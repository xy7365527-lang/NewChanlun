"""标的资产分类器——从标的代码推断资产类型和方向策略。

M1发现：做空效果取决于标的属性（指数不做空，个股做空）。
分类规则将这一发现编码为选股层的结构性约束。

分类规则：
  ETF/指数（QQQ, SPY, DIA 等）→ long_only（指数ETF不做空）
  个股（OKLO, HK700, MU 等）  → both（多空皆可）
  期货/大宗（BZ, BRN, CL 等）  → both（多空皆可）

认识论标注：L2（M1回测验证——指数做空无alpha）。
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum


class AssetType(Enum):
    """标的资产类型。"""

    INDEX = "index"
    STOCK = "stock"
    COMMODITY = "commodity"


class DirectionPolicy(Enum):
    """方向策略。

    long_only: 仅做多（M1验证：指数做空无alpha）。
    both:      多空皆可。
    """

    LONG_ONLY = "long_only"
    BOTH = "both"


@dataclass(frozen=True, slots=True)
class AssetProfile:
    """单个标的的资产画像。

    Attributes
    ----------
    symbol : str
        标的代码（如 "QQQ"、"OKLO"）。
    asset_type : AssetType
        资产类型。
    base_direction : DirectionPolicy
        基于资产类型的基础方向策略（不含regime调整）。
    reason : str
        分类依据。
    """

    symbol: str
    asset_type: AssetType
    base_direction: DirectionPolicy
    reason: str


# ═══════════════════════════════════════════════════════════════
# 已知 ETF/指数集合
# ═══════════════════════════════════════════════════════════════

INDEX_ETFS: frozenset[str] = frozenset({
    "QQQ", "SPY", "DIA", "IWM", "VOO", "VTI",
    "TQQQ", "SQQQ", "SPXL", "SPXS",
    "EEM", "EFA", "VEA", "VWO",
    "XLK", "XLF", "XLE", "XLV", "XLI", "XLP", "XLY", "XLU", "XLB", "XLRE",
    "ARKK", "ARKG", "ARKW", "ARKF",
    "GLD", "SLV", "IAU", "GDX", "GDXJ",
    "USO", "OIH", "XOP",
    "TLT", "IEF", "SHY", "LQD", "HYG", "BND", "AGG",
    "VNQ", "IYR",
    "FXI", "KWEB", "MCHI",
})

COMMODITY_FUTURES: frozenset[str] = frozenset({
    "BZ", "BRN", "CL",
    "GC", "SI", "PL", "PA",
    "NG", "HO", "RB",
    "ZC", "ZW", "ZS", "ZM", "ZL",
    "HG", "ALI",
    "KC", "SB", "CC", "CT",
    "LE", "HE", "GF",
})


def _normalize_symbol(symbol: str) -> str:
    """去除交易所前缀和合约后缀。"""
    base = symbol.split(":")[-1] if ":" in symbol else symbol
    for suffix in ("1!", "2!", "!"):
        if base.endswith(suffix):
            base = base[:-len(suffix)]
    return base.upper()


def classify(symbol: str) -> AssetProfile:
    """对单个标的进行资产分类。

    Parameters
    ----------
    symbol : str
        标的代码。支持带交易所前缀（如 "NYMEX:CL1!"）。

    Returns
    -------
    AssetProfile
        包含资产类型和基础方向策略的画像。
    """
    normalized = _normalize_symbol(symbol)

    if normalized in INDEX_ETFS:
        return AssetProfile(
            symbol=symbol,
            asset_type=AssetType.INDEX,
            base_direction=DirectionPolicy.LONG_ONLY,
            reason="ETF/指数，M1验证不做空",
        )

    if normalized in COMMODITY_FUTURES:
        return AssetProfile(
            symbol=symbol,
            asset_type=AssetType.COMMODITY,
            base_direction=DirectionPolicy.BOTH,
            reason="期货，多空皆可",
        )

    return AssetProfile(
        symbol=symbol,
        asset_type=AssetType.STOCK,
        base_direction=DirectionPolicy.BOTH,
        reason="个股，多空皆可",
    )


def classify_batch(symbols: tuple[str, ...]) -> tuple[AssetProfile, ...]:
    """批量分类。"""
    return tuple(classify(s) for s in symbols)
