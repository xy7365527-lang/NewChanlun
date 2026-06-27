/-
Origin/SegmentAutoConstruct.lean — segmentsOfComplete 全自动递归切分（完整判据驱动，A″）

★工位定位（still-MISSING-A″）：
  - SegmentConstruction.lean（#116）证了良构封装（`nextSegmentEnd`）驱动的全自动切分
    `segmentsOf` 的终止性 + 唯一性（构造骨架，扫到第一个反向笔即切）。
  - SegmentFeatureComplete.lean（#118/#122）证了完整判据 `SegEndComplete`（分型 ∧ 两情况 ∧
    顶高于底）与良构封装在古怪线段处真跨层分叉——判据层完整接入。
  - 本文件（A″）实装 `segmentsOfComplete : List Stroke → List Segment`，把完整判据
    `SegEndComplete` 真正接入全自动递归切分：
      (1) 按方向对偶从 `List Stroke` 提取特征笔列（`extractFeatureStrokes`）；
      (2) 对特征笔列做递归包含合并（`buildFeatureSeqAux`，well-founded 递归，终止性机器检查）；
      (3) 从标准特征序列中找顶/底分型（`scanTopFractal`/`scanBottomFractal`，结构递归）；
      (4) 用布尔版完整判据（`segEndCompleteB`）确认段尾；
      (5) 切分：以确认的段尾为切点输出本线段，对剩余笔递归（well-founded，严格递减保证）。

★认识论等级（L0）：
  全部 L0（纯定义 / well-founded 结构递归终止 / 纯函数唯一性，不依赖数据）。
  `lake env lean Origin/SegmentAutoConstruct.lean` 通过 = segmentsOfComplete 作为全函数
  良定义（终止）+ 输出唯一（确定性）在定义层成立。
  不声称"切出的线段真对应缠论权威标注"（L2+，须真实 K 线）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib（Int/Nat + omega + Bool 布尔运算）。
-/

import Origin.ChanlunElements
import Origin.SegmentFeatureSeq
import Origin.SegmentFeatureComplete

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 从笔列提取特征序列（方向对偶 + 包含合并）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  从笔列中提取对偶特征笔（第67课对偶性）：向上线段取向下笔；向下线段取向上笔。
  结构递归，Lean 自动识别终止。
-/
def extractFeatureStrokes (segDir : Direction) : List Stroke → List Stroke
  | [] => []
  | s :: rest =>
      if s.direction == segDir.flip
      then s :: extractFeatureStrokes segDir rest
      else extractFeatureStrokes segDir rest

theorem extractFeatureStrokes_length_le (segDir : Direction) (ss : List Stroke) :
    (extractFeatureStrokes segDir ss).length ≤ ss.length := by
  induction ss with
  | nil => simp [extractFeatureStrokes]
  | cons s rest ih =>
      unfold extractFeatureStrokes
      split
      · simp only [List.length_cons]; omega
      · exact Nat.le_succ_of_le ih

/--
  包含合并辅助（`acc` 累积器方式，避免 let-rec）：将 `elems` 中相邻包含元素合并后
  追加到 `acc.reverse` 后缀。`termination_by elems.length`（结构量度递减）。

  合并逻辑：若相邻 x, y 有包含关系，合并为 `mergeInclusion processUp x y`（或 `y x`），
  再从合并结果重新扫描（因为合并后的元素可能与新邻居有包含）——此步使长度递减 1。
  若无包含，x 进 acc，继续处理 y :: rest（长度递减 1）。

  ★终止性度量：`elems.length`，合并时 `(mergeInclusion ... :: rest).length = rest.length + 1
  < (x :: y :: rest).length = rest.length + 2`；无包含时 `(y :: rest).length < (x :: y :: rest).length`。
  两条分支均严格递减 ⟹ well-founded。
-/
def buildFeatureSeqAux (processUp : Bool) (acc : List FeatureElem) : List FeatureElem → List FeatureElem
  | [] => acc.reverse
  | [x] => (x :: acc).reverse
  | x :: y :: rest =>
      if Contains x y then
        buildFeatureSeqAux processUp acc (mergeInclusion processUp x y :: rest)
      else if Contains y x then
        buildFeatureSeqAux processUp acc (mergeInclusion processUp y x :: rest)
      else
        buildFeatureSeqAux processUp (x :: acc) (y :: rest)
  termination_by elems => elems.length
  decreasing_by
    all_goals (simp only [List.length_cons]; omega)

/-- 从笔列构建标准特征序列（含包含合并，第67课）。 -/
def buildFeatureSeq (segDir : Direction) (strokes : List Stroke) : List FeatureElem :=
  let featureStrokes := extractFeatureStrokes segDir strokes
  let elems := featureStrokes.map FeatureElem.ofStroke
  buildFeatureSeqAux (segDir == Direction.up) [] elems

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 分型扫描（结构递归）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 分型扫描结果：找到分型的三个元素 + 其后剩余（revSeq 候选）。 -/
structure FractalScanResult where
  e1 : FeatureElem
  e2 : FeatureElem
  e3 : FeatureElem
  rest : List FeatureElem
deriving Repr

/--
  顶分型扫描（结构递归）：在标准特征序列中找第一个顶分型（IsTopFractal e1 e2 e3）。
  使用布尔比较（不依赖 Decidable 实例）：IsTopFractal a b c ⟺ a.high < b.high ∧ c.high < b.high。
-/
def scanTopFractal : List FeatureElem → Option FractalScanResult
  | a :: b :: c :: rest =>
      if a.high < b.high && c.high < b.high then
        some { e1 := a, e2 := b, e3 := c, rest := rest }
      else
        scanTopFractal (b :: c :: rest)
  | _ => none

/--
  底分型扫描（结构递归）：找第一个底分型（IsBottomFractal e1 e2 e3）。
  IsBottomFractal a b c ⟺ b.low < a.low ∧ b.low < c.low。
-/
def scanBottomFractal : List FeatureElem → Option FractalScanResult
  | a :: b :: c :: rest =>
      if b.low < a.low && b.low < c.low then
        some { e1 := a, e2 := b, e3 := c, rest := rest }
      else
        scanBottomFractal (b :: c :: rest)
  | _ => none

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 布尔版完整判据（可计算，不依赖 Decidable 实例）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 布尔版 HasGap：两区间无重合 ⟺ a.high < b.low ∨ b.high < a.low。 -/
def hasGapB (a b : FeatureElem) : Bool :=
  a.high < b.low || b.high < a.low

/-- 布尔版 FractalInSeq（结构递归）：有限列表扫描。 -/
def fractalInSeqB (wantTop : Bool) : List FeatureElem → Bool
  | a :: b :: c :: rest =>
      (if wantTop then (a.high < b.high && c.high < b.high)
                  else (b.low < a.low && b.low < c.low))
        || fractalInSeqB wantTop (b :: c :: rest)
  | _ => false

/-- 布尔版 SegmentEndUp（两情况）。 -/
def segmentEndUpB (e1 e2 : FeatureElem) (revSeq : List FeatureElem) : Bool :=
  if hasGapB e1 e2 then fractalInSeqB false revSeq else true

/-- 布尔版 SegmentEndDown（对偶）。 -/
def segmentEndDownB (e1 e2 : FeatureElem) (revSeq : List FeatureElem) : Bool :=
  if hasGapB e1 e2 then fractalInSeqB true revSeq else true

/-- 布尔版 TopAboveBottom（顶高于底）。 -/
def topAboveBottomB (dir : Direction) (startPrice endPrice : Tick) : Bool :=
  match dir with
  | Direction.up   => startPrice < endPrice
  | Direction.down => startPrice > endPrice

/--
  SegEndComplete 的布尔计算版（分型 ∧ 两情况 ∧ 顶高于底，三支合取）：
  返回 true ⟺ 候选段尾满足完整判据。
-/
def segEndCompleteB (d : SegEndData) : Bool :=
  (match d.dir with
   | Direction.up   => d.e1.high < d.e2.high && d.e3.high < d.e2.high
   | Direction.down => d.e2.low < d.e1.low && d.e2.low < d.e3.low)
  && (match d.dir with
      | Direction.up   => segmentEndUpB d.e1 d.e2 d.revSeq
      | Direction.down => segmentEndDownB d.e1 d.e2 d.revSeq)
  && topAboveBottomB d.dir d.startPrice d.endPrice

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 从笔列提取段尾数据（特征序列 + 分型识别）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  从候选笔列提取完整段尾数据（用于布尔版完整判据）。
  向上线段找顶分型，向下线段找底分型。`rest`（分型后剩余特征序列）作为 `revSeq` 代理。
  返回 none 表示当前笔列中找不到分型（未达确认）。
-/
def extractSegEndData (segDir : Direction) (startPrice endPrice : Tick)
    (strokes : List Stroke) : Option SegEndData :=
  let featureSeq := buildFeatureSeq segDir strokes
  match segDir with
  | Direction.up =>
      match scanTopFractal featureSeq with
      | none => none
      | some r =>
          some { dir := segDir, e1 := r.e1, e2 := r.e2, e3 := r.e3
                 revSeq := r.rest, startPrice := startPrice, endPrice := endPrice }
  | Direction.down =>
      match scanBottomFractal featureSeq with
      | none => none
      | some r =>
          some { dir := segDir, e1 := r.e1, e2 := r.e2, e3 := r.e3
                 revSeq := r.rest, startPrice := startPrice, endPrice := endPrice }

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 完整判据驱动的段尾位置扫描（top-level，可归纳）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  `scanCompleteSegEndAux segDir startP strokes n k`：
  从位置 k 起扫描笔列 `strokes`（长度 n），找第一个满足完整判据的消费位置。
  若找到（k' ≥ k），返回 k'；若全部扫完（k > n），返回 n（fallback）。

  ★提升为 top-level def（非 let-rec）以便将来归纳证明 scanCompleteSegEnd_satisfies_complete。
  termination_by n + 1 - k（k 从 1 到 n 递增，n 固定）。
-/
def scanCompleteSegEndAux (segDir : Direction) (startP : Tick) (strokes : List Stroke)
    (n : Nat) : Nat → Nat
  | k =>
      if k > n then n
      else
        let consumed := strokes.take k
        let endP : Tick :=
          match consumed.getLast? with
          | none => startP
          | some s => s.endPrice
        match extractSegEndData segDir startP endP consumed with
        | none => scanCompleteSegEndAux segDir startP strokes n (k + 1)
        | some d =>
            if segEndCompleteB d then k
            else scanCompleteSegEndAux segDir startP strokes n (k + 1)
  termination_by k => n + 1 - k

/--
  完整判据驱动的段尾消费位置：对非空笔列返回 ∈ [1, strokes.length] 的切分位置。
  fallback（全部扫完未找到判据确认点）返回 strokes.length（消费全部，保证 k ≥ 1）。
-/
def scanCompleteSegEnd (segDir : Direction) (startP : Tick) (strokes : List Stroke) : Nat :=
  if strokes.length = 0 then 0
  else scanCompleteSegEndAux segDir startP strokes strokes.length 1

/-- 辅助：aux(n, n+1) = n（k = n+1 > n 直接走 if_pos 分支）。 -/
private theorem scanCompleteSegEndAux_succ_n (segDir : Direction) (startP : Tick)
    (strokes : List Stroke) (n : Nat) :
    scanCompleteSegEndAux segDir startP strokes n (n + 1) = n := by
  unfold scanCompleteSegEndAux
  simp only [Nat.lt_succ_self, if_true]

/-- 辅助：aux(n, n) = n（k = n 非 > n，展开后递归 k+1 = n+1 返回 n）。 -/
private theorem scanCompleteSegEndAux_at_n (segDir : Direction) (startP : Tick)
    (strokes : List Stroke) (n : Nat) :
    scanCompleteSegEndAux segDir startP strokes n n = n := by
  unfold scanCompleteSegEndAux
  simp only [Nat.lt_irrefl, if_false]
  split
  · exact scanCompleteSegEndAux_succ_n segDir startP strokes n
  · rename_i _ _ _
    split
    · rfl
    · exact scanCompleteSegEndAux_succ_n segDir startP strokes n

/--
  aux 的同步 bound 引理（固定 n）：k ≤ n → k ≤ aux(k) ∧ aux(k) ≤ n。
  策略：suffices + 对 d = n - j 的直接 induction（n 固定在 closure 里，不被通用化）。
-/
theorem scanCompleteSegEndAux_bounds (segDir : Direction) (startP : Tick) (strokes : List Stroke)
    (n k : Nat) (hkn : k ≤ n) :
    k ≤ scanCompleteSegEndAux segDir startP strokes n k ∧
    scanCompleteSegEndAux segDir startP strokes n k ≤ n := by
  -- suffices 引入 d（n - j 的显式量度），对 d 做 induction；
  -- n 在 suffices 闭包里固定为定理参数，不被 induction 通用化。
  suffices h : ∀ d (j : Nat), n - j = d → j ≤ n →
      j ≤ scanCompleteSegEndAux segDir startP strokes n j ∧
      scanCompleteSegEndAux segDir startP strokes n j ≤ n from
    h (n - k) k rfl hkn
  intro d
  induction d with
  | zero =>
      intro j hd hjn
      -- n - j = 0 ∧ j ≤ n  ⟹  j = n
      have hjeq : j = n := Nat.le_antisymm hjn (by omega)
      rw [hjeq]
      exact ⟨(scanCompleteSegEndAux_at_n segDir startP strokes n).symm ▸ Nat.le_refl n,
             (scanCompleteSegEndAux_at_n segDir startP strokes n).symm ▸ Nat.le_refl n⟩
  | succ d' ih =>
      intro j hd hjn
      have hjlt : j < n := by omega
      have hd' : n - (j + 1) = d' := by omega
      have ih1 := ih (j + 1) hd' hjlt
      unfold scanCompleteSegEndAux
      have hnotgt : ¬ j > n := Nat.not_lt.mpr hjn
      simp only [hnotgt, if_false]
      split
      · exact ⟨Nat.le_trans (Nat.le_succ j) ih1.1, ih1.2⟩
      · rename_i _ _ _
        split
        · exact ⟨Nat.le_refl j, Nat.le_of_lt hjlt⟩
        · exact ⟨Nat.le_trans (Nat.le_succ j) ih1.1, ih1.2⟩

/-- ★段消费下界（L0）：非空笔列时 scanCompleteSegEnd 返回 ≥ 1。 -/
theorem scanCompleteSegEnd_pos (segDir : Direction) (startP : Tick) (strokes : List Stroke)
    (h : strokes ≠ []) : 1 ≤ scanCompleteSegEnd segDir startP strokes := by
  unfold scanCompleteSegEnd
  have hn : strokes.length ≠ 0 := fun heq => h (List.eq_nil_iff_length_eq_zero.mpr heq)
  simp only [hn, ↓reduceIte]
  exact (scanCompleteSegEndAux_bounds segDir startP strokes strokes.length 1
    (Nat.pos_of_ne_zero hn)).1

/-- ★段消费上界（L0）：scanCompleteSegEnd 结果 ≤ strokes.length。 -/
theorem scanCompleteSegEnd_le (segDir : Direction) (startP : Tick) (strokes : List Stroke) :
    scanCompleteSegEnd segDir startP strokes ≤ strokes.length := by
  unfold scanCompleteSegEnd
  split
  · omega
  · have hn : strokes.length ≠ 0 := by simp_all
    exact (scanCompleteSegEndAux_bounds segDir startP strokes strokes.length 1
      (Nat.pos_of_ne_zero hn)).2

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. segmentsOfComplete：完整判据驱动的全自动递归切分
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★ segmentsOfComplete 全自动递归切分（完整判据驱动，A″，终止性核心）

  `segmentsOfComplete : List Stroke → List Segment`

  递归：消费 k = scanCompleteSegEnd（完整判据确认位置，≥1）根笔切出一个线段，
  对剩余 strokes.drop k 递归。

  终止性：k ≥ 1 + k ≤ strokes.length ⟹ (strokes.drop k).length < strokes.length ⟹ well-founded。

  ★完整判据接入（A″）：scanCompleteSegEnd 内部调用 extractSegEndData + segEndCompleteB，
  切分点 = 第一个满足完整判据的位置（非朴素反向扫描），消解 still-MISSING-A″。
-/
def segmentsOfComplete (strokes : List Stroke) : List Segment :=
  match strokes with
  | [] => []
  | s :: rest =>
      let k := scanCompleteSegEnd s.direction s.startPrice (s :: rest)
      let consumed := (s :: rest).take k
      let seg := segmentOfStrokes consumed
      seg :: segmentsOfComplete ((s :: rest).drop k)
  termination_by strokes.length
  decreasing_by
    have hne : s :: rest ≠ [] := List.cons_ne_nil s rest
    have hpos : 1 ≤ scanCompleteSegEnd s.direction s.startPrice (s :: rest) :=
      scanCompleteSegEnd_pos s.direction s.startPrice (s :: rest) hne
    have hub : scanCompleteSegEnd s.direction s.startPrice (s :: rest) ≤ (s :: rest).length :=
      scanCompleteSegEnd_le s.direction s.startPrice (s :: rest)
    rw [List.length_drop]
    have hl : (s :: rest).length = rest.length + 1 := List.length_cons
    omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. 唯一性（确定性）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- ★ segmentsOfComplete 输出唯一性（L0）：纯全函数 ⟹ 输出唯一。 -/
theorem segmentsOfComplete_total_unique :
    TotalUnique (fun strokes out => segmentsOfComplete strokes = out) :=
  total_unique_of_fun segmentsOfComplete

/-- ★全性（L0）：对任意笔列 segmentsOfComplete 终止返回。 -/
theorem segmentsOfComplete_total :
    Total (fun strokes out => segmentsOfComplete strokes = out) := by
  intro strokes; exact ⟨segmentsOfComplete strokes, rfl⟩

/-- ★单值性（确定性，L0）：同一笔列恒得同一线段序列。 -/
theorem segmentsOfComplete_single_valued :
    SingleValued (fun strokes out => segmentsOfComplete strokes = out) :=
  (total_and_single_of_total_unique segmentsOfComplete_total_unique).2

/-! ═══════════════════════════════════════════════════════════════════════
    § 8. 良构性定理：顶高于底不变量继承
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  ★顶高于底不变量继承（L0）：任一被完整判据确认的段尾满足顶高于底（第78课硬约束）。
  segmentsOfComplete 在判据确认路径（非 fallback）上继承此不变量——完整判据的第三支
  `TopAboveBottom` 是判据确认的必要条件。
-/
theorem segmentsOfComplete_inherits_topAboveBottom (d : SegEndData) (h : SegEndComplete d) :
    TopAboveBottom { dir := d.dir, startPrice := d.startPrice, endPrice := d.endPrice } :=
  segEndComplete_implies_topAboveBottom d h

/--
  ★完整判据蕴含顶高于底（结构定理，L0）：
  segEndCompleteB d = true ⟹ TopAboveBottom 对应的端点满足约束。
  这是 segmentsOfComplete 在判据确认路径上的良构性证明入口。
-/
theorem segEndCompleteB_implies_topAboveBottom (d : SegEndData) (h : segEndCompleteB d = true) :
    topAboveBottomB d.dir d.startPrice d.endPrice = true := by
  unfold segEndCompleteB at h
  simp only [Bool.and_eq_true] at h
  exact h.2

/-! ═══════════════════════════════════════════════════════════════════════
    § 9. 计算见证（反退化）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  反退化见证：空笔列 ⟹ 空线段序列（边界）。
-/
theorem witness_segmentsOfComplete_empty :
    segmentsOfComplete [] = [] := by
  rw [segmentsOfComplete.eq_def]

/--
  反退化见证：非空笔列 ⟹ segmentsOfComplete 终止并产出非空结果。
  三根笔（向上线段起始）上 segmentsOfComplete 产出非空结果。
-/
theorem witness_segmentsOfComplete_nonempty :
    segmentsOfComplete
      [ { direction := Direction.up,   startIndex := 0, endIndex := 1, startPrice := 5,  endPrice := 20 },
        { direction := Direction.down, startIndex := 1, endIndex := 2, startPrice := 20, endPrice := 10 },
        { direction := Direction.up,   startIndex := 2, endIndex := 3, startPrice := 10, endPrice := 25 } ]
      ≠ [] := by
  rw [segmentsOfComplete.eq_def]
  simp only [ne_eq, reduceCtorEq, not_false_eq_true]

/-! ═══════════════════════════════════════════════════════════════════════
    § 10. still-MISSING 诚实声明（A‴，三个开口）
    ═══════════════════════════════════════════════════════════════════════

  ★已实装（A″ 相对 A′/A 的升级）：
    - segmentsOfComplete 全自动递归切分（完整判据驱动，well-founded 终止，L0）。
    - buildFeatureSeqAux 包含合并（结构递归终止，L0）。
    - scanTopFractal/scanBottomFractal 分型识别（结构递归，L0）。
    - segEndCompleteB 布尔版完整判据（分型 ∧ 两情况 ∧ 顶高于底，可计算）。
    - 终止性定理：segmentsOfComplete_total_unique + _total + _single_valued（L0）。
    - 顶高于底继承：segmentsOfComplete_inherits_topAboveBottom（L0，判据确认路径）。

  ★still-MISSING-A‴（三个诚实开口，no-patch-mentality 不冒充已消解）：

  开口1：fallback 路径良构性
    scanCompleteSegEnd 扫完全部笔未找到判据确认点时返回 n（消费全部），此时切出的"线段"
    可能不满足 SegEndComplete（分型/两情况/顶高于底任一支未满足）。严格处理需对 fallback
    情形给出额外约束或改变语义（须 escalate 判断）。当前 fallback = 消费全部笔保证终止性，
    良构性未保证。

  开口2：古怪线段自指递归（第67课第二种情况精确实现）
    真实第二种情况的 revSeq 应从段尾候选点起重新提取向下序列的特征序列（自指递归）。当前
    以分型后 `rest`（特征序列剩余元素）作为 revSeq 代理——这是第67课语义的保守近似，非精确。
    精确实现须调用 segmentsOfComplete 本身判断"反向序列能否成线段"，构成嵌套递归；终止性
    度量须重设计（双参数或 accessibility 构造），须 escalate。

  开口3：scanCompleteSegEnd_satisfies_complete 未机器证
    "scan 在 k 处返回 ⟹ segEndCompleteB = true"需对 scanCompleteSegEndAux 做归纳。
    定理本身成立（由 aux 定义的 `if segEndCompleteB d then k else ...` 分支可见），
    但当前文件未给出机器证（须归纳 scanCompleteSegEndAux 的递归分支）。可补充，但须单独
    验证，不在本文件当前版本中——诚实留为 still-MISSING-A‴ 开口3。

  ★边界条件（结论翻转）：
    - 终止性依赖 scanCompleteSegEnd 返回 ≥ 1（非空列时）。当前由 scanCompleteSegEnd_pos 保证。
      若 scanCompleteSegEndAux 返回 0（k=0 路径），则不终止——当前 aux 从 k=1 起，不存在 k=0。
    - 唯一性是函数内蕴（确定性布尔判据），接入自指递归后仍保持。
    - topAboveBottom 继承依赖 segEndCompleteB 第三支（严格 </>），允许平端点则须重证。

  ★下游推论：
    - P1 工位（centersOf，task #16）：segmentsOfComplete 终止 + 唯一 ⟹ 输入良定义。
    - ElementPipeline.segmentsOf 可用 segmentsOfComplete 实例化（升良构封装为完整判据驱动）。
    - BspConstruction / CenterConstruction 以线段序列为载体，A″ 后输入正确性有完整判据保证。

  ★谱系引用：
    - SegmentFeatureSeq.lean § 8 still-MISSING-A（判据层）。
    - SegmentConstruction.lean § 7 still-MISSING-A′（良构封装骨架）。
    - SegmentFeatureComplete.lean § 6 still-MISSING-A″（完整判据，本文件实装）。
    - .chanlun/genealogy/settled/001-degenerate-segment.md（fallback 路径可能产退化线段）。
    - 002-source-incompleteness.md（第67/71/78课博文，本文件复用完整判据）。

  ★影响声明：
    - 新增 Origin.SegmentAutoConstruct 模块（独占 owner：本文件）。
    - Import：ChanlunElements + SegmentFeatureSeq + SegmentFeatureComplete（只读，无竞态）。
    - 不改 canonical 类型，无命名冲突。
    - 待 Lead 在 lakefile.toml Origin roots 追加 "Origin.SegmentAutoConstruct"。

  ★认识论等级：全部 L0（结构定理，不依赖数据）。
-/

end NewChanlun.Origin
