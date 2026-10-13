# #1394 S16 shadow 阶段成本实测

两个独立新域均跑满原 source 的前16条；同源 clock、全部16 envelope SHA逐代一致。两个域均退出0、完成冷审计。与旧S16的完整schema、表列及全部typed行（BLOB原件完整hex）逐字段对拍，未剔除或重命名任何字段：包括原生逻辑根、身份、消息、控制历史、来源与时钟；结果完全相同，SQLite主文件SHA也恰好相同。此为S16耗时诊断。

| 指标 | fresh1 | fresh2 |
|---|---:|---:|
| 16次handle合计秒 | 77.253351 | 77.573306 |
| 第16次整个handle秒 | 13.418855 | 13.466318 |
| 末冷审计秒 | 4.555985 | 4.557989 |
| 接纳前audit秒 | 3.922546 | 3.975270 |
| Begin前audit秒 | 3.966964 | 3.952914 |
| 写准备前audit秒 | 3.977082 | 3.971460 |
| 生成秒 | 0.354081 | 0.355739 |
| 本代来源audit秒 | 0.196231 | 0.197752 |
| 最终SQL COMMIT秒 | 0.037359 | 0.040672 |

第16次handle中三次完整audit的观测时间合计占比为 88.43% / 88.37%。这证明本影子样本的阶段归属：旧记录12.331993958s是整个handle，而不是SQL COMMIT。不能据此删除三个不同事务/状态的审计，也不能宣布原生产因果瓶颈或线性外推9011。

本代producer完整逻辑比较216次，实际新求值48次；unit_computed=2、reused=7、invalidated=0，来自真实执行点与现memo差值。实际比较计数与完整逻辑量分账；不是由存储节点数推导。

Store在实际read/put点记录调用、found/missing/new/exists/errors及body字节；按phase记录请求digest唯一数/重复次数，overlay派生写另记。read唯一数为SQLite与overlay请求的联合去重，不能当磁盘唯一依赖；body读字节是返回body次数累计，不能当磁盘IO。每phase耗时/F包含其嵌套子phase，work计数仅当前phase，不将父子时间/F相加。F是整个受测线程的实际frame调用；state.traced(true)阶段原件另存，writer未传AuditBudget的检查/唯一审计节点数仍未测。

| 采样观测下界（字节） | fresh1 | fresh2 |
|---|---:|---:|
| RSS | 69222400 | 70434816 |
| DB st_blocks×512 | 6025216 | 6025216 |
| WAL st_blocks×512 | 7421952 | 7421952 |
| SHM st_blocks×512 | 32768 | 32768 |

独立sampler请求周期100ms，实际最大间隔 120.521ms / 115.441ms；RSS从启动采到终止，DB/WAL/SHM从旧start公告出现后采样。均为观测下界，漏掉采样间瞬时峰值；RSS含仪表/诊断自身开销，不是严格资源峰值验收。sampler phase标签是最近已结束span，不能当精确阶段归因。

已采局部renderer：history标量buffer、generation小字段buffer、witness哈希流输出字节。全部renderer/materialized字节和Value heap峰值为UNMEASURED；不填0，不编造装载量。

原无仪表对照16次handle合计72.079085789s、末12.331993958s、冷audit4.364045458s、测试76.53s。此次合计差额约 5.174s / 5.494s；它含计量、独立采样与时间/机器负载差异，仅是历史同源观测差，不能分离为纯仪表开销上界。

cfg(test)差异清单（隔离shadow，不整合产品）：
- `rust/src/bin/s_session_v2/content_producer.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v2/content_store.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/commit.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/driver.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/generation.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/history.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/prepared.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。
- `rust/src/bin/s_session_v3/state.rs`：测试专用span/计数；其中commit.rs是已明确允许的实际resolve与SQL COMMIT执行点例外。

8文件剥除cfg(test)项/语句后非test源码（词法、注释/空白归一）相同；另实建release非test单bin/features成功。原有storage损坏拒绝、prepared SQL失败/回滚与live-driver当前节点损坏再审计定向检查通过；保留43条non-test构建warning日志，未扩修。

构建失败原件保留：Python3.14高于原PyO3支持上限，换用已有具名3.12；第二次纯仪表模块私有性错误，通过将cfg(test)计量模块移至现共享content_store修正。没有改公共API/权限/默认/资源/生产语义。未启服务、未跑9011、未改main/push。原bars/旧DB/clock/log SHA前后不变。所有测试/构建PID均已terminal、reap且ps确认不存在，详见PID-CLEANUP.json。

冻结与原件索引：SOURCE-BINDING.json / BINARY-BINDING.json / instrument.patch / instrument-source/ / s16-shadow-test-binary；PHASE-SUMMARY.json；attempt1/fresh{1,2}/phases.jsonl、handles.jsonl、sampler.jsonl、stdout.log、domain/audit-trace-*.json、COMPLETE-LOGICAL.json、LOGICAL-COMPARISON.json。单一最终MANIFEST.json按原件列SHA。
