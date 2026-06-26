# T-Lean 层B（task #48）操作层 T₁₇-T₃₂ 全 E-set Lean L0 形式化 — 详情

**模块**：`formal/Tlayers/Operational.lean`（自包含单一权威源，命名空间 `Formal.Tlayers.Operational`，**54 定理**）
**owner**：t-operational ｜ **状态**：completed ｜ **日期**：2026-06-25（编排者覆盖 codex D1：全都要 Lean，E-set 全进）

## 结论

操作层 T₁₇–T₃₂ **全部 E-set 进 Lean L0**（编排者裁定）。L 类（结构归纳）+ R 类（运行时律 L0 结构形式，Rust 守卫保留 L2）+ A 类（轴范式 reform→真完全分类，**用初代数构造子推导**）全部机器验证通过。唯一合法排除 X = T₃₂ 的 λ 定量罕见率（L2 经验，定性侧已入 E）。

**★回退纠正记录（team-lead 2026-06-25）**：曾因乱序处理"先做L类、R暂挂等codex"的**已作废**消息（编排者"全都要lean"已覆盖 codex D1），一度删 44 定理版退回 6 定理半版。已恢复全 E-set，现 **54 定理**。`OperationalStructural.lean`（6 定理半版）已删除（no-patch 不留半版，L 类已并入自包含 Operational.lean）。

**★推导口径（编排者新补 + codex 019f0001 加强）**：reform 项（§6B/T₂₄）用 **初代数构造子推导** 真完全分类，非轴枚举：
- `t6b_interaxis_initiality`：三轴 = InterAxis 初代数 initiality 泛性质（依赖消去/结构归纳，非"数了三个"）
- `t6b_axis_surjective_forall`：三轴满射标准 ∀∃ 形态（codex 加强，每轴有 witness 关系，闭合空枚举漏洞）
- `t6b_derivation_disposition`：initiality + 满射 + (σ-等变 iff H¹ ∧ ∃ 未捕获) 三合一（codex 加强第三支折入 iff）
- `t24_direction_initiality` / `t24_involutive_by_initiality`：方向初代数推导 ℤ/2 对合（非方向枚举）

codex 异质审计 session **019f0001-fd86-7961-9291-d5cd61236509**（high，turn.completed）：推导口径整体 **PASS**（已把完全分类从析取枚举提升到 inductive initiality/依赖消去 + 满射 witness 闭合空轴漏洞），两条可选加强已全部应用。

## 各 T 状态 + 定理名

| T | 类 | 定理 | 形式化内容 |
|---|----|------|-----------|
| T₁₇ 区间套桥梁 | L | `t17_bridge_narrows` / `t17_bridge_chain_finite` | 相邻级别严格收窄 + 链长 ≤ 首段级别+1（复用 DivergenceNesting） |
| T₂₃ voice 自相似 | L | `t23_self_similar` / `t23_self_similar_finite` | 子树同样良构 + 深度有限（VoiceTree 递归数据类型） |
| T₄₄ 递归深度有限 | L | `t44_voice_depth_le_level` / `t44_recursion_finite` | 深度 ≤ 根级别（级别严格−1 良序下降，结构归纳） |
| T₁₈ 配额 σ-不变 | R | `t18_quota_sigma_invariant`(满足 `SigmaInvariant` 谓词) / `t18_level_dependent_breaks_invariance`(反例否证) / `t18_quota_conserves` | σ-不变=代数等式 `∀k p, q(k+1)p=q k p`；**f=1/λ 数值是 L2 不入 Lean** |
| T₁₉ 成本门=终止 | R | `t19_cost_gate_total`/`t19_base_terminates`/`t19_below/above_friction`/`t19_termination_upward_closed`(向上封闭)/`t19_leaf_from_cost_gate`/`t19_none_is_leaf` | 成本门完全分类 + 向上封闭 + leaf↔成本门绑定 |
| T₂₀ 降成本自层 | R | `t20_decost_self_level_only` / `t20_distinct_traversals` | 遍历分离：降成本投影 ⊥ 全局链 |
| T₂₁ 并发 per-voice | R | `t21_per_voice_independent` / `t21_same_voice_no_double` | per-voice 不互阻 + NoDup |
| T₂₂ 赋格 stretto | R | `t22_stretto_overlap` / `t22_stretto_realizable` | 子/父 active 区间重叠（**未暴露 cd_ℚ=1 冲突**） |
| T₂₄ 多空对称 | A reform | `t24_flip_symmetric`/`t24_flip_involutive`(ℤ/2)/`t24_direction_total`(构造子穷尽)/`t24_both_directions_perfect` | reform=Direction 二构造子 + flip 对合 ℤ/2 |
| T₂₅ 方向交替 | A | `t25_alternates` | 父反向，连续两级回同向 |
| T₂₆ 三阶段 | R | `t26_phase_total`/`t26_phase_cycles`(周期3)/`t26_exit_to_enter` | 三构造子穷尽 + 循环闭合 |
| T₂₇ 永远在场 | R | `t27_always_in_market` / `t27_exit_stays_in_market` | 三阶段都在场（清仓测度零=T₃₂ L2 不入 Lean） |
| T₂₈ 最高级别进入 | R | `t28_entry_at_top` / `t28_segment_not_source` | source=链顶 ∧ ≥PENDING_LO |
| T₂₉ 先势后定位 | R | `t29_causal_transitive` / `t29_confirm_not_before_compress` | compress≤confirm≤bar 时序偏序 |
| T₃₀ 根涌现=重组 | R | `t30_relabel_preserves` / `t30_emergent_monotone` | relabel 保 units/NAV + 级别单调爬升 |
| T₃₁ 出场对齐 source | R | `t31_exit_aligns_source` / `t31_below_entry_is_decost` | 出场级别 ≥ 入场级别 |
| T₃₂ 清仓门槛 | R(定性)+X(频率) | `t32_liquidation_requires_all` / `t32_no_top_perfect_no_liquidation` | 三条件合取门槛（**λ 定量罕见率=X，L2 不入 Lean**） |
| §6B 三轴(#45) | A reform | `interaxis_total`/`interrelation_axis_total`/`t6b_sigma_captures_iff_h1`/`t6b_sigma_misses_groupoid`/`t6b_sigma_misses_morphological`/`t6b_paradigm_disposition` | reform=三轴构造子穷尽 sum type；σ-等变 ⊊ 三轴（有效域收窄非冲突） |

## 边界条件

- **R 类"L0 结构形式"的有效域**：Lean 证 conditional/代数定理（"若 model 满足公理则不变量成立"），**不**直接证运行时行为——后者由 Rust panic 守卫（L2）验证。若编排者要求 Lean 证运行时行为本身（而非结构形式）→ 超出 L0 定义内蕴有效域，需 L2。
- **X 排除翻转条件**：若 T₃₂ 的 λ 罕见率被要求结构证（而非经验测）→ 违 formalization-validity-domain（频率是 L2 经验，无结构侧）。
- **reform 翻转条件**：T₂₄/§6B reform 成立依赖"方向/三轴是构造子完备 sum type"。若发现第三方向或第四轴 → reform 失败需 /escalate。当前 Direction(2)/InterAxis(3) 构造子穷尽，reform 成立。

## 下游推论

- Rust 引擎 N1-N8 守卫（§0.5 映射）= 本 L0 形式的 L2 经验补充，互补不替代。
- §6B #45 范式归宿已钉：三轴构造子穷尽（结构层 L0 封闭）+ H⁰ 内容层开放（恰证 σ-等变不足）。
- 实装对接（bit-exact/env-gate）：可形式化的结构条件（成本门/配额/时序偏序）对接 Rust recursive_t（task #38 Phase3-impl）。

## 谱系引用

- 603 递归数据类型范式（构造子穷尽）：T₂₄/§6B reform + VoiceTree/Phase/InterAxis sum type 的范式依据。
- codex 异质审计 session **019effee-c23f-7063-9b03-86fd29c99107**（codex-cli 0.125.0，high，turn.completed，input 664k tokens）：去重言化 T₁₈ + 去标题膨胀 + 补 T₂₇ + T₁₉ leaf 绑定，已全部吸收。
- 540 号压缩↔展开（T₂₉ 时序）；542 号配额 σ-不变（T₁₈）。

## 影响声明

- 新增 `formal/Tlayers/Operational.lean`（48 定理，自包含）+ 本 `.md`。
- **未碰** `lakefile.toml` / `Formal.lean`（硬约束）。前版 `OperationalStructural.lean` 已删除（合并入自包含模块，避免本地 import 路径问题 + 双份漂移）。
- 不改动任何已结算定义/谱系；§6B #45 从 pending 推进为 reform 完成（构造子穷尽）。

## 验证（证据）

- `cd formal && lake env lean Tlayers/Operational.lean` → 零 error/warning/sorry/admit/axiom
- `lake build`（全项目）→ `Build completed successfully (16 jobs)`，不受扰动
- `#print axioms`：`t18_quota_sigma_invariant` 无 axiom 依赖（非重言）；其余仅 Lean 标准 propext/Quot.sound（无 user axiom，无 sorryAx）

## escalate

**无**。T₂₂ stretto（区间重叠可自洽）/ §6B cd_ℚ=1 两层闭合（三轴 reform 构造子穷尽，有效域收窄非冲突）均检验 → **未暴露真定义冲突**。若后续 R 类深化暴露 cd_ℚ=1 两层闭合定义冲突 → 立即 SendMessage to:'team-lead' /escalate。
