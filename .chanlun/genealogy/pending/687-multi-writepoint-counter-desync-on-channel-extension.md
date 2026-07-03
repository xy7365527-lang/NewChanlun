---
id: "687"
number: 687   # 当前最大占用=686。撞车让向下一空号，最终编号 /ritual 统一分配。
title: "多写点计数器在新增数据通道时口径分叉（meta-rule候选）：dx真封①(sig_post漏计PanDiv,commit b220f6341e) 与 Stop-Guard熔断计数器(PRE_ACTIVE_TASKS/ACTIVE_TASKS双写点,commit 5457bb78dc)同构；两者均非675号坐标系分叉——探针/生产对拍全程通过，是同一实体多写点未同步演化"
status: 生成态   # 两次同构实例已确认，meta-rule候选待编排者/escalate辨认→/ritual显式化。
date: "2026-07-03"
type: meta-rule
source: "[新缠论] ws-rerun5 真封①判别实验定性补发 + #147(ws-dxsig) commit b220f6341e + #136(stopguard-fix2) commit 5457bb78dc"
negation_source: homogeneous   # ws-rerun5 工位判别实验（父commit 9b06a7265c 即违反，非裁定C引入）+ #147 ws-dxsig 收口分析
negation_form: unclassified   # 非否定既有结论，是模式提炼（两次独立实例达可辨认密度）——不落入waiting/expansion/separation，判定见下
depends_on: []
related: ["675", "686", "090"]

# 拓扑效果标注（147号下游推论3）
# negates：commit b220f6341e 自身 commit message 的归类——"纯探针记账缺口（675号实例）"。
# 实际拓扑后果（retrospective，141号结论1）：该归类不准确。675号的核心机制是"探针读取的坐标系与生产
#   消费的坐标系不同源"；本次 signals_dx==signals_prod 逐条对拍全程通过（数据源坐标系一致，无分叉），
#   真正缺陷是 sig_post 计数器（探针内部的一个累加字段）在 #145 新增 PanDiv 通道后未同步扩展计入——
#   是"同一坐标系内，多写点计数器口径分叉"，与675号"跨坐标系分叉"是不同机制。
#   scope=local：仅纠正 b220f6341e 自身的归类偏差，不影响675号settled文件已记录的复发实例3（该实例
#   对应另一 commit 1464634a2f，机制确系675号坐标系分叉，归类准确，不受本条影响）。
topo_effect: "sever:b220f6341e-self-classification-as-675-instance:local"

# 矛盾（commit自我归类 vs 实际机制）
contradiction:
  description: |
    commit `b220f6341e` message 自称"纯探针记账缺口（675号实例）"。但 ws-rerun5 判别实验给出更精确
    定性：signals_dx==signals_prod 逐条对拍全程通过——即探针读取的数据本身与生产完全一致，不存在
    675号定义的"诊断坐标系≠生产坐标系"问题。真正缺陷在于 dx 探针内部的 `sig_post` 计数器：#145 给
    生产 collect_signals 与 dx 镜像加 PanDiv 承接通道时，`sig_post` 只计 Γ 二通道，PanDiv 信号虽然
    进入 signals_dx 但不进 sig_post 计数——门后计数字段本身未随新通道同步扩展。
    与此同构：Stop-Guard 熔断计数器（task #136，commit `5457bb78dc`）——放行判断读 `PRE_ACTIVE_TASKS`、
    check2 写点写 `ACTIVE_TASKS`，两个变量口径分叉永不相等，counter 涨至 11 仍不放行。
    两例共性：**判断/门槛消费的计数值分散在多个写点/字段，其中至少一处未随系统演化（新增数据通道、
    重构变量名）同步更新，导致判断长期基于口径不一致的数据而不自知**——两次独立实例达可辨认密度。
  layer: 代码   # 计数器口径分叉是确定性代码结构事实（多写点未同步），与数据窗无关
  trigger: "ws-rerun5 判别实验（worktree @9b06a7265c 判别父commit即违反）→ #147(ws-dxsig) 根因收口（commit b220f6341e）→ 定性补发：非675号实例，是独立的计数器口径分叉模式，建议与#136熔断计数器同族归类。"

# 涉及的定义
definitions_involved:
  - name: "675号 meta-rule（探针必须走生产路径，禁诊断坐标系分叉）"
    version: "settled/675-meta-rule-...md（已结算）"
    role: "被排除对象——本次 sig_post 缺陷不满足675号"跨坐标系分叉"的判定条件（探针与生产数据源对拍全程通过），不计入675号实例。"
  - name: "多写点计数器一致性（本号新提炼概念）"
    version: "无先例（本号首次显式化）"
    role: "两次独立实例（#136 Stop-Guard熔断计数器 / #147 dx真封①sig_post）共享的机制：同一判断消费的计数值有多个写点，其中之一未随系统扩展同步更新。"

# 解决方式
resolution:
  type: 未解决   # 模式已提炼，meta-rule候选待编排者/escalate辨认，规则化措辞待/ritual
  description: |
    两个具体实例均已各自修复（#136：write_counter() 单一写点统一，commit 5457bb78dc；#147：
    sig_post_pan 独立计数 + 真封①口径改 sig_post+pan>=n_signals + 新增收集闭合断言，commit
    b220f6341e）。本号不重复记录修复细节，只提炼跨实例的共同机制并建议其独立于675号成为新的
    meta-rule候选。
    **元规则候选措辞**："若一个判断/门槛消费的计数值由多个写点/字段分别累加，当系统新增数据通道
    或重构变量命名时，必须同步核查所有写点/消费点是否已扩展覆盖新通道——否则判断会长期基于
    残缺口径而不产生任何错误信号（因为判断本身逻辑正确，只是输入不完整）。建议的防护：优先用
    单一写点/单一计数入口（#136 已采用 write_counter() 模式），或在新增通道时强制新增'收集闭合
    断言'（总计数==全集长度，#147 已采用该模式）。"
  decided_by: 蜂群内部（ws-rerun5 判别实验 + genealogist 模式提炼；meta-rule 辨认待编排者 /escalate）

# 被否定的方案
negated:
  description: "维持 commit b220f6341e 的自我归类——本次 sig_post 记账缺口是675号'探针走了与生产不同坐标系'的第三/第四实例。"
  why_negated: |
    (1) signals_dx==signals_prod 逐条对拍全程通过——探针数据源与生产完全一致，不存在坐标系分叉。
    (2) 真正缺陷是同一探针内部的计数字段（sig_post）未随新数据通道（PanDiv）同步扩展——这是"同
        坐标系内多写点未同步"，与675号"跨坐标系分叉"是不同机制（同构于633号"单坐标系内下标越
        界 vs 675号跨坐标系分叉"的既有区分逻辑）。
    (3) 与 #136 Stop-Guard熔断计数器（不同代码库、不同模块、无任何坐标系概念）同构，说明本模式
        的抽象层级高于675号——是"多写点计数器一致性"，不是"坐标系"问题。

# 新产出
new_output:
  definitions:
    - "多写点计数器口径分叉（multi-writepoint counter desync）：一个判断/门槛消费的计数值若由多个
      写点分别累加，当系统新增数据通道或重构变量时，若非所有写点被同步核查扩展，判断会基于残缺
      口径长期运行且不产生任何显式错误（逻辑本身正确，只是输入不完整）——与675号（跨坐标系分叉）
      正交，是同坐标系内部的多写点一致性问题。两次独立实例（#136/#147）达可辨认密度。"
    - "遮蔽链条的分层性（ws-rerun5 观察，方法论价值）：本次真跑暴露死门基线失败（外层）遮蔽真封①
      失败（内层）遮蔽 sig_post 记账漏计（根因）——断言失败可以分层遮蔽，外层断言的'通过'不能
      保证内层断言同样被真实评估过。rerun5『诚实重跑』的价值正是逐层剥开这类遮蔽，而非只看最外层
      断言是否绿灯。"
    - "死门三段漂移史（实测佐证）：(75,3150)[#137 dx-deadgate-update 初封]→(292,11602)[#142-#145
      修复序后判别实测]→(160,657)[686号裁C 后重封]——同一断言的基线随修复序推进持续漂移，须每次
      修复后重新校准基线，不能沿用上一版本的绝对数值。"
  code_changes: "无（本号是模式提炼谱系补记）。两个实例的具体修复已由各自 commit 落地（5457bb78dc / b220f6341e）。"
  orchestration_changes: "方法论：判断/门槛的输入若来自多写点计数器，系统扩展新数据通道时须核查所有写点——建议纳入 genealogist 常设巡检范围（同683号'else分支/TODO占位复核'并列的检索模式：新增通道/参数时检索所有消费该计数字段的判断点）。"

# 影响范围
impact:
  affected_modules:
    - "rust/src/theta_v0/backtest/econ_positive.rs、classifier/mod.rs、signal.rs（dx真封①,commit b220f6341e）"
    - ".claude/hooks/（Stop-Guard熔断计数器,commit 5457bb78dc，非本次任务范围，仅作同构实例引用）"
  affected_definitions:
    - "675号（settled）：本号明确排除本次 sig_post 缺陷计入675号实例范围，675号settled文件已同步追加排除说明，675号复发实例计数不受影响。"
  downstream_implications:
    - "若未来出现第三例（除#136/#147外）同构实例，本号 meta-rule 候选达到编排者辨认的实证密度阈值，建议直接触发 /escalate。"
    - "genealogist 常设巡检可将'新增数据通道/参数时核查所有消费该计数字段的判断点'纳入张力检查范围（同683号'else分支/TODO占位复核'并列）。"

# 回溯结算（如适用）
retroactive_settlement:
  settled_by: "编排者 /escalate 辨认 meta-rule 候选是否成立 → /ritual 显式化后，由 genealogist 回填 settled/。"
  settlement_date: null
  settlement_description: null

# 谱系关联
related_records:
  parent: "无直接父记录（首次提炼'多写点计数器口径分叉'模式）。"
  children: []
  related:
    - "675（探针必须走生产路径，已结算meta-rule）：姊妹但正交——675是跨坐标系分叉，本号是同坐标系内多写点未同步。本号明确排除 commit b220f6341e 计入675号实例范围（675号settled文件已追加排除说明）。"
    - "686（Q7-#1裁定C）：本号是686号实装后暴露的死门重封链的延伸发现（死门三段漂移史第三段即裁C后重封160/657）。"
    - "090（声明膨胀禁止）：commit b220f6341e 自我归类"675号实例"是一次轻度的声明不准确（非恶意膨胀，是判断仓促）——本号订正体现090号的持续适用性：即使是commit message的归因，也需要经得起谱系核实。"

# 认识论等级标注（formalization-validity-domain 231号，强制）
epistemological_levels:
  - proposition: "signals_dx==signals_prod 逐条对拍全程通过（300K窗），排除675号坐标系分叉可能"
    level: "L2（#147 ws-dxsig 真实数据对拍实测，可否证）"
    increment: "高：排除675号归类，定位真正机制"
  - proposition: "sig_post 计数器只计Γ二通道，PanDiv 通道未同步计入，差14（300K窗1431 vs 1445）"
    level: "L0（源码事实：sig_post 累加逻辑与#145新增PanDiv通道的时序关系）+ L2（真实数据差分验证）"
    increment: "高：根因定位"
  - proposition: "Stop-Guard熔断计数器（#136）与dx真封①（#147）共享'多写点计数器口径分叉'机制"
    level: "L0（两处代码结构事实：PRE_ACTIVE_TASKS/ACTIVE_TASKS 双写点 vs sig_post 单写点缺通道覆盖）"
    increment: "中：两次同构实例，达可辨认密度但未达三次以上的高置信阈值"
---

# meta-rule候选 687：多写点计数器在新增数据通道时口径分叉

## 结论

真封①违反的定性（ws-rerun5判别实验补发）：**既有潜伏，非686号裁定C引入**——父commit（`9b06a7265c`，
裁C之前）即已违反死门断言（sig_post_sum 1517 < n_signals 1534，差17；裁C后HEAD差14）。根因由
#147(ws-dxsig) 收口（commit `b220f6341e`）：**非生产bug**——`signals_dx==signals_prod` 逐条对拍全程
通过，差值 = #145 PanDiv 通道在 dx 探针的 `sig_post` 记账漏计（PanDiv push 进 signals_dx 但不进
sig_post 计数）。

**谱系归类订正**：commit `b220f6341e` 自称"675号实例"不准确。675号机制是跨坐标系分叉（探针数据源
≠生产数据源），本次探针与生产逐条对拍全程通过——不存在坐标系分叉。真正机制是**同一坐标系内，多
写点计数器未随新增数据通道同步扩展**，与 Stop-Guard 熔断计数器（task #136，commit `5457bb78dc`：
放行判断读 `PRE_ACTIVE_TASKS`、写点写 `ACTIVE_TASKS`，两变量口径分叉永不相等）同构——两次独立实例，
足以提炼为独立于675号的 meta-rule 候选。

## 遮蔽链条（方法论价值，ws-rerun5观察）

死门基线失败（外层）遮蔽真封①失败（内层）遮蔽 sig_post 记账漏计（根因）——断言失败可以分层遮蔽，
rerun5「诚实重跑」逐层剥开。死门三段漂移史：`(75,3150)[#137]→(292,11602)[#142-#145后判别实测]→
(160,657)[686号裁C后]`——同一断言基线随修复序推进持续漂移，每次修复后须重新校准，不能沿用上一版本
绝对数值。

## 元规则（候选措辞）

> 若一个判断/门槛消费的计数值由多个写点/字段分别累加，当系统新增数据通道或重构变量命名时，必须
> 同步核查所有写点/消费点是否已扩展覆盖新通道——否则判断会长期基于残缺口径而不产生任何错误信号
> （因为判断本身逻辑正确，只是输入不完整）。防护：优先用单一写点/单一计数入口，或在新增通道时
> 强制新增"收集闭合断言"（总计数==全集长度）。

## 边界条件（翻转）

- 若出现第三例同构实例，本号达到编排者辨认的实证密度阈值，建议直接触发 `/escalate`。
- 若后续分析发现本次 sig_post 缺陷实际上也存在坐标系层面的分叉（当前证据不支持，逐条对拍全程
  通过），则本号排除675号归类的判断需回退。

## 影响声明

谱系补记，零代码改动归属本号（两处修复已分别由 commit `5457bb78dc`/`b220f6341e` 落地）。675号
settled文件已同步追加排除说明，675号复发实例计数不受本号影响。meta-rule候选辨认待编排者 `/escalate`。
