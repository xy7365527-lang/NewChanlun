/-
Origin/CausalEndpointImpossibility.lean

W5 — 不可能定理 II（因果策略不可无延迟吃满事后端点）。canonical 条目 **C23**
（`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md` §B 表 C23 行 + §9 页7–8 + §D W5 行）。

════════════════════════════════════════════════════════════════════════
## ★定位：声明膨胀防火墙（否定性定理）

本文件是**否定性结果**——它证明的不是"系统能吃满每一笔"，而是"在**因果（无前视）约束**下，
任何策略**不可能**在笔右端点 ρ_b 当下无延迟精确平仓（若 ρ_b 须等未来数据确认）"。

PDF §9 原文（C23）：若笔右端点 ρ_b 只在未来数据出现后确认，则因果策略不能在 ρ_b 当下精确平仓。
证明（反事实）：两条历史路径 h、h′ 在时刻 t 及之前完全相同 `h_{≤t} = h′_{≤t}`，
h 中该笔在 t 结束、h′ 中该笔延伸（未来分叉）；因果策略 ⟹ `π(h_{≤t}) = π(h′_{≤t})`，
但"端点吃满"要求 h 平仓、h′ 持有 ⟹ `π(h_{≤t}) ≠ π(h′_{≤t})`，**矛盾**。
⟹ 因果策略只能吃"操作语义中已可判定的笔元素"，不能吃事后才确认的端点。

⟹ 推论：**声部级/语法元素级覆盖（C16/C37 主覆盖定理）成立的前提是"边界操作语义可判定/执行无延迟"**
（spec §C 有效域：仅可判定元素，非事后端点）。任何下游模块若声称"因果系统无延迟吃满事后才
确认的端点"，被本定理**否决**。这正是声明膨胀防火墙的语法位置：它封死了把 L0 主覆盖定理的
有效域膨胀为"实时无延迟吃满未来确认端点"的越界（231号）。

════════════════════════════════════════════════════════════════════════
## ★与 Strict/Causal 的关系（no-workaround，关键）

本文件**复用** `Strict.Classification` 的 `PrefixEq`（前缀相等：两条事件流在时刻 t 及之前
逐点相等），它是 C23 反事实论证的核心载体——"因果"的精确结构形式就是"当下动作只依赖前缀"。

- `Strict/Causal.lean` 已证 located 流分类器的 `causal_no_lookahead`（`PrefixEq ω ω' t ⟹
  Cstream ω t = Cstream ω' t`）+ 非平凡见证 `lookahead_not_causal`（偷看未来的分类器不因果）。
  那是**分类器侧**的因果性（C 读前缀 ⟹ 无前视）。
- 本文件是**策略/动作侧**的不可能性：把"因果"约束施加到**策略动作** `act`（而非分类状态），
  证明因果约束 + 端点吃满规格**不可同时成立**。这是 C23 的独有内容（C 的因果性是"能做到无前视"，
  C23 是"无前视约束下**不能**做到端点吃满"——前者肯定、后者否定，互补而非重复）。

故本文件**不** import `Strict.Causal`（那是分类器侧实例），只复用最上游的 `PrefixEq` 抽象
（`Strict.Classification`，纯定义层、不依赖 Mathlib、所有 Layer2 共享内核）。这尊重
no-workaround（同一因果约束只有一个定义，不自造第二个 PrefixEq）。

════════════════════════════════════════════════════════════════════════
## 认识论等级（formalization-validity-domain，231号强制标注）

全文件 **L0**（纯结构/逻辑层，零数据依赖）。`lake env lean` 通过 = 反事实矛盾
（因果 ∧ 端点吃满 ⟹ False）在纯逻辑下成立，不依赖任何真实走势数据。

**这是否定性结果**——它**缩小**有效域的边界（封死"因果无延迟吃满事后端点"声称），其价值
**高于**确认性结果（231号 L2/L3 价值序）。本定理虽在 L0（逻辑必然），但其**功能是有效域
防火墙**：它否决的正是把声部级/元素级 L0 主覆盖（在"可判定边界 + 执行无延迟"前提下成立）
膨胀为"实时因果系统能无延迟吃满未来确认端点"（那在因果约束下**逻辑上不可能**，不是工程难度问题）。

禁 sorry/admit/axiom，不依赖 Mathlib（只复用 `Strict.Classification` 的 `PrefixEq`）。

谱系：222/223/230号（守恒律/分类/直积有效域 ≠ 定义域）——C23 是 PDF §9 自带"有效域<定义域"
      声明（仅可判定元素，非事后端点）的形式化，是该模式的又一显现。
      与 C22（NetValueImpossibility）并列为 PDF 的两个不可能定理（声明膨胀防火墙双闸）：
      C22 封"组合净值吃每笔"，C23 封"因果无延迟吃满事后端点"。
-/

import Strict.Classification

namespace NewChanlun.Origin.CausalEndpointImpossibility

open Strict (PrefixEq)

/-! ════════════════════════════════════════════════════════════════════
## §1 事件流与策略动作

PDF §9 把历史路径抽象为事件流 `ω : Nat → E`（时刻 → 事件），策略在每个时刻基于已观测的
历史输出一个动作。C23 只需区分"平仓"与"持有"两个动作（端点吃满 = 在笔结束时刻平仓）。
================================================================════════ -/

/-- 一个最小事件类型：每个时刻观测到一个 `Bar`（整数编码，仅用于构造前缀分叉的见证）。 -/
abbrev Bar : Type := Int

/--
**策略动作**：因果策略在每个时刻输出的动作。C23 只需区分
- `exit`：平仓（端点吃满要求在笔右端点 ρ 当下平仓）；
- `hold`：持有（笔延伸时要求继续持有）。
-/
inductive Action where
  | exit   -- 平仓（吃满端点 ⟹ 笔结束时刻必须 exit）
  | hold   -- 持有（笔延伸 ⟹ 必须 hold）
  deriving DecidableEq, Repr

/-- `exit ≠ hold`（动作互异，C23 矛盾的最后一步所需）。 -/
theorem exit_ne_hold : Action.exit ≠ Action.hold := by
  intro h; exact Action.noConfusion h

/-! ════════════════════════════════════════════════════════════════════
## §2 因果策略：动作只依赖前缀（复用 PrefixEq）

PDF §9 的"因果策略"= 时刻 t 的动作只依赖 `≤ t` 的已观测事件（前缀）。其精确结构形式正是
`Strict.PrefixEq`：`PrefixEq ω ω' t`（两流在 ≤ t 逐点相等）⟹ 策略在 t 的动作相同。
这与 `CausalOnlineSystem.causal`（分类器侧）同构，此处施加到**策略动作**侧。
================================================================════════ -/

/--
**因果策略** `CausalStrategy`：从事件流读出每个时刻的动作，满足**因果无前视**约束。

- `act : (Nat → Bar) → Nat → Action`：在流 `ω` 的时刻 `t` 输出动作。
- `causal`：`PrefixEq ω ω' t → act ω t = act ω' t`——时刻 t 的动作只依赖 t 及之前的事件
  （未来事件 索引 > t 不影响当下动作）。这是"实时因果约束下不能用未来端点信息"的结构编码。
-/
structure CausalStrategy where
  act    : (Nat → Bar) → Nat → Action
  causal : ∀ ω ω' t, PrefixEq ω ω' t → act ω t = act ω' t

/-! ════════════════════════════════════════════════════════════════════
## §3 端点吃满规格：要求区分前缀相同但未来不同的两条路径

PDF §9：笔右端点 ρ_b 只在未来确认时，"端点吃满"要求策略对
- ω（笔在 t 结束）⟹ 输出 `exit`（在 ρ=t 当下平仓）；
- ω′（笔延伸到未来）⟹ 输出 `hold`（继续持有）；
**且 ω、ω′ 在 t 及之前完全相同**（`PrefixEq ω ω' t`，端点须等未来数据才能区分）。

这正是"事后才确认的端点"的形式化：在时刻 t，可观测前缀无法区分 ω（已结束）与 ω′（未结束），
但端点吃满规格强行要求二者动作不同。
================================================================════════ -/

/--
**事后确认端点的吃满规格** `EndpointPerfectExit`：存在一个时刻 `t` 与两条事件流 `ω`、`ω'`，
- `pre  : PrefixEq ω ω' t`（在 t 及之前逐点相等——端点须等未来 index > t 才能区分）；
- `exitOmega : act ω  t = Action.exit`（在 ω 中该笔于 t 结束 ⟹ 端点吃满要求 t 当下平仓）；
- `holdOmega' : act ω' t = Action.hold`（在 ω' 中该笔延伸 ⟹ 要求继续持有）。

即：策略须在前缀完全相同的两条路径上，于同一时刻 t 输出**不同**动作（exit vs hold）——
这就是"无延迟吃满事后才确认的端点"的精确要求。
-/
def EndpointPerfectExit (act : (Nat → Bar) → Nat → Action) : Prop :=
  ∃ (ω ω' : Nat → Bar) (t : Nat),
    PrefixEq ω ω' t ∧ act ω t = Action.exit ∧ act ω' t = Action.hold

/-! ════════════════════════════════════════════════════════════════════
## §4 核心不可能定理：因果约束 ∧ 端点吃满规格 ⟹ False

PDF §9 反事实论证的形式化：因果策略对前缀相同的两条路径输出相同动作，但端点吃满规格
要求它们输出不同动作（exit ≠ hold），矛盾。
================================================================════════ -/

/--
★★**不可能定理 II（C23，因果策略不可无延迟吃满事后端点）核心矛盾**：
没有任何因果策略能满足事后确认端点的吃满规格。

证明（PDF §9 反事实论证）：设因果策略 `S` 满足端点吃满规格 `EndpointPerfectExit S.act`，
取见证 `ω`、`ω'`、`t`，其中 `PrefixEq ω ω' t`（前缀相同）、`S.act ω t = exit`、`S.act ω' t = hold`。
由因果约束 `S.causal ω ω' t pre`：`S.act ω t = S.act ω' t`。代入得 `exit = hold`，与 `exit ≠ hold` 矛盾。

**否定性结论**：因果约束（动作只依赖前缀）与端点吃满（区分前缀相同的路径）**不可同时成立**——
"实时因果系统无延迟吃满事后才确认的端点"逻辑上不可能。
-/
theorem causal_cannot_eat_endpoint (S : CausalStrategy) :
    ¬ EndpointPerfectExit S.act := by
  rintro ⟨ω, ω', t, hpre, hexit, hhold⟩
  -- 因果约束：前缀相同 ⟹ 动作相同
  have hsame : S.act ω t = S.act ω' t := S.causal ω ω' t hpre
  -- 但 端点吃满要求 exit（ω）≠ hold（ω'）
  rw [hexit, hhold] at hsame
  exact exit_ne_hold hsame

/--
★**等价对偶形式（全称否定）**：对任意因果策略，**不存在**满足吃满规格的见证三元组。
这是 `causal_cannot_eat_endpoint` 的展开形式，直接表达"无任何 (ω, ω', t) 见证端点吃满"。
-/
theorem no_causal_endpoint_witness (S : CausalStrategy) :
    ¬ ∃ (ω ω' : Nat → Bar) (t : Nat),
        PrefixEq ω ω' t ∧ S.act ω t = Action.exit ∧ S.act ω' t = Action.hold :=
  causal_cannot_eat_endpoint S

/-! ════════════════════════════════════════════════════════════════════
## §5 非平凡性：因果约束确有内容（去掉则规格可满足）

C23 若对任何策略都不可满足端点吃满（无论因果与否），则定理是同义反复（无内容）。
本节给一个**偷看未来**的非因果策略，证它**能**满足端点吃满规格——即不可能性**只来自因果约束**，
端点吃满本身在非因果策略下可实现。这关死"C23 平凡"指控（对应 Causal.lean 的 lookahead 非平凡性）。
================================================================════════ -/

/--
★**非因果策略（偷看未来）**：`peekAhead ω t` 读 `ω (t+1)`（时刻 t **之后**的事件）来决定动作——
若 `ω (t+1) = 0` 则 `exit`，否则 `hold`。它依赖 `PrefixEq` 不约束的索引（> t），故**非因果**。
-/
def peekAhead (ω : Nat → Bar) (t : Nat) : Action :=
  if ω (t + 1) = 0 then Action.exit else Action.hold

/--
★★**非平凡见证（L0）**：偷看未来的 `peekAhead` **满足**端点吃满规格 `EndpointPerfectExit`。

构造两条流 `ω`、`ω'`，在时刻 0 及之前逐点相等（`PrefixEq ω ω' 0`），但在未来索引 1 处不同
（`ω 1 = 0` ⟹ exit，`ω' 1 = 1` ⟹ hold）。这证端点吃满规格在**非因果**策略下**可实现**——
不可能性（`causal_cannot_eat_endpoint`）的全部责任在因果约束，端点吃满本身非自相矛盾。

⟹ C23 是**非平凡判据**：它真实区分"因果策略"（不可吃满端点）与"偷看未来的非因果策略"
（可吃满端点，但代价是用了未来数据——这正是 PDF §9 否定的）。
-/
theorem peekAhead_can_eat_endpoint : EndpointPerfectExit peekAhead := by
  -- ω：恒 0（含未来 ω 1 = 0）  ⟹ peekAhead ω 0 = exit
  -- ω'：仅未来索引 1 处为 1（ω' 1 = 1），其余 0 ⟹ peekAhead ω' 0 = hold
  let ω  : Nat → Bar := fun _ => 0
  let ω' : Nat → Bar := fun n => if n = 1 then 1 else 0
  refine ⟨ω, ω', 0, ?_, ?_, ?_⟩
  · -- PrefixEq ω ω' 0：索引 ≤ 0 即 i = 0，两流都取 0（i=1 的差异在前缀外，未来）
    intro i hi
    rw [Nat.le_zero.mp hi]
    show (0 : Int) = (if (0 : Nat) = 1 then 1 else 0)
    rw [if_neg (by decide : ¬ (0 : Nat) = 1)]
  · -- peekAhead ω 0 = exit：ω (0+1) = ω 1 = 0
    show (if ω 1 = 0 then Action.exit else Action.hold) = Action.exit
    rw [if_pos (rfl : (0 : Int) = 0)]
  · -- peekAhead ω' 0 = hold：ω' (0+1) = ω' 1 = 1 ≠ 0
    show (if ω' 1 = 0 then Action.exit else Action.hold) = Action.hold
    simp only [ω', if_pos rfl]
    rw [if_neg (by decide : ¬ (1 : Int) = 0)]

/--
★**防火墙总结（C23 顶层否定形式）**：因果约束 `causal` 与端点吃满规格 `EndpointPerfectExit`
**不可兼得**——任何携带 `causal` 证据的策略，其动作函数**不满足** `EndpointPerfectExit`。
而剥离因果约束（如 `peekAhead`），端点吃满**可实现**（§5 见证）。

下游任何声称"实时因果系统无延迟吃满事后确认端点"的模块，被此定理证伪：
因果约束在结构上排除了对前缀相同路径的动作区分，而端点吃满恰要求这种区分。
-/
theorem causal_xor_endpoint :
    (∀ S : CausalStrategy, ¬ EndpointPerfectExit S.act)
    ∧ (∃ act : (Nat → Bar) → Nat → Action, EndpointPerfectExit act) :=
  ⟨causal_cannot_eat_endpoint, ⟨peekAhead, peekAhead_can_eat_endpoint⟩⟩

end NewChanlun.Origin.CausalEndpointImpossibility
