---
name: Order pricing rule
description: Always LMT at mid/bid (buy) or mid/ask (sell), never MKT orders
type: feedback
---

买单：用MID或BID价格。卖单：用MID或ASK价格。绝对禁止MKT单。

**Why:** 用MKT单平仓原油combo仓位时滑点损失严重，期权价差很宽。
**How to apply:** TWS脚本下单前拉bid/ask/mid，只用LMT。未成交由用户手动调整，不自动升级为MKT。
