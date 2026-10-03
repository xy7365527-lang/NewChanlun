# Stage16：真实采集前缀的可用盘口视图

日期：2026-10-03，#1467/#1468，命题 `CB-CAUSAL-VIEW-v1`。接续 [开工卡](Stage16WorkCard.md)及 Stage9 数据条件。仅使用既有一分钟探索样本，未启动确认实验或选择最终市场。

## 本轮结果

已导出20,868行采集前缀视图：前339行没有已初始化盘口，quote为null；第340行收到快照并补应用43条缓存消息后，只发布一行可用状态；其后20,528行按实际接收前缀继续更新。205笔原始记录成交按到达时刻出现，不因后来补应用而重复发布。

新程序的最终完整State SHA与Stage9完全一致。四个原样本检查点的新进程恢复、四个物理截断文件、短延迟补齐、未补齐缺口、源/二进制/状态错配均通过具名检查；另验证了检查点含9,000条pending消息时的跨进程恢复。空侧和锁定报价控制也正确拒绝给出可用数值。

这是固定样本与具名扰动的经验验证，不是全部行情协议上的因果性证明，也不是F₁/F₂或市场价值验收。

## 输出含义

[coinbase_causal_view.rs](coinbase_causal_view.rs)在每次 `State.consume` 完成全部可用补应用后写一行。行内包括：

| 字段 | 含义 |
|---|---|
| line / available_at_capture | 本次采集行及供应商采集时间；不拿事件时间代替可见时间 |
| native_type / native_sequence / native_event_at | 本行原消息类型、序号与事件时间，不冒充整批补应用的原因 |
| frontier / pending | 当前可连续应用到的序号及仍待缺口补齐的消息数 |
| status / quote | ready才提供双边严格报价；未初始化、缺口、空侧或锁定/交叉报价时quote为null，并保留具体状态 |
| transition | single_event、snapshot_batch、catchup_batch、no_new_book_application或unavailable |
| applied_messages / delayed_applied_messages | 本次应用总数及此前已捕获、此刻才可应用的数量 |
| applied_from / applied_to | 本次连续补应用的原生序号范围 |
| new_trade | 本次新接收到的成交观察；其可知时间不依赖盘口是否已同步 |

价格、数量、序号用精确整数字符串，价量尺度为1e8，避免JSON消费者先转浮点损失精度。尺度是表示精度，不宣称交易所最小报价单位等于1e-8。行中不包含order_id或client_oid；原日志和检查点保存在仓外。

`ready`仅表示相对于当前收到的消息和快照，可以恢复双边严格报价；不证明没有未观测后缀、隐藏流动性或其他场所交易。空侧是已观测的域边界，不是数值零价；quote=null也不意味着“市场上没有任何信息”，原日志仍保留完整来源。

供应商采集时刻给出本研究的信息可得时间口径，**不是本机实际发布/下单时刻**。实际处理延迟、同一时间戳下能否插入决策及执行延迟尚未测量。

## 核心复用与终态

Stage9源文件未修改。新研究文件复制其解析、记录/状态定义和全部State方法；[来源记录](stage16-core-origin.json)锁定前16,954字节及其SHA。该前缀逐字一致，只在后面增加视图、检查点与驱动。

| 项目 | 实测 |
|---|---|
| 输入SHA | `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd` |
| 完整State SHA | `c17603a9b18a1d885d580663ff360560ebdcfc451f09700963af334687d2b1cf`，与Stage9相同 |
| 可用/等待快照行 | 20,529 / 339 |
| 快照发布 | 第340行，补应用43条，frontier=19,247,051,589 |
| 输出新到成交 | 205 |
| 视图文件 | 13,416,347字节 |

从旧检查点的订单表独立重新聚合，以及从其已保存价位表读取，均与新导出在第340/10000行的报价一致。这检查的是导出与保存状态的符合性，仍不是独立交易所终态oracle。

编译保留一个未使用旧Checkpoint类型的警告，以保持复制核心逐字一致；没有将它冒称为零警告构建。Rust格式和JavaScript语法检查另行执行。

## 前缀与恢复证据

四个新进程分别从行0、339、340、10000恢复，输出后缀逐字等于连续运行相应后缀，最终State、book与视图链摘要一致。

另生成只含前339、340、1000、10000行的**物理截断输入文件**，各自重放。它们的全文件SHA与原文件不同，却给出相同模型视图前缀。源/二进制摘要只进收据和检查点，不进逐行特征视图。

行339的有界运行正常结束，但final_ready=false；它只证明成功导出了未知状态。完整运行最后不可用时退出2，具体原因看status；不能把进程退出0等同于行情已合格。

检查点绑定固定源SHA、当前可执行文件SHA、状态SHA及已输出视图链。错配源、错配二进制、修改状态但不更新摘要都在输出视图前拒绝。该机制依赖可信检查点，不声称抗恶意重写，也没有验证生产崩溃写入原子性。整个文件SHA绑定还意味着它是离线固定源恢复方案，不是可直接接上追加文件的实时流恢复协议。

## 缺口和迟到控制

删除原第1001行、序号19,247,052,250后，frontier停在19,247,052,249；其后19,867行维持sequence_gap、quote=null，完整运行退出2。

将同一消息延迟到原第1005行之后，并把采集时间改为该位置的时刻：先有四行缺口，补齐时一次应用5条消息，其中4条此前已捕获。只在补齐行发布一份状态，报价等于原基线相同序号前缀的报价；最终book与原基线一致。

该补齐行的原消息类型是received，却触发了其他消息的补应用。**不能把整批净量变化当成该received事件的新增挂单。**这正是snapshot/catchup批次标记需要保留的原因。

补充将它延迟到原第10005行之后：

- 第10000行检查点内有9,000条pending消息；
- 新进程恢复后，缺口仍保持不可用；
- 第10005行补齐时一次应用9,005条，其中9,004条迟应用；
- 恢复后缀与该扰动的连续运行逐字相同，最终book与原始基线相同。

两种延迟都属于人工扰动，不是声称真实数据发生过此事故。累计捕获跳号/逆序计数与“当前尚未补齐的缺口”也不是同一判断：补齐后前者仍可非零，当前状态可以重新ready，而先前缺口行不被回写。

最后以单行合成快照检查空买侧和锁定价：分别输出missing_quote和crossed_or_locked_quote，quote均为null、退出2。原样本和所有旧证据保持不变。

## 复现与资源

汇总、命令回执位置、代码和数据哈希见 [stage16-evidence.json](stage16-evidence.json)。原始视图、检查点与扰动文件位于 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-v1/`，留在仓外。新增证据实际大小见该JSON，低于本轮300MiB上限；没有新下载。

新目录复现主要矩阵及补充，重新编译后应创建新基线，不复用绑定旧可执行文件的检查点：

```sh
cargo build --offline --locked --manifest-path .chanlun/review-results/issue1467-f1-proof/stage16-replay/Cargo.toml --target-dir /tmp/nc1467-cargo-target
node .chanlun/review-results/issue1467-f1-proof/stage16_verify.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-causal-view \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-reproduction-2
node .chanlun/review-results/issue1467-f1-proof/stage16_gap_resume_verify.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-causal-view \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-reproduction-2/baseline \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-reproduction-2/gap-recovery
node .chanlun/review-results/issue1467-f1-proof/stage16_quote_contract_verify.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-causal-view \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-reproduction-2/baseline \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/checkpoints-a \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/causal-view-reproduction-2/quote-contract
```

主矩阵实测复用了本轮已经运行且校验过的基线，未重复生成它；脚本的可选最后参数用于这一复用，省略则新跑基线。补充测试只执行新情形，没有重跑已过矩阵。各子进程上限45秒，主矩阵最多并发2个。

## 到实验之间还缺什么

此导出提供共同接收前缀及L1/成交可用视图，**没有替代完整原事件日志**。A/B/B_seed仍需在同一截止点读取各自被允许的数据；共同决策域、特征、标签和预算尚未冻结。

它也不是原生序号顺序的全部L1中间状态：补应用批次被明确折叠成当时的一次发布。若F₁需要批内逻辑路径，必须从原日志重建并把全部依赖传播到真正可见时刻；若选择“发布后状态变化”作为观察时钟，应另行具名，不能无声替换旧候选。缺口后的观测也不能自动与缺口前当作连续完整价格路径拼接。

下一步是明确这些候选输入适配及firstKnown/knownAt映射，再检验真实数据上的非空F₁与前缀忠实性。完整F₂、独立语义审查、确认实验及生产采用仍开放；本轮不提供收益、排队或延迟性能结论。
