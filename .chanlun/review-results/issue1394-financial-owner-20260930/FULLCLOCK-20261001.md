# #1394 同源 S/Q 与完整时钟前缀验收增补

本文件为 open #1394 工作草稿。源码仍为 `ed1f184`，Rust tree 为 `5a2acd1e72c3846be38cc29689a82aeb57c68dc6`，本轮没有修改生产代码、输入价格、领域判断、预算数值、main 或 tracker。

完整 9011 原源已冻结，六列精确整数无需缩放或 round，raw_text 的 volume 1.0 保留；首 16 条 envelope 与两份旧冻结件逐字一致。完整时钟 790,213 字节从初始化起固定，前 16 条只是该全输入的消费前缀。旧时钟前缀的 native 根不能拿来要求新完整时钟根相等。

正常可写单 bar 来源下，正式 S3/Q3 联合运行通过。初版观察器人为 chmod 的有限证据保持；最终新 fresh 移除 chmod，DB/WAL/SHM 保持自然 0644，Q 的只读由正式 READ_ONLY/query_only 与 fd 证据成立。写前 generation0、写后 generation1、原 Committed、非空快照和 Watch、旧 cut 查询通过独立事实复核。

同完整时钟、同七字段 finite probe 资源下，dev 构建首轮 A/B 只收回 9/8 条，下一条 30 秒 DeliveryUnknown，原收据确为 AcceptedPending，失败、Begin 和停机原件全部保留。它们不是未发生，也不是同代业务输出不确定。

当前同源码的正式 release 构建已重新冻结并实跑，A/B 各 16 Committed。完整 schema、24 表 48,403 行及 BLOB 严格一致，所有回复、原身份收据、时钟和 native publication 相同。整次输送最大分别 10.536/10.500 秒，七项资源与完整时钟、源、顺序都未变。新二进制 SHA 为 `b4c81c9468be1d1911eecbd48deb1be9e9a2f3a0d082fd8f7474cf9e30efc7f5`。构建实际 opt-level3、非测试程序，源码与 Cargo.lock 未变。

generation16 正式 Q 在原 Q21 限额下通过：完整 Snapshot 16 页 472 行，独立投影根与 native 来源绑定；content[0,7) 的 63 个局部 proof 节点、范围引用及数组根校验通过。AsKnown0 空代与保留窗口 13–16 对应的真实 Gap[1,12] 通过。首观察器错误比较引用与描述对象的断言，以及合法状态目录续接前检均原样保留，缺项只由必要新 Q 上下文补验，没有伪造结果或提高额度。

只读 Q 与 SQL 观察器可修改 owned SHM 的协调 mtime，字节与 SHA 未变。不能因此宣称全部物理 metadata 零变化，也不能归因 Q 单方；完整业务表语义和原 source 三文件保持不变。所有实际子进程已 wait/reap，自有 socket 清理、HTTP 端口重绑记录齐全。

本轮独立复核覆盖单 bar joint 132 件、dev 失败 244 件、release 846 件和 Q16 265 件原件。采样 RSS/库/WAL 最大值只是观察下界；内部 handle、Hash、audit、build、store、SQLCOMMIT 独占耗时未测。固定每请求 clock decode/hash/clone 的静态计数不证明它是全部增长根因，也不以 build profile 对比外推 9011 工期。

完整 9011 双跑、旧新全字段 crosswalk、全部逻辑比较与资源峰值、完整 Client 正文闭包/原子安装/实际 allocation、Q 与 E/B 全负载联合验收、其它 TB 和六 Destination 仍未完成。产品前端 #1460 保持独立。客户端存储计费范围的选择仍待答，不能以缺计量填零。主线与真实外效批准门保持。

逐文件摘要和 raw 位置见同名 JSON 与 `fullclock-inputs/`；旧签署源及前 92+24 inputs 原样保留，没有把大体量 build/DB/raw 库冒称已入仓。
