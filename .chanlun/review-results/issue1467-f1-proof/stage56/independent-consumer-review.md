# Stage56 消费者与标签切分独立审查

日期：2026-10-05。绑定 #1467、#1468 与研究图 #1465。结论：限定范围内通过，未发现阻断缺陷。

本轮确认了一个实际进程消费者只通过 stdin 收到具名时点的历史载荷，以及评价器按标签封桶可知时刻处理分区边界。结果可作为 P2/P3 数据前置复用。它没有完成真实训练模型接入、操作系统权限隔离、强 B 或四臂公平验收，也不证明 F₁/F₂ 资格、预测增量或交易价值。

## 冻结输入和审查范围

审查由新建的独立上下文执行，未继承作者会话，未改作者文件。读取研究树 AGENTS.md、math-research-workflow、rigorous-open-math-research 及其 v2 verification loop。按指定路径直接读取三个新增 MJS、causal-consumer.md 和 manifest 所列必要合同、特征/标签依赖。没有另作代码发现、旧恢复矩阵、枚举、训练、主 C−B_seed 比较或下载。

研究树为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。冻结入口为其中 `.chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/author-manifest.json`，SHA256 为 `53a9d080e6dab1a01015ca0d6eedb67a73f45ae16e75a427a987832fcbdd7580`。有效作者 receipt 为仓外 `stage56/consumer-author/run-v4/receipt.json`，SHA256 为 `ce69b5eb1c026eb4cce8dfd316845d9381a753615d1b2fbd7d799db69f0dc07c`。

manifest 的 25 个文件项和 5 个固定输入项逐项核对 SHA 与字节数，全部相符。退出前再次核对。完整身份记录保存在 [frozen-identity.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/frozen-identity.json)。作者包和 Stage35/44 旧包均保持原字节。

承接的旧证据有明确边界。Stage44 独评负责已有 B 生成器及完整历史接口在 572 个固定截止上的验证；Stage35 固定 `published-mid-first-change/2` 的目标、同刻桶和 EOF 语义。本轮读取其源码和合同，复用该来源资格，不重新证明 Stage16/26 从交易所原生日志生成投影的一般正确性。新增消费者不直接承袭旧接口的“通过”状态，下面另检其序列化输出与实际进程路径。

## 实际核验

全部新证据保存在 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/`。

| 核验 | 本次独立执行结果 |
|---|---|
| 作者检查重跑 | `verify-consumer.mjs` exit 0，13 项通过；receipt 与 run-v4 字节相同，9 个列明输出的 SHA 与字节数相同，合计 3,666,147 字节，不含 receipt |
| 实际 stdin 消费 | 7 个独立 Node 进程均 exit 0；索引 0/1/2/3/250/570/571；各进程返回的载荷 SHA 与宿主发送值相同；业务数据只从 fd 0 读取，argv 没有数据源或标签路径 |
| 独立完整载荷参考 | 自写 Python，不导入作者 MJS，从原始接收字符串、发布行、原生应用记录重组上述 7 个完整载荷并独立重算其 A/B 特征；全部逐字节 SHA 相同；保存的 3 份 gzip 解压后逐字节相同 |
| 迟应用 | 从 Stage26 源数组独立识别 43 条；在可用时刻 `1609459203067142800` 的载荷中均不存在，其原接收消息已经存在；到 +1ns 则全部出现，应用数从 0 变为 44，即 1 anchor 加 43 applied |
| 两套评价切分 | Python 逐行线性寻找首个更晚 capture，重算普通和压力切分共 1,144 条记录，全部字段相同；未按结果删除原 572 个身份 |
| 合法未来对照 | 3 组短输入均为原始文件真实字节前缀；短输入保留决策之后的 3 条接收记录，水位覆盖 t；与长输入的模型载荷相同 |
| 手算边界 | 8 个定向例实际通过，覆盖变化/none 的封桶时刻等于边界、边界后移 1ns、EOF、连续性删失、初始化不可用及同刻桶内先变后回 |

[independent-check.py](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/independent-check.py) 的期望值来自源文件内容和合同运算，不读取作者已投影的历史作为参考，也不调用其截止、分桶、特征或评价函数。7 点完整载荷的构建与比较证据在 [independent-reference.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/independent-reference.json)。这增加了新适配器的独立验证，范围仍是七个固定时点。

[independent-boundary-probe.mjs](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/independent-boundary-probe.mjs) 调用实际接口，对照已独立识别的迟应用身份和手算时钟结果。结果在 [independent-boundary.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/independent-boundary.json)。其中合成输入符合发布视图合同，不声称是新重建的交易所原生流。

## 信息与时钟边界

[consumer-boundary.mjs:14](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/consumer-boundary.mjs:14) 将原接收、发布和应用历史分别投影。原接收/发布只含 `capture < t`，应用同时要求 `available_capture < t` 和 `available_line <= cutoff`。它保留全部已见原始消息，包括 pending 与已收到的 snapshot；应用中的前后簿状态按真正可用时刻出现。没有用较早 captured_at 回填 43 条迟应用，没有缩成仅一秒历史来制造 B 的信息劣势。

序列化载荷仅含 profile、产品、当前 decision、固定尺度/窗口、当前 as_of、已见起点、现有特征和四份历史。全文件 SHA、路径、总行数、最终水位、网格总长度、label、label_known_ns、loss_eligible、purge 和分区信息没有进入载荷。原消息中已收到的原生时间/序号可以保留；这些内容属于已知消息，不被当作提前发布派生状态的时钟。应用字段 disposition 是上游原生应用属性，和评价标签 disposition 分开。

新增适配器直接调用冻结的 `featuresAt`，没有读取旧检查结果来决定成功，也没有把保存的 B 当作运行时答案。独立参考确认七个时点的全部 A/B 字段与保存值一致。当前 features 仍只是既有有限 L1 压缩，完整深度/订单身份仅在历史入口中可供未来消费者使用，尚不能据此宣布强 B 已实现。

[consume-stdin.mjs:7](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/consume-stdin.mjs:7) 的业务数据入口是 `fs.readFileSync(0)`；[verify-consumer.mjs:119](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/verify-consumer.mjs:119) 确实新建子进程并传入该字符串。消费者是合同探针，不是已训练预测器。子进程仍有宿主文件权限，知道脚本位置，未采用 OS 沙盒；当前结果不能约束恶意或以后另写的消费者主动读取未来文件。作者文档已明确该限制，无越界的权限保证。

三组未来对照的短/长源行数分别为 73/20,868、7,282/20,868、20,777/20,868。Python 重新拼接原始行字节核对短源 SHA 和字节前缀关系，未改价、改序号或改应用顺序。中段与末段短输入为 EOF 删失，长输入分别确定 down/up，但模型输入相同。它们验证合法有限捕获的未来延长不影响已知前缀，仍然来自同一分钟；没有穷尽合法未来，也不是第二份独立市场样本。配套投影前缀的因果合法性依赖既有 Stage16/26 与 Stage44 合同。

## 标签可知时间和分区

[partitionLabels:43](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage56/consumer-evidence/consumer-boundary.mjs:43) 只在评价侧读取标签。对变化目标，resolved_at 是 witness 桶的时间；对 none，它是 deadline。两类都要求首个严格更晚的 capture 才形成可知时间，损失要求 `known_at < split_end`。等于 split_end 的见证不属于左分区，不能以 `resolved_at < split_end` 代替。

独立手算变化例为 0.2s 发生上移、0.3s 才封桶。左区间止于 0.3s 时排除，止于 0.300000001s 时允许。none 例的 deadline 为 1.1s，1.2s 才封闭；同样按 1.2s 而非 1.1s 判分区。EOF 未封闭、连续性删失和初始化不可用全部保留记录、不进入损失，未知没有补成 none 或零。

普通 20s/40s 切分的 572 行中有 569 行损失可用、3 行初始化不可用，没有跨界排除。压力切点 3.5s/40s 有 565 行损失可用、3 行初始化不可用、4 行跨界排除。两套结果均由独立逐行参考核实。普通切点没有触发 purge，不能把它单独作为该分支覆盖。

方向标签按产品和 witness_line 组成未来事件组，评价器检查损失可用行没有同组跨分区。三百零九个共享事件组在两套切分中均满足这一要求。作者另用奇偶交替拆行展示同一事件会被拆散；这是随机拆行风险的确定性见证，实际评价没有采用随机拆行。这不把 309 变成独立样本数，也没有解决跨日不确定性或最终实验划分。

## 缺口是否关闭

已关闭的是具名、有限的数据前置缺口：现在有真实探针进程接收仅含截止历史的独立 JSON，且与原始证据可逐字节追溯；标签切分也已有封桶知识时钟和跨边界排除的实际检查。

Stage44 所指的更广问题仍未全部消失。真实训练/推理消费者尚未接入，权限隔离未实现，训练拟合只用早期数据、共同预热与重置、历史预算、模型族和搜索预算尚未冻结。合格 C、最终强 B、未见确认集、主要 C−B_seed 比较和实质效应阈值也仍缺。因此不能把本轮称为 P3 公平因果实验已经可运行，或以有限非干涉对照宣告全域程序精化完成。

本轮没有改变教义、formal、生产、旧包或 Progress，没有提交或外发。最终证据清单、实际命令与退出码见 [receipt.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage56/consumer-review/receipt.json)。
