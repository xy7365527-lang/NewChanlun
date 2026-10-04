# #1467 Stage46 DirectHull 独立语义比较

日期：2026-10-04。结论：**可接受为此次限定的 L1 几何合同及深档事件价反例。未发现阻断这份合同的声明或证明错误。** 本件由非作者、非声明冷读者的独立上下文完成，比较原任务、实际 Lean 声明、作者报告和独立冷读，不裁定缠论原义资格，不改原稿、原 project 或既有机器回执。

接受的精确内容是：原 MovingQuote 同号且每个状态均有有效双边 QuoteOf 的有限 Flow，在全部事件切点取闭合整数 spread 区间，其最小闭区间外包络恰为原 leg 的两端范围；另有一个非空、已被首个反号事件确认的符号块，它自己拥有的深档事件价落在该范围之外。这项接受不扩展为全来源外缘无损、完整 F₁/F₂、原义同级走势完成或生产采用资格。

## 绑定与验证范围

| 项目 | 本次实际核验值 |
|---|---|
| 只读研究树 HEAD | `84ddc99529d5f2c5e043d0bf3c4ae5539a82e4f8` |
| 精确根 | `DirectHull.exact_root : SPLIT_TARGET ∧ HULL_TARGET ∧ LIMIT_TARGET` |
| DirectHullSpec.lean SHA-256 | `14f4d869525d1c9eaba3c353ff3aff8ffa7dec07b295068742b40bffe7ce088c` |
| DirectHullProof.lean SHA-256 | `62779c7ab71c0469f840ddeef4694896bedaf60a8bc8791f8ee46101e048d21f` |
| semantic SHA-256 | `83c95e8fba961d9a19fa0e88c83a208fcf366dce3e56119afd92c29585aa0d77` |
| 作者机器运行 | `a7166b7d233147b88226a6c5effc948c` |
| 独立冷读机器运行 | `ed74a9224d7b4dcca693709d8713830c` |
| 独立 manifest SHA-256 | `d6a65f075f3455487f5b34d4b1226c0a173862fed044437afeb83330d416741e` |
| 作者 manifest SHA-256 | `effd0f13f42f8a1f4f66e52886eae0a97499c1bd7e8bfee257deee4e9df237f6` |

实际读取[声明](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/project/DirectHullSpec.lean:8)、[证明](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/project/DirectHullProof.lean:6)、相关 MovingQuote/DirectQuote 定义与所用引理，并对照[作者报告](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-reentry.md)和[独立声明冷读](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-hull-readback.md)。图查询返回该研究树未索引，随后沿指定文件与 import 读取。

本次重新计算了七份项目 Lean 源和 toolchain 文件哈希、六项作者原源锁、独立运行的八份编译产物，以及七个正式 Origin 模块的源码与预编译 `.olean` 哈希。全部与绑定记录相符。作者与冷读的 input hashes、根类型、公理集和 semantic hash 相同。冷读记录中的七个本地模块均 fresh 编译且 exit 0；正式 Origin 七模块是预编译导入，本件不将其写成源码重新构建。

独立运行记录的实际根依赖公理仅为 `propext`、`Classical.choice`、`Quot.sound`；异常公理、未知依赖、unsafe 依赖及身份不匹配均为空。本件未发现新增编译疑点，因此复用并核验该独立运行，不再重编同一七模块，也未重建 formal。证据字段和当前哈希存于[identity-check.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/independent-direct-hull-evidence/identity-check.json)。semantic hash 只绑定抽取内容身份，不自行表达本件的语义判断。

## 与原任务的逐项比较

| 原任务义务 | 实际声明与独立判断 |
|---|---|
| 每个原事件切点都有来源状态 | `SPLIT_TARGET` 对任意 `es=pre++post` 给出原 Flow 的中间簿和报价，含空前缀、空后缀及整个空流。证明对原 Flow 归纳拆分，没有将切点缩成首末两个。 |
| 支撑独立于候选外包络 | `Support` 使用事件拼接、两段原 Flow 和 `v.bid≤x≤v.ask`，定义中没有 leg、segLow、segHigh。它不是将输出函数的范围改名为待证支撑。 |
| 精确而非松包络 | `HULL_TARGET` 同时证明两个端点属于 Support，以及每个 Support 值都在两端之间。正号取得首 bid、末 ask；负号取得末 bid、首 ask。端点取到性使其确为最小闭区间外包络。 |
| 深档自有事件价可在范围外 | `LIMIT_TARGET` 给出实际 Step、三个 QuoteOf/ValidBook、非空单事件 Flow、Completed、locate 确认字段、leg 范围与 `¬ Support ... 90`。反向确认事件没有被计入第一块的成员。 |

证明的承重点符合任务。`support_bounds` 分别消费前半和后半 Flow 的弱单调结论，以约束任意中间报价。`hull` 用空前缀与空后缀取得极值端点，再用严格 spread 和弱单调把 leg 的 min/max 化为对应的报价边。`SPLIT_TARGET` 单独保证所有源切点可用，不因 `hull` 的证明只需端点见证就被省略。

Support 对全部与相同起终条件及事件串相容的分段见证取存在量化，不固定某个唯一中间列表轨迹。接受它不需要轨迹唯一性：每条合法路径的每个切点都落入该关系；每条路径的同一对首末极值已被取到；额外兼容见证也受相同边界约束。因此关系合并所有兼容路径的做法没有改变本次要证明的外包络。它没有给出选择或恢复唯一轨迹的算法。

## 前提域与反例成员边界

`Flow` 是条件关系。它要求逐事件 Step、每个切点 QuoteOf，以及每个事件的 `positive` 等于同一 Bool。`positive` 只由买卖侧和 add/cancel/execute 决定；正压力不等于成交价上涨。Step 要求数量为正自然数，add 在价位单位列表前置指定重复元素，cancel/execute 删除一段连续同价元素。列表无订单 ID、队列优先级、物理时钟或行情序号。该模型既不是任意消息串必有合法轨迹的定理，也不是交易所适配正确性定理。

价格为 `Int`，负数、零和重复价位均允许；并未预设货币单位、正价格或连续实数价格轴。QuoteOf 要求两侧非空、取得真实列表极值且 bid 严格小于 ask。单独 ValidBook 对空侧可真，不能替代 QuoteOf。锁定、交叉或显式无有效双边报价的状态无法提供本定理所需 Flow。

HULL 对任意自然数 start、任意有限事件列表成立，未要求该列表非空、最大或 Completed。空 Flow 强制首末簿和报价相同，仍有该状态的 spread 包络，但 leg 的起终索引相同，不能获得正时间长度或完成资格。活动同号尾部也在定理域内，其当前包络随新事件继续变化并无矛盾。

`ExactHull S lo hi` 只包含 `S lo ∧ S hi ∧ ∀ x, S x → lo≤x ∧ x≤hi`。它不包含逆向的区间填满命题。冷读用稀疏集合 `{0,2}` 说明这一通用谓词的边界是正确的；本件没有据此声称特定 MovingQuote.Support 必然有空洞。spread 内整数价位可属于几何支撑，同时没有对应的成交、挂单或源事件。

原反例逐项复算如下。

| 观察事件数 | 已执行事件 | 买侧列表 | 卖侧列表 | 报价 |
|---|---|---|---|---|
| 0 | 无 | `[100]` | `[102]` | 100/102 |
| 1 | bid add 90，一单位 | `[90,100]` | `[102]` | 100/102 |
| 2 | ask add 110，一单位 | `[90,100]` | `[110,102]` | 100/102 |

第一个块的成员是索引 `[0,1)`，仅含价位 90 的事件。第二事件索引为 1，提供首个反号确认，观察数 2 时 `Completed p 2 0 1` 成立，knownAt 为 `[some 2, none]`。第一块的两个状态切点均为 100/102，故其 Support 是 `{100,101,102}`，leg 为 100→102；90 是自有事件价且存在于块后买侧，却不在该包络内。110 不属于第一块，不参与制造这个失败。

该固定见证足以反驳“所有合法已确认直接块的 leg 都包住全部自有事件价”以及“包住全簿所有深度价位”这两条全称说法。它不否定 L1 合同，不否定 Runs 对事件列表的 decode 无损，也不证明所有订单递归路线不可能。这里的事件和盘口是抽象模型中的具体合法见证，没有市场采集或实盘真实性证据。

自有[model-checks.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/independent-direct-hull-evidence/model-checks.json)保存有限复算，另检查了一个内部列表不唯一的实例：初始买侧 `[100,99,100]`，依次 cancel 100、cancel 99，卖侧恒为 `[102]`；中间买侧可为 `[99,100]` 或 `[100,99]`，终态均为 `[100]`，全部报价均为 100/102。它说明无须偷加列表轨迹唯一性。这些有限核算不替代一般 Lean 证明，也不扩展机器通过范围。

## 继承范围与仍未覆盖的原义

此次根的实际依赖闭包没有 `MovingQuote.exact_root`、`DirectQuote.exact_root`、`MovingQuote.construct`、PressureDC 目标或 `CenterConfirmedComplete`。导入模块里存在其它定理，不代表本根证明或消费了它们的全部结论。实际闭包包含 group、locate、Completed 定义，只在本固定反例中核验相应数值；本根没有重新证明它们的一般资格。

已回读 [MovingQuoteWireSpec](/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/direct-route-l1-hull/project/MovingQuoteWireSpec.lean:9)、[BookQuoteSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/BookQuoteSpec.lean:18)、[LocatedSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/LocatedSpec.lean:21)、[PressureDCSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/PressureDCSpec.lean:59)及 [PressureDCKernelSpec](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/PressureDCKernelSpec.lean:35)。作者的限定继承口径与这些声明一致：

- Runs 的分组唯一和 decode 来源无损，不给出完整走势身份。
- BookQuote 的相同全价位数量、合法逐步演进和 Reads 前提下的报价确定，不是列表或订单身份唯一，也不证明任意输入存在轨迹。
- construct 只检查报价列表长度并读取可空几何；`some out` 或 `geometry=some s` 本身不检查 Step/QuoteOf 来源。将本 hull 接到任意已确认真实输出的逐字段来源桥，本根尚未完成。
- Completed 和 locate 说明符号块的边界与观察确认，不是原义走势结束证明。
- PressureDC 在合法全程报价下保留选边路径的方向、分组和确认视图，另有 leg 公式等式；整数 δ=1、同价留末是具名前提。它没有使完整 payload、原生适配、物理时钟、最低中枢或原义 Move 一并等价。

Stage45 的[范围订正](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage45/ScopeClarifications-v1.md)固定 P/E 平台目录及各自来源原子。本次则固定原压力块及每切点的两边报价；同号 Flow 保证两报价边按同一方向弱单调，leg 已包含起点状态所需的极值。因此不能将平台 E 的“加入进入前状态后外缘扩大”算术机械搬来，充当本合同的反例。

反方向的推论也不成立。L1 包络没有覆盖深档全部价位、事件触价集合、真实核心成员或原义走势外缘。组合多个已指定块的准确范围只能得到其已指定 L1 支撑并集的几何包络；它不决定哪些块属于哪一原义对象。独立语义载体 S、固定对象映射 φ、同级同型、真实完成、核心归属及结束位置对应仍待证明；J 对 R_D 的适用与否也未由本根裁定。EXT-rel 的几何可用性须先给出具资格成员，EXT-own 和完整 F₂ 保持开放。

## 非阻断的文字订正

以下三点不要求改变声明或重编译，但后续引用须明确。

1. 作者“Support 保留事件、簿与中间状态”仅表示参数和存在见证仍关联原 Flow。`Support ... x` 是价格成员谓词，不能由它恢复唯一轨迹、订单身份、来源重数或成员顺序；decode 来源无损属于另一份关系和定理。
2. 作者“真实簿”“真实源事件价 90”在本件只能读作抽象模型中的具体合法见证，不能读作真实市场采集。
3. 作者第 39 行“缺口不在本 Flow 定理的域内”必须限定为已显式表现为缺失有效双边报价的状态。Flow 没有行情序号、时间戳或消息完整性字段；未建模的消息缺失可能留下另一条合法抽象 Flow，定理不能据此排除数据源丢包。

本件接受的是以上锁定声明对原任务限定目标的数学对应性。作者和冷读机器回执中的 `semantic.status=not_reviewed` 均保持原值，本件没有替任何旧阶段或机器回执改写通过状态。
