---
id: '542'
number: 542
title: "配额比例 σ-不变（降成本 spawn 仓位分配的几何必然）——释放给子 voice 的配额 f=m/p_units 必须级别无关（σ-不变常数），由 T48（units=σ-不变 Casimir）+ T59（自相似 σWσ⁻¹=W）+ T23（递归 step-replication）强制；「势∝r」是径向坐标 r 的定义本身（r 标记级别=递归深度=势能，L0，信息增量为零不可经验否定）⟹ f=子势/父势=r_{k−1}/r_k=λ^{k−1}/λ^k=1/λ（几何强制，零自由度形式）；旧全局 θ_sub/θ_total 归一化（theta_weights 固定窗口 [FIRST_BSP,MAX) 不随 σ:k↦k+1 平移 ⟹ f=λ^sub/Σλᵏ 随级别变，即使 A₅-理想 θ_k=θ₀λᵏ 仍破 T59）是**隐性违反 σ-不变性**；实装 SUB_SPAWN_FRAC 替代 try_spawn_cost_gated 的 θ 配额；**成本门（角色A，depth_ref.theta）保留经验 θ 不变**——势存在性判定（θ=None/θ<friction⟹N4 终止）本质需 L2 经验量（λ^k 恒正会使 N4 终止失效违 T19）；证明基础=T23 自相似递归+T59 step-replication（**非** R₃-on-W_form——W_form 是形态学结构生成算子 g5g6g7g8 非配额算子，符号碰撞已修正，结论不变）；值 1/λ 因 λ（级别尺度比 A₅）是涌现量（T50 Δt(k)∝λ^k）未独立测，作 leverage_triad 唯一自由度回测扫描（53课配额「留白」被「势∝r 公理」填补为 1/λ 形式，值留经验）"
type: 概念发现（语法记录 / 会计层 σ-等变约束）
status: 已结算
date: 2026-06-16
settled_date: 2026-06-16
settlement: "编排者裁决（2026-06-16 用户指令——「会计层也要从螺旋推导重写」）。16-agent workflow（8 会计运算各螺旋推导+对抗验证）穷尽审计 unn 会计层：**7/8 运算 CONFIRMED_CONFORMS**（nav 径向 ambient 观测量/settle 角向读数/close_voice 三返还 P&L σ-守恒/earning τ-不对称正确/N_base 双向重定基/双层记账径向一致/prove_n8 守 Σunits σ-不变）——会计核心已是螺旋推导的逐字正确实装（26.9M bar N8 零 panic 淬炼）；**唯一 theta_quota = GENUINE_OPEN**（已上浮 [[2026-06-15-theta-allocation-non-necessity]]，环15「必然性争议」未裁决）。theta 分双角色：**角色A 成本门 = CONFORMS**（经验 θ 正确，不可替代为 λ^k——λ^k 恒正使 N4 终止永不触发违 T19，势存在性本质需 L2 经验量）；**角色B 配额比例 = 真偏差**（f 应 σ-不变常数 by T48+T59+T23，实装 f=θ_sub/θ_total 固定窗口随级别变破 T59），但修复需改 T18 已结算定理含义=定义冲突=no-workaround 上浮。**编排者裁决读法A**：「势∝r」不是可弃前提，是径向坐标 r 的**定义本身**（r 标记级别即势能；若势不∝r 则 r 不是级别，整个螺旋径向维失去意义；L0 定义层信息增量为零不可经验否定）⟹ 第53课「配额留白」被公理填补 ⟹ f=r_{k−1}/r_k=1/λ（零自由度几何强制）。R2（自由常数）被收紧（留白被公理填补无独立自由度）；R3（保留 θ）被 090号声明膨胀禁。**实装**：`SUB_SPAWN_FRAC`（positional_fusion.rs，=1/λ 形式）替代 `try_spawn_cost_gated:950-962` 旧 `θ_sub/θ_total`；成本门（角色A）保留经验 θ。**对抗验证修正（formalization-validity-domain，非翻转）**：上浮诊断 §3.3 的 σ-等变证明把 spawn 算子误等同 R₃ 的 W_form（符号碰撞——W_form 是形态学生成算子非配额算子），结论不变但证明基础须用 T23/T59 自相似递归（不引 unsound 的 R₃-on-W_form lemma）。**iso bit-exact**：iso 不调 try_spawn_cost_gated（其 spawn 路径独立，isolated_fugue.rs:407 自有 θ 配额，不在本裁决范围），close_voice 逻辑逐字不变 ⟹ iso 回测零影响。"
source: "rust/src/trading/positional_fusion.rs（SUB_SPAWN_FRAC 常数 = 1/λ σ-不变配额）/ rust/src/trading/unified_necessity.rs（try_spawn_cost_gated:950-962 m_quota=p_units×SUB_SPAWN_FRAC 替代 θ_sub/θ_total；成本门 depth_ref.theta 保留）/ docs/necessity_derivation.md 环15 T₁₈（✓ 裁决，势∝r 公理⟹f=1/λ）+ §3 会计层定理（T33-T42 CONFORMS）/ analysis/unn_theta_allocation_necessity_diagnosis.md §3（σ-等变诊断）/ .chanlun/escalations/2026-06-15-theta-allocation-non-necessity.md（已裁决）/ 16-agent workflow accounting-spiral-derivation（7/8 CONFIRMED_CONFORMS + theta GENUINE_OPEN）；缠师原文 第53课配额（留白）+ 第15环（操作量∝势）"
epistemological_level: "「f 必须 σ-不变（级别无关）」= L0（群论：T48 units=σ-不变 Casimir + T59 自相似 σWσ⁻¹=W + T23 递归 step-replication 强制 f_k=f_{k+1}，条件 A₅）；「f=1/λ 具体形式」= L0（势∝r 是径向坐标 r 的定义/公理，配额=r_{k−1}/r_k=1/λ，零自由度）；「旧全局 θ 归一化破 σ-不变」= L0（固定窗口 [FIRST_BSP,MAX) 不随 σ:k↦k+1 平移，f=λ^sub/Σλᵏ 随级别变，可证）；「λ 值」= L2（级别尺度比 A₅ 涌现量，T50 Δt∝λ^k 间隔比，未独立测⟹SUB_SPAWN_FRAC 作单一自由度回测扫描，初始 λ=2⟹f=0.5）；「成本门角色A 保留经验 θ」= 角色分离必然（N4 势存在性判定需 L2 经验量，λ^k 恒正违 T19）；「θ→σ-不变配额改变全标的 L3 读数」= L2 有效域读数（非验收，验收=A4/N8 守恒 prove 零 panic 与 f 值无关）；7/8 会计运算 CONFORMS = L0 推导（手算 NAV 守恒）+ L2 旁证（26.9M bar N8 零 panic）"
depends_on:
  - '541'   # 螺旋覆盖空间——σ-不变性的几何根（T48 units Casimir / T59 自相似 / 径向 r=λ^k）；本节点是 T18 在径向 σ 对称性下的结算
  - '538'   # 嵌套赋格会计双视图——配额 spawn 是双层记账（父降成本视图 N→N−m + 子独立头寸 m）的会计入口
related:
  - '534'   # 嵌套递归会计 regime 分离——配额是会计层；θ→1/λ 改变全标的 L3 有效域读数（清仓/spawn 频率 regime 函数）
  - '539'   # 清仓判据 regime 门控开放轴——配额改变与清仓判据同属操作有效域 L3 读数（回测扫描 f 精化）
  - '527'   # σ 存在论=走势方向态——σ 尺度移位是配额 σ-不变性的群论载体
contrast:
  - '541'   # 541 标 theta_quota 为 OPEN（环15 必然性争议未裁决，4 永久残余之外的可工程闭合点之一）；542 裁决之（读法A 势∝r 公理⟹f=1/λ）——OPEN→SETTLED 的推进
provenance: "[旧缠论]（第15环操作量∝势 + 第53课配额留白，博文一级权威）+ 蜂群形式化建构（θ 经验配额⊥螺旋 σ-自相似的群论诊断，16-agent workflow 穷尽审计）+ 编排者价值裁决（势∝r 是径向坐标定义/公理，非可弃前提⟹f=1/λ）+ 引擎实装（SUB_SPAWN_FRAC 替代 θ_sub/θ_total，375 单测 + N8 守恒零 panic）"
negation_form: "全局 θ 归一化配额（f=θ_sub/θ_total，随级别变）的否定——配额比例必须 σ-不变（级别无关 f=1/λ）。隐性违反 T59 的旧实装被显式 σ-等变约束取代。"
negates: null
tensions_with: null
---

# 542号：配额比例 σ-不变（降成本 spawn 仓位分配的几何必然）

## 一句话

降成本释放给子 voice 的配额比例 **f = m/p_units 必须 σ-不变（级别无关）**——这是 T48（units
= 唯一 σ-不变 Casimir）+ T59（自相似 σWσ⁻¹=W）+ T23（递归 step-replication）的 L0 必然；
「势∝r」作为径向坐标 r 的**定义**进一步把 f 固定为 **1/λ**（子势/父势=r_{k−1}/r_k）。旧全局
`θ_sub/θ_total` 归一化（固定窗口不随 σ 平移 ⟹ f 随级别变）是**隐性违反 σ-不变性**。

## 发生史（OPEN → SETTLED）

| 时点 | 事件 |
|------|------|
| 2026-06-15 | unn 三问诊断工位上浮：θ 全局归一化 ⊥ T59（环15「必然性争议」，三读法 R1/R2/R3 待裁决） |
| 2026-06-16 | 用户指令「会计层从螺旋推导重写」⟹ 16-agent workflow 穷尽审计 |
| 2026-06-16 | workflow 判决：7/8 会计运算 CONFIRMED_CONFORMS，唯一 theta_quota GENUINE_OPEN |
| 2026-06-16 | 编排者裁决**读法A**（势∝r 是径向坐标 r 的定义/公理）⟹ f=1/λ，实装 SUB_SPAWN_FRAC |

## 核心论证（编排者裁决，读法A）

**「势∝r」不是可弃前提，是径向坐标 r 的定义本身**：r 标记级别 = 递归深度 = 势能。若势不∝r，
则 r 不是级别，整个螺旋的径向维度失去意义。这是 L0 定义层（信息增量为零，不可经验否定，与
A₀ 同级）。

由此第53课「配额留白」**被公理填补**：一旦势∝r 是定义，配额不再有自由度——
$$f = \frac{\text{子势}}{\text{父势}} = \frac{r_{k-1}}{r_k} = \frac{\lambda^{k-1}}{\lambda^k} = \frac{1}{\lambda}$$
零自由度几何强制。R2（自由常数）的「留白」前提被取消；R3（保留 θ）被 090号声明膨胀禁。

## 双角色分离（关键）

θ 在旧实装有双角色，裁决只动角色B：

| 角色 | 函数位置 | 判定 | 处理 |
|------|---------|------|------|
| A 成本门（N4 递归终止） | `try_spawn_cost_gated:941-949` depth_ref.theta | **CONFORMS** | 保留经验 θ（θ=None/θ<friction⟹终止；λ^k 恒正会使 N4 失效违 T19，势存在性需 L2 经验量） |
| B 配额比例（仓位分配） | `try_spawn_cost_gated:950-962` θ_sub/θ_total | **DEVIATES→裁决** | 替换 SUB_SPAWN_FRAC（σ-不变 f=1/λ） |

## 证明基础修正（formalization-validity-domain，非翻转）

上浮诊断 §3.3 的 σ-等变证明把 spawn 算子误等同于 R₃ 的 `W_form`（形态学结构生成算子
g5g6g7g8），这是**符号碰撞**——W_form 生成结构类型，不是会计层配额算子。结论不变（f 必须
σ-等变），但严格证明基础应引 **T23 自相似递归 + T59 step-replication**，不引 unsound 的
R₃-on-W_form lemma。

## 验收与有效域分离

- **验收（L0/L2）**：A4/N8 守恒 prove 零 panic——σ-不变配额不破 Σunits=N_base（spawn 父−m
  子+m，Σ 守恒与 f 值无关）。375 单测 + BTC/CL/ES 真实数据零 panic。
- **有效域读数（L2，非验收）**：θ→1/λ 配额改变全标的 L3 回测读数；SUB_SPAWN_FRAC 值（=1/λ）
  作 leverage_triad 唯一自由度回测扫描（初始 λ=2⟹f=0.5；待 T50 涌现 λ 测量精化为
  f=1/λ_measured 零自由度 R1 形式——扫得最优 f≈1/λ_emergent 则势∝r 公理获 L2 经验旁证）。

## iso bit-exact

iso 不调 `try_spawn_cost_gated`（其 spawn 路径独立，`isolated_fugue.rs:407` 自有 θ 配额，
**不在本裁决范围**——若要统一须用户另开 iso 范围）；`close_voice` 逻辑逐字不变 ⟹ iso 回测
零影响。本裁决仅作用于 unn。

## 语法记录

「仓位分配 spawn 比例必须 σ-不变（级别无关），全局 θ 归一化是隐性违反 T59」——这条隐性规则
本已在 T48/T59 中运作，本节点将其**显式化**为会计层的 σ-等变约束。
