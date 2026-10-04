# Stage56：按时点交付的消费者边界与标签封桶切分

绑定 #1467/#1468、研究图 #1465；P3 前置工作草稿。没有取得 P1 资格，没有运行 B_seed/C 主确认、拟合、调参、收益比较或新行情采集。

本轮增加一个已运行的单时点序列化接口：可信宿主读取既有文件，只把严格截止前的特征和全部已见历史交给消费者；标签与分区排除结果留在评估侧。另找到一个具体切分风险：`resolved_at_ns` 是标签变化桶的时刻，**不能直接当标签可知时刻**。新增评价器按更晚 capture 的封桶见证排除跨界标签。13 项作者检查通过，独立审查仍待主控另行验收。

## 承接证据与实际边界

依据是 `GoalReframe-v4.md`、`Stage35ExternalTarget.md`、`Stage44BaseCompletionAndB.md`、`BookGridB-v1.md`、`ExperimentDesignCandidate-v1.md` 与 `stage44/independent-book-grid-b-review.md`。截至本轮只读搜索，没有更晚替代 B 或标签接口的独评。代码图无本工作树索引，随后读取这些指定源码。

Stage44 的 `historyAt` 已通过 572 截止独评，但调用者仍持有 `input/native/raw` 的完整源对象、全文件路径和 manifest。其说明与独评都明确没有训练消费者的强制权限隔离。本轮复用它已验证的投影、特征和固定样本，仅新增 [consumer-boundary.mjs](consumer-evidence/consumer-boundary.mjs)、[consume-stdin.mjs](consumer-evidence/consume-stdin.mjs) 与 [verify-consumer.mjs](consumer-evidence/verify-consumer.mjs)。不重跑旧恢复矩阵或 B 全网格导出。

`packetAt(input,native,raw,t)` 是**可信宿主** API；消费者既不取得该函数，也不取得其参数。其返回值是独立 JSON 字符串，实际新进程消费者只从 stdin 读一个字符串，argv 不携带数据源或标签路径。可信宿主检查输入 SHA 与既有投影合同，不接受任意未审来源替代已固定文件。

| 模型侧载荷 | 截止或来源 |
|---|---|
| product、decision、schema、尺度、100ms 网格与1秒窗口常量 | 当前决策身份与事先已知协议 |
| as_of | `capture < t` 的已见末事件序号；无全文件行数 |
| initialization | 已见历史起点；空态开始、仅收到快照后才可用；无起点前补历史 |
| features | Stage44 固定 A 五项与有限 B 全字段，逐字段白名单 |
| receipts | 截止前全部原始消息，包括 pending 与已收到的完整 snapshot；保持原始字符串精度 |
| publications | 截止前全部发布行和状态、barrier、价量，保留桶内行路径 |
| applications | `available_capture < t` 且 `available_line <= cutoff`；保留应用顺序和原接收序号，不按较早 captured_at 回填 |
| trades | 截止前全部已见成交 |

模型侧不含标签、标签可知时点、loss_eligible、purge、分区、未来状态、全文件 SHA/路径/大小/末行/总数/最大时间、总网格长度。完整原始消息中的原生时间及序号属于当时已收到内容，允许保留；原生时间不用于提前发布派生状态。当前前缀长度是当时已知的历史，不是未来总长度。native quote 的固定尺度在协议常量中给出，源位置由已见 event_seq 表达，不传完整文件地址。所有源身份与产物 SHA 留在宿主证据收据中。

这里保证的是具名 API 的信息契约与实际 stdin 路径。Node 子进程仍与宿主同权限，能够自行调用文件系统；未实现 OS 沙盒、敌意代码防护、文件权限隔离或任意模型训练框架接入。进程调度耗时等侧信道也不在保证内。不能把此结果写成恶意消费者绝不可能读取未来文件。

## 新增验证结果

现役结果目录为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-author/run-v4/`。

| 检查 | 实测范围与结果 |
|---|---|
| 共同身份 | 保存的 B 与 profile/2 标签全部 572 行逐行一致；不按结果选择决策 |
| 时点载荷和真实 stdin | 网格索引 0/1/2/3/250/570/571 共7点，涵盖初始化前、首次应用后、历史中段、末端；各新进程退出0，接收字节 SHA 与宿主一致 |
| 原字段与路径 | 7点 A/B 字段逐项等于保存 B；已见接收/发布序号、应用身份和成交路径均与严格时钟过滤一致 |
| 初始化 | 最初70条接收记录没有未来 anchor/snapshot；第0点确实还无成交，第1/2点盘口未初始化但已见成交保留 |
| 迟应用 | 已有43条逐条在其 available_capture 时点被排除；较早已见接收消息仍可存在 |
| API 额外边界 | JSON 修改不回写源；宿主附带未来元信息不复制入载荷；起点与水位外请求拒绝 |
| 时间切分 | 572行评价记录保留，未知目标不进损失；全部方向标签按未来事件分组校验，不允许同组跨损失分区 |

只保存3个完整模型载荷 gzip，其余保存进程收据、身份与 SHA。当前有效输出（不含 receipt）共 3,666,147 字节；没有物化572份完整历史。Stage44 全572历史截止的已审结论仍按原独评复用；本轮不冒称7点等于新做了一次全域消费者证明。

## 有效协议域内的后缀非干涉

使用原始真实接收文件的**字节前缀**，配套取 Stage16/26 同位置的前缀，不修改价格、序号、载荷、原应用顺序或历史初始化。两侧都保留决策之后的非空后缀；短文件是合法有限捕获，EOF不封闭末桶。若系统把完整输入的末时间/行数/SHA泄给消费者，这些对照会不同。

| 决策 t | 短 / 长文件行数 | 短 / 长标签 | 模型载荷 |
|---|---:|---|---|
| 00:00:02.8 | 73 / 20,868 | 不可用 / 不可用 | 逐字节相同，无未来快照 |
| 00:00:27.8 | 7,282 / 20,868 | EOF删失 / down | 逐字节相同 |
| 00:00:59.8 | 20,777 / 20,868 | EOF删失 / up | 逐字节相同 |

均为2021-01-01 UTC，同一BTC-USD样本。原始文件 SHA、短前缀 SHA、两侧末水位及模型字节 SHA 只在 `suffix-comparisons.json` 评价证据中保存。该对照改变的是合法未来观察长度与后续标签可决定性，不用非法改写未来簿来冒充反例；它不是两个独立市场样本，也没有声称穷尽所有可能未来。

一般的条件性理由是：在同一可信、已校验协议下，若两输入截止前原始接收/发布/应用/成交历史相同，且两者都有 `watermark >= t`，`packetAt` 的各白名单字段只取这些前缀或固定常量；`featuresAt` 也按同一截止计算，JSON确定性序列化因而相同。该理由依赖已有特征与投影合同，有限实测不替代任意上游实现正确性证明。

## 标签切分的新缺口和最小反例

`order_label_probe.mjs` 的 observed_change 把变化桶 `next.t` 写入 `resolved_at_ns`；只有 `next.t < lastT` 才准许决定方向。即使 `resolved_at_ns < partition_end`，使它可知的下一采集桶仍可能在分区外。

`seal-counterexample.json` 保存完整有效 causal-view 合成输入：0.0s与0.1s报价不变，0.2s报价上移，0.3s才提供封桶见证。决策0.2s只消费前两桶；标签为 up、resolved_at=0.2s。若训练区间为 `[0.0s,0.3s)`，只检查 resolved_at 会错误留下该行；本轮 evaluator 算 `label_known_ns=0.3s` 并排除。此例验证发布视图合同，不声称是重新构建的交易所原生流。

`partitionLabels` 只在评价侧运行：方向目标取 witness 桶后首个更晚 capture；none目标取 deadline 后首个更晚 capture；没有见证或 censored/ineligible 不进损失。损失使用要求 `label_known_ns < partition_end`，等于边界也排除。标签或排除标志均不送模型，不能声称这个未来排除标志当时已知。

对原572网格的工程切分结果：

| 切点 / 分区 | 全部行 | 可入该分区损失的已知标签 | 跨界排除 |
|---|---:|---:|---:|
| 20s、40s：early | 172 | 169 | 0 |
| 同上：middle / late | 各200 | 各200 | 0 |
| 压力切点3.5s、40s：early | 7 | 0 | 4 |
| 同上：middle / late | 365 / 200 | 365 / 200 | 0 |

early 的另3行属于初始化不可用。普通切点碰巧没有跨界目标，不能宣称它覆盖了 purge 分支；压力切点明确落在 Stage35 已知的第一个共享未来事件组内，用于工程边界检查。checker 还以交替分配行的朴素“随机拆行式”分配见证同一未来事件被拆开，而按可知时点purge后的损失组不跨分区。这不把309个事件变成统计独立样本，也不规定最终训练/确认划分。

## 复现、冻结与剩余责任

在研究工作树运行（输出目录必须不存在）：

```sh
node .chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/verify-consumer.mjs \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467 \
  /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-author/REPRO_NEW
```

`run-v4/receipt.json` 保存输入身份、源码和合同 SHA、13项检查名称、各原始输出 SHA 与复跑命令。外层 `run-v4.stdout.json` 和 `run-v4.stderr.txt` 保留实际输出，exit=0。`consumer-evidence/author-manifest.json` 冻结最小作者材料及这些产物，不包含动态 Progress。早期作者检查两次失败：误把第0点当作“已有成交”，及误以为20/40s切分必定跨界；按保存数据订正断言并另列压力切点，未修改旧市场数据、旧B或标签。失败尝试与代码在仓外保留。第三次通过后将未知目标单测换成协议自身生成的合法EOF删失前缀，第四次13项通过。

本轮消除的是“尚无实际消费者只接收截止载荷”的具名 API 前置缺口，并补上切分必须按标签封桶可知时间的机械检查。它不建立最终强 B、四臂信息公平、共同模型预算、完整 F₁/F₂、合格 C 或未见确认集。实际模型的预热、缺口后状态重置、训练拟合只看早期数据及最终搜索预算仍须具名冻结；本工具只保留原历史和已知状态，并未替真实模型决定这些合同。
