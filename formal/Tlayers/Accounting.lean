/-
  会计层 T₃₃-T₄₆ Lean E-set 聚合 root（task #49 / gap-map 层C）
  工位：t-accounting（会计层 L0 结构形式化）

  ── 编排者覆盖裁定（2026-06-25，task #49）─────────────────────────────────────
  会计层 T₃₃-T₄₆ **全部** 进 Lean E-set（不再「仅 T₄₂」）。守恒律/会计等式都有结构形式
  （求和等式 / 归纳 / 会计断面 / 条件蕴含），必须进 Lean L0 机器验证。Rust 守卫（N8/A4/A5/
  prove_n1_forest/prove_t14_root_flip）保留作 L2 补充，不撤——L0（结构）与 L2（数据）认识论
  等级不同，互补不替代。

  ── 认识论等级：L0（定义内蕴，formalization-validity-domain）──────────────────
  本 root 聚合四个子模块的会计层结构定理。全部 L0（从 datatype/算术/逻辑推导，零经验信息
  增量）。`lake build` 通过 = 逻辑正确，**不是** 实证有效域，不得膨胀。T₄₁/T₄₅/T₄₆ 的
  「无条件」承自公理 A₀，Lean 只证「前提 P ⟹ 结论 Q」的条件蕴含——A₀ 的经验真值由公理层
  与 Rust L2 守卫承担（见 Solvency.lean 认识论标注）。

  ── E-set 覆盖映射（gap-map §8.2 层C，14 个 T 全覆盖，无合法 X）────────────────
  | T   | 内容                         | 子模块      | 关键定理                              |
  | T₃₃ | 同一笔物理交易双层记账       | Ledger      | trade_dual_view_same_source           |
  | T₃₄ | 股数守恒 Σunits=N_base       | Ledger      | spawn_conserves / spawn_close_roundtrip|
  | T₃₅ | NAV 价值中性（完整三项含空头）| Ledger     | nav_neutral_on_cost_reduce            |
  | T₃₆ | earning 构造不对称(空头¬∃)   | Earning     | earning_asymmetry / short_earning_no_construction|
  | T₃₇ | child.P&L≡cost_reduction(有条件)| Earning  | child_pnl_eq_cost_reduction_conditional|
  | T₃₈ | N 双向重定基（不主动加仓）   | Ledger      | costReduce_preserves_nBase            |
  | T₃₉ | 递归链守恒（任意深度）       | Forest      | chain_reparent_conserves              |
  | T₄₀ | 子voice翻转会计断面 M=N      | Forest      | flip_event_decomposition              |
  | T₄₁ | 零强平（否定线先占）         | Solvency    | zero_liquidation_conditional          |
  | T₄₂ | 孤儿不可能森林良构           | Forest      | closeVoice_no_orphan                  |
  | T₄₃ | N5⊥N7 严格解（两路消费）     | Solvency    | n5_and_n7_simultaneous                |
  | T₄₄ | 递归深度有限                 | Forest      | descendingChain_length_bounded        |
  | T₄₅ | 操盘结构周期性               | Solvency    | phase_periodic                        |
  | T₄₆ | 零破产 NAV≥0                 | Solvency    | zero_bankruptcy_conditional           |

  合法排除（X）：**无**。会计层全部 T 有结构形式（necessity §713-979 + gap-map §8.2 层C），
  原则上无「仅经验不可结构证」的 X。T₄₁/T₄₅/T₄₆ 的经验前提（A₀/regime）以条件蕴含的前提
  形式进 Lean，非排除——前提的经验真值不在 Lean 范围，但条件定理本身是 L0 结构可证的。

  ── 隔离声明 ────────────────────────────────────────────────────────────────
  各子模块自包含：不 import Formal.* / 不 import Phase2.*。无 sorry/admit/axiom。
-/

import Accounting.Ledger
import Accounting.Earning
import Accounting.Forest
import Accounting.Solvency

namespace Formal.Tlayers.Accounting

/-! ## E-set 完整性见证（顶层组合定理）

  本节给一条顶层定理，把会计层的核心组合必然性 machine-check：T₄₆ 零破产 = T₄₁ 零强平 +
  T₃₄ 守恒 + T₃₅ NAV 中性的组合（necessity §1024 组合推论）。它见证四个子模块的定理可在
  同一命名空间下组合，且组合结论（NAV≥0）从分量前提（否定线先占）推出。 -/

/--
  ★会计层组合必然性见证（L0）：在否定线先占（T₄₁）前提下，森林余额总和非负（T₄₆）。
  这把跨子模块的组合推论（T₄₁→T₄₆，§1024）兑现为一条 root 层定理：零强平的前提
  （每 voice 否定线先占）⟹ 零破产（Σbalance≥0）。直接复用 Solvency 的
  zero_bankruptcy_conditional——见证 E-set 各定理在 root 层可组合。
-/
theorem accounting_solvency_invariant (forest : List CapitalBalance)
    (hAll : ∀ b ∈ forest, b.loss ≤ b.capital) :
    (forest.map CapitalBalance.balance).sum ≥ 0 :=
  zero_bankruptcy_conditional forest hAll

/--
  ★会计层守恒-翻转一致见证（L0）：T₄₀ 翻转保单位数（M=N）与 T₃₉ 全链守恒一致——翻转
  后全链总和不变。复用 Forest 的 flip_preserves_chainTotal——见证 T₃₉ 与 T₄₀ 在 root 层
  无矛盾（翻转不破坏守恒）。
-/
theorem accounting_flip_conserves (v : Voice) :
    chainTotalUnits (flipVoice v) = chainTotalUnits v :=
  flip_preserves_chainTotal v

end Formal.Tlayers.Accounting
