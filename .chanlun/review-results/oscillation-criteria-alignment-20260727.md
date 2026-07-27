# #366 实装报告：中枢震荡短差判据对齐缠师原文 + 清算两终局

- 票：#366（parent #379 图；SPEC #386 §4 验收对照；blocked-by #380/#381 已落地）
- 日期：2026-07-27
- 范围：`center_oscillation_trade.rs` / `oscillation_campaign.rs` / `short_diff_bucket.rs` /
  `fill.rs`（`step_center_oscillation` + `drive_campaign_wiring`）/ `wverify_run.rs`（见证打印）
- 不碰：`coverage.rs` / `persistent.rs` / `center.rs` / `recursive_tower.rs`（#59 线区域）；
  主策略决策路径（短差仍是自包含旁路账本）

## 1. 判据对齐（票面「新判据」，均有课号）

| 项 | #366 前（已废） | #366 后 | 依据 |
|---|---|---|---|
| 高抛位置 | `price >= 中轴`（中轴=(ZD+ZG)/2，**博文全库 0 命中**） | `price >= ZG`（向上离开中枢） | 80 课 / 89 课 / 20 课 |
| 回补位置 | `price <= 中轴` | `price <= ZD`（向下离开中枢） | 33 课 / 49 课 / 20 课 |
| 回补前置过滤 | 无 | **中枢不下移**（`alive.zg < prev.zd` ⟹ 拒绝） | 89 课 |
| 边沿恰等 | 中轴含端点 | ZG/ZD 含端点（实现决策，可复检） | — |

- 「上半区/下半区」自研规则已删除，行为化反例锚：
  `sell_signal_inside_center_upper_half_is_rejected_after_366`（旧判据放行 151、新判据拒绝）
  与买侧对应用例。
- 「中枢不下移」只约束**回补侧**——高抛侧无对应原文条件，不臆造对称门
  （`sell_signal_is_unaffected_by_center_moved_down`）。
- 归因顺序：先位置后下移，`CenterMovedDown` 桶只计「本可成立的回补被下移否掉」
  （`position_predicate_precedes_down_shift_filter_in_error_attribution`）。
- 级别归属与仓位来源未动（票面「不动」项）。

## 2. 清算两终局（2026-07-27 补充裁定，grilling）

`SuspensionOutcome` 新增 `settlement: TerminationSettlement`：

| 终局 | 触发 | 处置 |
|---|---|---|
| `CoverAndClose` | 多头遇三类**买**点 / 空头遇三类**卖**点 | 收手回补（回补价>卖出价亦然）→ 亏损如实入账（#380 项一通道）→ 往返闭合 → 转持股 |
| `WriteOffUnclosed` | 多头遇三类**卖**点 / 空头遇三类**买**点 | 不回补，挂起**核销**：货缺口与现金盈余**分列呈报，不冲销** |
| `Forfeit` | `Reset`/`Superseded`/`RebaseVanished` | #292 既有口径，**本票未改**（票面只裁三买/三卖两终局）——如实标注为未经 #366 复核 |

「不冲销」的行为化断言（反例锚）：`write_off_does_not_offset_anything_tw_and_cash_are_byte_identical`
——核销**不产任何 TwEvent**，TW 三量、R 账本、`realized_cash` 逐字节不变；只把 `open_units`
归位并累计 `written_off_units` 留痕。冲销（造一笔虚拟回补抵平）才是「装没发生」。

核销范围 = **同来源中枢**的批次：其他中枢未死，「挂起继续等」
（`write_off_settles_only_the_terminated_center_and_reports_gap_and_cash_separately`）。

## 3. wf8 实测（独立 worktree `/tmp/wt-366`，独立 `CARGO_TARGET_DIR`，纯态）

命令（沿用既有 env-gate）：

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_CENTER_OSCILLATION=1 OPSEM_DUMP_DIR=... \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

BTC wf8（2023-08-17..2024-02-16，264960 bar），**同一 worktree 内 HEAD 纯态 vs 本票**对照：

| 读数 | #366 前（HEAD 1ed977f00d） | #366 后 | 变化 |
|---|---|---|---|
| `trigger_attempts` | 1437 | 1437 | 不变（分母=次级别买卖点数，判据不改分母） |
| `dropped_center_not_alive` | 113 (7.9%) | 113 (7.9%) | 不变 |
| `dropped_other`（**PriceOutsideZone 主体**） | 768 | **961** | **+193**（新判据更严：离开中枢 vs 中轴二分） |
| `dropped_center_moved_down`（新桶） | —（无此桶） | **37** | 「中枢不下移」过滤实际拦下 37 次 |
| 落地触发（`Ok` 构造） | 556 | **326** | **−41.4%** |
| 动作总数 | 647 | **349** | **−46.1%** |
| ├ `reduce` | 556 | 326 | −41.4% |
| └ `replenish` | 91 | **23** | **−74.7%** |
| `cover_by_side`（冲抵明细） | 8 条 | **{}** | 归零 |
| `loss_round_trip_accounted_count` | long 2 / short 1 | **{}** | 归零 |
| `defense_units_exceed_current_holding_count` | long 4 / short 7 | long 1 | −11→−1 |
| `replenish_triggered_but_full_count` | short 1 | {} | 归零 |
| `unclosed_write_off_*`（新桶四项） | — | **全零** | 见下 §3.1 |
| `other_violation_count` | 0 | **0** | 接线/记账错误警报恒 0 ✓ |
| `execR` / `R` / `MaxDD` | +4040483 / +4417092 / 0.0917 | **逐位相同** | 短差仍是自包含旁路账本 ✓ |

### 3.1 未闭合减出核销 = 零读数（照实，090）

本窗 `suspension_by_source` 中 `("long","broken_by_third_class_sell")` 与
`("short","broken_by_third_class_buy")` 两键**均不存在**（前后两版皆然）——即 wf8 窗内
**没有发生过**「多头挂起遇三类卖点 / 空头挂起遇三类买点」的终局，故核销四桶全零。
这不是接线未生效：核销路径的端到端接线由单测覆盖
（`gate_on_third_class_sell_produces_write_off_request_not_a_cover_action`、
`drive_campaign_wiring_settles_write_off_and_reports_gap_and_cash_separately`），
wf8 只是未产出该形态样本。**声明=能力**：本票核销路径在真实窗口**未被数据触达**，
其读数留待其他窗口/#384 终验。

### 3.2 减/补失衡（新判据的真实后果，留给 #384）

回补动作 −74.7%（91→23）而减仓 −41.4% ⟹ 减/补比从 6.1:1 拉大到 14.2:1，挂起悬着不收口的
比例显著上升。成因是判据不对称：高抛只要 `price>=ZG`，回补要 `price<=ZD` **且**中枢不下移。
这是缠师原文口径的直接后果（89 课本就把回补条件写得比高抛严），非实装缺陷；但它把
「未闭合减出」从边缘情形推向常态，**加重了 §3.1 核销路径的重要性**——本窗零读数不可外推。

## 4. 双锚零翻动（默认关）

```
M8_WIN_FILTER=wf8 VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/wt366_default_dump \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture

cmp /tmp/wt366_default_dump/trades.jsonl       /tmp/vocab_align_dump/trades.jsonl       → IDENTICAL (948340B)
cmp /tmp/wt366_default_dump/tower_events.jsonl /tmp/vocab_align_dump/tower_events.jsonl → IDENTICAL (354946B)
```

构造性理由：本票改动全部落在 `if let Some(hist) = pan_div_hist`（`config.center_oscillation.enabled`
为 false 时恒 `None`）之内，默认关时整段不执行。实测与构造性理由一致。

## 5. 测试读数（双侧）

| 侧 | 结果 |
|---|---|
| `cargo test --lib`（debug） | **2064 passed / 0 failed / 141 ignored** |
| `cargo test --release --lib` | **2062 passed / 0 failed / 141 ignored** |

发车基线 2043 passed（debug），本票净新增 **21** 条：判据组 +5（10 新条替换 5 条中轴二分旧条）、
两终局分账 +4、campaign 核销 +5、桶层核销 +3、fill 接线 +4。
release 少 2 条 = 既有 `debug_assert` 相关用例的既有差异（非本票引入）。

慢锁（`--ignored` 重型族）除本报告所载 wf8 两臂 + 默认关一臂外未跑，留重型窗口。

## 6. 未能判定 / 如实标注

1. **核销路径在 wf8 零读数**（§3.1）——单测覆盖，真实窗口未触达，不得声称「已实测验证」。
2. **`Forfeit` 三源（Reset/Superseded/RebaseVanished）的清算口径未经 #366 复核**——票面只裁
   三买/三卖两终局，本票按 #292 既有口径保留，未改亦未验证其与补充裁定教义链
   （「中枢唯一定理三终结，无第二种死法」）是否相容。这是本票范围外的悬置项，建议 #384 或
   后续票复核。
3. **边沿恰等含端点**（`price == ZG`/`ZD` 计入离开）是实现决策，非原文明文，可复检。
4. **减/补失衡是否需要口径调整**（§3.2）不在本票裁定范围，只呈报读数。
5. 多窗读数未跑（慢锁纪律），wf8 单窗结论不外推。
