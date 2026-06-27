/-
Origin/CompleteStateEvent.lean — FULL 结果包 §3 完整状态 x_t（17 分量）+ §20 外部事件 e（8 元组）
的逐分量显式 schema（组C 补全工位，零遗漏）。

★工位定位（cov-rust-impl E1 状态~50% / D7 事件仅价格笔 的根源补全）：

  现有 Origin canonical 的状态是 `FullDefinitionStrategy.StrictState`（FullDefinitionStrategy.lean
  :205-212）——一个 **6 分量乘积态摘要**（parsed × trend × actionClass × riskMode × phase × ledger）。
  它是 `hybridStep` 闭环用的**工程化简态**：声部/订单/记忆被折叠进抽象 parsed 或根本不出现，
  σ_r 根方向、ω 订单状态、E 清算权益、Cash、ν（借券/保证金/场所）等 §3 分量**未作为逐分量
  schema 存在**。同样，`FullDefinitionSystem.Event`（:215）是抽象 `Type`，§20 的 8 元组事件
  e=(y,Fill,Reject,Fee,Funding,MarginUpdate,BorrowUpdate,CorpAction) **从未被实例化为显式结构**。

  这正是 cov-rust-impl 报告「rust state 仅 ~50%、事件仅价格/笔」的形式化根源：被镜像的 StrictState
  本身就是摘要态。本文件把 FULL §3 的完整 17 分量与 §20 的 8 元组事件**逐分量显式化**——不摘要、
  不折叠、零遗漏——作为 Origin canonical 的完整状态/事件 schema。

★诚实标注（no-patch / formalization-validity-domain）：

  - 本文件 = **L0**（纯结构 schema：17 分量结构 + 8 元组结构 + 记账恒等 `R=Π-A-W` 作类型层不变量）。
    L0 = 不依赖任何数据，从 §3/§20 定义直接转写为 Lean 结构。零信息增量（同义反复：FULL 文字 ↦ Lean 类型）。
  - 记账恒等 `R=Π-A-W` 作为结构字段携带证明项（类型层强制），复用 Origin `LedgerState` 的既有不变量
    （不重新定义账本——`LedgerState` 已在 FullDefinitionStrategy.lean:184-203 携带 `inv` +
    `ledger_invariant_preservation`）。本文件不重证账本保持律，只把 §3 的「合法状态必须满足 R=Π-A-W」
    显式锚到 §3 完整态的 `ledger` 分量上。
  - **跨组依赖诚实标注**：`D_t`（所有级别缠论递归结构）依赖中枢/走势递归——组B（CenterConstruct）
    在做该层。本文件**不臆造** `D_t` 的递归内容，用 Origin canonical 已存在的 `ParseStruct`
    （ChanlunElements.lean:111，已含 mergedBars/fractals/strokes/segments/centers/moves/tail）作为
    `D_t` 的结构承载——这是**真实存在的 Origin 类型**，非占位桩。h_t（市场历史）= `List Bar`。
    若组B 后续把 D_t 精化为多级别递归树（RecursiveLevelSystem），本 schema 的 `recStruct` 分量
    类型可重锚到那个更细的类型——届时是 schema 精化，非本文件的缺陷。

依赖方向（单向无环，全 committed 只读）：CompleteStateEvent → {ChanlunElements, FullDefinitionStrategy}。
不依赖 CenterConstruct（组B）/ Can 塔（组A）——只引 ChanlunElements 的 ParseStruct/Bar 与
FullDefinitionStrategy 的 CapitalPhase/LedgerState/RiskMode（均 Origin canonical committed）。

命名空间 NewChanlun.Origin。待 Lead 登记 root：`NewChanlun.Origin.CompleteStateEvent`。
-/

import Origin.ChanlunElements
import Origin.FullDefinitionStrategy

namespace NewChanlun.Origin.CompleteStateEvent

open NewChanlun.Origin
  (ParseStruct Bar CapitalPhase LedgerState mkLedger ledgerStep RiskMode)

/-! ## §3 分量级原子类型（FULL line 148-164 逐条转写）

每个原子类型对应 §3 `\begin{aligned}` 块中的一行定义。按依赖序：先所有原子，再组装 17 分量
`CompleteState`，最后 §20 的 8 元组 `ExternalEvent`。-/

/--
根方向 `σ_{r,t} ∈ {-1,0,+1}`（FULL §3 line 150：空头/空仓/多头）。

★三态完全分类——这是缠论操作的最顶层极性。子声部方向由 `σ_v = σ_r·(-1)^{d(v)}` 递归导出
（§8 line 463-467），故根方向是整棵声部树方向的生成元。
-/
inductive RootDirection where
  | short    -- σ_r = -1（空头）
  | flat     -- σ_r =  0（空仓）
  | long     -- σ_r = +1（多头）
deriving Repr, DecidableEq

/-- 根方向到 `Int` 的语义投影（-1/0/+1，对齐 FULL §3 line 150 的数值约定）。 -/
def RootDirection.toInt : RootDirection -> Int
  | .short => -1
  | .flat  =>  0
  | .long  =>  1

/--
声部订单状态 `ω_{v,t}`（FULL §3 line 153：空仓、待开、持仓、待平）。

★四态完全分类——这是单个声部的开平状态机相位（§10 line 580-590 的 `\tilde a_{v,t+1}` 状态递归
在「持仓 a=1」与「空仓 a=0」之间转移，「待开/待平」是订单已发未成交的中间相）。`ω` 区别于 `a`
（是否激活，bool）：`a` 是**持仓事实**，`ω` 是**订单相位**——同一 a=0 可处于「空仓」（无挂单）
或「待开」（开仓单已发未成交）。
-/
inductive OrderPhase where
  | flat      -- 空仓（无持仓、无挂单）
  | pendingOpen   -- 待开（开仓单已发，未成交）
  | held      -- 持仓（已成交持有）
  | pendingClose  -- 待平（平仓单已发，未成交）
deriving Repr, DecidableEq

/--
单个声部的状态三元组 `(q_{v,t}, a_{v,t}, ω_{v,t})`（FULL §3 line 138/151-153）。

- `q : Nat`（`q_{v,t} ≥ 0`，FULL line 151：声部绝对单位数，非负——用 `Nat` 类型层强制非负）。
- `active : Bool`（`a_{v,t} ∈ {0,1}`，FULL line 152：声部是否激活）。
- `phase : OrderPhase`（`ω_{v,t}`，FULL line 153：订单状态四态）。

★对齐 §9 公理（line 520-525）：`a_{v,t}=1 ⟹ q_{v,t}=q_{p(v),t}`（精确同单位双开）由声部**树**
层的不变量约束（见 `VoiceForest.exactUnitMatch`），单个 `VoiceState` 只承载该声部自身三元组。
-/
structure VoiceState where
  q : Nat
  active : Bool
  phase : OrderPhase
deriving Repr, DecidableEq

/--
声部集合 `(·)_{v∈V}`（FULL §3 line 138：声部族）+ 有根有序树结构（§8 line 443-445：`𝒯=(V,p)`）。

★`voices : List VoiceState` 承载 §3 的「按 v∈V 索引的三元组族」；`parent : List (Option Nat)`
承载 §8 的父函数 `p`（节点 i 的父为 `parent[i]`，根的父为 `none`）——有限有根有序树由「节点列表 +
父索引列表」无环表示（FULL §8 line 443 `有限有根有序树`）。`rootIndex` 标记根声部在 `voices` 中
的位置（§10 根声部双向状态机的作用对象）。

★诚实标注：祖先闭合 `a_v ≤ a_{p(v)}`（§9 line 517）、精确同单位 `a_v=1⟹q_v=q_{p(v)}`（line 525）、
方向递归 `σ_v=σ_r(-1)^{d(v)}`（§8 line 466）是**树层一致性条件**（FULL §3 line 172「以及后面给出的
声部、方向和风险一致性条件」）——本 schema 承载树**结构**，一致性条件作为可分离的 `Prop` 谓词列出
（见 `VoiceForest.ancestorClosed` 等），不硬编进结构构造（避免把约束和数据耦死）。
-/
structure VoiceForest where
  voices : List VoiceState
  parent : List (Option Nat)
  rootIndex : Nat
deriving Repr

/--
声部树一致性条件 · 祖先闭合（FULL §9 line 516-518：`a_{v,t} ≤ a_{p(v),t}`）。

子声部激活蕴含父声部激活——可分离谓词，不耦进 `VoiceForest` 构造。
-/
def VoiceForest.ancestorClosed (F : VoiceForest) : Prop :=
  ∀ (i j : Nat) (vi vj : VoiceState),
    F.voices[i]? = some vi ->
    F.parent[i]? = some (some j) ->
    F.voices[j]? = some vj ->
    (vi.active = true -> vj.active = true)

/--
声部树一致性条件 · 精确同单位双开（FULL §9 line 522-525：`a_{v,t}=1 ⟹ q_{v,t}=q_{p(v),t}`）。

激活的子声部单位数等于父声部单位数——次级别短差是独立反向**同单位**双开（§9 公理）。
-/
def VoiceForest.exactUnitMatch (F : VoiceForest) : Prop :=
  ∀ (i j : Nat) (vi vj : VoiceState),
    F.voices[i]? = some vi ->
    F.parent[i]? = some (some j) ->
    F.voices[j]? = some vj ->
    (vi.active = true -> vi.q = vj.q)

/--
订单动作（`O_t` 未完成订单的动作维度，§3 line 162 + §20 line 1437 `T_Θ(x,O,e)` 中 `O` 的载体）。

★对齐 FullDefinitionStrategy `ActionClass`（FullDefinitionStrategy.lean:13）的语义但独立列出，
因为 `O_t` 是「已发未成交」的挂单（带方向+量），非已结算动作。开/平/加/减四类覆盖声部状态机
（§10 开平 + §11 §8 加减核心单位）。
-/
inductive OrderAction where
  | open_      -- 开仓
  | close_     -- 平仓
  | add_       -- 加（增核心单位，§11 line 879 新增单位）
  | reduce_    -- 减
deriving Repr, DecidableEq

/--
单个未完成订单 `Order'`（`O_t` 的元素，§3 line 162）。带 `'` 避免与 Origin `ChanlunElements` 潜在
同名冲突；这是状态侧「挂单簿」的条目（区别于 strategy 段输出的瞬时 order）。

- `action : OrderAction`（开/平/加/减）。
- `voiceIndex : Nat`（该订单归属的声部在 `VoiceForest.voices` 中的索引——订单总挂在某声部上）。
- `qty : Nat`（订单量，非负）。
- `submittedAt : Int`（提交时刻 t——挂单的因果时间戳）。
-/
structure Order' where
  action : OrderAction
  voiceIndex : Nat
  qty : Nat
  submittedAt : Int
deriving Repr, DecidableEq

/--
信号使用记录与结构记忆 `M_t`（FULL §3 line 163）。

★`Fresh` 谓词的状态载体：§10 开启谓词 `E_{v,t}` 含 `Fresh_{v,t}`（line 573）——「同一信号不重复
使用」要求记录已消费的信号。逐子分量：
- `consumedSignals : List Int`（已消费信号的标识序列——`Fresh_{v,t}` 检查信号是否已在此列表）。
- `lastBarSeen : Int`（结构记忆：最近处理的 bar 序号，单调，因果时间锚）。

★诚实标注：`M_t` 在 FULL 是「信号使用记录**和**结构记忆」——本 schema 承载二者的最小因果载体
（信号消费序列 + bar 时间锚）。具体「结构记忆」的内容（哪些中枢/背驰已记忆）随 §11 信号语义细化，
当前承载 Fresh 判据所需的消费记录（这是 §10 状态机真正读的部分）。
-/
structure SignalMemory where
  consumedSignals : List Int
  lastBarSeen : Int
deriving Repr, DecidableEq

/--
运行/经纪状态 `ν_t`（FULL §3 line 163：借券、保证金、交易场所及运行状态）。

★逐子分量显式化（§13 杠杆保证金完全分类 line 975+ 的状态侧载体）：
- `borrowable : Bool`（借券可行性——空头/反向双开需借券，§9 `F^{eq}` 含「借券规则成立」line 578）。
- `marginUsed : Int`（已占用保证金，§13 `IM_t`/`MM_t` 的当前占用值；整数域）。
- `venueOpen : Bool`（交易场所开放/可交易，§3「交易场所」+ §13 line 1098「交易场所规则成立」）。
- `running : Bool`（系统运行状态——halt/正常，对齐 Bar.untradable 的账户侧镜像）。

★诚实标注：`ν` 是 `RootSel`（§10 line 609-610 `RootSel_Θ : {0,1}²×D_t×ν_t → {-1,0,+1}`）与
`F^{eq}`（§9 line 578）的输入分量——根方向选择与双开可行性都读 `ν`。具体阈值（保证金率等）是
Θ_leverage 参数（FULL line 111），非缠论可导——本结构只承载状态，不含阈值。
-/
structure VenueState where
  borrowable : Bool
  marginUsed : Int
  venueOpen : Bool
  running : Bool
deriving Repr, DecidableEq

/-! ## §3 完整状态 x_t（17 分量逐分量组装，FULL line 133-144 boxed） -/

/--
完整状态 `x_t`（FULL §3 line 133-144 boxed 17 分量，逐分量显式，零摘要）。

分量对照（与 FULL line 137-143 的 `boxed` 表达式逐项对应）：

| # | FULL 符号 | 字段 | 类型 | FULL 行 |
|---|-----------|------|------|---------|
| 1 | `h_t` | `history` | `List Bar` | line 129/137 市场历史 `(y_0,…,y_t)` |
| 2 | `D_t` | `recStruct` | `ParseStruct` | line 137/149 所有级别缠论递归结构（Origin ParseStruct 承载，见模块头跨组依赖标注） |
| 3 | `σ_{r,t}` | `rootDir` | `RootDirection` | line 137/150 根方向 |
| 4 | `(q_v,a_v,ω_v)_{v∈V}` | `voices` | `VoiceForest` | line 138/151-153 声部三元组族 + 树 |
| 5 | `Φ_t` | `phase` | `CapitalPhase` | line 139/154 资本阶段（复用 Origin 5 态完全分类，§12 line 929） |
| 6 | `I_0` | `initialCapital` | `Int` | line 140/156 最初外部资本 |
| 7-10 | `Π_t,A_t,W_t,R_t` | `ledger` | `LedgerState` | line 140/157-160（`R=Π-A-W` 由 `LedgerState.inv` 强制） |
| 11 | `E_t` | `equity` | `Int` | line 141/161 清算权益 |
| 12 | `Cash_t` | `cash` | `Int` | line 141 现金 |
| 13 | `O_t` | `openOrders` | `List Order'` | line 142/162 未完成订单 |
| 14 | `M_t` | `memory` | `SignalMemory` | line 142/163 信号使用记录和结构记忆 |
| 15 | `ν_t` | `venue` | `VenueState` | line 143/164 借券/保证金/场所/运行状态 |

★分量计数说明：FULL boxed 表达式把 `(Π_t,A_t,W_t,R_t)` 列为四个独立符号（line 140），但 §3 line
166-170 的记账恒等 `R_t=Π_t−A_t−W_t` 使四者由「三独立 + 一导出」的 `LedgerState`（Origin canonical，
携 `inv : R=Π-A-W` 证明项）原子承载——故 17 个 FULL 符号 ↦ 14 个 Lean 字段（账本 4 符号 ↦ 1 字段），
记账恒等由 `LedgerState.inv` **类型层强制**（不可构造违反 R=Π-A-W 的 `CompleteState`）。这是 §3 line
166「合法状态必须满足记账恒等式」的严格 Lean 实现：非法状态在类型层不可表达。
-/
structure CompleteState where
  /-- 1. `h_t` 市场历史 `(y_0,…,y_t)`（FULL line 129/137）。 -/
  history : List Bar
  /-- 2. `D_t` 所有级别缠论递归结构（FULL line 137/149；Origin `ParseStruct` 承载，跨组依赖见模块头）。 -/
  recStruct : ParseStruct
  /-- 3. `σ_{r,t}` 根方向（FULL line 137/150）。 -/
  rootDir : RootDirection
  /-- 4. `(q_v,a_v,ω_v)_{v∈V}` 声部三元组族 + 有根有序树（FULL line 138/151-153 + §8 line 443）。 -/
  voices : VoiceForest
  /-- 5. `Φ_t` 资本阶段（FULL line 139/154；复用 Origin `CapitalPhase` 5 态完全分类，§12 line 929）。 -/
  phase : CapitalPhase
  /-- 6. `I_0` 最初外部资本（FULL line 140/156）。 -/
  initialCapital : Int
  /-- 7-10. `(Π_t,A_t,W_t,R_t)` 账本（FULL line 140/157-160）。`R=Π-A-W` 由 `LedgerState.inv` 类型层强制。 -/
  ledger : LedgerState
  /-- 11. `E_t` 清算权益（FULL line 141/161；§13 杠杆分母 `L^G=G/E`，line 1001）。 -/
  equity : Int
  /-- 12. `Cash_t` 现金（FULL line 141）。 -/
  cash : Int
  /-- 13. `O_t` 未完成订单（FULL line 142/162）。 -/
  openOrders : List Order'
  /-- 14. `M_t` 信号使用记录和结构记忆（FULL line 142/163）。 -/
  memory : SignalMemory
  /-- 15. `ν_t` 借券/保证金/场所/运行状态（FULL line 143/164）。 -/
  venue : VenueState

/--
合法状态记账恒等（FULL §3 line 166-170：`R_t = Π_t − A_t − W_t`）。

★这是 §3「合法状态必须满足记账恒等式」的**定理**——对**任意** `CompleteState`，其 `ledger` 分量
恒满足 `R=Π-A-W`。证明直接引 `LedgerState.inv`（Origin canonical 携带的证明项）：恒等在类型层已强制，
本定理是该强制性的可观测投影（非法账本状态根本无法构造进 `CompleteState.ledger`）。
-/
theorem complete_state_ledger_identity (x : CompleteState) :
    x.ledger.R = x.ledger.Pi - x.ledger.A - x.ledger.W :=
  x.ledger.inv

/-! ## §20 外部事件 e（8 元组逐分量组装，FULL line 1416-1429） -/

/--
成交回报 `Fill`（§20 line 1422）。`none` = 本步无成交；`some (price, qty)` = 成交价×量（整数域）。

★`T_Θ(x,O,e)` 的「成交作为外部事件输入以后，下一状态才唯一」（§20 line 1447）——`Fill` 是策略发单
`O` 后市场返回的实际成交，状态唯一性依赖它（策略只唯一决定订单，不决定成交）。
-/
structure Fill where
  price : Int
  qty : Int
deriving Repr, DecidableEq

/-- 拒单 `Reject`（§20 line 1423）：`rejected = true` 表示挂单被拒（含被拒订单标识）。 -/
structure Reject where
  rejected : Bool
  orderRef : Int
deriving Repr, DecidableEq

/-- 手续费 `Fee`（§20 line 1424）：本步产生的手续费（整数域，进账本 Π 的成本侧）。 -/
structure Fee where
  amount : Int
deriving Repr, DecidableEq

/-- 资金费 `Funding`（§20 line 1425）：持仓资金费（永续合约 funding，正负皆可）。 -/
structure Funding where
  amount : Int
deriving Repr, DecidableEq

/-- 保证金更新 `MarginUpdate`（§20 line 1426）：经纪侧保证金率/占用变化（更新 `ν.marginUsed`）。 -/
structure MarginUpdate where
  newMarginUsed : Int
deriving Repr, DecidableEq

/-- 借券更新 `BorrowUpdate`（§20 line 1427）：借券可得性/成本变化（更新 `ν.borrowable`）。 -/
structure BorrowUpdate where
  borrowable : Bool
  borrowCost : Int
deriving Repr, DecidableEq

/--
公司行为 `CorpAction`（§20 line 1428）：拆股/分红/合并等改变价格连续性与持仓数量的外部事件。

逐子分量：`splitRatio`（拆股比，分子/分母）、`dividend`（每单位分红）——`none`-等价用 `(1,1,0)` 表
「无行为」。
-/
structure CorpAction where
  splitNum : Int
  splitDen : Int
  dividend : Int
deriving Repr, DecidableEq

/--
外部事件 `e_{t+1}`（FULL §20 line 1416-1429 boxed 8 元组，逐分量显式，零摘要）。

分量对照（与 FULL line 1421-1428 逐项对应）：

| # | FULL 符号 | 字段 | 类型 | FULL 行 |
|---|-----------|------|------|---------|
| 1 | `y_{t+1}` | `bar` | `Bar` | line 1421 下一行情（K 线/价格） |
| 2 | `Fill_{t+1}` | `fill` | `Option Fill` | line 1422 成交回报 |
| 3 | `Reject_{t+1}` | `reject` | `Reject` | line 1423 拒单 |
| 4 | `Fee_{t+1}` | `fee` | `Fee` | line 1424 手续费 |
| 5 | `Funding_{t+1}` | `funding` | `Funding` | line 1425 资金费 |
| 6 | `MarginUpdate_{t+1}` | `marginUpdate` | `MarginUpdate` | line 1426 保证金变化 |
| 7 | `BorrowUpdate_{t+1}` | `borrowUpdate` | `BorrowUpdate` | line 1427 借券变化 |
| 8 | `CorpAction_{t+1}` | `corpAction` | `CorpAction` | line 1428 公司行为 |

★`fill` 用 `Option`（§20 line 1447：策略发单后**成交未知**，可能无成交）；其余七元组每步都有值
（行情/拒单/费用等是每步外部输入，无值时取「零」实例如 `Fee.mk 0`）。
-/
structure ExternalEvent where
  /-- 1. `y_{t+1}` 下一行情（FULL line 1421）。 -/
  bar : Bar
  /-- 2. `Fill_{t+1}` 成交回报（FULL line 1422；`none`=无成交，成交决定下一状态唯一性，line 1447）。 -/
  fill : Option Fill
  /-- 3. `Reject_{t+1}` 拒单（FULL line 1423）。 -/
  reject : Reject
  /-- 4. `Fee_{t+1}` 手续费（FULL line 1424）。 -/
  fee : Fee
  /-- 5. `Funding_{t+1}` 资金费（FULL line 1425）。 -/
  funding : Funding
  /-- 6. `MarginUpdate_{t+1}` 保证金变化（FULL line 1426）。 -/
  marginUpdate : MarginUpdate
  /-- 7. `BorrowUpdate_{t+1}` 借券变化（FULL line 1427）。 -/
  borrowUpdate : BorrowUpdate
  /-- 8. `CorpAction_{t+1}` 公司行为（FULL line 1428）。 -/
  corpAction : CorpAction

/-! ## §20 转移签名 `T_Θ(x_t, O_{t+1}, e_{t+1})`（FULL line 1433-1445）

转移把（完整状态 × 未完成订单 × 外部事件）映到下一完整状态。FULL line 1442-1445 要求
`∀ x,O,e, ∃! x' = T_Θ(x,O,e)`（存在唯一）——本 schema 给出转移的**类型签名**作为接口；具体
转移函数的实例化（含唯一性证明）由闭环工位（`FullDefinitionStrategy.hybridStep` 已对摘要态
`StrictState` 证 `hybrid_step_complete_unique`）在完整态上承载。-/

/--
转移接口 `TransitionTheta`（§20 line 1433-1438：`x_{t+1} = T_Θ(x_t, O_{t+1}, e_{t+1})`）。

★这是**接口签名**（函数类型），不是某个具体 Θ 的转移。`O : List Order'` 是策略发的未完成订单
（§20 line 1447「策略唯一决定的是订单」），`e : ExternalEvent` 是市场返回的外部事件（含 `Fill`），
二者一起把 `x` 映到唯一的 `x'`。
-/
def TransitionTheta : Type :=
  CompleteState -> List Order' -> ExternalEvent -> CompleteState

/--
转移存在唯一性（§20 line 1442-1445：`∀ x,O,e, ∃! x' = T_Θ(x,O,e)`）。

★对**任意**转移函数 `T : TransitionTheta` 与任意输入，下一状态存在且唯一——这对函数是平凡真
（函数求值的确定性），但显式陈述把 §20 的唯一性要求锚到本 schema 的转移接口上：任何实例化的 `T`
都自动满足 §20 line 1442 的 `∃!`。与 `FullDefinitionStrategy.hybrid_step_complete_unique`（摘要态
上的同型定理）平行——此处是完整态版本。
-/
theorem transition_exists_unique (T : TransitionTheta)
    (x : CompleteState) (O : List Order') (e : ExternalEvent) :
    ExistsUnique (fun x' => T x O e = x') := by
  refine ⟨T x O e, rfl, ?_⟩
  intro y hy
  exact hy.symm

end NewChanlun.Origin.CompleteStateEvent
