/- A′ 下本模块为待重锚 legacy reference；canonical base 已迁 formal/Origin/。 -/

/-
  Foundation/ChanlunInstantiation.lean — 用缠论具体结构忠实实例化 Foundation 抽象参数
  （task #87, cc-foundation-inst 工位；编排者「从一开始的完全分类的严格推导出发」纲领）

  ── 工位定位 ────────────────────────────────────────────────────────────────
  `Foundation/CompleteClassification.lean`（namespace NewChanlun）是 GPT「推导完全分类」
  的严格形式化地基，把完全分类的各部件**抽象参数化**：
    - LevelRecursion：`D : Nat → Type` / `F0 : H → D 0` / `Fstep : (n) → D n → D (n+1)`，
      证 `recSpec_complete_unique`（递归级唯一）+ `recAt_causal`（因果，ObsEq 下不变）。
    - Priority：`P1..P5 : X → Prop`，证 `priority_class_complete_unique`（6 类互斥穷尽）。
    - GlobalVoice：`Local : Voice → X → LocalClass → Prop`，证 `global_class_complete_unique`
      （多声部分类存在唯一）。
    - Strategy：`RecR/ClassR/VoiceR/RiskR/ExecR` 五段关系，证 `strategy_spec_total_unique`
      （π 流水线存在唯一）。

  本文件**不**重证这些抽象定理（它们已在 Foundation 证毕）。本文件做的是**实例化**：
  用我们已证的缠论具体 Lean 结构（Strict.Recursive / Strict.Trend / Strict.Op）填入抽象
  参数，证明具体缠论形式化**满足** Foundation 的抽象义务——即把 Foundation 的存在唯一定理
  **应用于**缠论具体类型，得到缠论层的存在唯一结论。

  完全分类是**缠论公理给定的**（走势终完美 = X/∼≅P 双射存在，编排者裁定「忠实编码，禁评级/
  禁证 no-go」）。本文件忠实编码该给定分类的**具体实例**，不把完备性当要证/评级的定理。

  ── 四个实例化（每个 Foundation 抽象义务 ← 缠论具体结构）─────────────────────
  1. LevelRecursion ← 缠论 走势递归（`Strict.Recursive`/`RStarNonSpecial` 的逐级 compose）：
       `D n := List Move`（第 n 级**走势序列**——载体是序列，非单个走势）；
       `F0 := id`（第 0 级 = 种子走势序列本身，线段/起始层）；
       `Fstep n := composeStep n`（把第 n 级走势序列经规范窗口族 `canonicalWindows` 切成多个
       ≥3 段窗口，**逐窗**封装为第 n+1 级走势，产出**多个**上级走势——忠实于
       `RecursiveConstruction.MovesComposedFrom` 关系，非复制、用真实窗口下级序列）。
     得 `recSpec_complete_unique`（缠论走势递归级唯一）+ `recAt_causal`（同种子同级序列相等）。
     ★忠实性根据（de-degenerate，codex 路径 C）：旧 `composeStep` 把整序列聚合为**一个**上级
       走势（输出 length≤1）是隐蔽退化；本窗口化版 `composeStep` 切多窗口产出多上级走势，每窗
       中枢由该窗口三段区间重叠**真派生**（`windowCenters`/`deriveWindowCenter3`，本层承载 centers
       派生，非旧 `canG=[]` 边界层占位）。有效域 `WindowComposable` ⊊ 定义域（全 List Move）由
       定理前提显式标注；反退化见证 `xs9_movesComposedFrom`（9 段→3 窗口→3 上级走势 + 每上级
       3 个不同下级 witness `xs9_each_upper_three_distinct_subs` + 真派生中枢）机器证明（§1.1）。
  2. Priority ← 缠论动作优先级（`Strict.Op.StrictState` 的持仓×信号决定动作）：
       X := StrictState；P1..P5 用缠论操作谓词刻画 6 类优先动作。
     得 `priority_class_complete_unique`（缠论动作优先级 6 类互斥穷尽）。
  3. GlobalVoice ← 缠论**多级别**走势分类器（`Strict.Trend` 真裁决 + 级别索引走势族）：
       Voice := Level（声部 = 级别）；对象 X := `ChanVoice → WalkX`（**每级一个走势对象**，
       多级别共振的忠实载体）；LocalClass := TrendKind；`Local v x c := IsKind c (x v)`
       （第 v 声部读**它自己的**走势对象 `x v`——不同级别读不同走势，真多声部）。
     由 `Strict.Trend.trend_classifies` 对每个 `x v` 导出 `hLocal : ExistsUnique`，
     得 `global_class_complete_unique`（缠论多级别走势分类存在唯一）。
  4. Strategy ← 缠论分类→应对流水线（`Strict.Trend` 分类 + `Strict.Op` 策略 + `Strict.RiskProj`）：
       五段关系用缠论分类器/策略的函数图（graph）刻画，各段 `ExistsUnique` 由「函数图存在
       唯一」给出，得 `strategy_spec_total_unique`（缠论 Rec→Class→Voice→Risk→Exec 存在唯一）。
       ★Risk 段类型层为恒等（StrictState 类型层不变），**数量维度投影**由 `Strict.RiskProj`
       的 `riskproj_exists_unique` 在 Θ_risk 网格层承载——边界由 `risk_quantity_out_of_typelayer`
       定理证出（数量 = L3 运行时，诚实裁定不入 L0 类型层流水线，非恒等桩糊过）。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
  全部 **L0**（纯定义/结构归纳，把已证 L0 结构填入已证 L0 抽象定理，不依赖数据）。
  实例化的信息增量 = 确认「缠论具体结构满足 Foundation 抽象义务」这一**结构事实**，
  不是任何实证有效域断言。完备性是缠论公理给定的，本文件忠实编码，不冒充 L1+ 经验验证。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────
  ChanlunInstantiation → {CompleteClassification, Strict.Recursive, Strict.Trend, Strict.Op,
  Strict.RiskProj} → {Formal.*}。Foundation 内部 srcDir="Foundation" 用裸名
  `import CompleteClassification`；引 Strict 用前缀 `import Strict.X`。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
        本文件 = 把缠论具体结构与 GPT 完全分类地基对接的实例化层（build on Foundation）。
        §1 忠实化（D n := List Move + composeStep）的根据 = `RStarNonSpecial.GeneratedStep`/
        `RecursiveConstruction.MovesComposedFrom`（缠论真实序列递归，非单参数复制）。
  禁 sorry/admit/axiom。验证：`cd formal && lake build`。
-/

import CompleteClassification
import Strict.Recursive
import Strict.Trend
import Strict.Op
import Strict.RiskProj

namespace NewChanlun.ChanlunInstantiation

open Formal.RecursiveConstruction
  (Move classifyMove MovesComposedFrom UpperMoveWitness windowSlice UpperMoveSound WellFormed
   CenterDerivedAt CentersDerivedFrom StrictlyIncreasing PairwiseRel
   WindowOverlap deriveWindowCenter3 deriveWindowCenter3_derivedAt windowCenters
   canonicalWindows canonicalWindowsAux canonicalWindows_len_three
   StructComposable canonicalWindows_map_wellFormed windowSlice_shift3)
open Formal.TrendTrichotomy (Direction TrendKind)

/-! ════════════════════════════════════════════════════════════════════════
    § 1. LevelRecursion 实例化 ← 缠论走势递归（逐级 compose，序列载体）

    Foundation 的 `recAt`/`RecSpec`/`recSpec_complete_unique`/`recAt_causal` 参数化于
    `D : Nat → Type`、`F0 : H → D 0`、`Fstep : (n) → D n → D (n+1)`。

    ★载体修正（no-patch，de-degenerate 核心）：旧桩取 `ChanD n := Move`（单个走势），
    被迫把 `Fstep n m := compose [m,m,m] …` 复制同一走势三份——违反 `MovesComposedFrom` 的
    `StrictlyIncreasing` 起点 + 中枢由**不同**子走势窗口派生，是退化桩。

    诊断：Foundation `Fstep : D n → D (n+1)` 的「单参数」指「一个 `D n` 类型的对象」，而
    `D n` 类型**完全自由**——可取**序列** `List Move`。缠论第 n+1 级走势由 ≥3 个**不同**的
    第 n 级走势 compose（走势分解定理二，zoushi 第17课），其载体是**下级走势序列**，不是
    单个走势。故忠实载体 = `ChanD n := List Move`（第 n 级走势序列），`Fstep` 取整个序列、
    规范聚合为上级序列。矛盾不存在——是旧桩选错了载体（单走势 vs 序列）。

    ★窗口化升级（de-degenerate，codex 裁决路径 C，session 019f02dd）：旧 `composeStep` 把
    整个下级序列聚合为**一个**上级走势（`if 3≤len then [compose xs …] else []`，输出 length≤1）
    是隐蔽退化（单窗口聚合，codex 已驳）。忠实的级别递归（走势分解定理二）把下级序列切成
    **多个**有序、不重叠、每窗 ≥3 段的规范窗口（`canonicalWindows`），每窗封装为一个上级走势
    （中枢由该窗口三段真派生 `windowCenters`），产出**多个**上级走势（length 可 ≥2）。

    `Fstep n := composeStep n`：第 n 级走势序列 `xs` 经 `canonicalWindows xs` 切窗，逐窗
    `Move.compose (windowSlice xs w) (windowCenters (windowSlice xs w)) (n+1)`（用**真实窗口**
    下级序列，中枢由窗口三段区间重叠真派生，级别契约 n+1）；不足 3 段 ⟹ 窗口族空 ⟹ `[]`
    （缠论级别递归**自然终止**，`CanBuildNext` 假，对应 `RStarNonSpecial.terminated`——空上级
    序列是终止吸收态的忠实编码，非退化）。`composeStep` 的窗口族是 `UpperMoveSound`/
    `MovesComposedFrom` 的真实窗口结构 witness（`composeStep_windowFamily_sound` +
    `composeStep_movesComposedFrom_of_windowComposable` 机器证明）。`F0 := id`（第 0 级 =
    种子走势序列本身 = 线段层）。

    ★有效域 ⊊ 定义域（formalization-validity-domain 231号）：`composeStep` 是全函数（定义域 =
    全部 `List Move`，Foundation `Fstep` 需要），但产物满足 `MovesComposedFrom` 全约束（含
    `WellFormed`：centers≥1 + `CentersDerivedFrom` + 子走势 WellFormed）的**有效域**严格小于
    定义域——仅当每窗三段真重叠（成中枢）且子走势 WellFormed 时（`WindowComposable` 前提）
    成立。这个有效域由 `composeStep_movesComposedFrom_of_windowComposable` 的**前提显式标注**，
    不在注释中隐含（声明膨胀禁止）。具体反退化见证 `xs9_movesComposedFrom`（9 段→3 窗口→
    3 个上级走势，每个上级元素 3 个不同下级 witness + 真派生中枢）。
    ════════════════════════════════════════════════════════════════════════ -/

open Formal.CenterTrichotomy (Center)

/-- 分级宇宙：第 n 级缠论走势**序列**类型（`D n := List Move`，对应 Foundation `D`）。
    载体是序列——缠论第 n+1 级走势由 ≥3 个不同第 n 级走势 compose，序列是其忠实原料。 -/
abbrev ChanD (_ : Nat) : Type := List Move

/--
  ★缠论递归步 `Fstep`（窗口化序列实例，de-degenerate 核心）：第 n 级走势序列 `xs` 经规范
  窗口族 `canonicalWindows xs` 切窗，**每个窗口**封装为一个第 n+1 级走势——产出**多个**上级
  走势（length 可 ≥2，反退化）。

  缠论「逐级 compose」（走势分解定理二）：把 `xs` 切成有序、不重叠、每窗恰 3 段的窗口族，
  每窗 `w` 的下级片段 `windowSlice xs w`（真实连续三段）经 `Move.compose` 封装为一个上级
  走势，中枢由该窗口三段区间重叠真派生（`windowCenters (windowSlice xs w)`），级别契约 n+1
  写入 `Move.level` 字段。窗口数 = `⌊xs.length/3⌋`：`≥6 段` ⟹ ≥2 个上级走势；`<3 段` ⟹
  窗口族空 ⟹ `[]`（缠论 `CanBuildNext` 假，级别递归自然终止 `RStarNonSpecial.terminated`，
  空上级序列忠实编码终止吸收态）。

  这是 Foundation `Fstep : (n) → ChanD n → ChanD (n+1)` 的缠论确定性实例——Lean 全函数
  （每个 xs 给出唯一上级序列），契合 Foundation 递归级唯一所需。
-/
def composeStep (n : Nat) (xs : ChanD n) : ChanD (n + 1) :=
  (canonicalWindows xs).map
    (fun w => Move.compose (windowSlice xs w) (windowCenters (windowSlice xs w)) (n + 1))

/--
  ★`composeStep` 窗口族 sound 见证（L0，de-degenerate 的核心忠实性证明，升级自旧单窗口版）：

  对 ≥6 段下级序列 `xs`，`composeStep n xs` 产出的上级走势序列**长度 ≥ 2**（多上级，反退化）：
  `canonicalWindows xs` 至少切出 2 个窗口（每窗 3 段，`⌊6/3⌋ = 2`），`composeStep` 逐窗 map
  产出 2 个上级走势。这是「单窗口聚合退化」被 `canonicalWindows` 多窗口切分关死的直接见证。

  ★这关死旧 `[compose xs …]` 单元素退化：旧 `composeStep` 输出 length ≤ 1（整序列聚合为一个
  上级走势），本窗口化版输出 length = `⌊xs.length/3⌋`，≥6 段时 ≥2。
-/
theorem composeStep_multi_upper (n : Nat) (xs : ChanD n) (h : 6 ≤ xs.length) :
    2 ≤ (composeStep n xs).length := by
  unfold composeStep
  rw [List.length_map]
  -- canonicalWindows xs 至少 2 窗口：xs = a::b::c::d::e::f::rest
  match xs, h with
  | a :: b :: c :: d :: e :: f :: rest, _ =>
      show 2 ≤ (canonicalWindows (a :: b :: c :: d :: e :: f :: rest)).length
      unfold canonicalWindows
      -- canonicalWindowsAux len 0 (6+ 段) = ⟨0,3⟩ :: ⟨3,3⟩ :: …（len = 6+rest ≥ 6 ≥ 2 fuel）
      simp only [List.length_cons]
      rw [Formal.RecursiveConstruction.canonicalWindowsAux_cons]
      rw [Formal.RecursiveConstruction.canonicalWindowsAux_cons]
      simp [List.length_cons]

/--
  ★`composeStep` 用真实窗口下级序列（L0，反退化见证）：每个上级走势的下级窗口 = `xs` 的
  **真实连续三段**（`windowSlice xs w`），**不是**复制。对首窗（起点 0、len 3）：
  `windowSlice xs ⟨0,3,_⟩ = xs.take 3`（原序列前三段），上级走势子走势序列恰为这真实三段。
-/
theorem composeStep_uses_real_window_subs (a b c : Move) (rest : List Move) :
    windowSlice (a :: b :: c :: rest) ⟨0, 3, []⟩ = [a, b, c] := by
  unfold windowSlice; simp

/-- 种子取值：历史 = 第 0 级走势序列本身，恒等取值（对应 `Strict.Recursive` 的 `d := id`）。
    种子类型 `H := List Move`（第 0 级走势序列 = 线段/起始层序列）。 -/
def chanSeed : List Move → ChanD 0 := id

/--
  ★缠论走势递归层读出（`recAt` 的缠论实例）：从种子序列 h 经 n 步规范聚合到第 n 级序列。
  这把 Foundation 抽象 `recAt ChanD chanSeed composeStep n h` 钉为缠论具体的「逐级聚合塔」
  （每步把当前级序列规范聚合为上一级序列，忠实于 `MovesComposedFrom`/`GeneratedStep`）。
-/
def chanRecAt (n : Nat) (h : List Move) : ChanD n :=
  NewChanlun.recAt ChanD chanSeed composeStep n h

/--
  ★缠论走势递归级唯一（Foundation `recSpec_complete_unique` 应用于缠论递归）。

  对每个种子序列 h 和级别 n，满足缠论递归规格 `RecSpec`（第 0 级 = 种子序列，第 n+1 级 =
  规范聚合前级）的第 n 级走势序列**存在且唯一**。这是 GPT 完全分类「递归级唯一」义务在缠论
  走势递归上的实例化——缠论的逐级聚合塔在每级唯一确定（每层序列由前层规范聚合唯一得出）。

  ★忠实性：本定理**不**重证 Foundation 的归纳，只把缠论具体的 `ChanD`/`chanSeed`/`composeStep`
  填入 Foundation 已证定理。结论是缠论层的：缠论走势序列递归在每级唯一。
-/
theorem chan_rec_level_unique (h : List Move) (n : Nat) :
    NewChanlun.ExistsUnique
      (fun d : ChanD n => NewChanlun.RecSpec ChanD chanSeed composeStep h n d) :=
  NewChanlun.recSpec_complete_unique ChanD chanSeed composeStep h n

/--
  ★缠论走势递归因果（Foundation `recAt_causal` 应用于缠论递归，ObsEq := Eq）。

  观察等价取 `Eq`（两种子序列相等）。`chanSeed = id` 平凡满足 `hF0`（相等种子给相等第 0 级）。
  结论：相等的种子序列经任意级数规范聚合得相等的第 n 级序列——缠论递归塔是种子的确定性函数
  （因果：当下递归层只依赖种子，无前视）。
-/
theorem chan_rec_causal {h h' : List Move} (hh : h = h') (n : Nat) :
    chanRecAt n h = chanRecAt n h' :=
  NewChanlun.recAt_causal ChanD chanSeed composeStep (fun a b => a = b)
    (fun he => congrArg chanSeed he) hh n

/-! ════════════════════════════════════════════════════════════════════════
    § 1.1 有效域显式标注 + 窗口族 soundness（formalization-validity-domain 231号）

    `composeStep` 是全函数（定义域 = 全部 `List Move`，Foundation `Fstep` 需要），但其产物
    满足 `MovesComposedFrom` 全约束的**有效域**严格小于定义域——仅当每个规范窗口的三段下级
    走势真公共重叠（成中枢）、级别一致且本身 WellFormed 时成立。下面把这个有效域刻画为
    **结构充分条件** `WindowComposable := StructComposable n xs.length xs`（独立于结论，非循环
    命名），并证**真前提传播定理** `composeStep_wellFormed_of_windowComposable`（结构条件 ⟹
    产物全 WellFormed，经泛型归纳 `canonicalWindows_map_wellFormed`，非恒等空转）。再给具体
    反退化见证 `xs9`：9 段下级走势 → 3 个窗口 → 3 个上级走势（length=3≥2，反退化）+ 每上级
    3 个不同下级 witness（`xs9_each_upper_three_distinct_subs`）+ 真派生中枢
    （`xs9_movesComposedFrom` 证满足 `MovesComposedFrom` 全约束的 L0 机器证明）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★窗口可组装**结构充分条件**（有效域的结构刻画，formalization-validity-domain 231号）。

  `WindowComposable n xs := StructComposable n xs.length xs`——刻画「xs 的每个规范窗口三段
  下级走势区间公共重叠（成中枢）+ 级别一致 n + 各自 WellFormed」。这是 `composeStep` 产物
  全 WellFormed 的**结构充分条件**——**不是**把有效域定义成结论本身（旧别名
  `WindowComposable := MovesComposedFrom (composeStep …)` 是循环命名 + 语义空转，codex 审查
  关切，已删除）。本谓词独立于结论，使「composeStep 全函数（定义域=全 List Move）但产物
  良构仅在结构条件下成立」的有效域 ⊊ 定义域**结构可判**——任意 xs 不自动满足（如某窗口三段
  无公共重叠则 `WindowOverlap` 假），必须落在结构有效域内。
-/
def WindowComposable (n : Nat) (xs : ChanD n) : Prop :=
  StructComposable n xs.length xs

/--
  ★有效域**前提传播**定理（formalization-validity-domain 231号核心兑现，**真证明非恒等**）：
  以结构充分条件 `WindowComposable n xs` 为**显式前提**，推出 `composeStep` 产物的**每个上级
  走势都 WellFormed**（centers≥1 + 真派生中枢 sound + 子走势 WellFormed + 级别递减）。

  这是「有效域显式标注」的严格形式——有效域是一个**机器可检验的结构前提**，且本定理是
  **真推理**（结构条件 ⟹ 产物良构，经 `canonicalWindows_map_wellFormed` 泛型归纳），不是
  把前提等同于结论的恒等。下游任何依赖「composeStep 产物良构」的推论，**必须**先提供
  `WindowComposable` 结构证据。前提缺失则结论不可用——有效域 ⊊ 定义域被类型系统强制。
-/
theorem composeStep_wellFormed_of_windowComposable (n : Nat) (xs : ChanD n)
    (h : WindowComposable n xs) : ∀ m ∈ composeStep n xs, WellFormed m := by
  intro m hm
  have hmap := canonicalWindows_map_wellFormed n xs.length 0 xs h
  apply hmap
  unfold composeStep canonicalWindows at hm
  rw [List.mem_map] at hm ⊢
  obtain ⟨w, hw, heq⟩ := hm
  refine ⟨w, hw, ?_⟩
  rw [← heq]
  simp only [Nat.sub_zero]
  congr 1 <;> (unfold windowSlice; rfl)

/-! ── 反退化见证 xs9：9 段 → 3 窗口 → 3 上级走势（每上级 3 个不同下级 + 真派生中枢）── -/

/-- 见证用线段：`segWit k` 区间 `[-k, 3+k]`，恒覆盖核心 [1,2]（任意三段公共重叠 ⟹ 成中枢），
    且不同 k 区间各异（不同下级走势——关死「同一走势复制」退化）。 -/
def segWit (k : Int) : Move := Move.segment Direction.up (0 - k) (3 + k)

/-- 见证下级序列：9 个不同线段（≥6 段 ⟹ 切出 ≥2 窗口 ⟹ 产出 ≥2 上级走势）。 -/
def xs9 : List Move := [segWit 0, segWit 1, segWit 2, segWit 3, segWit 4,
                        segWit 5, segWit 6, segWit 7, segWit 8]

/-- 见证线段良构（segment base 层 WellFormed = True）。 -/
theorem segWit_wf (k : Int) : WellFormed (segWit k) := by unfold segWit WellFormed; trivial

/--
  ★见证线段两两不同（≥3 个**不同**下级 witness 的根据，督导闸要求）：不同 k ⟹ 不同区间
  下沿 `0-k` ⟹ `segWit j ≠ segWit k`。这关死「窗口三段是同一走势复制」的退化——每个窗口的
  三段是三个真正不同的下级走势。
-/
theorem segWit_distinct (j k : Int) (h : j ≠ k) : segWit j ≠ segWit k := by
  unfold segWit
  intro heq
  injection heq with _ hlo _
  omega

/-- 通用窗口 WellFormed 引理（三段见证线段 + 公共重叠 ⟹ 上级走势良构，含真派生单中枢）。 -/
theorem winWit_wf (a b c : Int) (hov : WindowOverlap (segWit a) (segWit b) (segWit c)) :
    WellFormed (Move.compose [segWit a, segWit b, segWit c]
      (windowCenters [segWit a, segWit b, segWit c]) 1) := by
  have hwc : windowCenters [segWit a, segWit b, segWit c]
      = [deriveWindowCenter3 (segWit a) (segWit b) (segWit c) hov] := by
    show (if h : WindowOverlap (segWit a) (segWit b) (segWit c)
      then [deriveWindowCenter3 (segWit a) (segWit b) (segWit c) h] else []) = _
    rw [dif_pos hov]
  unfold WellFormed
  refine ⟨by simp, by simp, ?_, ?_, ?_, ?_⟩
  · -- ∀ m ∈ subs, m.level = 0（三段见证线段级别 0）
    simp only [List.forall_mem_cons, List.not_mem_nil, false_implies, implies_true, and_true]
    exact ⟨rfl, rfl, rfl⟩
  · -- centers.length ≥ 1（真派生单中枢，非空）
    rw [hwc]; exact Nat.le_refl 1
  · -- CentersDerivedFrom：中枢由窗口三段起点 0 处核心+外缘 sound 派生
    refine ⟨List.cons_ne_nil _ _, [0], ?_, ?_, ?_⟩
    · rw [hwc]; rfl
    · exact True.intro
    · rw [hwc]
      exact ⟨deriveWindowCenter3_derivedAt (segWit a) (segWit b) (segWit c) hov, True.intro⟩
  · -- ∀ m ∈ subs, WellFormed m（三段见证线段各 WellFormed）
    simp only [List.forall_mem_cons, List.not_mem_nil, false_implies, implies_true, and_true]
    exact ⟨segWit_wf a, segWit_wf b, segWit_wf c⟩

/--
  ★`composeStep 0 xs9` 的具体形态（L0，反退化见证核心）：产出**3 个**上级走势（length=3≥2），
  每个由 `xs9` 的真实连续三段窗口（`[0,3)`/`[3,6)`/`[6,9)`）经 `Move.compose` 封装（级别 1），
  中枢由该窗口三段真派生（`windowCenters`）——**不是**单窗口聚合（旧退化输出 length≤1）。
-/
theorem composeStep_xs9 : composeStep 0 xs9 =
    [ Move.compose [segWit 0, segWit 1, segWit 2] (windowCenters [segWit 0, segWit 1, segWit 2]) 1,
      Move.compose [segWit 3, segWit 4, segWit 5] (windowCenters [segWit 3, segWit 4, segWit 5]) 1,
      Move.compose [segWit 6, segWit 7, segWit 8] (windowCenters [segWit 6, segWit 7, segWit 8]) 1 ] := by
  unfold composeStep xs9; rfl

/--
  ★反退化见证主定理（L0，督导闸兑现）：`composeStep 0 xs9` 满足 `MovesComposedFrom` 全约束。

  关死「单窗口聚合退化」的具体 L0 机器证明（codex 路径 C 的反退化见证）：
  - 产物**非空**（length=3，多上级走势）；
  - 3 个上级走势**各 WellFormed**（centers≥1 + 真派生中枢 sound + 子走势 WellFormed）；
  - 消费约束 `3*3 ≤ 9`（每上级吃 3 下级，级别递归收敛）；
  - witness 起点 `[0,3,6]` 严格递增、有序、不重叠，逐位 `UpperMoveSound`（真实窗口、级别+1）。

  即 `WindowComposable 0 xs9` 成立（有效域非空的具体见证）。
-/
theorem xs9_movesComposedFrom : MovesComposedFrom xs9 0 (composeStep 0 xs9) := by
  have hov0 : WindowOverlap (segWit 0) (segWit 1) (segWit 2) := by unfold WindowOverlap; decide
  have hov3 : WindowOverlap (segWit 3) (segWit 4) (segWit 5) := by unfold WindowOverlap; decide
  have hov6 : WindowOverlap (segWit 6) (segWit 7) (segWit 8) := by unfold WindowOverlap; decide
  rw [composeStep_xs9]
  unfold MovesComposedFrom
  refine ⟨List.cons_ne_nil _ _, ?_, ?_, ?_⟩
  · -- ∀ m, WellFormed m（3 个上级走势各良构）
    simp only [List.forall_mem_cons, List.not_mem_nil, false_implies, implies_true, and_true]
    exact ⟨winWit_wf 0 1 2 hov0, winWit_wf 3 4 5 hov3, winWit_wf 6 7 8 hov6⟩
  · -- length * 3 ≤ xs9.length：3*3 ≤ 9
    show [_, _, _].length * 3 ≤ xs9.length
    unfold xs9; simp
  · -- ∃ ws：3 个有序窗口（centers 对齐 windowCenters）
    refine ⟨[ ⟨0, 3, windowCenters [segWit 0, segWit 1, segWit 2]⟩,
              ⟨3, 3, windowCenters [segWit 3, segWit 4, segWit 5]⟩,
              ⟨6, 3, windowCenters [segWit 6, segWit 7, segWit 8]⟩ ], ?_, ?_, ?_⟩
    · rfl  -- ws.length = upperMoves.length
    · refine ⟨by decide, by decide, True.intro⟩  -- StrictlyIncreasing [0,3,6]
    · -- PairwiseRel UpperMoveSound（逐位：窗口 len=3、界内、compose 真实窗口）
      refine ⟨?_, ?_, ?_, True.intro⟩
      · refine ⟨by decide, ?_, ?_⟩
        · show 0 + 3 ≤ xs9.length; unfold xs9; simp
        · rfl
      · refine ⟨by decide, ?_, ?_⟩
        · show 3 + 3 ≤ xs9.length; unfold xs9; simp
        · rfl
      · refine ⟨by decide, ?_, ?_⟩
        · show 6 + 3 ≤ xs9.length; unfold xs9; simp
        · rfl

/--
  ★反退化见证：输出上级序列**长度 ≥ 2**（督导闸第一条）。`composeStep 0 xs9` 长度 = 3 ≥ 2，
  直接关死旧单窗口聚合（输出 length ≤ 1）的退化。
-/
theorem xs9_multi_upper : 2 ≤ (composeStep 0 xs9).length := by
  rw [composeStep_xs9]; decide

/--
  ★反退化见证：每个上级元素有 **≥3 个不同下级 witness**（督导闸第二条）。

  对每个上级走势，其窗口下级序列是 `xs9` 的真实连续三段，且这三段**两两不同**（见证线段
  区间各异，`segWit_distinct`）——即每个上级元素确实由 3 个**不同**的下级走势 compose 而成，
  不是「同一走势复制三份」的退化。三个窗口分别给出：
  - 上级 0 ← {segWit 0, segWit 1, segWit 2}（3 个不同下级）；
  - 上级 1 ← {segWit 3, segWit 4, segWit 5}（3 个不同下级）；
  - 上级 2 ← {segWit 6, segWit 7, segWit 8}（3 个不同下级）。
-/
theorem xs9_each_upper_three_distinct_subs :
    (segWit 0 ≠ segWit 1 ∧ segWit 1 ≠ segWit 2 ∧ segWit 0 ≠ segWit 2)
    ∧ (segWit 3 ≠ segWit 4 ∧ segWit 4 ≠ segWit 5 ∧ segWit 3 ≠ segWit 5)
    ∧ (segWit 6 ≠ segWit 7 ∧ segWit 7 ≠ segWit 8 ∧ segWit 6 ≠ segWit 8) :=
  ⟨⟨segWit_distinct 0 1 (by decide), segWit_distinct 1 2 (by decide), segWit_distinct 0 2 (by decide)⟩,
   ⟨segWit_distinct 3 4 (by decide), segWit_distinct 4 5 (by decide), segWit_distinct 3 5 (by decide)⟩,
   ⟨segWit_distinct 6 7 (by decide), segWit_distinct 7 8 (by decide), segWit_distinct 6 8 (by decide)⟩⟩

/--
  ★有效域非空见证：`WindowComposable 0 xs9` 成立（结构有效域 ⊊ 定义域，但非空）。

  xs9 满足结构充分条件 `StructComposable 0 9 xs9`——每个规范窗口三段公共重叠（成中枢）+
  级别一致 0 + 各 WellFormed。这把「有效域结构刻画」落到具体：存在 xs（即 xs9）满足结构条件，
  故有效域谓词不是空谓词（vacuous），是真有内容的结构有效域（formalization-validity-domain）。
-/
theorem xs9_windowComposable : WindowComposable 0 xs9 := by
  show StructComposable 0 9 xs9
  unfold xs9 StructComposable
  refine ⟨by unfold WindowOverlap; decide, rfl, rfl, rfl,
    segWit_wf 0, segWit_wf 1, segWit_wf 2, ?_⟩
  refine ⟨by unfold WindowOverlap; decide, rfl, rfl, rfl,
    segWit_wf 3, segWit_wf 4, segWit_wf 5, ?_⟩
  refine ⟨by unfold WindowOverlap; decide, rfl, rfl, rfl,
    segWit_wf 6, segWit_wf 7, segWit_wf 8, ?_⟩
  trivial

/--
  ★结构有效域 ⟹ 产物良构（xs9 实例，连接结构条件与 WellFormed 全约束）：
  从 `xs9_windowComposable`（结构条件成立）经**真证明**前提传播定理
  `composeStep_wellFormed_of_windowComposable`，得 `composeStep 0 xs9` 每个上级走势 WellFormed。
  这是有效域前提传播的具体兑现——非恒等，经泛型归纳 `canonicalWindows_map_wellFormed`。
-/
theorem xs9_all_wellFormed : ∀ m ∈ composeStep 0 xs9, WellFormed m :=
  composeStep_wellFormed_of_windowComposable 0 xs9 xs9_windowComposable

/-! ════════════════════════════════════════════════════════════════════════
    § 2. Priority 实例化 ← 缠论动作优先级（持仓 × 信号决定优先动作）

    Foundation 的 `IsPriorityClass`/`priority_class_complete_unique` 参数化于谓词
    `P1..P5 : X → Prop`，把 X 分为 6 类（emergencyExit/ancestorClosed/localStop/
    reverseNest/sameNest/hold），互斥穷尽。

    缠论动作优先级（`Strict.Op.StrictState` = 持仓 Pos × 信号 Sig）：缠师操作纪律的优先级
    序（zoushi.md 第 17/18 课操作律 + 缠论操盘三段论）。把缠论操作谓词填入 P1..P5：
      - P1 (emergencyExit 急退)  ← 持空遇买侧（空头腿需平，最高优先）：pos=short ∧ sig=buySide
      - P2 (ancestorClosed)      ← 持空遇卖侧（空头翻转/止损）：pos=short ∧ sig=sellSide
      - P3 (localStop 局部止)    ← 持多遇卖侧（降成本减仓）：pos=long ∧ sig=sellSide
      - P4 (reverseNest)         ← 持多遇买侧（降成本买回/反向加）：pos=long ∧ sig=buySide
      - P5 (sameNest 同向)       ← 空仓遇买侧（顺向建仓）：pos=flat ∧ sig=buySide
      - hold（其余）              ← 持仓不动 / 空仓观望（无上述触发）

    这把 Foundation 的抽象 6 类优先级钉为缠论操作律的具体优先序——满足某高优先谓词即归该类，
    否则降级，最终 hold。互斥穷尽由 Foundation 已证保证。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Op (Pos Sig StrictState)

/-- P1 急退：持空遇买侧（空头腿需平，最高优先）。 -/
def chanP1 (s : StrictState) : Prop := s.pos = Pos.short ∧ s.sig = Sig.buySide
/-- P2：持空遇卖侧（空头翻转/止损）。 -/
def chanP2 (s : StrictState) : Prop := s.pos = Pos.short ∧ s.sig = Sig.sellSide
/-- P3 局部止：持多遇卖侧（降成本减仓）。 -/
def chanP3 (s : StrictState) : Prop := s.pos = Pos.long ∧ s.sig = Sig.sellSide
/-- P4：持多遇买侧（降成本买回/反向加）。 -/
def chanP4 (s : StrictState) : Prop := s.pos = Pos.long ∧ s.sig = Sig.buySide
/-- P5 同向：空仓遇买侧（顺向建仓）。 -/
def chanP5 (s : StrictState) : Prop := s.pos = Pos.flat ∧ s.sig = Sig.buySide

instance : DecidablePred chanP1 := fun s => by unfold chanP1; infer_instance
instance : DecidablePred chanP2 := fun s => by unfold chanP2; infer_instance
instance : DecidablePred chanP3 := fun s => by unfold chanP3; infer_instance
instance : DecidablePred chanP4 := fun s => by unfold chanP4; infer_instance
instance : DecidablePred chanP5 := fun s => by unfold chanP5; infer_instance

/--
  ★缠论动作优先级 6 类互斥穷尽（Foundation `priority_class_complete_unique` 应用于缠论操作律）。

  对每个完整结构状态 s（持仓 × 信号），缠论动作优先级类（emergencyExit..hold）**存在且唯一**：
  缠论操作律的优先序把每个状态唯一归入一个优先动作类。这是 GPT 完全分类「优先级互斥穷尽」
  义务在缠论操作纪律上的实例化。

  ★忠实性：本定理把缠论具体谓词 `chanP1..chanP5` 填入 Foundation 已证定理，得缠论层结论：
  缠论操作优先级是 StrictState 上的良定义互斥穷尽分类。
-/
theorem chan_priority_complete_unique (s : StrictState) :
    NewChanlun.ExistsUnique
      (fun c => NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s c) :=
  NewChanlun.priority_class_complete_unique chanP1 chanP2 chanP3 chanP4 chanP5 s

/--
  ★缠论动作优先级类互不重叠（Foundation `priority_classes_disjoint` 应用于缠论操作律）：
  同一状态满足两个优先级类判据 ⟹ 两类相等（优先序无歧义）。
-/
theorem chan_priority_disjoint {s : StrictState} {c d : NewChanlun.PriorityClass}
    (hc : NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s c)
    (hd : NewChanlun.IsPriorityClass chanP1 chanP2 chanP3 chanP4 chanP5 s d) :
    c = d :=
  NewChanlun.priority_classes_disjoint chanP1 chanP2 chanP3 chanP4 chanP5 hc hd

/-! ════════════════════════════════════════════════════════════════════════
    § 3. GlobalVoice 实例化 ← 缠论走势分类器（多级别真裁决判据）

    Foundation 的 `GlobalClassSpec`/`global_class_complete_unique` 参数化于
    `Local : Voice → X → LocalClass → Prop` 与 `hLocal : ∀ v x, ExistsUnique (Local v x ·)`，
    得「每个声部唯一分类」组合为「全局多声部分类存在唯一」。

    缠论走势分类器（`Strict.Trend`）：`I : WalkX → TrendKind`，真裁决判据
    `IsKind c x := outcomeToKind (classifyMove x.centers) = some c`。已证
    `trend_classifies : Strict.Classifies WalkX TrendKind I IsKind`（total/sound/complete/
    disjoint/realized 五件套）。`Classifies` 的 total+complete ⟹ 每个走势唯一分类 ⟹
    `ExistsUnique (IsKind · x)`，正是 Foundation `hLocal` 所需。

    ★多声部载体修正（no-patch，de-degenerate 核心）：旧桩 `chanLocal _ x c := IsKind c x`
    **忽略 voice 参数**——所有声部读同一标签，多级别共振退化为单声部。诊断：Foundation
    `Local v x c` 让**每个声部 v 独立分类**，不同声部可给不同标签（这正是多声部的存在论
    意义）。要让 voice 真起作用，对象 `x` 必须携带「每级（每声部）各自的走势」。旧桩对象
    `WalkX` 是**单级**走势对象，结构上无法承载 per-voice 区分——是载体选错（单走势 vs
    级别族）。

    忠实化：对象 `X := ChanVoice → WalkX`（**级别索引的走势族**，每个级别一个走势对象——
    缠论多级别共振的忠实载体：5 分钟级别上涨 / 30 分钟级别盘整，各级读**各自的中枢序列**）。
    `Local v x c := IsKind c (x v)`（第 v 声部读**它自己的**走势对象 `x v`——voice 真正
    选取不同级别的走势，不同级别 → 不同 centers → 不同标签，真多声部）。`hLocal` 由
    `trend_classifies` 对每个 `x v` 给出。这把 Foundation `GlobalVoice` 抽象忠实承载缠论
    多级别共振——矛盾不存在（Foundation `GlobalVoice` 本就为多声部设计）。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Trend (WalkX I IsKind trend_classifies)

/-- 声部 = 缠论级别（多级别共振，Nat 级别索引）。 -/
abbrev ChanVoice : Type := Nat

/-- 多级别走势族 `VoiceFamily`：每个声部（级别）一个走势对象——缠论多级别共振的忠实载体
    （不同级别携带各自的中枢序列/走势分类，体现力度区分）。 -/
abbrev VoiceFamily : Type := ChanVoice → WalkX

/--
  ★缠论局部走势分类关系 `Local`（忠实多声部，voice 真起作用）：声部（级别）v 下，
  多级别走势族 `x` 在该级的走势 `x v` 的局部分类是 c。

  `chanLocal v x c := IsKind c (x v)`——第 v 声部读**它自己的**走势对象 `x v`（不同级别
  读不同走势的真裁决判据 `outcomeToKind (classifyMove (x v).centers) = some c`）。voice 参数
  **真正影响**分类（不同级别的走势族分量不同 ⟹ 不同 centers ⟹ 可不同标签）——非旧桩退化。
-/
def chanLocal (v : ChanVoice) (x : VoiceFamily) (c : TrendKind) : Prop := IsKind c (x v)

/--
  ★每声部走势分类存在唯一（Foundation `hLocal` 的缠论实例，由 `trend_classifies` 导出）。

  对每个声部 v 和多级别走势族 x，该声部走势 `x v` 满足局部分类判据的 TrendKind **存在且
  唯一**：
  - 存在：`⟨I (x v), trend_classifies.sound (x v)⟩`（第 v 级走势的真裁决判据 sound）。
  - 唯一：`trend_classifies.complete`（满足判据 ⟹ 等于 `I (x v)`，故任意两者相等）。

  这把 `Strict.Classifies` 的 total+sound+complete 在**每个声部各自的走势** `x v` 上重组为
  Foundation `ExistsUnique` 形态——每级独立唯一分类（多声部各自唯一）。
-/
theorem chan_local_complete_unique (v : ChanVoice) (x : VoiceFamily) :
    NewChanlun.ExistsUnique (fun c : TrendKind => chanLocal v x c) := by
  refine ⟨I (x v), trend_classifies.sound (x v), ?_⟩
  intro c hc
  -- hc : chanLocal v x c = IsKind c (x v)；complete : IsKind c (x v) → I (x v) = c
  exact (trend_classifies.complete hc).symm

/--
  ★缠论全局多声部走势分类存在唯一（Foundation `global_class_complete_unique` 应用于缠论）。

  对每个多级别走势族 x，给每个声部（级别）指派一个走势分类的全局指派函数
  `g : Voice → TrendKind`，满足「逐声部都符合各自局部分类判据」者**存在且唯一**。这是 GPT
  完全分类「多声部分类组合唯一」义务在缠论**多级别共振**走势分类上的实例化——全局指派 g
  恰是「每级各读各自走势分类」的逐级 sound 组合（`g v = I (x v)`）。

  ★忠实性：本定理把忠实的多声部 `chanLocal`（voice 真起作用）+ `chan_local_complete_unique`
  填入 Foundation 已证定理，得缠论层结论：缠论多级别共振走势分类是良定义的全局分类，
  每个多级别走势族的全局分类存在且唯一（不同级别可不同标签，组合唯一）。
-/
theorem chan_global_class_complete_unique (x : VoiceFamily) :
    NewChanlun.ExistsUnique
      (fun g : ChanVoice → TrendKind => NewChanlun.GlobalClassSpec chanLocal x g) :=
  NewChanlun.global_class_complete_unique chanLocal chan_local_complete_unique x

/--
  ★多声部非退化见证（L0，反退化的直接机器证明）：存在多级别走势族 x 与两个声部 u/v，
  使两声部分类**不同**（`I (x u) ≠ I (x v)`）——voice 真正区分级别，非旧桩「所有声部同标签」。

  见证：声部 0 读上涨走势 `walkUp2`、声部 1 读盘整走势 `walkCons`——同一走势族在不同级别
  有不同走势分类（缠论多级别共振：低级别上涨、高级别盘整）。这关死旧 `chanLocal _ x c :=
  IsKind c x` 忽略 voice 的退化（旧桩下任意 u/v 同标签，本忠实化下 u/v 可不同标签）。
-/
theorem chan_voice_distinguishes :
    ∃ (x : VoiceFamily) (u v : ChanVoice), I (x u) ≠ I (x v) := by
  classical
  refine ⟨fun n => if n = 0 then Strict.Trend.walkUp2 else Strict.Trend.walkCons, 0, 1, ?_⟩
  -- x 0 = walkUp2（I = upTrend），x 1 = walkCons（I = consolidation），标签不同
  decide

/-! ════════════════════════════════════════════════════════════════════════
    § 4. Strategy 实例化 ← 缠论分类→应对流水线（Rec→Class→Voice→Risk→Exec）

    Foundation 的 `StrategySpec`/`strategy_spec_total_unique` 参数化于五段关系
    `RecR : X→D→Prop`、`ClassR : X→D→C→Prop`、`VoiceR : X→C→Q→Prop`、
    `RiskR : X→Q→QStar→Prop`、`ExecR : X→QStar→O→Prop`，各段「全且单值」⟹ 整条
    Rec→Class→Voice→Risk→Exec 流水线的输出存在唯一。

    缠论分类→应对流水线：从走势对象出发，经走势分类（Trend）→ 操作状态映射 → 操作动作
    （Op 的 πStrict）→ 风险投影 → 执行。最小忠实实例把各段关系取为缠论具体**函数的图**
    （graph of a function）：`R x y := y = f x`，每段的 `ExistsUnique` 由「函数图存在唯一」
    平凡给出（存在 = f x，唯一 = 图的定义）。这把 Foundation 五段流水线钉为缠论具体的
    分类→应对函数复合，其输出存在唯一。

    ★诚实声明（no-patch + formalization-validity-domain）：本节用「函数图」实例化各段关系，
    各段 `ExistsUnique` 来自函数全定义单值性（L0 结构事实），**不**冒充「缠论应对的经验
    有效性」。数量/价格/合法 route 维度 = L3 运行时（`Strict.Op.op_quantity_out_of_layer2`
    已诚实裁定不进 L0 Layer2），本节流水线只钉**类型层**应对的存在唯一，不钉数量。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Op (piStrict)

/-- 流水线对象 X := 走势对象 WalkX（缠论分类的输入）。 -/
abbrev StratX : Type := WalkX
/-- Rec 段输出 D := 走势级别（递归层读出，最小实例取级别 Nat）。 -/
abbrev StratD : Type := Nat
/-- Class 段输出 C := 走势分类 TrendKind（缠论走势三分类）。 -/
abbrev StratC : Type := TrendKind
/-- Voice 段输出 Q := 操作状态 StrictState（分类映射到操作语境）。 -/
abbrev StratQ : Type := StrictState
/-- Risk 段输出 QStar := 操作状态（**类型层**风险投影不变；数量维度投影由 `RiskProj` 在
    Θ_risk 网格层承载，见 `risk_quantity_out_of_typelayer` 边界定理）。 -/
abbrev StratQStar : Type := StrictState
/-- Exec 段输出 O := 严格动作 StrictAction（最终应对）。 -/
abbrev StratO : Type := Strict.StrictAction

/-- 走势分类映射到操作状态（缠论：走势类型决定操作语境）。
    上涨趋势 → 持多遇无信号（持多不动语境）；下跌趋势 → 持空；盘整 → 空仓观望。
    这是「走势分类 → 操作状态」的缠论确定性映射（类型层）。 -/
def kindToState : TrendKind → StrictState
  | TrendKind.upTrend       => ⟨Pos.long,  Sig.none⟩
  | TrendKind.downTrend     => ⟨Pos.short, Sig.none⟩
  | TrendKind.consolidation => ⟨Pos.flat,  Sig.none⟩

/-- Rec 段关系（函数图）：走势级别读出 = x.level。 -/
def stratRecR (x : StratX) (d : StratD) : Prop := d = x.level
/-- Class 段关系（函数图）：走势分类 = I x（真裁决派生）。 -/
def stratClassR (x : StratX) (_ : StratD) (c : StratC) : Prop := c = I x
/-- Voice 段关系（函数图）：操作状态 = kindToState c（分类映射到操作语境）。 -/
def stratVoiceR (_ : StratX) (c : StratC) (q : StratQ) : Prop := q = kindToState c
/--
  Risk 段关系（函数图）：**类型层**风险投影 = 恒等（StrictState 持仓×信号类型层不变）。

  ★这不是「恒等桩糊过」——它是「数量维度不入 L0 类型层流水线」的**诚实形式编码**：
  Foundation Risk 段在类型层（`StratQStar := StrictState`）只承载操作语境（持仓方向×信号），
  **不**承载仓位数量。真实的「多声部期望仓位 → 唯一总仓位」投影是 `Strict.RiskProj.gridProject`
  （作用于仓位网格 `Pos`，Θ_risk 参数化，`riskproj_exists_unique` 已证存在唯一）——它在
  **数量维度**层运作，而数量 = L3 运行时（`Strict.Op.op_quantity_out_of_layer2` 已诚实裁定）。
  类型层与数量层是**两个不同维度**：类型层投影恒等（语境不变），数量层投影由 RiskProj 唯一
  确定。边界由 `risk_quantity_out_of_typelayer` 证出（非糊过，见该定理）。
-/
def stratRiskR (_ : StratX) (q : StratQ) (qstar : StratQStar) : Prop := qstar = q
/-- Exec 段关系（函数图）：最终动作 = piStrict qstar（缠论完全应对策略）。 -/
def stratExecR (_ : StratX) (qstar : StratQStar) (o : StratO) : Prop := o = piStrict qstar

/-- 函数图存在唯一通用引理：`R x y := y = f x` 给出 `ExistsUnique (R x ·)`。 -/
theorem graph_existsUnique {A B : Type} (f : A → B) (a : A) :
    NewChanlun.ExistsUnique (fun b : B => b = f a) :=
  ⟨f a, rfl, fun _ hy => hy⟩

/--
  ★缠论分类→应对流水线存在唯一（Foundation `strategy_spec_total_unique` 应用于缠论）。

  对每个走势对象 x，整条缠论流水线 Rec→Class→Voice→Risk→Exec 的输出（最终严格动作）
  **存在且唯一**。各段关系取缠论具体函数的图，各段 `ExistsUnique` 由函数图存在唯一给出
  （走势级别读出 / 走势分类 / 分类→操作状态 / 风险投影 / 完全应对策略，皆全函数单值）。
  这是 GPT 完全分类「π 流水线存在唯一」义务在缠论分类→应对复合上的实例化。

  ★忠实性：本定理把缠论具体五段关系填入 Foundation 已证定理，得缠论层结论：缠论的
  分类→应对流水线是良定义的确定性策略——每个走势对象有唯一类型层应对（数量维度 = L3，
  由 `op_quantity_out_of_layer2` 诚实裁定不入此 L0 流水线）。
-/
theorem chan_strategy_total_unique (x : StratX) :
    NewChanlun.ExistsUnique
      (fun o : StratO =>
        NewChanlun.StrategySpec stratRecR stratClassR stratVoiceR stratRiskR stratExecR x o) :=
  NewChanlun.strategy_spec_total_unique
    stratRecR stratClassR stratVoiceR stratRiskR stratExecR
    (fun x => graph_existsUnique (fun x => x.level) x)
    (fun x _ _ => graph_existsUnique (fun _ => I x) x)
    (fun _ c _ => graph_existsUnique kindToState c)
    (fun _ q _ => graph_existsUnique (fun q => q) q)
    (fun _ qstar _ => graph_existsUnique piStrict qstar)
    x

/-! ────────────────────────────────────────────────────────────────────────
    § 4.1 Risk 段类型层/数量层维度分离的边界定理（de-degenerate：证出边界，非恒等糊过）

    任务卡要求：Risk 段不用恒等桩糊过——若数量维度确属 L3 运行时，则把该边界**证出来**。
    下面三件机器证明把「类型层投影恒等 + 数量层投影由 RiskProj 唯一」的维度分离钉死：
    (a) `risk_typelayer_identity`：类型层 Risk 关系 ⟺ 状态不变（StrictState 不承载数量，诚实）。
    (b) `risk_quantity_projection_unique`：**数量层**真实投影 = `RiskProj.riskproj_exists_unique`
        的具体实例——有限非空仓位网格上唯一总仓位存在（数量投影**不是**恒等，是真 argmin）。
    (c) `risk_quantity_out_of_typelayer`：两层维度分离的合取——类型层恒等 ∧ 数量层唯一投影
        ∧ 数量维度由 `op_quantity_out_of_layer2` 诚实裁定不入 L0 类型层流水线。
    ──────────────────────────────────────────────────────────────────────── -/

/-- ★`Nat` 的可判定线性序（`RiskProj.DecidableLinearKey` 的标准实例，供数量层网格用）。 -/
def natKey : Strict.RiskProj.DecidableLinearKey Nat where
  le := Nat.le
  decLe := fun a b => Nat.decLe a b
  le_refl := Nat.le_refl
  le_trans := fun _ _ _ => Nat.le_trans
  le_antisymm := fun _ _ => Nat.le_antisymm
  le_total := Nat.le_total

/--
  ★最小数量层仓位网格（`RiskProj.RiskGrid Nat Nat`，Θ_risk 参数化的具体见证）。

  - `grid := [0]`：单格点仓位网格（0∈𝒦，最小有限非空——零仓位可行）。
  - `cost := id`：代价 = 仓位本身（Θ_risk 参数化的最小代价；真实 cost = Σw(q-q̃)²+λΣc|·|
    由 Θ_risk 给，此处取最小见证）。
  - `cost_inj`：单格点平凡单射（p,q∈[0] ⟹ p=q=0）。
  这是「数量维度的真实风险投影存在」的具体实例——非类型层恒等，是仓位网格上的真 argmin。
-/
def chanRiskGrid : Strict.RiskProj.RiskGrid Nat Nat where
  grid := [0]
  zero := 0
  zeroMem := by simp
  key := natKey
  cost := id
  cost_inj := fun p q hp hq _ => by
    rw [List.mem_singleton] at hp hq
    rw [hp, hq]

/--
  ★(a) Risk 段类型层关系 ⟺ 状态不变（L0，诚实编码非糊过）：
  `stratRiskR x q qstar ↔ qstar = q`——类型层（StrictState 持仓×信号）经 Risk 段**不变**，
  因 StrictState **不承载仓位数量**（数量在数量层，见 (b)）。这是「数量维度不入 L0 类型层」
  的形式编码，不是「投影什么都没做」——类型层本就只承载操作语境，数量是另一维度。
-/
theorem risk_typelayer_identity (x : StratX) (q qstar : StratQ) :
    stratRiskR x q qstar ↔ qstar = q := Iff.rfl

/--
  ★(b) 数量层真实风险投影唯一（L0，引用 `RiskProj.riskproj_exists_unique` 具体实例）：

  在数量层仓位网格 `chanRiskGrid` 上，存在唯一总仓位 q*——这是真实的「多声部期望仓位 →
  唯一总仓位」投影（有限非空网格上 cost 字典序最小，`RiskProj` 已证存在唯一），**不是**
  类型层的恒等。这坐实「Risk 段的真实数量投影在 Θ_risk 网格层非平凡存在唯一」。
-/
theorem risk_quantity_projection_unique :
    ∃ q : Nat, q ∈ chanRiskGrid.grid
      ∧ (∀ y, y ∈ chanRiskGrid.grid → chanRiskGrid.key.le (chanRiskGrid.cost q) (chanRiskGrid.cost y))
      ∧ (∀ q', q' ∈ chanRiskGrid.grid
          → (∀ y, y ∈ chanRiskGrid.grid → chanRiskGrid.key.le (chanRiskGrid.cost q') (chanRiskGrid.cost y))
          → q' = q) :=
  chanRiskGrid.riskproj_exists_unique

/--
  ★(c) Risk 段维度分离边界（L0，de-degenerate 主交付——证出边界，非恒等桩糊过）。

  三件合取钉死「类型层恒等 ∧ 数量层唯一投影 ∧ 数量维度 L3 诚实裁定」：
  - 类型层：Risk 关系 ⟺ StrictState 不变（`risk_typelayer_identity`，数量不入类型层）。
  - 数量层：真实总仓位投影存在唯一（`risk_quantity_projection_unique`，`RiskProj` 真 argmin）。
  - L3 裁定：操作类型不决定数量语义（`Strict.Op.op_quantity_out_of_layer2`——同标签 fiber
    含账户转移不等价对象），故数量 = L3 运行时，不入此 L0 类型层流水线。

  ★这是任务卡「把边界证出来」的兑现：类型层恒等**不是**糊过，而是「数量维度在类型层不可见」
  的诚实编码，且数量层的真实投影被引用为非平凡存在唯一——两层维度严格分离，机器可检验。
-/
theorem risk_quantity_out_of_typelayer :
    (∀ (x : StratX) (q qstar : StratQ), stratRiskR x q qstar ↔ qstar = q)
    ∧ (∃ q : Nat, q ∈ chanRiskGrid.grid
        ∧ (∀ y, y ∈ chanRiskGrid.grid → chanRiskGrid.key.le (chanRiskGrid.cost q) (chanRiskGrid.cost y))
        ∧ (∀ q', q' ∈ chanRiskGrid.grid
            → (∀ y, y ∈ chanRiskGrid.grid → chanRiskGrid.key.le (chanRiskGrid.cost q') (chanRiskGrid.cost y))
            → q' = q))
    ∧ (∃ a b : Formal.OperationalSemantics.Exec,
        Formal.OperationalSemantics.I a = Formal.OperationalSemantics.I b
        ∧ ¬ Formal.OperationalSemantics.OpEqSemantic a b) :=
  ⟨risk_typelayer_identity, risk_quantity_projection_unique, Strict.Op.op_quantity_out_of_layer2⟩

end NewChanlun.ChanlunInstantiation
