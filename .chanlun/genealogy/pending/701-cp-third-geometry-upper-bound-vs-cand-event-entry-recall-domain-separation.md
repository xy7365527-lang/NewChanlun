---
id: "701"
number: 701
title: "第三类几何召回上界 U 与 CandDeltaEvent 入口集合 E 的对象域分离：U\\E=202、E\\U=20，31 事件不得冒充第三类候选全集"
status: 已实装待复核
date: "2026-07-12"
type: domain
record_type: 矛盾发现
source: "[新缠论] P52 BTC 1m 全量只读召回审计（chanlun/review-results/p52-recall-upper-bound-audit-20260712.md）+ [旧缠论] 第20课第三类买卖点定理"
negation_source: homogeneous
negation_model: "P52 只读诊断工位 + genealogist 张力检查"
negation_form: separation
depends_on: ["700", "673", "683", "686"]
related: ["647", "675", "692", "090", "231"]
negates: []
topo_effect: "split:third-geometry-candidate-vs-cand-delta-event-entry:local"
---

# 矛盾发现 701：第三类几何召回上界与 Cand 事件入口对象域分离

## 一句话结论

BTC 1m 全量 4,613,599 bar、`ThetaConfig::default()` 下，绕过 `cand_delta` 入口、直接在
L1–L4 的 2,630 个稳定 `B_p/c_p` 身份上复用现行 `judge_third_cert`，得到第三类几何成功集合
`U=213`；现有 `cand_delta=true` 事件身份集合为 `E=31`，且：

```text
|U ∩ E| = 11
|U \ E| = 202 = 9（同 CpId 有事件但 cand_delta=false）+ 193（同 CpId 无事件）
|E \ U| = 20
```

因此，“第三类几何合法候选”与“CandDeltaEvent 算法背驰事件”是两个不同对象域，既非相等，也不能由
P52 证据单独证明一方应无条件替代另一方。P53 的后续 P0 裁决已授权另立几何入口事件集合并全纳
`U=213`，但不改写历史 `cand_delta` 真值。P47/P50 的 `31/11/0/20` 仍是 **旧 E 内部**的生命周期/完整趋势
资格基线，不得膨胀成第三类候选全集的召回陈述；反过来，`U\E=202` 也只坐实几何召回漏出，不自动
取得背驰事件、strict-chain 或 `FullTrendCQualified` 资格。

## 1. 触发矛盾与 L2 证据

### 1.1 隐含同一性前提

700 号及 P47/P50 在 `E=31` 内回答了：确认时 0、终态 third-closed 11、仍 Pending 20、完整趋势
资格 0。该口径对事件生命周期审计是正确的，但若把 31 个事件继续当作“可出现合法第三类的全部
`B_p/c_p` 身份”，就隐含了：

```text
judge_third_cert 几何成功对象 ⊆ cand_delta=true 事件对象
```

P52 的独立稳定对象直扫给出 202 个反例，否定了这个集合包含关系。

### 1.2 集合分解

| 集合/桶 | 总计 | L1 | L2 | L3 | L4 |
|---|---:|---:|---:|---:|---:|
| 稳定 `B_p/c_p` 对象 | 2,630 | 2,088 | 445 | 84 | 13 |
| `U`：现行 `judge_third_cert` 成功 | 213 | 136 | 53 | 17 | 7 |
| `E`：现有 `cand_delta=true` 事件 | 31 | 7 | 13 | 9 | 2 |
| `U ∩ E` | 11 | 1 | 6 | 3 | 1 |
| `U \ E` | 202 | 135 | 47 | 14 | 6 |
| `E \ U` | 20 | 6 | 7 | 6 | 1 |

认识论等级为 L2：真实 BTC 全历史数据、默认配置、全对象穷尽扫描，可由 P52 报告命令复现。诊断与
生产生命周期复用 `nearest_confirmed_center_idx`、合法 `anchor_direction` provenance 及未修改的
`judge_third_cert`；区别仅是从稳定对象全集出发，而非从 `CandDeltaEvent` 出发。

## 2. 必须分离的两个对象

### 2.1 第三类几何候选 `U`

`U` 回答：“给定稳定 `B_p/c_p` 身份，是否存在一对相邻、同级、身份连续的 leave/retest，使现行
第三类原子全部成立？”它对应第20课第三类买卖点定理的机器几何投影。`U` 不自行要求该身份已经成为
背驰事件或 strict-chain 事件。

### 2.2 Cand 事件入口 `E`

`E` 回答：“现有事件构造与 `cand_delta` 真值链是否产出该 `B_p/c_p` 身份？”它承载背驰候选入口、
事件时点、通道 provenance 及 strict-chain 的上游资格，不是纯第三类几何枚举器。

两者分离后，四个象限都有明确语义：

1. `U∩E=11`：现有事件且终态第三类闭合；
2. `E\U=20`：现有事件但截至终点第三类未闭合，继续保持 classification-review；
3. `U\E=202`：第三类几何成功但未进入现有 `cand_delta=true` 事件集；
4. 其余稳定对象：现行第三类原子未全部成立。

这解释了为何 `E` 与 `U` 不是简单的“严格/宽松”子集关系，而是两个条件轴的交叉投影。

## 3. 入口漏出归因边界

202 个 `U\E` 中：

- 9 个同一 `(level, B_p ID, departure ID)` 已有事件对象，但 `cand_delta=false`；
- 193 个事件输出中没有同一身份，标为 `NO_EVENT_FOR_CP_ID`。

第二类标签只定位到“同身份未进入事件输出”，不能进一步声称是哪一个背驰、时点、候选构造或其它
上游原子失败。本号因此只登记“事件入口不是第三类几何集合的召回上界”，不把 193 例统一归因为
`cand_delta` 单一条件，也不把 202 例命名为 strict-chain 的真实漏事件。

若后续要放宽入口，必须逐例补齐上游事件资格证书，并作为独立 P0 裁决；不得用本集合差直接修改
`cand_delta` 真值或扩写历史事件。

## 4. 与相邻谱系的张力检查

### 4.1 700：深化而非结算

700 已把 31 事件内的 0/11/20 分成生命周期、时序与定义张力；P47 只结算生命周期半边。P52 证明
31 不是稳定第三类几何对象的全集，使 700 的开放问题从“E 内 20 个为何未闭合”扩展为两个正交问题：

1. `E\U=20`：事件为何没有第三类几何闭合；
2. `U\E=202`：第三类几何成功为何没有对应事件入口资格。

这不结算第37课完整趋势 `c` 映射；反而进一步禁止用 31 事件样本替代全对象召回口径。

### 4.2 673 / 683：同族但净新维度成立

- 673 处理统一 `Cand^δ` 谓词跨 BSP 类型的范围误用；本号不是再次证明某一谓词条件错误，而是量化
  **稳定第三类几何对象域与事件入口对象域**的集合差。
- 683 处理 L≥1 一/三类候选生成曾被 `else { Vec::new() }` 架构禁闭；该缺口已经结算。本号在候选
  生成可运行之后仍得到 `U\E=202`，净新维度是入口召回域，而非恢复 683 的旧分支缺口。

### 4.3 686：无矛盾，资格边界完全继承

P52 的 `U` 继续要求合法 `anchor_direction=Some(direction)`；`ANCHOR_NONE` 的 1,335 个稳定对象不
进入成功集合。没有恢复 endpoint fallback，也没有用 `direction` 占位补齐成功项。因此 686 的方向
provenance 禁令不受否定。

### 4.4 692：轴不同，不构成反例

692 结算的是 `cand_channel` provenance 轴与 `i_class` 类别轴正交，以及**已有候选**的 Type2/Type3
分派不造成类别坍缩。本号审计的是候选是否进入事件输出的召回轴。`U\E=202` 不证明 692 的类别/
通道结论错误，也不要求在 Cand 层增加第二道第三类门；两号有效域不同，可分层成立。

### 4.5 647 / 675 / 090 / 231

- 647：先用完整 `(level, B_p ID, departure ID)` 锁定对象，再做集合差，未拿价格近似替代身份；
- 675：诊断复用生产对象、路由与 judge，不另建回填/fallback 坐标系；
- 090：报告明确把 202 限定为几何漏出，未宣称 strict chain 已具备扩展能力；
- 231：`E=31` 的有效域被收窄为事件入口集合，不外推为全部第三类几何候选。

张力可通过对象域/资格轴分层解决，不触发中断 #1，也不需要概念层立即选边。

## 5. 被否定的隐含方案与解决状态

negated:
  description: >-
    用现有 31 个 cand_delta=true 事件作为第三类几何候选的召回全集，并把 11/20 当作全稳定对象的
    closed/pending 完整分解。
  why_negated: >-
    全量穷尽扫描构造了 202 个同一生产 judge 成功但不属于 E 的反例，因此 U⊆E 的集合命题为假。

resolution:
  type: 已实装待复核
  description: >-
    P53 P0 已裁决入口全纳：生产生命周期中经 judge_third_cert 闭合的稳定 B_p/c_p 对象全部生成
    CandDeltaEntryEvent。旧 cand_delta=true/false 算法背驰真值与 31 个既有事件记录保持不变；放宽后
    E=213，来源分桶为旧 true 11、旧 false 9、无旧事件 193。E旧\U 的 20 例继续保持
    ConsolidationReviewPending，不冒充几何成功。
  decided_by: P0 #53 入口放宽裁决（2026-07-12）

## 6. 影响与明确禁区

impact:
  affected_modules:
    - "P52 只读诊断：稳定 B_p/c_p 召回枚举与事件集合对齐"
    - "后续所有引用 P47/P50 31/11/0/20 的报告：必须标注其有效域为 E 内部"
  affected_definitions:
    - "CandDeltaEvent 与第三类几何候选的映射继续生成态，二者不得再无标注视为同一对象域"
  downstream_implications:
    - "入口资格审计须对 9 个 cand_delta=false 与 193 个无同 CpId 事件分别逐例追踪上游原子"
    - "P50 full-qualified=0 不变；202 个 U\\E 对象未获得完整趋势证书"

明确不修改、不授权：

- 不改 `judge_third_cert` 条件分支；
- 不改 `cand_delta` 真值或 strict-chain 事件集；
- 不回填历史快照，不把事后几何成功伪装成事件时可知；
- 不恢复 686 fallback 方向；
- 不自动降类 `E\U=20`，保留 #49 R4；
- 不把 `U\E=202` 静默纳入交易、订单或风控路径。

## 7. 递归完成与后续回溯条件

第0层：P47/P50 在 31 事件内得到 11/20；第1层：P52 扩到 2,630 个稳定身份，发现 U=213、
U\E=202；第2层：按同 CpId 事件存在性分成 9/193，但 193 的更细上游失败原子尚未追踪；第3层：
与 692/686 碰撞后排除类别轴坍缩与 fallback 伪修法，只剩独立入口资格问题。范围由全对象集合差收缩
到逐例入口原子，且无新定义可在本轮合法推出，达到背驰+分型的结构完成点。

回溯结算条件：独立 P0 审计逐例给出 `U\E` 的上游事件资格证书，并由定义裁决明确
`CandDeltaEvent` 与第20课第三类几何对象的映射关系；在此之前保持生成态。

related_records:
  parent: "700（31 事件内生命周期/时序/定义张力）+ 673（Cand 谓词有效域分离）"
  children: ["chanlun/review-results/p53-entry-relaxation-20260713.md"]

## 8. P53 实装回溯（2026-07-13）

状态更新为 **已实装待复核**。实装没有把 `cand_delta` 的算法背驰真值改义为第三类几何：

1. `CandDeltaEvent` 继续承载历史算法背驰事件与双时点字段；
2. 新 `CandDeltaEntryEvent` 只消费生产 `CpScanOwnership::Closed`，其 Closed 已由原
   `nearest_confirmed_center_idx + judge_third_cert + legal anchor` 生命周期 writer 产生；
3. 同一 CpId 的来源按 `cp_ownership`、再按 `c_structure` 回退匹配，严格得到 `11/9/193`；
4. Closed 缺 departure、结构、third 或确认时点时硬失败并打印对象例外，不静默漏项；
5. 原 31 个 `CandDeltaEvent` 为只读输入，20 个几何失败对象继续逐例登记
   `ConsolidationReviewPending`。
6. 【已验证】全量测试为 `1620 passed / 0 failed / 133 ignored`，相对 P52 的 1,617 基线净增
   3 条 P53 回归；fresh batch 为 `upper=event=aligned=213 / missed=0 / extra=0`。
7. 【已验证】fresh batch 与 2026-07-13 完成的 4,613,599-bar 逐 bar 增量日志各抽取 218 条
   `P52_` 行，SHA-256 同为
   `6c5599dc161a3a3a72dd79bc800d8391fa8e1a6e7b9a820f0412c49e0bc936e4`，零差异；
   两路径均无 `P53_EXCEPTION`，旧 20 例仍逐例分为 11 个方向不配对与 9 个回试重入。
8. 【已验证】`git diff --check` 通过；精确 `git add` 因 sandbox 无法创建 `.git/index.lock`
   （`Operation not permitted`，exit 128）而受阻，本记录不把未提交工作树冒充已提交状态。

【已验证】P53 报告中的全量 cargo test、batch/逐 bar 增量 `P52_` 零差异与
`git diff --check` 已成立。【推断】只有提交落盘并由复核者确认后，才具备另案迁入 settled 的条件；
本任务不自行迁移谱系状态。
