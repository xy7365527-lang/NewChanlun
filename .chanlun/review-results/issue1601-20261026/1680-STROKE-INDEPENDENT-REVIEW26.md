# PR1680：Stroke 名分与资金接口集成独审

结论：**PASS（本次集成代码与合同）**。固定 head `06a095112fb6fbf94f90d55c19e027ad76c63830`；真实 GitHub CI 尚未通过，主控仍须消费终态后判断合入。

审查者：`/root/review1680_stroke26`，非作者，2026-10-26。工位 `/Volumes/AgentStorage/pr1680-consume1723-20261026`。只读代码；未合入、关票、改共享 ledger 或本机编译 Rust。本轮是已审 PR 的集成尾差核验，不重启全 PR 评审。

## Standards

无本次集成新增阻断。实际 `git show --remerge-diff 06a0951` 只出现 `rust/src/recursive_t/stream.rs` 一个冲突；解法保留 #1654 的 Stroke 已否证对照臂注释，同时保留 #1601 的 q_total 注释及三参签名。未恢复旧“研发-生产同源／来源可切换”说明。

实际双亲分别为 `49b71870634dec950fd077b9fe3ac02a3fd20438` 与 `3a6866ad15c9e3f4f51ba26c7dc7b63a749c9107`。逐项读取相对父一的 12 个 Rust 文件差异：均为名分注释调整及测试断言说明文字／相应格式；没有新增生产表达式、条件或资金默认值。自动合并的 ffi、rec_stream、trading/config、nested_fugue、positional、positional_fusion 均保留本来行为。其余 6 份评审／收尾文档与父二完全相同；`git diff HEAD^2 HEAD -- .chanlun/review-results` 为空。差异 whitespace 检查通过，工位干净。

## Spec

无本次集成新增阻断。`stream.rs:239–247` 保留 `new_with_a0(mode, a0_source, q_total)`，构造体仍调用 `TPositionEngine::new(q_total)`；`ffi.rs:243–248` 仍先 `py_funding_record(funding_profile)?`，再传 `funding.q_total_f64()`。测试默认构造仍限定 `#[cfg(test)]` 并传 FIXTURE_Q_TOTAL，没有生产硬编码资金回流。

`types.rs`、`stream.rs`、`ffi.rs` 等仍声明 Stroke 非 a₀ 合法值、仅供已否证对照复现；本次未改变该臂准入行为。资金记录、NT 策略及 funding Python 测试相对父一无差异。复用 `1680-INTEGRATION-REVIEW.md` 中已审功能面，本次只证明新增集成保留两侧意图，不把注释变更宣称为运行时隔离新实现。

## 远端验证原件与射程

目录：`/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1680-integration-validation26/`。
- `20261026-082155-06a095112f.log` 明写完整候选 SHA、nc-remote 工位、命令及 EXIT=0：`cargo check --lib` 成功；`cargo test --lib funding_record:: -- --nocapture` 实跑 6 项通过（legacy_fixture_matches_fixture_constants、active_profiles_carry_signed_q_total_and_version、negative_controls_missing_unsigned_invalid、replay_records_label_names_every_record_and_version、active_records_match_adr_0040、production_sources_hold_no_funding_constants）。
- `20261026-081938-06a095112f.log` 同 SHA：fmt 通过；stream 测试过滤后为 **0 tests**，不计运行通过。已核 lib.rs 的 recursive_t 受 `#[cfg(not(test))]` 控制；上项非测试 cargo check 覆盖该模块的接口编译，六项测试覆盖资金记录，二者射程不同。

## 下一消费者

本轮 gh 回读 PR1680 精确 head 与候选一致，MERGEABLE。CI `40249141867`、DevSkim `40249141833` 尚 QUEUED，macos-session 为 SKIPPED。主控须核这些真实运行的终态及合入授权；若 head 再变化，针对新差异补审。本次代码／合同 PASS 不代替该 CI 门。

Standards：0 阻断；Spec：0 阻断。未留下本次集成返修项。
