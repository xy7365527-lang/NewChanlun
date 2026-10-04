# #1467/#1468 固定平台域的数据适用性首段检验

日期：2026-10-04。作者：plateau_data_domain 独立子任务。名分：[新缠论:候选] 的数据适用性诊断，不是 literal v2 的真实数据验收，不是收益实验。

结论：既有原始日志和 Stage26 应用账足以在具名输入适配下检验固定档 Q 和实际固定参数 W 的首段。已有数据不支持在本次首段形成三个完成候选或父核心。literal W 在初始化就不满足 100/102/10；因果初始化参数版本的 W 在第12个真实簿变事件因实际卖量 K 变化而退出。具名新增/撤单适配器同时遇到 match 的未覆盖边界；这不构成数学 D_Q 禁止成交的结论。另以明确成交扣残量规则复核 Q：第55个簿变事件封口一个两事件短平台，是 D_Q 平台条件的实际反例；此前仍零完成候选。

## 数据为何足够，哪些材料单独不够

- [v2 §2](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/PlateauBaseCandidate-v2.md:33) 要求初态、真实事件、订单身份与残量、连续源序号和后状态。第35行仅限定原见证探针实现范围；不把这段实现自述升格为禁止其他原生事件的数学公理。
- [Stage16](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/Stage16CausalViews.md:15) 每捕获行只发布 consume 完成后的最终视图。snapshot_batch/catchup_batch 聚合了原生路径；无订单身份，也没有固定非最优价档数量。因此仅从 rows.jsonl 不能恢复批内各事件、每个中间 q，或验证单订单残量合法性。无法把收到快照的一行、43条补应用形成的一次发布、或339条未知状态算成平台种子。
- [Stage26 逐应用账](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/native_quote_blocks.rs:923) 包含 native_sequence、原捕获行、真正可知行、native time、normalization；实际簿变还包含 before/after quote、buy/price、signed_delta、level_before/after。noop 仍在连续序号账里。配合原始 L3 快照和 full 消息，可在内存重建订单身份、全价位与特定 b0 档，无须猜造丢失字段。应用账本身匿名，因此仍不能替代原日志。
- [Stage29](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/Stage29NativeStateTime.md:15) 绑定的是发布后 frontier 对应的最后原生消息时间，并不是批内所有原生路径。它说明来源时间与可知时间必须分别保存；本探针直接用 Stage26 原生账，不把该发布状态时间当成交或报价首次变化时间。

## 因果起点、时钟与具名版本

初始化取完整样本中**最早 ready 的捕获前缀**，即 line340，frontier=19247051589，可知时刻 2021-01-01T00:00:03.0671428Z。这是停止时刻规则，不按后续完成数量选择窗口。此前339个捕获前缀保持未知；line340的完整快照和43个补应用只形成 B0，不计平台事件，不向过去发布输出。参数仅由这个 B0 读取并永久固定。

精确参数用 1e8 整数尺度保存在机器摘要；本次值为 b0=2899007000000，a0=2899008000000，K0=115337272，q0=1206358847。literal Q 的 100 档初量为 85207076229。以上为本地衍生参数，不是新增下载或原始载荷副本。价格尺度只说明表示精度，不声称最小报价单位为1e-8。

主适配使用**实际改变在簿状态的原生消息**作观测时钟，每条最多一个 e，不把 remove/readd 实现步骤拆开。received 和非在簿 done 等不计观测，但全部检查、保留在原日志及连续源序号账中；模型索引 n 与连续原生序号 s 是不同坐标。该时钟是显式新 profile，不声称等同“每条 full 消息都给一个同值观测”的另一候选，也不继承 literal v2 通过状态。若以后把 no-op 也计观测，平台长度与成熟时间可能改变，须另行具名检验，不能在失败后换时钟放行。

| profile | 固定参数/状态规则 | 本次边界含义 |
|---|---|---|
| literal-Q-100-add-cancel-prefix/1 | 固定100档；仅新增/撤单适配 | e12 为适配器覆盖边界，尚不能据此宣布数学 D_Q 域外 |
| causal-init-fixed-b0-add-cancel/1 | b0来自首次ready；仅新增/撤单适配 | 同上，不继承literal Q结论 |
| causal-init-fixed-b0-a0-K0-add-cancel/1 | b0/a0/K0来自首次ready；固定实际盘口条件 | e12发生真正 W 域外，同时超出适配器覆盖 |
| causal-init-fixed-b0-native-removal-diagnostic/1 | 同一B0/b0/观测时钟，原生match按maker残量扣除；明确新状态规则 | 检查Q平台数学条件，不恢复W、不声称协议全面实现 |

没有因果重启或换锚点策略。本轮也没有把 match 默改为撤单：其 native_type 与 normalization 始终保存。新状态规则只按原消息数量扣 maker 残量，检查方向、价位、正量及不超残量。

## 首次边界与失败前结构

| 检验 | 首次边界 | 边界前 | 三完成候选/父核心 |
|---|---|---|---|
| literal W (100/102/10) | B0实际最优价格与固定参数不符，n=0 | 无有效W前缀 | 否 |
| literal Q 固定100档，新增/撤单适配 | e12，capture356，seq19247051605：match 未在此适配器覆盖 | 11个同值真实簿变观测；1成熟核心；0完成 | 否；不是Q数学反例 |
| 因果 b0 的新增/撤单 Q 适配 | 同一e12覆盖边界 | 11观测；1成熟核心；0完成 | 否；数学Q是否继续另按声明规则检查 |
| 因果固定 b0/a0/K0 的 W | e12，capture356，seq19247051605：**实际K改变** | 11观测；1成熟核心；0完成 | 否；W固定域确实退出 |
| 原生成交扣残量诊断 Q | e55，capture422，seq19247051671：open封口两事件平台 | 54观测；3成熟核心；待定平台长2；0完成 | 否 |

W退出事件的 native_time 为 2021-01-01T00:00:01.624130Z，真正可知时刻为 2021-01-01T00:00:03.0696484Z。Q短平台封口事件的 native_time 为 2021-01-01T00:00:02.072849Z，可知时刻为 2021-01-01T00:00:03.0699408Z。两者都只在捕获前缀内判定，未用原生时间回填提前发布。

允许成交扣残量的诊断Q在e54的最大平台依次为 e1–17（17）、e18–22（5）、e23–52（30）、e53–54（2）。前三个核心均已成熟，但成熟边方向为 [-1,-1]，只有同向活动尾，没有反向成熟证据，所以完成数为0。e53–54保留为待定平台，e55第一次把它短封口；分析冻结于e54，不把新值拼回、不丢两事件平台、不重启。全部未消费事件及边界之后的原记录仍由原始文件保存，机器摘要给出未分析采集后缀范围；没有靠丢记录取得成功。

这里的“3成熟核心”不是“3完成对象”。没有三个完成对象，所以未计算或伪造严格父交集；父核心数为0。严格 Q/W 适配首段也没有完整F1/F2输出资格。

## 实际验证、复跑与绑定

独立小探针从原始快照建立 BigInt 订单表和全价位表，重放初始化43次应用，以及随后直到首个Q数学失败位置的82次原生应用；逐条核 native_sequence 连续，源行/时间一致，归一化与 Stage26 一致，实际簿变的前后报价逐字段一致。共核 125 次原生应用、82 对簿变前后报价。初始化之后82次应用包含26 open、28 canceled done、1 match、26 received、1 nonresting done；其中55条是真实簿变。line341–422 没有 catchup、snapshot 或未知状态被计入种子。整个输入20868行，但本探针只独立重建到line422，没有声称重新验证全分钟协议。

探针只用Node内置模块，价量计算用BigInt整数，未运行交易/训练/收益计算。结构函数保存0/1核心启动、开放平台、待定平台和完成列表；机器摘要列每个成熟核心的三条原生序号与捕获行种子。每次域外只冻结前一有效前缀，不把当前平台视作已完成。

复跑（工作树只读，输出到本目录）：

```sh
node --check /Users/silencehan/Documents/Codex/research-evidence/issue1467/f2-interface-v1/plateau_data_domain_probe.mjs
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/f2-interface-v1/plateau_data_domain_probe.mjs
```

当前脚本 SHA-256：`8deeb0bbfb6f4110c610542fdf4e9c4bcb632168de937cb8187186f76d6a0a95`。运行退出0；脚本内固定输入哈希与预期首次边界断言通过。结果为 [plateau-data-domain-summary.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/f2-interface-v1/plateau-data-domain-summary.json)。它保存输入、源码、探针指纹、精确参数、首个边界、冻结状态、种子来源和计数。本报告不包含订单身份或完整原行情。

| 输入 | SHA-256 |
|---|---|
| 原始full.ndjson | `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd` |
| Stage16 rows.jsonl | `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5` |
| Stage26 rows.jsonl.gz | `5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf` |
| Stage26解压字节 | `6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30` |

| 实际读取源码/文档 | SHA-256 |
|---|---|
| PlateauBaseCandidate-v2.md | `ff4e1d062e78891ff2c563ae06ba3ac842ac10885a5d275795fcd852624daa43` |
| plateau_base_probe.mjs | `0a630a6bbc418a0fa57be621ea2089123005a5b3dcd712622c81738f2a3515ac` |
| Stage16CausalViews.md | `5cc8292241275d4a580e3e05f1eeaa7cd39c513cc22c5e25a6388851a7ecc1e1` |
| Stage26NativeQuote.md | `b6f05948cbad4913831fb068f6d37f9747481ac6fe0eecfd00047d06971fdb5f` |
| Stage29NativeStateTime.md | `32dd0f8474351890f61f822de674156eeeff57f8871105a62993d65c0f8e8ce3` |
| native_quote_blocks.rs | `d06e47ed4c2bddbf5b1fdce2d1d0bac1d8dcd44cdb1f118a1642b87281b1c3f9` |
| stage26_inspect.mjs | `2dd6ec4a63e540e2483659040ab63566942dd1e9ec8d516f281903da2d8d914b` |
| stage29_native_time.mjs | `471aa843137b0443a6cefc69dc191a051b152824f3c9d1f140e11a948103251d` |

## 不能从本结果推出什么

原始日志、检查点、Stage16/26/29旧输出完整保留；没有改仓内文件、提交、推送、下载或对外发消息。Stage26曾做的恢复矩阵不是本轮新跑结果；本轮**没有执行交易所协议验收，也没有给出一般恢复证明**。raw应用规则只针对已读取的有限消息路径，经与既有账逐项核对，不声称覆盖所有场所或全部合法消息。

首次ready选点是明确的在线前缀规则，但此分钟已在先前探索中被使用，不是冻结后的确认集；任何当前分钟的失败/计数都不能当独立统计验证。没有模型训练、参数搜索、窗口扫描、资金/排队收益或市场价值主张。

当前可交付的是“数据具备可追溯的首段检验前提，具名窄域在首段缺乏三个完成对象且遭遇明确边界”，不是“原始订单数据永远不能产生候选”，也不是“新增观察轴已取得原义或生产资格”。
