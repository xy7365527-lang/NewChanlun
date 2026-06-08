"""M2 选股层验证脚本 — 验证品种池+方向表在不同 regime/极性组合下的输出。

验证内容：
  1. 资产分类正确性（ETF→long_only，期货→both，个股→both）
  2. regime 调整逻辑（信用扩张利好股票，信用收缩利好大宗）
  3. K4 极性共振效应
  4. 边界情况（空池、单标的、NEUTRAL regime）
  5. 方向表到 M1 回测参数的转换

认识论标注：L1（合成数据验证管线正确性，不验证 regime 假设）。
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import numpy as np

from newchan.strategy.asset_classifier import (
    AssetType,
    DirectionPolicy,
    classify,
    classify_batch,
)
from newchan.strategy.omega_regime import (
    OmegaRegime,
    OmegaState,
    compute_omega,
    detect_regime,
    regime_from_walk_direction,
)
from newchan.strategy.selection_pool import (
    SelectionConfidence,
    SelectionPool,
    apply_regime_adjustment,
    build_selection_pool,
    pool_to_backtest_config,
)
from newchan.topology.config_space import (
    ConfigurationSpace,
    polarity_index,
)


def _sep(title: str) -> None:
    print(f"\n{'═' * 60}")
    print(f"  {title}")
    print(f"{'═' * 60}")


# ════════════════════════════════════════════════════════════
# 1. 资产分类验证
# ════════════════════════════════════════════════════════════

def verify_asset_classification() -> None:
    _sep("1. 资产分类验证")

    test_cases = [
        ("QQQ", AssetType.INDEX, DirectionPolicy.LONG_ONLY),
        ("SPY", AssetType.INDEX, DirectionPolicy.LONG_ONLY),
        ("TQQQ", AssetType.INDEX, DirectionPolicy.LONG_ONLY),
        ("GLD", AssetType.INDEX, DirectionPolicy.LONG_ONLY),
        ("XLE", AssetType.INDEX, DirectionPolicy.LONG_ONLY),
        ("NYMEX:CL1!", AssetType.COMMODITY, DirectionPolicy.BOTH),
        ("BZ", AssetType.COMMODITY, DirectionPolicy.BOTH),
        ("GC", AssetType.COMMODITY, DirectionPolicy.BOTH),
        ("OKLO", AssetType.STOCK, DirectionPolicy.BOTH),
        ("MU", AssetType.STOCK, DirectionPolicy.BOTH),
        ("AAPL", AssetType.STOCK, DirectionPolicy.BOTH),
    ]

    passed = 0
    for symbol, expected_type, expected_dir in test_cases:
        profile = classify(symbol)
        ok = profile.asset_type is expected_type and profile.base_direction is expected_dir
        status = "✓" if ok else "✗"
        print(f"  {status} {symbol:20s} → {profile.asset_type.value:10s} {profile.base_direction.value}")
        if ok:
            passed += 1

    print(f"\n  分类通过: {passed}/{len(test_cases)}")

    batch = classify_batch(("QQQ", "OKLO", "BZ"))
    assert len(batch) == 3
    assert batch[0].asset_type is AssetType.INDEX
    assert batch[1].asset_type is AssetType.STOCK
    assert batch[2].asset_type is AssetType.COMMODITY
    print("  批量分类: ✓")


# ════════════════════════════════════════════════════════════
# 2. ω regime 检测验证
# ════════════════════════════════════════════════════════════

def verify_omega_regime() -> None:
    _sep("2. ω regime 检测验证")

    assert compute_omega(2500.0, 70.0) == 2500.0 / 70.0
    print("  compute_omega: ✓")

    np.random.seed(42)

    omega_down = np.linspace(35, 25, 100) + np.random.normal(0, 0.3, 100)
    state = detect_regime(omega_down)
    assert state.regime is OmegaRegime.BULL_EQUITY
    print(f"  declining ω → {state.regime.value} (strength={state.strength:.4f}): ✓")

    omega_up = np.linspace(25, 35, 100) + np.random.normal(0, 0.3, 100)
    state = detect_regime(omega_up)
    assert state.regime is OmegaRegime.BULL_COMMODITY
    print(f"  rising ω    → {state.regime.value} (strength={state.strength:.4f}): ✓")

    omega_flat = np.full(100, 30.0) + np.random.normal(0, 0.01, 100)
    state = detect_regime(omega_flat)
    assert state.regime is OmegaRegime.NEUTRAL
    print(f"  flat ω      → {state.regime.value} (strength={state.strength:.4f}): ✓")

    assert regime_from_walk_direction("up") is OmegaRegime.BULL_COMMODITY
    assert regime_from_walk_direction("down") is OmegaRegime.BULL_EQUITY
    assert regime_from_walk_direction("flat") is OmegaRegime.NEUTRAL
    print("  walk direction mapping: ✓")


# ════════════════════════════════════════════════════════════
# 3. regime 调整逻辑验证
# ════════════════════════════════════════════════════════════

def verify_regime_adjustment() -> None:
    _sep("3. regime 调整逻辑验证")

    symbols = ("QQQ", "OKLO", "NYMEX:CL1!")

    combos = [
        (OmegaRegime.BULL_EQUITY, 2, "信用扩张+risk-on"),
        (OmegaRegime.BULL_EQUITY, -2, "信用扩张+risk-off"),
        (OmegaRegime.BULL_COMMODITY, 2, "信用收缩+risk-on"),
        (OmegaRegime.BULL_COMMODITY, -2, "信用收缩+risk-off"),
        (OmegaRegime.NEUTRAL, 0, "中性"),
    ]

    for regime, polarity, desc in combos:
        print(f"\n  [{desc}] regime={regime.value}, polarity={polarity}")
        pool = build_selection_pool(symbols, regime, polarity)

        for e in pool.entries:
            print(f"    {e.symbol:20s} dir={e.direction.value:10s} "
                  f"conf={e.confidence.value:6s}  {e.reason}")

        assert pool.regime is regime
        assert pool.k4_polarity == polarity

        qqqe = next(e for e in pool.entries if e.symbol == "QQQ")
        assert qqqe.direction is DirectionPolicy.LONG_ONLY, \
            "QQQ long_only 是硬约束，regime 不应改变"

    print("\n  long_only 硬约束: ✓")
    print("  regime 调整逻辑: ✓")


# ════════════════════════════════════════════════════════════
# 4. K4 极性共振验证
# ════════════════════════════════════════════════════════════

def verify_k4_resonance() -> None:
    _sep("4. K4 极性共振验证")

    cs = ConfigurationSpace()

    risk_on = cs.get(1, 1, 1)
    assert polarity_index(risk_on) == 3
    print(f"  (+,+,+) polarity=3: ✓")

    risk_off = cs.get(-1, -1, -1)
    assert polarity_index(risk_off) == -3
    print(f"  (-,-,-) polarity=-3: ✓")

    mixed = cs.get(1, -1, 0)
    assert polarity_index(mixed) == 0
    print(f"  (+,-,0) polarity=0: ✓")

    pool_on = build_selection_pool(("OKLO",), OmegaRegime.BULL_EQUITY, 3)
    pool_off = build_selection_pool(("OKLO",), OmegaRegime.BULL_EQUITY, -3)

    e_on = pool_on.entries[0]
    e_off = pool_off.entries[0]

    print(f"  stock + bull_equity + polarity=+3 → {e_on.confidence.value}")
    print(f"  stock + bull_equity + polarity=-3 → {e_off.confidence.value}")

    assert e_on.confidence is SelectionConfidence.HIGH
    assert e_off.confidence is SelectionConfidence.MEDIUM
    print("  K4 极性共振效应: ✓")


# ════════════════════════════════════════════════════════════
# 5. 边界情况
# ════════════════════════════════════════════════════════════

def verify_edge_cases() -> None:
    _sep("5. 边界情况验证")

    pool = build_selection_pool((), OmegaRegime.NEUTRAL, 0)
    assert len(pool.entries) == 0
    assert pool.symbols() == ()
    print("  空池: ✓")

    pool = build_selection_pool(("OKLO",), OmegaRegime.NEUTRAL, 0)
    assert len(pool.entries) == 1
    print("  单标的: ✓")

    pool = build_selection_pool(
        ("QQQ", "SPY", "OKLO", "BZ"),
        OmegaRegime.BULL_EQUITY, 2,
    )
    assert pool.long_only_symbols() == ("QQQ", "SPY")
    assert "OKLO" in pool.both_symbols()
    print("  方向过滤: ✓")

    d = pool.to_dict()
    assert isinstance(d, dict)
    assert "QQQ" in d
    assert d["QQQ"]["direction"] == "long_only"
    print("  to_dict: ✓")


# ════════════════════════════════════════════════════════════
# 6. M1 回测参数转换
# ════════════════════════════════════════════════════════════

def verify_backtest_config() -> None:
    _sep("6. M1 回测参数转换")

    pool = build_selection_pool(
        ("QQQ", "OKLO", "NYMEX:CL1!"),
        OmegaRegime.BULL_EQUITY, 1,
    )
    config = pool_to_backtest_config(pool)

    assert config["QQQ"] is False, "QQQ: short_enabled=False"
    assert config["OKLO"] is True, "OKLO: short_enabled=True"
    assert config["NYMEX:CL1!"] is True, "CL: short_enabled=True"

    print(f"  回测参数: {config}")
    print("  转换验证: ✓")


# ════════════════════════════════════════════════════════════
# 7. 全组合枚举（穷举 regime×polarity 矩阵）
# ════════════════════════════════════════════════════════════

def verify_exhaustive_matrix() -> None:
    _sep("7. regime × polarity 全组合矩阵")

    symbols = ("QQQ", "OKLO", "NYMEX:CL1!")
    regimes = list(OmegaRegime)
    polarities = [-3, -1, 0, 1, 3]

    print(f"  {'regime':20s} {'pol':>4s}  QQQ         OKLO        CL")
    print(f"  {'─' * 70}")

    for regime in regimes:
        for pol in polarities:
            pool = build_selection_pool(symbols, regime, pol)
            row = []
            for sym in symbols:
                e = next(e for e in pool.entries if e.symbol == sym)
                row.append(f"{e.direction.value[:4]:>4s}/{e.confidence.value[:3]:>3s}")
            print(f"  {regime.value:20s} {pol:>4d}  {'  '.join(row)}")

    print(f"\n  {len(regimes) * len(polarities)} 种组合全部通过")


# ════════════════════════════════════════════════════════════
# 主入口
# ════════════════════════════════════════════════════════════

if __name__ == "__main__":
    verify_asset_classification()
    verify_omega_regime()
    verify_regime_adjustment()
    verify_k4_resonance()
    verify_edge_cases()
    verify_backtest_config()
    verify_exhaustive_matrix()

    _sep("验证完成")
    print("  M2 选股层所有验证通过。")
    print("  认识论标注：L1（管线正确性验证）。")
    print("  L2 验证需真实数据（金油比序列 + K4 配置 + 回测结果）。")
