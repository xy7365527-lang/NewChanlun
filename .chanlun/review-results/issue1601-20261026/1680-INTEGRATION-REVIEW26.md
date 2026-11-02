# #1601 / PR1680 主线整合尾部独审

代码与合同结论为 **PASS**。审查候选为 `5eb523f6a7b1e8ce8997addeda74470bb15f952a`。新候选 CI 尚未结束，合入仍须等其终态；旧候选 CI 不替代新组合检查。

## 核验范围

本地工位 `/Volumes/AgentStorage/issue1601-integration-20261026` 的 HEAD 与实时 PR head 一致，工作区 clean。merge 双亲实际为已审候选 `107336f3d4ae0e4b2b9b52d3916d62311685829c` 和主线 `7cacd14135d12e2339fce58cd1c87cd0ee132ee0`。复用 `/Volumes/AgentStorage/final-review-wave-20261025/1680/REVIEW.md` 的完整代码合同审查，本轮仅独立核整合差异。

## 决定性证据

1. `git show --remerge-diff 5eb523f6` 仅显示 `policy_records.rs` 一个冲突。人工解决只删除三行冲突标记，两侧新增声明均保留，没有改表达式、删除测试或改默认值。
2. 对 `13ed4d45c8..107336f3` 与 `13ed4d45c8..7cacd14135` 的修改文件集合取交集，仅为 `rust/src/bin/theta_backtest.rs` 和 `rust/src/theta_v0/policy_records.rs`。已逐项读取两份合并后的差异。
3. `theta_backtest.rs` 同时保留 main 新增的 C-19 统计记录处理与 #1601 的生产资金入口。Nautilus 段仍调用 `FundingRecord::production`，失败返回 FAILURE，不启用备用资金。研究统计段的 Result 处理及版本输出未被吞掉。
4. `policy_records.rs` 同时保留 arm-sub-expiry、research-stats、evidence-gate、L3、W-VERIFY 各自记录。main 的 `PolicyRef::label` 只对以 `#` 开头的记录加 `v`，arm-sub-expiry 不满足该分支，其 `arm-sub-expiry/1` 输出及测试合同保持。解析辅助函数的可见性提升不改其判据。
5. `git diff 107336f3 5eb523f6 -- rust/src/funding_record.rs rust/src/funding_record tests/test_nt_funding_records.py rust/src/theta_v0/nautilus rust/src/theta_v0/ffi.rs trading_system/strategy/theta_rec_strategy.py` 为空。资金同源、evidence_only 生产拒绝、非法 nav 拒绝及对应测试没有在整合时变化。
6. 相对第二双亲的差异仍为 54 文件、2335 增483删；`git diff HEAD^2 HEAD --check` 通过。owner 的 18 项 funding Python 测试通过记录可复用，本审未重复运行，也未在 macOS 编译 Rust。

## CI 与接续

实时核验时 CI `40233901558`、DevSkim `40233901592` 已存在且绑定当前候选。Python test 与 DevSkim 运行中，其余 Rust 等任务排队，macos-session 为 SKIPPED；没有据此预判整体通过。
主控等待这两条 run 的最终结论。当前 head 未再变化且 CI 通过后，本尾审不留下代码或合同阻断。若集成时 head 或补丁再变，仅对实际新差异补审。

独立审查者 `/root/review1680_integration`，2026-10-26。采用 Prove It Works 原则，核真实双亲、remerge-diff 和交集文件，不从 owner 自报推断冲突安全。未改代码、票据、分支或共享账本。
