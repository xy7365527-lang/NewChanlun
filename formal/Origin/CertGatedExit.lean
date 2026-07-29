/-
Origin/CertGatedExit.lean — 证书索引（身份绑定）+ cert-gated exit 门控性质的机器见证
（票 #300 [P0b]，价值序 P0b；实装 session 指定 Opus 5）

── 存在论位置 ───────────────────────────────────────────────────────────────
信源（两份，教义层权威）：
  1. `chanlun/escalate/g1-g2-window-ban-ruling-20260721.md`（G1/G2 终裁，编排者拍板）
     §1 G2：证书-BSP 的 bar 距离**窗口绑定全域禁用**（240/1440 是「拿时间距离近似结构
         身份的经验主义近似」）；替代路径 = **身份绑定**（`NestEventIdentity{level,
         turn_source, interval_b}` 直挂结构确认的 BSP 点，**无窗口概念**）；DRAFT
         条款 2（judge_max 绑定）/条款 3（240/1440 分档）作废。
     §2 T1 supersede：证书**当门**（typed 真链证书作准入门），不当信号源、不当窗口贴标。
     §3 G1：约束转化为「**窗口绑定产物不得进任何路径**」；门消费侧不受 grep 约束
         （门是教义原生路径，其产物是准入判定不是贴标）。
  2. `chanlun/review-results/issue76-acceptance-20260721.md`（#76 出场门真链切换·验收，判定通过）
     §2 红线①门关短路 / ②禁第二查法 / ③**miss ⟹ false + v0 只进对照**。

rust 侧被形式化的机器语义（当前 HEAD `2bf3c71b58` 实读，非转述）：
  · `rust/src/theta_v0/classifier/nest_index.rs`：`NestCertificateIndex`（身份键 → typed 证书）
    ——:288 `for exec in 1..`（L0 设计性跳过）、:306 `Entry::Vacant`（first-wins 去重）、
    :299-305（rungs 三桶会计）。
  · `rust/src/theta_v0/classifier/nest.rs:395-399`：`NestEventIdentity{level, turn_source,
    interval_b}`——三字段**全是结构坐标，无任何 bar 距离/窗口分量**。
  · `rust/src/theta_v0/backtest/admission.rs:869-995` `NestChainGate::chain_lookup`
    ——:920-923 因果守卫（`judge_at > anchor` ⟹ 整证剔除）、:930-940 级状态、
    :979-985 三态裁决（零闭合 ⟹ NoChain；全闭合 ⟹ Pass；否则 Reject）。
  · `rust/src/theta_v0/backtest/admission.rs:1327-1359` `ExitNestGateCtx::reverse_admit`
    ——出场门：Flat ⟹ 拒（:1329-1338）；`pass = verdict == Pass`（:1345）；v0 基例只进
    `stats.observe` 对照落账（:1351-1354）。
  · `rust/src/theta_v0/backtest/admission.rs:1048-1086` `NestChainGate::admit_inner`
    ——进场门：Pass ⟹ 准；Reject ⟹ 拒；**NoChain ⟹ Xzd 回退**（可准）。

── 认识论等级与有效域（formalization-validity-domain 231号，强制；090 照实）──────
  全部 **L0**（纯定义 / 结构归纳 / 布尔代数，不依赖任何市场数据）。`lake build` 通过 =
  「上述门谓词与索引结构在定义层具有本文件所述性质」，**不是**任何经验有效性断言。

  ★有效域三条硬边界（不得外推，实装 session 2026-07-28 实读核定，已记 #300 留言）：
  (1) 本文件见证的是 **`reverse_admit` 这个门谓词** 满足 typed-only，**不是**「生产出场
      路径已 typed-only」。`ExitNestGateCtx` 的唯一宿主是 `fill.rs:2456`/`:2833`
      （`plan_and_fill_mtm`/`_dual`），其唯一生产调用方 `runner.rs:245`/`:330` 分属
      `run_theta_v0`（`runner.rs:218`，`#[deprecated]` 在 :215）与 `run_theta_v0_dual`
      （:320，`#[deprecated]` 在 :317）。现役因果回放 `run_theta_v0_pi` **无独立出场门**。
  (2) 「门只放行 typed 身份绑定链」**对进场门为假**——`admit_inner` 的 NoChain 分支走
      Xzd 回退（`admission.rs:1048-1086`）。本文件 §5 把这条**证成定理**（`entry_gate_not_typed_only`
      构造性反例）而非隐去：有效域 ≠ 定义域（222/223/230/231号）。
  (3) `chain_lookup` 的锚解析（`resolve_foot`）与恰好存在扫描（`cross_level_query`）
      **范围外**（T1 供给线语义，另票）——本文件的 `levels` 是「链区间逐级证书集」这一
      **已给定**的输入，`LevelStatus` 因此不含 `MissingExistence` 构造子（诚实开口，
      非遗漏）。

── 纯 core 硬约束 ───────────────────────────────────────────────────────────
  无 Mathlib/Batteries/Std（与 Origin 全库同范式）；禁 sorry/admit/axiom/native_decide。
  §1-§5 的每个 list 引理均本文件自证，不依赖 core 之外的 List 引理库。

── 谱系 ─────────────────────────────────────────────────────────────────────
  `Origin.IntervalNestCertificate`（N^δ 全定义 Bool 算子 + Sel_Θ 构造，L2-B 工位）给出
  **单点证书**的结构形式；本文件给出其**索引化 + 门化**的机器语义（身份绑定索引 +
  链三态门控），两者无重叠：前者答「一张证书怎么算出来」，后者答「一批证书怎么按身份
  存取、门怎么用它放行」。无新概念分离——身份绑定 / 三态裁决 / 因果守卫均是 G1/G2 终裁
  与 #75/#76/#208 已结算结构的形式化，本文件不引入任何新定义口径。
-/

namespace NewChanlun.Origin.CertGatedExit

/-! ═══════════════════════════════════════════════════════════════════════
    § 0. 自证 List 引理（纯 core，不依赖 core 之外的 List 引理库）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- `filter` 不增长度。 -/
theorem filter_len_le {α : Type} (p : α → Bool) :
    ∀ l : List α, (l.filter p).length ≤ l.length := by
  intro l
  induction l with
  | nil => simp
  | cons a t ih =>
    cases hp : p a <;> simp [List.filter, hp] <;> omega

/-- `filter` 保长 ⟹ 全体满足谓词。 -/
theorem filter_len_eq_all {α : Type} (p : α → Bool) :
    ∀ l : List α, (l.filter p).length = l.length → ∀ a ∈ l, p a = true := by
  intro l
  induction l with
  | nil => intro _ a ha; cases ha
  | cons a t _ih =>
    intro hlen b hb
    cases hp : p a
    · -- p a = false ⟹ filter 丢一项，长度必严格小，与保长矛盾
      have hle := filter_len_le p t
      simp [List.filter, hp] at hlen
      omega
    · simp [List.filter, hp] at hlen
      cases hb with
      | head => exact hp
      | tail _ hbt => exact hlen b hbt

/-- 全体满足谓词 ⟹ `filter` 保长。 -/
theorem all_filter_len {α : Type} (p : α → Bool) :
    ∀ l : List α, (∀ a ∈ l, p a = true) → (l.filter p).length = l.length := by
  intro l
  induction l with
  | nil => intro _; simp
  | cons a t ih =>
    intro hall
    have hpa : p a = true := hall a (List.Mem.head _)
    have hrest : ∀ b ∈ t, p b = true := fun b hb => hall b (List.Mem.tail _ hb)
    simp [List.filter, hpa, ih hrest]

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 证书索引：身份键 + first-wins 去重 + L0 跳过 + 三桶会计

    锚：`nest_index.rs`（模块头「去重 first-wins」/:288/:299-305/:306）、
        `nest.rs:395-399`（`NestEventIdentity` 三字段）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **事件身份键**（`nest.rs:395-399` `NestEventIdentity` 逐字段对应）：
  `level`（事件级）、`turnSource`（拐点源坐标）、`intervalB`（B 区间两端）。

  ★G2 §1 机器见证的**结构前提**：三字段**全是结构坐标**——没有「证书确认 bar 与 BSP
  bar 的距离」这一维。窗口绑定（240/1440）之所以在类型层就装不进来，是因为绑定裁决
  的输入里根本没有 bar 距离的位置（§4 `ident_binding_bar_invariant` 把这一点证成定理）。
-/
structure EventIdentity where
  level : Nat
  turnSource : Nat
  intervalB : Nat × Nat
deriving DecidableEq, Repr

/--
  **typed 证书记录**（`nest.rs:412-425` `TypedNestCertificate` 的判定相关投影）：
  · `judgeAt` = 链上每级首次可证钟（高→低含基例）——因果守卫与链深 rungs 的唯一来源；
  · `nDelta` = `cert.certificate().n_delta()`——**判定谓词唯一来源**（nest.rs 递归核，
    禁第二查法，`admission.rs:926` 逐字）。

  诚实开口：`kinds`/`identities`/`confirmed` 三个 sidecar 字段不进判定（`nest.rs:421-425`
  明载「不进证书真值」），本文件不建模——非遗漏。
-/
structure CertRecord where
  judgeAt : List Nat
  nDelta : Bool
deriving Repr

/-- 索引 = 身份键 → 证书的关联表（`NestCertificateIndex.by_id`）。 -/
abbrev Index := List (EventIdentity × CertRecord)

/-- 身份键查证书（`NestCertificateIndex::get`，`nest_index.rs:214`）。 -/
def lookup : Index → EventIdentity → Option CertRecord
  | [], _ => none
  | (k, v) :: rest, id => if k = id then some v else lookup rest id

/--
  **first-wins 插入**（`nest_index.rs:306` `Entry::Vacant` 逐字）：键已在 ⟹ 原样返回
  （后到者**不覆盖**）；键不在 ⟹ 追加。
-/
def insertFirstWins (idx : Index) (e : EventIdentity × CertRecord) : Index :=
  if (lookup idx e.1).isSome then idx else idx ++ [e]

/--
  **索引构建**（`build_nest_certificate_index`，`nest_index.rs:281-311`）：
  按扫描序逐个基例装配 —— `level = 0` 的基例**设计性跳过**（`nest_index.rs:288`
  `for exec in 1..`，「nest 不听 L0，p105 §3」），其余 first-wins 入表。
-/
def buildStep (idx : Index) (e : EventIdentity × CertRecord) : Index :=
  if e.1.level = 0 then idx else insertFirstWins idx e

def buildIndex (evs : List (EventIdentity × CertRecord)) : Index :=
  evs.foldl buildStep []

/-! ── § 1.1 A1：first-wins（后到者不覆盖） ─────────────────────────────── -/

/-- 查表在拼接下的分配律（自证辅助）。 -/
theorem lookup_append_some (idx tail : Index) (id : EventIdentity) (v : CertRecord)
    (h : lookup idx id = some v) : lookup (idx ++ tail) id = some v := by
  induction idx with
  | nil => simp [lookup] at h
  | cons p rest ih =>
    obtain ⟨k, w⟩ := p
    by_cases hk : k = id
    · simp [lookup, hk] at h ⊢; exact h
    · simp [lookup, hk] at h ⊢; exact ih h

/--
  **A1（单步）**：一旦某身份键在索引中有值，任何后续插入都不改变它——
  first-wins 的机器见证（锚：`nest_index.rs:306`）。
-/
theorem insert_first_wins (idx : Index) (e : EventIdentity × CertRecord)
    (id : EventIdentity) (v : CertRecord) (h : lookup idx id = some v) :
    lookup (insertFirstWins idx e) id = some v := by
  unfold insertFirstWins
  split
  · exact h
  · exact lookup_append_some idx [e] id v h

/--
  **A1（全构建）`index_first_wins`**：整个构建过程中，已绑定的键值**恒不被改写**。
  锚：`nest_index.rs` 模块头「去重 first-wins（与 gate `seen` 语义同款）」+ :306。
-/
theorem index_first_wins (evs : List (EventIdentity × CertRecord)) :
    ∀ (idx : Index) (id : EventIdentity) (v : CertRecord),
      lookup idx id = some v → lookup (evs.foldl buildStep idx) id = some v := by
  induction evs with
  | nil => intro idx id v h; simpa using h
  | cons e rest ih =>
    intro idx id v h
    refine ih (buildStep idx e) id v ?_
    unfold buildStep
    split
    · exact h
    · exact insert_first_wins idx e id v h

/-! ── § 1.2 A2：L0 不入索引 ────────────────────────────────────────────── -/

/-- 索引中每个键的级 ≥ 1。 -/
def NoLevelZero (idx : Index) : Prop := ∀ p ∈ idx, p.1.level ≠ 0

theorem lookup_mem (idx : Index) (id : EventIdentity) (v : CertRecord)
    (h : lookup idx id = some v) : (id, v) ∈ idx := by
  induction idx with
  | nil => simp [lookup] at h
  | cons p rest ih =>
    obtain ⟨k, w⟩ := p
    by_cases hk : k = id
    · subst hk
      simp [lookup] at h
      subst h
      exact List.Mem.head _
    · simp [lookup, hk] at h
      exact List.Mem.tail _ (ih h)

theorem noLevelZero_append (idx : Index) (e : EventIdentity × CertRecord)
    (h : NoLevelZero idx) (he : e.1.level ≠ 0) : NoLevelZero (idx ++ [e]) := by
  intro p hp
  rcases List.mem_append.mp hp with hl | hr
  · exact h p hl
  · simp at hr; subst hr; exact he

theorem buildStep_noLevelZero (idx : Index) (e : EventIdentity × CertRecord)
    (h : NoLevelZero idx) : NoLevelZero (buildStep idx e) := by
  unfold buildStep insertFirstWins
  split
  · exact h
  · rename_i hlvl
    split
    · exact h
    · exact noLevelZero_append idx e h hlvl

/--
  **A2 `index_no_level_zero`**：`buildIndex` 产出的索引中，任何可查出的证书其身份级
  **必 ≥ 1**——L0 基例设计性不入索引。
  锚：`nest_index.rs:288`（`for exec in 1..`）+ 模块头「exec 循环从 1 起扫——
  `events_by_level[0]` 被跳过（nest 不听 L0，p105 §3）」。
-/
theorem index_no_level_zero (evs : List (EventIdentity × CertRecord))
    (id : EventIdentity) (v : CertRecord)
    (h : lookup (buildIndex evs) id = some v) : id.level ≠ 0 := by
  have key : ∀ (l : List (EventIdentity × CertRecord)) (idx : Index),
      NoLevelZero idx → NoLevelZero (l.foldl buildStep idx) := by
    intro l
    induction l with
    | nil => intro idx hi; simpa using hi
    | cons e rest ih => intro idx hi; exact ih (buildStep idx e) (buildStep_noLevelZero idx e hi)
  have hbase : NoLevelZero ([] : Index) := by intro p hp; cases hp
  have hfin : NoLevelZero (buildIndex evs) := key evs [] hbase
  exact hfin (id, v) (lookup_mem _ _ _ h)

/-! ── § 1.3 A3：rungs 三桶会计恒等式 ───────────────────────────────────── -/

/-- 链深（`nest_index.rs:301` `cert.judge_at().len().saturating_sub(1)`）。 -/
def rungsOf (c : CertRecord) : Nat := c.judgeAt.length - 1

/-- 三桶归属（`nest_index.rs:302-305`：0 / 1 / 2+）。 -/
def rungsBucket (c : CertRecord) : Nat :=
  match rungsOf c with
  | 0 => 0
  | 1 => 1
  | _ => 2

theorem rungsBucket_cases (c : CertRecord) :
    rungsBucket c = 0 ∨ rungsBucket c = 1 ∨ rungsBucket c = 2 := by
  unfold rungsBucket
  match h : rungsOf c with
  | 0 => exact Or.inl rfl
  | 1 => exact Or.inr (Or.inl rfl)
  | _ + 2 => exact Or.inr (Or.inr rfl)

def countBucket (b : Nat) : Index → Nat
  | [] => 0
  | p :: rest => (if rungsBucket p.2 = b then 1 else 0) + countBucket b rest

/--
  **A3 `index_stats_accounting`**：三桶计数之和 = 索引基数——
  `NestCertificateIndexStats` 的 `rungs_0 + rungs_1 + rungs_2_plus = indexed` 恒等式。
  锚：`nest_index.rs:299-307`（三桶自增与 `indexed` 自增在同一 `Vacant` 分支内）。
-/
theorem index_stats_accounting (idx : Index) :
    countBucket 0 idx + countBucket 1 idx + countBucket 2 idx = idx.length := by
  induction idx with
  | nil => simp [countBucket]
  | cons p rest ih =>
    rcases rungsBucket_cases p.2 with h | h | h <;>
      simp [countBucket, h] <;> omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 链三态：因果守卫 → 级状态 → 三态裁决

    锚：`admission.rs:869-995`（`chain_lookup`）——:920-923 因果守卫、
        :930-940 级状态、:979-985 三态裁决。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **因果守卫**（`admission.rs:920-923` 逐字）：链上**任一**确认钟越过锚定 bar ⟹
  整证剔除。这是 #76 相对旧固定 ℓ+1 桥的**新增严格项**（旧桥无守卫，确认钟越界照读）。
-/
def causalClean (c : CertRecord) (anchor : Nat) : Bool :=
  c.judgeAt.all (fun t => decide (t ≤ anchor))

/--
  **级闭合**（`admission.rs:912-940`）：该级存在**因果干净**且 `n_delta = true` 的证书。
  与 rust 的 `pass |= cert.certificate().n_delta()`（只对 clean 证书累或）逐语义等价。
-/
def levelClosed (certs : List CertRecord) (anchor : Nat) : Bool :=
  certs.any (fun c => causalClean c anchor && c.nDelta)

/--
  **级状态**（`admission.rs:930-940`）。诚实开口：rust 的第五态 `MissingExistence`
  （:900-910）由**恰好存在扫描**给出，属本票范围外的 T1 供给线语义（见文件头有效域
  (3)），故本枚举只有四态——非遗漏。
-/
inductive LevelStatus where
  | missingCert
  | missingCausal
  | broken
  | closed
deriving DecidableEq, Repr

def levelStatus (certs : List CertRecord) (anchor : Nat) : LevelStatus :=
  if certs.isEmpty then .missingCert
  else if (certs.filter (fun c => causalClean c anchor)).isEmpty then .missingCausal
  else if levelClosed certs anchor then .closed
  else .broken

theorem any_of_filter_nil :
    ∀ (certs : List CertRecord) (anchor : Nat),
      certs.filter (fun c => causalClean c anchor) = [] →
      levelClosed certs anchor = false := by
  intro certs anchor
  induction certs with
  | nil => intro _; rfl
  | cons c rest ih =>
    intro h
    cases hc : causalClean c anchor with
    | true =>
      rw [List.filter_cons_of_pos (p := fun x => causalClean x anchor) (a := c) (l := rest) hc] at h
      exact absurd h (List.cons_ne_nil _ _)
    | false =>
      rw [List.filter_cons_of_neg (p := fun x => causalClean x anchor) (a := c) (l := rest)
        (by intro hh; rw [hc] at hh; exact Bool.noConfusion hh)] at h
      have hr := ih h
      unfold levelClosed at hr ⊢
      simp only [List.any_cons]
      rw [hc, hr]
      rfl

theorem any_of_filter_empty :
    ∀ (certs : List CertRecord) (anchor : Nat),
      (certs.filter (fun c => causalClean c anchor)).isEmpty = true →
      levelClosed certs anchor = false := by
  intro certs anchor h
  refine any_of_filter_nil certs anchor ?_
  cases hf : certs.filter (fun c => causalClean c anchor) with
  | nil => rfl
  | cons a t => rw [hf] at h; exact absurd h (by simp)

/--
  **级状态与级闭合同真**：`levelStatus = closed ⟺ levelClosed = true`——
  状态枚举（诊断面）与判定谓词（裁决面）无漂移。
-/
theorem levelStatus_closed_iff (certs : List CertRecord) (anchor : Nat) :
    levelStatus certs anchor = .closed ↔ levelClosed certs anchor = true := by
  unfold levelStatus
  constructor
  · intro h
    split at h
    · exact absurd h (by simp)
    · split at h
      · exact absurd h (by simp)
      · split at h
        · rename_i hcl; exact hcl
        · exact absurd h (by simp)
  · intro h
    have hne : certs.isEmpty = false := by
      cases certs with
      | nil => simp [levelClosed] at h
      | cons _ _ => rfl
    have hfe : (certs.filter (fun c => causalClean c anchor)).isEmpty = false := by
      cases hf : (certs.filter (fun c => causalClean c anchor)).isEmpty with
      | true => rw [any_of_filter_empty certs anchor hf] at h; exact absurd h (by simp)
      | false => rfl
    simp [hne, hfe, h]

/-- 闭合级计数。 -/
def nClosed (levels : List (List CertRecord)) (anchor : Nat) : Nat :=
  (levels.filter (fun certs => levelClosed certs anchor)).length

/--
  **链三态裁决**（`admission.rs:979-985` 逐字）：零闭合级 ⟹ `noChain`；
  全闭合（闭合数 = 链长）⟹ `pass`；否则 ⟹ `reject`。
-/
inductive ChainVerdict where
  | pass
  | reject
  | noChain
deriving DecidableEq, Repr

def chainVerdict (levels : List (List CertRecord)) (anchor : Nat) : ChainVerdict :=
  if nClosed levels anchor = 0 then .noChain
  else if nClosed levels anchor = levels.length then .pass
  else .reject

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. cert-gated exit：门控性质（票 #300 主命题）

    锚：`admission.rs:1327-1359`（`ExitNestGateCtx::reverse_admit`）+
        #76 验收 §2 红线③（miss ⟹ false + v0 只进对照）。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 候选方向（`VoiceSide`）。 -/
inductive Dir where
  | long
  | short
  | flat
deriving DecidableEq, Repr

/--
  **出场门**（`admission.rs:1327-1359`）：
  · Flat ⟹ 拒（:1329-1338，无方向 ⟹ 无证书）；
  · 否则 `pass = (verdict == Pass)`（:1345）——**Reject / NoChain 一律不准出**
    （:1311-1312「诚实口径，禁 v0 fallback——Xzd 回退只服务进场」）。
-/
def reverseAdmit (dir : Dir) (levels : List (List CertRecord)) (anchor : Nat) : Bool :=
  match dir with
  | .flat => false
  | _ => decide (chainVerdict levels anchor = .pass)

/--
  **B1 `exit_gate_typed_only`（票面主命题：门只放行 typed 身份绑定链）**：
  出场门准出 ⟹ 链裁决为 `Pass`（即 typed 证书链在身份键下逐级闭合）**且**方向非 Flat。
  锚：`admission.rs:1345`；#76 验收 §2③。
-/
theorem exit_gate_typed_only (dir : Dir) (levels : List (List CertRecord)) (anchor : Nat)
    (h : reverseAdmit dir levels anchor = true) :
    chainVerdict levels anchor = .pass ∧ dir ≠ .flat := by
  cases dir with
  | flat => simp [reverseAdmit] at h
  | long => exact ⟨by simpa [reverseAdmit] using h, by simp⟩
  | short => exact ⟨by simpa [reverseAdmit] using h, by simp⟩

/--
  **B2a `exit_gate_nochain_reject`（禁 v0 fallback）**：链 `NoChain`（锚不可解 / 存在性
  全无 / 因果守卫全剔）⟹ **不准出**，不退回 v0 基例、不退回 Xzd。
  锚：`admission.rs:1311-1312` 文档 + :1345；#76 验收 §2③「miss 与 n_delta=false 均 ⟹
  false，无 fallback」。
-/
theorem exit_gate_nochain_reject (dir : Dir) (levels : List (List CertRecord)) (anchor : Nat)
    (h : chainVerdict levels anchor = .noChain) : reverseAdmit dir levels anchor = false := by
  cases dir <;> simp [reverseAdmit, h]

/--
  **B2b `exit_gate_reject_reject`（缺环/断链即拒）**：链 `Reject` ⟹ 不准出。
  锚：同 B2a（`admission.rs:1345`）。
-/
theorem exit_gate_reject_reject (dir : Dir) (levels : List (List CertRecord)) (anchor : Nat)
    (h : chainVerdict levels anchor = .reject) : reverseAdmit dir levels anchor = false := by
  cases dir <;> simp [reverseAdmit, h]

/--
  **B3 `exit_gate_flat_reject`**：Flat 候选 ⟹ 拒（无方向 ⟹ 无证书）。
  锚：`admission.rs:1329-1338`。
-/
theorem exit_gate_flat_reject (levels : List (List CertRecord)) (anchor : Nat) :
    reverseAdmit .flat levels anchor = false := rfl

/--
  **对照读出的门**（`admission.rs:1351-1354`）：v0 基例读出 `v0` 随判定一并落账
  （`stats.observe`），构成 (判定, 对照) 二元组——**判定分量不消费 v0**。
-/
def reverseAdmitObserved (v0 : Bool) (dir : Dir) (levels : List (List CertRecord))
    (anchor : Nat) : Bool × Bool :=
  (reverseAdmit dir levels anchor, v0)

/--
  **B4 `exit_gate_v0_irrelevant`（v0 只进对照读出）**：v0 基例取任何值，门的**判定分量
  恒不变**——「pass 的计算不依赖 v0」的机器见证；同时 v0 仍被携带进对照分量（不是删掉
  v0 后的平凡陈述）。
  锚：`admission.rs:1351-1354`；#76 验收 §2③。
-/
theorem exit_gate_v0_irrelevant (v0 v0' : Bool) (dir : Dir)
    (levels : List (List CertRecord)) (anchor : Nat) :
    (reverseAdmitObserved v0 dir levels anchor).1
      = (reverseAdmitObserved v0' dir levels anchor).1 := rfl

/-- 对照分量确实携带 v0（防止 B4 退化为「v0 根本不在签名里」的空洞陈述）。 -/
theorem observed_carries_v0 (v0 : Bool) (dir : Dir)
    (levels : List (List CertRecord)) (anchor : Nat) :
    (reverseAdmitObserved v0 dir levels anchor).2 = v0 := rfl

/--
  **B5 `exit_gate_pass_all_closed`（逐级 N^δ 闭合）**：链裁决 `Pass` ⟹ 链上**每一级**
  都闭合——`Pass` 不是「某级过证」而是「[L0, 链顶] 全线闭合」。
  锚：`admission.rs:981-982`（`n_closed == levels.len()`）。
-/
theorem exit_gate_pass_all_closed (levels : List (List CertRecord)) (anchor : Nat)
    (h : chainVerdict levels anchor = .pass) :
    ∀ certs ∈ levels, levelClosed certs anchor = true := by
  unfold chainVerdict at h
  split at h
  · exact absurd h (by simp)
  · split at h
    · rename_i hfull
      exact filter_len_eq_all _ levels hfull
    · exact absurd h (by simp)

/-! ── § 3.1 B6：因果守卫单调（无前视） ─────────────────────────────────── -/

/-- 确认钟表的整体上界随锚定 bar 单调放宽（自证辅助）。 -/
theorem all_le_mono :
    ∀ (js : List Nat) (a a' : Nat), a ≤ a' →
      js.all (fun t => decide (t ≤ a)) = true → js.all (fun t => decide (t ≤ a')) = true := by
  intro js
  induction js with
  | nil => intro _ _ _ _; simp
  | cons t rest ih =>
    intro a a' hle h
    simp only [List.all_cons, Bool.and_eq_true, decide_eq_true_eq] at h ⊢
    exact ⟨Nat.le_trans h.1 hle, ih a a' hle h.2⟩

theorem causalClean_mono (c : CertRecord) (a a' : Nat) (hle : a ≤ a')
    (h : causalClean c a = true) : causalClean c a' = true :=
  all_le_mono c.judgeAt a a' hle h

theorem levelClosed_mono (certs : List CertRecord) (a a' : Nat) (hle : a ≤ a')
    (h : levelClosed certs a = true) : levelClosed certs a' = true := by
  unfold levelClosed at h ⊢
  induction certs with
  | nil => simp at h
  | cons c rest ih =>
    simp only [List.any_cons, Bool.or_eq_true, Bool.and_eq_true] at h ⊢
    rcases h with hc | hr
    · exact Or.inl ⟨causalClean_mono c a a' hle hc.1, hc.2⟩
    · exact Or.inr (ih hr)

/--
  **B6 `chain_causal_monotone`（无前视）**：锚定 bar 后移只会**放宽不会收紧**——
  在 bar `a` 判 `Pass` 的链，在任何更晚的 bar `a'` 仍判 `Pass`。

  内容：因果守卫（`judge_at > anchor` ⟹ 整证剔除）是**单调**的剔除器，故门不会因为
  「等得更久」而反悔。这正是 #76「出场准入判定不新增前视分量」的机器见证——门在早
  bar 的准出不依赖任何尚未确认的证书。
  锚：`admission.rs:920-923`；`ExitNestGateCtx` 文档「因果守卫是新增严格项」（:1320-1321）。
-/
theorem chain_causal_monotone (levels : List (List CertRecord)) (a a' : Nat) (hle : a ≤ a')
    (h : chainVerdict levels a = .pass) : chainVerdict levels a' = .pass := by
  have hall : ∀ certs ∈ levels, levelClosed certs a = true := exit_gate_pass_all_closed levels a h
  have hall' : ∀ certs ∈ levels, levelClosed certs a' = true :=
    fun certs hc => levelClosed_mono certs a a' hle (hall certs hc)
  have hfull' : nClosed levels a' = levels.length := all_filter_len _ levels hall'
  -- Pass ⟹ 闭合数非零 ⟹ 链非空 ⟹ a' 处闭合数 = 链长 ≠ 0
  have hne : nClosed levels a ≠ 0 := by
    unfold chainVerdict at h
    split at h
    · exact absurd h (by simp)
    · assumption
  have hlen : levels.length ≠ 0 := by
    have := filter_len_le (fun certs => levelClosed certs a) levels
    unfold nClosed at hne
    omega
  unfold chainVerdict
  rw [hfull']
  simp [hlen]

/-- 出场门的因果单调推论：早 bar 准出 ⟹ 晚 bar 仍准出。 -/
theorem exit_gate_causal_monotone (dir : Dir) (levels : List (List CertRecord))
    (a a' : Nat) (hle : a ≤ a') (h : reverseAdmit dir levels a = true) :
    reverseAdmit dir levels a' = true := by
  obtain ⟨hp, hnf⟩ := exit_gate_typed_only dir levels a h
  have hp' := chain_causal_monotone levels a a' hle hp
  cases dir with
  | flat => exact absurd rfl hnf
  | long => simp [reverseAdmit, hp']
  | short => simp [reverseAdmit, hp']

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 身份绑定 vs 窗口绑定（G2 §1 / G1 §3 机器见证）

    裁定 §1：「证书-BSP 的 bar 距离窗口绑定（240/1440）全域禁用……替代路径 = 身份
    绑定：证书经 `NestEventIdentity` 直挂其结构确认的 BSP 点，**无窗口概念**」。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 两种绑定法：身份绑定（教义）vs 窗口绑定（已全域禁用的经验主义近似）。 -/
inductive Binding where
  | ident
  | window (radius : Nat)
deriving DecidableEq, Repr

/--
  **绑定裁决**：给定「该 BSP 点的结构身份在链上确认成立」这一结构事实 `structOk`，
  以及「证书确认 bar 与 BSP bar 的距离」`dist` ——
  · `ident`：**只读结构事实**，`dist` 不进裁决（裁定 §1「无窗口概念」）；
  · `window r`：**只读 bar 距离**，结构事实不进裁决（裁定 §1「拿时间距离近似结构身份」）。
-/
def bindsTo : Binding → Bool → Nat → Bool
  | .ident, structOk, _ => structOk
  | .window r, _, dist => decide (dist ≤ r)

/--
  **A4 `ident_binding_bar_invariant`**：身份绑定的裁决对 bar 距离**恒不变**——
  240/1440 一类参数在身份绑定里没有位置。
  锚：裁定 §1（替代路径 = 身份绑定，无窗口概念）。
-/
theorem ident_binding_bar_invariant (structOk : Bool) (d₁ d₂ : Nat) :
    bindsTo .ident structOk d₁ = bindsTo .ident structOk d₂ := rfl

/--
  **A5 `window_binding_not_bar_invariant`（构造性反例）**：窗口绑定的裁决**随 bar 距离
  翻转**——同一结构事实下，`dist = 0` 放行、`dist = 241` 拒（radius 取裁定点名的 240）。
  锚：裁定 §1（240 是「数据拍值，教义无此口径」）。
-/
theorem window_binding_not_bar_invariant :
    bindsTo (.window 240) true 0 ≠ bindsTo (.window 240) true 241 := by decide

/--
  **A5′ `window_binding_admits_without_identity`（构造性反例，更重的一条）**：窗口绑定
  在**结构身份根本不成立**（`structOk = false`）时照样放行——这正是裁定 §1 所指
  「拿时间距离近似结构身份的经验主义近似」的确切失效形态。
-/
theorem window_binding_admits_without_identity :
    bindsTo (.window 240) false 0 = true := by decide

/-- 参数化门：把绑定法作为参数注入出场门（生产门 = `ident` 实例）。 -/
def exitAdmitBound (b : Binding) (dir : Dir) (levels : List (List CertRecord))
    (anchor dist : Nat) : Bool :=
  match dir with
  | .flat => false
  | _ => bindsTo b (decide (chainVerdict levels anchor = .pass)) dist

/-- 生产出场门 = 参数化门在 `ident` 处的实例（任意 `dist`，因 `ident` 不读它）。 -/
theorem exitAdmitBound_ident_eq (dir : Dir) (levels : List (List CertRecord))
    (anchor dist : Nat) :
    exitAdmitBound .ident dir levels anchor dist = reverseAdmit dir levels anchor := by
  cases dir <;> rfl

/--
  **A6 `gate_bar_invariant`（G1 §3 机器见证）**：生产出场门（身份绑定实例）的裁决对
  bar 距离**恒不变** —— 窗口绑定产物**进不了**门的判定路径。
  锚：裁定 §3（约束转化为「窗口绑定产物不得进任何路径」；门消费侧是教义原生路径）。
-/
theorem gate_bar_invariant (dir : Dir) (levels : List (List CertRecord))
    (anchor d₁ d₂ : Nat) :
    exitAdmitBound .ident dir levels anchor d₁ = exitAdmitBound .ident dir levels anchor d₂ := by
  cases dir <;> rfl

/--
  **A6′ `gate_window_variant_bar_dependent`（构造性反例）**：同一门若改用窗口绑定，
  裁决即随 bar 距离翻转——两种绑定法在门这一层**不可互换**（不是口味差异）。
-/
theorem gate_window_variant_bar_dependent :
    exitAdmitBound (.window 240) .long [] 0 0
      ≠ exitAdmitBound (.window 240) .long [] 0 241 := by decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 有效域判别：进场门 ≠ 出场门（231号，诚实分层）

    ★本节是文件头有效域 (2) 的定理化：「门只放行 typed 链」**只**对出场门成立。
      进场门 `admit_inner`（`admission.rs:1048-1086`）在 `NoChain` 时走 Xzd 回退。
      不隐去这条偏差 = 有效域 ≠ 定义域（222/223/230/231号）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **进场门**（`admission.rs:1048-1086` 三分支逐字）：
  `Pass ⟹ nest_pass`（准）；`Reject ⟹ nest_n_delta_false`（拒）；
  **`NoChain ⟹ Xzd 回退**（`xzd` 参数 = `build_xzd_fallback` / 复用旧臂 Xzd 读出的结果）。
-/
def entryAdmit (levels : List (List CertRecord)) (anchor : Nat) (xzd : Bool) : Bool :=
  match chainVerdict levels anchor with
  | .pass => true
  | .reject => false
  | .noChain => xzd

/-- 反例用证书：确认钟在 bar 5，锚定 bar 0 ⟹ 因果守卫整证剔除 ⟹ 零闭合 ⟹ `NoChain`。 -/
def lateCert : CertRecord := { judgeAt := [5], nDelta := true }

theorem lateCert_noChain : chainVerdict [[lateCert]] 0 = .noChain := by decide

/--
  **C1 `entry_gate_not_typed_only`（构造性反例）**：存在输入使**进场门准入而链非 Pass**
  ——「门只放行 typed 身份绑定链」**对进场门为假**。
  锚：`admission.rs:1051-1086`（NoChain → Xzd 回退臂）。
-/
theorem entry_gate_not_typed_only :
    entryAdmit [[lateCert]] 0 true = true ∧ chainVerdict [[lateCert]] 0 ≠ .pass := by decide

/--
  **C1′ `exit_rejects_where_entry_admits`（两门分歧的构造性见证）**：同一输入下出场门拒、
  进场门准——两门**不同律**。故本文件 §3 的 typed-only 结论**不得**外推到进场路径。
-/
theorem exit_rejects_where_entry_admits :
    reverseAdmit .long [[lateCert]] 0 = false ∧ entryAdmit [[lateCert]] 0 true = true := by decide

/--
  **C2 `entry_gate_typed_only_iff_no_fallback`**：回退臂关闭（`xzd = false`）⟹ 进场门
  也满足 typed-only。即两门的差异**恰好**是 Xzd 回退臂，不是别的地方。
-/
theorem entry_gate_typed_only_iff_no_fallback (levels : List (List CertRecord)) (anchor : Nat)
    (h : entryAdmit levels anchor false = true) : chainVerdict levels anchor = .pass := by
  unfold entryAdmit at h
  cases hv : chainVerdict levels anchor with
  | pass => rfl
  | reject => rw [hv] at h; exact absurd h (by simp)
  | noChain => rw [hv] at h; exact absurd h (by simp)

/--
  **C3 `gates_agree_when_chain_decisive`**：链裁决为 `Pass` 或 `Reject`（即链有话说）时，
  两门**同值**；分歧**只**出在 `NoChain`。
  锚：`admission.rs:1048-1050`（Pass/Reject 两分支）vs :1051-1086（NoChain 分支）。
-/
theorem gates_agree_when_chain_decisive (dir : Dir) (levels : List (List CertRecord))
    (anchor : Nat) (xzd : Bool) (hdir : dir ≠ .flat)
    (h : chainVerdict levels anchor ≠ .noChain) :
    reverseAdmit dir levels anchor = entryAdmit levels anchor xzd := by
  cases dir with
  | flat => exact absurd rfl hdir
  | long =>
    cases hv : chainVerdict levels anchor with
    | pass => simp [reverseAdmit, entryAdmit, hv]
    | reject => simp [reverseAdmit, entryAdmit, hv]
    | noChain => exact absurd hv h
  | short =>
    cases hv : chainVerdict levels anchor with
    | pass => simp [reverseAdmit, entryAdmit, hv]
    | reject => simp [reverseAdmit, entryAdmit, hv]
    | noChain => exact absurd hv h

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 公理依赖机器检查（零 sorry / 零 axiom 的可复现见证）
    ═══════════════════════════════════════════════════════════════════════ -/

#print axioms index_first_wins
#print axioms index_no_level_zero
#print axioms index_stats_accounting
#print axioms ident_binding_bar_invariant
#print axioms window_binding_not_bar_invariant
#print axioms window_binding_admits_without_identity
#print axioms gate_bar_invariant
#print axioms gate_window_variant_bar_dependent
#print axioms exit_gate_typed_only
#print axioms exit_gate_nochain_reject
#print axioms exit_gate_reject_reject
#print axioms exit_gate_flat_reject
#print axioms exit_gate_v0_irrelevant
#print axioms exit_gate_pass_all_closed
#print axioms chain_causal_monotone
#print axioms exit_gate_causal_monotone
#print axioms levelStatus_closed_iff
#print axioms entry_gate_not_typed_only
#print axioms exit_rejects_where_entry_admits
#print axioms entry_gate_typed_only_iff_no_fallback
#print axioms gates_agree_when_chain_decisive

end NewChanlun.Origin.CertGatedExit
