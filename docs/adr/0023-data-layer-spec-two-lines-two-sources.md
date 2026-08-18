# ADR 0023：数据层规格正本——两线两源（个股 Massive / K4 Databento）

- 状态：Accepted（2026-08-18，图 [#1029](https://github.com/xy7365527-lang/NewChanlun/issues/1029) D1-D4 全裁闭环）
- 裁定链：[D1 #1030](https://github.com/xy7365527-lang/NewChanlun/issues/1030)（订阅+历史起点）· [D2 #1031](https://github.com/xy7365527-lang/NewChanlun/issues/1031)（回测粒度）· [D3 #1032](https://github.com/xy7365527-lang/NewChanlun/issues/1032)（K4 线）· [D4 #1033](https://github.com/xy7365527-lang/NewChanlun/issues/1033)（存储+实时+预算）
- 关联：[ADR 0020](0020-backtest-cash-live-perp-filtered.md)（决策源统一美股）、#967（tick 观察臂，名分已由 D2 作废并扶正）、#973（tick loader seam，生产形态）

## 一、两线两源（D1+D3）

| 线 | 源 | 历史起点 | 粒度 |
|---|---|---|---|
| **个股线**（缠论信号，十标） | **Massive** Stocks（Advanced 档） | **2003-09-10**（晚于 2003 上市的从上市日起） | tick（trades） |
| **K4 线**（M/P/C/R 六边） | **Databento** tick 档 | CME **2010-06-06** / ICE **2018-12-23** / Eurex **2025-03-10**（各自地板） | tick，消费层照 K4 固有节奏聚合（日级 regime、顶点 1m/1h）——数据粒度与消费聚合是两回事，不构成宽严两档 |

- Massive 期货统一案否（D3）：真实起点 2017-04-03、无 ICE/Eurex ⟹ 不成立（#1037）；
- EQUS.MINI 17.55GB（2023-03 起）已拉取完毕：定位 = **对拍/校验源**，不作生产供给（Massive 2003 全史覆盖同段且口径为主源）；
- 中国品种（AU9999/SC/沪深300）仍是独立缺口（#1029 雾区移交，另图评估）。

## 二、回测引擎粒度（D2）

**tick 直算为生产正典**：逐笔成交 → `Bar(O=H=L=C=成交价，亚秒时间戳)` → 现役判定链（零改，粒度无关 #969 已证），#973 seam 即生产形态。

- #967「tick 级观察臂（与现役 1min 并存）」名分**作废**——tick 链由观察臂**扶正**；个股回测线上 1m 管线不再并存；
- 期货/K4 线不因此退役（D3 边界）；
- 生产切换前置 = #973 重标定门槛（`new_stroke_min_gap` 扫参）随正典上位；转生产走 #847 系（#960 换装）。

## 三、存储与实时（D4）

**存储（两线统一）**：`Parquet(ZSTD) + 全字段保留 + per-symbol/per-day 分区`。JSON/CSV 为传输形态不入正本；复权另做不改原始。K4 侧 Databento SDK `encoding=PARQUET` 直出。

**实时流接入（四条）**：
1. schema 基 = Massive REST 13 字段；WS 短码映射入同一 schema（`sym→ticker`、`t/pt/trft` 毫秒×1e6 归一纳秒、WS 无的 `correction` 置 null）；
2. 时间精度口径写死：历史段 ns 全精度、实时段 ms 精度；
3. 实时落盘：当日 per-symbol 文件边写边 flush、日终转正；
4. 断线重连后按 `sip_timestamp` 区间从 REST 回补对账。

**预算口径**：
- 个股线 **三判据口径落盘**（per-venue 全量拉取 → 条件码黑名单+丢 TRF+消歧键 sip_seq，前移到落盘层）≈ **250-380GB**（#1064 拍 a 订正：官方 consolidated 语义不可复现——#1069 对拍 precision 0.11-0.21，第四判据不上）；
- K4 期货 tick 保留（ES 16 年 ≈ 0.5-1TB），磁盘编排者扩容，不设 ×3 硬性；
- 引擎 O(1)/bar 流式，全史不驻内存。

## 四、登记给实施（图外 #847 系）

1. 去重规则具体口径（主所判定/条件码清单）在实施票定稿——**去重是引擎前置，不是省盘优化**（per-venue 重复不先去会虚增笔的 K 线根数，tick→Bar 直算吃不了原样数据）；
2. 历史 quotes 全史不默认回补；实时 quotes 流按上节落盘；
3. Massive 全史拉取管线（十标 2003→今，分页遍历 + Parquet 落盘）待实施票开工。

## 代价照实

- 两源并存 = 两套凭证/账单/管线维护面（Massive $199/月 + Databento 按量）；
- K4 Eurex tick 仅 2025-03 起（FESX 跨国管线历史受限，按现状接受）；
- 实时段 ms 精度与历史段 ns 的精度差是**口径写死的妥协**（WS 不给 ns），对账以 `sip_timestamp` 区间锚定。
