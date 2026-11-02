# #1737 测试同步修复交付

## 当前结果

PR：https://github.com/xy7365527-lang/NewChanlun/pull/1738
候选：a053ca2322ac9ab8d27d98e376f9c569024046bb。
基线：origin/main b0bab458f04d2afea3282475c56e6134236c8a52。
工位：/Volumes/AgentStorage/issue1737-deadline-sync-20261026。
分支：codex/1737-deadline-sync-20261026。

修复已提交并普通推送，PR为OPEN、非草稿，已attach_artifact。Rust靶向检查通过，macOS读数仍运行，非作者独审待主控消费。不合main、不关票。

## 根因与修复

旧测试用sleep20ms推断I/O已销毁receiver；墙钟已过10ms期限不建立线程完成同步。历史同名失败和来源见MACOS-DEADLINE-TRIAGE26AI.md。

候选只改rust/src/session_transport.rs测试段第725行起。真实Unix客户端设置两秒测试等待上限，读到EOF后才断言原deadline已到及reply sender断开。原I/O错误路径先离开reply receiver作用域，再销毁stream；客户端EOF使该断言有实际生命周期依据。两秒不是生产回包期限，生产逻辑零修改。

删掉固定sleep、两行注释及自造local_reset断言。local_reset未调用生产代码，不能证明actor不重置deadline。没有新增生产字段、函数或TestOnly hook。

另一文件为本票roster，登记独立工位。仓内实现差异仅上述测试，无其他业务改动。

## 可复跑实验与原日志

证据目录：/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1737-deadline26ai/。

受控channel实验源码controlled-channel.rs。真实receiver线程将reply sender和deadline送入队列后停在release channel；主线程等原deadline到达，receiver仍活，旧try_send断开断言必败。同步臂先确认此时send可以成功，释放receiver，等待完成信号后确认sender断开。

实验在nc-remote的WSL执行，不是macOS编译。它验证旧假设的错误，不冒充完整生产I/O受控调度复现。实际Unix测试另外执行。原同面macOS失败已由历史CI保留。

20261026-152857-a053ca2322.log原输出：
```text
old assertion failed: deadline expired while receiver is held alive
REPRO_OLD_EXIT=101
CONTROLLED_SYNC_PASS
running 1 test
test session_transport::tests::queued_unix_job_keeps_ingress_response_deadline_in_actor ... ok
test result: ok. 1 passed; 0 failed
```

同次cargo fmt --all -- --check通过。首次经济测试用了错误的finance_tests模块全名，实际0 tests，不计通过；随后已更正。

20261026-153154-a053ca2322.log更正后的原输出：
```text
test economic_session::finance::tests::trusted_ingress_deadline_is_not_restarted_by_economic_job ... ok
test result: ok. 1 passed; 0 failed
test session_transport::tests::queued_unix_job_keeps_ingress_response_deadline_in_actor ... ok
test session_transport::tests::lf_without_peer_eof_still_expires_without_accepting_a_frame ... ok
test result: ok. 2 passed; 0 failed
EXIT=0 SECONDS=44
```

以上均精确a053ca候选。两次日志里的同一Unix测试不加总为不同测试。原deadline锁验证operation_deadline保留传入Instant且到期拒绝；它不是完整/2服务无新增持久回执的端到端证据，不据此替代#1716或整图真实消费。

## macOS与独审

已读取ci-macos.yml，无输入参数，workflow_dispatch可直接触发。执行gh workflow run ci-macos.yml --ref codex/1737-deadline-sync-20261026，未触发全CI手动矩阵。

run：https://github.com/xy7365527-lang/NewChanlun/actions/runs/40286381749。
job：121450922238，macos-session。
本机实读2026-10-26T07:33:24Z附近，run与job均in_progress，head与候选相同。PR push另触发既有普通CI，未重复启动。

非作者Opus独审由主控安排。当前不自批PASS。main批准门保留。

## 方法与后续

读取Poteto全文及Principles、Bug fix、Opening a PR、how、Model the Domain、Prove It Works、Test Behavior Not Implementation、no-comments、technical-writing、unslop。how按窄实施叶不再派孙代理。生产函数边界未改，不触发architect。deslop不可用，提交前直接核diff。

数据形状沿既有TimedJob加Unix客户端EOF，不新建状态机。Test Behavior使同步取客户端可观测EOF，删掉只断言自造Instant的无效检查。Prove It Works使0-test误筛选单列并更正，不把CI在跑记通过。

Bug fix步骤1采用历史macOS原失败、WSL实际Unix测试及受控channel实验。步骤2根因由调度反例确认。步骤3只改测试。步骤4远端同Unix测试通过，macOS仍待。步骤5未额外提交永久失败测试；远端需编译的集成路径采用外部反例留证，避免把纯通道语义演示塞进生产测试。步骤6已开PR。

throughput checkpoint为单测试同步修复，无扇出。实现与验证顺序为读取原件、独立工位、最小修复、远端实验和靶向测试、PR、macOS必要读数与主控独审。

后续由当前owner取macOS终态并补本报告；主控消费独审和精确CI，再依用户main批准推进。新增生产期限政策不在本票。

## 主控接管

主控收到报告及精确head后已接管现有macOS run40286381749和普通CI终态消费，并安排异家族独审。本实施owner按指令结束，不再占用槽位等待。仅停止自己的gh run watch本地观察进程，退出130；未取消GitHub CI。

最后实查2026-10-26T07:42:05Z，macOS job121450922238仍in_progress，已执行到economic_session/session_transport库测试。这个状态保留为进行中，不写成功。候选、分支与PR不变，工作区干净。
