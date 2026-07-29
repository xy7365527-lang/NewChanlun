# 影子评审 #334：#308 LEE M2 订单归因（commit `37a56deab1`）

新上下文独立评审，只读。复跑在临时 detached worktree（`37a56deab1` 检出，独立 target）完成——主工作区已被并行 #309 改写（`level_order.rs`/`mod.rs`/`coverage.rs` + 新增 `level_clock.rs`），就地复跑测的不是本 commit。

## 复跑（本 commit 检出）

- `cargo test --release --lib`：**1856 passed / 1 failed**，唯一失败 `extract_signals_bit_exact_digest_guard`（#110 在案，未修未归因）。与票体一致。
- `level_order` 8/8、`lee_m2` 3/3、`level_ledger_wired` 1/1。
- 实测读数：`decisions=3000 orders=492 max|ΣΔq|=11022 max|p_t|=5700 L0=0 L1=0 L2 gap=5654 rescaled=1190/3000 residual_bucket=0`；`LEE_NET obs=2000 resid=0 max|N|=5715 voices=1`；digest `0x92f7a2f65ed55862` 与冻结常量相符。

## Spec 轴

| 核对点 | 判定 | 证据 |
|---|---|---|
| M2 守界（M3/M4 未越界） | PASS | 无事件门控（`fill.rs:1125` 每 bar 无条件 plan）、无级别 sizing；风控门 `k_theta_risk_gate` 仍在决策前（`fill.rs:772`）每 bar 生效 |
| 「同义反复护栏非级别独立定价」如实性 | PASS | `level_order.rs:1-60` 模块头明写推导方向与 §C.2 伪码相反、T 由 `qty_M0` 锚定；`fill.rs:430` 构造可验 |
| 订单流 bit-exact | PASS | 冻结 golden（`runner.rs:2452`）+ 逐位 trades/PnL/equity（`runner.rs:2404-2444`）；i64 求和消除 §D M2 风险栏预期的浮点重排，强于 spec 的 eps 容差 |
| #289 ① release 非平凡恒等 | PASS | `level_ledger.rs:168` / `runner.rs:2304-2318`，obs=2000、max\|N\|=5715 |
| #289 ② wired 非空前置 | 部分 PASS | 断言存在（`runner.rs:2306`）但只保证 ≥1；实测 voices=1、closed 桶 1 ⟹ 见 MED-1 |
| #289 ③ 单源化 | PASS | `forced_flat_anchor`（`fill.rs:1554`）、`build_level_targets`/`level_nets`（`level_ledger.rs:57/85`）+ 同源测试 |
| L2 40% 分歧登记口径 | PASS | 只登记不断言，`runner.rs:2390-2398` 附待裁声明；数值 1190/3000=39.7% 复现 |

## Standards 轴

模块形状 PASS（473 行 = 文档 60 / 实现 190 / 测试 150，均在 800 上限内，immutable 风格）；三类读数分级标注 PASS（231 号 L0/L1/L2 三处一致：模块头表 / 字段注释 / bin 输出）；单源化 PASS；`identity_witnessed` 双条件（残差 0 且量级 >0）PASS。

## 问题（无 HIGH，不回票）

- **MED-1 归因维度在集成验收上是平凡的**。探针实测三个 fixture 全跑批仅 2 / 2 / 1 个声部（`nets` 终态空、closed 桶 1–2）⟹ `Σ_ℓ` 实际只有 1 项：`attribute_total` 的多级最大余数法与 `merge_levels` 多级分支在集成路径**从未触达**，只有单测覆盖。`runner.rs:2482-2487` 自称验证「归因维度可读」，断言却是 `levels().next().is_some() || n_closed>0`——单级别下同样通过，名实不符。#289 ② 把「0 声部平凡」修成「1–2 声部平凡」，结构性平凡未除。建议：加 `n_levels_max` 读数并断言 ≥2，或换能产多级结构的 fixture。
- **MED-2 golden 再生不可自动复现**。`runner.rs:2452` 常量的取得方式是手工改一行源码后重跑（`runner.rs:2432-2440` 文档）。任何上游分类器/fixture 变动都会让该测试以「M2 契约破，须回票」的措辞失败，误导后续评审。建议留再生开关或把剥离臂做成 cfg。
- **LOW-1** `random_walk_dataset`（`runner.rs:1145`）种子不含 symbol ⟹ `RW3000M2D` 与 `RW3000M2` 数据逐字节相同（探针：stats 完全一致），`runner.rs:2471` 是同一跑批重跑，无独立信息。
- **LOW-2** `runner.rs:2396` 的 L2「自洽」断言恒真（abs 累计必 ≥0、`n_rescaled` 构造上必 ≤ `n_decisions`），形式是 assert 实为零信息。
- **LOW-3** release 下若 L1 失配，`fill.rs:446` 无条件用 `Σ_ℓΔq_ℓ` 覆盖订单量，护栏只有 `debug_assert` + 事后累计，无 release 阻断路径。

## 结果包

**结论**：#308 两轴通过，M2 契约（量经归因出口 + bit-exact + 守界 + 能力边界如实）兑现，建议合入并把 MED-1/MED-2 挂为 M3 前置。**边界条件**：若 M3 要求归因表跨 ≥2 级别真实分配，MED-1 升为 HIGH（多级路径无集成证据）。**下游推论**：M3 起 golden 应随票废止而非放宽（设计文档 §D M3）。**谱系引用**：090 反声明膨胀（本票如实声明达标）、231 号认识论分级（三类读数标注达标）。**影响声明**：本评审只读，唯一写入为本文件。
