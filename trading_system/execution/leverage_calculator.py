"""LeverageCalculator —— L_max = 1/(D_struct + mm) 动态杠杆上限（设计 §5.3）。

在册框架（analysis/notional_exposure_leverage_research.md，L0）：
    D_struct(t) = (c − P_neg)/c + θ_q·(j−1)
    L_max(t)    = 1 / (D_struct(t) + mm)

    c      : 当前价格
    P_neg  : 否定价位——相位定义（MOVE↑→新中枢 ZG；OSC→中枢 ZD）
    θ_q    : 配额步长（嵌套层级折扣）
    j      : 嵌套深度（骨架恒 1 ⟹ θ_q 项为 0）
    mm     : venue 维持保证金率（阶段3 从 Instrument.margin_maint 读）

纯函数层：无状态，每 bar 输入结构量输出上限，独立可测。
每 bar 影子价格，零自由参数（在册结论）。
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class LeverageResult:
    """单次计算结果（诊断可回溯）。"""

    d_struct: float
    l_max: float
    max_notional: float
    max_quantity: float


class LeverageCalculator:
    """L_max 计算器。一个实例绑定一个标的（mm 不同）。"""

    def __init__(self, maint_margin_rate: float, theta_q: float = 0.0) -> None:
        if not 0.0 <= maint_margin_rate < 1.0:
            raise ValueError(f"非法维持保证金率: {maint_margin_rate}")
        self._mm = maint_margin_rate
        self._theta_q = theta_q

    def compute(
        self,
        price: float,
        zd: float,
        zg: float,
        nesting_depth: int = 1,
        phase_is_up_move: bool = False,
    ) -> LeverageResult:
        """逐 bar 计算 L_max。

        P_neg 选择（在册相位定义）：
            MOVE↑ 相位 → P_neg = 新中枢 ZG（phase_is_up_move=True）
            OSC   相位 → P_neg = 中枢 ZD（默认）
        TODO(阶段3): 相位由引擎 PhaseView 三值接口读出，替换 phase_is_up_move 参数。
        """
        if price <= 0:
            raise ValueError(f"非法价格: {price}")
        p_neg = zg if phase_is_up_move else zd
        # 结构止损距离比例；价格已破 P_neg 时距离为 0 ⟹ 仅 mm 项兜底
        d_struct = max(0.0, (price - p_neg) / price) + self._theta_q * (nesting_depth - 1)
        l_max = 1.0 / (d_struct + self._mm)
        return LeverageResult(
            d_struct=d_struct,
            l_max=l_max,
            max_notional=0.0,  # 由 max_quantity 填充（需 equity）
            max_quantity=0.0,
        )

    def max_quantity(
        self,
        equity: float,
        price: float,
        zd: float,
        zg: float,
        notional_frac: float = 1.0,
        current_exposure: float = 0.0,
        nesting_depth: int = 1,
        phase_is_up_move: bool = False,
        contract_multiplier: float = 1.0,
    ) -> float:
        """意图量钳制（设计 §5.3）：

        每手名义 = price × contract_multiplier（期货乘数，如 CL/BZ=1000桶/手；
        现货/perp 张=1）。
        qty = min(notional_frac × equity, L_max × equity − current_exposure) / 每手名义

        返回最大可下手数（float；调用方按 instrument size_precision 取整）。
        """
        r = self.compute(price, zd, zg, nesting_depth, phase_is_up_move)
        unit_notional = price * contract_multiplier
        intent_qty = notional_frac * equity / unit_notional
        cap_qty = (r.l_max * equity - current_exposure) / unit_notional
        return max(0.0, min(intent_qty, cap_qty))
