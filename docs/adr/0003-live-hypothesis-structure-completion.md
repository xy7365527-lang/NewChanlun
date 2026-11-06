# 活假设的结构完成判据 = 通道切换；活假设账本与区间套证书是两层

活假设（061:26「先假设是进入背驰段」）需要一个「走完了」的判据才能收束——缺它则活窗右端永远追当下、记录只增不减（OKLO 10 万根实测：213 个假设滚出 563.7 万条修订，≈56 条/bar），且「已确认」态永不可达。我们裁定：**结构完成 = provider 由行进中通道切换到完成事件通道的那一刻**——不另造判据，因为 provider 只在段已走完时才产出完成事件，这个切换本身就是信号；契约原文（「pan = seg_c 进入已完成段集」）本就是此义，此前只是无人填。同时裁定：**活假设账本与区间套证书是两层不同的东西，不是强弱之别**——账本把区间套基例那一格的力度谓词沿时间轴展开成假设生命史，证书则是每个当下各自独立重算的一次性判定。

> #1599（用户 2026-10-26 批准）：字段进入所属具名版本，一次运行只读一个已签版本，改值即新版本并重推导；不得因失败换值放行。

`completion_anchor={channel_switch,center_break}`，默认 channel_switch。center_break 只有接通中枢生命周期与活假设账本的桥后才可启用，仍是原裁定允许的第二层清账事实；不能冒充与通道切换相同的完成时点，也不作其失败兜底。

## Considered Options

- **锚在中枢破坏（三类点）**：教义最正——092:16「中枢震荡，最终一定以某级别的第三类买卖点结束」＋049:62 明分「中枢震荡中的类盘整背驰段」与「中枢完成后的背驰段」在三类点前后＋018:64 定理三给出破坏判据，代码侧亦已实装（中枢生命周期机的 Doctrinal 死亡形态即三类点破坏）。**可选但尚未接桥（#1599）**：该机与活假设账本双向零引用，需新写桥；且原裁定时该机及其整批消费栈只存在于 `main`，不在当时工作分支（历史读数：两树 76 ahead / 117 behind、文件集差 22）——接它须先解树面合流。通道切换在语义上落在同一侧（段走完 ⊆ 震荡未必结束），代价却是零。**保留为后续可加的第二层**：若「从未可证的死假设需在震荡结束时统一清账」成为实需，此锚是现成的。
- **锚在段级完成（当前离开段的段终点）**：模块内现成粒度，零桥。**被否**：它与通道切换实为同一时点的两种说法（provider 正是靠「段已走完」筛选才产出完成事件），但直接读段终点是绕过 provider 自行判定，与契约「判据属调用方，本模块不越权自判」的分工相悖；走通道切换则判据仍由 provider 持有。
- **不设完成判据，靠区间套递归定位**（编排者初始质疑：「我有区间套背驰的」）：**查证后被否，但它纠正了问题的形状**。061:26 同一句话里前半是活假设、后半即区间套定位法，二者教义同源；然而代码层查实两者**不是同一件事**：区间套证书 = 单级力度 ∧ 终端买卖点背书 ∧ 跨级包含链，账本「已确认」= 单级力度 ∧ 结构完成 ∧ 复核，账本零跨级、零终端背书（键内只有单个级别数字，全模块无跨级结构）；更根本的是**维度不同**——证书是每个当下独立重建重算的一次性判定，系统中无任何位置记录「上一当下为真、这一当下翻假」，而账本正是这条时间轴。两者唯一重合面是基例的力度原语（盘整域逐字同一函数）。故「账本的已确认 = 区间套背驰确认了」是**过载表述**：账本的已确认里不含任何区间套。此比较此前从未有人做过（tracker 全量搜索零命中）。
- **新增第四态承载「从未可证」**：被否。改三态结构会连带重写待证的状态机不变量，而原因码机制现成。

## Consequences

- **完成即停延展**。右端一经冻结即无新内容可记，继续追加修订是缺陷而非行为。这是记录洪水的直接止血点。
- **「从未可证」归 Invalidated，配新原因码**（现有两码为「被反超」「身份消失」）。教义上这两类必须分开：061:26「一旦力度大于前者，那么就可以断定背驰段不成立」是**曾构成、后被否证**；061:28「因为背驰如果没有创新高，是不存在的」是**根本未构成**。实测该批占 115/213。沿用 Invalidated 使谱系学保留（终态禁删）与终态吸收性不变，无需重定义三态不变量。
- **两通道窗口不同，力度值可变**。完成事件通道的比较窗是定位到的完成段，活窗通道是「起点到当下」；切换那一刻同一身份的力度判定可能翻转。规格须显式处理该跃变，不得默认连续。
- **原文存在未调和的张力，本裁定站其一侧并登记在案**。024:24「C 段的走势类型完成时对应的 MACD 柱子面积……」与 024:28「看 MACD 柱子的面积不需要全出来……实际操作中根本不用回跌后才发现背驰」并列于同一课，缠师本人未调和。裁定 #64 §3.2 引前者立「完成时复核」，本 ADR 沿之；后者所述「不必等完成即可判」在系统中由**区间套证书**承担（它本就每个当下重算、不等完成）。故两句在系统中各有承载者，非择一弃一。
- **准入口径不受影响**。本裁定只填一个此前恒假的信号位，不改力度判定的计算，不改判定时点的写入——不触碰 ADR-0002 与裁定 #64 §1 明列的不授权面。

## 附带订正（本轮考证浮出，与本决策同批）

- **「中枢死亡」一词在两级原文中均无出处**：一级博文 018:64 作「**破坏**」，二级编纂版作「**终结**」。词汇表沿用的「死亡」是第三种说法，已订正。
- **编纂版擅自收窄了盘整背驰的比较对象**：其「当下笔或线段比前一笔或线段」的措辞在一级博文中查无出处；博文口径为「只要是围绕一中枢的两段走势都可以比较力度」（061:26）、「围绕中枢震荡的前后两段」（029:402）。按三级权威链以博文为准。

## #1599 批复与受影响清单

[用户批复](https://github.com/xy7365527-lang/NewChanlun/issues/1599#issuecomment-6403278294)；[受影响清单](../../.chanlun/review-results/issue1599-policy-20261026/affected-code.md)含固定基线、Rust/Python/Lean 接缝及下游未承接项。本次为政策文书，不声称实现或关票；新增自由度为 `[新缠论:选择]`，既裁不变量不变。

## #1719 U1 修订：出生当步具名完成政策版本（2026-11-06）

（append-only：上文裁定与 #1599 节不改，本节是 center_break 完成政策的现行增补。依据 [#1719 裁决评论 6679867936](https://github.com/xy7365527-lang/NewChanlun/issues/1719#issuecomment-6679867936)，主控按 Goal 偏好第 25 条常设授权批准；被批对象是 [#1719 评论 6553495463](https://github.com/xy7365527-lang/NewChanlun/issues/1719#issuecomment-6553495463) 的影子规则，非作者审查 PASS。旧 Q-H r2 与 P-PACKET 中超出该影子的推导不随本节获批。审查与裁决原件在仓外 `/Volumes/AgentStorage/issue1709-pstack-orchestrate-20261025/orchestrate/goal1323/` 的 `REVIEW-1719-U1-SUCCESSOR.md` 与 `RULING-1719-U1-SUCCESSOR.md`。本节是政策文书，不声称实现、接线或关票。）

**问题。** 破坏事实在 bar 94 已知并已留档；依赖该中枢的活假设在 bar 95 首次出生并登记。I4 组件（PR #1833，merge `60133aca03`）登记时不重评已留档事实，之后重送时门卫钟已大于 94，返回 `LatePendingRecovery`，不在出生这一步清账。这只说明当前组件没有在出生当步清账，不说明旧版本的恢复义务可以免除。

### 一、具名版本与绑定

| 版本 | 语义 |
|---|---|
| `#1719/center-break-completion/v1` | PR #1833 组件现行语义的登记名，冻结。新独立 center_break 运行不得选择 v1；历史 v1 的适用域见本节末段。出生当步不重评已留档事实；迟到判据保持 `entry.last_as_of > fact.known_at()`，不以对象曾推进或消费较晚替代该关系。 |
| `#1719/center-break-completion/v2` | v1 加本节第二条的出生当步规则，其余与 v1 相同。 |

`CompletionPolicy.version` 取上表完整字符串。映射到 ADR 0051 裁定一的 `policy_ref` 时，`policy_id` 取该字符串，`version` 取末段（`v1`/`v2`），`profile_id` 为 `null`，`content_hash` 按 ADR 0051 `content_hash_algo` 对政策记录 `{policy_id, version, anchor}` 三字段的 JCS 字节计算，`anchor` 取绑定里的完成锚。登记前在 main `9ae28877b3`、PR #1855 head `a7a7102587` 与 PR #1898 head `15fc698155` 上查过，仓内没有已登记的完成政策版本，只有测试用 `synthetic-*` 字符串。

只有绑定 v2 的对象适用出生当步规则。绑定 v1 的对象保持 v1 语义。生产版本解析只接受可解析的具名已签政策，未知或未签版本不得默认为 v1。旧组件的 opaque 版本字符串调用合同及 `synthetic-*` 测试夹具在原使用条件内保留，不据此把任意字符串登记为生产 v1 别名；已有对象按原绑定及原可解析定义恢复，不改写版本值。不在 v1 判定失败后改用 v2，不把在途对象的绑定从 v1 换成 v2；换版本只能走 ADR 0051 裁定一末段的显式迁移事件。组件已有的 `PolicyFrozen` 拒绝同对象换政策，保持不变。

**绑定时点与粒度。** 版本在对象首次登记时随绑定写入：`pan.rs:241-269` 在选中真实 B 的位置以当时的 `as_of` 作 `registered_at` 生成绑定，`book.rs:315-320` 只在 `registered_at == observed_at` 时接受首次登记。此后同一对象不换政策（`PolicyFrozen`）。按本文件第 5 行（#1599），一次运行只读一个已签完成政策版本，运行内所有绑定都取该版本；对象级 `PolicyFrozen` 是这条规则的防线，不授权一次运行里混用 v1 与 v2。改版本即新独立运行按新版本重推导；旧运行的对象、旧黄金与持久历史保持原版本，恢复与 seek 读原绑定版本。「新进程」不等于「新运行」：历史会话的恢复、回放或录放启动新进程时，仍属原运行，读原运行的已签版本。进程重启不是 ADR 0051 的显式迁移事件，不触发换版。

**两版在同一输入上的差别。** 输入为 K94 留档、O95 首次出生时，v1 运行的 seek(95) 是 Provisional，该事实之后重送判 `LatePendingRecovery` 并待恢复；v2 运行的 seek(95) 是 Invalidated。两版输出不同，不是同一判据的两种写法。不同政策版本的输出不要求逐位相等，也不拿跨版相等作验收；同一版本的原件、重放与恢复仍按原合同逐位比较。历史 v1 与新 v2 的测试可以并列比较差异，但任一版本都不给另一版本兜底。

**新独立运行只绑定 v2（#799 / ADR 0041；[#1719 补充裁定评论 6697247649](https://github.com/xy7365527-lang/NewChanlun/issues/1719#issuecomment-6697247649)，2026-11-06）。** 对同一份（事实，对象，步骤），v2 等于 v1 加一条 `Applied` 分支，属于同一判断的宽严两档。按 AGENTS.md 收敛通则（#799）与 ADR 0041 裁定二判别句，两档不能同时作为可选版本，事前单选也不行；ADR 0041 裁定四把「与现行判据构成同一判断的宽严两档」列为排除项。因此，以 center_break 为完成锚的新独立运行只能选择 v2。v1 不是 ADR 0041 的可调字段，也不是新独立运行的默认或备选生产档。v1 的适用域按已有身份认定，不按年代推断：凡已冻结绑定 v1 的对象，以及其运行身份记录的已签版本为 v1 的历史运行，原版本继续有效，原合同下的回放、seek 与恢复全部保留，按原绑定读取，不追改（I4 所属政策增量 SPEC 共同约束：「改值形成新版本，不追改在途对象」「新语义以新合同版本并列」，原件 `/Volumes/AgentStorage/i4-review43i/spec-6403536453.md:21,24`）。这些历史运行恢复或回放时即使启动新进程，也不受 v2 新准入条件限制，不缩小历史合法域；本节不为它们新增存量证明门。运行内不换版兜底；显式迁移仍按 ADR 0051 裁定一末段。main `9ae28877b3` 上没有生产绑定，v1 目前只有 PR #1833 组件测试使用。center_break 的生产启用条件仍是第 7 行的接桥，默认完成锚仍是 `channel_switch`。

### 二、v2 出生当步规则 `[新缠论:选择]`

前提全部成立时适用：

1. 事实是真实事件机输出的 `Broken`，在对象出生之前已持久留档，留档的先后关系有因果证据；
2. 对象在 bar t 首次出生并首次登记，匹配在同一逻辑步骤内完成；
3. 中枢身份、level、修订身份与冻结政策都匹配，政策 anchor 是 `center_break`；
4. 对象仍为 Provisional 且从未可证，来源事实在 t 时仍有效（未撤回）。

结果：对象以 `CenterBreak` 原因转 Invalidated，`application_at = invalidated_at = t`，`fact_known_at` 保留原 K。例子是 K=94、t=95：seek(94) 无对象，seek(95) 见 Invalidated，审计保留 K=94 及原身份。

记录必须分开保存四个时点：`fact_known_at`、`physical_bar`、`target_registered_at`、`application_at`，以及实际收到和留档的因果证据。它们不是互相复制的同一个数字。源 generation、source_coord 与 bar 是三个名字，source_coord 到 `physical_bar`、`known_at` 的映射由源 owner 证明，不能直接等同。

### 三、不在本规则内

- 对象在出生当步没有完成匹配，之后在 bar 96 或更晚才重评这份旧事实：仍按与 v1 相同的 `LatePendingRecovery` 与 ADR 0051 裁定三第 4 条处理。本规则不泛化为 `application_at = max(K, registered_at)` 或 `max(K, current_bar)`。
- 不改原 K，不回填旧 cut、AsKnown 与持久读面，不新增冻结晋升状态。
- 不放宽来源：Confirmed、已有终态、错误中心、缺政策或已撤回的来源都不参与匹配。
- 两种完成锚的关系不变：center_break 仍不冒充通道切换的完成时点，也不给严格判据失败兜底；默认 `channel_switch` 不变。观察结果不流入旁路、下钻、准入、成交或持仓。
- 不豁免旧版本的恢复义务。v1 下的迟到事实仍须交付具体恢复与 `RecomputedWithRevision`；恢复链尚未实现，不能据此判为不可能。

### 四、责任与写区

- **I4 合同/组件线**（#1719）：版本分派、v2 窄规则、`RecomputedWithRevision` 结果合同、组件测试。
- **S 结构域单写者**（#1419）：真实出生步骤、留档事实匹配、持久化与恢复。这项归属由裁决评论 6679867936 指定，不是 ADR 0027 已裁的内容：ADR 0027:25-28 只把纵向主干塔与唯一 Classification 划给结构判定域，全文不提活假设账本。裁决依据是 `book.rs:20-22`（本 book 是观察记录，不进交易输入）、ADR 0037:12（S 发布已有 `observations` 等版本内容出口）与 ADR 0027:32（任何结构事实或操作状态只有一个生产 owner）。仓内 `.chanlun/review-results/issue1719-recovery-20261026/REPORT.md:90,181` 曾写「B 原子持久」，按裁决不据此派给 B owner，也不新增第二个结构事实权威。
- **事实来源**（#1434，连同 #1419）：真实 `Broken`、稳定对象修订、source_coord 到 `physical_bar` 与 `known_at` 的映射。
- **launcher 消费**（#1523、#1769）：正式回放与 paper 同核证据。
- PR #1855 的在飞写区（含 `nest_lifecycle.rs`、`nest_lifecycle/window.rs`、ADR 0037）不另派竞争写者；写区交接由主控安排。两阶段提交等布局由技术 owner 证明结果一致，本节不规定事务数量。

### 五、受影响代码（行号固定到 main `9ae28877b3`；`nest_lifecycle/` 目录与 merge `60133aca03` 逐字相同）

路径前缀 `rust/src/theta_v0/classifier/nest_lifecycle/`。仓内除 `nest_lifecycle.rs:140-144`、`:149-153` 的再导出和 `#[cfg(test)]` 模块外，没有生产调用方调用 `provide_active_pan_live_windows_with_policy`、`feed_replay_bar_with_bridges`、`apply_center_breaks`、`register_center_bridge` 或构造 `CompletionPolicy`。Python 与 Lean 零命中：`git grep -n -P '\bcenter_break\b|CenterBreak|CompletionPolicy|completion_anchor|HypothesisBridge|apply_center_break|LatePendingRecovery' 9ae28877b3 -- '*.py' '*.lean'` 无输出。

- `center_break.rs:14-19` `CompletionPolicy`：版本字符串须能解析到第一条的登记名。
- `center_break.rs:21-28` `CenterBreakFact`：只存事件、`physical_bar`、`known_at`，不能证明「出生前已留档」。v2 需要可查的留档先后证据。
- `center_break.rs:118-123` `BreakApplicationKey`：去重键含 `policy_version`，同事实重送须返回原应用或 `AlreadyApplied`。
- `center_break.rs:125-134` `CenterBreakAudit`：已含 `fact`（带 `known_at`、`physical_bar`）、`bridge.registered_at` 与 `consumed_at`，缺 `application_at`。v1 的应用钟等于 `known_at`，不等于 `consumed_at`；v2 须单独存 `application_at`。
- `center_break.rs:144-155` `CenterBreakOutcome`：v1 分支保持。
- `pan.rs:241-269` `provide_active_pan_live_windows_with_policy`：绑定与政策的真实入口，`policy.clone()` 写入绑定，`registered_at = as_of`。版本在此随绑定固定。
- `book.rs:270-322` `register_center_bridge`：首次登记要求 `registered_at == observed_at`，登记不重评事实。v2 的出生当步匹配落在这一步或同一逻辑步骤内。
- `book.rs:324-442` `apply_center_break`：`:390-393` 迟到分支保持 v1 语义；`:400-411` 以 `fact.known_at()` 调 `admit` 与 `invalidate`，v2 出生当步须以 t 作应用钟并保留 K；`:412-419` 写审计，v2 须写入 `application_at`；`:421-438` 追加 attempts；`:439-440` 在 debug 构建调用不变量检查。
- `book.rs:765-775` `assert_invariants` 的 `CenterBreak` 分支：`:768-770` 要求存在 `audit.fact.known_at() == invalidated` 的审计，`:771-774` 要求 `invalidated <= last_as_of`。v1 保留这条时钟关系；v2 下 `invalidated_at = 95`、`fact_known_at = 94`，原断言会失败，须按版本分派，改为核 `application_at == invalidated` 并另核 `fact_known_at` 原值。
- `feed.rs:255-286` `feed_replay_bar_with_bridges`：先 `feed_replay_bar` 再登记绑定，出生当步匹配的调用顺序在此。
- `feed.rs:288-299` `apply_center_breaks`：调用方须处理待恢复与未就绪，不能只收集修订。

**测试交棒。** `i4_bridge_tests.rs:424-473` `i4_bridge_fact_before_binding_is_retained_for_matching_and_recovery` 在 as_of=110 留档、对象 observed_at=95、111 重送，不是「94 留档、95 出生同一步」的时序。其中 K94 分支保持 `LatePendingRecovery`，K100 分支保持 `Applied` 且 `invalidated_at=100`；两支原期望均不改。这个测试与其他 v1 测试的身份是历史兼容与组件负控，不作新生产档的验收。v2 正控另行新增：K=94 留档，O 在 95 首次出生并同一步应用。只查 `registered_at == observed_at` 或 `last_as_of == 95` 不够，必须同时证明事实此前已留档、应用发生在首次出生的逻辑步骤内。

### 六、验收（实施未完成，以下均待交付）

1. 正控：K94 留档、O95 出生同一步应用；seek(94) 无对象，seek(95) 为 Invalidated，`invalidated_at = application_at = 95`，审计保留 K94 与原身份。同输入下历史 v1 运行与 v1 组件负控保持原行为；两版输出不同是预期，不以跨版相等作验收。
2. 两处杀停（事实已持久而对象未出生；对象出生应用后）均经真实持久边界与新进程恢复，结果与不中断同输入逐字段一致，含对象、绑定、原件、修订、applications、审计与 attempts。若布局有中间提交态，恢复协议不得让它成为 95 的混合已提交读面。
3. 同事实重送返回原应用或 `AlreadyApplied`，不新增清账修订；身份相同内容冲突按原拒绝合同处理。两个全新进程在同输入、同政策/模型/seed/scheduler 下逐位相等（ADR 0051 裁定二、三；ADR 0038 裁定一、二）。
4. 负控：96 才首次重评旧事实仍为 `LatePendingRecovery`，旧 cut、AsKnown 与持久历史不回填。另交具体的 `RecomputedWithRevision` 输出，含输入、目标身份、`policy_refs`/`model_ref`、查询与 seek 证据。原件可查或 Late 计数存在不等于恢复完成。
5. 已应用的 Broken 被撤回时，旧终态与历史不变，追加撤回与重算；当前读面显示该来源已撤回。
6. 正式 S 生产输入经唯一增量路径，以精确身份和冻结版本到达真实观察消费者，再由 launcher 给出回放/paper 同核证据。L1 夹具不替代 L2/L3。默认 `channel_switch` 与不进交易准入的负控保持；资源、隔离、资金与原完整恢复验收保留。
