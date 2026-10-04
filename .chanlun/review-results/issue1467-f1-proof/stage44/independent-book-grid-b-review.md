# #1468 Stage44 B 臂独立审查

审查日期：2026-10-04。结论：本次限定范围内通过，未发现阻断缺陷。可接受为 P3 的共同网格数据与历史截止接口前置；不能据此验收最终强 B、四臂信息公平、F₁/F₂ 资格或预测/交易增量。

## 范围与独立性

只读研究树 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun` 中的 `book_grid_features.mjs`、`book_grid_features_test.mjs`、`BookGridB-v1.md`，并读取 `trade_grid_features.mjs` 与 `ExperimentDesignCandidate-v1.md`。审查对象位于 `.chanlun/review-results/issue1467-f1-proof/`。原始证据根为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/`。

新鲜审查上下文独立读码，除实际执行作者 27 项检查外，自写 Python `verify.py`，以 Python 整数、Decimal、UTC 日历解析从原始接收文件重新计算全部 A 特征，再从已发布原始行计算 B 全字段。此脚本未导入任何作者 MJS，也未调用其时间解析、投影、分桶、截止或特征实现。另用 `api-cutoffs.mjs` 调作者公开接口，把结果与 Python 独立过滤结果逐行比较。知识图谱无该工作树索引，才直接读取已指定源码。

独立文件全部保存在 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1-independent/`，未覆盖作者结果。未重建一分钟市场簿、下载、训练、比较收益、改作者文件、提交或推送。

## 阻断发现

无。

## 已验证结果

| 对象 | 本次实测 |
|---|---|
| 作者检查 | 原作者目录及新导出目录各执行一次，均 27/27，exit 0 |
| 同网格 | 572 行；product_id、decision_ns 与保存的 A 和 Stage35 标签逐行一致 |
| A 保留 | 从原始全接收文件的 205 条 match，以 Decimal 精确缩放重算五项；572 行全部等于保存 A 和 B 内对应字段 |
| B 有限特征 | 572 行全部字段与独立 Python 结果精确一致，包括四项价量、价差、有理队列不平衡及两项滚动摘要 |
| 完整接收历史 | 20,868 条；逐条验证原始消息字符串、字节起止及总字节数；全 572 截止的末行和 exclusive 字节偏移一致 |
| 应用历史 | 20,571 applied + 1 anchor；其中 7,082 noop；全投影逐字段等于对 Stage26 源应用白名单的独立映射 |
| 应用来源 | 每条 applied 的 captured_line 指向原始接收记录，native_sequence、native_type、captured_at 均与原始记录一致 |
| 可用时钟 | 全 572 截止逐个过滤源应用的 available_capture，数量和全部 application 身份序列 SHA 与 historyAt 一致 |
| pending/迟应用 | 第一次应用前已有 339 条原始接收记录；43 条迟应用只按 available_capture 出现，未按 captured_at 回填 |
| 缺失保留 | 3 行当前盘口缺失，2 行仍有成交；13 行滚动盘口摘要为 null |
| 重导出 | 新目录七个导出文件与作者目录逐字节相同，包括 features、audit、两份 gzip、trade_history、manifest、report |

代码的截止保证见 `book_grid_features.mjs:47`：严格 `capture < t <= watermark`，二分返回最后完整桶末行。网格不会消费 EOF 最后一桶；同 capture 桶先遇缺口再恢复，`barrier ||= barrier` 保留缺口，见 `:38`。上述桶边界、纳秒、超 Number 价量、窗口不足、snapshot/catchup/未知状态和 transition 均由实际通过的作者合成检查覆盖；独立 Python 另覆盖真实样本全部 572 行。

滚动特征在 `:58–66` 使用窗口左端最后状态作为锚，仅在观察长度够 1 秒、锚有报价且窗口无边界时计算。边界恰在左端仍保守为 null。当前不可用状态自身成为 barrier，因此不会出现当前报价为空而继续算滚动 delta。零计数仅表示封桶末 L1 状态没有变化，不意味着原生事件或桶内路径不存在。

`rawInput` 在 `:87–96` 保留不经 JSON.parse 的消息字符串与字节位置，pending 信息在接收时即可见。`nativeInput` 在 `:110–121` 校验应用可用时刻、原接收位置并白名单投影；`historyAt` 在 `:128–133` 对应用使用 available_capture，而接收历史用原始接收 capture。两种历史没有混成一个回填时钟。

特征函数没有读取标签或资格字段，未将 F₁/F₂ 状态输入 B。审计里的 status、ready_at_decision、source_line、native_application_end 与有限特征文件分离。作者测试读取标签只核对 product_id 和 decision_ns；独立脚本也只取这些身份列。原始 buy 字段作为协议数据保留，未推断主动方。

## 非阻断边界

1. 有限 B 是 L1 压缩，缺少深度、订单身份和桶内路径的特征表达。全原始消息及 Stage26 应用来源可追溯，不等于这些信息已被有限预测特征消费。说明书明确写出这一点，当前不存在过称；后续 C 不能只与该小特征集比较后宣布信息公平。
2. `native_history` 是全部已应用记录，pending 和尚未应用的原文在 raw history 中。完整性仅相对于固定保存文件，未建立交易所全流无缺失，也没有补足捕获起点前的历史。
3. 公开接口是可信投影之间的研究接口，尚无强制隔离的训练/实时权限边界；未来消费者必须只消费 historyAt 的截止结果，不能直接把 manifest 所指的完整源文件或 audit 元数据当模型特征。本轮不存在这样的消费者，故不阻断本数据前置验收。
4. 初始化、重置、历史预算、模型与搜索预算和合格 F₁/F₂ 尚待四臂合同冻结。未将 572 行或 569 个可标目标称为已具备共同完整特征的训练样本。

## 实际命令与收据

以下命令均实际执行，退出码为 0。前两条工作目录是上述研究树；相对源码路径均位于该树。

```sh
node .chanlun/review-results/issue1467-f1-proof/book_grid_features_test.mjs /Users/silencehan/Documents/Codex/research-evidence/issue1467 /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1
node .chanlun/review-results/issue1467-f1-proof/book_grid_features.mjs /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/rows.jsonl /Users/silencehan/Documents/Codex/research-evidence/issue1467/native-quote-v1/baseline/rows.jsonl.gz /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1-independent/export
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1-independent/verify.py
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1-independent/api-cutoffs.mjs
node .chanlun/review-results/issue1467-f1-proof/book_grid_features_test.mjs /Users/silencehan/Documents/Codex/research-evidence/issue1467 /Users/silencehan/Documents/Codex/research-evidence/issue1467/book-grid-b-v1-independent/export
```

另以 Python 断言 `independent-cutoffs.json == api-cutoffs.json`，并断言 `author-tests-rerun.json` 的 passed 为 27，产出 `final-receipt.json`。两个截止文件不只数量一致，而是 572 行的 receipt_end、byte_end、native_end 和 native identity SHA 全部一致。完整哈希清单位于该 final receipt，源码/合同哈希位于 `independent-verification.json`。

| 文件 | SHA256 |
|---|---|
| B 源码 | `fbd18461dc3917184cb58e8229c727cfb69abcb5ae1a06eccea2395ffe032d39` |
| 作者测试 | `d3318ec6ac22ef51cf4d37b61edd595ea0f3255d2b8c6ebff01ab54bf69e56c2` |
| A 源码 | `7dba29c959e9ff04cea98c123cff567d902631d047e593cb26575781690c4499` |
| B 说明 | `10d6d356f9ff61376eb1d15a707038b64cf530c9b402f6a3ab9709339fb8c5d7` |
| 实验候选合同 | `fa5cc0c57fadf723629969267ae58ab85ac5bd478ac7ab9ebe742f0565c57dac` |
| 独立 verify.py | `da156368381881be5106d2a0a4e04785664457a5a621f4c3fea9c866b29e71a5` |
| API 检查脚本 | `5b868cb65a84ddd6a427c01de1b5769a4c14e6c6dd08f9093fb40b25e3ae5c6c` |
| 两份截止比较文件，各自 | `ac87cd731e567781480d28d54d86012cc4826d011b44e82ae5fea6a0d0326f0c` |
| 作者测试复跑收据 | `14e1b0f202b1e02021292e15e9217b1e16a83e2381eeee4d3dd8db797fb8bf67` |
| B features，新旧相同 | `be957271f54b1b912e537bd6a2802ab0646d6e77b13d623ed070dd0ad3659adc` |

原始三输入的 SHA 由导出器实际校验，分别是 causal-view `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5`、Stage26 gzip `5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf`、原始接收文件 `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd`。Stage26 解压内容 SHA 为 `6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30`。
