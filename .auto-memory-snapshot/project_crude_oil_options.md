---
name: Crude oil options strategy
description: S3 three-leg+kicker, 11×C130 existing, COIL/IPE execution rules, file locations
type: project
---

- **品种：** Brent原油COIL，Exchange=IPE，Multiplier=1000，数据源BZ/NYMEX
- **已有持仓（截至2026-04-08）：** 11×C130 Mar2027，账户总值~$118,112，BZ期货2手均价$109
- **推荐方案S3：** 三腿（Buy C80 + Sell P80 + Buy P70）+ 6手C150 kicker，总资本$97,020
- **执行规则：** 只用LMT，下单前拉bid/ask/mid，IPE单腿tif='DAY'，combo可GTC
- **凸性分析（真实TWS报价）：** Dec2026 C160最优(47x)，但现有C130(56x)成本更低不换仓

**Why:** 用户核心持仓，基于美元体系ω崩溃论题
**How to apply:** 交易相关指令优先参考此文件的执行规则
