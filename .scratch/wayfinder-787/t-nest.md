Part of #787

## Question

**区间套的教义正本**——第二批概念票第 6 张，**排最后**。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)。

排最后的理由（[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 原话）：分歧最深，但**每一条都以中枢 / 背驰 / 买卖点已定为前提**，前面不定这张只能空转。[#789](https://github.com/xy7365527-lang/NewChanlun/issues/789) 判它是七概念里**最烂的一块**（4 坐标系 + 4 停止级 + `Cand^δ_ℓ` 源码自陈无定义 + **零对拍**）。

### 要裁的 6 组（N-1…N-6）

| 组 | 问题 |
|---|---|
| **N-1** | 坐标系是什么——时间（`classifier/nest.rs:71`）↔ `source_index`（`cand_sub.rs:38`）↔ bar 区间（`a_nested_divergence.py`）↔ **根本不是区间**（`Divergence.lean:172` `NestNecessary` 是纯蕴含）。 |
| **N-2** | 下降到哪一级停——停执行级 e 且 partial chain 合法（注释明写「设计选择」）↔ 末级必须恰为 ev ↔ 必须降到 a0 ↔ 降到 level 1 停。**四种。** |
| **N-3** | 「区间套」这个词指什么——纵向级别下钻 ↔ `nesting_operator.py:20` 的 `NestingType{HORIZONTAL,VERTICAL}`，**横向＝搜索空间收缩（配置→角→板块→标的）**，与纵向并列为同一算子 N 的两个实例，且**现役**（三条 pipeline 都在消费）。**本票须正面回答：区间套正本是不是算子 N。** |
| **N-4** | 装配方向与递归骨架——top-down ↔ bottom-up（标为对照基线）；Lean 侧 `NestingCertificate.lean`（Nat 级别差递归）↔ `IntervalNestCertificate.lean`（列表链递归），**两文件头互相声明「刻意不合并」，无等价证明**。 |
| **N-5** | `Cand^δ_ℓ` 的定义式是什么——**无定义式**（`nest.rs:155` 标 [需人工确认]，「spec 中仅作符号出现」）↔ 四条件合取（`cand_predicate.rs:107`）。**教义空洞。** |
| **N-6** | 两条生产 `N^δ` 严格度不同——`classifier/nest.rs` `n_delta`（宽，回测准入门用）↔ `strategy/nest.rs` `chi_bool`（严，环 2 用）。**两条通则都判不了**（不接在同一 if-else、不按级别分叉），须本票裁。 |

### 本票另须回答的两件（上游明记归此）

1. **「小转大与区间套下钻是不是同一件事」**——[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 未裁并明记归本票。教义侧第 43 课「c 中出现 1 分钟背驰」+ 第 53 课「二类点构成含次级别一类点」已在案（见 [#166](https://github.com/xy7365527-lang/NewChanlun/issues/166)）。
2. **π 门 rung 链 `Type2|Type3 => true` 的性质**——[#809](https://github.com/xy7365527-lang/NewChanlun/issues/809) 判为违规 V2 但**性质判不死**：限定词①（不同判断）的辩护技术上站得住，但代码里没写成举证书，且 `Cand^δ` 自陈无定义式。本票须给它定性，并与 N-5 一起解。**在案实测**：[#802](https://github.com/xy7365527-lang/NewChanlun/issues/802) 查出 BTC 300K 中 **95.36% 通过门信号 rungs 空**、仅 4.64% 真跨级且最大深度 = 1。

### 关票判据

正本落 `.chanlun/definitions/qujiantao.md` + 受影响代码清单。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
