# 影子评审：#351 M4 级别帽四 MED 补课（commit `19aea33a26`）

- 票：#355（parent #59）；对象：`19aea33a26`（fill.rs +68 / runner.rs +105 / coverage.rs +68）
- 评审者：独立新上下文（claude opus，非实装 lineage）；只读，唯一写入 = 本文件
- 工作区：`/tmp/kimi-nest-mainline`，`git status` 干净（0 项，无删除流）；HEAD=`760c3520c5`（#350），对象为 HEAD~1

## 裁决

**通过，不回票**（无 HIGH）。MED×3 / LOW×5。

## Spec 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| MED-1 真 `assert!` 位置 + release 生效 | PASS | `coverage.rs:2806`（唯一施加点入口）；实测 Σw=1.2 panic |
| MED-2 二次 clamp + 残差桶透传 | PASS | `fill.rs:532-554`；`coverage.rs:2775-2778` 透传；红绿测绿 |
| MED-2 注释「不留旁路」一致性 | **FAIL** | MED-A |
| MED-3 `cap_narrowed` 两点归属 145/145 | PASS | 实测 `off_clock=145 explained=145` |
| MED-3 逐级孪生判据 | **FAIL** | MED-C |
| MED-4 distinct≥2 + 1500-bar 回归 | PASS | 实测 `distinct_identities=1`，判不足 |

## Standards 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| `assert` vs `debug_assert` 选型 | PASS | `coverage.rs:2769-2773` 论证正确（配置校验≠内部不变量） |
| `plan_level_gated_order` 行数 | FAIL | LOW-4 |
| t_lee 深层缺口登记照实 | 部分 PASS | #356 已开，措辞窄化见 LOW-3 |

## 问题

**MED-A｜「不留旁路」在物理下单层仍为假（090 声明膨胀重演）**
`fill.rs:498-502` 断言「两次裁剪合起来才是不留旁路」「`pan_div_child_units` …不再是不受限的旁路」。但下单量 = `schedule_order(t_lee, p_t)`（`fill.rs:577`），`t_lee` 由 `p_tilde_lee`（含 `pan_div_child_units`，`fill.rs:520`）经投影得出，**从未二次裁剪**——同 commit 自己在 #356 承认这一点。#310 MED-2 指控的「注释断言写下时即为假」未被消除，只换了一句更长的。订正方向：注释限定「账本层」，物理层明标未兑现。

**MED-B｜MED-2 的后置护栏零执行**
`fill.rs:555-567` 的 `debug_assert!`（二次裁剪后无级别越 cap）在 release 编译掉；debug 下唯二 cap-on 测试（`runner.rs:2851`/`2893`，均 9000-bar）实测双双崩于既有 `coverage.rs:2594`（4000-bar 起触发，已登记跨票缺陷）——**任何构建下均未执行过**。叠加 commit 自认「9000-bar 该场景 0 次触发」，MED-2 在 `fill.rs` 层的验证等级仅 L0（纯函数），端到端零覆盖。

**MED-C｜逐级稀疏性判据未同步修**
`cap_narrowed` 只并入 bar 级 `risk_or_cap_active`；逐级 `n_levels_off_clock_delta_unexplained` 的解释项仍只有 `plan.rescaled`（`level_order.rs:501-503`），二次裁剪路径的 `..plan` 未更新 `rescaled`（`fill.rs:548`）。新测 `runner.rs:2912` 也只断言 bar 级，未断言 `per_level_sparsity_has_no_unexplained_violation()`（cap-on 下仍零覆盖）。同一 MED 的孪生判据只修一半。

**LOW-1**｜`gated_pre_cap = gated.clone()`（`fill.rs:507`）无条件克隆，默认 M0–M3 路径逐决策点多一次分配（行为 bit-exact 不变）。
**LOW-2**｜`clamp_..._rejects_misconfigured_weights_over_one` 用 `catch_unwind` 而非 `#[should_panic(expected=…)]`：不锁 panic 消息，且输出 panic 噪音（实测可见）。
**LOW-3**｜#356 措辞「只改内部归因账本 `plan.targets`」窄化——二次裁剪同时改 `order_units`（发不发单判据，`fill.rs:569`）与 `commit_planned` 的计划态（下一 bar `t_prev` 锚，`fill.rs:522/618`）。
**LOW-4**｜`plan_level_gated_order` = `fill.rs:473-620`（148 行，语句 ~53）超 coding-style「函数 <50 行」；二次裁剪块可外提至 `coverage.rs` clamp 家族。fill.rs 2904 / coverage.rs 6913 行远超「800 max」（既有，本次未恶化数量级）。
**LOW-5**｜1500-bar 回归 `assert_eq!(fired.len(), 1)` 硬钉上游产出，脆；漂移处置已在注释登记，可接受。

## 复跑

- `cargo test --release --lib`：**1889 passed / 1 failed**（`extract_signals_bit_exact_digest_guard`，#115 线，未修未归因）✓ 与票体一致
- 定向 8/8 绿：`clamp_levels*`、`attribute_total_scaling*`、`bsp_identity*`、`lower_bound_catches_shrunk`、`lee_m4_cap_on_sparsity`、`level_weights_sum*`
- `cargo test --lib`（debug，用于 MED-B 取证）：两 cap-on 测试 FAILED 于 `coverage.rs:2594`（pre-existing，非本 commit 引入）

## 结果包

1. **结论**：`19aea33a26` 通过，无 HIGH，不回票；MED-A/B/C 建议在 #356 同批处置（三者与 t_lee 缺口同源）。
2. **定义依据**：四 MED 逐条对照 `shadow-review-310-20260727.md` 原文与 #355 票体核对点；判据取「注释断言 ≤ 实装能力」（090）与「有效域 ≤ 定义域」（231）。
3. **边界条件**：MED-A/B/C 全部以 `enforce_level_cap=true` 为前提，default（false）下不可达，M0–M3 bit-exact 声明成立；若 M4 长期只作 default-off 备用开关，三条可降 LOW。MED-B 若 `coverage.rs:2594` 既有缺陷被修（#350 域），debug 跑批打通后该护栏即获执行证据，MED-B 自动解除。
4. **下游推论**：#356 的裁定应覆盖三处而非一处——下单量 `t_lee`、发单判据 `order_units`、计划态 `commit_planned`；逐级解释项须同批并入 `cap_narrowed`。
5. **谱系引用**：090（严格性/声明膨胀）→ MED-A 定级；231/formalization-validity-domain → MED-B 的 L0 vs L2 等级判据。未触及既有概念分离谱系的新分歧。
6. **影响声明**：本次评审只读，唯一写入为本文件；未改任何源码、未动 git 状态、未干扰并行 #356 工作。
