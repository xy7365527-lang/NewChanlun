# #1467 Stage45 延伸归属独立审查

2026-10-04。审查对象为 `extension-contract-v1` 的指定冻结包。审查者未参与原稿编写。来源树只读，HEAD 为 `41a5a649ce7c471d58fbf5c4486ba5cda7a74e9d`。本报告和自有证据目录是全部产出；未修改仓内文件、作者原件、教义、formal 或 RT-v1，也未提交、推送或发送外部消息。

**结论：条件数学结果通过。** 在相同的已获同级核心身份的目录、点外缘合同 P、同型完成表示与 J 同时成立的条件下，RT-v1 每一份实际发射且拟上送的对象，都无法满足 EXT-own。四核和 30 事件例、两组分区计数及一般证明均经独立复核。两个有限模型只证明 J 相对于明示有限 Γ 的独立性。它们不证明 J 独立于全部 Source，也不证明 Join、Split 或 RT 具有 GoodΩ 资格。

有两项非阻断缺陷需要在引用时订正：开放成熟平台的末端记号应区分当前观察末端和最终封口点；作者脚本的 `EXTown` 字段只执行了完整公式的一部分。独立核验补足了本轮两个具体模型的有限解释检查，没有替它们取得原义资格。

## 1. 审查对象和复算方式

受审文件的 SHA-256 与任务给定值全部一致：

| 文件 | SHA-256 |
|---|---|
| `extension-ownership.md` | `4064ba28c0f19dbacf9693897502fec803990a1b7e3b7bb81a3a0f219e6158e3` |
| `extension-check.mjs` | `0f5de4fc2face7e157f7f4ffc85afadb5fcc0ce8da5a91e75df7f3727d232988` |
| `extension-check-results.json` | `09c0b58d63271d7e568ae6ca6dca178be3199193755c89513efe60ecf4f17c91` |

证据目录为 [independent-evidence-20261004T113209Z](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/independent-evidence-20261004T113209Z)。其中保存作者程序原样副本及复跑结果、独立脚本、独立结果和运行记录。副本利用脚本按自身目录写结果的既有行为，不改源码，不覆盖原件。复跑结果与冻结 JSON 逐字节相同。

独立脚本不导入作者程序。它逐事件形成平台，在第三次观测成熟时更新方向并触发 RT；分区用组合长度递归生成，没有使用作者的位掩码实现。另按对象 X 逐一检查 J，并手算计数与一般证明。[独立脚本](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/independent-evidence-20261004T113209Z/independent-check.mjs)与[完整结果](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/independent-evidence-20261004T113209Z/independent-results.json)记录了确切输入、归属分区、见证、末端、确认和来源哈希。

## 2. 两项 EXT 和 J 的量词

以下是本审查采用的类型化读法。固定 B、其历史域 H_B、独立语义解释 S 与表示映射 φ。h_n 只取 H_B 的有限合法前缀，I_n 只含该前缀已成熟且已具同级身份的核心。N、外缘及 D 都来自同一个目录，不能在切界之后换成局部目录。若一般外缘会变动，应写成 N_n、D_{d,n}；P 中点值固定，所以本轮省略 n 不改变计算。

```text
EXT-rel(B) :⇔
  ∀ h_n∈Pref(H_B), ∀d∈{+,−}, ∀i,j∈I_n,
  N_n(i,j) ⇒ [Rel_B(h_n,d,i,j) ↔ D_{d,n}(i,j)].

EXT-own(B,S,φ) :⇔
  ∀ h_n∈Pref(H_B), ∀M, ∀e,q∈ℕ,
  Closed_B(M,e,q) ∧ q≤n ∧ UseUp_B(M,h_n)
  ⇒ [SameLevelTypeCompleted_S(φ(M),M)
      ∧ end_S(φ(M))=e
      ∧ ∀i∈I_n, Own_B(M,i)↔Own_S(φ(M),i)].

J(S) :⇔
  ∀ h_n∈Pref(H_B), ∀X∈Obj(S), ∀d∈{+,−}, ∀i,j,k∈I_n,
  Trend_d(X) ∧ Own_S(X,i) ∧ Own_S(X,j)
  ∧ N_n(i,j) ∧ N_n(j,k) ∧ D_{d,n}(i,j) ∧ D_{d,n}(j,k)
  ⇒ Own_S(X,k).
```

这里 h_n、X、d 不属于 I_n，原稿紧缩的量词排列应按各自类型展开。φ 对已发射对象的身份必须固定，不能针对某一对核心临时选择一个 X。不同历史上的对象应带历史身份；这些记号补全不增加原稿的数学要求。

EXT-rel 约束相邻核对，不规定非相邻对的 Rel。RT 的完整 K 目录在 P 下足以导出所有这些关系，所以本轮通过表示“关系证据仍可恢复”。它没有给出完成证明。一般非点外缘下，必须有真实 DD/GG 的保留或可恢复依据，不能仅靠 c_i。

EXT-own 是对已确认、拟上送对象的要求。对未发射活动尾，它没有强行要求完成。单凭这条公式也没有建立 S 的合法性、全部源事件忠实性或全域构成独占，这些仍需外部合同；Γ 的具体解释另外具有独占分区。原稿关于原义完成和 037 独占性的说明，只能在这些外部要求成立时使用。

J 的新内容是把两次同向外缘关系接到同一 X 的身份上。它没有把新核归属写进 EXT-own，也没有把 MAX 放进定义。全单核分格会使 J 前件全假，因而 J 不等于完整最大单调段分解算法。原稿已经说明“最小”是本争点的局部选择，并非全局最弱或唯一；本审查接受这一限度。

## 3. Γ 与两个有限模型

本轮 Γ 的量词范围是冻结的四核几何及有限关系解释，可记为 Γ₄。核值为 10、20、30、25，事件区间依次为 1…3、4…6、7…9、10…12，出生为 3、6、9、12。以 ℓ 为固定级别，P 假定这些核确有 ℓ 级身份，且 DD=GG=c。Γ₄ 还要求：

- 核索引 0…3 恰被连续非空格划分，每核唯一归属且全部覆盖。
- 每个多核格内部的相邻外缘方向严格相同，单核格保留单核身份。
- 全目录关系按同一 D 解释。
- 本模型声明的完成项精确拥有对应语义格及其事件，结束位置与最后成员一致。

这些是假定和有限关系条件，没有内含完整 F2、全部原课定理或 GoodΩ。

Join 解释为 `{0,1,2}|{3}`，左项 Up、end=9、confirm=12；Split 为 `{0,1}|{2,3}`，左项 Up、end=6、confirm=12。两者右格均未声明完成。独立检查给左项设置明确的类型、完成标志、结束位置和事件成员，φ 固定映射到左语义格，并检查 n=0…12 的 EXT-own 前件。n<12 没有声明完成，故该义务尚不触发；n=12 各项相符。这里完成标志是有限模型的解释值，不能成为原义完成证书。

Join 满足 J。唯一可能强制承接的三元组是 0、1、2，第三枚本就在左格；1、2、3 的两条边方向不同。Split 在 0、1、2 失败。两模型在 Γ₄ 下分别满足 J、¬J，所以 `Γ₄ ⊬ J` 且 `Γ₄ ⊬ ¬J`。这不是对完整历史域 H_B 的原义语义模型穷尽，也不是 `Source ⊬ J` 或 `Source ⊬ ¬J` 的证明。原稿明确保留了该边界，未发现借有限模型偷补 GoodΩ 的结论。

## 4. 一般拒绝证明

固定任一 RT 域内历史及真正发射的 M。按冻结定义，令 a 为该块起核，b 为其结束所用转折，拥有的核心恰为 a…b−1，且 b−a≥2。相邻平台值不同，从 a 到 b 之间没有内部转折，所以 δ_{a+1}=…=δ_b=d≠0。取 i=b−2、j=b−1、k=b，即有两个连续同向关系，i、j 归 M，而 k 不归 M。

在 P 下，两次标量同向等于两次同级外缘分离。确认 q=β_{b+1} 时，i、j、k 早已成熟；b+1 已出现，故被留右的 k 平台此时也已封口。若 M 拟以上升或下降的已完成成员上送，且 EXT-own 对某 S、φ 成立，则“同型”一项给出 Trend_d(φ(M))，成员对应给出 Own_S(φ(M),i) 和 Own_S(φ(M),j)。J 得到 Own_S(φ(M),k)，反向成员对应便得到 Own_RT(M,k)，矛盾。

所以准确结论是：

```text
∀合法 RT 历史 h_n，∀已确认并拟上送的发射 M，
  P 与同一目录成立
  ⇒ 不存在 S、φ，使 J(S) 与该 M 的 EXT-own 同时成立。
```

这不是仅由 P 就得到“所有 S 都违反 J”。Join 的 S 可以满足 J，只是无法精确表示 RT 的那个完成对象。若不把 RT 结果当作已完成同型成员上送，EXT-own 前件不成立；若 J 或 P 的原义资格未立，则目前只有条件拒绝。零发射或薄段后未完成的 R 也不在该否证的对象中。

事件成员不等式还能得到 end(M)≥t_k>s_k−1，而 RT 的 end=t_{b−1}<s_k。这个端点矛盾正确，但核心成员的双向对应已经足够产生矛盾，不依赖最终开放尾核有无封口。一般证明不依赖每个平台恰好三事件；三事件长度只用于本轮数值实例。

## 5. 样例、计数与尾部

独立结果如下，计数顺序为全部连续分区、内部单向分区、与已发射 C 精确相容的分区、再加 J 后的分区。

| 输入 | 计数 | 发射 | 原始事件尾 |
|---|---|---|---|
| 10³、20³、30³、25³ | 8 / 6 / 2 / 0 | own={0,1}，end=6，confirm=12 | e7…e12 |
| 30 事件十核例 | 512 / 216 / 2 / 0 | own={0,1}、{2,3,4}、{5,6,7} | e25…e30 |

四核共有三个可切缝，因此 2³=8。方向为 U、U、D；在 K2 转折两侧至少切一条，排除同时不切的 2 种，余 6。十核共有九个可切缝，方向为 U、U、D、D、D、U、U、U、D；三次转折对应三组不共边的两缝限制，每组四种选择中三种合法，所以为 `2⁹×(3/4)³=216`。

固定已发射 C 的核心成员后，四核只剩 `{0,1}|{2,3}` 与 `{0,1}|{2}|{3}`；十核只剩 `{0,1}|{2,3,4}|{5,6,7}|{8,9}` 与最后尾格拆为 `{8}|{9}`。两种区别都发生在未消费核心的语义试分格上，RT 的 R 始终是一份完整未消费尾。它们不是 RT 的两个完整输出解。四核的两种解释都在 0、1、2 违反 J；十核都在 0、1、2 及 3、4、5 及 6、7、8 违反 J，因此末项计数均为 0。

| 30 事件对象 | 最小见证 i、j、k | end | k 首事件 | k 封口事件 | confirm |
|---|---|---:|---:|---:|---:|
| M1 Up | K0、K1、K2 | 6 | 7 | 9 | 12 |
| M2 Down | K3、K4、K5 | 15 | 16 | 18 | 21 |
| M3 Up | K6、K7、K8 | 24 | 25 | 27 | 30 |

三个完整路径范围独立算得 `[10,20]`、`[15,30]`、`[5,20]`，严格交集 `[15,20]`。后两份范围包含入口运动，未误用自有核心首末值或共享状态端点包络。几何交集通过不补完完成成员的原义资格。

辅助枚举也独立复得 1533 条相邻不同的坐标词和 324 次 RT 发射，每次均有上述三核见证。1533 是 `Σ(n=1…9)3×2^(n−1)`；324 是各坐标词的发射次数之和。它们不是市场样本数，也不证明全 D_Q 的订单重放或数值精化。

## 6. 原文和教义依据

本审查直接读取固定来源树的正本路径，核行号、正文界、作者署名以及行首和行内编注。以下强度可采用：

| 来源 | 已核归属与可支持强度 | 本轮不能升级成的结论 |
|---|---|---|
| [018:38–44](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/018-第18课.md:38) | 作者署名在 12，正文界在 68。38、40、42、44 为正文；44 明说趋势延伸在于同级同向中枢不断产生。达到两核条件与实际结束须分开。46–48 为娇注，不采用。 | 不足以把本轮未获资格的平台标签直接认作原义核心，或把任意新 F1 的具体归属算法说成已证明。其延伸责任仍须正面回应，不能任意更名消失。 |
| [020:52、58](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/020-第20课.md:58) | 作者署名在 12，正文界在 94。58 本行无编注；明确用 DD/GG 外缘分离对应上涨、下跌及其延续。52 给 GG/DD 来源。 | 核心交集为点不能自动推出其全部外缘为点；不能拿标量排序替代一般外缘。56 的行内注不作为依据。 |
| [035:14–24](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/035-第35课.md:14) | 作者署名在 12，正文界在 38，所用行无编注。16 对具名逐笔最低构造明写依次同向则延续、首次不再同向才结束，18、22、24 给上递归与接续。 | 它直接否定把 RT 前移端点说成这套旧算法的结论；推广到所有不同 F1 仍需说明共同语义前提。结合律不单独证明 RT 的指定完成证书。 |
| [084:52–56](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/084-第84课.md:52) | 作者署名在 12，正文界在 68，所用行无编注。允许不同初始定义、要求规则内唯一，F2 中枢过程保持。a1 仍称最低中枢与走势类型。 | 初始自由不证明任一指定候选合格，也不证明 ¬J；不能用它抹掉 018 的延伸责任。 |
| [037:164–174](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/chanlun/text/blog/037-第37课.md:164) | 164–166 是作者署名和时间；170、172、174 混有读者问题，只采用 `==` 或 `=` 后回答。作者允许初始定义可变，并反对同一式中的重复归属。 | 支持构成独占，不禁止多个证明读取同一证据；没有为某次提前切界给完成证书。 |
| [qushi M-2:124–144](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/qushi.md:124) | 已裁正本以外缘分离判同向，与 020:58 一致。M-1 的相应级别计数限定同时适用。 | 两核同向与核心计数本身不认证全部候选对象的完成或原义归属。 |
| [level_recursion:283–297](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/definitions/level_recursion.md:283) 和 [ADR0011:55–70](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/docs/adr/0011-operation-decomposition-layer.md:55) | 纵向构造使用延伸，横向只读、不回灌；ADR 明标“按角色分”是推断。 | DEP 通过不能替代 EXT；这几段也没有单独给出对全部新 F1 的 J 形式化证明。 |

018 对 J 的语义动机很强，020/M-2 把同向关系钉到真实外缘；本审查没有证明 J 与完整 Source 独立。现在欠缺的是把候选核心及趋势实例取得原义身份的条件，与全历史上的身份承接连接成完整推导。原稿将这一点保留为待证是可接受的。接受有限 Split 不能成为否定 018 的依据。

## 7. 两项局部缺陷

**D1，开放成熟核的 t_i 记号不完整，非阻断。** [原稿第 29 行](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/extension-ownership.md:29)对每枚成熟核直接写末源事件 t_i；[冻结 RT 第 21 行](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-candidate.md:21)则区分观察末端 u_i 与封口后 t_i。e12 时 K3=25 已成熟，却仍是开放末平台；追加两次 25 后它仍是同一枚核，观察末端从 12 变为 14，最终封口点尚未存在。应写 `u_i(n)` 为当前末观测；已封口才写 `t_i`，或明写 `t_i(n)` 只是当前已知末事件。事件成员界限对开放核使用当前已知末端。拒绝证明用的极值 K2 已在 e9 封口，故该缺陷不影响一般结论。独立脚本也核了平台长度 4、5、6、5：end=9、被留右核起于 10 且封口于 15、confirm=18，同样矛盾。

**D2，机器字段名大于执行范围，非阻断。** [作者脚本 53–55、75 行](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/extension-check.mjs:53)将 `EXTown` 设为“每个完成核块等于某语义核块”。它不检查同型、完成解释、end 相等、源事件或全前缀的条件。`gamma` 函数同样主要检查有序穷尽与格内同向，其余 Γ 条件来自固定数据和模型解释。报告不能把这些布尔值说成通用的完整 Γ/EXT-own 验证器。对本轮两个实际模型，我已另行给出固定 φ、类型、有限完成解释、end、事件成员和 q≤n 检查，并用错误 end、错误类型作为拒绝负控制，均通过。补核说明实际例子成立，不把原脚本追称为已执行这些检查，也不把解释中的完成布尔值升级成原义证明。

## 8. 验证记录与未证事项

实际使用 Node `v22.23.1`。作者副本运行 exit=0；与冻结结果 `cmp` 为 exit=0；独立脚本运行 exit=0，11 份来源 SHA 全部一致。独立脚本 SHA 为 `0e7b4e7fe9f6fc9cdbad11daa57cbdf4b8d3cd211d7a1efbf81d1087b5540ad4`。完整命令、结果哈希见[运行记录](/Users/silencehan/Documents/Codex/research-evidence/issue1467/extension-contract-v1/independent-evidence-20261004T113209Z/run-record.md)。

来源树已有两项跟踪文件修改 `ResearchProgress.md`、`Stage40CompletionGate.md`，以及研究工具、技能、`.codex/` 和工作流文档等未跟踪内容。它不是干净 HEAD 快照。本轮所用来源文件均逐个核哈希，不依赖“HEAD 相同即全部文件相同”的假设；没有改动这些既有内容。

本次未跑 Lean、生产 F2、订单重放或全仓测试，未改 formal，未触发 fixture 漂移检查。尚未证明的内容包括 P 的原义核心身份、一般新 F1 上 J 的原义必然性、完整 Ω、GoodΩ(RT)、父走势完成及递归闭合。本审查没有扩大到其他候选或完整缠论体系。

可沿用的结论是本报告开头的条件不可同满足，以及有限 Γ₄ 独立性和精确计数。两项局部订正应与作者稿一并引用。

## 附：实际读取来源的 SHA-256

下表路径均相对于只读来源树 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。全部与受审 JSON 中的来源指纹相符。

| 路径 | SHA-256 |
|---|---|
| `docs/chanlun/text/blog/018-第18课.md` | `4a661fc843d0d9fcfdf32861b7dc903ee09d824d0075dd0ea7b2c19542bb2f92` |
| `docs/chanlun/text/blog/020-第20课.md` | `25f8f0bbe9ffbab48a27b552a081ff3925b6c01bfe1cae30c2616fe9b99b49ef` |
| `docs/chanlun/text/blog/035-第35课.md` | `0bb81a62b58ecf8900f6c40227194c1fbb139975bae5aa34d7a881363067b023` |
| `docs/chanlun/text/blog/037-第37课.md` | `f360d985b3f3b1fa567ca13e15f2ffaf9eae39dbb418519f75d1f6175ac70753` |
| `docs/chanlun/text/blog/084-第84课.md` | `36978e2c0ee0d49fc316611af94a228f7e5d6bfa0e0a05bcd3ad66ad41879c26` |
| `.chanlun/definitions/qushi.md` | `ce48f8cda6a3f02afd9fddbc0f93cb43431524b66472e579a927af5ce4bc3c46` |
| `.chanlun/definitions/level_recursion.md` | `06ac7aee96b089536766ac172b52ff0f8d702f2e4ec01ca2b47b1f3a590e1637` |
| `docs/adr/0011-operation-decomposition-layer.md` | `30098873c67878af850e8ff9502fdf69e332343e3576e551bab6768d6eb59860` |
| `.chanlun/review-results/issue1467-f1-proof/Stage44BaseCompletionAndB.md` | `df996cb667b7de83232bc93dd0471236b21f174be80d3d6493a5c0025f85c831` |
| `.chanlun/review-results/issue1467-f1-proof/stage44/ScopeCorrections-v2.md` | `6d6efc677c2b0aad9a759f2ea5c35c2a5a626897568fc0c6a276668ff6868aa6` |
| `.chanlun/review-results/issue1467-f1-proof/stage44/turn-boundary-candidate.md` | `fe8ac47c1f34991ed1767a6cf2c323786653bbe0213e21e4d0394e9c3252ff02` |
