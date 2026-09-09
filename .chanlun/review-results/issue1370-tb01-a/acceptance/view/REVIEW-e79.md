> 阅读版：仅转换链接；原始独评字节与 SHA 见验收索引。

# TB-01-A 耐久性修复定向复核（e79c74a）

**结论：本轮指定修复及受影响路径通过，0H / 0M / 0L。** DUR-M02“批次先持久、后发布”已有实际双连接可见性证据；DUR-L01 改为按真实取消结果诊断。原 DUR-H01/H02/M01 复核继续通过。该结论只关闭这条耐久定向审阅的发现，不替代 #1370 的其他正式独评、GUI/Python、整票或父图验收。

固定 commit `e79c74a17a800e660b515e1f88fc9a5706826b42`，tree `cfb051d6f5e2b42fb715c340abc095a372309a91`。native SHA-256 `85f247650531d2ec807f56f36e1c379d0ed34b4906d455108129737bc5b0769f`，冻结 Rust 入口 SHA-256 `c376159b6fb0a9542fc00cccd9061cfccd94080fd0b44ce53d0cb124ad7c9841`。根的 408 项构建源指纹已逐项核对，入口同时与该 commit 的 git blob 相同；实际运行本目录 `frozen-e79/s_structure_session` 的同字节副本。执行前后源码和二进制 SHA 均一致，绑定见 `EXECUTION-BINDING-e79.json`、`SOURCE-BINDING-CHECK-e79.json`。

## DUR-M02：已真正观察先持久、后发布

冻结源码 `s_structure_session.rs:852–877` 的 `persist_batch_before_publish` 在独立 IMMEDIATE 事务内核 Begin 所有权/epoch/generation、校验 batch_id、插入完整批次并核字节/长度，随后提交。`:1128` 先完成这一阶段，`:1133–1138` 再开发布事务并重核同一所有权和已持久的批次；对象、关系、修订、根及目录状态仍在发布事务中共同提交（`:1306`）。这与已签 #1370 AC4、SPEC C09.4 `:325`、接口合同 `:55` 的先后顺序一致。

本轮使用实际 native 在独占新库先发布 7 条输入的 cut-1，再接纳 512 条追加。先从只读连接看见真实 Begin，暂停并以 WUNTRACED 核实自己创建的进程；继续后捕获新批次已提交，再暂停自己的 PID。**暂停后重新创建两个只读连接**，均确认下列中间状态，避免拿保留的旧读事务冒充“根仍旧”：

| 读数 | 批次已持久、正式根仍旧 | 续跑 Commit 后 |
|---|---|---|
| generation / cut | `1 / cut-1` | `2 / cut-2` |
| 根 | `batch-cb49be6f405a8812a1b2e34bda0c098e3fc8a22d366fa6bad5a5086bc59f44a6` | `batch-04b7ff7f02f8be5c5b01800846b278e6c6c5d14d27e61ac113fe91c149cd8d80` |
| batch 数量 | 2 | 2 |
| 已发布对象表 | 原有 5 个对象 | 517 个对象 |
| advance_state | 真实 `begun`，前沿 518 | `idle` |

新增批次在根仍旧时已包含 **519 条 raw、517 个对象、1551 份见证和 2068 条关系**。独立标准库 checker 在这个阶段就完成 canonical 字节、batch/profile/raw/收据/对象/见证哈希及全字段引用闭包校验；续跑后正式根准确指向该批次，预先持久的字节未改变，旧批次也保持原字节。

独占库的观测触发器仅记录阶段，不代写结构。它在实际 Rust 写连接中分别记录 `batch_written`（gen1/旧根）与 `root_changed`（gen2/新根），两个阶段均为 SQLite 3.53.2、`synchronous=2(FULL)`、`fullfsync=1`。完整 sourceid 与记录在场景摘要中。这是正常进程的调度控制，未杀进程测试恢复，也不声称它覆盖断电/磁盘故障或全部平台部署前件。

主要证据：`runs/publication-e79/publication-order/real-begin-paused.json`、`durable-batch-old-root.json`、`fresh-second-reader-confirmation.json`、`advance-second.json`、`summary.json`。本次第一次执行即捕获目标状态，没有失败后换样本或省略未命中记录。

## 受影响路径及取消诊断

共运行 **7 个场景、46 个场景级断言、39 条实际 native 命令**；故意制造的过期/冲突/前沿变化拒绝是预期结果。另复用根已执行的同源码 8 项 bin tests，不重复跑无变化面。

| 项目 | 本轮结果 |
|---|---|
| 正常 512+1 交错 | Begin 后真实 Accept 第 513 条成功；原 attempt 先留下一份完整但未引用的 batch，再因前沿变化拒绝发布，根/generation/对象/关系/见证/观察均未抬升。自己的 Begin 正常取消后，重试无需 reset 成功；新根对应 input_frontier=512，且原未引用批次字节保留。拒绝后 batch 数为 1，重试后为 2，符合新协议。 |
| 过期 epoch | 独占库设 writer_epoch=2 后真实 Accept 报 StaleWriter；所有应用表完整逻辑指纹不变，含输入、收据、profile 和 meta。此处零写仍只指应用记录，不声称物理零 IO。 |
| profile 重放 | 同 profile 同输入逐项返回原收据；同 profile_id 仅改说明文字的规范字节仍报 IdentityConflict；已发布 cut/root/profile hash 不变，Snapshot 与已封存批次来源一致。 |
| 两次普通发布与旧批次闭包 | 原 7 条再追加 3 条，两份批次的字节、长度和所有引用闭包通过；旧批次没有被覆盖。独立校验 17 份批内 raw 记录、13 个对象和全部相关见证/关系。 |
| token / epoch / generation 保护 | 在三个专属新库的真实 Begin 后构造相应前件，真实运行均拒绝继续持久/发布并原样保留受保护字段和旧根。已发布 generation 场景仍为受控标记，未伪称第二个真实写者完成了 Commit。 |
| DUR-L01 | helper `:774–815` 返回 bool：不持有该 Begin 时返回 false，仅实际清理成功返回 true；调用者 `:1313–1320` 分别显示已取消或“未清理推进占用”。静态分支与根同源码 `cancellation_preserves_foreign_begin_and_reports_actual_release` 测试相符，未放宽 CAS 以迎合文案。 |

沿用探针时已在新文件 `run_probes_e79.py` 明确调整协议断言：允许未引用 batch，仍严格核根和发布对象不动；重试须覆盖新前沿并保留旧字节。原 `run_probes.py` 和 9a 报告保持不变，调整清单见 `PROBE-ADAPTATION-e79.json`。

native 保护反例先触发阶段所有权校验，不能拿它们冒称动态捕获了 cancel helper 开始前的极窄竞态；helper 的各拒清分支由固定源码及下述实际单测接回。0/1/2、三点同价与部分包含事实的构造/去重没有本轮修改，复用 `REVIEW-9a` 的既有实测，不重复跑；没有将那些局部诊断扩大为未定 G 或非退化 OHLC 的验收。

## 复用的实际检查与边界

根的 `cargo-session-test-typed-meta.json` 绑定入口 SHA `c376159b6fb0a9542fc00cccd9061cfccd94080fd0b44ce53d0cb124ad7c9841`，与本次冻结入口一致；对应日志实录 **8 passed / 0 failed**，包含批次先可读及不可覆写、取消保护与 bool 结果、过期接纳零写、profile 绑定、同价域事实/坏 JSON 失败和 wire 坏类型拒绝。之前缺 std::time 导入、测试目录冲突两次 harness 问题的原始日志仍在根证据目录；最终同源码检查已成功。本轮未将旧 harness 错误包装成程序失败，也未覆盖或删除记录。

所有创建的 native 子进程均已退出；本轮没有访问根数据库、18770–18773、GUI、E/B/X，也未修改仓库或 GitHub。只运行固定二进制与独占新库，并保留每一步命令/输出、触发器 SQL、完整可见性快照和哈希清单。外部可部署性、断电/崩溃恢复、完整历史及其他任务的验收不由本轮通过自动获得。
