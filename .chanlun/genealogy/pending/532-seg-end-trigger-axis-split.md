---
id: '532'
number: 532
title: "38课段终结判定的单轴性被数据质询——master 出场语义与 voice REV 腿的概念分离候选"
type: 概念分离候选
status: 生成态
date: 2026-06-10
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
