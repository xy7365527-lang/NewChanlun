## 来源与范围

承接 #1588 关闭后遗漏的计时测试债，原交棒为 #1406 评论 https://github.com/xy7365527-lang/NewChanlun/issues/1406#issuecomment-6302451174 。服务 #1323 / SPEC #1340 的真实验收，不重开 #1394，不混入 #1527 schema修复。

## 问题与证据

`session_transport::tests::queued_unix_job_keeps_ingress_response_deadline_in_actor` 以sleep20ms推断I/O线程已销毁reply receiver，缺少线程完成同步，try_send可合法成功。历史main run39448001873与PR1714旧候选run40278590515均在同一断言失败。不是只有当前慢runner才发生。

分诊原件：/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/MACOS-DEADLINE-TRIAGE26AI.md。
已完成tracker含closed及analysis/docs/.chanlun双搜，关键词和结果记录在原件。#1588 CLOSED，未核到专门承接的开放票。

## 已定技术边界

修测试级完成同步，不加长生产期限、不增加sleep碰运气、不删测试。分开验证原deadline传递与回包receiver生命周期，必要的调度seam仅TestOnly。若发现生产合同缺陷，返回具体证据另判范围，不顺带改变/1或/2合同。

## 验收

- [ ] 受控调度复现旧断言错误，修后通过。
- [ ] 回包生命周期检查有实际完成同步及有界失败退出。
- [ ] 原绝对deadline不在actor出队时重置的行为仍有有效测试锁，优先复用。
- [ ] Rust只nc-remote/CI，精确head取得macOS必要读数。
- [ ] 非作者独审、必要CI、用户main批准后按关票门收尾。

实施使用独立worktree，保留在飞任务，不扩大隔离资源或修改生产资金政策。
