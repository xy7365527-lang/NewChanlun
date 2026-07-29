/-
Origin/NestLifecycleBook.lean — NestLifecycleBook 三态、五钟与留档纪律的机器见证
（票 #299，P0a）

── 权威锚 ────────────────────────────────────────────────────────────────────
以下 Rust 机器语义均按当前
`main@59c8eb1a79d96313a22b0fb8f4bcd4a98bcf4b50`（2026-07-29）实读，
非票体旧行号、非转述：
  · `rust/src/theta_v0/classifier/nest_lifecycle.rs`：
    `consumable_closed` :1054-1059；`advance` :1070-1295（桥匹配终态提前吸收
    :1094-1109，倒退拒绝 / 终态吸收 :1164-1171，Unavailable 审计 :1198-1219，
    消失成因分类 :1260-1288）；`assert_invariants` :1297-1463
    （IdentityVanished 钟臂 :1401-1407）。
  · `rust/src/theta_v0/classifier/ledger_kernel/mod.rs`：
    `first_write_clock` :152-160，append-only 唯一追加点 :253-268，
    已建仓准入流程 :380-397，`migrate` :401-419。
裁定：
  · `chanlun/escalate/r43-lifecycle-ruling-20260721.md` §2(a)(b)、§4。

── 编号对照（票体 I1–I7 ↔ 定稿①–⑤ / 本文 I1–I5）─────────────────────────
本文 I1–I5 按定稿五项重编号，不沿用票体 I1–I7；后续验收以本表为准：
  · 票体 I1（五钟偏序）→ 定稿② / 本文 I2：`clock_discipline`；
  · 票体 I2（终态互斥 + 吸收）→ 定稿① / 本文 I1：`advance_legal`、
    `confirmed_absorbing`、`invalidated_absorbing`、`confirmed_invalidated_side_empty`；
  · 票体 I3（Closed-only 消费）→ 定稿④ / 本文 I4：`toClosed_some_iff_confirmed`、
    `provisional_not_closed`、`invalidated_not_closed`；
  · 票体 I4（Invalidated 留档；票体原文含 book 键集单调）→ 定稿⑤ / 本文 I5：
    `history_append_only`、`newly_invalidated_archived`、`invalidated_has_payload`；本文只证
    单 entry 修订链 append-only，不证 book 键集单调；
  · 票体 I5（first_provable 只写一次）→ 定稿② / 本文 I2：
    `first_provable_write_once`、`new_first_provable_at_current`；
  · 票体 I6（原因码互补）→ 定稿③ / 本文 I3：`force_reason_first_complement`；
  · 票体 I7（倒退拒绝 ⟹ entry 零改动）→ 定稿①的倒退子义务：
    `retrograde_no_change`。

── 认识论等级与有效域（090 照实）────────────────────────────────────────────
本文定理的实际结论均为 **L0 machine-checked**：它们证明本文给出的纯函数状态机具有
三态合法转移、当前喂入点首写、已证分支的五钟偏序、Confirmed-only 类型门与单 entry
失效留档性质。`ClaimStatus090.unprovedModelWithoutBridge` 携带反例，不提供其索引命题
的证明。

本文不声称：
  1. Rust 与本模型逐 bit 等价；本文件只形式化 #299 定稿的状态机核心；
  2. 力度计算正确或行情经验有效；力度结果是参数化输入；
  3. Rust 生产消费路径已经改接 Lean 类型；
  4. `ClosedEntry` 能力值必来自真实账本：它是公开的 `ConfirmedSnapshot` 能力形；本文只证
     `toClosed?` 对一般 `Entry` 当且仅当 Confirmed 产出能力，且 `consumeClosed` 只接该类型；
  5. `Feed.vanished cause` 的成因分类正确：`cause` 是参数化输入；Rust 基于 book/seen 集合的
     两类 `VanishCause` 分类，以及 `Supersedes` / `CenterUpgraded` 多 entry 迁移语义均在范围外；
  6. Rust 的 `revision_count == revisions.len()`：本文 `Entry` 无独立计数字段，
     `history_append_only` 只证单 entry 历史前缀关系。

── 降级与范围外登记（只登记，不冒充已证命题）────────────────────────────────
⑥ **未证（R 域参数化前提）**：力度三值判定门。`ForceCheck` 只接收 R 域给出的
   `verifiedWeaker / verifiedOvertake / unavailable`；本文不证明 R 域如何算出该值。
   `unavailable` 不等于 `verifiedOvertake`，不得由不可验制造失效。本模型该分支
   `delta := []`，不模型化 Rust 的 `ForceUnavailable` 修订与
   `CompletionForceUnavailableAudit` 审计侧效应，因而不声称审计留痕逐位等价。
⑦ **定义域禁令（不立证）**：白名单工程桥只负责工程身份迁移，裁定明令
   「不进证书真值路径」；本文的状态真值、
   `ClosedEntry` 与消费入口均不含桥参数；`Supersedes` / `CenterUpgraded`
   的 book 级迁移、认领与来源链语义不在本模型内。
⑧ **挂起 / 未能判定**：EventKey 并轨仍待后续裁定；本文不定义
   `LifecycleKey ↔ WireV1 EventKey`
   等价或转换定理。

**IdentityVanished 钟臂未证（模型无桥匹配分支）**：Rust 真谓词是
`last_as_of ≤ invalidated_at`（`nest_lifecycle.rs:1401-1407`），不是 `True`。
本模型无 book / 桥匹配分支，且终态推进会经 `touchInvalidated` 前移 `lastAsOf`；因此
`advance (advance (openEntry 0) 5 (.vanished .hypothesisRefuted)) 9
  (.seen .verifiedWeaker true)`
真实可达 `invalidatedAt = 5 < lastAsOf = 9`。两处 IdentityVanished 臂均以
`ClaimStatus090 (lastAsOf ≤ invalidatedAt)` 写出真实谓词；反例状态只能携带
`.unprovedModelWithoutBridge`，该构造子不能当作谓词证明。Rust 当前生产路径靠
`advance:1094-1109` 的桥匹配终态提前吸收规避；模型没有该路径，结论不得外推。
另票线索：ledger kernel 准入流程先前移终态门卫钟（:380-397）与该断言存在张力，
目前依赖路径耦合未炸；本票只登记，由编排另开质询票。

── 谱系与影响声明 ───────────────────────────────────────────────────────────
Lean 侧此前无可复用的 `NestLifecycleBook` 状态机前驱；本文从已结算 Rust 三态、五钟、
原因载荷与留档契约建立纯 core 模型。`Origin.CertGatedExit` 只提供 typed-only 能力门范式，
`Origin.IntervalNestCertificate` 只给区间套证书的计算 / 选择语义；本文不导入、不改写二者，
与其无语义重叠。本文新增类型只是既有 Rust 契约的类型化承载，不作新教义概念分离。

── 纯 core 硬约束 ────────────────────────────────────────────────────────────
无 Mathlib/Batteries/Std；零证明占位 / 新公理。
-/

namespace NewChanlun.Origin.NestLifecycleBook

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 三态、失效载荷与五钟
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 账本域三态；Closed 是 Confirmed 的消费能力，不是第四态。 -/
inductive LifecycleState where
  | provisional
  | confirmed
  | invalidated
deriving DecidableEq, Repr

/-- 终态恰为 Confirmed / Invalidated。 -/
def LifecycleState.isTerminal : LifecycleState → Bool
  | .provisional => false
  | .confirmed => true
  | .invalidated => true

/--
**I1 / L0**：三态穷尽；类型中没有第四态。
-/
theorem state_trichotomy (s : LifecycleState) :
    s = .provisional ∨ s = .confirmed ∨ s = .invalidated := by
  cases s <;> simp

/-- 身份消失仍分真否证与观测接缝；这里只保留可审计名分，不判 provider 对错。 -/
inductive VanishCause where
  | hypothesisRefuted
  | observationSeam (successorCStart : Nat)
deriving DecidableEq, Repr

/--
失效载荷把原因码与其审计材料绑在同一值里。

力度证据必须是 `Option Evidence`：Rust 明载材料缺失时诚实为 `None`；
`identityVanished` 没有力度语义，携带的是消失成因。
-/
inductive InvalidationPayload (Evidence : Type) where
  | forceOvertake (evidence : Option Evidence)
  | neverConstituted (evidence : Option Evidence)
  | identityVanished (cause : VanishCause)
deriving Repr

/-- 三个外显原因码。 -/
inductive InvalidationReason where
  | forceOvertake
  | neverConstituted
  | identityVanished
deriving DecidableEq, Repr

def InvalidationPayload.reason : InvalidationPayload Evidence → InvalidationReason
  | .forceOvertake _ => .forceOvertake
  | .neverConstituted _ => .neverConstituted
  | .identityVanished _ => .identityVanished

/--
力度证据投影。身份消失恒无力度证据；这防止把“载荷有证据字段”偷换成
“每个 Invalidated 都有非空力度证据”。
-/
def InvalidationPayload.forceEvidence : InvalidationPayload Evidence → Option Evidence
  | .forceOvertake evidence => evidence
  | .neverConstituted evidence => evidence
  | .identityVanished _ => none

/-- 五钟；`lastAsOf` 是倒退门卫钟，故单列在快照中，不冒充第六个领域钟。 -/
structure Clocks where
  observedAt : Nat
  firstProvableAt : Option Nat
  structureEndAt : Option Nat
  confirmedAt : Option Nat
  invalidatedAt : Option Nat
deriving Repr

/-- append-only 留档中的核心修订。 -/
inductive Revision (Evidence : Type) where
  | observed (asOf : Nat)
  | firstProvable (asOf : Nat)
  | structureCompleted (asOf : Nat)
  | confirmed (asOf : Nat)
  | invalidated (asOf : Nat) (payload : InvalidationPayload Evidence)
deriving Repr

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 只允许合法快照的状态索引

    三种结构分别承载三态。证明字段不是外加市场前提，而是快照构造资格：
    状态机只能构造满足已证钟纪律与原因名分的快照。唯一例外是
    IdentityVanished 的 Rust 反向钟谓词：`ClaimStatus090` 只登记正证或反例状态，
    不把未证谓词冒充为构造资格。
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 未决快照：没有结构完成钟或终态钟。 -/
structure ProvisionalSnapshot (Evidence : Type) where
  observedAt : Nat
  firstProvableAt : Option Nat
  lastAsOf : Nat
  observed_le_last : observedAt ≤ lastAsOf
  first_bounds :
    ∀ t, firstProvableAt = some t → observedAt ≤ t ∧ t ≤ lastAsOf

/-- 成立快照：`observed ≤ first ≤ structure ≤ confirmed ≤ lastAsOf`。 -/
structure ConfirmedSnapshot (Evidence : Type) where
  observedAt : Nat
  firstProvableAt : Nat
  structureEndAt : Nat
  confirmedAt : Nat
  lastAsOf : Nat
  clock_order :
    observedAt ≤ firstProvableAt ∧
      firstProvableAt ≤ structureEndAt ∧
      structureEndAt ≤ confirmedAt ∧
      confirmedAt ≤ lastAsOf

/--
090 降级载体：索引参数始终写出待证的真实命题；`proved` 才携带该命题的证明，
`unprovedModelWithoutBridge` 则携带该状态下的反例，明记“模型无桥匹配分支，未证”，
不能当作命题成立使用。
-/
inductive ClaimStatus090 (claim : Prop) : Prop where
  | proved (proof : claim)
  | unprovedModelWithoutBridge (counterexample : ¬ claim)

/--
失效快照先满足全分支偏序：每个已存在的 first/structure 钟都不晚于
invalidated，且二者同在时 `first ≤ structure`。原因分支再收紧：
* ForceOvertake：曾可证，结构钟可空；
* NeverConstituted：从未可证，`structure = invalidated ≤ lastAsOf`；
* IdentityVanished：Rust 命题为 `lastAsOf ≤ invalidatedAt`；本模型无桥匹配分支，
  终态门卫钟可继续前移，故用 `ClaimStatus090` 显式登记该命题“未证”，不以 `True` 充数。
-/
structure InvalidatedSnapshot (Evidence : Type) where
  observedAt : Nat
  firstProvableAt : Option Nat
  structureEndAt : Option Nat
  invalidatedAt : Nat
  lastAsOf : Nat
  payload : InvalidationPayload Evidence
  observed_le_last : observedAt ≤ lastAsOf
  first_bounds :
    ∀ t, firstProvableAt = some t → observedAt ≤ t ∧ t ≤ lastAsOf
  structure_bounds :
    ∀ t, structureEndAt = some t → observedAt ≤ t ∧ t ≤ lastAsOf
  first_le_structure :
    ∀ first structureAt,
      firstProvableAt = some first →
      structureEndAt = some structureAt →
      first ≤ structureAt
  first_le_invalidated :
    ∀ first, firstProvableAt = some first → first ≤ invalidatedAt
  structure_le_invalidated :
    ∀ structureAt, structureEndAt = some structureAt → structureAt ≤ invalidatedAt
  observed_le_invalidated : observedAt ≤ invalidatedAt
  reason_discipline :
    match payload with
    | .forceOvertake _ =>
        ∃ t, firstProvableAt = some t ∧ t ≤ invalidatedAt ∧ invalidatedAt ≤ lastAsOf
    | .neverConstituted _ =>
        firstProvableAt = none ∧
          structureEndAt = some invalidatedAt ∧
          invalidatedAt ≤ lastAsOf
    | .identityVanished _ =>
        ClaimStatus090 (lastAsOf ≤ invalidatedAt)

/-- 合法快照只有三个构造子。 -/
inductive Snapshot (Evidence : Type) where
  | provisional (entry : ProvisionalSnapshot Evidence)
  | confirmed (entry : ConfirmedSnapshot Evidence)
  | invalidated (entry : InvalidatedSnapshot Evidence)

/-- 账本条目 = 合法快照 + append-only 修订史。 -/
structure Entry (Evidence : Type) where
  snapshot : Snapshot Evidence
  history : List (Revision Evidence)

def Snapshot.state : Snapshot Evidence → LifecycleState
  | .provisional _ => .provisional
  | .confirmed _ => .confirmed
  | .invalidated _ => .invalidated

def Snapshot.lastAsOf : Snapshot Evidence → Nat
  | .provisional entry => entry.lastAsOf
  | .confirmed entry => entry.lastAsOf
  | .invalidated entry => entry.lastAsOf

def Snapshot.clocks : Snapshot Evidence → Clocks
  | .provisional entry =>
      { observedAt := entry.observedAt
        firstProvableAt := entry.firstProvableAt
        structureEndAt := none
        confirmedAt := none
        invalidatedAt := none }
  | .confirmed entry =>
      { observedAt := entry.observedAt
        firstProvableAt := some entry.firstProvableAt
        structureEndAt := some entry.structureEndAt
        confirmedAt := some entry.confirmedAt
        invalidatedAt := none }
  | .invalidated entry =>
      { observedAt := entry.observedAt
        firstProvableAt := entry.firstProvableAt
        structureEndAt := entry.structureEndAt
        confirmedAt := none
        invalidatedAt := some entry.invalidatedAt }

def Snapshot.invalidation :
    Snapshot Evidence → Option (InvalidationPayload Evidence)
  | .provisional _ => none
  | .confirmed _ => none
  | .invalidated entry => some entry.payload

def Entry.state (entry : Entry Evidence) : LifecycleState :=
  entry.snapshot.state

def Entry.clocks (entry : Entry Evidence) : Clocks :=
  entry.snapshot.clocks

def Entry.lastAsOf (entry : Entry Evidence) : Nat :=
  entry.snapshot.lastAsOf

def Entry.invalidation (entry : Entry Evidence) :
    Option (InvalidationPayload Evidence) :=
  entry.snapshot.invalidation

/-- 首次观察建仓；观察钟与门卫钟同取当前喂入点。 -/
def openEntry (current : Nat) : Entry Evidence :=
  { snapshot :=
      .provisional
        { observedAt := current
          firstProvableAt := none
          lastAsOf := current
          observed_le_last := Nat.le_refl current
          first_bounds := by simp }
    history := [.observed current] }

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 参数化力度门与纯函数 advance
    ═══════════════════════════════════════════════════════════════════════ -/

/--
R 域给出的三值结果。本文只消费它，不证明其计算：
`verifiedWeaker` = `Verified(true)`，`verifiedOvertake` = `Verified(false)`。
-/
inductive ForceCheck (Evidence : Type) where
  | verifiedWeaker
  | verifiedOvertake (evidence : Option Evidence)
  | unavailable

/-- 当前喂入点的观察或身份消失审计。 -/
inductive Feed (Evidence : Type) where
  | seen (force : ForceCheck Evidence) (structureComplete : Bool)
  | vanished (cause : VanishCause)

structure StepResult (Evidence : Type) where
  next : Snapshot Evidence
  delta : List (Revision Evidence)

private def touchProvisional
    (entry : ProvisionalSnapshot Evidence)
    (current : Nat)
    (hcurrent : entry.lastAsOf ≤ current) :
    ProvisionalSnapshot Evidence :=
  { observedAt := entry.observedAt
    firstProvableAt := entry.firstProvableAt
    lastAsOf := current
    observed_le_last := by
      have hobs := entry.observed_le_last
      omega
    first_bounds := by
      intro t ht
      have bounds := entry.first_bounds t ht
      omega }

private def touchConfirmed
    (entry : ConfirmedSnapshot Evidence)
    (current : Nat)
    (hcurrent : entry.lastAsOf ≤ current) :
    ConfirmedSnapshot Evidence :=
  { entry with
    lastAsOf := current
    clock_order := by
      rcases entry.clock_order with ⟨h₁, h₂, h₃, h₄⟩
      exact ⟨h₁, h₂, h₃, by omega⟩ }

private def touchInvalidated
    (entry : InvalidatedSnapshot Evidence)
    (current : Nat)
    (hcurrent : entry.lastAsOf ≤ current) :
    InvalidatedSnapshot Evidence :=
  { entry with
    lastAsOf := current
    observed_le_last := by
      have hobs := entry.observed_le_last
      omega
    first_bounds := by
      intro t ht
      have bounds := entry.first_bounds t ht
      omega
    structure_bounds := by
      intro t ht
      have bounds := entry.structure_bounds t ht
      omega
    first_le_structure := entry.first_le_structure
    first_le_invalidated := entry.first_le_invalidated
    structure_le_invalidated := entry.structure_le_invalidated
    reason_discipline := by
      cases hp : entry.payload with
      | forceOvertake evidence =>
          have discipline := entry.reason_discipline
          rw [hp] at discipline
          simp only at discipline ⊢
          rcases discipline with ⟨t, ht, hti, hil⟩
          exact ⟨t, ht, hti, by omega⟩
      | neverConstituted evidence =>
          have discipline := entry.reason_discipline
          rw [hp] at discipline
          simp only at discipline ⊢
          rcases discipline with ⟨hfirst, hstructure, hil⟩
          exact ⟨hfirst, hstructure, by omega⟩
      | identityVanished cause =>
          by_cases hclaim : current ≤ entry.invalidatedAt
          · exact .proved hclaim
          · exact .unprovedModelWithoutBridge hclaim }

/--
一步纯函数。

倒退喂入零改写；终态只允许门卫钟前移。Provisional 分支严格按三值门与完成信号：
* weaker 首写 first，完成则 Confirmed；
* overtake 且曾写 first 则 ForceOvertake；
* overtake、从未写 first 且结构完成则 NeverConstituted；
* unavailable 不写 first、不制造 Invalidated。
-/
def advanceSnapshot
    (snapshot : Snapshot Evidence)
    (current : Nat)
    (feed : Feed Evidence) :
    StepResult Evidence :=
  if hretro : current < snapshot.lastAsOf then
    { next := snapshot, delta := [] }
  else
    have hcurrent : snapshot.lastAsOf ≤ current := by omega
    match snapshot with
    | .confirmed entry =>
        { next := .confirmed (touchConfirmed entry current hcurrent), delta := [] }
    | .invalidated entry =>
        { next := .invalidated (touchInvalidated entry current hcurrent), delta := [] }
    | .provisional entry =>
        have hentryCurrent : entry.lastAsOf ≤ current := by
          simpa [Snapshot.lastAsOf] using hcurrent
        match feed with
        | .vanished cause =>
            let payload : InvalidationPayload Evidence := .identityVanished cause
            { next :=
                .invalidated
                  { observedAt := entry.observedAt
                    firstProvableAt := entry.firstProvableAt
                    structureEndAt := none
                    invalidatedAt := current
                    lastAsOf := current
                    payload := payload
                    observed_le_last := by
                      have hobs := entry.observed_le_last
                      omega
                    first_bounds := by
                      intro t ht
                      have bounds := entry.first_bounds t ht
                      omega
                    structure_bounds := by simp
                    first_le_structure := by simp
                    first_le_invalidated := by
                      intro t ht
                      have bounds := entry.first_bounds t ht
                      omega
                    structure_le_invalidated := by simp
                    observed_le_invalidated := by
                      have hobs := entry.observed_le_last
                      omega
                    reason_discipline := by
                      simp only [payload]
                      exact .proved (Nat.le_refl current) }
              delta := [.invalidated current payload] }
        | .seen force structureComplete =>
            match force with
            | .unavailable =>
                { next := .provisional (touchProvisional entry current hentryCurrent), delta := [] }
            | .verifiedWeaker =>
                match hfirst : entry.firstProvableAt with
                | some first =>
                    match structureComplete with
                    | false =>
                        { next := .provisional (touchProvisional entry current hentryCurrent)
                          delta := [] }
                    | true =>
                        have firstOrder := entry.first_bounds first hfirst
                        have hobs := entry.observed_le_last
                        { next :=
                            .confirmed
                              { observedAt := entry.observedAt
                                firstProvableAt := first
                                structureEndAt := current
                                confirmedAt := current
                                lastAsOf := current
                                clock_order := by omega }
                          delta := [.structureCompleted current, .confirmed current] }
                | none =>
                    match structureComplete with
                    | false =>
                        have hobs := entry.observed_le_last
                        { next :=
                            .provisional
                              { observedAt := entry.observedAt
                                firstProvableAt := some current
                                lastAsOf := current
                                observed_le_last := by omega
                                first_bounds := by
                                  intro t ht
                                  simp at ht
                                  subst t
                                  exact ⟨by omega, Nat.le_refl current⟩ }
                          delta := [.firstProvable current] }
                    | true =>
                        have hobs := entry.observed_le_last
                        { next :=
                            .confirmed
                              { observedAt := entry.observedAt
                                firstProvableAt := current
                                structureEndAt := current
                                confirmedAt := current
                                lastAsOf := current
                                clock_order := by omega }
                          delta :=
                            [.firstProvable current, .structureCompleted current,
                              .confirmed current] }
            | .verifiedOvertake evidence =>
                match hfirst : entry.firstProvableAt with
                | some first =>
                    have firstOrder := entry.first_bounds first hfirst
                    have hobs := entry.observed_le_last
                    let payload : InvalidationPayload Evidence := .forceOvertake evidence
                    { next :=
                        .invalidated
                          { observedAt := entry.observedAt
                            firstProvableAt := some first
                            structureEndAt := none
                            invalidatedAt := current
                            lastAsOf := current
                            payload := payload
                            observed_le_last := by omega
                            first_bounds := by
                              intro t ht
                              simp at ht
                              subst t
                              exact ⟨firstOrder.1, by omega⟩
                            structure_bounds := by simp
                            first_le_structure := by simp
                            first_le_invalidated := by
                              intro t ht
                              simp at ht
                              subst t
                              omega
                            structure_le_invalidated := by simp
                            observed_le_invalidated := by omega
                            reason_discipline := by
                              simp [payload]
                              omega }
                      delta := [.invalidated current payload] }
                | none =>
                    match structureComplete with
                    | false =>
                        { next := .provisional (touchProvisional entry current hentryCurrent)
                          delta := [] }
                    | true =>
                        let payload : InvalidationPayload Evidence :=
                          .neverConstituted evidence
                        have hobs := entry.observed_le_last
                        { next :=
                            .invalidated
                              { observedAt := entry.observedAt
                                firstProvableAt := none
                                structureEndAt := some current
                                invalidatedAt := current
                                lastAsOf := current
                                payload := payload
                                observed_le_last := by omega
                                first_bounds := by simp
                                structure_bounds := by
                                  intro t ht
                                  simp at ht
                                  subst t
                                  exact ⟨by omega, Nat.le_refl current⟩
                                first_le_structure := by simp
                                first_le_invalidated := by simp
                                structure_le_invalidated := by
                                  intro t ht
                                  simp at ht
                                  subst t
                                  exact Nat.le_refl current
                                observed_le_invalidated := by omega
                                reason_discipline := by
                                  simp [payload] }
                          delta :=
                            [.structureCompleted current, .invalidated current payload] }

/-- 外层唯一写史点：旧史只在尾部拼接本步 delta。 -/
def advance
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence) :
    Entry Evidence :=
  let result := advanceSnapshot entry.snapshot current feed
  { snapshot := result.next
    history := entry.history ++ result.delta }

/--
**票体 I7 / L0**：倒退喂入被拒绝时，快照与留档逐位零改动。
-/
theorem retrograde_no_change
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence)
    (hretro : current < entry.lastAsOf) :
    advance entry current feed = entry := by
  rcases entry with ⟨snapshot, history⟩
  have h : current < snapshot.lastAsOf := hretro
  simp [advance, advanceSnapshot, h]

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 三态合法性与五钟纪律
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 合法状态边：P 可留 P 或落两个终态；两个终态只能自吸收。 -/
def LegalStateStep (source target : LifecycleState) : Prop :=
  match source with
  | .provisional => True
  | .confirmed => target = .confirmed
  | .invalidated => target = .invalidated

/--
**I1 / L0**：`advance` 只产生 P→P/C/I、C→C、I→I。
-/
theorem advance_legal
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence) :
    LegalStateStep entry.state (advance entry current feed).state := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      trivial
  | confirmed confirmed =>
      by_cases hretro : current < confirmed.lastAsOf <;>
        simp [LegalStateStep, Entry.state, advance, advanceSnapshot, Snapshot.state,
          Snapshot.lastAsOf, hretro]
  | invalidated invalidated =>
      by_cases hretro : current < invalidated.lastAsOf <;>
        simp [LegalStateStep, Entry.state, advance, advanceSnapshot, Snapshot.state,
          Snapshot.lastAsOf, hretro]

/--
终态吸收的语义 frame：门卫钟可前移，但三态、五钟、终态载荷与留档不变。
-/
def SettlementFrame (before after : Entry Evidence) : Prop :=
  before.state = after.state ∧
    before.clocks = after.clocks ∧
    before.invalidation = after.invalidation ∧
    before.history = after.history

/-- **I1 / L0**：Confirmed 终态吸收。 -/
theorem confirmed_absorbing
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence)
    (hstate : entry.state = .confirmed) :
    SettlementFrame entry (advance entry current feed) := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      simp [Entry.state, Snapshot.state] at hstate
  | confirmed confirmed =>
      by_cases hretro : current < confirmed.lastAsOf <;>
        simp [SettlementFrame, Entry.state, Entry.clocks, Entry.invalidation, advance,
          advanceSnapshot, Snapshot.state, Snapshot.clocks, Snapshot.invalidation,
          Snapshot.lastAsOf, touchConfirmed, hretro]
  | invalidated invalidated =>
      simp [Entry.state, Snapshot.state] at hstate

/-- **I1 + I5 / L0**：Invalidated 终态吸收且既有留档不删。 -/
theorem invalidated_absorbing
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence)
    (hstate : entry.state = .invalidated) :
    SettlementFrame entry (advance entry current feed) := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      simp [Entry.state, Snapshot.state] at hstate
  | confirmed confirmed =>
      simp [Entry.state, Snapshot.state] at hstate
  | invalidated invalidated =>
      by_cases hretro : current < invalidated.lastAsOf <;>
        simp [SettlementFrame, Entry.state, Entry.clocks, Entry.invalidation, advance,
          advanceSnapshot, Snapshot.state, Snapshot.clocks, Snapshot.invalidation,
          Snapshot.lastAsOf, touchInvalidated, hretro]

/--
**I1 / L0**：Confirmed 的类型臂没有 Invalidated 钟或载荷位置。
-/
theorem confirmed_invalidated_side_empty
    (entry : Entry Evidence)
    (hstate : entry.state = .confirmed) :
    entry.clocks.invalidatedAt = none ∧ entry.invalidation = none := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot <;>
    simp [Entry.state, Entry.clocks, Entry.invalidation, Snapshot.state,
      Snapshot.clocks, Snapshot.invalidation] at hstate ⊢

/--
五钟偏序是路径敏感的，不是总链。IdentityVanished 臂保留 Rust 真实谓词，
但 `ClaimStatus090.unprovedModelWithoutBridge` 只携带反例，不证明该谓词。
-/
def ClockDiscipline (snapshot : Snapshot Evidence) : Prop :=
  match snapshot with
  | .provisional entry =>
      entry.observedAt ≤ entry.lastAsOf ∧
        ∀ t, entry.firstProvableAt = some t →
          entry.observedAt ≤ t ∧ t ≤ entry.lastAsOf
  | .confirmed entry =>
      entry.observedAt ≤ entry.firstProvableAt ∧
        entry.firstProvableAt ≤ entry.structureEndAt ∧
        entry.structureEndAt ≤ entry.confirmedAt ∧
        entry.confirmedAt ≤ entry.lastAsOf
  | .invalidated entry =>
      entry.observedAt ≤ entry.lastAsOf ∧
        (∀ t, entry.firstProvableAt = some t →
          entry.observedAt ≤ t ∧ t ≤ entry.lastAsOf) ∧
        (∀ t, entry.structureEndAt = some t →
          entry.observedAt ≤ t ∧ t ≤ entry.lastAsOf) ∧
        (∀ first structureAt,
          entry.firstProvableAt = some first →
          entry.structureEndAt = some structureAt →
          first ≤ structureAt) ∧
        (∀ first, entry.firstProvableAt = some first →
          first ≤ entry.invalidatedAt) ∧
        (∀ structureAt, entry.structureEndAt = some structureAt →
          structureAt ≤ entry.invalidatedAt) ∧
        entry.observedAt ≤ entry.invalidatedAt ∧
        match entry.payload with
        | .forceOvertake _ =>
            ∃ t, entry.firstProvableAt = some t ∧
              t ≤ entry.invalidatedAt ∧ entry.invalidatedAt ≤ entry.lastAsOf
        | .neverConstituted _ =>
            entry.firstProvableAt = none ∧
              entry.structureEndAt = some entry.invalidatedAt ∧
              entry.invalidatedAt ≤ entry.lastAsOf
        | .identityVanished _ =>
            ClaimStatus090 (entry.lastAsOf ≤ entry.invalidatedAt)

/--
**I2 / L0**：每个可构造快照都满足已证分支的准确五钟偏序；IdentityVanished
只得到其 Rust 钟谓词的 090 状态，不把“未证”升级成谓词成立。
-/
theorem clock_discipline (entry : Entry Evidence) :
    ClockDiscipline entry.snapshot := by
  cases entry.snapshot with
  | provisional provisional =>
      exact ⟨provisional.observed_le_last, provisional.first_bounds⟩
  | confirmed confirmed =>
      exact confirmed.clock_order
  | invalidated invalidated =>
      exact
        ⟨invalidated.observed_le_last, invalidated.first_bounds,
          invalidated.structure_bounds, invalidated.first_le_structure,
          invalidated.first_le_invalidated, invalidated.structure_le_invalidated,
          invalidated.observed_le_invalidated, invalidated.reason_discipline⟩

/--
**I2 / L0**：已有 first_provable 永不改写（既不后移，也不前移）。
-/
theorem first_provable_write_once
    (entry : Entry Evidence)
    (current first : Nat)
    (feed : Feed Evidence)
    (hfirst : entry.clocks.firstProvableAt = some first) :
    (advance entry current feed).clocks.firstProvableAt = some first := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      rcases provisional with
        ⟨observedAt, firstAt, lastAsOf, observed_le_last, first_bounds⟩
      cases firstAt with
      | none =>
          simp [Entry.clocks, Snapshot.clocks] at hfirst
      | some stored =>
          have hstored : stored = first := by
            simpa [Entry.clocks, Snapshot.clocks] using hfirst
          subst stored
          by_cases hretro : current < lastAsOf
          · simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
              Snapshot.lastAsOf, hretro]
          · cases feed with
            | vanished cause =>
                simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
                  Snapshot.lastAsOf, hretro]
            | seen force structureComplete =>
                cases force <;> cases structureComplete <;>
                  simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
                    Snapshot.lastAsOf, hretro, touchProvisional]
  | confirmed confirmed =>
      have hp : confirmed.firstProvableAt = first := by
        simpa [Entry.clocks, Snapshot.clocks] using hfirst
      by_cases hretro : current < confirmed.lastAsOf <;>
        simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
          Snapshot.lastAsOf, hretro, hp, touchConfirmed]
  | invalidated invalidated =>
      have hp : invalidated.firstProvableAt = some first := hfirst
      by_cases hretro : current < invalidated.lastAsOf <;>
        simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
          Snapshot.lastAsOf, hretro, hp, touchInvalidated]

/--
**I2 / L0**：若本步从空槽首写 first_provable，写入值只能是当前喂入点。
这同时见证“只在当前 advance 首写”与“禁回填历史”。
-/
theorem new_first_provable_at_current
    (entry : Entry Evidence)
    (current first : Nat)
    (feed : Feed Evidence)
    (hbefore : entry.clocks.firstProvableAt = none)
    (hafter : (advance entry current feed).clocks.firstProvableAt = some first) :
    first = current := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      rcases provisional with
        ⟨observedAt, firstAt, lastAsOf, observed_le_last, first_bounds⟩
      cases firstAt with
      | some stored =>
          simp [Entry.clocks, Snapshot.clocks] at hbefore
      | none =>
          by_cases hretro : current < lastAsOf
          · simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
              Snapshot.lastAsOf, hretro] at hafter
          · cases feed with
            | vanished cause =>
                simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
                  Snapshot.lastAsOf, hretro] at hafter
            | seen force structureComplete =>
                cases force <;> cases structureComplete <;>
                  simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
                    Snapshot.lastAsOf, hretro, touchProvisional] at hafter ⊢ <;>
                  omega
  | confirmed confirmed =>
      simp [Entry.clocks, Snapshot.clocks] at hbefore
  | invalidated invalidated =>
      have hp : invalidated.firstProvableAt = none := hbefore
      by_cases hretro : current < invalidated.lastAsOf <;>
        simp [advance, advanceSnapshot, Entry.clocks, Snapshot.clocks,
          Snapshot.lastAsOf, hretro, hp, touchInvalidated] at hafter

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 原因码互补（严格限定力度两码族）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 互补定理的有效域：只含 ForceOvertake / NeverConstituted，不含 IdentityVanished。 -/
def IsForcePayload (payload : InvalidationPayload Evidence) : Prop :=
  (∃ evidence, payload = .forceOvertake evidence) ∨
    (∃ evidence, payload = .neverConstituted evidence)

/--
**I3 / L0**：在力度两码族内严格互补：
ForceOvertake 当且仅当曾写 first；NeverConstituted 当且仅当从未写。

`IdentityVanished` 的 first 可有可无，因此明确排除在本定理有效域之外。
-/
theorem force_reason_first_complement
    (entry : InvalidatedSnapshot Evidence)
    (hforce : IsForcePayload entry.payload) :
    ((∃ evidence, entry.payload = .forceOvertake evidence) ↔
        ∃ first, entry.firstProvableAt = some first) ∧
      ((∃ evidence, entry.payload = .neverConstituted evidence) ↔
        entry.firstProvableAt = none) := by
  cases hp : entry.payload with
  | forceOvertake evidence =>
      have discipline := entry.reason_discipline
      rw [hp] at discipline
      simp only at discipline
      rcases discipline with ⟨first, hfirst, _⟩
      simp [hfirst]
  | neverConstituted evidence =>
      have discipline := entry.reason_discipline
      rw [hp] at discipline
      simp only at discipline
      rcases discipline with ⟨hfirst, _⟩
      simp [hfirst]
  | identityVanished cause =>
      rw [hp] at hforce
      simp [IsForcePayload] at hforce

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. Confirmed-only 类型门
    ═══════════════════════════════════════════════════════════════════════ -/

/--
沿用 `Origin.CertGatedExit` 的 typed-only 范式：先把合法命中收窄成能力值，
消费函数只接能力值。Closed 能力就是 Confirmed 快照本身；Invalidated 虽为终态，
但不是可消费 Closed。
-/
abbrev ClosedEntry (Evidence : Type) := ConfirmedSnapshot Evidence

/-- 一般条目只有 Confirmed 才能取得 Closed 能力。 -/
def toClosed? : Entry Evidence → Option (ClosedEntry Evidence)
  | { snapshot := .confirmed entry, .. } => some entry
  | _ => none

/-- 消费结果只暴露已闭合钟；形参在类型层不接受 Provisional / Invalidated。 -/
structure ConsumedClosed where
  observedAt : Nat
  firstProvableAt : Nat
  structureEndAt : Nat
  confirmedAt : Nat
deriving Repr

/--
Closed-only 消费入口。不存在 `Entry → ...` 的旁门。
-/
def consumeClosed (entry : ClosedEntry Evidence) : ConsumedClosed :=
  { observedAt := entry.observedAt
    firstProvableAt := entry.firstProvableAt
    structureEndAt := entry.structureEndAt
    confirmedAt := entry.confirmedAt }

/--
**I4 / L0**：取得 Closed 能力当且仅当原条目是 Confirmed。
-/
theorem toClosed_some_iff_confirmed (entry : Entry Evidence) :
    (∃ closed, toClosed? entry = some closed) ↔ entry.state = .confirmed := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot <;> simp [toClosed?, Entry.state, Snapshot.state]

/-- **I4 / L0**：Provisional 在类型门前被拒绝。 -/
theorem provisional_not_closed (entry : ProvisionalSnapshot Evidence) (history) :
    toClosed? ({ snapshot := .provisional entry, history := history } : Entry Evidence) = none :=
  rfl

/-- **I4 / L0**：Invalidated 留档可查，但不可作为 Closed 消费。 -/
theorem invalidated_not_closed (entry : InvalidatedSnapshot Evidence) (history) :
    toClosed? ({ snapshot := .invalidated entry, history := history } : Entry Evidence) = none :=
  rfl

/-! ═══════════════════════════════════════════════════════════════════════
    § 7. Invalidated append-only 留档
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 自证前缀谓词：新史 = 旧史 ++ 尾增量。 -/
def HistoryPrefix (before after : List α) : Prop :=
  ∃ tail, after = before ++ tail

/--
**I5 / L0**：每一步的修订史都是旧史追加尾增量，无删除/覆盖路径。
-/
theorem history_append_only
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence) :
    HistoryPrefix entry.history (advance entry current feed).history := by
  refine ⟨(advanceSnapshot entry.snapshot current feed).delta, ?_⟩
  rfl

/--
**I5 / L0**：若本步新进入 Invalidated，则同一失效钟、原因码与可选力度证据
已经作为一条 `Revision.invalidated` 写入 append-only 留档。
-/
theorem newly_invalidated_archived
    (entry : Entry Evidence)
    (current : Nat)
    (feed : Feed Evidence)
    (hbefore : entry.state ≠ .invalidated)
    (hafter : (advance entry current feed).state = .invalidated) :
    ∃ invalidatedAt payload,
      (advance entry current feed).clocks.invalidatedAt = some invalidatedAt ∧
        (advance entry current feed).invalidation = some payload ∧
        Revision.invalidated invalidatedAt payload ∈ (advance entry current feed).history := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      rcases provisional with
        ⟨observedAt, firstAt, lastAsOf, observed_le_last, first_bounds⟩
      by_cases hretro : current < lastAsOf
      · simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
          Snapshot.lastAsOf, hretro] at hafter
      · cases feed with
        | vanished cause =>
            refine ⟨current, .identityVanished cause, ?_⟩
            simp [advance, advanceSnapshot, Entry.clocks, Entry.invalidation,
              Snapshot.clocks, Snapshot.invalidation, Snapshot.lastAsOf, hretro]
        | seen force structureComplete =>
            cases force with
            | unavailable =>
                simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
                  Snapshot.lastAsOf, hretro] at hafter
            | verifiedWeaker =>
                cases firstAt with
                | none =>
                    cases structureComplete <;>
                      simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
                        Snapshot.lastAsOf, hretro] at hafter
                | some first =>
                    cases structureComplete <;>
                      simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
                        Snapshot.lastAsOf, hretro] at hafter
            | verifiedOvertake evidence =>
                cases firstAt with
                | some first =>
                    refine ⟨current, .forceOvertake evidence, ?_⟩
                    simp [advance, advanceSnapshot, Entry.clocks, Entry.invalidation,
                      Snapshot.clocks, Snapshot.invalidation, Snapshot.lastAsOf, hretro]
                | none =>
                    cases structureComplete with
                    | false =>
                        simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
                          Snapshot.lastAsOf, hretro] at hafter
                    | true =>
                        refine ⟨current, .neverConstituted evidence, ?_⟩
                        simp [advance, advanceSnapshot, Entry.clocks, Entry.invalidation,
                          Snapshot.clocks, Snapshot.invalidation, Snapshot.lastAsOf, hretro]
  | confirmed confirmed =>
      by_cases hretro : current < confirmed.lastAsOf <;>
        simp [advance, advanceSnapshot, Entry.state, Snapshot.state,
          Snapshot.lastAsOf, hretro] at hafter
  | invalidated invalidated =>
      exact (hbefore rfl).elim

/--
**I5 / L0**：任一 Invalidated 快照必有原因载荷；力度两码保留 `Option Evidence`
字段，IdentityVanished 则按定义没有力度证据。
-/
theorem invalidated_has_payload
    (entry : Entry Evidence)
    (hstate : entry.state = .invalidated) :
    ∃ payload, entry.invalidation = some payload := by
  rcases entry with ⟨snapshot, history⟩
  cases snapshot with
  | provisional provisional =>
      simp [Entry.state, Snapshot.state] at hstate
  | confirmed confirmed =>
      simp [Entry.state, Snapshot.state] at hstate
  | invalidated invalidated =>
      exact ⟨invalidated.payload, rfl⟩

/-- **I5 / L0**：ForceOvertake 载荷精确保留调用方给出的可选力度证据。 -/
theorem force_overtake_evidence_in_payload (evidence : Option Evidence) :
    (InvalidationPayload.forceOvertake evidence).forceEvidence = evidence :=
  rfl

/-- **I5 / L0**：NeverConstituted 载荷精确保留调用方给出的可选力度证据。 -/
theorem never_constituted_evidence_in_payload (evidence : Option Evidence) :
    (InvalidationPayload.neverConstituted evidence).forceEvidence = evidence :=
  rfl

/-- **I5 / L0**：IdentityVanished 无力度语义，其力度证据投影恒空。 -/
theorem identity_vanished_has_no_force_evidence (cause : VanishCause) :
    (InvalidationPayload.identityVanished (Evidence := Evidence) cause).forceEvidence = none :=
  rfl

/-!
公理审计：输出不得含证明占位公理或项目新增公理；`simp` 证明可列出 Lean 内建的
`propext` / `Classical.choice` / `Quot.sound`。
-/

#print axioms advance_legal
#print axioms retrograde_no_change
#print axioms confirmed_absorbing
#print axioms invalidated_absorbing
#print axioms confirmed_invalidated_side_empty
#print axioms clock_discipline
#print axioms first_provable_write_once
#print axioms new_first_provable_at_current
#print axioms force_reason_first_complement
#print axioms toClosed_some_iff_confirmed
#print axioms history_append_only
#print axioms newly_invalidated_archived
#print axioms invalidated_has_payload
#print axioms force_overtake_evidence_in_payload
#print axioms never_constituted_evidence_in_payload
#print axioms identity_vanished_has_no_force_evidence

end NewChanlun.Origin.NestLifecycleBook
