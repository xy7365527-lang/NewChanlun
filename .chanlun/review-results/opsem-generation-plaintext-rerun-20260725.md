# opsem dump generation 明文补打 + 重跑：exit_type × 级别 × 代际 分桶（issue #262）

- 日期：2026-07-25（跑批 06:28–06:29 EDT）
- git HEAD：`93abf6496a84318f84ae007f53b5e7a3f4535e36`（main，与 #153 终验B 重跑同点 + 本报告所述 1 处 dump 改动）
- 执行人：wayfinder 子代理（未 git mutation / 未回关 issue / 未评论 / 未动既有脏文件）

## 1. 改动位置与语义说明

**唯一改动**：`rust/src/theta_v0/backtest/opsem_dump.rs` `OpsemDump::write_trade`（原 257 行附近），+6/−2 行。
在 `"position_node_id":<hash64>` 之后**纯追加** `"generation_plain":<u32>` 字段；hash 字段逐字保留，JSON 其余键不动。

**generation 语义（照实记录，与任务票猜测不符处明示）**：

- `PositionNodeId.generation`（`rust/src/theta_v0/strategy/interp.rs:298-299`）：**「campaign 代次（同 carrier 顺序 campaign 单调递增，close→reopen 高水位 +1）」**——即同一 carrier（= `voice_id`，`ElementId{level, ordinal}`，持仓容器）被平仓后再次开仓的序号，首 campaign = 0（runner.rs:3903-3904 单测锚定：首 campaign generation=0、同 carrier 再入场 generation=1）。
- 任务票猜的「父子嵌套的代数」**不是**该字段语义。代际轴读的是「同一结构声部第几次 campaign」，不是父子树深度。
- `PositionNodeId` 本身携带 generation（四元组分量之一，interp.rs:291-300），**无需回溯源结构**，直接读 `t.position_node_id.generation`。
- 生产构造点：`LedgerOpen` 入场登记（generation=同 carrier campaign 高水位），关腿写入 `TypedTrade::position_node_id`（ledger.rs:51-56）。

**纯追加验证**：新 dump（`/tmp/v4_C5_rerun_gen_20260725/trades.jsonl`，856 行）vs #153 dump（`/tmp/v4_C5_rerun_20260725/trades.jsonl`，856 行）逐行 JSON 解析后去掉 `generation_plain` 再比——**0 行差异**；`tower_events.jsonl` 两跑**逐字节一致**（`cmp` 通过，3183 行 / 354946 字节）。

## 2. 跑批命令与环境（#153 同口径）

```
cd /Users/silencehan/Projects/NewChanlun/rust
VOICE_EXEC=1 OPSEM_DUMP_DIR=/tmp/v4_C5_rerun_gen_20260725 \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

- `THETA_NEST_CERT_GATE`：未设（无门直通，与 C5 原轨一致）。`M8_WIN_FILTER`：未设（三窗全跑）。`RUSTC_WRAPPER` 未设。
- 增量编译 22.99s + 测试 33.44s，`test result: ok. 1 passed`，EXIT=0。运行日志 `/tmp/v4_C5_rerun_gen_run.log`，分析脚本 `/tmp/analyze_gen.py`。
- [m8] 三窗总账行与 #153 报告第 2 节**逐字一致**（p3fold execR=-2478192 / wf7 -365958 / wf8 -659473，MaxDD 0.1527/0.0516/0.0880）⟹ 生产行为零变化（dump 只读外化）。

## 3. exit_type × 级别 × 代际 全表（wf8 逐笔，856 笔）

代际取值集合 g0–g25（26 个代次）。单元格 = 笔数 / `pnl_raw_unlevered` 总和；「—」= 0 笔。

**CloseRoot**（合计 629 / +4387.34）：

| 级别 | g0 | g1 | g2 | g3 | g4 | g5 | g6 | g7 | g8 | g9–g24 | g25 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| L0 | 502 / +1427.37 | 20 / −71.95 | 1 / −2.52 | — | — | — | — | — | — | — | — |
| L1 | 28 / +695.38 | 13 / −658.56 | 10 / −34.88 | 7 / +66.50 | 3 / +45.29 | 1 / +3.32 | — | — | — | — | — |
| L2 | 1 / −19.28 | 1 / −6.17 | 1 / +25.57 | 1 / −7.80 | 1 / +10.80 | 1 / −14.53 | 1 / +11.12 | 1 / −108.45 | — | — | — |
| L3 | 3 / +2535.57 | 2 / +19.99 | 2 / +2.55 | 2 / +24.65 | 2 / +3.83 | 2 / +3.45 | 2 / −24.94 | 2 / −10.51 | 2 / +14.11 | 各 1（见下行） | 1 / +435.99 |

L3 CloseRoot g9–g24（各 1 笔）：g9 +0.00 / g10 −0.06 / g11 +5.58 / g12 +1.07 / g13 −4.60 / g14 −0.01 / g15 +3.32 / g16 −1.41 / g17 +5.51 / g18 +15.06 / g19 −6.67 / g20 +0.19 / g21 +7.64 / g22 −0.01 / g23 +8.05 / g24 −12.22。

**ReduceCore**（合计 6 / −315.71）：L0 g0 = 5 / −419.07；L2 g0 = 1 / +103.36；其余全 —。

**CloseShortDiff**（合计 221 / −525.74）：

| 级别 | g0 | g1 | g2 | g3 | g4 | g5 | g6 |
|---|---|---|---|---|---|---|---|
| L0 | 166 / −471.87 | 6 / +33.30 | — | — | — | — | — |
| L1 | 18 / −108.77 | 9 / −1.12 | 6 / −4.13 | 4 / +27.90 | 2 / +38.66 | 1 / −6.91 | — |
| L2 | 3 / −36.75 | 1 / +5.58 | 1 / +1.07 | 1 / −4.60 | 1 / −0.01 | 1 / +3.32 | 1 / −1.41 |

**Hold / RiskExit**：0 笔（全代际）。

**代际 × 级别边际（笔数）**：L0（700）= g0:673、g1:26、g2:1；L1（102）= g0:46、g1:22、g2:16、g3:11、g4:5、g5:2；L2（18）= g0:5、g1–g6 各 2、g7:1；L3（36）= g0:3、g1–g8 各 2、g9–g25 各 1。

## 4. 与 #153 表的总数对账

- 代际轴聚合回 exit_type × 级别，与 #153 报告第 4 节新跑表**逐格一致**：CloseRoot 629 / +4387.34（L0 523/+1352.90、L1 62/+117.05、L2 8/−108.74、L3 36/+3026.13）；ReduceCore 6 / −315.71；CloseShortDiff 221 / −525.74；Hold 0；RiskExit 0。**总 856 笔 / +3545.89，对账通过。**
- #153「L3 ord=2 于同一批结构点位形成 25 条互异 nid 的 1-bar 链」的间接推断，代际轴直接证实：**ord=2 单 carrier 26 代（g0–g25）**——g0–g24 为 25 条 1-bar prune CloseRoot（entry = 14011, 14023, 14058, 14086, 14280, 14563, …, 17965，每个 L3 结构事件点开仓、次 bar 子树清仓），g25（entry=18252）hold=199、+435.99。
- #153「2 条共有长持 +2537.27」精确分解 = ord=21 g0（hold=8127，+2530.04）+ ord=2 g0（hold=1，+7.23，与 07-22 基线同 nid）；#153「34 条仅新跑 L3 合计 +488.86」= 其余 33 条 1-bar prune（+52.87）+ ord=2 g25（+435.99）= +488.86 ✓ 精确闭合。

## 5. 代际轴读出的关键事实

- **L3 的 1-bar prune（34 笔，占 L3 36 笔的 94.4%，合计 +60.10）**：分布在 g0–g24；g0–g8 每代 2 笔（来自 ord=2 与 ord=24 两条 carrier 链），g9–g24 每代 1 笔（全在 ord=2 链上）。L3 = ord=2（26 代）+ ord=24（9 代）+ ord=21（仅 g0 长持）三条 carrier 链。
- **L1 的 1-bar prune（76 笔，占 L1 102 笔的 74.5%，合计 +130.66）**：g0:28 / g1:17 / g2:15 / g3:10 / g4:5 / g5:1；最深链 ord=262、ord=464 各 6 代（g0–g5），pnl 分别 +14.20 / −16.54；5 代链 ord=504 +250.94 是 L1 最大正贡献链。
- **L0 高度集中于 g0**（673/700 = 96.1%）：L0 几乎不重开；多代际（g≥1）主要发生在 L1（56/102）与 L3（33/36）。全窗 52 条 carrier 出现多代际（g≥1 同 carrier 多笔）。
- 代际号与级别正相关：L0 最深 g2，L1 最深 g5，L2 最深 g7，L3 最深 g25——越高级别 carrier 越少、单 carrier 被 close→reopen 复用次数越多。

## 6. 回归检查

```
cd rust && cargo test --lib
test result: ok. 1917 passed; 0 failed; 136 ignored; finished in 6.27s   EXIT=0
```

- 与基线 1917 全绿一致，0 failed。日志 `/tmp/cargo_test_lib_gen_20260725.log`。
- 既有 dump 消费测试 `opsem_dump_env_gated_bit_exact`（runner.rs:3316）断言方式为 `contains` 既有字段，纯追加新键不受影响（本批全绿背书）。
- 未跑 release 全量（按任务票豁免）；release 单测 m8_e2e_all_systems_oos 已单独跑过（第 2 节，1 passed）。

## 7. 未能判定项与诚实边界

- **任务票的 generation 语义猜测（父子嵌套代数）不成立**——实际是同 carrier campaign 代次（第 1 节）。若 issue 想要的是父子树嵌套深度，需另开字段/侧车，本报告未实装也不伪造。
- **「代际链是否教义预期」仍超出数据本身**（同 #153 第 8 节末条）：代际轴证实了 ord=2 26 次 close→reopen 的事实与点位序列，但「该快速链语义 vs 旧长持语义」的裁定属教义层。
- 代际轴只在 wf8 末窗逐笔上读出（OPSEM_DUMP_DIR 三窗逐窗覆写，同 #153 口径）；p3fold/wf7 的代际分布未取。
- pnl 口径为费前未杠杆（opsem_dump.rs:243-245），与 [m8] 含成本总账不可互证（费率未标定，数值禁作 alpha 论据）。
- `generation_plain` 取值上界（26 代）是数据事实，不代表结构上限；`generation` 为 u32 单调高水位，无回绕机制可证（当前数据量级下无意义，仅照实标注）。
