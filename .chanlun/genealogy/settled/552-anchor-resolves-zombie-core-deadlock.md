---
id: '552'
number: 552
title: "anchor 趋势底仓解僵尸核心死锁——吃涨 L3 八标的实证（强牛解踏空 / 震荡是结构税）+ 三个半架构（多空双开多重赋格的第一次成立）"
type: 概念发现
status: settled  # 死锁解机制 + anchor 吃涨已结算（L3 八标的）；cascade 半步 L3 否证（见 553）；B 路 L3 否证；A 路 D_TOP @ a0=segment L3 否证（见 554）；吃跌未试仅剩 a0=stroke（开放轴）
date: '2026-06-22'
settled_date: '2026-06-22'
level: "死锁解机制 = L0（rec_engine 行号）；anchor 吃涨 = L3（HOLD_ANCHOR 8 标的 ×3 strat_pct 交叉验证，OFF bit-exact 对照，108 单测绿，commit 486a391db3 @ worktree agent-ad3eb3b39639b8306）；anchor 吃涨有效域 = 强牛 regime ⊊ 全标的（震荡 regime 是结构税，非 bug）；B 递归 anchor 吃跌 = L3 否证（吃跌幅与吃涨幅互斥）；three-half（anchor+cascade）matrix = **已 L3 否证**（cascade flip 有效域空集 8/8 ≤ anchor，见 553，commit 260dfc4545）；A 涌现最高级别卖点翻核心（D_TOP 区间套精确切换）= **L3 否证 @ a0=segment**（区间套链坍缩 n_a_path_switches 21/24=0，有效域≈空集，退化为 anchor 锁核心 + 低级别空腿失血混合态，见 554，worktree wf_2f91805d-f6f-1）；吃跌未试仅剩 a0=stroke 更细尺度（开放轴）"
epistemological_level: "死锁解机制 = L0（逐字源码）；anchor 吃涨 = L3（八标的 ×3 真实数据，含否定性结果——震荡 3/8 恶化划定有效域边界）；B 路吃跌 = L3 否证（BTC 三 flag + 深度变体）；cascade 半步 = L3 否证（three-half matrix，见 553）；A 路 D_TOP = L3 否证 @ a0=segment（区间套链坍缩，见 554）；有效域 = 强牛 regime（5/8 解踏空）⊊ 全标的，震荡 regime（3/8）是结构税。**不声称 anchor 普适**（有效域膨胀禁止，formalization-validity-domain 模式 1/3）"
负责工位: "CC session d41059a8（死锁主线）+ 编排者深挖对话（2026-06-22）+ 8 标的 L3 后台 agent aaa6a98433eeb7c15"
provenance: "[新缠论:实装+诊断+编排者裁决+八标的 L3 回测]"
negation_source: homogeneous
negation_form: aufhebung  # 扬弃：cascade 翻空(歧路)被否定保留为A最高级别实例，anchor(核心多腿)提升为死锁正解
negates: "隐含命题『核心僵尸死锁只能靠 cascade 整翻解』——被否证：cascade 翻空在次级别卖点翻主力套牢(547)；正解是 anchor 让核心多腿不僵死、持多吃趋势，根本不翻空"
topo_effect: "split:546-zombie-core:downstream"
# split（扩张型）：anchor 把僵尸核心节点（546 sink quota=units/3 几何衰减成 1e-6 僵尸）分裂为两个——
# 核心趋势底仓 2/3（死扣，不受 sink 几何衰减，吃大趋势）+ 机动仓 1/3（受 sink/drain，吃次级别短差）。
# 一个保留原 sink 规定（机动仓），一个携带违反记录（底仓豁免 sink）。
# scope=downstream：分裂作用于 546 死锁链下游（sink/drain 配额计算 → 几何衰减 → enter 门）整条路径。
depends_on:
  - '546'   # 僵尸核心死锁（enter 全空仓门 × 几何衰减）——本号是其 split 解
  - '547'   # cascade 级别错配（次级别卖点翻主力）——cascade 歧路否定的来源
related:
  - '539'   # 清仓判据 regime 门控 + capture-early-then-preserve——anchor 震荡恶化 = 同一 regime 谱型的吃涨侧镜像（539 在 NRF 清仓维度 / 546 在重建维度 / 本号在底仓豁免维度）
  - '545'   # emergent_top 方向锚——A 路涌现最高级别识别滞后的来源
  - '553'   # cascade flip L3 否证——本号三个半架构中 cascade 半步的否证（cascade 毁 anchor 底仓）
  - '554'   # A 路 D_TOP L3 否证——本号 §四 A 路吃跌的 L3 裁决（区间套链 @ a0=segment 坍缩，有效域空集）
tensions_with: []
---

# 552 号：anchor 趋势底仓解僵尸核心死锁

**认识论**：死锁解机制 L0；**anchor 吃涨 8 标的 L3 实证**（强牛 5/8 解踏空 / 震荡 3/8 结构税）；B 路吃跌 L3 否证；cascade 半步 L3 否证（553）；A 路 D_TOP @ a0=segment L3 否证（554）；吃跌未试仅剩 a0=stroke。

## 一、死锁全图（编排者卡三天的问题）
546 号僵尸核心死锁的本质 = **多空双开多重赋格（目的）从未实现**：核心多腿被 sink `quota=units/3` 几何衰减成 1e-6 僵尸，核心持多却吃不到大趋势（踏空），一直在「做空亏 vs 踏空」兜圈子（编排者：用 RTAS 前卡三天）。

## 二、anchor 解死锁（吃涨那一半，8 标的 L3 实证）

**anchor**（`HOLD_ANCHOR` flag，commit 1f38fbd0eb→486a391db3）：enter 把核心分趋势底仓 m×(1−MOBILE_FRAC)=2/3（死扣吃大趋势）+ 机动仓 m×MOBILE_FRAC=1/3（sink 短差）；sink/drain 配额只作用机动仓（units−anchor）→ **核心趋势腿不被几何衰减成僵尸**（546 死锁的 split 解，见 topo_effect）。

### 2.1 L3 矩阵：HOLD_ANCHOR 8 标的 ×3 strat_pct（OFF bit-exact 对照，108 单测绿）

> 后台 agent `aaa6a98433eeb7c15` 跑完，commit `486a391db3` @ worktree `agent-ad3eb3b39639b8306`。
> S/A/O = strat_pct 三模式（Structural / AND / OR）。OFF S(旧) = HOLD_ANCHOR 关闭（即 546 僵尸核心基线）。

| 标的 | ANCHOR S | ANCHOR A | ANCHOR O | BH | OFF S(旧) | regime | P1(超BH) |
|---|---:|---:|---:|---:|---:|---|:---:|
| OKLO | **+458.9** | +452.8 | +428.5 | +307.1 | +173.8 | 强牛 | ✓ |
| BTC | +1136.9 | +970.9 | +1112.3 | +1380.4 | +37.3 | 强牛(82%BH) | ✗ |
| ES | +398.2 | +398.2 | +400.5 | +594.3 | −8.3 | 强牛(67%BH) | ✗ |
| GC | +151.3 | +152.6 | +57.5 | +257.3 | −28.3 | 强牛(59%BH) | ✗ |
| QQQ | +108.6 | +108.4 | +109.6 | +174.6 | −0.7 | 强牛(62%BH) | ✗ |
| CL | **−71.7** | −72.4 | −90.0 | +28.2 | +22.2 | 震荡(灾难) | ✗ |
| BRN | **−30.8** | −31.5 | −78.8 | +87.4 | −9.8 | 震荡(恶化) | ✗ |
| DX | **−10.5** | −10.5 | −11.9 | +4.1 | −1.0 | 震荡(恶化) | ✗ |

（单位均为 %。粗体 = 该 regime 极值标的。）

### 2.2 结论：anchor = 干净的 regime 分离器

- **强牛 5/8 解踏空**（OKLO/BTC/ES/GC/QQQ）：anchor S 全部从 OFF 的 ≈0%（甚至负）跃升到吃到大趋势 59%–146% 的量级——OKLO +173.8→+458.9（超 BH）、BTC +37.3→+1136.9（吃到 82% BH）、ES −8.3→+398.2、GC −28.3→+151.3、QQQ −0.7→+108.6。OFF 的「踏空 ≈0%」正是 546 僵尸核心；anchor 让核心趋势底仓死扣不被 sink 几何衰减 → 持多吃涨。
- **震荡 3/8 恶化**（CL/BRN/DX）：anchor S 从 OFF 的正/小负翻成深负——CL +22.2→−71.7、BRN −9.8→−30.8、DX −1.0→−10.5。死扣底仓在震荡反转时套牢、不能减仓防御 → 强平死亡（见 §2.4）。
- **regime 边界清晰**：强牛 anchor 解踏空 / 震荡 anchor 是结构税，二分锐利、无中间地带。**这是 anchor 的有效域 = 强牛 regime ⊊ 全标的**，不是「anchor 普适」。

### 2.3 per-level long_pnl（S 模式，吃涨机制的直接证据）

anchor 吃涨机制 = 核心多腿（高涌现级别趋势底仓）吃高级别大趋势。per-level long_pnl 坐实：

| regime | 标的 per-level long_pnl（S 模式） | 机制 |
|---|---|---|
| 强牛 | OKLO L3 +147152 / L4 +33684；BTC L3 +43927 / L4 +19001；ES L3 +7935；QQQ L3 +14032；GC L4 中性 | **高涌现级别 anchor 多腿集中为正 → 核心底仓死扣吃大趋势** |
| 震荡 | CL L4 **−37647**；BRN L3 **−36887** | **死扣底仓在震荡高层反转时套牢 → 高层负** |

强牛的盈利集中在高涌现级别（L3/L4）= anchor 底仓在大趋势那一层死扣持多吃涨幅；震荡的失血也集中在高层 = 同一死扣底仓在震荡反转时无法减仓防御。**同一 anchor 机制，强牛是护城河、震荡是套牢——对称的 regime 后果。**

### 2.4 liq（强平）= 震荡恶化的机制

| regime | liq（强平次数） | 机制 |
|---|---|---|
| 强牛 | 0–5 | 死扣底仓持多吃涨，几乎不触发强平 |
| 震荡 | **CL = 19** | **死扣底仓不能减仓防御 → 震荡反转时被强平死亡** |

震荡 CL liq=19 是 anchor 震荡灾难（−71.7%）的直接机制：anchor 把底仓 2/3 豁免 sink（这正是它解强牛踏空的原因），代价是底仓在震荡反转时**不能被 sink 减仓防御**，反复套牢直到强平。**强平是 anchor 在震荡 regime 的结构税，不是 bug**——是「底仓豁免 sink」机制在震荡 regime 的必然后果（强牛护城河的对称面）。

### 2.5 这是死锁正解（吃涨那一半）

**核心多腿不僵死、持多吃趋势，不是 cascade 翻空**（编排者点破：「核心持多吃趋势，不是核心翻空」）。强牛 5/8 解踏空是 546 死锁吃涨侧的真解。**但仅在强牛 regime——震荡是结构税，吃跌待 A 路。**

## 三、三个半架构（目的=吃所有级别涨跌幅绝对值）
完整 = 每级别 [anchor 多腿吃涨 + 卖点翻空吃跌]，递归（=滤波器组多空双开，[[feedback_filter_bank_metaphor_prove]]）：
1. **核心能动**（cascade `T_CASCADE_FLIP`，commit bdad6c8ab6）：打破僵尸核心恒占 highest_active，n_enters 4→78 = Face A（enter 全空仓门）解。**但 L3 否证为净负反作用——本错误形式（emergent_top.completed 粗识别）在强牛中翻空=毁 anchor 底仓（见 553）。**
2. **核心多腿不僵死**（anchor `HOLD_ANCHOR`）：吃涨已 **L3 八标的实证**（强牛 5/8）= Face B（recover 欠触发/几何衰减）解
3. **次级别 sink 空腿**（已有）：吃次级别回调
4. **级别归属**（547，commit 44f042877d）：cascade 翻主力只在 `e_level≥top_active_trend_level`（涌现最高级别本身完成），次级别卖点不翻主力走 sink。**553 §2.3 暴露其在强牛中不足（最高活跃级别走势仍会 completed 误触发）。**

## 四、吃跌待补（两条都要，编排者 2026-06-22 裁决）
- **A = 涌现最高级别走势卖点翻核心**（吃最高级别大跌）。最高级别是**涌现的**（`top_active_trend_level` 动态 L4→L5）。卡点 = 涌现最高级别走势 `completed` 跨年滞后（emergent_top 用 completed 过滤，545 号方向锚）。解 = **区间套级联**（次级别背驰确认涌现最高级别完成，减滞后），不违反 547（翻的仍是核心）。**辅——最高级别那次罕见。** **【554 号 L3 裁决，2026-06-22】A 路 D_TOP 区间套精确切换已实装（worktree 3cc8c49b7a，113 单测绿 bit-exact）并 L3 否证 @ a0=segment**：区间套链全面坍缩（n_a_path_switches 21/24=0），A 路退化为「anchor 锁核心 + 低级别空腿照常失血」混合态，**比 OFF 和 ANCHOR 都差**（强牛 A_PATH ≪ ANCHOR：BTC −1086pp / OKLO −285pp / ES −410pp）；安全（无 OKLO 穿仓）来自瘫痪不是精度。根因 = 第27课区间套三必要条件（链贯通∧本级趋势∧力度衰减）@ a0=segment 几乎从不同时满足。**否证的是形式（@ a0=segment 区间套精度），不是「吃最高级别大跌」目标本身**（见 554）。
- **B = 递归 anchor 到每级别空腿**（吃中间级别回调，如 BTC 2022 −51%，本质是中间级别 L4 下跌走势）。当前 anchor 只核心一层；B 递归到每级别 [anchor 多腿 + sink 空腿]。**主力——吃跌主体（曾期望）。** commit c67acc87ea（RECURSIVE_ANCHOR）+ 4fcfd2840d（变体2 深度门），recursive-anchor worktree。**本错误形式（深层空腿全程持空）已 L3 否证（见 §八）；正确形式未试（见 553 §三）。**

## 五、张力检查 / 上游谱系

### 5.1 与 539（清仓判据 regime 门控 / capture-early-then-preserve）——同一 regime 谱型的吃涨侧镜像
> **link 修正（谱系优先于汇总）**：本号旧版 related 把 539 标注为「做空有效域⊂非上行 regime」。文件系统中实际 539 号 = **「清仓判据的 regime 门控开放轴」**（NRF v5 否证 + capture-early-then-preserve）。「根翻空有效域⊂非上行 regime」是 memory `project_t_short_leg_regime_function` / `project_t14_t5_root_flip_necessity` 对早期 539 的旧标签（memory 是 point-in-time 快照，与重构后的 539 文件不符）。本号 related 539 指代**当前文件系统 539**（clearance regime 门控）。

539 的核心命题（§9.4）：**清仓频率是 regime 函数——强牛需不清仓，深崩需清仓，无一普适**。546 §3.2 已确立 539（NRF 清仓维度）与 546（recursive_t 重建维度）是 capture-early-then-preserve 同一谱型的两个显形。**本号 anchor 是该谱型在「底仓豁免 sink」维度的第三个显形**：
- 强牛：底仓死扣不减仓 = 不清仓 → 吃涨（解踏空）。
- 震荡：底仓死扣不能减仓防御 = 该清不清 → 套牢强平（结构税）。
- **anchor 震荡恶化 = 539「该清仓时不清仓」在吃涨侧的镜像**。最优清仓/持仓频率本身是 regime 函数，anchor 把它钉在「永不清底仓」一端，故强牛对（吃涨）、震荡错（套牢）。**capture-early-then-preserve 系列第三例**（539 清仓 / 546 重建 / 552 底仓豁免）。

### 5.2 与 [[project_t_cross_level_coupling_falsified]]——不可减仓的固定腿在逆 regime 被套（同构）
cross_level（commit 5f93000e11）的深层逆势腿在强牛 −89.7%💥 穿仓：单一 free 池的中间级别 1/3 短差腿在强趋势被 1x 强平级联。**anchor 震荡强平（CL liq=19）与 cross_level 强牛强平是同构**：
- cross_level：深层**逆势空腿**（强牛中做空）不能减仓 → 强牛 regime 被套 → 强平。
- anchor：核心**顺势多底仓**（震荡中死扣多头）不能减仓 → 震荡 regime 被套 → 强平。
- **共同根 = 不可减仓的固定腿在逆 regime 被套**（cross_level 的腿逆强牛 / anchor 的腿逆震荡），都是 regime 结构税、非可调参。这也正是 §八 B 路否证的同构依据——B 路的深层持久空腿正是 cross_level 那条逆势腿。

### 5.3 上游谱系
- 546（僵尸核心死锁）/547（cascade 级别错配）：本号的死锁诊断前置。本号是 546 的 **split 解**（topo_effect）。
- 545（emergent_top 方向锚）：A 路涌现最高级别识别滞后的来源；554 进一步确证 A 路 D_TOP @ a0=segment 区间套链坍缩。
- 553（cascade flip L3 否证）：本号三个半架构中 cascade 半步的否证——cascade 能动=翻空=毁 anchor 底仓（净负反作用）。
- 554（A 路 D_TOP L3 否证）：本号 §四 A 路吃跌的 L3 裁决——区间套链 @ a0=segment 坍缩，有效域空集；吃跌三条路（B/cascade/A）全否证，未试仅剩 a0=stroke。
- [[feedback_filter_bank_metaphor_prove]]（滤波器多空双开=吃每级别涨跌幅）/[[project_deadlock_dual_open_target]]（memory 死锁全图）/[[project_cascade_level_misattribution]]（547）。

### 5.4 张力检查结论
**无不可分层解决的矛盾，不新建张力记录。** anchor 与 539/546/cross_level 不冲突——三者是 capture-early-then-preserve / 「固定腿逆 regime 被套」同一 regime 谱型的不同维度显形，分层关系清晰（539 清仓 / 546 重建 / 552 底仓豁免 / cross_level 短差腿）。本号修正了与 539 的 link 标签错配（5.1），属回溯性 link 修复，非新矛盾。

## 六、开放轴（吃跌 A/B 待结算）
1. **B 递归 anchor 有效域**：**本错误形式（深层空腿全程持空）已 L3 否证**（§八）——全程持空在强牛失血，吃跌幅与吃涨幅互斥。B 路这一实现关闭；「吃中间级别回调」目标的正确形式（每级别精确切换）未试（553 §三）。
2. **A 区间套级联减滞后**：cascade 卖点识别从 emergent_top.completed 改次级别背驰级联，提前翻涌现最高级别。A 不与吃涨互斥（吃涨在大趋势中、A 翻在大趋势完成后），但卡识别滞后。**【554 号 L3 裁决】A 路 D_TOP 区间套精确切换已实装并 L3 否证 @ a0=segment**：区间套链坍缩（n_a_path_switches 21/24=0），第27课区间套三必要条件（链贯通∧本级趋势∧力度衰减）@ a0=segment 几乎从不同时满足——精确识别「走势完成」@ a0=segment 不可操作，有效域≈空集（见 554）。**A 路 @ a0=segment 这一形式关闭；吃跌的唯一活路径移至 a0=stroke 更细尺度（526 A0Source）——前景不乐观（pcf 高层链坍缩可能结构性），未测。**
3. **three-half（anchor + cascade）matrix——cascade flip 已 L3 否证（cascade 有效域空集 8/8 ≤ anchor，见 553）**：后台 agent `aaa6a98433eeb7c15` 跑完 anchor+cascade 联合 8 标的 ×3 matrix（commit 260dfc4545）。**cascade flip（核心能动翻转）有效域 = 空集**——8/8 标的 THREE-HALF ≤ ANCHOR，OKLO +458.9→**−106.6💥 穿仓**（cascade 核心翻空毁 anchor 趋势底仓：long_pnl Σ +165815→+2195、short_pnl −108618）。**cascade 与 anchor 在同一核心仓位上语义对立**（cascade 要翻核心 / anchor 要死扣核心）——核心不需要「能动翻转」来解踏空，anchor 死扣持多即真解，cascade 这一粗识别实现是反作用（净负）。**注意：被否证的是 cascade 的 emergent_top.completed 粗识别实现形式，不是「核心在真正顶部翻空」目标本身——A 路（精确识别）是检验后者的下一步，A 路 D_TOP @ a0=segment 现亦由 554 否证。** 详见 553/554 号。
4. **入主树**：代码全 flag 控制（HOLD_ANCHOR/T_CASCADE_FLIP/RECURSIVE_ANCHOR/T_A_PATH_NESTED，默认 OFF=bit-exact），在 worktree 分支，待 a0=stroke 结果 + 编排者裁决入主树。cascade（T_CASCADE_FLIP）粗识别实现已 L3 否证、A 路 D_TOP @ a0=segment 已 L3 否证，均不入主树。

## 七、影响声明
- 死锁吃涨已解（**anchor 8 标的 L3 实证，强牛 5/8 解踏空 / 震荡 3/8 结构税**，编排者卡三天问题在强牛 regime 的解）；吃跌 A 路 D_TOP @ a0=segment 否证（554）/ B 否证 / cascade 半步否证（553），未试仅剩 a0=stroke。
- **准确表述**：anchor 解的是「吃涨那一半」且仅在强牛 regime；震荡 regime 是结构税（不是 anchor 失败，是底仓豁免 sink 机制在震荡的对称后果）；吃跌 B（全程持空形式）否证、A 路 D_TOP @ a0=segment 否证（554）、cascade（粗识别形式）否证（553）。**不是「死锁完全解」。**
- 未改主树生产路径（全 worktree 分支 + flag OFF bit-exact，108 单测绿）。
- 修正与 539 的 link 标签错配（§5.1）。

## 八、B 路否证 + anchor 全程最优（recursive-anchor-B 结果，2026-06-22）
**递归 anchor（B 路）在 BTC 否证——吃跌幅与吃涨幅互斥。** BTC 三 flag + 深度变体：
- **HOLD_ANCHOR（单层）**：2022 纯持多骑跌(−51%)，大牛 +1273%，**全程 +1136.9%（BTC 最优工作形态）**，强平3
- REC 全递归 / depth=2：2022 空腿吃到(+1584 / +447)✓ 但大牛崩(−55% / +82%，核心空73%)，全程 **−87.4% / −45%**，强平5
- REC depth=1：= HOLD_ANCHOR（2022 冻结0，全程 +1077%）

**二分锐利**：depth≥2 吃跌 ∧ 崩涨 / depth≤1 保涨 ∧ 不吃跌——**吃 2022 跌幅的深层持久空腿，正是在 2020-21 强牛失血、把核心翻空触发强平级联的那条深腿**。与 [[project_t_cross_level_coupling_falsified]]（深层逆势腿强牛 −89.7% 穿仓）同构 = regime 结构税，非可调参（恰 3 变体即停，否定干净）。flag OFF bit-exact，108 单测绿。

> **B 路否证的精确范围**（接 553 修正）：B 路否证的是「深层空腿**全程持空不切换**」这一**错误实现形式**——空腿在 2022 吃到跌、却在 2020-21 强牛失血。它**不否证**「吃中间级别回调」这一目标本身；正确形式（每级别骑该级别走势、走势完成时精确切换）未试（见 553 §三）。

**结论**：anchor 吃涨（HOLD_ANCHOR）= BTC 最优、且 8 标的 L3 强牛 5/8 解踏空（§二）。吃跌剩 **A 路**（涌现最高级别卖点翻核心，最高级别完成时翻——**不与吃涨互斥**，因吃涨在大趋势中、A 翻在大趋势完成后；但卡识别滞后）—— **A 路 D_TOP @ a0=segment 现已由 554 否证（区间套链坍缩）**。吃跌未试仅剩 **a0=stroke 更细尺度**（开放轴，前景不乐观）。**B 路（全程持空形式）、cascade 半步（553 粗识别形式）、A 路 D_TOP @ a0=segment（554）全关闭。**
