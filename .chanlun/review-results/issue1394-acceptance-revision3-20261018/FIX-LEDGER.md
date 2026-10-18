# #1394 冻结审面返修03矩阵

固定审查起点 `dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0`。正式独审02仍是该起点的 **BLOCK**；本表仅列作者修复自检，九项均 **待管理者固定新字节后独审**，没有新批准。原报告与完整探针命令/输出逐字保存于 references/REVIEW-02-INDEPENDENT.md、references/REVIEW-02-INDEPENDENT-PROBES.json，SHA分别 `8d90c9c037aeaea13d826cf57c77d62f7d6fb8e04010b64d5f2dbac062baf7f5`、`51c16f90e41be4a66e3eb288b603c4cc14c29a5615858c8a2560b3a06d39642b`。

## 旧已闭合项：保持身份，不纳入九项新计数

| 原身份 / 类别 | 保持的修复与本轮证据 |
|---|---|
| F1 HIGH / 旧已闭合 | Requirements仍按原ce719894…硬pin重导完整量词。test_review01.test_F1_requirements_and_steps_both_removed_fixed_probe：删成一场景、删负域/compare及再删proof均拒绝；原30场景510阶段模拟成功。固定合同、24故障、2normal、3extra、cap80、5负域、10compare无变化。 |
| F2 HIGH / 旧已闭合 | 每owner原请求有序全集与各自epoch保持。test_review01.test_F2_second_owner_original_probe_and_ordered_full_set：第二owner换99/新key/hash、删增重排及错ordinal均拒绝；原完整路径通过。scheduler只新增Journal.accept两行，validate_plan等原逻辑逐字未动。 |

## 本轮正式九项

下表的方法名均在 `s_session/tests/issue1394_acceptance/test_review02.py`；旧回归仍按原名保留在 test_review01。文件路径基于同一工具目录。全部正例仅为UNIT_TEST_ONLY手写原件或fake控制路径。

| 身份 / 源类别 | 固定原例 | 实际修复写集 | 正锁 / 负锁 | 结论 |
|---|---|---|---|---|
| R-F3 HIGH / 旧残余F3 | 独立E Reserve=10且pin不变，B持久行/冻结操作=9、本地回执=11仍过六态 | financial_oracle.py:167：记录各决定原Commit的seq与冻结commit_ns，核原B行=冻结操作=本地回执，再与原E授信时间比较。test_review01及test_financial_oracle成功夹具补齐原缺的行/操作时间；E cut仍三字段 | test_R_F3_fixed_early_B_and_each_of_three_times：11/11/11及10/10/10通过；9/9/11、每处单独9或12、全9拒绝，E原字节/pin保持。test_R_F3_missing_times_refused：三个来源各缺一个时间拒绝。原假E/错根/错会话等回归通过 | 自检通过，待独审 |
| R-F4 HIGH / 旧残余F4 | finish撤尾保护后LOCK_UN推clock到51，stage50；返回FAILED/0，旧Journal却receipt/next1/pendingNone | journal.py:89/166/181与scheduler.py:457：v2日记落candidate；解锁完成后、同deadline检查后才允许当前调用者accept。无解锁后截断。重开全部candidate隔离，不自动恢复或退款。test_journal成功调用显式accept | test_R_F4_unlock_late_exception_reopen_and_no_truncate：迟到、释放前后异常、另一持锁写者追加均拒绝且保原件；重开pending0或损坏拒绝。normal_acceptance_and_reopen_quarantine：正常阶段接受、后续正常消费，重开隔离。candidate_cannot_be_accepted_without_finish_or_twice与fsync_failure_and_late_accept_keep_pending锁缺握手/重放/fsync/序列化。旧late fsync、late证据/解析回归及跨进程争consume锁通过 | 自检通过，待独审 |
| R-F5 MEDIUM / 旧残余F5 | 首合法ACK或已登记父owner的后代REGISTER已排队，clock前进0.25；receive=0就拒陌生后代 | supervisor.py:198：先按原work/heartbeat窗口接收具名登记，核消息身份后采样；返回后再核截止。未放宽未知PID、合同、父owner、nonce | test_R_F5_already_queued_ACK_and_descendant：两条排队路均2次receive、登记并回收。queued_forgery_unknown_and_expired_not_adopted：伪ACK、陌生PID、过期/超时接收拒绝。旧receive内出生及恶意后代锁仍过 | 自检通过，待独审 |
| R-F6 MEDIUM / 旧残余F6 | 双owner102 TERM/wait后越cleanup，101无signal/wait且活着 | supervisor.py:147：一次共享TERM截止，先遍历信号再遍历wait；后续已过期只沿原绝对deadline做非阻塞wait/KILL/reap，逐次核信号目标身份，逾期事实保持INCOMPLETE | test_R_F6_late_any_position_live_dead_kill_all_touched：101/102任一位置越界×存活/已退/需KILL/无reap，所有已登记owner确实wait，存活者全TERM、需KILL者全KILL。expired_entry_still_nonblocking_cleans_known_only：入回收即超期仍只清具名owner、不收养999。原按时回收/重用PID锁过 | 自检通过，待独审 |
| N1 HIGH / 新引入captured_cut | 真实首after=""被拒，伪null被接受；b46/C1均要求文本 | captured_cut.py:288、test_capture_bridge.py、新CONNECTION-CONTRACT勘误；不改producer | test_N1_empty_cursor_legal_null_and_broken_order_refused：空串全页通过；请求null、回复null、错next、后页重新空串拒绝。完整旧token/anchor/六族/source锁仍过 | 自检通过，待独审 |
| N2 HIGH / 新引入captured_cut | session.max_requests=1、max_request_bytes=1、Q.max_page_size=1仍接受17财务请求/page2 | captured_cut.py:107/275：每route逐对象帧帽、逐session请求、Q页长、显式Job wire/请求及适用节点/记录帽。未知运行维度明列unmeasured_resources | test_N2_fixed_session_request_frame_and_Q_page_caps：session请求/请求帧/回复帧/节点、reader双帧、Q双帧/页长，逐项原件精确边界通过、边界减一和1拒绝。原缺维/retained/旧读取帽锁不变 | 自检通过，待独审 |
| N3 MEDIUM / 新引入captured_cut | 4Q+17财务被误计reader=21，reader请求帽17误拒 | captured_cut.py:22/95/107：合同升/2，object_ids、具名jobs owner/limits/started_at/deadline及每capture binding均外部pin冻结；reader帽须一致；不隐式共享、不默认发帽 | test_N3_separate_Q_and_reader_jobs_exact_boundaries：4与17各自精确请求/wire通过，各自减一拒绝。binding_old_ambiguity_and_cross_object_rejected：旧/1、缺绑定、错Job/session、偷共用及错reader帽拒绝。node_and_record_caps_stay_with_their_object：节点/节点字节/两对象记录边界成对测试 | 自检通过，待独审 |
| N4 MEDIUM / 新引入captured_cut | Voice64/128MiB误套捕获原件，新增未授权帽 | captured_cut.py:107/249删除借帽，仅按所属对象、Job与原ReadBudget；voice_bridge.py字节保持 | test_N4_size_observation_not_Voice_64_128_caps：小len观察对象模拟65MiB×4，原对象帽66MiB、Job260MiB时通过，再一次超过所属Job拒绝；没有大分配。原Voice64/128负锁通过 | 自检通过，待独审 |
| N5 MEDIUM / 新引入captured_cut | 已完成历史stage截止过期、当前export有效仍被min(all stages)拒 | captured_cut.py:22/47：stage具名绑定，读取预算不得晚于当前stage、whole及所属Job最早截止；Job不得延长原elapsed | test_N5_historical_stage_expired_current_valid：历史过期而当前有效通过；当前/whole/Q Job/reader Job过期、elapsed帽缩短、缺stage均拒绝；未把所有stage改为同一未来值 | 自检通过，待独审 |

## R-F4 接受与系统边界

同步flock/fsync不是可中断事务，不能保证调用一到deadline就停止，也不能通过返回列表清空来撤回已经释放锁的写入。本轮采用用户允许的保守隔离：持久candidate只说明有一份待确认原件；`finish`解锁返回、`accept`再核同一截止才赋予本次活调用者继续的资格，不发新的持久成功声明。重新打开时即使原调用者曾成功，candidate也不自动升级为成功，原链逐行保留，投影状态停在首个隔离消费。正常成功的Scheduler收据须由管理者另验；本工具不凭该日记自动恢复。这是明确的能力代价，不伪称崩溃后仍能无歧义重建接受时点。

持锁写入失败/迟到仍只撤自己的candidate尾段，consume不退款；解锁前后异常不再截断。并发追加原件保留，损坏则失败封闭。控制日记新版本为issue1394-control-journal/2，旧/1不自动迁移。

## R-F6 回收边界

TERM宽限只计算一次，KILL沿原cleanup绝对截止；从未按owner加新窗口。Probe.wait对过去的deadline必须非阻塞poll/wait，返回必须是真实reap证明。未知或异常保留errors，晚reap仅记事实，结果INCOMPLETE。这里验证了fake Probe协议与原短命toy；未新增或认证真实OS Probe，也不能把任意阻塞的外部实现伪称可被Python强制中断。

## 实际验证边界

TEST-RESULTS.json记录最终普通/-O各68方法及完整命令，原输出在normal-targeted.txt、optimized-targeted.txt。本轮新增17个方法；不是68个新增，也不把两模式加成136个独立断言。既有91/82/host123未重跑。STATIC-CHECKS.json核原41/27方法无删除，driver/control/closure/Voice/P0等10文件对固定起点字节一致。

execution_authorized=false；产品全NOT_RUN。真实producer、Voice、P11与根准入仍是外部成功义务，不重复列作本轮代码bug。所有九项仍须独审。
