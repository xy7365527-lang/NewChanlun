# 盘整协议三项裁决点深度研究与决策菜单

- 日期：2026-07-14
- 对应审计：`chanlun/review-results/mutex-consolidation-gap-audit-20260714.md` 的 O-3 / O-4 / O-5
- 任务边界：只做权威链研究、裁决菜单和工程距离评估；不冒充证明闭合，不修改代码
- 认识论标记：**已定**＝原文与既有形式链足以排除其他语义；**设计**＝语义已有约束，但接口形状仍需裁定；**L2**＝收益、参数或经验阈值，不能由 L0 原文推出

## 0. 结论先行

三项 OPEN 并非都需要编排者凭口味决定。证据已经把合理空间压缩为下面这一组推荐组合 **R**：

1. **D-A / 协议状态**：第三类点只产生 `Pending(PreTrend)`，不得直接激活趋势；只有相邻同级别**已完成**中枢满足 `central-ggdd-v1` 才进入 `Trend(dir)`。趋势背驰或回到旧中枢只把旧趋势置为 `Pending(PostTrend)`；后继已结算为单中枢/级别扩张时才进入 `Consolidation`，后继 GG/DD 成立时进入相应 `Trend(dir)`。
2. **D-A / 同 bar**：订单裁决继续使用既有 P1..P10；协议事件另走固定优先级。输出采用积类型 `(OrderDecision, ProtocolEvent/ModeTransition)`，不把无订单的协议迁移伪装成 P11 或订单。协议侧按证据成熟度排序：`SettledCenterRelation > CompletedMoveEvidence > CompletedRetrace/Reentry > Type3Candidate > Hold`。背驰与回中枢并存且同归 `Pending(PostTrend)` 时合并 reason，不人为制造胜负。
3. **D-B / 中枢震荡动作**：区间 `[ZD,ZG]` 只提供语境和边界，不凭价格触边裸开平。经盘整背驰或下级买卖点确认后，操作一个有稳定身份的 `OscillationLot/ShortDiff` 子腿：开动作创建新子腿，平动作只关闭该匹配子腿，父腿不动；长父腿上方卖出/开反向短差、下方如数回补/平短差，空父腿镜像。复用 P9/P7 的动作语义，但候选必须有独立 `CenterOscillation` 原因，禁止伪造 B1/B2/B3。
4. **D-B / 数量**：语义目标为“同股数往返”，即 `target_units = matched_parent_or_lot_units`；实际成交量再经 `SizeΘ`/`KΘ` 风险、成本、交易所约束投影。固定减半、固定 1/10、按背驰力度线性比例都没有可升级为普遍规则的原文依据。
5. **D-C / PanDivCert**：只有通过现有 Nest/XZD 承接门的 `PanDivCert` 才进入生产候选。它不变成同级第一类买卖点：若方向与父腿相反且无匹配短差腿，走 P9 开 `ShortDiff`，形成经济减仓；若用于恢复父方向且已有匹配短差腿，走 P7 平该腿；无父腿、身份不明、门未通过或风险不可行则 P10 Record。若同 bar 已有真正 B1/B2/B3，沿用 P5/P6/P7/P8/P9 的现有优先级，PanDiv 只保留证据归因，不能重复下单。

若编排者接受整组推荐，**最少只需回复一个字：`R`**。真正不能由原文替编排者决定的 L0 自由度只剩一项：

- O-8 的输出形状：推荐“**双轨**”（订单 × 协议事件），备选是协议迁移只在 reducer 内消费；两者语义等价，但可观测性不同。

PanDiv 的 Nest/XZD 承接门不是新口味题：task #145 的既有裁决和验收已固定“PanDiv 不消失、不冒充同级 B1/S1，须由下级确认或 XZD 承接”。如果不接受整组 R，只需按 `输出=双轨/内消` 回复。数量也不再设口味题：默认采用“同股数目标 + 风控投影”，另行选择固定半仓或力度比例等于明确引入无权威依据的新策略。

---

## 1. 证据基线：哪些边界已经被权威链锁死

### 1.1 原文层短锚点

以下只摘技术要点，不转录长段：

| 主题 | 原文技术要点 | 锚点 |
|---|---|---|
| 盘整/趋势定义 | 完成走势仅一中枢为盘整；至少两个依次同向中枢才是趋势，方向属于趋势 | `docs/chanlun/text/blog/017-第17课.md:42-48` |
| 当下不可预判 | 当前趋势之后可以继续、反转或转为盘整；完成后才转换为其他类型 | `docs/chanlun/text/blog/017-第17课.md:30-36` |
| GG/DD 判据 | 后继同级别中枢外缘满足 `后DD>前GG`/`后GG<前DD` 才是上涨/下跌延续；外缘重叠为高级别中枢 | `docs/chanlun/text/blog/020-第20课.md:54-60` |
| 三类点非趋势充分条件 | 三类点之后可能是趋势，也可能进入更大级别盘整 | `docs/chanlun/text/blog/020-第20课.md:60-62` |
| 扩张/新生二分 | 三类买点来自中枢扩张或新生；扩张形成更大中枢，新生才形成趋势 | `docs/chanlun/text/blog/021-第21课.md:28-36` |
| 盘整背驰动作 | C 上破中枢但力度弱时先出来；不回中枢则在次级别一买回补并构成三买，回中枢则继续盘整 | `docs/chanlun/text/blog/024-第24课.md:32-46` |
| 盘背影响受级别约束 | 盘整背驰不保证大幅下跌；小级别逆大级别时只适合短差或不参与 | `docs/chanlun/text/blog/025-第25课.md:30-36` |
| 量随级别 | 任何买卖点都可操作，需控制的是量；级别主要决定操作量；卖出后恢复原数量而不净加仓 | `docs/chanlun/text/blog/026-第26课.md:34-36` |
| 盘背的分类地位 | 盘整背驰是中枢离开受阻并返回；多数二、三类点可由盘背提供确认，而第一类主要来自趋势背驰 | `docs/chanlun/text/blog/027-第27课.md:14-18` |
| 机动资金不是普遍减仓比例 | “例如 1/10”是给单只股票预留机动资金的举例；每次短差不得增加总股票数 | `docs/chanlun/text/blog/031-第31课.md:24-30` |
| 机械化卖出条件 | 同级别第三上段不创新高或发生盘整背驰则卖出；否则持有 | `docs/chanlun/text/blog/038-第38课.md:20-36` |
| 大级别约束小级别 | 小级别买卖点逆未衰竭的大级别走势时风险高，不能脱离上级别语境机械扩大动作 | `docs/chanlun/text/blog/041-第41课.md:20-24` |
| 操作级别 | 在选定操作级别等待相应买卖点；小级别部分操作属于预先安排的操作风格 | `docs/chanlun/text/blog/045-第45课.md:14-32` |
| 中枢三位置 | 当下相对最后一个操作级别中枢只有中、下、上三种位置 | `docs/chanlun/text/blog/049-第49课.md:16-28` |
| 中枢震荡契约 | 中枢上方减仓、下方增加；前提是震荡仍在，一旦三卖不能回补 | `docs/chanlun/text/blog/049-第49课.md:46-52` |
| 全平与短差的边界 | 完成后的向上走势背驰要全部卖出；中枢震荡本质上可全仓卖出并如数接回，不熟练者可不全仓 | `docs/chanlun/text/blog/049-第49课.md:54-64` |
| 两种背驰不得混同 | 三买前的中枢震荡盘背与三买后完成走势的背驰是不同对象 | `docs/chanlun/text/blog/049-第49课.md:60-64` |

由此可直接排除四个常见误读：

- “三类点一出现就是趋势”被第20、21课正面证伪。
- “趋势背驰后必然是盘整”不是原文定理；原文只锁定旧走势完成/逆转，后继仍可能是盘整或反向趋势。
- “中枢震荡只能减半”或“力度弱多少就线性减多少”没有原文规则。
- “盘整背驰就是同级第一类买卖点”与原文和已结算定义冲突。

### 1.2 形式链层已经采取的立场

PDF 以 `pdftotext` 抽取核对，引用按文档章节：

1. `docs/formal-chain/完整的策略.pdf` §7 固定 P1..P10：风险/TW 事件在前，P5/P6/P7 为 typed close，P8/P9 为 root/ShortDiff open，P10 Record；§9 明定一类反向 `CloseRoot`、三类反向 `ReduceCore`、短差反向确认 `CloseShortDiff`；§11 明定数量是 `SizeΘ(level, direction, certificate, depth, higher direction, role, stage, risk, cost, margin)` 的状态函数，而不是常数。
2. `docs/formal-chain/买卖点.pdf` §7-§9 把短差定义为保留父声部的反向子声部；父多时卖点开短差、买点平短差，父空镜像；同股数短差是严格目标。冲突序为风险、已有订单、同级反向、短差平仓、根开仓、短差开仓、次级顺向、保留。
3. `docs/formal-chain/缠论的全互斥定义策略.pdf` §2 把动作写为 `(side,size,exit rule,risk rule)`，数量由风险预算、结构失效距离、名义上限等投影；§17 把 `ShortDiffEntry/Exit` 绑定到父腿方向和子腿身份。
4. `docs/formal-chain/缠论的全互斥定义策略2.pdf` 把中枢语境 `κ`、背驰度量 `β` 纳入状态，允许背驰强度用于经验分层；但它同时明确“背驰强度不是定义买卖点的必要条件”，没有给出 `size ∝ β` 的生产定律。
5. `docs/formal-chain/递归完全分类买卖点.pdf` 继续使用 `ShortDiff` 反父方向子操作与同单位数约束；没有单列 PanDiv 动作量。
6. `docs/formal-chain/proofs-full-strategy-20260703.md:308-339` 只证明既定 typed exit 和 P1..P10 的接线；其互斥定理只保证“给定布尔谓词向量，最小成立 P 唯一”，不保证扩域市场状态中的每个必需动作都已有谓词覆盖。审计已在 `chanlun/review-results/mutex-consolidation-gap-audit-20260714.md:120-161` 明确揭示这一真空真边界。
7. `docs/formal-chain/一类买卖点.pdf` p.6-7 Q4 已裁标准第一类只锚趋势背驰，PanDiv 必须经下级确认 `N` 或 `XZD` 承接；它决定“是否承接”，没有决定承接成功后减哪腿、减多少，故 O-5 仍只在动作层 OPEN。

所以，形式链已经决定“用角色化声部和状态化数量”，却没有决定：协议状态事件怎样进入输出、PanDiv 在什么确认门后进入生产，以及中枢边界事件是否另立候选种类。这三处才是本备忘录要补的设计契约。

### 1.3 既有裁决层不可翻转的约束

- 盘整是单中枢、无方向；趋势至少两同向中枢且有方向；盘整背驰不保证走势终结：`.chanlun/genealogy/settled/009-consolidation-identity.md:38-77`。
- 构造层以中枢为核心，分类/操作层才实体化趋势、盘整及各自背驰：`.chanlun/genealogy/settled/010-construction-vs-classification.md:35-83`。
- 当前若不做显式类型扩展，就缺少趋势背驰/盘整背驰的类型安全分离：`.chanlun/genealogy/settled/099-trend-consolidation-implementation-gap.md:23-62`。
- 三类点的 first retrace 必须绑定 `(同级别中枢, 次级别离开段)`，且回试走势已完成；几何 SUCCESS 不证明 firstness/identity：`chanlun/escalate/c2-pending-rulings-material-20260714.md:69-80`。因此 D-B 的三类点边界动作仍受 O-2 阻塞。
- D3 已裁唯一方向真值为 `central-ggdd-v1`，单中枢和外缘重叠均无趋势方向：`chanlun/escalate/d3-direction-ruling-20260714.md:9-19`。
- 盘整背驰不定义同级第一类点；它可作为二、三类点的确认层证据；周线以上“类一类”是退化边界而不是标准 B1/S1：`.chanlun/definitions/maimai.md:295-318`。
- task #145 已裁并验收 PanDiv 承接路由：`PanDiv -> 下级 Conf 或 XZD`，两门皆闭诚实丢弃，不冒充同级 B1/S1：`.chanlun/review-results/xzd-c3-review-20260703.md:55-62`、`.chanlun/goals/events.jsonl:281,287`。本次 O-5 只能裁承接成功后的**生产动作**，不能把“裸证直接生产”重新包装成未决选项。

---

## 2. D-A（O-3）：盘整/趋势协议状态机 guard

### 2.1 激活选项矩阵

| 选项 | 精确语义 | 原文依据强度 | 形式链一致性 | 新证明义务代价 | 实装距离 | 主要风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DA-A：三类点立即 `Trend` | `Type3Confirmed -> Trend(side)` | **反证** | 低 | 中 | 中：Bsp 已有，状态机未有 | 扩张分支被误标趋势；方向真值被提前伪造 | **排除** |
| DA-B：后继 Completed 中枢 GG/DD 才激活 | `Up/DownContinuation -> Trend(dir)`；外缘重叠不激活 | **强** | **强** | 低-中 | **近**：`classify_relation` 已实现 | 只增加确认延迟，不增加语义错误 | **已定基线** |
| DA-C：三类点预激活为 Pending，可回退；GG/DD 才结算 Trend | `Type3 -> Pending(PreTrend)`；`Continuation -> Trend`；`LevelExpansion -> Consolidation` | **强**：完整容纳扩张/新生二分 | **强** | 中 | 中：需新增状态、原因、reducer | Pending 身份若不绑定 center/departure 会串案 | **推荐** |
| DA-D：三类点先按 Trend 交易，扩张后回滚 | 交易协议先切 Trend，事后撤销标签/持仓 | **无依据，属发明** | 低 | 高 | 远 | 回滚无法撤销成交；look-ahead 式语义漂移 | **排除** |

DA-B 与 DA-C 不是互斥的理论口径：DA-B 是最终 truth guard，DA-C 是在线状态机。严格实现应采用 DA-C，且其 settled truth 正是 DA-B。

若采纳各选项，新增义务与看守点：

- **DA-A**：必须证明“所有三类点均不发生级别扩张”；第20/21课已有反例，义务不可满足。看守 `type3_expansion_counterexample_rejected` 应固定为反例，不应写成成功性质。
- **DA-B**：证明相邻中枢均 Completed、`classify_relation` 三分 MECE、单中枢不产方向。看守：`only_completed_successor_can_activate_trend`、`central_ggdd_relation_total_exclusive`、`single_center_never_trend`。
- **DA-C**：在 DA-B 之外，定义 `PendingKey=(level, center_id, departure_id, reason)`、消费/回退规则和不可跨对象复用。看守：`type3_enters_pretrend_not_trend`、`pretrend_expansion_rolls_to_consolidation`、`pretrend_newborn_settles_direction`、`pending_identity_never_crosses_departure`。
- **DA-D**：除 DA-C 全部义务外，还要证明成交可逆与回滚损失边界；这不是 L0 语义证明，且不应进入严格路线。

### 2.2 退出选项矩阵

| 选项 | 精确语义 | 原文依据强度 | 形式链一致性 | 新证明义务代价 | 实装距离 | 主要风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DA-E1：趋势背驰立即 `Consolidation` | `TrendDiv -> Consolidation` | 中：只支持旧趋势完成，不支持后继类型 | 低-中 | 中 | 中 | 把反向趋势的前缀误标为盘整 settled truth | 不作分类真值；可作保守交易政策 |
| DA-E2：价格回旧中枢立即 `Consolidation` | `price ∈ old [ZD,ZG] -> Consolidation` | 弱 | 低 | 高 | 中 | “回到旧中枢”不等于新单中枢走势已完成；bar 噪声抖动 | **排除** |
| DA-E3：背驰/回中枢只结束旧趋势，后继保持 Pending；Completed 关系再分类 | `TrendDiv or CompletedReentry -> Pending(PostTrend)`；后继单枢/Expansion -> Consolidation；GG/DD -> Trend | **强** | **强** | 中 | 中-远 | 需要明确旧 move 与新 move 身份，但不伪造后继 | **推荐** |
| DA-E4：趋势与盘整协议并存，后续再择一 | 同一 `(level,move)` 同时 active | 无原文必要性 | 低：破坏互斥目标 | 高 | 远 | 仓位/动作重复消费，无法证明恰一 mode | **排除** |

关键二分：**走势完成**与**后继类型已知**不是同一命题。`.chanlun/definitions/zoushi.md:232-248` 已结算“背驰充分导致所属走势完成”，并未给出“后继必为盘整”。因此 DA-E3 不是新增口味，而是防止把未知后继冒充已知盘整的最小状态空间。

若采纳各选项，新增义务与看守点：

- **DA-E1**：若只作交易政策，必须把字段命名为 `OperationalBias::Conservative`，不得写入 `MoveKind::Consolidation`；看守 `conservative_exit_does_not_relabel_move_kind`。若坚持作真值，需证明反向趋势不可能，当前无此依据。
- **DA-E2**：需定义回中枢的完成性、去抖窗口、旧/新 center identity；仍无法从价格位置推出完成盘整。看守应以反例为主：`price_reentry_alone_not_completed_consolidation`。
- **DA-E3**：证明 `Pending(PostTrend)` 全定义、后继 settled relation 唯一消费、无后继时保持；看守：`trend_div_completes_old_move_not_next_kind`、`reentry_and_divergence_merge_pending_reasons`、`settled_successor_consumes_posttrend_once`。
- **DA-E4**：需重写“任意时刻恰一协议”目标和所有动作冲突证明，不符合本审计任务。

### 2.3 同 bar 并轨选项矩阵

| 选项 | 机制 | 原文依据强度 | 形式链一致性 | 新证明义务代价 | 实装距离 | 风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DA-Q1：把协议事件插成 P11/P12… | 订单与 mode 迁移共享单一最小 P | 无直接依据 | 低-中 | 高：扩 m、重排全部 P | 远 | 无订单事件被伪装为订单；P1..P10 冻结顺序漂移 | 不推荐 |
| DA-Q2：订单轨 P1..P10 + 协议轨固定优先级，输出积类型 | 两条轨各自互斥；同 bar 可同时有订单和 mode 事件 | 语义中性 | **强**：保留 §7 与无订单 TW 先例 | 中 | 中 | 需证明两轨组合全定义，但不改订单排序 | **推荐** |
| DA-Q3：订单轨先裁，协议事件仅 reducer 内消费 | 对外只见订单，状态内部更新 | 语义中性 | 强 | 中 | 中 | 可观测性差，审计重放难解释 mode 变化 | 可选，属 O-8 真自由度 |
| DA-Q4：协议事件延后一 bar | 用时间偏移避冲突 | **无依据，属发明** | 低 | 低-中 | 近 | 人为迟滞、跨 bar 身份错配 | 排除 |

推荐 DA-Q2 的协议侧固定序不是拍脑袋的“事件重要性”，而是**证据成熟度序**：

```text
SettledCenterRelation
  > CompletedMoveEvidence
  > CompletedRetrace/Reentry
  > Type3Candidate
  > NoProtocolEvent
```

- `SettledCenterRelation` 内部由 `central-ggdd-v1` 三分，本身已互斥。
- `TrendDivergence` 与 `CompletedReentry` 同 bar 时都只证明旧趋势结束/后继待定；合并 `reason_set`，不需要人为决定谁压谁。
- 若同 bar 既出现三类候选又出现已完成后继关系，settled relation 胜出；三类候选只保留 provenance，不再改变 mode。
- 订单侧完全不变：P1 > P2 > P3/P4 > P5/P6/P7 > P8/P9 > P10。协议 settled 与否不能越过 P1 风险门，也不能把一个 P10 记录事件变成订单。

DA-Q2 的看守：`order_and_protocol_product_total`、`protocol_settled_relation_beats_candidate`、`same_bar_pending_reasons_commute`、`protocol_event_does_not_reorder_p1_p10`、`zero_qty_protocol_event_remains_observable`。

其余并轨选项若采纳的义务：

- **DA-Q1**：必须给 P11+ 在 P1..P10 中的唯一插入位置，重证扩展互斥、生产 shadow-fold、无订单事件不会生成伪成交；看守 `extended_p_order_preserves_legacy_precedence`、`protocol_p_never_emits_order`。
- **DA-Q3**：必须证明 reducer 内消与外显事件重放得到相同 mode，快照仍可解释迁移来源；看守 `internal_protocol_replay_deterministic`、`mode_snapshot_retains_transition_provenance`。
- **DA-Q4**：必须证明一 bar 延迟不改变任何同 bar 动作、Pending identity 和成交；该证明一般不成立。反例看守：`delayed_protocol_transition_can_change_same_bar_action`。

### 2.4 D-A 的“已定”与“真自由度”

**原文/形式链已定，无需拍板：**

- 趋势最终激活必须等待后继 Completed 中枢的 GG/DD 判据。
- 三类点最多是预趋势候选；扩张分支必须可回退到盘整。
- 趋势背驰/回旧中枢不能单独证明后继盘整，必须有 post-trend Pending。
- 同一 `(level,move identity)` 不能同时拥有两个 active protocol mode。

**真自由度：**

- O-8 输出是 DA-Q2“积类型对外可见”还是 DA-Q3“reducer 内消”。原文没有软件 API；两者都能保持同一市场语义。推荐 Q2，因为 replay、审计和 `qty=0` 事件不丢失。

### 2.5 D-A 推荐裁决文本

```text
ProtocolMode ::= Consolidation
               | Trend(Direction)
               | Pending(PreTrend | PostTrend, PendingKey, ReasonSet)

Type3Confirmed(key)                         -> Pending(PreTrend,key,{Type3})
CompletedRelation(key,Up/DownContinuation) -> Trend(dir)
CompletedRelation(key,LevelExpansion)      -> Consolidation
TrendDivergence(move)                      -> Pending(PostTrend,move,{Divergence})
CompletedReentry(move)                     -> Pending(PostTrend,move,{Reentry})

同 bar：SettledRelation > CompletedMove > CompletedRetrace/Reentry > Type3 > Hold；
同目的 Pending 事件合并 reason；订单 P1..P10 独立互斥，输出为积类型。
```

---

## 3. D-B（O-4）：区间边界开平的 typed action 契约

### 3.1 先定对象：边界不是裸买卖点

第49课的 `[ZD,ZG]` 三位置是操作语境；真正动作仍由中枢震荡力度/盘整背驰或下级买卖点确认。第41课又要求小级别动作服从大级别语境。因此，`price > ZG` 或 `price < ZD` 单独触发订单没有权威依据。边界合同至少需要：

```text
CenterOscillationCandidate {
  center_id, level, parent_leg_id, oscillation_id,
  boundary_side, action_side,
  evidence: PanDivCert | LowerLevelBsp | ConfirmedType3,
  target_units, executable_units,
  provenance
}
```

### 3.2 候选/谓词映射选项矩阵

| 选项 | 动作契约 | 原文依据强度 | 形式链一致性 | 证明义务代价 | 实装距离 | 风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DB-A：裸边界新 P | `above/below/within` 直接产 `P_boundary_open/close` | 弱 | 低 | 高 | 远 | 价格触边噪声直接交易；扩展并重排 P | 排除 |
| DB-B：独立 CenterOscillation 候选，复用 P9/P7 动作 | 有确认才产候选；开新 ShortDiff 子腿走 P9，平匹配子腿走 P7 | **强** | **强** | 中-高 | 中-远 | 需扩 Γ 候选种类和身份，但不污染 BSP | **推荐** |
| DB-C：边界只作 BSP 上下文，不产生独立候选 | 只有已有 B1/B2/B3 才按原 P5-P9 动作 | 中 | 强 | 低 | 近 | 第24/49课独立的盘背短差会继续落 P10/统计层 | 可作第一阶段降级，不足以关闭 O-4/O-5 |
| DB-D：伪造 Bsp class 后复用 P | PanDiv/触边强塞 B1/B3 bits | **反证** | 低 | 表面低、实际高 | 近 | 破坏“PanDiv 非同级 B1”和 typed exit 语义 | 排除 |
| DB-E：新增 P11/P12，但保留独立候选 | 独立 `Open/CloseOscillation` 原子加入全序 | 中 | 中 | 高 | 远 | P1..P10 冻结顺序与所有证明/对拍需重做 | 仅当 P7/P9 无法扩候选原因时兜底 |

DB-B 的关键是“**复用动作，不复用定义**”：P7/P9 是关闭/开启 ShortDiff 的动作原子；`CenterOscillation` 是新的原因/证书类型。不能因为动作相同就把 PanDiv 写成三类点，也不能因为候选非 Bsp 就另造一套仓位解释器。

各选项的证明义务与看守：

- **DB-A**：需证明每次价格触边都有必需动作、无确认也不会误触；原文不给此定理。看守 `bare_boundary_is_not_action` 应固定否定该路线。
- **DB-B**：证明候选总定义、父腿存在、开平方向镜像、`oscillation_id` 一次开一次平、没有 naked child、同一证书只消费一次。看守：`center_oscillation_candidate_requires_evidence`、`open_creates_child_without_mutating_parent`、`close_matches_exact_oscillation_child`、`no_naked_shortdiff`、`center_candidate_maps_only_p7_or_p9_or_p10`。
- **DB-C**：只需证明边界字段不改变既有 BSP 语义，但无法证明 required-action coverage；应标注“保守不交易”，不能宣称 O-4 闭合。
- **DB-D**：需推翻 `.chanlun/definitions/maimai.md:295-318`，不可采纳。看守 `pan_div_has_zero_standard_bsp_bits` 已由 `signal.rs` 现状支持。
- **DB-E**：需扩展互斥定理到 `2^12`、重定优先级、生产 shadow-fold、typed ledger 和全部 golden；除非 DB-B 被实际类型约束证伪，不值得承担。

### 3.3 开平对象选项矩阵

| 选项 | 开对象 / 平对象 | 原文依据 | 形式链 | 实装距离 | 风险 | 裁决 |
|---|---|---:|---:|---:|---|---|
| DB-O1：直接减/恢复根腿 | 开＝删除已有 root/core；平＝重建 root | 中：现货叙述像卖出/回补 | 低-中 | 中 | root identity 与成本基丢失；容易与 P5/P6 混同 | 不作规范对象 |
| DB-O2：专用逻辑 `OscillationLot` | 开＝登记卖出的机动 lot；平＝只恢复同 lot | 强 | 中-强 | 中-远 | 需新增稳定 lot identity | 适合 inventory-only 后端 |
| DB-O3：反向 `ShortDiff` 子腿 | 开＝创建 parent 的反向子腿；平＝关闭该子腿；parent 保留 | 强（经济效果等价） | **最强** | 中：声部、父子、AncOK 已有 | hedge 与现货成交表达需适配 | **规范推荐** |
| DB-O4：任意已有腿先到先关 | 平对象不携身份 | 无依据 | 低 | 近 | 串腿、重复平、跨中枢污染 | 排除 |

规范语义选 DB-O3；执行后端可投影为：

- hedge-capable：真正创建/关闭反向 `ShortDiff`；
- inventory-only：用 DB-O2 记录“已卖未回补”的逻辑子腿，实际成交为减少/恢复父方向库存。

两者必须共享同一个 `oscillation_id`、父身份和数量守恒，不应让后端差异变成两套市场语义。

各对象选项若采纳的义务：

- **DB-O1**：证明删除/重建 root 后成本基、父子关系和同一仓位身份仍连续，且不会与 P5/P6 重复；看守 `root_trim_restore_preserves_identity`、`boundary_trim_not_double_counted_as_typed_exit`。
- **DB-O2**：证明逻辑 lot 与真实 inventory fill 一一对应、残量守恒、跨 bar 可重放；看守 `oscillation_lot_fill_bijection`、`inventory_restore_matches_exact_lot`。
- **DB-O3**：证明 child 方向恒反父、父 active、AncOK、开平身份唯一；看守 `shortdiff_is_opposite_live_child`、`shortdiff_close_preserves_parent`、`shortdiff_round_trip_identity_unique`。
- **DB-O4**：必须证明“任意已有腿”选择与身份化选择等价；多腿状态已有直接反例。看守 `multiple_legs_make_first_match_ambiguous` 应固定拒绝该路线。

### 3.4 仓位量纲选项矩阵

| 选项 | 数量规则 | 原文依据强度 | 形式链一致性 | 证明代价 | 实装距离 | 风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DB-S1：字面全量 | `q = parent live units` | 强：第49课“全部抛出/如数接回” | 中：未投影风险约束 | 中 | 中 | 大资金、流动性、保证金不可行 | 仅作目标量 |
| DB-S2：固定减半 | `q = 0.5 parent` | **无依据，属发明** | 低-中 | 低 | 近 | 把示例风格固化为理论 | 排除 |
| DB-S3：固定 1/10 | `q = 0.1 parent` | 弱：第31课只是机动资金举例 | 低-中 | 低 | 近 | 把资金预留比例误作每次动作比例 | 排除 |
| DB-S4：力度比例 | `q = f(C/A) parent` | **无生产公式，属发明/L2** | 中：β 可入状态但未定 f | 高 | 远 | 参数挖掘、量纲和单调性未证 | 不进 L0 裁决 |
| DB-S5：同股数目标 + `SizeΘ/KΘ` 可行投影 | `target=matched units`；`exec=Project_KΘ(SizeΘ,target)` | **强** | **最强** | 中 | **近-中**：SizeΘ/风险门已有，匹配 lot 缺 | 可能只部分成交，但语义与风险均诚实 | **推荐** |

DB-S5 不是折中发明，而是两个既有硬约束的组合：原文给出“同股数往返”目标，形式链给出所有动作都必须经过状态化仓位和风险可行集。若 `exec < target`，未完成差额必须留在同一 `oscillation_id` 的 pending quantity，不能静默宣称已如数回补。

DB-S5 看守：`oscillation_target_equals_matched_units`、`risk_projection_never_exceeds_target`、`partial_fill_preserves_residual_identity`、`round_trip_never_net_adds_parent_units`、`close_quantity_never_exceeds_open_quantity`。

其余数量选项若采纳的义务：

- **DB-S1**：证明所有状态下全量均满足流动性、毛头寸、保证金和交易所约束；看守 `literal_full_size_can_be_infeasible`。因此它只能是 target，不能越过 `KΘ`。
- **DB-S2**：必须提供普遍 `1/2` 的权威或独立 L2 prereg，并证明不同级别/角色都适用；看守 `fixed_half_has_no_l0_source`。
- **DB-S3**：必须证明第31课的“预留资金例如 1/10”等价于每张证书操作 1/10；文本语义不支持。看守 `reserve_ratio_is_not_action_ratio`。
- **DB-S4**：必须冻结力度 `β`、函数 `f`、量纲、单调性、上下界、成本后 OOS 收益；看守 `strength_sizing_preregistered_before_backtest`、`strength_size_respects_risk_projection`。

### 3.5 与 P1..P10 的建议映射

| 事件 | 动作 | P 映射 | 说明 |
|---|---|---|---|
| 长父腿，中枢上方，卖向确认，尚无匹配子腿 | `OpenShortDiff` / inventory trim | P9 | 新建反向子腿；经济净敞口下降，父腿身份不变 |
| 长父腿，中枢下方，买向确认，有匹配子腿 | `CloseShortDiff` / restore lot | P7 | 只关闭该 `oscillation_id`；如数恢复目标 |
| 空父腿，下方买向确认，尚无匹配子腿 | `OpenShortDiff`（多子腿） | P9 | 镜像 |
| 空父腿，上方卖向确认，有匹配子腿 | `CloseShortDiff` | P7 | 镜像 |
| 边界处出现真正一/二/三类 BSP | 走既有 typed close/open | P5/P6/P7/P8/P9 | 边界只补语境，不改 class |
| 无父、身份不明、slot 冲突、风险不可行 | `Record(reason)` | P10 或风险门 | 不得裸开逆势腿 |

这里不复用 P6：当前 P6 被形式链和代码明确绑定为“三类反向 -> ReduceCore”。把盘背卖出也塞进 P6 会丢失触发原因，且错误暗示 PanDiv 是三类点。

### 3.6 D-B 的“已定”与“真自由度”

**已定：**边界不能裸交易；中枢震荡操作不应销毁父腿语义；开新机动/短差子腿、平同一子腿；目标同股数、实际量受风险投影；不净加仓；不得伪造 BSP。

**不需编排者在 L0 拍板的后端差异：**hedge 后端用 ShortDiff，inventory 后端用逻辑 OscillationLot。两者是同一经济动作的执行投影。

**残余 L2 自由度：**不熟练/大资金是否主动把参与率设为 `<1`。原文承认可以不全仓，却没有给比例。严格默认应为 `participation_target=1`，再由 `KΘ` 裁剪；任何主动小于 1 的策略参数需单独 prereg 和收益/滑点验证，不进入本次 L0 裁决。

---

## 4. D-C（O-5）：PanDivCert 从统计承接升级为生产动作

### 4.1 先分清三种“卖出”

1. **中枢震荡 PanDiv**：三类点之前，父腿仍保留，做短差；对应 D-B 的 `OscillationLot/ShortDiff`。
2. **三类点反向**：已有 P6 `ReduceCore`，由标准 B3/S3 触发，不是 PanDiv 的别名。
3. **完成走势背驰**：第49课要求全出；这是趋势/完成走势的 typed root exit，不能由任意盘整背驰触发。

这一区分由 `docs/chanlun/text/blog/049-第49课.md:54-64` 和 `.chanlun/definitions/maimai.md:295-318` 共同锁定。

### 4.2 生产动作选项矩阵

| 选项 | PanDivCert 动作 | 原文依据强度 | 形式链一致性 | 证明义务代价 | 实装距离 | 风险 | 裁决 |
|---|---|---:|---:|---:|---:|---|---|
| DC-A：继续只 Record/统计 | 永不产生产候选 | 弱：违背“先出来/要走人”的可操作语境 | 中：fail-closed | 低 | 已是现状 | 语义漏单；O-5 不关闭 | 可作安全降级，不是终局 |
| DC-B：每张 PanDiv 全平 root | P5 CloseRoot / 全部清仓 | 仅适用于完成走势背驰，不适用于一般盘背 | 低 | 中 | 中 | 把短差信号升级成趋势一卖；丢失回补腿 | 排除 |
| DC-C：固定减半 core | P6 ReduceCore，`q=1/2` | **无依据，属发明** | 低 | 近 | 错把 PanDiv 当三类点；比例无来源 | 排除 |
| DC-D：按力度比例减 core | P6，`q=f(C/A)` | **无生产公式，属发明/L2** | 中 | 高 | 远 | 数据挖掘、非单调、成本后失效 | 不作 L0 语义，可另立 L2 实验 |
| DC-E：角色感知的短差开平 | 卖向 PanDiv 开反向子腿 P9；回补向 PanDiv 平匹配子腿 P7；不可行 P10 | **强** | **强** | 中-高 | 中-远 | 需稳定身份、生产 Γ 扩展和一次消费 | **推荐** |
| DC-F：只在 PanDiv 同时已升级成标准 B2/B3 时交易 | 使用标准 BSP 动作；否则 Record | 中 | 强 | 低-中 | 近 | 丢掉原文允许的独立中枢短差，但安全 | 可作上线第一阶段 |

DC-E 与 DC-F 可以组成分阶段上线，而不是语义冲突：先实现 DC-F 保证不伪造动作，再实现经承接门的 DC-E 完整关闭 O-5。

各选项的证明义务与看守：

- **DC-A**：证明“没有生产动作是刻意安全降级”，并暴露漏单计数；看守 `pan_div_recorded_not_silently_dropped`。不能宣称 `required_action_covered`。
- **DC-B**：需证明每张 PanDiv 都标志整个操作级别走势完成；第25/49课已有反例，义务不可满足。看守 `pan_div_does_not_close_root_by_default`。
- **DC-C**：需新增固定 1/2 的权威来源并证明 P6 语义扩张不碰撞；目前无来源。看守 `pan_div_never_forges_type3_reduce_core`。
- **DC-D**：需预注册 `f`、力度量、单调性、上下界、成本后收益和样本外验证；属于 L2，不得以 L0 文档裁掉。
- **DC-E**：需证明 cert 身份、承接门、父/子腿、开平配对、同 bar 去重、数量守恒、风险投影与标准 BSP 的优先级。看守见 4.5。
- **DC-F**：证明 PanDiv 只作为 `ConfirmationEvidence`，不改变 B2/B3 的结构定义；看守 `pan_div_confirmation_does_not_set_bsp_bits`。

### 4.3 “减哪腿”的裁决

推荐规则不是“随便减一条 core”，而是：

```text
PanDivCert(side=opposite(parent), center_id)
  + eligible parent active
  + no matching oscillation child
  -> OpenShortDiff(parent_id, oscillation_id, target_units)   // P9，经济减仓

PanDivCert(side=parent direction, center_id)
  + matching oscillation child active
  -> CloseShortDiff(oscillation_id, matched_open_units)       // P7，回补

otherwise -> Record(reason)                                  // P10
```

- **减的是净敞口，不删除父腿身份。** hedge 后端通过开反向子腿实现；inventory 后端通过卖出机动 lot 实现。
- **平的是匹配短差腿，不是任意 core。** 当前 `coverage.rs:1930-1943,2141-2145` 的 close 语义是从活动集完整移除被点名的腿；所以“减半某腿”不是现成 primitive。若需要半仓，只能先把可分割 lot 建模清楚，不能把 `ReduceCore` 名字当数量实现。
- 当前 sizing 已有 `LegTarget.units = base_units × depth_weight × direction_weight`，但这只是资本目标量，不是盘背同股数往返合同：`rust/src/theta_v0/strategy/coverage.rs:1373-1429`。需要显式的 matched units 约束。

### 4.4 数量与类一类关系

| 量方案 | 结论 |
|---|---|
| 全平所有 root/core | 只适用于真正的操作级别完成走势背驰；不适用于通用 PanDiv |
| 全量短差目标 | 推荐：全量指匹配的机动/父腿数量，不是把全账户清仓 |
| 固定减半 | 无依据，排除 |
| 力度比例 | 可作 L2 参数实验，不能当 L0 生产契约 |
| `SizeΘ/KΘ` 投影 | 所有方案都必须经过；它决定可执行量，不改 PanDiv 的类型身份 |

与“类一类”的关系：

- 标准同级 B1/S1 必须来自趋势背驰，PanDivCert 保持零 six-bit；代码也如此声明：`rust/src/theta_v0/classifier/signal.rs:525-551`。
- 周线以上“类一类”只允许独立标记 `ClassOneLikeBoundary`，不得路由进标准 P5/P8 的 B1 分支。
- PanDiv 可作为已满足结构条件的 B2/B3 的确认层证据；若同 bar 标准 Bsp 已成立，只执行标准 Bsp 所属 P，PanDiv 作为 provenance 记录，不再开第二腿。

### 4.5 优先级、门和建议看守

推荐把 PanDiv 承接为新的候选原因，仍落既有动作原子：

```text
P1 risk
> P2 CloseOverlay
> P3/P4 TW
> P5 CloseRoot / P6 ReduceCore / P7 CloseShortDiff
> P8 OpenRoot / P9 OpenShortDiff
> P10 Record
```

因此：

- 同 bar 真一/二类反向关闭 P5 胜过 PanDiv 的 P7/P9。
- 真三类反向 P6 胜过 PanDiv 开短差 P9；若命中的是已存在短差腿的关闭，则候选身份决定 P7，仍在 open 之前。
- P1-P4 成立时 PanDiv 候选只能推迟/记录，不得绕过风险和 TW。
- 同一 PanDiv 既作为 B2/B3 确认又作为短差候选时，候选去重键必须阻止双单。

建议生产门：复用当前统计承接的 Nest/XZD 二通道。代码现状已经有可复用判据：`rust/src/theta_v0/backtest/econ_positive.rs:429-474`；但订单生产投影显式清空 PanDiv，当前无消费者：`rust/src/theta_v0/backtest/runner.rs:711-747`。这使 DC-E 的信号检测距离近、订单接线距离中等，不需要重写 PanDiv 判据。

现行承接在证书首见 bar 评门，先写 `seen_pan`，两门皆闭便永久丢弃：`rust/src/theta_v0/backtest/econ_positive.rs:433-451`。代码把它声明为全信号通道共同的 `τin` 约定，而非 PanDiv 特例。生产升级应沿用该时点，或另案对所有通道统一重裁；不能只让 PanDiv 在后续 bar 重试而制造坐标系分叉。

建议看守：

- `pan_div_raw_cert_never_directly_orders_when_gate_closed`
- `pan_div_first_seen_terminal_matches_all_signal_channels`
- `pan_div_gate_pass_emits_one_production_candidate`
- `pan_div_candidate_keeps_standard_bsp_bits_zero`
- `pan_div_opens_shortdiff_only_with_live_parent`
- `pan_div_close_targets_exact_oscillation_id`
- `pan_div_round_trip_target_units_conserved`
- `pan_div_and_bsp_same_bar_emit_at_most_one_order_per_identity`
- `p1_p4_mask_pan_div_candidate`
- `pan_div_unexecutable_becomes_record_with_reason`
- `pan_div_class_one_like_never_enters_standard_b1_bucket`

### 4.6 D-C 的“已定”与“真自由度”

**已定：**不能默认全平 root、不能固定减半、不能按力度直接定量、不能冒充 B1/B3；应操作匹配短差/机动腿，数量沿用 D-B 同股数目标和风险投影；标准 BSP 优先，PanDiv 不重复下单。

**L0 无残余自由度：**生产候选必须来自“通过 Nest/XZD 承接后的 PanDivCert”。这已由 task #145 裁决并验收，不是本次可重开的口味轴。裸证直接交易只在获得新的权威翻转裁决后才可进入菜单；若未来 L2 证明承接门漏掉有价值样本，应先重开 #145 并版本化放宽，不能由 O-5 静默绕过。

---

## 5. 代码现状与实装距离总表

| 所需能力 | 当前真值/路径 | 已有部分 | 缺口 | 距离 |
|---|---|---|---|---|
| GG/DD 趋势判据 | `rust/src/theta_v0/classifier/center.rs:203-215` | `classify_relation` 已逐字实现 D3 | 无 ProtocolMode reducer | 近-中 |
| 趋势/盘整三值方向 | `rust/src/theta_v0/classifier/decompose.rs:41-71,124-140` | Trend `Some(dir)`，Consolidation `None`，尾块 Active | 无 Pre/Post Pending、无协议事件输出 | 中 |
| P1..P10 固定优先级 | `rust/src/theta_v0/strategy/mutex.rs:1-17,133-150` | oracle 与生产 P 映射已冻结 | 只覆盖 Bsp/TW/risk，不覆盖边界/PanDiv/protocol | 中 |
| typed exit | `rust/src/theta_v0/strategy/interp.rs:223-257` | P5/P6/P7 已按入场角色与 Bsp class 单源 | 无 PanDiv 原因类型；P6 不应被复用 | 中 |
| PanDiv 检测 | `rust/src/theta_v0/classifier/signal.rs:525-646` | 独立证书、零 Bsp bit、A/C/回中枢/力度均已有 | 身份键不足以直接支撑跨 bar lot 生命周期 | 近-中 |
| PanDiv nest 谓词 | `rust/src/theta_v0/classifier/nest.rs:380-390` | 明确只作 diagnostic，不进 `Cand^δ` | 生产 action candidate 不存在 | 中 |
| PanDiv 统计承接 | `rust/src/theta_v0/backtest/econ_positive.rs:429-474` | Nest/XZD gate、seen 去重、Mu provenance 已有 | 不进入 π 订单解释器 | 中 |
| PanDiv 生产路径 | `rust/src/theta_v0/backtest/runner.rs:711-747` | 新确认 Bsp 路径已有 | `pan_div` 被显式清空，注释“此处无消费者” | 中-远 |
| 子腿/父腿/数量 | `rust/src/theta_v0/strategy/coverage.rs:1370-1429` | `ElementId`、parent、ShortDiff、q_units、AncOK 已有 | `oscillation_id`、matched-units、partial residual 未有 | 中 |
| 真正关闭量 | `rust/src/theta_v0/strategy/coverage.rs:1930-1943,2141-2145` | 精确关闭被点名的完整 active leg | 没有“同一腿减半” primitive | 固定半仓并不近；独立子腿较近 |

实装距离的核心判断：**沿 ShortDiff 子腿做 DC-E 比把 P6 改造成可分数减仓更近，也更符合形式链。** PanDiv 检测和统计承接已在；真正缺的是候选类型、稳定 lot identity、生产解释器接线和 required-action coverage 证明。

---

## 6. 最小可行裁决集

### 6.1 推荐组合 R

| 裁决点 | 选择 |
|---|---|
| D-A 激活 | DA-C：三类点 PreTrend Pending；Completed GG/DD 才 Trend |
| D-A 退出 | DA-E3：背驰/回中枢 PostTrend Pending；后继 Completed relation 再分类 |
| D-A 并轨 | DA-Q2：订单与协议双轨积类型；证据成熟度优先 |
| D-B 候选 | DB-B：独立 CenterOscillation 原因，复用 P7/P9 动作 |
| D-B 对象 | DB-O3 规范语义；inventory 后端可投影 DB-O2 |
| D-B 数量 | DB-S5：同股数目标 + SizeΘ/KΘ 风控投影 |
| D-C 动作 | DC-E：角色感知的 ShortDiff 开平；无资格 P10 |
| D-C 生产门 | 承接后：Nest/XZD 任一通过 |
| D-C 与 BSP | 标准 Bsp 优先；PanDiv 只作 confirmation/provenance，不重复订单 |

### 6.2 编排者最少要答的字

- 全部接受推荐：`R`
- 只改真自由度：`输出=双轨/内消`

其余选项已经被原文反证、既有裁决锁死，或明确属于需另立 prereg 的 L2 新策略，不建议再让编排者承担伪自由度。

### 6.3 推荐裁决后的新增 L0 证明义务总清单

1. `ProtocolMode` 枚举全定义、任意 `(level,move_id,t)` 恰一 active mode。
2. Pending 身份绑定 `(level,center/departure/move)`，结算、回退、超时均不串案。
3. 协议事件固定优先级全序；同目的事件 reason 合并满足交换律/幂等律。
4. `(OrderDecision, ProtocolTransition)` 积类型全定义；协议事件不改 P1..P10 顺序。
5. `CenterOscillationCandidate` 必须有确认 evidence，裸位置不产动作。
6. ShortDiff/OscillationLot 父存在、方向相反、祖先闭合、开平一一配对。
7. 目标同股数，实际量不超过目标；partial fill 残量和身份守恒；往返不净加仓。
8. PanDiv 标准 BSP bits 恒零；作为 B2/B3 confirmation 时不替换结构定义。
9. PanDiv 通过承接门后最多产生一个生产候选；与 BSP 同 bar 不重复下单。
10. 每个 required action 至少有一个 P 动作原子覆盖；C0/P10 只能带显式不可执行原因，不得静默吞掉必需动作。

收益、回撤、最优参与率、力度分箱收益不属于上述 L0 证明；它们必须另做 L2 prereg 和样本外验证。

---

## 7. O-1..O-9 立项依赖 DAG 与排序

### 7.1 依赖图

```text
O-1 在线中枢四态 ───────────> O-3 协议 guard ──> O-8 π 输出协议事件 ──┐
                                                                       v
O-2 firstRetrace 身份 ───────> O-4 边界 typed action ──> O-5 PanDiv 动作 ──> O-6 扩域 z schema ──> O-7 动作语义完备
          │                         │                      │             ^
          └─────────────────────────┴──────────────────────┴─────────────┘

O-9 Active 前缀机器锚：独立并行；不挡 O-3..O-8，也不被 O-1 阻塞。
```

硬依赖解释：

- **O-1 挡 O-3**：没有 Pending 在线态，DA-C/DA-E3 无合法状态空间。
- **O-9 不挡其他项**：T3' 的数学结论和现有 `MoveStatus::Active` 已足以单独落机器锚；它不需要等待 O-1 的在线中枢发展四态。O-1 完成后只需补一条兼容性 lemma，不能虚构成硬依赖。
- **O-2 局部挡 O-4**：三类点边界动作的 first retrace 消费/重启需要 `(center,departure)` 身份；纯 PanDiv 短差支可先做，但 O-4 不能整体结项。
- **O-4 挡 O-5**：先定机动腿/ShortDiff 的开平对象和数量守恒，才能回答 PanDiv “减哪腿、减多少”。
- **O-3 挡 O-8**：不知道协议事件种类，就不能裁 `πΘ` 输出积类型或 reducer 内消。
- **O-3/O-4/O-5/O-8 挡 O-6**：schema 必须在 mode、PendingKey、候选原因、lot identity、输出事件均定后冻结；提前扩 z 会反复迁移。
- **O-6 挡 O-7**：required-action coverage 的量化域就是扩域 z；域未定，完备定理只能再次真空真。
- **O-2/O-4/O-5 直接挡 O-7**：first retrace、边界动作和 PanDiv 动作任一未定，`required_action -> some P` 都无法全称量化。

### 7.2 建议波次

| 波次 | 立项 | 为什么 |
|---|---|---|
| W1 | **O-1、O-2、O-9 并行** | O-1/O-2 是两条根依赖；O-9 已有数学结论，可独立快速落机器锚 |
| W2 | **O-3、O-4 并行** | O-3 消费 O-1；O-4 消费 O-2 |
| W3 | **O-5 与 O-8 并行** | O-5 消费 O-4；O-8 消费 O-3；两者共同决定扩域输出/动作字段 |
| W4 | **O-6** | 汇总冻结 schema、版本投影与分类全函数 |
| W5 | **O-7** | 最后证明 required-action coverage、C0/P10 诚实兜底和生产对拍 |

优先级不是按代码量，而是按“谁会令下游定义返工”排序。最不应先做的是 O-7：在 O-3/O-4/O-5 未裁前扩 `2^m` 穷举，只会再次证明一个不含必需谓词的布尔选择器全互斥。

### 7.3 每个 O 的建议完成口径

| O | 完成口径 |
|---|---|
| O-1 | `PendingReason`、身份、进入/退出/回退/消费全部定义；settled 三分与 online 四态分离 |
| O-2 | `(center_id, departure_id)` firstRetrace 首次、失败、重启、消费、Completed 门版本化 |
| O-3 | 采用 R 的 Pre/Post Pending、GG/DD 激活、后继关系退出、协议侧优先级 |
| O-4 | `CenterOscillationCandidate`、ShortDiff/OscillationLot、matched units、P7/P9/P10 映射 |
| O-5 | PanDiv 承接门、production candidate、一次消费、BSP 去重、角色化开平 |
| O-6 | 扩域 z 含 mode、PendingKey、candidate cause、oscillation identity、事件输出版本；旧投影显式版本化 |
| O-7 | `required_action_covered_by_some_predicate`、`c0_only_when_no_required_action`、生产 shadow 对拍 |
| O-8 | 明裁双轨积类型或 reducer 内消；`qty=0` 协议事件可审计 |
| O-9 | `trend_first_confirmation_has_single_center_active_prefix`，且反证 `ActivePrefix != CompletedConsolidation` |

---

## 8. 最终裁决菜单（供直接复制）

### 推荐直接裁决

```text
采纳 R：
1) 三类点只进 PreTrend Pending；趋势仅由相邻 Completed 中枢 central-ggdd-v1 激活。
2) 趋势背驰/回中枢只进 PostTrend Pending；后继 Completed relation 再判盘整/趋势。
3) 订单 P1..P10 与协议事件双轨输出；协议按 settled evidence 优先，pending reasons 合并。
4) 中枢震荡动作使用独立 CenterOscillation candidate；开新 ShortDiff/OscillationLot，平匹配子腿，父腿不动。
5) 数量为同股数目标，经 SizeΘ/KΘ 投影；禁固定半仓、禁力度比例冒充 L0。
6) PanDiv 经 Nest/XZD 承接后进生产；P9 开短差、P7 平短差、不可执行 P10；标准 BSP 优先且不重复下单。
```

### 仅保留的替代菜单

```text
输出：A 双轨积类型（推荐） / B reducer 内消
```

除这个软件输出轴外，其余被排除项若要复活，必须先提交新的一级原文锚、翻转既有裁决或独立 L2 prereg；不能以“编排者口味”替代缺失的依据。
