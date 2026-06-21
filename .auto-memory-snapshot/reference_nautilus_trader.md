---
name: nautilus-trader-platform
description: 量化系统研究和生产平台是Nautilus Trader(nt)，流式回测+实盘共用代码
metadata:
  type: reference
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户的量化交易系统运行在 **Nautilus Trader** (简称 nt) 上。这是研究和生产共用的平台。

**Why:** 用户明确说"我们的量化是在nt下做的"、"这是我们现在研究和生产的平台"
**How to apply:** 
- 回测应该在Nautilus Trader框架下以流式（event-driven, bar-by-bar）方式运行，最大模拟实盘
- Rust引擎通过PyO3/FFI接入Nautilus Trader的Strategy框架
- 现有的 t_engine_run.rs 批量回测是简化版，生产版应走Nautilus Trader
- trading_system/ 目录下有数据缓存文件（t_fugue_*.json）已在使用

Related: [[trading-setup]], [[order-pricing]]
