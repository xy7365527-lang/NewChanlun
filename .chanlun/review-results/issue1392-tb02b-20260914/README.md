# #1392 新笔交付材料

名分：#1392 工作草稿，随本票交付。2026-09-15 当前交付代码另含全量解析成本修复和六项CI回归修复；本目录的34次公开采集及八次故障恢复属于此前 `c67d1a9` 交付版本，不冒充新二进制重跑。本目录不授予合入或关票，也不表示 #1323 总图完成。

本次修复的证据集中于 `ci-repair/MANIFEST.json`：两组9000根逐前缀 Classification/完整Tower对拍、2000根自持输入对拍、两个构造证书测试和BSP扫描守卫均通过。第二组9000根实际触发空投影1646次、pop的T==1分支14701次和T>1分支779次；证书测试实际产生18条事务（frontier_pop10、cascade4）。合成价序显式细化消除同价身份歧义，原非空/分支断言保留。四个扫描豁免仅更新原位置行号，类型仍为包含组/Fractal，非BspPoint。

全量 `parse_layer` 改用紧凑包含状态，再从头调用同一新笔transition；原始位置映射显式传入，缓存证书字段仍为0/0/None。完整证据API和S写入调用它的路径未改变；S仅在不可用层时调用的空 `parse_layer` 结果也保持。原增量解析和S/Q正常、故障证据因此按其已验代码路径复用，不能表述成当前构建全部重跑。该解析修改的109项parser测试已有实际通过日志；本次没有重复全库长跑。

固定代码 `10be313` 的合并独评已完成：Sandcastle 的 DeepSeek V4.1 Flash/max 实读代码、差异、原测试日志及文件 Hash，T1（全量解析）和 T2（夹具/扫描）均为 `approve_bounded`。原文节选与审查来源 Hash 见 `ci-repair/1392-CONSOLIDATED-INDEPENDENT-REVIEW.json`。独评没有重跑长测，且明确保留三项同价域失败及合入批准门。

原CI `34851673814` 实际为2931通过、9失败、161忽略，六项已修复。余下真实BTC的两项非空/标定断言和合成鞅的一项非空账簿断言，因同价原始极值身份未定而失败，原价和断言仍保留。G-002决定票#1343与实施票#1405尚未完成；#1392票面明确限定无同价竞争域且不得反向等待以B为前件的V。此次不宣称同价一般域成功、不把全库CI写成绿色；合入须明确审阅这三项域外失败及复用范围。

`baseline/` 保存提交 `5eeaba4ae19ee6cfe038ee4e25eb61fa41779a0f` 对应的已有功能证据。`ARCHIVE.json` 逐份绑定原路径、字节数与 SHA256；归档时未改原文。旧 `VERIFIED-BUNDLE.json` 写于整体审查之前，其中“功能证据已齐”不能覆盖后来发现的性能问题。

该基线完成 17 条轨迹、82 个声明前缀、476 份公开候选的独立 Oracle 对照，34 次正常 S/Q/Chromium 采集及四个事务点的八次真实 SIGKILL 恢复。结果限于报告声明的域；同价一般域、S-4 跨实现等价及总图验收仍未结清。完整 JSONL、HAR、截图与失败尝试保留在各结果文件绑定的外部目录，不把摘要当作这些原件。

`baseline/APPEND-SCALING.json` 是同一生产 `OwnedIncrementalClassifier::append_bar` 的实测。512、1,024、2,048 根分别约 0.36、2.84、23.28 秒，4,096 根在 60 秒超时。当前修复在同一真实追加入口的四个量测点分别约0.00053、0.00144、0.00586、0.01480秒，四点Classification和Tower均与全量结果逐字段相同，见 `incremental/SCALING.json` 和 `FULL-COMPARISON.json`。解析回归108通过、0失败、8忽略。

`review/` 保存 Sandcastle 的完整审查原文、封装器结果和仅修复 JSON 格式后的解析件。工蜂使用 `deepseek/deepseek-v4.1-flash / max`，最终消息多出一个字符，故封装器状态仍为 `failed`；原始进程正常退出不等于结构化结果成功。审查指出性能回归、复现工具未入仓和交付手续缺项。

根任务不采用原审查中两条无效主张：它没有证明第三条件为假的候选不可达；`origin/main` 实际是该基线的祖先，该基线有四个未合入提交、25 个差异路径。具体订正在 `REVIEW-NORMALIZED.json` 的 `root_qualifications`，没有覆盖原文。

复现入口位于 `s_session/tests/compare_tb02b_independent.cjs` 和 `run_tb02b_faults.py`。比较器直接消费冻结原始 Oracle 与公开产物，不导入产品 reducer。Oracle 正本字节归档于 `s_session/tests/fixtures/tb02b/independent-source-oracle.json`，SHA256 为 `0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56`。采集仍使用 `tb02b_independent_harness.py`；已有计划引用其原始绝对路径，复查时不修改已签计划或原件。

比较既有采集时执行 `node s_session/tests/compare_tb02b_independent.cjs <原计划绝对路径> <新的结果文件>`。路径参数化已通过实际 prepare 检查：从 runner.main 接收相对 Node/Playwright 路径后，计划中的命令均为绝对路径；任意新结果子目录可准备，重复输出、根目录自身和越界输出均拒绝，原计划字节保持。该检查没有启动服务，见 `REPRO-PREPARE-CHECK.json`。重新执行故障矩阵时，先用 `python3 s_session/tests/run_tb02b_faults.py --help` 查看二进制、Node、Playwright 和输出目录参数。`tools/run-faults-r2-original.py` 是已运行版本的逐字归档，含当时机器路径；可复用入口的路径参数化不改变故障点、并发上限、清理或比较要求。

本目录会与产品修改、复现工具及必要复核一起进入同一交付版本。最终扫描、具体版本合入批准、主线 CI 和工位处置仍须核实，不能从上述基线通过推导。

增量差异另经 `/root/tb02b_incremental_review` 独立只读审查，结论为 `approve_bounded`：全量和增量共享分型、端点及单端点转移，checkpoint覆盖封口、同价未定撤回与撤尾恢复，只有真实稳定笔前缀才允许段缓存复用。适用域是合法OHLC、source坐标唯一严格递增且配置固定的逐根追加；非递增/重复坐标不在已证域。两个mid的扫描计数不保证巨大同价根集合、保留旧Rc快照及段扫描的常数成本；包含facts的外层分支仍分别实现。

`current/` 对应2026-09-14整合交付工位的二进制，正常34次采集和17/82独立对照已完成；`baseline/` 仍是修复前功能证据，不替代当前版本。`current/FAULT-RESULT.json` 对应同一二进制的八次新运行，覆盖四事务点、原 writer 隔离、最终原身份收据、独立Q存活及完整语义核心双跑一致。
