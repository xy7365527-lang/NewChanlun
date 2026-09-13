# #1372 P4 独立审查：旧候选固定结论

对象：WR 提交 `f23d6016c29584d6f6d6e8281b362cc8b5f769ce`，根已转入 `8bbaaffd42`。冻结 binary SHA `bc1ad32b6343b3f325e8080a80bfcad768f6c15110fd67db97e36b59b262afba`。本工位未实现 Rust P4；原生 Codex 独立只读审查。知识图未索引此工作树，按允许的不足回退读取指定源码。

**发现 1 项 P2：响应期限过后可能再次入队。** `rust/src/bin/s_session_v2/mod.rs:1423–1435` 每轮先 `try_send`，仅在返回 Full 时核期限。Full 后等待跨过原期限、且槽位已经释放时，下轮先成功入队，期限判断不再发生。最小修复是每次尝试入队之前核同一个 `response_deadline`；不取消已经入 actor 的权威工作，不改 DeliveryUnknown 的原名分。

`QUEUE-COUNTEREXAMPLE.json` 固定原循环全文摘录的 SHA、源行号、编译器调用产物和 stdout/stderr。独立原循环在“Full 后期限已过、槽位已释放”的确定性下轮入口状态中继续接纳，复现时入口已超期 10013µs；健康期限对照可入队。该证据是原控制循环的摘录反例，不冒充冻结 S 二进制的实际线程调度测量。根已确认此窄缺陷；新根源码的修复另出复验，不改这份旧结论。

缓存和 Arc 范围没有发现新的实质绕过：

- `s_structure_session.rs:758–797` 每次从当前事务读取所有可达 BLOB 和 byte_len；先检查新鲜长度，ID 与原字节逐字节相同才复用解码 Arc；失配回内容寻址和严格 JSON。Arc 只通过不可变引用供控制和结构核验，不存在从可变 Value 重写既有缓存原件的路径。
- `:804–807` 控制门先执行；`:834–884` 新鲜核必要 meta、全独立索引发布代、全 raw（含 pending）、profile 绑定和全部 Delta 11 列；缓存没有绕开这些步骤。
- `cache.rs:93–116` 要求 session/catalog/profile 绑定与旧 Delta 全列前缀相同。`audit.rs:189–252` 还核旧 raw 完整前缀、三族独立内容与出生代，以及对象在旧 cut 的完整投影；只有全部成功才移动出生/撤回游标并复用 active/seen/首版观察。新代出生/撤回仍经新增代 batch/Delta 关联核；失配不会留下部分前移的游标。末代目录和根元组仍在 `s_structure_session.rs:1000–1028` 新鲜对齐后才保存 root memo。
- `cache.rs:68–91,118–152` 新缓存源字节越限就清空，未改严格全核路径。此度量按已声明的源字节记账，不能援引为 allocator/RSS 上限；临时全表捕获仍随存量增长。
- `mod.rs:1258–1261,1313–1317` 保留 handle 与最终查询门；accept/Begin/批次持久/Commit/recover 的当前事务健康门和 epoch/Begin 所有权栅栏未被本 diff 移除。没有按 generation/mtime/data_version 信任旧库的捷径。
- `mod.rs:1372–1398` 仍要求单个 LF 帧、请求 EOF、完整帧字节上限、同一 read deadline 及解码后复核。新 response deadline 只在 read_frame 成功后创建；write deadline 仍独立。

独立运行原件：`run-20260913T085339.743854Z/`。创建 6 输入/6 代的新库（4 对象、12 见证、16 关系、1 观察），分别从健康副本启动本工位独有 S，热身后以独立 SQLite 连接提交反例。

11 项小域结果符合预期：6 类坏库（合法格式 raw 价格改值、同 BLOB 错 byte_len、独立对象改身份、旧见证 slot 改值、控制 attempt 序号缺口、当前 meta.index_frontier 改值）都拒绝 Ready 和后续 ingest，SQL 权威内容不再改变；1 类旧 Delta JSON 仅加空白而改变字节的健康对照通过；缓存预算 0/1 的健康对照通过；延迟第二帧及缺 EOF 超过 read deadline 均拒绝，未因较长 response deadline 获准。11 个本工位 S 子进程全部已停止，未启动或控制根的 S/Q。

本轮逐项核对冻结 SOURCE.json 的 **425 份源码文件 SHA**，前后不变；完整清单原样保存到运行目录，四份审查源码全 SHA 及结果索引见 `REVIEW-OLD-P4.json`。冻结二进制与 manifest 摘要均吻合。实施者的 64 项测试和 160 输入性能报告只是已查证的既有收据，本工位没有重跑或冒称独立新执行。

范围限制：6 代反例不能覆盖任意坏库；没有重新运行完整 64 项、160 输入、Q/GUI、两轮 495 或长期性能验收。小域没发现 memo/Arc 绕过不等于完整 C 通过。本工位未改 WR/root 源码、refs、GitHub 或任何旧运行原件。
