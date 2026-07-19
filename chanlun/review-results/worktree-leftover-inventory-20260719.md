# Worktree 遗留改动清单（只读盘点，2026-07-19）

来源：/tmp/kimi-nest-mainline（branch `kimi-nest-mainline-20260717`，HEAD fe03fd5c28）。
背景：原独占会话（归并 PID 72171）已退出；本会话按台账约定**未写入** worktree，本清单为只读盘点，处置权留用户。
主仓对照：main @ 6075840687（task-106 顶）。

## 1. 已跟踪文件的未提交改动（31 files，+4820/−252）

- **docs 镜像修复 ×2**（090 条目1 的 worktree 侧应用）：
  - `docs/recursive_fugue_necessity_proof.md`（:79 伪原文降级）
  - `docs/spiral_physical_reinterpretation.md`（:106 上游伪标同步降级）
- **rust 生产改动**（含 C-4 遗留 `runner.rs`/`wverify_run.rs`，及 #105/#106/#111 等实装）：
  - 大头：`strategy/mod.rs` +703、`strategy/risk.rs` +248、`strategy/exit.rs` +208、`strategy/ledger.rs` +141、`classifier/*`（bsp/divergence/nest/recursive_tower/signal/level_view）、`backtest/{mod,runner,wverify_run}.rs`、`config.rs` +22、`Cargo.toml`
  - 终局门物证：cargo test --lib 1732 绿 0 败（/tmp/c4c_test.out，矩阵④⑤在档）

## 2. 未跟踪（untracked）分组

- **chanlun/ 产物区**（escalate ×9、review-results ~50 份、plans/）——已镜像至主仓 chanlun/ 区：p124-s8-merge-recon、11 关矩阵、C-5 四终稿（stage3-c5/m8e2e/rdecomp/reach-distribution，@2026-07-19 字节全等校验 ✓）；其余见 worktree 原地。
- **rust 新探针 bins ×21**（p100–p125 系列）+ `backtest/dual_ledger.rs`、`classifier/turn_class.rs`（关④⑤实装件）
- **`docs/formal-chain/有效域定理-20260704.md`**：090 条目2 补档**已在 worktree 就位**（git 对象库逐字恢复）；另 `gap3-rework-codex9-fix` 分支持有同文件（已验证 `git cat-file -e` ✓）
- 运维件：`.chanlun/locks/`（WORKTREE_OWNER、CARGO_BAN.retired-20260719、cargo_gate.sh）、`.chanlun/{agent-roster,gate-ledger,scene-ledger}-2026071x.md`、`p7_inputs/`、`rust/.cargo/`

## 3. 挂起决策点（均待用户拍板，本会话不越界）

| # | 事项 | 现状 | 选项 |
|---|---|---|---|
| 1 | 090 条目1（伪原文降级）主仓应用 | 镜像补丁在 worktree，主仓 docs/ 禁写未获解除确认 | 解禁后按 090-mirror-fixes-20260718.md 打正式补丁 |
| 2 | 090 条目2（有效域定理补档）主仓应用 | 补档文件在 worktree + gap3 分支双源在档 | (a) 解禁后落 main；或随归并 gap3 分支自动解悬（090 §边界说明） |
| 3 | worktree 遗留 rust 改动（31 modified + 23 untracked） | 未提交，会话已退出无人认领 | 重新进场（按 WORKERS.md §2 预检）审阅后提交 / 冻结现状 |

## 4. 关联在档

- 台账：`chanlun/harness/harness-ledger-20260719.md`（环3 并发实证已回写）
- 矩阵：worktree `chanlun/review-results/mainline-11guan-acceptance-matrix-20260719.md`（⑪ 关全绿，备注行记两处「待禁写解除」即本清单 §3.1/§3.2）

## 收口补记（2026-07-19，438f5dc7，用户指令「挂起留痕 + 代办完成」）
- §3 三项全部落地：090 条目1（fugue:79 + spiral:106 降级）与条目2（有效域定理补档）已应用主仓并逐字节校验 ✓；worktree 遗留改动已按 WORKERS.md §2 预检（无活进程、OWNER=本会话）后提交归档：commit 640609071d @ kimi-nest-mainline-20260717。
- 例外：p7_inputs/（23M 单文件派生数据）不入库，仍留 /tmp，可由主仓 analysis/data_cache 重建。
- kimi 账本（wire.jsonl）已先后落挂起留痕与 complete 事件；#116 关账。
