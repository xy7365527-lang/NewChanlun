# #1343 受影响代码清单（同价端点与极值根集）

> 票：[#1343](https://github.com/xy7365527-lang/NewChanlun/issues/1343)；教义正本：[`bi.md`](../../definitions/bi.md) §7 / §7.1、[`baohan.md`](../../definitions/baohan.md)「组锚与序号关系」。
> 基线：**main `3da0b2c872e6e7d69ffcfe29f989a08491d92dd1`**。#1394 财务线 `0d8a6017` 无受影响项。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票（票 A = 既有 [#1405](https://github.com/xy7365527-lang/NewChanlun/issues/1405) TB-02-V），本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/parser/stroke.rs:1-4` | main `3da0b2c8` | header：同价不再是未定事实；措辞改引 #1343 裁定 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:14` | main `3da0b2c8` | RULE 升 new-bi-dual-coordinate/2；新增 SAME_PRICE_POLICY 常量并进 StrokeFacts | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:17-27` | main `3da0b2c8` | Endpoint.raw_position: Option<usize> → raw_first/raw_last: usize（极值根集首末 raw 序位）；extreme_roots 保留 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:181-188` | main `3da0b2c8` | endpoint_at：删 raw_extreme_identity_tie waiting；roots 空改为不变式（debug_assert + fail-loud） | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:201-205` | main `3da0b2c8` | raw_position 计算改为 positions[min roots]/positions[max roots] | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:224-249` | main `3da0b2c8` | pair：selection 只有 KEEP/REPLACE；EQ 时 selection=KEEP、comparison=EQ、证据标签 same_kind_equal_price_keep_earlier；retained_anchors 只留先者 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:259-262` | main `3da0b2c8` | raw_between = b.raw_first − a.raw_last − 1（极值区）；不再 Option | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:279-280` | main `3da0b2c8` | 删 actual_extreme_mapping_missing 补标 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:326-332` | main `3da0b2c8` | build_stroke_facts_with_positions doc：去「EQ 未定」 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:368-372` | main `3da0b2c8` | ScanState 删 blocked 字段 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:382-390` | main `3da0b2c8` | 删 blocked 直通分支（earlier_endpoint_identity_unsettled 退役） | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:393-407` | main `3da0b2c8` | 删 UNDETERMINED 分支与弹笔；EQ 走 KEEP | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:410-428` | main `3da0b2c8` | REPLACE 分支去 waiting 门（only_unselected_roots 判定退役），直接延末笔/换 current | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:452-460` | main `3da0b2c8` | 删异型未知阻断分支（条件向量恒为三个 Some） | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:468-508` | main `3da0b2c8` | ScanCheckpoint 删 blocked；restore 去「pop 旧尾再恢复」分支 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:704-743` | main `3da0b2c8` | check_incremental_prefixes 断言文案去 blocked；保留作 P2/P4 锁 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:838-864` | main `3da0b2c8` | tb02b_threshold_crossing_roots_remain_unsettled：期望改为确定向量（极值区），改名 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:866-911` | main `3da0b2c8` | tb02b_strict_replacement_before_first_stroke_keeps_all_roots：raw_position 断言改 raw_first/raw_last | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:995-1032` | main `3da0b2c8` | checkpoint 测试去 blocked 断言 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:1034-1070` | main `3da0b2c8` | tb02b_checkpoint_restores_popped_tail_before_extension：弹笔场景不复存在，改为「EQ 不弹笔 + 后续 REPLACE 延笔」锁（syn1） | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:1177-1196` | main `3da0b2c8` | hand-oracle 驱动测试：UNDETERMINED/ambiguous_root_anchor 分支按新 rule_revision | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/stroke.rs:new` | main `3da0b2c8` | 新增测试锁：六见证期望、尾5万根 strokes=4016+指纹、前缀阶梯、选择无关性质测试、双进程哈希 | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/mod.rs:611` | main `3da0b2c8` | 读 e.raw_position 处改 raw_first（稀疏坐标探测语义不变） | #1405（票 A） |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:729-733` | main `3da0b2c8` | equal_extreme_identity 保留为证据标签；审计无消费者把它当准入门（tb02a_facts.rs:162/204、s_structure_session.rs:4834 只转发） | #1405（票 A）（审计） |
| Rust | `rust/src/bin/s_session_v2/tb02b.rs:24` | main `3da0b2c8` | policy_version 并列记录 RULE 与 SAME_PRICE_POLICY | #1405（票 A） |
| Rust | `rust/src/bin/s_session_v2/tb02b.rs:259` | main `3da0b2c8` | raw 范围 start.extreme_roots[0]..=end.extreme_roots[0] → start.raw_first_src..=end.raw_last_src（覆盖整个极值区） | #1405（票 A） |
| Rust | `rust/src/bin/s_session_v2/tb02b.rs:290` | main `3da0b2c8` | event_at 取端点极值区首根 ts（代表，注明不进判据） | #1405（票 A） |
| Rust | `rust/src/theta_v0/backtest/tick.rs:410-446` | main `3da0b2c8` | seam_btc_tick_bars_yield_non_trivial_structure 转绿作为验收（不改断言） | #1405（票 A） 验收 |
| Rust | `rust/src/theta_v0/backtest/tick.rs:450` | main `3da0b2c8` | calibration_sweep_finds_non_collapsed_segments：#1392 列为同价域红项，票A 实测后填状态 | #1405（票 A） 验收 |
| Rust | `rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:2535` | main `3da0b2c8` | m3_partition_synthetic_holds：#1392 列为同价域红项，票A 实测后填状态（可能另有成因） | #1405（票 A） 验收/分诊 |
| Lean | `formal/Origin/ChanlunElements.lean:29-40` | main `3da0b2c8` | Fractal/Stroke 单 Index，无 raw/merged 双域与根集合；本票裁定无 Lean 定义式可对齐（如实登记） | Lean 跟进义务（不挡#1405（票 A）） |
| Lean | `formal/Origin/ChanlunElements.lean:130-137` | main `3da0b2c8` | strokesOf 接口：建议补同型归约（≤ 取先）与 raw 域条件 2（极值区）的定义式，作为本票退场条件的 Lean 侧承载 | Lean 跟进义务 |
| Lean | `formal/Origin/SegmentAutoConstruct.lean:103/115` | main `3da0b2c8` | 线段层分型严格 < 与「找第一个」：先到惯例旁证，不改 | — |
| Python | `s_session/tests/fixtures/tb02b/hand-oracle.json:119,240,248,256` | main `3da0b2c8` | tie_top / mirror_gap2_raw3 selection UNDETERMINED→KEEP、retained_anchors 只留 5；raw_root_tie / mirror_raw_root_tie 期望改为确定向量（极值区）；以新 rule_revision 发布，旧 object 不删不改 | #1405（票 A） |
| Python | `s_session/tests/tb02b_verify.py:30-34` | main `3da0b2c8` | ambiguous_root_anchor 断言（raw_position is None、raw_extreme_identity_tie）改为 raw_first/raw_last 与 extreme_roots 集合 | #1405（票 A） |
| Python | `src/newchan/a_stroke.py:164-182` | main `3da0b2c8` | 族I 对照实现：不改；登记其同型去重无相等分支、raw 间隔用组边界（bi.md §4 既有偏差） | —（TB-10 旧入口） |
