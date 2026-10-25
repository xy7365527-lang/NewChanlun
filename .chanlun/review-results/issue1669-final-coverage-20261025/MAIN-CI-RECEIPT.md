# #1669 主线 CI 与收尾凭据

记录日期：2026-10-26；仅补交文书，不改实现或验收口径。

## 最终独审与实际落地

同目录 `FINAL-REVIEW.md` 原样复制自 `/Volumes/AgentStorage/final-review-wave-20261025/1676/REVIEW.md`，覆盖候选 `c9e8a74a6e2d5a09d6e3dba59edd3626e652315f`。代码／合同 PASS；原文所记 CI 尚未完成、整票合入 BLOCK 属于当时状态，原样保留。

[PR #1676](https://github.com/xy7365527-lang/NewChanlun/pull/1676) 已 squash 为 main `7cacd14135d12e2339fce58cd1c87cd0ee132ee0`。

## 主线 CI 采用决定

原 main [run 40233787543](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40233787543) 为 cancelled，仅 extras job `121290902700` 取消。GitHub annotation 为 `The job has exceeded the maximum execution time of 1h0m0s`；库测试与 Native 384 passed / 0 failed / 9 ignored 已完成，Owned history 最末步编译时达到上限，尚无测试结果。

主控已采纳后继 #1674 main `92dc58eafcb8cd71b51b0a2b5ed8a4406e1ea581` 的 [SUCCESS run 40233866011](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40233866011)，补足未执行的同实现末步。extras `121291139085`：Native 384 / 0 / 9；Owned history 35 passed / 0 failed / 1 ignored，699 filtered，测试耗时 1.76 秒。

末步命令：`cargo test --locked --features s_session,s_allocation_json_testonly --bin s_structure_session v3::history --jobs 1`。

两主线提交有实际祖先关系；`.github/`、Cargo 配置、S bin、s_content 精确差异为空。完整 Rust 差异仅为 #1674 economic_session 测试新增及登记。因此复用的是后继主线同实现行为证据，原 run 仍为 cancelled，不称原 head 全绿。本轮未重跑 Rust、CI 或扩大资源帽。

## 工位与剩余关票门

原 `/Volumes/AgentStorage/issue1406-1669` 已非 force 移除，目录及 worktree 注册均无；本地与远端 `codex/1669-oq2a-short-lock` 保留，待主控处置。此报告不删除分支、不关闭票。

#1669 在本轮核查时仍 OPEN。最终独审及本凭据进入 main、分支处置完成后，主控才能用实际文书提交 SHA 补最终 resolution。本票证据不代替 #1378 的跨方向边界验收或六项 Destination 整图验收。

取证来源：`/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/1669-1674-CLOSEOUT-READINESS26.md` 与 `MAIN-CI-TERMINAL-CLOSEOUT26.md`；前者为后续工位处置状态，覆盖后者较早的工位待清理记录。
