# Stage52 BRS-v1 独立复核

2026-10-05，#1467。新上下文审查；未继承作者会话，未使用另一源义独评的结果。本报告只审冻结作者包 v2、其完整原稿、定义、来源和可执行证据。研究草稿，不是教义裁定或生产验收。

**结论为有范围的通过。** 四条冻结历史的逐 ID 重放、全部 124 个前缀、两侧各 27 个离线分区、来源归属、RDW 点证书、后缀分叉以及限定局部类的命题 2 均通过独立核验。`ScopeCorrection-v1.md` 明确撤回原命题 1 的全域读法；按其“首个坏外离对立即停止、余下事件全留尾部”的前提，替代关系引理成立。没有新增阻断缺陷要求改动冻结包。

原主探针与第二核算器不构成全域实现，也不在全域等价。本审查另造了一条合法六事件诊断流，实际复现前者在 BRS 角色门抛断言、后者却签发片段。这个结果支持撤回的必要性。它不反驳订正后的条件关系，也不否定四条冻结流的实测结果。原义 P 完成、同级接续、Z-3、父核和固定 F2 闭合均未取得资格。

冻结入口、订正、原稿的 SHA-256 分别为：

| 文件 | SHA-256 |
|---|---|
| `frozen-review-manifest-v2.json` | `62488be22dad2717b222bdc828aa0e0217ef5166b117b9579c5c147260bdda1a` |
| `ScopeCorrection-v1.md` | `c661fc00c03bda65756e618e8fa9ff67e69ba201b040c6bf5bdf6424d5391059` |
| 原 `p-completion-candidate.md` | `e6d818c7f09b0295ad5d7ff9ca5035b5ce11ff5f5b7af65cb62e6547d134726b` |

独立计算校验了清单中全部 **18 件作者文件、6 件外部输入、6 件正本来源**，包括适用的字节数；两条 Stage50 的 `source` 与对应 Stage49 的 `source` 也逐字段相同。原冻结 CLI 使用原文件和原输入锁，输出定向到仓外，两个探针均 exit 0。产出的五件结果文件与冻结结果逐字节相同。每条实际命令、Node v22.23.1、退出码和日志见 [execution-receipts.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/execution-receipts.json)，全部哈希见 [integrity.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/integrity.json)。字节冻结证明内容未变；文件自身的“首次运算前冻结”标签不能单独证明历史时间顺序，本报告不把它当作外部时间戳。

独立重放的入口是 [independent-replay.mjs](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/independent-replay.mjs)。它没有 import 作者算法，也不把终局核目录、完成标签或作者状态数组送进重建函数。每个前缀都从初态建订单字典，逐 ID 应用新增与减量，然后重新计算买卖最优档、数量及精确有理数 `W(q)=(1000+102q)/(10+q)`。派生结构形成后才与冻结目标比较。腿用另一种构造获得：先分出连续等值块，再找相邻块之间的严格极值，取极值块末端作为腿端点，下一块首事件作为封定证据。核取游标后最早严格三交种子，返回触核时按序吸收；遇首个外离对先检角色，坏对停止，不搜索后面的成功对。

四流重建结果如下。每个前缀都核查覆盖与此前已发原始证书不变。两条主流还逐字段对拍全部订单快照、封定腿、原始核、L/R 源事件、确认支持、候选片段与尾部。两条分叉流的终局腿和核也从原消息重建后对拍，未只借用冻结的后继 outer。

| 冻结流 | 事件引用 | 含初态前缀 | 终局已封核 | 首片关系发布 | 首三片关系发布 |
|---|---:|---:|---:|---:|---:|
| `same_up_counterexample` | 40 | 41 | 9 | e7 | e15 |
| `same_down_mirror` | 40 | 41 | 9 | e7 | e15 |
| `p_completion_overlap` | 20 | 21 | 4 | e7 | e15 |
| `p_completion_disjoint` | 20 | 21 | 4 | e7 | e15 |

120 是四条冻结流的事件引用总数；124 是含初态的前缀数；2060 是 `2×(1+…+40)+2×(1+…+20)` 次从头重放操作。两算法、镜像和共享前缀不增加独立市场样本。完整中间证据保存在仓外四个 `*-all-prefixes.json` 中；[independent-summary.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/independent-summary.json) 汇总检查结果。

新分区的三片是 `(0,5]、(5,9]、(9,13]`，相邻片可共读边界状态，但不重复拥有边界事件。与旧 `0→5→8→12` 相比，只有 e9 和 e13 改变所有者。前三片视图的尾部为 e14…e40；全解析继续签发后续片段时仍保持全部事件覆盖。三个核心的成员区间分别为 `(0,4]、(5,8]、(9,12]`，其全部成员未被切开。

| 主流 | 三片 fullRange | 严格交集 H | 各片 RDW 方向集合 | 唯一交替方向词 | 点证书数 |
|---|---|---|---|---|---:|
| 正向 | `[1000,1140] / [1100,1140] / [1135,1240]` | `[1135,1140]` | `{U}/{U,D}/{U,D}` | UDU | 15 |
| 下降镜像 | `[1560,1700] / [1560,1600] / [1460,1565]` | `[1560,1565]` | `{D}/{U,D}/{U,D}` | DUD | 15 |

正向 UDU 的点对集合可以直接手核。第一片 U 是 `(0,5)…(4,5)` 五对；第二片 D 是 `(5,6)、(5,7)、(5,8)` 三对；第三片 U 只有 `(10,13)`。三者积为 15。下降镜像交换方向，事件点对相同。独立实现逐对比较 q 与 W 的不等式，且逐项核了完整证书数组。方向词唯一不推出点证书唯一、分解唯一或原义方向资格。三片均非 Std，首末值不能替代 fullRange。

两侧各 27 个阶段组合，独立以核心源事件集合与片段事件集合求交核归属，结果均为 8 个保持完整专属原核、4 个再满足严格三交、其中 3 个有交替 RDW 词。四个严格且核完整的分区是 `0→5→8→12`、`0→5→8→13`、`0→5→9→12`、`0→5→9→13`；第三个没有交替词。统一 before 无严格三交；统一 after 割入后继核心成员；统一 peak 通过。完整 54 份对照见两个 `*-partitions.json`。

**这些是完整 40 事件历史的离线对照。** 例如 K3 源状态为 B13…B16，`bornAt=17`、`sealedAt=19`。因此“B14 切入 K3”不能作为 e15 已知的排除条件。独立逐前缀重建在 e14 只有 K0/K1 两片，e15 才有 K2 的 R 封定证据并发布 `[0,5,9,13]`。当时无需 K3 目录。边界取 `L.end=R.start`，下一实例从 R 起扫描，所以后继源事件严格在该边界之后。这是当前构造的源事件关系，不是未来 K3 提供的准入许可。e13 只够读取三片范围；e15 是新原始关系的首次发布时间，旧 D* 的发布时间和新旧 `semanticEligibleAt` 仍为空。

后缀分叉也通过独立核验。两条 20 事件流的初态与 e1…e7 相同，e8 对 reservoir 分别减 90 与 40。e7 的完整 K0 证书与 `(0,5]` 片段在两边相同，片段范围 `[1080,1200]`；逐前缀到 e20，证书成员、端点、范围和可知时刻均不变。独立重建的下一核 outer 分别为 `[1130,1220]`、`[1160,1220]`。这支持同一原始证书兼容不同后继，不证明整个原义 P、后续成员或高级结构已由共同前缀决定。

原命题 1 的全域表述不能单独继续援引。订正文件第 3、11、15、34、44 行已经分别规定优先级、撤回量词、说明程序断言的有限用途、保留失败尾部分支、列出未关闭义务。原稿不改字节与撤回并不矛盾，但后续引用必须同时带 v2 入口。全称算法唯一性或普遍追加稳定性不能从四条历史的通过恢复出来。

本次新增的诊断输入有 **额外 6 个原始事件和 7 个状态**。它只测试已指出的坏 L/R 分支，未加入上述 120／124／2060，也不声称全局最短。初态为 bid `reservoir@100×100`、ask `ask@102×10`；所有新增 ID 此前不存在，所有减量均针对剩余量充足的 reservoir，最优买卖价始终为 100／102。

| 时刻 | 原始消息 | reservoir 剩余量 | 最佳买量 q |
|---|---|---:|---:|
| B0 | 初态 | 100 | 100 |
| e1 | 新增 `add-1`，bid@100，量 10 | 100 | 110 |
| e2 | `reservoir` 减 6 | 94 | 104 |
| e3 | 新增 `add-3`，bid@100，量 8 | 94 | 112 |
| e4 | `reservoir` 减 1 | 93 | 111 |
| e5 | 新增 `add-5`，bid@100，量 2 | 93 | 113 |
| e6 | `reservoir` 减 1 | 92 | 112 |

前三腿为 `100→110→104→112`，种子 core 是 `[104,110]`。紧随的 L 是 B3→B4，即 `112→111`；R 是 B4→B5，即 `111→113`，由 e6 封定。R 整个在 core 上方，但 L 从 112 开始也已在上方，且方向为 Down，不能成为所需的向上离开腿。此时冻结原始规则碰到的是首个外离对，不能绕过去寻找后来的好对。订正后的停止关系在这个输入上应无片段输出，完整尾部为 e1…e6。

另一个可手核的数值事实是，种子末腿 B2→B3，即 `104→112`，与随后 B3→B4，即 `112→111`，已经具有向上离开 `[104,110]` 后不返回的数值形状，后一腿在 e5 的 `111→113` 变化时封定。它与当前算法选到的 L/R 不是同一对。是否允许种子末腿兼任离开证据，须另查原义资格与成员规则；当前规则从末成员腿之后开始，故这项观察不能自动放宽该规则，也不构成程序修复或原义完成结论。

诊断调用边界很明确。冻结 CLI 只按锁定 fixture 运行，两条 CLI 均通过。新诊断读取原源文件，按函数边界原样抽取 `rawLegs/rawCoreSeams` 及 `signedRuns/parse`，不修改函数体；先在无 I/O 的 `vm` 中调用，再生成独立子进程 wrapper，提供必要数学助手和独立重放得到的状态。它没有声称未经修改的冻结 CLI 接受新输入。源文件 SHA-256 为 `9e9e5f8efa3dc8fef67c619b638d0ce50b6cbc1ce6c7c1f4b2bcb932a98c8485` 与 `1f869eba9015790e4ef5fb4f8a751db60c4e6ac096700fc9cd13cfddf8a52747`，抽取函数体 SHA-256 为 `97b462654789aa286ca8c12845860b555e0606eb20d2978d697ad7b82776add1` 与 `ff11369447af27a9e5606b0e40e91025f60b6950460e00f464b703eafe38bc71`。

实际主函数子进程 exit 1，错误是 `assert.ok(startsOnCoreSide)`，对应冻结 `brs-probe.mjs:80`，并非适配形状错误。第二函数子进程 exit 0，输出一片 `K0: (0,4]`、`qRange=[100,112]`、`relationshipPublishedAt=6`，因为其 `parse` 没有检查离开角色。冻结包只把第二算法用于四条历史的复算，这个局限已被订正文承认；本审查不要求把它改成通用实现。

完整输入、逐 ID 快照、W、结果、命令和两套 stdout/stderr 分别在 [diagnostic-input.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/diagnostic-input.json)、[failure-branch-diagnostic.mjs](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/failure-branch-diagnostic.mjs)、[failure-branch-diagnostic.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/failure-branch-diagnostic.json)、[diagnostic-main.stderr](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/diagnostic-main.stderr)、[diagnostic-prefix.stdout](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/diagnostic-prefix.stdout)。主函数 stdout 与第二函数 stderr 均为空，空文件也保留。此例是原始算法适用性诊断，不是缠师原义命题的反例。

替代关系引理可独立证明，但必须保留停止分支。给定 h≼h′，已封腿的首反向事件已经发生，因此腿端点、源成员、范围和封定时刻不会改变。固定游标后最早的严格三交种子由这些已封腿决定；在它之前未满足的完整三腿窗口也不会被追加改变。随后按固定次序检查 L/R。返回触核则吸收，未封齐则停止；遇首外离对时先检 BRS 角色及完整成员门。门失败则停止并保留自上次边界起的全部尾部，**不消费坏对、不改种子、不跳过坏对求下一个成功对**。门通过才签发 `L.end` 并从 R 继续。

这个递推的每一步由已封证据唯一决定。当前缀 h 已签发一个片段时，它的种子、历次吸收、首外离对及成员门都已确定；h′ 不能改变这些有限前提。对已发片段数归纳，之前所有片段的成员、端点、范围和原始可知时刻不变。边界严格递增，事件所有权取开左闭右，尾部取末边界之后的全部原事件，故覆盖无漏无重。这里证明的是明写停止关系的确定性与已发原始证书稳定性，程序是否在所有输入上实现该关系是另一个尚未关闭的问题。

订正第 38 行“首个满足所需角色”的措辞只有与第 32、34 行合读才成立。它应理解为“首个外离对已经通过角色门”；若解释为“跳过失败的外离对，继续找首个合格对”，则更换了冻结停止规则，本报告不认可该读法。以上证明补出了该处省略的分支，没有让坏 L/R 获得放行。该文字点不影响明写停止前提下的结论，也不改变已冻结结果。

命题 2 的一般性范围成立。它的输入是具名七角色点的有序局部视图，并保留种子方向及离开侧；其余原事件、ID、时间、队列、幅度、前界、epoch 身份及前史均不在输入内。正向 K0/K2 的索引分别为 `[0,2,3,4,5,6,7]`、`[9,10,11,12,13,14,15]`，共同秩型为 `[3,0,2,1,6,4,5]`。七结点严格递增映射为 `1000→1135, 1010→1136, 1030→1138, 1040→1140, 1100→1200, 1130→1230, 1140→1240`，每段可作正斜率线性延拓。下降镜像也独立核验了对应秩型和严格递增映射。

设 f 是这个视图上的单值角色选择器，对严格递增坐标变换等变。上述映射把每个具名点映到另一视图的同角色点，故 f 的角色选择相同。D* 却要求 K0 选第五角色 peak、K2 选第四角色 before，矛盾。两输入足以见证这个要求不是该类函数；没有事件长度最小性结论。K0 的 e1 是真实深档新增，q 不变；K2 没有对应停顿。只有明确按腿内等值停顿作商化之后，二者才属于相同输入类。允许读取 e1、原始订单消息、根片身份或其他全局信息的 F1 不属于该类，未被排除。W 的导数为 `20/(q+10)^2>0`，在这里的正数量域保持次序；q 上的秩型核验没有偷偷引入价格距离不变量。

来源校验逐行比较了摘录中的 163 行与六份原文件，哈希及正文界见 [source-excerpt-integrity.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/source-excerpt-integrity.json)。机器逐字比较之外，本审查回读了作者标记和行内注。

| 来源 | 归属核验与本次允许的用途 |
|---|---|
| [018:24、26、64](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/018-第18课.md:24) | 正文界 68 前。24 行的娇注与 60 行注不作为作者依据；26 的单核分类以完成为前提，64 讲核破坏，不能直接给任意含核片段的完成端点。 |
| [035:14…24](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/035-第35课.md:14) | 作者署名 12，正文界 38；所用递归与结合运算段无编注。它们不直接认证这里的原始腿。 |
| [038:290…300](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/038-第38课.md:290) | 正文界后的答疑，290 为作者署名，292 为时间；300 等号前是提问，等号后是作者答复。仅后者支持“分界须为前段结束点”。 |
| [039:26…30](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/039-第39课.md:26) | 正文界 48 前；14、16、32…40 的编注不援引，30 行内娇注单独排除。28 的讨论以 a、Ai 已有同级角色及奇偶方向为前提。 |
| [084:52、54、56](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/084-第84课.md:52) | 正文界 68 前，相关三行无编注。许可研究其他 F1；不自动证明当前候选满足唯一性与固定 F2 义务。 |
| [ADR0011:74、78、83、92](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/adr/0011-operation-decomposition-layer.md:74) | 当前项目架构裁定，与缠师原文分开。没有把“旁路不回流主干”升格成所有订单 F1 的数学不可能定理。 |

039 的有限角色对照复算相同。将最初三片作 A1…A3，B0 前没有具名 a；将 X0 作 a 后的第一组为 cuts `[0,5,9,13,17]`，四范围为 `[1000,1140]/[1100,1140]/[1135,1240]/[1200,1240]`，A3 不返回，Ai 三交为空。全部九片形成六个连续四片窗口；每侧六个均数值命中严格不返回分支，其中两组 Ai 同时有严格局部三交。所有窗口的完成、同级和方向角色都为空，合格 039 应用数为 0。039 的不返回支在角色合格时给条件重组 a′；这里没有这些角色，不能签发 a′ 完成，也不能把“数值不返回与局部三交同现”当作原文反例。RDW 方向词不能代填 Ai 的原义方向。

可接受的结果是四条冻结流上的原始证据重建、这份条件停止关系的普通证明、限定七点类的等变障碍，以及一个新发现的坏 L/R 诊断见证。原义资格边界保持空置。本轮没有修改冻结件、历史稿、Progress、教义、formal 或生产代码，没有提交、外发或触发全仓测试；全部新增运行证据只在指定仓外目录，本文件是唯一仓内新增。

复现独立检查可运行以下两条命令；第一条要求本目录中的作者复跑结果仍在，相关原样 CLI 命令已完整记录在 `execution-receipts.json`。

```bash
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/independent-replay.mjs
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage52/brs-review/failure-branch-diagnostic.mjs
```
