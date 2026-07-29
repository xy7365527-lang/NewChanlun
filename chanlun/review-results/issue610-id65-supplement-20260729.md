# #610 补记：ID-6.5「面 A 二次漂移登记」补齐 level_origin 面

- 日期：2026-07-29｜票据：issue #610（处置，sonnet 常规档）
- 工位：`/tmp/kimi-nest-mainline`（只动 `signal.rs` GOLDEN 常量/注释 + 本登记文件）
- 前置：`chanlun/review-results/issue610-digest-guard-attribution-20260728.md`（归因调研，已实证引入提交 `bbbd8f89fa` + 双因机制）

## 背景：既有登记的覆盖缺口

`chanlun/review-results/owner-attribution-fix-readings-20260724.md` §5 已就
`extract_signals_bit_exact_digest_guard` 登记过一次 ID-6.5（`0x90c7_9ee6_17e1_1392` →
`0xe6a2_63e3_43e4_3845`），但归因文本只覆盖了 `bbbd8f89fa` 两处 Debug 面改动中的**一处**
（`BspPoint.center: Option<Center> → Option<OwnerRef>`，#218 面 A）。该次登记未提及同一提交
**同时**新增的 `level_origin: u32` 字段（#110 面，`bsp.rs` 三处构造点全部硬编码 `0`，进入
`#[derive(Debug)]`，247 点 Debug 串每条尾部新增 `, level_origin: 0`）。且该登记落档后 GOLDEN
常量本身未被同步改写（`signal.rs` 仍是旧值），guard 持续红，直到本票（#610）实际执行重锚。

本文件补齐 level_origin 面的登记，与 07-24 §5 的 center 面登记合并构成完整的 ID-6.5 履行记录。

## 登记表（补充行，格式同 07-24 §5）

| 面 | 旧值 | 新值 | 归因 |
|---|---|---|---|
| `extract_signals_bit_exact_digest_guard` GOLDEN 实际值（level_origin 面，补记） | `0x90c7_9ee6_17e1_1392`（07-24 §5 已登记 center 面漂移，但 GOLDEN 常量当时未随之改写，guard 持续红） | `0xe6a2_63e3_43e4_3845` | **同一提交 `bbbd8f89fa` 内与 center 面同时发生的第二因**：`BspPoint` 新增 `level_origin: u32` 字段（#110 面，`make_first_point`/`make_second_point`/`make_third_point` 三处构造硬编码 `0`），进入 `#[derive(Debug)]` ⟹ 247 点 Debug 串逐条尾部新增 `, level_origin: 0`。该字段全仓库恒为 0、从未真正接线赋非零值（issue #434 独立实证，交叉互证一致）；本次登记不评价 #434 应删除该字段还是补写真实值——若 #434 判定删除，本次重锚窗口随之过期，届时需第三次重锚（issue610 归因报告 ④ 已预警此路径）。六 bit + pivot + center + struct_break_dir 逐字段不变仍由 `extract_signals_bit_exact_vs_orig_per_case` 锁定（level_origin 恒常量 0，per-case 对拍无需额外覆盖） |

## 结论

- ID-6.5「既线失败在案 + 实际值二次漂移归面 A」的登记义务，经本文件 + `owner-attribution-fix-readings-20260724.md` §5 合并，**两处成因（center 面 + level_origin 面）均已登记完毕**。
- `signal.rs` GOLDEN 常量本次（#610）同步重锚为 `0xe6a2_63e3_43e4_3845`，与两份登记文档的「新值」列一致。

## 边界条件

若未来 #434 判定删除 `level_origin` 字段（而非补写非零值），该字段退出 `#[derive(Debug)]`，摘要将发生第三次漂移，须按同一先例（本文件 + #308 先例）再次诚实重锚并登记——不沿用本次值静默放行。

## 影响声明

新增登记文件（补齐 ID-6.5 义务），未改动任何生产逻辑；与 `signal.rs` 的 GOLDEN 常量/注释改动构成同一票（#610）的两个交付物。
