# #1467 OC-v1 独立审查

日期：2026-10-04。审查对象仅为 SHA-256 `5b0d92d2a32d885ff89647be31c089226ea228fa1cad1385db1825cf20270990` 的 `owned-core-candidate.md`。名分：仓外研究审查，未裁定教义、未修改候选或生产代码。

## 1. 判定

OC-v1 作为报告明定域内的数学关系，可以接受其有限前缀存在唯一、事件全覆盖、自有核心不重不漏和完成列表扩展稳定的结论。下文给出独立普通证明，并以独立实现抽核。没有发现阻断这份“候选关系”交付的构造错误。

它尚不能获得“当前原义完成走势直接接续、直接进入固定 F2”的资格。两项明确未过的接口义务是：

1. 五个平台产生的 Down→Down 相邻完成对象，与 `017:60`、`018:38` 对原义下跌完成后接续的要求冲突。冲突以“OC-v1 完成对象就是原文相邻完成对象”为前提，不能推广为所有 F1 均不可能。
2. 七个交替平台产生 Up/Up/Up，三个真实范围严格重叠，但当前 `zhongshu.md` 的 Z-3 完整判据要求方向交替。几何交集正确仍不足以证明新 a1 满足固定 F2 的组件与方向接口。

原报告已保留这两项义务，没有把它们声称为通过。审查同意这项范围控制。§10 的有限纯趋势家族推论成立，须保留其假设与“已经出现至少两份完成对象”的前件。

另有一处应改的统计措辞：“403 条历史”应为“403 组坐标测试输入，7,708 次前缀检查”。其中只有三份独立的原生订单事件历史被实际簿重放，Q/W 坐标各检查一次；381 与 16 是坐标列测试。此项不影响关系结论，但不能作为 403 份不同真实订单历史的证据。

## 2. 独立普通证明

下述是人工可检查的普通数学证明，不是 Lean 机器证明，也不以 Node 程序确定性代替关系唯一性。

### 2.1 定义域与存在唯一

令事件后观察列为 x1…xn。最大常值平台分解唯一。D_Q 中，每个封口平台至少三事件，开放末平台可为一、二或更多事件，因此成熟核心必为平台序列的连续前缀。空历史与初态不计作事件，非空历史的 x0=x1 条件保证启动没有额外未表示的跃变。

从首个未分配成熟核心 a 开始。零个或一个核心时输出被规则唯一指定。有两个以上时，c_(a+1)−c_a 非零，其符号 d 唯一。若还有完成对象，它的终核 b 必须同时满足：

- a…b 内相邻差号全为 d；
- b+1 存在，且 c_(b+1)−c_b 的符号为 −d。

在首次变号之前结束不满足第二条，越过首次变号不满足第一条，所以 b 唯一。若没有变号，不得发射已完成对象，只保留活动块。移到 a=b+1 后应用同一论证。每次发射消耗至少两个自有核心，故有限输入必终止，存在且只有一份完成列表及活动核心分配。各确定字段由唯一平台、块和事件序列求出，整个声明元组随之有值且唯一。

这是 OC-v1 规则的唯一性；没有证明其他合法 F1 必须采用这份规则。

### 2.2 事件与核心分别覆盖

完成对象核心块依次是 [a,b]，后一块从 b+1 起；残余成熟核心全部属于活动对象。完成支持 K_(b+1) 被读取不改变归属，故成熟核心恰分配一次。

K_(b+1) 成熟时，其首事件早已封口前平台，故 t_b=s_(b+1)−1 有确定值。完成对象的事件区间依次为 `(0,t_b1]`、`(t_b1,t_b2]` 等，尾部 R 为 `(T,n]`。这些区间逐一相接并不交，联合为全部源事件。每个自有核心的整个平台也确实位于对应事件区间内。状态 B_T 可在几何边界复用，不是把事件 e_T 再拥有一次。

零核、单核、尚未成熟的反向平台均留在 R，没有缺口。R 的保留只证明事件所有权完整，不证明它已是一份原义完成走势。

### 2.3 D 内扩展稳定

已完成对象的截止平台已经封口，确认核心已有固定三事件种子。D 内追加既不能改变旧平台起点、值、封口末端，也不能改变旧核心的成熟生辰和既存首次反向位置。第一份完成对象不变；移到其下一起点归纳，得到旧 C 是新 C 的逐字段相同前缀。range 仅读取固定的 B_T…B_(t_b)，故也不变。

该定理量化的是有效域内追加，并不声称短平台被封口后的域外记录仍属此关系；也不声称活动尾部必须在未来完成。开放平台目录字段可以继续增长，不能把“完成 C 不重绘”写成“整个 O_n 永不变化”。

### 2.4 Q/W 的条件

在固定 b=100、a=102、K=10、q>0 的 D_W 中，

`W(v)−W(u)=20(v−u)/((v+10)(u+10))`。

因此 W 严格递增，平台身份、成熟时点、差号、所有权及完成确认保持，有限 range 的上下界直接映射。D_Q 允许 q=0，不能据此把含零量的检验说成 D_W 检验。W 与 Q 的结构等价只在声明的固定盘口条件下成立。

## 3. 15 与 21 事件见证

### 3.1 最短相邻同向完成

`10³,20³,10³,20³,10³` 的独立复算为：

| 对象 | 自有核心 | 独占事件 | 原始路径 | Q range | 确认 |
|---|---|---|---|---|---|
| M1 Up | K0、K1 | e1…e6 | 10→20 | [10,20] | e9 |
| M2 Up | K2、K3 | e7…e12 | 20→10→20 | [10,20] | e15 |

K4 在 R=e13…e15。两份 W range 均为 `[101,304/3]`。M2 起终值同为 20，端点包络 `[20,20]` 漏掉 10，故不能把该对象无损装入仅按首末价恢复全部范围的旧 Segment 表达。

镜像 `10³,5³,10³,5³,10³` 产生同样事件归属的 Down→Down，M2 原始路径是 5→10→5。没有把其入口上涨删掉；问题是它被整个标为第二份下跌完成对象。

两份完成对象至少消耗 2+2 个不同成熟核心，第二份完成又需要其后一个确认核心，故至少五平台。每平台成熟至少三源事件，15 达到下界。初态不能补作其中一次事件。一般地，本关系 r 份完成对象至少需 2r+1 个成熟核心；这给三份完成对象的 21 事件下界。结论仅针对 OC-v1。

### 3.2 原 21 事件长例

独立订单簿重放得到 `10³,20³,12³,6³,10³,16³,11³`，不存在复制快照充当源事件的步骤。

| 对象 | 自有核心值 | 方向 | 事件 | Q range | W range |
|---|---|---|---|---|---|
| M1 | 10、20 | Up | e1…e6 | [10,20] | [101,304/3] |
| M2 | 12、6 | Down | e7…e12 | [6,20] | [403/4,304/3] |
| M3 | 10、16 | Up | e13…e18 | [6,16] | [403/4,1316/13] |

确认分别为 9、15、21，R=e19…e21。Q 三范围交 `[10,16]`；W 三范围交 `[101,1316/13]`，宽 `3/13`。这份输入的数值恰与 v2 一致，不支持两种关系在所有输入上相同。

### 3.3 起终反向与固定 F2 的见证

`10³,30³,15³,20³,10³,25³,10³` 的三份完成对象全为 Up，range 依次是 `[10,30]`、`[15,30]`、`[10,25]`。M2 从 30 到 20，内含 15；M3 从 20 到 25，内含 10。真实 Q 交为 `[15,25]`，W 交为 `[506/5,710/7]`，宽 `8/35`，均正确。

更短数值表达的同一接口压力是七个交替平台 `10³,20³,10³,20³,10³,20³,10³`：三份完成对象全 Up、全 range `[10,20]`。纯区间交判真，方向交替判假。把中间一份改标 Down、删去入口路径、共享核心或移动既定事件边界，都会改变本次受审关系。

## 4. 原义证据与作用域

所有课号均以指定研究 worktree 的 `docs/chanlun/text/blog/` 为准。本审查现场复核行号、正文界、行首编注和行内编注。

| 来源 | 现场归属核查 | 能支持的结论 |
|---|---|---|
| [017:60](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/017-第17课.md:60) | 正文界 98 之前；该句无行首、行内编注 | 下跌完成后只能转化为上涨或盘整；直接支持接续反例的前提约束 |
| [018:38](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/018-第18课.md:38) | 正文界 68 之前；该句无编注 | 判明下跌结束后须面对盘整或上涨，与上句一致 |
| [035:16、18](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/035-第35课.md:16) | 正文界 38 之前；所用分类、结束和经过区间句无编注 | 最低层的局部核心计数、同向与结束，以及三份类型的全程区间交；前提仍包含类型资格 |
| [018:24、26、28、40、42](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/018-第18课.md:24) | 24 行确有行内“娇”注；本审查只依赖注外的原文。其余所用句无编注 | 三个完成组件、分类与允许延伸；不把“可以完成”读为“已经完成” |
| [020:48、52、54、58](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/020-第20课.md:48) | 正文界 94 之前；所用句无编注。56 行有行内注，未采用 | 全程高低、交替回升回调形态的简式、核心和外缘不同、趋势依外缘分离 |
| [037:164–174](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/037-第37课.md:164) | 作者署名与 2007-03-20 17:39:28 的答疑；不是正文。170、172、174 同时含读者问题与等号后的作者回答 | 只取作者回答支持初始自由与构成项不能重复入括号；未把读者提问当作者断言，也未用 176 行加粗复写充作独立来源 |
| [038:18、22、24、26](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/038-第38课.md:18) | 正文界 44；14、42 的明确注释未采用 | 同级别读法的自由有自身作用域，不能自动证明本候选可递归造上一级 |
| [084:52、54、56、60](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/084-第84课.md:52) | 正文界 68 之前；所用句无编注 | F1 可不同且可用成交量，F2 递归规则保持；级别无绝对时间承诺 |

在报告的条件 H 下，点核心确有 `DD=GG=c`，OC-v1 的内部单调核心满足外缘分离，首个反向成熟核心符合第 35 课的局部趋势结束条件。这是一个正确的条件推导。H 本身没有被证明。尤其不能从平台的 Q/W 值直接取得原文逐笔成交核心资格，也不能把整对象 range 当作单个核心外缘。

本审查不新增“任何新 F1 都必须等价成交价”的义务。084:52 明许不同初始定义与成交量。需要解决的是候选所声称继承的原义完成、连接与固定 F2 接口，不能一边援引第 35 课取得趋势和完成语义，一边把第 17/18 课的接续约束无说明地移出作用域。

只有在 OC-v1 的相邻 M1/M2 被声称为原义相邻完成走势时，Down→Down 才给直接反例。若改称研究内部对象，或另设合法映射到原义对象，需要证明映射的成员、事件、边界和资格，现报告没有该证明。结合律要求先有可合法结合的对象，不能直接填补这个缺口。

无已完成盘整输出本身不构成分类反例。无限单核历史也不单独构成失败：018:40 明许无限延伸，084:60 不给级别增长的绝对时间承诺。本审查没有添加“每个活动尾必须完成”的公理，也没有据这些段落宣布全体无限历史原义完备性已通过。

## 5. 当前 Z-3 与代码证据的不同身份

[zhongshu.md:83](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/zhongshu.md:83) 明定三个次级别单元方向必须交替，117–126 的完整判据再次列出该项及严格 `ZD<ZG`。89–91 的“方向交替恒真、实现等价”论证只针对旧 L0 Segment 可达域，不能移植为新 F1 的性质。

因此，三份 Up 且严格相交说明 OC-v1 新输入触及 Z-3 接口，不说明当前正本已经撤销方向要求。第 35 课的区间交原句也以三个“走势类型”为前提，不能略去候选的类型资格直接认定它产生了合格父核心。这同时解释了为什么单次 F2 返回 None 不是一般非法组件证明：合法三元组也可能没有中枢；这里的具体压力来自“假设该三元组就是原义连续完成类型”与当前完整判据无法同时落实。

源代码仅作载体查验。图工具没有本研究 worktree 索引，先用主仓 Rust 图定位函数，再直接读取指定只读树，未把图缓存当成该树版本：

- [UnitRange](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/classifier/center.rs:83) 能分别保存 direction、lo、hi，说明可容纳这些数值，不证明组件合格。
- [center_from_segments](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/classifier/center.rs:241) 先检查交替，再检查严格交集；Up/Up/Up 会在第一支被拒。
- [center_from_window](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/classifier/center.rs:283) 只查几何交集；它接受数值部分不能授权新的 F1 跳过教义。
- [segHigh/segLow](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterConstruction.lean:51) 取首末价极值。短例 M2 给出该投影对 OC-v1 不保真的直接例子。
- [centerHolds](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterConstruction.lean:110) 当前源码明确是严格几何必要条件；[DirAlternates](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/formal/Origin/CenterComplete.lean:83) 另有方向要求。Rust 附近仍有把旧 Lean 描述为 `≤` 的历史注释，不能覆盖当前 Lean 定义式。

没有运行生产 F2，也没有运行 Lean 构建；以上实现结论来自所引代码及独立数值计算。没有用实现现状支持语义放行。

## 6. §10 的量词核查

取从首核开始的交替两值列 H,L,H,L,…。令候选家族满足：每份已完成对象有至少两个连续严格同向的独占核心；完成前缀从首核开始、不漏核心、无其他类型完成对象插入；趋势方向取本对象核心方向。

任意连续三核心都不严格单调，所以每份完成块只能恰取两个核心。由不漏、不共享，从首核起的各块只能是 `(H,L)`、`(H,L)` 等，方向均 Down。于是：

`家族假设 ∧ 已存在至少两份这样的完成对象 ∧ 直接原义接续身份 ⇒ 与 017:60 接续要求冲突`。

这不是 `每个候选都会最终输出两份对象`，也不是 `每个活动尾必完成`，更不是 `所有 F1 都不可能`。允许只保留一个或零个完成对象会使前件不成立，不能据本推论强迫其发射。L,H 镜像同理给全 Up，但直接原句最充分的见证仍可使用 Down 镜像。

## 7. 机器证据及独立性

证据目录为 [owned-core-independent](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent)。

### 7.1 实际运行作者嵌入脚本

从原稿唯一 `javascript` 代码块提取代码内容，末尾加一个 LF 保存为 [author-embedded.mjs](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/author-embedded.mjs)。除这一个文件末尾换行外，内容逐字节相同。Node `v22.23.1` 运行，退出码 0。完整 stdout 存于 [author-stdout.jsonl](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/author-stdout.jsonl)。尾行实际为：

```json
{"totalEnumeratedHistories":381,"histories":403,"checkedPrefixes":7708,"firstSame":[10,20,10,20,10],"first3Same":[10,20,10,20,10,20,10],"allAssertionsPassed":true}
```

计数拆开为：三份订单历史的 Q/W 各一次共 6 组、120 前缀；381 组坐标列、7,302 前缀；16 组变长坐标列、286 前缀。总计 403 组、7,708 前缀，包含空事件前缀和重复坐标测试，不是互异真实历史计数。

### 7.2 独立实现与非贪心关系求解

[independent_check.py](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/independent_check.py) 没有导入作者库、平台函数、域判定或 Rat。它使用三种独立路径：

1. 按事件在线更新平台，只在第三事件出生时处理核心，以流式状态机形成完成对象。
2. 对成熟核心列枚举所有满足声明关系的可能分块和活动后缀，检查解集合恰有一个，再与在线结果比较；没有贪心选择首次切点。
3. 手工录入三个例子的订单动作，独立校验订单 ID、价位、正量和撤单残量，逐事件重建最优档及 Q/W。Python `Fraction` 精确运算，作者 stdout 仅用于最后对象逐字段比较。

Python `3.14.6` 实际运行退出码 0。[independent-stdout.jsonl](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/independent-stdout.jsonl) 记录三份订单重放及以下结果：

- 坐标字母表 `{0,1,2}`，1…8 个相邻不同平台，两种长度安排为全 3 及 `3+i%4`，共 1,530 组测试输入、41,286 个前缀；这些仍是测试调用计数，未声称互异。
- 每个前缀穷举关系恰一个解，与流式实现一致。实际枚举全部事件拥有者并查重，而不只检查相邻区间端点；成熟核心也逐个查不重不漏。
- 已完成字段前缀稳定、整个平台事件归属正确、确认等于三事件种子的末事件，全部通过。
- 零量 D_Q、空历史、单核长延伸、短未成熟尾均覆盖；四个初态不一致或封口短平台的非法坐标列被拒绝。含零量用例没有套用 D_W。
- 首个两份同向见证为五平台、15 事件；首个三份同向为七平台、21 事件。三份独立原生订单历史的 Q/W 最终结果分别与作者输出完全相同，逐前缀 Q/W 结构与范围映射也通过。

这加强了构造检查的独立性，仍不能取代 §2 的全域普通证明或 §4–5 的语义桥。嵌入脚本和独立脚本输出的是便于核验的关系投影，并未序列化正文 O_n 的全部原始事件载荷与 K 目录；完整元组的存在唯一仍由定义与证明承担。

### 7.3 可复跑命令与哈希

在本机执行：

```bash
node /Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/author-embedded.mjs
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/independent_check.py
shasum -a 256 /Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-candidate.md
```

最后一条实际结果为：

```text
5b0d92d2a32d885ff89647be31c089226ea228fa1cad1385db1825cf20270990  /Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-candidate.md
```

其余关键 SHA-256 已现场核实：

```text
ff4e1d062e78891ff2c563ae06ba3ac842ac10885a5d275795fcd852624daa43  PlateauBaseCandidate-v2.md
0a630a6bbc418a0fa57be621ea2089123005a5b3dcd712622c81738f2a3515ac  plateau_base_probe.mjs
a021c88c8b60b788ae1d87097109d6658db679b7f2fa1b962861d6bc567dc6d0  stage42/classification-bridge.md
f44d1623803ceab8443f09cfc7b9fd4d4f6a477281221a007c9d4414fa0c34cc  stage42/shared-core-candidate.md
428a34f58e4e4b678509f69afba6d7bc06f9ede94f798ed75e4b0bbe5dd41955  author-embedded.mjs
340560eba2a4fb3f72a1f298bf5249322b2aa2f150984b0b1172f6703b2ec58b  author-stdout.jsonl
a54e1885885520e37e33eec4149a38c5048deca04152629f32ad66c774a0ae2f  independent_check.py
cce504a18f65ed23d3b324db4a5c7610ac1bb479ed9e7b0d261603e3ac859384  independent-stdout.jsonl
```

原稿代码块内容不带末尾 LF 时，SHA-256 为 `cac5aabbaecb9f1d7a3b9972a87fca8ae768ca0ccf246426f77aefdf2458e9fd`；保存为带一个末尾 LF 的文件后是上表 `428a34…`。二者哈希不同仅由该换行造成，已按完整内容比对确认；不声称这两个字节文件哈希相同。

[manifest.json](/Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/owned-core-independent/manifest.json) 保存完整绝对路径、原课与代码哈希、运行时版本及只读 worktree HEAD `8a6894b488943de95dc2c25c1b3e9832893f4293`。该工作树原有未提交面，不把 HEAD 当成全部工作树内容哈希。

## 8. 阻断项和建议

| 项目 | 对本次候选研究交付 | 对原义/固定 F2 资格 | 建议 |
|---|---|---|---|
| 关系存在唯一、覆盖、D 内扩展稳定 | 普通证明与独立有限检验均通过 | 不单独授予资格 | 保留关系及证据，无需为迎合方向交替改动它 |
| Down→Down 相邻完成 | 反例真实且范围声明正确 | 阻断“直接就是原义相邻完成走势” | 证明连接约束的层级作用域或提出有证据的合法映射；不能只把入口反向片段改名 |
| Up/Up/Up 严格范围交与 Z-3 | 数值计算通过 | 固定完整接口尚无一致证明 | 明示组件资格与方向前提，另证新 a1 的接入；不能用几何函数返回值放行 |
| §10 家族推论 | 有限条件命题成立 | 不外推全 F1 | 总结中保留家族假设、完成前缀前件及直接原义身份前提 |
| 403 的统计称谓 | 非阻断性报告修正 | 无额外语义效力 | 改称 403 组坐标测试输入，分别列三份原生事件历史与符号列测试 |

没有据此主张全体无限历史的原义分解完成、递归闭包成立、市场信息增益、交易角色或盈利。没有修改原报告、候选文档、教义、生产代码或 Lean；所有新文件只在本报告及自有仓外证据目录中，没有提交、推送或外部消息。
