# P48：`cand_delta` 命名与第 37 课完整趋势 `c` 资格——P0 裁决准备材料（2026-07-12）

- 任务：#48，只读研究。
- 当前分支 / HEAD：`p0-replay-dparent` / `99ce554b8a`。
- 最终裁决权：**P0**。本文仅提供论证、推荐排序与裁决草案。
- 一级权威：[第 37 课第 18/20/22 行](/tmp/p0-replay-work2/docs/chanlun/text/blog/037-第37课.md:18)。
- 数据权威：[P46 §2.1/§2.2/§5](/tmp/p0-replay-work2/chanlun/review-results/p46-third-closure-zero-20260712.md:68)。
- 上游硬边界：[686 号](/tmp/p0-replay-work2/.chanlun/genealogy/settled/686-q7-fallback-direction-anchor-separation-narrows-121-decision-a.md:132)、[#37 P0 效力](/tmp/p0-replay-work2/chanlun/escalate/p0-jparent-dparent-ruling-20260710.md:1)、[#43 P0](/tmp/p0-replay-work2/chanlun/review-results/dparent-leftend-p0-review-20260711.md:1)。
- #47 状态：当前未提交工作树已有双时点和 `B_p/c_p` 生命周期实现；本轮直接运行既有 release 二进制复现全量 `31 / 0 / 11 / 20`。这属于当前工作树事实，不冒充已合并、已结算事实。

## 0. 总结论

四候选并非互斥四选一，而是三个不同裁决轴：

1. 双时点——时态轴；
2. 严格资格 + 完成态限定——命名与资格轴；
3. 分类分流——趋势/盘整分类轴。

推荐顺序：

1. **双时点口径**：第一推荐，可作 adopted-default。
2. **严格资格口径**：第二推荐；先收窄命名，不改 `cand_delta` 真值。
3. **完成态限定口径**：第三推荐；与前两项组合使用。
4. **分类分流口径**：第四推荐；只能形成独立复核池，禁止自动降类。

最小安全口径：

> `cand_delta=true` 保留为因果时点上的算法背驰事件；`cp_certificate_confirm_src` 表示同一 `B_p/c_p` 第三类证书首次可证时点；“完整趋势 c 背驰”是更严格的派生资格，至少要求第 37 课第 18、20、22 行全部机器闭合。终态 `third=None` 只进入未决复核池，不自动改判为盘整背驰。

## 1. 权威链与先例边界

### 1.1 第 37 课的三条连续限定

- 第 18 行：`c` 必须是次级别，且至少包含对 `B` 的第三类；否则可看作 `B` 的小级别波动、按盘整背驰处理。
- 第 20 行：上涨 `c` 必须创新高、下跌 `c` 必须创新低；即使已有第三类，不创新高/低仍可视作围绕 `B` 的次级别震荡。
- 第 22 行：因 `c` 包含第三类，`c` 内至少包含两个次级别中枢，才能满足离开后回拉不重回中枢，并继续递归套用 `a+A+b+B+c`。

因此，现有 `judge_third_cert` 只覆盖第 18 行的第三类几何核心，不能自动证明第 20 行新高/新低或第 22 行两个次级别中枢。

### 1.2 已结算定义

- [beichi.md](/tmp/p0-replay-work2/.chanlun/definitions/beichi.md:103)：趋势背驰前提是趋势；趋势至少有两个同向中枢。
- [qushi.md](/tmp/p0-replay-work2/.chanlun/definitions/qushi.md:48)：趋势是至少包含两个依次同向中枢的走势类型，其内部结构才是 `a+A+b+B+c`。
- [zoushi.md](/tmp/p0-replay-work2/.chanlun/definitions/zoushi.md:16)：趋势/盘整是对完成走势类型的分类，完成与延伸必须区分。
- [maimai.md](/tmp/p0-replay-work2/.chanlun/definitions/maimai.md:132)：第三类要求完成的次级别离开与回试，并以 `[ZD,ZG]` 为边界。
- [698 号](/tmp/p0-replay-work2/.chanlun/genealogy/settled/698-source-tracing-type2-panzhengbeichi-vs-maimai-def4.md:40)：盘整背驰不能单独定义 Type2/Type3，但可作为确认层证据；`trend` 背驰门只适用于 Type1。

较旧定义仍留有“盘整背驰不产生买卖点”的宽表述。涉及 Type2/Type3 时，应服从 2026-07-02 的 `maimai.md` v1.1 和 698 号收窄，不能把旧表述反向用作自动降类依据。

### 1.3 686、#37、#43

- **686**：Consolidation/endpoint fallback 方向不得作为一/三类方向锚。因此不能用 fallback 补齐 P46 的 `anchor=None` 或强行补出 third。
- **#37**：拆除 `confirm_src ≡ J.end` 同义绑定。同一生产路径上数值相等不代表语义同一，也不得互相派生。当前代码已显式分出 `divergence_confirm_src`。
- **#43**：`c_episode_start` 不等于 `c_start_full`；只有逐事件结构证明相等时才能共享数值。`D_parent` 只消费 `c_interval_full`，不得回退 episode、确认点或产量锚。

本次张力与后二者同构：算法事件可归属于 `c_p`，但不因此等于已完成、已具全部资格的 `c_p`。

## 2. P46 的共同事实分桶

### 2.1 确认时：0/31 third 闭合

| 最小失败原子 | 数量 | `L/confirm_src` |
|---|---:|---|
| `NO_B_OWNED_ADJACENT_PAIR` | 21 | L1/759272、1044073、2365194、3306324、4264505；L2/174527、354036、510836、658767、1967685、2182560、2636582、3427468、3436636；L3/693716、960644、1835970、1870550、2534466、4001207；L4/2826563 |
| `LEAVE_ANCHOR_NONE` | 8 | L1/345518、2168349；L2/132770、1761310、1843204、2197213；L3/206731、2329634 |
| `RETEST_REENTERS_B` | 1 | L3/1036108 |
| `DIRECTION_PAIR_MISMATCH` | 1 | L4/4071883 |

确认时，31 例都是 `cand_delta=true` 算法事件，但没有一例具备现有 third/cp 证书。

### 2.2 数据终点：11 Closed / 20 Pending

终态 third 证书闭合 11 例：

- L1：3306324。
- L2：354036、510836、1761310、1967685、2182560、3427468。
- L3：693716、960644、1870550。
- L4：4071883。

终态仍 Pending 20 例：

| 终点最小失败原子 | 数量 | `L/confirm_src` |
|---|---:|---|
| `DIRECTION_PAIR_MISMATCH` | 11 | L1/345518、2168349、2365194、4264505；L2/132770、174527、658767、2636582；L3/1835970、2329634、2534466 |
| `RETEST_REENTERS_B` | 9 | L1/759272、1044073；L2/1843204、2197213、3436636；L3/206731、1036108、4001207；L4/2826563 |

11 例的首次可证延迟为 102..103,750 bar，中位数 2,137 bar。“终态”只是本数据截断点，因此 20 例只能称“截至终点未形成”。

### 2.3 #47 双时点覆盖

本轮执行：

```text
CP_SMOKE_MAX_BARS=999999999 CP_SMOKE_MODE=batch CP_SMOKE_VERBOSE=0 \
  ./rust/target/release/cp_capability_smoke
```

得到：

```text
bars=4613599
parent_events=483
stable_edges=483
snapshot_objects=108
terminal_objects=202
P46_L1_L4 objects=31 snapshot_closed=0 terminal_closed=11 terminal_pending=20
L1=7/0/1/6
L2=13/0/6/7
L3=9/0/3/6
L4=2/0/1/1
```

当前覆盖：

- `divergence_confirm_src`；
- 快照 `cp_certificate_confirm_src`；
- 稳定 `CandDeltaEvent -> B_p/c_p` 归属边；
- `CpScanOwnership.lifecycle: Pending|Closed`；
- 终态 `third_class_in_c`、`c_structure`、`c_interval_full`；
- 快照/终态两个显式消费 API。

尚未覆盖：

- 第 20 行创新高/新低证书；
- 第 22 行两个次级别中枢的显式 ID 链；
- 完整趋势分解最终完成态。

故 #47 解决了时态与生命周期，不等于完成第 37 课全部资格。

## 3. 候选一：严格资格口径

> `cand_delta=true` 只称算法背驰候选；完整证书后才升级为趋势 `c` 背驰。

### 原文一致性

正面：

- 与第 18 行直接一致：无 third 不宣称完整 `c`。
- 与第 16 行“没有趋势，没有背驰”一致：工程早期事件不抢占原文趋势背驰名称。
- 可把第 18/20/22 行表达为完整合取资格。

反面：

- 如果“完整证书”只指现有 `judge_third_cert`，仍缺第 20/22 行；11 例不能直接叫完整趋势 `c`。
- 早期算法候选是 `[新缠论]` 工程对象，原文没有直接授权其操作含义。

### 裁定兼容与 supersede

- 686：兼容，无需 supersede。
- #37：兼容，是“同值不等于同义”的延伸。
- #43：兼容，完整资格只消费完整 `c_p`。
- 已结算定义：兼容其趋势前提。

需要部分 supersede 的是旧命名，不是算法真值：

- “`Cand^δ = 趋势背驰段谓词/趋势背驰确认`”收窄为“算法背驰确认候选”；
- `cand_delta ↔ buy1/sell1` 的逐 bit 事实保留，但不再外推为完整趋势资格。

若直接改变 `cand_delta` 真值，则会改变 P1 事件集和严格证书生产，必须单独 supersede 并全量重跑；不建议在 #48 执行。

### 下游影响

- 当前直接消费者是 `nest::assemble_certificates`、`strict_nest_check`、`cp_capability_smoke` 和可选 runner sidecar；runner 明确声明 sidecar不参与订单、候选、风控或账本。
- 仅收窄名称时不改变因果信号。
- 若把完整资格变成原事件 gate，31/31 都会延迟或消失，属于新策略、新回测。
- `econ_positive.rs` 内另有同名 `cand_delta` dispatcher，不能未经同一性证明机械联动改名或改义。

### 机器成本与 #47 覆盖

- 命名收窄：低成本。
- 第 18 行 third 资格：#47 已覆盖。
- 第 18/20/22 行完整合取：中高成本，需新增新高/新低、内部中枢 ID 链和完成状态。

### P46 分桶

- 确认时：31 个算法候选；0 个 third-qualified。
- 终点：11 个 third-closed；20 个不具 third。
- 若只按第 18 行做阶段升级：11/20。
- 若严格要求三条合取：11 也只是“third 已闭合、完整资格待证”；当前 full-qualified 数未知。

## 4. 候选二：双时点口径

> 保留 `divergence_confirm_src`，另设 `cp_certificate_confirm_src`。

### 原文一致性

正面：

- third 必须等离开和回试完成，晚于算法确认完全合理。
- 第 20/22 行可以继续在证书时点后追加。
- 精确解释确认时 0/31 与终点 11/31 同时为真。

反面：

- 双时点解决可知性，不自动解决命名。
- 当前 `cp_certificate_confirm_src` 只证明 third/cp 证书，不能改称 `full_trend_confirm_src`。

### 裁定兼容与 supersede

- 686：正交兼容。
- #37：直接继承“确认时点与结构端点独立”。
- #43：直接继承稳定归属边和完整对象视图。
- 通常不需 supersede 已结算定义，只需收窄旧报告中过宽的字段说明。

### 下游影响

- 因果路径只能读取事件快照。
- 终态对象只能用于事后结构研究，或作为真正晚到信号在 `cp_certificate_confirm_src` 后消费。
- 回测必须分别报告 event-time 与 terminal/completed-object，禁止终态标签回填历史下单。
- 报告至少分 `divergence_event_count`、`snapshot_cp_closed`、`terminal_cp_closed`、`terminal_pending`。

### 机器成本与 #47 覆盖

#47 已具备字段、稳定边、状态机和两个消费 API；当前剩余成本主要是消费者迁移和分支落地验证。

必要硬门：

- snapshot 不读终态对象；
- terminal 只能经稳定边；
- 历史事件不回填；
- `cp_certificate_confirm_src == third.point_source_index`。

### P46 分桶

- 第一时点：31 个算法事件，快照闭合 0。
- 第二时点：11 个对象随后闭合，20 个仍 Pending。
- 11 不得回写为确认时已知；20 不得写成永久不成立。

## 5. 候选三：分类分流口径

> 缺 third 的事件进入围绕 `B` 的小级别波动/盘整背驰复核，但不自动降类。

### 原文一致性

正面：

- 第 18 行确实给出缺 third 时按 `B` 波动/盘整背驰处理的观察角度。
- 第 20 行又说明：即使有 third，不创新高/低仍可能属于围绕 `B` 的震荡。

反面：

- 原文是“可以看成”，不是 `third=None ⇔ 盘整背驰`。
- 20 例的方向不匹配或回试重入，不等于独立盘整背驰证书已经成立。
- 确认时 31/31 都缺 third，但随后 11 闭合；确认时立即降类会造成至少 35.48% 的重分类。

### 裁定兼容与 supersede

- 686：依赖 fallback 方向即冲突。
- #37/#43：独立对象、独立时点、独立证书时兼容；复用确认点或 episode 作为充分条件则冲突。
- 698/673：支持判据分离，反对一个谓词替代另一类定义。
- 只有 P0 决定把盘整分流纳入 strict chain 时，才需 supersede 旧 P0“盘整背驰不入链”；若仅作诊断或独立策略，无需 supersede。

### 下游影响

- 必须新增独立 signal class 和分类时点。
- 趋势候选与盘整候选必须分别回测、分别归因。
- 报告只能先写 `consolidation_review_pending`；独立证书出现前不能写 `consolidation_divergence=true`。

### 机器成本与 #47 覆盖

至少需要：

- MoveKind/中枢数量及对象身份；
- 独立 `judge_pan_div` 证书；
- 第一次离开/回试、`[ZD,ZG]`、新高/新低；
- as-of 分类时点和对象否定/重分类账本；
- 686 provenance 守卫。

#47 不提供“20 例中哪些属于合法盘整背驰”的证明。

### P46 分桶

- 确认时：31 个分类未决算法事件，不能全部转为盘整。
- 终点：11 个 third-closed，继续留在趋势资格路径。
- 20 个进入 `consolidation_review_pending`：
  - 11 个方向不匹配；
  - 9 个回试重入。
- 当前可确认的独立盘整背驰数是**未知**，不是 20。

## 6. 候选四：完成态限定口径

> 第 37 课资格只施加于已完成趋势分解；当前事件是生成态。

### 原文一致性

正面：

- 第 18/20/22 行讨论的是作为完整结构组成部分的 `c`。
- 与走势完成后分类、但又可以延伸的体系兼容。
- 可诚实容纳 0/31 快照与 11/31 晚到闭合。

反面：

- 若生成态事件仍叫“趋势背驰确认”，只加 Pending 标签并不能解决“没有趋势，没有背驰”的冲突；必须配合命名收窄。
- 生成态不能成为永久模糊地带，必须有明确状态转移和否定事件。

### 裁定兼容与 supersede

- 686、#37、#43：兼容。
- 与“背驰导致走势完成”的已结算表述存在条件性张力：
  - 若 `cand_delta` 收窄为算法候选，无需 supersede；
  - 若坚持它已是真正趋势背驰，却又称所属 `c` 尚未完成，则需 supersede 或重新解释旧定义。

### 下游影响

建议状态机：

```text
CandidateGenerated
  -> ThirdCertificateClosed
  -> FullTrendQualified
  -> 或由独立对象证书 Reclassified
```

- 每个状态只能消费当时可知字段。
- 候选策略与完成策略分别回测。
- 现 #47 `Closed` 宜在报告中写成 `ThirdCertificateClosed`，另设 `FullTrendQualified`。

### 机器成本与 #47 覆盖

- #47 已覆盖 `Pending -> Closed` 的 third 生命周期。
- 仍需第 20/22 行、完整走势完成态及延伸/否定事件。
- Pending 不能因超时或数据结束而被否定。

### P46 分桶

- 确认时：31 `CandidateGenerated`，0 `ThirdCertificateClosed`。
- 终点：11 `ThirdCertificateClosed`，20 `Pending`。
- `FullTrendQualified`：当前字段无法判定，不能把 11 自动抬升。

## 7. 推荐排序与落地门

### 可先行 adopted-default

1. **双时点/双视图**
   - `divergence_confirm_src` + 确认快照；
   - 稳定边 + 终态对象；
   - 禁止前视回填。
   - 当前工作树已满足全量 31/0/11/20；正式生效仍需确认 #47 改动已进入 P0 指定分支。

2. **命名收窄**
   - `cand_delta=true`：算法背驰事件；
   - `ThirdCertificateClosed`：现有 third 证书闭合；
   - `FullTrendCQualified`：第 18/20/22 行全部闭合。
   - 先改报告语义，不改 P1 bit 或交易行为。

3. **继承硬边界**
   - 686 不恢复 fallback；
   - #37 不绑定确认时点与结构端点；
   - #43 不以 episode 代替完整 `c_p`。

### 必须等待验证或另案裁决

- **#47-GATED，当前工作树已满足但尚需落地主分支**：消费者显式选择 snapshot/terminal、31/31 稳定边、最小复现 A/B 和完整回归。
- **POST-#47**：第 20 行创新高/低、第 22 行内部中枢 ID 链、`FullTrendQualified` 状态。
- **SEPARATE-RULING**：20 例的盘整/小级别波动分流。
- **FULL-RERUN**：任何改变 `cand_delta` 真值、strict chain 事件集、订单信号或历史入场时点的方案。

## 8. P0 裁决草案

> 状态：DRAFT FOR P0，未生效。

### R1：对象与命名

`CandDeltaEvent.cand_delta=true` 裁为 `[新缠论] 算法背驰确认事件`，表示既有算法在 `divergence_confirm_src` 成立；它不等价于第 37 课完整趋势 `c` 资格。

旧“`Cand^δ = 趋势背驰段谓词/趋势背驰确认`”表述 `SUPERSEDED-IN-PART`：算法真值与逐 bit 同源事实保留，完整趋势资格外推撤销。

### R2：双时点

保留两个不可互换时点：

```text
divergence_confirm_src
cp_certificate_confirm_src
```

任何消费者必须显式选择确认快照或终态对象，禁止未来信息回填。

### R3：完整趋势资格

```text
FullTrendCQualified(p) :=
    TrendContext(p)
AND ThirdClassInsideC(p, B_p)          // 第18行
AND NewExtremeInDirection(p)           // 第20行
AND InternalSublevelCenters(c_p) >= 2  // 第22行
AND CompletedTrendDecomposition(c_p)
```

现 #47 `Closed` 只证明 `ThirdClassInsideC` 生命周期闭合。

### R4：31 例登记

- 确认时：31 个算法事件，0 个 third 快照闭合。
- 终点：11 个 `ThirdCertificateClosed`，20 个 `Pending`。
- 11 个仍等待第 20/22 行与完成分解证明。
- 20 个只进入 `ConsolidationReviewPending`，不得自动降类。

### R5：因果与回测

- 早期策略只能使用 `divergence_confirm_src` 当时快照。
- 晚到资格策略最早只能在相应证书确认后行动。
- 终态标签不得回填历史交易。
- 当前 strict runner consumer 是旁路 sidecar，不参与订单；若未来接入交易，必须新建因果回测基线。

### R6：继承边界

- 686 完整保留。
- #37 完整保留。
- #43 完整保留。
- 盘整背驰继续不入 strict chain，除非 P0 另案显式 supersede。

### R7：机器验收

1. 复现 #47 的 `31/0/11/20` 与 `1/6/3/1`；
2. 增加第 20 行新高/新低证书；
3. 增加第 22 行两个次级别中枢的确定性 ID 链；
4. 报告分别列 event-time、third-closed、full-qualified、classification-review；
5. 改变 `cand_delta` 或交易消费时重跑无前视回测；
6. 20 例分类分流另立 P0。

### R8：权限声明

R1-R7 均为裁决草案。是否 adopted、哪些条目 supersede、是否进入生产或回测，最终裁决权均在 **P0**。

---

本轮仅重新执行了既有 release 全量诊断；没有重新执行 `cargo test`。P47 谱系回溯记录中的 `1554 passed / 127 ignored / 0 failed` 属既有工作树记录，仍应由 P0 在最终分支上复核。

记忆仅用于定位 #37 的历史上下文；上述易漂移事实均已用当前仓库与当前二进制重新核验。

<oai-mem-citation>
<citation_entries>
MEMORY.md:71-73|note=[used to locate confirm_src and J separation precedent]
</citation_entries>
<rollout_ids>
</rollout_ids>
</oai-mem-citation>