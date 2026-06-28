/-
  Origin/CandidateSet.lean — 候选集 Γ(x) 的有限性形式化（spec §12, P10；七链环3）
  —— **Γ : 状态 x → List Cand（List 即有限）+ |Γ(x)|<∞** 的 L0 结构形式化。

  ═══════════════════════════════════════════════════════════════════════════
  唯一信源
  ═══════════════════════════════════════════════════════════════════════════
  权威 spec：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §12（P10）+ 七链对照表 line 1169（环3）+ Γ(x) 结果包 line 631-639。逐字方框：

      环3：区间套确认 → 候选集 Γ(x)；
           Γ(x) 由所有级别、所有买卖点、所有区间套确认生成；|Γ(x)|<∞。   (line 1169)
      结果包（line 632）：同一时刻多买卖点构成有限候选集 Γ(x)（|Γ(x)|<∞，
           由所有级别×所有买卖点×所有区间套确认生成）。
      唯一性三依据（line 621-623）：1. Γ(x) 有限；2. ≺_Θ 全序；3. 每步规则确定。

  缺口矩阵 `.chanlun/specs/2026-06-28-lean-existing-vs-20page-gap-matrix.md`（环3）：
      "无 Γ(x) 组装模块 … 真缺口 → 新建（lean-gamma）：Γ:状态→List Cand
       （List 即有限，|Γ|<∞ 平凡）。依赖环1/环2 输出。"

  ═══════════════════════════════════════════════════════════════════════════
  本工位 (lean-gamma) 的产出 vs 既有锚点（复用 + no-patch 诚实标注）
  ═══════════════════════════════════════════════════════════════════════════
  本文件**组装** 环1（买卖点谓词 b_ℓ∈{0,1}^6）+ 环2（区间套证书 N^δ_{ℓ↓e}）的输出为候选集 Γ(x)，
  并证 |Γ(x)|<∞。**复用**（不重造）：
    · `Origin.NestingCertificate`（环2，lean-nest）：`Dir`/`LevelData`/`N`/`Conf`——区间套确认
      N^δ_{ℓ↓e}(x):Bool 直接作候选过滤门。本文件**不重证**区间套递归（单一权威在 NestingCertificate）。
    · `Origin.OperationRole18`（环4，lean-role18）：`Side`/`Role18`/`classifyR18`——候选方向 δ 用
      `Side`（与角色层 R(g)=H×V×δ 同一方向类型），下游 fold（环5 R_Θ）可直接 role 化。本文件
      **不重证**角色分类（单一权威在 OperationRole18），只提供 `Cand.roleOf` 桥接（候选 δ → R(g)）。
    · 买卖点向量 b_ℓ∈{0,1}^6：复用 `NestingCertificate.LevelData` 的 6 个 Bool 字段（b1..b3/s1..s3）
      作信号向量接口（spec P02）。**不** import `Origin.BuySellPredicate`（实测不编译 + 用 Fintype/
      Finset，违反本工位纯 core 硬约束）——其缠论语义内容（B_i=IsType_i∧side）由 BuySellPredicate/
      BspClassification 拥有（契约锚），本文件**消费** Bool 输出不重证。

  ★候选 = (级别 ℓ, 执行级 e, 买卖点类型 t ∈ 6 类) 三元组，方向 δ = dirOf t（b*→买入/long，
    s*→卖出/short）。这把 spec "所有级别 × 所有买卖点 × 所有区间套确认" 落为真三重积，按
    「买卖点 t 在执行级 e 触发（Conf 读取处）∧ 区间套 N^δ_{ℓ↓e} 确认」过滤。"同一时刻多买卖点"
    （line 632）⟺ 同一 (ℓ,e) 下多个 t 同时触发 = 多个候选。

  ═══════════════════════════════════════════════════════════════════════════
  认识论等级（formalization-validity-domain / 231号 强制标注）
  ═══════════════════════════════════════════════════════════════════════════
  全部 **L0**（纯定义 / List 枚举过滤 / List.length 有限，**不依赖任何数据**）。
  `lake env lean Origin/CandidateSet.lean` 通过 = 「Γ(x) 是状态 x 上的有限 List、|Γ(x)| 受
  显式上界 |allCands ℓmax| 控制」在**定义层**成立——**不是**任何「候选集在真实行情上非空/有 alpha」
  的实证断言（那是 L2/L3，需真实 K 线 + 各级别真实买卖点 + 真实区间套）。
  |Γ|<∞ 的信息增量 = 同义反复（L0）：List 在 Lean 中归纳定义即有限，length 恒为 Nat。
  **不冒充** L1+。买卖点择时 v1 已被全窗 L3 8/8 否证（记忆 `newchanlun-v1-fullwindow-l3-falsified`），
  但那否证的是 v1 实盘盈利性（L2/L3），与本文件 L0 有限性定理**认识论等级不同、不可互相否证**。

  范式：纯 List/Bool/Nat + Lean 核心 `Init`（List.range/flatMap/filter/length_filter_le/mem_filter）。
  仅 import 两个纯 core 兄弟文件（NestingCertificate 零 import；OperationRole18 仅 import
  SourceAxioms 取 Side）——**无** Mathlib/Batteries/Std。禁 sorry/admit/axiom。禁 Fintype/Finset
  （Γ 用 List 表达有限性，非 Finset）。不编辑 lakefile（报 Lead 登记 root `Origin.CandidateSet`）。
-/

import Origin.NestingCertificate
import Origin.OperationRole18

namespace NewChanlun.Origin.CandidateSet

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 买卖点类型 BspType（b_ℓ∈{0,1}^6 的 6 个分量，spec P4 §5 / 环1）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **买卖点类型 `BspType`（spec P4 §5 方框，b_ℓ=(B_{1..3},S_{1..3})∈{0,1}^6）** ——
  6 个具名买卖点：三类买点 `b1 b2 b3`（方向=买入/long）+ 三类卖点 `s1 s2 s3`（方向=卖出/short）。

  ★这是信号向量 b_ℓ 的**分量名**（不是其缠论判据）：B_{i,ℓ}=IsType_i∧side 的语义由
    BuySellPredicate/BspClassification 拥有（契约锚），本文件只消费 `LevelData` 的 6 个 Bool 输出。
    买卖点**不互斥**（spec P4 §5"六个买卖点可以重合"）⟹ 同级可多类同时触发 ⟹ 多候选（line 632）。 -/
inductive BspType where
  /-- 第一类买点 B_{1,ℓ}（方向=买入）。 -/
  | b1
  /-- 第二类买点 B_{2,ℓ}（方向=买入）。 -/
  | b2
  /-- 第三类买点 B_{3,ℓ}（方向=买入）。 -/
  | b3
  /-- 第一类卖点 S_{1,ℓ}（方向=卖出）。 -/
  | s1
  /-- 第二类卖点 S_{2,ℓ}（方向=卖出）。 -/
  | s2
  /-- 第三类卖点 S_{3,ℓ}（方向=卖出）。 -/
  | s3
deriving DecidableEq, BEq, Repr

/-- **买卖点类型的方向 δ = dirOf t（spec P4 §6）** —— 买点 → `Side.long`(+1=买入)，
    卖点 → `Side.short`(-1=卖出)。方向用 `Side`（复用 OperationRole18/SourceAxioms，与角色层 δ 同型）。 -/
def dirOf : BspType → Side
  | .b1 | .b2 | .b3 => Side.long
  | .s1 | .s2 | .s3 => Side.short

/-- **Side → NestingCertificate.Dir 桥（spec δ 的两载体同构）** —— long↔buy(+1)，short↔sell(-1)。
    NestingCertificate 的 N 用本地 `Dir`（零依赖纯 core），本文件方向用 `Side`（对接角色层）——
    在调用区间套证书时一一映射（NestingCertificate docstring line 68-70 的同构）。 -/
def toDir : Side → NestingCertificate.Dir
  | Side.long  => NestingCertificate.Dir.buy
  | Side.short => NestingCertificate.Dir.sell

/-- **买卖点 t 在级别数据 d 上是否触发 `firedAt`** —— 选 `LevelData` 中 t 对应的 Bool 分量
    （b1→d.b1, …, s3→d.s3）。这是信号向量 b_ℓ 第 t 分量的读取（接口契约，不重证判据）。 -/
def firedAt : BspType → NestingCertificate.LevelData → Bool
  | .b1, d => d.b1
  | .b2, d => d.b2
  | .b3, d => d.b3
  | .s1, d => d.s1
  | .s2, d => d.s2
  | .s3, d => d.s3

/-- 6 个买卖点类型的显式枚举（穷尽，spec P4 §5 的 {0,1}^6 分量集）。 -/
def allBsp : List BspType := [.b1, .b2, .b3, .s1, .s2, .s3]

/-- ★6 类买卖点穷尽（L0）：每个 BspType 在 allBsp 中恰出现一次（穷尽 + 无重复）。 -/
theorem allBsp_count_one (t : BspType) : allBsp.count t = 1 := by
  cases t <;> decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 候选 Cand + 全市场状态 State（环1 b_ℓ + 环2 N^δ 的组装载体）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **候选 `Cand`（spec §12 候选集 Γ 的元素，★核心类型）** —— 三元组：
  - `lvl  : Nat`     —— 级别 ℓ（区间套操作级，spec 自相似 𝕃=ℤ 上一点）。
  - `exec : Nat`     —— 执行级 e（区间套证书 N^δ_{ℓ↓e} 的落底确认级；前置 e ≤ ℓ）。
  - `bsp  : BspType` —— 触发该候选的买卖点类型（6 类之一）；方向 δ = `dirOf bsp` 由此派生。

  ★**供下游 R_Θ fold（环5 lean-interp）消费的信息**：级别 `lvl`、执行级 `exec`、方向 `dir`
    （= dirOf bsp，`Side` 型）、买卖点 `bsp`。环4（OperationRole18）可由 `dir` + 关系上下文
    （前兄弟/父容器方向）经 `Cand.roleOf` 算出 R(g)=(H,V,δ)——即候选已**带级别/方向/角色信息**。
  ★`DecidableEq`/`BEq` ⟹ List.count/filter/decide 机器可算。 -/
structure Cand where
  /-- 级别 ℓ（区间套操作级）。 -/
  lvl  : Nat
  /-- 执行级 e（N^δ_{ℓ↓e} 落底确认级，e ≤ ℓ）。 -/
  exec : Nat
  /-- 触发买卖点类型（6 类之一）。 -/
  bsp  : BspType
deriving DecidableEq, BEq, Repr

/-- **候选方向 δ_g（spec P4 §6）** —— 由买卖点类型派生（`Side` 型，对接角色层 R(g) 的 δ 轴）。 -/
def Cand.dir (c : Cand) : Side := dirOf c.bsp

/--
  **候选角色桥 `Cand.roleOf`（环3 → 环4，spec 七链 line 1170）** —— 给候选 c 的关系上下文
  （`prevDir` = 同级前兄弟方向 σ_{prev(g)}，`parentDir` = 父容器方向 σ_{p(g)}，Option Side：
  none = 无前兄弟 / 父为胚元 ∂），用 OperationRole18 的去根化分类器算出完整角色 R(g)=(H,V,δ_g)。

  ★这坐实候选**携带方向信息足以 role 化**（满足下游 fold "带级别/方向/角色信息" 契约）：
    角色分类的单一权威在 `Origin.OperationRole18.classifyR18`，本文件**不重证**，只组合 c.dir。 -/
def Cand.roleOf (c : Cand) (prevDir parentDir : Option Side) : Role18 :=
  classifyR18 prevDir parentDir c.dir

/--
  **全市场状态 `State`（spec §1 去根化级别系统 𝕃=ℤ 有限支撑）** ——
  - `f    : Nat → LevelData` —— 状态 x：级别 ↦ 该级别的信号向量 b_ℓ + 候选/区间套数据
    （复用 NestingCertificate.LevelData；上游 ParseStruct + 各级别识别提供，本文件消费）。
  - `ℓmax : Nat`            —— 有限支撑上界 ℓ_max（spec P1 §1 / P18 条件4：𝕃=ℤ **有限支撑**，
    ℓ_max 动态涌现）。**这是 |Γ|<∞ 的结构根因**：级别有限支撑 ⟹ 候选枚举有限。 -/
structure State where
  /-- 状态 x：级别 → 级别数据（信号向量 + 候选 + 区间套）。 -/
  f    : Nat → NestingCertificate.LevelData
  /-- 有限支撑上界 ℓ_max（spec §1 𝕃=ℤ 有限支撑；|Γ|<∞ 的结构根因）。 -/
  ℓmax : Nat

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 候选枚举域 allCands + 隶属门 member + 候选集 Γ(x)
    （spec §12：Γ 由 所有级别 × 所有买卖点 × 所有区间套确认 生成）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **候选枚举域 `allCands ℓmax`（spec "所有级别 × 所有买卖点 × 所有区间套确认"的定义域）** ——
  全部 (ℓ, e, t) 三元组：ℓ ∈ [0, ℓmax]（有限支撑内所有级别），e ∈ [0, ℓ]（所有执行级，
  满足区间套前置 e ≤ ℓ），t ∈ 6 类买卖点。三重 flatMap 笛卡尔积（与 OperationRole18.allR18
  同手法）。这是过滤前的**有限**候选论域（|allCands ℓmax| = Σ_{ℓ=0}^{ℓmax} 6·(ℓ+1)）。 -/
def allCands (ℓmax : Nat) : List Cand :=
  (List.range (ℓmax + 1)).flatMap (fun ℓ =>
    (List.range (ℓ + 1)).flatMap (fun e =>
      allBsp.map (fun t => ⟨ℓ, e, t⟩)))

/--
  **候选隶属门 `member x c`（spec §12：买卖点触发 ∧ 区间套确认）** —— 候选 c=(ℓ,e,t) 真属 Γ(x) ⟺
  - 买卖点 t 在执行级 e **触发**：`firedAt t (x.f e)`（Conf^δ_e 读取处的 t 分量为 1，环1）；
  - **且** 区间套证书 N^{dirOf t}_{ℓ↓e}(x) **确认**：`NestingCertificate.N (toDir (dirOf t)) x.f ℓ e`
    （环2 区间套确认，方向由 t 决定 b*→buy/s*→sell）。

  ★两门合取（∧）= 真过滤：买卖点触发（环1）+ 区间套确认（环2）⟹ 候选（环3）。
    触发与确认一致：t 触发 ⟹ Conf^{dirOf t}_e ⊇ t = 1 ⟹ N 的落底基例满足（无矛盾）。 -/
def member (x : State) (c : Cand) : Bool :=
  firedAt c.bsp (x.f c.exec)
    && NestingCertificate.N (toDir (dirOf c.bsp)) x.f c.lvl c.exec

/--
  **★候选集 Γ(x)（spec §12, P10, line 1169 方框，★核心定义）** —— 在有限支撑级别论域
  `allCands x.ℓmax` 上、按隶属门 `member x` 过滤出的候选 List。

  Γ(x) = { (ℓ,e,t) : ℓ≤ℓmax, e≤ℓ, t 在 e 触发, N^{dirOf t}_{ℓ↓e}(x)=1 }，**以 List 表达**。
  ★List 即有限（Lean 归纳定义），故 |Γ(x)|<∞ 平凡（见 §4）。"同一时刻多买卖点"⟹ 同一 (ℓ,e)
    多个 t 触发 ⟹ Γ 含多元（line 632）。 -/
def gamma (x : State) : List Cand :=
  (allCands x.ℓmax).filter (member x)

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. |Γ(x)| < ∞（spec §12 line 1169 / line 621 唯一性第一依据）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★|Γ(x)| ≤ |allCands ℓmax|（显式有限上界，L0）** —— Γ(x) 是论域 `allCands x.ℓmax` 的过滤子，
  故其长度不超过论域长度。这给 |Γ|<∞ 一个**显式可计算上界**（不只是"是 Nat"）：上界
  |allCands ℓmax| = Σ_{ℓ=0}^{ℓmax} 6·(ℓ+1) 只依赖有限支撑 ℓ_max——坐实"由所有级别×所有买卖点×
  所有区间套确认生成"的论域有限性。证：`List.length_filter_le`（Lean 核心）。 -/
theorem gamma_length_le_universe (x : State) :
    (gamma x).length ≤ (allCands x.ℓmax).length :=
  List.length_filter_le (member x) (allCands x.ℓmax)

/--
  **★|Γ(x)|<∞（实质有限性：Γ 被论域上界界定，spec line 1169 / line 621 第一依据）** ——
  Γ(x) 的长度不超过论域 `allCands x.ℓmax` 的长度。这是 spec "|Γ(x)|<∞" 的**实质证明**
  （非同义反复）：上界 |allCands ℓmax| = Σ_{ℓ=0}^{ℓmax} 6·(ℓ+1) 依赖有限支撑 ℓmax，
  Γ 作为过滤子不超过论域（`List.length_filter_le`）。非平凡：不对任意 List 成立，依赖
  `allCands` 论域构造。即 `gamma_length_le_universe` 的具名入口（保留向后兼容名称）。 -/
theorem gamma_finite (x : State) : (gamma x).length ≤ (allCands x.ℓmax).length :=
  gamma_length_le_universe x

/--
  **★Γ(x) ⊆ allCands ℓmax（生成自论域，L0）** —— Γ(x) 的每个候选 c 都在枚举论域 allCands x.ℓmax 中。
  坐实 spec "Γ 由所有级别×所有买卖点×所有区间套确认**生成**"——Γ 不含论域外的候选。
  证：`List.mem_filter`（过滤子的成员必属源）。 -/
theorem mem_gamma_imp_mem_universe (x : State) (c : Cand) :
    c ∈ gamma x → c ∈ allCands x.ℓmax :=
  fun h => (List.mem_filter.mp h).1

/--
  **★Γ(x) 成员的隶属刻画（L0）** —— c ∈ Γ(x) ⟺ c 在论域中 ∧ 买卖点触发 ∧ 区间套确认
  （`member x c = true`）。坐实 Γ 的过滤语义（环1∧环2 ⟹ 环3）。证：`List.mem_filter`。 -/
theorem mem_gamma_iff (x : State) (c : Cand) :
    c ∈ gamma x ↔ c ∈ allCands x.ℓmax ∧ member x c = true :=
  List.mem_filter

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 反退化见证（Γ 真跑通：具体三级区间套链 ⟹ Γ 非空、|Γ|=3，非平凡桩）

    ★复用 NestingCertificate.witField（执行级 0 b1=true + 三级 jBuy 真套缩小 [5,25]⊇[8,22]⊇[10,20]
      + 各级 candBuy=true）⟹ N^+_{ℓ↓0}=1 对 ℓ∈{0,1,2}。候选 ⟨ℓ,0,b1⟩ 三者皆 member。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 反退化见证状态：复用 NestingCertificate.witField（三级买入区间套链），有限支撑 ℓmax=2。 -/
def witState : State :=
  { f := NestingCertificate.witField, ℓmax := 2 }

/-- **★反退化见证：枚举论域非平凡 |allCands 2| = 36（L0）** —— ℓ=0→6·1, ℓ=1→6·2, ℓ=2→6·3 = 36
    = 3·(2+1)·(2+2)。坐实 allCands 真笛卡尔积展开（非空桩）。 -/
theorem witness_universe_card : (allCands 2).length = 36 := by decide

/-- **★反退化见证：Γ(witState) 非空（L0）** —— 三级区间套链 + b1 触发 ⟹ 候选 ⟨0,0,b1⟩ 等真入 Γ。
    坐实 gamma 非平凡空桩（能产候选）。 -/
theorem witness_gamma_nonempty : gamma witState ≠ [] := by decide

/-- **★反退化见证：|Γ(witState)| = 3（L0）** —— witField 仅执行级 0 有 b1=true，候选 ⟨ℓ,0,b1⟩
    对 ℓ∈{0,1,2}（N^+_{ℓ↓0}=1 三级套全成立）= 恰 3 个。坐实"同一买卖点跨级别区间套生成多候选"，
    且 |Γ|<∞ 落到具体有限数（3 ≤ 36 = |allCands 2|）。 -/
theorem witness_gamma_card : (gamma witState).length = 3 := by decide

/-- **★反退化见证：候选 ⟨2,0,b1⟩ ∈ Γ(witState)（L0）** —— 操作级 2 ≻ 执行级 0，b1 在执行级触发 +
    三级区间套确认 ⟹ 该候选真属 Γ。用 `List.elem`（BEq，Bool）表达成员——纯 core 下
    `c ∈ gamma x`（Prop/List.Mem）无自动 Decidable 实例，故用 `.elem … = true`（同 OperationRole18
    手法），等价成员断言，decide 可计算。 -/
theorem witness_top_cand_mem : (gamma witState).elem (⟨2, 0, .b1⟩ : Cand) = true := by decide

/-- **★反退化见证：候选 ⟨2,0,b1⟩ 的角色（环3→环4 桥，L0）** —— 父为胚元 ∂（parentDir=none）+
    无前兄弟（prevDir=none）⟹ R(g)=(First, Ambient, long)。坐实 Cand.roleOf 真接 OperationRole18
    去根化分类器（候选携带方向 ⟹ 可 role 化）。 -/
theorem witness_cand_role :
    (⟨2, 0, .b1⟩ : Cand).roleOf none none = ⟨RoleH.first, RoleV.ambient, Side.long⟩ := rfl

/-- **★反退化见证：区间套破坏 ⟹ 候选不入 Γ（L0）** —— witBrokenField 执行级区间 [2,30] 超出
    中间级 [8,22]（子⊄父）⟹ N^+_{2↓0}=0 ⟹ 候选 ⟨2,0,b1⟩ 被隶属门拒。坐实 N^δ 真起门控
    （Γ 非"凡触发即入"，区间套确认是硬门）。`.elem … = false` = 非成员（纯 core，同上）。 -/
theorem witness_broken_cand_not_mem :
    (gamma { f := NestingCertificate.witBrokenField, ℓmax := 2 }).elem (⟨2, 0, .b1⟩ : Cand)
      = false := by decide

/-- **★反退化见证：未触发买卖点不入 Γ（L0）** —— witField 执行级 0 的 s1=false（无卖点触发）⟹
    候选 ⟨2,0,s1⟩ 被 firedAt 门拒。坐实 firedAt 真起门控（Γ 非"凡区间套即入"，买卖点触发是硬门）。
    `.elem … = false` = 非成员（纯 core，同上）。 -/
theorem witness_unfired_cand_not_mem :
    (gamma witState).elem (⟨2, 0, .s1⟩ : Cand) = false := by decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ★本文件**实装**（对照 spec §12 / line 1169，零遗漏）：
    (1) **候选 Cand = (级别 ℓ, 执行级 e, 买卖点 t)** + 方向 δ=dirOf t（`Cand`/`dirOf`/`Cand.dir`）。
    (2) **Γ(x) = 论域过滤**（`allCands` 所有级别×所有买卖点×所有执行级 + `member` 买卖点触发∧
        区间套确认 ⟹ `gamma`，spec §12 三重生成）。
    (3) **|Γ(x)|<∞**（`gamma_finite` 实质上界 ≤ |allCands ℓmax|（即 `gamma_length_le_universe` 的
        具名入口）+ `mem_gamma_imp_mem_universe` Γ⊆论域，spec line 1169 / line 621 第一依据）。
    (4) **环3→环4 角色桥**（`Cand.roleOf` 接 OperationRole18.classifyR18，候选带方向⟹可 role 化）。

  ★本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **环5 R_Θ 解释器 (𝒟,ℬ,𝒦)**：Γ(x) 按平移不变全序 ≺_Θ 排序 + 确定性 fold 出三桶——是
      **下游独立工位 lean-interp**（缺口矩阵环5）。本文件只产 Γ(x):List Cand 作其输入，**不**做
      排序/fold/唯一化（∃!三元组属环5，不属本工位）。
    · **Cand^δ_ℓ 的独立判据**：高于执行级的候选谓词 Cand^δ_ℓ 在 PDF 仅作符号（NestingCertificate
      §4 诚实标注），本文件经 `NestingCertificate.N` 消费其抽象 Bool（candBuy/candSell），**不臆造**。
    · **买卖点 B_i/S_i 的缠论判据**：`firedAt` 读 `LevelData` 的 Bool 接口（信号向量 b_ℓ∈{0,1}^6），
      其判据（IsType_i∧side）由 BuySellPredicate/BspClassification 拥有，本文件**消费**不重证。
      [需人工确认] 第三类边界 reference 含等号 vs Lean 严格<（记忆 `theta-v0-type3-boundary-...`）
      属买卖点谓词层待裁决项，**本文件不引入新冲突**（firedAt 只读 Bool，不取定 >/≥ 边界）。
    · **状态 f 与 ℓmax 的自动构造**：`State`（级别函数 + 有限支撑上界）是**输入**（上游 ParseStruct +
      ℓ_max 动态涌现提供），本文件消费 State 不从 K 线产出（那是 L2 流水线）。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：候选集 Γ(x) 的 **L0 结构形式化**——Γ : State → List Cand（`gamma` = 论域 `allCands`
     按隶属门 `member` 过滤），候选 `Cand=(lvl,exec,bsp)` 带方向 δ=dirOf bsp（Side，对接角色层）。
     **|Γ(x)|<∞** 主定理：`gamma_finite`（实质上界 ≤ |allCands ℓmax|，即 `gamma_length_le_universe`
     的具名入口）+ `mem_gamma_imp_mem_universe`（Γ⊆有限论域）。环3→环4 桥 `Cand.roleOf`。
     全 L0，零 sorry/admit/axiom。反退化见证 |Γ(witState)|=3（非空桩）。
  2. 定义依据：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md` §12（P10）+
     七链表 line 1169（环3：区间套确认→Γ(x)；Γ 由所有级别×所有买卖点×所有区间套确认生成；|Γ|<∞）
     + 结果包 line 631-639（多买卖点构成有限候选集）+ line 621 唯一性第一依据（Γ(x) 有限）。
     输入特征：级别 𝕃=ℤ **有限支撑**（spec §1 P1 / P18 条件4）⟹ State.ℓmax:Nat 有限 ⟹ 论域
     allCands 有限；买卖点 6 类（spec P4 §5 b_ℓ∈{0,1}^6）⟹ BspType 6 构造子；执行级 e≤ℓ
     （spec P4 §6 区间套前置）⟹ allCands 内 e∈[0,ℓ]；买卖点不互斥（P4 §5）⟹ 同 (ℓ,e) 多候选。
  3. 边界条件（结论翻转）：
     · **有限支撑（核心）**：|Γ|<∞ 依赖 State.ℓmax:Nat **有限**（spec §1 𝕃=ℤ 有限支撑）。若级别
       无界（ℓmax=∞ / 无有限支撑），allCands 无限，Γ 可无限 ⟹ |Γ|<∞ **翻转**。这是 spec 去根化
       级别系统"有限支撑"公理的有效域边界——本文件忠实编码（ℓmax:Nat），不臆造无限支撑语义。
     · **执行级范围 e≤ℓ**：allCands 内 e∈[0,ℓ]（区间套前置 e≤ℓ，spec P4 §6）。若放开 e>ℓ，
       N 退化为基例 Conf（NestingCertificate 截断），候选语义越出区间套定义域——当前严格 e≤ℓ。
     · **隶属门合取 vs 析取**：`member` = 买卖点触发 **∧** 区间套确认（spec §12 两者皆需）。若改
       析取（触发 ∨ 确认），Γ 膨胀 ⟹ 隶属翻转——当前合取（环1∧环2⟹环3）。
     · **买卖点单点 vs 向量**：候选 bsp 取**单个** BspType（"多买卖点"= 多候选，line 632）。若改
       为整向量 b_ℓ∈{0,1}^6 作单候选，候选粒度翻转（6 类合 1）——当前单点（更细，忠实"多买卖点"）。
  4. 下游推论：
     · `gamma : State → List Cand` 全定义 ⟹ 环5 R_Θ 解释器（lean-interp）可直接 fold 此 List
       产 (𝒟,ℬ,𝒦)——本文件提供其**有限输入**（line 621 唯一性第一依据"Γ(x) 有限"由本文件坐实）。
     · `Cand.dir`（Side）+ `Cand.roleOf`（接 classifyR18）⟹ 环4 可对每候选算 R(g)=(H,V,δ)——
       候选**带级别/方向/角色信息**，满足下游 fold 消费契约。
     · `gamma_length_le_universe` 显式上界 ⟹ 策略层可静态界定候选规模（≤ 3(ℓmax+1)(ℓmax+2)），
       无需运行时担心候选爆炸——支撑 R_Θ 排序/fold 的有限终止（环5 ≺_Θ insertionSort 有限）。
     · 与 `Origin.NestingCertificate`（环2 N^δ）/`Origin.OperationRole18`（环4 R）**互补**：本文件
       是七链环3 装配器，把环2 输出（N^δ:Bool）+ 环1 输出（b_ℓ:{0,1}^6）聚为 Γ，喂环4/环5。
  5. 谱系引用：
     · 缺口矩阵 `2026-06-28-lean-existing-vs-20page-gap-matrix.md` 环3 判"真缺口→新建（lean-gamma）"
       ——本文件即该缺口的填补（首个 Γ(x) 组装模块，此前候选块仅散见 IntervalNestCertificate
       的 `List NestLevel`，无"把每级 b+N^δ 聚成 Γ:List Cand"的模块）。
     · 触及记忆 `theta-v0-type3-boundary-reference-lean-conflict`（第三类边界冲突）：本文件 firedAt
       只读 LevelData 的 Bool，**不**触碰 B_3/S_3 的 >/≥ 边界，**不引入新冲突**（该冲突属买卖点
       谓词层待裁决项，不属本工位 escalate 范围）。
     · 触及记忆 `newchanlun-v1-fullwindow-l3-falsified`（v1 全窗 8/8 否证）：本文件是 L0 有限性
       结构定理，与 v1 实盘盈利性（L2/L3 经验命题）**认识论等级不同、不可互相否证**——不预判 alpha。
     · **不确定**是否有「候选集 Γ 有限性」的更早专属谱系记录——本工位不臆造谱系，明确标注此
       不确定性（建议 genealogist 核 `.chanlun/genealogy/` 确认环3 Γ 组装是否首次形式化）。
  6. 影响声明：新建 `Origin/CandidateSet.lean`，仅 import 两纯 core 兄弟文件
     （`Origin.NestingCertificate` 环2 + `Origin.OperationRole18` 环4），**无** Mathlib/Batteries/Std。
     **不改**任何既有 Lean/rust/定义/spec。namespace `NewChanlun.Origin.CandidateSet`（无命名冲突）。
     下游：为环5 R_Θ 解释器（lean-interp）提供有限候选集输入 + 环4 角色桥。不碰 lakefile
     （报 Lead 登记 root `Origin.CandidateSet`）。`lake env lean Origin/CandidateSet.lean` 单文件验证。
-/

end NewChanlun.Origin.CandidateSet
