# Stage49 F₂ 输入合同独立审查

2026-10-05，#1467。在制研究证据。本工位独立核原文、正本、当前 Lean/Rust 源码，独立冷编与复算后，再读另工位的声明盲读报告。只写本报告和具名仓外证据目录；未改作者冻结包、formal、生产、教义、Progress，未运行 Git 或外发。

## 采用结论

**采用 `f2-input-contract.md`，范围限于研究输入合同、Norm 的量词辨析，以及当前局部中枢接口的有限区分见证。未发现阻止这一范围采用的问题。** 所核来源不支持把任意既定成员分区的首末价必须为该区间两极值，升格为所有替代 F₁ 的共同前提。052 的标准化答复不能推出原分区已经标准化，更不能推出重分保持原成员或旧核归属。

这项采用不授予三对象完整 a₁/F₂ 资格，也不证明存在一个原义合格而非 Norm 的完成走势。三个数值对象没有各自的 `ownCenters` 证书、完成证书、同级证书或原义方向来源证明；父局部交集 `[10,25]` 不能补齐这些缺口。P 的方向桥、P 的实际完成、连接的资格、存在与唯一分解、确认因果性和完整父实例交付仍待完成。P3 主确认不因此解锁。

## 冻结身份和证据层级

研究树为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。以下称 `P=.chanlun/review-results/issue1467-f1-proof`。作者记录的 HEAD 为 `2e3551802306b704e759631487a75498b3b08b12`；本工位遵守无 Git 范围，没有独立重核 HEAD，而是逐项核实际源字节。

| 对象 | SHA-256 / 核对结果 |
|---|---|
| `P/stage49/f2-input-contract.md` | `43cbc60ebdf67bc75b2a6c2a5fcab55f5e93e2ea0eb6d0c0a19adb126033f531` |
| `P/stage49/f2-contract-evidence/SHA256SUMS` | `a0412b81ccb40d3a2b4e5c56cce31eb9eb6e7e730c595a10a7a35775aa3cfddd` |
| 该清单的 8 项产物 | 全匹配，独立复跑前后均未改变 |
| 作者 `sources.json` 的 27 项来源 | 全匹配，独立复跑前后再次核对 |
| 另工位 `P/stage49/f2-probe-readback.md` | `cf456561b32781c5c5da21450d5bf102e2e628ae04d8f705577293d1107a7092` |
| 另工位 `P/stage49/f2-probe-readback-input-hashes.json` | `3a5db2b04de00de2922389fa2d763a9227af5c7b2cb557d0907b71c0a272328f` |

证据分四项记账。作者已有 direct Lean 冷编和 `#check/#print axioms` 回执；另工位提供未见作者合同及 Rust 的声明盲读；本工位重新冷编实际源、运行探针；普通数学与原义的比较由本报告承担。它们均不是 `verify_lean_project` exact-root 流程回执。本次没有为抬高工具等级新造无关工程。

## 原义与输入合同

代码发现先调用图工具，对本研究树查询 Segment、segLow/segHigh、UnitRange 和 center_from_segments，返回 `project not found or not indexed`。随后按给定路径读源，没有借其他工作树索引替代。

`035-第35课.md:18` 在正文界 `:38` 之前，作者署名见 `:12`，所引句无行首或行内编注。它消费三个走势“所分别经过的价格区间”，不是三个子核心交集，也没有把首末端价定义成所有原分区的极值。`:22–24` 允许拆散重分与结合，这些操作本来就可能改变成员对应。

`052-第52课.md:310–328` 在正文界 `:26` 之后，是带作者署名和时间的答疑。`:314–324` 为读者提问，`:320` 的“起点和终点一定是最高点或最低点吗”不能当作者结论；`:326` 分隔后 `:328` 才是作者答复，所引句没有编注。答复给的是通过结合归到标准形式。合同 `:31` 用 `∀D ∃D′` 说明语境量词，并明确没有形式证明标准化定理；它没有偷换成 `∀D Norm(D)`。本审查也不从这段话倒推出任意非 Norm 分块均合法。

正本 `zhongshu.md:53–63` 的 Z-1 规定全三段范围，`:71–77` 的 Z-2 规定严格非单点，`:81–93` 的 Z-3 保留方向交替，`:95–126` 的 Z-8 要计入真正合格的连接。这些内容与合同一致。Z-3 的正本用语和 P 的概念无方向之间仍需接口桥；`qushi.md:44–48,179–200` 没有给任意技术方向直接取得原义方向资格的许可。不能以生产 `center_from_window` 不查方向为由删 Z-3。

逐行核合同 `:41–50` 的结果如下。这里的字段和见证格式属于 #1467 v4 研究可审计性要求，不是宣称原文逐字规定了这些字段名。

| 合同项 | 审查结果与适用界限 |
|---|---|
| 来源、成员、priceSupport、confirmationSupport | 采用。完整去向、未决尾部和独占成员计数来自研究协议；`(s,t]` 是有条件选用的约定。合同明确允许共读边界状态，没有偷加“所有证明支持必须不交”。 |
| actualStart/actualEnd、真实连接、重分映射 | 采用。新极值只能是范围代理，不能直接冒充旧真实边界；若另建标准化分解，需另证新成员对应，不能沿用旧归属。 |
| fullRange | 采用。对具名非空有限源路径，既要求包住全部值，也要求上下界由路径取得，排除了虚增范围；没有要求包络中每个数值都出现。包含实例真正拥有的连接/延伸，不把仅供确认的外部事件混入价格范围。 |
| kind、ownCenters | 采用。P 一核、趋势同向多核针对相应级别，计数不能由任意窗口替代。`018:24,26–32` 支持完成与核身份的义务；`:24` 的“娇”行内注已排除。基底不被强迫无穷下分。 |
| levelEvidence | 采用于本候选构造。相同 Nat 标签不证明同级。`level_recursion.md:229–237` 已说明追到底不是一切本级有效性的前提，合同不应在后续实施中被重新解释成这项附加门。 |
| DirForZ3 及桥 | 采用。数值 U/D/U 只证明字段交替。P 无概念方向，不意味着全部 P/P 被禁，也不意味着随填 break_direction 可以放行；桥未证明时仍未获准入。 |
| Completed | 采用。`018:40–42` 区分达到可完成条件与实际结束；合同要求从历史取得具名依据和可知时刻，没有用“算法输出”自证，也没有偷加“必须本级背驰”。 |
| Consecutive | 采用。`049:138` 的作者答复赋予具条件连接次级别角色，没有把任意索引 gap 证明为完整同级走势。列表顺序也不单独证明没有漏成员。 |
| LocalCore | 采用。资格独立成立后，统一计算全三段 `max(lo)` 与 `min(hi)`，再核交替及严格 `<`。公式不需要 Norm 参数；这不是完整父走势终结判据。 |
| 上层实例与递归交付 | 采用为尚待证明的研究义务。核心形成后仍需延伸/新生/扩展、全成员范围、分类、完成和尾部归属；不能用本轮局部通过宣布 P1/P2 完成。 |

`priceSupport` 是参与对象完整范围运算的源状态；`confirmationSupport` 是使判定可知的依据。`wholeMoveRange` 即合同的 `fullRange`，是完整走势包络；`ownCenterCore` 是该走势拥有的中枢核心；`CenterOuter` 是中枢构成及延伸的外缘。这五种对象不能只因某例数值相同就互换。正本 `zhongshu.md:195–208` 对冻结核心和外缘的区分也没有提供“完整走势范围等于自身中心外缘”的普遍定理。

## 当前代码与有限算例

实际读取 `formal/Origin/ChanlunElements.lean:43–49` 后确认，`Segment` 仅有 direction、两个索引、两个端价，没有内部路径、fullRange、level 或完成证明。`CenterConstruction.lean:52–57` 的 segLow/segHigh 只读取端价。`:92–117` 的 ZD/ZG/DD/GG 和 centerHolds 看不到内部极值；`:178–195` 的 centersOf 不查交替。`CenterComplete.lean:147–160` 的两个谓词均只有交替和严格三交，`:168–171` 的等价是同定义的 rfl。名称 Complete 没有编码完整 Move 资格。

`Origin.Move` 在 `ChanlunElements.lean:81–88` 也是不同结构，不因具备价格端点就取得 fullRange。`CenterFull.lean:53–59` 仅要求外缘包含核心；`CompleteClassification.lean:36–40` 的抽象行为商合同不提供本候选的具体分类。另一命名空间 `Formal.RecursiveConstruction.Move.interval:48–53` 从 centers 的 dd/gg 以 0 为初值聚合；WellFormed 和 MovesComposedFrom 也不补真实完成与无漏重成员证明。原报告对这些入口的限制说明与实际声明相符。

Rust `center.rs:83–92` 的 UnitRange 独立存 lo/hi。实际 `center_from_segments:241–262` 核方向交替与 `ZD<ZG`，取三输入外缘，边界只用第一起点和第三终点。它不查中间接续、真实端价、来源、层级或完成；`:283–299` 的 window 入口进一步省略方向。公有 struct 的注释不构成 lo/hi 的证明字段。这些是实现事实，没有据此更改原义。

本工位从全 12 个状态重新切取路径，逐个重算极值、取得位置、事件所有权和接缝，结果保存在仓外 `coordinate-recomputation.json`。此文件是普通有限算术记录，不是新 Lean 定理。

| 对象 | 完整状态路径 | 实际端价 | 完整范围及极值位置 | 给定方向、Norm |
|---|---|---|---|---|
| A，事件 `(0,7]` | 10,10,10,10,30,20,20,20 | 10→20 | [10,30]；低 x₀…x₃，高 x₄ | Up；末价20≠30 |
| B，事件 `(7,9]` | 20,25,5 | 20→5 | [5,25]；低 x₉，高 x₈ | Down；首价20≠25 |
| C，事件 `(9,11]` | 5,28,10 | 5→10 | [5,28]；低 x₉，高 x₁₀ | Up；末价10≠28 |

事件 1…11 各消费一次；x₇ 和 x₉ 是共读边界状态。真实索引接缝为 7=7、9=9，价格接缝为 20=20、5=5。给定方向与净价格变化一致，但这不证明原义 U/D 趋势身份。

| 独立重跑/复算 | 结果 |
|---|---|
| 原 Rust segments，完整范围 U/D/U | `Some`，ZD/ZG/DD/GG = 10/25/5/30，索引 0…11；window 同值 |
| 实际端点范围 | [10,20]、[5,20]、[5,10]；ZD=ZG=10；严格判据拒绝。闭区间交集为单点，不是空集 |
| 原 Lean actual | 方向交替、字段接续成立；CenterConfirmedComplete 因 10<10 为假而拒绝 |
| 原 Lean 范围重编码 | 10→30、25→5、5→28；局部谓词接受，数值 10/25/5/30；三处实际端价被改，A→B 价格接缝30≠25。B→C仍为5=5，不能写成“所有接缝破坏” |
| 原 Rust Up/Up/Up 负控制 | segments=None，window 仍 Some。window 不能替相同判断的方向失败兜底 |
| 原 Rust B.start_index=100 负控制 | segments 仍 Some。该无效接续不获语义认可，只说明局部函数没有检查它 |

这里算出的 `[10,25]` 是三个数值范围的父局部核心，绝不能填进 A/B/C 的 ownCenters 当作各自资格证。A 的重复10/20也没有在本探针中被定义成归属已证明的中枢；B/C同样没有核证书。缺资格不是本例某个局部函数返回 None 的原因，因为函数没有消费这些证书；它们是通过局部数值检查之后仍存在的缺口。

## 声明盲读与普通数学比较

另工位在不知道作者合同与 Rust 结果的条件下，还原了 10 条实际 Lean 声明。本工位读完并复跑上述源后才读取其报告，报告哈希如前。两项结论一致。

前九条都是固定样例的封闭命题。它们分别覆盖三个范围、三个非 Norm、四个连接字段等式、方向字段交替、三个范围丢失、单点拒绝、范围重编码数值相等、局部接受，以及三处端价改变和第一接缝失败。没有 Move 参数、level 或 Completed 关系。

第十条只在任意整数 s<e 下化简两个 if，证明端点 min/max 等于 lo/hi，当且仅当 s=lo 且 e=hi。它没有路径或 Norm 参数。将它用于真实路径，需要另外给真实首末价与路径绑定，以及 lo/hi 为完整范围的证据；作者 `:65` 把它称为“纯整数条件等价”，符合声明。下行对偶可作普通数学推论，本轮未新增下行 Lean 声明。不能把这条 if 化简或样例的可判定证明写成一般适配器保真、一般 fullRange 正确性或原义标准化定理。

盲读还指出 Norm 的 Lean 定义本身没有把列表首末与 Segment 首末绑定。此例两组 actual 与 path 的对应可直接逐值核实，本工位也已这样复算；这不等于已证明任意索引分区的关联谓词。冻结合同要求下一候选给真实边界与源路径，恰好保留了这项义务，没有把探针推广到任意路径。

## 独立复现与残余限制

专属仓外目录：

`/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage49-f2-contract-independent/`

本次独立运行目录：

`run-20261004T163745458881Z/`

复现命令：

```sh
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage49-f2-contract-independent/rerun.py
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/stage49-f2-contract-independent/recompute_coordinates.py
```

`rerun.py` 每次新建空构建目录，将 LEAN_PATH 设为该目录，从原文件依次冷编 SourceAxioms、CompleteClassification、ChanlunElements、CenterStates、CenterFull、CenterConstruction、CenterComplete。然后编译字节相同的冻结 NonNormProbe；Rust 驱动字节不改，仍以原绝对路径包含真实 types.rs 和 center.rs。没有执行作者 run.sh，也没有覆盖作者固定目录或 formal/.lake。

七次原模块构建、探针 elaboration/olean、Rust 编译和运行均 exit=0。独立产物保留，七个模块 olean 和探针 olean 的 SHA-256 均与作者 source-build.json 一致。独立 Lean 输出中，前九声明无公理依赖，第十声明仅 `[propext, Quot.sound]`，没有 sorryAx。工具版本为 Lean 4.31.0、Rust 1.98.0；工具可执行文件与 lean-toolchain 哈希也匹配。命令、退出码、原始日志、源/olean/工具哈希在运行目录 `commands.json` 和 `independent-source-build.json`。

作者初次失败回执确有两个 Decidable 合成失败及相应 sorryAx，不能作为证明使用。最终源与本轮独立输出已排除这两个占位项。初次失败源未另行冻结，因此本审查不为“历史修复只改了哪几行”的完整差异背书；采用依据是最终冻结声明与独立成功回执。这不影响当前有限结论。

本次哈希绑定覆盖具名项目源码、编译产物和工具可执行文件，没有穷尽 Lean 隐式标准库和动态库的全部字节；不能称完整工具链供应链认证。没有运行 exact verifier、正式 fixture 漂移或全仓测试。formal 源未修改，本次靶向复跑已直接覆盖所采用的10声明和原Rust两入口。进一步复用时必须继续保留完整走势身份、方向桥与完成性尚未证明的边界。
