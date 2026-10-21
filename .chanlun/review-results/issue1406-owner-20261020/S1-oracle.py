#!/usr/bin/env python3
"""#1406 S1 独立手算 oracle（不读产品输出，只从 TestOnly 原始输入推导）。

输入（与 rust/src/economic_session/tb03c_chain_tests.rs 夹具逐字同值）：
- alpha 开局：observed units=4，acquisition_cash=80000（book::tests::document），无本金批。
- FIN 输入 tb03b-finance/1：baseline 既有责任 20；alpha 计划 quantity=2、limit_price=14、fee_tail=2；
  私有现金权利 40；cap=100；fixed_q=12。
- Commit 后的成交（netted，单参与重 alpha）：from=0 to=2，unit_price=13，commission=2。
- 估值观察：reference_price 14 -> 9（零现金分量）。
"""
import json

opening_units, opening_cash = 4, 80000
baseline = 20
q, limit, fee_tail, right_cash, cap, fixed_q = 2, 14, 2, 40, 100, 12
fill_from, fill_to, price, commission = 0, 2, 13, 2

lock = q * limit + fee_tail                    # 完整保守界：数量×限价+费用尾
assert lock <= right_cash
total = baseline + lock                        # E 公共责任
assert total <= cap                            # 全部能装下 -> Reserved
worst = opening_units + q
assert worst <= fixed_q
fill_units = fill_to - fill_from
assert fill_units == q and price <= limit and commission <= fee_tail
alloc_cash = fill_units * price                # 单参与重独得全部增量与佣金
k_before = opening_cash                        # tb03c-book/1：K = Σacq + Σcommission
k_after = k_before + alloc_cash + commission
units_after = opening_units + fill_units
cost_remaining = k_after                        # 多头 cost_remaining = K（ADR 0036 裁定五）
k_after_valuation = k_after                    # 估值只记 E，B 拒收，不改成本

print(json.dumps({
    "lock": lock, "responsibility_total": total, "worst_quantity": worst,
    "allocation": {"units": fill_units, "acquisition_cash": alloc_cash, "commission": commission},
    "K_before": k_before, "K_after": k_after, "units_after": units_after,
    "K_after_valuation": k_after_valuation, "cost_remaining": cost_remaining,
}, sort_keys=True))
