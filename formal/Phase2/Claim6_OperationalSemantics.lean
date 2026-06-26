/-
  操作语义完全分类（Phase 2 claim6，task #29）—— 操作【类型】层完全分类
  ===========================================================================

  对象（lead task #29）：T 算子操作 + 三类买卖点操作（#39 操作商）。
  操作 = 从已解析的端点操作信号到操作类型 {买/卖/加/减/持/平} 的全函数。

  ★本模块与已有 `Formal/OperationalSemantics.lean`（605 谱系，codex R2 PASS，machine-checked）
    的分工（不重复、不碰 Formal.lean/lakefile.toml）：

    | 模块 | 形式化的层 | 来源 |
    |------|-----------|------|
    | Formal/OperationalSemantics.lean | 几何**原子动作** Σ={e,h⁺,h⁻,τ}（第0层句法 path）+ 净效果商 | operation_route_exhaustion §2 Σ-exh |
    | **本模块** | 缠论**操作类型** {买/卖/加/减/持/平}（π_op 层）的构造子穷尽 + totality 全函数 | operation_route_exhaustion §4.1 π_op 表 + #39 操作商 + 267/338 三阶段 |

    两者是操作语义 D∞ 三层结构里**不同的层**：Σ 是几何生成元（h/τ），操作类型是 π_op 投影
    （direction 转移聚合为缠论动作 F/C/E/D/A/hold）。本模块**复用** Formal.BSPLabels.BSPLabelSet
    作为操作触发端点（import Formal.BSPLabels），不重定义 BSP。

  ★★严格性裁定（no-workaround / no-patch-mentality / formalization-validity-domain）：

    lead task 字面要求"操作 = 从端点 BSP 标签集到操作类型的**全函数**"。但
    operation_route_exhaustion §0.1（句法存在性 ≠ 路线合法性）+ §4.1（π_op 表）已证：
    操作类型不由 BSP 端点标签集**单独**决定——同一 BSP 端点（如某级别 1B），
    在空仓时是「建仓(买)」、在已持仓降成本阶段是「加仓」、在反手语境是「平+反手」。
    即操作类型 = f(操作触发**信号**, **当前持仓方向**)，不是 f(BSP 端点)。

    605 谱系记录：codex 真 session 019eff21 R1 **FAIL** 判定「操作完全分类 = BSP totality
    的 composition」为**有效域膨胀**（090 声明膨胀）。

    ★本模块经 codex 真 session 019effa4-1aa5-7e82-8d1e-23903cb67349（gpt-5.5 high）二次审计——
    第一版用 `decideOp : BSPLabelSet × PositionState → OpType`，codex 判 **FAIL**：
    `BSPLabelSet` 定义域太宽（允许多级别/多类型/**双侧共振**标签），用"买侧优先"硬压成单一
    操作类型 = **补丁思维**（把未裁决的共振冲突硬编码为默认）。承认共振未裁决 ⟹ 不能同时声称
    `BSPLabelSet × PositionState` 是忠实定义域。

    ★本版（重写，非打补丁，no-workaround）按 codex 修复方向**收窄定义域**到忠实有效域：
    - 先定义 `ResolvedOpSignal`：**已解析**的操作触发信号（含**唯一主导 side**）。
    - `decideOp : ResolvedOpSignal → PositionState → OpType` 全函数（totality 严格 + 忠实，无补丁）。
    - "如何从 `BSPLabelSet` 解析出唯一主导 side"（双侧共振裁决 / 级别主导 / 区间套定级别）
      **显式标注为 claim7（606 背驰区间套）/ Phase2+ 引擎层职责**（脱钩点，605 §张力检查已记录），
      **不**在本模块用"买侧优先"偷偷裁决。
    - 提供忠实桥接定理：**单侧**标签集（无共振）可无歧义解析为 `ResolvedOpSignal`（resolveSingleSided）。

    ★L0/L3 分层（lead task 强约束）：本模块只形式化操作**类型**（L0，由解析后的触发 side +
    持仓方向钉死，第31课成本阶段不入此层）。操作**数量**（加/减多少仓，依成本阶段=L3 payoff，
    603 §诚实分层 + 597 sub11）**不在本模块**——OpType 只给"加/减"的**类型**，不给数量系数 M。
    L0 命题里没有任何 L3 数量变量。（codex 019effa4 确认 L0/L3 分层 PASS。）

  认识论：L0（操作类型构造子穷尽继承 operation_route_exhaustion §4 dual-exh 定理 +
    #39 操作商 + 缠论买卖点完备性 maimai:48；全函数 totality 由 Lean 函数定义强制）。
    有效域 = 操作**类型**层 × **已解析触发信号**（**非** BSPLabelSet 原始多标签集，
    **非**操作数量层，**非**完整 LegalRoute 含守恒/级别塔/共振裁决）。
-/

import Formal.BSPLabels

namespace Formal.Phase2.Claim6

open Formal.BSPLabels
open Formal.BSPLabels.BSPType

/-! ## 一、操作类型 OpType（缠论操作【类型】穷尽，π_op 层 L0）

  缠论单 bar 单级别的操作类型，由 operation_route_exhaustion §4.1 π_op 表 +
  267/338 三阶段操作 + #39 三类买卖点操作 穷尽为 **6 个构造子**：

  | OpType | §4.1 π_op / §7.1 缠论操作 | 267/338 三阶段 | maimai 操作 |
  |--------|---------------------------|----------------|-------------|
  | `buy`    | F 建仓（open long，0→+1）       | 阶段1 建仓（一次性满仓满融进入） | 买点→建仓 |
  | `sell`   | C 翻转 / A 强平（→0 / +1→−1）   | 阶段3 退出本金 / 止损             | 卖点→清仓 |
  | `add`    | D④ 回补升回（+f 升回核心仓）    | 阶段2 降成本·买点加回            | 买点→加回 |
  | `reduce` | E① 减仓下沉（−f 流出本级别）    | 阶段2 降成本·卖点减仓            | 卖点→减仓 |
  | `hold`   | hold / 持仓推进（d→d，0）        | 持仓不动（主级别持仓不动）       | 无操作信号→持有 |
  | `close`  | D③ 平空 / β⁻¹（−1→0 平掉反向腿）| 次级别短差平腿                  | 反向腿了结 |

  穷尽性（§4 dual-exh）：操作投影 × 会计投影的合法组合恰 8 个，聚合到 direction 动作层
  为这 6 类（E①②/D③④ 的 H₁ 四步闭路在 direction 投影下聚合为 add/reduce/close）。
  **无第 7 类操作类型**——由 direction 转移 {long,flat,short} 的 3×3=9 转移经守恒律
  （T34/T40）筛选 + 缠论动作聚合穷尽。这是**内涵式构造子穷尽**（603 范式），非外延式轴枚举。
-/

/--
  ★操作类型 OpType（6 构造子穷尽，L0）。

  对应 lead task 的 {买/卖/加/减/持/平}：buy=买, sell=卖, add=加, reduce=减, hold=持, close=平。
  （maimai 的"平"在缠论里区分两种：sell=主仓清仓/止损（C/A），close=次级别反向腿了结（D③）。
   lead 的"卖/平"在 §4.1 分别对应 C翻转-A强平 与 D③平空——本模块忠实保留这个区分，
   不把两个不同的 π_op 动作硬塞进一个构造子。）
-/
inductive OpType where
  | buy      -- 买：F 建仓（open long，§4.1 #2）
  | sell     -- 卖：C 翻转 / A 强平（清主仓，§4.1 #3/#8）
  | add      -- 加：D④ 回补升回（核心仓 +f，§4.1 #7，降成本买回）
  | reduce   -- 减：E① 减仓下沉（本级别 −f，§4.1 #4，降成本卖减）
  | hold     -- 持：hold / 持仓推进（d→d，§4.1 #1）
  | close    -- 平：D③ 平空（平反向腿 −1→0，§4.1 #6）
deriving DecidableEq, Repr

open OpType

/--
  ★操作类型完全分类（构造子穷尽，L0）：任意操作类型必属 6 构造子之一（无第 7 类）。
  这是对 `OpType` 的结构归纳——"无第 7 个操作类型"是 inductive 的结构归纳原理，
  继承 §4 dual-exh（合法组合穷尽）+ direction 动作聚合。
-/
theorem optype_exhaustive (op : OpType) :
    op = buy ∨ op = sell ∨ op = add ∨ op = reduce ∨ op = hold ∨ op = close := by
  cases op
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl))))

/-! ## 二、持仓状态 PositionState（操作类型的第二参数，L0 方向层）

  ★关键（§0.1 句法存在性 ≠ 路线合法性 + §4.1 π_op 表）：操作类型不由触发信号单独决定，
  还依赖**当前持仓方向** d ∈ {flat, long, short}（operation_route_exhaustion §2.1 状态
  (φ,ε,q) 的 ε 手性 + direction）。

  ★L0/L3 分层：这里只取持仓**方向**（flat/long/short，L0 离散三态），**不取数量** q
  （L3 payoff，第31课成本阶段）。方向决定操作**类型**（建/加/减/平/反手），数量决定操作
  **数量**（不在本模块）。（codex 019effa4 确认：PositionState 无数量字段，分层干净。）
-/

/-- 持仓方向（L0 离散三态，不含数量 q——数量是 L3 payoff）。 -/
inductive PositionState where
  | flat    -- 空仓（d=0）
  | long    -- 持多（d=+1）
  | short   -- 持空（d=−1）
deriving DecidableEq, Repr

open PositionState

/-! ## 三、已解析操作触发信号 ResolvedOpSignal（codex 019effa4 修复方向：收窄定义域）

  ★codex 019effa4 否定（成立）：原始 `BSPLabelSet` 作为 `decideOp` 定义域**太宽**——
  它允许多级别、多类型（type1/2/3）、**双侧共振**（同时含买卖侧标签，maimai:160）的标签集。
  把这些**未裁决**状态用"买侧优先"硬压成单一操作类型是**补丁思维**（no-patch-mentality）。

  ★修复（非打补丁，收窄到忠实有效域）：操作类型决策的忠实定义域是**已解析**的触发信号——
  携带**唯一主导 side**（买/卖方向已裁决）。`ResolvedOpSignal` 即此类型。

  "如何从原始 `BSPLabelSet` 解析出唯一主导 side"——即双侧共振裁决（哪级别买卖点主导）、
  级别主导、区间套定级别——是 **claim7（606 背驰区间套）/ groupoid 轴 / Phase2+ 引擎层职责**
  （605 §张力检查·脱钩点已记录）。本模块**不**承担这个解析，只承担"解析后→操作类型"的全函数。
-/

/--
  ★已解析操作触发信号（唯一主导 side 已裁决，L0）。
  `domSide`：经共振裁决/级别主导/区间套定级别后得到的**唯一**操作触发方向（买或卖）。
  （type1/2/3 与 level 的区别在操作**类型**层不改变 direction 转移聚合——
   1B/2B/3B 买侧都触发"买方向"动作类，差异在数量/时机=L3/groupoid，不在本 L0 类型层；
   故 ResolvedOpSignal 在类型层只需 domSide。这是忠实的：§4.1 π_op 聚合只看 direction。）
-/
structure ResolvedOpSignal where
  domSide : Side
deriving DecidableEq, Repr

/-! ## 四、操作类型决策全函数 decideOp（totality + 忠实，本模块核心）

  ★lead task 的"全函数（totality）"——忠实精确化为
  `decideOp : ResolvedOpSignal → PositionState → OpType`（定义域已收窄，无共振补丁）。

  这是 §4.1 π_op 表的**直接编码**：给定已解析的触发方向 + 当前持仓方向，唯一确定操作类型。

  全函数性（totality）由 Lean 函数定义强制——`decideOp` 对**任意** `(sig, pos)`
  返回良定义 `OpType`，无 partial、无 Option、无 sorry。

  决策矩阵（§4.1 π_op + 267/338 三阶段，L0 类型层）：

  | 主导 side | 持仓 flat | 持仓 long | 持仓 short |
  |----------|-----------|-----------|------------|
  | buy（买侧）| buy（F建仓） | add（D④降成本加回） | close（D③平空腿） |
  | sell（卖侧）| hold（空仓无可卖，§4.4 裸空非缠论） | reduce（E①减仓下沉） | sell（C翻转/止损） |

  读法（267/338）：
  - 买侧 + flat = 阶段1 建仓（buy）。
  - 买侧 + long = 阶段2 降成本买点加回（add，§4.1 D④ +f）。
  - 买侧 + short = 平掉空头反向腿（close，§4.1 D③）。
  - 卖侧 + long = 阶段2 降成本卖点减仓（reduce，§4.1 E① −f）。
  - 卖侧 + short = 空头方向的 C 翻转/止损（sell）。
  - 卖侧 + flat = 空仓遇卖侧信号无主仓可减、裸做空非缠论操作（§4.4）⟹ hold（不动）。
-/

/--
  ★操作类型决策全函数（totality + 忠实，L0）。

  定义域 = `ResolvedOpSignal × PositionState`（**非** `BSPLabelSet`——后者太宽含未裁决共振，
  codex 019effa4 FAIL）。值域 = `OpType`（6 构造子）。返回类型是 `OpType` 而非 `Option OpType`
  ⟹ totality 强制；定义域已收窄到唯一主导 side ⟹ 无"买侧优先"补丁，忠实。
-/
def decideOp (sig : ResolvedOpSignal) (pos : PositionState) : OpType :=
  match sig.domSide, pos with
  | Side.buy,  flat  => buy      -- 阶段1 建仓
  | Side.buy,  long  => add      -- 阶段2 降成本买点加回（D④）
  | Side.buy,  short => close    -- 平空头反向腿（D③）
  | Side.sell, flat  => hold     -- 空仓遇卖侧无主仓可减、裸空非缠论（§4.4）⟹ 不动
  | Side.sell, long  => reduce   -- 阶段2 降成本卖点减仓（E①）
  | Side.sell, short => sell     -- 空头 C 翻转 / 止损

/--
  ★全函数 totality（L0）：对任意已解析触发信号 + 任意持仓状态，
  `decideOp` 返回 6 个良定义操作类型之一（操作类型穷尽 + 函数全定义）。

  这是 lead task "完全性=操作类型穷尽（machine-checked）" 的精确兑现。
-/
theorem decideOp_total (sig : ResolvedOpSignal) (pos : PositionState) :
    decideOp sig pos = buy ∨ decideOp sig pos = sell ∨ decideOp sig pos = add ∨
    decideOp sig pos = reduce ∨ decideOp sig pos = hold ∨ decideOp sig pos = close :=
  optype_exhaustive (decideOp sig pos)

/-! ## 五、决策矩阵的钉死性（每个 (主导 side, 持仓) 唯一操作类型，L0）

  下列定理逐格钉死 §4.1 π_op 表——证明 decideOp 是 §4.1 表的精确实现（共 6 格，全覆盖）。
  （现在用 `rfl` 即可——定义域已是唯一主导 side，无 hasBuySide 分支补丁。）
-/

/-- 买侧 + 空仓 = 建仓（阶段1，F，buy）。 -/
theorem buy_flat_builds (sig : ResolvedOpSignal) (h : sig.domSide = Side.buy) :
    decideOp sig flat = buy := by simp [decideOp, h]

/-- 买侧 + 持多 = 降成本加回（阶段2，D④，add）。 -/
theorem buy_long_adds (sig : ResolvedOpSignal) (h : sig.domSide = Side.buy) :
    decideOp sig long = add := by simp [decideOp, h]

/-- 买侧 + 持空 = 平空腿（D③，close）。 -/
theorem buy_short_closes (sig : ResolvedOpSignal) (h : sig.domSide = Side.buy) :
    decideOp sig short = close := by simp [decideOp, h]

/-- 卖侧 + 持多 = 降成本减仓（阶段2，E①，reduce）。 -/
theorem sell_long_reduces (sig : ResolvedOpSignal) (h : sig.domSide = Side.sell) :
    decideOp sig long = reduce := by simp [decideOp, h]

/-- 卖侧 + 持空 = 翻转/止损（C/A，sell）。 -/
theorem sell_short_sells (sig : ResolvedOpSignal) (h : sig.domSide = Side.sell) :
    decideOp sig short = sell := by simp [decideOp, h]

/-- 卖侧 + 空仓 = 不动（裸空非缠论 §4.4，hold）。 -/
theorem sell_flat_holds (sig : ResolvedOpSignal) (h : sig.domSide = Side.sell) :
    decideOp sig flat = hold := by simp [decideOp, h]

/-! ## 六、忠实桥接：单侧标签集可无歧义解析（无共振时 BSPLabelSet → ResolvedOpSignal，L0）

  ★这是把 `BSPLabelSet`（原始端点）与 `ResolvedOpSignal`（已解析）连接的**忠实**定理——
  **不**承担双侧共振裁决（那是 claim7/Phase2+），只声明：当标签集**单侧**（无共振）时，
  主导 side 由该唯一侧无歧义确定。这是 totality 在"无共振"子域上的落地（升跌完备性 010 +
  单侧前提 ⟹ 唯一主导 side）。
-/

/-- 标签集含某买侧 BSP。 -/
def hasBuySide (s : BSPLabelSet) : Bool :=
  s.labels.any (fun l => l.side = Side.buy)

/-- 标签集含某卖侧 BSP。 -/
def hasSellSide (s : BSPLabelSet) : Bool :=
  s.labels.any (fun l => l.side = Side.sell)

/--
  ★操作触发方向 totality（非空标签集 ⟹ 至少一侧触发，L0，继承 BSP totality 010）。
  端点标签集非空（s.nonempty，升跌完备性 010）⟹ 第一个标签的 side 必是 buy 或 sell
  ⟹ hasBuySide 或 hasSellSide 至少一真（无"非空却无方向"的死端点）。
-/
theorem trigger_side_total (s : BSPLabelSet) :
    hasBuySide s = true ∨ hasSellSide s = true := by
  cases h : s.labels with
  | nil => exact absurd h s.nonempty
  | cons l₀ rest =>
    cases hside : l₀.side with
    | buy =>
      left
      simp only [hasBuySide, h, List.any_cons]
      simp [hside]
    | sell =>
      right
      simp only [hasSellSide, h, List.any_cons]
      simp [hside]

/--
  ★单侧解析（无共振，L0）：若标签集**仅买侧**（有买侧、无卖侧），
  则无歧义解析为主导买侧的 ResolvedOpSignal。
  这是忠实桥接——单侧时解析唯一，不需要任何"优先"裁决（无补丁）。
-/
def resolveSingleSidedBuy (s : BSPLabelSet)
    (_hb : hasBuySide s = true) (_hs : hasSellSide s = false) : ResolvedOpSignal :=
  ⟨Side.buy⟩

/--
  ★单侧解析（无共振，L0）：若标签集**仅卖侧**，无歧义解析为主导卖侧的 ResolvedOpSignal。
-/
def resolveSingleSidedSell (s : BSPLabelSet)
    (_hb : hasBuySide s = false) (_hs : hasSellSide s = true) : ResolvedOpSignal :=
  ⟨Side.sell⟩

/-- ★单侧买解析的主导 side 忠实为 buy（L0）。 -/
theorem resolveSingleSidedBuy_domSide (s : BSPLabelSet)
    (hb : hasBuySide s = true) (hs : hasSellSide s = false) :
    (resolveSingleSidedBuy s hb hs).domSide = Side.buy := rfl

/-- ★单侧卖解析的主导 side 忠实为 sell（L0）。 -/
theorem resolveSingleSidedSell_domSide (s : BSPLabelSet)
    (hb : hasBuySide s = false) (hs : hasSellSide s = true) :
    (resolveSingleSidedSell s hb hs).domSide = Side.sell := rfl

/-! ## 七、★有效域诚实标注（formalization-validity-domain + no-patch-mentality + codex 019effa4）

  本模块**忠实交付**的（L0，machine-checked，codex 019effa4 二次审计后重写）：
  1. 操作**类型** OpType 6 构造子穷尽（optype_exhaustive，无第 7 类）。
  2. 操作类型决策全函数 `decideOp : ResolvedOpSignal → PositionState → OpType`
     （totality 严格 + 忠实，decideOp_total）；定义域是**已解析唯一主导 side**（无共振补丁）。
  3. §4.1 π_op 决策矩阵逐格钉死（buy_flat_builds … sell_flat_holds，6 格全覆盖）。
  4. 操作触发方向 totality（trigger_side_total，继承 BSP totality 010）。
  5. 单侧（无共振）忠实桥接 BSPLabelSet → ResolvedOpSignal（resolveSingleSided*）。

  本模块**不声明**（有效域边界，诚实标注，避免膨胀；codex 019eff21 R1 + 019effa4 双重把关）：
  - ✗ "操作类型 = BSPLabelSet **直接**全函数"——codex 019eff21 R1 + 019effa4 FAIL 的膨胀。
       本模块定义域是 `ResolvedOpSignal`（已裁决主导 side），不是原始多标签集。
  - ✗ **双侧共振裁决**（同含买卖侧时哪级别/类型主导）——本模块**不**用"买侧优先"硬压
       （那是 codex 019effa4 否决的补丁）。共振裁决 = claim7（606 背驰区间套）/ groupoid 轴 /
       区间套定级别 / Phase2+ 引擎层职责（605 §张力检查·脱钩点）。本模块只提供单侧无歧义桥接。
  - ✗ 操作**数量**（加/减多少仓）——L3 payoff，第31课成本阶段（603 §诚实分层 + 597 sub11）。
       OpType 的 add/reduce 只是**类型**，数量系数 M=f(成本阶段) 不在本模块（codex 019effa4 确认
       L0/L3 分层 PASS，L0 命题无 L3 变量）。
  - ✗ type1/2/3 与 level 在操作**类型**层不改变 direction 聚合（1B/2B/3B 买侧都触发买方向类），
       其差异在数量/时机（L3/groupoid），故 ResolvedOpSignal 类型层只需 domSide——这是忠实的
       （§4.1 π_op 聚合只看 direction），不是遗漏。
  - ✗ 完整合法 operate route（含手性 {±1} / σ 级别塔 / T34/T40/T48 守恒约束）——
       operation_route_exhaustion §7 总式 = Σ*×{±1}×σ-tower×M，是 Phase2+/Rust 引擎层职责。

  ★与 Formal/OperationalSemantics.lean（605）的关系：该模块形式化几何原子 Σ + 净效果商
    （第0层 path / 第1层 群元素）；本模块形式化操作**类型** π_op（缠论动作层）。两层不冲突——
    Σ 是"怎么走"的几何生成元，OpType 是"做什么操作"的缠论语义投影。本模块复用
    BSPLabels.BSPLabelSet（一致引用，无符号碰撞），是 605 弱声明 trigger 的**类型层补全**
    （605 给"是否触发 Bool"，本模块给"已解析触发 + 持仓 → 操作类型 OpType"全函数）。
-/

/-!
  ## ★C5 两层定理收口（task #67 T-op，严格分类标准 §A codex#1 + §B 第6部分）

  本模块的 `decideOp : ResolvedOpSignal → PositionState → OpType` 是标准第6部分的 **完全应对 π**
  的类型层：全函数（`decideOp_total`）+ 逐格钉死 §4.1 π_op 表（6 格全覆盖）。下面收口两层定理：
  - **Layer1（标签商，本模块已有）**：`OpType` 6 构造子穷尽（`optype_exhaustive`）= 按操作类型
    **标签商分类**（codex#1 规则：显式命名，不冒充语义双射）。
  - **Layer2（语义双射失败，诚实刻画）**：操作类型标签 **不决定** 完整操作语义（数量 M / 价格 /
    账户转移效果）——complete 失败的语义反例在 `Formal/OperationalSemantics.lean`（`i_not_complete`：
    买1单位 vs 买10单位 同 OpType 异账户转移）。本模块诚实标注 OpType = 标签商，非 `Exec/∼ ≅ OpType`。
  - **§6 无前视·弱形式**（codex 019f001b 诚实标注）：`decideOp` 是 (已解析触发, 持仓) 的纯函数——
    同参同值。**有效域更窄**：`ResolvedOpSignal × PositionState`（已解析买/卖 side × 3 持仓 = 2×3，
    **不覆盖** SignalSide.none），故 decideOp 是"已解析触发信号域上的 π_op 类型层"，**不**单独冒充
    完整 3×3 π（完整 π 见 `Formal/OperationalSemantics.lean` pi）。完整无前视因果（历史模型/前缀）
    属 T-causal #64/#66 工位。
-/

/--
  ★C5 §6 decideOp 无前视·弱形式（L0，codex 019f001b 诚实降级）：decideOp 是当前 (sig, pos)
  的纯函数——相同当前状态 ⟹ 相同应对（同参同值）。**弱形式**：前提"(sig,pos) = 当前及之前
  数据摘要"下表达应对只由当前决定；完整无前视因果（历史/前缀依赖）属 T-causal #64/#66，
  不在本模块（诚实标注不冒充强因果）。
-/
theorem decideOp_no_lookahead (sig sig' : ResolvedOpSignal) (pos pos' : PositionState)
    (hsig : sig = sig') (hpos : pos = pos') : decideOp sig pos = decideOp sig' pos' := by
  rw [hsig, hpos]

/--
  ★C5 §6 decideOp 应对穷尽于 OpType（L0）：π 的值域落在 6 操作类型内（完全应对类型完全分类）。
  配合 `decideOp_total`（全函数）⟹ decideOp 是完全应对 π（∀状态有应对，应对类型穷尽）。
-/
theorem decideOp_in_optype (sig : ResolvedOpSignal) (pos : PositionState) :
    decideOp sig pos = buy ∨ decideOp sig pos = sell ∨ decideOp sig pos = add
    ∨ decideOp sig pos = reduce ∨ decideOp sig pos = hold ∨ decideOp sig pos = close :=
  optype_exhaustive (decideOp sig pos)

end Formal.Phase2.Claim6
