---
id: '552'
number: 552
title: "anchor 趋势底仓解僵尸核心死锁——吃涨 L2 实证 + 三个半架构（多空双开多重赋格的第一次成立）"
type: 概念发现
status: settled  # 死锁解机制 + anchor 吃涨已结算；吃跌 A/B 进行中（开放轴）
date: '2026-06-22'
settled_date: '2026-06-22'
level: "死锁解机制 = L0（rec_engine 行号）；anchor 吃涨 = L2（BTC btc_bull_bear_segments 实证）；B 递归 anchor 吃跌 / 8 标的 L3 天花板 = 待补（recursive-anchor-B + deadlock-3half-L3 进行中）"
负责工位: "CC session d41059a8（死锁主线）+ 编排者深挖对话（2026-06-22）"
provenance: "[新缠论:实装+诊断+编排者裁决]"
negation_source: homogeneous
negation_form: aufhebung  # 扬弃：cascade 翻空(歧路)被否定保留为A最高级别实例，anchor(核心多腿)提升为死锁正解
negates: "隐含命题『核心僵尸死锁只能靠 cascade 整翻解』——被否证：cascade 翻空在次级别卖点翻主力套牢(547)；正解是 anchor 让核心多腿不僵死、持多吃趋势，根本不翻空"
depends_on:
  - '546'   # 僵尸核心死锁（enter 全空仓门 × 几何衰减）
  - '547'   # cascade 级别错配（次级别卖点翻主力）
related:
  - '539'   # 做空有效域⊂非上行 regime（做空亏的旧诊断，被本号修正：非regime本质是回补/级别错配）
tensions_with: []
---

# 552 号：anchor 趋势底仓解僵尸核心死锁

**认识论**：死锁解机制 L0；anchor 吃涨 BTC L2 实证；吃跌 A/B 待补。

## 一、死锁全图（编排者卡三天的问题）
546 号僵尸核心死锁的本质 = **多空双开多重赋格（目的）从未实现**：核心多腿被 sink `quota=units/3` 几何衰减成 1e-6 僵尸，核心持多却吃不到大趋势（踏空），一直在「做空亏 vs 踏空」兜圈子（编排者：用 RTAS 前卡三天）。

## 二、anchor 解死锁（吃涨那一半，L2 实证）
**anchor**（`HOLD_ANCHOR` flag，commit 1f38fbd0eb）：enter 把核心分趋势底仓 m×(1−MOBILE_FRAC)=2/3（死扣吃大趋势）+ 机动仓 m×MOBILE_FRAC（sink 短差）；sink/drain 配额只作用机动仓（units−anchor）→ 核心趋势腿不被几何衰减成僵尸。

**L2 实证（BTC btc_bull_bear_segments）**：大牛段从踏空（OFF +0.1%）→ 吃到 80%：
- 牛 3811→64800（BH+1600%）引擎 **+1273%**（吃 80%）
- 牛 30101→69000（BH+129%）引擎 **+125%**（吃 96%），核心多% 100%
- 全程 OFF +37% → anchor **~+550% 量级**

**这是死锁正解——核心多腿不僵死、持多吃趋势，不是 cascade 翻空**（编排者点破：「核心持多吃趋势，不是核心翻空」）。

## 三、三个半架构（目的=吃所有级别涨跌幅绝对值）
完整 = 每级别 [anchor 多腿吃涨 + 卖点翻空吃跌]，递归（=滤波器组多空双开，[[feedback_filter_bank_metaphor_prove]]）：
1. **核心能动**（cascade `T_CASCADE_FLIP`，commit bdad6c8ab6）：打破僵尸核心恒占 highest_active，n_enters 4→78 = Face A（enter 全空仓门）解
2. **核心多腿不僵死**（anchor `HOLD_ANCHOR`）：吃涨已 L2 实证 = Face B（recover 欠触发/几何衰减）解
3. **次级别 sink 空腿**（已有）：吃次级别回调
4. **级别归属**（547，commit 44f042877d）：cascade 翻主力只在 `e_level≥top_active_trend_level`（涌现最高级别本身完成），次级别卖点不翻主力走 sink

## 四、吃跌待补（两条都要，编排者 2026-06-22 裁决）
- **A = 涌现最高级别走势卖点翻核心**（吃最高级别大跌）。最高级别是**涌现的**（`top_active_trend_level` 动态 L4→L5）。卡点 = 涌现最高级别走势 `completed` 跨年滞后（emergent_top 用 completed 过滤）。解 = **区间套级联**（次级别背驰确认涌现最高级别完成，减滞后），不违反 547（翻的仍是核心）。**辅——最高级别那次罕见。**
- **B = 递归 anchor 到每级别空腿**（吃中间级别回调，如 BTC 2022 −51%，本质是中间级别 L4 下跌走势）。当前 anchor 只核心一层；B 递归到每级别 [anchor 多腿 + sink 空腿]。**主力——吃跌主体。** commit c67acc87ea（RECURSIVE_ANCHOR）+ 4fcfd2840d（变体2 深度门），recursive-anchor worktree，进行中。

**进行中（compact 时未出结果）**：`recursive-anchor-B`（B 路递归 anchor，死终止判据：2022 吃到跌幅=成/否定≤3变体）+ `deadlock-3half-L3`（anchor 单层 8 标的天花板=B 对照基线）。

## 五、上游谱系
- 546（僵尸核心死锁）/547（cascade 级别错配）：本号的死锁诊断前置。
- 539（做空有效域⊂非上行 regime）：**本号修正**——做空亏不是 regime 本质，是回补缺失 + 级别错配（次级别卖点翻主力）；级别归属修对后核心持多吃趋势、次级别 sink 吃回调。
- [[feedback_filter_bank_metaphor_prove]]（滤波器多空双开=吃每级别涨跌幅）/[[project_deadlock_dual_open_target]]（memory 死锁全图）/[[project_cascade_level_misattribution]]（547）。

## 六、开放轴（吃跌 A/B 待结算）
1. **B 递归 anchor 有效域**（recursive-anchor-B 验证中）：2022 中间级别回调由中间级别空腿吃到没有？否定则递归 anchor 不是 2022 解。
2. **A 区间套级联减滞后**（排 B 后）：cascade 卖点识别从 emergent_top.completed 改次级别背驰级联，提前翻涌现最高级别。
3. **入主树**：代码全 flag 控制（HOLD_ANCHOR/T_CASCADE_FLIP/RECURSIVE_ANCHOR，默认 OFF=bit-exact），在 worktree 分支，待 B/L3 结果 + 编排者裁决入主树。

## 七、影响声明
死锁吃涨已解（anchor L2 实证，编排者卡三天问题的解）；吃跌 A 待 / B 否证。未改主树生产路径（全 worktree 分支 + flag OFF bit-exact）。

## 八、B 路否证 + anchor 全程最优（recursive-anchor-B 结果，2026-06-22）
**递归 anchor（B 路）在 BTC 否证——吃跌幅与吃涨幅互斥。** BTC 三 flag + 深度变体：
- **HOLD_ANCHOR（单层）**：2022 纯持多骑跌(−51%)，大牛 +1273%，**全程 +1136.9%（BTC 最优工作形态）**，强平3
- REC 全递归 / depth=2：2022 空腿吃到(+1584 / +447)✓ 但大牛崩(−55% / +82%，核心空73%)，全程 **−87.4% / −45%**，强平5
- REC depth=1：= HOLD_ANCHOR（2022 冻结0，全程 +1077%）

**二分锐利**：depth≥2 吃跌 ∧ 崩涨 / depth≤1 保涨 ∧ 不吃跌——**吃 2022 跌幅的深层持久空腿，正是在 2020-21 强牛失血、把核心翻空触发强平级联的那条深腿**。与 [[project_t_cross_level_coupling_falsified]]（深层逆势腿强牛 −89.7% 穿仓）同构 = regime 结构税，非可调参（恰 3 变体即停，否定干净）。flag OFF bit-exact，108 单测绿。

**结论**：anchor 吃涨（HOLD_ANCHOR +1136.9%）= BTC 最优。吃跌剩 **A 路**（涌现最高级别卖点翻核心，最高级别完成时翻——**不与吃涨互斥**，因吃涨在大趋势中、A 翻在大趋势完成后；但卡识别滞后）/ 或接受只吃涨。**下一步看 A，B 路关闭。**
