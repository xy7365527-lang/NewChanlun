# 影子评审：#356 级别帽下单量二次裁剪（t_lee 与归因账本同源，commit `ed52398192`）

- 票：#362（parent #59）；对象：`ed52398192`（`rust/src/theta_v0/backtest/fill.rs` +122/−2，单文件）
- 评审者：独立新上下文（claude opus，**非**实装 lineage）；只读，唯一写入 = 本文件
- 工作区：`/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD=`500b3ac4bb`（docs），对象为 HEAD~1
- 开评时 `git status` 干净（0 项，无删除流）；评审中途出现并发 lineage 污染，见 LOW-4（已隔离，未影响证据）

## 裁决

**核心修复成立，但票不宜按「验收四条全绿」收口**（HIGH×1 / MED×4 / LOW×4）。

- 代码层：裁定三条（下单量走 `t_lee'`、裁剪点统一、一致性断言接线）全部落地，两分支恒等经我独立推导为构造性成立，红绿**手工还原实测**复现（left 100 / right 60，与 resolution 逐值一致）。**不建议回滚代码。**
- 票层：编排者「验收输入增补」三条 MED 中，**MED-B/MED-C 未落实且未在任何处登记**，resolution 通篇未提该增补即声明「验收四条全绿」——按 090（声明与实际一一对应）须补一轮：或处置，或显式登记 deferred 并附推导。

## Spec 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| ① `t_lee_raw`/`t_lee` 命名拆分真消歧 | PASS | `t_lee_raw` 唯一消费者 = `plan_gated`（fill.rs:527/530，全文件仅此两处）；注释「不是最终发单量」与实际一致 |
| ② 两分支构造性恒等 `t_lee ≡ Σ_ℓ targets_ℓ` | PASS | 见下「恒等推导」；`plan_gated` 原样分支与 reclamp 重建分支各自成立（读源码非注释） |
| ③ 端到端红绿真触发重越 | 部分 PASS | 红绿实测成立（手工还原，release）；但前置断言只是代理（MED-4），全环路 cap-on 对本修复零可观测（MED-1） |
| ④ default（`enforce_level_cap=false`）零行为变化 | PASS | L0 纯整数退化成立（推导见下）；release 全量 1890 passed/1 failed 与票体基线逐值一致 |
| ⑤ `Σ_ℓΔq_ℓ ≡ ΔN`（M2）在新口径保持 | PASS 且严格改善 | 修复前 reclamp 态下双重分叉（`ΔN=t_lee_raw−p_t` vs `Σδ=t_lee'−t_prev`），且 `on_fill` 按更大的实际成交量回缩比例 ⟹ `held_ℓ` 可越 `cap_ℓ` 而 `planned_ℓ` 已裁；修复后分歧收敛为既有登记的 `max_abs_plan_fill_gap`（`p_t` vs `t_prev`） |
| ⑥ resolution 声明 ↔ 实际能力（090） | **FAIL** | HIGH-1（增补三条静默省略 + 约定引用不实）；另 MED-1 证据框定、MED-3 跨文件文档、LOW-1 数字 |

**恒等推导**（L0，全部读源码核实，不采信注释）：
`t_prev = planned_total() = Σ self.planned`（level_order.rs:396）；`attribute_total` 三分支均满足 `Σ out ≡ total`（level_attrib.rs:56-96，property 测试 level_attrib.rs:156 覆盖全形态）；`sub_levels = merge_levels` 覆盖 **level 之并**、缺席记 0（level_attrib.rs:99/109-138）⟹ `Σ deltas = Σ targets − Σ planned` 在 i64 上精确。
- 原样分支：`order_units = Σδ = Σtargets − t_prev`，且 `Σtargets ≡ target_total = t_lee_raw` ⟹ `t_lee = t_prev + order_units = t_lee_raw`（**default 路径逐字节退化，纯 i64 无浮点重排**，④ 成立）。
- reclamp 分支：`deltas = sub_levels(reclamped, planned)`、`order_units = Σδ` ⟹ `t_lee = Σ reclamped`（fill.rs:544-554）。
两分支均恒等 ⟹ `debug_assert_eq!`（fill.rs:568）确实只是防重构回退的护栏，不是可证伪证据（resolution 此处自陈准确）。

## Standards 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| `debug_assert_eq` 选型 + 「与本仓既有约定一致」标注 | **FAIL** | HIGH-1 第二段：约定是**成对**（debug 失败 + release 计数），#356 无 release 侧计数；且 level_order.rs:156-161 明文反对 |
| 函数体量既有债务增量登记照实 | PASS | `plan_level_gated_order` 148→**168** 行（fill.rs:473-640），远超 coding-style「<50 行」；resolution 登记为既有债务增量恶化、未处理——照实（数值见 LOW-3） |
| fill.rs 内注释 ↔ 代码行为一一对应 | PASS | #356 三条声明（同源 / 未 binding 时退化 / 构造性恒等）逐条与源码一致 |
| 跨文件文档同步 | **FAIL** | MED-3：`struct_gap` 文档声明的量在 reclamp 分支下已非物理目标 |
| 不可变性 / 命名 / 错误处理 | PASS | 新增全为 `let` 绑定，无新 mutation；无新增 unwrap/panic 路径 |

## 问题

**HIGH-1｜编排者「验收输入增补」MED-B/MED-C 未落实且未登记，resolution 仍声明「验收四条全绿」（090）**
#356 的验收输入增补（编排者评论）逐字要求三条。实际：
- **MED-B「修时一律用真 assert」未执行**：MED-2 的 cap 后置护栏仍是 `debug_assert!`（fill.rs:575-587），新增 #356 一致性断言亦为 `debug_assert_eq!`（fill.rs:568-574）。resolution 用「与本仓既有约定一致（见 fill.rs M3 稀疏性段）」背书——**该引用不成立**：M3 段的约定是**成对**结构「debug 立即失败 + release 由 `n_orders_off_structural_clock` 累计」（fill.rs:604-611），#356 无任何 release 侧读数；而 level_order.rs:156-161（#289 MED 同款纪律）明文「恒等证据必须在 release 下非平凡可读——只有 `debug_assert` 的恒等在 release 跑批里零执行，等于没有证据」。可辩护的一半：#356 的恒等已被我证为构造性成立，真 assert 恒不触发、release 计数恒 0，选 debug 护栏是合理工程判断——但那需要**写出这条推导**并声明偏离编排者指令，而非引用一条不成立的约定；MED-B 的实质靶子（`plan.targets` 越 cap 的**非构造性**不变量）至今在 release 下零执行。
- **MED-B 第二半「端到端覆盖缺口须照实登记」未登记**：我 debug 实测两个 cap-on 9000-bar 测试（`lee_m4_level_cap_narrows_position_when_enabled` / `lee_m4_cap_on_sparsity_has_no_unexplained_violation`）仍双双崩于既有 `coverage.rs:2606`（`next_active 含重复 ElementId`，pre-existing，同 #355 记的 2594 同源）⟹ 叠加 release 剥离，MED-2 cap 护栏在全环路任何构建下仍零执行。
- **MED-C 未接线**：reclamp 重建仍用 `..plan`（fill.rs:551），`rescaled` 不更新；无 cap-on 的 `per_level_sparsity_has_no_unexplained_violation()` 断言（该判据已存在，level_order.rs:292，仅被 cap-**off** 的 M3 测试消费，runner.rs:2476）。**缓解（我的推导，commit/resolution 均无）**：reclamp 重写 ⟹ `rescaled` 必为 true——三分支穷举：`Σgated==t_lee_raw` 时 `attribute_total` 逐字节返回 `gated`（已过①处 clamp，全部 ≤cap）⟹ `reclamped==targets` 不重写；另两分支 `rescaled=true`。故逐级 unexplained 误计**不可达**，MED-C 无活体缺陷，但「验收须含该断言」未兑现。
- **部分抵扣（正面）**：新测在 debug 下让 MED-2 的 `debug_assert!` 首次真正执行（单元级，我实测 `cargo test --lib physical_order_shares` 绿）。

**MED-1｜唯一全环路 cap-on e2e 对本修复完全不可观测（我实测坐实）**
手工把 `t_lee` 还原为 `t_lee_raw` 后，`lee_m4` 两测读数**逐值不变**：`baseline 8055/48 | capped 761/169`、`off_clock=145 explained=145`。⟹ 9000-bar fixture 上 reclamp 分支从不 binding，本修复在**唯一**全环路 cap-on 覆盖上零效应。resolution 反把这些不变数值列为「default 不变」证据（其中 `761/169` 是 cap-**on** 臂，与「default 不变」无关），并未登记「修复路径全环路零覆盖」这一边界。物理路径的唯一证据 = 单元级直调新测（票体「非纯函数」字面满足，但**不是**全环路）。

**MED-2｜「物理下单量不越 Σcap_ℓ」在残差桶路径仍为假（L0 推理，可达性未实测）**
`Σgated=0 ∧ t_lee_raw≠0` 时 `attribute_total` 把全额落 `LEVEL_ACCOUNT_RESIDUAL`（level_attrib.rs:62-67）；`clamp_levels_to_weighted_cap` 对该桶**原样透传**（coverage.rs:2827-2829），fill.rs 的 cap 断言亦显式豁免它（fill.rs:578-584）⟹ `t_lee = Σtargets` 的上界只剩账户层 `γ̄·U`，可 > `Σcap_ℓ`。且残差桶经 `commit_planned`→`regate` 跨 bar 沉淀，无 tick 时保前值持续绕过。**非本次回归**（修复前 `t_lee_raw` 同样不受级别帽约束），但票体验收字面「物理下单量不越 Σcap_ℓ」的有效域 < 声明域（231），新测只覆盖比例缩放一支，未登记该边界。

**MED-3｜`struct_gap` 语义静默漂移（090，跨文件未同步）**
level_order.rs:194-196/311-312 声明 `max_abs_struct_gap = max|Σ_ℓ basis^gated_ℓ − T_lee|`，`T_lee` 逐字为「账户层投影后**物理**目标」。#356 后 reclamp 分支的物理目标是 `t_lee'`，而 `struct_gap` 仍按 `t_lee_raw` 计算（level_order.rs:455，`plan_gated` 内）⟹ cap-on 且 reclamp binding 时，这个 L1 登记读数不再是它文档声明的量。单文件 commit 未同步该文档。

**MED-4｜新测非平凡性靠代理断言，可静默平凡化**
前置 `assert!(targets_sum < 100)` 只在「`t_lee_raw` 恒 = 100」时等价于「reclamp 真触发」。我的红态实测坐实**今日** `t_lee_raw=100`（物理端 100 vs 账本 60）⟹ 场景今日非平凡；但若上游（`pi_theta_position`/账户 cap/lot 网格/`base_units`）漂移使 `t_lee_raw<100` 而各级未越帽，则 reclamp 不触发、`assert_eq!(60,60)` 平凡绿——本修复的**唯一**护栏静默失效（叠加 MED-1 = 全无覆盖）。宜直接钉 `cap_narrowed`/`rescaled` 或 `t_lee_raw` 本身。

**LOW-1**｜「修复前后均 1890 passed / 1 failed」算术不成立：diff 净增 1 个 `#[test]`、零删除；我实测 HEAD = **1890 passed / 1 failed**（1891 跑）⟹ 父提交必为 1889/1（与 #355 影子评审在 #350 态实测 1889、且 #358 零新增测试一致）。`ea027130f7`（#358）commit message 已含同一 off-by-one，本票沿用未复核。
**LOW-2**｜新测 per-level cap 映射 `if lvl == 0 { cap_0 } else { cap_1 }`（fill.rs:2997）：`lvl≥2` 与残差桶都会被按 `cap_1=10` 校验，断言措辞「归因账本已裁到位」强于其实际覆盖（本场景 `lvl∈{0,1}` 故今日无误判）。
**LOW-3**｜函数体量：`plan_level_gated_order` = fill.rs:473-640（**168 行**，父提交 148 行），远超 coding-style「函数 <50 行」；fill.rs 3024 行远超「800 max」。resolution 登记照实（既有债务增量恶化，未处理）——本条仅记增量数值，不追加要求。
**LOW-4**｜工作区并发污染（**非本 commit**）：评审中途 `runner.rs` 出现另一 lineage 的 #361 临时诊断测试（`diag361_multi_confirm_bits_sequences`，`#[ignore]`，落盘 04:25:59）。我的 release 全量基线完成于 04:24:13（未受污染）；我的临时还原只碰 fill.rs，`git checkout --` 后与 commit blob **逐字节一致**（已用 `diff <(git show ed52398192:…)` 核）。评审未做任何 git mutation（无 add/commit/stash/branch）。

## 复跑

- `cargo test --release --lib`：**1890 passed / 1 failed**（唯一失败 `extract_signals_bit_exact_digest_guard`，#115 线 pre-existing，未修未归因）✓ 与票体基线逐值一致
- 定向 release：`fill::level_cap_reclamp_tests::physical_order_shares_same_reclamped_target_as_ledger` ok；`lee_m4`×2 ok（`8055/48 | 761/169`；`off_clock=145 explained=145`）
- **红绿独立复现 = 手工还原实测**（非纯推理）：fill.rs:567 `t_lee = t_prev + plan.order_units` → `t_lee = t_lee_raw`，release 下新测 **FAILED**（`left: 100 / right: 60`，与 resolution 声明逐值一致）；同还原态 `lee_m4` 读数**逐值不变**（MED-1 依据）；复原后校验与 commit blob 一致
- `cargo test --lib`（debug 取证）：新测 ok（⟹ #356 `debug_assert_eq!` 与 MED-2 `debug_assert!` 在该路径真执行）；`lee_m4` 两 cap-on 测试 **FAILED** 于 `coverage.rs:2606`（pre-existing，非本 commit 引入）

## 结果包

1. **结论**：`ed52398192` 代码层通过（裁定三条落地、恒等构造性成立、红绿实测复现），**不建议回滚**；票层因编排者验收增补 MED-B/MED-C 未落实且未登记（HIGH-1）不宜按「验收四条全绿」收口，须补一轮处置或显式 deferred 登记。MED-1/2/3/4 均为「有效域 < 声明域」型，建议同批订正措辞与护栏。
2. **定义依据**：#356 票体 + 裁定评论（下单量走 `t_lee'`/裁剪点统一/`Σq_ℓ≡t_lee'` 接线）+ 验收输入增补（MED-A/B/C）逐条对照；判据取「注释/resolution 声明 ≤ 实装能力」（090）与「有效域 ≤ 定义域」（231）。恒等与 default 退化经源码（level_order.rs:396/451-457、level_attrib.rs:56-138、coverage.rs:2813-2835）独立推导，未采信注释。
3. **边界条件**：HIGH-1 与 MED-1/2/3/4 全部以 `enforce_level_cap=true` 为前提，default（false）下不可达 ⟹ 若 M4 长期只作 default-off 备用开关，HIGH-1 可降 MED。MED-1 若 `coverage.rs:2606` 既有缺陷被修（#350 域），debug 全环路打通后本修复即获全环路证据，MED-1/HIGH-1 后半自动解除。MED-2 若能证「残差桶在生产 config 下不可达」（需 `Σgated=0 ∧ t_lee_raw≠0` 的可达性判定），可降 LOW。MED-4 的红态非平凡性只对**今日**上游成立。
4. **下游推论**：(a) 级别帽的「物理不越 Σcap_ℓ」若要成为无条件声明，必须处置残差桶（要么纳入某个帽，要么把该声明限定为「真实级别桶」）；(b) `struct_gap`/`rescaled`/`used_residual_bucket` 三个 `..plan` 透传字段的语义都以 `t_lee_raw` 为锚，与新物理口径已分家，M4 后续读数解释须按此订正；(c) 只要 `coverage.rs:2606` 未修，任何依赖 debug 断言的 cap-on 护栏都拿不到全环路执行证据——这条对 #59 线后续所有 M4 票同样成立。
5. **谱系引用**：090（严格性/声明膨胀）→ HIGH-1、MED-3 定级；231/formalization-validity-domain（有效域≠定义域）→ MED-1/MED-2 定级；#289 MED「恒等证据须 release 可读」→ HIGH-1 的约定判据来源；no-patch-mentality「TODO 遗留」→ 增补静默省略的定性。未触及既有概念分离谱系的新分歧；本次未发现定义层矛盾（无需 `/escalate`）。
6. **影响声明**：本次评审只读，唯一写入 = 本文件。零 git mutation（无 add/commit/stash/branch/worktree）；为红态取证对 `fill.rs:567` 做过一次临时工作区改动，已 `git checkout --` 复原并逐字节核验；`runner.rs` 的并发 lineage 改动未被我触碰。
