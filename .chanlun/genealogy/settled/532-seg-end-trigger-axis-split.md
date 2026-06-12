---
id: '532'
number: 532
title: "38课段终结判定的单轴性被数据质询——master 出场语义与 voice REV 腿的概念分离候选"
type: 概念分离
status: 已结算
date: 2026-06-10
settled_date: 2026-06-10
settlement: 修正形式分离成立——master 轴单轴性否证 L3 确立（双标的），voice 轴正增量未确立（regime 依赖）
source: "analysis/organic_fugue_design.md §5.2 + analysis/organic_fugue_backtest.md（OKLO 首轮）"
depends_on:
  - '267'   # 操作方法论 FSM
  - '525'   # 笔中枢退化基底（segment 级真实 BSP）
affects:
  - "analysis/organic_fugue.py（OrganicConfig.master_seg_end 拆解位）"
  - "analysis/organic_fugue_design.md §5.2 master 表（'同 T1 触发'的单轴声明）"
---

## 分离候选

设计（organic_fugue_design.md §5.2）把第38课段终结判定（confirmed type1 卖 ∨
div(up,*) 卖侧 ∨ confirmed type3 卖）读作**一根轴**：同一触发集同时作用于
master 出场（持股→持币）与 voice REV 开腿（段尺度反向）。理由是 §0 统一原则
（单一 FSM 多声部转位——master 的 REV 态 = 持币）。

OKLO 首轮数据（O1 vs P5）质询了这个单轴性：

- O1 vs_B0 = −175.6pp（P5 = +450.6pp），交易数 220→803；
- 但 **rev 腿独立净现金 +55,639 为正**（与 P5 域腿 +32K 同量级）；
- 归因：master 出场从 sell1（confirmed type1 卖）扩展到三触发后，平均持仓
  时长锐减 → 满仓暴露被砍——负贡献来自 master 出场语义变化，不来自 voice
  REV 腿本身。

即：**同一个"段终结"触发集，在 master（真实仓位的存在论：持股/持币）和
voice（共享仓位上的反向短差腿）两个声部上的经验后果符号相反。**
"账本对操作的解释不同"（§0 承认的唯一差别之一）在这里不只是解释差别——
它使同一触发集的有效域分裂。

## 拆解工具

OrganicConfig.master_seg_end 配置位 + O1v 探索性变体（voice-only REV：
master 出场保持 P5 sell1）。Δ(O1v) 与 Δ(O1) 的差 = master 触发扩展的独立
贡献。

## 结算条件（待 QQQ/BRN 数据）

- 若 O1v 在 ≥2/3 标的 Δ>Δ(O1) 且 rev 腿净现金为正 → 分离成立：38课段终结
  判定需按声部角色分裂为两个概念（master 出场判定 vs voice 反向腿触发），
  设计 §5.2 master 表"同 T1 触发"修正；
- 若 O1v ≈ O1 同负 → 分离不成立，负贡献另有来源（如 REV 腿对 cost_basis
  路径的扰动），本候选作废；
- 若 O1v 转正（Δ(O1v)>0）→ 最强形式：voice REV 腿是 P5 的正增量，仅 master
  触发扩展被否证。

## 关联

- 走势方向代理陷阱（project_trend_direction_proxy）：同构先例——把一个判定
  用于其有效域之外的声部/层级。
- E3（买回侧次级别确认否证）：同为"机制在不同操作位上的有效域分裂"。

## 结算记录（2026-06-10，QQQ 数据落盘后）

**数据**（analysis/organic_fugue_backtest.md，OKLO 447K + QQQ 728K）：

| 标的 | Δ(O1) | Δ(O1v) | Δ(O1v)−Δ(O1) = master 触发扩展独立贡献 | O1v rev 净现金 |
|------|-------|--------|---------------------------------------|---------------|
| OKLO | −626.2pp | −63.7pp | **−562.5pp（master 扩展之害）** | +40,100 |
| QQQ | −80.4pp | −41.4pp | **−39.0pp（master 扩展之害）** | −13,926 |

**预注册分支评估**：

- 条件2（O1v ≈ O1 同负 → 作废）：**排除**——两标的 Δ(O1v) 均显著优于 Δ(O1)。
- 条件3（Δ(O1v)>0 最强形式）：**排除**——两标的 Δ(O1v) 均 <0。
- 条件1（≥2/3 标的 Δ>Δ(O1) 且 rev 净现金正）：Δ>Δ(O1) 于 2/2 成立；
  rev 净现金正仅 1/2（OKLO）；**BRN 未跑**（回测覆盖 OKLO+QQQ 两标的）——
  条件1 按字面（分母 3）不可评估，按可得数据（分母 2）部分满足。

**裁决：修正形式的分离成立**

1. **单轴性否证（L3，双标的一致）**：master 出场触发从 sell1（confirmed
   type1 卖）扩展到 38课三触发，携带独立的大额负贡献（OKLO −562.5pp /
   QQQ −39.0pp，方向一致）。§5.2 master 表"同 T1 触发"的单轴声明被否证：
   **master 出场判定与 voice 反向腿触发是两个概念**——前者保持 sell1，
   后者保留三触发。
2. **voice 轴正增量未确立（L2，regime 依赖）**：rev 腿独立净现金 OKLO 正
   （+40,100，强趋势标的）/ QQQ 负（−13,926）；两标的 Δ(O1v)<0。voice
   REV 腿不是 P5 的已证正增量，其有效域依赖标的 regime——这本身强化分离
   （voice 轴有独立于 master 轴的有效域边界）。

**与预注册条件1的偏差声明**（090号严格性）：本结算不声称条件1满足。分离
成立的证据基础不是"voice 腿为正"，而是"两轴经验后果可独立测量且 master 轴
被独立否证"——这已足够支撑概念分裂，但分离的强度弱于条件1预期。BRN 缺失
使三标的判据不可按字面评估，结算基于可得的 2/2 标的（master 轴）。

**下游动作**：

- organic_fugue_design.md §5.2 master 表已加 532号修正标注（master RIDE→REV
  保持 sell1，三触发扩展否证）。
- OrganicConfig.master_seg_end 拆解位保留（O1v 即其消费者），无代码改动。
- 若未来 BRN/第三标的数据落盘且 rev 净现金为正于 ≥2/3，voice 轴可升级为
  "已证正增量"——需新谱系条目，不复用本号。

## BRN 三标的补充记录（2026-06-10，Phase 5 全量落盘后）

**数据**（analysis/organic_fugue_backtest.md，BRN 2.4M bars）：

| 标的 | Δ(O1) | Δ(O1v) | Δ(O1)−Δ(O1v) = master 扩展之害 | O1v rev 净现金 |
|------|-------|--------|-------------------------------|---------------|
| BRN | −449.8pp | −186.2pp | **−263.6pp** | −41,431 |

**对既有裁决的影响**：

1. **master 轴单轴性否证：双标的 → 三标的（3/3 方向一致）**。master 触发
   扩展之害 OKLO −562.5 / QQQ −39.0 / BRN −263.6pp，跨股票强趋势、低波动
   ETF、期货高频三个 regime 全部同号——L3 等级以满分母确立，原结算
   "BRN 缺失使三标的判据不可按字面评估"的保留条款解除。
2. **voice 轴升级条件不满足**：结算尾注预留"若 BRN rev 净现金为正于 ≥2/3
   → voice 轴升级为已证正增量"——实际 rev 净现金正仅 1/3（OKLO +40,100 /
   QQQ −13,926 / BRN −41,431），不升级，无新谱系条目。voice 轴维持
   "正增量未确立（regime 依赖）"，且 BRN 数据精化其有效域边界：高频出场域
   （4868 交易）下 rev 腿 avg_diff ≈ 0.002%，执行摩擦敏感性极端
   （@2bps 即 −90,031），与 P5 边界 (c) 同构。
3. **裁决文本不变**：修正形式分离成立的两个分量（master 轴否证 / voice 轴
   未确立）均被第三标的同向加强，无翻转。

预注册条件1（≥2/3 标的 Δ>Δ(O1) 且 rev 净现金正）按满分母评估：Δ 分量
3/3 ✓，净现金分量 1/3 ✗——合取不成立，与原结算"不声称条件1满足"的偏差
声明一致（090号）。
