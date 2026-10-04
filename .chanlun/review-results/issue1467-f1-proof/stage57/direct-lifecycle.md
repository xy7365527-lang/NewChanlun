# Stage57 直接订单路线的初始生命周期适配

2026-10-05，研究 #1467，归图 #1465。名分：工作草稿。范围只含本稿与 `direct-evidence/`；仓外验证在 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/direct-author/`。不改旧包、教义、formal 或生产。采用本树 math-research-workflow、rigorous-open-math-research、verification loop 与 lean-verify。以下是作者结果，独立审查由主控另行安排。

结论是有条件的肯定。直接 R_D 的已确认 pressure Flow，在原 Step 合法、报价 Reads 真实、长度匹配且全程有效双边 QuoteOf 的历史上，满足 D54/L56 实际使用的相接、严格交替端差和完整范围条件。本稿证明一个保留订单簿与原事件的 primitive-leg 适配定理，并给出一个非空数值 Closed⁰ 见证。它不是原始每事件实数序列上 D54 定义的字面实例；原订单簿对象有双边报价和原始事件，不能省掉这层适配。原义 P、B56 与固定 F₂ 仍未证明。

## 1. 依赖与限定域

实际定义采用 `MovingQuoteSpec.lean` 的 Event、Book、Step、QuoteOf、Flow、leg、readout、group、locate，构造仍为 `MovingQuoteWireSpec.lean` 的原 `construct`。原 `DirectOutput.exact_root` 与 `DirectOutput.actual_exact_root` 的声明、证明及 Stage47 独评已读回。这里复用其源所有权、相邻输出来源及实际 L1 支撑包络，未把 Stage46/47 的包络换名当作本轮新结果。

设源事件为 `es=[e₁,…,eₙ]`，实际簿状态为 B₀,…,Bₙ，报价读数为 qs。声明域要求：

- `BookQuote.Through es B n`，每条原事件确实满足原 `Step`。
- `BookQuote.Reads B qs` 且 `qs.length=n+1`，所有已记录报价与实际簿一致。
- 每个原状态切点 k≤n 都有 `qs[k]=some q_k` 和 `QuoteOf(B_k,q_k)`。

最后一项是明示的全程双边有效子域，不由 constructor 的长度检查代替，不等于全部合法订单历史。深档和数量可变，最佳报价可不动；不固定100/102，不要求存在生命周期成功。价格仍是原 Int。ID 控制仅为单位订单的保守来源补充；原 Book 没有 orderId 字段，投影后的合法性由 Step 单独核。

当某切点没有有效双边报价时，Reads 将其如实记为 none。原输出 geometry=None 必须保留，不能过滤后把两段拼为相邻。本轮没有为带 None 的全域生命周期定义新恢复协议。

## 2. 从直接 Flow 到可复用关系的适配定理 A57

记未筛选原输出的第 i 块拥有 `(aᵢ,bᵢ]`，pressure 为 σᵢ，已确认时刻为 κᵢ。只把 `knownAt=some κᵢ` 的原始连续前缀交给生命周期关系。活动尾仍保留。成功几何端值按原构造为：

- 正 Flow：`uᵢ=q_(aᵢ).bid`、`vᵢ=q_(bᵢ).ask`。
- 负 Flow：`uᵢ=q_(aᵢ).ask`、`vᵢ=q_(bᵢ).bid`。

源证书保留整个 `B[aᵢ..bᵢ]`、`qs[aᵢ..bᵢ]`、自有原事件、Step、Quotes、pressure 与 κᵢ。完整范围的对象是实际所有 L1 spread 的支撑 `Sᵢ=⋃_(aᵢ≤k≤bᵢ)[bid_k,ask_k]` 的最小包含闭区间 `Iᵢ`，不是所有深档价格、事件触价或成交价的范围。

**A57。** 在第1节域内，原 construct 的每份输出都有几何。其已确认前缀具有：

1. `aᵢ<bᵢ=aᵢ₊₁`，原事件连续且无重无漏，几何索引等于源索引；相邻边界是同一个原簿状态及同一实际 Quote。
2. σᵢ₊₁=−σᵢ；`vᵢ=uᵢ₊₁`；正块 `uᵢ<vᵢ`，负块 `vᵢ<uᵢ`。
3. `Iᵢ=[min(uᵢ,vᵢ),max(uᵢ,vᵢ)]`，其两端来自该块实际首尾报价；块内实际 bid/ask 分别沿 σᵢ 弱单调。
4. κᵢ=bᵢ+1，是原第一条反号事件到达时刻；已确认块、源事件、报价窗口及全部证书字段追加稳定，κ 严格递增。

因此在这个 primitive-leg 接口上逐字采用 D54 的最早三范围 seed、逐邻 L/R、触闭核、首个坏角色停尾、M/H 区分、whole 到 L.end、R.knownAt 发布及下一游标规则，得到一个明确的直接订单关系 `D54ᴰ`。它继承 D54 普通证明中关系内存在唯一、有限终止、seed 保留、成员源包含、事件完整覆盖和已发稳定。L56 的精确结论也成立：成功 whole 所有完整连续三条 primitive 腿的正宽交 J，都与固定 C 闭交非空。

**证明。** group 的非空同号块按原事件唯一连续分割。原 `DirectOutput.source_contract` 给出每块的位置、uniform、确认；`adjacent_contract` 给出相邻源切点、反号及双 some 共端价。全双边域下 readout 的全部报价检查成立。`quote_step` 和传递性给每块 bid/ask 同方向弱单调；正块有 `q_a.bid<q_a.ask≤q_b.ask`，负块有 `q_b.bid≤q_a.bid<q_a.ask`，故 readout 的严格端差检查成立，所有原块都是 some。这没有假定几何成功后再借其存在来证明全域成功。

得到 some 后，`DirectOutput.actual_exact_root` 把 Iᵢ绑定到该输入 B、qs 的实际全部切点，而非另一条同端点兼容 Flow。跨度、方向和严格差还由新增 Lean 辅助引理 `Stage57Direct.primitive_strict` 直接对 actual output 证明。确认稳定由第一反号事件已经存在及原分组规则推出；它不需要任何生命周期标签。到此取得1至4。

为检查 D54 迁移没有偷加标量前提，取任意有限的这些 primitive 腿，定义边界骨架 `y=(u₀,v₀,v₁,…)`。1、2保证这个骨架每个相邻差严格非零且交替，其每个 primitive 恰是一条最大单调边。骨架只是证明用的有限代数表示，不称原始逐事件状态，索引也不是原事件编号。3保证任意完整连续腿片的骨架端点包络，恰等该片全部真实 L1 支撑的包络。因此 D54 的所有区间判断、端值角色判断和外缘运算都保持；用源切点映射回原事件，所有权、M/H、前导与释放 L 都保持。最早seed唯一、逐邻检查唯一、游标严格增加的证明逐步成立；成员末端等于 L.start，故 MemberConflict 在本域不可达。只使用已确认腿及 κ，得到因果与追加稳定。这是新适配证明，不是调用旧标量域标签就宣称合法。

L56 的复用也可直接核最承重一步。seed 各 I 含 C。若第一次 L/R 即成功，好角色迫使 L 跨 C 的相应边界。若先有一次触核，当 seed 末腿向上，其末值≥C.high；随后 L向下、R向上，共端值及R触C迫使L末值≤C.high，所以L包含C.high；向下对称。此后每条新腿曾作为触C的R，最终L也触C。于是 seed 至 L 每个I均触C。三腿正宽交 `J=[max low,min high]` 满足 `J.low≤C.high`、`J.high≥C.low`，因而闭交非空。seed之前的完整三窗由最早性排除，跨seed起点的窗也曾被检验。whole从cursor腿起点到L末点，只包含完整腿，证据R在外。这个论证不需要将二维Quote误作单一真实价轨迹。证毕。

这不是另证“每个原事件的某一既定标量观测都是单调”。例如正深档事件可能不动任何L1报价，但原leg仍跨一个正价差。R_D允许这份多变量来源证书。若改成 W(q)、中价、成交价或另一逐事件标量再封腿，须另给适配；本稿没有把它偷换为 R_W。

## 3. 8条原始订单事件的非空见证

输入锁为 `direct-evidence/controls-v1.json` 中 `D57-main8`。初簿三个单位订单为 b100@100、b1@1、a102@102；事件/订单ID逐条保存，撤单必须命中活跃的相同ID及价格。每次add按原Step前插，cancel删除实际列表中的对应价格单位。状态表中的买卖列是投影后原Book的完整价格列表。

| 切点 | 原事件 | pressure | 买列表 | 卖列表 | 实际Quote |
|---|---|---:|---|---|---|
| 0 | 初簿 | — | 100,1 | 102 | 100/102 |
| 1 | E1 add bid b101a@101 | + | 101,100,1 | 102 | 101/102 |
| 2 | E2 cancel b101a | − | 100,1 | 102 | 100/102 |
| 3 | E3 add bid b101b@101 | + | 101,100,1 | 102 | 101/102 |
| 4 | E4 cancel b101b | − | 100,1 | 102 | 100/102 |
| 5 | E5 cancel b100 | − | 1 | 102 | 1/102 |
| 6 | E6 add ask a99@99 | − | 1 | 99,102 | 1/99 |
| 7 | E7 add bid b2@2 | + | 2,1 | 99,102 | 2/99 |
| 8 | E8 cancel b2 | − | 1 | 99,102 | 1/99 |

全程bid<ask，八个Step、九个Reads及所有原construct前缀单独核。E6新增ask99不交叉，因E5已经留下bid1；提示中的事件无需修正。

| 原Flow | 自有事件 | 原leg端值 | 实际完整L1范围 | 确认可知 |
|---|---|---|---|---|
| F0 | E1 | 100→102 | [100,102] | E2 |
| F1 | E2 | 102→100 | [100,102] | E3 |
| F2 | E3 | 100→102 | [100,102] | E4 |
| F3=L | E4,E5,E6 | 102→1 | [1,102] | E7 |
| F4=R | E7 | 1→99 | [1,99] | E8 |
| F5，活动 | E8 | 99→1 | [1,99] | 尚无 |

第一次seed固定F0/F1/F2，C=[100,102]，发生末端3，可知4。首次L/R为F3/F4。R.high=99<100，L向下且102≥100>1，R向上，好角色成立。没有触核迭代。H=M={F0,F1,F2}；L原本不在H，releasedFromDevelopment为空，不能冒称展示了“先触核再释放”的分支。

唯一数值Closed⁰/P⁰对象的身份为`D57-main8:K0`。whole拥有E1…E6，起点0、末点6；核成员拥有E1…E3，memberOuter=developmentOuter=[100,102]；wholeOuter=[1,102]。L拥有E4…E6，全部归whole且不作最终核成员。R的E7归右侧尾部，E8为确认支持，二者均不归此whole。尾部拥有E7、E8，真实闭L1范围[1,99]。M、L、R、确认支持和whole拥有关系分别保存。

whole内两个完整三腿窗F0/F1/F2与F1/F2/F3的交都为[100,102]，满足L56。没有计算原义Zn或DD/GG。所有原义资格字段保持null。

## 4. 时钟与再次封腿的区别

原construct在n=0…8的已确认Flow数为 `0,0,1,2,3,3,3,4,5`。D54ᴰ在n≤3为NoCore，n=4…7为AwaitPair，n=8首次发布上述whole。E6只是拟发生末点；当时L与R都没有完整已封证据。E7封L但R仍活动，仍不得发布。E8第一条负事件封R，发布时刻为8。

D54ᴰ直接消费原Flow的封定证书，不把它们再等待一次。有限端点骨架可用来比较区间计算，但其活动末端会随同号订单更新，不能称为一个普通逐原事件追加的固定标量串。把冻结D54纯关系用于某一前缀的完整骨架，包括活动Flow当前端值，算出的数值分块与D54ᴰ一致；其封腿位置k须映射为原第k−1条Flow的firstKnown，而不是该活动Flow后来延长的end。真实κ由原第一反号事件冻结。

若另一个接口只输出“已确认Flow的端点”，然后再次调用标量batchLegs，n=8只有骨架 `100,102,100,102,1,99`。最后的1→99尚缺下一骨架反向值，该二次封腿接口此时只有四条封腿，没有D54输出。要到F5自己确认并向该接口交付端点之后，才能二次封R；这可能多延一条或多条原事件。本包不把这种接口在E8提前发表。两个接口不同，A57证明的是保留原封定证书的D54ᴰ。

## 5. 必要None控制与边界

第二且最后一个控制为 `D57-none7`。初簿bid100/ask102；依次addBid101、cancel101、cancel100、addBid2、addBid3、cancel3、addBid4，ID在输入锁中具名。切点3买侧为空，Reads=None有其不存在QuoteOf的证明。七事件原输出isSome为 `[true,false,false,true,true]`，knownAt为 `[2,4,6,7,null]`。

F0结束在切点1；其后两个None块拥有E2…E5；下一some F3始于切点5。删None即丢四条原事件并把不相邻的来源伪作相邻，即使筛后的几何端价偶然相等也不合法。检查器保留全部五块和七条原事件，不执行A57范围外的生命周期拼接。这个例子是有效Step/Reads而不在全程双边子域的历史；它不是feed丢包或未知报价的模型。

本轮只运行这两份控制，共15条原事件、17个含初态前缀，市场样本数0。没有扩大枚举或下载数据，没有声称一般JS程序精化。JS核原ID状态、每个前缀L1几何、来源、时钟、数值D54ᴰ、L56窗口及冻结标量D54的边界骨架结果；原Lean另核Step、Reads与原construct的逐前缀完整字段。

## 6. 可复用范围与后续接口

A57是普通数学证明。新增Lean的`primitive_strict`只覆盖实际some输出的源跨度、方向与严格端差；`witness_root`覆盖两控制全部前缀的源合法性与原构造字段。它们不形式化整个D54ᴰ扫描关系，不证明全部订单协议或原义B56。完整Lean回执、源哈希及作者检查结果见 `direct-evidence/verification-summary.json` 与 `manifest-v1.json`。

可交给独立TouchSpan命题的输入是固定seed/C、连续primitive源腿、s…j各完整范围触C、下一已封R=j+1严格不触C，以及κ后封腿稳定。本控制s=0、j=3、R=4满足这些算术前件；TouchSpan结论由另一包证明和独评，本稿不借它预授完成。

L56只排除whole内同一primitive三腿谓词的正宽J与C完全分离；不推出J=C、同一原义核实例或原义核数。P没有已证概念方向；pressure、离开side和whole净变化不能补作固定F₂所需方向资格。原義P、B56相容桥、同向Zn/DD/GG、固定F₂合格孩子与父生命周期均留空。新结果到直接订单来源下的非空初始数值生命周期为止。
