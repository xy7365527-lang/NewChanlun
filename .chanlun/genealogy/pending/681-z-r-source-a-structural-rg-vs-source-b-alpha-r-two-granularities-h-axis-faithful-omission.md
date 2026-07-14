---
id: "681"
number: 681
status: 生成态   # codex b1 审计裁定"P0 不补 H"（选择类全权裁定）；概念分离已辨认。回溯结算待编排者 /ritual。
date: "2026-07-02"
type: 概念分离   # z.r 分离为源 A（结构唯一性层 R(g)）与源 B（收益估计层 r）两个不同粒度定义。
source: "[新缠论] codex-b1-audit-20260702.md §H轴裁定 + §H轴裁定核实 + codex-lines-port-plan-20260702.md §4（Round137 形式输入）"
negation_source: heterogeneous
negation_model: "codex-cli（b1 审计：不补 H 进 MuClass 的 P0）"
negation_form: separation   # 统一符号 z.r 内部暴露两个不兼容粒度：源 A 含 H（R(g)=(H,V,δ)18类）/ 源 B 不含 H（r=操作角色枚举）。
depends_on: ["680", "615", "640", "639"]
related: ["616", "617", "656", "637"]

# 拓扑效果标注（147号下游推论3）
# negates：680 §2.3 把"H 轴进 z"框为纯选择类（补维完备 vs 样本经济）；隐含"z.r 是单一定义、H 轴缺失=疏漏或纯价值偏好"
# 实际拓扑后果（retrospective 141号结论1）：z.r 单一定义被切断，分裂为两条独立路径——
#   源 A（结构唯一性/递归分类层 R(g)，含 H）与源 B（收益估计/alpha 层 r，不含 H）；
#   代码（MuClass）连源 B，H 轴缺失是忠实实装源 B 的结果，非疏漏。scope=local（z.r 定义本身）。
topo_effect: "sever:z-r-single-definition:local"

# 概念分离（type=概念分离 必填）
separation:
  before: "z 状态向量的 r 分量被当作单一定义。680 §2.3 把'H 轴是否进 z'框为选择类（补维完备 vs 样本经济），隐含 H 缺失是价值权衡下的取舍或疏漏。"
  after:
    - name: "源 A — 结构唯一性层 R(g)=(H,V,δ) 18 类（含 H）"
      definition: "递归完全分类的结构角色：H(水平·同父前兄弟关系 First/SameFollow/SameReverse)×V(垂直·Ambient/FollowParent/ShortDiff)×δ = 3×3×2 = 18 类。是递归分类器的精确纤维——Round137 exactFiberIffAxisEquiv+mutuallyExclusive+uniqueClass 证纤维恰为 H/V/δ 三轴，去掉 H 就不是那个精确分类器。"
      source: "递归完全分类买卖点.pdf §7-8 / P6-P7；形式桥 formal/Origin/Round137（codex/complete-classification-origin 线，cite-not-merge）"
    - name: "源 B — 收益估计层 r（操作角色枚举，不含 H）"
      definition: "alpha z 定义的直接出处的 r 分量：操作角色枚举（Ambient/FollowParent/ShortDiff 的 V 轴语义 + parent_dir/position），用于按状态分桶估收益 μ(z,a)。不含 H——H 是结构关系非操作极性。"
      source: "推导完全分类.pdf（z 定义直接出处）；代码 mu_estimator.rs:54（MuClass '六维全互斥'，按源 B 的 z 实现）"
  pending_verification: "H 对 μ 是否有 OOS/LCB/置换去污后稳定增量（L2/L3）——若有，源 B 应补 H 向源 A 收敛；若无，源 B 的 6 维是收益估计层的正确粒度，H 留在源 A 结构层。"

# 涉及的定义
definitions_involved:
  - name: "680 分类层算出≠alpha消费（231 消费侧形态）"
    version: ".chanlun/genealogy/pending/680（生成态）"
    role: "直接前置。680 §2.3 把 H 轴框为选择类；本号精化：H 缺失不是纯选择，是 z.r 两粒度源分离下忠实实装源 B 的结果。重构 680 的 H 轴框架。"
  - name: "MuClass（六维全互斥 z 载体）"
    version: "rust/src/theta_v0/backtest/mu_estimator.rs:54"
    role: "忠实实装源 B。代码从未声称是源 A 的 R(g) 载体，H 轴缺失是设计忠实性非疏漏。coverage.rs 的 H 判别函数已正确算出 H，只是不接入 z（源 B 不需要）。"
  - name: "615 完全分类 Layer1⊊Layer2 / 640 函数唯一性 vs 结构唯一性"
    version: ".chanlun/genealogy/pending/615 + settled/640"
    role: "同族。源 A（结构唯一性层）是 615 Layer2 / 640 结构唯一性的角色维；源 B（收益估计层）是 alpha 消费粒度。两层粒度分离与 615 的 Layer1/Layer2、640 的函数/结构唯一性同构。"

# 解决方式
resolution:
  type: 概念分离   # 可分层解决（源 A=结构唯一性/递归分类层，源 B=收益估计/alpha层），非不可弥合矛盾——不触发中断#1。
  description: "z.r 分离为源 A（结构唯一性层 R(g)18类，含 H）与源 B（收益估计层 r，6维不含 H）。代码忠实实装源 B。codex b1 审计裁定：P0 不补 H，MuClass 维持 6 维。H 轴缺失是忠实源 B 的结果，非疏漏。"
  decided_by: 蜂群内部   # codex b1 审计（选择类全权裁定）；最终辨认待编排者 /ritual。

# 被否定的方案
negated:
  description: "把 z.r 当单一定义，H 轴缺失视为纯选择类取舍（680 §2.3）或实装疏漏。"
  why_negated: "z.r 有两个不同粒度的源定义：源 A（递归完全分类买卖点.pdf，R(g)含H，结构唯一性层）vs 源 B（推导完全分类.pdf，r无H，收益估计层）。代码按源 B 实现，H 缺失是忠实结果。Round137 形式桥确认源 A 含 H（纤维=H/V/δ），但形式桥是'目标规定'非'实装证据'——把它当'H 已实装'会声明膨胀（090/231）。故 H 轴问题不是单一定义下的疏漏，是两粒度源分离下的层归属问题。"

# 新产出
new_output:
  definitions:
    - "z.r 两粒度源分离：源 A（结构唯一性层 R(g)=(H,V,δ)18类，含 H）⊇ 源 B（收益估计层 r，6维不含 H）。粒度差=H 轴。"
    - "忠实缺失判据：某维在代码中缺失，须先核它属哪个源粒度——若代码忠实实装的源不含该维，缺失是忠实结果非疏漏，不构成实装缺口。"
  code_changes: "无（codex 裁定 P0 不补 H，MuClass 维持 6 维）。若翻转（见边界条件），MuClass 加 horizontal:Horizontal 字段，桶键升 3×54 类。"
  orchestration_changes: "方法论：审计'某维缺失'须先定位它属哪个源粒度层，再判是否忠实。归 knowledge-crystallization skill 候选（与 680 的'消费侧核对'判据配套）。"

# 影响范围
impact:
  affected_modules:
    - "MuClass（mu_estimator.rs:54）——维持 6 维（源 B 粒度），不补 H。"
    - "680 §2.3（H 轴选择类框架）——重构为源 A/源 B 粒度分离 + 忠实源 B。"
    - "任何'完全分类含 18 类 R(g)'的 alpha 声明——须区分声明的是源 A（结构）还是源 B（收益估计）粒度。"
  affected_definitions:
    - "z=(ℓ,δ,I_γ,r,σ_p,…) 的 r 分量——澄清为源 B 收益估计粒度（6维），非源 A 结构粒度（18类含H）。"
  downstream_implications:
    - "b2 实装（task #5）按源 B 6 维接入 MuClass 桶键，不待 H 裁定。"
    - "H 轴裁定翻转条件明确：更晚权威文档写 z.r=R(g)（源 A/B 实为同概念不同阶段），或 H 对 μ 有 L2/L3 稳定增量证据。L0 形式桥（Round137）两者都不是——它规定目标（z 应含完整 R(g)），不证 H 实际改变 μ/决策。"

# 谱系关联
related_records:
  parent: "680（分类层算出≠alpha消费）——本号精化其 H 轴框架。"
  children: []   # H 对 μ 的 L2/L3 增量验证若产出，派生实证子记录。
---

# 概念分离 681：z.r 的源 A（结构唯一性 R(g)）⊇ 源 B（收益估计 r）——H 轴缺失是忠实结果

## 结论

z 状态向量的 r 分量有**两个不同粒度的源定义**：**源 A**（`递归完全分类买卖点.pdf`，R(g)=(H,V,δ) 18 类，
结构唯一性/递归分类层，**含 H**）⊇ **源 B**（`推导完全分类.pdf`，r=操作角色枚举，收益估计/alpha 层，
6 维**不含 H**）。代码（`MuClass`，mu_estimator.rs:54"六维全互斥"）**忠实实装源 B**——**H 轴缺失是
忠实实装源 B 的结果，非疏漏**。codex b1 审计裁定：P0 不补 H，`MuClass` 维持 6 维。

## 与 680 的关系（重构 H 轴框架）

680 §2.3 把"H 轴是否进 z"框为**选择类**（补维完备 vs 样本经济）。本号精化：H 缺失不是单一定义下的
纯价值取舍，而是**两粒度源分离下的层归属问题**。`coverage.rs` 的 H 判别函数已正确算出 H，只是
不接入 z——因为源 B（收益估计层）不需要它。

## 形式桥不翻转裁定（声明膨胀防护）

Round137（`formal/Origin/Round137`，codex/complete-classification-origin 线，cite-not-merge）证完全互斥
关系分类器**纤维恰为 H/V/δ 三轴**——即源 A 的 R(g) **内在含 H**。但这是 **L0 形式桥 = 目标规定，非
实装证据**。裁定翻转需两者之一（皆非 L0 形式桥可满足）：

1. 更晚权威文档明确写 `z.r = R(g)=(H,V,δ)`（即源 A/B 实为同一概念不同阶段表述，B 是 A 的简化误记）；
2. H 对 μ 在 OOS/LCB/置换去污后有稳定增量的 L2/L3 证据。

把 Round137 当"H 已实装"会声明膨胀（090/231）——形式桥自我限定"非当前引擎满足假设、非 Rust bit-extraction"。

## 定义依据

源 A：递归完全分类买卖点.pdf §7-8 + Round137 exactFiberIffAxisEquiv/mutuallyExclusive/uniqueClass。
源 B：推导完全分类.pdf（z 定义直接出处）+ mu_estimator.rs:54（MuClass 六维全互斥）。

## 边界条件（翻转）

见"形式桥不翻转裁定"两条件。b2 实装若在裁定前进行，按当前 6 维（源 B）接入即可。

## 下游推论

- b2 实装（task #5）按源 B 6 维接入，不待 H 裁定。
- 新判据（结晶候选）：审计"某维缺失"须先核它属哪个源粒度——代码忠实实装的源不含该维时，缺失=忠实结果非缺口。
- 与 680 判据配套：680=消费侧须核桶键是否吃该维；681=须核该维属哪个源粒度层。

## 谱系引用

- 前置：680（消费侧形态）← 本号精化其 H 轴框架为源 A/源 B 分离。
- 同族：615（Layer1⊊Layer2）/ 640（函数唯一性 vs 结构唯一性）——源 A=结构唯一性层，源 B=收益估计层，粒度分离同构。
- 相邻：639（σ_p 源=父容器方向）、616/617（分类 θ 参数化）、656（V(Z)≥V(Y) 表达力）。

## 影响声明

纯谱系记录，零 git，零代码改动。裁定结果：MuClass 维持 6 维（源 B），P0 不补 H。不改 coverage.rs（H 判别已正确，只是不接入 z）。
认识论如实：源 A 含 H（形式确认，Round137 L0），源 B 不含 H（代码忠实），H 对 μ 的 L2/L3 增量未验——形式桥不替代经验验证。
