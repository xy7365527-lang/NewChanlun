# Stage15：直接供需块的报价边界读出及几何退化

日期：2026-10-03，#1467，候选 `R_D-Q-v0`。承接 [开工卡](Stage15WorkCard.md)，重新推进原图的直接订单簿路线。名分：新候选的操作定义、定义内证明和限定退化结果；不是缠论语义资格或经济效用的确认。

## 为什么另立候选

旧 R_D-v2 已证明：供需符号不能直接当作原触价路径的价格方向。本候选保留极大同向事件块，**另行声明报价边选择读出**：正块从块前bid读到块末ask，负块对偶。所选择的价格确实属于假设中的固定报价，但它们不是原触价顺序，也不表示实际成交按这个路径发生。

这改变了被观察的对象，不是修复了旧触价桥。程序没有在旧判据失败后自动切到此读出；旧反例与无损全序强桥的不可能性仍保留。该读出不能用来结算收益，不能把价差两端的交替当成已成交利润。

## 完整操作定义与有效域

| 项目 | 本候选定义 |
|---|---|
| 输入 | 初始bid/ask固定且bid<ask；两侧正数量；有序顶层单侧量事件 `(side,action,amount)`，action为新增/撤销/执行，amount>0，所有更新后的数量仍正 |
| 事件保留 | 块内保留全部模型Event记录；不是只保留符号。本模型没有声称恢复原生MBO订单ID、完整载荷或隐藏流动性 |
| 符号 | 加买/撤卖/卖侧执行为正；撤买/买侧执行/加卖为负。这是声明的供需记号，不是对真实价格涨跌的断言 |
| 划分 | 极大同号连续块，非空、块内同号、相邻异号；空输入输出空块列表 |
| 成员和时间 | 块覆盖事件[s,e)，状态跨度s→e；第一成员到达的s+1为活动块首见时刻，第一条反号事件在e+1确认前块，最后块保持活动。文件结束不确认尾块 |
| 价格读出 | 正块：bid→ask；负块：ask→bid；不改原事件触价记录。两个相邻反号块共享所选报价端点 |
| 几何范围 | 固定报价下每块范围恒为[bid,ask]；方向只指这条合成报价选择路径 |
| 边界 | 报价变动、耗尽为空侧、缺口、复合事件和跨场所未获本候选定义；不得跳过这些输入后宣称全域通过 |

`group`是结构划分器，在数学上可作用于任何事件列表；市场输入资格必须另有 `ValidHistory` 与固定报价条件。它不是原生交易所适配器，也没有接入生产。

## 一般证明

`Runs`是独立的块关系：空列、单事件、同号前接与异号新块四种规则。已证明任意有限事件列存在唯一满足关系的分块，并证明原事件解码等于输入、每块同号且相邻块异号。这里的唯一性是具名同向分块的唯一性，不是完整缠论分解唯一。

完成块的独立边界谓词 `Completed(p,n,s,e)`要求：s<e<n、左边界是流开始或一次变号、块内同号、e处出现反号。一般前缀定理证明：两个原事件历史在已见的前n项相同时，任何具名s/e边界的完成命题相同。它没有读n之后的事件。

`locate`按真实块长度标来源及knownAt。其一般性“所有输出均对应上述Completed关系”的连接，本轮根据最大块性质作纸面推导，**未新增为单独的Lean根声明**；下述20事件见证的具体位置和确认序列则在根中检查。在线增量实现和真实采集时钟映射也未验证。

起点状态s的bid和ask都已存在，但选择其中哪一边，要等第一成员事件确定块符号后才知道。因此不能把“状态s的报价来源”写成“状态s已发布的单一价格观测”；这里的选择最早在s+1可知。当前载体中的start不能直接替代firstKnown，首次发布字段的接线仍属P2未完项。

几何一般定理覆盖任意bid<ask、任意非空块和起点：所选端点时间有序，范围恰为[bid,ask]；相反符号的相邻块端价接续；三个符号交替的块满足实际Origin局部中枢谓词，而且核心**恒为[bid,ask]**，与量的大小无关。

这仍未给出F₂完成证书。更高层递归、同级连接和完整分类不能仅从三个局部合格块推出。

## 两份不同历史的同值几何

初始两侧数量100，bid100/ask102。循环“撤卖、加买、撤买、加卖”五轮，共20个事件；一份每笔数量10，另一份每笔数量5。

| 结果 | 数量10历史 | 数量5历史 |
|---|---|---|
| 数量状态始终为正 | 通过 | 通过 |
| 原事件列 | 与右侧不同 | 与左侧不同 |
| 最大同向块数 | 10 | 10 |
| 前三块几何核心 | [100,102] | [100,102] |

数量10见证的位置检查得到knownAt为 `3,5,7,9,11,13,15,17,19,None`：9完成块，1活动尾块。首块在只见两个事件时未完成，见到第三个反号事件后才完成。

旧问题没有消失：首块原触价仍按 `[102,100]` 出现，新选择路径却是 `100→102`。本轮明确证明这两条读出不同，不能把新路径说成旧触价方向已经合法。

## 对“有没有用”的限定结论

在固定报价子域内，**局部中枢的数值边界退化为原来已知的bid/ask**。数量幅度差异不会进入这些边界，虽然原事件及数量仍在块标记中完整保留。因此，造出局部中枢、甚至反复出现相同区间，不能算作增量价值证据。

这不等于出现时机、块长度、事件类型、数量标记或其他统计用途没有价值。若继续实验，必须分别比较这些来源，沿A/B/B_seed/C保持同样历史与资源预算；不能把保留在标记中的订单信息收益归给几何递归。本轮没有收益、预测或净成交结果。

固定报价是用户原先关注的queue-only子域，尚不能替代一般报价演化研究。报价变化时如何拼接、是否仍满足原义F₂，以及此合成观察是否有足够缠论语义依据，都保持开放，不靠限定子域宣布总目标完成。

## 证据与复现

根 `DirectQuote.exact_root`，类型：

`RUN_TARGET ∧ CAUSAL_TARGET ∧ GEOMETRY_TARGET ∧ WITNESS_TARGET`。

最终run `6fb6a2f71c86428997a7be2a9148863e`，Lean4.31 精确检查exit0，machine/exact_root均通过。实际假定仅 `propext/Quot.sound`，无sorryAx、自定假定或不安全依赖。声明锁未变；开发时仅将可判定性实例改成可编译的结构递归，并修有限列表成员证明，未改对象或验收。

摘要见 [stage15-lean-verification-summary.json](stage15-lean-verification-summary.json)，源绑定见 [source-sha256-stage15.json](source-sha256-stage15.json)。原始日志位于 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-verified-v1/`。semantic仍为not_reviewed，独立读回/比较包待派发。

工作树根目录复现，使用新输出目录：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file DirectQuoteProof.lean --declaration DirectQuote.exact_root \
  --expected-type 'DirectQuote.RUN_TARGET ∧ DirectQuote.CAUSAL_TARGET ∧ DirectQuote.GEOMETRY_TARGET ∧ DirectQuote.WITNESS_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/direct-quote-reproduction-2
```

本轮没有新取数、安装工具、修改旧候选或formal/生产，也没有选择最终市场或启动确认实验。

## 独立审查通道的当前限制

已再次检查专用工具，没有父模型切换入口。随后通过当前支持的CUA入口读取Codex应用，被工具明确拒绝：`Computer Use is not allowed to use the app 'com.openai.codex' for safety reasons.`

因此没有更改模型、没有派发代理，也未用其他低级接口绕过。此处是工具的应用访问安全限制，不是数学工作流要求用户裁决定理。记录位于上述证据目录的 `review-channel-observation.json`，明示它是本次工具结果的人工记录而非原始日志导出。独立语义审查不能被作者核对或Lean编译代替；其他已授权数学与数据工作可继续。
