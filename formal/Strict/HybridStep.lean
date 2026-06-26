/-
  Strict/HybridStep.lean — 单一闭环混合状态机 S_Θ + hybridStep + T（全定义策略装配，L0）
  ★装配工位（codex 编排者代理裁决 R1=B/R2，2026-06-26）：吸收 GPT 全定义策略的「装配/闭环」优势。

  ════════════════════════════════════════════════════════════════════════
  ## 问题：我们有 60 个零件，但从未装配成闭环（Chain 是并列合取）

  - `Strict/Chain.lean` 的 `chanlun_chain_total_unique` 把 C_Θ 与 π_Θ **并列合取 ⟨_,_⟩**：
    `(∃! C_Θ(x)) ∧ (∃! π_Θ(h,z))`——**π 不读 C，无转移 T，无闭环，无单一态 x_t**。
  - `Strict/StrategyFamily.lean` 的 `piTheta` 抽象 4 段产出**订单就停**（recog 读 (h,z) 不读 C_Θ）。
  - `Tlayers/Dynamics.lean` 的 δ 是**解析级微状态**转移（4 字段），非完整策略态 x_t。

  GPT 包的全定义策略**已装配**：单一闭合 S_Θ=(X,E,Rec,C,Intent,K,J,Schedule,T) + 完整态 x_t
  + `x_{t+1}=T(x_t,π(x_t),e)` 闭环 + 单一 hybrid_step 总定理。**装配维度 GPT 领先一整层。**

  本文件兑现 docs/formal/full-definition-strategy-v1.md 蓝图的**闭环接口**：把六段接成单一
  `hybridStep : HybridState → Event → HybridState`，**强制**两条 Chain 没有的结构约束：
    ① `Intent` 段**真读 `classify` 的输出**（π̄ = Intent∘…∘Classify，即 π_Θ = π̄_Θ ∘ C_Θ）；
    ② `transition` 段**真写回完整 HybridState**（T 闭环，非产出订单就停）。

  ════════════════════════════════════════════════════════════════════════
  ## 诚实标注（formalization-validity-domain + 090，codex R3）

  - `hybridStep_total_unique` 是**函数图 ∃!**（hybridStep 是 Lean 全函数，自动成立，最弱必要侧，
    同 Chain.totalUnique_of_fun / StrategyFamily.given_theta_total_unique）。本文件的**实质内容
    不在 ∃!**，而在**结构约束**：`policy_factors_through_classify`（π̄ 真读 C_Θ）+
    `hybridStep_is_closed_transition`（T 真写回 x_t）——这两条把 Chain 的「并列合取」升级为
    「闭环复合」，是 GPT 装配优势的 L0 兑现。
  - **不证**（codex R3 诚实边界，装配不得声明膨胀）：
    ✗ 盈利/最优/实盘有效（T 只声明闭环全定义，L3 经验）；
    ✗ RiskProj 连续 argmin 存在性（`risk` 段只能确定选择器/有限网格，见 StrategyFamily.RiskProjector）；
    ✗ 缠论无参数唯一策略（K 各段含 Θ，616/617）；
    ✗ 标签缠论语义（C_Θ=classify 只给 fiber partition，逐 claim 承载，615）；
    ✗ 10 级动作优先级由缠论唯一推出（`intent` 段是确定优先级选择器）。
  - **账户因果洞补法（codex R3）**：`transition` 段把 ledger/accounting 更新写进闭环 x_t，
    使账户态 z 的生成因果进入转移（补 StrategyFamily.piTheta_causal「只比同一 z」的洞）。
    本骨架把 transition 抽象为字段，**因果义务挂到 adapter**（Causal/Accounting 实例化时承载）。

  范式：纯 Prop/Type，不依赖 Mathlib。禁 sorry/admit/axiom。
  验证：`cd formal && lake env lean Strict/HybridStep.lean`。
  谱系：615/616/617（C 与 π 都 Θ-参数化）→ 本文件（闭环装配，C 与 π 由并列升级为复合）。
-/
import Strict.Chain

namespace Strict.HybridStep

open Strict.Chain (TotalUnique totalUnique_of_fun totalUnique_prod)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 闭环组件接口 HybridComponents（六段 + 完整态 + Event）

  把全定义策略的六段抽象为接口字段——**不**先硬塞具体实现（Parse/Fugue/RiskProj/Dynamics
  作为 adapter 后续实例化）。关键的两条结构约束直接编码在字段签名里：
  - `intent : HybridState → Class → Intent`——Intent **以 Class 为输入**（π̄ 真读 C_Θ）。
  - `transition : HybridState → Order → Event → HybridState`——T **产出完整 HybridState**（闭环）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★闭环组件接口 `HybridComponents`（codex R2，L0 抽象接口）。

  完整态 `HybridState` = 蓝图 §2 乘积态（hist/microState/globalState/ledgerState/voiceState/
  riskMode/phase/positions/orders/memory/metrics）——本接口对其内部结构不可知（全称抽象）。

  六段（数据流顺序 Rec→Class→Intent→Risk→Schedule→T）：
  - `recStruct : HybridState → Event → Struct`：Rec 段（Parse/Level，Θ_parse 参数化）。
  - `classify : HybridState → Struct → Class`：**C_Θ 段**（globalClassify，fiber partition）。
  - `intent : HybridState → Class → Intent`：**Intent 段，输入含 Class**（π̄ 真读 C_Θ，
    10 级动作优先级确定选择器）。
  - `risk : HybridState → Intent → Control`：RiskProj 段（确定选择器/有限网格，非连续 argmin）。
  - `schedule : HybridState → Control → Order`：Schedule 段（Exec，执行顺序 Θ_exec）。
  - `transition : HybridState → Order → Event → HybridState`：**T 段，产出完整 HybridState**
    （闭环写回，含 ledger/accounting 更新）。
-/
structure HybridComponents where
  HybridState : Type
  Event : Type
  Struct : Type
  Class : Type
  Intent : Type
  Control : Type
  Order : Type
  recStruct : HybridState → Event → Struct
  classify : HybridState → Struct → Class
  intent : HybridState → Class → Intent
  risk : HybridState → Intent → Control
  schedule : HybridState → Control → Order
  transition : HybridState → Order → Event → HybridState

/--
  ★★闭环一步 `hybridStep`（codex R2，L0）：`x_t ─e→ x_{t+1}` 单一闭环转移。

  组合链（六段，数据流强制 π̄∘C + T 写回）：
  ```
  d := recStruct x e        -- Rec
  c := classify x d         -- C_Θ
  i := intent x c           -- Intent（真读 c = C_Θ 输出 ⟹ π̄∘C）
  u := risk x i             -- RiskProj
  o := schedule x u         -- Schedule
  transition x o e          -- T（产出完整 x_{t+1}，闭环写回）
  ```
  这是编排者公式 `x_{t+1}=T_Θ(x_t, Schedule(x_t, RiskProj(x_t, Intent(C_Θ(x_t)))), e)` 的兑现。
-/
def hybridStep (K : HybridComponents) (x : K.HybridState) (e : K.Event) : K.HybridState :=
  let d := K.recStruct x e
  let c := K.classify x d
  let i := K.intent x c
  let u := K.risk x i
  let o := K.schedule x u
  K.transition x o e

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 闭环全定义 + 唯一（函数图 ∃!，诚实标注最弱必要侧）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★闭环每步全定义 + 唯一（L0，函数图 ∃!）：`∀ x e, ∃! x', hybridStep K x e = x'`。

  ★诚实标注（codex R3，避免声明膨胀）：`hybridStep` 是 Lean 全函数，故存在（= 自身）+ 唯一
  （= 函数确定性）**自动成立**，证明体仅 `rfl` + 对称。**本定理不是实质内容**——它兑现「闭环
  每步确定」这一最弱必要侧（同 Chain.totalUnique_of_fun / StrategyFamily.given_theta_total_unique）。
  实质内容在 §3（π̄∘C + T 写回的结构约束）。各段的语义唯一性由 adapter 的 *_total_unique 承载。
-/
theorem hybridStep_total_unique (K : HybridComponents) (x : K.HybridState) (e : K.Event) :
    ∃ x', hybridStep K x e = x' ∧ ∀ x'', hybridStep K x e = x'' → x'' = x' :=
  ⟨hybridStep K x e, rfl, fun _ h => h.symm⟩

/--
  ★闭环 = TotalUnique（L0，接入 Chain 内核）：`hybridStep K · e` 是 `TotalUnique`。
  复用 Chain.totalUnique_of_fun——任意全函数自动 TotalUnique。这让闭环步可接入 Chain 的
  乘积组合内核（adapter 实例化时用 totalUnique_prod 把各段唯一性组装为闭环唯一性）。
-/
theorem hybridStep_totalUnique (K : HybridComponents) (e : K.Event) (x : K.HybridState) :
    TotalUnique (fun x => hybridStep K x e) x :=
  totalUnique_of_fun (fun x => hybridStep K x e) x

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 实质内容：π̄∘C + T 写回（把 Chain 并列合取升级为闭环复合）

  这是本文件相对 Chain.lean 的真增量——两条结构约束，是 GPT 装配优势的 L0 兑现。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★策略经分类因子 `policyOutput`（L0）：从 x、e 经 Rec→Class→Intent→Risk→Schedule 得订单
  （**不含** T）。这是 π̄_Θ 的输出——注意它**经过 classify**（C_Θ）。
-/
def policyOutput (K : HybridComponents) (x : K.HybridState) (e : K.Event) : K.Order :=
  K.schedule x (K.risk x (K.intent x (K.classify x (K.recStruct x e))))

/--
  ★★π_Θ = π̄_Θ ∘ C_Θ（L0，本文件核心结构约束之一）：
  `policy_factors_through_classify` — 策略输出真的**穿过分类**。

  `policyOutput K x e = schedule x (risk x (intent x (classify x (recStruct x e))))`——
  Intent 段的输入 `classify x (recStruct x e)` **就是 C_Θ 的输出**。故整条策略 π̄ 经
  `intent ∘ classify` 因子化——**π_Θ 真读 C_Θ**。

  这与 `Chain.chanlun_chain_total_unique` 的**并列合取**（C_Θ 与 π_Θ 各自唯一、无连接）形成
  对照：本文件把 π 接在 C 之后，决策**穿过**分类瓶颈——这是「决策充分性 L1 可表达」的结构
  前提（D2：在 C^dec_Θ 上可证 C(x)=C(y)⟹π(x)=π(y)），Chain 的并列合取连表达都表达不了。

  ★诚实：本定理是定义性展开（rfl）——它**断言数据流结构**（intent 读 classify 输出），
  不断言「C_Θ 的标签有缠论语义」（后者逐 claim 承载）。实质 = 把策略钉死为经分类的复合。
-/
theorem policy_factors_through_classify (K : HybridComponents) (x : K.HybridState) (e : K.Event) :
    policyOutput K x e
      = K.schedule x (K.risk x (K.intent x (K.classify x (K.recStruct x e)))) :=
  rfl

/--
  ★★T 闭环写回 `hybridStep_is_closed_transition`（L0，本文件核心结构约束之二）：
  闭环一步 = transition 作用于 (x, 策略订单, e)——**T 真写回完整 HybridState**。

  `hybridStep K x e = transition x (policyOutput K x e) e`——下一态由 T 从当前态 x、
  策略订单 `policyOutput`、外部事件 e 唯一产出。这与 `StrategyFamily.piTheta`（产出订单就停，
  无 T）+ `Chain`（无闭环）形成对照：本文件**闭合了循环**（x_t → x_{t+1}）。

  这是 L2 回测/实时一致性的**形式落点**（codex D5/R1）：没有 `x_{t+1}=T(...)`，回测与实时的
  状态推进没有共同的形式契约；有了它，两者都消费同一个 hybridStep。

  ★诚实：T 只声明**闭环状态转移全定义**（产出确定的下一态），**不**声明该转移盈利/最优/
  实盘有效（L3，codex R3）。账户因果（z 的生成）由 transition 的 adapter（Accounting）承载。
-/
theorem hybridStep_is_closed_transition (K : HybridComponents) (x : K.HybridState) (e : K.Event) :
    hybridStep K x e = K.transition x (policyOutput K x e) e :=
  rfl

/--
  ★决策充分性蕴含的结构前提（L0，L1 证书的入口）：
  `same_policy_input_same_output` — 若两个 (x,e) 与 (y,e') 在 policyOutput 的全部上游输入
  （即经 classify 后的链）一致，则策略输出一致。

  本骨架给出 L1（决策充分性）的**可表达形式**：因 policyOutput 穿过 classify
  （`policy_factors_through_classify`），「同分类链 ⟹ 同策略」可被陈述——这是 D2 裁决
  「装配后 π=π̄∘C 使决策充分性可表达」的兑现入口。完整 L1（C^dec_Θ(x)=C^dec_Θ(y)⟹π(x)=π(y)）
  由后续 C^dec_Θ adapter 在含账户/执行统计量的扩展分类上证（本文件只给结构前提）。

  ★诚实：这是数据流的平凡推论（同输入同输出 = 函数性），**不是** L1 本身——L1 需证「分类
  标签塌缩到同类的两个不同 x 仍同策略」，那依赖 classify 的具体粒度（D2/D3：连续风险量下
  全局 L1 可能 false，须 C^dec_Θ 局部证）。本定理只兑现「π 穿过 C ⟹ L1 可表达」的结构前提。
-/
theorem same_policy_input_same_output (K : HybridComponents)
    (x y : K.HybridState) (e e' : K.Event)
    (h : K.classify x (K.recStruct x e) = K.classify y (K.recStruct y e'))
    (hx : x = y) :
    policyOutput K x e = policyOutput K y e' := by
  subst hx
  unfold policyOutput
  rw [h]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 诚实标签（gatekeeper：ClosedLoopAssembly，禁标盈利/最优/真完全分类）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★闭环装配标签 `AssemblyTag`（gatekeeper，诚实分层）。
  - `ClosedLoopStructure`：π̄∘C + T 写回的闭环结构（本文件实质，相对 Chain 并列合取的增量）。
  - `FunctionGraphUnique`：闭环步 ∃! 是函数图平凡侧（最弱必要侧，非实质）。
  - `EmpiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称（T 只闭环全定义）。

  ★**没有** `ProfitableStrategy` / `TrueCompleteClassification` 构造子——类型层拒绝把闭环装配
  标为盈利策略或真完全分类。闭环装配 = 「给定 Θ 的确定执行策略的闭合形式」，非缠论无参数真理。
-/
inductive AssemblyTag where
  | ClosedLoopStructure
  | FunctionGraphUnique
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★闭环装配子类 `AssemblySubkind`：ClosedLoopGivenTheta（唯一构造子，类型层钉死诚实标签）。 -/
inductive AssemblySubkind where
  | ClosedLoopGivenTheta
deriving DecidableEq, Repr

/-- ★诚实标签包：闭环结构 + 函数图唯一 + 经验有效域，子类 ClosedLoopGivenTheta。 -/
def assemblyLabels : List AssemblyTag × AssemblySubkind :=
  ([AssemblyTag.ClosedLoopStructure, AssemblyTag.FunctionGraphUnique, AssemblyTag.EmpiricalDomain],
   AssemblySubkind.ClosedLoopGivenTheta)

/-- ★禁标盈利/真完全分类（L0，gatekeeper 见证）：闭环装配子类必是 ClosedLoopGivenTheta。 -/
theorem assembly_subkind_is_given_theta (k : AssemblySubkind) :
    k = AssemblySubkind.ClosedLoopGivenTheta := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（装配工位，codex R1=B/R2）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. 闭环组件接口 `HybridComponents`（六段 + 完整态 + Event，π̄∘C + T 写回编码在签名里）。
  2. 闭环一步 `hybridStep`（x_t ─e→ x_{t+1} 单一转移）+ `hybridStep_total_unique`（函数图 ∃!）+
     `hybridStep_totalUnique`（接入 Chain 乘积内核）。
  3. ★实质增量（相对 Chain 并列合取）：`policy_factors_through_classify`（π_Θ=π̄∘C_Θ 真读分类）+
     `hybridStep_is_closed_transition`（T 真写回完整 x_t，L2 回测形式落点）+
     `same_policy_input_same_output`（L1 决策充分性可表达的结构前提）。
  4. 诚实标签 `assemblyLabels`（ClosedLoopStructure + FunctionGraphUnique + EmpiricalDomain +
     ClosedLoopGivenTheta）+ `assembly_subkind_is_given_theta`（禁标盈利/真完全分类）。

  本文件**不证**（codex R3 诚实边界）：盈利/最优/实盘（T 只闭环全定义，L3）；RiskProj 连续
  argmin（risk 段确定选择器）；缠论无参数唯一策略（K 各段含 Θ）；标签缠论语义（逐 claim）；
  10 级优先级缠论唯一（intent 是确定选择器）。

  ★后续 adapter（codex R2 ③，各证满足 HybridComponents 字段义务）：
  - recStruct ← Parse/LevelState；classify ← Chain.globalClassify；intent ← Fugue + 优先级选择器；
  - risk ← RiskProj（确定选择器/RiskGrid）；schedule ← StrategyFamily.exec；
  - transition ← 新建闭环 T（Dynamics.δ 更新 microState 分量 + Accounting 更新 ledger 分量）。
  ════════════════════════════════════════════════════════════════════════ -/

end Strict.HybridStep
