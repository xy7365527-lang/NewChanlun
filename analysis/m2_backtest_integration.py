"""M2 → M1 回测对接示例 — 品种池+方向表如何被回测脚本消费。

本脚本演示两种对接模式：
  1. 静态模式：回测开始前设定 regime，全程固定方向表
  2. 动态模式：回测过程中根据金油比变化动态更新方向表

认识论标注：L1（示例代码，验证接口可用性）。
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import numpy as np

from newchan.strategy.asset_classifier import DirectionPolicy
from newchan.strategy.omega_regime import (
    OmegaRegime,
    OmegaState,
    compute_omega,
    detect_regime,
)
from newchan.strategy.selection_pool import (
    SelectionPool,
    build_selection_pool,
    pool_to_backtest_config,
)
from newchan.topology.config_space import ConfigurationSpace, polarity_index


# ════════════════════════════════════════════════════════════
# 模式一：静态对接
# ════════════════════════════════════════════════════════════


def demo_static_integration() -> None:
    """静态模式：回测前设定 regime，全程固定。

    适用场景：对单一 regime 区间做回测对比（如 2024.9-2025.2 vs 2025.2-至今）。
    """
    print("═" * 60)
    print("  模式一：静态对接")
    print("═" * 60)

    symbols = ("QQQ", "OKLO", "NYMEX:CL1!", "GLD", "MU")
    regime = OmegaRegime.BULL_EQUITY
    k4_polarity = 2

    pool = build_selection_pool(symbols, regime, k4_polarity)
    config = pool_to_backtest_config(pool)

    print(f"\n  regime: {regime.value}")
    print(f"  K4 polarity: {k4_polarity}")
    print(f"\n  品种池 ({len(pool.entries)} 个标的):")

    for e in pool.entries:
        short_flag = "可做空" if config[e.symbol] else "仅做多"
        print(f"    {e.symbol:15s} [{short_flag}]  置信: {e.confidence.value}")

    # ── 模拟回测循环 ──
    print("\n  模拟回测:")
    for i, symbol in enumerate(pool.symbols()):
        short_enabled = config[symbol]
        if short_enabled:
            print(f"    bar {i}: {symbol} — 检查买卖点（多空）")
        else:
            print(f"    bar {i}: {symbol} — 检查买卖点（仅做多，跳过卖点做空）")


# ════════════════════════════════════════════════════════════
# 模式二：动态对接
# ════════════════════════════════════════════════════════════


@dataclass
class RegimeTracker:
    """regime 跟踪器 — 仅在 regime 变化时触发品种池更新。"""

    omega_buffer: list[float]
    current_regime: OmegaRegime
    current_pool: SelectionPool | None
    update_count: int

    @staticmethod
    def create(initial_regime: OmegaRegime = OmegaRegime.NEUTRAL) -> RegimeTracker:
        return RegimeTracker(
            omega_buffer=[],
            current_regime=initial_regime,
            current_pool=None,
            update_count=0,
        )

    def feed_omega(self, omega: float) -> bool:
        """喂入新的 omega 值，返回 regime 是否变化。"""
        self.omega_buffer.append(omega)
        if len(self.omega_buffer) < 60:
            return False

        state = detect_regime(np.array(self.omega_buffer))
        if state.regime is not self.current_regime:
            old = self.current_regime
            self.current_regime = state.regime
            self.update_count += 1
            print(f"    regime 变化: {old.value} → {state.regime.value} "
                  f"(strength={state.strength:.4f})")
            return True
        return False

    def refresh_pool(
        self,
        symbols: tuple[str, ...],
        k4_polarity: int,
    ) -> SelectionPool:
        """regime 变化时刷新品种池。"""
        self.current_pool = build_selection_pool(
            symbols, self.current_regime, k4_polarity,
        )
        return self.current_pool


def demo_dynamic_integration() -> None:
    """动态模式：回测中根据金油比变化动态更新方向表。

    适用场景：长周期回测（跨越多个 regime 切换）。
    """
    print("\n" + "═" * 60)
    print("  模式二：动态对接")
    print("═" * 60)

    symbols = ("QQQ", "OKLO", "NYMEX:CL1!")
    k4_polarity = 1

    tracker = RegimeTracker.create()

    np.random.seed(123)
    gold_base = np.concatenate([
        np.linspace(2400, 2800, 150),
        np.linspace(2800, 2500, 100),
        np.linspace(2500, 2900, 100),
    ])
    oil_base = np.concatenate([
        np.linspace(80, 65, 150),
        np.linspace(65, 80, 100),
        np.linspace(80, 70, 100),
    ])
    noise = np.random.normal(0, 0.5, len(gold_base))

    regime_history: list[str] = []

    print(f"\n  模拟 {len(gold_base)} 根 bar 的回测:")

    for i in range(len(gold_base)):
        omega = compute_omega(gold_base[i] + noise[i], oil_base[i] + noise[i] * 0.3)
        changed = tracker.feed_omega(omega)

        if changed or tracker.current_pool is None:
            pool = tracker.refresh_pool(symbols, k4_polarity)
            config = pool_to_backtest_config(pool)
            labels = ", ".join(
                f"{s}:{'空' if v else '多'}" for s, v in config.items()
            )
            print(f"    bar {i}: 品种池更新 → {labels}")

        regime_history.append(tracker.current_regime.value)

    print(f"\n  regime 切换次数: {tracker.update_count}")
    print(f"  最终 regime: {tracker.current_regime.value}")

    if tracker.current_pool:
        print(f"\n  最终品种池:")
        for e in tracker.current_pool.entries:
            print(f"    {e.symbol:15s} dir={e.direction.value:10s} "
                  f"conf={e.confidence.value}")


# ════════════════════════════════════════════════════════════
# 模式三：与帕萨卡利亚品种池串联
# ════════════════════════════════════════════════════════════


def demo_passacaglia_integration() -> None:
    """串联模式：M2 方向过滤 → 帕萨卡利亚精选。

    M2 selection_pool 提供宏观方向+置信度。
    scanner_pool（帕萨卡利亚）提供折叠等价类排序。
    两层串联：先过滤方向，再精选标的。
    """
    print("\n" + "═" * 60)
    print("  模式三：与帕萨卡利亚品种池串联")
    print("═" * 60)

    from newchan.topology.config_space import Configuration, WalkDirection
    from newchan.trading.scanner_pool import layer1_target_matrix

    config = Configuration(
        sigma_e=WalkDirection.UP,
        sigma_c=WalkDirection.UP,
        sigma_r=WalkDirection.DOWN,
    )
    pol = polarity_index(config)
    target_matrix = layer1_target_matrix(config)

    print(f"\n  K4 配置: {config.label}")
    print(f"  极性指数: {pol}")
    print(f"  目标矩阵: {target_matrix}")

    symbols = ("QQQ", "OKLO", "MU", "NYMEX:CL1!", "GLD")
    pool = build_selection_pool(symbols, OmegaRegime.BULL_EQUITY, pol)

    print(f"\n  M2 方向过滤结果:")
    for e in pool.entries:
        print(f"    {e.symbol:15s} → {e.direction.value:10s} "
              f"conf={e.confidence.value}")

    print(f"\n  高置信标的（进入帕萨卡利亚精选）:")
    from newchan.strategy.selection_pool import SelectionConfidence
    high_conf = [e for e in pool.entries
                 if e.confidence is SelectionConfidence.HIGH]
    for e in high_conf:
        print(f"    {e.symbol}")

    print(f"\n  串联流程:")
    print(f"    1. M2 selection_pool → {len(pool.entries)} 个标的方向表")
    print(f"    2. 过滤 HIGH 置信 → {len(high_conf)} 个标的")
    print(f"    3. 帕萨卡利亚 scanner_pool → 折叠等价类排序 → 代表元")


# ════════════════════════════════════════════════════════════
# 主入口
# ════════════════════════════════════════════════════════════


if __name__ == "__main__":
    demo_static_integration()
    demo_dynamic_integration()
    demo_passacaglia_integration()

    print("\n" + "═" * 60)
    print("  M2 → M1 对接示例完成")
    print("═" * 60)
