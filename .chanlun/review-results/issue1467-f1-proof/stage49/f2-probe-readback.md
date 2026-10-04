# Stage49 F2：Lean 声明冷读

本报告只还原 `f2-contract-evidence/NonNormProbe.lean` 及实际导入定义的数学内容。共有 **10 个 theorem**：前九个是固定整数样例上的封闭命题，第十个是在严格递增端点前提下的整数等价式。样例证明端点投影与列表极值可以不同；它们没有建立完整 Move 资格。`CenterConfirmedComplete` 的实际定义只有方向交替与严格核心不等式。

采用 `.agents/skills/lean-verify/SKILL.md` 的声明、前提、依赖和证据分离口径；未读取输入契约、Intent-v1、其他 Stage49 作者稿或 Rust 源码，未把源码注释或 receipt 的非 Lean 输出作为数学依据。知识图谱没有此工作区索引，故直接读指定导入源码。没有运行 exact verifier，也没有在本轮重新运行 Lean 编译；下述机器事实均明确归于保存的 receipt。

## 载体与定义

下列路径均相对于工作区 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。探针简称 `N = .chanlun/review-results/issue1467-f1-proof/stage49/f2-contract-evidence/NonNormProbe.lean`。

- `Direction` 只有 `up/down` 两值（`formal/Origin/SourceAxioms.lean:53–56`）。`Tick := Int`、`Index := Nat`（`formal/Origin/ChanlunElements.lean:13–14`）。整数不是浮点数、实数或正价格子类型。
- `Segment` 仅含方向、两个自然数索引、两个整数价格，**没有证明字段**（`ChanlunElements.lean:43–49`）。类型本身不保证索引递增、方向与价格一致、端点是路径极值、来自某列表、连续、同级或已完成。`Move` 是另一结构，含 kind、索引、价格、中心列表（同文件 `81–88`）；这 10 条声明没有 Move 参数或 Move 完成谓词。
- `fullRange : List Int → Option (Int × Int)`（`N:12–14`）：空列表为 `none`；非空列表以首元素为初值，对余项分别折叠 min、max，给出 `some (最小值,最大值)`。不存在以 0 补值的分支。它只返回范围，不保留时间顺序、极值所在位置或访问次数。整数对不是一个显式区间集合，也不自带连续插值。
- `Norm s p : Prop`（`N:25–28`）：若标记为 up，要求 `fullRange p = some (s.startPrice,s.endPrice)` 且 `s.startPrice < s.endPrice`；down 则交换价格并要求 `s.endPrice < s.startPrice`。它不要求列表首末元素分别等于 Segment 边界，不涉及索引、路径拼接或 Move 完成。空列表不满足任何 `Norm`；同价端点也因严格不等式不满足。
- `segLow/segHigh` 只取 Segment **两个价格字段**的 min/max，忽略 direction、索引与所有列表（`formal/Origin/CenterConstruction.lean:52–57`）。`tmin/tmax` 是整数条件表达式（`69–70`）。`computeZD = max(三段低)`，`computeZG = min(三段高)`，`computeDD = min(三段低)`，`computeGG = max(三段高)`（`92,99,104,108`）。若把每对端点解释为闭整数区间，前二者是交集候选边界；后二者是外包范围边界。
- `DirAlternates s₁ s₂ s₃ := s₁.direction ≠ s₂.direction ∧ s₂.direction ≠ s₃.direction`（`formal/Origin/CenterComplete.lean:83–84`）。
- `CenterConfirmedComplete s₁ s₂ s₃ := DirAlternates s₁ s₂ s₃ ∧ computeZD s₁ s₂ s₃ < computeZG s₁ s₂ s₃`（同文件 `147–149`）。**Complete 在此没有额外语义参数或隐藏前提**：不检查索引连接、价格连接、列表存在、实际全程极值、三段已完成或次级别关系。`TrueCenterCore` 也是字面相同的两项合取（`158–160`），名称或二者等价都不能补入那些资格。

严格比较 `<` 拒绝下沿等于上沿的单点交集；不能把这种拒绝说成“闭区间交集为空”。在整数域，`lo < hi` 也不能一般性解读成存在严格位于两端之间的整数（相邻整数即不满足后者）。另一个结构 `Center.valid` 仅要求 `zd ≤ zg`（`ChanlunElements.lean:51–56`）；本探针的否定指向 `CenterConfirmedComplete`，并未否定单点 `Center` 可构造。

## 固定数据与连接

`N:7–9,16–23` 定义以下对象；括号中的“端点范围”为按定义展开得到的值。

| 名称 | 整数列表 | actual：方向；索引；起价→终价 | range：方向；索引；起价→终价 |
|---|---|---|---|
| A | `[10,10,10,10,30,20,20,20]` | up；0→7；10→20（10,20） | up；0→7；10→30（10,30） |
| B | `[20,25,5]` | down；7→9；20→5（5,20） | down；7→9；25→5（5,25） |
| C | `[5,28,10]` | up；9→11；5→10（5,10） | up；9→11；5→28（5,28） |

A 中连续四次 10 与末尾三次 20 是列表里的独立位置；未去重，也未定义成平价折返点、笔或其他实体。列表元素没有附带时间索引。样例的长度和 Segment 索引差虽可人工对应，但代码没有从索引取样或由列表构造 Segment 的定义。

actual 的价格首末值确实与各列表首末值一致，然而没有独立定理以一个通用关联谓词表达这件事。连接声明严格指向四个字段等式：`7=7`、`9=9`、`20=20`、`5=5`，未定义任意两段的 `Connected` 关系，也未构造拼接列表。range 保持索引和方向；第一处价格接缝变成 `30 ≠ 25`，第二处仍为 `5=5`。

## 逐条还原

完整声明命名空间为 `Stage49.F2InputContract`。第 1–9 条均为 `Prop`，没有量化参数、显式前提或隐藏完成性前提，证明体使用 `decide`（第 6、8 条先展开 `CenterConfirmedComplete`）。

| # | 声明与源码位置 | 实际证明内容 | 不证明的内容 |
|---|---|---|---|
| 1 | `real_ranges`，`N:34–36` | 三个固定列表的 fullRange 依次为 `some (10,30)`、`some (5,25)`、`some (5,28)`。 | 未证明任意路径上的极值定理，也未赋予这些列表完整走势身份。 |
| 2 | `actual_all_non_norm`，`N:38–39` | `¬Norm actualA pathA ∧ ¬Norm actualB pathB ∧ ¬Norm actualC pathC`。本例每段方向价格严格关系均成立，但相应 fullRange 等式失败：A 高为30非20，B 高为25非20，C 高为28非10。 | 非 Norm 不等于不完成、非法 Segment、方向不交替或不可连接；Norm 也不是完整 Move 的定义。 |
| 3 | `actual_boundaries_connect`，`N:41–43` | actual 的两处索引相等与两处价格相等，共四项合取。 | 未证明任意轨迹连续性、列表拼接正确性或 range 对象连接。 |
| 4 | `actual_directions_alternate`，`N:45` | up≠down 且 down≠up。 | 未证明 direction 标记对真实走势方向的普遍忠实性、核心存在或完成性。 |
| 5 | `endpoint_projection_loses_range`，`N:47–50` | actual 两点投影 `(10,20)`、`(5,20)`、`(5,10)` 分别不等于全列表范围 `(10,30)`、`(5,25)`、`(5,28)`。 | 只给三个样例；未证明所有端点投影均失真，也未给出对完整走势域的普遍反例。 |
| 6 | `endpoints_reject_at_singleton`，`N:52–56` | actual 的 ZD=10、ZG=10，并且 `¬CenterConfirmedComplete actualA actualB actualC`。方向交替已成立，失败处是 `10<10`。 | 不是交集为空：闭区间解释下交集恰为 `{10}`；也不是任意中枢/Move 判据的否定。 |
| 7 | `range_encoding_is_exact`，`N:58–61` | 三个列表的 fullRange 分别等于 range 对象的 `(segLow,segHigh)`。 | “exact”限于这三个范围对相等；不包含实际边界、路径顺序、连接或完成性保真。 |
| 8 | `ranges_pass_local_criterion`，`N:63–68` | range 的 ZD=10、ZG=25、DD=5、GG=30，且其方向标记交替并满足 `10<25`。 | 未证明它们保持真实边界或互相连接，也未证明被检查对象是完整 Move/次级别走势。 |
| 9 | `range_encoding_changes_boundary_and_breaks_connection`，`N:70–72` | `rangeA.endPrice ≠ actualA.endPrice`（30≠20）、`rangeB.startPrice ≠ actualB.startPrice`（25≠20）、`rangeC.endPrice ≠ actualC.endPrice`（28≠10），以及 `rangeA.endPrice ≠ rangeB.startPrice`（30≠25）。 | 不是所有连接均破坏：索引连接不变，B→C 价格仍为5；也未证明任何可能的范围载体设计都必然破坏连接。 |
| 10 | `upward_endpoint_projection_iff`，`N:75–80` | 对任意 `s e lo hi : Int`，在 `s<e` 下，`((if s≤e then s else e)=lo ∧ (if s≥e then s else e)=hi) ↔ (s=lo ∧ e=hi)`。即严格递增时两个条件表达式分别化简为 s、e。 | 声明没有 Segment、path、fullRange、方向标记或 Norm；没有证明 `Norm ↔ 完成性` 或任何适配器正确性。未覆盖 s=e 或 s>e 的定理实例。 |

第 10 条的域非空且前提可满足，例如 s=0、e=1；没有发现用不可满足前提获得结论的情况。前九条固定样例也不是空域的全称断言。

从以上**定义与已列命题**可直接得到的有限结论是：此组 actual 连接且方向交替，但其端点范围遗漏列表内部高价，严格核心判据拒绝；另一组 range 复现列表极值且局部判据接受，但它改写价格边界并失去第一处价格连接。这不自动给出完整走势范围上的接受/拒绝差异，因为那一域与成员资格未在探针中定义。

## 保存的机器输出与本轮核对

`f2-contract-evidence/receipt.txt:9–27` 的十个 `#check @...` 输出与上述类型一致，前九条无参数，第十条显式量化四个 Int 并要求 `s<e`。`:28–36` 报告前九条不依赖任何公理；`:37` 报告第十条依赖 `[propext, Quot.sound]`。这些是保存的逐目标公理输出，不能用导入文件名 `SourceAxioms` 推断其反面；本报告未独立重跑公理闭包。

`source-build.json` 记录的方法是直接源码构建、目标 elaboration/olean 生成及 `#check/#print axioms`，明确不是 `verify_lean_project`；记录 `lean_root_exit=0`，Lean 4.31.0，环境 head 为 `2e3551802306b704e759631487a75498b3b08b12`。head 是证据中的绑定值，本轮未运行 Git 来核对。记录称临时 olean 已删除，故这里只能核对源文件与工具哈希，不能重新核对已删除产物。

本轮逐一计算 SHA-256，确认探针、七个项目导入源码、`formal/lean-toolchain` 和记录路径下 Lean 可执行文件均与 `source-build.json` 对应摘要一致。项目导入闭包为：CenterComplete → ChanlunElements / CenterStates / CenterFull / CenterConstruction；后三个几何模块最终依赖 ChanlunElements → CompleteClassification → SourceAxioms。隐式 Lean 基础库环境没有由这七份项目源码哈希穷尽；可执行文件哈希也不等于标准库所有产物哈希。

关键输入 SHA-256：

- `NonNormProbe.lean`：`d1d58f53aefe1ca0bf1272b73d928272e2b5b11311f7c44a2f3b4399071441cd`
- `receipt.txt`：`72700c16d49af9781579075faefd762b7008aaba014f5ded09b4f5ab7eba6b2a`
- `source-build.json`：`ec15f826d041885a50f400e65a63db9ac6eafd2be7082cc55aecee4f8d693709`

其余输入的绝对路径、字节数、当前摘要及匹配结果在同目录 `f2-probe-readback-input-hashes.json`。本报告交付的是声明语义冷读及保存证据与当前源码的哈希绑定；**没有交付独立 exact 根验证通过结论**。
