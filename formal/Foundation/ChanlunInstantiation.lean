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
  1. LevelRecursion ← 缠论 走势递归（`Strict.Recursive` 的 `Move`/`canOp` 逐级 compose）：
       `D := fun _ => Move`（分级宇宙，级别作字段，U n = Move）；
       `F0 := id`（第 0 级 = 种子走势本身，线段/起始层）；
       `Fstep n := canStep n`（把第 n 级走势规范装箱为第 n+1 级走势，缠论「逐级 compose」）。
     得 `recSpec_complete_unique`（缠论走势递归级唯一）+ `recAt_causal`（同种子同级走势相等）。
  2. Priority ← 缠论动作优先级（`Strict.Op.StrictState` 的持仓×信号决定动作）：
       X := StrictState；P1..P5 用缠论操作谓词刻画 6 类优先动作。
     得 `priority_class_complete_unique`（缠论动作优先级 6 类互斥穷尽）。
  3. GlobalVoice ← 缠论走势分类器（`Strict.Trend` 的 `IsKind` 真裁决判据）：
       Voice := Level（声部 = 级别）；LocalClass := TrendKind；Local := per-level IsKind。
     由 `Strict.Trend.trend_classifies`（total+complete）导出 `hLocal : ExistsUnique`，
     得 `global_class_complete_unique`（缠论多级别走势分类存在唯一）。
  4. Strategy ← 缠论分类→应对流水线（`Strict.Trend` 分类 + `Strict.Op` 策略）：
       五段关系用缠论分类器/策略的函数图（graph）刻画，各段 `ExistsUnique` 由「函数图存在
       唯一」给出，得 `strategy_spec_total_unique`（缠论 Rec→Class→Voice→Risk→Exec 存在唯一）。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
  全部 **L0**（纯定义/结构归纳，把已证 L0 结构填入已证 L0 抽象定理，不依赖数据）。
  实例化的信息增量 = 确认「缠论具体结构满足 Foundation 抽象义务」这一**结构事实**，
  不是任何实证有效域断言。完备性是缠论公理给定的，本文件忠实编码，不冒充 L1+ 经验验证。

  ── 依赖方向（单向无环）──────────────────────────────────────────────────────
  ChanlunInstantiation → {CompleteClassification, Strict.Recursive, Strict.Trend, Strict.Op}
  → {Formal.*}。Foundation 内部 srcDir="Foundation" 用裸名 `import CompleteClassification`；
  引 Strict 用前缀 `import Strict.X`。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
        本文件 = 把缠论具体结构与 GPT 完全分类地基对接的实例化层（build on Foundation）。
  禁 sorry/admit/axiom。验证：`cd formal && lake build`。
-/

import CompleteClassification
import Strict.Recursive
import Strict.Trend
import Strict.Op

namespace NewChanlun.ChanlunInstantiation

open Formal.RecursiveConstruction (Move classifyMove)
open Formal.TrendTrichotomy (Direction TrendKind)

/-! ════════════════════════════════════════════════════════════════════════
    § 1. LevelRecursion 实例化 ← 缠论走势递归（逐级 compose）

    Foundation 的 `recAt`/`RecSpec`/`recSpec_complete_unique`/`recAt_causal` 参数化于
    `D : Nat → Type`、`F0 : H → D 0`、`Fstep : (n) → D n → D (n+1)`。

    缠论走势递归（`Strict.Recursive`，zhongshu.md/zoushi.md）：分级宇宙 `U n := Move`
    （级别作 `Move.level` 字段，非 type index——kernel 硬约束，见 Recursive.lean 文件头）。
    递归算子 `canOp n : {xs // len ≥ 3} → U(n+1)` 把 ≥3 段第 n 级走势装箱为第 n+1 级。

    ★接缝（no-patch 诚实声明）：Foundation 的 `Fstep` 是**单参数** `D n → D (n+1)`，而缠论
    `canOp` 取**序列** `{xs // len ≥ 3}`。这是两种递归形态的真实差异，不可隐藏。忠实实例化
    需把「单个第 n 级走势」确定地展开为「≥3 段下级序列」：`canStep n m := compose [m,m,m] …
    (n+1)`——用同一走势 m 三次构成最小合法（len=3）下级序列，规范装箱为第 n+1 级。
    这是 Foundation 单参数递归在缠论 compose 上的**确定性实例**：每个第 n 级走势经一步规范
    递归得唯一第 n+1 级走势。`F0 := id`（第 0 级 = 种子走势本身 = 缠论起始/线段层）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- 分级宇宙：第 n 级缠论走势类型（`U n := Move`，级别作字段，对应 Foundation `D`）。 -/
abbrev ChanD (_ : Nat) : Type := Move

/-- 派生中枢（复用 `Strict.Recursive.canG` 的契约：边界不读 centers，centers 派生忠实性
    由 WellFormed 层契约，不在此层重复——见 Recursive.lean 分工边界）。 -/
def chanCenters (_ : Move) : List Formal.CenterTrichotomy.Center := []

/--
  ★缠论递归步 `Fstep`（单参数实例）：第 n 级走势 m 规范装箱为第 n+1 级走势。
  缠论「逐级 compose」：把走势 m 作为下级构件，以最小合法 ≥3 段序列 `[m,m,m]` 规范化为
  第 n+1 级 `Move.compose`，级别契约 `(n+1)` 写入 `Move.level` 字段。

  这是 Foundation `Fstep : (n) → ChanD n → ChanD (n+1)` 的缠论确定性实例——确定性体现为
  Lean 全函数（每个 m 给出唯一第 n+1 级走势），契合 Foundation 递归级唯一所需。
-/
def canStep (n : Nat) (m : ChanD n) : ChanD (n + 1) :=
  Move.compose [m, m, m] (chanCenters m) (n + 1)

/-- 种子取值：历史 = 走势数据本身，恒等取值（对应 `Strict.Recursive` 的 `d := id`）。 -/
def chanSeed : Move → ChanD 0 := id

/--
  ★缠论走势递归层读出（`recAt` 的缠论实例）：从种子走势 h 经 n 步规范递归到第 n 级走势。
  这把 Foundation 抽象 `recAt ChanD chanSeed canStep n h` 钉为缠论具体的「逐级 compose 塔」。
-/
def chanRecAt (n : Nat) (h : Move) : ChanD n :=
  NewChanlun.recAt ChanD chanSeed canStep n h

/--
  ★缠论走势递归级唯一（Foundation `recSpec_complete_unique` 应用于缠论递归）。

  对每个种子走势 h 和级别 n，满足缠论递归规格 `RecSpec`（第 0 级 = 种子，第 n+1 级 = 规范
  装箱前级）的第 n 级走势**存在且唯一**。这是 GPT 完全分类「递归级唯一」义务在缠论走势
  递归上的实例化——缠论的逐级 compose 塔在每级唯一确定。

  ★忠实性：本定理**不**重证 Foundation 的归纳，只把缠论具体的 `ChanD`/`chanSeed`/`canStep`
  填入 Foundation 已证定理。结论是缠论层的：缠论走势递归在每级唯一。
-/
theorem chan_rec_level_unique (h : Move) (n : Nat) :
    NewChanlun.ExistsUnique (fun d : ChanD n => NewChanlun.RecSpec ChanD chanSeed canStep h n d) :=
  NewChanlun.recSpec_complete_unique ChanD chanSeed canStep h n

/--
  ★缠论走势递归因果（Foundation `recAt_causal` 应用于缠论递归，ObsEq := Eq）。

  观察等价取 `Eq`（两种子走势相等）。`chanSeed = id` 平凡满足 `hF0`（相等种子给相等第 0 级）。
  结论：相等的种子走势经任意级数规范递归得相等的第 n 级走势——缠论递归塔是种子的确定性函数
  （因果：当下递归层只依赖种子，无前视）。
-/
theorem chan_rec_causal {h h' : Move} (hh : h = h') (n : Nat) :
    chanRecAt n h = chanRecAt n h' :=
  NewChanlun.recAt_causal ChanD chanSeed canStep (fun a b => a = b)
    (fun he => congrArg chanSeed he) hh n

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

    多声部 Voice := 级别（缠论多级别共振——同一走势数据在不同级别有各自的走势分类，体现
    力度区分）。这里 Voice 取 `Unit`（单声部最小实例）或 Level——我们用 Level（Nat）作声部，
    Local v x c := 「x 在级别 v 的走势分类是 c」。最小忠实实例用走势对象本身携带级别，
    Local 退化为 IsKind（每声部读同一真裁决判据）——这是单声部的诚实最小实例。
    ════════════════════════════════════════════════════════════════════════ -/

open Strict.Trend (WalkX I IsKind trend_classifies)

/-- 声部 = 缠论级别（多级别共振；最小实例下各声部共享真裁决判据）。 -/
abbrev ChanVoice : Type := Nat

/--
  ★缠论局部走势分类关系 `Local`：声部（级别）v 下，走势 x 的局部分类是 c。
  最小忠实实例：每声部读同一真裁决判据 `IsKind`（走势分类由 centers 经 classifyMove 唯一
  决定，独立于声部标号——单声部最小实例，多级别力度区分留待 LevelState/StrategyFamily 层）。
-/
def chanLocal (_ : ChanVoice) (x : WalkX) (c : TrendKind) : Prop := IsKind c x

/--
  ★每声部走势分类存在唯一（Foundation `hLocal` 的缠论实例，由 `trend_classifies` 导出）。

  对每个声部 v 和走势 x，满足局部分类判据的 TrendKind **存在且唯一**：
  - 存在：`⟨I x, trend_classifies.sound x⟩`（真裁决判据 sound）。
  - 唯一：`trend_classifies.complete`（满足判据 ⟹ 等于 I x，故任意两者相等）。

  这把 `Strict.Classifies` 的 total+sound+complete 重组为 Foundation `ExistsUnique` 形态。
-/
theorem chan_local_complete_unique (v : ChanVoice) (x : WalkX) :
    NewChanlun.ExistsUnique (fun c : TrendKind => chanLocal v x c) := by
  refine ⟨I x, trend_classifies.sound x, ?_⟩
  intro c hc
  -- hc : chanLocal v x c = IsKind c x；complete : IsKind c x → I x = c
  exact (trend_classifies.complete hc).symm

/--
  ★缠论全局多声部走势分类存在唯一（Foundation `global_class_complete_unique` 应用于缠论）。

  对每个走势 x，给每个声部（级别）指派一个走势分类的全局指派函数 `g : Voice → TrendKind`，
  满足「逐声部都符合局部分类判据」者**存在且唯一**。这是 GPT 完全分类「多声部分类组合
  唯一」义务在缠论多级别走势分类上的实例化。

  ★忠实性：本定理把 `chanLocal` + `chan_local_complete_unique` 填入 Foundation 已证定理，
  得缠论层结论：缠论多级别走势分类是良定义的全局分类，每个走势的全局分类存在且唯一。
-/
theorem chan_global_class_complete_unique (x : WalkX) :
    NewChanlun.ExistsUnique
      (fun g : ChanVoice → TrendKind => NewChanlun.GlobalClassSpec chanLocal x g) :=
  NewChanlun.global_class_complete_unique chanLocal chan_local_complete_unique x

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
/-- Risk 段输出 QStar := 操作状态（风险投影最小实例取恒等，类型层不变）。 -/
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
/-- Risk 段关系（函数图）：风险投影 = 恒等（类型层最小实例；数量维度 = L3 不入此处）。 -/
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

end NewChanlun.ChanlunInstantiation
