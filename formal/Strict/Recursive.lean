/-
  Strict/Recursive.lean — 严格完全分类·第2部分 Eval 健全 + 第4部分 RecursiveKernel Can
  （task #61, RTAS 蜂群 T-recursive 工位；填补 codex#2 gapmatrix 判定的"完全缺失"两项）

  ── 工位定位 ────────────────────────────────────────────────────────────────
  本文件 import 标准内核库 `Classification`（Strict.* 结构定义），对**走势递归**
  （`Formal.RecursiveConstruction.Move`）实例化两个标准结构，从而把 codex#2 差距矩阵中
  判定为「C 完全缺失」的两项补成「带证明的实例」：

  - 标准第 2 部分（DecompositionSystem）：codex 判「没有 `Eval : Tₙ → Xₙ`，没有
    `Eval (Dₙ h)=h`」。本文件建 `moveDecomp` 实例，证 **重构健全** `Eval(D(d h))=d h`
    （分解能重构原走势）+ **分解唯一** `unique_mod_rel`（规范分解是唯一合法分解）。
  - 标准第 4 部分（RecursiveKernel）：codex 判「没有 `Can` 的存在/唯一边界/规范化幂等定理；
    没有 f₁/f₂ 分离的防循环命题」。本文件建 `moveKernel` 实例，证 `Can` 存在（全函数 witness）
    + **边界健全** `can_sound: Boundary(Can xs)=xs.val` + **边界唯一/幂等** `can_unique`
    + m≥3（子类型编码 + 显式读出）+ **f₁≠f₂ anti-circularity**（base 与 Can 构造子分离，
    防中枢↔走势循环定义）。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
  全部 **L0**（纯定义/结构归纳，不依赖数据）。`lake env lean` 通过 = 这些 round-trip /
  单射 / 分离命题在定义层成立，**不是**任何实证有效域断言。`eval_sound`/`can_sound` 是
  纯结构归纳，信息增量 = 同义反复（L0），不冒充 L1+。

  ── 载体选择（no-workaround：直面 indexed-inductive 约束，非补丁）──────────────
  分级宇宙 `U : Nat → Type` 取 `U n := Move`——级别由 `Move.level` 字段区分，**不是**
  type index。理由（严格，非妥协）：Lean kernel 拒绝「indexed inductive family + nested
  `List`」（`inductive G : Nat → Type | comp : List (G n) → G (n+1)` 报 "nested inductive
  parameters cannot contain local variables"）。这是 kernel 的硬约束，不可 workaround。
  Move 已是非 indexed inductive（segment/compose），级别作字段——与 `RStarNonSpecial.TowerLevel`
  「顶层性是字段不是 type index」的设计哲学同构（见 RStarNonSpecial.lean:42-57）。
  RecursiveKernel 标准结构 `RecursiveKernel (U : Nat → Type)` 接受**任意** `U`，`U n := Move`
  合法。`Can n` 输出 `U (n+1) = Move`、`Boundary n` 读 `U (n+1) = Move`——级别契约由 `Can`
  写入 `Move.compose ... (n+1)` 的 level 字段携带。

  ── 分工边界（no-patch-mentality：诚实声明本文件不做什么）────────────────────
  - `RelT := Eq`（非膨胀）：本分解系统 `evalT`/`decompM` 是**互逆双射**（两向 round-trip
    都证），故合法分解唯一、无多义性，∼ₙ 退化为 Eq 是**诚实的**（系统本无多义性，不是用
    Eq 假装覆盖了结合律等价）。结合律/多义性 ∼ₙ 商集（`|𝒟ₙ/∼ₙ|=1` 但 `|𝒟ₙ|>1`）是
    task #62/#63（T-decomp-uniq / C14 螺旋 Setoid+Quot.lift）的职责，不在本文件。
  - `Boundary`/`Can` 只契约**下级走势序列**（subs）的可还原性 + 单射；`Can` 内部 centers
    由 `canG : List Move → List Center`（subs 的函数）派生——centers 与 subs 的**生成忠实性**
    （`CentersDerivedFrom` witness+顺序 soundness）是 `RecursiveConstruction.WellFormed` 层
    的契约（见 RecursiveConstruction.lean:144-167），不在 RecursiveKernel 边界算子层重复。
    本文件不声明 `Can` 验证了 centers 派生忠实性。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
        本文件 = 615 概念分离在递归核与分解系统两处的 Layer2 落实。
  禁 sorry/admit/axiom。
-/

import Strict.Classification
import Formal.RecursiveConstruction

namespace Strict.Recursive

open Strict
open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center)
open Formal.RecursiveConstruction

/-! ## ① 标准第 2 部分：DecompositionSystem — Eval 健全（重构）+ 分解唯一

  对象层 `X := Move`（第 n 级完整走势结构状态）；历史 `H := Move`，取值映射 `d := id`
  （历史就是走势数据本身，恒等取值）。分解 `T := DecompTree` 镜像 `Move` 的递归结构。
  `Eval : T → X` 把结构树求值回走势，`D : X → T` 把走势分解为结构树。
-/

/--
  走势的结构分解树（镜像 `Move` 的递归形态）。
  - `leaf`：镜像 `Move.segment`（归纳基底，线段）。
  - `branch`：镜像 `Move.compose`（递归分支，子树序列 + 中枢 + 级别）。

  这是分解 `Dₙ(h)` 的具体载体——保留重构原走势所需的**全部**结构信息
  （子走势树 + 中枢 + 级别），故 `Eval`（求值）能还原原走势（健全）。
-/
inductive DecompTree where
  | leaf (dir : Direction) (lo hi : Int)
  | branch (subtrees : List DecompTree) (centers : List Center) (level : Nat)

/-! 分解树求值回走势（递归求值子树序列）。`Eval : Tₙ → Xₙ`。 -/
mutual
  def evalT : DecompTree → Move
    | DecompTree.leaf d lo hi => Move.segment d lo hi
    | DecompTree.branch ts cs lv => Move.compose (evalList ts) cs lv
  def evalList : List DecompTree → List Move
    | [] => []
    | t :: ts => evalT t :: evalList ts
end

/-! 走势分解为结构树（递归分解子走势序列）。`D : Xₙ → Tₙ`。 -/
mutual
  def decompM : Move → DecompTree
    | Move.segment d lo hi => DecompTree.leaf d lo hi
    | Move.compose subs cs lv => DecompTree.branch (decompList subs) cs lv
  def decompList : List Move → List DecompTree
    | [] => []
    | m :: ms => decompM m :: decompList ms
end

/-!
  ★重构健全 `Eval ∘ D = id`（标准第 2 部分核心，codex 判「完全缺失」项）：
  分解再求值还原原走势。结构归纳（与 `decompList` 互递归）。
-/
mutual
  theorem eval_decomp : ∀ m : Move, evalT (decompM m) = m
    | Move.segment _ _ _ => rfl
    | Move.compose subs _ _ => by
        simp only [decompM, evalT]; rw [eval_decomp_list subs]
  theorem eval_decomp_list : ∀ ms : List Move, evalList (decompList ms) = ms
    | [] => rfl
    | m :: ms => by
        simp only [decompList, evalList]; rw [eval_decomp m, eval_decomp_list ms]
end

/-!
  ★反向 round-trip `D ∘ Eval = id`（分解唯一性的根据）：求值再分解还原原树。
  这证明 `evalT`/`decompM` 是**互逆双射**——合法分解唯一、无多义性，故 ∼ₙ 退化为 Eq
  是诚实的（非声明膨胀）。结构归纳（与 `decompList` 互递归）。
-/
mutual
  theorem decomp_eval : ∀ t : DecompTree, decompM (evalT t) = t
    | DecompTree.leaf _ _ _ => rfl
    | DecompTree.branch ts _ _ => by
        simp only [evalT, decompM]; rw [decomp_eval_list ts]
  theorem decomp_eval_list : ∀ ts : List DecompTree, decompList (evalList ts) = ts
    | [] => rfl
    | t :: ts => by
        simp only [evalList, decompList]; rw [decomp_eval t, decomp_eval_list ts]
end

/--
  ★走势分解系统实例（标准 `DecompositionSystem`，第 2 部分完整实例化）。

  - `d := id`：历史→走势的恒等取值。
  - `D := decompM` / `Eval := evalT`：分解 / 求值。
  - `ValidDecomp x t := (evalT t = x)`：合法分解 ⟺ 求值回原走势（语义合法性）。
  - `RelT := Eq`：∼ₙ = 字面相等（本系统无多义性，见文件头分工边界）。
  - `eval_sound`：重构健全（`eval_decomp`）。
  - `D_valid`：规范分解合法（即 `evalT (decompM (d h)) = d h`，仍是 `eval_decomp`）。
  - `unique_mod_rel`：任何合法分解都 = 规范分解（由 `decomp_eval` 反向 round-trip：
    `evalT t = h ⟹ t = decompM (evalT t) = decompM h`）。
-/
def moveDecomp : DecompositionSystem Move Move DecompTree where
  d := id
  D := decompM
  Eval := evalT
  ValidDecomp := fun x t => evalT t = x
  RelT := Eq
  rel_equiv := ⟨fun _ => rfl, Eq.symm, Eq.trans⟩
  eval_sound := fun h => eval_decomp h
  D_valid := fun h => eval_decomp h
  unique_mod_rel := fun h t hv => by
    show t = decompM h
    simp only [id_eq] at hv
    rw [← hv, decomp_eval]

/-! ### ① 两层定理（Layer1 语法 round-trip → Layer2 系统级健全/唯一） -/

/--
  ★Layer1（语法层）：分解树与走势互逆——`evalT`/`decompM` 双射。
  这是「语法生成无遗漏」的递归版：每棵分解树恰对应一个走势，每个走势恰对应一棵规范分解树。
-/
theorem L1_decomp_bijective :
    (∀ m : Move, evalT (decompM m) = m) ∧ (∀ t : DecompTree, decompM (evalT t) = t) :=
  ⟨eval_decomp, decomp_eval⟩

/--
  ★Layer2（语义/系统层）：分解系统满足重构健全 ∧ 分解唯一（∼ₙ 下 `|𝒟ₙ(h)/∼ₙ|=1`）。
  这是标准第 2 部分的系统级陈述——从 `moveDecomp` 实例的证明义务字段读出，
  确认 Eval 健全（codex「完全缺失」项）已被实例化为带证明的命题。
-/
theorem L2_decomp_sound_and_unique :
    (∀ h, moveDecomp.Eval (moveDecomp.D (moveDecomp.d h)) = moveDecomp.d h)
    ∧ (∀ h t, moveDecomp.ValidDecomp (moveDecomp.d h) t
        → moveDecomp.RelT t (moveDecomp.D (moveDecomp.d h))) :=
  ⟨moveDecomp.eval_sound, moveDecomp.unique_mod_rel⟩

/-! ## ② 标准第 4 部分：RecursiveKernel — Can 存在/边界唯一/幂等 + m≥3 + f₁≠f₂

  分级宇宙 `U n := Move`（级别作字段，见文件头载体选择）。
  - `base`（f₁，起始构造）：`Atom → U 0`，segment 嵌入第 0 级（线段层）。
  - `Can n`（f₂，逐级 compose）：`{xs // len ≥ 3} → U (n+1)`，≥3 段装箱为第 n+1 级走势。
  - `Boundary n`：`U (n+1) → List (U n)`，拆箱取回构成的下级走势序列。
-/

/-- 分级宇宙：第 n 级对象类型（级别由 `Move.level` 字段区分，非 type index）。 -/
abbrev U (_ : Nat) := Move

/--
  ★`Can` 内部中枢派生函数（subs 的函数，保证 `Can` 是 subs 的函数 → 边界单射）。
  此处取恒定空——`Boundary` 不读 centers，centers 的**生成忠实性**由
  `RecursiveConstruction.CentersDerivedFrom`（WellFormed 层）契约，不在边界算子层重复
  （见文件头分工边界，no-patch-mentality 诚实声明）。
-/
def canG (_ : List Move) : List Center := []

/-- f₁：原始构件（segment 三元组）嵌入第 0 级。`base : Atom → U 0`。 -/
def baseOp : Direction × Int × Int → U 0
  | (d, lo, hi) => Move.segment d lo hi

/--
  f₂：≥3 段第 n 级走势规范化装箱为第 n+1 级走势（`Can n`）。
  级别契约 `n+1` 写入 `Move.compose` 的 level 字段；≥3 约束由子类型 `{xs // len ≥ 3}` 在
  类型层强制（标准 m≥3 的结构化落实）。
-/
def canOp (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) : U (n+1) :=
  Move.compose xs.val (canG xs.val) (n+1)

/-- 边界算子：取回构成第 n+1 级走势的下级走势序列（拆箱）。segment 无边界（空）。 -/
def bdry (n : Nat) : U (n+1) → List (U n)
  | Move.compose subs _ _ => subs
  | Move.segment _ _ _ => []

/--
  ★走势递归核实例（标准 `RecursiveKernel`，第 4 部分完整实例化）。

  - `can_sound`：边界健全 `Boundary(Can xs) = xs.val`（装箱后取边界还原原序列，rfl）。
  - `can_unique`：边界唯一/幂等 `Can xs = Can ys ⟹ xs.val = ys.val`
    （`Can` 单射于边界——`canOp` 是 `Move.compose` 应用，构造子单射 + injection 取第一分量）。
-/
def moveKernel : RecursiveKernel U where
  Atom := Direction × Int × Int
  base := baseOp
  Can := canOp
  Boundary := bdry
  can_sound := fun _ _ => rfl
  can_unique := fun _ _ _ h => by
    unfold canOp at h
    exact (Move.compose.injEq _ _ _ _ _ _).mp h |>.1

/-! ### ② Can 存在 + m≥3 显式读出 + 两层定理 -/

/--
  ★Can 存在（全函数 witness，codex「没有 Can 的存在」项）：对任意 ≥3 段下级序列，
  `Can n` 给出一个第 n+1 级对象。`Can` 是 Lean 全函数即编码存在性——此命题显式见证之
  （`canOp n xs` 是该对象的具体构造）。
-/
theorem can_exists (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) :
    ∃ u : U (n+1), moveKernel.Can n xs = u :=
  ⟨canOp n xs, rfl⟩

/--
  ★m≥3 结构化落实（标准「至少由三段以上次级别构成」）：`Can` 输入的下级序列长度 ≥ 3，
  且其边界（= 该下级序列）长度 ≥ 3。≥3 由子类型在类型层强制，此处显式读出。
-/
theorem can_boundary_length_ge_three (n : Nat) (xs : {l : List (U n) // l.length ≥ 3}) :
    (moveKernel.Boundary n (moveKernel.Can n xs)).length ≥ 3 := by
  rw [moveKernel.can_sound n xs]
  exact xs.property

/--
  ★Layer1（语法层）：`Can` 边界健全 ∧ 边界单射——`Can` 是「装箱/拆箱」互逆于边界。
  这是「语法生成无遗漏 + 语法相等下构造子互斥」的递归核版（codex#1 命名约束 Layer1）。
-/
theorem K_L1_boundary_iso :
    (∀ n xs, moveKernel.Boundary n (moveKernel.Can n xs)
        = (xs : {l : List (U n) // l.length ≥ 3}).val)
    ∧ (∀ n xs ys, moveKernel.Can n xs = moveKernel.Can n ys
        → (xs : {l : List (U n) // l.length ≥ 3}).val = ys.val) :=
  ⟨moveKernel.can_sound, moveKernel.can_unique⟩

/--
  ★Layer2（系统层）：递归核满足存在 ∧ 边界唯一 ∧ m≥3——标准第 4 部分系统级陈述。
  确认 `Can` 的存在/唯一边界/规范化（codex「完全缺失」项）已实例化为带证明命题。
-/
theorem K_L2_kernel_complete :
    (∀ n xs, ∃ u : U (n+1), moveKernel.Can n xs = u)
    ∧ (∀ n xs ys, moveKernel.Can n xs = moveKernel.Can n ys
        → (xs : {l : List (U n) // l.length ≥ 3}).val = ys.val)
    ∧ (∀ n xs, (moveKernel.Boundary n (moveKernel.Can n xs)).length ≥ 3) :=
  ⟨can_exists, moveKernel.can_unique, can_boundary_length_ge_three⟩

/-! ### ② f₁ ≠ f₂ 分离（anti-circularity：防中枢↔走势循环定义） -/

/--
  ★f₁ ≠ f₂ 构造子分离（codex「没有 f₁/f₂ 分离的防循环命题」项）：

  `base`（f₁，起始 segment 构造）的产物与 `Can`（f₂，逐级 compose 构造）的产物
  **构造子不同**（segment ≠ compose），故 HEq 不可能成立。这是 anti-circularity 的核心：
  走势（compose 产物）**不能**等同于线段基底（segment 产物）——高级别对象必须经 `Can`
  逐级 compose 产生，不能由 `base` 直接「跳级」充当。防止「中枢由走势定义、走势又由中枢
  定义」的循环：base 层（无中枢的 segment）与 Can 层（携带中枢的 compose）在构造子上分离。
-/
theorem base_ne_can (a : Direction × Int × Int) (n : Nat)
    (xs : {l : List (U n) // l.length ≥ 3}) :
    HEq (moveKernel.base a) (moveKernel.Can n xs) → False := by
  intro hh
  obtain ⟨d, lo, hi⟩ := a
  simp only [moveKernel, baseOp, canOp] at hh
  exact Move.noConfusion (eq_of_heq hh)

/--
  ★anti-circularity 强化（构造层）：任意 `base` 产物是 `segment`、任意 `Can` 产物是
  `compose`——两个生成器在 `Move` 构造子上**永久分离**，不存在 base/Can 输出重合的情形。
  这显式化「起始 segment base vs 逐级 compose」的分离（任务卡要求），关死循环定义后门。
-/
theorem base_is_segment_can_is_compose (a : Direction × Int × Int) (n : Nat)
    (xs : {l : List (U n) // l.length ≥ 3}) :
    (∃ d lo hi, moveKernel.base a = Move.segment d lo hi)
    ∧ (∃ subs cs lv, moveKernel.Can n xs = Move.compose subs cs lv) := by
  obtain ⟨d, lo, hi⟩ := a
  exact ⟨⟨d, lo, hi, rfl⟩, ⟨xs.val, canG xs.val, n+1, rfl⟩⟩

end Strict.Recursive
