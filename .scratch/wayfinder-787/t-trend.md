Part of #787

## Question

**走势类型与级别的教义正本**——第二批概念票第 4 张。清点见 [总缝规则过筛 #809](https://github.com/xy7365527-lang/NewChanlun/issues/809)。

**排在中枢票之后**：M-1/M-2 直接消费中枢的成立判据与关系判据，中枢没定这张空转。

### 要裁的 4 组（M-1…M-4）

| 组 | 问题 |
|---|---|
| **M-1** | 「盘整」是什么——恰好 1 个中枢（3 处）↔ 一段 `LevelExpansion` 关系 run、可含任意多中枢（`decompose.rs:3`）↔ 方向断裂即收尾（`recursive_t/trend.rs:47`）。 |
| **M-2** | 「同向」判据——核心分离 `c2.zd>c1.zg`（3 处）↔ 外缘分离 `next.dd>prev.gg`（3 处）↔ 双升双降 `high/low`（`a_trendtype_v0.py:131`）。**三者接受集互不包含。** 第三种的兜底接法已判 V5 违规，作为口径本身仍待裁。 |
| **M-3** | 「级别」是递归级别还是数据涌现的簇——`level_id = 下级+1`（3 处）↔ persistence log-gap 递归分裂簇（`a_level_detection.py:282`，经 `CalendarPeriodNamer:77` 映射到「30 分钟/日线」）。**直接命中图 [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787) 待裁的「统一递归算子 T / 每级别一个 T 实例」，也是记忆里那条「regime＝级别截断伪影」的落点。** |
| **M-4** | 「小转大」的教义位置——全仓**只有一处**实现（`a_xiaozhuan_da.py:142`），Rust 三族 + Lean 五文件全无对应；`turn_class.rs` 的 `XiaozhuandaCandidate` 自陈「纯只读派生、永远只是必要条件」。**这是空洞不是分歧**，但同样只能由概念票裁。 |

**⟹ 与区间套票的交叉项**：[#799](https://github.com/xy7365527-lang/NewChanlun/issues/799) 未裁的「小转大与区间套下钻是不是同一件事」（教义侧第 43 课 + 第 53 课已在案，见 [#166](https://github.com/xy7365527-lang/NewChanlun/issues/166)）**归区间套票**，本票只裁 M-4 的名分（是不是一个独立的走势层概念）。

### 关票判据

正本落 `.chanlun/definitions/zoushileixing.md` 与 `.chanlun/definitions/jibie.md` + 受影响代码清单。

### 纪律

`/grill-with-docs`，**一次一问**，agent 不得替人答。落完文档才关票。
