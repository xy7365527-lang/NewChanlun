# DirectOutput 四声明独立冷读

结论：这四个命题表达的是有限事件序列的原始符号块归属、逐事件唯一覆盖、成功几何输出的账本与 L1 报价区间包络绑定，以及相邻原始输出的切点和端点一致性。它们没有直接断言所有块都有几何，也没有断言这些块是缠论线段或中枢。未发现把四个待证结论直接放进前提的循环定义；有效前提域可以非空。以下是声明语义审阅，不是根证明通过、编译通过或传递公理闭包通过的报告。

审阅方式：只读冻结的 `DirectOutputSpec.lean` 及其实际导入源，并按 `lean-verify` 的声明冷读要求检查类型和边界。未读取作者新写的 DirectOutput 证明、作者会话、上一轮报告或意图说明；源内说明性注释不作为数学依据。目标 SHA-256 核对为 `23d38c0d49b57d7438efd90757dc7a80e138d54400de123955280857164c8e6b`。源码路径均以研究工作树为基准；下文无目录前缀的 Lean 文件均在 `.chanlun/review-results/issue1467-f1-proof/`。准确输入清单见同目录 `direct-output-source-sha256.txt`。

## 1. 载体、量词和模型边界

四个 `*_TARGET` 都是 `Prop` 的定义，而不是本文件中的证明。它们没有自由的数学公理参数、隐含的非空类型假设或宇宙多态参数。各个未写类型的量词由使用位置恢复如下（`DirectOutputSpec.lean:16–56`）：

| 变量 | 实际载体 |
|---|---|
| `es` | 有限列表 `List MovingQuote.Event` |
| `qs` | `List (Option MovingQuote.Quote)` |
| `bs` | 全函数 `Nat → MovingQuote.Book` |
| `out`、相邻分解的 `pre/post` | `List MovingQuote.Output` |
| `x/y` | `MovingQuote.Output` |
| 索引、长度、观测数量 | `Nat`，从零开始的事件位置 |
| `q/r/v` | `MovingQuote.Quote`，两个整数 |
| `s/t`（几何量词） | `NewChanlun.Origin.Segment` |
| 支撑价格 | `Int`，不是实数或有理数 |

事件具有买卖侧布尔值、`add/cancel/execute` 三种动作、自然数量以及整数价格。数量为零、价格为负在事件类型本身都合法。`positive` 只看侧和动作：买侧新增、卖侧取消或执行为 true；卖侧新增、买侧取消或执行为 false。价格和数量不参与符号判定。因此符号变换不等于成交价反转，也不要求最佳报价严格移动（`DirectQuoteSpec.lean:7–18`；`MovingQuoteSpec.lean:7–11`）。

`Book` 的两侧分别是整数列表；列表重复项表达库存重数，类型本身不要求排序、非空或不交叉。`QuoteOf b q` 要求 q.bid 是 bids 中实际出现的最大值，q.ask 是 asks 中实际出现的最小值，且 q.bid < q.ask。两侧都必须非空，锁价或交叉账本不能有该报价。`ValidBook` 只要求每个 bid 小于每个 ask，因此一侧空时可空真，不能替代存在报价（`MovingQuoteSpec.lean:40–51`）。

`Step e a b` 要求事件数量严格正且只更新指定一侧，另一侧保持原列表。新增是在表头加入 amount 份价格；取消和执行使用同一个删除关系，要求原列表里存在连续的 amount 份该价格，并删除该连续子串。它没有订单 ID、时间优先级、成交撮合约束或执行必须在最优价的规则；amount > 1 的删除还依赖列表中这些价格能否连续出现（`MovingQuoteSpec.lean:52–58`）。

`Segment` 仅是方向、两个自然数索引、两个整数价格的记录，不含线段成立、价格单调、正长度等证明字段。Tick = Int，Index = Nat；`segLow/segHigh` 只是两端价格的 min/max，不扫描事件轨迹（`formal/Origin/ChanlunElements.lean:13–14,43–49`；`formal/Origin/CenterConstruction.lean:52–57`；方向二元类型见 `formal/Origin/SourceAxioms.lean:53–56`）。

## 2. 实际构造、原始来源和三层 None

每个 `Block` 都有一个 first 和任意 rest，故其事件列表永远非空。`group es` 按相同 positive 合并相邻事件，保留事件原序；`decode` 拼接块事件。`Uniform` 仅断言块内符号统一，`Separated` 断言相邻块符号不同（`MovingQuoteSpec.lean:13–35`）。`locate 0` 按块长度累加自然数切点：半开归属区间为 [start,finish)，firstKnown = start+1；除最后一块外 knownAt = some (finish+1)，最后一块为 none（同文件 `95–106`）。

`construct` 的唯一全局接受条件是 qs.length = es.length+1。满足长度便把所有 located 块逐个映射成 Output，不过滤 geometry = none；不满足长度才返回外层 none。它没有接收 bs，也不检查 Step、Reads、正数量、实际账本或来源文件真实性（`MovingQuoteWireSpec.lean:5–14`）。因此 SOURCE/COVER 的构造成功前提，单独不能证明事件在交易所或模型账本中可执行。

每块的几何读取包含其 start 至 finish 的全部状态报价，长度为块事件数 + 1。`readout` 先要求至少两个条目，再要求每项都是 some 且 bid < ask，最后检查正向 q.bid < r.ask 或负向 r.bid < q.ask；成功后只用首尾报价与窗口长度生成 leg。它并不检查窗口报价来自同一账本轨迹，也不检查中间报价单调。正向 leg 从起点 bid 到终点 ask，负向 leg 从起点 ask 到终点 bid（`MovingQuoteSpec.lean:71–83`）。

必须区分：

1. `construct = none`：报价列表长度不匹配，四个目标的构造成功前提不成立。
2. `qs[i]? = some none`：索引存在，但该状态没有报价。无 Reads 前提时它只是输入标记；有 Reads 时它严格表示不存在 QuoteOf。`qs[i]? = none` 则是越界，二者不同。
3. `x.geometry = none`：原块和来源仍存在，几何读取未成功。任何窗口内报价缺失都导致这个结果。`knownAt = none` 又是独立的完成状态，不能与 geometry 缺失等同。

`Source es l` 以存在 pre/post 断言：es 恰为 pre ++ events(l.block) ++ post，pre.length = l.start，l.finish = l.start + 块长，且块符号统一。它绑定原列表中的连续子串及其位置，不是仅比较事件集合。它自身不要求两侧最大化、不含 firstKnown/knownAt、不含账本可执行性，也不含价格几何（`DirectOutputSpec.lean:12–14`）。

`eventAt es i` 是列表第 i 项，越界填固定 e0。后面 Good 的有效区间界和 Completed 的 e < n 保证实质事件引用都在 es.length 内，故不能用越界补值替末块制造确认（`DirectOutputSpec.lean:9`；`LocatedSpec.lean:10–19`；`DirectQuoteSpec.lean:67–71`）。

## 3. SOURCE_TARGET：来源、最大符号块和已完成标记

对任意 es、qs、out，只要实际 construct 返回这个 out，就同时断言（`DirectOutputSpec.lean:16–22`）：

- 去掉几何后，located 列表逐项等于 locate 0 (group es)。
- 拼回所有输出块事件得到 es，事件的字段、顺序和重复次数全部保留。
- 每一个原输出 x 都满足 Source 与 Good，且 Completed 当且仅当 knownAt = some (finish+1)。这一全称量词包含 geometry = none 的输出。

`Good` 的 Span 部分要求 start < finish ≤ n，起点在零或前一事件符号不同，块内各符号相同，终点在 n 或下一事件符号不同；即相对于已观察前缀的非空最大符号区间。Good 还固定 firstKnown、块长，且 knownAt = none 当且仅当 finish = n，任何 some k 都只能是 finish+1（`LocatedSpec.lean:10–19`）。这里 Source 提供实际事件列表对应，Good 提供区间最大性；二者作用不同。

实际名字是 `Completed`，不是“全部研究或全部输出已完成”。它要求 start < finish < n、左边界和内部统一，并且 finish−1 与 finish 处符号相反。因此 [start,finish) 已完成要等待不属于该块的第一条反号事件 finish 被观察到，观测事件数至少 finish+1。无报价、无价位极值、无缠论走势完成条件；它也不判成交结束（`MovingQuoteSpec.lean:107` 转到 `DirectQuoteSpec.lean:67–71`）。末尾块 finish = n 始终不 Completed，即使其 geometry 已成功。四目标没有引用 imported `CenterConfirmedComplete`；那个名字在依赖中定义为三段方向交替及三段核心严格重叠，不能把它的 Complete 含义移植到本目标（`formal/Origin/CenterComplete.lean:83–84,147–149`）。

SOURCE 并未在文字上单独量化“任意 Completed 区间必为输出”；它首先限制实际输出。结合 COVER 与最大区间唯一性可以建立更广的完备对应，但不应把未展开的额外定理当成此处的直接文字。

## 4. COVER_TARGET：每个事件位置恰有一个所有者值

对任意构造成功的 es、qs、out 及每个 i < es.length，存在 x ∈ out 使 start(x) ≤ i < finish(x)，且任何同样覆盖 i 的 y ∈ out 都满足 y = x（`DirectOutputSpec.lean:25–27`）。这里唯一性落在整个 Output 值，包括 located 和 geometry；不是仅要求区间相等。

量词是列表成员值而非列表出现位置，所以仅就这段公式，两个完全相同的重复条目不会被区分为两个所有者。实际 construct 的正长度、累加切点和原始映射进一步排除了那种重复构造；应区分“值唯一”的公式与“出现位置唯一”的另一个表述。该目标不要求所有者有几何，不允许把 none 输出删掉后再拿剩余几何重新归属（构造定义及 SOURCE 的列表等式绑定这一点）。

## 5. HULL_TARGET：有几何的原块对应什么包络

这里比 SOURCE/COVER 多三个前提：Through es bs n（n = es.length）、Reads bs qs、qs.length = n+1，再加 construct 返回 out（`DirectOutputSpec.lean:31–45`）。最后的长度前提与 construct 成功在数学上重复，但不矛盾。

Through 要求每一个 i < n 都由原列表第 i 个事件以 Step 将 bs i 更新成 bs(i+1)。bs 在 n 之后任意，初始账本也没有另外给定条件。Reads 在每一个已给报价索引 j 上要求：some q 必须是 bs j 的真实最优有效报价；none 必须是 bs j 不存在 QuoteOf。长度使这些约束覆盖全部 n+1 个状态，而非只覆盖事件前的 n 个状态（`BookQuoteSpec.lean:9–16`）。没有全局 ValidBook 前提；锁价、交叉或单边空账本可经 none 报价进入输入域。

结论对每个实际输出 x、每个 s，只要 geometry(x) = some s，就存在其起终点报价 q/r，且它们恰是 qs 在原 start/finish 的 some 值，又是 bs 在这些原切点的 QuoteOf。随后要求同方向 Flow、s 恰为相应 leg、以及该 leg 的两端 min/max 是指定 Support 的 ExactHull。没有 knownAt/Completed 限制，所以末尾开放块也被覆盖。

`Flow pos a q b r es` 是归纳的有限合法重放关系：空事件流必须起终点为同一本书与同一有效报价；每一步满足 Step、前后 QuoteOf、事件 positive = pos，随后接余下 Flow。故全部中间状态都存在有效双边报价，全部事件数量正且符号统一。它不要求最优报价严格变动；导入证明的 flow_monotone 给出两个最优价分别弱单调（`MovingQuoteSpec.lean:65–69`；`MovingQuoteProof.lean:149–159`）。

`Support pos a q b r es x` 表示存在一个事件切分 es = pre ++ post，存在中间账本 mid 与报价 v，两段均为同方向 Flow，且 v.bid ≤ x ≤ v.ask。它是在有限事件各状态切点的闭合 L1 spread 中取整数价格，而不是所有事件 price、所有深档订单价、成交价、或连续实数时间插值轨迹的集合。pre/post 可以为空，因此两个端点报价区间都在支撑里（`DirectHullSpec.lean:8–11`）。

特别是，HULL 把 Flow 和 Support 的首尾账本钉到 bs start / bs finish，但 Support 内部的 mid 和两段 Flow 是存在量词，并未逐项写成该给定 bs 的中间状态。它描述的是满足同一事件及相同首尾约束的合法重放切点支撑。若需要与给定 bs 的逐点 L1 支撑相等，还应展示重放报价唯一性或该等式的桥接；不能把这种逐点等式说成目标已经显式断言。导入 BookQuote 对库存等价和读值唯一的证明可相关，但四目标未直接包含这个新等式（`BookQuoteProof.lean:63–72,77–107`）。

`ExactHull S lo hi` 精确要求 S lo、S hi 且所有 S 中整数均在 [lo,hi] 内；故 lo/hi 是取得到的最小/最大值，S 不可能空。定义本身没有断言中间每个整数都属于 S，也没有求和、体积或拓扑闭包。只有端点和界保证，足以把 [lo,hi] 认作最小包含区间（`DirectHullSpec.lean:13–14`）。正向 leg 的界是起点 bid 至终点 ask，负向为终点 bid 至起点 ask（`MovingQuoteSpec.lean:71–72`）。

HULL 没有把 Flow 或 ExactHull 作为根前提；前提是原路径 Through、Reads 与算法成功几何。Support 定义不引用 Segment、segLow/segHigh 或待求界，因而不是用候选包络反定义支撑。它确实使用 Flow 来定义所讨论的支撑域，这限制了数学对象，但没有把所需界或极值直接塞入定义。导入 hull 证明利用切分、单调性和端点支撑建立界（`DirectHullProof.lean:6–60`），本轮不对新根证明使用它的方式作任何判断。

## 6. ADJACENT_TARGET：未经筛选的邻接

在与 HULL 相同的全局前提下，对每一个原输出列表分解 out = pre ++ x::y::post，断言 x.finish = y.start、两块符号互反、两个 qs 查询值相同；如果两块各自几何都为 some，则 s.endPrice = t.startPrice（`DirectOutputSpec.lean:49–56`）。这里 pre/post 是输出列表，而不是事件列表。

同一个索引的报价查询一致，是切点相等的结果；公式没有单独声称该公共查询必须存在报价。若两几何都成功，就可由 readout 得到公共报价，正后负在公共 ask 相接，负后正在公共 bid 相接。若任一 geometry = none，端点等式的条件不满足，但来源切点和符号互反仍被要求。它没有断言删除 none 后新形成的相邻几何仍连接，也没有构造一条跨缺口连续曲线。只有零个或一个原输出时，整个相邻分解量词空真。

## 7. 空域、退化和非空检查

- es = []：当 qs 恰一项时 construct = some []；qs 可为 [none]。SOURCE 的两列表等式成立，而逐输出、逐事件及相邻输出部分均无实例。HULL/ADJACENT 仍需 Reads；Through 在 n=0 时空真。空数据是允许情形，不能作为非空数据结论的证据。
- 非空 es 但所有块 geometry = none：SOURCE/COVER 仍实质约束每个事件；HULL 的逐几何结论、ADJACENT 的端点子句可局部空真。四目标没有无条件保证至少一个 some 几何，也没有在文字上给出 geometry = none 的充分必要条件。
- 非空有效域可直接按定义构造：取 `DirectHullSpec.lean:23–28` 的 deep、reverse、b0、b1、b2、q；es = [deep,reverse]，bs 的前三态为 b0,b1,b2（以后任意），qs = [some q,some q,some q]。deep 是买侧新增 90，reverse 是卖侧新增 110；两次更新合法，三态最优报价均为 (100,102)。construct 得到正、负两个单事件输出，几何分别为 100→102 与 102→100，区间分别 [0,1)、[1,2)。第一块 knownAt = some 2 且 Completed，第二块 knownAt = none 且不 Completed。此例同时使有几何、原邻接、已确认与开放块条件有实际实例，排除“整体仅靠空域”的解释。这是按现有定义展开的语义见证，本轮未新增编译证明。
- 同一例中事件价格 90（以及另一事件价格 110）在 L1 [100,102] 外。故精确 L1 包络不蕴含全部真实事件价格或深档价格都在几何区间内；90 的排除还在导入的 `DirectHullSpec.lean:33–41` 中明确形式化。
- SOURCE/COVER 接受类型上合法但 amount=0、不可执行删除、随意报价的输入，只要长度匹配。HULL/ADJACENT 的 Through/Reads 将这些不一致排除；这属于明确的条件定理域，不能声称 constructor 自己验证了这些前提。

## 8. 本轮可交付边界

四目标没有直接隐藏 Source、Good、覆盖唯一性或 ExactHull 作输入假设；construct 也未接收这些证明。它的 located 映射等式接近定义展开，事件重构、最大区间与覆盖性质仍需依赖分组不变量；账本/几何绑定则需要把来源切片、全局路径和成功 readout 接起来。条件里确实假定全局账本重放与报价读取正确，本轮不能将其当成无条件数据真实性结论。

本审阅未读取目标的新 Proof，未运行 Lean，未提取 elaborated pp.all 类型，也未检查根声明的传递公理集合。这里的类型依据已读源码中的具体结构、函数签名和用法恢复；源哈希清单是显式 import 的本地源码闭包，不是 Lean 实际加载的 olean、运行时或宏环境清单。四个目标与任何外部非形式化意图是否吻合，应由收到本冷读与外部契约的另一独立审阅者判断；这份文件不预判该对照结果。
