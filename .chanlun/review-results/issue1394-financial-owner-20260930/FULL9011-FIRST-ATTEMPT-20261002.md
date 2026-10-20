# #1394 全 9011 正式首试与只读成本诊断

本文件是 open #1394 的工作草稿，归总图 #1323、SPEC #1340 已批准 C 增补。本轮首次向正式 S3 会话提交完整 9011 目标；真实结果为 `FAIL_OR_PARTIAL`。A 客户端确认 27 条，第 28 条在原 30 秒请求门返回 DeliveryUnknown。B 按“失败 case 停止”未启动，未重投、未删除或恢复 pending；完整 9011、C 和六 Destination 均未完成。#1460 前端独立。

生产 Root HEAD 为 `3e5a5ba31dbe5c59beeee03de227fd8408eaac55`，Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395` 未变。复用已绑定的非 test release `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`，没有新构建或生产改码。独立 runner 私有提交 `a1dd0a82d37915a34ae053d3ac034b402b0f169f` 只证明脚本来源，不能冒称 binary 构建 HEAD。50 项离线检查、固定输入 prepare/gate、两个独立 A 失败阻断 B 的反例及六文件 SHA 已核；它们只放行首试，不证明服务可用。

全 9011 原始 bar、原整数/原 raw_text 中 volume 1.0、每条 envelope LF/payload SHA、完整 clock 和 ordinal 顺序逐条核对。新具名首试显式保留既有 S7 七值和全部 Q21 原值，没有默认、新量纲或预先容量证明门。caller 逐请求/启动 30s、每 case 观察 1h、cleanup 总 30s，以及私有磁盘观察门独立冻结，不作为生产容量或工期保证。只运行 Controller init→serve→Ready→逐条原生消息；未启动 Q、HTTP、E/B 或场所动作。

前 27 条原请求/reply wire、原生因果身份和 Hash、持久 receipt/clock、进度摘要及 raw 来源全部列均经独立核对。第 28 条 1,004 B 完成 sendall、回复原 wire 0 B，整次 exchange 为 30,007,694,666 ns。停机快照中 `message-28=AcceptedPending`、attempt_count=1、first_published_generation=NULL，只存在 accept/begin，无 commit、publication28 或 Delta28。最终库 raw/message 各 28 条，publication/Delta 各 27 条，clock 83 条、native 节点 104,465 个；advance_state 保持 begun28，next_attempt_ordinal=29、clock frontier=185。客户端确认与持久接纳是不同事实，不能合并成“28 条成功”。

A 总 case 观测 304.220243792s，六项清理均通过。init PID91598 exit0、S PID91605 SIGTERM exit-15，均由原 Popen reap，socket 目录移除。Root 外层 27 原件前后 bytes/SHA/device/inode/mtime/mode/st_blocks 完全相同；原 SQLite 没有打开。runner 的 `final_snapshot=UNMEASURED` 表示完整9011最终校验未过，已落盘 `final.fullDB.json` 的存在和 native 节点完整性另有独立证据，不把该旧状态改写成成功。

| input | 整次 exchange 秒 | 客户端事实 |
|---|---:|---|
| 1 | 0.517005167 | Received/Committed |
| 10 | 3.741713458 | Received/Committed |
| 16 | 9.484797333 | Received/Committed |
| 24 | 22.641386959 | Received/Committed |
| 27 | 28.844123583 | Received/Committed |
| 28 | 30.007694666 | DeliveryUnknown |

这条曲线是实际整次交换，不是内部 Handle/SQLCOMMIT 或 audit 专属时长，也不能预测剩余历史。正式 RSS 2,582 样本最大 222,101,504 B、目标 100ms，均只作采样观测；严格 RSS/heap/IO/内部阶段峰仍 UNMEASURED。

随后在终态 A DB/WAL/SHM 的完整字节相同 owned 副本上，复用冻结 cfg(test) binary `5f9ef56e7cd0ee27dd00094faaf5c73821d6501fe38b4783c238c67d80cb286f`，只启动一次指定 ignored readonly test，内部两次 `state::audit`。没有构建、Driver、checkpoint、recover、commit、pending删除或重投。测试 PID19340 exit0/reaped，原三件身份未变、副本三件 bytes 均不变，owned SHM mtime 变化如实记录。

两次只读 audit 分别为 11.468526542/11.495750583s，均返回 raw28/cuts27，完整返回指纹 `42444317d86a03e066867bdfccfd0bb9f07010a1a2655483fdc377b7a115b8a0` 相同。Root 独立重算指纹及每轮 963 个累计事件的相邻时间、source_read 增量和尾部余量，全部吻合。每轮实际 read_material found 8,792,023 次、unique digest66,953、F3,402,722、reuse hits8,658,383/misses133,640；局部保存峰65,536 entries/3,818,233 B，entries 资格拒16,123。资格拒后仍走原严格 F，不是请求资源拒绝；4MiB/65,536 也不是进程内存界。

首轮相邻事件累计到阶段的主要耗时为 history_object_members 2.013186963s、history_resolve_publication 1.544439915s、source_shapes 1.350489379s、history_metadata_source 1.325270163s、source_observations 1.071735878s。标签与计数保留 observer 的相邻差分定义，不充当独占 wall/CPU 的因果分解。stored clock/parse 仍每次各一次；当前节点 SQL 已 prepare_cached，不能再次声称省重复编译。诊断 RSS 213 样本最大112,345,088B，仅下界。

这证明该 pending28 副本的 audit 能成功，且历史/对象/逐代来源复核有真实重复读量；没有证明 S 整个第28请求在哪阶段超时。该材料不是输入28之前的 cut27快照，不能删除未发布 raw28 模拟另一材料。后续优化须针对已测重复物与完整判据上下文，不删除三次完整审计、不跳来源/shape/clock/索引/计费，不提高30s修绿。C5.8完整旧新crosswalk、Client实际分配及原TB与六Destination继续留待完成。

旧279份 tracked packet文件原样保留。本轮 68 份固定脚本、状态、输入、运行/独审/清理与计数报告在 `full9011-first-attempt-inputs/`；273 个原大型DB/全快照/二进制关联与观察文件留为caller-owned，同名JSON列完整路径/大小/SHA。含硬换行、patch空白或末尾空行的原文字用 `.raw.json content_utf8` 保持字节，不修剪旧失败。整份为一个增补，未合入 main、未推送，也未改写goal事件。
