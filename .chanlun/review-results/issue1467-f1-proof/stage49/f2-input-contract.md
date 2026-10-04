# Stage49：固定原义 F₂ 的输入合同与端点标准形边界

2026-10-05，#1467，在制研究证据。研究树 `HEAD=2e3551802306b704e759631487a75498b3b08b12`。只读原文、教义及已指定实现；本轮只写本文件与 `f2-contract-evidence/`。未改 formal、生产、教义、旧研究、Progress、GitHub 或 Git 提交。作者自检通过；独立语义审查待父任务另行组织。

## 结论及其量词

**不能把“每个既定原成员区间首末价已经是该区间两极值”列成所有替代 F₁ 的共同准入门。** 所核原文要求三份合格次级别走势**分别经过的完整价格区间**相交，正本 Z-1 取全三段范围，Z-3 要方向交替，Z-8 要把真正的同级连接计入成员；它们没有给出对任意既存原事件分区逐块成立的端点 `Norm` 公理。052 的作者答复说可经结合归到标准形式，不能将其换成“任意固定分区已经标准化”，更不能附送“重分后保留每块原成员和旧核心归属”。

**当前 `Origin.Segment` 的端点表示确实另有范围保真前提。** 它只存 `startPrice/endPrice`，`segLow/segHigh` 只读两端。因此，将一份非 Norm 对象的实际端点直接映入此类型，会丢失内部极值。Rust `UnitRange` 单独保存 `lo/hi`，所查局部中枢入口可以消费正确全范围而不要求原边界 Norm。这个接口差异不是两套原义 F₂。

本轮给出三份实际边界接续、保存完整范围、方向字段交替、全部非 Norm 的有限坐标对象。原 Rust 局部中枢函数接受全范围并算得 `[10,25]`；原 Lean 局部谓词在全范围的数值重编码上接受，在实际端点投影上因 `[10,10]` 单点而拒绝。**本例未提供原义完成、走势身份、层级和方向来源证明；不是一个完整 a₁ 正面见证。** 消掉多加的 Norm 门，仍须补这些真正的缺口。

本结论的范围是：所核来源不支持该普遍附加门；局部公式对完整范围的依赖不需要原边界 Norm；已查实现有一份可运行的区分见证。它不是“任意非 Norm 分块都合法”的定理，也没有证明原义合格走势可以随意切开。

## 1. 来源与作者归属

下列路径相对本研究树，行号均回查当前字节。`sources.json` 保存输入 SHA-256。引用区分原作者、读者提问和编注；不重开已关闭的订单来源桥，也未用生产行为裁定教义。

| 来源定位 | 归属及本轮承重内容 | 不能推出的内容 |
|---|---|---|
| `docs/chanlun/text/blog/035-第35课.md:12,16–24,38` | 12 署名，38 正文界；16–24 无所引片段内编注。16 在三笔同价成交的具名最低构造中给趋势延续/结束条件；18 明说三个走势“所分别经过的价格区间有交集”；22–24 给拆散重分、结合律 | 新观察已有成交语义；结束证据已决定唯一原事件切点；结合必保原块成员或旧核归属 |
| `docs/chanlun/text/blog/052-第52课.md:310–328` | 正文界在 26；310–312 为作者署名/时间，314–324 是读者，326 分隔，328 才是作者答复：“通过结合律，都可以把归到你说的这种标准形式上，也就是连接点都是高、低点。” | 把 320 的“起点终点一定是最高最低吗”疑问当成作者肯定；把可归标准形当成给定分区已满足标准形；本轮不使用 330 混合追问/答复作独立前提 |
| `docs/chanlun/text/blog/084-第84课.md:52–56,68` | 正文，界 68，所引段无行首/行内编注。F₁ 可替代，F₁/F₂ 可不同；a₁ 是最低级别中枢、走势类型；52 保留唯一分解要求 | 任意块、指标值或可编译 `Segment` 就是完整 a₁；允许改掉 F₂ |
| `docs/chanlun/text/blog/018-第18课.md:24,26–42,68` | 正文，界 68。24 给全三段高低公式并明定最初三次级走势已完成；该行 `(娇：盘整和趋势）` 是行内编注，本轮排除。26–28 的分类针对完成走势，40–42 区分达到可完成条件与仍可延伸 | 三交出现即整份盘整已经终结；两同向核即趋势已经结束；用外置 `completed=true` 补证据 |
| `docs/chanlun/text/blog/049-第49课.md:128–138` | 正文界 78，128 作者署名、130 时间，132–134 读者问题，136 分隔，138 作者回答：含三类点的相应连接也是一个次级别 | 任意两个核之间的索引 gap 自动是完整次级别连接；所有中心/确认支持集合都必须不交 |
| `docs/chanlun/text/blog/031-第31课.md:384–390` | 正文界 32；386 是问题与 `===` 后作者回答的混合行，390 重复作者回答。“盘整哪里有什么方向，只有趋势才有方向”取作者部分，不取读者的中枢方向假设 | 每个 P 可随意补 U/D，或所有 P/P 接续一概被禁止 |
| `.chanlun/definitions/zhongshu.md:45–126,132–146` | 正本：Z-1 全三段；Z-2 严格核心；Z-3 交替；Z-8 合格连接；S-5 允许替代起始构造，F₂ 不随基底选择改变 | 所有 F₁ 必须先成为现用线段表示；现有实现缺连接可撤销 Z-8 |
| `.chanlun/definitions/level_recursion.md:29–31,75–85,229–237,329–340` | 次级别身份、完成义务；递归到底是特定构造不变量，不是所有已完成本级对象有效性的先决条件；实现选择 `settled`，并明确背驰只是完成的充分非必要条件 | 只写数字 level 或 settled 就完成语义桥；早段 85 的“最后一段背驰”可压倒后段明确的非必要性 |
| `.chanlun/definitions/qushi.md:40–61,179–200` | P 恰一相应级别核、无概念方向；技术方向与概念方向分开，Compose 兼容缝有具名登记；趋势依次同向与可继续延伸分开 | 技术性的 `break_direction` 或首末子走势 `hi` 规则已经解决 P 在 Z-3 中的原义方向问题 |

052 的逻辑形式至多可按本语境读作“给定合格分解，可经允许的结合重分取得一个标准表示”。即使记为 `∀D ∃D′ (Recombine(D,D′) ∧ Norm(D′))`，也不蕴含 `∀D Norm(D)`，不蕴含 `Members(D′ᵢ)=Members(Dᵢ)`，更不指定同价极值的 First/Last。该式是对原句量词的说明，不是本轮已形式证明的一般标准化定理。

Stage48 的十事件反例精确击中后两种偷加：原词 `10³,30,20³,15³`、固定 s=0、拥有 K10/K20、排除触发 K15 时只剩 t=7，而 `x7=20 < hi=30`；取 t=4 虽到高点却切掉第二核。见 `stage48/mature-core-completion.md:108–124`。撤去 Norm 后只恢复这条候选边的必要可行性，不追认其完成或唯一分解。

## 2. 给下一候选的独立输入合同

以下是将已查义务分清的**研究输入合同**，不是新教义定义。候选须具名为 c，固定有效历史域 `H_c`、观察/对象解释与版本；对每个 `h∈H_c`、截止时刻 τ 和实际送入 F₂ 的层 k 序列 `L=[X₀,…]`，分别举证下表。没有声称每个有限前缀必须产出完成对象。

| 字段/关系 | 量词与必须证明的内容 | 归属与失败条件 |
|---|---|---|
| `sourceVersion, ownedMembers, priceSupport, confirmationSupport` | 对每个实际消费对象 X，给可重建的有序原成员及其区间；对声明消费的整段历史给完整去向。若合同采用 `(s,t]` 独占事件，连续边依次衔接；端点状态可共读。范围对应的状态路径 `priceSupport`、观察值的计算来源、确认支持分别记录 | 研究的可追溯/完整分解义务。漏事件、暗删 NoCore/尾部、同一独占成员多次消费即失败；不能把“所有证明支持不交”强加进去。Stage20 Cover 只证给定分配覆盖，不证语义唯一 |
| `actualStart/actualEnd, s,t` | 对各 X 给真实边界事件/状态与实际观察值；对被宣称相邻的 X/Y 给接续关系。若选择重分，另给旧到新成员映射，不只改显示端点 | 来源和连接义务。新高低点仅作范围代理不能冒充真实端价；“列表里相邻”不足以证明中间没有被漏掉的合格成员 |
| `fullRange=[lo,hi]` | 对各 X 的整个具名源路径，`∀j∈priceSupport(X), lo≤x_j≤hi`，且有该路径点达到 lo、有点达到 hi；非空有限路径据此唯一确定范围。事件区间 `(s,t]` 的本例取状态 `x_s,…,x_t`，不把区间外仅供确认的后继事件并入其范围。复合对象须包含其全部构成范围及属于该实例的连接/延伸 | 035:18、Z-1 的范围义务。这里区间是高低包络，不要求每个中间数值都实际成交。只交子核心、只包种子窗、0 哨兵扩大、实际端点遗漏内极值均失败 |
| `kind, ownCenters` | 送入者须确实是该级完成走势类型/获相应角色资格的连接，核身份、层级、延伸/新生及归属有来源；P 的一核与 U/D 的同向多核针对相应级别，不是任意滚动窗计数 | 018/035、P/U/D 正本。三个区间存在、局部种子出现或类名叫 Move 均不足；基底不能因递归要求而被强迫无限再下分 |
| `levelEvidence` | `∀X∈L, level(X)=k`，并给何以是原义同一级的递归/初始构造证据；父中心级别为 k+1 | 原义次级别合同。只手填同一个 Nat、把不同角色/层级的 gap 改名、把未完成低级块上贴标签均失败 |
| `DirForZ3(X)` 及桥 | 对实际三对象窗口证明适用的方向含义，并证明相邻异向。U/D 趋势的概念方向应由其合法同向核链支持；P 的概念分类仍是无方向。若使用 P 的遍历/连接方向，必须单独证明该表示适合 Z-3，而不能拿任意字段代替 | Z-3 保留；P 方向接口桥在所核正本仍有未闭合处。正本没有“P/P 一概禁”条文，也没有“随填 break_direction 即放行”。缺桥就是该候选未证明，不能改走不查方向入口接管失败 |
| `Completed_c(h≤τ,X)` | 对每个作为前三构成者上送的 X 给具名、非自指、可核的实际完成依据及可知时刻；此关系不能定义成“算法输出了 X”。完成可迟知，但不能回标为此前已知 | 018:24、40–42。形成一个核心是盘整可完成条件，不是终结证书；035 具名趋势的后继核证据不自动定位切点或解决 P。`settled` 是承载证据的工程标记，不是证据来源 |
| `Consecutive_k(A,B,C)` | 三对象在当前原义层序列确实连续，按 Z-8 包含该计入的连接；计数单位是走势单元，不是中枢核心或任意事件 | Z-1/Z-8。中心+连接+中心中的连接资格须证明；不要求所有原核心种子窗都恰等于完整实例所有成员（Stage34 已限定） |
| `LocalCore_k(A,B,C)` | 在上述资格成立时，按同一判据算 `ZD=max(lo_A,lo_B,lo_C)`、`ZG=min(hi_A,hi_B,hi_C)`，检查交替及 `ZD<ZG`；`ZD=ZG` 必拒绝 | 原义/正本固定部分。对所有送入窗口同样适用；这里没有 Norm 参数。局部中心成立仍不推出父走势完成 |
| 上层实例与递归交付 | 证明核心形成后的延伸/新生/扩展、完整父成员及 full range、P/U/D 分类、完成与未决尾部归属，且下一层仍满足同一输入合同；有限递归停止须对应实际构造 | 完整 F₂/P1–P2 义务。本轮局部函数和十个小声明均未完成这一层；当前仍不得启动以合格 C 为前提的主确认结论 |

`fullRange(X)`、X 所拥有中心的 `core=[ZD,ZG]`、中心的外缘 `[DD,GG]` 是三类量。即使某个特例数值恰同，也要分别给相等证据。`zhongshu.md:195–208` 将中心冻结核心和延伸外缘分开；035:18 消费的是完整走势经过的范围。Stage39 的 57 课实例已排除“父中心一律取三个子核心交集”的解释，本轮不重新包装或重跑该旧反例。

## 3. Norm 是哪一种附加义务

在本研究既有坐标约定中，

```text
NormU(s,t) := x_s=min(x_s…x_t) ∧ x_t=max(x_s…x_t) ∧ x_s<x_t
NormD(s,t) := x_s=max(x_s…x_t) ∧ x_t=min(x_s…x_t) ∧ x_t<x_s.
```

这是对**选定原边界与全部内部值**的关系，不是“内部必须单调”。它适用于选择标准形的候选或具体端点适配，不从“有两枚核”“已结束”“层级正确”推出。

若适配必须保留实际首末价，且唯一范围读法为 `lo=min(start,end)`、`hi=max(start,end)`，则其额外义务恰为这两个读数等于真实 full range。对于实际首价小于末价的输入，这与首价=lo、末价=hi 等价；本轮 `upward_endpoint_projection_iff` 已证明这个纯整数条件等价。下行对偶。**因此 Norm 可以是该端点表示保真的条件；表示只有两端价并不能反向证明源对象理应 Norm。**

若接口分开保存实际端点与 full range，则局部 Z-1 不需要这项条件。若仅把全范围的 lo/hi 假写到 `Segment.startPrice/endPrice`，可以用于核公式的数值调用，却改变了端价和可能的价格接续；不得称其为保成员、保连接的完整 a₁ 适配。只有下一候选明确选择并证明一份新的标准化分解，才能把新端点当成真实新边界。

## 4. 本 HEAD 的具体入口与依赖

代码发现先调用 `codebase-memory-mcp.search_graph`，传入研究树绝对路径及 `MovesComposedFrom|UnitRange|center_from_segments|center_from_window|Move.interval` 模式，返回 `project not found or not indexed`。随后仅沿指定文件/已知路径查源；没有借另一 worktree 的索引冒充本 HEAD，也没有全面扫描旧引擎。

| 当前声明与源行 | 实际承担 | 未承担/调用者义务 |
|---|---|---|
| `formal/Origin/ChanlunElements.lean:43–49`：`structure Segment`，字段 direction、start/endIndex、start/endPrice | 一个二值方向+双端点载体 | 无 full range、成员、level、完成或连接证明；类型允许任意字段组合 |
| `formal/Origin/CenterConstruction.lean:52–57`：`segHigh/segLow (s : Segment) : Tick` | 两端 max/min | 非 Norm 源的真实范围必须由适配层另外保证；函数看不到内点 |
| 同文件 `:92–117`：`computeZG/ZD/GG/DD (s1 s2 s3 : Segment) : Tick`，`centerHolds … : Bool` | 全三段极值运算；centerHolds 只核严格三交 | 不查方向、层级、完成、成员及接续；其中 GG/DD 只是该三输入数值的外包络 |
| `formal/Origin/CenterComplete.lean:83–87,147–160`：`DirAlternates … : Prop`、`CenterConfirmedComplete … : Prop`、`TrueCenterCore` | 两支：二值相邻异向 ∧ 全三段严格三交；后两谓词字面相同，`:168–171` 的 iff 是 rfl | 名字有 Complete 不代表消费完整走势的合同已编码。没有 s1.endIndex=s2.startIndex、价格接续、完成、同级或来源约束 |
| `formal/Origin/CenterConstruction.lean:129–159,178–195`：`centerFromThree (s1 s2 s3) (h:ZD<ZG) : CenterFull`；`centersOf : List Segment → List CenterFull` | 构造三段核心/外缘，核心消费3否则滑1，有限终止及函数确定性 | centerFromThree 不接收交替证明；centersOf 亦不查交替，不能整体继承为原义 F₂；确定性不等于独立语义分解唯一 |
| `formal/Origin/CenterFull.lean:53–59`：core、dd、gg、outer_lo/hi | 只要求外缘包含核心；`Center.valid` 在 `ChanlunElements:51–57` 仍是 `zd≤zg` | 无成员或极值取得证明；任意手填包含区间不自动是真实 full range；类型存在不等于严格构造接受 |
| `formal/Origin/ChanlunElements.lean:81–88,125–139`：`Move`；`ElementPipeline.movesOf : List Segment → List Center → List Move` | Move 有 kind、index、实际价格端点、centers；注释要求价格透传 | 无成员列表、full range、level 或完成证据字段；不能以价格端点已经存在代替全范围 |
| `formal/Origin/CompleteClassification.lean:14–16,36–40`：`CompleteClassifier` | 需提供 `∀x y, classify x=classify y ↔ ∀ω, trace x ω=trace y ω` | 是抽象行为商合同，不给本候选 P/U/D 分解、完成或三对象资格；实例化需自己提供 trace 与该双向证明 |
| `formal/Formal/RecursiveConstruction.lean:37–53,160–167`：另一命名空间的 `Move`、`Move.interval`、`WellFormed` | segment 带 lo/hi；compose 带 subs/centers/level；WellFormed 核 ≥3 子对象、同级、非空且派生的中心等 | 与 Origin.Move 不是同一类型；segment 良构为 True。compose.interval 仍从中心 dd/gg 用 0 哨兵 fold，不能继承真实范围；无真实终结证明 |
| 同文件 `:190–219,245–246`：`UpperMoveSound`、`MovesComposedFrom`、`WindowOverlap` | 窗口长度/在界/级别、上级良构和总数量、起点递增；WindowOverlap 只查范围严格三交 | 起点递增不保证窗口不重叠或完整去向；WindowOverlap 不查方向/完成。Stage19 已冻结的反例继续有效，本轮不重复修复/重证 |
| `rust/src/theta_v0/classifier/center.rs:83–92,156–185,241–262`：`UnitRange`；`center_from_segments(&UnitRange,&UnitRange,&UnitRange)->Option<Center>` | lo/hi 是独立字段；检查方向交替、ZD<ZG；给核心和三段外缘，输出边界取首/末输入 | 没有实际端价、成员、level、完成字段；甚至不验证下个 start_index 接前个 end_index。文档称 lo≤hi 不变量，public struct 本身不携带证明 |
| 同文件 `:283–299`：`center_from_window(&UnitRange,&UnitRange,&UnitRange)->Option<Center>` | 全范围严格三交，**不查方向** | 不能在同一原义判断中接管 Z-3 失败；该文件旧注释的“Lean仍≤”已与本 HEAD Lean 实码不符，本报告以实际声明和运行结果为准 |

本轮探针的直接依赖仅是 `CenterComplete → {ChanlunElements,CenterStates,CenterFull,CenterConstruction}`，再经 `ChanlunElements → CompleteClassification → SourceAxioms`。这7份原文件全部在本轮临时构建目录冷编译；未读取原 `.lake` 缓存来代替当前源检查。Rust 驱动用 `#[path]` 直接包含当前 `types.rs` 与 `center.rs`，没有抄一份“同样公式”假装调用原实现。

## 5. 非 Norm、真实范围保存的三对象区分见证

此例是有限**坐标诊断**：给定一个完整标量路径并按原事件区间分成三对象，保存它们所有坐标；不宣称是新订单协议样本，不将这些对象赋原义完成真值。A 使用 Stage48 十事件例的前七事件坐标，随后另接仅为本探针使用的四事件；不替换原十事件历史。

```text
x₀…x₁₁ = 10,10,10,10,30,20,20,20,25,5,28,10
事件所有权：A=(0,7]，B=(7,9]，C=(9,11]
共读边界状态：x₇=20，x₉=5；事件不重复消费
```

| 对象 | 完整状态路径 | 实际首末价 | 由全部路径算出的范围 | 诊断方向字段 | Norm 失败点 |
|---|---|---|---|---|---|
| A | 10,10,10,10,30,20,20,20 | 10→20 | [10,30] | Up | 末价20不是最高30 |
| B | 20,25,5 | 20→5 | [5,25] | Down | 首价20不是最高25 |
| C | 5,28,10 | 5→10 | [5,28] | Up | 末价10不是最高28 |

方向字段与本例实际端价净变化一致、彼此交替；本轮不把这一数值方向赋予原义 U/D 趋势身份。三对象确实按时间/价位接续；整个路径每个值都参与自己所属对象范围运算。

实际检验如下：

| 调用/负控制 | 有限结果 | 解释 |
|---|---|---|
| 原 Rust `center_from_segments` 输入完整 lo/hi + U/D/U | `Some(Center{zd:10,zg:25,dd:5,gg:30,start_index:0,end_index:11})` | 在此入口无 Norm 拒绝；局部核心/三输入外缘正确 |
| 原 Rust `center_from_window` 同一输入 | 同一个 Some | 仅核几何的一致性，不增加语义资格 |
| 实际端点映入原 Lean Segment | A=[10,20]，B=[5,20]，C=[5,10]；`computeZD=computeZG=10`；`¬CenterConfirmedComplete` | **范围丢失**使严格核心退化；实际接续和交替都已另证，不能把拒绝记为连接/方向失败 |
| 把完整范围编码到原 Lean Segment 两端 | A=10→30、B=25→5、C=5→28；`CenterConfirmedComplete` 真；ZD/ZG/DD/GG=10/25/5/30 | 只检验原数值谓词；A.end=30≠B.start=25，且三处端价改变，不能作为完整 a₁ 或保连接编码 |
| 原 Rust 输入改成 Up/Up/Up，其余完整范围不变 | segments=None，window=同一个 Some | 固定原义 Z-3 时不能以 window 的接受消除方向失败 |
| 原 Rust 只把 B.start_index 改为100，其余不变 | segments=同一个 Some | 函数忽略中间连接索引；它不会替调用者守住接续。本负控制不作为有效分解 |

本例中“缺完成/层级/身份”不是函数返回 None 的原因——这些字段根本不进入所查局部函数。它们是**函数接受之后仍未证明的语义义务**。须区分计算拒绝原因与完整合同未获证，不能把前者的通过冒充后者全部通过。

## 6. 有限回执、复现与下一候选责任

运行 [f2-contract-evidence/run.sh](f2-contract-evidence/run.sh)，当前树中命令为：

```sh
sh .chanlun/review-results/issue1467-f1-proof/stage49/f2-contract-evidence/run.sh
```

成功回执 [receipt.txt](f2-contract-evidence/receipt.txt)：Lean 4.31.0、Rust 1.98.0，退出0。7份原 Origin 源冷编通过；10个研究声明中9个具体有限声明无公理依赖，一般整数表示等价使用 `propext, Quot.sound`，没有 `sorryAx` 或新增 axiom。原 Rust 两入口的全部具名检查通过。原模块 source/olean、本探针 source/olean、实际 Lean/Rust 可执行文件及工具链文件 SHA-256 均保存于 [source-build.json](f2-contract-evidence/source-build.json)，10条精确声明在探针源码，并由成功回执中的 `#check` 读回。

本回执是 **direct Lean 冷编、具名 root elaboration/olean 生成、`#check` 和 `#print axioms` 作者检查**；没有调用 `verify_lean_project`，不称正式 exact-root 验证流程通过，也不称独立 readback 或独评通过。初次开发失败 [receipt-attempt1.txt](f2-contract-evidence/receipt-attempt1.txt) 退出1：两个目标未展开 `CenterConfirmedComplete`，导致缺 Decidable，Lean 的失败诊断带 sorryAx；修复仅展开实际定义再 decide，未改目标、输入或正式源。最终回执与初次失败分存，不能引用失败声明为证明。

临时 olean 和可执行文件只写本次证据目录下的临时子目录，运行退出后清理；formal 无构建写入。本轮不改 formal 源，因此未重复 fixture 漂移或全仓测试。来源、读码范围与成功机器版本另见 [sources.json](f2-contract-evidence/sources.json)，冻结字节见 [SHA256SUMS](f2-contract-evidence/SHA256SUMS)。复现脚本绑定本研究树绝对路径和 HEAD；迁移目录时须显式更新路径，并生成新身份，不能沿用原冻结 hash。

下一候选可以采用“实际端点 + 完整范围”并存的研究载体，或给一份有证明的标准化重分关系。前者无须为适配旧 Segment 强行丢原成员；后者须回答重分如何改变成员/核心归属和确认支持。两条路径均需独立解决：完整最低走势及连接的身份、P 的实际完成、P 参与 Z-3 时的方向桥、给定规则的存在与唯一分解、确认因果性，以及父实例完整交付。这些未决项不会因本轮局部三交成功而消失。
