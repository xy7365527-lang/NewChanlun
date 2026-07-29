# 影子评审 #309 — LEE M3 事件门控（commit `2fd5ee08ba`）

评审位：独立 session（新上下文），只读。日期 2026-07-27。
参照：设计文档 `multi-level-native-execution-design-20260719.md` §D M3 / §F②③；#308 MED 前置；后续订正 `f27ccb992a`（#340，与 M3 无关）。

## 复跑

| 环境 | 结果 | 说明 |
|---|---|---|
| 工作区（脏，另一 session 在改 classifier/parser） | 1869 passed / 1 failed | 受未提交改动干扰（+2 测试），不作判据 |
| 干净 worktree `a96d90042a` | **1867 passed / 1 failed** | 与 commit 声称基线逐值一致 |
| 定向 `level_clock` + `lee_m3` | **11/11 passed** | — |

唯一失败 = `classifier::signal::tests::extract_signals_bit_exact_digest_guard`（未修，见 LOW-3）。

实测读数（RW3000M3，干净态）：
```
CLOCK  decisions=3000 struct_tick_bars=6 ticks=9 | bsp=6 opened=2 closed=1 silent/overlay/pandiv/riskexit=0
GATE   regated=6 held_by_clock=1930 | orders=6 off_clock=1/1 | risk_gate_active=1191 with_order=2
       plan_fill_gap=0 struct_gap=5401 divergence_vs_m0=11098 rescaled=2/3000
PERLEVEL off_clock_delta=1 unexplained=0
FIVECLOCK(RW1500M3C) bsp_ticks=1 causal_violations=0 duplicate_ticks=0
```

## Spec 轴

| # | 项 | 判定 | 证据 |
|---|---|---|---|
| ① | clock_ℓ 事件集最小完备（§F②） | **PASS** | `level_clock.rs:17-33` 六字段 + 两外源逐字段对齐表；`:45-68` 三缺口照实（段完成/中枢生灭无载体、BSP 灭无载体、五钟 1/5）；`tw_event` 抑制器论证 `:35-39` 成立 |
| ② | 只在事件时点重估、无事件 bar 目标=前值 | **PASS** | `level_order.rs:387-410` `regate` 无 tick 取 `planned` 前值；单测 `no_tick_bar_yields_zero_delta_even_when_basis_drifts` 证 basis 漂移下 Δ≡0（构造性非碰巧） |
| ③ | 订单时点 ⊆ 事件时点并集（稀疏性） | **PASS（弱）** | `runner.rs:2400+` 四项断言；剥离对照 495→6 订单、491→1 off_clock。但实测分子仅 6/3000，见 MED-4 |
| ④ | 与 E2E 五钟时点一致性 | **未兑现（已照实降级）** | `runner.rs:2540-2560` 明确登记两重障碍（四钟无 rust 载体、`judge_at` 挂默认关的 nest 门），改交付首次观察纪律。符合 090，但该替代测本身平凡，见 MED-4 |
| ⑤ | 风控门每 bar 生效 | **PASS（作用面弱）** | `fill.rs` ② 段无条件调 `pi_theta_position(..., gate)`，不读 ticks；单测 `risk_flatten_fires_on_bar_with_no_clock_tick`。但 1191 次 binding 仅 2 次产订单；"1191→1191 不变"是伪对拍，见 MED-1 |
| ⑥ | q^plan 锚点论证 | **PASS（有未登记副作用）** | `level_order.rs:83-105` 三处 spec 表述一致 ⟹ 无歧义；放弃自动重试的代价照实。副作用见 MED-3 |
| ⑦ | #308 两条 MED 处置 | **FAIL** | 见 MED-1 |

## Standards 轴

- **090 反声明膨胀**：PASS。缺口登记密度高（四通道零命中、五钟未兑现、放弃重试、`max_abs_order_residual` 等级迁移），无冒充。
- **L2 40% 分歧相交点登记**：PASS。`level_order.rs:107-131` 分两句说（频率 40%→0.07% 是算术后果 / 幅度 5654→5401 ⟹ 根因未触及），剥离态两值与 #308 登记逐值一致，明确不裁决。
- **形式化有效域等级标注**：FAIL，见 MED-2。
- **测试质量**：单元层扎实（可证伪双侧见证、剥离态失败已验），集成层证据强度偏低（MED-4）。

## 问题分级（无 HIGH，不回票）

**MED-1｜#308 两条 MED 均未处置，且新增同类 golden**
`gh issue view 308` 挂入的 MED-2 原文为「M2 golden 常量不可自动再生……**M3 落地时一并处理再生机制**」。本票未处理，反而在 `runner.rs:2587` 新增 `const RISK_FACE_BOTH_ARMS: u64 = 1191`。该断言并非两臂对拍（剥离臂不在测试内），锁的是单个数字；其失败措辞「事件门控污染了风控域（票体硬约束破）」在任何上游分类器/风控参数变动时都会误导性失败——正是 MED-2 预警的模式复现。MED-1（多级 fixture）亦未造：`lee_m3_attribution_dimension_is_readable_and_not_residual_only`（`runner.rs:2682`）只断言"至少一个真实级别"，`off_clock_delta=1` 印证多级分配仍未触达 ⟹ 逐级稀疏性验收与 bar 级近乎等价。

**MED-2｜等级标注同文件自相矛盾（formalization-validity-domain 强制项）**
`level_order.rs:44-46` 声明"下表所有 L2 应为 L1"，`:55` 已订正；但 `:91`、`:159`、`:182`、`:295`、`:435` 仍标 L2，而对应字段注释（`:190`、`:196`）标 L1。同一读数（`max_abs_struct_gap`/`n_rescaled`/`max_abs_plan_fill_gap`）两个等级并存，下游引用哪一处都可自证。

**MED-3｜`pi_theta_position` 的持仓入参被换成计划态，未登记**
`fill.rs` ② 段 `pi_theta_position(p_tilde_lee, t_prev as f64, ...)`——第二参在 M0/M2 是真实持仓 `p_t`。该参数进 `j_theta_key` 的 `trade_cost = λ·|p−p_t|`（次键）与 `feasible_candidates` 的 `anchor_pt` 候选（`coverage.rs:2730/2756`）。换成 `t_prev` 后：换手成本按虚拟位置估、"保持真实持仓不动"这个候选可能不在可行集。该替换是保稀疏性构造所**必需**（否则 `t_prev≠p_t` 时无 tick bar 也产 Δ），但 `plan_level_gated_order` 的"两锚分工"表只讲了 `order_raw` 与 `K_Θ_gate`，未提投影输入的锚也被换。且实测 `plan_fill_gap=0` ⟹ 该分歧路径在本验收上完全不可观测。

**MED-4｜集成层证据强度接近平凡**
① `lee_m3_clock_obeys_first_observation_discipline` 在 RW1500M3C 上 `n_bsp_ticks=1`——"同身份至多响一次"（`duplicate_ticks==0`）在单元素下**构造上不可失败**，而非平凡前置只要求 `>0`，恰被最弱情形满足。② 稀疏性分子 6/3000、风控作用面 2/1191，`sparsity_witnessed()` 与 `n_risk_gate_active_with_order>0` 均以最小裕量通过。建议 M4 前换更密事件的 fixture 重取读数。

## LOW

- **LOW-1**：`LEVEL_ACCOUNT_RESIDUAL`（`u32::MAX`）不产生任何 clock 事件 ⟹ 一旦经 `commit_planned` 进入 `planned`，`regate` 永远走 else 分支保前值，成为不可清零的幽灵结构目标。默认 config 下不可达（pan_div 惰性 ⟹ `b_sum==total==0` 走恒等分支，实测 `n_residual_bucket=0`），故仅 LOW。
- **LOW-2**：`fill.rs:1254` 与 `:1257` 两行注释逐字重复。
- **LOW-3**：commit message 称「唯一失败 #110 在案」，实测失败测试为 `classifier::signal::tests::extract_signals_bit_exact_digest_guard`，其注释显式归因 `#115`（beta-route `force` 字段）。#110 已 CLOSED。归因链建议核准。

## 结论

**无 HIGH，不回票。** M3 主体（事件集定义、门控重估、域分离、锚点选型）实装正确且诚实登记密度高；构造性稀疏性由单测独立坐实，不依赖集成层读数。四条 MED 中 MED-1 是票面遗漏（#308 明文要求"M3 落地时一并处理"），MED-2/3 是登记缺口，MED-4 是证据强度——建议全部挂入 M4 前置，不阻塞本票合入。
