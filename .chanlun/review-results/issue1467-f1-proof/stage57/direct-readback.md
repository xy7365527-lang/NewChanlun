# Stage57Direct 实际声明读回

实际证明了什么：`Stage57Direct.witness_root` 是两条固定有限事件序列的逐前缀来源及输出字段等式证明。独立重新编译成功。本报告只翻译实际声明及定义，不作原义资格或 B56 签署。

## 实际声明、参数与前提

根声明的实际类型为 `Stage57Direct.WITNESS_TARGET : Prop`。机器提取的 `binder_kinds = []`、`universes = []`：根声明没有自由输入、显式或隐含参数、实例参数、外加前提，也没有宇宙参数。展开后是下述两个命题的合取：

- 对每个 `n : Fin 9`，令 `es = Main.es.take n.val`、`qs = Main.qs.take (n.val+1)`，同时断言 `Through es Main.books es.length`、`Reads Main.books qs`、`qs.length = es.length+1`，以及 `(construct es qs).map (List.map view) = some (Main.expected n.val)`。
- 对每个 `n : Fin 8`，以 `NoneControl.es/books/qs/expected` 作完全相同的断言。

`Fin 9` 与 `Fin 8` 分别只有 0–8、0–7 的自然数值；其小于界限的证明是有限类型自身的字段，不是对任意输入的假设。两域均非空；0 前缀的 `Through` 无事件步骤需要验证，但仍读取初始报价并得到 `some []`。根实际覆盖 9 + 8 = 17 个固定前缀，不量化任意事件流。

这里 `Main.expected` 与 `NoneControl.expected` 是**实际类型直接引用的 Lean 定义**，不是本次审阅使用的外部预期合同。两者都逐项列出常量元组，`prefix_fields` 通过有限分情况与 `rfl` 证明构造器计算结果等于这些常量。

## 实际数据和字段含义

`Event` 含 `core = (bidSide : Bool, action : add/cancel/execute, amount : Nat)` 和 `price : Int`。`Book` 的两侧都是 `List Int`；重复价格项表达重复单位。`Step` 要求数量严格正；add 在该侧列表前接指定数量价格，其他动作要求可删除相邻的指定数量同价项；另一侧保持相等。它不含订单 ID、真实时间戳或交易所协议。

`Through es bs n`：每个 `i < n` 都能从 `es[i]?` 取得事件，且该事件满足 `Step e (bs i) (bs (i+1))`。`Reads bs qs`：`qs` 的每个已有索引都给出相应账本的 `Read`。`Read (some q)` 要求 bid 是买侧最大值、ask 是卖侧最小值，且 `bid < ask`；`Read none` 要求不存在满足该条件的报价。

`positive` 对 bid add 为 true、bid cancel/execute 为 false；ask 的对应符号相反。`group` 依这一布尔值分连续同号事件块。`locate` 为块设置 `start`、`finish = start + 块长`、`firstKnown = start+1`；有下一块时 `knownAt = some (finish+1)`，最后一块为 `none`。这是由事件数定义的字段，读回不额外赋予市场确认含义。

`construct` 先检查报价数量是否比事件数量多一，再给每块计算几何；几何计算读取该块起点至终点的报价窗口，要求至少两个报价、每项均 some 且 bid < ask，并检查严格端点价差。正向几何起价为窗口初始 bid、终价为末尾 ask；负向起价为初始 ask、终价为末尾 bid。索引为 `Nat`，价格为 `Int`。

`view` 精确比较以下数据：`(start, finish, firstKnown, knownAt, positive(first), events(block), geometry.map (direction == up, startIndex, endIndex, startPrice, endPrice))`。因此等式检查字段数值、原事件完整列表、几何 some/none 与 some 时全部五项几何字段。独立打印的 `Direction` 只有 up/down 两个构造子；几何方向在 view 中以布尔值编码。形式命题本身是投影后列表的等式，没有另行声明任意 Output 的等同性定理。

## 两条固定输入实际覆盖的状态

下表事件均为数量 1；`bid+101` 表示买侧 add、价格 101，`bid−101` 表示买侧 cancel、价格 101，`ask+99` 表示卖侧 add、价格 99。

| 输入 | 初始账本 bids / asks | 事件序列 |
|---|---|---|
| Main | `[100,1] / [102]` | bid+101、bid−101、bid+101、bid−101、bid−100、ask+99、bid+2、bid−2 |
| NoneControl | `[100] / [102]` | bid+101、bid−101、bid−100、bid+2、bid+3、bid−3、bid+4 |

Main 的 9 个报价依次为 `(100,102),(101,102),(100,102),(101,102),(100,102),(1,102),(1,99),(2,99),(1,99)`，全部 some。NoneControl 的 8 个报价依次为 `(100,102),(101,102),(100,102),none,(2,102),(3,102),(2,102),(4,102)`；第 3 索引账本买侧空，不存在合法报价。

为完整读回各前缀的字段，下表用 `i…j` 表示该输入第 i 至 j 个事件形成一个块（事件从 1 编号），其来源区间为 `[i−1,j]`、`firstKnown = i`，几何索引在 some 时也是该区间。块后 `@k` 表示 `knownAt = some k`，`@–` 表示 none。`↑a→b` / `↓a→b` 给出几何方向、起价、终价；`无几何` 表示 geometry = none。完整事件内容由上表确定，块符号分别由 positive 决定。

| Main 前缀 n | 输出块及字段 |
|---|---|
| 0 | 空列表 |
| 1 | 1 @– ↑100→102 |
| 2 | 1 @2 ↑100→102；2 @– ↓102→100 |
| 3 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @– ↑100→102 |
| 4 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @4 ↑100→102；4 @– ↓102→100 |
| 5 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @4 ↑100→102；4…5 @– ↓102→1 |
| 6 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @4 ↑100→102；4…6 @– ↓102→1 |
| 7 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @4 ↑100→102；4…6 @7 ↓102→1；7 @– ↑1→99 |
| 8 | 1 @2 ↑100→102；2 @3 ↓102→100；3 @4 ↑100→102；4…6 @7 ↓102→1；7 @8 ↑1→99；8 @– ↓99→1 |

| NoneControl 前缀 n | 输出块及字段 |
|---|---|
| 0 | 空列表 |
| 1 | 1 @– ↑100→102 |
| 2 | 1 @2 ↑100→102；2 @– ↓102→100 |
| 3 | 1 @2 ↑100→102；2…3 @– 无几何（负块） |
| 4 | 1 @2 ↑100→102；2…3 @4 无几何（负块）；4 @– 无几何（正块） |
| 5 | 1 @2 ↑100→102；2…3 @4 无几何（负块）；4…5 @– 无几何（正块） |
| 6 | 1 @2 ↑100→102；2…3 @4 无几何（负块）；4…5 @6 无几何（正块）；6 @– ↓102→2 |
| 7 | 1 @2 ↑100→102；2…3 @4 无几何（负块）；4…5 @6 无几何（正块）；6 @7 ↓102→2；7 @– ↑2→102 |

这些等式具体包含：尚无 knownAt 的末块可以已有几何；反之 knownAt 已 some 的块可以没有几何。NoneControl 的报价恢复后，含缺失报价的两块仍无几何，后续不含缺失报价的块重新有几何。Main 的末尾负块从 n=4 到 5 变更终点价格与跨度，从 5 到 6 保持终价但继续扩展跨度，并到 7 才得到 knownAt。这里只记录列出的前缀数值事实。

## 辅助声明实际证明了什么

`Stage57Direct.primitive_strict` 是独立于根的条件性一般声明。隐含参数是 `es : List Event`、`bs : Nat → Book`、`qs : List (Option Quote)`、`out : List Output`、`x : Output`、`g : Segment`；无类型类实例前提。显式前提为：

1. `Through es bs es.length`；
2. `Reads bs qs`；
3. `qs.length = es.length+1`；
4. `construct es qs = some out`；
5. `x ∈ out`；
6. `x.geometry = some g`。

其结论仅为 `g.startIndex = x.located.start`、`g.endIndex = x.located.finish`、`g.startIndex < g.endIndex`，以及 positive 为 true 时 `g.direction = up ∧ g.startPrice < g.endPrice`，否则 `g.direction = down ∧ g.endPrice < g.startPrice`。没有 `knownAt` 已确认这一前提或结论，也不保证每个输出都有几何。它未进入根的实际依赖闭包；不能把这条一般性条件命题写成根已断言的范围。

`Main/NoneControl.prefix_source` 分别是相应固定输入全部有限前缀的 Through、Reads、长度三项合取；`prefix_fields` 分别是相应投影字段等式。`step1…`、`through`、`reads` 提供固定账本步骤和报价事实。它们是根实际使用的局部声明，而非任意输入的来源完备性命题。

## 独立机器证据及 source / olean 边界

入口所有 9 个带 SHA 的文件记录（含源码、打印、编译目标、提取文件）均逐字节核对通过。入口存在 `expectedType`；原机提取还包含 `comparison` 和 `expected_statement`。本次将这些合同/匹配信息排除，不据其作结论；检查键时曾显示 comparison 的匹配元数据，未使用该匹配结果。源码注释与源文件中无关的叙述也不作语义依据。未读取作者稿、Intent、Progress、旧 verdict 或其他代理消息。使用了本树 math-research-workflow、lean-verify、verification-loop 及其工具说明；未读取项目历史记忆。

独立 run 为 `04e249c751b34b8fb5d35f02b72f49eb`。以同一 Lean 4.31.0（commit `68218e876d2a38b1985b8590fff244a83c321783`）和记录中的 LEAN_PATH 编译，16 个本地模块均从 direct-author/project 的实际 `.lean` 重新编译到本读回目录，所有 Lean 编译命令 exit=0；未覆盖作者源文件或产物。入口给出的 4 份定义源与实际重新编译源哈希相同；目标源亦相同。另以新产物进行独立 #print / #check，exit=0。

根机器提取：actual_type 为 `Stage57Direct.WITNESS_TARGET`；transitive axioms 精确为 `[propext, Quot.sound]`；unexpected_axioms、unsafe_dependencies、unknown_dependencies 均为空。独立打印的辅助 `primitive_strict` 公理闭包为 `[propext, Classical.choice, Quot.sound]`。两者均没有 sorryAx；辅助的 choice 不能归给根。

未指定 expected type。验证器报告 `machine_verification_passed=true`、`declaration_closed=true`、`root_closure.status=declaration_closed_uncompared`、`exact_root_passed=false`、`semantic.status=not_reviewed`。本次 `--strict-exit` 导致 CLI exit=1，原因是没有比较合同，不是 Lean 编译失败；因此不拿 exact-root 标签替代实际编译证据。

`formal/Origin` 是本次使用的外部预编译依赖，未从 Lean 源重建。特别是 `Origin.ChanlunElements.olean` SHA 为 `0c3721be5b3f77ae503583ffd9fecf94f4471e12fdc4cc04619f680d3373274b`；源文件 SHA 为 `19c33e884e6949cdd56a51ff7c44dc086354994b7f54431f57bec4a0849fa201`。独立打印支持本报告使用的 Segment/Direction 字段与类型，但不建立整份 Origin 源和 olean 的逐字构建对应。`Origin.SourceAxioms.olean` 也被载入，其文件 SHA 为 `6afd004ad95850a7d931a02b9ca827d83c6dbf8a6f8019b05f34da96d8b628c0`；载入模块不能直接等同于根使用其全部假设，根的实际公理集仍以上述收集结果为准。完整加载模块及哈希在 manifest 中；工具记录 2282 个加载模块。

本证据依赖当前 Lean 编译器及其已构建库，不是第二个内核实现的独立验证。

## 没证明什么

根没有证明任意输入或无限延续上的生命周期性质、任意追加后所有既有字段永久不变、任何通用确认/撤销规则，或每个同号块都存在几何。未量化价格、数量、初始账本或动作的各种情况：两样本数量都为 1，未使用 execute，未穷尽无报价或非法报价的成因。

根没有陈述中枢、背驰、买卖点、线段资格、完整走势分类、成交可行性、收益、生产实现行为或原文语义对应。仅用了 `Segment` 数据载体，并不由其名称自动获得其他数学或交易含义。Through/Reads 为固定源数据建立模型内事实，不是外部行情真实性、整个生产账本实现或输入适配层的证明。geometry 的严格端点方向由辅助条件声明及具体元组支持，不是所有中间报价逐点严格单调的结论。

本读回没有比较任何外部预期合同，也不签 B56、原义资格或全项目验收。
