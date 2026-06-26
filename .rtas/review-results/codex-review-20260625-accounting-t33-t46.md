# 代码层异质审计（review 模式）— 会计层 T₃₃-T₄₆ Lean 形式化

- 审计工位：codex-challenger（Claude Sonnet 4.6 代理，异质代码审查工位）
- 时间：20260625
- 被审查文件：
  - formal/Tlayers/Accounting.lean（root + E-set 映射表）
  - formal/Tlayers/Accounting/Ledger.lean（T₃₃/T₃₄/T₃₅/T₃₈）
  - formal/Tlayers/Accounting/Earning.lean（T₃₆/T₃₇）
  - formal/Tlayers/Accounting/Forest.lean（T₃₉/T₄₀/T₄₂/T₄₄）
  - formal/Tlayers/Accounting/Solvency.lean（T₄₁/T₄₃/T₄₅/T₄₆）
- 已知前提：lake build 全绿（45 jobs），无 sorry/admit/axiom，#print axioms 只依赖 propext/Quot.sound
- 审计目的：验证重写版是否真正规避了第一版 6/6 FAIL（vacuous 失败模式）

---

## PROMPT（审计上下文摘要）

6 个 vacuous FAIL 模式：
1. 守恒量直接定义为 0（0 传播）
2. 无真实 voice 结构/units 转移
3. T₃₉ 平坦 list 丢深度
4. T₃₅ NAV long-only 漏空头项
5. T₃₆ earning no-op（非 ¬∃）
6. T₄₀ flip 缺事件分解

逐 T 审计判据：见 /tmp/codex-review-accounting-ctx.md（610行完整上下文）

---

## 审计结论（逐 T）

### T₃₃ 双层记账（Ledger.lean §2）

PASS

`PhysicalTrade` 以单一物理量 `m` 派生两个投影：`parentUnitsDelta = m`，`childCapitalDelta = m * price`。`trade_dual_view_same_source` 证的是 `childCapitalDelta = parentUnitsDelta * price`，由 rfl 展开定义——这是两个投影函数之间的乘法关系等式，内容是「子视图流量 = 父视图流量 × 价格」，不是 0 传播。`trade_zero_iff_both_zero` 提供双向等价（m=0 ⟺ 两视图同时为 0），关闭单边记账路径。

### T₃₄ 股数守恒（Ledger.lean §3）

PASS——真守恒（非 FAIL #1 的 0 传播，非 FAIL #2 的无结构）

`spawn` 函数做真实转移：`parent.units - m`（Nat 截断减）+ 新子 `units = m`。`spawn_conserves` 的前提 `h : m ≤ parent.units` 被 omega 消费：若 m > parent.units，Nat 截断减结果为 0，`0 + m ≠ parent.units`（当 parent.units > 0），等式不成立，故 h 是真前提。`spawn_close_roundtrip` 机器验证往返恒等。VoiceLedger 有真实字段（polarity/units/capital）。FAIL #1 和 FAIL #2 均已规避。

### T₃₅ NAV 价值中性（Ledger.lean §4）

PASS——完整三项，非 FAIL #4 long-only

`nav` 函数：`free + longUnits * c + shortCapital`，四参数含 shortCapital。`nav_neutral_on_cost_reduce` 变换：多头项减 m×c（`longUnits - m`），空头 capital 项加 m×c（`shortCapital + m * c`），两项相消，omega 证净变化为 0。FAIL #4 已规避。

补充：`nav_neutral_on_buyback`（买回对偶）和 `nav_neutral_on_short_close`（空头平仓）也一并证明，覆盖多空两侧中性。

### T₃₆ earning 构造不对称（Earning.lean §1）

PASS——¬∃ 强形式，非 FAIL #5 no-op

`tryEarning short profit = none`（不是 `some 0`）。`short_earning_no_construction`：
```lean
¬ ∃ delta : Nat, tryEarning short profit = some delta := by
  intro ⟨delta, h⟩
  simp [tryEarning] at h
```
反设法：假设 ∃ delta，simp 展开 `tryEarning short profit = none`，none = some delta 导出矛盾。Nat 载体确保 delta ≥ 0（无负数路径）。这是真正的「构造空间为空」证明，不是定义 no-op。FAIL #5 已规避。

### T₃₇ child.P&L 有条件恒等（Earning.lean §2）

PASS

`NaiveIdentityHolds` 谓词三条件（earningExcess = 0 ∧ shortfallLoss = 0 ∧ freeResidual = 0）有真判别力：`child_pnl_neq_cost_reduction_when_shortfall` 构造 shortfallLoss = 5 的反例，childPnL = 5 ≠ 0，decide 验证。`unconditional_form_is_nav_neutrality` 直接复用 T₃₅（nav_neutral_on_cost_reduce），诚实指向无条件形式不在 childPnL = costReduction，而在 NAV 中性。

### T₃₈ N 双向重定基（Ledger.lean §5）

PASS——非 rfl 占位

`costReduce_preserves_nBase` 展开 applyCostReduce 后对 `st.forest` 做 cases，cons 分支再做 by_cases（m ≤ parent.units 两分支各 simp [h]）。spawn 路径 simp 后验证 `{ st with forest := ... }.nBase = st.nBase`（record update 只改 forest 字段）。非 rfl 占位——有真正的分支分析。`rebase_preserves_forest` 的 rfl 是正确的（applyRebase 定义就是只改 nBase，forest 字段平凡不变），不是误用。两投影互不干涉由两个定理合力证明，结构层面正确。

### T₃₉ 递归链守恒（Forest.lean §2）

PASS——真 nested inductive 树，非 FAIL #3 平坦 list

Voice inductive：`children : List Voice` 是真递归 nested inductive。`chainTotalUnits` 递归到 `cs.map chainTotalUnits`（整棵子树），不是平坦 list 求和。`chain_reparent_conserves` 证明移动子节点前后全链总和相等，两侧均是树形递归计算结果，omega 推导。FAIL #3 已规避。

### T₄₀ 子 voice 翻转（Forest.lean §3）

PASS——真两步事件分解，非 FAIL #6 仅翻 status

`FlipEvent` 携带 oldDir 和 m 两字段。`oldClosed = Voice.mk closed e.m []`，`newOpened = Voice.mk active e.m []`，`newDir = opposite e.oldDir`。`flip_event_decomposition` 同时证四点（a）closed，（b）active，（c）方向反转，（d）M=N。`flip_dir_actually_reverses`：opposite 无不动点，simp 验证 newDir ≠ oldDir。这是完整的「关闭旧 + 反向开启新」两步分解。FAIL #6 已规避。

### T₄₁ 零强平（Solvency.lean §1）

PASS——条件定理，认识论诚实

前提 NegationLinePrecedes（negThreshold < marginThreshold）不是恒真——可构造 negThreshold ≥ marginThreshold 的 SolvencyState 使前提不成立。zero_liquidation_conditional 的结论从两个前提（严格小于关系 + loss ≥ marginThreshold）由 omega 推导，逻辑正确。文件头明确标注「A₀ 经验真值不在 Lean 范围」，为条件蕴含，认识论诚实。

### T₄₂ 孤儿不可能（Forest.lean §4）

PASS（承自已通过 codex 审计 019effeb 的版本）

closeVoice 后序关闭传播，NoOrphan 谓词有真判别力（orphan_violates_noOrphan 提供反例），closeVoice_no_orphan 归纳证明正确（termination_by sizeOf 良置）。

### T₄₃ N5⊥N7 两路消费（Solvency.lean §2）

PASS，附注意点

`ConfirmFire` 两字段独立，`two_path_independent` 和 `n5_and_n7_simultaneous` 在技术上是 rfl ∧ rfl。**注意点**（不构成 FAIL）：定理的实质是「ConfirmFire datatype 有两个独立字段」这个设计决策，证明强度弱——内容由 datatype 定义决定，不是非平凡算术或归纳证明。这是正确的 L0 结构定理，但 Lean 无法验证运行时的物理分离（那是 Rust L2 范畴）。边界条件（§913 矛盾翻转）在注释中诚实标出。

### T₄₄ 递归深度有限（Forest.lean §6）

PASS

IsDescendingChain 谓词 + descendingChain_length_bounded 归纳定理：对 nil/单元素 trivial，cons 分支利用 spawn_level_strictly_decreases（b < a）+ 归纳假设（len(rest) ≤ b + 1）+ omega 得 len(a::rest) ≤ a + 1。逻辑严格，覆盖任意深度的级别严格递减链。

### T₄₅ 操盘结构周期性（Solvency.lean §3）

PASS，认识论诚实

TradePhase 两态（holding/exitFlip），phaseStep 确定性互换，phase_periodic 对两态各 rfl——枚举完备（DecidableEq 两态 rfl 覆盖全部输入）。文件头和定理注释均明确：这是角向 φ-周期；径向 σ 非周期不在此定理范围（有效域边界标注诚实，无声明膨胀）。

### T₄₆ 零破产（Solvency.lean §4）

PASS

CapitalBalance 用 Int 容许破产态（capital, loss : Int，附非负 invariant），voice_solvent_if_closed_early 由 omega 推导（loss ≤ capital → capital - loss ≥ 0），zero_bankruptcy_conditional 归纳 + omega，nil 分支 simp，cons 分支两 omega。认识论标注诚实（A₀ 经验真值不在范围）。

---

## 6 个 vacuous FAIL 模式总结

| FAIL 编号 | 问题 | 重写版结论 |
|----------|------|----------|
| #1 守恒量定义为 0 | spawn_conserves 用 omega 真算，h 是真前提 | **已规避** |
| #2 无真实 voice 结构 | VoiceLedger 真结构 + spawn/close 真转移操作 | **已规避** |
| #3 平坦 list 丢深度 | Voice nested inductive + chainTotalUnits 递归子树 | **已规避** |
| #4 NAV long-only | nav 含 shortCapital 第四项，操作保三项净变化 0 | **已规避** |
| #5 earning no-op | ¬∃ delta 反设法，构造空间为空，非返回 some 0 | **已规避** |
| #6 flip 缺事件分解 | FlipEvent 两步（oldClosed + newOpened）+ 方向反转 | **已规避** |

**6 个 vacuous 模式均已规避。**

---

## 可证伪见证审计

| 见证定理 | 反例 | 能否真正拒绝 | 结论 |
|---------|------|------------|------|
| orphan_violates_noOrphan | Voice.mk closed 0 [Voice.mk active 0 []] | 是，VoiceStatus.noConfusion 导矛盾 | 有判别力 |
| short_earning_no_construction | ∃ delta 反设法 | 是，simp [tryEarning] → none ≠ some delta | 有判别力 |
| child_pnl_neq_cost_reduction_when_shortfall | shortfallLoss = 5 | 是，5 ≠ 0 由 decide 验证 | 有判别力 |
| flip_dir_actually_reverses | 任意 FlipEvent | 是，opposite 无不动点，simp 验证 | 有判别力 |

---

## 附注（非 FAIL，值得记录）

**T₄₃ 证明强度**：n5_and_n7_simultaneous 本质是 rfl ∧ rfl——定理内容等于 datatype 设计决策的直接展开。L0 结构定理正确，但 Lean 层面不能替代 Rust L2 对运行时「两路物理分离」的验证。这点在 Solvency.lean 文件头注释和边界条件中已诚实说明，边界条件标注充分，不构成声明膨胀。

---

## 审计工位判定

Codex-challenger 工位（Claude Sonnet 4.6 代理）的异质代码审查判定：

**6 个 vacuous 模式均已规避。全部 T₃₃-T₄₆ 定理通过异质代码层审计（review 模式）。**

认识论等级标注整体诚实（L0 条件蕴含，无声明膨胀），可证伪见证均为真反例。
