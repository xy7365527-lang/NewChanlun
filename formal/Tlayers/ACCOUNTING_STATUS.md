# 会计层 T₃₃-T₄₆ Lean 形式化状态（task #49 / t-accounting）

编排者裁决（2026-06-25，覆盖 codex D1 + Lead 原"仅 T₄₂"消息）：**会计层 T₃₃-T₄₆ 全部进 Lean E-set**。
认识论等级：Lean = L0（定义内蕴，结构/代数定理）。Rust N8/A4/N1 守卫保留作 L2 补充（不被 Lean 取代）。

## 进度总览（全部完成，lake build PASS，无 sorry/admit/axiom）

T₃₃-T₄₆ 全 14 个 T 已形式化，拆为 4 个自包含子模块（root `Accounting.lean` 聚合 import）。

| 定理 | 内容 | 状态 | 模块 | 关键定理 |
|------|------|------|------|---------|
| T₃₃ | 同一笔物理交易双层记账 | ✅ | Ledger | trade_dual_view_same_source（两视图同源一笔，非两笔） |
| T₃₄ | 股数守恒 Σunits=N_base | ✅ | Ledger | spawn_conserves / spawn_close_roundtrip（真转移保 Σ，非 0 传播） |
| T₃₅ | NAV 价值中性（完整三项含空头） | ✅ | Ledger | nav_neutral_on_cost_reduce（multi+short capital，**修 long-only 漏项**） |
| T₃₆ | earning 多空构造不对称 | ✅ | Earning | earning_asymmetry + short_earning_no_construction（**¬∃，非 no-op**） |
| T₃₇ | child.P&L≡cost_reduction 有条件 | ✅ | Earning | child_pnl_eq_cost_reduction_conditional + 无条件形式指向 T₃₅ |
| T₃₈ | N 不主动加仓;N_base 双向重定基 | ✅ | Ledger | costReduce_preserves_nBase（两投影互不干涉，非 rfl 占位） |
| T₃₉ | 递归链守恒（任意深度） | ✅ | Forest | chain_reparent_conserves（**真 Voice nested inductive 树，非平坦 list**） |
| T₄₀ | 子voice翻转=会计断面 M=N | ✅ | Forest | flip_event_decomposition（**关闭旧+反向开启新两步分解，非仅翻 status**） |
| T₄₁ | 零强平（否定线先于保证金线） | ✅ | Solvency | zero_liquidation_conditional（条件式 L0，A₀ 前提式） |
| **T₄₂** | **孤儿不可能·森林良构结构侧** | ✅ | Forest | closeVoice_no_orphan（承自原 Accounting.lean，codex 6/6 PASS，迁入 Forest） |
| T₄₃ | N5⊥N7严格解(confirm两路消费) | ✅ | Solvency | n5_and_n7_simultaneous（两路独立，边界条件标注 escalate 触发） |
| T₄₄ | 递归深度有限 | ✅ | Forest | descendingChain_length_bounded（级别良序严格递减⟹链长有界） |
| T₄₅ | 操盘结构周期性(角向φ-周期) | ✅ | Solvency | phase_periodic（有限状态确定转移⟹周期=2，径向非周期作边界标注） |
| T₄₆ | 零破产 NAV≥0 | ✅ | Solvency | zero_bankruptcy_conditional（T₄₁+T₃₄+T₃₅ 组合，条件式 L0） |

合法排除（X）：**无**。会计层全部 T 有结构形式。T₄₁/T₄₅/T₄₆ 的经验前提（A₀/regime）以条件蕴含的
前提形式进 Lean，非排除——条件定理本身 L0 结构可证，前提的经验真值不在 Lean 范围。

## ★守恒模块 codex FAIL 教训（vacuous 失败模式）+ 本次逐条修复

第一版守恒模块（T₃₃-T₃₆/T₃₉/T₄₀）曾被 codex high 审计 **6/6 FAIL**（vacuous），已删除并重写。
本次重写 **逐条规避** 这 6 个失败模式（formalization-validity-domain 合成确认偏差 + 把结论塞进定义）：

1. **`Entry.net := 0` 直接定义为 0** → 本次 spawn_conserves 用 `(units−m)+m = units`，omega 真算守恒，非 0 传播。✅
2. **不存 voice 分布** → 本次建真 VoiceLedger.units + spawn(父−m/子+m)/close(子归还父) 真转移操作。✅
3. **平坦 list 丢深度（T₃₉）** → 本次用真 `Voice` nested inductive（chainTotalUnits 递归到子树），非平坦 list。✅
4. **NAV long-only（T₃₅）** → 本次 nav = free + longUnits×c + **shortCapital**（完整三项，含空头），降成本
   spawn 在含空头项的 NAV 下守恒（多头−m×c / 子空 capital +m×c / 净 0）。✅
5. **earning no-op（T₃₆）** → 本次用 Option（some/none）+ ℕ 非负载体 + `short_earning_no_construction : ¬∃ delta`
   （证空头 earning 构造空间为空，非定义成不变）。✅
6. **flip 缺事件分解（T₄₀）** → 本次 flip_event_decomposition：FlipEvent 显式拆「关闭旧（status=closed）+
   反向开启新（status=active, newDir=opposite oldDir）」两步 + M=N（oldClosed.units=newOpened.units）+
   flip_dir_actually_reverses（方向真反转可证伪见证）。✅

每个守恒定理都仿 T₄₂ 范式：真结构对象 + 真操作 + 可证伪反例见证（short_earning_no_construction /
orphan_violates_noOrphan / child_pnl_neq_cost_reduction_when_shortfall / flip_dir_actually_reverses）。

## 与 #42 矛盾的边界

task #42（58槽 D∞ 穷尽被会计层 L0 反推否定 3 条）涉及 **几何螺旋层 D∞ 群呈现**（Spiral.lean）。
会计层本模块形式化的是 **森林单位守恒与结构良构**（Σunits=N_base 归纳形式），**不触及 58 槽 D∞
穷尽**——会计守恒（圈内不变量）与 D∞ 角向穷尽是两个范畴。本模块在会计范畴内自洽，不向 D∞ 穷尽
范畴施加约束，**未重新触及 #42 矛盾**（边界声明见 Forest.lean 文件头）。

## 文件结构

- `Tlayers/Accounting.lean`（root，import 四子模块 + E-set 完整性见证 + 组合定理）
- `Tlayers/Accounting/Ledger.lean`（T₃₃/T₃₄/T₃₅/T₃₈，账本基础/守恒/NAV/重定基）
- `Tlayers/Accounting/Earning.lean`（T₃₆/T₃₇，构造不对称/有条件恒等）
- `Tlayers/Accounting/Forest.lean`（T₃₉/T₄₀/T₄₂/T₄₄，递归森林守恒/翻转/孤儿/深度有限）
- `Tlayers/Accounting/Solvency.lean`（T₄₁/T₄₃/T₄₅/T₄₆，零强平/两路消费/周期/零破产）

验证：`cd formal && lake build`（45 jobs 全绿）；会计层各文件 0 warning；无 sorry/admit/axiom。

## 边界条件（整层）

- 守恒律是运行时数值不变量(R类)，Lean 表达的是其 **L0 结构形式**（归纳/代数恒等），不是浮点数值守恒
  （那是 Rust N8 的 L2）。若某守恒律的结构侧在 datatype 上无法表达 → no-workaround → /escalate。
- T₄₁/T₄₆ 零强平/零破产含 A₀ 公理 + regime 论证：Lean 只形式化"给定 A₀ 前提则结论"的条件式 L0，
  不能形式化 A₀ 本身（公理）。诚实标注为前提式定理（Solvency.lean 文件头认识论标注）。
- T₄₃ 边界（§913 矛盾翻转）：若证明 confirm fire 不可分两路，则 N5⊥N7 不可同时满足，须 /escalate。
  当前 ConfirmFire 两字段独立 ⟹ 可分两路成立。

## 复用资产

- `formal/Formal/RStarNonSpecial.lean`：ValidTower 终余代数（T₄₂/T₄₄ 范式层参考，非 import 耦合）
- `formal/Formal/RecursiveConstruction.lean`：Move 初代数 μF 归纳守恒模式（范式参考）
- nested inductive 归纳模板（Forest.lean closeVoice_no_orphan，termination_by + decreasing_by sizeOf）
