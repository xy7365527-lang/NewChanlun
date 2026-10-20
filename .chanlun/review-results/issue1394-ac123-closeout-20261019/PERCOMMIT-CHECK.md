# PR #1573 逐 commit 可编译性核查（关票门第 4 子句）

范围：`6badaa6049^1..6badaa6049^2` 中触及 `rust/` 的 220 个非合并提交与全部 7 个合并提交，去重 227 个（合并提交在原始输出里出现两次）。命令：windows-5090 WSL 逐个 `git checkout --detach <c>` 后跑 `cargo check -q --features s_session --lib --bins --tests`，原始输出见同目录 PERCOMMIT-CHECK.txt。只改非 Rust 文件的 53 个提交不在 cargo 范围。

结果（去重）：215 ok、12 FAIL。HEAD `9b00359052` 与全部 7 个合并提交可编译。失败提交全部是 2026-10-03 至 10-11 的历史中间提交，已随 PR 进入 main，无法在不改写 main 历史的前提下修正：

- `1a3a38dfc0` 2026-10-03 test(s-content): retain fixed owned refs through closed lookup result：error[E0432]: unresolved import `super::ref_owner`
- `658951031b` 2026-10-03 test(s-content): retain owned-ref verifier renderer and JsonInner rej：error[E0432]: unresolved import `super::ref_owner`
- `80430337e4` 2026-10-03 test(s-content): seal owned lookup inputs to actual closed capture ow：error[E0432]: unresolved import `super::ref_owner`
- `d3df2b92ea` 2026-10-03 refactor(s): 共用 TB-02-B 借用 JSON 输入核 (#1394)：error[E0432]: unresolved import `super::ref_owner`
- `05d42b006d` 2026-10-03 test(s-session): retain metadata text and roots in shared owned manif：error[E0432]: unresolved import `super::ref_owner`
- `06d081f72d` 2026-10-03 test(s-session): inspect metadata error through public detail accesso：error[E0432]: unresolved import `super::ref_owner`
- `78ff5b94d1` 2026-10-03 test(s-content): adapt normal array enumeration to borrowed ref kerne：error[E0432]: unresolved import `super::ref_owner`
- `f2bdb01d3d` 2026-10-03 test(s-session): adapt borrowed metadata to explicit normal manifest ：error[E0432]: unresolved import `super::ref_owner`
- `0605ffc799` 2026-10-03 test(s-session): compare owned publication against actual indexed tem：error[E0432]: unresolved import `super::ref_owner`
- `86d093b907` 2026-10-08 refactor(s-session): 复用完整冷审计关系索引 (#1394)：error[E0425]: cannot find function `projection_fixture` in this scope
- `28e58f0c34` 2026-10-08 test(s-session): 保留初始包含待定发布拒绝的正常复现 (#1394)：error[E0425]: cannot find function `projection_fixture` in this scope
- `88bf08b7c3` 2026-10-11 perf(s): lightweight startup timing instrumentation：error[E0425]: cannot find value `_t_load` in this scope

成因：`1a3a38dfc0` 新增 `rust/src/s_content/ref_owner.rs` 但未声明 `mod ref_owner;`，到 `905ffa559a` 才补；`86d093b907`/`28e58f0c34` 调用的测试夹具 `projection_fixture`（`92ad0a90b9` 引入）在这两个提交的作用域里不可见；`88bf08b7c3` 引用了未定义的计时变量 `_t_load`，由 `0d8a601723` 移除。
