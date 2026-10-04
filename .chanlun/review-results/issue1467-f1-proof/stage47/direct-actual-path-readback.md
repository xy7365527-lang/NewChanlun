# 实际来源路径声明冷读

`ACTUAL_PATH_TARGET` 的结论直接绑定本次输入的账本路径 bs 和报价列表 qs：对每一个实际构造出的 some 几何，原块起点至终点的每个状态切点都有来自同一 qs 的有效报价，并且这些实际状态的 L1 买卖报价闭区间之并，其最小包含整数区间恰由该几何的两端最低价与最高价确定。没有发现将这一结论直接放进前提的循环，也不存在迫使全部有效输入为空的条件。

这是声明语义冷读，不是证明或编译通过报告。读取对象为冻结的 `DirectOutputSourcePathSpec.lean` 及必要实际定义；未读取新增 Proof、作者稿、Intent 或其他 stage47 报告。本审阅由前次定义冷读的同一代理接续，保留此前已读定义的上下文，不能冒称全新无状态代理；本轮未接收非形式化意图。冻结目标 SHA-256 为 `9cccbdf9b546c51aa3eaf323f282ea81a58032d3bb15c889c18efdc17bf9d648`。下文无目录前缀的 Lean 文件均位于研究树 `.chanlun/review-results/issue1467-f1-proof/`；源码哈希清单为 `direct-actual-path-source-sha256.txt`。

## 1. 量词和实际载体

目标是一个 `Prop` 定义，尚不是这个文件中的已证定理（`DirectOutputSourcePathSpec.lean:12–19`）。完整量词次序为：

1. 任意有限事件列表 es、总账本状态函数 bs、有限可缺失报价列表 qs、输出列表 out。
2. 假定 es 在 bs 上全部可逐步执行，qs 如实读取 bs，qs 有事件数加一项，并且本次 construct es qs 恰返回 some out。
3. 对每个 x ∈ out 和任意几何记录 s，若 x.geometry = some s，则两项结论同时成立：窗口内每个状态切点存在正确报价；ActualSupport 的 ExactHull 为 [segLow s, segHigh s]。

具体类型是 es : List MovingQuote.Event，bs : Nat → MovingQuote.Book，qs : List (Option MovingQuote.Quote)，out : List MovingQuote.Output，x : MovingQuote.Output，s : NewChanlun.Origin.Segment。状态索引 k、start、finish 为 Nat；支撑价格 z 和 bid/ask 是 Int。Segment 的 Tick = Int、Index = Nat，且记录本身没有非空、正确方向或有效线段的证明字段（`MovingQuoteSpec.lean:7–11,40–47,95–101`；`MovingQuoteWireSpec.lean:5–7`；`formal/Origin/ChanlunElements.lean:13–14,43–49`）。

本命题没有类型多态的参数，也没有自动引入“至少一个事件”“至少一个 some 几何”或“末块已完成”之类的非空前提。价格域并非实数，索引也不是物理时间戳。

## 2. 给定 es、bs、qs 如何相互约束

令 n = es.length。Through es bs n 要求每个 i < n 都有 es[i]? = some e，并且 Step e (bs i) (bs(i+1))；不是只要求存在一条未命名的账本路径。Step 要求事件数量正，指定侧按事件价格和数量新增或删除，另一侧保持原列表。删除对 cancel 和 execute 使用相同关系，要求被删的重复价格在原列表连续出现。没有额外的订单 ID、实际撮合、成交必须在最优价等约束（`BookQuoteSpec.lean:13–14`；`MovingQuoteSpec.lean:52–58`）。

Reads bs qs 对列表内每个条目都约束其与同索引 bs 的关系：some q 必须满足 QuoteOf (bs i) q；none 则表示不存在任何 q 满足 QuoteOf (bs i) q。QuoteOf 要求 bid 是买列表中实际出现的最大值，ask 是卖列表中实际出现的最小值，且 bid < ask。因此单边空、锁价和交叉账本没有该有效报价。qs.length = n+1 使 Reads 覆盖状态 0 到 n，包含处理最后一条事件后的状态（`BookQuoteSpec.lean:9–16`；`MovingQuoteSpec.lean:40–51`）。

这些是待检验输入的数学前提，未断言 qs 或 bs 由外部真实数据抓取得到。bs 0 任意，bs 在 n 之后也任意；目标没有全局 ValidBook 假设，允许某些状态无有效双边报价，前提要求用 none 如实表示。

construct 的输入只有 es 与 qs，没有 bs。它只以长度是否等于 n+1 决定外层 some/none；成功后按原始符号块逐一保留 Output，并将该块 start 到 finish 的报价窗口交给 readout。readout 要求至少两个条目、全部为 some 且 bid < ask，并满足首尾方向价差条件，才产生 leg。它自身不核验报价属于实际账本；这种绑定来自 Through 与 Reads 前提（`MovingQuoteWireSpec.lean:9–14`；`MovingQuoteSpec.lean:71–83`）。因此“构造成功”不能替代“输入账本和报价关系正确”。长度等式作为前提与构造成功在此数学上重复，但没有造成矛盾或空域。

## 3. ActualSupport 的严格含义

定义为（`DirectOutputSourcePathSpec.lean:7–10`）：

> z 属于支撑，当且仅当存在一个自然数状态切点 k 与报价 q，使 start ≤ k ≤ finish，qs[k]? 恰为 some (some q)，QuoteOf (bs k) q，且 q.bid ≤ z ≤ q.ask。

故它是给定路径中各可用状态报价闭区间的整数点之并。k 的两端都包含；这里 start/finish 是状态切点，原块事件归属仍为 [start,finish)。bs start 是第一条块事件前的状态，bs finish 是最后一条块事件后的状态。若紧接着存在反向事件，其事件位置为 finish，这个反向事件执行后的状态 finish+1 不在当前 ActualSupport 内（块长度与切点定义见 `MovingQuoteSpec.lean:102–106`）。

定义同时钉住 qs[k] 与 bs k，不能用另一个合法重放中的中间账本替代 bs k，也不能任择一个与 qs[k] 不同的报价。它没有要求 q 先成为某条 Flow 的存在见证；甚至不接收 Flow、事件列表、方向或候选几何参数。事件来源关系由上面的全局 Through 和实际 construct 关联到窗口。

必须区分两个层次：

- ActualSupport 单独使用时，可以有缺口。若 qs[k]? = none（越界）、qs[k]? = some none（明确无报价），或某个 some q 不满足 QuoteOf，则这个切点不贡献任何价格。它没有要求所有切点都贡献支撑。
- 在 ACTUAL_PATH_TARGET 的成功几何结论中，第一合取项进一步要求每个 start ≤ k ≤ finish 都存在正确 some q，排除了窗口内全部这些缺口。第二合取项不能单独替代第一项：两个极值存在并有全局界，并不自动表达每个状态都存在报价。

相对于 `DirectHull.Support` 的定义，ActualSupport 没有存在中间重放路径的量词。前者用 es = pre ++ post 与两段 Flow 约束存在的 mid/v；此处直接读取实际 bs/qs（`DirectHullSpec.lean:8–11`）。本目标也没有声称两种支撑集合相等；它直接为 ActualSupport 指定几何包络。

## 4. ExactHull 与目标的两项结论

`ExactHull S lo hi` 逐字是 S lo ∧ S hi ∧ ∀ z, S z → lo ≤ z ∧ z ≤ hi。它保证 lo/hi 是实际取得到的最小值和最大值，因而 S 非空，且 [lo,hi] 是包含 S 的最小整数闭区间。它的定义没有另要求所有中间整数都属于 S，不是成交价格集合、深档订单集合，也不是连续时间轨迹的等式（`DirectHullSpec.lean:13–14`）。

应用在 ACTUAL_PATH_TARGET 时（`DirectOutputSourcePathSpec.lean:15–19`）：

1. 对窗口内每一个 k，都存在 qs[k] 明确给出的 q，而且它是 bs k 的有效最优报价。它是对所有原切点的全称存在结论，而非仅对首尾的存在结论。
2. segLow s 和 segHigh s 各自必须出现在某个实际切点的报价闭区间里，且每个实际切点的整个报价闭区间都不能越过这两个界。

`segLow/segHigh` 仅取 s 两端价格的 min/max（`formal/Origin/CenterConstruction.lean:52–57`）。来自 readout 的正向 leg 起价为起点 bid、终价为终点 ask；负向反过来取起点 ask 和终点 bid（`MovingQuoteSpec.lean:71–82`）。因此这个几何是报价侧读取，不是把所有事件 price 连成的价格路径。仅在目标的前提成立且 geometry 为 some 的域里，该两端界才被要求成为实际 L1 支撑的精确界。

没有 knownAt 或 Completed 前提，所以它同时覆盖已确认块和最后开放块，只要相应 geometry 为 some。它没有保证来源序列里存在任何完成块，也没有赋予 Segment 缠论线段或走势完成的语义。

## 5. 空域、None 与具体非空情形

ActualSupport 在 start > finish 时为空；在整个窗口没有任何满足定义的 qs/bs 报价时也为空。ExactHull 对空支撑不可能成立，因为它要求 S lo 和 S hi。目标没有把这个不可能要求施加于 geometry = none 的输出。

es = [] 且 qs 恰一项时，construct = some []。Through 空真，Reads 仍约束唯一初始状态；目标的逐输出结论没有实例。非空 es 若所有几何都是 none，也会让目标的逐 some 几何部分空真。这些是允许的退化输入，不应当当成一般成功几何的验证。

整体有效域并非只能退化。按定义可取一条买侧 add 事件，数量 1、价格 90；令 bs 0 的 bids/asks 为 [100]/[102]，bs 1 为 [90,100]/[102]，以后任意；令 q = (100,102)，qs = [some q,some q]。此时 Through 的唯一步合法，两个报价都真实，construct 产生一个正向、索引 [0,1]、价格 100→102 的 some 几何。两个实际状态切点的报价区间都为 [100,102]，故 ActualSupport 的包络非空且恰为这个区间。该块同时是开放末块。这个定义展开见证可直接由已导入的 deep/b0/b1/q 取值重现（`DirectHullSpec.lean:23–28`）；本轮没有新增或编译见证证明。

同一例的真实事件价格 90 不在 L1 包络中，虽已进入 bs 1 的买侧库存。因此本命题并不保证所有事件价格或订单簿全深度价格落入几何范围。

## 6. 循环性和未保证事项

ActualSupport 不含 s、segLow、segHigh 或 ExactHull；ExactHull 不接收目标证明。ACTUAL_PATH_TARGET 的前提只有 Through、Reads、长度和实际 constructor 输出，以及 geometry = some 的局部条件，没有假设 ActualSupport 已有正确界，也没有直接假设目标自身。Reads 中 none 的“不存在有效报价”限制是真实且明确的输入假设，不是将包络结论换名。成功 readout 检查全部条目可用，确实是“每个切点有报价”结论所需的信息来源，不构成逻辑循环。

这条声明没有直接保证：constructor 总会产生 some 几何、每个原始输出都有几何、None 输出被修复、真实交易所数据满足 Through/Reads、深档或成交价覆盖、Support 与 ActualSupport 集合相等、相邻几何连续、Completed 的时间对应，或三段中枢成立。它只约束已由当前构造实际给出的 some 几何；原输出的其他性质需要各自声明或构造定义支持。

本轮没有运行 Lean、查看新根 Proof、读取 Intent、提取 elaborated pp.all 或检查传递公理闭包。类型由实际源码结构和签名恢复。哈希清单绑定 20 份显式 import 可达本地源码，不代表 Lean 实际加载的运行时或 olean 清单。是否与外部意图对应应另作独立对照；旧冷读文件没有改动。
