# tick 数据源落地探针 · research（issue #968，本体直办）

日期：2026-08-15　对象：map #967 雾 1（Databento/Binance tick 历史深度/成本/schema + 坏 tick 清洗）
结论：**数据源现成（同 vendor 换 schema），但历史深度/成本「查不到」（docs 需登录/JS 渲染）；坏 tick 清洗有在案漏洞（#922），tick 落地前必须先修。**

## 一、Binance aggTrade（官方文档逐字核到，GitHub markdown 可 curl）

`binance-spot-api-docs/web-socket-streams.md`「Aggregate Trade Streams」：

```json
{ "e":"aggTrade", "E":1672515782136, "s":"BNBBTC", "a":12345,
  "p":"0.001", "q":"100", "f":100, "l":105, "T":1672515782136, "m":true }
```

- 字段：`p` 价 / `q` 量 / `T` 成交时间（**ms**）/ `a` 聚合成交 ID / `f`/`l` 首尾 trade ID / `m` 买方是否 maker。
- 流名 `<symbol>@aggTrade`，实时；历史走 REST `aggTrades`（分页）+ 数据归档（data.binance.vision）。
- **逐笔语义**：aggTrade = 单个 taker 单聚合的成交，比 raw `trade` 流（每个成交）更粗一档；严格「每一笔成交」用 `trade` 流。

## 二、Databento（三档 tick schema，现役 1min 已在用）

- **三档**：`Trades`（逐笔成交）/ `MBP-1`（L1 盘口 bid/ask）/ `MBO`（全订单簿，逐订单）。现役 8 品种（ES/CL/GC/BRN/DX/QQQ/OKLO）已用 Databento **1min OHLCV**（data.rs:55-62），tick 是同 vendor 换 schema。
- **⚠ 查不到（090 照实）**：Trades/MBP-1 的**精确字段清单**（ts_event ns 精度/price/size/side/rtype）与**历史深度/成本**——docs 是 JS 渲染 SPA，curl 拿不到正文；license 成本需登录 portal。**未逐字核，不作为承重断言**。落地前须人工在 databento.com portal 核这两项。

## 三、坏 tick 清洗现状（在案漏洞，落地前置）

- 现役清洗（`data.rs:22-23`）：缺 OHLC / `high<max(open,close,low)` / `low>min(open,close,high)` / `volume=0` ⇒ `untradable=true`（**不删除**，保序列连续）。
- **#922 OPEN（漏洞）**：BRN 2024-02-19 全日价格≈83 而 low=0.080，**通过全部校验**，当日 σ_GK 被算成 491.6%——清洗逻辑挡不住「单点异常 low」这类坏 tick。
- ⟹ **tick 数据源落地前必须先修 #922**，否则逐笔级坏数据直接污染分型/笔（一个坏 tick = 一个假分型）。

## 四、落地建议（诚实，未裁项单列）

1. 数据源：加密用 Binance `trade`/`aggTrade`（ms 精度，够亚秒排序）；股/期货用 Databento `Trades`（ns 精度）。
2. **前置**：先修 #922（坏 tick 清洗漏洞）。
3. **未裁**：历史深度/成本（Databento 需登录核）；「trade vs aggTrade」取哪档；逐笔时间戳编码（ms vs ns，数据侧归 #971 的亚秒时间戳）。

报告出处：Binance 官方 GitHub docs（curl 可核）；Databento 仅「三档 schema 存在 + 现役 1min 在 data.rs」可核，其余查不到。
