# #1467 Stage46：J 在趋势完成后的适用范围

日期：2026-10-04。研究工作草稿，绑定既有 #1467。只读来源树为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`，本轮核到 HEAD `84ddc99529d5f2c5e043d0bf3c4ae5539a82e4f8`。本文不修改教义、候选、formal 或生产，不提交、推送或外发。

**本轮没有取得“已完成上涨 X 的末核 j，与新下降 Y 的首核 k 仍满足 Up(j,k)”的合格原义反例，也没有证明这种情形在全部合法 F₁ 上不可能。** 新取得的正面源证是：第 43 课明确排除了其中一个最容易误作反例的分支——旧高级别趋势末段只有低级别背驰，而随后同级新核仍与旧末核分离。原文直接把该新核接回原趋势，并否定原趋势已经完成。因此，真实发生低级别转折不等于旧趋势已经完成；用它给 Split 的完成布尔值背书仍不成立。

一般非点核心与 P 平台域必须分开。本文不给范围骨架颁发完成资格；也不拿短尖顶、不同级别的末段核心或另一份外缘，反驳 Stage45 在同一 P、同一目录、精确完成表示和无状态 J 下的条件不可同满足。

## 1. 固定争点与来源层次

Stage45 的 J 是对**任意趋势实例**量化的，包括在当前观察前缀已经完成的实例：

```text
J_all(S) :⇔ ∀ h,ℓ,X,d,i,j,k,
  Trend^ℓ_d(X) ∧ Own_S(X,i) ∧ Own_S(X,j)
  ∧ N_ℓ(i,j) ∧ N_ℓ(j,k) ∧ D_d(i,j) ∧ D_d(j,k)
  ⇒ Own_S(X,k).
```

历史 h 属于声明的合法域；X 是该历史中的语义实例；i、j、k 是同一全目录内已成熟的 ℓ 级核心。`D_+(j,k)` 使用真实外缘 `DD_k > GG_j`。对象和历史不是核心索引。这里不把“当前已完成”当作矛盾本身：若 k 原已属于 X、只是晚些知道，完成后的精确表示仍须包含它。真正待查的是 **X 已在 k 之前真实终结，但外缘比较仍同向**。

已回读 Stage45 摘要、延伸作者稿、独评及 `ScopeClarifications-v1.md`。本轮沿用其条件边界，不重复 Γ₄ 独立性和 1533 条坐标串枚举。尤其“Γ₄ 中可以赋完成值”不是本文需要的完成支持。

本文原课行号均指固定来源树的 `docs/chanlun/text/blog/`。018/020/024/029/035/037/043/044 的作者署名均在 :12；正文界依次在 :68/:94/:52/:90/:38/:40/:52/:66。所用正文逐行核查；018:46–48、044:28、:40–42、:46、:52 及 043:48 的两处行内注不作为作者依据。037:170–174 只采用署名 :164、时间 :166 下问答分隔符之后的作者回答，不采用读者问题或 :176 重复加粗。029:328 的作者署名/时间在 :316，:324 是读者质疑，不倒置归属。

## 2. 一条确有对象身份结论的原文证明

**命题 T43（只在该课明示场景内）。** 设 X 是向上的 30 分钟 `a+A+b+B+c`，A、B 为同级中枢；c 内出现 1 分钟背驰，而 c 对 b **没有** 30 分钟背驰；随后演化出下一个 30 分钟中枢 C。若 C 在 B 上方且真实外缘严格分离，则不能把 X 说成在 C 之前已经完成，并把 C 单独归作新下降的首核。这里用该分解中已形成 C 的合法外缘；没有将新候选的第一次读数当作永久外缘，也没有把原文 C 的形成时刻未经对齐就认作任意新 F₁ 的 β_C。

证明：

1. [043:32–34](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/043-第43课.md:32) 明写第二种情形是本级背驰未成立、只有较小级别背驰。这是本命题的范围，不能省略。
2. `DD_C > GG_B` 强于 B/C 核心或外缘不重叠，满足 [043:48](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/043-第43课.md:48) 的“没有任何重叠”分支。
3. 该分支的作者原话明确给出两项结果：“原来的a+A+b+B+c并不是一个完成了的30分钟走势类型”；“该走势类型将延伸为a+A+b+B+c+C”。这里有原来的实例、否定完成以及将 C 接回它的表达式，已经给出了身份承接；不需要再以“句中没有对象变量 X”为由把它降为无归属的几何描述。
4. 因而 `CompletedBefore_S(X,C)` 与该场景及分离条件不可同真。把同一 C 解释成新 Y 独占的首核，会与这条原文指定的接回冲突。证毕。

这是本轮唯一提出的源义证明。它没有构造一个完成标志，也没有假设必须找到反例。该课 [043:42](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/043-第43课.md:42) 另说本级背驰成立时以背驰点分界；[043:46](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/043-第43课.md:46) 明示接下来分析“第二种情况”。因此，T43 **不能不加推导地提升为所有本级背驰、所有 F₁、所有核心目录上的 J_all**。

## 3. 末段 c 与 B 外缘：哪一步可以说，哪一步不可以说

[037:16–22](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/037-第37课.md:16) 约束标准趋势背驰：A/B 同级；c 必为次级别且至少包含 B 的第三类点；b 的级别不大于 c；上涨 c 创新高；c 内有次级别中枢。它带来两个与本题直接相关的区别。

第一，**X 的最后一个 ℓ 级中枢 B，不等于 X 的价格终点或最高点。** c 可以在 B 之后继续到新高；c 内的核心属于较低级别，不能自动插入 ℓ 级目录并替换 j。一般非点域内，“B 之后有更高价格”本身不否定 B 是末个同级核心。这和 P 每封口平台都出生一枚同目录候选核的机制不同。

第二，**“围绕 B 的波动”不等于“整个 c 自动属于 B 的外缘支撑”。** 037:16 允许另一角度把 b、c 当 B 的次级波动，同时要求保留原趋势与同级 A/B 的读法。GG/DD 的严格取值域却是 [020:52](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/020-第20课.md:52) 的“中枢中所有 Zn”；已裁正本 [zhongshu:194–208](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/zhongshu.md:194) 保留这项成员限定，[zhongshu:374–390](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/zhongshu.md:374) 又把连接段独立于中枢成员。

故既不能无来源地把 c 的最高点直接设成 `GG_B`，据此使 `DD_k > GG_B` 自动失败；也不能仅看 `a+A+b+B+c` 的加号，就宣布 c 内所有后续波动绝不影响 B。应逐段核 Z 身份、离开/回试、破坏和实际归属。特别是 [zhongshu:318–328](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/zhongshu.md:318) 明确反对只见离开就结算中枢；回试是否回来是独立条件。

这些区分排除了两种快捷“证明”，但本身不产生一个完成的 X 或新 Y。

## 4. 本级背驰后的返回，不自动给首个新核的顺序证书

本级背驰具有真实完成支持：[043:18、42](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/043-第43课.md:18) 给原走势终止和背驰点分界；正本 [beichi:521–537](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/beichi.md:521) 给同实例不再延伸与带时间的完成/让位。**可是，某个数值例里真的存在这份背驰，仍要验证 037 的结构条件与力度比较，不能从核范围直接赋值。**

返回条款的强度也要保留原有差别：[024:46](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/024-第24课.md:46) 说回最后 B；[029:30](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/029-第29课.md:30) 对下跌后最弱反弹精确到触及最后中枢 DD，方向反转对应上涨后的 GG；[029:52](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/029-第29课.md:52) 又单列“第 1 个次级别走势是否回抽中枢”作为判别，保留首波未回核心的情形。

从这些话可获得回到末核的幅度约束；本文尚未推得下面这个更强的时间顺序式：

```text
本级背驰在 τ 终结 X
⇒ 在后续第一个 ℓ 级核心 k 出生/完成之前，
   新走势已经触及 B 的相应外缘，且该触及被计入 k 的外缘支撑。
```

该式至少新增“首个同级核心以前”与“触及归 k 支撑”两项，后者不能由新走势整体范围代替。反过来，尚未证此式也**不等于**它已被反驳，更不等于能取 `DD_k > GG_B`。020:58 的上涨及其延续和 018:44 的同实例延伸责任仍须过关。本文没有给出满足这些来源的首核顺序见证，因此不声称 Source 不蕴含 J_all。

## 5. P 域为什么不能用隐藏短尖顶救反例

冻结 RT 的 [域与平台规则 :17–26](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-candidate.md:17) 是：每个已转值封口的最大常值平台至少三个事件；第三次观察唯一候选核出生；整个平台归一个拥有者。P 再把它的真实外缘定为该平台值。**这不是一般非点中枢域。**

若在 j 与声称的下一核 k 之间插入一个更高极值，再跌到 k，那个极值所在平台若只有一两事件即被转值封口，历史已经域外；若平台至少三事件，则它成熟成另一个同目录候选核 m，`N(j,k)` 失效。若 m 与 k 实为同一最高平台，就不能一面将该平台整核给 Y，一面又把该平台的源事件作为 X 结束成员。后一点要求精确成员合同，不是禁止证明跨界读取。

这个域检查只阻止“隐藏的无核尖顶”策略；它没有证明平台候选已取得 ℓ 级原义核心身份，也没有证明独立新 F₁ 上的 J。若明确采用 [035:16](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/035-第35课.md:16) 的旧逐笔最低完成规则，同向新核本身就排除了提前结束；但将该规则迁移到一般独立 F₁ 正是不能预先代填的事项。

## 6. J 的时间量词应怎样写，Stage45 因而缺哪一步

若把 J 用作**活动趋势的承接规则**，至少应把活动性写成独立语义事实，时间放在 k 被承接的时刻，而不是最终报告时刻。设 β_k 是 k 的成熟时刻，`Active_S^ℓ(X,t)` 表示同一语义实例 X 在 t 仍未真实终结。一个保守的待证版本为：

```text
J_live(S) :⇔ ∀ h,ℓ,X,d,i,j,k,
  Trend^ℓ_d(X) ∧ Own_S(X,i) ∧ Own_S(X,j)
  ∧ N_ℓ(i,j) ∧ N_ℓ(j,k) ∧ D_d(i,j) ∧ D_d(j,k)
  ∧ Active_S^ℓ(X,β_k)
  ⇒ Own_S(X,k).
```

活动性按**真实语义终结**解释；若真实终结在 β_k 或更早，前件为假。它不是“程序尚未确认”、不是候选的 `¬Closed_B`，也不是用“尚未分配给 Y”循环定义。若需要覆盖恰在 β_k 终结的边界，应另给同刻事件/核心归属规则；本式不偷取这个端点。X 可以在晚于 β_k 的当前前缀 n 已经完成，式子仍可读取过去 β_k 时的活动事实。

**本轮建议把 J_all 保留为待证的强桥，把 J_live 单列为研究所需的状态版本；不是宣布 J_all 已被反例推翻，也不是宣布 J_live 已由全部 Source 证明。** T43 给出一个具体场景内的活动性和身份承接证据；它比单说“需要 active”更强，又没有越过自身前提。

Stage45 的 RT 条件拒绝在 J_all 假定下仍正确。若改用 J_live，要拒绝某份 RT 完成输出，必须先独立证明其映射语义实例在被留右核 β_k 时仍活动。不能用 RT 自己的提前结束断言令 guard 为假以逃过检查；也不能只因为 D 同向就把 guard 填为真。原文 T43 或已获适用资格的 035 完成规则可以在各自范围内、核对实际时点后承担这一步，单纯 P 算术不能承担。

## 7. 本轮交付边界

已经取得的是 T43 的具名、有角色和级别、有作者身份、且直接否定“先已完成”的来源推导；以及 c 的角色/外缘和返回定理的量词审计。没有取得一般非点域或 P 域的合格原义反例，没有给完成 Bool，没有冻结 Ω，也没有替 RT 或 R_D 取得资格。

本轮只写本仓外本文，未更改 formal，未触发 fixture 漂移检查。未运行市场数据、Lean、订单重放或数值枚举：它们都不能替本文缺失的源义顺序证明。本稿完成后交由新的独立上下文审查；未经接回不称独评通过。

## 附：来源指纹

下表路径均相对于固定来源树。所列原课、定义、Stage45 摘要/订正的工作区差异查询为空；本轮没有据此声称整棵共享树干净。哈希为实际读取文件的 SHA-256。

| 文件 | SHA-256 |
|---|---|
| `docs/chanlun/text/blog/018-第18课.md` | `4a661fc843d0d9fcfdf32861b7dc903ee09d824d0075dd0ea7b2c19542bb2f92` |
| `docs/chanlun/text/blog/020-第20课.md` | `25f8f0bbe9ffbab48a27b552a081ff3925b6c01bfe1cae30c2616fe9b99b49ef` |
| `docs/chanlun/text/blog/024-第24课.md` | `f599722c2c15265fa52bb67139b8f9b11b01a11ff443be05eac51d20e02ccf28` |
| `docs/chanlun/text/blog/029-第29课.md` | `5716dbd674811392ac563cde255edec00c2902e3c3561159faa5ce2118fa369f` |
| `docs/chanlun/text/blog/035-第35课.md` | `0bb81a62b58ecf8900f6c40227194c1fbb139975bae5aa34d7a881363067b023` |
| `docs/chanlun/text/blog/037-第37课.md` | `f360d985b3f3b1fa567ca13e15f2ffaf9eae39dbb418519f75d1f6175ac70753` |
| `docs/chanlun/text/blog/043-第43课.md` | `f28824c26843e5799ad8f07db6bb9b02956d0fc7ac97d9d3d2fc70a7c3004909` |
| `docs/chanlun/text/blog/044-第44课.md` | `827191f7e01799d606520b8774bcc5948b35324dbde6d9a3f05fb5a83be8f70b` |
| `.chanlun/definitions/beichi.md` | `3a3fbe6cad78a402bb3d3d8931e85ce5bfb009c30007f3fa4573e7eb3a2055c6` |
| `.chanlun/definitions/qushi.md` | `ce48f8cda6a3f02afd9fddbc0f93cb43431524b66472e579a927af5ce4bc3c46` |
| `.chanlun/definitions/zhongshu.md` | `069fc240c8476cc3df62c01d1b658c42f00ec4428777d479163beaadc48ec882` |
| `.chanlun/definitions/level_recursion.md` | `06ac7aee96b089536766ac172b52ff0f8d702f2e4ec01ca2b47b1f3a590e1637` |
| `.chanlun/review-results/issue1467-f1-proof/Stage45ExtensionAndExtent.md` | `5ebdaca2e25551785beb92e445c6e16474404a3e3eb03d1ba8e7fbffc239083b` |
| `.chanlun/review-results/issue1467-f1-proof/stage45/ScopeClarifications-v1.md` | `192d3d5f314064116374b9fd5fad3d339310f875873966e228ebc68a39c05ebb` |
| `.chanlun/review-results/issue1467-f1-proof/stage45/extension-ownership.md` | `4064ba28c0f19dbacf9693897502fec803990a1b7e3b7bb81a3a0f219e6158e3` |
| `.chanlun/review-results/issue1467-f1-proof/stage45/independent-extension-ownership-review.md` | `668b8a2b2c7919766f6aa2fd00c3f3c7bdb1ac7ff027801360e183f634721b65` |
