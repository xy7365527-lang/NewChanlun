# #1602 / PR #1681 最新候选独立复审

日期：2026-10-25。候选：`14de447c8361f1da7cb2157503100fdc60ab2387`。

## 结论

代码与兼容性复审 PASS：前次唯一 HIGH（旧库缺 policy_records）已闭合，本次没有新增阻断发现。**尚不能给出整体验收 PASS 或直接合入**：同 head CI 的 S-session shard 0 仍在运行，coverage 尚未出现最终结果，PR 仍 Draft；合入 main 仍由主控核具体授权。

## 审查范围与连续性

复用 `/Volumes/AgentStorage/issue1602-receipt-provenance-20261025/FINAL-REVIEW.md` 对 `f8dec1a54f54544b57957f16ffce2b57dbbbe45f` 的完整审查；本次独立阅读全文尾部差异（4 文件，287 新增、3 删除）及生产开门调用链、测试源与原始远端日志。不是把旧 BLOCK/PASS 直接套到新 head。

已核 PR head、本地候选 HEAD 均为上述 SHA，工位 `/Volumes/AgentStorage/rebase-pr1602-20261024` 无未提交变更。`git diff --check f8dec1a54f54544b57957f16ffce2b57dbbbe45f 14de447c8361f1da7cb2157503100fdc60ab2387` 通过。知识图未收录该入口，改读精确 commit 源码。

## HIGH 闭合依据

- `rust/src/bin/s_structure_session.rs:328-380`：共用 `open_db` 在返回连接前执行迁移；表不存在或已含列时不迁移；缺列时 `BEGIN IMMEDIATE` 后重核再补列，事务提交。
- 历史标记固定为 `legacy/unknown`，迁移函数不接收 ThetaConfig；既有 generation 不会被当前版本回填。新库与迁移库默认列定义对齐，Python 严格 schema 同步；默认值本身不代表有效政策版本。
- 旧规则只读分支在迁移前返回，未扩大为可写。v2 `handle_with_policy_and_config` 实际使用共同开门入口；v3 生产 `open_production_with_request_and_mode` 先构造正式配置，随后共同 context 经 `AuditConnection::open`（audit_owner.rs:129-130）进入 `open_db`，再启动审计和 pending 恢复。迁移不是只在测试包装器执行。
- 新发布的生产 INSERT 未改动，仍显式写实际生成的记录戳；旧库补列不替换新生成的配置来源。
- v2 新测试（mod.rs:3765）先真实发布 generation 1，再 DROP 新列形成有数据旧表；覆盖旧 receipt、幂等重投、无输入 advance、recover、第二版发布，逐项核原件/身份/frontier 不变，旧值 unknown、新值 risk/2，并对同一迁移库调用 Python 只读严格审计。
- v3 新测试（driver.rs:873）先留下 generation 1 和 generation 2 的 pending Begin，再删列；recover 是旧库首次开门者，重开实际 resume_pending，分别核历史 unknown 和恢复生成版本，并换回配置复开核历史不被覆盖。

## 实际执行证据

复核原始日志 `/tmp/nc-remote/20261025-171955-14de447c83.log`：首行明确绑定本候选 SHA；`RUSTFLAGS=-D warnings cargo check --locked --all-targets --features s_session` 成功；policy_records 4/4（含两条新增旧库测试），另三条恢复/重开测试各 1/1，EXIT=0。测试源直接断言生产共同路径输出和数据库原件，不是自比空壳。本次未重复有效远端测试，未运行本机 cargo/rustc。

## 同 head CI 与剩余门

CI：[40180393627](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40180393627)，返回 headSha 与候选一致。复审时主工作流 in_progress：test、rust-check、rust-test-default、backtest-bin、S-session shard 1/2/3、extras、s-observer、fixture-drift、sandcastle-ts 均 success；shard 0 in_progress；macos-session skipped。DevSkim [40180393607](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40180393607) success。没有把 skipped 或未结束写成通过，也没有为本返修新增 macOS 验收门。

下一步：主控等本 head CI 终态，失败则按真实失败返原负责人；通过后可携本次代码 PASS 进入具体 main 合入批准门。PR 正文仍写旧 cc8d/07e 候选及旧 CI 状态，应由负责人更新到当前候选与本报告。报告当前为外盘工作草稿；关票前由负责人按交付纪律入 git，并核 main 落地、主线 CI、工位处置和评审面一致。本次未修改源码、认领票、修改 GitHub、合入或关票。
