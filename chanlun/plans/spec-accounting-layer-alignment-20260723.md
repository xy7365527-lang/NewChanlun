# Spec：出场通道结构收敛——会计层对准已有蓝图（地图 #174）

- 日期：2026-07-23
- 状态：已定稿（2026-07-23）：seam 两条与三项开放决定（解释器接线档位、归属键粒度、parent_kappa 处置）均经用户裁定；发布为 [issue](https://github.com/xy7365527-lang/NewChanlun/issues/193)（`ready-for-agent`），随后 /to-tickets 出票
- 母地图：#174「出场通道结构收敛（ADR 0001 教义-生产接线图）」
- 规范性输入（裁定全文，不得违背）：#177 裁定一/二/三/五、#178 裁定 1–5、#179 裁决、#185 审计修复项、#174 Notes 两道防线
- 词汇口径：`CONTEXT.md`；教义口径：`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md`（注意 S4 与优先级条目编号错位一格属文本债，随 #184 处理）
- 勘察依据：`p1p8-wiring-cost-memo-20260723.md`（#175）、`splitlegledger-wiring-cost-memo-20260723.md`（#176）、`exit-account-identity-audit-20260723.md`（#185），均在 `.chanlun/review-results/`
- 查重依据：`.chanlun/implementation-index-20260723.md`（#192）

## Problem Statement

生产 π loop（`run_theta_v0_pi`）的会计层已偏离蓝图：#152 终验A 判「结构偏离」，#186/#187/#188 研究簇查明病根不是「仲裁存在」，而是**归属缺失**——

- 主账是净额单池：全部 fill 落在一个有符号标量 units 上，分账本（P_sep，腿 `e_v^±` 分记、禁止只记净额）在生产不存在；蓝图确有的会计层（P_sep 分账本 + T₃₃–T₄₆ 共 14 定理，Lean 全证无 sorry）没有被接进来。
- 执行账无 (account, level) 键：`voice_qty` 按代际（depth）不按级别，「一类点后该级别本仓=0」不可断言，错账零成本。
- 卖出归属不唯一且存在确定性错账：silent_drops 把 FollowParent 核心级联清仓错标为 CloseShortDiff（污染 typed trade 与 μ 分桶）；二类反向被实现为无条件 CloseRoot（缺「仅残余才纠错」谓词），二类点开空（C 账户）通道缺失。
- 建而未接复发：SplitLegLedger（分账本教义忠实实现）与 P1–P8 通道解释器（channel.rs）全部消费者都在测试内，生产零调用；生产跑的是散装裁决链（interp fold），#152 判「行为近似、无结构对应」。
- 全局次序表的本体定位被误读：它是**消歧确定性装置**（Σ1[C_j]=1 有 Lean 证明），不是凌驾各账的经济仲裁。

用户面对的后果：教义（ADR 0001）与生产行为之间无法建立可断言的对应，「结构对应是硬要求」（#179 裁决）无法满足，终验A 的「结构偏离」判定无法翻转；每卖出一次都可能记错账而无任何断言能发现。

## Solution

按用户裁定（2026-07-23，#177 裁定一）把目的地重描为：**生产 π loop 恢复蓝图会计结构——声部为最小账单位、卖出按归属唯一记账、分账禁止净额化、级别为分账维度、全局次序表仅作消歧装置；现金单一根池维持已结算裁定（2026-06-20）。不重设计、不发明新教义。**

解决方案是把「建了不接」的三个既有实现接进生产，并修复 #185 查出的四处归属现病，全部走「接线/采用/修复」而非新建：

1. **SplitLegLedger 档A 共存接线**（#178 裁定 1）：OscillationBook 保留路由与 lot 生命周期，SplitLegLedger 作为其下数量不变量层（每 parent+lot 一实例），apply 双写 + 一致性断言；净额出口不动、行为零变化。档A 即蓝图级严格（T₃₃「同源一笔、不可能单边记账」正是「合并下单、账分两边」的形状），生产的病根由本档治愈。
2. **执行账归属键落地**（#185 审计推荐路线）：引入 `AccountIdentity::{Core{level}, ShortDiff, Short}` 与 `AccountOrder{account, reason}`——账户身份与退出理由正交，与蓝图声部/carrier 归属一致（Short 对应 CONTEXT.md 的反向根：无父反向声部）。修复 FollowParent 错标、二类反向无条件 CloseRoot、(account, level) 键缺席、二类开空通道缺失四个现病。
3. **解释器分阶段接线**（#177 裁定五的 spec 阶段处置，建议见 Implementation Decisions）：channel.rs P1–P8 解释器的新身份 = 蓝图消歧装置 + typed exit 单源；按备忘最小风险序「shadow 双链比对 → trace/裁决层统一 → 逐通道替换（仅 P2/P3）」接线，阶段 A 零行为变更。
4. **ADR 0001 落稿/修订**（随 #184 执行）：写入 #177 裁定二/三、#178 让步条款与档B 标记，修正 S4/优先级编号错位与 #184 口径错位。
5. **既有票只接线引用**：#183（T4 子树清仓）、#181（SellDecision 迁移）、#184（ADR 修订）不重建、不重新切。

完成后重跑终验A 条目 1/3/4，确认「结构偏离」判定翻为对齐或让步合法化。

## User Stories

1. 作为实施工程师，我要 SplitLegLedger 作为 OscillationBook 之下的数量不变量层接入生产（每 parent+lot 对映射一实例），以便分账本 P_sep 结构在生产账上真实存在而非仅测试存在。
2. 作为实施工程师，我要 apply 双写 + 一致性断言且净额出口不动，以便在行为零变化、翻动面最小的前提下获得结构对应（档A 即蓝图级严格）。
3. 作为验收者，我要双写一致性断言在生产路径（run_theta_v0_pi）被显式见证，以便接线不是测试级自我安慰（#191 实证 B 类防线）。
4. 作为教义维护者，我要「父仓不动 fill 层证实 = v1 显式让步」写入 ADR 并含自指记录（ADR 行34 批判净额视角而 v1 执行层仍净额单出口，fill 级「父仓不动」不可证实，挂账），以便让步是显式条款而非默认现状（no-patch mentality）。
5. 作为教义维护者，我要档B（逐腿下单出口）在 ADR 标记「不是最严格的实现，以后再搞」，以便最严格口径有案可查、本轮拦下理由（funding/borrow 毛暴露口径蓝图无明文，换出口=发明新钱规矩）不丢失。
6. 作为实施工程师，我要执行账引入 `AccountIdentity::{Core{level}, ShortDiff, Short}`，以便每个卖出动作恰有一个账户身份（卖出按归属唯一记账）。
7. 作为实施工程师，我要 `AccountOrder{account, reason}` 把账户与退出理由拆成两个正交字段，以便不再用 ExitType 同时表达两者、同类错桶不再犯。
8. 作为验收者，我要「一类点后 Core(level)=0」成为对实际成交仓位可强断言的账本事实，以便错账零成本状态结束。
9. 作为实施工程师，我要修复 silent_drops 把 FollowParent 核心级联清仓错标 CloseShortDiff，以便核心腿退出不再污染短差桶与 μ 分桶。
10. 作为实施工程师，我要二类反向从无条件 CloseRoot 改为「仅残余才纠错」（CoreResidualCorrection 仅在核心残余实测非零时触发），以便一类点全平的本仓不被二类点重复记卖出。
11. 作为实施工程师，我要二类点开空通道（OpenShort{level, certificate}）落地，以便二类点合法卖出只剩短差、开空两身份（#177 裁定后的行为口径）。
12. 作为实施工程师，我要风险强平展开为多条单账户 AccountOrder，以便跨账户动作不丢身份（venue 侧净额投影只允许在最外层）。
13. 作为教义维护者，我要首开反向仓标准位 = 一类点（带标记、可复检）落入 ADR，以便多重赋格的多空双开有裁定依据，并 delta 修正 #184「二类为开空/加空确认点」口径。
14. 作为教义维护者，我要二类点 = 第二入场/加仓（加空）位落入 ADR，以便与首开=一类点的口径自洽。
15. 作为教义维护者，我要 B/C 撞车按 ambient 守卫读法（父声部 active⇒记短差账；无 active 父⇒记空仓账）落入 ADR，以便撞车有确定性消歧，且与 CONTEXT.md 反向根「有父即短差、无父即反向根」互斥完备一致。
16. 作为实施工程师，我要「一类卖点当场全平、允许当场反手（无新走势类型确认前置）」成为生产行为口径，以便时序与蓝图三源互证的结论一致（#189）。
17. 作为实施工程师，我要解释器按 shadow → trace/裁决层统一 → 逐通道（仅 P2/P3）分阶段接线，以便每步可逆、行为变更被隔离在已声明的阶段内。
18. 作为验收者，我要 shadow 阶段订单轨 bit-exact 不变，以便阶段 A 是零行为变更的纯见证、可安全先进生产。
19. 作为回测使用者，我要每 bar 每声部一枚显式裁决记录（含显式 Hold），以便 typed exit 单源完整、Hold 不再隐式。
20. 作为编排者，我要 T4 子树清仓接线（#183，OPEN）作为既有依赖被引用，以便散装路径归一（归一即删除散装等价物）不重新切票。
21. 作为编排者，我要 #181（SellDecision 迁移）与 #184（ADR 修订，待人审）只接线引用，以便既有票不重建。
22. 作为出票者，我要任何「建成」类新票出票前查《实现索引》并声明查重结果，以便分账本五重实装、子树清仓两实装的重造失守（G16/G17）不再复发。
23. 作为审计者，我要执行账 (account, level) 键全链携带（订单、pending fill、持仓、成本基、成交结果），以便级别作为分账维度贯穿执行层。
24. 作为验收者，我要对准落地后重跑终验A 条目 1/3/4 且判定翻转为对齐或让步合法化，以便 #152 结构偏离结案。
25. 作为实施工程师，我要现金单一根池维持不变，以便不触碰 2026-06-20 已结算裁定，且本 spec 内无任何现金分池设计。

## Implementation Decisions

### ID-0 规范性框架（目的地，不可协商）

会计层对准已有蓝图：声部为最小账单位、卖出按归属唯一记账、分账禁止净额化、级别为分账维度、全局次序表仅作消歧装置；现金单一根池维持已结算裁定（2026-06-20）。**不重设计、不发明新教义。** 出处：#177 裁定一、#174 目的地节。蓝图锚点：P_sep 分账本（推导完全互斥分类.pdf p.4–5/19 §5–6）；T₃₃–T₄₆ 会计层 14 定理（`docs/necessity_derivation.md:701-979`，Lean `formal/Tlayers/Accounting*` 全证无 sorry）；全局次序表 = 消歧确定性装置 Σ1[C_j]=1（买卖点.pdf p.6–8/15 §8–9）；短差定义（递归完全分类买卖点.pdf p.7–8/20 §9）；毛暴露递归约束（ADR 0001:44-45）。

### ID-1 出票查重声明（#174 Notes 防线二）

本 spec 全部工作项为「接线 / 采用 / 修复」，**无新建账本、新建解释器、新建引擎票**。已查《实现索引》（`.chanlun/implementation-index-20260723.md`）并按其能力节登记：

- 分账本 / 双腿不净额 → 能力 1：SplitLegLedger 已存在（仅测试，#149/#151 交付），OscillationBook 生产已接线但出口净额 → 本 spec 为**接线**（档A 共存），不新建。
- 通道解释器 / 消歧次序表 → 能力 2：channel.rs P1–P8 解释器已存在（仅测试，#147/#149/#150 交付）→ 本 spec 为**接线**（分阶段），不新建。
- 子树清仓 → 能力 3：#179 已裁「接线进生产、归一即删除散装等价物」，#183（OPEN）为其实施票 → 本 spec 仅**引用**。
- 执行账 (account, level) 键 → 缺口 G3（数学有、实现无）→ 本 spec 为**补缺口**（引用 G3）。
- 做空腿会计 / 二类开空通道 → 缺口 G4 + #185 审计条目 2（无 Type-2 开空 C 通道）→ 本 spec 为**补缺口**（引用 G4）。
- 多级毛暴露递归 → 缺口 G5：生产触发条件不存在（无子腿级 PanDivCert 信号源，#176 备忘 §2）→ 本轮**排除**（见 Out of Scope）。
- 重造警示 G16（分账本五重实装互不引用）、G17（子树清仓两实装）→ 本 spec 全部工作项均已对照，无再犯。

### ID-2 工作包划分（出票骨架，交 /to-tickets 细化）

**WP-1 SplitLegLedger 档A 共存接线**（#178 裁定 1，主战场）
- OscillationBook 保留路由与 lot 生命周期（多 lot、parent_leg_id 归属路由、live parents 同步均不动）；SplitLegLedger 作为其下数量不变量层，每 parent+lot 对映射一实例（ShortDiffAlreadyOpen 禁止单实例多子腿，故每-lot 一账本）。
- apply 双写 + 一致性断言：同步点（bar 级）1 处 + open/close 双写 2 处；断言不一致即拒，不允许静默漂移。
- 净额出口不动、行为零变化；加法抵消补丁不退役（退役条件随档B，#176 备忘 §4）。
- 依据：蓝图 T₃₃（Lean 已证）正是「合并下单、账分两边」的形状——档A 即蓝图级严格，非凑合。
- 连带裁（#178 裁定 2/3/4）：父仓不动 fill 层证实 = v1 显式让步（写入 ADR，含自指记录，见 WP-4）；档B 标记「不是最严格的实现，以后再搞」；nest() 多级递归本轮排除。

**WP-2 执行账归属键与四处现病修复**（#185 审计修复项；采纳其推荐路线，与蓝图声部/carrier 归属一致）
- 引入 `AccountIdentity::{Core{level}, ShortDiff, Short}`：单值 enum、禁 Option/集合/缺省；Short 对应 CONTEXT.md 反向根（无父反向声部）。引入 `AccountOrder{account, reason}`：账户与退出理由正交。
- (account, level) 键全链携带：订单、pending fill、持仓、成本基、成交结果（补缺口 G3）。
- 归属键粒度（用户裁定 2026-07-23）：**不用每级总余额**——短差、头寸（Core）、次级别做空（Short）三账必须分清；余额按（账户, 级别, 仓位节点）分实例记账，聚合值仅作派生视图（可求和呈现，不作 canonical 存储）。此裁定结案 #185 存疑区 1。
- 修复 a（最严重单点）：silent_drops 归属判定改为仅 `entry_v == ShortDiff` 才记 CloseShortDiff，其余走 core structural exit（account=Core{level}、reason=StructuralPrune）。
- 修复 b：二类反向从无条件 CloseRoot 改为「仅残余才纠错」——`CoreResidualCorrection{level}` 仅在核心残余实测非零时触发；否则二类不得生成本仓卖单。
- 修复 c：二类开空 C 通道——新增 `OpenShort{level, certificate}`，在候选进入 close 分支被消费之前先分账户（最早单源接入点）；不把开空塞进 ExitType，账户身份与退出理由正交。
- 修复 d：风险强平展开为多条单账户 AccountOrder；venue 侧只允许最外层净额投影，内部三账户先原子过账 + 可逆 aggregation/reconciliation。
- 行号锚点（runner.rs:1709-1713、interp.rs:357-362、interp.rs:1325、coverage.rs:3029 等）以 #185 审计报告为准，随代码漂移以报告口径复核。

**WP-3 解释器分阶段接线**（#177 裁定五委托 spec 阶段处置；**用户已批准（2026-07-23）：接，按阶段 A→B→C（仅 P2/P3）**）
- 解释器新身份 = 蓝图消歧装置 + typed exit 单源（#177 裁定五）；全局次序表仅作消歧装置，非经济仲裁。
- 批准依据（原建议理由）：① 与目的地一致——消歧装置不接线则「typed exit 单源」不存在，散装链继续「行为近似、无结构对应」；② 备忘证实阶段 A 零行为变更（shadow 双链比对，订单轨 bit-exact 不动，§3a 全部测试不翻），风险最小先行；③ 阶段 B 冻结裁决 schema（per-voice 裁决序列 + 显式 Hold），为后续换内核锁定契约；④ 阶段 C 仅 P2/P3 与散装链同单源判据（find_reverse + reverse_exit_type 同源），单腿单候选场景等价性已有既有同构见证（coverage.rs:5068）。
- 阶段 A：shadow 双链比对。在组合层裁决点之后并行跑 channel 适配层，仅记录分歧，不改裁决与订单流；parent_kappa 恒 Unknown（P7 记录语境降级：用户裁定（2026-07-23）接受为 v1 状态并计入 fog；缺口 G7 记录在案）；隔离边界 = StepTrace 只增字段或 sidecar。
- 阶段 B：trace/裁决层统一。散装链产出显式 per-voice 裁决序列（含显式 Hold），StepTrace 增字段（带 Default 增量），runner 消费段加轨不减轨；裁决 schema 冻结。
- 阶段 C：仅替换 P2/P3（本级证书平仓）。多腿/多候选场景两链裁决结构不同（channel 每声部每步一枚 vs interp 每候选归桶），非 bit-exact——分歧输入逐条核对或经裁决确认为新基线。
- 明确不接（本 spec 内）：TW 三通道维持组合层分支现状（channel 无槽位；扩槽 vs 外挂终局出范围）；P4/P5/P7 待 parent_projections 生产构造与 parent_kappa 源另行裁决；P8 加仓槽不实装（占位恒 false 保留）。
- 前置：输入适配层（VoiceState 多级持久化、candidates 按级别分发、parent_projections 构造）为新建适配代码，非新建解释器；channel 只出裁决不建腿（生产侧建腿保留，advance 不进生产路径或改写）。

**WP-4 ADR 0001 落稿/修订**（随 #184 执行；#184 修订清单 9+5 条待人审，本 spec 提供 delta）
- 写入 #177 裁定二：首开反向仓标准位 = 一类点（带标记、可复检）；delta 修正 #184「二类为开空/加空确认点」口径为「二类点 = 第二入场/加仓（加空）位」。
- 写入 #177 裁定三：B/C 撞车 ambient 守卫读法（父声部 active⇒短差账；无 active 父⇒空仓账）；注明与「最高级别走势类型」关联，机制细节留 fog。
- 写入 #178 让步条款：父仓不动 fill 层证实 = v1 显式让步 + 自指记录（ADR 行34 批判净额视角，而 v1 执行层仍净额单出口，fill 级「父仓不动」不可证实，挂账）。
- 写入档B 标记：「不是最严格的实现，以后再搞」（用户原话 2026-07-23）+ 拦下理由（funding/borrow 毛暴露口径蓝图与 ADR 均无明文）。
- 写入行为口径（ID-3 全文）。
- 修正文本债：S4 与优先级条目编号错位一格；#184 口径错位按上述 delta。

**WP-5 既有依赖接线引用（不重新切票）**
- #183（OPEN）：T4 子树清仓镜像函数接线进生产 π loop——coverage/runner 两处散装调用点归一到镜像函数，restore 语义保留，BTC 基线重跑核对，归一即删除散装等价物。
- #181（OPEN）：SellDecision 死路径迁移下线。
- #184（OPEN）：ADR 0001 文本债修订（待人审；WP-4 的 delta 并入）。

**WP-6 结构层复审**
- 对准落地后重跑终验A 条目 1/3/4，确认判定翻转为对齐或让步合法化；结果回报 #174 收口。

### ID-3 行为口径（#177 裁定二/三 + #189 研究结论，写入 spec 与 ADR）

- 一类卖点：当场全平（该级该方向全部清仓），允许当场反手；**无「新走势类型确认」前置**（冻结链确错，三源互证，#189）。一类反手 = 允许非要求。
- 首开反向仓标准位 = 一类点（带标记）：多重赋格的多空双开要求首开反向只能在一类点；一类点本身经背驰 + 区间套确认。
- 二类点 = 第二入场/加仓（加空）位。
- 二类点合法卖出仅两身份：短差、开空；二类清本仓只能是残余纠错（CoreResidualCorrection，实测残余非零才触发）。
- B/C 撞车按 ambient 守卫读法（子声部.pdf §15）：父声部 active⇒记短差账；无 active 父⇒记空仓账。与 CONTEXT.md 反向根条款（有父即短差、无父即反向根，互斥且完备，不存在第三种、不存在同时双记）一致。

### ID-4 优先级仲裁原文张力的登记（不裁决）

策略 PDF 假设 4「解释器固定优先级」（缠论的全互斥定义策略.pdf p.20/27）与 necessity T₂₁「去全局互斥是定理」（`docs/necessity_derivation.md:517-530`）存在原文分歧（缺口 G10）；会计层数学对此中立（T₃₃–T₄₆ 不含仲裁结构）。本 spec 采纳 #177 裁定的定位：全局次序表 = 消歧确定性装置；G10 本身留待裁决，不在本 spec 处置。

## Testing Decisions

### 好测试的标准

只测外部行为，不测实现细节：订单流、typed ledger 序列、账本守恒/一致性断言、显式裁决记录为外部行为；内部数据结构、fold 顺序、私有中间态不是。生产路径见证优先于测试级见证——#191 实证 B 类失守（切票丢母 spec 生产限定）为本项目 orphan 主根因，故每票验收以「生产路径上断言被真实触发」为准，不以测试内自证为准。

### 两道防线（每票验收硬要求，#174 Notes / #178 裁定 5）

1. 每张实施票验收必须显式写明「**接进 run_theta_v0_pi + 生产路径测试见证**」；WP-1 进一步显式写明「双写断言的生产路径见证」。
2. 任何「建成」类新票出票前必须查《实现索引》并声明查重结果；本 spec 的查重声明见 ID-1，后续细化票沿用。

### seam 决策（最高可行处、优先既有 seam；两条均经用户确认采纳，2026-07-23）

- **WP-1 的首选 seam = 档A 自身**：OscillationBook↔SplitLegLedger 同步点与 open/close 双写处的一致性断言直接落在生产路径，是「既测试又生产见证」的断言面，无需新建 seam。
- **WP-3 的 seam = 既有双链同构见证的全量化**：coverage.rs 既有 `pi_theta_step_traced` 双链同构断言（coverage.rs:5068，内部 5118–5166 交叉断言 channel 与散装裁决一致）是仓内唯一双链同构见证；shadow 阶段即其全量化——复用不新建。
- 回归基线 seam（既有，不动）：runner typed ledger 测试系；bit-exact 回归锁（订单轨、成本模型、opsem dump 三把锁）。

### Prior art（既有测试，直接沿用）

- coverage.rs:5068 `..._typed_close_root_and_reduce_core`——双链同构见证（shadow 的原型）。
- channel.rs 自测 23 枚（tests mod）：2^8 穷举见证（873）、S2 二分与 reverse_exit_type 交叉（922）、显式 Hold（960、1466）、P7 记录桶 8 枚、同时刻多候选 θ 序（1516）——接线后应继续通过。
- ledger.rs 双写/typed 拒绝测试（GrossExposureExceeded 等，985–1105 区间）——档A 下预期零翻动。
- runner typed ledger 系（typed_ledger_reverse_close_root 等）与主循环回归/因果无前视/determinism——回归基线。
- bit-exact 回归锁三枚（订单轨默认非活跃、成本模型 none、opsem dump env 门控）。
- interp.rs 自测（interpret 本体 10 枚）——接线保留函数则继续过；本 spec 不删 interpret。

### 预期翻动（按工作包，账出自两份备忘 §3/§5）

- WP-1（档A）：ledger.rs/oscillation.rs/overlay_state.rs 三文件合计 44 枚预期**零翻动**；runner.rs 消费 pan_div 路径者**新增**同步/一致性断言（翻动=增不改）；pan_div.rs 测试构造器面小改。
- WP-2：FollowParent 修复改变 typed trade 的 exit_type 分桶（silent_drops 相关断言与 μ 分桶统计随口径更新）；归属键全链携带触及订单/持仓/typed ledger 结构，凡断言 ExitType 序列与订单流者按新口径逐条核对；新增断言 4 条（下）。
- WP-3 阶段 A：**零翻动**（订单轨 bit-exact 不变为前提，任何翻动即阶段 A 失败信号）。阶段 B：StepTrace 增字段带 Default 增量，构造点显式列字段处少数翻动（四个 return 点中三处 `..Default::default()` 可容忍）；opsem dump 若增 Hold 行则其 bit-exact 锁翻动（格式断言）。阶段 C（P2/P3）：散装链行为/trace 断言约 30 枚（备忘 §3a 清单：pi_theta_step_traced 系 10 枚、runner typed ledger/端到端 8+ 枚、主循环回归与三把 bit-exact 锁）在分歧输入上翻——逐条解释或经裁决确认为新基线（沿用 #179 BTC 基线重跑核对的程序）。
- WP-5（#183）：行为等价性重新论证 + BTC 权威基线重跑核对，位级差异逐条解释或经裁决确认为新基线（#179 裁决原文）。

### 新增断言（#185 审计「建议断言」，随 WP-2 落地）

1. T1 目标态：一类卖(L) ⇒ target_qty(Core{L}) == 0（裁决生成后插入）。
2. T1 实际成交：filled(T1CoreClose{L}) ⇒ execution_ledger.qty(Core{L}) == 0（**批次边界口径**：一类批末=反向关闭循环结束点按 level 集合检查；前置 = pending order 携 account/level/reason。注：原子全平前提已被 #209 废除为多腿批处理，本断言随之为批次口径，Spec 评审 2026-07-24 认可）。
3. T2 身份：二类卖(L) ⇒ action.account ∈ {ShortDiff, Short} 且 ≠ Core；qty(Core{L})>0 时必须另发 CoreResidualCorrection{L}。
4. 短差隔离：fill.account == ShortDiff ⇒ Δqty(Core{*})==0 且 Δqty(Short{*})==0（模型层 SplitLegLedger 已有等价保证，本断言补生产执行层同款）。
5. 恰一账户：类型约束优于运行时计数——AccountOrder.account 为单值 enum，禁 Option/集合/缺省。

## Out of Scope

- **档B 逐腿下单出口**：已标记「不是最严格的实现，以后再搞」（#178 裁定 3）；待 funding/borrow 毛暴露口径蓝图明确后单独立案；加法抵消补丁退役随该票。
- **多级毛暴露递归**（SplitLegLedger::nest，孙⊆子⊆父）：本轮排除（#178 裁定 4）——无子腿级 PanDivCert 信号源，属新语义非接线；入 fog，信号源出现时再立。
- **parent_kappa 生产源**：本轮不建（新语义）；P7 记录语境降级接受为 v1 状态（用户裁定 2026-07-23），入 fog，与 P4/P5/P7 通道终局一并再裁。
- **orphan 处置图**（45 个 Lean 孤岛 + trading/analysis 老线）：域外清理，本图收口时另立地图（种子 = #191 六块 dossier + #192 索引）。
- **#153 终验B（实证锚）本体**：独立票；本图只定先后序（本图收口后按原规矩开跑），不含其内容。
- **TW 通道扩槽/外挂终局**：channel 8 槽无 TW 位，扩槽与「不得重排字段序」冻结注记及 2^8 穷举见证冲突，且须同步改 ADR 优先级条文——终局裁决不在本 spec；本 spec 内 TW 三通道维持组合层分支现状。
- **P8 加仓槽实装**：占位恒 false 保留，不接不删。
- **最高级别走势类型 × ambient 守卫的机制细化**（父声部 active 谓词与最高级别走势类型存活的精确关系）：fog，列入地图 Not yet specified。
- **G10 优先级仲裁原文张力的裁决**（策略 PDF 假设 4 vs necessity T₂₁）：登记在案（ID-4），不在本 spec 处置。
- **禁空市场执行层降级**：#185 审计已明确本轮暂缓、不在范围（涉及段落仅存档）。

## Further Notes

- 文本债提示：ADR 0001 的 S4 与优先级条目编号错位一格（短差开启在 ADR 链位 4、代码位 5；S4 的 `¬P2∧¬P3` 语义在 channel.rs 成立但编号所指对象自相矛盾）。本 spec 引用通道编号时以 channel.rs 槽表与 mutex.rs 口径为准并随 #184 统一。
- 存疑区（来自 #185 审计，留人裁/后续票，不阻塞本 spec）：
  1. ~~`Core{level}` 粒度：总余额 vs 分实例~~【已裁 2026-07-23】不用总余额；短差/头寸/次级别做空三账分清，按（账户, 级别, 仓位节点）分实例记账，聚合仅派生视图（见 WP-2 归属键粒度裁定）。
  2. ReduceCore 在 channel 状态机实际整腿关闭；三类点减仓比例未裁。
  3. venue 最终 NETTING 还是 HEDGING 未定；NETTING 须保留内部三账户过账 + 可逆 aggregation ledger，不能把 venue 净仓当 canonical 账户。
  4. 主净额路径窗口终点「强平」仅追加 PnL、未真实成交归零；若窗口边界也纳入「每个卖出恰一账户」需另行统一。
  5. Type2Missing 定性为 legacy 诊断标签（账户效果为零）；其改名隔离与 #184 口径新经济样本为审计推荐并行项，可随 WP-2 或另立小票。
- 谱系：#174（母地图）、#177/#178/#179（裁定）、#175/#176（勘察备忘）、#185（混桶审计）、#186–#191（研究簇）、#192（实现索引）、#152（终验A）、#181/#183/#184（既有票）。
- 090 纪律：本 spec 全部事实性断言均可回溯至上列裁定评论、三份报告与实现索引；行号锚点以各报告基线为准（#185 基线 main@80d1540a62），未重新跑测试核实处已在相应节标注。
