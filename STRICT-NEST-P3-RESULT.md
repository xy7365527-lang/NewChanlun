# STRICT-NEST-P3-RESULT（生产重放 sidecar 接入）

- 工作目录：`/tmp/strict-nest-work`（分支 `strict-nesting-20260708`）
- 前序基线：P1 `b48e8d9b11`，P2 `e0fb9e85fb`
- P3 范围：只把严格区间套证书流作为 sidecar 接入 `run_theta_v0_pi` 生产重放路径；默认关闭，不参与订单、候选、风控、账本。
- 开关：`THETA_STRICT_NEST_SIDECAR=1/true/yes/on` 开启；未设置时关闭。

## 总判

P3 硬门 **PASS**。代码、报告与验证产物纳入同一 worktree 提交。

## 硬门结果

1. sidecar 接入生产重放路径并可开关：**PASS**
   - `run_theta_v0_pi_inner` 内接入 `StrictNestSidecarCollector`。
   - 默认关闭时走原 `IncrementalClassifier::classify_at(i)` 分支。
   - 开启时走 `classify_at_with_l0(i)`，同帧观察 L0、Classification、tower、TowerCache，产出 `RunResult.strict_nest_sidecar`。

2. 开关关闭时全量重放 bit-exact：**PASS**
   - `./target/release/strict_nest_check`
   - 数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`
   - 全量 4,613,599 bar；checkpoint 4,613 次；比较 26,618 次；不一致数 **0**。
   - `STRICT-NEST-CHECK.md` 已同步更新本轮耗时：1455.3s。

3. 开关打开时证书流与 P2 装配层口径一致：**PASS**
   - 单测 `strict_nest_sidecar_summary_matches_p2_assembly` 直接比较 sidecar 汇总与 `classifier::nest::assemble_certificates` 的逐证书输出。
   - 生产入口小窗验证：
     `THETA_STRICT_NEST_SIDECAR=1 ./target/release/theta_backtest BTC 2024-01-01 2024-01-03`
     - bar 数 4320
     - sidecar 帧数 4320
     - base cand_delta 1
     - terminal 查无 0
     - 证书总数 0
     - 每级证书数 `[(1, 0)]`

4. 判据行为零改动：**PASS**
   - `git diff --quiet -- rust/src/theta_v0/classifier/divergence.rs rust/src/theta_v0/classifier/bsp.rs rust/src/theta_v0/classifier/signal.rs`
   - exit = 0；三份判据文件无 diff。

## diff --stat

核心实装与验证产物（不含本报告文件自身）：

```text
STRICT-NEST-CHECK.md                      |   2 +-
rust/src/bin/theta_backtest.rs            |   8 ++
rust/src/theta_v0/backtest/incremental.rs |  15 ++-
rust/src/theta_v0/backtest/runner.rs      | 159 +++++++++++++++++++++++++++++-
4 files changed, 180 insertions(+), 4 deletions(-)
```

## 偏差说明

1. sidecar 只读观察生产 replay 同帧状态，不向 `VoiceDecision`、`Order`、risk gate、ledger 写入任何输入。
2. 证书判据没有复制或放宽：证书装配唯一入口是 P2 的 `classifier::nest::assemble_certificates`；terminal 只从 L0 一类买卖点映射，查无单独计数。
3. `IncrementalClassifier` 新增 `classify_at_with_l0` 与 `tower_cache()`，旧 `classify_at` 保持原 API 与调用语义。
4. 生产入口小窗产量为 0，和 P2 严格装配稀疏口径一致；没有为凑产量放宽 `Cand^δ`、`Sub` 或 terminal 条件。
5. `STRICT-NEST-CHECK.md` 的唯一变化是本轮全量重放耗时从 1420.3s 更新为 1455.3s。

## 测试记录

- `cargo check --lib`：通过（仅既有 warnings）
- `cargo test --lib strict_nest_sidecar_summary_matches_p2_assembly -- --nocapture`：1 passed
- `cargo test --lib`：1529 passed / 0 failed / 125 ignored
- `cargo build --release --bin strict_nest_check`：通过
- `./target/release/strict_nest_check`：总判 PASS；P1 mismatch 0；P2 terminal 查无 0；证书合计 0
- `THETA_STRICT_NEST_SIDECAR=1 ./target/release/theta_backtest BTC 2024-01-01 2024-01-03`：通过；sidecar summary 已输出

doctest 说明：本轮按要求跑的是 `cargo test --lib`。既有 `rust/src/theta_v0/ledger/separate.rs` 3 个 doctest 失败属于前序已知豁免，本 P3 未触碰该文件。
