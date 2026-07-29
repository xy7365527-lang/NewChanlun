# 影子评审 #305：#295 backtest_bin 编译修复（commit 6f52b9f6c4，3 文件 +13/−4）

- 评审者：独立 Opus 新上下文（禁自评成立，未参与 #295 实装）
- 只读评审；本报告为唯一落盘文件
- 结论：**PASS**（0 HIGH / 0 MED / 2 LOW）；不回票

## 复跑证据

| 闸门 | 结果 |
|---|---|
| `cargo build --release --features backtest_bin --bin theta_backtest --bin theta_overlay` | exit 0，`Finished release ... in 36.95s`（仅存量 113 条 dead-code warning） |
| `cargo test --release --lib` | `1834 passed; 1 failed; 132 ignored` — 唯一失败 `classifier::signal::tests::extract_signals_bit_exact_digest_guard`（#110 在案），与基线逐字一致 |
| 靶向 `opsem_dump_env_gated` | 1 passed |
| 靶向 `voice_exec` | 9 passed |

## Spec 轴（对照 #295 票体 + resolution）

1. **牵连面结论 — PASS**。`OPSEM_DUMP_DIR_OVERRIDE` 在 fill.rs 全文零使用（grep 后消费面仅 opsem_dump.rs:195、runner.rs:1448/1450）；`pi_theta_fill_loop_voice` 本体在 fill.rs:366 即 `#[cfg(test)]`，消费者仅 runner.rs tests:2272/2319/2347；生产 VOICE_EXEC 路径实测走 runner.rs:675→687 `pi_theta_fill_loop_overlay(..., voice_book)`，不经该 wrapper。「bin 不该引 test-only 符号」口径成立，未触发票体停手条件。
2. **改法最小性 — PASS**。仓内先例核实：父 commit runner.rs:93 与 fill.rs:339 `#[cfg(test)] use super::admission::{VOICE_EXEC_OVERRIDE, ...}` 同式；本次两处 `#[cfg(test)] use/pub(super) use` 严格同构，无扩面。
3. **theta_overlay.rs:80 判定 — PASS（附 LOW-1）**。旧字段 `n_overlay_orders` 已在 lib 侧改名消失（E0609 必修），属同票必修面成立；新标签「overlay 声部总数」与 `n_overlay_voices` 语义（runner.rs:584-586）相符，090 合规。
4. **pure_bsp_timing.rs:119 守界 — PASS**。既存 E0277 与本票 import/字段面无因果，未动；#311 已立票（OPEN），登记链完整。

## Standards 轴

- **无行为变化的机械证据 — PASS**：三处改动均为 import 门控 / 打印字段名，无生产分支变化；lib 数字逐字一致 + 靶向 10/10 全绿构成证据闭环。
- **门控后 tests 可见性 — PASS**：tests mod（runner.rs:929）经 `use super::*` 取 `#[cfg(test)]` 层符号，实跑证实。
- **090 声明诚实性 — PASS**：新增两条注释逐项可核（opsem_dump.rs:180 确为 `#[cfg(test)]` 行；「消费面仅 `opsem_dump_env_gated_bit_exact`」与 runner.rs:1425 函数体一致；「生产接入走 VOICE_EXEC=1 gate」如上第 1 条已验）。

## LOW（不阻断，登记待处）

- **LOW-1｜与既有修法记录分岔未声明**：`chanlun/review-results/gap4-theta-overlay-fix-20260719.md:23-25` 与 `l1-l2-implementation-20260719.md:46`（FIX §4.3 双字段方案）均规定 :80 → `n_overlay_fill_events`（+补打印 `n_overlay_voices`）。本 commit 改取 `n_overlay_voices` 单字段且未说明分岔。**数值影响为零**（`n_overlay_fill_events ≡ net_result.n_orders`，已由 :74 打印）；副作用是新行恰为 :78+:79 两行之和（冗余读数）。建议 #305 收口时补一句口径说明。
- **LOW-2｜既存 090 面（非本票引入）**：`theta_overlay.rs:74` 标签「净额订单数」在 `VOICE_EXEC=1` 下承载声部执行投影口径（runner.rs:610-613 明载），标签与实际不符。出 #295 范围，建议另票。

## 影响声明

本评审只读，未改动 rust/ 及任何非本报告文件。
