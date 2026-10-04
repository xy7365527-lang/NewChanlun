# A 臂成交历史网格导出 v1

日期：2026-10-04。绑定 #1468，沿用 #1467 的固定探索输入。名分为票内工作草稿。本轮只补 A 臂数据前置，不变更实验候选合同、F₁/F₂、formal 或生产程序。

已从20,868行因果视图提取205笔新到成交，在100ms UTC外生网格导出572行特征。572个 `product_id + decision_ns` 身份与已保存的 Stage35 profile/2 标签逐项一致，没有按标签或特征结果删行。569行符合共同盘口资格；另外3行仍保留，其中2行已有成交信息。1行尚未观察到成交，最近成交价和距成交时间均为 null。

## 固定特征与截止接口

[导出程序](trade_grid_features.mjs)提供 `tradeInput(rows)`、`gridFeatures(input)` 和 `featuresAt(trades, t, origin, watermark)`。最后一个接口消费 `tradeInput` 产生的有序精确成交历史、共同观察起点及采集水位。它可供相同截止点的对照调用，不读取标签。

| 特征 | 定义 | 未观察历史 |
|---|---|---|
| `last_trade_price` | 截止前最后收到的成交价，尺度1e8的整数字符串 | 从起点尚未见成交时 null |
| `seen_trade_count` | 从共同历史起点到截止前的已见成交数 | 0只表示在观察范围内未见成交 |
| `since_last_trade_ns` | 决策时间减最近成交的 capture，整数字符串 | 尚未见成交时 null |
| `observed_trade_count_1s` | `[t−1秒,t)` 中已见成交数 | 观察起点晚于窗口左端时 null |
| `observed_trade_quantity_1s` | 同窗已见成交数量之和，尺度1e8的整数字符串 | 观察起点晚于窗口左端时 null |

没有调参或特征挑选。这五项只是小型、可重跑的 A 接口探针，不是最终 A，也不代表最强 B。全量已见成交历史另存为 `trade_history.jsonl`，供后续相同观察范围下的历史表示使用。该文件含原样本最后桶里的成交时也只表示已捕获，消费方仍须执行水位截止。

价格与数量从 Stage16 的字符串直接转 BigInt，运算后转字符串。生产导出、公开截止接口与朴素参考均保持该精度。没有使用浮点价格、收益率或主动买卖标记；`maker_buy` 不进入投影或特征，未把协议侧别猜成主动方。

`history_start_ns` 为输入第一条采集记录时刻，并非第一笔成交时刻。本样本为 `1609459202770648200`，最终水位为 `1609459259999443100`。起点以前的成交历史未观察，不补造零价。前10个网格点的1秒窗口不足，窗口计数与数量均为 null。

## 同刻、EOF 和重复处理

时间戳保留纳秒。共同网格从严格晚于第一条采集时刻的UTC 100ms刻度开始，到最后采集水位为止，使用与 Stage35 profile/2 相同的范围。它不由订单候选、盘口刷新或结构事件调度。

特征只消费 `capture<t`。同一 capture 的全部成交按原接收次序一起进入下一个截止，最后一笔表示该完整桶内最后收到的成交；不把桶内中间状态用于同刻决策。桶只有观察到更晚 capture 才算封闭，EOF不封闭最后桶。所有导出决策满足 `t<=watermark`，故被使用的桶必有更晚水位。`capture==t` 的成交不进入该行。物理截断在同刻桶中间时，不产生依赖这个未封闭桶的决策。

Stage16 在 `State.consume` 收到 `match` 时保存成交，早于盘口连续应用。其原生 sequence 已见表会忽略同序号同载荷的重复消息，同序号冲突和重复 trade_id 报错。`new_trade` 仅在成交集合增加时发布，补应用不重发，见 `coinbase_causal_view.rs:423–448` 及 `:704–721`。本导出再拒绝重复 `new_trade.sequence`，不静默重复计数或选择保留哪个冲突版本。不同 sequence、相同价量的两笔成交都保留。现有视图不导出 trade_id，跨不同 sequence 的 trade_id 唯一性依赖已核验的上游。

## A 与审计信息隔离

特征累积器只接收成交的 capture、price、quantity。sequence只在入口检查重复，不进入成交投影或特征。盘口值、订单数、原生事件时间、行号、刷新标志、资格、标签、目标确认时间均不进入 A。测试把这些字段改成不同值甚至非法报价，A 输出仍逐字一致。

`features.jsonl` 只包含 profile、产品及决策身份和五个特征。`audit.jsonl` 独立保存截止行、截止采集时间、共同盘口资格/状态、是否覆盖1秒观察窗口和窗口内是否见 sequence_gap。审计路径不向特征函数回传数据。资格域仍是实验合同指定的可恢复报价域，不能称为无条件全市场样本。

本文件中的成交数和数量始终是“已观察到”的量，不能据0断言交易所实际没有成交。供应商漏报、未收消息、未观测场所及事件前历史均未被排除。审计固定声明 `trade_stream_completeness=not_established`，窗口内 sequence_gap 另行提示；该提示也不是完整性证书。遇到缺口，后续若要以完整成交流解释窗口值，必须另行冻结共同覆盖资格规则，不能拿当前观察计数冒充完整成交统计。盘口未初始化也不等于成交缺失，因此本程序保留初始化前已知成交。

## 实测检查

[检查程序](trade_grid_features_test.mjs)共25项通过，包含精确纳秒和超出Number安全范围的价量、`capture==t` 排除、初始化前成交、同刻多笔及桶内截断、窗口左端、未见成交、EOF、重复拒绝、不同序号同价量保留、字段隔离、缺口审计、非法输入及防覆盖。

朴素参考直接对原始行逐决策过滤，并重新求和；不调用导出的时间解析、投影、分桶或游标状态。手工样例、64份固定种子混合历史和全部真实572行均与它一致。另对真实输入前50/339/340/1000/10000/20867行截断，水位内全部特征与完整输入一致；修改第251个截止之后的成交价量不改变此前含该截止的251行。参考由同一作者编写，不能称为独立代理审查或一般正确性证明。

与当前 `order_label_probe.mjs` 的共同身份、截止行、采集时刻、资格和状态均对齐。又读回仓外已保存的 `label-grid-v2/labels.jsonl`，单独核对572个身份，未将其标签传入 A。该标签文件SHA为 `b945b5c6e0148b67ee6b1dda958b9200b91e16db4fff3c25f55622f6e83805fa`。

## 证据、哈希与复跑

仓外目录为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/trade-grid-a-v1/`，包含三个JSONL、`report.json`、`tests.json` 和 `saved-grid-alignment.json`。真实输入固定SHA为 `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5`，13,416,347字节。源输入未打印、未更改、未入仓。

| 文件 | SHA256 |
|---|---|
| 导出源码 | `7dba29c959e9ff04cea98c123cff567d902631d047e593cb26575781690c4499` |
| 检查源码 | `c829d2f1f1f6c70918b94e397a88edc81b09c15791c60cf4ebb1f59f6c325292` |
| features.jsonl | `030d090b4303542f4c7cc9961abd3419692f2684ef75dfb410ddc9c30f6cfed2` |
| audit.jsonl | `5d5632745d67482d965eeb595363171f40b7a67c8f93269bb1d442d69e5fd3d9` |
| trade_history.jsonl | `0d0c7383aec4b31d464e7a8fee3b0b964fdc1b29ea8d003f5121725943978b68` |

实际导出通过 `node --input-type=module` 导入 `run(inputPath, outputDirectory, expectedSha)`，检查目录未存在后调用。实际测试使用下列测试命令参数，由 Node 子进程运行并以 `wx` 将stdout保存到 `tests.json`。等效CLI复跑命令如下，必须指定尚不存在的目录；程序拒绝覆盖已有输出。

```sh
node --check .chanlun/review-results/issue1467-f1-proof/trade_grid_features.mjs
node .chanlun/review-results/issue1467-f1-proof/trade_grid_features_test.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/trade-grid-a-v1
node .chanlun/review-results/issue1467-f1-proof/trade_grid_features.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/trade-grid-a-v1-reproduction \
  a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5
```

尚未运行训练、参数搜索、预测比较或收益计算，未裁定完整F₁/F₂或递归增量。样本仍是已看过的一分钟探索材料，不进入未见确认集。供应商采集时钟也不包含本机处理或执行延迟。主确认所需的覆盖规则、共同全历史表示、合格C、模型预算及未见样本仍须由后续合同冻结。
