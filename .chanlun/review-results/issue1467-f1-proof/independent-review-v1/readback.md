# #1467：`ReferenceBase.exact_root` 独立声明读回

本报告仅回答声明实际上表达并证明了什么，不判断它是否符合作者的研究目标。检查日期：2026-10-04。源码工作树：`/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。

读回输入为指定 `ReadbackTask.md`、`stage40-review-manifest.json` 的 **readback** 项，以及沿真实 `import` 到达的 `formal/Origin/` 定义。未读取 manifest 的 comparison 项、作者报告、ResearchProgress 或旧会话。manifest 中的 8 个 readback 文件 SHA-256 全部匹配。编译在仓外临时目录完成，源文件与 fixture 未修改。

## 1. 声明的完整外形与量词

实际声明为：

```lean
ReferenceBase.exact_root :
  ReferenceBase.SOURCE_TARGET ∧
  ReferenceBase.STROKE_TARGET ∧
  ReferenceBase.FEATURE_TARGET ∧
  ReferenceBase.GEOMETRY_TARGET
```

它没有数据参数、时间参数、类型参数或前提箭头。四个命题引用的是同一命名空间内已经定义好的常量；没有对任意价格列表、任意数量列表、任意笔列或任意三段作全称量化。证明只组合 `source_checked`、`strokes_checked`、`features_checked`、`geometry_checked` 四个证明（[Proof:22–55](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean:22)）。

这是针对一份固定有限输入的 Lean 定理。这里实际使用 `List Int` 和 `List Stroke`，不是 Lean `Array`；命题内部的 `Fin n` 是有界全称量词。概括为“有限数组检查”时应保留这一类型区别。源码中的通用函数和可判定性实例可以接收一般输入，但这不把 `exact_root` 变成关于一般输入的正确性、完备性或存在性定理。

内部量词全部如下：

| 合取项 | 有界量词及范围 |
|---|---|
| `SOURCE_TARGET` | `Fin 95`，即 `i=0,…,94`；`Fin 94`，即 `i=0,…,93` |
| `STROKE_TARGET` | `Fin 94` 的相邻价格；`Fin 23` 的各笔；`Fin 22` 的相邻笔 |
| `FEATURE_TARGET` | 三个 `NoAdjacentContainment`，量化过滤后特征元素的所有相邻对：分别 3、4、3 对 |
| `GEOMETRY_TARGET` | 三个 `SpanSource`，分别量化 5、7、5 根笔；三个索引界检查分别量化前 8、15、20 根笔 |

## 2. 数据域和直接给定的对象

`Tick = Int`，`Index = Nat`。`Direction` 只有 `up`、`down`；`flip` 交换它们。`Stroke` 和 `Segment` 都是五字段结构：方向、起始索引、终止索引、起始价、终止价，其结构类型自身不携带合法性证明（[ChanlunElements:13–49](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/ChanlunElements.lean:13)；[SourceAxioms:53–60](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SourceAxioms.lean:53)）。

下列值由定义直接写入：

- `prices` 是 95 个整数的字面列表；`bidQuantities` 是 95 个整数的字面列表；`askQuantity = 11068016086592845380000`。价格与数量都先给定，证明随后检查二者的方程关系（[Fixture:4–6](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseFixture.lean:4)）。
- `strokes` 是 23 个 `Stroke` 的字面列表，并非本声明从价格列表运行某个笔生成函数所得（[Fixture:7–29](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseFixture.lean:7)）。
- 三个 `Segment` 直接给定为下表；三个证据窗口也通过固定 `take/drop` 给定（[Spec:37–42](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:37)）。

| 段 | 方向 | 起止索引 | 起止价 | `SpanSource` 检查的笔下标 | 提取特征时使用的笔下标 |
|---|---|---|---|---|---|
| `s0` | up | 1 → 21 | 10000 → 11000 | 0…4，共 5 根 | 0…7，共 8 根 |
| `s1` | down | 21 → 49 | 11000 → 10100 | 5…11，共 7 根 | 5…14，共 10 根 |
| `s2` | up | 49 → 69 | 10100 → 10800 | 12…16，共 5 根 | 12…19，共 8 根 |

`price i`、`qty i` 在越界时返回 0；`stroke i` 越界时返回 `⟨up,0,0,0,0⟩`（[Spec:8–10](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:8)）。这使函数对所有 `Nat` 都有值。当前有限目标检查的主要下标均在字面列表范围内；声明并没有把越界访问定义成错误或一般前置条件。

## 3. `SOURCE_TARGET` 实际证明的条件

令 `Pᵢ = price i`，`Qᵢ = qty i`，`A = askQuantity`。声明检查：

1. 两份列表长度均为 95，且 `A > 0`。
2. 对 `0 ≤ i < 95`：`Qᵢ > 0`、`9900 < Pᵢ < 11200`，并且整数等式
   `11200 Qᵢ + 9900 A = Pᵢ (Qᵢ + A)` 成立。
3. 对 `0 ≤ i < 94`：`Qᵢ₊₁ ≠ Qᵢ`；若 `Qᵢ₊₁ < Qᵢ`，则 `0 < Qᵢ − Qᵢ₊₁ < Qᵢ`。

出处：[Spec:12–18](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:12)。证明展开 `SOURCE_TARGET price qty` 后用 `decide` 关闭（[Proof:22–24](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean:22)）。

因此这里证明了固定数字满足一个加权关系与相邻数量约束。Lean 声明内没有除法式，也没有从任意输入数量生成价格的算法及其正确性定理。数量下降的蕴含只是一条数值不等式，没有把下降解释为某个事件执行结果的语义结构。

## 4. `STROKE_TARGET` 实际证明的条件

`top i` 只表示 `Pᵢ₋₁ < Pᵢ` 且 `Pᵢ₊₁ < Pᵢ`；`bottom i` 取两条反向不等式。这里的 `i-1` 是自然数截断减法；`StrokeSource` 另要求起点大于 0，从而当前笔的端点检查不依赖 `i=0` 的截断行为（[Spec:20–27](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:20)）。

对固定 23 根笔，证明如下条件：

- `strokes.length = 23`，95 个价格中每一对相邻值都不相等。
- 第 `i` 根笔起点为 `1+4i`、终点为 `5+4i`。每根笔满足起点大于 0、起止间距严格大于 3、终点的后一项仍在价格列表内，端点价格与价格列表相符。
- 向上笔从严格邻域底到严格邻域顶且终价更高；向下笔相反。
- 相邻笔共享索引和价格端点，方向不同。

出处：[Spec:22–35](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:22)；证明为有限判定（[Proof:26–28](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean:26)）。

这给定了从索引 1 到 93 的交替相接笔列，并验证指定局部端点性质；没有声明所有局部分型都被选入、每笔内部没有可替代端点、笔分解唯一，或该笔列等于某个通用笔解析器的输出。`Bar` 的 OHLC 结构没有进入这些命题。

## 5. `FEATURE_TARGET` 的计算、选择与确认

`extractFeatureStrokes d` 保留方向为 `d.flip` 的笔；`FeatureElem.ofStroke` 把每根笔变成 `[min(起价,终价), max(起价,终价)]`，结构中仅保留 `low`、`high` 与 `low ≤ high` 的证明，不保留笔索引（[SegmentAutoConstruct:40–45](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentAutoConstruct.lean:40)；[SegmentFeatureSeq:61–76](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureSeq.lean:61)）。

`buildFeatureSeq` 对该列表作相邻包含合并。`Contains b a` 即 `b.low ≤ a.low ∧ a.high ≤ b.high`；向上处理取两端较大值，向下处理取较小值（[SegmentAutoConstruct:69–87](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentAutoConstruct.lean:69)；[SegmentFeatureSeq:143–166](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureSeq.lean:143)）。本定理对三份输入另外检查了原始特征元素所有相邻对都互不包含，并检查标准化后的数值列表精确等于：

| 输入 | `pairs` 的结果 |
|---|---|
| `input0`，方向 up | `(10300,10500), (10600,10800), (10700,11000), (10500,10900)` |
| `input1`，方向 down | `(10700,10900), (10500,10700), (10300,10500), (10100,10400), (10200,10600)` |
| `input2`，方向 up | `(10200,10400), (10400,10600), (10500,10800), (10300,10700)` |

出处：[Spec:43](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:43)、[Spec:59–76](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:59)。这三个实例未行使实际包含合并分支；不能从本定理推出该合并算法对任意含包含输入正确。

`d0/d1/d2` 也是先定义的对象：其三元素分别来自笔 `(3,5,7)`、`(10,12,14)`、`(15,17,19)`，`revSeq` 全部直接设为 `[]`，方向与起止价也直接写入（[Spec:44–49](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:44)）。

`extractSegEndData` 构造特征序列，再找第一处所需分型：顶分型只比较中间元素的 high 严格大于两邻；底分型只比较中间元素的 low 严格小于两邻。函数把传入的方向、起价和终价原样放入结果；并不自行求出这两个端点价格（[SegmentAutoConstruct:105–123](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentAutoConstruct.lean:105)；[SegmentAutoConstruct:177–192](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentAutoConstruct.lean:177)）。

`SelectedBy d ss` 证明提取结果不是 `none`，且结果的方向、起价、终价、三个元素的 low/high 分别与 `d` 相同。它**没有比较 `revSeq`，也没有断言整个 `SegEndData` 结构相等**（[Spec:51–57](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:51)）。当前三次扫描均在给定特征列表末端找到所选三元组；但关于所有字段的相等不是 `SelectedBy` 的声明内容。

`SegEndComplete d` 展开为三项合取：所需顶/底分型；按 e1/e2 是否有缺口决定是否需要后续对偶分型；符合方向的起止价严格关系（[SegmentFeatureComplete:81–110](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureComplete.lean:81)）。`HasGap a b` 表示 `a.high < b.low ∨ b.high < a.low`。无缺口时 `SegmentEndUp/Down` 的该分支直接为 `True`（[SegmentFeatureSeq:102–103](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureSeq.lean:102)；[SegmentFeatureSeq:290–324](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureSeq.lean:290)）。

本定理证明三次均无缺口，且三个 `SegEndComplete` 都成立；还额外检查上行顶分型的 low 也严格居中最高、下行底分型的 high 也严格居中最低。三组具体元素为：

| 对象 | e1 | e2 | e3 | 端点方向条件 |
|---|---|---|---|---|
| `d0` | `[10600,10800]` | `[10700,11000]` | `[10500,10900]` | `10000 < 11000` |
| `d1` | `[10300,10500]` | `[10100,10400]` | `[10200,10600]` | `11000 > 10100` |
| `d2` | `[10400,10600]` | `[10500,10800]` | `[10300,10700]` | `10100 < 10800` |

因此三个实例都使用无缺口分支；没有用这些实例验证有缺口时的后续对偶分型确认。`features_checked` 分别用 `decide` 证明三个确认命题，再用 `decide +kernel` 完成其余有限合取（[Proof:30–48](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean:30)）。

## 6. `GEOMETRY_TARGET`、组成关系与时间索引

`SpanSource s startAt count` 要求 `count ≥ 3` 且为奇数；所选连续笔区间的首笔起点和末笔终点等于段的两个索引；段端点价等于原价格列表值；区间内每根笔的两端价均落在段端点价所围闭区间中（[Spec:78–86](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:78)）。它检查的是笔端点，不直接量化每一项原始价格。

`initialCore startAt` 检查从该笔下标起前三根笔的区间交集严格非空，即 `max(三低) < min(三高)`。本定理在下标 0、5、12 各检查一次（[Spec:88–96](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:88)）。

三段共覆盖前 17 根笔，即笔 0…16；声明没有要求把全体 23 根笔全部分解为线段。三段共享两处边界索引和价格，段终价分别等于 `d0.e2.high`、`d1.e2.low`、`d2.e2.high`（[Spec:95–99](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:95)）。这把事先给出的段终价和被选分型极值联系起来。

时间方面只有自然数索引，实际明写三条界：

- 前 8 根笔的 `endIndex < 38`；
- 前 15 根笔的 `endIndex < 66`；
- 前 20 根笔的 `endIndex < 86`。

出处：[Spec:100–102](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:100)。由已证 `endIndex=5+4i`，这些前缀中最大终点分别为 33、61、81；三个特征提取窗口的末笔也分别结束在这三个索引。三段自身的终点则是 21、49、69。必须区分段终点、证据窗口终点与三个人工给定的界 38、66、86。

声明没有给出真实时间戳、事件间隔、确认时间函数，或关于每个时间前缀的全称命题；没有证明上述三个界是最早可确认时刻，也没有量化未来延展下结论保持不变。这里只能读成固定自然数坐标上的不等式与固定窗口检查。

最终 `CenterConfirmedComplete s0 s1 s2` 只有两项：相邻段方向不同，以及三段共同区间严格非空（[CenterComplete:83–84](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterComplete.lean:83)；[CenterComplete:147–149](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterComplete.lean:147)）。段 low/high 取两端价最小/最大；`computeZD` 取三低的最大值，`computeZG` 取三高的最小值（[CenterConstruction:52–70](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterConstruction.lean:52)；[CenterConstruction:92–99](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterConstruction.lean:92)）。

在当前三个固定段上，区间分别是 `[10000,11000]`、`[10100,11000]`、`[10100,10800]`。证明方向为 up/down/up，且交集为 `[10100,10800]`，因此 `10100 < 10800`；两个精确边界值也是定理的显式合取项（[Spec:103](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean:103)）。`GEOMETRY_TARGET` 的证明仍为展开后有限判定（[Proof:50–52](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean:50)）。

这里没有声明三段等于 `segmentsOfComplete strokes` 的输出，也没有声明中枢是某个通用 parser 的最终输出、所有可能中枢中的唯一一个，或任意更高层级递归后仍满足这些性质。`CenterConfirmedComplete` 本身不检查时间连续性；当前时间相接由本文件另外列出的合取项提供。

## 7. 已证范围、未进入类型的信息与外部假设

已证范围是：**这一份固定整数序列、固定笔列、固定三段与三个固定特征窗口，满足四个明确列出的合取条件。** 其中特征提取、分型扫描及边界计算确实执行了导入函数；并非所有结果都只由重命名得到。但价格/数量列表、笔端点、段端点、窗口范围和候选数据本身都先给定，没有从统一外部数据模型自动推出。

`exact_root` 没有外部命题前提，也没有量化订单对象、订单 ID、账户、交易场所、订单状态、增删改事件、撮合/成交、事件到状态的转移函数、实际时钟、价格/数量单位、任意输入族或层级参数。因此不能将它读成这些信息之间的一般关系证明。列表名称本身不增加这些数学对象或语义。

没有进入声明的性质还包括：一般构造算法的正确性/完备性、分解唯一性、全数据覆盖、所有前缀的在线一致性、未来不重绘、任意含包含或有缺口实例的正确处理。这里的“未证明”指目标类型及其定义没有声称这些性质，不是对它们真假的判断。

独立 `#print axioms` 读回显示：

```text
'ReferenceBase.exact_root' depends on axioms: [propext, Quot.sound.{u}]
'ReferenceBase.source_checked' depends on axioms: [propext]
'ReferenceBase.strokes_checked' depends on axioms: [propext]
'ReferenceBase.features_checked' depends on axioms: [propext, Quot.sound.{u}]
'ReferenceBase.geometry_checked' depends on axioms: [propext]
```

这些是本次实际编译后的依赖输出；未出现 `sorryAx` 或项目自定义语义公理。没有把 import 文件中其他声明的公理误算为当前定理依赖。此检查确认 Lean 对当前闭合命题的证明接受情况，不核定命名、注释或领域解释是否合适。

## 8. 实际检查与局限

1. 用 `python3` 读取 manifest 的 `['readback']`，对 8 个文件执行 `hashlib.sha256(path.read_bytes()).hexdigest()`，逐项与 manifest 的 `sha256` 比较，全部匹配；写报告前再次执行同一比较。
2. 按文件真实 `import` 递归收集 14 个模块，只复制允许的三个 ReferenceBase 文件和 `formal/Origin/` 导入闭包；未复制或复用仓内 `.olean`。
3. MCP `search_graph(project="NewChanlun", file_pattern="formal/Origin/.*", name_pattern="^(Direction|flip|computeZD|computeZG|segLow|segHigh)$", limit=20)` 返回 `project not found or not indexed`。随后按已知 import 路径直接读源码并标行号；未建立或修改图索引。
4. 仓外构建目录：`/var/folders/ft/1lyklpls4n16xj54q4l148qm0000gn/T/issue1467-readback-nrqbt_sz`。每次子进程 cwd 都是此目录，环境 `LEAN_PATH` 也设为此目录。实际工具链版本为 `Lean 4.31.0, arm64-apple-darwin24.6.0, commit 68218e876d2a38b1985b8590fff244a83c321783`。
5. 下列实际命令均由 Python `subprocess.run` 执行；所有模块编译和最终探针均退出 0。编译开始与结束时比较了复制来源的内容 SHA，14 个源文件均未变化。

```text
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean --version
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/SourceAxioms.olean Origin/SourceAxioms.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/CompleteClassification.olean Origin/CompleteClassification.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/ChanlunElements.olean Origin/ChanlunElements.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o ReferenceBaseFixture.olean ReferenceBaseFixture.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/SegmentFeatureSeq.olean Origin/SegmentFeatureSeq.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/SegmentConstruction.olean Origin/SegmentConstruction.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/SegmentFeatureComplete.olean Origin/SegmentFeatureComplete.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/SegmentAutoConstruct.olean Origin/SegmentAutoConstruct.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/CenterStates.olean Origin/CenterStates.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/CenterFull.olean Origin/CenterFull.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/CenterConstruction.olean Origin/CenterConstruction.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o Origin/CenterComplete.olean Origin/CenterComplete.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o ReferenceBaseSpec.olean ReferenceBaseSpec.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean -o ReferenceBaseProof.olean ReferenceBaseProof.lean
/opt/homebrew/bin/elan run leanprover/lean4:v4.31.0 lean ReadbackProbe.lean
```

探针文件在仓外生成，内容为：

```lean
import ReferenceBaseProof
set_option pp.universes true
#check @ReferenceBase.exact_root
#print axioms ReferenceBase.exact_root
#print axioms ReferenceBase.source_checked
#print axioms ReferenceBase.strokes_checked
#print axioms ReferenceBase.features_checked
#print axioms ReferenceBase.geometry_checked
```

逐模块退出码、耗时、完整 stdout/stderr 与命令参数保存在本次临时文件 [verification.json](/var/folders/ft/1lyklpls4n16xj54q4l148qm0000gn/T/issue1467-readback-nrqbt_sz/verification.json)，探针输出保存在 [ReadbackProbe.log](/var/folders/ft/1lyklpls4n16xj54q4l148qm0000gn/T/issue1467-readback-nrqbt_sz/ReadbackProbe.log)。临时目录可能由系统清理；本报告保留结论、实际命令和源哈希。

没有执行生产程序、通用解析器对拍、原文语义审查或 fixture 漂移检查；本次没有修改 `formal/`，不将这些未执行事项记作通过。导入闭包整模块被编译；语义读回仅针对目标声明及其实际所用定义。

## 9. 源码 SHA-256

前 8 项为 manifest 的 readback 清单；其余为实际 import 闭包中本次读取/编译的补充文件，补充项的哈希由本次独立计算，并非 manifest 提供。

| 源文件 | SHA-256 | 绑定来源 |
|---|---|---|
| [.chanlun/review-results/issue1467-f1-proof/ReferenceBaseFixture.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseFixture.lean) | `bf89332a05592d0010a3107433887b2798a7ed297dd5755740862d912ef9082d` | manifest 匹配 |
| [.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseSpec.lean) | `b65a89ee4042504c87c1e92eb3af83607765636f894ba5cdfadbc1cee33c2918` | manifest 匹配 |
| [.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/ReferenceBaseProof.lean) | `a11d0bd08f3e34e1b533c02eb4e657a6ff626cd4a72e121b841547175ee7a4d1` | manifest 匹配 |
| [formal/Origin/ChanlunElements.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/ChanlunElements.lean) | `19c33e884e6949cdd56a51ff7c44dc086354994b7f54431f57bec4a0849fa201` | manifest 匹配 |
| [formal/Origin/SegmentAutoConstruct.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentAutoConstruct.lean) | `3ffd1f05eb003e812c4c6a68327bec3897e5f432bdc39cf80f498e7245df026c` | manifest 匹配 |
| [formal/Origin/SegmentFeatureSeq.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureSeq.lean) | `4f727a53f24ee548abd340ab6894be5f976d21ef4bd1d12bde18f5e3973429dc` | manifest 匹配 |
| [formal/Origin/SegmentFeatureComplete.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentFeatureComplete.lean) | `c740b9a77bbda167f6ef3d08f2ad3d24dbe0ba0f617d5eedc0f88ab1bd07ef7e` | manifest 匹配 |
| [formal/Origin/CenterComplete.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterComplete.lean) | `478e34e703801742e6808c834488b7fe009d94bb0d896eb65e7d6a20105d64c2` | manifest 匹配 |
| [formal/Origin/SourceAxioms.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SourceAxioms.lean) | `2154cb85c1ac80685e8d2091259818e3543ad76dfeeb08640df0ae236d39ff6f` | 真实 import 补充 |
| [formal/Origin/CompleteClassification.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CompleteClassification.lean) | `066f1feaec8f6e8a93a4f3c4a516bff51af31689e382d20964e06526fa831b26` | 真实 import 补充 |
| [formal/Origin/SegmentConstruction.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/SegmentConstruction.lean) | `b945d45b5e6466631a34559a5feab87594d7461118dea36e61094b6d6925c374` | 真实 import 补充 |
| [formal/Origin/CenterStates.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterStates.lean) | `8355eaba2ca8b0f1ef50b8167802ce2f5c6abdab188dd3f49ea25859936a1a0a` | 真实 import 补充 |
| [formal/Origin/CenterFull.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterFull.lean) | `00b40dd9f75756edb1a3155c04f301de9c8986bb148f753b56e7707686d73959` | 真实 import 补充 |
| [formal/Origin/CenterConstruction.lean](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterConstruction.lean) | `478a00a239e77b6ac66554c8428efaff7f1af69f9edbf73f2f4250aaa895bc3c` | 真实 import 补充 |
