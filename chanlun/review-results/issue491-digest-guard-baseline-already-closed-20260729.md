# #491 复核：`extract_signals_bit_exact_digest_guard` 基线红——已由 #610/#631 提前收口

- 日期：2026-07-29｜票据：issue #491（原票面：全仓唯一基线红收口）
- 工位：`/tmp/kimi-nest-mainline`｜方法：git 历史核对 + 全量测试门实跑复核，**零源码改动**

## 结论

票面描述的"基线红"在本 session 开工前已消失。二分链已由前序 session 完整走完并落盘两条提交：

1. **引入提交**：`bbbd8f89fa`（2026-07-24，"P1→#214→#218 证书索引口径三部曲"）——对
   `BspPoint` 做两处同时进入 `#[derive(Debug)]` 的改动（新增 `level_origin: u32` 字段 +
   `center: Option<Center>` → `Option<OwnerRef>` 类型改写），使 `bit_exact_battery_digest()`
   翻转但 GOLDEN 未同步重锚，致 guard 持续红。归因坐实见
   `chanlun/review-results/issue610-digest-guard-attribution-20260728.md`（git bisect +
   逐行 diff 定性）。
2. **第一次重锚**：`f6cd18aebc`（#610，2026-07-29 00:57）——`GOLDEN: 0x90c7_9ee6_17e1_1392`
   →`0xe6a2_63e3_43e4_3845`，登记 owner 载体 + level_origin 面的诚实翻转。
3. **第二次重锚**：`d9f1860124`（#631，2026-07-29 11:18）——删除全仓恒为 0、从未真正写入的
   `level_origin` 空转字段（`LevelProjectionLayer.identity.level` 已是正主，该字段是死拷贝），
   `GOLDEN: 0xe6a2_63e3_43e4_3845` → `0xe371_3897_d9bf_978c`。该提交自带验收：
   `cargo test --release --lib` 删前删后 2204/0/138 完全一致 + `p123_fast_replay` 20k bars
   stdout 对拍零 diff（临时 worktree + patch 隔离并行工位改动后对照）。

三条提交均已是当前 `HEAD`（`b242452174`）的祖先（`git merge-base --is-ancestor d9f1860124 HEAD`
返回真）。**本票的"故意漂移 vs 真 bug"判定 = 故意漂移**（#218 面语义修正 + #110/#434
空转字段清理，均有独立 spec/终裁支持，无一处静默翻转），且已完整走完"重算 GOLDEN + 留痕"
收口流程，本 session 无需重复。

## 复核实跑（本 session，`CARGO_TARGET_DIR=/tmp/kimi-nest-target-491`，净树口径）

| 命令 | 结果 |
|---|---|
| `cargo test --lib theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard -- --nocapture` | 1 passed；digest = GOLDEN = `0xe371_3897_d9bf_978c` |
| `cargo test --release --lib`（全仓） | 2205 passed, 0 failed, 138 ignored |
| `cargo test --lib`（全仓，debug） | 2205 passed, 0 failed, 138 ignored |
| `cargo test --lib theta_v0::classifier::signal::`（定向） | 56 passed, 0 failed, 3 ignored |

（passed 数 2205 对比 `d9f1860124` commit message 记录的 2204——差值来自 #631 之后落地的
`#633` 批次分解提交新增测试，非本票范围内异常，不影响"零红"判据。）

## 定义依据

无新增概念定义；纯执行层验收复核。

## 边界条件

若未来再有新字段进入 `BspPoint` 的 `#[derive(Debug)]`，GOLDEN 会按同一先例（`struct_break_dir`
→`force`→`level_origin`→本次）再次诚实翻转——这是该 guard 的固有机制，不是回归信号。

## 下游推论

原票面"cargo test --lib 全仓唯一红"的前提已不成立；下游若仍在验收清单里引用 #491 为
"待收口红线"，应更新为"已闭合（#610+#631）"。

## 谱系引用

无新增谱系张力；沿用 #610/#631/#434/#308/#218/#211 既有裁定链，见
`chanlun/review-results/issue610-digest-guard-attribution-20260728.md`、
`owner-attribution-fix-readings-20260724.md`、
`chanlun/review-results/issue610-id65-supplement-20260729.md`。

## 影响声明

未改动任何源文件；仅新增本复核文档。工位内其余未提交改动（`chanlun/agent-roster-2026-07-21.md`、
`issue571-ledger-generation-key-fix-20260728.md`、`treasury-reverify-t1-armR-trades-golden-20260727.json`、
`rust/src/bin/p100_cert_bsp_recon.rs`）均属并行工位在制品，本票未触碰、未 stage。
