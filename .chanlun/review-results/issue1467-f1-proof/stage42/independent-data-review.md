# #1467/#1468 数据适用性独立代码审查报告

日期：2026-10-04。审查者：独立子任务 `review_data_domain`。票型与范围：既有研究票 #1467/#1468 的只读独立复核，覆盖固定平台域的首次因果初始化及到捕获 line422 的首段。名分：[新缠论:候选] 的有限数据适用性验证。

结论：PASS。冻结报告中的首段读数、初始化规则和域外分类得到独立核对；未发现需要修复的高置信问题。此结论不赋予候选原义走势、生产接口或市场价值资格。

## 概念层质询结果

通过。逐条回读 `PlateauBaseCandidate-v2.md` §2–3，并对照 Stage16/26/29 对发布状态、原生应用和可知时刻的定义。审查对象明确声明为候选，未把候选的单点核心或完成规则冒称教义正本。本轮不重审候选的一般存在唯一性证明，也不重新裁定缠论语义；没有发现需要进入谱系矛盾比对的定义冲突。

| 审查点 | 当前证据及判断 |
|---|---|
| 首次 ready 与 B0 | Stage16/26 最早 ready 均为 line340，前339行 Stage16 quote 均为 null，原始前339行没有 full_snapshot。独立从 line340 快照及此前已捕获消息中选出快照序号之后的43条，按源序号重放，所得 B0 与两份既有输出一致。参数在此确定，未读后续完成数选窗口。 |
| 初始化不充种子 | 快照和43条补应用只形成 B0。模型 e1 从其后第一条真实簿变开始；未知前缀不置零，不给初始化回填核心。line341–422 的82条应用均为单条即时应用，未夹入 snapshot/catchup 平台种子。 |
| 两种时钟 | 完整源序号包含全部82条应用，其中55条实际簿变。26 received 和1 nonresting done 不计模型观测但参与连续性检查。此簿变时钟是具名 profile，不能继承“每条 full 消息计一次”的另一候选结论。 |
| 固定档与最优档 | Q 从固定价位读取数量；因果 b0 只在 B0 选择，此后没有改为逐时追踪最优买档。W 另行检查最优买价、卖价及实际卖量，未将这组条件偷加给 Q。literal 100 档与因果 b0 档分别记录。 |
| e12 两类边界 | 第12条簿变为 match，capture356，seq19247051605。新增/撤单适配不覆盖它，因此 strict Q 只有覆盖边界。独立重建同时得到实际 K 从115337272降到115268421，因此因果固定参数 W 确实域外。禁止成交不是 D_Q 的数学条件。 |
| e55 短平台 | 明确允许按 maker 残量扣成交量的诊断 Q 中，e54 的平台长度依次为17、5、30、2。e53–54仍为开放待定平台，e55转值才封口长度2平台并离开 D_Q。失败前有3个成熟核心，方向为[-1,-1]，故完成数确实为0。 |
| 冻结与余项 | 失败事件不进入有效历史；严格适配冻结于e11，诊断Q冻结于e54。代码没有删除短平台、拼回新值、重启或换锚点；原文件保留完整20868行，摘要列出未分析采集后缀。 |
| 成熟与完成 | 1个或3个成熟核心不等于完成对象。没有反向成熟边，0完成是已核的结构结果；前339行未知盘口和e54的两事件待定平台保持各自状态。无三份完成对象，父交集没有输入资格，不能把未执行的几何检验称为“交集为空”。 |
| literal 与新 profile | literal W 的100/102/10在B0即不符。按首次ready确定的 b0/a0/K0 是另一个具名固定参数实例；复核没有把它升级为 literal 模型的真实数据验证。 |

可知时间也逐应用核对到了原始捕获记录。e12 的 native time 为 `2021-01-01T00:00:01.624130Z`，available time 为 `2021-01-01T00:00:03.0696484Z`；e55 分别为 `2021-01-01T00:00:02.072849Z` 和 `2021-01-01T00:00:03.0699408Z`。该时间语义仍是供应商捕获前缀下的信息可得口径，未证明本机实际发出信号或可执行交易的时刻。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | pass |
| LOW | 0 | pass |

对 `plateau_data_domain_probe.mjs:24` 的首次ready选择、`:31` 的残量重建、`:60` 的有限前缀计数、`:70` 之后的时钟与域外停机逐段审查，并回读 `native_quote_blocks.rs:923` 的应用账生成。图工具未索引此研究工作树，故按明确源码路径读取；未使用其他分支图结果代替当前文件。

逐字复制作者探针到独立子目录后运行，其全部断言通过，生成摘要与冻结摘要逐字相同。为补核作者探针中未独立断言的消息时间、簿变标签及档位增减，新增 `independent-check.mjs`：由原始快照独立建立订单表，每步重新聚合全价档，不复用作者的增量价档更新函数；按原始事件判定实际净簿变，再与 Stage26 的 disposition、normalization、方向、价位、signed_delta、level_before/after 及前后报价比较。

独立核验实际通过125条连续原生应用、125组原生/捕获/可知时间关系、82组簿变报价及档位增减。初始化后82条应用为26 open、28 canceled done、1 match、26 received、1 nonresting done，其中55条真实簿变。观察列单独按最大常值段计数，核得17/5/30/2及0完成。

未给未触发的 change 分支或所有合法交易所消息发放验收结论。原探针是冻结输入的有限探针，复核按此范围判断；测试通过不等于一般协议实现正确。

## 命令与证据绑定

工作目录为本报告所在的 `f2-interface-v1/`。实际执行命令如下，均退出0：

```sh
mkdir -p independent-data-rerun-20261004
cp plateau_data_domain_probe.mjs independent-data-rerun-20261004/plateau_data_domain_probe.mjs
node --check independent-data-rerun-20261004/plateau_data_domain_probe.mjs
node independent-data-rerun-20261004/plateau_data_domain_probe.mjs
cmp plateau-data-domain-summary.json independent-data-rerun-20261004/plateau-data-domain-summary.json
node --check independent-data-rerun-20261004/independent-check.mjs
node independent-data-rerun-20261004/independent-check.mjs
```

| 文件 | SHA-256 |
|---|---|
| 冻结 data-domain.md | `5b9e94feaecb9872c09484499b427f01af849479c51ce73f82b87c178342ffc3` |
| 冻结及逐字复制 plateau_data_domain_probe.mjs | `8deeb0bbfb6f4110c610542fdf4e9c4bcb632168de937cb8187186f76d6a0a95` |
| 冻结及复跑 plateau-data-domain-summary.json | `32e59e88edcbe18c8b90b5dcccf6de50a86177474f111cb1f8cadad61169ddc9` |
| 独立 independent-check.mjs | `ee356ed55b4c900ea40efd2d76bd0e8a18412def4c20f84ab8e5dc9bd4d48780` |
| 独立 independent-check-summary.json | `e9fe7b18855bb8b5aeb66c7c0ff670471f9373e0f905fcbfa14dee83c3d4e6c4` |
| 原始 full.ndjson | `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd` |
| Stage16 rows.jsonl | `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5` |
| Stage26 rows.jsonl.gz | `5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf` |
| Stage26 解压字节 | `6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30` |

源文件指纹还由复跑摘要与冻结摘要的逐字比较覆盖。源码/文档路径与对应SHA完整保留于原摘要的 source_hashes，不另造新的来源清单。

## 验收边界

本轮确实读取既有真实 raw、Stage16 发布视图及 Stage26 应用账，但只独立重建到 line422。没有重新审完交易所完整协议、全分钟状态重建、恢复矩阵、一般因果性证明或全部 F1/F2 语义。该分钟是已探索材料，不是冻结后的统计确认集；没有验证预测、交易或市场价值。

仓内文件、旧报告、原始行情和作者摘要未写入。未提交、推送或发送外部消息。工作树已有其他人的两份已跟踪文档改动与未跟踪研究设施，均保持原状。新增物仅为本报告和指定目录下的独立复跑证据；不保存订单ID或原始消息副本。
