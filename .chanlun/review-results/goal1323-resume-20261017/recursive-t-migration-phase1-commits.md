# recursive_t 测试迁移第一阶段记录

- 目标：#1323 / SPEC #1340 Destination ③，降低旧引擎测试对 deprecated `backtest.rs` / `backtest_run.rs` 的依赖。
- 基线：分支 `codex/1523-1525-launcher-riskgate-v1`，HEAD `381fda3f70`；与审计报告记载的当前 HEAD 一致。
- 审计依据：同目录 `recursive-t-test-migration-report.md`，并复核 Rust 源引用与 runner/API。
- 第一阶段结论：**没有证据充分的可直接替换测试；本阶段迁移数为 0。** 按要求不移除测试、不注释 import，也不把语义不同的 flat runner 当作等价实现。

## 证据与分组

1. `cd rust && cargo test --no-run`：exit 0，约 26.9 秒；仅见已有 warning。证明当前基线测试可编译，不构成旧支退出证据。
2. 源码搜索命中 `rust/src/recursive_t/{mod.rs,backtest_run.rs,center.rs,rec_stream.rs,t_engine_run.rs,stream.rs,types.rs,rec_engine.rs}`。共 95 个源码匹配行（包括调用、注释和命令示例；不是测试函数数量）。外部 `rust/tests/` 未见直接调用旧 T runner。
3. `backtest_run.rs` 可见数据依赖测试：`t_backtest_8x3`、`t_cseg_diagnostic`、`t_consol_no_leave_diagnostic`，均为 ignored 重型真实数据诊断；模块本身在 `mod.rs` 由 `#[cfg(test)]` 门控。
4. `center.rs` 的 `靶向_相切延伸口径逐级对照` 为 ignored 真实数据口径对照；消费旧 loader 与批量 a₀ 构造。尽管统计算法主体属于 center，本测试启动/数据及 a₀ 管线尚未迁出。
5. `rec_stream.rs` 中对旧 loader 的真实数据诊断/对拍包括：`rec_flat_btc_bit_exact`、`rec_btc_consol_no_leave`、`rec_btc`、`prop4_nest_l3`、`prop4_consume_l3`、`prop4_reading_b_recursive_l3`、`prop4_intrinsic_quota_l3`、`prop4_reading_b_pair_l3`、`lconfirm_4axis_l3`、`faceb_l2_validate`、`faceb_l2_bear_window`、`facea_l2_validate`、`bear_validate_core_short_l3`、`btc_bull_bear_segments`、`scale_invariance_1s`、`btc_no_reentry_trace`。均按审计记录保留为 ignored 或数据驱动诊断；其中第一项本身用于 rec≡flat bit-exact 验证。
6. `t_engine_run.rs` 是 flat 现役支测试 runner，但目前其真实数据诊断仍复用旧 loader；`t_level_structure_dump`、`t_downtrend_diagnosis`、`t_bsp_dump` 还引用旧 batch a₀ 构造。`t_engine_8x3` / `t_bsp_op_diagnosis` 通过共用 runner/loader 依赖旧 backtest harness。runner 名称相似不证明其与 rec 支诊断等价。

## 替代入口核验与阻塞项

- 已找到候选 flat runner：`TFugueStreamCore::push_bar/finish`（`stream.rs`）；同属现役 flat 支的测试 runner 是 `t_engine_run.rs`。旧 batch a₀ 入口为 `backtest::build_a0_from_segments`。
- 候选 `stream::build_a0_fast` 为 crate 私有函数，要求 orchestrator 的 segments/strokes、merged-to-raw 和 online MACD 前缀；它使用 online MACD，旧 batch builder 使用 batch MACD。并且 stream runner 的成交/确认语义不同于旧 batch backtest。替换这些真实数据诊断前，必须在同一输入上对拍对应观测量。
- 候选 `theta_v0::backtest::data::load_by_symbol` 返回保留异常 bar 并标记不可交易的 Dataset；旧 `load_clean_ohlc` 删除异常/spike bars。序列和坐标变化使直接替换不保语义。
- 因此 **第一批为空**；不能以 import 可编译或 flat runner 可运行作为迁移证据。

## 阶段归类与下一步

- **第一批（直接替换）**：空。当前没有找到同时满足目标语义不依赖 rec 特性、数据入口一致且现役 flat API 逐项等价的测试。
- **第二批（先对拍）**：center 的相切延伸真实数据诊断；rec/flat BTC bit-exact；各 rec 真实数据回测/prop4/Face/bear/1s 诊断；flat t_engine runner 的旧 loader 与 a₀ 构造依赖。先建立同数据 fixture/runner 对拍并逐个锁定输出，再迁移。
- **保留批**：loader、日期/时间窗清洗工具、符号映射及上述 ignored 数据诊断，需保留至共享 loader/API 有经验证的替代归属；旧 backtest 文件本阶段不删除。
- 单元测试 `iso_day_to_unix_ns_锚点与边界` 是 loader 内纯日期算术，不依赖 recursive_t 的判据或 runner；可以未来抽至共用日期模块，但此举不能迁移任何真实数据测试或解除旧 loader 依赖，因此不纳入本阶段。

## 验收与提交

- 本阶段没有测试修改，所以没有逐项 `cargo test <test_name>` 运行记录；执行了基线 `cargo test --no-run`。
- 文件仅记录阻塞与下一阶段证据门。若需继续实施第二批，应先确认/落实旧 loader 的新归属与清洗语义，再在同输入数据上对拍输出。
