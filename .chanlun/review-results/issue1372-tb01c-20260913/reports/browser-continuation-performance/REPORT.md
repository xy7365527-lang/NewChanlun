# #1372 浏览器续接耗时诊断

结论：**本次30秒失败主要集中在等待Q响应首字节，暂不需要修改生产Client。** 已完成响应的“请求写完→首字节”累计28,395.864毫秒，占目标命令的94.648%。HTTP跨度外只约999.137毫秒，占3.330%，其中还含helper尾部保存、哈希和系统调度，不能全算成JS CPU。没有产品改动，没有新增160输入运行，没有改变页31、Watch4、30秒、8MiB响应或64MiB命令界限。

## 实测来源及可分辨范围

读取r3全部256份原始HTTP元数据及命令结果。每份元数据的时间来自同一helper进程的单调时钟；两份原事件流用于轨迹背景。四份实际生产/采集器源码从r3快照复制后逐SHA校验，见 [SOURCE-BINDING.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/browser-continuation-performance/SOURCE-BINDING.json)。本工位没有运行新的Q/HTTP服务、没有重新处理权威数据库，也没有用模拟响应替代实际测量。

| 目标Watch分段 | 累计毫秒 | 能说明什么 |
|---|---:|---|
| 建连、保存/写出请求 | 102.147 | 包括本地记录与loopback建连，不是纯网络 |
| 请求写完→首字节 | 28,395.864 | 含Q排队、抓取、核验、编码、内核及客户端调度；不是纯Q CPU计时 |
| 首字节→EOF | 38.755 | 含服务写出、传输、同步保存响应和事件循环调度 |
| EOF→helper结束时间戳 | 57.378 | 含HTTP解析、正文保存、Response构造与关闭文件 |
| 最后无首字节请求等待 | 408.118 | 到取消时为止的删失等待，不当作完整响应延时 |
| HTTP跨度外 | 999.137 | Client严格校验、helper尾部哈希/保存、调用间调度及命令首尾 |
| 命令合计 | 30,001.399 | 原期限触发，未完成完整提交 |

时间计算：首字节等待=`first_response_monotonic_ms - request_written_monotonic_ms`；接收段=`response_eof_monotonic_ms - first_response_monotonic_ms`；每HTTP跨度=`finished_monotonic_ms - started_monotonic_ms`；跨度外=`命令elapsed_ms - 所有HTTP跨度之和`。helper结束时间戳早于尾部response哈希和metadata落盘，所以这部分会落入跨度外，不能将其冒称纯Client时间。

目标命令的相邻请求间隔中位数6.029毫秒，最大120.838毫秒；其合计986.476毫秒。最大间隔包括整cut收尾和同cut完整比较，不能用单页间隔替代整个客户端工作量。完整逐请求数据在 [TIMING.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/browser-continuation-performance/TIMING.json)。

## 固定cut请求随运行变慢

| 请求固定cut | 已完整页 | 首字节等待平均毫秒 | 响应validated_capture_digest不同值数 |
|---|---:|---:|---:|
| 96 | 28 | 192.560 | 21 |
| 97 | 28 | 308.850 | 28 |
| 98 | 28 | 399.244 | 28 |
| 99 | 6 | 464.698 | 6 |

这些是请求固定cut的分组，不把cut编号冒充当时Q捕获head。后续页面仍取同一固定cut，但响应的已验证捕获摘要不断变化；这是可实见的变化版读取背景。原件本身无法将首字节等待继续精确拆成Q捕获/核验/编码；须接Qowner的服务端剖析。分组原数在 [GROUPS.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/browser-continuation-performance/GROUPS.json)。

相对而言，在S故障暂停附近完成的准备Watch90→94，总2,971.341毫秒，其中首字节等待1,673.429毫秒，HTTP跨度外1,107.554毫秒；准备Watch94→95总664.253毫秒，跨度外319.008毫秒。客户端工作在健康静止窗口中比例较大，但绝对量仍是亚秒至约1秒；目标失败的新增几十秒主要落在首字节之前。

原目标共需113个Snapshot页（96/97/98各28页，99有896行需29页）及1个Watch。实际完成90个Snapshot页，在cut99第7页取消。去掉所有已观测的跨度外时间只是约1秒的极端上界，而且该做法本身并不合法；不能据此证明JS去重足以补齐余下23页。

## Client实现检查与优化取舍

冻结Client `collect` 第230–271行每页执行严格JSON/公共头/hash、token身份和checksum、完整metadata一致性、记录键与顺序；全部页齐后核六族计数、全投影摘要与正式HTML状态校验。`watch`第340–363行按每个完整Delta依次应用并取得该cut的全部分页，再比完整六族及发布语义，最后一次原子提交。没有发现可以删除某个cut完整对照的依据。

存在局部重复工作：`equal(base, metadata)`每页重新规范化已经固定的base；Watch外层与applyBatch内层都调用validateDelta；每行有重复的排序tuple计算与规范化/复制。这些不等于可删除校验。若后续仍需要JS优化，最窄候选是缓存**本次collect内部不可变base的规范文本**，每个新页仍完整规范化并逐字比较；它只去掉重复编码同一base，不弱化任何页身份或字段检查。本轮没有测出该局部占比，也没有实施它，不能报告性能收益。

同cut分页下一请求依赖上页实际返回且验证后的token，不能伪造offset跳页。不同cut的完整collect理论上可用至多两个并发工位预取，但仍须共享原30秒/64MiB预算、每份完整校验、失败取消并等待其他请求、按原Delta次序对拍且只在全部成功后提交。它也会与Q的两个查询工位和缓存发生争用；现有串行原件不能证明并发获益，更不能把并发模型当实际通过证据。Qowner正在针对首字节之前的重复编码实施小修复，本工位建议先验证该改动，暂不增加客户端并发与额外内存/取消状态复杂度。

## 待最终候选冻结的运行预案

[final-run-plans](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/browser-continuation-performance/final-run-plans)内三个模板均为 `PREPARED_NOT_EXECUTABLE_AWAITS_FINAL_CANDIDATE`，root_head为空：

- `ac3.json`：新目录trajectory-revision-final-ac3，原r1/r3 cut95准备、原160/500ms/S95故障，明确无Q故障；主控预留端口18880。
- `r5-a.json`、`r5-b.json`：分目录trajectory-r5-a/b，原完整输入驱动与原多lane消费者，明确Q109故障；主控预留端口18875/18876。两轮同最终候选完整495条对照；独立AC3只允许前494条子域对照。
- `run_frozen_plan.py`只有在主控已固定完整HEAD且模板状态改为FROZEN_BEFORE_EXECUTION后才可运行。语法检查通过；对准备模板的实际执行在任何端口/文件/进程动作之前明确拒绝，退出码1。未创建三个未来runtime目录，未占用端口；启动时必须重新bind与路径预检。

执行顺序固定为最终AC3→R5-a→R5-b，不同机并发跑三臂。等待主控Q源码冻结与独评完成通知；本报告不启动新负载、不作最终产品裁判。r1/r2/r3失败原件保持原状。
