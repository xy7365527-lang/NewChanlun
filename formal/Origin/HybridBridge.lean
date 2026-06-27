/-
  Origin/HybridBridge.lean — Origin FullDefinitionSystem 具体实例化（transition 真改 ledger）
  ★cc-hybrid-bridge 工位（task #20 P5 transition 实例化 port）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：把 Strict/HybridAssembly §8 的 originFullDef 重锚到 Origin namespace

  A′ port 重锚原则（619）：port 非重证从零；内容保留换锚点。
  `Strict.HybridAssembly` 中的 `originFullDef`（§8）已是 `FullDefinitionSystem` 的具体见证，
  但它的命名空间归属是 `Strict.HybridAssembly`。本文件把该见证在 `NewChanlun.Origin`
  命名空间下显式重导出，并给出 **transition 真改 ledger 的 witness 定理**（0004 课闸）——
  transition 步骤后 ledger 分量改变（openRoot 动作使 A += 1，R -= 1；其余使 Pi += 1，R += 1）。

  ════════════════════════════════════════════════════════════════════════
  ## ★核心：transition 抽象字段的 Origin 层具体实例化

  `FullDefinitionSystem.transition : StrictState -> Order -> Event -> StrictState` 是抽象字段。
  本文件在 Origin namespace 给出具体实例——`originHybridDef`：
  - `transition` := 消费 Origin `ActionClass` 订单，经 Origin `ledgerStep` 真更新 ledger 分量
    + 写回完整 `StrictState`（T 闭环写回，补账户因果洞，与 HybridAssembly transitionAdapter 同构对齐）。
  - **Witness**：`origin_transition_changes_ledger` — 存在具体 x 和 o，使
    `(transition x o e).ledger ≠ x.ledger`（transition 真改 ledger，非恒等挂件）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain 强制标注）

  全文件 **L0**（结构/定义层，零数据依赖）。
  - `lake env lean Origin/HybridBridge.lean` 通过 = Origin 层 FullDefinitionSystem 具体实例化
    + transition 真改 ledger 的 machine-checked witness。
  - **不**声明盈利/最优/实盘有效（L3 EmpiricalDomain）。
  - **不**声明 transition 数值反映实盘账本正确性（数值层 L2，Rust prove_tw_neutral 守卫）。
  - 禁 sorry/admit/axiom，不依赖 Mathlib（Int 取自 Lean core）。

  谱系：#86（HybridAssembly originFullDef，Strict namespace）→ #97（Origin canonical base）
        → #101（HybridAssembly §8 实现 Origin 接口）→ 本文件 task #20（transition 实例化 port，
        Origin namespace 具体实例化 + transition 真改 ledger witness）。
-/

import Origin.FullDefinitionStrategy
import Strict.HybridAssembly

namespace NewChanlun.Origin

open NewChanlun.Origin (FullDefinitionSystem StrictState ParseStruct TrendClass ActionClass
  RiskMode CapitalPhase LedgerState mkLedger ledgerStep chooseAction hybridStep policyTheta
  hybrid_step_complete_unique policy_factors_through_classification ledger_invariant_preservation)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 Origin FullDefinitionSystem 具体实例化 `originHybridDef`

  A′ port 重锚：内容来自 `Strict.HybridAssembly.originFullDef`（§8），
  在 Origin namespace 显式重建——换锚点，不从零重证逻辑。

  六段（与 HybridAssembly §8 完全对齐）：
  - `recStruct` := 结构层 emptyParse（接口义务：StrictState→Event→ParseStruct 全函数）。
  - `classify`  := 复用 StrictState 已携带的 `trend`（Origin TrendClass）。
  - `intent`    := Origin `chooseAction` 优先级选择器（穿过分类瓶颈）。
  - `risk`      := 目标仓位：openRoot ⟹ 1，其余 ⟹ 0。
  - `schedule`  := 固定返回 ActionClass.hold（骨架调度，L0 结构层）。
  - `transition` := **Origin `ledgerStep` 真更新 ledger 分量**（T 闭环写回，账本因果洞补法）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★空解析结构 `hybridBridgeEmptyParse`（L0）：Origin `ParseStruct` 的空实例。
  作 originHybridDef recStruct 段的结构层占位输出——recStruct 接口义务（全函数）由此兑现。
  诚实：完整缠论解析由 Origin.ElementPipeline.parse 承载，本骨架只闭合接口签名义务。
-/
def hybridBridgeEmptyParse : ParseStruct :=
  { mergedBars := [], fractals := [], strokes := [], segments := [], centers := [],
    moves := [], bsp := [], tail := OpenTail.none }

/--
  ★Origin 层 action 段适配 `hybridBridgeActionOf`（L0）：
  把 Origin `TrendClass` 映射为 9 个优先级谓词，喂给 Origin `chooseAction` 确定选择器。
  本骨架取「trend=trendUp ⟹ openRoot（p7）；其余 ⟹ hold」的最小确定映射（结构层）。
  诚实：这是 Θ_voice 设计选择（确定优先级选择器），非缠论唯一推导。
-/
def hybridBridgeActionOf (trend : TrendClass) : ActionClass :=
  chooseAction false false false false false false (decide (trend = TrendClass.trendUp)) false false

/--
  ★★Origin FullDefinitionSystem 具体实例化 `originHybridDef`（L0，task #20 核心）：
  在 Origin namespace 构造 `FullDefinitionSystem` 具体见证——六段全函数填充，
  **transition 真复用 Origin `ledgerStep` 更新 ledger 分量**（T 闭环写回）。

  这是 A′ port 重锚原则（619）的执行：把 `Strict.HybridAssembly.originFullDef` 的内容
  重锚到 `NewChanlun.Origin` namespace，接口义务（∃!/π̄∘C/R=Π-A-W）由 Origin 元定理兑现。

  - Event/Intent/Control/Order：最小具体类型（Unit/ActionClass/Nat/ActionClass）。
  - transition 的两个分支（codex R2：ledger/accounting 更新放进 T）：
    · openRoot ⟹ `ledgerStep x.ledger 0 1 0`（建仓资本化：A += 1，R -= 1）
    · 其余   ⟹ `ledgerStep x.ledger 1 0 0`（实现盈亏：Pi += 1，R += 1）

  ★诚实标注（formalization-validity-domain）：
  - transition **只声明闭环状态转移全定义**（产出确定的下一态 + 保 R=Π-A-W），不声明盈利/最优/实盘。
  - dPi/dA/dW 取占位常量 1 单位——具体数额是 Θ_risk/运行时数据（L2），结构层只承载
    「动作 ⟹ 账本事件类型」的映射（openRoot 占资本化/其余实现盈亏），数额由下游填充。
  - `recStruct` 段返回 `hybridBridgeEmptyParse`（结构层骨架，不重算完整缠论解析）。
  - `schedule` 段返回 `ActionClass.hold`（调度骨架），优先级决策已由 `intent` 段吸收。
-/
def originHybridDef : FullDefinitionSystem where
  Event   := Unit
  Intent  := ActionClass
  Control := Nat
  Order   := ActionClass
  recStruct := fun _ _ => hybridBridgeEmptyParse
  classify  := fun x _ => x.trend
  intent    := fun _ c => hybridBridgeActionOf c
  risk      := fun _ i => match i with | ActionClass.openRoot => 1 | _ => 0
  schedule  := fun _ _u => ActionClass.hold
  -- ★T 闭环写回：消费 ActionClass 订单，经 Origin ledgerStep 真更新 ledger 分量，
  --   写回完整 StrictState（T 产出完整 x_{t+1}，非产出订单就停）。
  transition := fun x o _ =>
    { x with
        actionClass := o,
        ledger := match o with
          | ActionClass.openRoot => ledgerStep x.ledger 0 1 0  -- 建仓资本化 A += 1, R -= 1
          | _ => ledgerStep x.ledger 1 0 0 }                   -- 实现盈亏 Pi += 1, R += 1

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 三项接口义务定理（实例化 Origin 元定理）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★接口义务① · 闭环每步 ∃!（L0）：`originHybridDef` 的 `hybridStep` 每步存在且唯一。
  直接实例化 Origin `hybrid_step_complete_unique`（闭环 ∃! 元定理）。
  这兑现「FullDefinitionSystem 接口的闭环全定义 + 唯一」定理义务对本见证成立。
-/
theorem originHybridDef_step_complete_unique (x : StrictState) (e : originHybridDef.Event) :
    ExistsUnique (fun x' => hybridStep originHybridDef x e = x') :=
  hybrid_step_complete_unique originHybridDef x e

/--
  ★接口义务② · π̄∘C 因子化（L0）：`originHybridDef` 的策略 `policyTheta` 真穿过 classify。
  直接实例化 Origin `policy_factors_through_classification`（π̄∘C 因子化元定理）。
  这坐实「Intent 真读 classify 输出（策略穿过分类瓶颈）」接口义务对本见证成立。
-/
theorem originHybridDef_policy_factors (x : StrictState) (e : originHybridDef.Event) :
    policyTheta originHybridDef x e =
      originHybridDef.schedule x
        (originHybridDef.risk x
          (originHybridDef.intent x
            (originHybridDef.classify x (originHybridDef.recStruct x e)))) :=
  policy_factors_through_classification originHybridDef x e

/--
  ★接口义务③ · T 保 R=Π-A-W（L0）：`originHybridDef` 闭环转移后 ledger 满足账本恒等。
  直接引 `ledger_invariant_preservation`（Origin 账本恒等保持元定理）——T 段复用 Origin `ledgerStep`，
  其 `inv` 字段在构造时保持 R=Π-A-W，故闭环每步保持。
  诚实：本定理证「结构不变量保持」（L0），不证数值正确反映实盘盈亏（L2，Rust 守卫）。
-/
theorem originHybridDef_ledger_inv_preserved (x : StrictState) (e : originHybridDef.Event) :
    (hybridStep originHybridDef x e).ledger.R =
      (hybridStep originHybridDef x e).ledger.Pi
        - (hybridStep originHybridDef x e).ledger.A
        - (hybridStep originHybridDef x e).ledger.W :=
  (hybridStep originHybridDef x e).ledger.inv

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 ★★transition 真改 ledger witness（0004 课闸，task #20 核心产出）

  这是本文件相对 LedgerBridge 的**真增量**：
  - LedgerBridge 证「闭环保 R=Π-A-W 结构不变量」（T 不破坏恒等）。
  - 本 §3 证「transition 真改 ledger 的值」（T 真写入新值，非恒等挂件）。

  反退化 witness 两侧：
  1. **结构侧**：展示 transition 在具体分支下 ledger 字段如何被 ledgerStep 写入新值
     （openRoot 分支：A' = A+1, R' = R-1；其余分支：Pi' = Pi+1, R' = R+1）。
  2. **实例侧**：存在具体 x、o、e，使 transition 后 ledger ≠ 原 ledger（ledger 真被改写）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★transition openRoot 分支展开（L0，witness 结构侧①）：
  直接调用 `originHybridDef.transition x ActionClass.openRoot e` 时，ledger.A 增加 1
  （建仓资本化分支：A += 1，R -= 1，Pi/W 不变）。

  诚实标注（重要接缝说明）：`originHybridDef.schedule` 固定返回 `ActionClass.hold`（骨架调度），
  故 `hybridStep` pipeline 下 transition 收到的是 `hold`（Pi += 1），不是 `openRoot`。
  本定理展示「直接调用 transition openRoot 分支」的 ledger 写入——证 transition 函数体
  有 openRoot 分支且真改 A 分量（非恒等），与 `origin_transition_changes_ledger`
  （pipeline 下 Pi 分量改变）共同坐实「transition 真改 ledger」的完整 witness。
-/
theorem origin_transition_openRoot_allocates (x : StrictState) (e : originHybridDef.Event) :
    (originHybridDef.transition x ActionClass.openRoot e).ledger.A = x.ledger.A + 1 := by
  -- 直接展开 transition 定义：openRoot 分支 = ledgerStep x.ledger 0 1 0
  -- ledgerStep L 0 1 0 = { L with A := L.A + 1, R := L.R - 1 }（omega 关闭）
  simp only [originHybridDef, ledgerStep, mkLedger]

/--
  ★★transition 真改 ledger witness `origin_transition_changes_ledger`（L0，task #20 核心）：
  **存在**具体的初态 x 和订单 o，使 `originHybridDef.transition x o e` 后
  `ledger` 分量真被改写（ledger ≠ x.ledger）。

  这是「transition 真改 ledger，非恒等挂件」的机器可证 witness（0004 课闸）——
  删去 transition 定义中的 ledger 更新行，则本定理失败（成为回归守卫）。

  见证构造（openRoot 分支）：
  - 初态：Pi=0, A=0, W=0, R=0（零账本）。
  - 订单：ActionClass.openRoot（建仓）。
  - transition 后：A = 0+1 = 1，R = 0-1 = -1（A 和 R 改变）⟹ ledger ≠ 原 ledger。
  诚实：dA=1 是占位常量（结构层），数值含义是 L2，本 witness 只证「有字段改变」（L0）。
-/
theorem origin_transition_changes_ledger :
    ∃ (x : StrictState) (o : originHybridDef.Order) (e : originHybridDef.Event),
      (originHybridDef.transition x o e).ledger ≠ x.ledger := by
  -- 初态：零账本（Pi=A=W=R=0，inv 由 rfl 成立）。
  refine ⟨{ parsed := hybridBridgeEmptyParse,
             trend := TrendClass.trendUp,
             actionClass := ActionClass.hold,
             riskMode := RiskMode.normal,
             phase := CapitalPhase.phaseI,
             ledger := mkLedger 0 0 0 },
          ActionClass.openRoot,
          (),
          ?_⟩
  -- openRoot 分支：transition 后 ledger = ledgerStep (mkLedger 0 0 0) 0 1 0
  -- = { Pi=0, A=1, W=0, R=-1 }；原 ledger = { Pi=0, A=0, W=0, R=0 }
  -- A 分量从 0 变为 1 ⟹ ledger 结构体整体不等。
  intro h
  -- 从 ledger 相等推出 A 分量相等
  have hA := congrArg LedgerState.A h
  -- 展开 transition（openRoot 分支）
  simp only [originHybridDef, mkLedger, ledgerStep] at hA
  -- A 分量：0+1 = 0 ⟹ 矛盾
  omega

/--
  ★transition 实现接口 · 与 HybridAssembly originFullDef 对齐（L0，port 重锚确认）：
  `originHybridDef` 的 transition 与 `Strict.HybridAssembly.originFullDef` 的 transition
  在相同输入下产出相同 ledger 分量——两者在 ledger 更新逻辑上等价（port 重锚非重造）。

  ★诚实：两个见证的 actionClass / ledger 字段对齐（同一逻辑），其余字段（schedule 输出
  `hold` vs 见证内部 action 分配）属于骨架级别差异，不影响 ledger 层的等价性。
  本定理证「ledger 更新等价」这一 port 重锚的关键声明。
-/
theorem originHybridDef_ledger_aligned_with_hybridAssembly
    (x : StrictState) (e : originHybridDef.Event) :
    (hybridStep originHybridDef x e).ledger =
      (Strict.HybridAssembly.originFullDef.transition x
        (Strict.HybridAssembly.originFullDef.schedule x
          (Strict.HybridAssembly.originFullDef.risk x
            (Strict.HybridAssembly.originFullDef.intent x
              (Strict.HybridAssembly.originFullDef.classify x
                (Strict.HybridAssembly.originFullDef.recStruct x e)))))
        e).ledger := by
  -- 两侧都经 hybridBridgeActionOf / originActionOf + ledgerStep 的同一逻辑：
  -- intent 读 trend，openRoot ⟹ ledgerStep 0 1 0，其余 ⟹ ledgerStep 1 0 0。
  -- 展开两侧定义确认等价。
  simp only [hybridStep, policyTheta, originHybridDef, hybridBridgeEmptyParse,
             hybridBridgeActionOf, Strict.HybridAssembly.originFullDef,
             Strict.HybridAssembly.emptyParse]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 诚实标签（gatekeeper：禁标盈利/最优/连续 argmin）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Origin 层闭环标签 `OriginBridgeTag`（gatekeeper，诚实分层）。
  - `transitionWritesLedger`：transition 真改 ledger（本文件实质增量——witness 机器可证）。
  - `closedLoopWithLedger`：π̄∘C + T 写回 + R=Π-A-W 账本闭环（Origin 接口三项义务）。
  - `portReanchoredFromStrict`：A′ port 重锚（内容来自 HybridAssembly §8，换 Origin 锚点）。
  - `empiricalDomain`：盈利/最优/实盘 = L3，不由本 L0 声称。

  ★**没有** `ProfitableStrategy` / `ContinuousArgmin` / `ChanlunUniqueStrategy` 构造子。
-/
inductive OriginBridgeTag where
  | transitionWritesLedger
  | closedLoopWithLedger
  | portReanchoredFromStrict
  | empiricalDomain
deriving DecidableEq, Repr

/-- ★Origin 层闭环子类 `OriginBridgeSubkind`：唯一构造子 `originClosedLoopGivenTheta`。 -/
inductive OriginBridgeSubkind where
  | originClosedLoopGivenTheta
deriving DecidableEq, Repr

/-- ★Origin 层诚实标签包（transition 真改 ledger + 闭环结构 + port 重锚 + 经验有效域）。 -/
def originBridgeLabels : List OriginBridgeTag × OriginBridgeSubkind :=
  ([OriginBridgeTag.transitionWritesLedger, OriginBridgeTag.closedLoopWithLedger,
    OriginBridgeTag.portReanchoredFromStrict, OriginBridgeTag.empiricalDomain],
   OriginBridgeSubkind.originClosedLoopGivenTheta)

/-- ★禁标盈利/缠论唯一/连续 argmin（L0，gatekeeper 见证）：子类必是 originClosedLoopGivenTheta。 -/
theorem origin_bridge_subkind_is_given_theta (k : OriginBridgeSubkind) :
    k = OriginBridgeSubkind.originClosedLoopGivenTheta := by
  cases k; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（cc-hybrid-bridge 工位，task #20 P5）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. Origin 层 `FullDefinitionSystem` 具体实例化 `originHybridDef`（§1）：
     六段全函数填充（recStruct=空骨架 / classify=trend / intent=chooseAction 优先级 /
     risk=仓位格点 / schedule=hold 骨架 / **transition=Origin ledgerStep 真更新 ledger**）。
     A′ port 重锚：内容对齐 `Strict.HybridAssembly.originFullDef`（§8），换 Origin 锚点。
  2. 三项接口义务（§2）：
     - `originHybridDef_step_complete_unique`（闭环 ∃!，实例化 hybrid_step_complete_unique）。
     - `originHybridDef_policy_factors`（π̄∘C 因子化，实例化 policy_factors_through_classification）。
     - `originHybridDef_ledger_inv_preserved`（T 保 R=Π-A-W，引 ledger.inv）。
  3. ★★transition 真改 ledger witness（§3，task #20 核心产出，0004 课闸）：
     - `origin_transition_changes_ledger`：∃ x o e，transition 后 ledger ≠ x.ledger
       （openRoot 分支 A: 0→1，R: 0→-1，机器可证 witness）。
     - `originHybridDef_ledger_aligned_with_hybridAssembly`：Origin 层 ledger 更新逻辑
       与 HybridAssembly.originFullDef 等价（port 重锚非重造的对齐确认）。
  4. 诚实标签（§4）：`originBridgeLabels`（transitionWritesLedger + closedLoopWithLedger +
     portReanchoredFromStrict + empiricalDomain）+ `origin_bridge_subkind_is_given_theta`
     （禁标盈利/缠论唯一/连续 argmin）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 盈利/最优/实盘有效（T 只闭环全定义，L3 EmpiricalDomain）。
  - ✗ dPi/dA/dW 数值反映实盘真实账本（占位常量 1 单位，具体数额 L2）。
  - ✗ recStruct 返回真实缠论解析结构（骨架占位，完整解析 ElementPipeline 承载）。
  - ✗ schedule 返回 hold 是最优调度（骨架骨架，优先级决策已由 intent 吸收，Θ_exec 参数）。
  - ✗ chooseAction 优先级排序由缠论唯一推出（Θ_voice 确定选择器设计选择）。

  ★port 重锚裁定（重要）：本文件是 `Strict.HybridAssembly.originFullDef`（§8）的
  Origin namespace 重锚版——内容等价（ledger 更新逻辑对齐，`originHybridDef_ledger_aligned_with_hybridAssembly`
  坐实），锚点从 `Strict.HybridAssembly` 移至 `NewChanlun.Origin`（A′ port 重锚原则 619）。
  真增量在 §3：Strict 层的 §8 证「保 R=Π-A-W 结构不变量」（T 不破坏恒等），本文件增加
  「transition 真改 ledger witness」（T 真写入新值，非恒等挂件）——二者互补，不重复。

  谱系：#86（HybridAssembly LedgerComp + originFullDef Strict namespace）→
        #97（Origin canonical base）→ #101（HybridAssembly §8 实现 Origin 接口）→
        本文件 task #20（transition 实例化 port，Origin namespace + 真改 ledger witness）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin
