# Stage9：有类型订单事件、成交与恢复的数据条件

日期：2026-10-03。命题 `DATA-READY-CB-20210101-1M-v1`，服务于 #1467 的事件域和 #1468 的实验备料。承接 [开工卡](Stage9WorkCard.md)及 [v4 协议](GoalReframe-v4.md)。名分：固定探索样本的协议/程序证据，不是一般恢复定理、候选缠论资格或经济效应。

## 结论

已取得 Coinbase/BTC-USD 的一个历史分钟：同一文件含订单新增、撤销/结束、改量、执行及 L3 初始快照。按具名协议连续重建后，四个持久化检查点的新进程恢复得到相同最终状态；三种扰动均按预期拒绝或保留缺口。可进入带来源和可用时钟的探索性输入准备，原先六列报价缺少事件原因和同步成交的限制在此样本上得到解除。

尚未导出逐决策时点的可用状态轨迹，尚缺独立终态快照或独立语义审查。主确认实验仍未开跑，最终市场、用途及参数未选定。不能据这一分钟认定整个数据源完整，也不能认为 F₁/F₂、预测或交易价值已通过。

## 来源与固定输入

[Tardis HTTP API](https://docs.tardis.dev/api/http-api-reference)说明，无 API key 可访问每月首日历史；每行带采集时间和原生消息，空行表示采集断线，分钟切片依据采集时间。本轮使用既定 `2021-01-01, offset=0, sliceSize=1`，无 Authorization，HTTP 200；没有扩大下载范围。

[Coinbase 数据说明](https://docs.tardis.dev/historical-data-details/coinbase)将 `full_snapshot` 标为由 REST 生成的 L3 快照，序号重叠验证自 2020-06-11 开始。选择该日期用于协议探针，不据此决定最终市场。[交易所 full channel 文档](https://docs.cdp.coinbase.com/exchange/websocket-feed/channels#full-channel)要求先缓存消息，取得快照后丢弃书簿应用中序号不大于快照的消息，再顺序应用余下更新。`received` 不代表已挂入，未上簿的 `done/change` 不改簿，`match.side` 是 maker 侧。

原计划的 LOBSTER 旧样本入口本轮没有取得文件；当前首页样本申请需要购买凭证核验，未申请或购买。这个访问结果不证明所有历史免费样本都已撤销。Browserbase 免费时长不足，本机隔离浏览器完成首页核对；两会话此前已停止，没有成功远程会话的回放链接。

| 输入项目 | 本轮实测 |
|---|---|
| 压缩/解压大小 | 2,107,081 / 8,401,206 字节 |
| 消息 | received 6,952；open 6,782；done 6,925；match 205；change 3；full_snapshot 1 |
| 总记录/断线空行 | 20,868 / 0 |
| 原生 WS 序号 | 19,247,051,251–19,247,072,117，共 20,867 个；本样本连续、无重复、无逆序 |
| 采集时间覆盖 | 00:00:02.7706482–00:00:59.9994431 UTC；不补写此前空窗 |
| 快照 | 第 340 行，序号 19,247,051,546；初始 bids 40,723、asks 8,443 个订单 |
| NDJSON SHA256 | `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd` |
| gzip SHA256 | `ce86aa40c9fdc387ed5ed86483cc3139ef7695dcfa8fd29f8a98bfd650daa66a` |

原始文件、检查点和扰动副本只保存在仓外 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/`。研究仓保存程序、摘要、请求参数和哈希，不上传原消息、订单标识或 `client_oid`。下载回执是本轮实际请求证据，不保证未来同 URL 的内容或访问条件不变。

## 实际验证了什么

[coinbase_sample_audit.rs](coinbase_sample_audit.rs)使用精确的 10⁻⁸ 整数价量及纳秒时间比较，按序号缓存并应用消息，检查 maker 数量/价格/方向及已上簿订单的变更。当前 profile 针对该历史形状，遇到含 `new_price` 的现代修改消息明确拒绝；没有宣称适配全部 Coinbase 协议。

收到快照时，已有 339 个待处理消息。其中 296 个被快照覆盖，余下 43 个在第 340 行、采集时刻 `00:00:03.0671428` 才能补应用，frontier 到 19,247,051,589。这 43 个含 received 16、open 15、done 12。快照本身没有交易所事件时间；不能倒填为其请求或生成之前已经知道。该采集时钟属于供应商观测，不是本地未来交易系统的接收时钟。

快照之后累计应用 match 201，另有 4 笔原始成交位于快照覆盖范围，保留为成交观察但不重复扣书簿；忽略未上簿 done 229、change 1。最终有 49,188 个订单；从最终订单表重新聚合价位数量，与运行时聚合表一致。这是**内部状态一致性**，报告字段 `independent_aggregation_matches` 不代表第二个交易所终态或独立审查者。

逐事件应用统计共有 13,489 次书簿变更：其中 12,215 次最佳买卖价均未改变，406 次同时改变了顶层数量。**406 包含 match 引起的变化，不能称作 406 次纯挂撤。**由于应用 match 总共只有 201 次，可以推出其中至少 `406−201=205` 次来自非 match 事件；这是计数下界，不是精确分类计数。这些也是补应用层的统计，不能直接视作 406 个可执行的实时决策时点。

| 检查 | 实际结果 | 结论范围 |
|---|---|---|
| 从第 0 行检查点恢复 | exit 0，最终状态一致 | 当前文件、当前程序、该切分 |
| 从快照前第 339 行恢复 | exit 0，最终状态一致 | 待处理消息随状态持久化 |
| 从快照后第 340 行恢复 | exit 0，最终状态一致 | 快照与补应用之后的状态 |
| 从第 10,000 行恢复 | exit 0，最终状态一致 | 中途状态切分 |
| 旧检查点配被修改源文件 | exit 1，`checkpoint/source binding mismatch`；无通过报告 | 源文件 SHA 绑定有效 |
| 原第 501 行后插入同序号、反转 side 的重复 | 第 502 行 exit 1，`same sequence with conflicting payload` | 冲突重复不能覆盖原消息 |
| 删除原第 1,001 行 received | exit 1，gap=1，frontier 停在 19,247,052,249，19,867 个消息留在 pending，readiness=false | 缺口之后不静默顺排应用，非书簿变更消息也不跳序 |

恢复比较覆盖报告全部字段，仅排除本来就应不同的 `restored_from_line`；最终完整状态 SHA256 均为 `c17603a9b18a1d885d580663ff360560ebdcfc451f09700963af334687d2b1cf`。这里是一份状态序列化的哈希，不是仅比较订单数量。原文件哈希在扰动之后保持不变。

最初独立执行连续重建、四次恢复和三次负控制，随后将同样步骤封装成 [stage9_verify.mjs](stage9_verify.mjs)，在新证据目录实际跑通一次，验证脚本自身可复跑；没有改变审计器以使结果通过。完整聚合回执见 [stage9-evidence.json](stage9-evidence.json)，包含代码、二进制、输入与运行器哈希。所有检查都是作者执行的程序证据，独立语义审查仍未完成。

## 复现

需已有固定输入、Cargo 锁定依赖及 Node.js。当前实测 Node `v22.23.1`。脚本无网络请求，不自动取数；输出目录必须是仓外尚不存在的新目录。每个子进程上限 45 秒，超时记失败而不当作反例。

从研究工作树根目录执行（`stage9-reproduction-2` 必须尚不存在）：

```sh
cargo build --offline --locked --manifest-path .chanlun/review-results/issue1467-f1-proof/stage9-audit/Cargo.toml --target-dir /tmp/nc1467-cargo-target
node .chanlun/review-results/issue1467-f1-proof/stage9_verify.mjs \
  /tmp/nc1467-cargo-target/debug/order-event-sample-audit \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/coinbase-20210101-0000-btcusd-full.ndjson \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/data-readiness-v1/stage9-reproduction-2
```

期望验证脚本 exit 0，摘要 `continuous=1, recovery=4, negative=3, all_passed=true`；三项负控制的被测程序应 exit 1。失败时保留目录和回执，不在原目录覆盖重试。最初构建因 `json!` 报告超过 Rust 默认展开深度失败一次，增加 `recursion_limit=256` 后通过；这属于构建问题，不是市场反例。

## 对研究主线的推进与未决项

| 候选/命题 | 本样本支持 | 仍不支持 |
|---|---|---|
| R_W 价量观察路径 | 可从具名有序事件重建 L1，观察固定报价时的数量变化 | 逐决策状态导出、F₁真实路径接线及语义资格尚未完成 |
| R_D 直接订单演化 | 新增、撤销/结束、改量、执行有类型和订单引用，能检查候选所需字段 | 不能据字段齐全推出多变量 F₁/F₂ 合法，快照身份也不提供原始队列优先级 |
| A/B/B_seed/C | 同一场所的 205 笔记录成交可作为 A 数据来源；订单流可备 B | 四臂仍需共同可见域、决策时点、候选资格、标签、划分和预算冻结；一分钟已看数据不能成为确认集 |
| P2 | 四个实际切分恢复一致，三项负控制有效 | 任意切分的一般证明、程序全域精化、崩溃写入原子性、断线后再同步、合法乱序/重发的完整资格均未获得 |
| P4 | 实证输入需保留原生序号、快照覆盖关系、未知/待处理状态和到达时钟 | 不据这次离线运行推断生产吞吐、排队成交、净收益或硬件必要性 |

进入价值实验前，须另写逐输入前缀的输出契约：消息在采集时刻到达并完成本次补应用后，才导出该时刻可用的状态；补应用中间态不得伪装成此前实时可知，必须保留 unknown/gap 标记和共同决策域。当前程序只记录相关计数与检查点，没有输出完整的逐决策轨迹。检查点用整文件哈希作离线来源绑定，不能据此宣称已实现在线流恢复或抗检查点篡改。

本轮不再扩下载或调参。下一数学工作仍按 v4 推进保留成员、同级与生命周期的 F₂ 组装关系；数据结论解除的是部分实验备料障碍，不替代这一语义义务。#1467/#1468 及活动 goal 保持开放。
