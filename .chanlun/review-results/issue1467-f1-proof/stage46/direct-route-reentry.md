# #1467 Stage46：直接压力块的 L1 外缘接续与来源边界

2026-10-04。研究工作草稿，只读基线 HEAD `84ddc99529d5f2c5e043d0bf3c4ae5539a82e4f8`，目录 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。范围为既有 `R_D-Q-moving-v1` 与 pressure/DC 桥。本文没有把平台 RT 改名、没有改仓内文件，也没有重放市场分钟或旧恢复矩阵。

本轮新增了一个一般 Lean 结果：**原 MovingQuote 同号合法 Flow 的全部源切点 L1 报价区间，其精确外包络就是原 leg 的两端范围。** 因此，Stage45 的“加入进入前状态后外缘扩大”不能机械地迁移为这条直接路线的新反例。另一方面，两事件合法见证证明：**一个已确认直接压力块可以拥有价位 90 的新增事件和真实深层买量，而选边 leg 的范围仅为 [100,102]。** 若把 leg 的范围当成全簿价位或全部事件触价的外缘，该命题失败。

这解除了一项具名 L1 支撑合同内的外缘疑问，没有补出最低中枢/走势的原义身份，也没有补出 EXT-own 或完整 F₂。新根为作者加机器检查，独立语义审查待办。

## 1. F₁ 当前结束在哪一层

下列相对源码位置均以只读研究包 `.chanlun/review-results/issue1467-f1-proof/` 为根。图工具返回该研究树未索引，随后沿指定文件与 import 读取原声明；没有用名字推测语义。

| 层 | 原声明实际保证的对象 | 仍不保证 |
|---|---|---|
| 原始来源与分解 | `MovingQuoteSpec.lean:20–38` 的独立 Runs 关系；任意事件列表恰有一个极大同号分解，完整 Event 含 side/action/amount/price，可全量 decode | 同号就是成交价格方向；块已经是完整走势 |
| 簿与报价 | `MovingQuoteSpec.lean:48–69` 的 Update/Step/QuoteOf/Flow。Stage24 `BookQuoteSpec.lean:18–33` 在同初始全价位数量、同前缀且两条轨迹均合法时证明报价及 construct 确定 | 任意消息流存在合法轨迹；列表排列或订单身份唯一；未知簿等于真实无报价 |
| 可空几何 | `MovingQuoteSpec.lean:71–93` 的 leg/readout；正块前 bid→末 ask，负块前 ask→末 bid；同边界反号块端点相接。`MovingQuoteWireSpec.lean:9–23` 组装输出并保留来源 | construct 自动验证全部 Step/QuoteOf；无几何块可被过滤后跨接；完整 F₂ 输入资格 |
| 符号块生命周期 | `DirectQuoteSpec.lean:67–71` 的 Completed 仅检查极大同号区间和已见反号后继。`LocatedSpec.lean:21–35` 把真实 locate 与这个独立关系相接，并证明共同前缀下确认不变 | 原义走势已经完成、同级趋势已经成立、背驰或小转大已有来源证书 |
| 表示对应 | `PressureDCSpec.lean:59–73` 与 `PressureDCKernelSpec.lean:35–40`；全程有效双边整数报价下，具名选边路径、δ=1、同价留末恢复原符号、边界、确认字段及 leg 公式 | 旧同价留早 DC-v0 可替换；完整 payload/原生适配/Rust/物理时钟精化；全信息等价 |

具体地，`PressureDC.TRACE_TARGET` 的结果是 `observedWord=eventWord ∧ dcViews=actualViews`，`GEOMETRY_TARGET` 是两个端点公式的恒等式。根没有独立原义 Move 载体、同级关系、真实核心归属或完整结束关系。因此“表示可共用”是准确继承范围；不能把它解释成直接路线的原义 F₁ 或 F₂ 已完成。

Stage23、24、25、28、30、33、37 的阶段包记录了机器检查，相关目标的独立语义审查仍为 `not_reviewed`。Stage26 是有限工程回放、自检和恢复证据。Stage41 的独评明确只绑定 `ReferenceBase.exact_root` 的固定见证、原文范围、平台候选与 A 臂，不给前述全部阶段盖章。Stage45 的 EXT 与 P/E 独评可继承其绑定合同和条件结论，不能继承为 MovingQuote/PressureDC 的语义通过。本次重新编译了所实际使用的 MovingQuote/DirectQuote 研究依赖，仍不冒称它们已有独评。

## 2. 本轮冻结并实际检查的关系

声明：[DirectHullSpec.lean](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/project/DirectHullSpec.lean)。证明：[DirectHullProof.lean](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/project/DirectHullProof.lean)。

固定原关系 `Flow d a q b r es`。定义 `Support(d,a,q,b,r,es,x)`：存在原事件切分 `es=pre++post`、真实中间簿 `mid` 和有效报价 `v`，使原 Flow 分别连接前后两段，而且 `v.bid≤x≤v.ask`。这份独立支撑关系不调用 leg、segLow、segHigh，也不以程序输出定义自己。它保留事件、簿与中间状态，不把来源集合缩成去重价格。

此处的区间是整数报价轴上的 L1 spread 几何支撑；区间内价格不因此成为已发生过的成交或事件。Support 允许所有与原首末条件相容的合法切分轨迹，未声称内部簿列表完全相同。实际 Flow 的每个源切点均在该关系内；其 extrema 仍由同一端点取得。原事件所有权与引用状态分开，读取块前状态不要求重复拥有前块事件。

精确根包含三项目标：

1. `SPLIT_TARGET`：任意原 Flow、任意 `es=pre++post`，都存在中间簿与报价，原 Flow 可以在该源切点分开。这排除了只检查首末状态、却漏掉内部来源的空泛包络。
2. `HULL_TARGET`：任意原 Flow，`segLow(leg)`、`segHigh(leg)` 都真实属于 Support，并且所有 Support 值位于两者之间。正块下界为首 bid，上界为末 ask；负块下界为末 bid，上界为首 ask。证明消费原 `flow_monotone`，同时保留两端取到性，不只证明一个松上界。
3. `LIMIT_TARGET`：以下两事件非空见证满足实际 Step、QuoteOf、ValidBook、Flow、Completed 和原 locate 时间；真实源事件价在 leg 范围外。

一般证明只需原 Flow 的全程有效报价与同号前提，不要求这个块已经 Completed。因而活动同号尾也能使用当时首末报价计算当前 L1 包络；这不固定其未来端点。空 Flow 同样有状态区间包络，但不因此取得非空 Segment 几何或任何完成资格。无报价/锁定/交叉/缺口不在本 Flow 定理的域内。

## 3. 两事件反例可直接复算

| 时点 | 事件 | 买侧价位单位列表 | 卖侧价位单位列表 | 有效最优报价 |
|---|---|---|---|---|
| 0 | 初始 | [100] | [102] | 100/102 |
| 1 | 买侧新增 90 一单位，正压力 | [90,100] | [102] | 100/102 |
| 2 | 卖侧新增 110 一单位，负压力 | [90,100] | [110,102] | 100/102 |

第一块拥有事件 `[0,1)`，在已见事件数 2 时被反号事件确认；原 locate 的 knownAt 为 `[some 2, none]`。第一块 leg 为 100→102，L1 精确外缘为 [100,102]。但其自有事件价格 90 及块后真实买量 90 小于下界。第二个反号事件只是完成支持，不归第一块所有；无需把 110 塞进第一块来制造反例。

被否定命题是：“原直接块的 leg 区间包住其所有源事件触价，或包住原簿的全部深度价位。”它不否定 L1 支撑定理，不否定原 Runs 来源无损，也不证明订单递归总体不可能。旧 Stage15 已否定把供需压力直接认作原触价方向；本例是范围包含方面的限定核验，不能把同一类旧失败再计为新的整体不可行性结果。本轮新增承重处是一般 SPLIT/HULL 关系及其精确适用边界。

## 4. 对 Stage45 的 source / extent / life 与 EXT 的影响

| 项目 | 本次可以落定 | 尚欠条件 |
|---|---|---|
| source | Runs 保存的自有事件与用于确认的首反号事件不同；L1 支撑读取源切点的两边报价 | 哪些压力块/波动取得真实中枢成员角色、哪些是核间连接 |
| extent | 对完整合法压力块，全程 L1 支撑外包络无需扩大 leg；把块前状态纳入也不遗漏新极值 | 若外缘改用全簿或全部触价，现 leg 不够，必须另立支撑合同和候选版本 |
| life | Completed 是原符号块封定；firstKnown=s+1，knownAt=e+1，EOF 无确认权 | 原义同级走势结束、确认、上递归准入不能由此推出 |
| EXT-rel | 已具资格的成员若采用本 L1 合同，可使用准确块范围计算局部外缘；原 Stage33 数值中心关系不因“补入前报价”自动变化 | 全目录的同级核身份与真实核外缘成员需先独立确定 |
| EXT-own | 本根没有把符号块重命名为同级完成走势 | 独立语义载体 S、固定对象映射 φ、同级同型、真实完成、核心归属逐项相等、结束点相等均仍待证 |

Stage45 的 E 结果固定“每个成熟最大常值平台一核”，把前后标量状态的区间归入该平台。这里固定的却是原 MovingQuote 极大压力块，两个报价边已在块内单调，所以包络机制不同。两者不能互换。反过来，本 L1 包络定理也没有证明 Stage45 的 J 对 R_D 成立或不成立；它压根没有对象 X 的核心所有权前提。

若把多个已确定成员块的 L1 支撑并起来，普通有限极值运算允许从这些块的准确范围计算总体外缘。这是几何合成事实，不确定成员归属、同级身份或完整走势生命周期；不能作为 EXT-own 的替代证明。

## 5. 机器证据、停止线与下一精确目标

运行 `a7166b7d233147b88226a6c5effc948c`，Lean 4.31，exit 0。精确声明为：

```lean
DirectHull.exact_root :
  DirectHull.SPLIT_TARGET ∧ DirectHull.HULL_TARGET ∧ DirectHull.LIMIT_TARGET
```

公理闭包只有 `propext`、`Classical.choice`、`Quot.sound`；无 sorryAx、自定公理、未知或 unsafe 依赖。7 个项目内模块重新编译；正式 Origin 依赖使用已存在、被工具记录哈希的编译产物，没有重建或修改 formal。声明 SHA-256 为 `14f4d869525d1c9eaba3c353ff3aff8ffa7dec07b295068742b40bffe7ce088c`，原模型源锁均未变化。前两次失败只发生在见证证明的 Decidable/展开写法，冻结声明未改，失败源和日志保留。收尾使用原 LEAN_PATH 运行 receipt 检查，返回 `status=current`、`snapshot_current=true`、`exact_root_passed=true`；首次漏设该环境变量的检查被准确拒为 `runtime_search_path`，没有改源或以拒绝结果冒称通过。

[完整回执](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/check3/run-manifest.json)、[摘要](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/summary.json)、[源锁](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/source-lock.json)与[复现脚本](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/verify.sh)均在仓外。复现命令为 `sh verify.sh <新的仓外输出目录>`。本根 `semantic=not_reviewed`；机器闭合与原义审查分账。

停止两条重复支路：不重做 Stage33 已核的“7 个确认成员不足不重用的 3×3 来源”障碍；不把 Stage45 同一平台目录的 P/E 外缘算术移植后再宣布直接路线失败。也不再尝试把选边位移直接当真实价格速度，Stage28/30 的时钟和 spread 边界仍在。

若继续现候选，最小新增实现目标应是**具名原始支撑到已确认 construct 输出的逐字段桥**，而不是重新发明另一套分块：固定 Stage24 的 `Through/Reads`、Stage25 的 `output/Completed` 和现有构造，证明每个 `knownAt≤n` 且 `geometry=some s` 的真实输出，都有其 `[start,finish)` 原事件和两端簿状态见证，且几何外缘等于本文 Support 的精确包络；`geometry=none` 必须保留原来源，不允许跨接。还须证明对应源区间覆盖和相邻共享状态，不能只以 `some` 宣称合法。这个 target 可以立刻沿现有 imports 执行，但**本轮没有执行或声称完成**；它仍只提升 P2/extent 的接线精度。

真正决定原义 P1 去留的下一项，是为 R_D 单独给出独立最低核心/走势关系及带来源的完成证据，并核它与原 F₂ 的对象、同级、成员、生命周期相容。若暂时给不出该关系，应记录这个具体缺项，不能拿已证明的压力确认或本轮包络填 EXT-own。现有结果保留 R_D 路线开放，P3 主确认与生产采用资格不变。
