# DirectHull.exact_root 独立声明冷读

本件只解释实际 Lean 声明及定义，不判断它们是否符合任何研究目的。冷读输入为七份 Lean 源码（阅读时过滤自然语言注释）、实际导入定义，以及作者 check3 回执的机器绑定字段；没有读作者研究报告或预期数学叙述。由独立 readback 子代理完成，未修改原 project、原 check3 或 formal。

## 1. 已检查的声明与证据范围

根是 `DirectHull.exact_root`，声明于 `direct-route-l1-hull/project/DirectHullProof.lean:80`，实际类型：

```lean
And DirectHull.SPLIT_TARGET
  (And DirectHull.HULL_TARGET DirectHull.LIMIT_TARGET)
```

该根没有 universe 参数、显式/隐式/实例参数。它的前两项定义内部有 `Flow` 前提；“根无参数”不把这些条件消掉。实际类型与给定类型文本 `DirectHull.SPLIT_TARGET ∧ DirectHull.HULL_TARGET ∧ DirectHull.LIMIT_TARGET` definitionally equal。

身份绑定：

- `DirectHullSpec.lean` SHA-256：`14f4d869525d1c9eaba3c353ff3aff8ffa7dec07b295068742b40bffe7ce088c`。
- `DirectHullProof.lean` SHA-256：`62779c7ab71c0469f840ddeef4694896bedaf60a8bc8791f8ee46101e048d21f`。
- 独立 run ID：`ed74a9224d7b4dcca693709d8713830c`。
- semantic SHA-256：`83c95e8fba961d9a19fa0e88c83a208fcf366dce3e56119afd92c29585aa0d77`；这是抽取内容身份，不是数学对应性判断。
- environment SHA-256：`c5ec67a91c596a2f0be46a92622fdfae1469ab887c3fa64f8f44d78272861926`。
- 独立 `run-manifest.json` SHA-256：`d6a65f075f3455487f5b34d4b1226c0a173862fed044437afeb83330d416741e`。

机器结果：七个本地模块重新编译，退出码 0；`machine_verification_passed=true`，`exact_root_passed=true`，`root_closure.status=closed`。检查器保留 `semantic.status=not_reviewed`，本件也不将机器通过写作“Lean 检查了原义语义”。

实际工具为 Lean 4.31.0，arm64-apple-darwin24.6.0，commit `68218e876d2a38b1985b8590fff244a83c321783`。直接使用已安装工具链，无安装、下载或 formal 重建。Lean 二进制 SHA-256 为 `1b370cfcbf44e80d1b004ab1b1ab9a4c73951f9f7c242140bcff9bc577576554`。检查器版本身份与完整工具哈希见独立 manifest；使用的说明是研究工作树 `.agents/skills/lean-verify/SKILL.md` 及其 `references/v2-verification.md`，脚本实际位于 `.agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/`。

fresh 编译顺序为 `DirectQuoteSpec → DirectQuoteProof → MovingQuoteSpec → MovingQuoteWireSpec → MovingQuoteProof → DirectHullSpec → DirectHullProof`，输出全部在自有 `readback-direct-hull/lean-verification-runs/<run-id>/lib/`。

预编译导入来自 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/.lake/build/lib/lean`，有七个 Origin 模块：`CenterComplete`、`CenterConstruction`、`CenterFull`、`CenterStates`、`ChanlunElements`、`CompleteClassification`、`SourceAxioms`。本次没有从源码重编这七个 formal 模块；相应源码和实际 `.olean` 分别记录哈希，并对相关已加载定义作 `#print`。不能把这描述成 formal 源码全量重建或独立内核复验。实际加载模块数 2273，包含 Lean 基础设施，完整 inventory 在 manifest。

实际根传递公理集是 `propext`、`Classical.choice`、`Quot.sound`。1752 项目标依赖中 `unsafe=true` 项为空；没有 `sorryAx` 或额外公理。七本地文件的 source scan 命中为空，source scan 仅作诊断；闭包结论来自 Lean 抽取。另行 `#print axioms` 得到：`split_flow` 仅依赖 `propext`；`hull`、`limit`、`exact_root` 为上述三项。不能据此宣称所有导入模块的所有未使用声明都无额外假设。

作者回执 run ID `a7166b7d233147b88226a6c5effc948c` 仅用于机器绑定比较：七源码及 toolchain 输入哈希、实际类型、公理集、semantic hash 与本次相同。作者回执本身 SHA-256 为 `effd0f13f42f8a1f4f66e52886eae0a97499c1bd7e8bfee257deee4e9df237f6`。本件的通过结论来自独立实际运行。

## 2. 载体与基础关系

`DirectQuote.Action` 有 `add/cancel/execute` 三个构造。`DirectQuote.Event` 只有 `bidSide : Bool`、`action : Action`、`amount : Nat`。`MovingQuote.Event` 再加 `price : Int`。事件本身不携带正数量、报价存在、订单身份、成交对象或时间戳证明。

`positive` 只看 side 与 action：bid add、ask cancel、ask execute 为 true；ask add、bid cancel、bid execute 为 false。它不根据价格升降、价格差、数量大小或是否触及最优价决定方向，零数量事件本身仍能分类。`Flow` 才排除零数量。

`MovingQuote.Book` 是两份有限 `List Int`，`bids/asks` 无排序或非空字段约束，允许重复、负数、零。`Quote` 是两个整数 `bid/ask`，其数据类型本身不带有效性证明。

`BestBid xs p` 是 `p ∈ xs` 且 `∀ x ∈ xs, x ≤ p`；`BestAsk xs p` 是 `p ∈ xs` 且 `∀ x ∈ xs, p ≤ x`。`QuoteOf b q` 同时要求 bid 是列表内最大值、ask 是列表内最小值，以及严格 `q.bid < q.ask`。因此获得 `QuoteOf` 必须两侧均非空，锁定或交叉报价没有该资格。对同一本账、若两个报价均满足 `QuoteOf`，`quote_unique` 证明二者相同；没有任意 Book 都有 Quote 的断言。

`ValidBook b` 只要求每个 bid 列表元素严格小于每个 ask 列表元素。任一侧为空时它可真；空账本 `⟨[],[]⟩` 的 ValidBook 真但没有 QuoteOf。已在独立 probe 编译该边界。也编译了 `QuoteOf ⟨[-2],[-1]⟩ ⟨-2,-1⟩`，确认价格载体是无正值限制的整数，而非实数、有理数或已定货币单位。

`Update action price amount before after`：add 要求 `after = replicate amount price ++ before`。cancel 与 execute 走完全相同的删除分支，存在列表 `pre/post`，满足 `before = pre ++ replicate amount price ++ post`、`after = pre ++ post`。这是从有序列表删除一段连续重复价格，不是任意位置挑出 amount 个同价项的定义；删除前后见证必须确实存在。add 是显式函数等式，删除是存在见证关系，定义未声称该关系对任意 before/amount 都存在或唯一。

`Step e a b` 首先要求 `0 < e.core.amount`，再要求事件指定的一侧满足 Update，另一侧保持列表完全相等。Step 单独不要求 QuoteOf 或 ValidBook，也不保证修改后仍有双边报价。

`Flow pos a q b r es` 是带事件列表索引的归纳关系：

- nil 构造只从 `QuoteOf a q` 得到 `Flow pos a q a q []`。
- step 构造要求当前 Step、当前及下一状态的 QuoteOf、当前事件 `positive e = pos`，以及尾部 Flow。

因此一个 Flow 见证已经提供每一步状态演进、每个切点的有效双边报价及同一极性。它不是从任意事件串自动推断有效状态的函数，根不证明任意输入都有 Flow。它也不要求 es 是 maximal run、已完成 run 或 group 的输出；任何满足关系的同极性子段均可。`flow_quotes` 提取两端 QuoteOf；`flow_monotone` 经 Step 证明报价两坐标按极性弱单调：true 时 bid/ask 均不减，false 时均不增。极性相同允许报价完全不变。

## 3. SPLIT_TARGET 的逐层含义

对任意 `pos : Bool`、起终账本 `a/b : Book`、起终报价 `q/r : Quote`、有限事件串 `es : List Event`，若已给 `Flow pos a q b r es`，则对任意列表 `pre/post`，只要 `es = pre ++ post`，存在某个 `mid : Book` 和 `v : Quote`，使前半段是 `Flow pos a q mid v pre`，后半段是 `Flow pos mid v b r post`。

结论是每个列表切点存在兼容的中间状态与报价，不是唯一性，不是新的无条件 Flow 存在性，也没有从外部流重建状态的算法规格。空前缀、空后缀、整个 es 为空均允许。证明确实对既有归纳 Flow 拆分；无需 `pre` 或 `post` 非空，也无需它们为已完成分组。

## 4. Support、ExactHull 与 HULL_TARGET

`Support pos a q b r es x` 量化整数 x 的资格：存在事件列表 `pre/post`、账本 `mid`、报价 `v`，满足 es 的前后拼接、两段 Flow 相接，以及 `v.bid ≤ x ≤ v.ask`。因此它取所有可被两段合法 Flow 连接的切点之闭合报价区间的并集。它包含区间内部没有挂单的整数价位；它不定义为事件价格集合，也不定义为整个盘口所有价位集合。

Support 没有输入一份具体 Flow 证明作为参数。其存在量词覆盖能满足给定两端与 es 的任意拆分见证，不能未证明就把它当作某份指定轨迹见证的图像。`SPLIT_TARGET` 保证已有 Flow 的每个切点有这种见证，但 Support 定义本身不依赖 SPLIT_TARGET 定理，hull 证明只需端点支持与弱单调边界。

`ExactHull S lo hi` 的完整内容是：`S lo ∧ S hi ∧ ∀ x, S x → lo ≤ x ∧ x ≤ hi`。即两端均被取到，并包住所有 S 点；没有 `lo ≤ x ∧ x ≤ hi → S x`。独立 probe 证明稀疏集 `{0,2}` 满足 `ExactHull ... 0 2` 且不含 1，说明单凭这个通用谓词不包含“填满整个区间”。本件未声称具体 Support 有空洞；其是否覆盖区间需另证。

`leg pos q r start len` 返回一个无额外证明字段的 `NewChanlun.Origin.Segment`：true 为 up、起价 q.bid、终价 r.ask；false 为 down、起价 q.ask、终价 r.bid；起终索引为 start 与 start+len。`start/len` 是 Nat，时间只体现事件计数索引。leg 不检验 q/r、len 或任一外部资格。

`HULL_TARGET` 对与 SPLIT_TARGET 相同的所有 pos/a/q/b/r/es，先要求 Flow，再量化任意 `start : Nat`，结论是 Support 的 ExactHull 为 `segLow/segHigh (leg pos q r start es.length)`。展开并使用已证弱单调后：

| 极性 | 达到的最低支持价 | 达到的最高支持价 |
| --- | --- | --- |
| true | q.bid | r.ask |
| false | r.bid | q.ask |

这里的 `segLow/segHigh` 仅取 Segment 的两个端点价格之 min/max。最高最低边界不是遍历事件价格后求得，而由 leg 的指定报价端点给定，再证明所有 Support 被包住。hull 证明分别以空前缀与空后缀给出两端可达支持点，经 `flow_monotone` 约束中间报价。

该命题不要求 es 非空。空串 Flow 强制两端账本和报价相同，Support 包含该报价的整个闭区间，ExactHull 仍成立，但 leg 起终索引相等。独立 probe 编译了空串 Flow、对应 HULL 和零索引长度等式。`MovingQuote.LEG_TARGET` 的第一项若要严格时间长度另有 `es ≠ []`，该条件没有出现在 HULL_TARGET。

HULL 的前提不含 Completed，结论不提反向事件、分组结束、分组唯一性、中枢、走势分类或递归层级。它可用于合法但尚未被反向事件关闭的同极性片段。

## 5. LIMIT_TARGET 的固定见证

LIMIT_TARGET 没有输入量词；它是下列固定值的一整组断言。名字不改变它是固定实例这一类型事实。

| 对象 | 实际值 |
| --- | --- |
| deep | bid add，数量 1，价格 90，positive=true |
| reverse | ask add，数量 1，价格 110，positive=false |
| b0 | bids=[100]，asks=[102] |
| b1 | bids=[90,100]，asks=[102] |
| b2 | bids=[90,100]，asks=[110,102] |
| q | bid=100，ask=102 |
| p | i=0 时 deep，其他所有自然数 i 时 reverse |

它逐项证明 deep 的 Step 从 b0 到 b1，reverse 的 Step 从 b1 到 b2；三个状态均 QuoteOf q 且 ValidBook；`Flow true b0 q b1 q [deep]`；`Completed p 2 0 1`；两个分组的 knownAt 是 `[some 2, none]`；上行 leg 的低高是 100 与 102；deep.price=90 属于 b1.bids 且小于 leg 低点；最终 `¬ Support true b0 q b1 q [deep] 90`。

所以实际反例边界是“一个合法、反向事件可关闭的 run 可以含有某一深档事件价，且该价存在于修改后盘口，却不属于本定义的 Support”。这里深档 bid add 不改最优报价；随后深档 ask add 改变事件极性也不改最优报价。命题未量化所有 deep price、所有盘口或所有 run，也未讨论极限过程。

`Completed` 最终展开为 DirectQuote.Completed：`s < e`、`e < n`；s=0 或 s-1 与 s 极性不同；所有 i∈[s,e) 极性等于起点；e-1 与 e 极性不同。e 是 run 右端的排除索引，事件 e 是关闭它的第一条反向事件；e<n 保证它已落入观察前缀。该关系只查看 core 极性，不检查价格、数量有效性或盘口。

`group` 按事件极性合并相邻事件；Block 数据类型保证每组非空。`locate` 从 start 累加事件长度，`firstKnown=start+1`，若后面还有组则 `knownAt=finish+1`，尾组为 none。这是函数按列表位置给出的元数据。LIMIT_TARGET 对指定两事件串计算并证明给定值；它没有给出任意 group/locate 与 Completed 的普遍等价定理。

## 6. 其他导入命题与函数图边界

实际根闭包用了 MovingQuote 的 quote_step、flow_quotes、flow_monotone 等引理，以及 group、locate、Completed 等定义；不依赖 `MovingQuote.exact_root` 或 `DirectQuote.exact_root` 这两个大合取根。读到导入定理存在，不等于本根包含或使用它的全部结论。

- 两版本 RUN_TARGET 都对任意有限事件串，证明 Runs 的分组存在唯一、decode 无损、组内极性一致、组间极性相异。Runs 是归纳关系，group_sound/runs_determined 连接它与 group 函数；不是仅把“group es = out”命名为关系。它们不要求合法订单事件或报价。
- DirectQuote.CAUSAL_TARGET 与 MovingQuote.CAUSAL_TARGET 都是前 n 项输入相同则 Completed 命题等价；它们不是未来价格预测或订单簿因果重建定理。
- DirectQuote.GEOMETRY_TARGET 对给定固定 bid<ask 和非空 Block 构造 leg，证明索引正长度与固定低高；异极性相接时端价相接；三个交替块满足 CenterConfirmedComplete 且固定 core 为 bid/ask。固定价带是参数，未从挂单事件推导。
- MovingQuote.STEP_TARGET 条件于 QuoteOf 与 Step，证明报价唯一及按事件极性的弱单调；没有无条件报价存在性。
- MovingQuote.LEG_TARGET 第一项条件于非空 Flow 才有严格时间长度和方向价格差；第二项相邻反极性 leg 共用 r 时端价相接是对任意报价、长度成立的定义式；第三项固定 q 的 leg 回到 DirectQuote.leg。
- MovingQuote.readout 的输入是外供 `List (Option Quote)`：长度至少 2、全部 some 且 bid<ask、起终方向价差严格时返回 leg，否则 none。它不验证这些报价来自任何 Book/Step/Flow，也不要求整串弱单调。
- MovingQuote.construct 仅在 quotes.length=es.length+1 时返回 some 输出；输出包含完整 located 分组与可为 none 的 geometry。OUTPUT_TARGET 证明“存在这样一个 construct 输出且结构恢复”当且仅当长度匹配，见证 out 是 map 构造结果。这确实含函数图等式 `construct es quotes = some out`，并不保证每个 geometry 都存在、或报价流与事件流物理一致。空 es 加一个任意 option quote 满足长度门，输出 some []。
- DirectQuote.WITNESS_TARGET 固定比较数量 5/10 的 20 事件历史、分组/已知时刻与固定价带中枢。MovingQuote.WITNESS_TARGET 固定六事件/七状态实例，包括一个空 bid 侧、无 QuoteOf、相关 readout 为 none，以及前三 leg 的指定中枢。WIRE_WITNESS_TARGET 给出固定 construct 输出见证，geometry 是否存在为 `[true,true,true,false,false]`。这些是实例，不是任意实盘输入保证。

formal 层实际载体：`Tick := Int`，`Index := Nat`。Segment 只有 direction、两个 index、两个 price，未自带非空时间段、方向价差或连接证明。`CenterConfirmedComplete` 精确等于相邻方向不同与三个价段的 `computeZD < computeZG`；computeZD 是三个端点低值的最大值，computeZG 是三个端点高值的最小值。该谓词没有内置时间先后、连接、结构级别或完成资格字段。这些定义在依赖模块的中枢实例里使用；DirectHull.exact_root 自身的实际闭包不含 CenterConfirmedComplete 或 computeZD/computeZG。

`Origin.ChanlunElements`、`Origin.CenterConstruction` 还存在将既给函数图证明为 TotalUnique 的定理，例如 `centersOf_total_unique`、ElementPipeline 的各阶段 total_unique。这类声明依赖已给函数，不自动保证它符合额外分类语义。它们没有进入 DirectHull.exact_root 实际依赖闭包；不能把这些导入的 total/unique 结论嫁接到本根上，也不能据它们断言本根是函数图同义反复。

## 7. 可复核文件

所有路径以下列目录为基准：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/`。

- `readback-direct-hull/run-manifest.json`：本次完整机器身份与闭包回执。
- `readback-direct-hull/lean-verification-runs/ed74a9224d7b4dcca693709d8713830c/`：独立 fresh 产物、生成抽取源码、命令和完整日志。
- `readback-direct-hull/readback-binding-summary.json`：精简声明身份、全部目标本地/formal 依赖、formal 源码/预编译物哈希、作者回执绑定对照。
- `readback-direct-hull/ReadbackProbe.lean`：实际 #print 与独立边界例证。
- `readback-direct-hull/ReadbackProbe.stdout.log`、`.stderr.log`、`probe-execution.json`：probe 实际执行、完整输出与退出码 0。

本件完成的是声明冷读及其运行身份检查。任意研究目标与这些声明之间是否相符，留给持有原目标的另一个独立比较上下文。
