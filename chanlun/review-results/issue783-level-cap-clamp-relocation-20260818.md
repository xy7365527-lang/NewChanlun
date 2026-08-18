# #783 返工留痕——级别帽 clamp 前移到投影前（shadow-review-755 HIGH×2 清偿）

- 票：GitHub #783（[task] #755 返工：级别帽 clamp 前移到投影前）。父票 #755；评审票 #777。
- 评审报告：`chanlun/review-results/shadow-review-755-20260729.md`（判 FAIL，HIGH×2 阻断）。
- 工位：`sandcastle/issue-783`（main 尖端切出），单线程前台，禁子代理。
- 改动面：12 文件（Rust 源码 9 + 测试 1 + 文档/留痕 2），+366 / −73 行。

## 1. 根因（照评审 §5 坐实，非纸面推理）

#755 把 M4 级别帽施加在账户层投影（`pi_theta_position`）**之后**：`fill.rs` 内取
`attribute_total(level_nets(sep_legs), standard_p_star.round())` 逐级二次裁剪、重新求和后直接
覆盖 `order`，**不回 `KThetaRiskGate` 可行集**，且 `sep_legs`/`standard_p_star`/腿级账本全部保留
未裁剪值。后果（BTC 200k 实测）：

- **HIGH-1**：帽后标量可落到 gate 可行集之外——票内配置越界 4 次、非对称权重越界 9 次；
  bar 8941 账户层唯一出口 `p*=0`（平仓）+ 风控 `stop_short`（禁净空）被二次裁剪成 −568 手净空实单。
- **HIGH-2**：帽只裁标量 `order`，腿级账本读未裁目标 ⟹ 两账分裂 31 倍；三条守恒断言两边都读
  未裁那份，对分裂不可见。

## 2. 修法（评审指明第三方案，冲突清单②原「二选一」遗漏的第三方案）

把 clamp **前移**到 `pi_theta_step_*` 之前，施加于 `level_nets`——不需要 `LevelOrderLedger`、
不动 M3 event clock，同解 HIGH-1/HIGH-2。

落点：`coverage_step_from_buckets_sep_with_risk_seeds`（`coverage/step.rs`）内、重内单向
（#879）之后、`p_tilde = net_target_units(&legs)` 折叠**之前**，新增

```rust
if let Some(r) = risk {
    if r.enforce_level_cap {
        let _level_cap = super::leg::apply_level_cap(&work, &mut legs, base_units, r);
    }
}
```

新函数 `apply_level_cap`（`coverage/leg.rs`，G7 `apply_gross_cap` 同款「legs 折叠前改 units」模式）：

1. `level_nets_from_legs`：按 `level_nets(sep_legs)` **同口径**（`round(units/lot)·lot` 取整、
   `q≤0` 剔除、`side_sign` 定号）从 `LegTarget` 算各级整数净额 `net_ℓ`；
2. `clamp_levels_to_weighted_cap(&nets, base_units, risk)`：**同一 clamp**（复 `level_cap`，
   `cap_ℓ = w_ℓ·γ̄·U_ℓ`，floor 到整数单位）；
3. 按 `clamped_ℓ/net_ℓ` 逐级缩放该级所有腿（各级净额同号 ⟹ 因子恒非负；`net_ℓ==0 ⟹ 1.0`）；
4. 此后 `p_tilde`（账户层投影输入）与 `sep_legs`（腿级账本）读**同一**裁剪后值。

随之删除 `fill.rs` 的投影后二次裁剪整段（原 M4 接线段），并把 #351 MED「Σw≤1 校验」按 kimi
原意**钉回 `clamp_levels_to_weighted_cap` 函数入口**（`assert!` 非 `debug_assert!`，release 生效）。
新增 HIGH-1 回归锁：`fill.rs` 内 `debug_assert_eq!(order, schedule_order(standard_p_star, p_t, exec))`
——`order` 必须是 `standard_p_star` 经 §16 单一决策出口的唯一产出，任何投影后覆盖 `order` 立即红。

## 3. 验证读数

| 项 | 读数 |
|---|---|
| `cargo check --lib` | 0 error（基线 58 warning → 现 55 warning；删 3 项 = `unused imports: clamp_levels_to_weighted_cap/level_cap`（1）+ `level_cap`／`clamp_levels_to_weighted_cap` 死代码（2），本票不再新增） |
| `cargo check --all-targets` | 0 error |
| `cargo test --lib` | **2749 passed / 0 failed / 153 ignored**（零新增红；+4 = 本票 `apply_level_cap` 四单测） |
| `cargo fmt --check` | 净 |
| 门关 golden（default `enforce_level_cap=false`） | `apply_level_cap`/`clamp_levels_to_weighted_cap` 零执行路径（step.rs 门禁 default off，frozen M0-M3 bit-exact）；新增 `debug_assert_eq!` 只重算恒真等式，不改任何决策值 |

新增 `apply_level_cap` 四单测（`coverage/leg_tests.rs`）：① 超帽级别按 `clamped/net` 等比缩放、
未超帽原样 + binding 计数逐级；② 空腿各级对称裁到 ±cap；③ 未超帽逐位不动（bit-exact）；④ 表外
级别权重 ⟹ cap=0 ⟹ 该级归零。

**HIGH-1/HIGH-2 的 BTC 200k 双配置实测**：本工位无真实 BTC 数据（`btc_1m_full.json` 缺席、
`/private/tmp/kimi-nest-mainline` 只读参照面不存在），未重放 200k 双跑。结构保证替代实证：

- 越界零次：`order` 恒 = `schedule_order(standard_p_star, …)`，`standard_p_star` 由
  `pi_theta_position` 在 `𝒦_Θ` 可行集上 `LexArgmin` 产出 ⟹ 订单目标恒在 gate 可行区间内，
  且上述 `debug_assert_eq!` 锁死「无投影后二次裁剪覆盖」。
- bar 8941 形态（`p*=0` 被裁成净空）：已无「投影后覆盖 `order`」路径，`p*=0 ⟹ order` 恒为
  Hold/Close（qty=0），不可能变成净空实单。
- 两账分裂：`sep_legs` 与 `p_tilde` 由同一份裁剪后 `legs` 折叠/打包，M5 ΔN、LEE-Net、
  reconcile_residual 三条守恒断言现读**裁剪后**一侧（原断言未弱化；新增 HIGH-1 回归锁）。

## 4. 尾件

1. **三处生产 doc 悬空引用 `plan_level_gated_order` 订正**：`config.rs`（`enforce_level_cap` 字段
   doc 指向新施加点）、`level_risk.rs` 模块头（`level_cap` 悬空链接一并订正为
   `clamp_levels_to_weighted_cap`）、`level_order.rs:355`（`cap_narrowed_levels` 照实订正为恒空）。
   另顺手清同族悬空引用：`level_order.rs:268/392`、`level_clock.rs:33`、`level_order.rs:202`
   （`fill.rs::level_cap_reclamp_tests` 已随 #363 删除）。
2. **`coverage/mod.rs` unused import 清理**：re-export 去掉 `level_cap`（仅
   `clamp_levels_to_weighted_cap` 内部消费，跨模块无调用点），保留 `clamp_levels_to_weighted_cap`
   （`apply_level_cap` 消费）。
3. **冲突清单②订正**：`issue755-lee-decision-wiring-20260729.md` 条目 2 追加订正段——原「二选一」
   论证不成立（遗漏第三方案），#783 已按第三方案返工。

## 5. 停手项

- BTC 200k 双配置实测未跑（数据 blocker），已用结构保证 + 回归锁断言替代，如实登记——若编排层
  要求实跑读数，需提供 `btc_1m_full.json` 数据后再跑 `issue755_level_cap_on_off_btc20k_diff`
  同款 200k 双配置。
- `LevelOrderPlan::cap_narrowed_levels` 仍恒空、`LevelOrderLedger::plan_gated`/`regate`（M3
  per-level 订单路由）仍未接生产——不在本票范围，留独立后续票（与 #755 报告条目 2 同口径）。
