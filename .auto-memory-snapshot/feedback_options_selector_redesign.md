---
name: Options selector redesign
description: Strike从缠论结构位来，不从期权数学来。Agent只做翻译不做判断。杠杆是派生变量。
type: feedback
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
options-selector skill 彻底重写（2026-04-21）：

核心原则：三个strike全部落在缠论关键位置上。杠杆是结构派生变量，不是事前设定的目标值。

建仓前四个必答问题：
1. 缠论买/卖点（入场时机）
2. 当前级别止损位 → 卖put/call strike
3. 更大级别支撑/阻力 → 买低put/高call strike
4. 结构立足点 → ITM call/put strike

Agent职责：翻译缠论判断为期权订单参数 + 验证delta合规 + 报告派生杠杆
Agent禁止：自行选strike、按杠杆倒推strike、按delta/IV/凸性选strike

TV MCP辅助：Agent可从TV拉labels/boxes建议四个位置，但最终确认由用户决定。

自动风控：结构清晰→杠杆高→重仓；结构松散→杠杆低→轻仓；结构找不到→不建仓

**Why:** 期权书（Natenberg/McMillan/Bennett）从期权自身数学性质选strike。用户的做法是从市场结构出发，期权数学是派生的。
**How to apply:** 每次建仓先拉TV缠论数据识别结构位，提交用户确认后生成订单
