# PR1702 main 集成增量独审 27Q

## 结论：静态 PASS，动态门未闭

对象为 [PR1702](https://github.com/xy7365527-lang/NewChanlun/pull/1702) 的 `e7a9668ad1924f1e8cc65896610aed6c65b09259`。本审确认自动合并完整保留原 FIN/2 合同，原 FIN/2 macOS 业务证据可复用于 e7a966。静态阻断项为 0。

结论待两项动态终态后才完整：远端 Linux `economic_session::` 308 测试，以及常规 CI 40419849850。两项均成功前，本报告不构成合入依据。审查时钟 `2026-10-26T23:48:14Z` 至 `23:51:06Z`（`date -u`）。审查者为 Opus，作者为 GPT6.1-sol，属异家族。

## 合并对象

- 本地 HEAD 与 `origin/codex/issue-1672-fin2-hfs` 均为 e7a966。fetch 后 `origin/main` 仍为 `cf72a9f91050a4b6af7ac4b6e0c2b9f0364a2584`。
- 两个父对象为 `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` 与 cf72，merge-base 为 `499841b59265c5395ccdd40b92684b5864802f4f`。
- `git show --remerge-diff` 输出为空，说明没有人工解冲突，也没有改写业务代码。
- 相对 cf72 仍是原 5 文件，953 增 22 删。ci-macos.yml、tb03c_allocation_tests.rs、tb03c_fin2_tests.rs、drive_tb03c_fin2_hfs.sh 对 0932 的 `git diff --quiet` 均返回 0。
- mod.rs 相对 main 只多 `#[cfg(all(test, target_os = "macos"))] mod tb03c_fin2_tests;`。相对 0932 只多 main 的 `phase_trace`、`quantity_calibration` 与 `cfg(test) quantity_calibration_tests` 声明。

## main 共享依赖对 FIN/2 的真实路径

FIN/2 的输入事实有四条。书账为 `tb03c-book/1`，见 tb03c_allocation_tests.rs:34。manifest 无 funding。通信协议为 `economic-session/2`，见 tb03c_fin2_tests.rs:157。运行面为 ImportResponsibility 加 Capture、Resolve、Apply，没有 Reserve、prepared_proofs 或 q_rebase。逐项核对结果如下。

1. **书账与通信版本分离成立。** book.rs 只新增 `/3` 分派。`/1` 的 `rules`、`apply_in` 分支与错误文本以外的行为都不变。tb03c_book.rs 的 `open_book` 在无 funding 时取 `Rules::Tb03cV1.revision()`，值仍为 `tb03c-book/1`，与旧 `v2=false` 分支等价。
2. **数量定标对本夹具不可达。** tb03c_funding.rs:756-759 的 `pending_reason` 在 `/1` 直接返回 None，0932 版在 `/1` 同样返回 None。`Contract::of` 在 `/1` 且无 funding 时返回 None。apply_with 的 q_rebase 新分支、transfer 开局分支和 `split` 中的定标重基，都要求 funding 存在或事件含 q_rebase，本夹具均不满足。tb03c_e.rs `resolve_image` 新增的定标等待槽拒绝只对 `/3` 产生的槽生效。
3. **#1697 per_leg 对 netted 路径等价。** apply_projected 中 `per_leg=false` 时，`report` 取 `netted_fill`，`income` 照旧调用，derivation 仍为 `E_resolved_netted_allocation`，`manifest_hash` 取值相同。tb03c_per_leg.rs 的改动只在 `per_leg_report` 解码与解析内。原六报为 3 开局加 3 netted_fill，不经过它。
4. **#1743 去重在路径上，但对 FIN/2 判定不变。** store.rs:1076-1083 对 SCHEMA_V2 域的每个提交图像调用 `verify_image_with_budget`，所以 content_store 的按值去重确实在 FIN/2 审计和恢复路径上。去重只删 canonical_json 相同的证据项，校验结果只取决于证据值和同一内容表，因此接受或拒绝不变，读预算只会减少。FIN/2 没有 prepared_proofs，嵌套重复也就不会出现。finance.rs 的 `verify_structure_once` 只在 Reserve 的 reduce 中调用，FIN/2 不可达。
5. **phase_trace 不影响持久化。** 测试构建中只写线程内存，非测试构建为空操作。它不读写期限，也不做 I/O。

FIN/2 文件直接用到的跨模块符号包括 `book::{apply_in,fact_key,hash,integer}`、`store::{Config,audit,init,latest,open,recover,row,rows}`、`content_store::{allocated,carrier_capacity}`、`tb03c_allocation::attach_resolution` 和 `tb03c_book_tests::Space`。这些符号在 0932 与 e7a966 之间均无签名改动，grep 退出码为 1。

## 证据复用范围

可复用的是 run 40389540213 在 9e777 上的 FIN/2 业务合同结论：两个测试通过、21 个原信封重送逐值相同、四域恢复、清理与回收，以及资金、期限 30000 ms、8 倍页帽、32 MiB 四卷和重送合同。依据有三条。FIN/2 自身 4 文件逐字未变。mod.rs 只有声明合并。main 引入的依赖在上述输入下行为等价或不可达。

不随复用迁移的是具体存储读数，包括 page_count、分配字节和 cleanup 余量。它们仍只绑定原 run。推断上，去重只减少只读校验，不应改变持久字节，但本审没有新读数可以证明这一点。

## 剩余动态门与最小补查

- **远端 Linux 308 测试。** 只读回读 `/home/hanju/nc-remote/logs/20261027-072837-e7a9668ad1.log`。`running 308 tests`，截至 `23:51:06Z` 已有 230 个 ok，无 FAILED 或 panicked，cargo 进程仍在运行。需等 owner 交出 cargo 与外层 EXIT 0。这项能证明新 main 依赖下 tb03c 与经济域回归，但 FIN/2 是 macOS-only，这次 Linux 运行不会编译它。
- **常规 CI 40419849850。** 本次查询时为 queued/in_progress，shard 0、1、economic extras 在途，已完成作业全部 success。DevSkim 40419849810 已经 success。
- **FIN/2 在 e7a966 上的 macOS 编译。** 该 head 没有 ci-macos run，PR 下 macos-session 为 skipped，所以合入前没有任何检查编译 tb03c_fin2_tests.rs。上文签名核对把风险降到很低。这里不要求重跑全量 FIN/2。合入后 main 的 macos-session 会编译并执行 FIN/2 步骤，主控应把这个 main run 记为第一次绑定新 head 的动态证据，并按结果决定下一步。如果主控要求合入前就消除风险，最小补查是一次只编译 macOS 测试的运行，不需要完整 HFS+ 回放。

## 方法

只读执行 git、gh 和 grep，并经 SSH 只读查看 WSL 日志，没有在 macOS 运行 cargo 或 rustc。没有修改代码、分支、票面、store，也没有干预 owner 的测试。Prove It Works 要求结论绑定实际 merge 对象、逐文件退出码与调用点，不以 mergeable 或测试文件未变代替判断。Fix Root Causes 要求逐项追到调用点，确认去重确实位于 FIN/2 审计路径上，而不是凭文件名判断不相关。
