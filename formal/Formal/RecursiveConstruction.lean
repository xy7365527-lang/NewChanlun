/-
  走势递归构造 = 初代数 μF（006 走势分解定理二 + level_recursion + 603 §二/§三）
  ★codex 异质审计修正版 R2（session 019eff21，R1 发现 1/2/3/5 + R2 发现 3/4/5/全链 已吸收）

  缠论定理（已结算）：
  - 走势分解定理二（zoushi.md:28，第17课）："任何级别的任何走势类型，都至少由三段以上
    次级别走势类型构成。" → 递归构造 `Move(ℓ) := f(Move(ℓ-1) list, len ≥ 3)`。
  - 归纳基底（level_recursion.md:21）：`Move[0] = Segment`（线段，codex 硬修正1）。
  - 趋势定义（zoushi 第17课）："至少两个以上**依次同向**中枢"——★全链同向，非仅首两个。
  - 盘整定义：恰好 1 中枢。完成走势 **必含 ≥1 中枢**（原理二）——0 中枢非盘整。
  - 级别之分有极限（zhongshu.md:18）：递归有底。

  ★codex 审计修正（faithfulness）：
  - R1#1：segment 不携带 TrendKind（线段无中枢）。`trendKind?` segment ↦ none。
  - R1#2：level 字段 + WellFormed 强制 `∀ sub, sub.level = lvl-1`（级别递减，Lean 可接受替代）。
  - R1#3 + R2#3：kind 不自由携带——由中枢计算；且 `CentersDerivedFrom subs centers`（中枢由子走势生成）
    + `centers.length ≥ 1`（完成走势必含中枢）关死"伪造 centers"漏洞。
  - R2#5：级别扩张 ↦ `MoveOutcome.higherCenterCandidate`，**不**直接 ↦ consolidation。
  - R2 全链：趋势 = **所有相邻中枢同向一致**（`AllAdjacent`），非只看首两中枢。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy

namespace Formal.RecursiveConstruction

open Formal.TrendTrichotomy
open Formal.CenterTrichotomy
  (Center CenterRelation classify center_trichotomy MoveOutcome relationToOutcome)

/--
  ★走势的初代数 μF（递归数据类型，006，codex 审计修正 R1/R2）。
  - `segment`：归纳基底 `Move[0] = Segment`（无 TrendKind，R1#1）。
  - `compose subs centers level`：递归分支。走势裁决由 `centers` **计算**（不自由携带，R1#3）；
    级别递减 + ≥3 段 + 中枢派生一致性由 `WellFormed` 强制（R2#3）。
-/
inductive Move where
  | segment (dir : Direction) (lo hi : Int)
  | compose (subs : List Move) (centers : List Center) (level : Nat)

namespace Move

def level : Move → Nat
  | segment _ _ _ => 0
  | compose _ _ lvl => lvl

/-- 走势的价格区间（segment 直接带；compose 由其中枢外缘合成）。 -/
def interval : Move → (Int × Int)
  | segment _ lo hi => (lo, hi)
  | compose _ centers _ =>
      let los := centers.map (·.dd)
      let his := centers.map (·.gg)
      (los.foldr min 0, his.foldr max 0)

/-- 走势区间下沿。 -/
def lo (m : Move) : Int := m.interval.1
/-- 走势区间上沿。 -/
def hi (m : Move) : Int := m.interval.2

end Move

open Move
open Formal.CenterTrichotomy.CenterRelation
open Formal.CenterTrichotomy.MoveOutcome

/--
  ★所有相邻中枢同向一致（codex R2 全链问题）：趋势 = "依次同向"中枢链。

  `allAdjacent rel centers` = 中枢序列中每对相邻中枢的 `classify` 都等于 `rel`。
  趋势要求全链同向（全 upContinuation 或全 downContinuation），不是只看首两个——
  否则 `[up, up, expansion]` 或 `[up, down]` 会被误判为趋势。
-/
def allAdjacent (rel : CenterRelation) : List Center → Bool
  | [] => true
  | [_] => true
  | c0 :: c1 :: rest =>
      (decide (classify c0 c1 = rel)) && allAdjacent rel (c1 :: rest)

/--
  ★走势裁决（codex R1#3 + R2#3/全链：由中枢序列计算，全链一致）。

  - 0 中枢：`higherCenterCandidate`（**非盘整**——完成走势必含 ≥1 中枢，R2#3；
    0 中枢是退化/未完成，不归盘整侧，归"待定/升级"）。
  - 1 中枢：consolidation（盘整定义）。
  - ≥2 中枢全链上涨延续：trend up；全链下跌延续：trend down。
  - ≥2 中枢非全链一致（含扩张或方向混合）：higherCenterCandidate（本级终结，交父级）。
-/
def classifyMove (centers : List Center) : MoveOutcome :=
  match centers with
  | [] => higherCenterCandidate
  | [_] => consolidation
  | c0 :: c1 :: rest =>
      if allAdjacent upContinuation (c0 :: c1 :: rest) then trend Direction.up
      else if allAdjacent downContinuation (c0 :: c1 :: rest) then trend Direction.down
      else higherCenterCandidate

/-- 走势裁决读出（codex R1#1：segment 无裁决，none）。 -/
def outcome? : Move → Option MoveOutcome
  | Move.segment _ _ _ => none
  | Move.compose _ centers _ => some (classifyMove centers)

/--
  ★中枢在某起点 `i` 由子走势窗口生成（codex R3#2 + R4：核心 **和外缘** 都 sound）。

  一个中枢 `c` 在起点 `i` 由 `subs` 生成 ⟺ 连续三段 `subs[i..i+2]` 的区间重叠产生 `c`：
  - 核心：`c.zd = max(三段 lo)`、`c.zg = min(三段 hi)`（zhongshu.md / formal_axioms §3.2）；
  - ★外缘（codex R4 必修）：`c.dd = min(三段 lo)`、`c.gg = max(三段 hi)`。
    外缘 dd/gg **驱动 up/down/expansion 判定**——不约束外缘则可伪造外缘翻转分类。

  （三段窗口版；延伸中枢的 start/finish 区间版留待 Rust 引擎，Lean 给三段 sound contract。）
-/
def CenterDerivedAt (subs : List Move) (c : Center) (i : Nat) : Prop :=
  i + 2 < subs.length
    ∧ (∀ (h0 : i < subs.length) (h1 : i + 1 < subs.length) (h2 : i + 2 < subs.length),
        c.zd = max (subs[i].lo) (max (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.zg = min (subs[i].hi) (min (subs[i+1].hi) (subs[i+2].hi))
        ∧ c.dd = min (subs[i].lo) (min (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.gg = max (subs[i].hi) (max (subs[i+1].hi) (subs[i+2].hi)))

/-- 存在某起点窗口生成该中枢（核心+外缘 sound）。 -/
def CenterDerivedFromWindow (subs : List Move) (c : Center) : Prop :=
  ∃ i, CenterDerivedAt subs c i

/--
  ★中枢序列由子走势生成（codex R3#2 + R4：witness 列表 + **时间顺序**）。

  `CentersDerivedFrom subs centers` ⟺ 存在一个 witness 起点列表 `starts`：
  - 长度等于 centers；
  - ★严格递增（codex R4：中枢时间顺序——趋势"依次同向中枢"依赖此序，否则可重排 centers 翻转分类）；
  - 每个 center 在对应起点处核心+外缘 sound。
  + subs 非空。这把"中枢非凭空"升级为 witness soundness + 顺序 contract（codex R3#2 + R4）。
-/
def StrictlyIncreasing : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)

/-- 逐对谓词（无 Mathlib 的 Forall₂ 替代）：两 list 等长且逐位满足 R（多态）。 -/
def PairwiseRel {α β : Type} (R : α → β → Prop) : List α → List β → Prop
  | [], [] => True
  | a :: as, b :: bs => R a b ∧ PairwiseRel R as bs
  | _, _ => False

def CentersDerivedFrom (subs : List Move) (centers : List Center) : Prop :=
  subs ≠ [] ∧ ∃ starts : List Nat,
    starts.length = centers.length
    ∧ StrictlyIncreasing starts
    ∧ PairwiseRel (fun c i => CenterDerivedAt subs c i) centers starts

/--
  走势良构（WellFormed，走势分解定理二 + codex R1#2/#3 + R2#3）。
  `compose subs centers lvl` 良构 ⟺
    (a) `subs.length ≥ 3`（走势分解定理二）；
    (b) `lvl ≥ 1`；
    (c) `∀ m∈subs, m.level = lvl-1`（级别递减，R1#2）；
    (d) `centers.length ≥ 1`（完成走势必含中枢，R2#3）；
    (e) `CentersDerivedFrom subs centers`（中枢由子走势生成，非伪造，R2#3）；
    (f) `∀ m∈subs, WellFormed m`（递归）。
-/
def WellFormed : Move → Prop
  | Move.segment _ _ _ => True
  | Move.compose subs centers lvl =>
      subs.length ≥ 3 ∧ lvl ≥ 1
      ∧ (∀ m ∈ subs, m.level = lvl - 1)
      ∧ centers.length ≥ 1
      ∧ CentersDerivedFrom subs centers
      ∧ (∀ m ∈ subs, WellFormed m)

/--
  ★单个上级走势的窗口 witness（codex R6：有序、连续、消费约束）。

  一个 upper move `m` 由 `lowerMoves` 的 **连续窗口** `[start, start+len)` 封装：
  - `len ≥ 3`（走势分解定理二，每上级走势 ≥3 下级走势）；
  - `start + len ≤ lowerMoves.length`（窗口在界内）；
  - `m = compose (lowerMoves 的 [start, start+len) 切片) centers (lowerLevel+1)`
    （上级走势 = 该窗口下级走势 compose，级别 +1）。

  `subsSlice` 用 `List.drop start |>.take len` 提取连续窗口（无 Mathlib extract 依赖）。
-/
structure UpperMoveWitness where
  start : Nat
  len : Nat
  centers : List Center

/-- 窗口提取：lowerMoves 的连续片段 [start, start+len)。 -/
def windowSlice (lowerMoves : List Move) (w : UpperMoveWitness) : List Move :=
  (lowerMoves.drop w.start).take w.len

/-- 单个 upper move 由窗口 witness sound 封装（连续窗口 + 级别 +1 + compose）。 -/
def UpperMoveSound (lowerMoves : List Move) (lowerLevel : Nat)
    (m : Move) (w : UpperMoveWitness) : Prop :=
  w.len ≥ 3
  ∧ w.start + w.len ≤ lowerMoves.length
  ∧ m = Move.compose (windowSlice lowerMoves w) w.centers (lowerLevel + 1)

/--
  ★上级走势由下级走势封装而成（codex R6：有序窗口 witness + 消费/数量约束，关死"重复封装"漏洞）。

  级别递归的封装步（封装恒等式 Move(k)≡Level-(k+1) 笔，rec_t types.rs:81）：
  `upperMoves` 由 `lowerMoves` 的 **有序、不重叠、连续窗口** 逐个封装而成，**不是集合成员式
  复用**（codex R6 反例：3 个相同 compose 重复消费同 3 段——被 `StrictlyIncreasing` 起点 +
  不重叠 + 数量约束关死）。约束：
  - `upperMoves ≠ []`（生成非空）；
  - 每个 upper move 良构（`WellFormed`）；
  - witness 列表与 upperMoves 等长，逐位 `UpperMoveSound`（连续窗口、级别+1、compose）；
  - witness 起点 `StrictlyIncreasing`（时间顺序，无重排）；
  - ★`upperMoves.length * 3 ≤ lowerMoves.length`（消费约束：每上级走势吃 ≥3 下级，
    有限下级走势不能无限封装——保证级别递归自然收敛，codex R6 核心）。
-/
def MovesComposedFrom (lowerMoves : List Move) (lowerLevel : Nat)
    (upperMoves : List Move) : Prop :=
  upperMoves ≠ []
  ∧ (∀ m ∈ upperMoves, WellFormed m)
  ∧ upperMoves.length * 3 ≤ lowerMoves.length
  ∧ ∃ ws : List UpperMoveWitness,
      ws.length = upperMoves.length
      ∧ StrictlyIncreasing (ws.map UpperMoveWitness.start)
      ∧ PairwiseRel
          (fun m w => UpperMoveSound lowerMoves lowerLevel m w) upperMoves ws

/-! ════════════════════════════════════════════════════════════════════════
    § 窗口化多上级序列设施（de-degenerate：把「单窗口聚合」升级为「窗口族」）

    `composeStep` 旧实装把整个下级序列聚合为**一个**上级走势（输出 length ≤ 1），是隐蔽
    退化。忠实的级别递归（走势分解定理二，zoushi 第17课）应把下级序列切成**多个**有序、
    不重叠、每窗 ≥3 段的窗口，每窗封装为一个上级走势——产出**多个**上级走势（length 可 ≥2）。

    本节给出与 centers 无关的**纯结构**窗口化设施（泛型，可在 RecursiveConstruction 层证）：
    - `canonicalWindows`：把下级序列切成起点 0,3,6,… 的恰 3 段连续窗口族（余数 <3 段丢弃——
      余数不构成完整上级走势，归 OpenTail 状态层，不强行成段）。
    - 窗口族 witness 引理：起点严格递增、每窗 len=3≥3、不重叠、覆盖前 3·(窗口数) 段。

    ★中枢派生（`CenterDerivedAt` 的核心+外缘 sound）由本节 `deriveWindowCenter3` 承载——
    这与 `Strict.Recursive.canG := []`（边界算子层不派生 centers）**不同层**：边界算子层只
    契约 subs 可还原，本节是 WellFormed 层的真中枢派生（窗口三段区间公共重叠 ⟹ 单中枢）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★窗口三段公共重叠条件（中枢成立的真实领域约束，zhongshu.md：中枢核心 = 前三段区间重叠）。

  `WindowOverlap a b c` ⟺ `max(三段 lo) < min(三段 hi)`——三段价格区间有非空公共重叠。
  这是「三段构成一个中枢」的**充要结构条件**：核心区间 `[max lo, min hi]` 非退化（`zd < zg`）。
  缠论中枢由连续三段区间重叠生成，无重叠则不成中枢（zhongshu.md 第17课）。
-/
def WindowOverlap (a b c : Move) : Prop :=
  max a.lo (max b.lo c.lo) < min a.hi (min b.hi c.hi)

instance (a b c : Move) : Decidable (WindowOverlap a b c) := by
  unfold WindowOverlap; infer_instance

/--
  ★三段窗口真派生中枢（核心+外缘 sound，由公共重叠证据 `h` 派生）。

  - 核心 `[zd, zg] = [max(三lo), min(三hi)]`（前三段区间重叠核心，I5 不变量）。
  - 外缘 `[dd, gg] = [min(三lo), max(三hi)]`（全部参与段定，含全波动）。
  - 不变量 `zd < zg`（核心非退化，由 `WindowOverlap h`）、`dd ≤ zd`、`zg ≤ gg`（外缘含核心，omega）。

  这是 `CenterDerivedAt` 的核心+外缘等式（RecursiveConstruction.lean:112）的**构造性 witness**——
  派生中枢恰满足三段窗口起点 0 处的 sound 约束（见 `deriveWindowCenter3_derivedAt`）。
-/
def deriveWindowCenter3 (a b c : Move) (h : WindowOverlap a b c) : Center :=
  { dd := min a.lo (min b.lo c.lo)
    zd := max a.lo (max b.lo c.lo)
    zg := min a.hi (min b.hi c.hi)
    gg := max a.hi (max b.hi c.hi)
    core_valid := h
    outer_lo := by unfold WindowOverlap at h; omega
    outer_hi := by unfold WindowOverlap at h; omega }

/--
  ★派生中枢满足 `CenterDerivedAt`（L0，反退化忠实性核心）：`deriveWindowCenter3 a b c h` 在
  三段窗口 `[a, b, c]` 的起点 0 处核心+外缘全 sound。这证明派生中枢**不是凭空**——它由三段
  下级走势的真实 lo/hi 边界算出（核心=max lo/min hi，外缘=min lo/max hi）。
-/
theorem deriveWindowCenter3_derivedAt (a b c : Move) (h : WindowOverlap a b c) :
    CenterDerivedAt [a, b, c] (deriveWindowCenter3 a b c h) 0 := by
  unfold CenterDerivedAt deriveWindowCenter3
  refine ⟨by simp, ?_⟩
  intro _ _ _
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [Move.lo, Move.hi]

/--
  ★窗口序列派生中枢（全函数）：取序列前三段，若公共重叠则派生单中枢，否则空。

  - `a :: b :: c :: _`：取前三段，`WindowOverlap` 成立 ⟹ `[deriveWindowCenter3 …]`（单中枢盘整）；
    不成立 ⟹ `[]`（三段不构成中枢，该窗口走势退化，不 WellFormed）。
  - 不足三段：`[]`。

  全函数性保证 `composeStep` 对任意输入有定义（Foundation `Fstep` 需要）。仅当窗口三段真重叠
  （`WindowComposable` 前提）时派生非空 sound 中枢，产物才 WellFormed——有效域 ⊊ 定义域，
  由 `composeStep` 的 soundness 定理前提显式标注（formalization-validity-domain 231号）。
-/
def windowCenters : List Move → List Center
  | a :: b :: c :: _ => if h : WindowOverlap a b c then [deriveWindowCenter3 a b c h] else []
  | _ => []

/--
  ★canonical 选择唯一性（640 谱系第三列，L0 by-construction 零信息增量）：

  `windowCenters` 是 def（全函数）⟹ 函数应用确定性 ⟹ 同输入唯一输出。这是 640 谱系第三列
  （canonical 选择唯一性）的机器见证——deterministic first-window selector 在其定义域上产唯一
  结果。有效域 = selector 的定义域（首窗口可产中枢的 es；首三段无中枢时产空，仍在 selector
  定义域内）。

  **这是纯 by-construction 函数性**（L0 零信息增量 = 函数应用确定性的同义反复），既不是
  `promote` 的函数性单值（`windowCenters` 不是 promote），也不是 `CentersDerivedFrom` 对所有
  合法 centers 的结构性单值（selector 只挑一个 canonical 窗口，不约束其他合法 centers 的存在性）。
  见 `promote_function_unique`（PromQual.lean:246）与 `derivation_not_unique`（PromQual.lean:459）。
-/
theorem windowCenters_deterministic (es : List Move) :
    ∃ c : List Center, c = windowCenters es ∧ ∀ c' : List Center, c' = windowCenters es → c' = c :=
  ⟨windowCenters es, rfl, fun _ h => h⟩

/--
  ★规范窗口族辅助（fuel 减 1 结构递归，对 `decide`/`rfl` 友好归约）：

  `canonicalWindowsAux fuel base xs` 从起点 `base` 起，把 `xs` 切成连续恰 3 段窗口
  （`a::b::c::rest` ⟹ 取 `⟨base, 3⟩` 窗口 + 递归处理 `rest`，起点 `base+3`），余数 <3 段丢弃。
  `fuel` 是窗口数上界（取 `xs.length` 足够）——fuel 减 1 递归保证终止且字面量可归约
  （well-founded 递归不 `rfl` 归约，故用 fuel；codex 路径 C）。窗口 `centers` 字段留空
  （窗口结构层不承载 centers——centers 由 `composeStep` 用 `windowCenters` 真派生）。
-/
def canonicalWindowsAux : (fuel base : Nat) → (xs : List Move) → List UpperMoveWitness
  | 0, _, _ => []
  | fuel + 1, base, xs =>
      match xs with
      | _ :: _ :: _ :: rest => ⟨base, 3, []⟩ :: canonicalWindowsAux fuel (base + 3) rest
      | _ => []

/--
  ★规范窗口族：把下级走势序列切成起点 0,3,6,… 的有序、不重叠、每窗恰 3 段的连续窗口族。

  窗口数 = `⌊xs.length / 3⌋`（每窗消费 3 段，余数 <3 丢弃）。`xs.length ≥ 6` ⟹ 窗口数 ≥ 2
  （产出**多个**上级走势，反退化）；`xs.length ∈ [3,6)` ⟹ 1 个窗口；`<3` ⟹ 空（终止吸收态）。
-/
def canonicalWindows (xs : List Move) : List UpperMoveWitness :=
  canonicalWindowsAux xs.length 0 xs

/-- 窗口族展开引理（≥3 段：首窗 + 递归尾，用于泛型归纳）。 -/
theorem canonicalWindowsAux_cons (fuel base : Nat) (a b c : Move) (rest : List Move) :
    canonicalWindowsAux (fuel + 1) base (a :: b :: c :: rest)
      = ⟨base, 3, []⟩ :: canonicalWindowsAux fuel (base + 3) rest := rfl

/-- 窗口族每窗 len = 3（故 ≥3，走势分解定理二）。归纳于 fuel。 -/
theorem canonicalWindowsAux_len_three (fuel base : Nat) (xs : List Move) :
    ∀ w ∈ canonicalWindowsAux fuel base xs, w.len = 3 := by
  induction fuel generalizing base xs with
  | zero => intro w hw; simp [canonicalWindowsAux] at hw
  | succ n ih =>
      intro w hw
      match xs with
      | a :: b :: c :: rest =>
          rw [canonicalWindowsAux_cons] at hw
          rcases List.mem_cons.mp hw with h | h
          · subst h; rfl
          · exact ih (base + 3) rest w h
      | [] => simp [canonicalWindowsAux] at hw
      | [_] => simp [canonicalWindowsAux] at hw
      | [_, _] => simp [canonicalWindowsAux] at hw

/-- 规范窗口族每窗 len = 3（≥3 直接推论）。 -/
theorem canonicalWindows_len_three (xs : List Move) :
    ∀ w ∈ canonicalWindows xs, w.len = 3 :=
  canonicalWindowsAux_len_three xs.length 0 xs

/-- 窗口族起点下界（`canonicalWindowsAux fuel base` 产物所有 start ≥ base）。归纳于 fuel，
    用于泛型 soundness 的递归窗 drop 累加对接。 -/
theorem canonicalWindowsAux_start_ge (fuel base : Nat) (xs : List Move) :
    ∀ w ∈ canonicalWindowsAux fuel base xs, w.start ≥ base := by
  induction fuel generalizing base xs with
  | zero => intro w hw; simp [canonicalWindowsAux] at hw
  | succ n ih =>
      intro w hw
      match xs with
      | a :: b :: c :: rest =>
          rw [canonicalWindowsAux_cons] at hw
          rcases List.mem_cons.mp hw with h | h
          · subst h; exact Nat.le_refl base
          · have := ih (base + 3) rest w h; omega
      | [] => simp [canonicalWindowsAux] at hw
      | [_] => simp [canonicalWindowsAux] at hw
      | [_, _] => simp [canonicalWindowsAux] at hw

/-- 窗口起点累加引理（`windowSlice (a::b::c::rest) ⟨3+δ,len⟩ = windowSlice rest ⟨δ,len⟩`）：
    在 3 段前缀上偏移 3 等价于在尾部 rest 上偏移 0——泛型 soundness 递归窗对接 ih 的核心。 -/
theorem windowSlice_shift3 (a b c : Move) (rest : List Move) (δ len : Nat) :
    windowSlice (a :: b :: c :: rest) ⟨3 + δ, len, []⟩ = windowSlice rest ⟨δ, len, []⟩ := by
  unfold windowSlice
  rw [show 3 + δ = δ + 1 + 1 + 1 by omega]
  simp [List.drop_succ_cons]

/--
  ★窗口可组装结构充分条件（有效域的**结构刻画**，formalization-validity-domain 231号）。

  `StructComposable lvl fuel xs` 镜像 `canonicalWindowsAux` 的递归形态，对每个规范窗口的
  三段下级走势 `a, b, c` 要求：(1) 区间公共重叠 `WindowOverlap a b c`（成中枢）；(2) 级别
  一致 `level = lvl`（级别递减约束）；(3) 各自 `WellFormed`（递归良构）。这是 `composeStep`
  产物全 WellFormed 的**充分条件**——**不等于**结论（区别于把有效域定义成结论本身的循环命名，
  codex 审查关切）。fuel 取 `xs.length` 覆盖所有窗口。
-/
def StructComposable (lvl : Nat) : (fuel : Nat) → (xs : List Move) → Prop
  | 0, _ => True
  | fuel + 1, xs =>
      match xs with
      | a :: b :: c :: rest =>
          WindowOverlap a b c
            ∧ a.level = lvl ∧ b.level = lvl ∧ c.level = lvl
            ∧ WellFormed a ∧ WellFormed b ∧ WellFormed c
            ∧ StructComposable lvl fuel rest
      | _ => True

/--
  ★单窗口 WellFormed（泛型）：三段下级走势区间重叠 + 级别一致 lvl + 各 WellFormed ⟹ 该窗口
  封装的上级走势 `compose [a,b,c] (windowCenters [a,b,c]) (lvl+1)` 良构。中枢由 `windowCenters`
  真派生（非空单中枢，核心+外缘 sound），子走势级别 = lvl（满足级别递减），三段各 WellFormed。
-/
theorem window_wellFormed (a b c : Move) (lvl : Nat) (hov : WindowOverlap a b c)
    (hla : a.level = lvl) (hlb : b.level = lvl) (hlc : c.level = lvl)
    (ha : WellFormed a) (hb : WellFormed b) (hc : WellFormed c) :
    WellFormed (Move.compose [a, b, c] (windowCenters [a, b, c]) (lvl + 1)) := by
  have hwc : windowCenters [a, b, c] = [deriveWindowCenter3 a b c hov] := by
    show (if h : WindowOverlap a b c then [deriveWindowCenter3 a b c h] else []) = _
    rw [dif_pos hov]
  unfold WellFormed
  refine ⟨by simp, by simp, ?_, ?_, ?_, ?_⟩
  · simp only [List.forall_mem_cons, List.not_mem_nil, false_implies, implies_true, and_true]
    refine ⟨?_, ?_, ?_⟩ <;> simp only [Nat.add_sub_cancel] <;> assumption
  · rw [hwc]; exact Nat.le_refl 1
  · refine ⟨List.cons_ne_nil _ _, [0], ?_, ?_, ?_⟩
    · rw [hwc]; rfl
    · exact True.intro
    · rw [hwc]; exact ⟨deriveWindowCenter3_derivedAt a b c hov, True.intro⟩
  · simp only [List.forall_mem_cons, List.not_mem_nil, false_implies, implies_true, and_true]
    exact ⟨ha, hb, hc⟩

/--
  ★窗口族 map 全 WellFormed（泛型归纳核心，base 累加版）：`StructComposable lvl fuel ys` ⟹
  `canonicalWindowsAux fuel base ys` 经窗口封装得到的每个上级走势都 WellFormed。归纳于 fuel：
  首窗用 `window_wellFormed`，递归窗用 `windowSlice_shift3` + `canonicalWindowsAux_start_ge`
  对接归纳假设（窗口起点 ≥ base+3，drop 累加偏移 3 = 尾部 rest 偏移 0）。
-/
theorem canonicalWindows_map_wellFormed (lvl : Nat) :
    ∀ (fuel base : Nat) (ys : List Move), StructComposable lvl fuel ys →
      ∀ m ∈ (canonicalWindowsAux fuel base ys).map
        (fun w => Move.compose (windowSlice ys ⟨w.start - base, w.len, []⟩)
          (windowCenters (windowSlice ys ⟨w.start - base, w.len, []⟩)) (lvl + 1)),
        WellFormed m := by
  intro fuel
  induction fuel with
  | zero => intro base ys _ m hm; simp [canonicalWindowsAux] at hm
  | succ n ih =>
      intro base ys hsc m hm
      match ys, hsc with
      | a :: b :: c :: rest, hsc =>
          rw [canonicalWindowsAux_cons, List.map_cons] at hm
          obtain ⟨hov, hla, hlb, hlc, ha, hb, hc, hrest⟩ := hsc
          rcases List.mem_cons.mp hm with h | h
          · subst h
            simp only [Nat.sub_self]
            have hsl : windowSlice (a :: b :: c :: rest) ⟨0, 3, []⟩ = [a, b, c] := by
              unfold windowSlice; simp
            rw [hsl]
            exact window_wellFormed a b c lvl hov hla hlb hlc ha hb hc
          · rw [List.mem_map] at h
            obtain ⟨w, hw, heq⟩ := h
            have hge : w.start ≥ base + 3 := canonicalWindowsAux_start_ge n (base + 3) rest w hw
            apply ih (base + 3) rest hrest
            rw [List.mem_map]
            refine ⟨w, hw, ?_⟩
            rw [← heq]
            have : w.start - base = 3 + (w.start - (base + 3)) := by omega
            rw [this, windowSlice_shift3]
      | [], _ => simp [canonicalWindowsAux] at hm
      | [_], _ => simp [canonicalWindowsAux] at hm
      | [_, _], _ => simp [canonicalWindowsAux] at hm

/--
  ★真完全分类·全级别（裁决结果穷尽，结构归纳，L0，codex R2 修正后）：

  任意 compose 走势的裁决必属 {trend up, trend down, consolidation, higherCenterCandidate}。
  这是 `classifyMove` 的全函数判定完备。注意：**良构** compose（centers≥1）排除 0 中枢的
  higherCenterCandidate-退化分支，故良构走势裁决 ∈ {trend, consolidation, higherCenterCandidate}
  且 consolidation/trend 是终值、higherCenterCandidate 交父级——无第五种结果。
-/
theorem outcome_total (centers : List Center) :
    classifyMove centers = trend Direction.up
    ∨ classifyMove centers = trend Direction.down
    ∨ classifyMove centers = consolidation
    ∨ classifyMove centers = higherCenterCandidate := by
  unfold classifyMove
  match centers with
  | [] => simp
  | [_] => simp
  | c0 :: c1 :: rest =>
      by_cases hu : allAdjacent upContinuation (c0 :: c1 :: rest) = true
      · simp [hu]
      · by_cases hd : allAdjacent downContinuation (c0 :: c1 :: rest) = true
        · simp [hu, hd]
        · simp [hu, hd]

/--
  ★裁决由中枢决定（codex R1#3 + R2#3：不自由携带 + 中枢派生）。
  `outcome?` 完全由 `centers` 决定——不存在"自由携带与中枢不一致的裁决"。
-/
theorem outcome_determined_by_centers (subs : List Move) (centers : List Center) (lvl : Nat) :
    outcome? (Move.compose subs centers lvl) = some (classifyMove centers) := rfl

/--
  ★≥2 全链上涨延续中枢 ⟹ 上涨趋势（codex R2 全链：检查所有相邻中枢）。
-/
theorem all_up_centers_is_uptrend (c0 c1 : Center) (rest : List Center)
    (h : allAdjacent upContinuation (c0 :: c1 :: rest) = true) :
    classifyMove (c0 :: c1 :: rest) = trend Direction.up := by
  simp [classifyMove, h]

/-- ★非全链一致 ⟹ higherCenterCandidate（含方向混合/扩张，本级终结交父级）。 -/
theorem mixed_centers_is_higher_candidate (c0 c1 : Center) (rest : List Center)
    (hu : allAdjacent upContinuation (c0 :: c1 :: rest) = false)
    (hd : allAdjacent downContinuation (c0 :: c1 :: rest) = false) :
    classifyMove (c0 :: c1 :: rest) = higherCenterCandidate := by
  simp [classifyMove, hu, hd]

/-- ★单中枢 ⟹ 盘整（zoushi 第17课盘整定义）。 -/
theorem one_center_is_consolidation (c : Center) :
    classifyMove [c] = consolidation := rfl

/-- ★0 中枢 ⟹ 非盘整（codex R2#3：完成走势必含中枢，0 中枢归 higherCenterCandidate）。 -/
theorem zero_centers_not_consolidation :
    classifyMove [] = higherCenterCandidate := rfl

/--
  ★良构走势必含 ≥1 中枢（codex R2#3）：良构 compose 的裁决不可能是 0 中枢退化分支。
  即良构走势裁决 ∈ {trend up, trend down, consolidation, higherCenterCandidate（来自≥2 混合）}，
  其中 consolidation 来自恰 1 中枢——0 中枢被 `centers.length ≥ 1` 排除。
-/
theorem wellformed_has_center (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    centers.length ≥ 1 := by
  unfold WellFormed at h
  obtain ⟨_, _, _, hc, _, _⟩ := h
  exact hc

/-- ★base case = Segment 无裁决（codex 硬修正1 + R1#1）。 -/
theorem segment_no_outcome (d : Direction) (lo hi : Int) :
    outcome? (Move.segment d lo hi) = none := rfl

/-- segment 级别为 0（递归底，zhongshu.md:18）。 -/
theorem segment_is_base (d : Direction) (lo hi : Int) :
    (Move.segment d lo hi).level = 0 := rfl

/-- ★级别递减约束生效（codex R1#2）：良构 compose 每个子走势级别 = 父级 -1。 -/
theorem wellformed_subs_one_lower (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    ∀ m ∈ subs, m.level = lvl - 1 := by
  unfold WellFormed at h
  obtain ⟨_, _, hlevel, _, _, _⟩ := h
  exact hlevel

/-- ★中枢由子走势生成（codex R2#3）：良构 compose 满足 CentersDerivedFrom。 -/
theorem wellformed_centers_derived (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    CentersDerivedFrom subs centers := by
  unfold WellFormed at h
  obtain ⟨_, _, _, _, hderiv, _⟩ := h
  exact hderiv

/--
  ★伪造中枢无 witness 窗口（codex R3#2：CentersDerivedFrom 真约束，非恒真）。

  子走势不足 3 段时（subs.length < 3），任何中枢都 **无** 生成窗口——
  `CenterDerivedFromWindow` 的存在量词 `i + 2 < subs.length` 无解。这证明该关系
  **真的会拒绝** 伪造的中枢，不是恒真占位（关死 codex R3#2 漏洞的可证伪性见证）。
-/
theorem no_window_when_too_short (subs : List Move) (c : Center)
    (h : subs.length < 3) : ¬ CenterDerivedFromWindow subs c := by
  unfold CenterDerivedFromWindow CenterDerivedAt
  intro ⟨i, hi, _⟩
  omega

/--
  ★实时走势候选（标准第5部分：未完成走势是 μF 的当下状态，不是新构造子）。
  `settled : Bool` 标记结构是否已封闭——`true`=已完成（可给最终结果），
  `false`=未完成（只给当下状态，不给最终结果）。
-/
structure CandidateMove where
  subs : List Move
  centers : List Center
  level : Nat
  settled : Bool

/--
  ★走势的当下状态（标准第5部分，μF 状态层：未完成走势只给状态，不给最终结果）。

  - `completed (o : MoveOutcome)`：**已完成**走势——结构已封闭，可给最终结果 `o`
    （trend/consolidation/higherCenterCandidate）。这是标准"已完成结构"侧。
  - `pending (tail : MoveOutcome)`：**未完成**走势——只刻画当下尾部倾向 `tail`
    （从已出现中枢读出的临时状态），**不是最终结果**。`pending` 携带 tail 仅供下游
    状态机/策略读用，携带 tail **不等于**断言最终结果是 tail（见 `pending_is_not_final`）。

  这是标准第5部分的核心结构区分：`completed` ⊎ `pending` 把"可给最终结果"与
  "只给当下状态"在**类型层**分离——未完成走势在类型上**无法**取得 `completed` 地位，
  从根上关死"把未完成压成最终 outcome"的越界（codex#2 #2）。
-/
inductive MoveState where
  | completed (o : MoveOutcome)
  | pending (tail : MoveOutcome)
deriving DecidableEq, Repr

open MoveState

/--
  ★走势候选的当下状态（标准第5部分，L0）：由 `settled` 字段路由。

  - `settled = true`（已完成）⟹ `completed (classifyMove centers)`：结构已封闭，
    `classifyMove` 在此**合法**（已完成走势可给最终结果）。
  - `settled = false`（未完成）⟹ `pending (classifyMove centers)`：未完成走势
    **只给当下状态**，中枢读数 `classifyMove centers` 被包进 `pending` 作为**临时尾部
    倾向**，类型上拒绝"未完成=最终结果"。

  中枢读数本身（`classifyMove c.centers`）是客观的——修正不抹掉它，而是按 `settled`
  把它**路由**进 `completed`（最终结果）或 `pending`（当下状态）。这正是标准第5部分
  "未完成走势唯一分类为当下状态 s"的精确落实。
-/
def CandidateMove.state (c : CandidateMove) : MoveState :=
  if c.settled then
    completed (classifyMove c.centers)
  else
    pending (classifyMove c.centers)

/--
  ★走势候选唯一分类为状态（标准第5部分，L0）：当下状态存在且唯一。

  `CandidateMove.state` 是全函数——任意候选（完成或未完成）的当下状态**唯一确定**。
  注意这**不**声称未完成走势有唯一最终结果——唯一的是"当下状态"，未完成走势的
  当下状态恰是某个 `pending`（标准第5部分允许的唯一分类是状态层，不是 outcome 层）。
-/
theorem candidate_state_unique (c : CandidateMove) :
    ∃ s, c.state = s ∧ ∀ s', c.state = s' → s' = s :=
  ⟨c.state, rfl, fun _ h => h.symm⟩

/--
  ★已完成走势 ⟹ completed 状态（可给最终结果，L0）。
-/
theorem settled_gives_completed (c : CandidateMove) (h : c.settled = true) :
    c.state = completed (classifyMove c.centers) := by
  simp [CandidateMove.state, h]

/--
  ★未完成走势 ⟹ pending 状态（只给当下状态，不给最终结果，L0，标准第5部分核心修正）。

  对 `settled = false` 的走势，`state` 必返回 `pending`——**类型上不可能**是 `completed`。
  这关死越界：未完成走势在 μF 形式化里无法取得"最终结果"地位（codex#2 #2 的根上修复）。
-/
theorem pending_gives_pending (c : CandidateMove) (h : c.settled = false) :
    c.state = pending (classifyMove c.centers) := by
  simp [CandidateMove.state, h]

/--
  ★`pending` 不是最终结果（标准第5部分，L0）：`pending o ≠ completed o`。

  `pending` 与 `completed` 是 `MoveState` 的两个**不同**构造子——携带相同的中枢读数
  `o` 时仍然不相等。这形式化"当下状态 ≠ 最终结果"：未完成走势的 `pending o` 即使
  临时尾部倾向是 o，也**不**等同于已完成走势的最终结果 o。
-/
theorem pending_is_not_final (o : MoveOutcome) : pending o ≠ completed o := by
  simp

/--
  ★状态层 totality（标准第5部分，L0）：取代旧 `candidate_preserves_totality` 的越界。

  任意候选的当下状态 `c.state` 必是 `completed _` 或 `pending _` 之一——这是 `MoveState`
  二构造子穷尽（状态层 totality）。**这才是标准第5部分允许的 totality**：

  - 旧 `candidate_preserves_totality`（已删除）：`c.outcome ∈ 四个最终 outcome`——把
    未完成压成最终 outcome（越界，codex#2 #2）。
  - 本 `candidate_state_totality`：`c.state ∈ {completed, pending}`——**状态层**穷尽，
    未完成走势落入 `pending`（不给最终结果）。

  从根上替换语义（no-patch）：μF 候选不再有"压成最终 outcome"的方法，只有状态层分类。
-/
theorem candidate_state_totality (c : CandidateMove) :
    (∃ o, c.state = completed o) ∨ (∃ t, c.state = pending t) := by
  unfold CandidateMove.state
  by_cases h : c.settled
  · exact Or.inl ⟨classifyMove c.centers, by simp [h]⟩
  · exact Or.inr ⟨classifyMove c.centers, by simp [h]⟩

/--
  ★未完成走势取最终结果地位是越界（标准第5部分 + no-patch，L0）。

  对未完成 `c`（`settled = false`），其当下状态 `c.state` 是 `pending`，与任何
  `completed o`（最终结果地位）**不相等**——即把未完成走势当最终结果用在类型层被拒。
  这是 codex#2 #2 指出的越界的可证伪见证（不是注释声明，是机器证明）。
-/
theorem candidate_overreach_rejected (c : CandidateMove) (h : c.settled = false) :
    ∀ o : MoveOutcome, c.state ≠ completed o := by
  intro o
  rw [pending_gives_pending c h]
  intro heq
  exact MoveState.noConfusion heq

end Formal.RecursiveConstruction
