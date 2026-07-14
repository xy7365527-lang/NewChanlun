---
id: "680"
number: 680
status: 生成态   # P0 接入未实装（task #1 codex 审计 b1 P0 接入设计进行中）；接入落地后回溯结算。最终辨认待编排者 /ritual。
date: "2026-07-02"
type: bias-correction   # 订正"回测已验证完全互斥分类 alpha"的声明膨胀 + 订正 A 组报告三条"原文有我们无"的 stale 判定。
source: "[新缠论] b1 工位 full-mutex-b1-design-20260702.md §5 明确请求结晶 + docs/formal-chain/ 原文 z 状态 + 代码核对 rust/src/theta_v0/"
negation_source: homogeneous   # b1 工位逐维对照代码核对（coverage.rs / mu_estimator.rs / selector.rs / econ_positive.rs 锚点）
negation_form: expansion   # 规定者在执行中违反自身规定：分类层声明并算出完全互斥 Z 状态，但 alpha 回测消费侧只吃 Y 粗投影——"完全互斥分类 alpha"声明膨胀于实际消费的 Y 有效域。
depends_on: ["231", "625", "656", "645"]
related: ["230", "623", "615", "639", "667"]

# 拓扑效果标注（147号下游推论3：negates 非空必填）
# negates：声明"alpha 回测验证了完全互斥分类（Z 细状态）"——实为 Y 粗投影 alpha
# 实际拓扑后果（retrospective 141号结论1）：alpha 回测节点分裂——
#   一支保留原声明（意图 Z 完备：z=(ℓ,δ,I_γ,σ_p,r,…)），
#   一支携带违反记录（实际 Y 粗投影：Y=(ℓ,δ,bsp_class)，econ_positive.rs 桶键丢弃 role/σ_p）。
#   下游所有"回测验证了完全互斥 alpha"声明继承此分裂，直到 P0 接入落地解除。
topo_effect: "split:complete-mutex-alpha-backtest-claim:downstream"

# 矛盾（bias-correction 的被订正对象）
contradiction:
  description: |
    分类层（coverage.rs:758-849）已完整算出原文 z 的全部 P0 维——角色 r=R(g) 18 类
    （Horizontal/Vertical/OperationRole，带 spec §7/§8 锚点）、父声部 σ_p（parent_sign 从 V(g) 推）、
    短差 ShortDiff；状态层 MuClass（mu_estimator.rs:68-110）承载 z 的 parent_dir/short_swing/position；
    统计路径桶键（wverify_run.rs:83、perm_test.rs:29）已含 σ^H 分层。
    **但 alpha 回测路径 econ_positive.rs 的信号元组（:222 collect_signals）与分桶键
    （:2070/2318/2729/2823 全部三元键 (level,delta,bsp_class)）丢弃已算出的 role/σ_p**——
    assemble_gamma_with_tower 返回的 c.role（:291 可读）被就地丢弃。
    结果：alpha 回测跑在粗投影 Y=(ℓ,δ,I_γ) 上，而非原文细状态 Z=(ℓ,δ,I_γ,σ_p,r,…)。
    "回测验证了完全互斥分类 alpha"的声明，实为 Y 粗状态的 alpha。这是编排者点破
    "回测跑在未完全实装的全互斥定义策略上"的代码级字面机制。
  layer: 代码   # 消费侧实装缺口（非定义冲突：分类层定义与实装均正确，缺的是 alpha 路径消费）。
  trigger: "b1 工位逐维对照定案——把 A 组报告（#70 纯读 PDF 未核代码）三条'原文有我们无'判定核为 stale，收敛到真缺口=消费侧桶键丢弃已算维度。"

# 与 231 家族的关系（bias-correction 的定位）
definitions_involved:
  - name: "231 形式化有效域规则（有效域 ≤ 定义域）"
    version: ".chanlun/genealogy/settled/231 + .claude/rules/formalization-validity-domain.md"
    role: "母规则。本号是 231 在实装消费路径的具体形态：分类层声明 Z 完备（定义域），alpha 回测消费 Y 粗投影（有效域）——有效域<定义域，声明膨胀。"
  - name: "625 L2 真实数据暴露 L1 合成掩盖的引擎 bug（231 引擎回测层活体实例）"
    version: ".chanlun/genealogy/settled/625"
    role: "姊妹实例，须区分。625=**计算侧错误**（引擎算错，被合成数据掩盖）。本号=**消费侧丢弃**（分类层算对，alpha 路径不消费）。231 在实装层分裂为两个不同形态：计算侧 bug（625）/ 消费侧 discard（680）。"
  - name: "656 细分类不劣 V(Z)≥V(Y)（645 对偶正面）"
    version: ".chanlun/genealogy/settled/656"
    role: "定理前件。656 证细分类的表达力优势 V(Z)≥V(Y)（L0 结构定理）。本号=该优势在实装未被兑现的机制——回测未真跑在 Z 上，故 V(Z)≥V(Y) 的 oracle 上界当前未被 alpha 数字反映。"
  - name: "645 覆盖≠盈利（π^cov 8/8 否证，wrong object）"
    version: ".chanlun/genealogy/settled/645"
    role: "相邻。645 是分类投影 vs bsp timing 的对象错配。本号是同一 econ_positive.rs 路径的另一维度丢弃（role/σ_p），与 645 正交但同属'声明的分类对象 ≠ 回测实际消费的对象'。"

# 解决方式（P0 接入设计）
resolution:
  type: 定义修正   # 非概念分离——修正实装消费侧，使回测状态从 Y 升到 Z。接入=复用既有桥，零新逻辑。
  description: |
    P0 接入 = 让 econ_positive.rs 复用两个已存在的桥：selector.rs:145-155（role→parent_dir 规范映射）
    + MuClass::from_certificate（mu_estimator.rs:92，已能吃 (level,delta,bits,parent_dir,position) 产完整 z）。
    最小 diff：(1) collect_signals 信号元组扩展携带 parent_dir/position（或直接携 OperationRole）；
    (2) 分桶键 (level,delta,bsp_class) 三元键 → 改用 MuClass 作 key（MuClass 已 Eq+Hash，是原文 z 的既有载体，
    不重造 z 的 Hash 键）；(3) dx 守恒断言从 Y 扩到 Z（分划细化保 Σ：Σ_{Z桶}pnl == Σ_{Y桶}pnl）。
    高维 z 桶稀疏时走既有 UClass 降维层（project_to_u，mu_estimator.rs:161）——Z 桶=oracle 上界 / U 桶=抗过拟合估计。
  decided_by: 蜂群内部   # b1 设计稿；待 task #1 codex 审计后实装。H 轴是否进 z 是独立选择类（上浮 codex）。

# 被订正的方案
negated:
  description: "A 组报告（#70）判定'原文有角色轴/σ_p/短差、我们无'（三条'缺失'）；隐含声明'当前 alpha 回测已验证完全互斥分类'。"
  why_negated: |
    (1) A 报告三条"缺失"判定 stale：role/σ_p/短差**已实装于分类层 + 状态层 + 统计桶键**（代码锚点核过），
    真缺口比 A 报告窄——不是"没有角色维"，是"角色维已算出但 alpha 桶键不消费"。
    (2) "alpha 已验证完全互斥分类"是声明膨胀（090/231）：桶键为 Y 三元键，当前 alpha 数字是 Y 粗状态的 alpha，
    非原文 Z——按 656 定理补维只增不减 oracle，但需真接入 Z 桶才兑现（现未接入）。

# 新产出
new_output:
  definitions:
    - "231 实装层双形态区分：计算侧 bug（625，算错被掩盖）/ 消费侧 discard（680，算对不消费）——同母规则在实装的两个不同 locus。"
    - "消费侧有效域缺口判据：判别维已在分类层实装 ≠ 回测消费它。核对分类层实装状态不足以断言 alpha 有效域=定义域，须核**消费侧桶键/信号元组**是否真吃该维。"
  code_changes: "待实装（task #1 codex 审计后）：econ_positive.rs 信号元组 + 分桶键接入 role/σ_p（复用 selector.rs + MuClass）+ dx 守恒断言 Y→Z 扩展。不改 coverage.rs/mu_estimator.rs（已实装正确）。"
  orchestration_changes: "方法论结晶候选：'分类层实装状态'与'回测消费状态'是两个独立核对维度——审计完全性声明须同时核两侧，只核分类层=漏消费侧有效域缺口。归 knowledge-crystallization skill 候选。"

# 影响范围
impact:
  affected_modules:
    - "econ_positive.rs（消费侧：信号元组 :222 + 桶键 :2070/2318/2729/2823 + dx 守恒断言 :2292/2318）——待接入 role/σ_p。"
    - "W-VERIFY(#13/#51) 与 econ_positive.rs 两条路径桶键统一到 Z=MuClass——消除'统计路径含 σ^H、alpha 路径不含'的分叉。"
    - "任何'回测验证了完全互斥分类 alpha'的声明——须核 econ_positive.rs 桶键（现为 Y 桶），当前 alpha 是 Y 粗状态 alpha。"
  affected_definitions:
    - "z=(ℓ,δ,I_γ,r,σ_p,…) 状态元组（权威全本①②）——实装 alpha 路径当前只投影到 Y=(ℓ,δ,I_γ)。"
    - "H 轴（Horizontal，coverage.rs:849 算出）从未进 MuClass z——独立选择类（补维完备 vs 样本经济），交 codex 裁（task #1）。"
  downstream_implications:
    - "接入后 MuClass 维数由 H 轴裁定定型（6 vs 7），是 goal g-full-mutex-impl 后续所有分桶断言的前置。"
    - "codex 矛盾-B 裁定（level0 免门=有限塔有效域边界，禁宣称 theta_v0 全域 C_Θ 完全实现，codex-decide-20260702-213839-7db4.md）是 231 消费侧/有效域边界的姊妹实例——同轮张力碰撞坐实 231 在实装层的两个 locus：消费侧 discard（本号）+ 级别域截断（矛盾-B）。"

# 谱系关联
related_records:
  parent: "231（形式化有效域规则）——本号是其实装消费路径形态。"
  children: []   # P0 接入落地后可派生'接入后 Z 桶稀疏→UClass 降维'的实证子记录。
---

# bias-correction 680：分类层算出 ≠ alpha 回测消费——231 的消费侧形态

## 结论

原文完全互斥分类的 P0 维（角色 r=R(g) 18 类、父声部 σ_p、短差 ShortDiff）**已实装于分类层
（coverage.rs:758-849，带 spec §7/§8 + 231 号 L0/L2 锚点）+ 状态层 z（MuClass）+ 统计桶键
（wverify/perm_test）**。真缺口 = **alpha 回测路径 econ_positive.rs 的信号元组与分桶键仍是
粗投影 Y=(ℓ,δ,bsp_class)，丢弃已算出的 role/σ_p**——即编排者点破"回测跑在未完全实装的
全互斥定义策略上"的代码级机制。这是 231（有效域<定义域）在**实装消费路径**的具体形态：
分类层声明 Z 完备（定义域），alpha 回测消费 Y 粗投影（有效域）。

## 与 231 家族的区分（本号的谱系增量）

231 在实装层已有一个活体实例 625（L2 真实数据暴露 L1 合成掩盖的引擎 bug）。本号与 625 是
**同母规则 231 的两个不同 locus**：

| | 625 | 680（本号） |
|---|-----|-----------|
| locus | **计算侧** | **消费侧** |
| 分类层/引擎 | 算**错**（bug），被 L1 合成数据全绿掩盖 | 算**对**（coverage.rs 带 spec 锚点核过） |
| 缺口 | L2 真实数据才否证出的引擎 bug | alpha 桶键丢弃已正确算出的维度 |
| 修复 | 修引擎 bug（工程实现层） | 接入消费侧（复用既有桥，零新逻辑） |

关键判据（新产出）：**核对分类层实装状态不足以断言 alpha 有效域=定义域，须核消费侧桶键/
信号元组是否真吃该维。** 只核分类层 = 漏消费侧有效域缺口。

## 定义依据

原文 z=(ℓ,δ,I_γ,r,σ_p,…)（权威全本①②）、R(g)=(H,V,δ) 18 类（递归完全分类买卖点.pdf §7-8）、
细分类优势定理 §13（V(Z)≥V(Y)，656 号）；代码锚点 coverage.rs:758-849（role 判别函数全实装）、
mu_estimator.rs:68-110（MuClass z 载体，缺 H 轴）、selector.rs:145-155（role→parent_dir 桥）、
econ_positive.rs:222/291/2070（signal 元组 + 桶键丢弃 role 的实证）、wverify_run.rs:83（统计路径已含 σ^H，证明桥可用）。

## 边界条件（结论翻转）

- (a) 若核实 econ_positive.rs 某处已按 role 分桶（本核对未见）⟹ 缺口消解为"仅文档滞后"，本号降为已结算无改动。
- (b) 若 codex 裁定 H 轴对 μ 无 L2 贡献且应删 ⟹ z 定型为 6 维（现 MuClass），H 分叉作废，只接入 V 轴+σ_p。
- (c) 若接入后 Z 桶普遍 n̄<大样本下界 ⟹ alpha 报告走 UClass 降维层，Z 桶键仅作 oracle 上界不作估计。

## 下游推论

- 任何"回测验证了完全互斥分类 alpha"的声明须核 econ_positive.rs 桶键——现为 Y 桶，故当前 alpha 数字是**粗状态 Y 的 alpha**。
- 接入后 W-VERIFY 与 econ_positive.rs 两条路径桶键统一到 Z=MuClass，消除"统计路径含 σ^H、alpha 路径不含"的分叉。
- H 轴裁定影响 MuClass 维数（6 vs 7），是 goal g-full-mutex-impl 后续所有分桶断言的前置。
- 同轮姊妹实例：codex 矛盾-B（level0 免门=有限塔有效域边界，非全域平移自相似公理实现）是 231 在**级别域截断**的形态——本号（消费侧 discard）+ 矛盾-B（级别域截断）同轮碰撞，坐实 231 在实装层的两个 locus。

## 谱系引用

- 母规则：231（形式化有效域）← 本号为实装消费路径形态。
- 姊妹：625（计算侧 bug）/ 623（自指工具层）/ 矛盾-B 裁定（级别域截断，codex-decide-20260702-213839-7db4.md）——231 的四个不同 scope。
- 定理前件：656（V(Z)≥V(Y) 表达力优势）← 本号=该优势在实装未兑现的机制。
- 相邻：645（覆盖≠盈利 wrong object）、639（σ_p 源=父容器方向）、667（level phase alpha vs fullsample beta 投影）。

## 影响声明

纯谱系记录，零 git，零代码改动。待实装（task #1 codex 审计后）：econ_positive.rs 信号元组 + 分桶键
接入 role/σ_p（复用 selector.rs + MuClass）+ dx 守恒断言 Y→Z 扩展 + H 轴进 z 的 codex 裁定。
不改 settled 定理，不改 coverage.rs/mu_estimator.rs（已实装正确），只改 econ_positive.rs 消费侧。
认识论如实：role/σ_p 分类层实装为真，alpha 桶键未接入为真，当前回测 alpha 是 Y 粗状态的（231 有效域实例）。
