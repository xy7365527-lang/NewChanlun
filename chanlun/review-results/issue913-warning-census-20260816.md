# #913 编译警告勘察：全量测量 + 逐条分类表（先量再动·第一步）

- 日期：2026-08-16
- 票据：#913（debt）
- 方法：`cargo build --release`（全新 CARGO_TARGET_DIR，非 check 缓存重放）全量警告采集 + 逐条消费方 grep

## 总量（release 全量构建，150 条警告）

| 类 | 条数 |
|---|---|
| dead_code（never used/read/constructed） | 115 |
| unused_imports | 14 |
| 其他（unused_mut/unused_variables/cfg 废弃等） | 21 |

**结构性发现（本票的元答案）**：`theta_v0/mod.rs:117` 的 `#[cfg(any(test, feature = "backtest_bin"))] pub mod backtest` ⟹ **裸 release 构建（无 feature、非 test）下，凡只被 backtest/test/探针 bin 消费的代码全部报 dead/unused**——150 条里绝大多数是这一类「feature 门后活件」，不是垃圾。直接 `cargo fix` 会把测量装置与生产 feature 件一起毁掉（票面担心实锤）。

## dead_code 逐条分类（115 条 → 四桶）

### A 桶：探针/诊断 sidecar（留；应 `#[allow(dead_code)]` + 注释带服务票号，待批量落地）

| 位置 | 服务面 |
|---|---|
| `signal.rs` `otherwise_domain_sidecar_begin/take`、`t3_in_c_grade_reason_to_pan_div_subtype`、`pan_div_subtype_channel_reason`、`PanDivShortRetraceObservation::from_record`、`drain_pan_div_short_retrace_observations` | #607 D4 观测面 / pan-div 通道诊断（econ 层探针消费） |
| `pipeline.rs` `historical_bound_third_cert`/`historical_bound_segment` | #487 挂起历史三类复核（fill.rs:1076 消费，feature 门内） |
| `strategy/shadow.rs` 全家（VoiceKey/ProductionFact/ShadowStepRecord/ShadowStats/ShadowVoiceBook/observe_and_compare/render_report 等 9 条） | 影子账对照装置（fill.rs:4416 消费，feature 门内） |
| `strategy/coverage/sizing.rs` `level_cap`/`clamp_levels_to_weighted_cap`/`position_bounds` 等 4 条 | LEE 容量闸门（fill.rs:5195 消费 + #890 S7 在飞接线） |
| `strategy/coverage/{compose,element,held,role}.rs` 6 条（`pi_theta_step_traced`/`held_leg_tree_index`/`operation_role_indexed` 等） | 覆盖步追踪/held 对位诊断（test 子树消费） |
| `recursive_t/` `t_engine::cost_basis`、`rec_engine::{reading_b_diverge, sink}` | C7-E2/E3 名分在案的 GUARD-ROLE 对照臂 |
| `fugue_v3/operate.rs` `dbg_sink_ok` | C7-E5 名分在案 |
| 旧引擎共享件（`bin/../{buysellpoint,divergence,macd,moves,segment,stroke,zhongshu,bi_engine,level,ph}.rs` 的 `as_str`/`Incremental*` 等 ~40 条） | 46 个探针 bin 的共享基件（单 bin 不消费全量成员，跨 bin 合集在用） |
| 探针 bin 未读字段（p102/p106/p109/p112/p113/p117/p122/p124/p940/strict_nest_check ~20 条） | 探针诊断载荷（为报告完整性写字段，不读回） |

### B 桶：有票在飞

- `sizing.rs` 容量闸门族 → **#890（S7 毛敞口接进决策路径，OPEN）**；
- `trading/ledger.rs::ShortBook`/`DirectionalBook`、`trading/third_point_book.rs` 全家 → C7-E4 名分层（29 文件纯标记，活物另案）——**归 #529 闭图登记的对照臂纪律，非本票删除面**。

### C 桶：feature 门后活件（非死）——见结构性发现，占绝大多数。

### D 桶：真孤儿（无票/无注释/无调用/无历史线索）——**0 条**

逐一 grep 消费方后，本批 115 条无一落入「四无」。**本仓警告零孤儿。**

## unused_imports 14 条处置（本票已落地）

| 位置 | 处置 |
|---|---|
| `classifier/mod.rs` 6 组（ThetaConfig/Segment/cache_series_ok/tower_cache 两件/Side/recursive_tower 三名） | test-only 消费 ⟹ `#[cfg(test)]` 门（#648 T2 prelude 的 test 消费面坐实） |
| `divergence.rs:61` Stroke / `rmove_compose.rs:60` Direction / `coverage/mod.rs:127` coverage_step_from_buckets_sep / `coverage/mod.rs` level_cap | 同上，`#[cfg(test)]` 门 |
| `signal.rs:104` HashMap | **真死**（使用点全 qualified）⟹ 删除 |
| `shadow.rs:56` VoiceVerdict、`coverage/mod.rs:141` TwStepCtx、`bin/p409:66` | test 子树消费 ⟹ `#[cfg(test)]` 门 |

## 残余（本票不关的部分）

- A 桶 ~110 条的 `#[allow(dead_code)]` + 服务票号注释**批量落地**（机械批，按上表执行即可）；
- `unused_mut`/杂项 21 条的逐个清理（基本可直接清）；
- `rec_engine.rs:3456` 的 `τ`：按票面**加 `#[allow(mixed_script_confusables)]`**（有意符号，不改名）。

## 验证

落地子集后：release 构建警告 150 → **139**（11 条消除）；全量 lib 2691/0/153 绿；fmt 0 Diff；白名单守卫绿（七点随迁重锚在案）。
