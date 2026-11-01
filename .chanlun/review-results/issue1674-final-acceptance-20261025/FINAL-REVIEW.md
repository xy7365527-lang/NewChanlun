# PR #1684 / issue #1674 最终独立非作者复审

日期：2026-10-25。角色：独立非作者最终复审，不是实施者。

## 结论

| 项目 | 判定 |
|---|---|
| CRITICAL | **0** |
| HIGH | **0** |
| MEDIUM | **0** |
| LOW | **0** |
| 代码 | **PASS** |
| #1674 资金 / 持久恢复合同 | **PASS** |
| 当前 head CI | **PASS（单独判定，见下文）** |

未发现需要返修的代码、测试或合同问题；没有删测试、减少 kill 组合或放宽断言求绿。本结论只覆盖下述精确候选，不授权合入 main，不关闭 #1674，也不替代 #1406 的其它验收。

## 候选与审查边界

- PR：<https://github.com/xy7365527-lang/NewChanlun/pull/1684>。
- 最终候选 / 本地 detached HEAD / 远端 PR head：`dbd2979e377c8f8b67a7c641c24689eb5ff280a0`，三者一致。
- base / 本地 `origin/main` / PR base：`13ed4d45c8ff85175819ca987690328ab2a247a4`，三者一致。
- 旧 head：`ad285c60543468df917958f29b469fe92a059ac6`；旧代码候选为 `055618db67`，旧基线为 `3ff38accdd`。
- 当前代码候选为 `631291dcd88e0e51d5a3f364bc5f5aca1e317162`；其后的 `dbd2979e37` 仅更新验收文档。
- 完整 base→head 差异：6 文件，1084 additions / 27 deletions；新增 `tb03c_v2_kill_tests.rs` 898 行，两份验收文档，另只改测试模块注册和测试助手。
- 只读审查；未在本机运行 cargo、rustc 或 cargo test，未改代码、票据、分支，未派孙代理。运行证据来自原始 Linux/CI 日志与最终 GitHub 状态。

## range-diff 与变更面

执行口径：

```text
git range-diff 3ff38accdd...ad285c60543468df917958f29b469fe92a059ac6 \
               13ed4d45c8ff...dbd2979e377c8f8b67a7c641c24689eb5ff280a0
```

结果：旧 7 个提交对应新前 7 个提交；首提交仅因 #1688 进入基线产生上下文差异，旧提交 2–7 与新提交 2–7 patch 相同。新增三项为：

1. `29d9a18ec9`：E 崩溃后各 B 也由新进程 recover 并升 epoch 2；
2. `631291dcd8`：rustfmt；
3. `dbd2979e37`：记录 rebase 红→绿及最终验证。

旧 head→当前 head 的 v2 测试代码差异只有上述 E/B 恢复轨修复与格式化；未删除测试函数、目标步或既有业务断言。`service.rs`、`store.rs`、`book.rs` 与 `s_session/tests/fixtures/` 在当前 base→head 中零差异。当前 PR 对 `tb03c_kill_tests.rs` 只开放测试助手可见性并复用 `Clock`，未改 BindVoice 目标、矩阵或负控。

## 决定性源码与测试证据

### 1. `tb03c-book/2` 持久边界覆盖完整

- `rust/src/economic_session/tb03c_v2_kill_tests.rs:85-96` 明确构造 `book_revision = tb03c-book/2`，包含 funding-record、每重 policy-record、margin_leverage 与故障计划。
- 同文件 `:884-898` 保留 **13 个目标步**：三 profile 的开局 Capture、开局 Apply、两类 E Pending Capture，以及 5m 的 L2 开局 Apply。
- 同文件 `:870-882` 每个目标仍生成 `before_commit` / `after_commit` 两个测试；`:818-851` 每个组合仍执行 `rep = 0..2` 两次独立轨迹并比较终态和续跑回复。
- 因此每轮仍是 **13 × 2 × 2 = 52 次 SIGKILL**；命名测试仍为 26 个 kill 测试 + 3 个无故障重放测试，另有 1 个 ignored 子进程入口，没有缩水。
- 同文件 `:568-573` 强制目标前后业务快照不同，防止边界判据退化；`:633-655` 分别核提交前回滚、提交后持久、原收据有无；`:724-782` 核旧写者 StaleWriter、新写者重送、提交后原收据及重复重送不新增提交。

### 2. 新进程与真实 SIGKILL 对拍成立

- 同文件 `:238-306` 通过 `current_exe` 和具名 ignored 测试启动真正子进程；`:563-691` 的轨迹是空前缀进程 P → 目标进程 A → 等待故障 marker → kill → 新进程 recover。
- `Child::kill` 后由 `wait()` 取得真实退出状态，`:625-628` 明确断言 Unix signal 为 9；不是模拟返回值或同进程故障注入代替进程死亡。
- 生产故障缝仍紧夹事务边界：`rust/src/economic_session/service.rs:1148-1164` 为 Apply 的 `pause(before_commit)` → `tx.commit()` → `pause(after_commit)`；`store.rs:386-423` 的 pause 仅在配置的 TestOnly 故障计划下停驻。
- `tb03c_v2_kill_tests.rs:484-540` 的无故障参照从空库进行两次新进程完整重放，比较 canonical bytes 和 hash；`:693-710` 对崩溃轨做全表及公开查询对拍；`:791-801` 再跑 E/B audit 和黄金值。
- `golden`（同文件 `:190-226`）核每个域 meta=`tb03c-book/2`、本金、完整 funding 出处和 typed `AwaitingPolicy{funding_revision|q_rebase}`；`Builder::send`（`:369-399`）另核未签 Apply 返回 typed 错误且任何域业务表均不变。

### 3. 104 次 kill 证据未被缩减或冒用

独立读取 `/tmp/nc-remote/20261024-174856-6848feea22.log`：

- 实际计数 **104 条 `V2-KILL`、104 个不同 PID、104 条 signal 9**；
- 命令在同一个 `TB03C_KILL_ROOT=/tmp/nc1674-reentrant-shared` 下连续运行两轮；
- 两轮分别为 **29 passed / 0 failed / 1 ignored，18.79s** 与 **29 / 0 / 1，16.77s**；整段 `EXIT=0 SECONDS=84`。

这 104 次属于旧代码候选的同根双跑证据，本报告不把它冒充为 rebase 后同根双跑。根目录函数在当前差异中未改，仍由 `root()`（`tb03c_v2_kill_tests.rs:322-337`）用进程号与纳秒运行号创建新现场并保留旧证据。

rebase 后另有独立证据：`/tmp/nc-remote/20261025-072201-29d9a18ec9.log` 为当前修复代码的 **29 / 0 / 1，17.00s**；最终 head CI 又执行同一 26+3 个命名测试并全部通过。每一轮自身都保留每组合两次真实 kill，未以旧 104 次为由把当前矩阵改成单次。

### 4. `cfg(test)` 仅限测试构建

- `rust/src/economic_session/mod.rs:44-49` 中，`tb03c_kill_tests` 与新增 `tb03c_v2_kill_tests` 都由 `#[cfg(test)]` 注册；被复用的 funding/book 测试模块也都在 test-only 注册区。
- 当前 PR 没有向生产模块暴露测试助手；`pub(super)` 改动只存在于上述 test-only 模块。
- 最终 head 的 `rust-check`（含 default、backtest_bin、s_session 和 rustfmt）及 `rust-test-default` 均成功，旧非 test 构建 E0432 未复现。

### 5. E/B 时钟因果修复是合法对齐，不是放水

- main 的正式写入门仍在 `rust/src/economic_session/service.rs:1110-1115` 调用 `verify_b_observation_after_e_commit`；判据本体 `store.rs:1449-1460` 仍拒绝 `B.first_known_ns < E.commit_ns`，恢复审计在 `store.rs:1463-1472` 复用同一门。
- `tb03c_v2_kill_tests.rs:732-752` 在 E 被杀后，逐一对每个 B 启动新进程 recover，断言 `Recovered` 与 `writer_epoch = 2`，随后 E/B 续跑均使用 epoch 2；B 被杀仍只恢复该 B。
- 共享 `Clock` 在 `tb03c_kill_tests.rs:374-393` 对 E 崩溃后的后续消息明确要求第二时钟列，没有删除时钟商检查；`compare` 仍先拒绝时刻商不合，再做规范化逐值比较（`:515-529`）。
- 原始红→绿可复核：rebase 未修的 `c7ac35b9c4` 完整 `tb03c_` 为 **150 passed / 10 failed / 3 ignored**，10 个失败均为 E Capture；最小用例修复前失败。`29d9a18ec9` 后最小用例 1/0、全部 `/2` 29/0/1，完整 `tb03c_` 为 **160 passed / 0 failed / 3 ignored，361.57s，EXIT=0**。对应日志为 `/tmp/nc-remote/20261025-070949-c7ac35b9c4.log`、`071810-c7ac35b9c4.log`、`072017-29d9a18ec9.log`、`072201-29d9a18ec9.log`、`072332-29d9a18ec9.log`。

### 6. #1688 的 BindVoice 与时钟锁均保留

- main `13ed4d45c8` 是 #1688 集成提交；已核其两份实施报告及生产时钟门。
- 当前 `tb03c_kill_tests.rs:884-894` 保留 BindVoice 计划，`:1270-1309` 保留步骤 21 的 before/after SIGKILL 矩阵，`:1390-1430` 保留跳过重送导致 Voice 与公开查询缺失的负控，`:1433-1440` 保留错误 allocation 负控。
- 当前 PR 没有改这些 BindVoice 代码段。最终 head extras 原日志明确显示步骤 21 before/after、跳过重送负控和错误 allocation 负控全部 `ok`；不是仅凭 259 总数推断。

### 7. 变异负控与范围限制

- 原提交 `625fab529e` / rebase 对应 `7120e8fee8` 的提交证据记录：把 before pause 移到 commit 后，P0 s00/s01/s02 的 before 侧 3/3 按预期红；把 after pause 移到 commit 前，对应 after 侧 3/3 红。
- 后续差异未改生产 pause 或目标边界。临时变异分支已删除，本次按只读约束没有重造变异体；此处结论是对既有实证的复核，不宣称本审重新执行了变异。
- 测试自述范围仍是本地事务进程故障缝，不扩大声称为正式 socket/S/浏览器故障验收（`tb03c_v2_kill_tests.rs:1-5`、`:810-816`）。

## CI（独立结论）

最终精确 head `dbd2979e377c8f8b67a7c641c24689eb5ff280a0`：

- CI run [40143234633](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40143234633)：`completed / SUCCESS`，更新时间 `2026-10-25T00:54:50Z`。
- `rust-check`、default、backtest-bin、s_session shard 0/1/2/3、coverage、extras、test、s-observer、fixture-drift、sandcastle-ts 全部 SUCCESS；最后完成的 shard 0 为 SUCCESS。
- extras job `121019149893` 原日志：economic_session 等相关库测试 **259 passed / 0 failed / 10 ignored**；其中 26 个 v2 kill 组合、3 个 v2 重放、BindVoice 两阶段及负控均逐项通过。
- DevSkim run [40143234608](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40143234608)：SUCCESS。
- PR 的 `macos-session` 为 SKIPPED，符合 `.github/workflows/ci.yml:416-421` 仅 `workflow_dispatch` 调用的条件，不是失败或悄悄缺测。base main 的 CI [40135530860]、macOS CI [40135530846]、DevSkim [40135530847] 也分别核实为 SUCCESS。
- 最终刷新时 PR 为 OPEN、MERGEABLE/CLEAN，head/base 仍与候选一致。

因此旧报告中的三项 funding 基线红已消失，不再作为当前 PR 的遗留 CI 门。CI PASS 不替代上述源码与合同审查，也不等于已合入 main。

## 剩余交付门

代码、#1674 合同和当前 head CI 均已通过；剩余仅为交付纪律：

1. 合入 main 仍需用户明确批准，本报告不执行、不授权自动合入。
2. 关票时另核 resolution 指向真实 main commit、工位清理、关票声明与最终评审面一致。
3. #1674 保持 OPEN；#1406 其它验收及 #1621 自身未闭合范围不因本 PASS 被提前宣告完成。
