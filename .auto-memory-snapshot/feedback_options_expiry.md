---
name: Options expiry date rule
description: 到期日 = T_target上限 + 45天theta buffer，禁止贴着事件/目标日期，不用凸性反推到期日
type: feedback
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
到期日计算规则（最终版）：

核心原则：用户只在缠论买卖点处置仓位。到期日的功能是为"等待卖点出现"提供足够的时间空间，不是为预测走势完成时间。

计算公式：到期日 = T_target_上限 + T_theta_buffer

输入要求：
1. T_target_上限：用户给出的"走势最晚完成时间"区间上限（用户必须提供，不可由 agent 推断）
2. T_theta_buffer：30-60 天，默认 45 天

事件驱动额外约束：到期日 ≥ 事件完成日 + 45 天，与 T_target_上限 + buffer 取较大者

建仓前必须与用户确认：驱动类型（纯缠论/纯事件/叠加）、T_target_上限的具体数值

禁止：到期日贴着事件完成日、到期日贴着 T_target_上限、用凸性最大化反推到期日

已明确接受的代价：premium 成本比最凸结构高 2-4 倍、杠杆相应降低、事件驱动行情中可能让 option 归零以等待缠论卖点。这些代价已由用户明确选择，agent 不需要优化掉。

**Why:** 之前多次因为到期日太近导致期权流动性问题（SI C79 七天到期没 bid）和时间压力
**How to apply:** 每次推荐期权前必须先问用户 T_target_上限，然后加 45 天 buffer
