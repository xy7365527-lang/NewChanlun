# 影子评审：#310 LEE M4 级别 sizing/risk（issue #349）

- 评审对象：commit `7d8b45be70`（9 文件，level_risk.rs 新建 + 8 修改）
- 评审者：独立新上下文 claude lineage（非实装 lineage），只读
- 日期：2026-07-27；worktree `/tmp/kimi-nest-mainline`，评审开始时 `git status` 干净（无删除流）
- 结论：**无 HIGH，不回票**；4 MED + 2 LOW

## 复跑

| 项 | 结果 |
|---|---|
| `cargo test --release --lib` | **1879 passed / 1 failed / 133 ignored**——唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#115 线，未修未归因） |
| 定向 `level_risk / lee_m4 / level_weights / level_cap` | **7 passed / 0 failed** |
| M4 集成读数实测 | `baseline max_abs_net_units=8055 n_orders=48 | capped 761 / 169`——与 commit message、runner.rs:2769 文档数字逐值一致 |

## Spec 轴（票 #349 核对点 + 设计文档 §D M4:143 + #309 评审四 MED）

| 核对点 | 判定 | 证据 |
|---|---|---|
| M4 守界：default 关 ⟹ M0–M3 bit-exact | **PASS** | `fill.rs:495` `if risk.enforce_level_cap` 短路；`config.rs:227-228` default 空表/false；M3 golden `RISK_TRIGGER_SURFACE_GATE_OFF_SNAPSHOT=1191` 与全部 M0–M3 测试原值通过 |
| Σw_ℓ ≤ 1 证明强度 | **FAIL** | MED-1 |
| 𝒦_Θ 协变 cap 按级别分解后仍满足（代数） | **PASS** | `coverage.rs:2677` `level_cap = feasible_net_cap·w_ℓ·|U_ℓ|` 与生产账户层 cap `coverage.rs:2882` 同口径；`level_cap_decomposition_never_exceeds_account_cap` L0 |
| 同上（作用到最终 targets） | **FAIL** | MED-2 |
| 对偶统一声明与实装一致 | **PASS** | `depth_weight` 施加点仅 `coverage.rs:1555/1579`（leg 生成，结构域）；`w_ℓ` 施加点仅 `fill.rs:495`（regate 后、`pi_theta_position` 前）——两集合不交，无双重定价 |
| 参数 Θ_risk 归属（禁冒充缠论可导） | **PASS** | `config.rs:200-206` + `level_risk.rs:39-44` 两处显式标注 |
| 四前置① RISK_FACE 改名/诊断指引/golden 再生 | **PASS** | `runner.rs:2583-2604`；再生流程需手改生产代码（非自动化），票体 resolution 已登记为「弱于字面接受」 |
| 四前置① 多级 fixture ≥2 真实并发 | **PASS** | 新 `LevelOrderStats::max_concurrent_real_levels`（`level_order.rs:248/495-499`）+ `>=2` 机器断言，9000-bar 实测绿 |
| 四前置② level_order.rs stale L2 → L1 | **PASS** | grep 残留 `L2` 全部为订正说明文本或级别标号（`level_order.rs:566` `L2:6`），无 stale 标注 |
| 四前置③ 第③锚补文档 + plan_fill_gap 可观测 | **PASS** | `fill.rs:423-441` 表新增第③行并照实登记副作用；`runner.rs:2802` `max_abs_plan_fill_gap > 0` 实测绿 |
| 四前置④ first-observation fixture 非平凡 | **FAIL** | MED-4 |

## Standards 轴（090 / 模块形状 / 避退化 / 去 Copy / 意外发现）

| 核对点 | 判定 | 证据 |
|---|---|---|
| `level_risk.rs` 模块形状 | **PASS** | 105 行、3 个纯函数、无状态、无 IO；权重定义与帽施加分离（施加在 `coverage.rs`），符合 coding-style「小文件高内聚」 |
| 集成验收避退化设计 | **PASS** | 刻意取 w=0.05（非 0）使两侧 `n_orders` 均非零，`runner.rs:2755-2761` 论证充分；实测复现一致 |
| RiskConfig 去 Copy 的必然后果登记 | **FAIL** | LOW-1 |
| 意外发现只登记未裁决（090） | **PASS** | `runner.rs:2661-2666`（`newly_confirmed_step` 3500+ bar 多次确认）与 `level_attrib.rs:32-41`（残差桶无 clock 事件类）均照实登记、未改语义、未擅自裁决 |

## 问题清单

### MED-1｜`Σw_ℓ ≤ 1` 从未接入生产路径，doc 声明强度超出实装（090 声明膨胀）
`level_risk.rs:64` 的 doc 写「**机器断言**（不是文档承诺——本函数是唯一权威判据）」，`config.rs:206` 复述同一强度。但 grep 全仓：`level_weights_sum_le_one` 的调用者只有本模块单测与 `runner.rs:2791` 的 M4 集成测试；`level_cap` / `clamp_levels_to_weighted_cap` / `fill.rs` 门控点**一次也不调用**，`RiskConfig` 也无构造期校验。
后果：配置 `level_weights=[0.6,0.6]` 时 `Σcap_ℓ = 1.2·γ̄·U_ℓ > 账户层总 cap`，票体验收项「𝒦_Θ 协变 cap 按级别分解后仍满足」的唯一前提被静默突破，无任何拦截。实际持仓仍受 `pi_theta_position` 的总 cap 约束 ⟹ 不造成穿仓，但级别帽形同虚设且用户无从察觉。
修法（不在本票内改）：`fill.rs` 门控分支加 `debug_assert!(level_weights_sum_le_one(risk))`，或在 `clamp_levels_to_weighted_cap` 入口断言。

### MED-2｜帽施加在 basis 上而非最终 targets；「不留一条未裁剪的旁路」自相矛盾
`fill.rs:491-494` 注释断言「级别帽在结构基准进入账户层之前统一生效，**不留一条未裁剪的旁路**」。但同一函数下方 8 行即是 `p_tilde_lee = Σ gated + pan_div_child_units`——`pan_div_child_units` 不经 `net_ℓ`、不经帽，正是一条未裁剪旁路（该事实由紧邻注释自己写明）。
更实质的是施加位置：帽裁的是 `gated`（basis），而最终写账本的是 `plan_gated → attribute_total(gated, t_lee)`（`level_attrib.rs:56`）。`attribute_total` 在 `Σbasis ≠ total` 时按比例**缩放**——`t_lee` 因 pan_div 加项或 π_Θ 的 lot 量化而大于 `Σ gated` 时，各级 target 被同比抬高，可重新越过 `cap_ℓ`。无任何断言锁 `|targets_ℓ| ≤ cap_ℓ`。
默认 `ThetaConfig` 下 pan_div 惰性 ⟹ 当前不可达；但注释断言本身在写下时即为假（090：代码能做什么就声明什么）。

### MED-3｜稀疏性在 cap 开启后零覆盖，且解释项对帽结构性不可见
`cap_ℓ = w_ℓ·γ̄·U_ℓ`，`U_ℓ = equity_nav/px` 逐 bar 重算（`fill.rs:883`）⟹ 无 tick 的 bar 上 `gated` 虽走保前值分支，clamp 结果仍随价格漂移 ⟹ `t_lee` 变动 ⟹ 发单。M3 硬约束「订单时点 ⊆ 事件时点并集」在 M4 开启后**没有任何断言覆盖**：`lee_m3_order_ticks_are_sparse_subset_of_clock_events` 用 `ThetaConfig::default()`（帽关），`lee_m4_level_cap_narrows_position_when_enabled` 不读 `n_orders_off_structural_clock`。
且解释项失灵：`fill.rs` 的 `risk_or_cap_active = risk_gate_active || |p_star_lee − p_tilde_lee| ≥ 0.5`，而帽在 `p_tilde_lee` **之前**施加 ⟹ 帽引起的偏离对该判据恒不可见（`level_order.rs:509`），帽驱动的 off-clock 订单会被计为**未解释**违例。
实测 `n_orders` 48→169（3.5×）恰是这条压力的直接信号，交付说明将其解释为「帽收紧反而逼出更多再平衡订单」，但未验证这些订单是否落在 clock 事件上。建议 M5 前补一条 cap-on 的稀疏性/解释率断言。

### MED-4｜#309 MED-4 处置强度弱于同批 MED-1，非平凡前置未加固
`runner.rs:2668` 把 fixture 从 1500→3000 bar，注释称「3000-bar fixture 产 6 个 tick」。但断言仍是 `assert!(n_bsp_ticks > 0)`（`runner.rs:2700`）——正是 #309 MED-4 指控的「`>0` 恰被最弱情形满足、零区分力」那条本身。6 tick 只是注释里的口头数字，上游分类器/fixture 任何变动使其退回 1 tick，测试照绿，「同身份至多响一次」重新退化为构造上不可失败。
对照同批 MED-1 的处置：加了 `max_concurrent_real_levels >= 2` 机器断言。同一评审批次内两条 MED 的处置强度不一致；MED-4 缺一条对应的 `n_bsp_ticks >= 2`（或 distinct identity ≥ 2）下界。

### LOW-1｜`RiskConfig` 去 `Copy` 未登记，且未论证 `Vec<f64>` 的必然性
`config.rs:165` `#[derive(Debug, Clone, Copy, PartialEq)]` → `#[derive(Debug, Clone, PartialEq)]`，由新增 `Vec<f64>` 字段强制。这是 `RiskConfig` 全局值语义的变更（Copy → Clone），票体 Standards 轴点名要求「去 Copy 的必然后果登记」，但 commit message、`config.rs` doc、`level_risk.rs` 模块头**均无一字**提及。
且必然性未论证：`level_attrib.rs:44` 已写明「级别数常态 ≤ 6」，`[f64; 6]`（或 `[f64; MAX_LEVELS]`）可保 `Copy` 且免逐 bar 堆间接寻址，与既有 `depth_weights` 的选型未做对照。本次无下游破坏（全仓无一处需补 `.clone()`，编译即证），故仅 LOW——但登记项确实缺失。

### LOW-2｜单测 doc 与测试体不符
`level_risk.rs:99` doc 写「★**空表**时禁止权重恰好落在合规边界之外的浮点误差误报」，但 `level_weights_sum_le_one_tolerates_float_noise_at_boundary` 测的是 `[1/3, 1/3, 1/3]`，与空表无关（空表已由上一测覆盖）。措辞误植，不影响行为。

## 结果包（六要素）

1. **结论**：#310 commit `7d8b45be70` 通过影子评审，无 HIGH，不回票；4 MED + 2 LOW 见上，建议 MED-1/MED-3 在 M5 前补断言，MED-2 的注释断言应订正为如实措辞。
2. **定义依据**：设计文档 `multi-level-native-execution-design-20260719.md:143`（§D M4 逐字：Σw_ℓ≤1；𝒦_Θ 协变 cap 按级别分解后仍满足；w_ℓ 全属 Θ_risk 禁冒充缠论可导）；#349 票体核对点；#310 第一条评论（#309 评审四 MED）。
3. **边界条件**：全部 MED 均以 `enforce_level_cap=true` 为前提；default（false/空表）下四条皆不可达，M0–M3 bit-exact 声明成立。MED-2 的旁路项额外需 pan_div 非惰性。若主控裁定 M4 长期只作 default-off 的备用开关，MED-1/2/3 可降为 LOW。
4. **下游推论**：M5（生产加固）若打算实际启用级别帽，须先补 (a) Σw_ℓ 生产期校验、(b) cap-on 的稀疏性覆盖、(c) `|targets_ℓ| ≤ cap_ℓ` 后置断言——否则「级别风险帽」在生产语义下不是硬约束。
5. **谱系引用**：090（严格性/声明膨胀禁止）→ MED-1/MED-2 的定级依据；`formalization-validity-domain`（231号）→ 本票 L2→L1 订正批的判据，已正确执行。未发现涉及既有概念分离谱系的新分歧。
6. **影响声明**：本评审为只读，除本报告文件外未改动仓库任何文件；未提交、未 reopen 任何 issue。
