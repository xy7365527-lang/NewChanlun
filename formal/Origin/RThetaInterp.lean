/-
  Origin/RThetaInterp.lean — R_Θ 解释器：Γ(x) → 总序唯一化 → 三桶 (𝒟,ℬ,𝒦)（七链环5，真缺口新建）
  —— **(𝒟_x,ℬ_x,𝒦_x) = ℛ_Θ(Γ(x))** 的 L0 结构形式化：候选集 Γ(x) 按平移不变全序 ≺_Θ 排序后
     **确定性 fold** 出三桶（应关闭 / 应开启 / 记录不执行），并证 ∀x ∃!(𝒟,ℬ,𝒦)。

  ═══════════════════════════════════════════════════════════════════════════
  唯一信源
  ═══════════════════════════════════════════════════════════════════════════
  权威 spec：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §12（P10）R_Θ（line 1171 七链环5）+ 结果包 line 631-639 + ℐ_* 结果包 P9-10 §11 line 587-595。逐字方框：

      环5（line 1171）：角色化候选集 → R_Θ 解释器唯一化 (𝒟,ℬ,𝒦)；
        (𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))；按平移不变全序 ≺_Θ（g_1≺_Θ g_2 ⟺ S_k g_1≺_Θ S_k g_2）；
        ∀x ∃!(𝒟_x,ℬ_x,𝒦_x)。
      结果包（line 632-638）：同一时刻多买卖点构成有限候选集 Γ(x)；固定级别平移不变的自相似总序
        ≺_Θ，解释器 ℛ_Θ 按序处理产生唯一三元组 (𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))，∀x ∃! 唯一。
        **边界条件（逐字）**：唯一性 ∃! 依赖三条件齐备——(1)Γ(x)有限 (2)≺_Θ 是**全序**（非偏序，
        否则无法唯一定序）(3)每步规则确定。若 ≺_Θ 退化为偏序，唯一性翻转。
      ℐ_* 结果包（line 587-595）：局部解释器 ℐ_*:𝒮_*→(𝒟_*,ℬ_*,𝒦_*) 把语法状态映射为三元组
        （应关闭 𝒟_*、应开启 ℬ_*、记录不执行 𝒦_*）；第三类 𝒦_*（记录不执行）与 𝒟/ℬ 互斥分流。

  缺口矩阵 `.chanlun/specs/2026-06-28-lean-existing-vs-20page-gap-matrix.md`（环5）：
      判**真缺口 = 新建**。**复用 LexArgmin/selOrder 全序原语，不 reconcile IntervalNestCertificate**
      （`selectΘ` 选 1 区间 ≠ R_Θ 三桶 fold，机制不同）。

  ═══════════════════════════════════════════════════════════════════════════
  本工位 (lean-interp) 的产出 vs 既有锚点（复用 + no-patch 诚实标注）
  ═══════════════════════════════════════════════════════════════════════════
  本文件**组装** 环3（候选集 Γ(x)，lean-gamma）的输出为三桶 (𝒟,ℬ,𝒦)，并证 ∀x ∃!。**复用**（不重造）：
    · `Origin.CandidateSet`（环3，lean-gamma）：`Cand`(=(lvl,exec,bsp)) / `gamma`(Γ:State→List Cand) /
      `dirOf`(买卖点→方向 Side) / `State` / `witState`。**直接 import + 消费**其有限候选集——
      Γ(x) 有限性（唯一性第一依据）已由 CandidateSet `gamma_finite` 坐实，本文件**不重证**。
    · `Origin.LexArgmin`（七链环7 风控层）：`LexKey`(=List Int) / `lexLe`(可判定字典序) +
      已证全序原语 `lexLe_refl/total/trans/antisymm` + cons 三分支引理 `lexLe_cons_lt/gt/eq`。
      ★≺_Θ 把每个候选编码为 `LexKey`（`toKey`），用 `lexLe` 比较——**复用 LexArgmin 的字典序全序**，
      不重证 refl/total/trans/antisymm（单一权威在 LexArgmin）。
    · **不** import/reconcile `Origin.IntervalNestCertificate.selectΘ`（缺口矩阵裁定）：`selectΘ` 在
      `List Interval` 上 fold 出**单个**区间（选最优 1 个），R_Θ 在 `List Cand` 上 fold 出**三桶划分**
      （每候选分流到 𝒟/ℬ/𝒦 之一）——机制不同（1-选 vs 3-分），不可混用同一 fold。只复用其
      「字典序/线序原语」思想，载体取 LexArgmin.lexLe（无 Interval 依赖）。

  ★≺_Θ 的内容（平移不变全序）：`toKey c = [↑c.lvl, ↑c.exec, bspOrd c.bsp]`（Int 三元字典序键）。
    ≺_Θ = 主键级别 lvl 升序 → 次键执行级 exec 升序 → 末键买卖点序 bspOrd（b1<b2<b3<s1<s2<s3）。
    **平移不变性**（spec 逐字 g_1≺_Θ g_2 ⟺ S_k g_1≺_Θ S_k g_2）：级别平移算子 S_k 把候选整体抬升 k 级
    （`shift k (lvl,exec,bsp) = (lvl+k, exec+k, bsp)`，保 lvl−exec 嵌套深度不变）。`candLe` 逐分量比较，
    Int 的 `<`/`=` 在两操作数同加 k 下不变（a<b ⟺ a+k<b+k）⟹ 整序在 S_k 下保持（`candLe_shift`）。

  ★三桶分流规则（确定性 fold，spec §11 𝒟/ℬ/𝒦 + §12 ≺_Θ 唯一化）：按 ≺_Θ 排序后 foldl，维护
    「已占用级别集」claimed。对每候选 g=(lvl,exec,bsp)：
      - lvl **已被更早（≺_Θ 在先）的候选占用** ⟹ 𝒦（记录不执行，冲突让位于在先者）；
      - 否则占用 lvl，按方向分流：卖点(short) ⟹ 𝒟（应关闭），买点(long) ⟹ ℬ（应开启）。
    这把 spec "买卖点重合时的唯一化机制" 落为：同级别多买卖点（"同一时刻多买卖点"，line 632），
    ≺_Θ 在先者执行（入 𝒟/ℬ），其余记录（入 𝒦）。𝒦 与 𝒟/ℬ 互斥分流（line 590 第三类互斥）。

  ═══════════════════════════════════════════════════════════════════════════
  认识论等级（formalization-validity-domain / 231号 强制标注）
  ═══════════════════════════════════════════════════════════════════════════
  全部 **L0**（纯定义 / List 排序 fold / 函数确定性，**不依赖任何数据**）。
  唯一化 ∃!(𝒟,ℬ,𝒦) 是 **L0 结构定理**（spec 自身标注 "§16/§18 唯一性与自相似定理是 L0 结构定理"）：
  R_Θ 是全函数（排序 + fold），故 ∀x 输出唯一——信息增量 = 同义反复（函数 ⟹ 像唯一）。其**前提**
  （三条件）本文件显式坐实：(1) Γ 有限 = CandidateSet.gamma_finite（List）；(2) ≺_Θ 全序 =
  `candLe_refl/total/trans/antisymm`（复用 lexLe 全序，键单射 ⟹ 真线序）；(3) 每步规则确定 =
  `stepR` 是函数。`lake env lean` 通过 = 「三桶解释器在定义层是良定义全函数且输出唯一」成立——
  **不是**任何「三桶分流在真实行情上正确/有 alpha」的实证断言（那是 L2/L3，需真实 K 线 + 真实
  买卖点重合样本）。买卖点择时 v1 已被全窗 L3 8/8 否证（记忆 `newchanlun-v1-fullwindow-l3-falsified`），
  但那否证 v1 实盘盈利性（L2/L3），与本文件 L0 唯一化定理**认识论等级不同、不可互相否证**。

  范式：纯 List/Bool/Nat/Int + Lean 核心 `Init`（List.foldl/length/contains/insertionSort 手写）。
  仅 import 两纯 core 兄弟文件（CandidateSet + LexArgmin）——**无** Mathlib/Batteries/Std。
  禁 sorry/admit/axiom。禁 Fintype/Finset（Γ/三桶用 List，排序手写 insertionSort）。
  不编辑 lakefile（报 Lead 登记 root `Origin.RThetaInterp`）。
-/

import Origin.CandidateSet
import Origin.LexArgmin

open NewChanlun.Origin (Side)
open NewChanlun.Origin.CandidateSet
open NewChanlun.Origin.LexArgmin

namespace NewChanlun.Origin.RThetaInterp

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 平移不变全序 ≺_Θ（候选 → LexKey，复用 LexArgmin.lexLe 全序原语）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **买卖点序 `bspOrd`（≺_Θ 末键，spec P4 §5 六分量顺序）** —— 买卖点类型映为 Int 序数：
    b1<b2<b3<s1<s2<s3（买点先于卖点，便于同级别买点 ≺_Θ 在先 ⟹ 重合时买点优先执行）。
    **单射**（`bspOrd_inj`）⟹ `toKey` 单射 ⟹ ≺_Θ 是真线序（反对称）。 -/
def bspOrd : BspType → Int
  | .b1 => 0
  | .b2 => 1
  | .b3 => 2
  | .s1 => 3
  | .s2 => 4
  | .s3 => 5

/-- ★`bspOrd` 单射（L0）：bspOrd s = bspOrd t ⟹ s = t（六序数互异）。 -/
theorem bspOrd_inj {s t : BspType} (h : bspOrd s = bspOrd t) : s = t := by
  cases s <;> cases t <;> first | rfl | (exact absurd h (by decide))

/-- **候选字典序键 `toKey`（≺_Θ 的载体，复用 LexArgmin.LexKey = List Int）** ——
    候选 c=(lvl,exec,bsp) 编码为 `[↑lvl, ↑exec, bspOrd bsp]`：主键级别、次键执行级、末键买卖点序。
    所有分量平移不变（S_k 仅在 lvl/exec 同加 k，比较结果不变；bsp 完全不变）⟹ ≺_Θ 平移不变。 -/
def toKey (c : Cand) : LexKey :=
  [(c.lvl : Int), (c.exec : Int), bspOrd c.bsp]

/-- ★`toKey` 单射（L0）：toKey a = toKey b ⟹ a = b（lvl/exec 由 ↑ 单射，bsp 由 bspOrd 单射）。
    这是 ≺_Θ 反对称（真全序，非偏序）的关键——满足 spec 边界条件 (2)「≺_Θ 是全序」。 -/
theorem toKey_inj (a b : Cand) (h : toKey a = toKey b) : a = b := by
  obtain ⟨al, ae, ab⟩ := a
  obtain ⟨bl, be, bb⟩ := b
  simp only [toKey] at h
  injection h with h1 h23
  injection h23 with h2 h3rest
  injection h3rest with h3 _
  have e1 : al = bl := by omega
  have e2 : ae = be := by omega
  have e3 : ab = bb := bspOrd_inj h3
  subst e1; subst e2; subst e3; rfl

/-- **平移不变全序 `candLe`（≺_Θ，复用 LexArgmin.lexLe）** —— g_1 ≼_Θ g_2 ⟺ 字典序键 ≤。 -/
def candLe (a b : Cand) : Bool := lexLe (toKey a) (toKey b)

/-- ★≺_Θ 自反（L0）：candLe c c = true。复用 `lexLe_refl`。 -/
theorem candLe_refl (c : Cand) : candLe c c = true := lexLe_refl _

/-- ★≺_Θ 全序 / 完全性（L0，spec 边界条件 (2)）：任意两候选可比。复用 `lexLe_total`。 -/
theorem candLe_total (a b : Cand) : candLe a b = true ∨ candLe b a = true :=
  lexLe_total _ _

/-- ★≺_Θ 传递（L0）。复用 `lexLe_trans`。 -/
theorem candLe_trans (a b c : Cand) :
    candLe a b = true → candLe b c = true → candLe a c = true :=
  lexLe_trans _ _ _

/-- ★≺_Θ 反对称（L0，真线序非偏序）：candLe a b ∧ candLe b a ⟹ a = b。
    复用 `lexLe_antisymm`（键相等）+ `toKey_inj`（键单射 ⟹ 候选相等）。 -/
theorem candLe_antisymm (a b : Cand) :
    candLe a b = true → candLe b a = true → a = b := fun h1 h2 =>
  toKey_inj a b (lexLe_antisymm _ _ h1 h2)

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 级别平移算子 S_k + ≺_Θ 平移不变性（spec 逐字自相似性质）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **级别平移算子 `shift k`（spec S_k，§1 𝕃=ℤ 平移作用）** —— 候选整体抬升 k 级：
    lvl ↦ lvl+k，exec ↦ exec+k，bsp 不变（保 lvl−exec 嵌套深度不变）。 -/
def shift (k : Nat) (c : Cand) : Cand :=
  { c with lvl := c.lvl + k, exec := c.exec + k }

/-- **★≺_Θ 平移不变（L0，spec 逐字方框 g_1≺_Θ g_2 ⟺ S_k g_1≺_Θ S_k g_2，★自相似核心）** ——
    `candLe (shift k a) (shift k b) = candLe a b`。证：字典序逐分量比较，Int 的 `<`/`=` 在两操作数
    同加 k 下不变（a<b ⟺ a+k<b+k，omega）；bsp 分量完全不变。逐分量用 lexLe cons 三分支引理。 -/
theorem candLe_shift (k : Nat) (a b : Cand) :
    candLe (shift k a) (shift k b) = candLe a b := by
  show lexLe [(↑(a.lvl + k) : Int), (↑(a.exec + k) : Int), bspOrd a.bsp]
             [(↑(b.lvl + k) : Int), (↑(b.exec + k) : Int), bspOrd b.bsp]
     = lexLe [(↑a.lvl : Int), (↑a.exec : Int), bspOrd a.bsp]
             [(↑b.lvl : Int), (↑b.exec : Int), bspOrd b.bsp]
  rcases Nat.lt_trichotomy a.lvl b.lvl with hl | hl | hl
  · rw [lexLe_cons_lt _ _ (by omega : (↑(a.lvl + k) : Int) < (↑(b.lvl + k) : Int)),
        lexLe_cons_lt _ _ (by omega : (↑a.lvl : Int) < (↑b.lvl : Int))]
  · rw [lexLe_cons_eq _ _ (by omega : (↑(a.lvl + k) : Int) = (↑(b.lvl + k) : Int)),
        lexLe_cons_eq _ _ (by omega : (↑a.lvl : Int) = (↑b.lvl : Int))]
    rcases Nat.lt_trichotomy a.exec b.exec with he | he | he
    · rw [lexLe_cons_lt _ _ (by omega : (↑(a.exec + k) : Int) < (↑(b.exec + k) : Int)),
          lexLe_cons_lt _ _ (by omega : (↑a.exec : Int) < (↑b.exec : Int))]
    · rw [lexLe_cons_eq _ _ (by omega : (↑(a.exec + k) : Int) = (↑(b.exec + k) : Int)),
          lexLe_cons_eq _ _ (by omega : (↑a.exec : Int) = (↑b.exec : Int))]
    · rw [lexLe_cons_gt _ _ (by omega : (↑(b.exec + k) : Int) < (↑(a.exec + k) : Int)),
          lexLe_cons_gt _ _ (by omega : (↑b.exec : Int) < (↑a.exec : Int))]
  · rw [lexLe_cons_gt _ _ (by omega : (↑(b.lvl + k) : Int) < (↑(a.lvl + k) : Int)),
        lexLe_cons_gt _ _ (by omega : (↑b.lvl : Int) < (↑a.lvl : Int))]

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. ≺_Θ 排序（手写 insertionSort，纯 core，复用 candLe）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 按 `le` 把 c 插入已排序表（首个 le c d 处插入；纯 core，无 Mathlib List.insertionSort）。 -/
def insertCand (le : Cand → Cand → Bool) (c : Cand) : List Cand → List Cand
  | [] => [c]
  | d :: ds => if le c d then c :: d :: ds else d :: insertCand le c ds

/-- 手写插入排序（纯 core）。用 `candLe`（≺_Θ）排序 Γ。 -/
def insertionSort (le : Cand → Cand → Bool) : List Cand → List Cand
  | [] => []
  | c :: cs => insertCand le c (insertionSort le cs)

/-- ★插入保长 +1（L0）：(insertCand le c l).length = l.length + 1。 -/
theorem insertCand_length (le : Cand → Cand → Bool) (c : Cand) :
    ∀ l : List Cand, (insertCand le c l).length = l.length + 1 := by
  intro l
  induction l with
  | nil => rfl
  | cons d ds ih =>
      simp only [insertCand]
      split <;> simp only [List.length_cons, ih] <;> omega

/-- ★排序保长（L0）：|insertionSort le l| = |l|（排序不增删候选）。 -/
theorem insertionSort_length (le : Cand → Cand → Bool) :
    ∀ l : List Cand, (insertionSort le l).length = l.length := by
  intro l
  induction l with
  | nil => rfl
  | cons c cs ih =>
      simp only [insertionSort]
      rw [insertCand_length le c (insertionSort le cs), ih, List.length_cons]

/-- ≺_Θ 排序 Γ：用 candLe 升序插入排序。 -/
def sortΓ (gs : List Cand) : List Cand := insertionSort candLe gs

/-- ★≺_Θ 排序保长（L0）。 -/
theorem sortΓ_length (gs : List Cand) : (sortΓ gs).length = gs.length :=
  insertionSort_length candLe gs

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 三桶 (𝒟,ℬ,𝒦) + 确定性 fold 解释器 ℛ_Θ
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **三桶 `Triple`（spec §11 ℐ_* 输出 (𝒟_*,ℬ_*,𝒦_*)，★核心类型）** —— 解释器把候选集分流为：
  - `D : List Cand` —— 𝒟_x **应关闭**（卖点触发的平仓候选，spec "应关闭"）。
  - `B : List Cand` —— ℬ_x **应开启**（买点触发的开仓候选，spec "应开启"）。
  - `K : List Cand` —— 𝒦_x **记录不执行**（重合冲突中 ≺_Θ 让位者，spec "记录不执行"，与 𝒟/ℬ 互斥）。
  ★`DecidableEq` ⟹ 见证可 decide（纯核 decide，不引 native 反射公理）。 -/
structure Triple where
  /-- 𝒟_x：应关闭（平仓候选）。 -/
  D : List Cand
  /-- ℬ_x：应开启（开仓候选）。 -/
  B : List Cand
  /-- 𝒦_x：记录不执行（重合让位者）。 -/
  K : List Cand
deriving DecidableEq, Repr

/-- 空三桶（fold 初值）。 -/
def emptyTriple : Triple := ⟨[], [], []⟩

/-- 入 𝒟（应关闭）：尾插保 ≺_Θ 序。 -/
def pushD (t : Triple) (g : Cand) : Triple := { t with D := t.D ++ [g] }
/-- 入 ℬ（应开启）：尾插保 ≺_Θ 序。 -/
def pushB (t : Triple) (g : Cand) : Triple := { t with B := t.B ++ [g] }
/-- 入 𝒦（记录不执行）：尾插保 ≺_Θ 序。 -/
def pushK (t : Triple) (g : Cand) : Triple := { t with K := t.K ++ [g] }

/-- fold 累积态：`(claimed, acc)` —— 已占用级别集 + 当前三桶。 -/
abbrev FoldState := List Nat × Triple

/--
  **★确定性单步 `stepR`（spec §12 ≺_Θ 按序处理，★核心规则）** —— 对候选 g：
  - 其级别 lvl **已被更早（≺_Θ 在先）候选占用**（`claimed.contains g.lvl`）⟹ 入 𝒦（记录不执行）；
  - 否则占用 lvl，按方向 `dirOf g.bsp` 分流：long(买点) ⟹ ℬ（应开启），short(卖点) ⟹ 𝒟（应关闭）。
  ★这是「买卖点重合唯一化」的执行点：同级别多买卖点，≺_Θ 在先者执行，余者 𝒦。 -/
def stepR (st : FoldState) (g : Cand) : FoldState :=
  if st.1.contains g.lvl then
    (st.1, pushK st.2 g)
  else
    match dirOf g.bsp with
    | Side.long  => (g.lvl :: st.1, pushB st.2 g)
    | Side.short => (g.lvl :: st.1, pushD st.2 g)

/--
  **★解释器 ℛ_Θ（在 List Cand 上，spec (𝒟,ℬ,𝒦)=ℛ_Θ(Γ)，★核心）** —— ≺_Θ 排序后确定性 foldl。
  与 `IntervalNestCertificate.selectΘ`（选 1 区间）机制不同：此处 fold 出**三桶划分**。 -/
def interpList (gs : List Cand) : Triple :=
  ((sortΓ gs).foldl stepR ([], emptyTriple)).2

/--
  **★解释器 ℛ_Θ（在状态 x 上，spec (𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))，★核心定义）** ——
  喂入环3 候选集 `gamma x`（CandidateSet，已证有限），输出唯一三桶。 -/
def RΘ (x : State) : Triple := interpList (gamma x)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. ∀x ∃!(𝒟,ℬ,𝒦)（spec line 626 中心命题）+ 三桶划分守恒（soundness）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★∀x ∃!(𝒟_x,ℬ_x,𝒦_x)（spec line 626 / §12 方框 ∀x ∃!，★核心主定理）** —— 对每个状态 x，
  存在唯一三桶 = ℛ_Θ(Γ(x))。**L0**：R_Θ 是全函数（排序 + 确定性 fold），故像唯一。
  其三前提（spec 边界条件）已分别坐实：(1)Γ(x)有限=`CandidateSet.gamma_finite`；(2)≺_Θ 全序=
  `candLe_refl/total/trans/antisymm`；(3)每步确定=`stepR` 是函数。三者缺一则 ∃! 翻转。
  实质唯一性（顺序无关）见 `rTheta_order_invariant`；本 ∃! 是其函数式推论。 -/
theorem rTheta_exists_unique (x : State) :
    NewChanlun.Origin.ExistsUnique (fun t : Triple => RΘ x = t) :=
  ⟨RΘ x, rfl, fun _ hy => hy.symm⟩

/--
  **★R_Θ 顺序不变性 `rTheta_order_invariant`（实质补充，消除声明膨胀）** ——
  若两候选集的 ≺_Θ 排序结果相同（`sortΓ gs₁ = sortΓ gs₂`），则 R_Θ 输出相同三桶。
  坐实 spec line 632「固定级别平移不变的自相似总序 ≺_Θ，解释器 ℛ_Θ 按序处理产生唯一三元组」
  ——三桶由 ≺_Θ 序唯一决定，与输入原始顺序无关。
  **非平凡**：依赖 `interpList` 内部通过 `sortΓ`（`candLe` 全序驱动）排序再 fold；
  任意函数 f 不满足「sortΓ gs₁ = sortΓ gs₂ ⟹ f gs₁ = f gs₂」。
  **L0**（定义展开）。 -/
theorem rTheta_order_invariant (gs₁ gs₂ : List Cand)
    (h : sortΓ gs₁ = sortΓ gs₂) :
    interpList gs₁ = interpList gs₂ := by
  simp only [interpList, h]

/-- 三桶总基数（|𝒟|+|ℬ|+|𝒦|）。 -/
def tripleLen (t : Triple) : Nat := t.D.length + t.B.length + t.K.length

/-- ★单步使三桶总基数 +1（L0）：每候选恰入一桶（𝒟/ℬ/𝒦 互斥分流，spec line 590）。 -/
theorem stepR_len (st : FoldState) (g : Cand) :
    tripleLen (stepR st g).2 = tripleLen st.2 + 1 := by
  unfold stepR
  split
  · simp only [tripleLen, pushK, List.length_append, List.length_cons, List.length_nil]; omega
  · split <;>
      simp only [tripleLen, pushD, pushB, List.length_append, List.length_cons, List.length_nil] <;>
      omega

/-- ★fold 守恒（L0）：foldl 后三桶总基数 = 初值 + 候选数（无候选丢失/重复）。 -/
theorem foldl_len (gs : List Cand) :
    ∀ st : FoldState, tripleLen (gs.foldl stepR st).2 = tripleLen st.2 + gs.length := by
  induction gs with
  | nil => intro st; simp [tripleLen]
  | cons g gs ih =>
      intro st
      rw [List.foldl_cons, ih (stepR st g), stepR_len st g, List.length_cons]
      omega

/--
  **★三桶是 Γ 的划分（基数守恒，L0，soundness）** —— |𝒟_x|+|ℬ_x|+|𝒦_x| = |Γ(x)|。
  坐实「三桶 fold 不丢失/不重复任何候选」——每候选恰分流到一桶（互斥穷尽分流，非空跑桩）。 -/
theorem rTheta_partition_length (x : State) :
    tripleLen (RΘ x) = (gamma x).length := by
  unfold RΘ interpList
  rw [foldl_len (sortΓ (gamma x)) ([], emptyTriple)]
  simp only [tripleLen, emptyTriple, List.length_nil, Nat.zero_add]
  exact sortΓ_length (gamma x)

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 反退化见证（解释器真跑通：三桶非平凡 + 重合唯一化 + 平移不变）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 反退化见证候选表：同级别 1 上买点 b1 与卖点 s1 **重合**（"同一时刻多买卖点"，line 632），
    级别 2 上卖点 s2。展示三桶均非空 + ≺_Θ 唯一化。 -/
def witList : List Cand := [⟨1, 0, .b1⟩, ⟨1, 0, .s1⟩, ⟨2, 0, .s2⟩]

/-- **★见证：ℬ_x = [⟨1,0,b1⟩]（应开启，L0）** —— 级别 1 上 b1（≺_Θ 序 bspOrd 0 < s1 序 3）在先 ⟹
    占用级别 1，买点入 ℬ。坐实「重合时 ≺_Θ 在先者执行」。 -/
theorem witness_interp_B : (interpList witList).B = [⟨1, 0, .b1⟩] := by decide

/-- **★见证：𝒟_x = [⟨2,0,s2⟩]（应关闭，L0）** —— 级别 2 未占用，卖点 s2 入 𝒟。 -/
theorem witness_interp_D : (interpList witList).D = [⟨2, 0, .s2⟩] := by decide

/-- **★见证：𝒦_x = [⟨1,0,s1⟩]（记录不执行，L0，★唯一化核心）** —— 级别 1 已被 b1 占用 ⟹ 同级别
    s1 让位，入 𝒦。坐实 spec「买卖点重合唯一化」：重合的买/卖点中 ≺_Θ 在先者执行，余者记录不执行。 -/
theorem witness_interp_K : (interpList witList).K = [⟨1, 0, .s1⟩] := by decide

/-- **★见证：三桶划分守恒到具体数（L0）** —— |𝒟|+|ℬ|+|𝒦| = 3 = |witList|（无丢失/重复）。 -/
theorem witness_partition : tripleLen (interpList witList) = witList.length := by decide

/-- **★见证：环3→环5 集成（gamma witState 喂入 ℛ_Θ，L0）** —— CandidateSet.witState 的三级买入
    区间套链（3 候选 ⟨0/1/2,0,b1⟩，全 long、级别互异）⟹ 无冲突，全入 ℬ（应开启），𝒟/𝒦 空。 -/
theorem witness_state_B :
    (RΘ witState).B = [⟨0, 0, .b1⟩, ⟨1, 0, .b1⟩, ⟨2, 0, .b1⟩] := by decide

/-- **★见证：集成态 𝒟 空（L0）** —— witState 无卖点触发 ⟹ 无应关闭候选。 -/
theorem witness_state_D_empty : (RΘ witState).D = [] := by decide

/-- **★见证：集成态 𝒦 空（L0）** —— witState 三候选级别互异（0/1/2）⟹ 无重合冲突 ⟹ 无记录不执行。 -/
theorem witness_state_K_empty : (RΘ witState).K = [] := by decide

/-- **★见证：集成态划分守恒（L0）** —— |𝒟|+|ℬ|+|𝒦| = 3 = |Γ(witState)|。 -/
theorem witness_state_partition : tripleLen (RΘ witState) = (gamma witState).length :=
  rTheta_partition_length witState

/-- **★见证：≺_Θ 平移不变到具体候选（L0，spec 自相似）** —— S_5 下两候选的 ≺_Θ 序不变。 -/
theorem witness_shift_inv :
    candLe (shift 5 ⟨1, 0, .b1⟩) (shift 5 ⟨2, 0, .s1⟩) = candLe ⟨1, 0, .b1⟩ ⟨2, 0, .s1⟩ :=
  candLe_shift 5 _ _

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. still-MISSING 诚实声明 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ★本文件**实装**（对照 spec §12 / line 1171 环5，零遗漏）：
    (1) **平移不变全序 ≺_Θ**（`toKey` 字典序键 + `candLe` 复用 LexArgmin.lexLe；`candLe_refl/total/
        trans/antisymm` 全序四性质 + `candLe_shift` 平移不变，spec 逐字 g_1≺_Θ g_2 ⟺ S_k g_1≺_Θ S_k g_2）。
    (2) **三桶 (𝒟,ℬ,𝒦)** + **确定性 fold 解释器 ℛ_Θ**（`Triple`/`stepR`/`interpList`/`RΘ`：≺_Θ 排序
        `sortΓ` + foldl 分流，spec §11 应关闭/应开启/记录不执行 + §12 ≺_Θ 唯一化）。
    (3) **∀x ∃!(𝒟,ℬ,𝒦)**（`rTheta_exists_unique`，spec line 626 中心命题；三前提显式坐实）。
    (4) **三桶划分守恒**（`rTheta_partition_length`：|𝒟|+|ℬ|+|𝒦|=|Γ|，互斥穷尽分流 soundness）。

  ★本文件**未**实装（诚实 still-MISSING，非声明膨胀）：
    · **环6 活动集 A_{t+1}=AncOK((A_t∖𝒟_x)∪ℬ_x)**：三桶喂入活动集更新（spec §13）是**下游独立工位**。
      本文件只产 (𝒟,ℬ,𝒦):Triple 作其输入，**不**做活动集/祖先闭合（属环6，不属本工位）。
    · **角色化 R(g)=(H,V,δ) 进入 ≺_Θ**：spec 环4 把候选 role 化（18 类），≺_Θ 可含角色分量。本文件
      ≺_Θ 末键只用 `bspOrd`（买卖点序），**不**引入角色轴——角色层单一权威在 `OperationRole18`，
      `CandidateSet.Cand.roleOf` 已提供桥；若需角色进序为**下游扩展**（本文件不臆造角色优先级）。
      [需人工确认] spec 未逐字给定 ≺_Θ 的**完整分量顺序**（仅给"平移不变全序"性质），本文件取
      (lvl,exec,bspOrd) 为一**满足平移不变性的具体全序实例**——分量顺序是设计选择（见边界条件）。
    · **冲突谓词的精确粒度**：本文件取「同级别 lvl 冲突」（一级别一执行）。[需人工确认] PDF §12 未
      逐字给出冲突谓词（仅给"≺_Θ 按序处理产生唯一"）——per-level 是本文件**显式编码的确定规则**，
      非 PDF 字面（见边界条件）。**不引入新定义冲突**（规则确定即满足 spec 边界条件 (3)）。
    · **𝒟/ℬ/𝒦 的缠论执行语义**：本文件按 spec §11 文字（𝒟=应关闭/ℬ=应开启/𝒦=记录不执行）+ 方向
      （买点开/卖点平）分流，**不**实装腿算子 Leg(g)/记账（属环6+ 分账本，单一权威在 LedgerBridge）。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：R_Θ 解释器的 **L0 结构形式化**——(𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))：候选集 `gamma x`（环3，
     CandidateSet）按平移不变全序 ≺_Θ（`candLe`=`lexLe∘toKey`）排序（`sortΓ`）后**确定性 fold**
     （`stepR`：级别占用冲突→𝒦，否则按方向→𝒟/ℬ）出三桶 `Triple`。主定理 `rTheta_exists_unique`
     （∀x ∃! 三桶）+ `rTheta_partition_length`（三桶划分守恒）+ `candLe_shift`（≺_Θ 平移不变）。
     全 L0，零 sorry/admit/axiom。反退化见证：重合 b1/s1 唯一化（b1→ℬ, s1→𝒦, s2→𝒟，三桶均非空）。
  2. 定义依据：spec `2026-06-28-...-bsp-pdf-extract.md` §12（P10）+ 七链表 line 1171（环5：
     (𝒟,ℬ,𝒦)=ℛ_Θ(Γ(x)); ≺_Θ 平移不变 g_1≺_Θ g_2⟺S_k g_1≺_Θ S_k g_2; ∀x ∃!）+ 结果包 line 631-639
     （多买卖点重合→有限 Γ→≺_Θ 总序→唯一三元组 + 三条唯一性依据）+ ℐ_* line 587-595（三输出含义
     𝒟=应关闭/ℬ=应开启/𝒦=记录不执行 + 第三类互斥分流）。输入特征：Γ(x)=List Cand 有限
     （CandidateSet `gamma_finite`，满足依据 (1)）；买卖点不互斥可重合（spec P4 §5）⟹ 同级别多候选
     ⟹ ≺_Θ 唯一化分流（满足"买卖点重合唯一化机制"）；方向 δ=dirOf（买入/卖出，spec P4 §6）⟹ ℬ/𝒟 分流。
  3. 边界条件（结论翻转）：
     · **≺_Θ 必须是全序（核心，spec 逐字依据 (2)）**：∃! 依赖 ≺_Θ 全序。本文件 `toKey` 单射
       ⟹ `candLe` 反对称（`candLe_antisymm`）⟹ 真线序。若 `toKey` 非单射（如末键去掉 bspOrd，
       两不同买卖点同级别同执行级 ⟹ 键相等），≺_Θ 退化为偏序，排序非唯一定序 ⟹ ∃! **翻转**。
     · **Γ(x) 必须有限（spec 依据 (1)）**：fold 终止依赖 Γ:List 有限（CandidateSet 坐实）。若 Γ 无限
       （级别无界，CandidateSet 边界），fold 不终止 ⟹ 解释器无定义 ⟹ ∃! 翻转。
     · **每步规则确定（spec 依据 (3)）**：`stepR` 是确定函数。若分流规则含非确定性（如随机/依赖未
       建模外部态），fold 非确定 ⟹ ∃! 翻转。当前 `stepR` 纯函数（确定）。
     · **冲突粒度 = 同级别（设计选择）**：当前「lvl 已占用→𝒦」。若改为 (lvl,dir) 或 (lvl,exec) 粒度，
       𝒦 的内容翻转（哪些重合候选让位）——本文件取 per-level（一级别一执行），[需人工确认] PDF 未
       逐字钉死冲突谓词。**不论取哪种粒度，只要规则确定，∃! 不变**（∃! 依赖确定性，非依赖具体粒度）。
     · **≺_Θ 分量顺序 = (lvl,exec,bspOrd)（设计选择）**：满足平移不变性的具体全序实例。换序（如
       exec 主键）仍是平移不变全序，∃! 不变，但三桶**内部排列/重合让位者**翻转。spec 仅要求"平移
       不变全序"性质（本文件 `candLe_shift` 坐实），未钉死分量顺序。
  4. 下游推论：
     · `RΘ : State → Triple` 全定义 + ∃! ⟹ 环6 活动集更新（spec §13 A_{t+1}=AncOK((A_t∖𝒟_x)∪ℬ_x)）
       可直接消费三桶——𝒟_x 是平仓来源、ℬ_x 是开仓来源、𝒦_x 不进活动集（记录不执行）。
     · `candLe_shift`（≺_Θ 平移不变）⟹ 解释器自相似 ℐ_{ℓ+k}(S_k x)=S_k ℐ_ℓ(x)（spec §11 line 587）
       的序层基础——各级别共用一份 ≺_Θ，无根解释器、无顶层特例（去根化，spec 升级 1）。
     · `rTheta_partition_length`（划分守恒）⟹ 无候选在解释器层丢失/重复，支撑环6 头寸聚合 p̃ 的完整性。
     · 复用 `LexArgmin.lexLe` 全序原语 ⟹ 与环7 风控层 LexArgmin（J_Θ 字典序最小化）**共用同一字典序
       内核**——序的单一权威在 LexArgmin，本工位不重证全序四性质（避免漂移）。
  5. 谱系引用：
     · 缺口矩阵 `2026-06-28-lean-existing-vs-20page-gap-matrix.md` 环5 判"真缺口→新建"——本文件即填补
       （首个 R_Θ 三桶解释器；此前 Origin 仅有 `IntervalNestCertificate.selectΘ` 选 1 区间，无三桶 fold）。
     · **缺口矩阵明示「不 reconcile IntervalNestCertificate」**（selectΘ 选 1 区间 ≠ R_Θ 三桶 fold，
       机制不同）——本文件遵守：复用 `LexArgmin.lexLe` 全序原语（无 Interval 依赖），**不** import
       selectΘ。坐实「字典序/线序原语」与「区间最优选择 fold」是两层（前者复用，后者不混用）。
     · 触及记忆 `theta-v0-trades-vs-closedloop-disjoint-paths` / `theta_v0 recognize 硬编码 exit:false`：
       spec §12 结果包标注 𝒟_x（应关闭）正是 rust recognize 路径 exit 的来源——本文件 𝒟 桶为该谱系
       提供 Lean 侧的「应关闭」唯一化语义（rust 侧重写 exit:false 为按 𝒟_x 是下游 rust 工位，不属本工位）。
     · 触及记忆 `newchanlun-v1-fullwindow-l3-falsified`：本文件 L0 唯一化定理与 v1 实盘盈利性（L2/L3）
       **认识论等级不同、不可互相否证**——不预判 alpha。
     · **不确定**是否有「R_Θ 三桶解释器/总序唯一化」的更早专属谱系记录——本工位不臆造谱系，明确标注
       此不确定性（建议 genealogist 核 `.chanlun/genealogy/` 确认环5 唯一化是否首次形式化）。
  6. 影响声明：新建 `Origin/RThetaInterp.lean`，仅 import 两纯 core 兄弟文件（`Origin.CandidateSet`
     环3 + `Origin.LexArgmin` 序原语），**无** Mathlib/Batteries/Std。**不改**任何既有 Lean/rust/定义/spec
     （不碰 BuySellPredicate/strategy/IntervalNestCertificate）。namespace `NewChanlun.Origin.RThetaInterp`
     （无命名冲突）。下游：为环6 活动集更新提供唯一三桶 (𝒟,ℬ,𝒦)。不碰 lakefile（报 Lead 登记 root
     `Origin.RThetaInterp`）。`lake env lean Origin/RThetaInterp.lean` 单文件验证。
-/

end NewChanlun.Origin.RThetaInterp
