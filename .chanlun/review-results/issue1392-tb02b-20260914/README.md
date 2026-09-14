# #1392 新笔交付材料

名分：#1392 工作草稿，随本票交付。逐 bar 性能回归已修复并经独立代码审查通过；最终二进制的34次正式公开采集和独立Oracle对照已通过，四个事务点的八次真实故障恢复复测也全部通过。本目录不授予合入或关票，也不表示 #1323 总图完成。

`baseline/` 保存提交 `5eeaba4ae19ee6cfe038ee4e25eb61fa41779a0f` 对应的已有功能证据。`ARCHIVE.json` 逐份绑定原路径、字节数与 SHA256；归档时未改原文。旧 `VERIFIED-BUNDLE.json` 写于整体审查之前，其中“功能证据已齐”不能覆盖后来发现的性能问题。

该基线完成 17 条轨迹、82 个声明前缀、476 份公开候选的独立 Oracle 对照，34 次正常 S/Q/Chromium 采集及四个事务点的八次真实 SIGKILL 恢复。结果限于报告声明的域；同价一般域、S-4 跨实现等价及总图验收仍未结清。完整 JSONL、HAR、截图与失败尝试保留在各结果文件绑定的外部目录，不把摘要当作这些原件。

`baseline/APPEND-SCALING.json` 是同一生产 `OwnedIncrementalClassifier::append_bar` 的实测。512、1,024、2,048 根分别约 0.36、2.84、23.28 秒，4,096 根在 60 秒超时。当前修复在同一真实追加入口的四个量测点分别约0.00053、0.00144、0.00586、0.01480秒，四点Classification和Tower均与全量结果逐字段相同，见 `incremental/SCALING.json` 和 `FULL-COMPARISON.json`。解析回归108通过、0失败、8忽略。

`review/` 保存 Sandcastle 的完整审查原文、封装器结果和仅修复 JSON 格式后的解析件。工蜂使用 `deepseek/deepseek-v4.1-flash / max`，最终消息多出一个字符，故封装器状态仍为 `failed`；原始进程正常退出不等于结构化结果成功。审查指出性能回归、复现工具未入仓和交付手续缺项。

根任务不采用原审查中两条无效主张：它没有证明第三条件为假的候选不可达；`origin/main` 实际是该基线的祖先，该基线有四个未合入提交、25 个差异路径。具体订正在 `REVIEW-NORMALIZED.json` 的 `root_qualifications`，没有覆盖原文。

复现入口位于 `s_session/tests/compare_tb02b_independent.cjs` 和 `run_tb02b_faults.py`。比较器直接消费冻结原始 Oracle 与公开产物，不导入产品 reducer。Oracle 正本字节归档于 `s_session/tests/fixtures/tb02b/independent-source-oracle.json`，SHA256 为 `0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56`。采集仍使用 `tb02b_independent_harness.py`；已有计划引用其原始绝对路径，复查时不修改已签计划或原件。

比较既有采集时执行 `node s_session/tests/compare_tb02b_independent.cjs <原计划绝对路径> <新的结果文件>`。路径参数化已通过实际 prepare 检查：从 runner.main 接收相对 Node/Playwright 路径后，计划中的命令均为绝对路径；任意新结果子目录可准备，重复输出、根目录自身和越界输出均拒绝，原计划字节保持。该检查没有启动服务，见 `REPRO-PREPARE-CHECK.json`。重新执行故障矩阵时，先用 `python3 s_session/tests/run_tb02b_faults.py --help` 查看二进制、Node、Playwright 和输出目录参数。`tools/run-faults-r2-original.py` 是已运行版本的逐字归档，含当时机器路径；可复用入口的路径参数化不改变故障点、并发上限、清理或比较要求。

本目录会与产品修改、复现工具及必要复核一起进入同一交付版本。最终扫描、具体版本合入批准、主线 CI 和工位处置仍须核实，不能从上述基线通过推导。

增量差异另经 `/root/tb02b_incremental_review` 独立只读审查，结论为 `approve_bounded`：全量和增量共享分型、端点及单端点转移，checkpoint覆盖封口、同价未定撤回与撤尾恢复，只有真实稳定笔前缀才允许段缓存复用。适用域是合法OHLC、source坐标唯一严格递增且配置固定的逐根追加；非递增/重复坐标不在已证域。两个mid的扫描计数不保证巨大同价根集合、保留旧Rc快照及段扫描的常数成本；包含facts的外层分支仍分别实现。

`current/` 对应整合后交付工位的新二进制，正常34次采集和17/82独立对照已完成；`baseline/` 仍是修复前功能证据，不替代当前版本。`current/FAULT-RESULT.json` 对应同一二进制的八次新运行，覆盖四事务点、原 writer 隔离、最终原身份收据、独立Q存活及完整语义核心双跑一致。
