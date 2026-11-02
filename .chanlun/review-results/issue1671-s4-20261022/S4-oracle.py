#!/usr/bin/env python3
"""#1671（#1406 S4 / SPEC A-RF-01-04）独立手算 oracle（不读产品输出，只从 TestOnly 原始输入推导）。

输入（与 rust/src/economic_session/tb03c_rf01_tests.rs 夹具逐字同值）：
- 开局（book::tests::document）：alpha units=4、beta units=15，各 acquisition_cash=80000，无本金批。
- FIN 输入 tb03b-finance/1：baseline 既有责任 20；cap=100；两个 slot 各一申请者。
  alpha：quantity=2、limit_price=14、fee_tail=2、私有现金 40、fixed_q=12。
  beta ：quantity=3、limit_price=12、fee_tail=4、私有现金 50、fixed_q=20。
- alpha 的迟到成交（netted，单参与重 alpha）：0->1 与 1->2，unit_price=13，commission 各 1。
  第一笔在 alpha 账不可用期间到达 E，第二笔在 alpha 恢复后补送。
"""
import json

baseline, cap = 20, 100
plans = {"alpha": (2, 14, 2, 40, 12, 4), "beta": (3, 12, 4, 50, 20, 15)}
fills = [(0, 1, 13, 1), (1, 2, 13, 1)]
opening_cash = 80000

locks = {}
for name, (q, limit, fee_tail, right, fixed_q, units) in plans.items():
    lock = q * limit + fee_tail                # 完整保守界：数量×限价+费用尾
    assert lock <= right and units + q <= fixed_q
    locks[name] = lock
total_after_alpha = baseline + locks["alpha"]
total = total_after_alpha + locks["beta"]      # E 公共责任；甲恢复与迟到成交不降（释放归 TB-04-F）
assert total <= cap

allocations = []
for frm, to, price, commission in fills:       # 单参与重 alpha 独得增量与佣金，按原价
    allocations.append({"units": to - frm, "acquisition_cash": (to - frm) * price, "commission": commission})
q_alpha, limit_alpha, fee_alpha = plans["alpha"][:3]
assert sum(a["units"] for a in allocations) == q_alpha
assert all(p <= limit_alpha for _, _, p, _ in fills)
assert sum(a["commission"] for a in allocations) <= fee_alpha

k_before = opening_cash                        # tb03c-book/1：K = Σacq + Σcommission
k_after = k_before + sum(a["acquisition_cash"] + a["commission"] for a in allocations)
units_after = plans["alpha"][5] + sum(a["units"] for a in allocations)

print(json.dumps({
    "locks": locks, "responsibility_total_after_alpha": total_after_alpha,
    "responsibility_total": total, "allocations": allocations,
    "K_before": k_before, "K_after": k_after, "units_after": units_after,
    "cost_remaining": k_after,
}, sort_keys=True))
