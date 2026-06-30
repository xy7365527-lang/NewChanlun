# Codex 异质审计：664 Ab_rev 反转交易腿对象正确性（#97）

模式=review，read-only。codex gpt-5.5 xhigh，146K tokens。raw=/tmp/abrev_audit_out.txt(8187行)。
（Lead 从 raw 尾部提取 codex 四问判定落盘——agent 落盘前 durable 保存。）

## codex 四问判定

**Q1 ✓ Ab_rev 对象正确，非隐藏同义反复。**
Ab_rev=δ(Pρ−Pλ) 非 |Pρ−Pλ|（δ 来自 BSP buy/sell bits→VoiceSide，非 Pρ−Pλ 回推）。
level=0,δ=+1 桶 ΣAb_rev<0 是强证据——若等价 eps=sign 则每项 Ab≥0，桶总和不可能负。排除绝对值伪装修复。
待补不变量: rho_rev_bar>lambda_rev_bar、lambda_rev_bar≤entry_bar、rho_rev_bar≤exit_bar。

**Q2 ✓ 但边界: λ/ρ 配对=π^bsp-only 信号腿，非完整 runner 真实持仓腿。**
λ_rev=入场BSP source_index, ρ_rev=首个后续反向BSP source_index=两次相反买卖点pivot间反转腿（非旧触发段）。
pivot价差(Ab_rev测理想反转空间) vs 确认bar执行价(ηin/ηout测滞后)分离=正确建模。
但未模拟账户状态/冲突序/止损/RiskClose → 不严格等于真实成交持仓腿。eb>entry_bar排除同bar反向(建议报same_bar_opposite数)。

**Q3 ⚠ 关键口径偏差: Σcaptured 是 adverse-only 保守口径，不等于实际 PnL。**
captured = Ab_rev − max(x,0) − max(y,0) − Ce, 其中 x=δ(Pτin−Pλ), y=δ(Pρ−Pτout)。
实际无成本价差 = δ(Pτout−Pτin) = Ab_rev − x − y。
差值 captured_impl − actual_pnl = min(x,0)+min(y,0) ≤ 0 → **有利滑移被丢弃，不利滑移全计，captured 系统性低估实际 PnL**。
⟹ "执行滞后吃光"(Σcaptured=−2.33e5)是 adverse-only 压力测试，**非真实成交 PnL**。
支撑"执行吃光"需同时输出 signed Σx/Σy 与 adverse-only Σx+/Σy+。

**Q4 ✓ 但右删失: 664 解决了 Ab 对象错配；完整性边界。**
无配对出场 continue 正确(不兜底末bar污染)，但右删失丢窗口末端未平仓腿，需报 n_unpaired 按δ/level分布 + 末端MtM敏感性。

## 总判定
Ab_rev 修复方向正确(旧触发段→post-signal反向BSP pivot配对反转腿，per-class负值排除绝对值同义反复)。
但"完整正确解决对象错配"仅限 paired BSP signal-leg 的 Ab_rev。captured 是 adverse-only 保守分解非实际成交PnL。
需补三审计统计: rho_rev_bar>lambda_rev_bar / same_bar_opposite / n_unpaired删失分布。

## 结果包(简化版)
- **结论**: Ab_rev对象正确(Q1✓非同义反复); captured口径=adverse-only保守(Q3⚠),"执行吃光"结论需signed分解支撑,不可声明真实PnL。
- **边界**: captured系统性偏负(min(x,0)+min(y,0)≤0); π^bsp信号腿≠runner真实持仓; 右删失未平仓腿。
- **影响**: econ-abrev-l2报告"执行滞后吃光"需标注adverse-only口径; 补signed Σx/Σy + 三审计统计。
