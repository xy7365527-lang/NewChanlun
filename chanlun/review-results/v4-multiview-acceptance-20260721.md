# V4 三窗三臂多级投影终验报告（issue #114，SPEC #109 验收票）

- 日期：2026-07-21
- 工位：worktree `/tmp/kimi-nest-mainline`，HEAD=ec728bf6cf + #113 未提交改动
- 前置：#110 投影层骨架 → #111 多级查询 expand → #112 进场门 contract → #113 出场门 contract
- 基线对照：#77 V4 三窗三臂（固定 ℓ+1 桥，20260721）

## 1. 执行

臂A 门关：`VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_A2 cargo test --release --lib m8_e2e_all_systems_oos -- --ignored --nocapture`（33s）
臂C 多级投影真链门：`THETA_NEST_CERT_GATE=1 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_C2 cargo test ...`（**39.20s**，#77 旧臂C 831s → 降 21×）

## 2. 关键读数

### wf7 震荡窗（硬验收线）

| 指标 | #77 臂A 门关 | #77 臂C 旧（固定ℓ+1） | #114 臂C 新（多级投影） |
|---|---|---|---|
| Long pnl | +5597.59 | -4521.68 | **-3297.03** |
| Short pnl | -5598.74 | -13986.31 | **-13194.12** |
| execR | -1414690 | -1847334 | **-1721065** |
| typed_found | — | 71/1724 (4.1%) | **89/1724 (5.2%)** |
| level_hits | — | — | **{L1:48, L2:27, L3:14}** |

### wf7 硬线判定：**✗ 仍有害**

Long 仍翻负（-3297.03，比旧臂C -4521.68 收窄但未翻正）；Short 仍亏（-13194.12，2.36× 基线）。wf7 execR -1721065 仍负。

**但方向正确**：Long 亏幅从 -4521.68 收窄至 -3297.03（改善 27%）；多级确实在命中（level_hits L2=27 L3=14 是旧桥够不到的）；命中率从 4.1% 升至 5.2%。

覆盖率提升幅度（4.1%→5.2%）远低于 #107 外推的 ~35.6%——原因待查（见 §4）。

### wf8 趋势窗

| 指标 | #77 臂C 旧 | #114 臂C 新 |
|---|---|---|
| execR | +4724613 | +4724613 |
| Long pnl | +23415.97 | +23415.97 |
| Short pnl | -8662.46 | -8662.46 |

wf8 三指标与 #77 旧臂C 逐字一致——wf8 窗多级投影无增量命中（level_hits {L1:37,L2:15,L3:4} 基数低）。

## 3. 多级投影机制读数（NEST_GATE_CHAIN，臂C）

| 窗 | typed_found | typed_none | level_hits | single_multi_divergence |
|---|---|---|---|---|
| p3fold | 111/1633 (6.8%) | 1522 | {L1:39, L2:30, L3:45} | 31 |
| wf7 | 89/1724 (5.2%) | 1635 | {L1:48, L2:27, L3:14} | 18 |
| wf8 | 56/1518 (3.7%) | 1462 | {L1:37, L2:15, L3:4} | 5 |

三窗 level_hits 证明多级查询在工作——L2/L3 命中是旧固定桥不可能拿到的。但 total typed_found 仍低（3.7%-6.8%）。

## 4. 为什么命中率提升远低于外推

#107 外推 ~35.6% 的前提 = "43.2% 的 miss 在 L2/L3/L4 有结构性覆盖"。实测 5.2% 意味着：

1. **#107 的"结构性覆盖"判定标准 ≠ typed_lookup_multi 的实际查询条件**——#107 只查"该 bar 在某级别有确认事件"，但 typed_lookup_multi 还要求身份桥值匹配（source_index==seg_c_full.1）、side 同向、因果前缀内——这些附加条件筛掉了大部分。
2. **single_level_share 仍 ≥0.958**（三窗几乎全单级链）——多级查询能命中的链本来就少。
3. 外推的 ~35.6% 是"代理上界"（#107 报告标了"精确量级标未确证"），实际命中受身份桥全条件约束。

## 5. 旧路径退役判定

wf7 ✗ → **旧固定路径保留**（不删，以便回退和对照）。#114 票面"如终验 ✗，旧路径保留，照实落账 + 归因"——按此处置。

## 6. 照实总结

- 多级投影方向正确（level_hits 证明，Long 亏幅收窄 27%）
- wf7 硬线未过（照实否定合格）——覆盖率提升不足以翻转震荡窗盈亏
- 根因更深层：typed_lookup_multi 的全身份桥条件（source_index+side+因果前缀）比 #107 调研的"结构性覆盖"判定更严格
- 旧路径保留，不删
- 臂C 跑批 39.20s（#93 churn 修复 + #110 投影层零开销 = 门开路径可行）

## 可复算索引

| 数字 | 出处 |
|---|---|
| 臂A2 读数 | /tmp/kimi-nest-mainline/tmp/v4_A2.out |
| 臂C2 STATS/CHAIN/INDEX | /tmp/v4_C2.out grep NEST_GATE |
| 臂C2 execR | /tmp/v4_C2.out [m8] 行 |
| 臂C2 trades | /tmp/v4_C2/{p3fold,wf7,wf8}/trades.jsonl |
| #77 基线 | chanlun/review-results/v4-three-window-typed-chain-acceptance-20260721.md |
