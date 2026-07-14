# STRICT-NEST-P4-RESULT（回归与封边界）

- 工作目录：`/tmp/strict-nest-work`（分支 `strict-nesting-20260708`）
- 前序提交：P1 `b48e8d9b11`，P2 `e0fb9e85fb`，P3 `e823626e14`
- P4 范围：回归复跑、汇总报告落盘、外推边界声明；不改 `divergence.rs` / `bsp.rs` / `signal.rs` 判据行为。

## 总判

P4 硬门 **PASS**。严格区间套实现按 P0 adopted-default 口径收束，证书产量 0 维持严格判据解释；未为产出放宽任何条件。

## 回归记录

1. `cargo build --release --bin strict_nest_check`：**PASS**
   - 仅既有 warnings。

2. `cargo test --lib`：**PASS**
   - `1529 passed / 0 failed / 125 ignored / 0 measured / 0 filtered out`
   - 既有 `rust/src/theta_v0/ledger/separate.rs` 3 个 doctest 失败为前序已知豁免；本轮按 P4 门跑的是 `cargo test --lib`，未触碰该文件。

3. `./target/release/strict_nest_check` 全量重放：**PASS**
   - 数据：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`
   - 4613599 bar（2017-08-17 04:00:00 .. 2026-05-31 23:59:00），40001 笔交易。
   - 全量因果重放 1409.8s。
   - sanity 全一致；P1 checkpoint 4613 次逐 bit 全比对，不一致数 0；E1 三元组逐项一致；P2 证书合计 0，terminal 查无 0。
   - `STRICT-NEST-CHECK.md` 已同步更新本轮耗时。

4. 判据文件零 diff：**PASS**
   - `git diff --quiet -- rust/src/theta_v0/classifier/divergence.rs rust/src/theta_v0/classifier/bsp.rs rust/src/theta_v0/classifier/signal.rs`
   - exit = 0。

## 产出文件

- `STRICT-NEST-P4-RESULT.md`
- `.chanlun/review-results/strict-nesting-implementation-report-20260709.md`

说明：用户给出的 `chanlun/review-results/...` 路径在本 worktree 中不存在；本项目实际 review-results 目录为 `.chanlun/review-results/`。

## 封边界

本轮只证明当前 HEAD、默认 `ThetaConfig`、当前 BTC 1m 数据与现行 `MacdArea` 背驰口径下的严格区间套回归通过。不得外推到其他定义分叉、非默认 Θ、其他品种/周期、数据修订、`l_max` 更深级别、或 sidecar 之外的生产 alpha / 执行效果。
