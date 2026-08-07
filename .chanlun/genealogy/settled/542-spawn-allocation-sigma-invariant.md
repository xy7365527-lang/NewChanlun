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

---

## ★§925 订正（编排者裁定 2026-08-07，经由票 #925；增补层，上文原文逐字保留为发生史）

**上游**：[ADR 0017](../../../docs/adr/0017-chong-quantity-numerator-shortdiff-amplitude.md) 连带处置 (b) 把本号的 `λ = 2` 上浮，
wayfinder 票 [#925](https://github.com/xy7365527-lang/NewChanlun/issues/925)（map #787）即该上浮。
**勘察报告**：`.chanlun/review-results/issue925-lambda-implementations-20260807.md`（commit `43a2f684df`，AFK 只读探针，未改任何 `.rs`）。

**处置 = 值层作废、形式层保留待判、落点冻结。** 三条：

### 处置一：值层作废（可断言）

`λ = 2`（⟹ `f = 0.5`）**作废**。底档实测 `w ≈ 0.323`（8 标的几何平均，
[#915](https://github.com/xy7365527-lang/NewChanlun/issues/915)）对应 **`λ ≈ 3.1`**。
这条已由 ADR 0017 连带处置 (b) 第 1 点断言（`docs/adr/0017-*.md:285`），本裁定**确认之**。

### 处置二：形式层保留待判（不可断言）

`f = 1/λ` 这个几何形式**否不掉**——否掉「级别无关」需要证明各档 `w` 不同，
而 ADR 0017 **裁定八已判上档测不出**（`docs/adr/0017-*.md:283`），
常数与非常数在当前数据上无从区分。⟹ **形式保留，标「待判」，不作废。**

### 处置三：落点冻结

`SUB_SPAWN_FRAC`（两处，见下表）**不改数值**。理由是 #925 勘察实测——**就 NT 生产而言它不可达**：
`grep -rnE "\bLAMBDA\b|SUB_SPAWN_FRAC|MOBILE_FRAC" rust/src/theta_v0/`（覆盖 `rust/src/theta_v0/` 全树）
**零命中**，π（唯一现役引擎）对这四处全部零引用。改与不改**都不动生产读数**。
形态与名分的统一（两处 `SUB_SPAWN_FRAC` 静默劈叉 + `MOBILE_FRAC` 那对改错一处即 panic）
归实施票 **[#943](https://github.com/xy7365527-lang/NewChanlun/issues/943)**。

---

## 一并登记的三条事实（#925 勘察查实，本次已逐条独立复核）

### 事实一：本号自陈的「回测扫描」，在已查到的范围内没做过

本号 `epistemological_level` 字段与 title 均自陈 `SUB_SPAWN_FRAC` 作
「leverage_triad 唯一自由度**回测扫描**」；实装侧同义句在 `rust/src/trading/positional_fusion.rs:100-103`。
**9 条检索式零命中**（覆盖目录逐条标注，见勘察报告 §Q3 表）：

| 检索式 | 覆盖 | 结果 |
|---|---|---|
| `grep -rln "SUB_SPAWN_FRAC" . --exclude-dir=.git --exclude-dir=target` | 全仓 | 15 文件全部打开，无一是扫描结果 |
| `grep -rln "leverage_triad" .`（同上排除） | 全仓 | 12 文件，均为定义/诊断/裁决文本 |
| `ls rust/src/bin/` + `grep -rln "SPAWN_FRAC\|MOBILE_FRAC\|LAMBDA\|lambda" rust/src/bin/` | 50 个 bin 逐个查 | 5 命中全为 `lambda_c`/`lambda_a`（走势段起点索引）同名异物 |
| `git log --oneline --all --grep="SPAWN_FRAC"` | 全 ref | 3 commit，全是落地非扫描 |
| `git log --oneline --all --grep="leverage_triad\|配额扫描\|f 扫描\|λ 扫描"` | 全 ref | 仅 `346ac05068`（542 实装） |
| `gh issue list --search "SUB_SPAWN_FRAC" --state all` | tracker **含 closed** | 仅 #925 |
| `gh issue list --search "leverage_triad" --state all` | 同上 | 仅 #925 |
| `gh issue list --search "配额" --state all` | 同上 | 30 条，无一是 f 扫描 |
| `grep -rn "env::var" rust/src/trading/positional_fusion.rs rust/src/spiral/` | — | 零命中 ⟹ 无参数化入口，扫描技术上须改源码重编译 |

**反面佐证**（扫描未做的正面痕迹）：`docs/spiral_engine_v2_architecture.md:49` 至今逐字写
「`λ=2`（SUB_SPAWN_FRAC=0.5） | **L2 待测** | A₅ 二分递归建模默认，待 T50 涌现 λ 测量精化」；
同文件 `:619` 参数审计表逐字「SUB_SPAWN_FRAC | 0.5 | ⚠ **形式✅/值❌** | L0 形式 + L2 值 | …待 T50 涌现 λ 测量精化为 `1/λ_measured`」
（**照实**：`:619` 那格写的是「形式✅/值❌」，**没有「L2 待测」四字**，同义不同词）。

**⟹ `0.5` 是一个既非理论也非实测的数**（090 照实：这是「已查到的范围内没做过」，**不是绝对否定**）。

> **⚠️ 防误引**：`.chanlun/escalations/2026-06-19-1922-t-recover-quota-full-vs-sigma-invariant.md` 那张
> 8 标的表（GC +50.7pp / ES −19.6pp 等）**不是这个扫描**——它的自变量是 `t_engine.recover` 的
> 「1/3 → 全量」**二值切换**，作用对象是 `recursive_t`（该上浮自陈「仅改 t_engine，不碰 operate.rs/542 守卫」），
> 与本号点名的 `positional_fusion`/`unified_necessity` 不同族。

### 事实二：同一概念 `f = 1/λ` 有四处实装、两个互斥 λ

穷举依据：`grep -rnE "\bLAMBDA\b" rust/src/` 全仓 **4 命中**；`grep -rn "const SUB_SPAWN_FRAC" rust/src/` **2 命中**。覆盖目录 = `rust/src/` 全树。

| # | 常量 | 文件:行 | 值 | 表达 | λ |
|---|---|---|---|---|---|
| 1 | `SUB_SPAWN_FRAC` | `rust/src/spiral/params.rs:37` | 0.5 | `1.0 / LAMBDA`（`:32` `LAMBDA = 2.0`） | 2 |
| 2 | `SUB_SPAWN_FRAC` | `rust/src/trading/positional_fusion.rs:104` | 0.5 | `0.5`（**硬编码**） | 2（隐含） |
| 3 | `MOBILE_FRAC` | `rust/src/fugue_v3/mod.rs:115` | 1/3 | `1.0 / LAMBDA`（`:111` `LAMBDA = 3.0`） | 3 |
| 4 | `MOBILE_FRAC` | `rust/src/recursive_t/rec_engine.rs:69` | 1/3 | `1.0 / 3.0`（**模块内私有**，遮蔽 #3） | 3（隐含） |

本号 `source` 字段只点 #2 一族（`positional_fusion.rs` + `unified_necessity.rs`），**#1/#3/#4 从不在本号裁决范围内**。

**#3 ↔ #4 那对改错一处即 panic**：守卫 `rust/src/recursive_t/prove_guards.rs:277`
逐字 `let canonical = units_before * MOBILE_FRAC;`，其 `MOBILE_FRAC` 由 `:31` `use crate::fugue_v3::MOBILE_FRAC;` 引入；
而 `rec_engine.rs:69` 逐字 `const MOBILE_FRAC: f64 = 1.0 / 3.0;` 是模块内私有硬编码。
只改 `fugue_v3/mod.rs:111` ⟹ 两值不等 ⟹ `prove_guards.rs:278-282` 的 assert 失败 ⟹ **NT 生产策略每次 sink/drain 直接 panic**。
两处 `SUB_SPAWN_FRAC` 之间则相反——全仓**无任何测试或断言**把二者挂钩，编译期与测试期都不报警，是名副其实的**静默劈叉**。
两者统归 [#943](https://github.com/xy7365527-lang/NewChanlun/issues/943)。

### 事实三：λ=3 那一支的依据须降名分

`rust/src/fugue_v3/mod.rs:110` 现逐字写：

> `/// 尺度比 λ。**值 L2**（reinterp §5.2：026:80「用其中的 1/3」⟹ λ=3）。`

读起来像**原文规定**。但回原课核对（正本路径 `docs/chanlun/text/blog/026-第26课.md`，行号已逐字复核）：

- **`026:80`** 逐字：「2、级别必须配套来看，最好不要单纯的短线，……短线必须坚持。但仓位可以控制，**例如**用其中的1/3，**慢慢养成好习惯以后，就可以更随心所欲一点**。」
  ——「例如」与「可以更随心所欲一点」在**同一句**内（分号级读点相连，非分属两行）。
- **`026:447`**【答疑】逐字：「另外，一定要灵活，不能光会先买后卖，也应该学会先卖后买。还有，该以什么比例运用也是一个关键，**不熟练的情况下**，如果仓位不太大，1/3或1/4是比较合适的。」
  ——位于 `:443` `========` 分隔符之后的缠师回复段（读者提问在 `:437-441`）。
- 仓内 `analysis/bc_architecture_research.md:325`（表第 6 行「**C 的 slice 比例**（1/3、1/4）」）已判：
  「026:80/447 给出 1/3、1/4，但语境是"不熟练的情况下"——是**训练轮，不是最优参数**」「比例是**自由参数，原文数字不可当作校准值**」。
  （⚠️ 行号为 **`:325`**；ADR 0016 `:131`/`:309` 与 #914 票面曾误记作 `:320`——`:320` 是同表第 1 行「层间配额公式」，`:324` 才是第 5 行。ADR 0017 `:360-363` 已登记此订正。）

**⟹ 名分从「原文规定值」降为「原文举例，且原文明写可调」。**

> **⚠️ `026:80` 的正文界名分本身存疑**：该课 `↑正文` 界标在 `:40`，`:80` 在其**后**；
> 但其内容明显是作者自己的补充（`:76` 逐字「下午走的太急，补充一段，有必要把一些前面已经多次提过的原则重复一次。」，且通篇「本ID」自称），
> 不是读者答疑。**本裁定不判它属【正文】还是【答疑】，照实标为「界后作者补充，名分待核」。**
> （已核：`:80` 行首无「（注：」「(娇注:」类标记，行内亦无嵌注；同区 `:72` 有「(娇注:」但不在 `:80`。）

---

## 因本次订正而过期的字段表述

上文与 YAML front matter **原文一律不改**（发生史逐字保留），但下列表述自 2026-08-07 起**已过期**，援引须以本订正段为准：

| 位置 | 过期的表述 | 现状 |
|---|---|---|
| `epistemological_level` 字段 | 「「λ 值」= L2（……未独立测⟹SUB_SPAWN_FRAC 作单一自由度回测扫描，**初始 λ=2⟹f=0.5**）」 | **值作废**（处置一）。且「作回测扫描」这件事在已查到的范围内**从未发生**（事实一） |
| `title` / `settlement` / `negation_form` 三字段中的 `f=1/λ` **值** | 一律隐含 λ=2 ⟹ f=0.5 | 同上；**形式 `f=1/λ` 仍保留（待判）**，只有值作废 |
| 正文「验收与有效域分离」节（`:75-77`） | 「SUB_SPAWN_FRAC 值（=1/λ）作 leverage_triad 唯一自由度回测扫描（初始 λ=2⟹f=0.5；**待 T50 涌现 λ 测量精化**为 f=1/λ_measured 零自由度 R1 形式——扫得最优 f≈1/λ_emergent 则势∝r 公理获 L2 经验旁证）」 | 该扫描**未做**；「待 T50 涌现 λ 测量精化」这条路径至今未走。**因此「势∝r 公理获 L2 经验旁证」这个预期的兑现条件从未被检验** |
| `source` 字段 | 只点 `positional_fusion.rs` / `unified_necessity.rs` | 事实准确，但**范围不足以覆盖同一概念**——全仓另有 3 处实装、含生产落点 `MOBILE_FRAC`（事实二）。归 #943 |

**未变的部分（照实）**：本号「配额比例 `f` 必须 σ-不变（级别无关）」这条 **L0 断言本身未被触动**——
本次只作废了它的**值**，形式层因「上档测不出」而**无从否证**，故保留待判（处置二）。
