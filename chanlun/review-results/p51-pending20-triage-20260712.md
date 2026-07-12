# P51：P46/P47 终点 Pending 20 对象分类分流——P0 SEPARATE-RULING 材料（2026-07-12）

- 任务：#51，只读研究；本文是裁决材料，不是已生效裁决。
- 当前分支 / HEAD：`p0-replay-dparent` / `fca8aefc81`。
- 数据：BTC 1m 4,613,599 bar，`ThetaConfig::default()`。
- 直接材料：`p46-third-closure-zero-20260712.md` §2、`p47-cp-lifecycle-fix-20260712.md` 终态对象、`p0-cand-delta-naming-dualtime-ruling-20260712.md` R4。
- 一级权威：`docs/chanlun/text/blog/037-第37课.md` 第 16/18/20/22 行。
- P0 先例：#28 严格区间套 adopted-default、#37 `J_parent/D_parent` 改判、#43 完整 `c_p` 左端裁决、#48 材料及其后 #49 adopted-default。
- 禁区：未修改代码真值、`judge_third_cert`、任何 `Pending/None`、strict chain 或交易消费；只运行既有只读诊断二进制并新增本报告。
- 并发边界：报告落盘期间工作树另出现一条未提交 R3 实装工作线（复核时可见
  `classifier/mod.rs`、`classifier/recursive_tower.rs`、`classifier/nest.rs`、
  `backtest/runner.rs`）；它们不是 P51 产出，本报告未编译、未消费、未修改、未回滚这些并发
  改动。P51 数据复现使用落盘前既有 release 二进制与已合入 #47 终态对象。

## 0. 结论

这 20 个对象不能在 (a)–(d) 中做单标签四选一。四项分属不同轴，严格答案是：

1. **(a) 成立，但只能采用 as-of 负资格口径**：20/20 截至数据终点均不满足
   `ThirdClassInsideC`，所以必然不满足 R3 的 `FullTrendCQualified`；可写
   `NotFullTrendQualifiedAsOf(T_end)`，不可写“永久不是趋势 `c`”或“已完成的非趋势对象”。
2. **(b) 未证**：第 37 课允许把缺 third 的 `c` 看作围绕 `B` 的小级别波动并用盘整背驰处理，
   但 `third=None` 不是盘整背驰证书。当前 20 例的合法盘整背驰数为**未知**，不是 20。
3. **(c) 不支持作为 third 缺失主因**：P47 后 20/20 都有稳定
   `(level, B_p, departure)` 归属边、`c_start_full` 和可计算的终点几何失败；终点没有一例停在
   “缺对象/缺映射”。但第 20/22 行完整趋势资格与独立盘整背驰证书仍有**能力覆盖缺口**；这与
   “20 个 third 因映射不足而漏判”不是同一命题。
4. **(d) 只能作为删失标记，不能作为已证原因**：20 个对象在有限数据右端仍是开放态，数据结束
   不是对象否定对象，故都应标 `RightCensoredAt(T_end)`。但从各事件到终点仍有
   349,094..4,480,829 bar（中位数 2,607,822.5），且终点已有 11 个方向不配、9 个严格重入的
   具体失败事实，所以不能把 20 例笼统解释为“只是离文件末尾太近”。

**推荐排序：`(a-as-of) > (d-删失修饰) > (b-独立复核候选) > (c-third 映射主因)`。**

当前最严格状态应写成四列，而不是压成一类：

```text
CandidateGenerated = true
ThirdCertificateLifecycle = Pending
FullTrendQualification = NotQualifiedAsOf(T_end)
ClassificationReview = ConsolidationReviewPending
Censoring = RightCensoredAt(T_end)
```

★ Insight ─────────────────────────────────────

- 确认时的 12 个“缺相邻 pair”与 7 个“缺 anchor”不是终点主因；到终点它们都已推进为可观察的
  方向/重入失败。这是 P47 生命周期修复后才能得到的对象级区分。
- “未取得趋势资格”“盘整背驰成立”“对象映射缺失”“数据右删失”分别是资格、分类、能力、观察窗
  四个命题。把它们互相替代会重复 #37/#43 已经否定的“同值即同义”错误。
- 第 37 课的“可以看成”授权的是处理/复核方向，不是 `third=None => consolidation_divergence` 的
  充分条件。

─────────────────────────────────────────────────

## 1. 口径与方法

### 1.1 对象身份

P47 后完整 `c_p` 生命周期对象没有独立 ordinal；其稳定身份是：

```text
CpId(p) := (event_level, B_p ElementId, cp_departure_move_id)
```

稳定边另带 `cp_source_start = c_start_full`。本文不使用 `c_episode_start`、事件确认点或随后事件
端点替代 `c_p` 身份/边界。

### 1.2 两个时点、两个状态栏

- 确认时失败：复用 P46 §2.1 的事件快照最小失败原子。
- 终点失败：复用 P46 §2.2 的同一生产判据终点回看，并以 P47 终态
  `CpScanOwnership.lifecycle` 验证 `Pending/Closed`。
- 分类复核：按 R4 另列 `ConsolidationReviewPending`；它不是生命周期 enum，也不是盘整背驰真值。

失败码：

| 码 | P46 原子 | 精确含义 |
|---|---|---|
| `PAIR` | `NO_B_OWNED_ADJACENT_PAIR` | 尚无可归属于该 `B_p` 的完整相邻 leave/retest 对；现有材料不能诚实再拆成“只缺 leave”或“只缺 retest”。 |
| `ANCHOR` | `LEAVE_ANCHOR_NONE` | 有 pair，但 leave 没有 686 允许的趋势 ownership anchor；禁止 fallback。 |
| `DIR` | `DIRECTION_PAIR_MISMATCH` | 已有合法 anchor/pair，但 leave 与 retest 的方向关系不构成第三类。 |
| `REENTER` | `RETEST_REENTERS_B` | retest 触及或重入 `B_p` 闭区间；严格不重入门失败。 |

### 1.3 只读复现

```text
CP_SMOKE_MAX_BARS=999999999 CP_SMOKE_MODE=batch CP_SMOKE_VERBOSE=1 \
  ./rust/target/release/cp_capability_smoke
```

当前 HEAD 复现：

```text
P46_L1_L4 objects=31 snapshot_closed=0 terminal_closed=11 terminal_pending=20
L1=7/0/1/6  L2=13/0/6/7  L3=9/0/3/6  L4=2/0/1/1
```

31 个事件与 31 个唯一对象一一对应；全级别 483 个事件均有稳定边。本报告从 31 个唯一对象中
排除 11 个 `ThirdCertificateClosed`，余下恰为下表 20 个。

为隔离并发工作树，本文没有在上述未提交源码出现后重新编译二进制；因此本报告只裁 P47
终态 20 对象，不对并发 R3 实装的正确性、产量或验收状态作任何声明。

## 2. 20 个对象逐例登记

表中 `B` 依次列 ElementId / source 区间 / core；`C` 是稳定 `CpId` 与
`c_start_full`；`dep` 列 departure ID / source 区间 / 方向 / 合法 anchor。所有行终态共同为：

```text
terminal_cp_confirm=None; terminal_end=None;
CpScanOwnership.lifecycle=Pending;
classification_review=ConsolidationReviewPending
```

| # | L / `divergence_confirm_src` | `B_p` 身份：ID / source / core | `c_p` 稳定身份 / `c_start_full` | departure：ID / source / dir / anchor | 确认时失败 | 终点失败 |
|---:|---|---|---|---|---|---|
| 1 | L1 / 345518 | `L2#148 / [340499,344222] / [782200000000,801223000000]` | `(L1,L2#148,L1#704) / 344233` | `L1#704 / [344233,344757] / Up / None` | `ANCHOR` | `DIR` |
| 2 | L1 / 759272 | `L2#360 / [757550,758668] / [341604000000,343350000000]` | `(L1,L2#360,L1#1625) / 758713` | `L1#1625 / [758713,759272] / Down / Down` | `PAIR` | `REENTER` |
| 3 | L1 / 1044073 | `L2#495 / [1042134,1043782] / [1011000000000,1028394000000]` | `(L1,L2#495,L1#2194) / 1043884` | `L1#2194 / [1043884,1044073] / Down / Down` | `PAIR` | `REENTER` |
| 4 | L1 / 2168349 | `L2#988 / [2160411,2165116] / [4746569000000,4790000000000]` | `(L1,L2#988,L1#4469) / 2165236` | `L1#4469 / [2165236,2166104] / Up / None` | `ANCHOR` | `DIR` |
| 5 | L1 / 2365194 | `L2#1066 / [2361351,2364547] / [4030000000000,4079900000000]` | `(L1,L2#1066,L1#4843) / 2364887` | `L1#4843 / [2364887,2365194] / Down / Down` | `PAIR` | `DIR` |
| 6 | L1 / 4264505 | `L2#1931 / [4261326,4264053] / [11344974000000,11420796000000]` | `(L1,L2#1931,L1#8591) / 4264209` | `L1#8591 / [4264209,4264505] / Up / Up` | `PAIR` | `DIR` |
| 7 | L2 / 132770 | `L3#8 / [109807,122447] / [705000000000,730000000000]` | `(L2,L3#8,L2#50) / 122472` | `L2#50 / [122472,124559] / Down / None` | `ANCHOR` | `DIR` |
| 8 | L2 / 174527 | `L3#12 / [160931,172156] / [1421286000000,1626930000000]` | `(L2,L3#12,L2#69) / 172192` | `L2#69 / [172192,174527] / Up / Up` | `PAIR` | `DIR` |
| 9 | L2 / 658767 | `L3#67 / [650224,656902] / [558500000000,566891000000]` | `(L2,L3#67,L2#309) / 657601` | `L2#309 / [657601,658767] / Down / Down` | `PAIR` | `DIR` |
| 10 | L2 / 1843204 | `L3#178 / [1823829,1835970] / [4627969000000,4840934000000]` | `(L2,L3#178,L2#860) / 1835988` | `L2#860 / [1835988,1838729] / Up / None` | `ANCHOR` | `REENTER` |
| 11 | L2 / 2197213 | `L3#207 / [2180264,2187028] / [5984445000000,6250000000000]` | `(L2,L3#207,L2#996) / 2187038` | `L2#996 / [2187038,2188509] / Up / None` | `ANCHOR` | `REENTER` |
| 12 | L2 / 2636582 | `L3#248 / [2624161,2634521] / [2135859000000,2155000000000]` | `(L2,L3#248,L2#1183) / 2635395` | `L2#1183 / [2635395,2636582] / Down / Down` | `PAIR` | `DIR` |
| 13 | L2 / 3436636 | `L3#331 / [3427576,3434763] / [6162500000000,6216779000000]` | `(L2,L3#331,L2#1568) / 3435436` | `L2#1568 / [3435436,3436636] / Up / Up` | `PAIR` | `REENTER` |
| 14 | L3 / 206731 | `L4#1 / [160931,187631] / [1508000000000,1619656000000]` | `(L3,L4#1,L3#15) / 190956` | `L3#15 / [190956,196098] / Down / None` | `ANCHOR` | `REENTER` |
| 15 | L3 / 1036108 | `L4#19 / [964857,1009736] / [1082000000000,1200000000000]` | `(L3,L4#19,L3#104) / 1015889` | `L3#104 / [1015889,1020721] / Down / Down` | `REENTER` | `REENTER` |
| 16 | L3 / 1835970 | `L4#31 / [1770084,1815305] / [3230000000000,3763639000000]` | `(L3,L4#31,L3#178) / 1823829` | `L3#178 / [1823829,1835970] / Up / Up` | `PAIR` | `DIR` |
| 17 | L3 / 2329634 | `L4#37 / [2253253,2299566] / [4806951000000,4859997000000]` | `(L3,L4#37,L3#214) / 2301262` | `L3#214 / [2301262,2307754] / Down / None` | `ANCHOR` | `DIR` |
| 18 | L3 / 2534466 | `L4#41 / [2477437,2520991] / [2908704000000,3077733000000]` | `(L3,L4#41,L3#238) / 2527548` | `L3#238 / [2527548,2534466] / Down / Down` | `PAIR` | `DIR` |
| 19 | L3 / 4001207 | `L4#70 / [3949783,3994672] / [8580000000000,8745367000000]` | `(L3,L4#70,L3#385) / 3996422` | `L3#385 / [3996422,4001207] / Down / Down` | `PAIR` | `REENTER` |
| 20 | L4 / 2826563 | `L5#6 / [2527548,2737472] / [2076300000000,2092773000000]` | `(L4,L5#6,L4#46) / 2741411` | `L4#46 / [2741411,2826563] / Down / Down` | `PAIR` | `REENTER` |

### 2.1 分桶与迁移

| 级别 | Pending | 终点 `DIR` | 终点 `REENTER` |
|---:|---:|---:|---:|
| L1 | 6 | 4 | 2 |
| L2 | 7 | 4 | 3 |
| L3 | 6 | 3 | 3 |
| L4 | 1 | 0 | 1 |
| 合计 | **20** | **11** | **9** |

确认时到终点的失败迁移：

| 确认时 \ 终点 | `DIR` | `REENTER` | 合计 |
|---|---:|---:|---:|
| `PAIR` | 7 | 5 | 12 |
| `ANCHOR` | 4 | 3 | 7 |
| `REENTER` | 0 | 1 | 1 |
| 合计 | **11** | **9** | **20** |

因此，确认时“缺 anchor / 缺 leave-retest 对”是早期窗口事实；P47 推进到数据终点后，20 例已经
全部具有足够对象材料进入 `DIR` 或 `REENTER` 几何门。终点没有 `PAIR`、`ANCHOR` 或未知 ID
残留。

## 3. 第 37 课与 P0 先例

### 3.1 一级权威给出的不是单条件

第 37 课形成连续约束链：

1. 第 16 行：没有趋势就没有“背驰”，只有盘整背驰；趋势的标准形态是 `a+A+b+B+c`。
2. 第 18 行：`c` 至少包含对 `B` 的第三类；否则可看作 `B` 的小级别波动，用盘整背驰处理。
3. 第 20 行：上涨 `c` 必须创新高、下跌 `c` 必须创新低；即使有 third，不创新高/低也仍可按
   围绕 `B` 的次级别震荡处理。
4. 第 22 行：因 `c` 包含 third，`c` 内至少包含两个次级别中枢，并可继续递归套用
   `a+A+b+B+c`。

所以：无 third 足以否定当前完整趋势资格，却不足以单独证明盘整背驰；有 third 也不足以单独
证明完整趋势资格。

### 3.2 先例效力

| 先例 | 仍有效的约束 | 对本案的直接后果 |
|---|---|---|
| #28 | `Cand^δ` 取趋势背驰；盘整背驰只作诊断、不入 strict chain；确认取完成时因果口径。其 `J_parent=确认区间` 后被 #37 部分改判。 | 即使未来 20 例中出现盘整背驰证书，也不能无 supersede/FULL-RERUN 自动注入 strict chain。 |
| #37 | `J_parent := D_parent`；闭包含保留；子级确认时点不参与区间包含否决；确认时点与结构端点不得互相回填。 | `T_end` 的分类事实不得写回 `divergence_confirm_src`；右删失与结构身份必须分列。 |
| #43 | `c_episode_start != c_start_full`；完整 `c_p` 必须来自递归结构归属；父窗不得扩到 A/整个趋势，也不得按产量调定义。 | 20 例只能按稳定 `(B_p,departure)` 对象复核，不能换 episode、确认窗或宽窗补 third/盘整证书。 |
| #48/#49 R4 | 31/0/11/20；20 例只进入 `ConsolidationReviewPending`，不得自动降类；R3 完整资格是第 18/20/22 行与完成分解的合取。 | 本案只能决定分流状态与后续证书门，不能以报告文字改 `cand_delta` 或 `judge_third_cert`。 |

另有 686 禁止 fallback 方向锚；698 把“盘整背驰单独定义买卖点”与“作为 Type2/3 确认证据”
分层。两者共同否定用 endpoint/fallback 或单一盘整背驰 bit 替代结构分类。

## 4. (a)–(d) 各候选的定义后果与谱系冲突

### (a) 非完整趋势 `c`

**安全定义**：

```text
NotFullTrendQualifiedAsOf(p, T) := NOT FullTrendCQualified(p) provable with data <= T
```

对本 20 例，在 `T=T_end` 时因 `ThirdClassInsideC=false`，安全定义 20/20 成立。

**不能采用的强定义**：`PermanentlyNonTrendC(p)`。当前没有对象终结/重分类证书，数据结束也不是
否定对象；强定义会把开放态冒充完成分类。

**后果与兼容性**：

- 兼容 #37 双时点、#43 完整对象、#48 R3/R4。
- 不改变 `cand_delta` 算法真值、交易信号或 strict chain。
- 若把 as-of 负资格膨胀为永久非趋势，会与 700 的认识论边界和“对象否定对象”纪律冲突。

### (b) 盘整背驰

**最低独立证书**至少应同时证明：

```text
CompletedConsolidationMove(p)          // 完成走势且恰好一个中枢
SameCenter(B_p)
TwoComparableSameDirectionDepartures
LaterDepartureStrength < EarlierDepartureStrength
AsOfClassificationTime
LegalDirectionProvenance               // 继承 686
```

**后果**：

- 需要独立对象类、独立确认时点和独立证书；当前 `judge_third_cert` 的反值不可充当证书。
- 仅进入研究/诊断时不需 supersede #28；若进入 strict chain、改变事件集或交易消费，必须显式
  supersede #28/#49 R6 并做 FULL-RERUN。
- `third=None => consolidation_divergence` 会直接冲突 #48/#49 R4，也与 698 的定义层/确认层分离
  冲突。

### (c) 递归对象映射仍不足

**可检验定义**：若 `B_p` 或 departure 身份缺失/歧义、稳定边不能重放、leave/retest 级别错配，
或同一输入下 `CpId` 漂移，才可把 third 缺失归为映射不足。

**本案结果**：该定义在 20 例上不成立。20/20 有稳定边、确定 `B_p`、departure、
`c_start_full`；同一生产路径已在 L1–L4 闭合 11 例，终点失败被穷尽为 `DIR/REENTER`。

**保留的能力缺口**：

- 第 20 行新高/新低证书；
- 第 22 行内部两个次级别中枢 ID 链；
- 完整走势完成/延伸/重分类状态；
- 独立盘整背驰证书。

这些缺口解释“为何目前不能在趋势/盘整间完成分类”，不解释“为何现有 third 判据漏掉了 20 个
本应合法的 third”。若把两者混写，会回退到 700 §2.3 在 P47 前的开放候选，否定 P47 已闭合的
生命周期层而没有新反例。

### (d) 数据右端截断

**安全定义**：

```text
RightCensoredAt(p, T_end) :=
    lifecycle(p) is open at T_end
AND no object-valued completion/reclassification/negation is observed by T_end
```

它是观察窗属性，不是走势分类，也不是失败原子的替代品。

**后果与边界**：

- 20/20 可标右删失；继续保持 Pending，不能因超时或文件结束删除对象。
- 延长同一数据序列后，只有新的合法 third、完成盘整证书或其它对象事件才能改变状态。
- 20 例均非“事件刚贴右边”：从 `divergence_confirm_src` 到 `T_end` 的 bar 余量最小 349,094、
  中位数 2,607,822.5、最大 4,480,829。故右截断不能替代 `DIR/REENTER` 的已观察事实。
- 若把删失标记解释为“未来必闭合”，同样是无证据外推。

## 5. 推荐排序

| 排名 | 口径 | 建议 | 理由 |
|---:|---|---|---|
| 1 | (a) as-of 负资格 | **采用** | 20/20 因第 18 行已知失败而不具 R3 完整资格；不外推永久分类。 |
| 2 | (d) 右删失修饰 | **采用为状态修饰** | 数据结束不能否定开放对象；保留未来对象事件回溯通道。 |
| 3 | (b) 盘整背驰 | **保留独立复核池** | 原文给出处理方向，但当前无完成盘整/力度比较证书。 |
| 4 | (c) third 对象映射不足 | **驳回为本批主因** | 20/20 稳定边和几何失败齐备；只保留第 20/22 行及盘整分类的能力 backlog。 |

这不是把 (d) 排在 (b) 前就声称“20 个都是被截掉的合法趋势”；排序表达的是状态陈述的证据强度：
as-of 负资格与删失都可由当前对象图直接证明，盘整分类仍待新证书。

## 6. P0 裁决草案

> 状态：**DRAFT FOR P0 / SEPARATE-RULING**，未生效。

### R1：20 例终态登记

采纳 §2 的 20 个 `CpId` 为本轮固定样本。统一登记：

```text
ThirdCertificateLifecycle = Pending
FullTrendQualification = NotQualifiedAsOf(T_end)
ClassificationReview = ConsolidationReviewPending
Censoring = RightCensoredAt(T_end)
```

禁止改写为 `FullTrendQualified`、`PermanentlyNonTrend` 或 `ConsolidationDivergence=true`。

### R2：失败原因裁定

终点 third 失败穷尽分为：

- `DIRECTION_PAIR_MISMATCH`：11；
- `RETEST_REENTERS_B`：9。

确认时 `PAIR/ANCHOR` 只保留为历史快照原因，不得继续用作“终点映射不足”的依据。

### R3：候选口径裁定

1. (a) 采用 as-of 负资格版本；永久非趋势版本不采纳。
2. (d) 采用右删失标记；不采纳“删失即未来闭合”。
3. (b) 保持 `ConsolidationReviewPending`，等待独立盘整背驰证书。
4. (c) 驳回为现有 third 缺失的主因；第 20/22 行和盘整证书能力缺口另列，不与 third 判据混同。

### R4：合法状态转移

对象只可被对象证书推进：

```text
ThirdCertificatePending
  -> ThirdCertificateClosed                 // 原 judge_third_cert 首次合法
  -> ConsolidationDivergenceQualified       // 独立完成盘整 + 力度证书
  -> OtherClassificationQualified           // 另案已裁定义证书
```

数据结束、等待时长、事件稀疏度、fallback 方向均不是状态转移源。若 later third 闭合，仍须另过
第 20/22 行和完成分解，不能直接跳到 `FullTrendQualified`。

### R5：先例继承

- #28 的“盘整背驰不入 strict chain”继续有效；本裁决不改变事件集。
- #37 的双时点/结构区间分离完整保留；终态分类不回填确认时。
- #43 的完整 `c_p` 稳定身份完整保留；禁止 episode/确认窗替代。
- #48/#49 R1–R7 完整保留；本轮仅兑现 R4/R7-5 的分类分流材料。
- 686 fallback 禁令与 698 三层分离完整保留。

### R6：权限与实现边界

本裁决若 adopted，只改变报告/对象状态语义，不授权修改：

- `CandDeltaEvent.cand_delta` 真值；
- `judge_third_cert`；
- 20 个历史快照或终态 `None`；
- strict chain、交易事件集、订单时点；
- 第 20/22 行尚未机器化的完整趋势资格。

任何把盘整背驰接入 strict chain 或交易的方案必须另案显式 supersede，并重跑无前视基线。

## 7. 验收门

### G1：样本与身份完整性

- [ ] 恰好 20 个唯一 `CpId=(L,B_p,departure)`，无重复、无缺行。
- [ ] 级别分布 `L1/L2/L3/L4 = 6/7/6/1`。
- [ ] 20/20 `stable_edge=true`，`terminal_cp_confirm=None`，`terminal_end=None`。
- [ ] 与全量基线 `31 snapshot 0 / terminal 11 / pending 20`、closed 分布 `1/6/3/1` 同时复现。

### G2：失败分桶

- [ ] 终点恰为 `DIR=11 / REENTER=9`，不得出现未解释 remainder。
- [ ] 确认→终点迁移矩阵复现：`PAIR→DIR 7`、`PAIR→REENTER 5`、
  `ANCHOR→DIR 4`、`ANCHOR→REENTER 3`、`REENTER→REENTER 1`。
- [ ] 任何新增 `ANCHOR/PAIR` 终点项先作为数据/实现漂移上浮，不得静默并桶。

### G3：趋势资格

- [ ] 报告分列 `CandidateGenerated / ThirdCertificateClosed / FullTrendQualified`。
- [ ] `FullTrendQualified` 必须逐项具备 R3：third、第 20 行新极值、第 22 行内部中枢 ID 链、
  completed decomposition；现有 `Closed` 不得替代完整资格。
- [ ] later third 不回填历史 event-time 快照。

### G4：盘整背驰独立证书

- [ ] 证明完成盘整走势且恰好一个中枢；不能仅用 `third=None`。
- [ ] 固定同一 `B_p` 与两次同向离开对象，给出确定性 ID、区间、力度 gauge 与严格比较。
- [ ] 给出 as-of 分类确认时点及合法方向 provenance；不得使用 686 禁止的 fallback。
- [ ] 逐例输出 20 个对象的 `qualified / not-qualified / still-pending`，不得只给总数。

### G5：右删失与后续数据

- [ ] `T_end` 只作删失边界，不触发否定、超时清理或自动降类。
- [ ] 延长数据时沿同一 `CpId` 继续推进；若 identity 漂移，先判映射缺陷，不得当成新对象掩盖。
- [ ] 每个状态改变必须登记触发对象与首次可知时点；不能以终态标签回填历史。

### G6：先例与回测边界

- [ ] #28/#37/#43/#49、686、698 的上述有效部分无隐式 supersede。
- [ ] `cand_delta` bit、`judge_third_cert` 和 strict chain 事件集 bit-exact 不变。
- [ ] 任何盘整分流进入 strict chain/交易，必须另立 P0、显式 supersede 并执行 FULL-RERUN
  无前视回测；本报告不能作为授权。

### G7：只读边界

- [ ] P51 自身补丁仅新增 `chanlun/review-results/p51-pending20-triage-20260712.md`。
- [ ] 工作树中的并发 Rust 改动另行归属、验证和提交，不纳入 P51，不用其结果改写本报告分桶。
- [ ] P51 无代码、定义、谱系、goal 事件或既有裁决文件改动。
- [ ] `git diff --check` 通过。

## 8. 裁决前的可证伪条件

以下任一出现，必须重开本材料而不是改桶保结论：

1. 同一 HEAD/数据/Θ 无法复现 20 个稳定 `CpId` 或 `11/9` 终点分桶；
2. 任一 Pending 对象其实已存在满足现 `judge_third_cert` 的合法 third；这将指向 P47 推进/读取 bug，
   而不是定义裁决；
3. 独立盘整证书证明某些对象为盘整背驰：只更新这些对象的分类轴，不反写其历史 third 快照；
4. 延长数据后某对象出现合法 third：该例从右删失 Pending 转为 `ThirdCertificateClosed`，说明 (d)
   对该例是观测窗限制，但仍不自动证明完整趋势资格；
5. 新证据显示稳定边、`B_p` 或 departure 身份错误：只对相应对象恢复 (c) 审查，不把局部反例
   外推为 20/20 全局映射失效。

## 9. 最终建议

P0 可直接 adopted R1–R6 作为**状态语义裁决**：它既落实第 37 课对完整趋势 `c` 的严格资格，
又保留盘整背驰独立证明与未来数据回溯的通道，同时不改变任何代码真值。

不要裁“20 个就是盘整背驰”，也不要裁“20 个永远不会有 third”。当前唯一有数据闭包的逐例结论是：
**20 个稳定 `B_p/c_p` 对象截至 `T_end` 均未取得 third；其终点失败为 11 个方向不配与 9 个严格
重入；因此全部保持 Pending + ConsolidationReviewPending，并带右删失标记。**
