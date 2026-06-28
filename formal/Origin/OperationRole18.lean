/-
  Origin/OperationRole18.lean — 18 类操作角色完全互斥分类 R(g)=(H(g),V(g),δ_g)
  （递归完全分类买卖点 20 页权威版 §7-§8，P6-P7）

  ════════════════════════════════════════════════════════════════════════════
  结果包六要素（result-package 规则强制）
  ════════════════════════════════════════════════════════════════════════════

  【1. 结论】
    候选事件的完整操作角色形式化为三轴笛卡尔积
        R(g) = (H(g), V(g), δ_g) ∈ ℛ = RoleH × RoleV × Side
    其中
        H ∈ {First, SameFollow, SameReverse}      （水平/同级兄弟关系，3 类）
        V ∈ {Ambient, FollowParent, ShortDiff}    （垂直/父容器关系，3 类）
        δ ∈ {long(+1=买入), short(-1=卖出)}        （方向，2 值；复用 Side）
    基数 |ℛ| = 3×3×2 = 18（`card18`）。严格互斥完全分类
        Σ_{r∈ℛ} 1[R(g)=r] = 1
    形式化为 `role18_count_one : ∀ r:Role18, allR18.count r = 1`（List 显式枚举 + decide）。
    分类器 `classifyR18`（去根化的真 R(g)）的产出亦满足此式（`classifyR18_count_one`）。
    **去根化**：无 RootRole/根特例——根被边界胚元 ∂（父方向 σ_p=0）+ Ambient 吸收
    （`classifyV_germ_ambient`：父为胚元 none ⟹ V=Ambient，非另立 Root 构造子）。

  【2. 定义依据】（引 spec §7-§8，P6-P7）
    - H(g)（P6 §7.1 方框）：prev(g)=∅ ⟹ First；prev(g)≠∅ ∧ δ_g=σ_{prev(g)} ⟹ SameFollow；
      prev(g)≠∅ ∧ δ_g=-σ_{prev(g)} ⟹ SameReverse。穷尽性依赖 δ_g,σ_{prev}∈{+1,-1} 二值
      （有兄弟时非顺即反）。Σ_h 1[H=h]=1（P6 方框）→ `roleH_count_one`。
    - V(g)（P6 §7.2 方框）：σ_{p(g)}=0 ⟹ Ambient；σ_{p(g)}≠0 ∧ δ_g=σ_{p(g)} ⟹ FollowParent；
      σ_{p(g)}≠0 ∧ δ_g=-σ_{p(g)} ⟹ ShortDiff。三分依赖 σ_{p(g)}∈{-1,0,+1} 含 0（父无方向→
      Ambient）。Σ_v 1[V=v]=1（P6 方框）→ `roleV_count_one`。本形式化把 σ_{p(g)}∈{-1,0,+1}
      建模为 `Option Side`：none=σ_p=0（胚元/无方向），some s=σ_p=±1（已定向）——去根化的载体。
    - R(g)=(H,V,δ)（P7 §8 方框）+ ℛ 笛卡尔积 + Σ_{r∈ℛ}1[R(g)=r]=1（P7 方框）→ 本文件主定理。
    - ShortDiff（P8 §9 方框）：ShortDiff(g) ⟺ σ_{p(g)}≠0 ∧ δ_g=-σ_{p(g)}（父级反向子操作，
      非绝对做空）→ `classifyV_shortDiff_iff`（δ=flipSide σ）。
    - 方向 δ：spec δ∈{+1,-1}，+1=买入(P4 §6)、-1=卖出。复用 `Origin.SourceAxioms.Side`
      （long=+1=买入，short=-1=卖出），不新造方向类型。"-σ" 用 `flipSide`（二值取反）。

  【3. 边界条件】（结论翻转的条件）
    - 18=3×3×2 的完备性依赖三轴各自互斥穷尽（H、V 由 P6 已证，δ 二值平凡）。若三轴间存在
      不可同时取值的耦合（某 (h,v,δ) 组合在语义上不可达），则**经验有效类 <18**——这是
      有效域<定义域（疑点5，见下 L2 标注），但代数完全分类 Σ=1 不翻转（L0 同义反复仍成立）。
    - H 穷尽性翻转条件：若 δ_g 或 σ_{prev} 允许取 {+1,-1} 以外的值（如 0），则"有兄弟时非顺即
      反"失效，H 需第四类。本模型 δ:Side 严格二值，σ_{prev}:Side（兄弟必已定向）。
    - V 三分翻转条件：若 σ_{p(g)} 取 {-1,0,+1} 以外更多值，V 分类翻转。本模型 Option Side 恰
      对应三值。**关键非对称**：σ_{p(g)} 三值（含 0=胚元），δ_g 二值（无 0）。
    - 去根化翻转条件：若为最高前沿/根开"无父"特例分支（RootRole 构造子），则违反 P4/P7 去根化
      定义（声明翻转）。本文件**结构上**无 Root 构造子——根特例不可表达。

  【4. 下游推论】
    - Rust classify 模块输出 (H,V,δ) 三枚举元组，**禁** RootRole 枚举值（spec Rust 指引 P6-7 行）。
    - 角色三轴是塔导出（LeveledMove 塔 / B2S2 接入）的前置对象升级——见谱系引用。
    - 互斥责任分层（spec 核心严格性升级 2）：互斥性在**角色层 R(g)（18 类，本文件）严格成立**，
      而非买卖点谓词层 b_ℓ（64 类不互斥，P4 §5）；解释器层 R_Θ 再用总序唯一化（P10 §12）。
    - 短差腿方向由父方向取负计算（`flipSide`），**不**硬编码 -1（spec P8 下游推论）。

  【5. 谱系引用】
    - **核心差异 vs 15 页版**：本 20 页版明确"买卖点不互斥(64类)"，互斥性下移到角色层(18类)。
      与 MEMORY `coverage-engine-needs-tower-export-bridge`"互斥全定义策略=买卖点入场+多级
      角色/嵌套对冲"吻合——18 类角色 = 多级角色框架的形式化。
    - **去根化谱系**：本文件相对既有 `Origin.OperationRole`（旧 MW3 四分类，含 RootDir 特例）的
      对象升级——旧版单轴 levelRelation∈{Root,Same,Sub}，本版双正交轴 H(同级兄弟)×V(父容器)+δ，
      根特例被 Ambient(σ_p=0) 吸收。与既有 `Origin.RootSelDisambig` 去根化主线一致。
    - ShortDiff 真子声部腿仍需真嵌套塔（MEMORY 同条）——本文件只形式化角色分类，不导出塔
      （塔导出是独立工位/对象升级，本文件不声明该能力）。
    - 与既有 `Origin.OperationRole` 的关系详见文末注释（并存，非替代/扩展——见"与旧文件关系"）。

  【6. 影响声明】
    - 新增文件 `Origin/OperationRole18.lean`（root 名 `Origin.OperationRole18`，待 Lead 登记
      lakefile.toml roots——本工位不编辑 lakefile）。
    - 仅 import `Origin.SourceAxioms`（取 `Side` 方向类型）。不 import、不修改既有
      `Origin.OperationRole`（旧四分类并存）。不碰其他文件。
    - 影响下游：角色层定义基底（classify 输出类型、R_Θ 解释器输入、活动集腿角色标注）。

  ════════════════════════════════════════════════════════════════════════════
  认识论等级标注（formalization-validity-domain 231号强制）
  ════════════════════════════════════════════════════════════════════════════
    - 【L0】18 类 Σ_{r∈ℛ}1[R(g)=r]=1（`role18_count_one`）、|ℛ|=18（`card18`）、H/V 各 3 类
      Σ=1、δ 2 值——全是**代数同义反复**（笛卡尔积纤维计数 + 归纳类型构造子穷尽），不依赖
      任何市场数据，信息增量为零。分类器 `classifyR18` 的完全性同为 L0（函数全定义 ⟹ 落唯一类）。
    - 【L2 未覆盖】各 18 类的**经验可达性**（哪些 (h,v,δ) 组合在真实数据上实际出现，还是部分
      组合经验为空 = 有效域<定义域）——PDF 仅 L0 声明（疑点5），本文件**不声称**任何 L1+ 经验
      有效性。"完全分类代数成立"≠"18 类全部经验可达"。禁声明膨胀（090号/231号）。

  ── 纯 core 约束（无 Mathlib/Batteries/Std）─────────────────────────────────────
    完备性证明只用 List 显式枚举 + decide（构造子 cases + 计算）。严禁 Fintype/Finset/
    Fintype.card/Finset.sum。验证：`cd formal && lake env lean Origin/OperationRole18.lean`。
    零 sorry/admit/axiom。简体中文。
-/

import Origin.SourceAxioms

namespace NewChanlun.Origin

/-! ════════════════════════════════════════════════════════════════════════
    § 1. 三轴归纳类型 + 方向取反（spec P6 §7.1/§7.2，P4 §6 方向）

    水平 H、垂直 V 各为 3 构造子归纳类型——3 分互斥穷尽是归纳类型的结构必然。
    方向 δ 复用 `Origin.SourceAxioms.Side`（long=+1=买入，short=-1=卖出），不新造。
    `flipSide` = spec 的 "-σ"（二值取反），用于 SameReverse / ShortDiff 的反向条件。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **水平关系 H(g)（spec P6 §7.1 方框，M-H）** —— 候选事件相对同级前兄弟 prev(g) 的关系：
  - `first`：prev(g)=∅（无前兄弟）。**去根化点**：这不是"根"，是任意级别第一个兄弟。
  - `sameFollow`：prev(g)≠∅ ∧ δ_g = σ_{prev(g)}（同级顺向）。
  - `sameReverse`：prev(g)≠∅ ∧ δ_g = -σ_{prev(g)}（同级反向）。
  ★三构造子 ⟹ 互斥穷尽结构必然。`DecidableEq`/`BEq` ⟹ List.count 可机器计算。L0。 -/
inductive RoleH where
  /-- 无前兄弟 prev(g)=∅（P6 §7.1）。 -/
  | first
  /-- 同级顺向 δ_g=σ_{prev(g)}（P6 §7.1）。 -/
  | sameFollow
  /-- 同级反向 δ_g=-σ_{prev(g)}（P6 §7.1）。 -/
  | sameReverse
deriving DecidableEq, BEq, Repr

/--
  **垂直关系 V(g)（spec P6 §7.2 方框，M-V）** —— 候选事件相对父容器方向 σ_{p(g)}∈{-1,0,+1} 的关系：
  - `ambient`：σ_{p(g)}=0（父容器无方向/胚元 ∂）。**去根化核心**：spec P7"Ambient 不是根规则，
    而是任意父容器处于无方向状态时的普通情形"——根特例被此构造子吸收。
  - `followParent`：σ_{p(g)}≠0 ∧ δ_g = σ_{p(g)}（顺父方向次级别腿）。
  - `shortDiff`：σ_{p(g)}≠0 ∧ δ_g = -σ_{p(g)}（短差=父级反向子操作，P8 §9）。
  ★三构造子 ⟹ 互斥穷尽结构必然。L0。 -/
inductive RoleV where
  /-- 父无方向 σ_{p(g)}=0（胚元 ∂；P6 §7.2 / P7 去根化）。 -/
  | ambient
  /-- 顺父方向 δ_g=σ_{p(g)}（P6 §7.2）。 -/
  | followParent
  /-- 短差 δ_g=-σ_{p(g)}（P6 §7.2 / P8 §9）。 -/
  | shortDiff
deriving DecidableEq, BEq, Repr

/--
  **方向取反 flipSide（spec 的 "-σ"，二值取反）** —— long↔short。Side 二值下 "δ=-σ" ⟺ "δ≠σ"
  （`ne_iff_eq_flipSide`），用于把 spec 原文的 `δ_g=-σ_{...}`（SameReverse / ShortDiff）忠实表达。 -/
def flipSide : Side → Side
  | Side.long => Side.short
  | Side.short => Side.long

/-- ★flipSide 必改变（L0）：flipSide σ ≠ σ（二值取反无不动点）。 -/
theorem flipSide_ne (σ : Side) : flipSide σ ≠ σ := by cases σ <;> decide

/-- ★二值取反等价不等（L0）：δ = -σ ⟺ δ ≠ σ（Side 仅两值）。坐实 spec "δ=-σ" 与判据用的
    "δ≠σ" 在方向二值下等价（对应 §8 P8 短差 δ_g=-σ_{p(g)}）。 -/
theorem ne_iff_eq_flipSide (δ σ : Side) : δ ≠ σ ↔ δ = flipSide σ := by
  cases δ <;> cases σ <;> decide

/-! ════════════════════════════════════════════════════════════════════════
    § 2. 完整角色 R(g)=H×V×δ（spec P7 §8 方框）

    `Role18` = 三轴积结构体（h:RoleH, v:RoleV, d:Side）。基数 3×3×2=18。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **完整操作角色 R(g)=(H(g),V(g),δ_g)（spec P7 §8 方框，★核心）** —— 三轴笛卡尔积，无根特例。
  ★`DecidableEq`/`BEq` ⟹ 互斥完全分类用 List.count + decide 机器可证。L0。 -/
structure Role18 where
  /-- 水平关系 H(g)（P6 §7.1）。 -/
  h : RoleH
  /-- 垂直关系 V(g)（P6 §7.2）。 -/
  v : RoleV
  /-- 方向 δ_g（P4 §6；long=+1=买入，short=-1=卖出）。 -/
  d : Side
deriving DecidableEq, BEq, Repr

/-! ════════════════════════════════════════════════════════════════════════
    § 3. 显式枚举 + 完全互斥分类 Σ=1（纯 core：List 枚举 + decide）

    spec P7 §8：ℛ = {First,SameFollow,SameReverse}×{Ambient,FollowParent,ShortDiff}×{+1,-1}，
    |ℛ|=18，Σ_{r∈ℛ}1[R(g)=r]=1。**禁** Fintype.card——用 allR18 显式 18 元 + List.count + decide。
    ════════════════════════════════════════════════════════════════════════ -/

/-- 水平 H 三类显式枚举（spec P6 §7.1）。 -/
def allH : List RoleH := [RoleH.first, RoleH.sameFollow, RoleH.sameReverse]

/-- 垂直 V 三类显式枚举（spec P6 §7.2）。 -/
def allV : List RoleV := [RoleV.ambient, RoleV.followParent, RoleV.shortDiff]

/-- 方向 δ 二值显式枚举（spec P4 §6）。 -/
def allDir : List Side := [Side.long, Side.short]

/-- **完整角色空间 ℛ 显式枚举（spec P7 §8）** —— allH×allV×allDir 的 18 元笛卡尔积
    （flatMap 展开三轴积，结构忠实于 R=H×V×δ）。 -/
def allR18 : List Role18 :=
  allH.flatMap (fun h => allV.flatMap (fun v => allDir.map (fun d => ⟨h, v, d⟩)))

/-- ★|ℛ|=18（L0，spec P7 §8）：角色空间基数 3×3×2=18。List 显式枚举长度，decide 计算。 -/
theorem card18 : allR18.length = 18 := by decide

/-- ★水平 H 完全互斥分类 Σ_h 1[H(g)=h]=1（L0，spec P6 §7.1 方框）：每个 H 值在 allH 中恰
    出现一次（穷尽=至少一次 + 互斥=至多一次）。 -/
theorem roleH_count_one (h : RoleH) : allH.count h = 1 := by
  cases h <;> decide

/-- ★垂直 V 完全互斥分类 Σ_v 1[V(g)=v]=1（L0，spec P6 §7.2 方框）：每个 V 值在 allV 中恰一次。 -/
theorem roleV_count_one (v : RoleV) : allV.count v = 1 := by
  cases v <;> decide

/-- ★方向 δ 完全分类 Σ_δ 1[δ_g=δ]=1（L0，spec P4 §6）：每个方向在 allDir 中恰一次。 -/
theorem dir_count_one (d : Side) : allDir.count d = 1 := by
  cases d <;> decide

/--
  ★★**18 类严格互斥完全分类 Σ_{r∈ℛ} 1[R(g)=r] = 1（L0，spec P7 §8 方框，★核心主定理）** ——
  对任意角色值 r∈ℛ，r 在 allR18 中恰出现一次。这同时给出：
  - 完全（穷尽）：count ≥ 1 ⟹ r∈allR18（每个 (h,v,δ) 组合都在枚举中）；
  - 互斥（无重复）：count ≤ 1 ⟹ allR18 无重复 r。
  由于任意候选事件的 R(g) 恒是某个 r∈ℛ，此式即 spec 的"对任意 g，Σ_{r∈ℛ}1[R(g)=r]=1"。
  证明：18 个构造子组合 cases + decide（纯 core，无 Fintype）。L0 同义反复。 -/
theorem role18_count_one (r : Role18) : allR18.count r = 1 := by
  cases r with
  | mk h v d => cases h <;> cases v <;> cases d <;> decide

/-- ★完全性推论（L0）：每个角色 r∈ℛ 都在枚举 allR18 中（spec P7 §8 完全分类的"穷尽"侧）。
    用计算的 `List.elem`（BEq，Bool）表达成员——纯 core 下 `r ∈ allR18`（Prop/List.Mem）无
    自动 Decidable 实例，故用 `allR18.elem r = true`（等价穷尽断言，decide 可计算）。 -/
theorem role18_mem (r : Role18) : allR18.elem r = true := by
  cases r with
  | mk h v d => cases h <;> cases v <;> cases d <;> decide

/-! ════════════════════════════════════════════════════════════════════════
    § 4. 角色分类器 R(g)（spec P6 §7.1/§7.2，去根化的真 R(g)）

    spec 把 R(g) 定义为从关系上下文（前兄弟、父容器方向、自身方向）派生的函数。
    去根化：父容器方向 σ_{p(g)} 用 `Option Side` 建模——none = σ_p=0（胚元 ∂，无方向），
    some s = σ_p=±1（已定向）。**无 Root 构造子**：根（最高前沿）的父是胚元 ∂ → V=Ambient。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  **水平分类器 H(g)（spec P6 §7.1 方框）** —— prevDir = 前兄弟方向 σ_{prev(g)}（Option：none=无
  前兄弟 prev(g)=∅）；δ = δ_g：
  - none ⟹ First；some σ ∧ δ=σ ⟹ SameFollow；some σ ∧ δ≠σ(=−σ) ⟹ SameReverse。全函数。 -/
def classifyH (prevDir : Option Side) (δ : Side) : RoleH :=
  match prevDir with
  | none => RoleH.first
  | some σ => if δ = σ then RoleH.sameFollow else RoleH.sameReverse

/--
  **垂直分类器 V(g)（spec P6 §7.2 方框，去根化载体）** —— parentDir = 父容器方向 σ_{p(g)}
  （Option：none = σ_p=0 胚元/无方向 ∂）；δ = δ_g：
  - none ⟹ Ambient（去根化：父胚元 → Ambient，非 Root）；some σ ∧ δ=σ ⟹ FollowParent；
    some σ ∧ δ≠σ(=−σ) ⟹ ShortDiff（P8 §9 父级反向子操作）。全函数。 -/
def classifyV (parentDir : Option Side) (δ : Side) : RoleV :=
  match parentDir with
  | none => RoleV.ambient
  | some σ => if δ = σ then RoleV.followParent else RoleV.shortDiff

/-- **完整分类器 R(g)=(H(g),V(g),δ_g)（spec P7 §8）** —— 去根化的真 R(g)，从前兄弟方向、
    父容器方向、自身方向派生。全函数（定义域=所有关系上下文）。 -/
def classifyR18 (prevDir parentDir : Option Side) (δ : Side) : Role18 :=
  ⟨classifyH prevDir δ, classifyV parentDir δ, δ⟩

/-- ★★分类器完全互斥分类（L0，spec P7 §8）：任意关系上下文下，真 R(g)=classifyR18 落入 ℛ 中
    恰一类（count=1）。这把 `role18_count_one`（对所有 r）落到 R(g) 这个具体函数值上——
    坐实"对任意 g，Σ_{r∈ℛ}1[R(g)=r]=1"。 -/
theorem classifyR18_count_one (prevDir parentDir : Option Side) (δ : Side) :
    allR18.count (classifyR18 prevDir parentDir δ) = 1 :=
  role18_count_one _

/-! ── 去根化桥 + ShortDiff 忠实（spec P7 Ambient / P8 §9 短差）──────────────────────── -/

/-- ★★去根化桥（L0，spec P4/P7）：父为胚元 ∂（parentDir=none，σ_p=0）⟹ V=Ambient。
    坐实"根级别不是特殊级别，它只是父级尚为胚元的普通子级别"——**无 Root 构造子**，
    旧 `Origin.OperationRole.rootDir` 特例在此被 Ambient 吸收。 -/
theorem classifyV_germ_ambient (δ : Side) : classifyV none δ = RoleV.ambient := rfl

/-- ★去根化桥（L0，spec P6 §7.1）：无前兄弟 prev(g)=∅（prevDir=none）⟹ H=First（非根特例，
    任意级别首兄弟）。 -/
theorem classifyH_first (δ : Side) : classifyH none δ = RoleH.first := rfl

/-- ★ShortDiff 忠实判据（L0，spec P8 §9 方框）：V=ShortDiff ⟺ 父已定向(some σ) ∧ δ_g=-σ_{p(g)}
    （flipSide σ）。坐实"短差不是绝对做空，而是父级方向的反向子操作"。 -/
theorem classifyV_shortDiff_iff (σ δ : Side) :
    classifyV (some σ) δ = RoleV.shortDiff ↔ δ = flipSide σ := by
  unfold classifyV
  cases σ <;> cases δ <;> decide

/-- ★父多头短差=做空（L0，spec P8 §9）：σ_{p(g)}=long(+1) ∧ δ_g=short(-1) ⟹ V=ShortDiff。 -/
theorem classifyV_long_parent_short_is_shortDiff :
    classifyV (some Side.long) Side.short = RoleV.shortDiff := rfl

/-- ★父空头短差=做多（L0，spec P8 §9）：σ_{p(g)}=short(-1) ∧ δ_g=long(+1) ⟹ V=ShortDiff。 -/
theorem classifyV_short_parent_long_is_shortDiff :
    classifyV (some Side.short) Side.long = RoleV.shortDiff := rfl

/-- ★FollowParent 忠实判据（L0，spec P6 §7.2）：V=FollowParent ⟺ 父已定向 ∧ δ_g=σ_{p(g)}。 -/
theorem classifyV_followParent_iff (σ δ : Side) :
    classifyV (some σ) δ = RoleV.followParent ↔ δ = σ := by
  unfold classifyV
  cases σ <;> cases δ <;> decide

/-! ════════════════════════════════════════════════════════════════════════
    § 5. 与既有 Origin.OperationRole（旧 MW3 四分类）的关系：并存（去根化对象升级）

    本文件 **不 import、不修改** `Origin.OperationRole`。两者并存，是同一概念（操作角色）的
    两套分类方案，由 20 页权威版 spec 的去根化重轴化区分：

    | 维度 | 旧 Origin.OperationRole（MW3 四分类） | 本文件 Origin.OperationRole18（18 类） |
    |------|--------------------------------------|----------------------------------------|
    | 分类轴 | 单轴 levelRelation∈{Root,Same,Sub}×方向 | 双正交轴 H(同级兄弟)×V(父容器)×δ |
    | 类数 | 4（RootDir/SameDir/SubFollow/ShortDiff） | 18（3×3×2） |
    | 根处理 | **RootDir 特例**（ρ_e=Root 独立做多/空） | **去根化**：无 Root 构造子，σ_p=0→Ambient |
    | 同级兄弟 | 不区分（Same 不看 prev 兄弟方向） | First/SameFollow/SameReverse 三分 |
    | 权威 | 旧版（15 页"操作语义"层可能假设互斥） | 20 页权威版（§7-8，凡冲突以本版为准） |

    旧版 `rootDir`（ρ_e=Root）≈ 本版 `RoleV.ambient`（父无方向/胚元）——旧的根特例在本版被
    边界胚元 ∂ + Ambient 吸收（spec P1-P7 去根化主线）。旧版 `shortDiff`（Sub∧反向）对应本版
    `RoleV.shortDiff`（父定向∧δ=-σ_p）；旧版 `subFollow` 对应 `RoleV.followParent`；旧版 `sameDir`
    （Same，不分兄弟方向）在本版被 H 轴细化为 SameFollow/SameReverse。

    **关系判定：并存（coexist）**，非替代/扩展——
    - 非"替代"：旧 OperationRole 被既有下游（MutexExhaustive/MutexRecursive/MutexFinalTheorem 等
      lakefile roots）依赖，删它越权且破坏既有 GREEN（no-patch：不碰契约锚定的复用原语，见 MEMORY
      newchanlun-no-patch-keep-primitive-when-contract-anchored）。
    - 非"扩展"：18 类不是 4 类的子类型加细（轴系不同——旧单轴 vs 新双轴），无法用 import+派生扩展。
    - 是"并存对象升级"：本文件是 20 页权威版去根化角色层的**新 canonical**，旧文件保留为 MW3
      历史/既有下游锚点。下游迁移到 18 类由后续工位决定（本工位不越权改下游）。
    ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin
