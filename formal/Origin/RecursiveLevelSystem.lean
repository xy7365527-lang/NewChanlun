/-
  Origin/RecursiveLevelSystem.lean — #89「窗口化 ≥3-witness 递归」重锚到 Origin canonical 接口
  （task #99, A′ Phase2, cc-origin-recursive-relevel 工位）

  ── 存在论位置（A′ 重锚，非重证）─────────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。Origin 的递归级别
  系统接口 `Origin.RecursiveLevelSystem`（`Origin/TrendCompleteClassification.lean:137`）：
    structure RecursiveLevelSystem where
      State : Nat → Type
      lift  : (n : Nat) → State n → State (n + 1)
  配 `RecursiveLevelSystem.at`（从 base 逐级 lift）+ `recursive_level_self_similar`
  （`R.at base (n+1) = R.lift n (R.at base n)`，自相似递归）。

  #89 成果（`Formal/RecursiveConstruction.lean` + `Foundation/ChanlunInstantiation.lean`）把缠论
  「走势分解定理二」（zoushi 第17课：任何级别走势 ≥3 段次级别走势构成）忠实形式化为窗口化
  递归：第 n 级走势**序列** `ChanD n := List Move` 经 `composeStep n`（规范窗口族 `canonicalWindows`
  切多窗，每窗封装一个上级走势，中枢由窗口三段区间重叠真派生）产出第 n+1 级走势序列。反退化
  见证 `xs9`：9 段下级 → 3 窗口 → 3 个上级走势（length=3≥2，多上级反退化），每上级由 3 个**不同**
  下级 witness compose（`xs9_each_upper_three_distinct_subs`），满足 `MovesComposedFrom` 全约束
  （消费约束 3*3 ≤ 9）。

  ── 本文件做的事（reexport / 重锚，L0）──────────────────────────────────────
  把 #89 的 `composeStep` 实例化为 Origin `RecursiveLevelSystem` 的 `lift`，`State n := ChanD n`：
    chanRecursiveLevelSystem : Origin.RecursiveLevelSystem
      := { State := ChanD, lift := composeStep }
  然后在**Origin 接口下**重锚 #89 核心定理：
    (1) `chan_recursive_self_similar`：Origin `recursive_level_self_similar` 特化到缠论 lift
        （`chanRecursiveLevelSystem.at seed (n+1) = composeStep n (chanRecursiveLevelSystem.at seed n)`，
        rfl——Origin 自相似递归在缠论实例上机器成立）。
    (2) `chan_lift_eq_composeStep` / `chan_state_eq_listMove`：lift = composeStep、State n = List Move
        的接口对接等式（rfl，证 Origin 接口字段与 #89 构造逐字段相等，无语义漂移）。
    (3) `xs9` 见证在 Origin lift 下仍成立：`origin_lift_xs9`（`lift 0 xs9 = composeStep 0 xs9`，rfl）+
        reexport `xs9_movesComposedFrom`/`xs9_multi_upper`/`xs9_each_upper_three_distinct_subs`
        到 Origin 接口命名空间下（原定理直接引用，机器证据 rfl/decide）。

  ── 严格性声明（no-workaround / no-声明膨胀）────────────────────────────────
  本文件**零 sorry**。Origin `RecursiveLevelSystem` 接口（State:Nat→Type + lift）与 #89
  `composeStep : (n) → ChanD n → ChanD (n+1)`（ChanD n := List Move）**逐字段类型相容**——
  `State := ChanD`（Nat→Type 满足）、`lift := composeStep`（签名相同）。**无定义冲突**：Origin
  接口的 lift 不约束「单个走势 vs 走势序列」，#89 的序列载体（List Move）正是 State n 的合法取值。
  重锚是把已证 #89 结构填入 Origin 接口，得 Origin 接口下的缠论结论，不重证 #89 引理。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────
  全部 **L0**（reexport / 结构填充，信息增量为零——把已证 L0 #89 结构填入已证 L0 Origin 接口）。
  本文件**不**声称任何 L1+ 经验有效性。`xs9` 见证的 `WindowComposable`（有效域 ⊊ 定义域）由 #89
  `xs9_windowComposable` 承载，本文件只把它在 Origin 接口下重新暴露，不扩大有效域声明。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────
  RecursiveLevelSystem → {Origin.TrendCompleteClassification, Formal.RecursiveConstruction,
  ChanlunInstantiation}。不 import 其他 Phase2 新文件（保单文件验证有效）。
  验证：`cd formal && lake env lean Origin/RecursiveLevelSystem.lean`。

  谱系：#89（窗口化 ≥3-witness 递归，de-degenerate）→ #96/#97（A′ Origin canonical base）→
        本文件 #99（#89 重锚 Origin RecursiveLevelSystem 接口）。
  禁 sorry/admit/axiom。
-/

import Origin.TrendCompleteClassification
import Formal.RecursiveConstruction
import ChanlunInstantiation

namespace NewChanlun.Origin.RecursiveLevelReanchor

open Formal.RecursiveConstruction (Move MovesComposedFrom WellFormed)
open NewChanlun.ChanlunInstantiation
  (ChanD composeStep chanSeed segWit xs9
   composeStep_xs9 xs9_movesComposedFrom xs9_multi_upper
   xs9_each_upper_three_distinct_subs xs9_windowComposable WindowComposable)

/-! ════════════════════════════════════════════════════════════════════════
    § 1. #89 `composeStep` 实例化 Origin `RecursiveLevelSystem` 接口

    Origin `RecursiveLevelSystem`（`Origin/TrendCompleteClassification.lean:137`）字段：
      State : Nat → Type      ← 取 `ChanD`（第 n 级走势序列 `List Move`，#89 序列载体）
      lift  : (n) → State n → State (n+1)  ← 取 `composeStep`（#89 窗口化递归步，逐窗封装上级走势）

    `ChanD n := List Move` 是 `Nat → Type`（落 `Type`），满足 Origin `State` 字段类型；
    `composeStep : (n) → ChanD n → ChanD (n+1)` 签名逐字段等于 `lift` 签名。无定义冲突——
    Origin 接口的「单参数 lift」指「一个 State n 类型对象」，State n = List Move 是序列，#89
    的窗口化递归正作用于序列（走势分解定理二的忠实载体）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★缠论走势递归级别系统（#89 `composeStep` 实例化 Origin `RecursiveLevelSystem` 接口）。
    `State := ChanD`（第 n 级走势序列 List Move），`lift := composeStep`（窗口化逐级聚合）。
    这是 #89 窗口化 ≥3-witness 递归在 Origin canonical 接口下的 canonical 见证。 -/
def chanRecursiveLevelSystem : NewChanlun.Origin.RecursiveLevelSystem where
  State := ChanD
  lift := composeStep

/-- ★接口对接等式：Origin 接口的 `State` 字段 = #89 `ChanD`（第 n 级 = List Move）。rfl，
    证 Origin `State : Nat → Type` 字段被 #89 序列载体逐级填充，无类型漂移。

    ★命名消歧（强制全限定）：`Move` 在 `NewChanlun.Origin` 命名空间下被 `Origin.ChanlunElements`
    的 `structure Move`（元素层走势）遮蔽——本文件内嵌 `NewChanlun.Origin.RecursiveLevelReanchor`，
    无限定的 `Move` 会解析到元素层 Move（与 #89 的 `Formal.RecursiveConstruction.Move` 是**不同**
    类型）。#89 序列载体是 `Formal.RecursiveConstruction.Move`，故此处用**全限定名**确保对接的是
    #89 走势 μF 类型，非元素层 Move（两类型不同——这是名称消歧，非定义冲突）。 -/
theorem chan_state_eq_listMove (n : Nat) :
    chanRecursiveLevelSystem.State n = List Formal.RecursiveConstruction.Move := rfl

/-- ★接口对接等式：Origin 接口的 `lift` 字段 = #89 `composeStep`（窗口化递归步）。rfl，
    证 Origin `lift` 字段被 #89 窗口化递归逐字段填充，无语义漂移（恒等重锚）。 -/
theorem chan_lift_eq_composeStep (n : Nat) (xs : ChanD n) :
    chanRecursiveLevelSystem.lift n xs = composeStep n xs := rfl

/-! ════════════════════════════════════════════════════════════════════════
    § 2. Origin `recursive_level_self_similar` 特化到缠论 lift（自相似递归重锚）

    Origin `recursive_level_self_similar`（`Origin/TrendCompleteClassification.lean:146`）：
      R.at base (n+1) = R.lift n (R.at base n)
    把它特化到 `chanRecursiveLevelSystem`：缠论逐级聚合塔 `chanRecursiveLevelSystem.at seed (n+1)`
    恰等于「对第 n 级序列再施一次 `composeStep`」——Origin 自相似递归在缠论实例上机器成立。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★缠论递归层读出（Origin `RecursiveLevelSystem.at` 的缠论实例）：从种子序列 seed 经 n 步
    窗口化聚合到第 n 级走势序列。这把 #89 的逐级聚合塔挂到 Origin canonical 接口的 `at` 上。 -/
def chanLevelAt (seed : ChanD 0) (n : Nat) : ChanD n :=
  chanRecursiveLevelSystem.at seed n

/-- ★Origin 自相似递归重锚（`recursive_level_self_similar` 特化，L0）：缠论递归塔第 n+1 级
    = 对第 n 级序列施 `composeStep`。这是 Origin canonical 接口的自相似律在 #89 窗口化递归
    实例上的机器成立——第 n+1 级走势序列由第 n 级序列再聚合一次唯一得出（因果递归）。 -/
theorem chan_recursive_self_similar (seed : ChanD 0) (n : Nat) :
    chanRecursiveLevelSystem.at seed (n + 1)
      = composeStep n (chanRecursiveLevelSystem.at seed n) :=
  NewChanlun.Origin.recursive_level_self_similar chanRecursiveLevelSystem seed n

/-- ★自相似律的 `chanLevelAt` 形式（同上，用本层读出别名，rfl 链）。 -/
theorem chanLevelAt_self_similar (seed : ChanD 0) (n : Nat) :
    chanLevelAt seed (n + 1) = composeStep n (chanLevelAt seed n) :=
  chan_recursive_self_similar seed n

/-! ════════════════════════════════════════════════════════════════════════
    § 3. xs9 反退化见证在 Origin lift 下机器成立（重锚核心，rfl/decide 证据）

    #89 的 `xs9` 见证（9 段 → 3 窗口 → 3 上级走势，每上级 3 不同下级 + 真派生中枢）在
    Origin `RecursiveLevelSystem.lift` 接口下**逐字成立**：`lift 0 xs9 = composeStep 0 xs9`（rfl，
    因 `lift := composeStep`），故 #89 关于 `composeStep 0 xs9` 的全部定理直接 reexport 到
    Origin 接口下。督导闸三条（多上级 length≥2 / 每上级 3 不同下级 / MovesComposedFrom 全约束）
    在 Origin 接口下机器可检验。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★Origin lift 作用于 xs9 = #89 composeStep 作用于 xs9（rfl，恒等重锚的核心桥）：
    Origin 接口的 `lift 0` 施于 9 段见证序列 `xs9`，逐字等于 #89 的 `composeStep 0 xs9`。
    这把后续所有 #89 关于 `composeStep 0 xs9` 的见证桥接到 Origin 接口侧。 -/
theorem origin_lift_xs9 :
    chanRecursiveLevelSystem.lift 0 xs9 = composeStep 0 xs9 := rfl

/-- ★Origin 接口下 xs9 的递归层第 1 级 = composeStep 0 xs9（rfl，从种子 xs9 经 1 步聚合）。
    `chanLevelAt xs9 1 = lift 0 (at xs9 0) = lift 0 xs9 = composeStep 0 xs9`。 -/
theorem origin_chanLevelAt_one_xs9 :
    chanLevelAt xs9 1 = composeStep 0 xs9 := rfl

/-- ★【督导闸第一条·Origin 接口】多上级反退化：Origin lift 施于 xs9 产出**≥2** 个上级走势。
    经 `origin_lift_xs9` 桥接到 #89 `xs9_multi_upper`（length = 3 ≥ 2）。机器证据：rfl + #89 decide。 -/
theorem origin_lift_xs9_multi_upper :
    2 ≤ (chanRecursiveLevelSystem.lift 0 xs9).length := by
  rw [origin_lift_xs9]; exact xs9_multi_upper

/-- ★【督导闸第三条·Origin 接口】`MovesComposedFrom` 全约束：Origin lift 施于 xs9 的产物满足
    `MovesComposedFrom xs9 0 _`（非空 + 各上级 WellFormed + 消费约束 3*3≤9 + 有序窗口 witness）。
    经 `origin_lift_xs9` 桥接到 #89 `xs9_movesComposedFrom`（L0 机器证明）。 -/
theorem origin_lift_xs9_movesComposedFrom :
    MovesComposedFrom xs9 0 (chanRecursiveLevelSystem.lift 0 xs9) := by
  rw [origin_lift_xs9]; exact xs9_movesComposedFrom

/-- ★【督导闸第二条·Origin 接口】每上级元素由 3 个**不同**下级 witness compose（reexport）：
    每个上级走势的窗口三段是 `xs9` 的真实连续三段且两两不同（关死「同一走势复制」退化）。
    直接 reexport #89 `xs9_each_upper_three_distinct_subs`（机器证据：segWit 区间各异 decide）。 -/
theorem origin_xs9_each_upper_three_distinct_subs :
    (segWit 0 ≠ segWit 1 ∧ segWit 1 ≠ segWit 2 ∧ segWit 0 ≠ segWit 2)
    ∧ (segWit 3 ≠ segWit 4 ∧ segWit 4 ≠ segWit 5 ∧ segWit 3 ≠ segWit 5)
    ∧ (segWit 6 ≠ segWit 7 ∧ segWit 7 ≠ segWit 8 ∧ segWit 6 ≠ segWit 8) :=
  xs9_each_upper_three_distinct_subs

/-- ★有效域非空见证在 Origin 接口下重锚（formalization-validity-domain 231号）：
    `WindowComposable 0 xs9`（xs9 落在 composeStep 产物良构的结构有效域内，有效域 ⊊ 定义域但非空）。
    reexport #89 `xs9_windowComposable`——本文件**不扩大**有效域声明，只在 Origin 接口下暴露同一见证。 -/
theorem origin_xs9_windowComposable : WindowComposable 0 xs9 :=
  xs9_windowComposable

/-! ════════════════════════════════════════════════════════════════════════
    § 4. Origin 接口 canonical 见证暴露（与 OriginAdapters.asRecursiveLevel 对接）

    `chanRecursiveLevelSystem` 是 Origin `RecursiveLevelSystem` 的缠论 canonical 实例——
    下游可经 `OriginAdapters.asRecursiveLevel`（恒等 adapter）把它作为 Origin 接口见证暴露。
    本文件**不** import OriginAdapters（保单文件验证有效，避免引入额外 Phase2 依赖），仅在此
    声明缠论实例本身已是合法 Origin 接口对象（类型即证）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★缠论递归级别系统是合法 Origin 接口对象（类型层见证，rfl 合取）：`chanRecursiveLevelSystem`
    既是命名的 `Origin.RecursiveLevelSystem`（类型即证），又满足 #89 接口对接全等式——State 逐级
    = 第 n 级缠论走势序列 `List Move`，lift 逐 = #89 窗口化递归步 `composeStep`。这坐实「#89 重锚
    到 Origin RecursiveLevelSystem 接口」成功（接口字段被 #89 结构逐字段填充，无语义漂移）。

    ★直接对具体见证 `chanRecursiveLevelSystem` 陈述（非抽象 `∃ R`）：抽象 R 的 `R.State n` 与
    `ChanD n` 不 defeq（R 字段未知），会使 `xs : ChanD n` 与 `R.lift n` 类型不容。对具体实例则
    `State n = ChanD n` defeq，类型相容，每条 rfl。 -/
theorem chan_is_origin_recursive_level_system :
    (∀ n, chanRecursiveLevelSystem.State n = List Formal.RecursiveConstruction.Move)
    ∧ (∀ n (xs : ChanD n), chanRecursiveLevelSystem.lift n xs = composeStep n xs) :=
  ⟨fun _ => rfl, fun _ _ => rfl⟩

end NewChanlun.Origin.RecursiveLevelReanchor
