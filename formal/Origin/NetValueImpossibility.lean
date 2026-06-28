/-
Origin/NetValueImpossibility.lean

W4 — 不可能定理 I（组合净值不可吃每一笔）。canonical 条目 **C22**
（`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md` §B 表 C22 行 + §8 页6–7）。

════════════════════════════════════════════════════════════════════════
## ★定位：声明膨胀防火墙（否定性定理）

本文件是**否定性结果**——它证明的不是"系统能吃每一笔"，而是"在**组合净值坐标**下，
父多子空同股数双开 ⟹ 净值抵消 ⟹ 净账户**不可能**每笔盈利"。

PDF §8 原文（C22）：父声部持多头 +Q、子声部同单位空头 -Q，开启期间净头寸 (+Q)+(-Q)=0；
反向笔（价格 P_λ→P_ρ 下跌）子声部毛收益 `G_child = Q·(P_λ - P_ρ) > 0`，
但父声部浮亏 `G_parent = Q·(P_ρ - P_λ) < 0`，二者相加 `G_parent + G_child = 0`
（无成本抵消，有交易成本则严格为负）。一般情形：父 σQ、子 -σQ，净 σQ - σQ = 0；
要组合净值吃下反向笔须 `|q_child| > |q_parent|`，**违反**同单位数双开约束 `q_child = q_parent`。

⟹ 推论：**声部级毛收益每笔为正（C16/C21 主覆盖定理）≠ 组合净值每笔为正**。
任何下游模块若声称"组合净值吃每笔盈利"，被本定理**否决**。这正是声明膨胀防火墙的语法位置：
它封死了把 L0 声部级覆盖膨胀为"净账户实盘每笔盈利"的有效域越界（231号）。

════════════════════════════════════════════════════════════════════════
## ★与 LedgerBridge / TotalWealth 的关系（no-workaround，关键）

本文件**不** import LedgerBridge / TotalWealth，也**不**复用其定理体。理由：

- LedgerBridge / TotalWealth 的"同单位约束"指 **ShareConserving 降成本腿**（`shortDiff` 同价
  free⇄holding 转换，TW 守恒），其代数对象是**单账本 TW 三阶段 + OQ-9 gate**——与 C22 的
  "父多 +Q 子空 -Q 同股数方向双开"**不是同一对象**。
- C22 需要的是**方向化毛收益** `G = q·ε·(P_ρ - P_λ)`（ε=方向，多 +1 / 空 -1）这一独立代数。
  把它硬塞进 TW 单账本 = 用净额映射 `Qe⁺ + Qe⁻ ↦ 0` 消解方向暴露 = 重演 **230号直积退化**。

故本文件**自包含**建立方向化毛收益代数，与 LedgerBridge 在**概念上对照**（同遵守"净值口径下
方向双开抵消"的精神，spec §D 标 W4 依赖 LedgerBridge 同单位约束 = 概念对照，非定理 import），
而非结构耦合。这尊重 #90 不同构（不把两个账本糊成一个）。

════════════════════════════════════════════════════════════════════════
## 认识论等级（formalization-validity-domain，231号强制标注）

全文件 **L0**（纯代数/定义层，零数据依赖）。`lake env lean` 通过 = 代数恒等
`G_parent + G_child = 0`（同单位双开净值抵消）在整数/方向化坐标下成立。

**这是否定性结果**——它**缩小**有效域的边界（封死"组合净值吃每笔"声称），其价值**高于**
确认性结果（确认性结果只是未能否证，否定性结果实际缩小了有效域，231号 L2/L3 价值序）。
本定理虽在 L0（代数恒等），但其**功能是有效域防火墙**：它否决的正是把声部级 L0 主覆盖
膨胀为净账户实盘盈利（那需 L2/L3，PDF 未提供也不可由 L0 推出）。

禁 sorry/admit/axiom，不依赖 Mathlib。

谱系：230号（直积退化：直积在定义域代数成立、有效域概率度量下退化）——C22 净值抵消是
      230号退化的**具体机制**（`Qe⁺ + Qe⁻ ↦ 0`）。222/223号（守恒律/分类有效域≠定义域）——
      C22 是 PDF §8 自带"有效域<定义域"声明（声部级非净值级）的形式化。
-/

namespace NewChanlun.Origin.NetValueImpossibility

/-! ════════════════════════════════════════════════════════════════════
## §1 方向化坐标：方向 ε 与方向化毛收益 G

PDF §1：缠论笔元素方向由端点确定 `ε·(P_ρ - P_λ) > 0`；毛收益 `G = q·ε·(P_ρ - P_λ)`。
价格用整数（整数 tick 口径，避免实数依赖；C22 是纯代数恒等，整数足够且严格）。
================================================================════════ -/

/-- 方向 `ε ∈ {+1, -1}`：多头 `+1`、空头 `-1`。用 `Sign` 标定。 -/
inductive Sign where
  | long   -- 多头方向 ε = +1
  | short  -- 空头方向 ε = -1
  deriving DecidableEq, Repr

/-- 方向的整数取值 `ε ∈ {+1, -1}`。 -/
def Sign.toInt : Sign → Int
  | Sign.long  => 1
  | Sign.short => -1

/-- 方向取反 `-ε`：多 ↔ 空。父多子空双开的核心机制。 -/
def Sign.flip : Sign → Sign
  | Sign.long  => Sign.short
  | Sign.short => Sign.long

@[simp] theorem Sign.flip_toInt (ε : Sign) : (ε.flip).toInt = - ε.toInt := by
  cases ε <;> rfl

@[simp] theorem Sign.toInt_long  : Sign.long.toInt  = 1  := rfl
@[simp] theorem Sign.toInt_short : Sign.short.toInt = -1 := rfl

/--
一条**头寸腿**（leg）：股数 `q ≥ 0`、方向 `ε`、入场价 `pLam`、出场价 `pRho`。
方向化毛收益 `G = q · ε · (pRho - pLam)`（PDF §1/§6 毛收益公式）。
-/
structure Leg where
  q  : Nat   -- 股数（非负）
  ε  : Sign  -- 方向（多/空）
  pLam : Int   -- 入场价（区间左端 λ）
  pRho : Int   -- 出场价（区间右端 ρ）
  deriving Repr

/-- 方向化毛收益 `G(leg) = q · ε · (pRho - pLam)`（整数口径）。 -/
def Leg.G (l : Leg) : Int := (l.q : Int) * l.ε.toInt * (l.pRho - l.pLam)

/-! ════════════════════════════════════════════════════════════════════
## §2 同单位双开：父多子空，同股数 q_child = q_parent，同价区间

PDF §8（C22）：父声部持 +Q、子声部同单位 -Q，**同一价格区间** [λ, ρ]（子在父开启期间内）。
"同单位双开"= 同股数 `q_child = q_parent` + 方向相反 `ε_child = ε_parent.flip` + 同价区间。
================================================================════════ -/

/--
**同单位反向双开对** `OppositeDoublePair`：父腿与子腿，
- 同股数 `q_child = q_parent`（同单位约束，PDF §8 `q_child = q_parent`）；
- 方向相反 `ε_child = ε_parent.flip`（父多子空 / 父空子多）；
- 同价区间 `pLam`/`pRho` 相同（子在父开启期间覆盖同一段价格）。

这正是 PDF §8 不可能定理 I 的前提构形。
-/
structure OppositeDoublePair where
  parent : Leg
  child  : Leg
  hq     : child.q = parent.q                 -- 同单位（同股数双开）
  hε     : child.ε = parent.ε.flip            -- 方向相反
  hpLam    : child.pLam = parent.pLam               -- 同价区间左端
  hpRho    : child.pRho = parent.pRho               -- 同价区间右端

/-! ════════════════════════════════════════════════════════════════════
## §3 核心不可能定理：G_parent + G_child = 0（净值抵消）

PDF §8（C22）核心代数恒等：父子方向相反、同股数、同价区间 ⟹ 毛收益之和恒为 0。
这是**否定性定理**——它证明组合净值坐标下父子双开净值必然抵消，从而封死"净账户每笔盈利"。
================================================================════════ -/

/--
★★**不可能定理 I（C22，组合净值不可吃每一笔）核心代数恒等**：
同单位反向双开对的父腿与子腿，方向化毛收益之和 `G_parent + G_child = 0`。

证明：`G_parent = q·ε·(pRho-pLam)`，`G_child = q·(-ε)·(pRho-pLam)`（同 q、同价区间、方向取反），
故 `G_parent + G_child = q·ε·(pRho-pLam) + q·(-ε)·(pRho-pLam) = q·(ε + (-ε))·(pRho-pLam) = 0`。

**否定性结论**：无论价格方向（pRho-pLam 正负），无论方向 ε，净值之和恒为 0 ⟹
组合净值坐标下父子双开**不可能同时为正**（一正一负相抵），故"净账户每笔盈利"不可能。
-/
theorem net_value_cancels (P : OppositeDoublePair) :
    P.parent.G + P.child.G = 0 := by
  unfold Leg.G
  rw [P.hq, P.hε, P.hpLam, P.hpRho, Sign.flip_toInt, Int.mul_neg, Int.neg_mul]
  omega

/--
★**否定性推论：净值恒非正（无成本则恰为 0）**——`G_child = - G_parent`。
组合净值坐标下，子腿收益恰是父腿收益的相反数：子腿赚多少，父腿亏多少，**严格抵消**。
这封死了"子腿吃到反向笔即净账户获利"的声称——父腿的浮亏恰好吃光子腿的毛利。
-/
theorem child_gain_is_parent_loss (P : OppositeDoublePair) :
    P.child.G = - P.parent.G := by
  have h := net_value_cancels P
  omega

/--
★**一般情形（σQ - σQ = 0）**：方向 σ、股数 Q 任意，父 `+σQ`、子 `-σQ`，同价区间 ⟹ 净 0。
这是 `net_value_cancels` 的等价显式形式，直接对应 PDF §8"一般：父 σQ 子 -σQ 净 σQ-σQ=0"。
-/
theorem general_net_zero (Q : Nat) (σ : Sign) (pLam pRho : Int) :
    (Leg.mk Q σ pLam pRho).G + (Leg.mk Q σ.flip pLam pRho).G = 0 := by
  exact net_value_cancels
    { parent := Leg.mk Q σ pLam pRho
      child  := Leg.mk Q σ.flip pLam pRho
      hq := rfl, hε := rfl, hpLam := rfl, hpRho := rfl }

/-! ════════════════════════════════════════════════════════════════════
## §4 反向笔具体机制：价格下跌 ⟹ 空头腿正、多头腿负、净 0

PDF §8 具体数值：价格 P_λ→P_ρ 下跌（pRho < pLam），父多头浮亏 G_parent<0，子空头毛利 G_child>0，
但二者抵消。这把"声部级毛利为正"与"组合净值为零"的分离显式化（声明膨胀防火墙的具体显现）。
================================================================════════ -/

/--
★**反向笔机制（父多子空，价格下跌）**：父多头 `ε=long`，子空头 `ε=short`，
价格下跌 `pRho < pLam` ⟹ 子空头毛利 `G_child > 0` 且 父多头浮亏 `G_parent < 0`，但 `G_parent + G_child = 0`。

这是 C22 的**经验直觉的形式化**：子声部"吃到"了下跌这一笔（毛利为正），
但在组合净值口径下被父声部的浮亏完全抵消——"声部级吃到 ⇏ 净账户盈利"。
-/
theorem reverse_stroke_short_positive_long_negative
    (Q : Nat) (hQ : 0 < Q) (pLam pRho : Int) (hdrop : pRho < pLam) :
    let parent := Leg.mk Q Sign.long  pLam pRho
    let child  := Leg.mk Q Sign.short pLam pRho
    0 < child.G ∧ parent.G < 0 ∧ parent.G + child.G = 0 := by
  intro parent child
  have hQpos : (0 : Int) < (Q : Int) := by omega
  have hgap  : (0 : Int) < pLam - pRho := by omega
  -- 共用：Q · (pLam - pRho) > 0（Q>0、价格下跌 ⟹ pLam - pRho > 0）
  have hprod : (0 : Int) < (Q : Int) * (pLam - pRho) := Int.mul_pos hQpos hgap
  refine ⟨?_, ?_, ?_⟩
  · -- G_child = Q · (-1) · (pRho - pLam) = Q · (pLam - pRho) > 0
    show (0 : Int) < (Q : Int) * Sign.short.toInt * (pRho - pLam)
    simp only [Sign.toInt_short]
    rw [Int.mul_neg, Int.mul_one, Int.neg_mul]
    have heq : (Q : Int) * (pRho - pLam) = - ((Q : Int) * (pLam - pRho)) := by
      rw [← Int.mul_neg]; congr 1; omega
    rw [heq]; omega
  · -- G_parent = Q · (+1) · (pRho - pLam) < 0（pRho - pLam < 0）
    show (Q : Int) * Sign.long.toInt * (pRho - pLam) < 0
    simp only [Sign.toInt_long, Int.mul_one]
    have heq : (Q : Int) * (pRho - pLam) = - ((Q : Int) * (pLam - pRho)) := by
      rw [← Int.mul_neg]; congr 1; omega
    rw [heq]; omega
  · -- 净值抵消
    exact net_value_cancels
      { parent := parent, child := child
        hq := rfl, hε := rfl, hpLam := rfl, hpRho := rfl }

/-! ════════════════════════════════════════════════════════════════════
## §5 同单位约束的必要性：吃下反向笔须 |q_child| > |q_parent|（违反双开）

PDF §8：要组合净值吃下反向笔（净值之和 > 0）须 `|q_child| > |q_parent|`——
但同单位双开要求 `q_child = q_parent`，矛盾。这显式化"要净值盈利 ⟹ 必须违反同单位约束"。
================================================================════════ -/

/--
★**同单位约束封死净值盈利**：在同单位反向双开对（`q_child = q_parent`）下，
净值之和 `G_parent + G_child` 恒为 0，**不可能严格为正**。

⟹ 否定性结论：组合净值坐标下，遵守同单位双开约束 ⟹ 净值不可能每笔盈利。
（对偶地：要净值严格为正，必须 `q_child > q_parent`，即放弃同单位约束——这超出 C22 前提。）
-/
theorem same_unit_forbids_net_profit (P : OppositeDoublePair) :
    ¬ (0 < P.parent.G + P.child.G) := by
  rw [net_value_cancels P]
  exact Int.lt_irrefl 0

/--
★**防火墙总结（C22 顶层否定形式）**：存在一类构形（同单位反向双开），
其组合净值恒为 0——故"∀ 笔，组合净值严格为正"这一**全称声称为假**（被本反例否决）。

下游任何声称"组合净值吃每一笔盈利"的模块，被此定理证伪：
只需取一对父多子空同股数同区间双开，其净值即为 0，不满足"严格为正"。
-/
theorem net_value_eat_every_stroke_impossible :
    ∃ P : OppositeDoublePair, ¬ (0 < P.parent.G + P.child.G) := by
  refine ⟨{ parent := Leg.mk 1 Sign.long 0 0
            child  := Leg.mk 1 Sign.short 0 0
            hq := rfl, hε := rfl, hpLam := rfl, hpRho := rfl }, ?_⟩
  exact same_unit_forbids_net_profit _

end NewChanlun.Origin.NetValueImpossibility
