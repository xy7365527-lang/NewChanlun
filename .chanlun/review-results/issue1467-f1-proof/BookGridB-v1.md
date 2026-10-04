# B 臂订单与报价历史共同网格接口 v1

日期：2026-10-04。绑定 #1468 Stage44，承接 #1467 固定探索样本。名分为票内工作草稿。本版是 P3 数据前置，未训练、选模或比较收益，未给 F₁/F₂ 装配输出，也不是最终强 B 或主确认结果。

本版导出与 A 相同的 572 个 UTC 100ms 决策。所有行保留，A 五项成交特征逐项相同。3 行盘口不可用，其中 2 行仍有成交；报价特征为空不会删除成交。27 项靶向检查通过，包含全部真实网格与独立朴素参考对拍。这里的独立指另一套实现，并非另一位审查者；新上下文独评由主控另行组织。

## 运行前固定的非递归特征

本轮固定以下字段后运行，没有读取标签来筛选特征、选择窗口或调参。

| 字段 | 定义与单位 |
|---|---|
| A 原有五项 | 最近成交价、已见成交数、距最近成交纳秒数、1秒已观察成交数及数量；直接复用 A 的精确接口，原有缺失与完整性口径不变 |
| `bid_price` / `ask_price` | 截止前最后完整采集桶的已发布最佳买卖价，USD/BTC × 1e8 整数字符串 |
| `bid_quantity` / `ask_quantity` | 同状态下买卖最优价档总量，BTC × 1e8 整数字符串 |
| `spread` | ask−bid，USD/BTC × 1e8 |
| `queue_imbalance` | `(bid_quantity−ask_quantity)/(bid_quantity+ask_quantity)`，无量纲，保存精确有理数 numerator/denominator，不转浮点、不要求约分 |
| `published_l1_changes_1s` | 以 t−1秒时已知末状态为锚，统计 `(t−1秒,t)` 的封闭采集桶末 L1 状态变化次数；任一价或量变化计一次，桶内多次变化被压缩 |
| `published_mid2_delta_1s` | 当前 `bid+ask` 减窗口锚的 `bid+ask`，单位为二倍中间价的 1e8 整数，除以 2e8 才是 USD/BTC 的中间价变化 |

价量和时间全程 BigInt 或十进制字符串。没有 W、主动方、价格压力方向、收益率或盘口刷新次数作为特征。原生历史里的 `buy` 只保留上游协议字段，不推断它等于成交主动方。

`published_l1_changes_1s=0` 仅说明完整、连续的已发布 L1 窗口没有观测到末状态变化。它不说明没有原生订单事件、桶内变化或未观测事件。A 的 observed 成交计数仍只描述已观察范围，`trade_stream_completeness` 恒为 `not_established`，不能据零断言真实完整成交流为零。

## 截止、初始化和连续性

[程序](book_grid_features.mjs)的 `bookInput(rows)` 从 causal-view 投影出成交与发布报价，`featuresAt(input,t)` 提供单截点特征，`gridFeatures(input)` 导出共同网格。它不接收标签。共同起点为 `1609459202770648200`，最终采集水位为 `1609459259999443100`，与 A 相同。

所有截止严格要求 `capture<t<=watermark`。同一 capture 的全部记录组成一个桶，报价取末状态，barrier 取桶内任一边界。只有见到更晚 capture 才能封闭该桶；EOF 不封闭最后桶，截断在桶中间不产生消费该半桶的决策。时间恰等于 t 的数据全部排除，不拿源序号或 native_event_at 当可知时间。

当前报价不可用时，当前报价、价差、不平衡及滚动报价摘要为 null，A 继续累积已知成交。滚动窗口要求起点已覆盖 1 秒、窗口左端有可用锚、左端至 t 之间没有连续性边界。任一条件不满足，两项滚动摘要都为 null。

不可用状态、sequence gap、未知状态、snapshot、catchup、reinitialized_or_batch 和未知 transition 都切断滚动连续性。仅 `single_event` 与 `no_new_book_application` 不切断。快照和补应用后的末报价可以立即成为下一网格的当前状态；滚动摘要等边界离开窗口且存在有效左端锚后恢复。边界恰在窗口左端也保守记 null。同桶先缺口后恢复，仍切断滚动窗口，不能靠末状态掩盖缺口。

本样本前 13 行滚动报价摘要为 null。A 的前 10 行窗口不足规则保持不变。这两个数量不同来自报价初始化边界，不是丢行或重新选择资格域。

## 所有可见历史与本版压缩的边界

当前固定特征只使用发布 L1 压缩。它会丢失桶内报价路径、深度与订单身份信息，不能被称为完整订单历史模型，更不能让 C 读取全历史后只与这组小特征宣称公平比较。

为保留同历史预算的后续入口，程序另外实现 `nativeInput(rows,input)`、`rawInput(bytes,input)` 与 `historyAt(input,native,raw,t)`。后者在同一严格截止返回四份历史前缀：全部已见成交、全部发布行、全部已应用原生记录、全部已捕获原始消息。

| 历史 | 保留范围和可知时间 |
|---|---|
| 全部原始接收记录 | 源文件所有已捕获消息，包括 pending、snapshot 和完整载荷。消息保持原始字符串，避免 JSON 数字转 Number 损失原生精度；使用供应商 capture 截止 |
| 全部 Stage26 应用记录 | 20,571 条 applied 加 1 条 snapshot anchor，包含 7,082 条无模型变化应用；使用 available_capture 截止。43 条晚应用不能按较早 captured_at 回填 |
| 发布 L1 历史 | 20,868 条发布行各有源行、capture、quote、状态和连续性标记。它们是已发布视图，不能计作 20,868 条原生事件 |
| 全部已见成交 | 205 笔，与 A 导出的 trade_history 文件逐字同 SHA |

原始消息在被捕获时已知，不因其尚在 pending 而藏到应用时刻；由补应用才得到的中间簿状态则只能在 available_capture 后使用。这两个历史接口分别保留，避免混淆“消息已收到”与“状态已重建”。

native projection 从原应用记录白名单提取协议类型、原生序号/时刻、捕获与应用位置、归一化名称、原始 buy 字段、价量变化和前后报价；剥除 model_event、model_state、active、confirmed、geometry 等候选结构。它不安装 F₁/F₂。源应用数组本身仍可通过压缩源文件 SHA、源行和 application_index 追溯。

`audit.jsonl` 对每个共同截点保存发布行末位置、原始文件的 exclusive 字节末偏移、原生应用前缀长度。`history-manifest.json` 记录三个已有源文件的绝对路径、SHA、大小及缺项；原始行情不复制入本轮目录。完整原始文件和 Stage26 源应用均为既有仓外材料，没有重新重建一分钟。

“完整历史”在此仅指这份固定文件观察范围内的全部接收记录和全部应用记录。它不是交易所未丢包证明，不补起点以前的历史，也不能保证此前已经发生但未捕获的消息。四臂共同初始化、覆盖资格、历史长度和模型预算，以及合格 F₁/F₂ 仍须冻结。本接口保留了该样本的全历史可追溯入口；本版有限 L1 特征本身仍不足以替代四臂合同所要求的最终 B。

## 验证与实际命令

[检查程序](book_grid_features_test.mjs)通过 27 项检查。朴素参考逐决策过滤原始行、重新折叠时间戳 Map 并求和，不调用生产时间解析、截止、分桶、A 投影或特征实现。手工边界、64 份固定种子历史及全部 572 行均对齐。

检查涵盖 capture==t、纳秒差、超 Number 价量、完整桶末状态、桶内截断、EOF、初始化和缺口保留 A、历史不足 null、窗口锚、未知状态/transition、同桶缺口恢复、非法输入拒绝、防覆盖。真实前缀取 50/339/340/1000/10000/20867 行；只比各自水位内的决策和历史，不重跑旧恢复矩阵。扰动第251个决策之后的价量、原生序号和原始载荷，早前特征及全历史截止保持不变。

另对全部 572 个截止，以独立原始行过滤和原生应用数组计数核对返回的完整前缀及原始字节位置。3 个盘口不可用网格中 2 个保留成交。所有 A 字段逐项相同。保存标签只在测试末尾读取产品与 decision_ns 来核身份，未把标签值或资格送入特征路径。标签文件 SHA 为 `b945b5c6e0148b67ee6b1dda958b9200b91e16db4fff3c25f55622f6e83805fa`。

实际导出命令，工作目录为研究工作树：

```sh
node .chanlun/review-results/issue1467-f1-proof/book_grid_features.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-v1/baseline/rows.jsonl.gz \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1
```

目录必须不存在，复跑须换新目录。本轮先直接运行合成检查和真实检查，再导出，最后由 Node `spawnSync(process.execPath,args,{encoding:'utf8',timeout:60000,maxBuffer:65536})` 运行下面参数，exit=0 后以 `wx` 保存 stdout 为 `book-grid-b-v1/tests.json`：

```sh
node .chanlun/review-results/issue1467-f1-proof/book_grid_features_test.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467 \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1
```

没有新增下载、市场决策、formal/生产修改、提交或推送。代码仅新增本说明和两个 MJS。机器结果在仓外，本轮导出不含 tests.json 为 1,440,378 字节，加测试结果后仍低于 16MiB；程序在写入前检查总预算并保留 64KiB 检查收据余量。

## 哈希

| 输入/源码 | SHA256 |
|---|---|
| causal-view rows | `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5` |
| Stage26 gzip | `5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf` |
| Stage26 解压内容 | `6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30` |
| 原始全接收文件 | `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd` |
| B 源码 | `fbd18461dc3917184cb58e8229c727cfb69abcb5ae1a06eccea2395ffe032d39` |
| B 检查源码 | `d3318ec6ac22ef51cf4d37b61edd595ea0f3255d2b8c6ebff01ab54bf69e56c2` |
| 复用 A 源码 | `7dba29c959e9ff04cea98c123cff567d902631d047e593cb26575781690c4499` |

| 输出 | SHA256 |
|---|---|
| features.jsonl | `be957271f54b1b912e537bd6a2802ab0646d6e77b13d623ed070dd0ad3659adc` |
| audit.jsonl | `a711c4db6405fdecebd5871680a033593bb1358d4fd69ed14f28326c6261f5f9` |
| quote_history.jsonl.gz | `840a269d194ece7a7668a1fd24a39e41baa6ba3669736d897acd4b8b9a0d1c6e` |
| native_history.jsonl.gz | `62d02bdfd2c53566e05730d29bc9eaaa9f204468f18615361e988decb73f6306` |
| trade_history.jsonl | `0d0c7383aec4b31d464e7a8fee3b0b964fdc1b29ea8d003f5121725943978b68` |
| history-manifest.json | `7b3cee347b8fab3be1c69ea44a262bd8a3fdbe1dd8dcb6ea9fa5c60d244fb4d4` |
