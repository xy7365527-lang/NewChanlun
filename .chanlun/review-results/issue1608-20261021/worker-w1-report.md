# #1608 工蜂 w1 报告（分支 issue1608/w1，基于 ed337f26f3）

## 1. 提交列表

| 提交 | 内容 |
|---|---|
| c810bbca96 | 第一批：未用 import／变量、多余 mut、无用比较、生命周期省略、测试名 τ→tau、探针记录字段 allow |
| 92c9f8a8f2 | 第二批：测试专用项加 cfg(test)、backtest_bin 专用项加 cfg_attr、共用模块与待接线 API 加最窄 allow |
| 38e5dad6dc | 收尾三处残留告警（econ_positive 测试导入、StepTrace 重导出门控、内嵌测试的 lint 挂载点） |
| abc487129f | 对本票改动执行 cargo fmt（排除 s_session_v3，见疑点 5） |

未 push，未开 PR，未改 git 配置。本机没有运行过 cargo/rustc，所有编译验证都在远端 nc-remote 上完成。

## 2. 验收结果（远端终验日志：/tmp/nc-remote/20261021-095320-abc487129f.log，对应 HEAD abc487129f）

计数口径：`--message-format=short` 下 grep ": warning" 后 sort -u 去重。

| 档 | 命令 | 编译 | 总告警 | 归我文件告警 |
|---|---|---|---|---|
| C1 | `cargo check --all-targets` | Finished，0 error | 0 | 0 |
| C2 | `… --features backtest_bin` | Finished，0 error | 0 | 0 |
| C3 | `… --features s_session` | Finished，0 error | 277 | 0 |
| C4 | `… --features s_session,s_allocation_json_testonly,s_capture_native_testonly` | Finished，0 error | 240 | 0 |
| C5 | `… --features s_session,s_allocation_sqlite_testonly` | Finished，0 error | 277 | 0 |

基线（ed337f26f3，/tmp/nc-remote/20261021-091707-ed337f26f3.log）：总数 C1 164 / C2 261 / C3 451 / C4 414 / C5 451；归我的 164 / 261 / 174 / 174 / 174。

C3 到 C5 剩下的告警全部落在不归我的文件里，本票没有改动这些文件。C3 按文件去重计数如下：
s_session_v3/history.rs 214、object_index.rs 110、s_session_v2/cache.rs 96、s_session_v3/capture.rs 84、s_session_v2/mod.rs 84、s_session_v3/business_json.rs 82、source_audit.rs 74、s_session_v2/evidence_store.rs 54、s_session_v3/request_audit.rs 50、aux_index.rs 50、s_content/path.rs 48、s_session_v3/control_audit.rs 48、state.rs 44、reader.rs 36、q_wire.rs 36、q_snapshot.rs 36、audit_store.rs 36、q_watch.rs 34、q_content.rs 32、s_content/capture_allocation/work.rs 30、s_session_v3/q_runtime.rs 18、q_http_input.rs 18、projection.rs 18、catalog_evidence.rs 18、s_content/capture_allocation/audit_json.rs 16、s_session_v3/prepared.rs 16、audit_owner.rs 16、s_session_v2/tb02a.rs 16、s_session_v3/source_prefix.rs 14、native_record_fields.rs 14、driver.rs 14、s_session_v2/content_store.rs 14、s_content/fragment.rs 12、s_session_v3/q_resources.rs 12、commit.rs 10、cross_request_source_cache.rs 8、s_session_v2/content_manifest.rs 8、session_transport.rs 6、s_content/capture_allocation.rs 6、s_structure_session.rs 6。
（这是 HUMAN 段逐行计数，同一告警在多个目标里出现会重复计入；去重后的条数以上表总数为准。）

终验同时跑了 `cargo fmt --all -- --check`：只剩 `src/bin/s_session_v3/prepared.rs:1707` 一处差异，不归我，见疑点 5。

## 3. 删除清单（规则 ⑤：孤儿遗留，且很小）

1. `rust/src/fugue_v3/operate.rs` 的 `total_long_at_entry`：带 `#[cfg(test)]` 的测试只读访问器，9 行。全仓包括测试和文档都没有引用。
2. `rust/src/theta_v0/strategy/persistent.rs` 中 `mod tests` 的辅助函数 `fn voice_side(up: bool) -> VoiceSide`：7 行，测试里没有任何调用。它和 `strategy::voice::voice_side` 只是同名，不是同一个函数。

以下删除的不是代码项，是未用的 import 或冗余表达式（rustc 能证明删掉不影响行为）：
- 未用 import：p409 `std::io::Write`；p856、p977、p985 的 `Classification` 与 `std::rc::Rc`；bsp_bridge/tests.rs 的 `CandidateState`；tests/third_class_anchor.rs 的 `super::fixtures::*`；financial_content.rs 测试模块的 `NodeSink`。
- backtest/runner.rs：`TwState`、`FillOutcome`、`bar_returns`、`TwLedgerThread` 重导出、`entry_structural_stop`／`newly_confirmed_step`、opsem_dump 的 6 个 `*_str`、admission 的 `LevelFingerprint`／`NestGateObs`／`kappa_policy_resolved`／`nest_cert_gate_enabled`。这些在测试和非测试两种构建下都没有用到。
- backtest/fill.rs：`AssemblyState`、`hybrid_step`／`AssemblyEvent`、`parser`、`TwEvent`、`voice_exec_gate`、test 门内的 `NEST_CERT_GATE_OVERRIDE`／`VOICE_EXEC_OVERRIDE`、`Dataset`、`TYPED_TRADE_SCHEMA_VERSION`、`Metrics`、`StrictNestSidecarCollector`／`strict_nest_sidecar_enabled`／`summarize_strict_nest_certificates`。同样是两种构建下都没用到。
- parser/segment.rs:2000 的无用比较 `st.end_index >= 0 &&`（usize 恒成立）。

## 4. 新增 allow：98 处，全部带 `reason = "#1608…"`

没有新增 crate 级或文件级 allow，也没有 cap-lints 或 RUSTFLAGS。没有用 `let _ =` 丢弃 Result。已有的文件级 allow 没动。

A. 无条件 `#[allow(...)]`，82 处：
- 规则 ③「与 lib 共用的模块，本 bin 只用其中一部分」，25 处：p940、p944、p946 对每个 `#[path="../X.rs"] mod` 声明各加一处。
- 规则 ④ 探针记录字段，20 处：p102、p109、p112、p113、p122、p124、p940、strict_nest_check 的诊断字段 19 处，加上 p106 `Trade.exit_bar`（trades.jsonl 反序列化 schema 字段）。
- 规则 ④ 探针、诊断、待接线 API，34 处：
  - #639 盘背短差观测通道（pan_div_short.rs 的 5 项，生产驱动源归 #575 后续票）；
  - strict_nest_check 近失诊断（4 项）；
  - ledger.rs 入场诊断插桩字段（4 个）；
  - wverify_run 的 `BucketStat.ucb/cv`（2 个）；
  - s_content `LeafIter::next/next_body`（2 个）；
  - 其余各 1 处：sizing `clamp_position`、rec_engine `reading_b_diverge`、rec_engine `sink`、t_engine `cost_basis`、orchestrator `buysellpoints`、fugue_v3 `dbg_sink_ok`、open_ledger `LiveOpenView`、open_ledger `shortdiff_carriers`、admission `old_channel`、admission `frozen_runs`、element `position`、element `iter`、held `held_leg_tree_index`、service `apply_allocation`、s_content `parse_canonical_u64`、p117 `side_of_dir`、econ_positive/tests `sum_rho`。
- 疑点留证，3 处：signal.rs 的 `unreachable_patterns`，以及 divergence.rs 的 `dead_code` 和 `unnameable_test_items`。详见疑点 1、2。

B. `#[cfg_attr(条件, allow(...))]`，16 处（allow 只在不用该项的那一档生效）：
- 规则 ②，`not(feature="backtest_bin")`，8 处：nest `new/from_parts`、pipeline `historical_bound_third_cert/segment`、signal `otherwise_domain_sidecar_begin/take`、element `overlay`、compose `StepTrace` 结构体。
- `not(test)`，只有测试读取的字段，5 处：FillOutput 的 3 个字段、admission `NestGateObs`、runner `otherwise_domain_sidecar`。
- `test`，1 处：orchestrator `merged_to_raw`，唯一消费方 recursive_t 是 `cfg(not(test))`。
- `not(test)` 的 `unused_mut`，1 处：pipeline.rs:850，`p` 只在 test 构建的 CASCADE_FULLCLEAR 分支里被改写。
- `not(target_os="macos")` 的 `unused_variables`，1 处：finance_tests.rs 的 `manifest_path`，只在 macOS 分支里读取。

C. 门控而非 allow（不计入上面的 98 处）：
- 新增 `#[cfg(test)]` 67 处。其中模块级 5 处：backtest/mod.rs 的 `l3_fullwindow`、`l3_pi_probe`、`l3_pi_falsify`、`l3_delta_r_alpha`、`open_ledger`。
- `#[cfg(any(test, feature="backtest_bin"))]` 3 处：strategy `shadow` 模块、coverage 的 `StepTrace` 重导出、`TwStepCtx` 拆行后保留的原门控。
- `#[cfg(target_os="macos")]` 1 处：content_store.rs 的 `SQLITE_SOURCE_ID`，唯一消费点在 macOS 分支。

D. 其它小改动：
- `_` 前缀：signal `_src`；profile `_fractals`×2、`_fractals_all`；wverify_run `_c`；recursive_tower `_c`×2；ancok_tests 和 step_tests `_reg`；element_tests `_t_i`；p117 `_b_verdict`；finance.rs `_stage_token`（见疑点 3）。
- 去掉多余 mut 9 处。
- test_support.rs:239 返回类型改为 `ElementView<'_>`。
- rec_engine.rs:3495 测试名 τ→tau（全仓无引用）。

两个 #1394 编译负例块原样保留：content_store.rs 的 `issue1394_capture_callback_refusal` 和 audit_store.rs 的 `issue1394_closed_owner_refusal`。ed337f26f3 已有的 try_update、钩子 expect、check-cfg 声明都没有动。

## 5. 疑点（只记录，未修）

1. **signal.rs `type1_funnel_dx`（#[cfg(test)] 测试漏斗探针，原 1588 行）第二个 match 臂永远匹配不到。** 它本应是 `(Some(Direction::Up), Direction::Up) => c.zg < end.price`，1592bf05ca（#988）把它误改成了和前一臂重复的模式。结果是测试漏斗里向上的 broke 恒为 false。按票面规矩没有改行为，只加了 `allow(unreachable_patterns)` 并在 reason 里写明。
2. **divergence.rs 的 `confirm_divergence_forcel_semantics`（#990 ForceL 专项锁）从来没运行过。** 它写在外层测试 `confirm_divergence_three_gauges` 的函数体里，rustc 报 "cannot test inner items"，测试框架收集不到它。另外它的注释写 `L(b)=-8`，与同一函数里的数值推导（-12）对不上。处理方式：在外层函数上加 `allow(unnameable_test_items)`（这个 lint 只能挂在外层才生效），内层函数加 `allow(dead_code)`。建议另开票把它移到 `mod tests` 顶层，并订正注释。
3. **economic_session/finance.rs 的 `persist_with_job` 收到 stage_token 但从不使用。** 调用方（finance.rs:1543 一带）传入的是 `Some(&token)`，函数体完全没读它。本票只改成 `_stage_token`。需要 owner 确认是故意不用还是漏了接线。
4. **pan_div_short.rs（#639）的文档自称「生产代码位置之一（非测试专用）」，但实测只有测试在调用。** 为了尊重文档里的裁定 B，用的是规则 ④ allow，没有改成 cfg(test)；signal.rs 里的重导出已改为 cfg(test)。open_ledger 模块情况类似：文档写的是供 runner 使用的已验证结构，但全仓只有它自己的测试在用，本票按规则 ① 在模块级加了 cfg(test)，并对 `LiveOpenView`／`shortdiff_carriers` 加了 ④。以后 runner 接线时要撤掉这个模块门。
5. **CI 的 fmt job（`cargo fmt --all -- --check`）会因为不归我的文件挂掉。** `src/bin/s_session_v3/prepared.rs:1707` 的格式差异来自基线提交 ed337f26f3 加的 `.expect("安装故障观察 update_hook")`。这个文件不归我，没有改，需要主控或该文件的负责工蜂执行 cargo fmt。
6. 关于 l3_* 模块门：l3_fullwindow、l3_pi_probe、l3_pi_falsify、l3_delta_r_alpha 的文档都写着「继承 backtest cfg(test) 门控」，但 backtest 后来改为 `cfg(any(test, feature="backtest_bin"))`，导致这些测试专用模块被编进 backtest_bin。本票在模块级补回了 `#[cfg(test)]`，与文档一致。
7. 关于 allow 与可达性的关系：`allow(dead_code)` 会让该项成为存活根，它调用的项也随之不再报死。例如 strict_nest_check 的 `NearMiss`／`closest_pair_misses` 让 `pair_gap_bars` 和 `failed_conditions` 保持存活，sizing 的 `clamp_position` 让 `position_bounds` 保持存活。以后删除这些 ④ 项时，要连同它们的被调用项一起复查。

DONE
