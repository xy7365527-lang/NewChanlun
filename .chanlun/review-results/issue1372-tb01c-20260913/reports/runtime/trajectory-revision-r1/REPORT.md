# #1372 修订续接补验 r1：失败原件封存

结论：**有效跨修订 Watch 响应已实际收到；完整客户端续接未完成，状态仍为 NOT_VERIFIED。** Q 在原计划的第109代故障窗口中终止，打断同 cut 分页校验；不得以原始四批响应替代完整安装成功。

候选 HEAD `1a57fb684e5a6aa18e4f962b68ba3d9f9a908c66`；S frozen SHA `5bb7f1dfd57e1d8e6cfc1097360246ae27333776504767df4aba1b9fba9ce71b`。启动前保全 **456** 份实际源码/配置/输入/产物原件（含425份Rust冻结源码），前后SHA全部一致。保持原160输入、500毫秒节拍、op95 afterBegin SIGKILL及writer epoch2恢复、Q最早观测到generation109时SIGKILL及query epoch2恢复、原S/Q资源。

| 实际公开 Client 操作 | 完整结果 | HTTP数量 | 用时 |
|---|---|---:|---:|
| 初始load cut90 | 完整安装cut90，26页 | 27 | 3.050秒 |
| 准备Watch 90→94 | 四批各自同cut完整分页后安装cut94 | 108 | 3.069秒 |
| 准备Watch 94→95 | 一批同cut完整分页后安装cut95 | 29 | 0.669秒 |
| 保持cut95游标，真实head95→99后Watch | 无Gap四批96..99已收到；同cut分页中途截断，未安装 | 23 | 5.173秒 |

冻结输入的首修订为 **op98→cut99**，`evt-0002 / seq2 / revision2`；op96和97是新事件。末次真实Watch请求使用之前完整安装的cut95原游标，响应 `gap=null`，observed_head与next_cursor均为99，四批代际为96、97、98、99。cut99的Delta实际包含 **3个withdrawals、3个replaces、1个raw_history_added**。

客户端随后逐批运行原有严格对拍，在cut96完成21页后，offset651的第22页收到0字节响应，错误为“HTTP头截断或超过界限”；该请求落在既定Q109停止窗口。末次命令 `full_state_installed=false`、`committed_changed=false`，`retained_cursor.after_generation=95`。没有自动重试、没有重建或注入状态，没有把已收到的Delta视为已提交。

原件入口：

- [冻结计划](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/PLAN.json)与[启动命令](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/EXECUTION.json)。
- [有效四批Watch原始响应正文](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/http-consumers/revisions/commands/revision-three-commit-reconnect/http/00001-response.body)。
- [中断分页的完整采集元数据](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/http-consumers/revisions/commands/revision-three-commit-reconnect/http/00023-metadata.json)及同目录0字节response.http原件。
- [末次Client命令结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/http-consumers/revisions/commands/revision-three-commit-reconnect/RESULT.json)与[业务未验证结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/http-consumers/RESULT.json)。
- [全部事实、文件SHA和命令摘要](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/SUPPLEMENT-FACTS.json)；[事件原件精确行号索引](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r1/EVIDENCE-INDEX.json)。

共 **187** 次HTTP交换：186份完整、1份截断；请求和响应原字节共 **21,144,784** 字节。每份请求/响应及已解出的正文附件逐SHA核验，未发现不符。实际helper使用同一正式Client和HTML校验器，没有新放行判据。

原160输入驱动完成495条核心记录，runtime退出码0；consumer进程退出码0只表示它完成记录，业务结果仍明确NOT_VERIFIED。495条与R4-a逐JSON行比较相同，此为本工位实际比较事实，交其他工位独立复核，不自称最终验收。

本轮只创建专属18877端口、socket、数据库及state。结束时本次S PID `82823`、Q PID `82899`按计划保留供主控；登记原字节另存 `owned-process-records-at-completion/`。记录仅证明当时的登记，不代替后续发信号前重新核身份。原A/B/主仓/原S/Q及R4证据未修改。

AC3有效续接仍欠一次完整成功。主控已另定将AC3与AC5的Q故障义务分臂补验：下一臂事先声明不注入Q故障，并保留本轮及R4原故障轨迹；它不能替代原R4双跑。本报告固定r1已发生的事实，不追改成功名分。
