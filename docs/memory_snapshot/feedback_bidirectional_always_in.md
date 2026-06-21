---
name: bidirectional-always-in
description: 缠论不是只做多系统。无论牛熊震荡都应赚钱，绩效=涨跌幅绝对值之和，永远在市场里有方向
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-12明确纠正："你让我的缠论操盘变成了一个只做多的，这样不行。无论牛熊还是震荡，都应该是赚的。并且以涨跌幅的绝对值为准。"

**Why:** 当前hold26/fusion_tr全部long-only是实现缺口。缠论走势完全分类+六种买卖点=任何走势都有操作方向。卖点=反手做空入场，买点=反手做多入场。系统永远在市场里。BTC两周跌13.57%，正确系统应赚~13.57%而不是"少亏5.5%"。

**How to apply:**
- 绩效benchmark从"跑赢BH"改为"吃到Σ|各段涨跌幅|"
- master FSM从 LONG→FLAT→LONG 改为 LONG↔SHORT（无FLAT状态）
- 嵌套递归赋格中每层voice跟随自己级别走势方向
- 所有回测报告加双向口径
- 平仓≠卖空——平仓退出市场，卖空反向进入市场

Related: [[recursive-nested-fugue]], [[no-asset-specialization]], [[session3-fusion]]
