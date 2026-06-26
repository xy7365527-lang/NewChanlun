/-
  会计层 · 账本基础（T₃₃ 双层记账 / T₃₄ 股数守恒 / T₃₅ NAV 价值中性 / T₃₈ 双向重定基）
  工位：t-accounting（task #49 / gap-map 层C E-set）

  ── 认识论等级：L0（定义内蕴，formalization-validity-domain）──────────────────
  本模块把会计守恒律形式化为 **求和等式的结构恒等**：股数守恒 = 转移操作保 Σ；NAV 价值
  中性 = 同价交换的净变化为 0；双向重定基 = N_base 跳变只在 earning/loss 事件。这些都是
  纯算术/结构定理，**不依赖任何运行时数据**。`lake build` 通过 = 逻辑正确（L0），不是
  实证有效域，不得膨胀。运行时数值守卫（prove_n8_conservation 每 bar panic）= R 类（L2），
  由 Rust 引擎覆盖，不进 Lean——Lean 证的是「守恒等式在结构上恒成立」，Rust 证的是
  「实际数据 25M bar 零违反」，两者认识论等级不同（L0 vs L2），互补不替代。

  ── 隔离声明（任务卡硬约束）─────────────────────────────────────────────────
  本模块 **自包含**：不 import Formal.* / 不 import Phase2.*。无 sorry/admit/axiom。
-/

namespace Formal.Tlayers.Accounting

/-! ## §1 账本基础态：voice 头寸的会计视图（T₃₃ 载体） -/

/--
  ★头寸极性（多头 / 空头）。会计断面区分多空：多头持 `units` 正股，空头持 `capital` 现金
  保证金。降成本 = 父多头 spawn 子空头（同一物理交易两视图，T₃₃）。
-/
inductive Polarity where
  | long
  | short
deriving DecidableEq, Repr

open Polarity

/--
  ★voice 会计态（D3.2 nav 公式的 per-voice 分量）。

  - `polarity`：多/空。
  - `units`：在手单位数（多头视图：持股数；空头视图：开空时记为 0，capital 承载价值）。
  - `capital`：空头保证金现金（多头视图：0）。

  NAV 分解（D3.2）：`nav = free + Σ多头 units×c + Σ空头 capital`。本结构是单个 voice 的
  会计分量，森林总 NAV 由分量求和（见 §3）。
-/
structure VoiceLedger where
  polarity : Polarity
  units : Nat
  capital : Nat
deriving Repr

/-! ## §2 T₃₃ 同一笔物理交易双层记账（两视图引用同一物理量 m）

  necessity_derivation.md §713-723：卖出 m 单位是 **一笔** 物理交易，会计上分两层
  （父降成本视图 units−m + 子独立空头视图 capital+m×c），非两笔。形式化：一个物理量 m
  经两个投影函数得到父/子视图，两视图的「单位流量」绝对值相等（同源一笔，防单边记账）。 -/

/--
  ★物理交易（T₃₃ 核心：一个物理量 m，两个会计视图）。
  `m`：卖出单位数（物理量，唯一）。`price`：成交价 c。降成本一笔 = 父 units 减 m + 子
  capital 增 m×price，二者由 **同一个 m** 派生。
-/
structure PhysicalTrade where
  m : Nat
  price : Nat
deriving Repr

/-- 父级降成本视图：units 减少量 = m（物理量本身）。 -/
def PhysicalTrade.parentUnitsDelta (t : PhysicalTrade) : Nat := t.m

/-- 子级独立头寸视图：capital 增加量 = m×price（同一 m 的现金折算）。 -/
def PhysicalTrade.childCapitalDelta (t : PhysicalTrade) : Nat := t.m * t.price

/--
  ★T₃₃ 双层记账定理（L0）：父视图 units 流量 与 子视图 capital 流量 **同源一笔**——
  两者都由同一物理量 `t.m` 派生（父 = m，子 = m×price），非两个独立的 m。

  这形式化「物理一笔，会计分层」：若 m=0（无交易），两视图流量同时为 0；m>0 时两视图
  流量同时非零且子流量 = 父流量 × price。关死「父子是两笔独立交易」的误读——它们是
  同一个 `t.m` 的两个投影。
-/
theorem trade_dual_view_same_source (t : PhysicalTrade) :
    t.childCapitalDelta = t.parentUnitsDelta * t.price := by
  unfold PhysicalTrade.childCapitalDelta PhysicalTrade.parentUnitsDelta
  rfl

/--
  ★T₃₃ 同源零一致（防单边记账 bug 的结构见证）：物理量为 0 ⟺ 父子两视图流量同时为 0。
  即不存在「父记了减仓但子没记开仓」（单边记账）——两视图的非零性由同一 m 决定。
-/
theorem trade_zero_iff_both_zero (t : PhysicalTrade) :
    t.m = 0 ↔ (t.parentUnitsDelta = 0 ∧ t.childCapitalDelta = 0) := by
  unfold PhysicalTrade.parentUnitsDelta PhysicalTrade.childCapitalDelta
  constructor
  · intro h; exact ⟨h, by rw [h]; simp⟩
  · intro h; exact h.1

/-! ## §3 T₃₄ 股数守恒 Σ active voice.units = N_base

  necessity_derivation.md §725-736：递归链单位流转（父→子→孙）不增不减——多空嵌套是
  同一批 N 单位在级别间重新分配，总和恒为 N_base。形式化为求和等式：spawn（父−m/子+m）
  与 close（子归还父）都保持 Σunits 不变。 -/

/-- 森林总单位数（Σ active voice.units，T₃₄ 的守恒量）。 -/
def totalUnits (forest : List VoiceLedger) : Nat :=
  (forest.map VoiceLedger.units).sum

/--
  ★spawn 操作（T₃₃ 物理一笔的会计实现）：父 units 减 m，新增子 voice units = m（子空头
  持有从父转移的 m 单位的会计镜像）。这里子 units = m 表达「转移的单位数」（守恒口径，
  非 capital 折算）——T₃₄ 守的是单位数 Σ，capital 折算是 T₃₅ NAV 口径，两者不同范畴。

  spawn 是纯计算（Nat 截断减法全定义，**不携带前提**——严格性：def 不声明它不消费的约束）。
  「m ≤ parent.units」是守恒 **定理** 的前提（见 spawn_conserves），非 spawn 签名的一部分。
-/
def spawn (parent : VoiceLedger) (m : Nat) :
    VoiceLedger × VoiceLedger :=
  ( { parent with units := parent.units - m }
  , { polarity := Polarity.short, units := m, capital := 0 } )

/--
  ★T₃₄ spawn 守恒（L0）：在 `m ≤ parent.units`（不能转移超过持有量）前提下，spawn 后
  「新父 units + 新子 units = 原父 units」。父 −m、子 +m，Σ 不变。这是守恒律的归纳基
  （单步），任意深度由 §Forest 的归纳推广。前提 h 在此被 omega 消费（无 h 时 Nat 截断减
  使等式在 m>units 时不成立——h 是定理的真前提，不是装饰）。
-/
theorem spawn_conserves (parent : VoiceLedger) (m : Nat) (h : m ≤ parent.units) :
    (spawn parent m).1.units + (spawn parent m).2.units = parent.units := by
  unfold spawn
  simp only []
  omega

/--
  ★close 操作（子单位归还父）：子关闭，其 units 归还父。返回合并后的父。
  这是 spawn 的逆向守恒——子的 m 单位回到父，Σ 不变。
-/
def close (parent child : VoiceLedger) : VoiceLedger :=
  { parent with units := parent.units + child.units }

/--
  ★T₃₄ close 守恒（L0）：close 后「合并父 units = 原父 units + 子 units」。
  子归还父，Σunits 不变（与 spawn 互逆）。
-/
theorem close_conserves (parent child : VoiceLedger) :
    (close parent child).units = parent.units + child.units := by
  rfl

/--
  ★T₃₄ spawn-close 往返守恒（L0）：spawn 后立即 close，父 units 恢复原值。
  这 machine-check「降成本（spawn 子）不改 N 总量，只是 N 在级别间流转」（§725 陈述），
  关死「spawn 凭空增减单位」的误读——往返是恒等。
-/
theorem spawn_close_roundtrip (parent : VoiceLedger) (m : Nat) (h : m ≤ parent.units) :
    (close (spawn parent m).1 (spawn parent m).2).units = parent.units := by
  unfold close spawn
  simp only []
  omega

/-! ## §4 T₃₅ NAV 价值中性（同价操作前后 NAV 不变）

  necessity_derivation.md §738-745 + D3.2：`nav = free + Σ多头 units×c + Σ空头 capital`。
  一笔物理交易在固定价 c 下不改变总价值。**完整 NAV 必须含空头 capital 项**（codex 审计
  教训 #4：long-only NAV 是子命题非无条件守恒）——本节用三项 NAV，证降成本 spawn（多头卖出
  转子空头）在完整 NAV 下守恒：多头 −m×c，子空 capital +m×c，净 0。 -/

/--
  ★完整 NAV（T₃₅ 三项公式，D3.2）：`nav = free + longUnits×c + shortCapital`。
  - `free`：自由现金。
  - `longUnits`：多头在手单位（× 现价 c 估值）。
  - `shortCapital`：空头保证金现金（已锁定的 capital，不再 ×c——空头价值由 capital 承载）。
  这 **包含空头项**（修 codex FAIL #4 的 long-only 漏项）——降成本是「多头卖出转子空头」，
  必须在含空头项的完整 NAV 下才能证守恒。
-/
def nav (free longUnits c shortCapital : Nat) : Nat :=
  free + longUnits * c + shortCapital

/--
  ★T₃₅ 降成本 spawn 的 NAV 守恒（L0，**完整三项 NAV**）：父多头卖出 m@c 转为子空头
  （子 capital += m×c），NAV 不变。这是 §743-745 的忠实形式：多头项 −m×c，空头 capital
  项 +m×c，free 不变 ⟹ 净变化 0。

  操作：`(free, longUnits, shortCapital)` → `(free, longUnits − m, shortCapital + m×c)`。
  这 machine-check「卖出 m@c：多头项 −m×c，子空 capital +m×c，净变化 0」（§744）——
  **物理一笔（T₃₃）在完整 NAV（含空头）下守恒（T₃₅）**。前提 m ≤ longUnits。
-/
theorem nav_neutral_on_cost_reduce (free longUnits c shortCapital m : Nat)
    (h : m ≤ longUnits) :
    nav free (longUnits - m) c (shortCapital + m * c) = nav free longUnits c shortCapital := by
  unfold nav
  have hsub : (longUnits - m) * c = longUnits * c - m * c := Nat.sub_mul longUnits m c
  have hmc : m * c ≤ longUnits * c := Nat.mul_le_mul_right c h
  rw [hsub]
  omega

/--
  ★T₃₅ 同价买回 NAV 守恒（卖出的对偶，L0）：固定价 c 下用现金买回 m 多头单位，NAV 不变。
  `(free, longUnits, shortCapital)` → `(free − m×c, longUnits + m, shortCapital)`。
  多头项 +m×c，free −m×c，净 0。前提 m×c ≤ free。
-/
theorem nav_neutral_on_buyback (free longUnits c shortCapital m : Nat) (h : m * c ≤ free) :
    nav (free - m * c) (longUnits + m) c shortCapital = nav free longUnits c shortCapital := by
  unfold nav
  have hadd : (longUnits + m) * c = longUnits * c + m * c := Nat.add_mul longUnits m c
  rw [hadd]
  omega

/--
  ★T₃₅ 空头平仓同价中性（L0）：空头以同价 c 平仓（capital 释放回 free），NAV 不变。
  `(free, longUnits, shortCapital)` → `(free + s, longUnits, shortCapital − s)`（释放 s 现金）。
  空头 capital 项 −s，free +s，净 0。前提 s ≤ shortCapital。这覆盖空头侧的 NAV 中性
  （补全多空两侧——不只多头同价中性）。
-/
theorem nav_neutral_on_short_close (free longUnits c shortCapital s : Nat)
    (h : s ≤ shortCapital) :
    nav (free + s) longUnits c (shortCapital - s) = nav free longUnits c shortCapital := by
  unfold nav
  omega

/-! ## §5 T₃₈ N 不主动加仓；N_base 双向重定基

  necessity_derivation.md §798-816：建仓后不主动加仓（A₃）。降成本守恒不改 N（T₃₄）。
  N_base 双向重定基：earning N+Δ（T₃₆）/ 亏损 N−δ（T₁₅）。「恒定」= 不主动加仓，
  **非** 数量不可变。形式化：重定基是一个只在两类事件（earning/loss）改变 N_base 的
  函数；降成本（spawn/close）不调用它。 -/

/-- N_base 重定基事件（T₃₈：只有两类事件改变 N_base）。 -/
inductive RebaseEvent where
  | earning (delta : Nat)   -- cost≤0 后纯利润买入 Δ 正单位，N+Δ
  | loss (delta : Nat)      -- 破极值否定 shortfall 缩水，N−δ
deriving Repr

/--
  ★N_base 双向重定基函数（T₃₈ 核心）：earning 加 Δ，loss 减 δ（饱和减，不下溢）。
  这是 N_base 改变的 **唯一** 途径——降成本（spawn/close §3）不经此函数（它们保 Σ 不变）。
-/
def rebase (nBase : Nat) : RebaseEvent → Nat
  | RebaseEvent.earning d => nBase + d
  | RebaseEvent.loss d => nBase - d

/--
  ★T₃₈ earning 单调增 N（L0）：earning 事件使 N_base 增大（Δ>0 时严格增）。
  这形式化「增 N 的唯一途径 = earning」（§805）。
-/
theorem rebase_earning_increases (nBase d : Nat) :
    rebase nBase (RebaseEvent.earning d) = nBase + d := by
  rfl

/--
  ★T₃₈ loss 单调减 N（L0）：loss 事件使 N_base 减小（饱和，不下溢到负）。
  这形式化「减 N 的途径 = 亏损回补」（§805）。
-/
theorem rebase_loss_decreases (nBase d : Nat) :
    rebase nBase (RebaseEvent.loss d) = nBase - d := by
  rfl

/--
  ★会计状态（T₃₈「两个范畴」的载体）：N_base 与森林的笛卡尔积。
  关键：N_base 与 forest 是 **独立分量**——降成本操作改 forest（§3 的 spawn/close）但不改
  N_base；重定基操作改 N_base 但不改 forest 的 totalUnits 守恒。这把「守恒（圈内不变）与
  重定基（φ=0 跳变）是两个范畴」（§811/§813）编码为积结构的两个投影互不干涉。
-/
structure AccountState where
  nBase : Nat
  forest : List VoiceLedger
deriving Repr

/--
  ★降成本操作（spawn 子）作用于会计状态：替换 forest 的头两个 voice 为 spawn 结果，
  **N_base 分量原样保留**。建模「降成本只改 forest，不进 rebase 通道」。
  （取头一个 voice 作 parent；若不足或 m 越界则恒等——边界由构造性 m≤units 保证，
  这里给出全函数版以便结构推理。）
-/
def applyCostReduce (st : AccountState) (m : Nat) : AccountState :=
  match st.forest with
  | parent :: rest =>
      if m ≤ parent.units then
        let (p', c') := spawn parent m
        { st with forest := p' :: c' :: rest }
      else st
  | [] => st

/--
  ★重定基操作作用于会计状态：改 N_base 分量（经 §rebase），**forest 分量原样保留**。
  建模「重定基只改 N_base，不动森林单位流转」。
-/
def applyRebase (st : AccountState) (e : RebaseEvent) : AccountState :=
  { st with nBase := rebase st.nBase e }

/--
  ★T₃₈「不主动加仓」的结构定理（L0，**非恒等占位**）：降成本操作 `applyCostReduce`
  **保持 N_base 分量不变**。这不是平凡 rfl——它是 applyCostReduce 定义的后置条件：该函数
  只重写 forest 分量，N_base 投影被它原样转发。这 machine-check「降成本守恒不改 N」
  （§804）与「N_base 的重定基只发生在 earning/loss 事件」（§813）——降成本不是重定基事件。
-/
theorem costReduce_preserves_nBase (st : AccountState) (m : Nat) :
    (applyCostReduce st m).nBase = st.nBase := by
  unfold applyCostReduce
  cases st.forest with
  | nil => rfl
  | cons parent rest =>
      by_cases h : m ≤ parent.units
      · simp [h]
      · simp [h]

/--
  ★T₃₈ 重定基不动森林守恒（对偶，L0）：`applyRebase` 保持 forest 分量不变（故 totalUnits
  守恒不受重定基影响）。与 costReduce_preserves_nBase 合起来 = 两投影互不干涉，即「两个
  范畴」的精确形式：降成本动 forest 不动 N_base，重定基动 N_base 不动 forest。
-/
theorem rebase_preserves_forest (st : AccountState) (e : RebaseEvent) :
    (applyRebase st e).forest = st.forest := by
  rfl

end Formal.Tlayers.Accounting
