---
id: "686"
number: 686   # 当前最大占用=685。撞车让向下一空号，最终编号 /ritual 统一分配。
title: "Q7-#1 codex 裁定C：UnitRange.direction 两种来源（趋势 ownership 方向 vs endpoint fallback 方向）概念分离——收窄（非推翻）#121 裁定A 的隐含消费假设；实装后 L≥1 一类信号 77~95% 被识别为 fallback 泄漏伪信号"
status: 生成态   # 裁定已实装（commit cdddaa78ac）并接入生产锚（1464634a2f）。最终结算待 #146 rerun5 下游重跑完成。
date: "2026-07-03"
type: 概念分离
source: "[新缠论] codex decide 裁定C（.chanlun/review-results/codex-q7-fallback-20260703.md，transcript codex-decide-20260703-222813-c563.md）+ commit cdddaa78ac 实装 + commit 1464634a2f 漏斗探针接生产锚"
negation_source: heterogeneous   # codex-cli decide（Q7-#1 跨裁决张力：#121 裁定A vs Q7 审计致命项）
negation_model: codex-cli
negation_form: separation   # UnitRange.direction 单一字段内部暴露两种不兼容语义来源（趋势 ownership 方向 / endpoint fallback 方向），需分离
depends_on: ["683"]
related: ["683", "685", "231", "090", "675"]

# 拓扑效果标注（147号下游推论3）
# negates：#121 裁定A 的隐含消费假设——"UnitRange.direction 不论来源都可当真实趋势方向消费"。
# 实际拓扑后果（retrospective，141号结论1）：该假设被分裂为两个字段/语义——direction（保留几何/结构方向，
#   继续成员/序列/面积用途）+ anchor_direction（新增，资格判定：仅 center_own_dir_at 非 None 时可作方向锚）。
#   #121 的核心架构（级别-N 直接判定、units 承担线段角色）不受否定，只是其消费点判据收窄。
#   scope=downstream：#145(ac4-impl) 产出的 L1 349/L2 102/L3 31 旧基线随之失效，新基线 L1 18/L2 14/L3 7
#   （L0 575 逐位不变=豁免路径实证）；675号 meta-rule 复发（探针parity基准分叉）在本次实装中被触发并修复。
topo_effect: "split:121-decision-a-direction-consumption-assumption:downstream"

# 矛盾（#121 裁定A 隐含假设 vs Q7 审计致命项）
contradiction:
  description: |
    #121（codex-t1，2026-07-03 早些时候）裁定 A："级别-N 直接判定"——units（携 UnitRange.direction，
    来源=fold_direction 的 endpoint 比较）整体承担线段角色，复用 L0 判据链。裁决时 direction 只有
    fold_direction 一种来源，无 provenance 区分必要性。
    约 12 小时后，`project_to_units` 新增 `blocks`/`center_own_dir_at` 参数（Q7 本轮引入），
    direction 出现第二种潜在来源：Consolidation 块 ownership 单元的 endpoint fallback（
    `center_own_dir_at==None` 时退化为 `fold_direction`）。Q7 审计（codex-review-20260703-221031-3801.md
    #1，致命/reject）发现：该 fallback 方向未标注 provenance 就流入 `extract_first_third_for_level`，
    被当作真实趋势方向消费——违反缠论定义（一类买卖点定义域="上涨/下跌趋势中"，盘整=恰1中枢无方向，
    盘整背驰是独立范畴非趋势背驰弱化版，第31课/知识库§7.1/§6.7/§9.1/§10）。
    两个裁决在同一字段（UnitRange.direction）上产生跨裁决张力：#121 假设"direction 来源单一，可直接
    消费"，Q7 审计证明"direction 实际有两种不兼容来源，其一（fallback）不能作趋势方向锚"。
  layer: 代码   # UnitRange.direction 单一字段内部两种语义来源的暴露，是确定性代码结构事实（project_to_units 签名扩展后触发）。
  trigger: "Task #146(rerun5) 描述——ac4-impl-20260703.md §8.4 遗留上浮项，编排者全权授权 codex decide 作为跨裁决张力裁决渠道 → Q7 审计致命项 vs #121 裁定A 张力 → codex decide 裁C。"

# 涉及的定义
definitions_involved:
  - name: "#121 codex-t1 裁定A（级别-N 直接判定）"
    version: "codex-decide-20260703-025537-8d3c.md（已结算，683号）"
    role: "被收窄对象。核心架构（units 承担线段角色）不受否定，隐含消费假设（direction 来源单一可直接消费）被收窄。"
  - name: "缠论定义：一类买卖点定义域=趋势中（第31课/知识库§7.1/§6.7/§9.1/§10）"
    version: "docs/chanlun/text/blog/INDEX.md + 缠论知识库.md"
    role: "裁定C的定义依据。盘整无方向，盘整背驰独立范畴——fallback 单元（落在 Consolidation 块 ownership 区间）不具备趋势方向资格。"
  - name: "center_own_dir_at（decompose.rs:449-457）"
    version: "Q7 本轮新增参数链（project_to_units 扩展 blocks 参数）"
    role: "方向 provenance 的判据来源：Some(dir)=趋势 ownership 方向（可作锚），None=fallback（Consolidation 块内/i==0，不可作锚）。"

# 解决方式
resolution:
  type: 概念分离   # UnitRange.direction 拆分为 direction（几何/结构，保留）+ anchor_direction（资格判定，新增）
  description: |
    codex decide 裁**C：分级处置**（拒A=维持现状不区分/拒B=fallback单元严格排除不作序列成员）。
    fallback 单元保留为序列/区间/面积累计**成员**（维持时间连续性/中枢关系），但不得作
    `locate_departure_move_a`（A段候选）/`judge_first_cached`（C段/破中枢段）/`departure_move_c_start`
    （离开段）的**方向锚**。新增 `anchor_direction: Option<Direction>` 字段：
    `Some(center_own_dir_at 结果)` 时可作锚，`None`（fallback）时不可。三处消费点判据从
    `s.direction == trend_dir` 改为 `s.anchor_direction == Some(trend_dir)`。
    实装（commit cdddaa78ac）：L0 入口恒等（575 一类逐位不变=豁免路径实证——L0 无 Consolidation
    fallback 消费问题，本裁定只影响 L≥1）；L1 349→18（排除331，94.8%）、L2 102→14（排除88，86.3%）、
    L3 31→7（排除24，77.4%）、L4 0→0；一类总计 1057→614。**旧基线 77~95% 是 fallback 方向泄漏
    伪信号，非真实趋势背驰信号**。
  decided_by: 蜂群内部（codex decide 裁C，编排者全权授权裁决渠道；ws-rerun5 工位实装+核验）

# 被否定的方案
negated:
  description: "维持 #121 裁定A 隐含假设：UnitRange.direction 不论来源（趋势 ownership / endpoint fallback）都可直接当真实趋势方向消费，无需 provenance 区分。"
  why_negated: |
    (1) 缠论定义域不符：一类买卖点定义域显式限定"趋势中"（第31课），盘整（1中枢无方向）的
        ownership 单元 endpoint fallback 方向不具备趋势方向语义——直接消费=方向泄漏。
    (2) #121 裁决时前提不成立冲突：direction 只有 fold_direction 一种来源时该假设无害；Q7 本轮
        `blocks`/`center_own_dir_at` 参数引入后，同一字段承载两种不兼容语义，假设过期未复核
        （与683号"过期未复核的文档化简化"同构模式）。
    (3) 实测坐实：Q7 前泄漏面=100%（全部 endpoint 占位）；裁C 实装后 L1/L2/L3 旧信号 77~95%
        被识别为 fallback 锚产生——不是市场结构信号，是消费未区分 provenance 的伪信号。

# 新产出
new_output:
  definitions:
    - "方向 provenance 分离（direction provenance separation）：一个"方向"字段若存在两种不兼容来源
      （结构性 ownership 方向 vs 边界占位 fallback），消费方必须用独立的资格字段（anchor_direction）
      区分，不能用同一字段值直接做趋势判定——否则未区分来源的消费=方向泄漏。与683号"过期未复核的
      文档化简化"同族：概念前提在新参数引入后过期，未随之复核的消费点即为泄漏面。"
    - "旧基线有效域收窄：#145(ac4-impl) 产出的 L1 349/L2 102/L3 31 一类信号，有效域=裁定C 前语义
      （direction 不分 provenance）。凡引用该旧基线或其汇总（一类总计1057）的下游结论，须标注
      裁定C 前后语义差异，不得跨裁定外推。"
  code_changes: "rust/src/theta_v0/classifier/{mod.rs,divergence.rs,signal.rs} 新增 anchor_direction 字段+三处消费点判据替换（commit cdddaa78ac，631行）。"
  orchestration_changes: "无（本记录为谱系补记，不改编排流程）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/classifier/mod.rs（unit_to_segment/extract_first_third_for_level 签名扩展）"
    - "rust/src/theta_v0/classifier/divergence.rs（locate_departure_move_a/departure_move_c_start）"
    - "rust/src/theta_v0/classifier/signal.rs（judge_first_cached 方向匹配条件）"
  affected_definitions:
    - "683（level≥1 一/三类候选生成，codex-t1 裁A）：核心架构不受否定，消费判据被本号收窄——683 的"过期未复核"模式在此复现（direction 单来源假设过期）。"
    - "685（一类=0 被推翻为趋势门/中枢延伸实现缺陷）：#142-#145 修复序完成后的中间态基线（一类1057）经本号裁C 过滤后收敛为614——与685反事实预测（L0局部650段同向）量级吻合，是685核心论点的追加实证支持。"
  downstream_implications:
    - "#146(rerun5) 下游重跑清单须以本号新基线（L0 575/L1 18/L2 14/L3 7，总计614）为准，不得沿用裁C前的1057。"
    - "'方向字段是否区分 provenance' 应纳入 genealogist 常设巡检——任何新增的 fallback/占位判据被后续参数扩展触发时，须核其原有消费点是否已被复核（683/686 同族方法论）。"

# 回溯结算（待 #146 rerun5 完成后补记）
retroactive_settlement:
  settled_by: "#146 rerun5 全下游重跑（三同族复测+新 prereg alpha）完成后，由 genealogist 回填 settled/。"
  settlement_date: "2026-07-04"
  settlement_description: "goal CLOSED a5 CHECK_PASS：裁定C 新基线 614（rerun5 实证冻结）；裁定C 实装 cdddaa78ac + 接生产锚 1464634a2f 已在。"

# 谱系关联
related_records:
  parent: "683（level≥1 一/三类候选生成过期未复核模式的同族复现）"
  children: []
  related:
    - "683：同族方法论——commit 内假设的阻塞/适用前提在后续参数扩展后过期未复核，产生消费泄漏。683=方向来源被误判'不存在'，686=方向来源被误判'单一'。"
    - "685：本号是685号修复序（#142-#145）完成后的裁决收口之一——旧中间态基线1057经本号过滤后新基线614，量级验证685反事实预测（650段局部同向）。"
    - "675（探针必须走生产路径，已结算meta-rule）：本号实装（commit cdddaa78ac）触发675复发实例——漏斗探针 type1_funnel_dx 在L≥1曾自锚（报旧语义349/102/31非新生产18/14/7），已由commit 1464634a2f修复接生产锚。详见675号settled文件复发实例章节。"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "L1 349→18/L2 102→14/L3 31→7（排除77~95%），L0 575→575 恒等，一类总计1057→614"
    level: "L2（真实 BTC 全历史数据，裁定C 实装前后差分实测，commit cdddaa78ac，可否证）"
    increment: "高：旧基线有效域坍缩，量化了 fallback 泄漏面"
  - proposition: "UnitRange.direction 两种来源（趋势 ownership 方向 / endpoint fallback）概念不兼容，需 provenance 分离"
    level: "L0（代码结构事实：center_own_dir_at 参数引入前后 direction 语义变化）+ 缠论原文对照（第31课趋势定义域）"
    increment: "高：#121 隐含假设过期定位，收窄而非推翻的边界判定"
---

# 概念分离 686：Q7-#1 裁定C——方向 provenance 分离收窄 #121 裁定A

## 结论

codex decide 裁定 **C（分级处置）**：`UnitRange.direction` 的两种来源——趋势 ownership 方向（结构性）
与 endpoint fallback 方向（Consolidation 块内占位）——是**不兼容的语义**，需分离。fallback 单元保留
为序列/区间/面积成员，但不得作一/三类判据的方向锚（新增 `anchor_direction` 字段区分）。

**与 #121 裁定A 的关系：收窄，非推翻**——#121"级别-N 直接判定"架构继续成立，只是其"direction 不论
来源都可消费"的隐含假设被废止（该假设在 #121 裁决时因 direction 只有单一来源而无害，Q7 本轮参数扩展
后过期未复核）。

**实装数据**（commit `cdddaa78ac`）：L0 575→575（豁免路径逐位不变）；L1 349→18（排除331，94.8%）；
L2 102→14（排除88，86.3%）；L3 31→7（排除24，77.4%）；一类总计 1057→**614**。**旧基线 77~95% 是
fallback 方向泄漏伪信号，非真实趋势背驰。**

## 溯源

#121 codex-t1 裁A（direction 单一来源时无害）→ Q7 本轮 `project_to_units` 新增 `blocks`/
`center_own_dir_at`（direction 出现第二来源）→ Q7 审计致命项（fallback 未标注 provenance 被当真实
方向消费）→ 跨裁决张力上浮 → codex decide 裁C（commit `cdddaa78ac`）→ 675号 meta-rule 复发触发
（漏斗探针自锚分叉，commit `1464634a2f` 修复）。

## 边界条件（翻转）

- 若领域定义正式授权"盘整 ownership 单元 endpoint 方向可作上级趋势方向锚"，或 `fold_direction` 被
  严格证明等价于某级别趋势 ownership 方向，本号订正方向翻转，裁C 需重新评估（同 codex 摘录§4）。
- 若实测发现保留 fallback 作序列成员仍间接制造方向泄漏且无法局部守卫，则升级为裁B（严格排除，不作
  序列成员）——本号"分级处置"结论回退。

## 影响声明

谱系补记，零代码改动归属本记录（实装=commit cdddaa78ac + 1464634a2f，已由 ws-rerun5 落地）。下游
影响：683 的过期未复核模式同族复现；685 修复序中间态基线（1057）经本号收窄为614——量级验证685反事实
（局部650段同向）；675 meta-rule 复发实例（探针parity基准分叉）已并入675号settled文件。最终结算待
#146 rerun5 全下游重跑完成。

---

## 结算节（ws-settle，2026-07-04）

**结算依据**：goal CLOSED（events.jsonl:291）。`a4`/`a5` CHECK_PASS：Q7 裁定C 实装（commit cdddaa78ac + 接生产锚 1464634a2f）已在，rerun5 实证冻结新基线一类 614（L0 575/L1 18/L2 14/L3 7）。#146 rerun5 下游重跑完成。关闭条件达成。
**结算日期**：2026-07-04。
