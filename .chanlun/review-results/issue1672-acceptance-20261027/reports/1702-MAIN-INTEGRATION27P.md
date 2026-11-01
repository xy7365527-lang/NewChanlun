# PR #1702 已接入 main 的 Reserve 修复

PR https://github.com/xy7365527-lang/NewChanlun/pull/1702 已推送正常 merge 提交 `e7a9668ad1924f1e8cc65896610aed6c65b09259`。两个父对象是原已审候选 `0932d3dbcdb18d138ce1e67ed05f971d87e75e3b` 和当前 main `cf72a9f91050a4b6af7ac4b6e0c2b9f0364a2584`。没有冲突，也没有手工修改业务代码。

本单元负责集成并交精确候选，未合 main。新候选仍须异家族增量独审及必要 CI。远端 Linux 靶向检查已通过，290 passed、0 failed、18 ignored，完整退出码为0。

## 从真实基线接续

开工于 `2026-10-26T23:25:14Z`。工位 `/Volumes/AgentStorage/issue1672-fin2-hfs-20261025/worktree` 的分支为 `codex/issue-1672-fin2-hfs`，HEAD 为0932，工作区干净。`lsof +d` 仅列本任务shell和lsof自身，进程查询没有其他本票执行者。

第一次 PR API 仍返回旧base `499841b59265c5395ccdd40b92684b5864802f4f`。`git fetch origin main` 得到 cf72，随后 GitHub 回读也确认 cf72。因此集成以实际当前main为准，没有把旧API快照当作main未更新。

main 相对旧基线带入四份已落地交付，即 PR #1727、#1733、#1697、#1743，共34个文件。名单在 `1702-integration27p-evidence/main-files.txt`，提交记录在 `main-commits.txt`。没有再改这些已合成果。

执行 `git merge --no-commit --no-ff origin/main` 自动成功。`mod.rs` 自动保留新 main 的 `phase_trace`、`quantity_calibration`、`quantity_calibration_tests`，以及原候选的 macOS FIN/2 测试注册。`remerge-diff.txt` 没有人工解冲突区间。

## 原 FIN/2 增量完整保留

相对 cf72，新候选仍只改原有五个文件，共953行新增、22行删除。原五文件中除 `mod.rs` 外，四文件对0932逐字无差异，`git diff --exit-code` 返回0。

- `.github/workflows/ci-macos.yml` 保留原16行 FIN/2 执行与原件归档步骤。
- `rust/src/economic_session/tb03c_allocation_tests.rs` 保留原测试helper可见性调整。
- `rust/src/economic_session/tb03c_fin2_tests.rs` 保留原807行正负例和存储观测。
- `s_session/tests/drive_tb03c_fin2_hfs.sh` 保留原97行驱动。
- `rust/src/economic_session/mod.rs` 相对main仅多3行 `#[cfg(all(test, target_os = "macos"))] mod tb03c_fin2_tests;`。

普通 `git push origin HEAD:codex/issue-1672-fin2-hfs` 返回0，没有rebase或force。候选head在GitHub回读一致，state OPEN、base main、MERGEABLE。主仓本地main分叉告警来自已有hook，本任务只提交和推送自己的分支，没有变动主仓main。

## 原证据复用的依据

完整消费 `1672-RUNTIME-ACCEPTANCE27.md`、`1672-INTEGRATED-INDEPENDENT27.md`、`1672-FINAL-CI27G.md` 和 `1743-INTEGRATION-INDEPENDENT27K.md`。

原FIN/2业务原件来自9e777的run40389540213，原独审已证明到0932仅rustfmt。该run的FIN/2两测试、四域恢复、21原信封重送、32 MiB四卷观测和回收均通过。0932的常规run40390534105及DevSkim40390534300也已通过。该run独立1716 job的旧期限失败由本次纳入的1743修复承接，不把旧run写成整run成功。

新main确实改了共用依赖，不能只凭测试文件未变就声称生产路径都未变。本轮核对了依赖差异，并把共用经济域测试作为新增靶向检查。

1. `tb03c_allocation_tests.rs:34` 冻结 `book_revision=tb03c-book/1`。`tb03c_fin2_tests.rs:151-159` 复用该manifest，只把通信协议改为 `economic-session/2`，没有改成 `tb03c-book/2` 或 `/3`。通信协议与书账版本不可混同。
2. `tb03c_funding.rs:756-759` 对非Tb03cV2/Tb03cV3直接返回None。新数量定标不会给本FIN/2原件增加签署条件。`tb03c_book.rs:796-803` 在没有funding时仍采用Tb03cV1。
3. RAW六报的前三报为开局，后三报为netted_fill，没有per_leg_report。#1697新增逐腿归属成功路径不把本夹具改为逐腿分配。既有netted路径的共享函数修改仍由经济域回归检查覆盖。
4. #1743去重与计时改动原样来自main，其独审和真实S正负例证据复用。不是本任务另写的备用判据，也没有扩大3000 ms/debug或FIN/2原30000 ms。`content_store.rs` 的同值证据去重仍须接受独审的集成判断，不将代码文字视为性能新读数。

本次不重新运行完整FIN/2 HFS+回放。四TestOnly卷32MiB、8倍页帽、cleanup公式、资源、恢复和重送合同没有改写。新候选没有引入另一套FIN/2驱动或跳过断言。

## 检查与执行记录

- `git diff --cached --check` 和最终 `git diff --check origin/main HEAD` 返回0。
- `bash -n s_session/tests/drive_tb03c_fin2_hfs.sh` 返回0。
- 原四文件逐字差异检查返回0，证据为 `reused-files-check.txt`。
- 没有在macOS运行cargo或rustc。Rust检查由nc-remote在Windows主机WSL执行。
- 第一次远端命令成功执行fmt，但测试命令漏了 `--features s_session`，因此实际0测试。原日志 `/tmp/nc-remote/20261027-072637-e7a9668ad1.log` 保留，不能作为经济域测试通过证据。
- 已按 `lib.rs:2915` 的feature门修正为 `RUSTFLAGS="-D warnings" cargo test --locked --features s_session --lib economic_session:: -- --test-threads=1`。实际执行308测试，290 passed、0 failed、18 ignored；cargo与nc-remote退出0。日志 `/tmp/nc-remote/20261027-072837-e7a9668ad1.log`，本报告证据目录另存 `economic-session-linux.log`。

push已自动触发精确head的常规CI [40419849850](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40419849850) 和DevSkim [40419849810](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40419849810)。本任务没有手动dispatch、重跑或取消CI。后续由主控收终态，不长期持有CI watcher。

## 交给非作者的增量范围

请按e7a966与两个父对象核自动合并，以及原五文件保留关系。重点核书账版本与通信版本分离、引入main依赖对FIN/2的影响、既有macOS证据复用范围及远端经济域回归。新main已审部分不重开同题评审。本轮没有新设计、手写实现或新注释；architect与model-the-domain不触发。no-comments仅需核本次集成没有新增作者注释，不修改已合main代码。

`Prove It Works`使本次结论绑定实际merge对象、远端head、逐文件diff和真实测试退出，不用mergeable或0测试代替业务检查。`throughput checkpoint: n/a, scoped branch integration`。Babysit采用background，仅交CI句柄。独审、main门和下游消费仍由主控负责。

## 最终交付与剩余门

远端命令已正常结束，自有本地exec与WSL测试进程不再运行。日志中的原始命令、source SHA、编译模式及 `EXIT=0` 均保存。首次0测试日志单独保留，不混入290项通过统计。旧1743日志为298通过，本次少8项的原因是过滤器只选economic_session，而旧命令另选session_transport和session_protocol。逐名比对308个economic_session测试无删除、无新增、无状态变化，遗漏的8项均在上述两个未选模块，详见 `test-name-scope.json`。串行测试用1972.49秒，整次远端命令2019秒；这是单次执行时长，不作性能对比或门值签署。

PR最后回读为 OPEN、MERGEABLE，head仍为 `e7a9668ad1924f1e8cc65896610aed6c65b09259`，base为 `cf72a9f91050a4b6af7ac4b6e0c2b9f0364a2584`。DevSkim run40419849810为 completed/success。常规CI run40419849850为 queued/未终态。

仍在途的常规CI项目为 rust-test-s-session-extras (native) (in_progress)、rust-test-s-session (shard 0) (in_progress)、rust-test-s-session-extras (owned) (queued)、rust-test-s-session (shard 1) (in_progress)。当前失败或超时项目为 无。这些是交付快照，主控按原run续接，不重跑已有FIN/2。

下一步由主控安排Opus对e7a966集成增量独审，收精确CI终态，再按现行main授权推进。未合入main、未关闭1672或其他票、未删除工位或分支。
