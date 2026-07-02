---
id: "682"
number: 682
status: 生成态   # meta-rule 从 #4 盘点单例产出；回溯结算待编排者 /ritual 及后续同构实例复现。
date: "2026-07-02"
type: meta-rule   # "别的线优化了算法可拿来用"须核目标文件存在性——分叉线可能整个缺失被优化对象。
source: "[蜂群方法论] codex-lines-port-plan-20260702.md §0/§1/§5（task #4 盘点）"
negation_source: homogeneous   # #4 工位 git 存在性核对（git cat-file/ls-tree/diff --stat）
negation_form: separation   # "移植增量"分离为算法层增量（=∅，目标文件缺失）vs 形式层增量（72 Round桥，仅可引用）。
depends_on: ["675", "231"]
related: ["090", "659", "623", "680"]

# 拓扑效果标注（147号下游推论3）
# negates：编排者假设"算法在别的线优化了，你可以拿去用"（隐含分叉线含被优化的目标文件）
# 实际拓扑后果（retrospective 141号结论1）：该假设被切断与"算法层可移植"的连接，重建为——
#   算法层拿零（三条 codex 线 econ_positive.rs 计数=0，一条不含该文件的分支无法优化它）
#   + 形式层真增量（72 个 Round*.lean 形式桥，cite-not-merge）。优化实际发生在 HEAD 自身 426 提交里。
#   scope=local（本次盘点的三条 codex 线）。
topo_effect: "sever:optimization-portable-from-fork-assumption:local"

# 矛盾（meta-rule 的被订正假设）
contradiction:
  description: |
    编排者指示"算法在别的线优化了，你可以拿去用"，隐含分叉线含被优化的目标文件。
    #4 核对：三条 codex 线（complete-classification-origin / strict-formal / codex-line-20260625）的
    alpha 回测引擎文件 econ_positive.rs **计数全为 0**（git ls-tree 核过），而真缺口整个落在
    rust/src/theta_v0/backtest/econ_positive.rs（alpha 桶键丢弃 role/σ_p，见 680）。
    **一条不含该文件的分支无法优化该文件**——rust 侧拿零。优化实际发生在 HEAD 自身的 426 提交里，
    不在分叉线。唯一真增量 = formal/Origin/Round130-137（72 文件依赖链）——增量在**形式层**不在**算法层**。
  layer: 编排   # 移植决策层（"从别的线拿算法"的假设核对）。
  trigger: "#4 盘点 Downloads 两条 codex 线的算法增量——git 存在性核对翻转任务假设。"

# 涉及的定义
definitions_involved:
  - name: "675 探针须穿越生产路径（无坐标分叉）"
    version: ".chanlun/genealogy/settled/675"
    role: "姊妹 meta-rule。675=验证/探针须核实际生产路径，不在分叉坐标上验证。本号=移植增量须核目标文件存在性，不在缺失被优化对象的分叉线上声明算法增量。同族：声明须核实际对象存在性/路径归属。"
  - name: "231 形式化有效域规则 / 090 声明膨胀禁止"
    version: ".chanlun/genealogy/settled/231 + .claude/rules/no-patch-mentality.md"
    role: "约束。把形式桥（Round133/137 自我限定'非当前引擎满足假设、非 Rust bit-extraction'）当算法实装移植=声明膨胀。形式桥=目标规定，非实装证据。"

# 解决方式
resolution:
  type: 定义修正   # 订正移植假设；产出 meta-rule。
  description: |
    "别的线优化了算法可拿来用"的声明，须先核该线含不含目标文件——不含则算法层拿零，
    优化实际在别处（本例 HEAD 自身）。分叉线的真增量可能整个落在形式层（Round 桥），
    此时对目标以"引用而非合并"（cite-not-merge）消费，物理合并算法零收益且有回退风险。
  decided_by: 蜂群内部   # #4 git 存在性核对；最终辨认待编排者 /ritual。

# 被否定的方案
negated:
  description: "假设分叉线含被优化的目标文件，直接把分叉线 rust diff 当算法优化移植进 HEAD。"
  why_negated: |
    (1) 三条 codex 线 econ_positive.rs 计数=0（git 核）——不含目标文件的分支无法优化它。
    (2) 分叉线 rust 是分叉前旧引擎（无 alpha 引擎），HEAD 是超集——误移植会**回退** HEAD 的 alpha 引擎（降级）。
    (3) 真增量在形式层（72 Round 桥），物理合并须带整棵 Origin Round 子树 + lakefile 改 + Lean build 风险，
        而 cite-not-merge 零成本零风险达成对 task #1 的形式依据消费。

# 新产出
new_output:
  definitions:
    - "移植增量归属判据（meta-rule）：'别的线优化了 X'的声明，须先 git 核该线含不含 X 目标文件。不含 ⟹ 算法层增量=∅，优化实际在别处；真增量可能整个在形式层（此时 cite-not-merge，物理合并算法零收益+回退风险）。"
    - "增量层区分：算法层增量（可移植的引擎/代码 diff）vs 形式层增量（Lean 形式桥=目标规定，引用消费非合并）。混淆二者 = 声明膨胀（把形式桥当实装）。"
  code_changes: "无（rust 拿零；Round130-137 对 task #1 cite-not-merge，不驻留 HEAD 除非编排者明示作独立 formal PR）。"
  orchestration_changes: "方法论结晶：移植/优化港移指令的前置核对=目标文件存在性 + 增量层归属。归 knowledge-crystallization skill 候选。"

# 影响范围
impact:
  affected_modules:
    - "econ_positive.rs（backtest/）——三条 codex 线均无此文件，P0 接入只能在 HEAD/main 独立实装（与 codex 线无关）。"
    - "formal/Origin/Round130-137——对 task #1 cite-not-merge；物理合并须整棵 Origin Round 子树（72 文件）+ lakefile + Lean build 护航，独立 PR。"
  affected_definitions: []
  downstream_implications:
    - "任何'从 codex 线拿算法'的后续指令须先核该线含不含目标文件——本盘点证明'别的线优化了算法'在 rust 层不成立（那些线无 alpha 引擎），优化实际发生在 HEAD 自身的 426 提交里。"
    - "若未来做 formal 链物理合并，须带溯源 pasted-text.txt（strict-formal 线，1620 行完全分类原文）保 Round 桥 sourceBacked 可回溯。"

# 谱系关联
related_records:
  parent: "675（探针须穿越生产路径）——同族 meta-rule（声明须核实际对象存在性/路径归属）。"
  children: []
---

# meta-rule 682：移植增量归属须核目标文件存在性——分叉线可能整个缺失被优化对象

## 结论

"别的线优化了算法，你可以拿去用"的声明，**须先核该分叉线含不含目标文件**。本例三条 codex 线
（complete-classification-origin / strict-formal / codex-line-20260625）的 alpha 回测引擎 `econ_positive.rs`
**计数全为 0**（git ls-tree 核过）——**一条不含该文件的分支无法优化该文件**，rust 侧拿零。优化实际发生在
HEAD 自身的 426 提交里，不在分叉线。唯一真增量 = `formal/Origin/Round130-137`（72 文件依赖链）——
**增量在形式层不在算法层**，对 task #1 以**引用而非合并**（cite-not-merge）消费。

## meta-rule

移植/港移指令的前置核对两步：
1. **目标文件存在性**：`git ls-tree` 核分叉线含不含被优化对象。不含 ⟹ 算法层增量=∅。
2. **增量层归属**：真增量在算法层（可移植 diff）还是形式层（Lean 形式桥=目标规定）。形式层增量 cite-not-merge——
   物理合并算法零收益 + 回退风险（误把分叉前旧引擎当优化=降级 HEAD alpha 引擎）。

混淆两层 = 声明膨胀：把形式桥（Round133/137 自我限定"非当前引擎满足假设、非 Rust bit-extraction"）当算法实装（090/231）。

## 定义依据

git 存在性：三线 econ_positive.rs count=0 vs HEAD/main=1（§1 拓扑总账）；分支独有提交 216/1，HEAD 独有 426/427；
Round130-137=8 文件（依赖 Round1-129 共 72）。形式桥自我限定 docstring（§3/§5.4 风险表）。

## 边界条件（翻转）

若编排者要求形式链物理驻留 HEAD ⟹ 从"引用"升为"合并整棵 Origin Round 子树"，风险从零升为 Lean build。
若 task #1 codex 裁 H 对 μ 有 L2 贡献 ⟹ Round137 从"参考"升为"必须实装 H 进 MuClass"的形式依据（见 681）。

## 下游推论

- "从 codex 线拿算法"的后续指令须先核目标文件存在性——"别的线优化了算法"在 rust 层不成立。
- formal 链合并须带溯源 pasted-text.txt 保 Round 桥 sourceBacked。

## 谱系引用

- 姊妹：675（探针须穿越生产路径）——声明须核实际对象存在性/路径归属。
- 约束：231/090（形式桥当实装=声明膨胀）。
- 相邻：659（滤波器谱系跨引擎范畴错误）、623（231 自指工具层）、680（econ_positive.rs 消费侧缺口=本次盘点的目标对象）。

## 影响声明

纯谱系记录，零 git，零代码改动。rust 拿零；Round130-137 对 task #1 cite-not-merge。不改任何 alpha 文件、不合并任何分支。
