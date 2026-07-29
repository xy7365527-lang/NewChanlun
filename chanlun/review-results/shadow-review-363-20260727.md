# 影子评审 #369：#363 逐级稀疏性判据同步（commit `d6f7e26430`）

- 评审人：独立影子评审（opus，新上下文，非实装 lineage；实装自述**未采信**，全部结论自行读码/跑测复核）
- 评审对象：`d6f7e26430`（`fill.rs` +369 / `level_order.rs` +91）
- 读码面：`/tmp/kimi-nest-mainline`（只读 `git show`）；运行面：`/tmp/iso363`（普通目录拷贝）
- 模式：快速（票面 Spec ①–⑥ + Standards 三条），不展开票外

## 裁决

**PASS**（硬违规 0，MED 0，LOW 2 + 登记确认 1）。核心判据的构造正确性、同源性、非平凡红绿、default 零行为变化，四项均独立复核成立。

## 复跑

| 项 | 结果 |
|---|---|
| `cd /tmp/iso363/rust && cargo test --release --lib` | **1894 passed / 1 failed**（唯一失败 = `classifier::signal::tests::extract_signals_bit_exact_digest_guard`，#115 线在案，与本票无因果） |
| 定向 `level_cap_reclamp_tests` | 5 passed / 0 failed（4 新 + 1 既有 `physical_order_shares_same_reclamped_target_as_ledger`） |
| `--nocapture` 读数 | `MED_C off_clock=2 unexplained=0 n_rescaled=0 planned=[(0,25),(1,5)]` |

与 resolution 自述的 1894/1 一致（此处是独立复跑，非采信）。

## Spec 轴（票面 ①–⑥）

**① 两施加点差集并的构造正确性 —— 成立。**
全仓 `clamp_levels_to_weighted_cap` 调用点经 grep 确认恰两处，均在 `fill.rs`（540 / 582），与「唯一施加入口」声明一致，无第三处漏记。`clamp` 实现为逐级 `map`（`coverage.rs:2824`），残差桶 early-return 透传、零项保留 ⟹ **保序等长**契约由构造保证，`levels_narrowed_by_cap` 的逐位 `zip` 比较即逐级比较，doc 与实现相符。`union_sorted_levels` 的并、去重、空表快路径均正确；输入升序由 `merge_levels`/`level_nets` 保证，故 doc 的「升序」claim 事实成立（尽管无消费者，见 LOW-2）。

**② bar 级派生与逐级判据同源 —— 成立且与旧行为等价。**
`cap_narrowed = !plan.cap_narrowed_levels.is_empty()`，单一来源。与修改前逐分支比对：①处旧 `gated != gated_pre_cap` ⟺ 新集合非空（等长同 level 前提下）；②处旧无条件 `true` ⟺ `reclamped != targets` ⟹ 至少一位值差 ⟹ 集合非空。**行为等价**，`risk_or_cap_active` 读数不变（既有 `lee_m4_cap_on_sparsity_has_no_unexplained_violation` 端到端仍绿即旁证）。

**③「②路径 rescaled 恒真、红点在①」—— 推导独立复核成立，实现覆盖两处。**
`attribute_total`（`level_attrib.rs:56`）在 `b_sum == total` 时逐字节返回 `basis`（`rescaled=false`）。②的触发前提是 `reclamped != targets`，即 `targets` 出 cap；而①已保证 `gated ≤ cap` ⟹ `targets ≠ gated` ⟹ 走非恒等分支 ⟹ `rescaled=true`。故②路径逐级红点不可触达，可触达红点在①——与 #362 评审推导一致。实装未据此省略②（`reclamp_path_records_cap_narrowed_in_plan` 明确登记该路径「修前即成立」），符合「同形同源、不留半修」，不构成 090 无效应代码（②的写入使 bar 级判据得以单源派生，有实际效应）。

**健全性补证（评审自行推导，实装未给）**：`regate`（`level_order.rs:456`）对无 tick 级别取 **`planned` 前值** ⟹ 恒等分支下无 tick 级别 `Δq_ℓ = gated_ℓ − planned_ℓ ≠ 0` ⟺ 该级被①裁过。且「他级被裁导致 Σgated 变化」不会外溢成本级的伪违例（未被裁的级 `gated_ℓ` 不变 ⟹ Δ=0）。故 `cap_narrowed_levels` 既**充分**（覆盖全部帽驱动 off-clock）又**不过度**（不赦免非帽级别）。此为该判据成立的关键前提，建议补进字段 doc（LOW-1 之外的可选项）。

**④ 红绿非平凡 —— 成立。**
`cap_narrowed_explains_per_level_off_clock_delta`：实测 `off_clock=2, n_rescaled=0, n_cap_narrowed=1`。旧判据为 `!rescaled ⟹ unexplained += off_clock_delta`，代入实测值 = 2 ⟹ **修前必红**，修后 0。三条非平凡前置（off_clock>0 / rescaled=0 / 帽真 binding）齐备，无平凡通过。
`per_level_explanation_is_level_scoped_not_plan_scoped`：判据层直测，`cap_narrowed_levels=[0]` 而两级均 off-clock ⟹ 断言 `unexplained==1`；plan 级 bool 形状下为 0 ⟹ 该测对「粒度退化」确有鉴别力。粒度选择与票体「**该级**」原文对齐。

**⑤ default 零行为变化 —— 成立。**
`enforce_level_cap=false` ⟹ 第二 `if` 整体跳过，`plan_gated` 恒置 `Vec::new()`（空 `collect` 无堆分配），`contains` 恒假 ⟹ 逐级判据 `!rescaled && !contains` **逐字节退化**为旧式 `!rescaled`；`n_cap_narrowed` 恒 0。`default_no_cap_path_is_unchanged` 用与 cap-on 场景逐字相同的构造反证两条路径干净分离（`rescaled=1 / n_cap_narrowed=0`，`planned=[(0,42),(1,8)]`），构造诚实。

**⑥ 对既有 cap-off / M3 读数的影响 —— 零。**
新判据在计数上**单调不增**（`unexplained_new ≤ unexplained_old` 恒成立：仅在 `contains` 命中时少计），且 cap-off 下逐字节相同。`LevelOrderStats` 无 `Serialize` derive，`n_cap_narrowed` 为新增字段、`Default` 派生，`bin/theta_overlay.rs` 打印面未触及 ⟹ 无输出 schema 变化、无下游读数漂移。

## Standards 轴

- **字段形状**：`Vec<u32>` 与既有 `LevelUnits = Vec<(u32, i64)>` 同族（level 升序表），未引入新容器范式；选 `Vec` 而非 `BTreeSet` 使 default 路径零分配，级别数 ~6 下 `contains` 线性可忽略——**权衡合理**。
- **`n_cap_narrowed` 声明与消费一致（090）**：字段 doc 明确写出「当前消费者仅 `level_cap_reclamp_tests` 三个单测，跑批验收侧尚未接入」，经核实恰为 3 个消费点，**无声明膨胀**。
- **doc ↔ 代码一一对应**：`level_order.rs` 三处 doc（两条路径、逐级判、解释项）与 `observe_decision` 实现逐条对得上；`fill.rs` 注释所述①②位置与代码位置一致。未见过度声明。
- 判断题④（`fill.rs` >800 行）归 #359 域、⑤（bool 结伴抽类型）超票面——两项均已在 resolution 登记为未做，评审确认属实且不阻断。

## 问题清单

**LOW-1｜`levels_narrowed_by_cap` 的契约破坏方向由「保守」反转为「漏报」**（`fill.rs:478`）
`filter(|(b, a)| b.0 == a.0 && b.1 != a.1)` 在 level 错位时**静默跳过**该位；`debug_assert_eq!` 只覆盖长度且 release 不生效。旧 `gated != gated_pre_cap` 在同样情形下取 `true`（保守：宁可多记 cap binding）。若日后 `clamp` 的保序等长契约被破坏，新实现的失效方向是「帽 binding 判假 ⟹ 稀疏性违例被漏报 + `risk_or_cap_active` 变假」。当前契约由同 crate 内 `map` 构造性保证，不可达；建议把 `b.0 == a.0` 从 `filter` 条件改为 `assert_eq!`（契约破 ⟹ 显式失败，不静默降级）。

**LOW-2｜`union_sorted_levels` 的「升序」是无消费者的声明**（`fill.rs:489`）
排序性事实上成立（输入升序 + `sort_unstable`），但唯一消费点 `observe_decision` 用线性 `contains`，不依赖有序。函数名与 doc 承诺了一条无人校验、无人使用的不变量——低度声明膨胀，且若日后有人据此改用 `binary_search`，空表快路径分支的有序性仅靠上游隐式保证。建议或在 doc 注明「有序性来自上游 `merge_levels`，当前无消费者」，或直接改名为 `union_levels`。

**登记确认｜跑批验收面未接逐级谓词（已有 #365，非本票缺陷）**
`runner.rs::lee_m4_cap_on_sparsity_has_no_unexplained_violation` 仍只断言 **bar 级** `sparsity_has_no_unexplained_violation()`；逐级谓词 `per_level_sparsity_has_no_unexplained_violation()` 与 `n_cap_narrowed` 非平凡前置在端到端跑批上**无断言**。即：本票的逐级修复目前只在单测面被证，跑批面未把关。resolution 已登记并转 #365，评审确认该缺口真实存在且范围判断正确——在 #365 落地前，「cap-on 逐级稀疏性成立」的有效域仅限单测构造场景。

## 结果包六要素

1. **结论**：#363 实装 PASS；Spec ①–⑥ 全项独立复核成立，Standards 三条无违规；2 项 LOW（防御性 / 声明），0 项阻断。
2. **定义依据**：逐级稀疏性 = `LevelOrderStats::per_level_sparsity_has_no_unexplained_violation`（「bar i 无 ℓ 级事件 ⟹ Δq_ℓ(i)=0」的逐级形式）；解释项合法性依 §F③ 域分离的 `CapTick`/`RiskTick` 例外。判定输入：`clamp` 逐级 `map` 的保序等长、`attribute_total` 的 `b_sum==total` 恒等分支、`regate` 无 tick 取 `planned` 前值——三条构造性事实共同支撑「集合充分且不过度」。
3. **边界条件**（结论翻转条件）：(a) `clamp_levels_to_weighted_cap` 不再保序等长 ⟹ LOW-1 升为真缺陷；(b) 出现第三个级别帽施加点而未并入差集 ⟹ ① 不再成立；(c) 上游新增在 `Σgated == t_lee_raw` 下仍改写 `targets` 的路径 ⟹ ③ 的「②路径 rescaled 恒真」失效（届时逐级解释项已就位，无需再补）；(d) `regate` 前值改取 `held` 而非 `planned` ⟹ 健全性补证失效，部分成交将产生伪违例。
4. **下游推论**：cap-on 场景的逐级稀疏性在**单测有效域**内成立（认识论 L1，合成构造）；跑批有效域待 #365。`n_cap_narrowed` 可作为后续 cap-on 验收的非平凡前置（#289 同类纪律）。
5. **谱系引用**：#351 MED-3（bar 级判据对帽结构性不可见）→ #355 MED-C（孪生判据半修）→ #362 评审推导（②路径红点不可达）→ 本票。090（声明与能力一致）为 Standards 轴判定依据；`formalization-validity-domain` 231 号为 L1/L2 标注依据。
6. **影响声明**：本评审仅产出本文件，未改动任何代码、未做 git mutation；`/tmp/kimi-nest-mainline` 全程只读，测试仅在 `/tmp/iso363` 运行。
