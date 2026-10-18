# #1394 独立审阅02（正式BLOCK）

固定提交dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0，相对b8c5；真实Sandcastle run 8c777767-2569-4716-bac0-6f57724ae7b6，external session01a14d57-1490-7681-ba78-5676fdc01409，04:54:51Z退出，入口/worker均0。审阅执行完成，内容结论BLOCK。

独审完成，结论 BLOCK：dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0 相对 b8c5 的修复尚未全部封口。F1/F2 修复及原 P0 保护保留；F3–F6 仍有尾差，captured_cut 新增协议与资源问题，共确认 9 项。执行完成不代表审查通过或产品验收。

## 正式发现

- [HIGH] s_session/tests/issue1394_acceptance/captured_cut.py:236、241：SessionClosure 首游标错误地固定为 None。已核 b46 和 C1 99e819 的 financial_reader.rs:745，真实协议要求 after=""，null 会报“after须为文本”。内存探针仅将首请求及回复的 after 改为合法空字符串并更新捕获清单，即被导出器拒绝；原 null 夹具却通过。影响：合法真实捕获无法导出。应对齐真实游标协议，同步修正 test_capture_bridge.py:99 和 CONNECTION-CONTRACT.md:43，补合法空串通过、null 拒绝锁。
- [HIGH] s_session/tests/issue1394_acceptance/financial_oracle.py:166：B→E 时间先后仍只相信 image 内 local_commit_receipt.commit_ns，未绑定 B 持久提交行和冻结操作时间。探针保持独立 E 原件及其 pin 不变，E Reserve=10，将 B 持久行和操作 commit_ns 均设为9，内嵌 receipt 仍声称11，audit 依然接受全部6个中间态。影响：真实提交早于授信仍可通过核账。应核对 B 原提交时间、操作时间与本地回执时间一致，再与独立 E Reserve 时间比较。
- [HIGH] s_session/tests/issue1394_acceptance/journal.py:160、83及scheduler.py:454：成功 receipt 的撤尾保护结束后仍有解锁操作；解锁跨过截止时，调度器拒绝，但持久日记保留成功。内存模拟真实 Journal 方法，让 LOCK_UN 将时钟推进至51、阶段截止50，结果为 FAILED、返回0条收据，日记却为 consume+receipt、next_step=1、pending=null。影响：重读日记会将本次失败阶段认作完成。应让最终接受状态、解锁异常/迟到处理和日记恢复语义一致，不能只撤去返回列表中的收据。
- [MEDIUM] s_session/tests/issue1394_acceptance/supervisor.py:197：循环开头仍先 tick，再接收登记消息，F5 竞态未完全消除。探针在合法首 ACK 已排队、尚未超过心跳/工作截止时推进0.25秒，monitor 在 receive 调用0次时就报未登记后代；owners为空，child仍活。已登记父进程的后代消息同样复现。应建立明确的待登记握手顺序，使合法排队登记先被核验，同时保持陌生PID和伪消息拒绝。
- [MEDIUM] s_session/tests/issue1394_acceptance/supervisor.py:158、163：首个 owner 的 wait 逾期后，后续存活 owner 会在发送清理信号前直接被截止检查跳过。双owner探针中102收到TERM并reap后越过cleanup截止，101既未收到信号也未wait，仍存活。整体INCOMPLETE标记正确，但“仍清理其余owner”未实现。应继续对已核身份的其余owner执行必要回收，保留原截止和逾期事实，不续期、不改判成功。
- [HIGH] s_session/tests/issue1394_acceptance/captured_cut.py:199、201：资源门未核实际捕获与session、Q部分可直接验证的限制。分别将session.max_requests设为1、session.max_request_bytes设为1字节、Q.max_page_size设为1，包含17次财务请求、page_size=2的捕获仍全部返回UNAUTHENTICATED_FACTS。影响：明显违反冻结资源的捕获可被接受。应逐route核session请求数/帧长及Q页大小等可由原件确定的限制；这不需要运行产品或测量RSS。
- [MEDIUM] s_session/tests/issue1394_acceptance/captured_cut.py:196：所有Q与financial捕获无条件累计到reader.job_limits，合同没有明确冻结这种跨对象共享Job关系。探针含4次Q请求和17次财务请求，将financial reader的max_total_requests冻结为17，其余帽充足，仍因总计21次而拒绝。应显式绑定捕获到所属资源对象/Job；若确需共同Job，应单独冻结共同对象，不能隐式借用reader资源。
- [MEDIUM] s_session/tests/issue1394_acceptance/captured_cut.py:195：将Voice authority原件的64MiB单件/128MiB集合帽直接套到全部捕获原件，新增了未获授权的捕获总帽。即使session/reader/Q及Job允许，单件65MiB或累计超过128MiB的合法捕获也必然被此分支拒绝；该结论由固定比较式直接确定，未分配大数据探针。应按捕获所属对象的既有冻结帽核验，保留Voice模块自身64/128MiB限制。
- [MEDIUM] s_session/tests/issue1394_acceptance/captured_cut.py:47：取所有阶段截止的最小值作为当前导出的截止，会把已完成阶段的截止带入后续阶段。探针仅增加一个已完成、截止已过的前置阶段，当前export/work/cleanup/seal及预算仍有效，导出即被拒绝。应绑定当前阶段，取当前stage、whole和所属Job等适用截止的最早值，不把历史阶段截止变成整次永久截止。

## 实际验证

- 前后HEAD均为dfdeda34886d4a8f0a3bfa7bc8e3f3930aab28f0，工作区状态为空。按b8c5→dfdeda的38文件差异审阅；工具代码git diff --check退出0。
- 已读取AGENTS、CLAUDE、wayfinder、CODEX-CHILD、ask-matt及指定报告、返修账、接线合同、来源角色和生产依赖交接。正式REVIEW-01及七条原探针的SHA分别匹配3c4ff1b9…和c51419e3…；未把作者自检作为独审结论。
- 本轮DELIVERY-FILES 46项、MANAGER-DELIVERY-FILES 49项长度与SHA全部匹配；24个Python文件最终指纹匹配并内存compile成功。历史b8c5清单47/53项另从Git原字节核对，全部匹配。19份协议来源及7个pin匹配。
- 核验91项普通/-O原输出和最终资源尾改10项普通/-O原输出；TEST-REUSE列出的22份源码/依赖指纹不变，支持其余82方法复用。没有重跑91、82、全套或旧ps/socket三项，也未声称最终92项重新执行。旧host 123项成绩仅保留为b8c5历史证据。
- captured_cut.py最终SHA为017aeda55be7bf3ebd26525da7adba0d452b3fd6d21e6335dd1dc0551be2aed5；test_capture_bridge.py为43a1bf1303f36a84082a54881d83d994ad2d5bc5e2fabd38e8159f85ad048dd9。资源尾改新增1个方法、最终该模块10个方法，未混算为10个新增方法。
- F1固定原字节重导出、F2逐owner原请求有序对拍已核；30场景/510步骤/24故障/5负域/10compare约束保留。41个driver及27个scheduler原方法无删除；driver/control/closure及P0回归源与b8c5字节相同。
- 针对新疑点实际执行只读内存探针：合法空游标拒绝、资源超帽接受、Q/reader计数混用、历史阶段截止误用、排队登记竞态、逾期后遗漏存活owner、B持久时间倒置、Journal解锁迟到留下成功receipt。脚本和原输出保留在工具记录；未写临时文件或checkout。
- 核对cut真实三字段及独立E根链；检查固定token/anchor、六族全集、独立源节点比较和Voice候选保留路径。Voice仍仅输出未认证候选/缺件，execute与BindVoice入口硬拒绝；UNIT_TEST_ONLY映射不是正式G格式，外部expected_sha仍须由根冻结。
- 未执行cargo/rustc、真实S/E/B/Q、Long、真实DB、网络、浏览器或资金动作，未读取凭据、开启工蜂或进行Git写操作。1488→1419→组合C1→统一新SHA及financial_reader组合语义重审要求保留；真实生产缺件未重列为代码bug。P11、正式A/B、恢复、资金、完整历史和隔离仍未验；前端归#1460、K4排除，相关图票及#1279保持开放。
