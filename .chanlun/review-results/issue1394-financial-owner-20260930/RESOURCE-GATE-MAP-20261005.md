# #1394 当前资源验收口径

本文件是 open #1394 的工作草稿，记录本轮只读溯源及主控复核。它不修改签署 SPEC、封存原件、生产启用权限或既有测试结果。当前继续已批准的隔离正常验收，前端 #1460 独立。

此前报告中的“全部 heap／所有 allocation 尚未资格”“strict peak 未测”限制了该报告的证明范围，不能自行成为新的正常验收前置。后续逐项核原声明资源门、v3 job/reader 限额和实际测量记录；不把整个 S 服务的全 heap 改造加在这一步之前。这个澄清不等于全部资源门已通过，也不授予 Native 生产启用资格。

| 对象 | 已核实口径 | 依据 |
|---|---|---|
| S writer | 帧字节、读／写／响应期限、队列数量、连接数量和保留审计源字节分别计量 | `s3-entry-inputs/S3-RESOURCE-PREPARATION.md:9–23`；数值须使用各具名域实际冻结输入，不能借该早期小域数值替换当前域 |
| S7 | `audit_cache_source_bytes` 限保留来源字节，不是 RSS 或 allocator Layout 总额 | `s_session/tests/fixtures/tb01c/runtime-plan.json:108–112` 明文；当前新隔离正常域另有 536870912 B 的批准记录 |
| Q capture/cache | 原 capture/cache 容量门及 capture/verify 期限继续适用；实现登记按拥有的保留对象容量计量 | `APPROVED-SOURCE.md:311–313`；`S3-RUNTIME-20260918.md:124,210–212` 明确临时 SQLite、解码和派生图峰值不包含在跨请求保留容量中 |
| job wire | 应用层实际读写字节累计，含错误及重传 | `APPROVED-SOURCE.md:300` |
| job nodes | 唯一根的节点分帧字节及节点数，含本地复用；不是全 Rust heap | `APPROVED-SOURCE.md:301–302` |
| job records/requests/time | 逻辑记录访问、请求次数和同一 job 绝对期限分别累计，不按片重置 | `APPROVED-SOURCE.md:303–305` |
| job staged/store | 明列文件、索引、日志与整体内容存储的实际分配字节，含既存内容；不是全进程 RSS | `APPROVED-SOURCE.md:306–307` |
| reader | 每次 request/reply/stderr 真实字节和启动至回收的单次期限；同一经济操作共用累计 job | `APPROVED-SOURCE.md:315–317` |
| 联合验收测量 | 分别记录新节点、唯一依赖读量、逻辑比较、Hash、审计、构造、落盘、RSS、库/WAL、网络/job 总量；原声明资源门和 v3 限额均须通过 | `APPROVED-SOURCE.md:394`，不能把 RSS 记录义务扩成该条没有给出的全 S heap 数值门 |

上表的 `APPROVED-SOURCE.md`、`S3-RUNTIME-20260918.md` 位于相邻 `issue1394-content-v3-20260916/`；其 §5.1–5.9 的批准状态由该目录 `SPEC-ADDENDUM.md:7–9` 确认，原件内“待采纳”是历史措辞。S7 后批原件为当前工作树 `.scratch/issue1394-source-cache-window150-151-20261004/USER-APPROVAL.json`。本轮读取了这些指定条款，不声称穷尽全部 496 项合同。

执行边界仍为当前批准的正常 TestOnly 域、原启动／请求30秒、同一536870912 B S7。记录观测方法及未测范围；采样最大值不能冒称严格峰值。用户停止的故障／坏数据测试不重跑，既有证据只在其适用范围内复用。完整9011双跑、跨域尚未执行的门及六Destination继续保持未完成，不由本表代销。
