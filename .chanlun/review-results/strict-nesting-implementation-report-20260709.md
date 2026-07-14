# 严格区间套 P0-P4 实装汇总报告（2026-07-09）

## 范围

本报告封存严格区间套 P4「回归与封边界」结果。P4 不改变 `divergence.rs` / `bsp.rs` / `signal.rs` 判据行为，只汇总 P0-P3 硬门、复跑全量回归，并声明外推边界。

本 worktree 的 HEAD 文件树没有 `chanlun/review-results/strict-nesting-divergence-plan-20260708.md`、`strict-nesting-rulings-20260708.md` 或 `strict-nesting-divergence-survey-20260708.md` 实体；P4 对 P0 的核对依据为前序结果文件与代码注释中已经冻结并实现的 adopted-default 口径。

## P0 adopted-default 对应关系

| P0 裁决项 | 当前落点 | P4 核对 |
|---|---|---|
| `Cand^δ_ℓ` 采用塔上 per-level 背驰段谓词 | `recursive_tower::level_cand_delta` 产 `CandDeltaEvent::cand_delta`；`nest.rs` 装配层消费该谓词 | PASS |
| A/C 段按该级 tower 段结构定位，跨中枢趋势配对 | P1 报告与 `recursive_tower.rs` 注释冻结该定位层 | PASS |
| gauge 复用 `divergence.rs` 默认 `MacdArea`，严格 `curr < prev` | `ThetaConfig::default()`，`DivergenceGauge::default() == MacdArea`；未调参 | PASS |
| 盘整背驰不入链 | `pan_div_diag` 仅诊断，不参与谓词/证书；全量末态为 0 | PASS |
| 确认时点取完成时 | `confirm_src` 取 C 段完成端点；进入时只作诊断对照 | PASS |

## P0-P3 硬门结果

| 阶段 | 硬门 | 结果 |
|---|---|---|
| P0 | adopted-default 口径冻结并由 P1/P2 消费：不新增定义分叉，不切 gauge，不纳入盘整背驰，不把进入时当确认时点 | PASS |
| P1 | 谓词输出与 `extract_signals` buy1/sell1 背驰确认支逐 bit 一致 | PASS：4613599 bar，checkpoint 4613 次，比较 26618 次，不一致 0 |
| P2 | `N^δ_{ℓ↓0}` 证书装配：E1 三元组复算一致，terminal 查无为 0，证书产量与 n_C 口径一致 | PASS：t1=942，n=1278，n_B=1，n_C=0，证书合计 0 |
| P3 | 生产重放 sidecar 接入，默认关闭 bit-exact，开启后证书流与 P2 装配层一致 | PASS：sidecar 默认不参与订单/候选/风控/账本；小窗开启验证通过 |

## P4 回归结果

| 命令 | 结果 |
|---|---|
| `cargo build --release --bin strict_nest_check` | PASS，仅既有 warnings |
| `cargo test --lib` | PASS：1529 passed / 0 failed / 125 ignored |
| `./target/release/strict_nest_check` | PASS：全量重放 1409.8s，总判 PASS |
| `git diff --quiet -- rust/src/theta_v0/classifier/divergence.rs rust/src/theta_v0/classifier/bsp.rs rust/src/theta_v0/classifier/signal.rs` | PASS：exit 0 |

doctest 说明：既有 `rust/src/theta_v0/ledger/separate.rs` 3 个 doctest 失败为前序已知豁免，本轮未触碰该文件；P4 验收门要求并已执行的是 `cargo test --lib`。

## 证书产量为 0 的口径解释

本轮证书产量 0 不是为了得到干净结果而放宽或改写判据；相反，它是严格合取后的结果。

`N^δ_{ℓ↓0}` 要求同一方向下从目标级到 L0 逐级成立：每级 `cand_delta=true`，相邻级区间满足 `Sub` 包含，锚点按完成时递降，且 L0 terminal 有对应 buy1/sell1 确认。终态全局口径下，L0 基例 `cand_delta=true` 为 452，terminal 查无为 0；高级别末态也有零散 `cand_delta=true`（L1=7, L2=13, L3=9, L4=2），但这些事件与下级事件联立后没有形成完整 `ℓ↓0` 链。

该结果与 E1 名单口径 `n_C=0` 一致。含义是「当前数据、当前默认 Θ、当前 strict adopted-default 定义下没有完整证书」，不是「严格区间套在任何数据或定义下理论上不可能产证书」。

## 外推边界

### 本实装有效域

- 代码域：当前 HEAD `strict-nesting-20260708`，前序 P1/P2/P3 提交均在链上。
- 判据域：`ThetaConfig::default()`，`l_max=6`，`min_parts_per_level=3`，默认 `MacdArea` gauge，严格 `curr < prev`。
- 定义域：`Cand^δ` = per-level 背驰段谓词；盘整背驰只诊断不入链；确认时点 = C 段完成时；证书为 `N^δ_{ℓ↓0}` 递降完整链。
- 数据域：`/tmp/codex-work-p7/analysis/data_cache/btc_1m_full.json`，4613599 根 BTC 1m bar，2017-08-17 04:00:00 到 2026-05-31 23:59:00；交易文件 `/tmp/codex-work-p7/p7_inputs/trades.jsonl`，40001 笔。
- 生产域：P3 sidecar 默认关闭；开启时只读观察 replay 同帧状态，不进入订单、候选、风控或账本。

### 不可外推场景

- 不能外推到其他标的、周期、数据供应商、数据修订、缺失 K 线、乱序 K 线或 2026-05-31 23:59:00 之后的数据。
- 不能外推到非默认 `ThetaConfig`、更深 `l_max`、不同 `min_parts_per_level`、不同 MACD 参数、或 `ThetaDom` / `Conjunction` / `Disjunction` 等非 `MacdArea` gauge。
- 不能外推到定义分叉：把盘整背驰纳入链、把确认时点改为进入时、把 `Cand^δ` 按 BSP 类型分叉、改用 bottom-up `Sel_Θ` 语义、或把结构候选扩大为非背驰段谓词。
- 不能外推到生产 alpha、执行收益、仓位/风控表现。P3 sidecar 是观察流，默认关闭，证书产量 0 也不构成收益结论。
- 不能外推到 doctest 全绿；本阶段门为 `cargo test --lib`，既有 ledger doctest 失败单独豁免。

## 结论

P4 完成。严格区间套 P0-P3 实装在当前有效域内通过全量回归；判据文件行为零改动；证书 0 产量按 adopted-default 严格口径解释并封边界。
