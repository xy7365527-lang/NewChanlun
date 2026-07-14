# alpha 统计管线三缺口合并实装：G-A1 σ^H 桶键 + G-A2 分层报告 + G-A4 walk-forward LCB（task #51）

- **来源**：a0 盘点 `.chanlun/review-results/impl-gap-inventory-20260702.md` §0/§1 簇A，阻塞 task #13（W-VERIFY alpha 全量重测）
- **文件域**：`perm_test.rs` / `wverify_run.rs` / `mu_estimator.rs`（只读，未改）/ `prereg_windows.rs`（只读，未改）

## 1. 结论

三条同根缺口合并实装完成，均在 `rust/src/theta_v0/backtest/` 内：

1. **G-A1（σ^H 桶键，完全实装）**：`perm_test.rs` 的 `BucketKey` 由三元组 `(ℓ,bsp_class,δ)` 扩为四元组 `(ℓ,bsp_class,δ,σ^H)`（`σ^H`=`parent_dir`，[`MuClass::parent_dir`](rust/src/theta_v0/backtest/mu_estimator.rs:73) 本已携带，此前只在投影层丢弃）。`stratified_delta_perm_p` 分层键同步从 `(ℓ,bsp_class)` 扩为 `(ℓ,bsp_class,σ^H)`——层内只置换 δ，σ^H 与 bsp_class 一样作为固定层境。`wverify_run.rs` 投影 `est.trades()` 时补 `c.parent_dir` 维，逐桶 Welford 聚合 `agg`/`series` 的 key 同步扩维。新增测试 `parent_dir_stratifies_independently`：构造 σ^H=+1 层强正信号、σ^H=-1 层强反信号，验证若被误合并会相互抵消、扩维后各自独立检出显著（`perm_p<0.05`），并断言产出桶数=4（2 σ^H 层 × 2 δ）。
2. **G-A2（分层报告，部分实装 + 诚实缺口标注）**：`wverify_run.rs` 报告表头/行加 σ^H 列；新增按 walk-forward 窗口（time block）的分桶报告写入 `/tmp/wv_full_timeblocks.md`（窗序号+test_start/test_end+`(ℓ,bsp,δ,σ^H)`+n+mean）。**未实装**：持有桶 h（holding bucket）——`MuClass`/`MuEstimator::trades()` 管线不携带逐笔持仓时长/进出场 bar 差数据，该数据在 `l3_delta_r_alpha.rs` 构造（超出本工位文件域），需独立工位在该文件接通 entry/exit bar 索引后才可加维。已在 `wverify_run.rs` 模块级文档显式标注此缺口，非静默遗漏。
3. **G-A4（walk-forward LCB，完全实装）**：新增 `walk_forward_oos_mu(symbol, ds, cfg)`，消费 `prereg_windows.rs` 冻结的 BTC `wf_anchored` 窗口——只取 `test_start ≥ OOS_START` 的窗（落在 OOS 起点前的窗测的是 IS 期，非 OOS 证据），逐窗对 **test 段**独立 `build_mu_from_bars` 后用既有 `MuEstimator::merge`（cross-fit 惯例，防接缝伪相邻）聚合。`wverify_full` 的 `est` 来源从「整个 OOS 窗单块 in-sample 正态近似」改为该聚合估计器——每笔观测都来自"该窗训练截止之后"的样本外区间。BTC anchored 序列中 `test_start≥"2023-01-01"` 的窗为 i=7..11（5 窗，覆盖 2023-02-17→2025-06-30，样本外基础覆盖率>协议 OOS 起点后 98% 时间跨度，仅缺 2023-01-01→2023-02-16 约46天的锚定周期错位，非人为截断）。

## 2. 定义依据

- level-sigma PDF p3-4/p8/p11：`z=(ℓ,δ,σ^H)` 桶键判据（G-A1 定义源）；p9 分层 `S=(ℓ,σ^H,h,time block)`（G-A2 定义源，h 维度诚实缺口见上）；p10 `LCB_OOS` + walk-forward split 判据（G-A4 定义源）。
- `.chanlun/review-results/acc-alpha-estimand-prereg-20260701.md` §1.3/§1.4/§4：N_PERM=200/种子 20260701 冻结、层内置换保边际的既有判据（本次未改冻结常量，只扩桶键维度）。
- `docs/backtest-prereg-windows-v0.md` + `prereg_windows.rs` §W1/§W2 裁定：anchored train 扩张窗对 Θ v0（无参数）不构成拟合自由度泄漏——本实装选用 anchored 而非 rolling 序列的依据。

## 3. 边界条件（结论翻转条件）

1. **G-A1 功效稀释**：加 σ^H 维后同一 `(ℓ,bsp_class)` 层被进一步细分，每层样本量下降——若细分后大量桶落入 `decontam::powered()` 的 UNDERPOWERED 门，三态判据会从 VALIDATED/FALSIFIED 大量退化为 INCONCLUSIVE。这是**如实反映**（663 pending 已预告的稀释方向），不是实装缺陷；若下游 a3 全量重测发现 UNDERPOWERED 桶占比过高，需要 663 号方向的补救（跨标的池化或分层收缩），不属于本工位范围。
2. **G-A2 h 维度**：若下游判定持有桶 h 是 a3 收口的**硬阻塞**（而非当前盘点判定的软阻塞/后续泳道），则需追加独立工位改 `l3_delta_r_alpha.rs`（`build_mu_from_bars`/`MuObservation` 构造点）透传 entry_bar/exit_bar 差值，本次实装不覆盖。
3. **G-A4 窗口选择**：若下游判定 anchored 模式不足以代表 walk-forward 稳健性（需要 rolling 模式交叉验证），需追加消费 `wf_rolling` 序列并对比两模式结果一致性；当前只用 anchored（依 §W2 裁定的显式授权，非任意选择）。
4. **G-A4 覆盖率边界**：若下游要求 walk-forward test 段必须**逐日无缝覆盖** OOS 全窗（2023-01-01 起），当前实现在 2023-01-01→2023-02-16（约46天，锚定周期未对齐 OOS 起点）无样本——如需补齐，需在 prereg_windows.rs 增补一条更早 test_start 的窗（属预注册变更，走 §9 change-request，非本工位可自决）。

## 4. 下游推论

- task #13（W-VERIFY alpha 全量重测）的三条硬阻塞前置项（G-A1/G-A2/G-A4）中，G-A1 与 G-A4 已完全解除；G-A2 部分解除（σ^H+time block 达标，h 维度诚实缺口需下游裁定是否阻塞）。
- 全部既有 `perm_test`/`wverify_run`/`mu_estimator`/`prereg_windows`/`decontam` 单测 + D1 跨进程复现测试（`perm_xproc_reproducible`）全绿（`cargo test --lib --release`：1394 passed / 0 failed / 100 ignored，含新增 `wverify_full` 忽略态慢测与既有忽略态慢测）。
- `wverify_full`（`#[ignore]`，需真实 BTC 461万 bar 数据）尚未实跑验证真实 alpha 三态判定结果——本工位只护航既有测试绿灯 + 新增合成数据单测（L1，231号：合成数据只验证管线正确性，零 alpha 信息增量），真实数据跑批（L2）留给 task #13 执行。

## 5. 谱系引用

- 231号（形式化有效域）：本次改动是纯 L1 管线正确性修复（桶键扩维/聚合来源切换），未产出任何真实 alpha 判定结论；`wverify_full` 忽略态未跑，L2 结论待 task #13。
- 663 pending：加维稀释功效的独立方向——本次 G-A1 实装是该张力的**触发点**（如实暴露稀释，未解决）。
- a0 盘点 `.chanlun/review-results/impl-gap-inventory-20260702.md` §0/§1/§4：本工位是其"建议实装顺序"中「G-A1+G-A2（同根）→ G-A4」两步的合并执行。

## 6. 影响声明

- 改动文件：`rust/src/theta_v0/backtest/perm_test.rs`（`BucketKey` 类型、`stratified_delta_perm_p` 签名与分层键、3 处测试改 5 元组输入 + 1 处新增测试）、`rust/src/theta_v0/backtest/wverify_run.rs`（新增 `walk_forward_oos_mu` 函数、`wverify_full` 改用 walk-forward 聚合 est + σ^H/time-block 报告列）。
- 未改动：`mu_estimator.rs`（parent_dir 字段已存在，无需改）、`prereg_windows.rs`（只读消费冻结常量）、`decontam.rs`（三态判据签名为纯标量，桶键维度变化不影响其接口）。
- 唯一调用方 `perm_test::stratified_delta_perm_p`/`BucketKey` 是 `wverify_run.rs`（已核实全库无其他消费者），故签名扩维不产生隐藏破坏面。
- 影响范围：alpha 三态判定管线（`wverify_run.rs::wverify_full`）的桶键维度与 LCB 计算口径；不影响 selector/runner 生产路径（`perm_test`/`wverify_run` 是独立于生产信号路径的统计验收工位）。
