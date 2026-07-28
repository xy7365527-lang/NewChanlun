# 概率推断决策面清除——五档树逐件处置登记（#71 子票，map #59）

- 日期：2026-07-28
- 票据：wayfinder #71 子票（阻塞方 #71），map #59
- 裁定依据：`chanlun/escalate/chi-line-falsification-ruling-20260728.md`（档2-修：χ 族撤销 + 验收统计推断核全砍 + 描述簿记留 + 历史裁决照旧）
- 性质：**本票纯机械登记与标注，不重裁**——裁定已出，本表逐件落档标注释 + 登记，禁 git mutation，禁删（#225 先例）
- 工作面：worktree `/tmp/kimi-nest-mainline`；全程前台单线程；主仓零写入
- 验收：指纹前后一致——`cargo test --release` 标注前/后均为 **2034 passed; 1 failed; 136 ignored**（唯一失败 `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`，#115 在案，与本票无关）⟹ 零行为变化确认

## 处置口径

- **诊断件保留**：该模块/函数虽随 χ 线或验收统计推断核撤销，但其历史产出（否证结论、实测证据）仍是本裁定或其他历史裁决的证据基座，保留供追溯
- **禁删**：所有 15 件均不删除，仅在文件头/函数头落档标注释（裁定链接 + 日期 + 一句理由），零行为变化
- 明确不碰（裁定 §3/§4 already 排除，本票不动）：`metrics.rs`/`treasury.rs`/`wverify_run/report.rs`（描述簿记）、`gamma_dump.rs`（结构频数，在役）、量纲③喂入（无害遗留）

## A. 决策统计族（χ 线，裁定 §1① 撤销）—— 8 件

| # | 文件 | 标注位置 | 处置 | 理由摘要 |
|---|------|---------|------|---------|
| A1 | `rust/src/theta_v0/backtest/mu_estimator.rs` | 模块头 | 诊断件保留 | μ/LCB/`mu_shrink`/`oos_gated_drop` 门侧接口——χ 门核心估计器，撤销不接生产 |
| A2 | `rust/src/theta_v0/backtest/selector.rs` | 模块头 | 诊断件保留 | `chi_t`/`filter_gamma*`——教义 §13 选择函数实现，本系统登记为不采用 |
| A3 | `rust/src/theta_v0/backtest/admission.rs` | 模块头 | 诊断件保留 | `ChiFilterCtx`（三门 seam 之一，nest/k_Θ 两门不受影响） |
| A4 | `rust/src/theta_v0/backtest/runner.rs` | `run_theta_v0_pi_chi` + `run_theta_v0_pi_chi_shrink` 函数头（两 χ 入口） | 诊断件保留 | 生产默认路径走 `run_theta_v0_pi`（`chi_theta=None`），两 χ 入口零生产影响 |
| A5 | `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs` | 模块头 | 诊断件保留 | walk-forward harness，检验对象（χ_t 选择器）已撤销；INCONCLUSIVE 历史结论照旧有效 |
| A6 | `rust/src/theta_v0/backtest/wverify_run/m8.rs` | `run_q4_fullpi_policy` 函数头（限定范围，同文件 `q4_margin_model`/`m6_cost_model` 非 χ 内容不在此列） | 诊断件保留 | Arm1/Arm2/Arm3 调用 χ 门；q4 π^full INCONCLUSIVE 历史裁决照旧有效 |
| A7 | `rust/src/theta_v0/backtest/pooling_icc.rs` | 模块头 | 诊断件保留 | pooling 目标（延长 μ 估计样本量）随 μ/χ 门一并撤销 |
| A8 | `rust/src/theta_v0/backtest/wverify_run/issue71_chi_gamma.rs` | 模块头 | 诊断件永久保留 | #563 影子评审 M6 订正补列；本文件产出的 #71 实测数据正是本裁定的证据基座 |

## B. 验收统计推断核（裁定 §1② 档2-修砍单）—— 6 件

| # | 文件 | 标注位置 | 处置 | 理由摘要 |
|---|------|---------|------|---------|
| B1 | `rust/src/theta_v0/backtest/perm_test.rs` | 模块头 | 诊断件保留 | 置换检验 p 值生产者，与 χ 门同构；下游 `decontam.rs` 随之断粮 |
| B2 | `rust/src/theta_v0/backtest/highlow_mu.rs` | 模块头 | 诊断件保留 | μ̂(z_L0\|D_hi) 条件期望估计（区间套统计版本），裁定原文点名「与 χ 同构」 |
| B3 | `rust/src/theta_v0/backtest/l3_pi_falsify.rs` | 模块头 | 诊断件保留 | Phase-2 显著性否证 harness；镜像 v1 口径 |
| B4 | `rust/src/theta_v0/backtest/l3_fullwindow.rs` | 模块头 | 诊断件保留 | Phase-1 全窗 L3 定论 harness；「全窗 8/8 L3 否证」历史定论照旧有效 |
| B5 | `rust/src/theta_v0/backtest/capture_oos.rs` | 模块头 | 诊断件保留 | S3 捕获率回测 |
| B6 | `rust/src/theta_v0/backtest/segment_gn.rs` | 模块头 | 诊断件保留 | S2 段口径复核 + λ_gap 校准分布 |

## 连带枚举（decontam / alpha分离层，消费 perm_p 断粮定档）—— 1 件

| # | 文件 | 标注位置 | 处置 | 理由摘要 |
|---|------|---------|------|---------|
| C1 | `rust/src/theta_v0/backtest/decontam.rs` | 模块头 | 诊断件保留 | beta 去污三态判据消费 `perm_test.rs` 产出的逐桶 `perm_p`；生产者已砍，本模块推断链断粮 |

## 明确排除（裁定 §3/§4，本票不动，供交叉核对）

| 文件 | 排除理由 |
|------|---------|
| `rust/src/theta_v0/backtest/metrics.rs` | 描述簿记（PnL/回撤/费用算术读数），非推断 |
| `rust/src/theta_v0/backtest/treasury.rs` | 描述簿记 |
| `rust/src/theta_v0/backtest/wverify_run/report.rs` | 描述簿记 |
| `rust/src/theta_v0/backtest/gamma_dump.rs` | 结构频数记录仪，在役 |
| 量纲③喂入（`chi_dimension_three_return` 等） | #65 执行后果，无害遗留，回滚反是 churn |

## 未在本票范围内、留意但不动的并行在飞面

- `open_ledger.rs` / `fill.rs`：#600 在飞（typed ledger 撞键修复），本票绕开
- `p123_fast_replay.rs` / `pi_bsp_timing.rs`：#532/#533 测试设施在飞，本票绕开
- `chanlun/agent-roster-2026-07-21.md`：并行线在飞面，本票不碰

## 验证

```
cd rust && cargo test --release
```
标注前后均：`2034 passed; 1 failed; 136 ignored`（唯一失败 #115，signal.rs bit-exact 摘要守卫，在案与本票无关）。零 git mutation；本票所有改动为 doc 注释追加，无逻辑/行为变更。
