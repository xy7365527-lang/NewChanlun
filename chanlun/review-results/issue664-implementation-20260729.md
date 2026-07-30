# #664 实施报告：CenterDeathCertificate 扩 side 字段——cert 杀路径方向语义闭合

日期：2026-07-29　worktree：`/private/tmp/wt-664`（分支 `ticket-664`，基 = main `d7d3d476f1`）
模型档：常规（sonnet）　范围：#637 尾部方向盲区（HIGH-5 + 编排者呈裁 4A）

## 一、改动摘要

| 文件 | 改动 |
|---|---|
| `rust/src/theta_v0/classifier/retrace_ledger/book.rs` | `CenterDeathCertificate` 增 `side: RetraceSide` 字段；`certificate_of` 取自落锤证据 `entry.terminal_evidence().side` |
| `rust/src/trading/center_book.rs` | `consume_death_certificate` 签名扩为 `(ladder, cert, hard_type3, events_out)`；按 `cert.side` 登记 `dead_down`（Sell）/ `frozen`（Buy && hard_type3）/ 补发 `CenterEvent::Terminated`（仅 `Broken` 分支）；模块头与方法 doc 的「⚠ 方向盲区」段落改写为「已闭合」；测试新增 5 条 + 既有 8 条测试签名同步更新 |
| `rust/src/theta_v0/classifier/retrace_ledger/mod.rs` | 消费面登记表第 2 行「④」措辞同步（`ThirdPointPack`/`CenterDeathCertificate` 已同步带 `side`，不再是单侧盲区） |
| `rust/src/trading/third_point_book.rs` | `#664 关系声明` 段落 + `consume_death_certificate` 签名引用同步更新 |

## 二、方向登记设计

`CenterDeathCertificate.side` 取自落锤拍证据 `RetraceEvidence.side`（`RetraceEntry::terminal_evidence()`），与 `ThirdPointPack.side` 同源同值（票面材料已确认二者共享 `TerminalEvidence.side` 落锤拍）。`certificate_of`（`book.rs`）在构造证明时一并带出，无需额外查询。

`CenterBook::consume_death_certificate` 在判定次序步骤 3（`known` 命中、非早退幂等/改口分支）落位：
- `side == Sell` ⟹ 插入 `dead_down`（镜像 `ingest` `:165-172` 的方向判据——三卖终结「不能回补」）；
- `side == Buy && hard_type3` ⟹ `frozen[ladder] = Some(anchor)`（镜像 `ingest` `:183-184` 字面判据）；
- 仅在**真正首次登记死亡**（`!already_dead`，对应 `Ok(Broken)`）时补发 `CenterEvent::Terminated { seg_start, direction }`（`direction` 映射同 `ingest`：Buy→Up / Sell→Down）；`AlreadyBroken`（无论是早退的同框重复消费，还是 `known` 命中但 `ingest` 已先杀）**不重发**——避免同一教义事件被两条通道各报一次事件，`version` 与事件发射同一门控。

**（修复轮 #2 订正：本段描述的是首轮已推翻的行为，见影子评审 MEDIUM-3 / 下方「十二、修复轮 #2」）**
`dead_down`/`frozen` 的登记与 `CenterEvent::Terminated` 补发**仅在本次是真正首次死亡登记时**（`!already_dead`，对应 `Ok(Broken)`）写入；`already_dead` 为真（`AlreadyBroken`）时本方法一律不动 `dead_down`/`frozen`、不补发事件——`consume_death_certificate` 自身不存在"照实落一遍"的补写分支。跨通道方向冲突（`ingest` 已以某方向杀锚、cert 携不同方向后到）的处置口径是**先杀者为准**，本票不设 fail-loud 检查；但如实登记（影子评审 MEDIUM-1）：这条"先杀者为准/不覆写"的纪律只对 `dead_down` 与 `Terminated` 单侧成立——`frozen` 因 `ingest` 侧的 Python parity 既有形状（`frozen` 置位在死亡守卫外）而有例外，cert 先杀后若 `ingest` 收到同锚 confirmed hard Buy3，`frozen` 仍会被 `ingest` 无条件覆写。详见下方「十二、修复轮 #2」与 `center_book.rs` 模块头/方法 doc。

## 三、frozen 语义设计取舍（票面要求「须给出等效判据或如实声明不置」）

`ingest` 路径的 `frozen` 置位判据是 `hard_type3 && side == Buy`，其中 `hard_type3` 是 **`OrganicConfig` 的策略配置位**（非单条事件的"够不够硬"质量标记，见 `config.rs:260` 与两处默认值 `true`/`false`），由 `runner.rs:763` 在调用 `ingest` 时逐次传入。

`consume_death_certificate` 此前无法访问该配置，票面给出三个选项：(a) cert 杀 Buy 侧一律置 frozen；(b) 一律不置 + 如实声明；(c) 其他等效判据。

**采用 (c)**：给 `consume_death_certificate` 增加 `hard_type3: bool` 参数，判据字面复制 `ingest` 的 `hard_type3 && side == Buy`——不是发明新语义，是把"同一个策略配置位"从"`ingest` 调用时的运行时旗标"改为"本方法调用方显式传入的同一旗标"。选择理由：
- (a) 会让 cert 路径与 `ingest` 路径的 frozen 语义在 `hard_type3=false` 配置下产生**不一致**（cert 恒 frozen，ingest 恒不 frozen）——两条通道观测同一教义事件却给出不同副作用，违反模块头"同一事件的两条观测通道"定位；
- (b) 安全但留下与票面意图相悖的功能空白（frozen 语义完全不闭合）；
- (c) 零语义发明、零跨路径不一致，且 `consume_death_certificate` 目前**无生产调用点**（#637 文档已declare「驱动入口的上游生产链…归 #575 后续票」），改签名不破坏任何生产调用方，只需同步测试调用点（已完成，8 条既有测试 + 5 条新测试全部更新）。

已用测试钉死该语义：`buy_certificate_end_to_end_registers_up_direction_and_frozen_semantics`（`hard_type3=true` + Buy ⟹ frozen）、`buy_certificate_without_hard_type3_does_not_freeze`（`hard_type3=false` + Buy ⟹ 不 frozen）。

## 四、CenterEvent::Terminated 补发评估（已落地，非声明不发）

评估结论：**落地补发**，理由见上方"二、方向登记设计"第三条。签名扩为 `events_out: Option<&mut Vec<CenterEvent>>`，风格与 `ingest` 的 `Option<&mut Vec<CenterEvent>>` 惯例一致。

对既有 8 条 #637 测试的影响：全部只需在调用点补 `hard_type3`/`events_out` 两个参数（多数传 `true, None`），**判定次序/返回语义零回归**（见下方「五、#637 语义零回归证据」）。

## 五、#637 语义零回归证据

`cargo test --lib trading::center_book` 全绿，8 条 #637 原有测试逐一核对：

| 测试 | 结果 | 备注 |
|---|---|---|
| `death_certificate_registers_broken_end_to_end` | ok | `real_death_certificate` 扩了 `leave_direction` 参数（用于驱动本票新增的 Sell 侧场景），此测试传 `LedgerDirection::Up` 维持原语义 |
| `no_matching_center_never_seen_fails_loud` | ok | 仅签名追加参数 |
| `superseded_anchor_certificate_still_registers_broken` | ok | 同上 |
| `ingest_kill_then_certificate_is_idempotent_and_archives_frame` | ok | `cert()` 补 `side: RetraceSide::Buy`（与该测试 ingest 侧 confirmed Buy3 叙事一致），判定/幂等逻辑不变 |
| `repeated_consumption_is_idempotent_zero_action` | ok | 同上，`side: RetraceSide::Sell` |
| `conflicting_certificate_same_anchor_different_frame_fails_loud` | ok | 同上 |
| `cert_kill_clears_pending_departure_window_and_negate_is_noop` | ok | `side: RetraceSide::Buy`（与该测试 candidate Buy3 窗口叙事一致） |
| `late_issued_as_of_does_not_block_broken_when_anchor_still_present` | ok | 仅签名追加参数 |

判同/幂等/改口 fail-loud 三条 #637 落地语义（同框 AlreadyBroken / 异框 ConflictingCertificate / 从未见过 NoMatchingCenter）**逐字未动**——本票只在判定次序步骤 3（登记 Broken 之内）追加方向副作用，未触碰步骤 1/2 的分支结构。

## 六、序列化面核查结论

- **audit.rs**：`CenterDeathCertificate` 不在 `RetraceAuditEvent`/`RetraceAuditRecord` 的字段里出现，audit 流不序列化本类型，无需改动。
- **golden 锚（#610 重锚 `0xe371_3897_d9bf_978c`）/ `golden_log.rs`**：`CenterDeathCertificate` 不是 append-only 日志（`RetraceRecord`/`RetraceRevision`）的组成部分——它是从 `Confirmed` 条目**现算**（`certificate_of`）的只读投影，不参与 JSONL 序列化/字节锚/指纹锚。`golden_log.rs:245` 只读 `pack[0].death_certificate.issued_as_of` 一个标量字段（非结构相等断言），未受影响。**结论：本票无需重生成任何 golden fixture**（`cargo test --lib retrace_ledger` 全绿，含 `golden_event_log_matches_the_repo_anchor` / `golden_log_digest_is_pinned_and_tamper_evident` / `golden_log_folds_back_to_the_anchored_ledger_state` 三层锚）。
- `CenterDeathCertificate` 本身不派生 `Serialize`/`Deserialize`（与 `RetraceEvidence`/`RetraceRevision` 等留档核心类型不同），新增 `side` 字段不产生任何 wire 格式变更。

## 七、测试清单

**（修复轮 #2 订正：以下清单第 5 条为首轮描述，已被修复轮改口，测试已更名且语义反转，见影子评审 MEDIUM-3 / 「十二、修复轮 #2」）**

`cargo test --lib trading::center_book`：本轮终跑 26 passed（19 原有 + 5 首轮新增 + 2 修复轮新增负控），0 failed。

首轮新增测试（票 #664）：
1. `sell_certificate_end_to_end_registers_dead_down_with_correct_direction_reads` —— Sell 证明端到端：`is_dead_down` 为真、`is_frozen` 为假、补发 `Terminated{Down}`。
2. `buy_certificate_end_to_end_registers_up_direction_and_frozen_semantics` —— Buy 证明端到端：`is_dead_down` 为假、`hard_type3=true` 时 `is_frozen` 为真、补发 `Terminated{Up}`。
3. `buy_certificate_without_hard_type3_does_not_freeze` —— `hard_type3=false` 时 Buy 证明不置 frozen（钉死设计取舍 (c) 的门控语义）。
4. `already_broken_outcome_does_not_reemit_terminated_event` —— 同证明重复消费（早退幂等路径）不重发事件。
5. ~~`ingest_kill_then_sell_certificate_backfills_dead_down_without_reemitting_event`~~ ——**首轮语义（已推翻）**：本条描述"方向信息幂等重申"（即 cert 仍照实补写一遍）。修复轮 #1 已将其更名为 `ingest_kill_then_same_side_certificate_does_not_overwrite_direction`，断言语义反转为"不覆写、方向读数保持 ingest 落位原值"（见「十二」）。

修复轮新增负控（票 #664 修复轮 #1 / #2）：
6. `ingest_kill_then_conflicting_side_certificate_does_not_overwrite_frozen`（修复轮 #1）—— `ingest` 先以 Sell3 杀（`frozen` 从未置位）→ cert 携冲突方向 Buy 后到 ⟹ `AlreadyBroken`，`frozen` 不被静默翻成 `Some`。
7. `ingest_kill_then_conflicting_side_certificate_does_not_overwrite_dead_down`（修复轮 #2，影子评审 MEDIUM-2）—— 镜像负控：`ingest` 先以 hard Buy3 杀（`dead_down` 保持 false）→ cert 携冲突方向 Sell 后到 ⟹ `AlreadyBroken`，`dead_down` 不被静默插入。此前只有 `frozen` 侧有真负控，`dead_down` 侧的"不覆写"此前无回归防护（把 `dead_down` 写移出 `!already_dead` 守卫，25 条测试仍全绿）——本条钉死。

`cargo test --lib retrace_ledger`：130 passed，1 ignored，0 failed（含 golden 三层锚，本轮未改动该模块判定逻辑）。

## 八、指纹对照

| 阶段 | passed | failed | ignored |
|---|---|---|---|
| 基线（`git stash` 回退到 main HEAD 状态） | 2583 | 0 | 138 |
| 终跑（含本票改动） | 2588 | 0 | 138 |

差值 +5 = 本票新增的 5 条测试，**0 failed 硬杠达标**。已知 flaky `incremental_tower_scaling_dominates_full_synthetic` 两次运行均 `ok`。

## 九、Standards 自查

- TDD：新增测试先行编写（未观察到先绿后改的顺序问题——本次是在既有 8 条测试基础上扩展签名 + 新增 5 条方向测试，均在实现改动的同一提交周期内交叉验证，编译失败→改实现→测试绿的顺序已走通）。
- 改动最小化：签名扩展只影响 `consume_death_certificate` 一个函数与其调用点（仅测试，无生产调用点）；未触碰判同/幂等/改口三条 #637 落地语义的分支结构。
- 无自造账本机制：方向来源直接复用既有 `RetraceEvidence.side`/`RetraceSide`词汇，未新增枚举或状态位。
- 文档同步：模块头「⚠ 方向盲区」段落、方法 doc、`mod.rs` 登记表第 2 行、`third_point_book.rs` 的 #664 关系声明与签名引用均已改写为「已闭合」措辞，无遗留的过期声明。

## 十、遗留风险 / 未裁项

1. **跨通道方向冲突未设 fail-loud 检查（修复轮 #2 订正：本条首轮描述已被修复轮改口推翻）**：修复轮 #1 之后，`consume_death_certificate` 自身在 `AlreadyBroken` 分支**一律不再**写 `dead_down`/`frozen`/补发事件——不存在"cert 的方向信息也照实落一遍"的行为。真实遗留形态是：跨通道方向冲突（`ingest` 已以某方向杀死某锚，cert 携带**不同**方向后到）的处置口径是**先杀者为准，本票不设 fail-loud**——后到通道的方向信息被静默丢弃（读数可能与真实方向相反），不像"同锚不同框"（`ConflictingCertificate`）那样拒收。且这条"先杀者为准"本身**单侧成立**（影子评审 MEDIUM-1）：`dead_down`/`Terminated` 侧确实先杀者为准且不被后到通道覆写；`frozen` 侧有 Python parity 携带的例外——`ingest` 的 `frozen` 置位在死亡守卫外（`#664` 之前既有形状，非本票引入），故 cert 先以 Sell 杀锚后，若 `ingest` 收到同锚 confirmed hard Buy3，`frozen` 仍会被 `ingest` **无条件翻成** `Some`（不推 `version`、不发第二次 `Terminated`）——即 `frozen` 上不是"先杀者为准"，而是"`ingest` 的 parity 行为为准"。理由：①两条通道理论上观测同一教义事件，方向理应一致；②`broken_by_certificate` 目前只比较 `CenterFrame` 四条边，不含 side，改动该判同逻辑超出票面「不动 #637 已落的锚判同/幂等语义」的边界；③`frozen` 例外受 Python parity 约束，代码不宜动。**未裁**——如需堵住此边界情形，建议开新票，按 #637 的 `ConflictingCertificate` 同族扩展判同键（锚 + 框 + side），并处理 `frozen` 的 parity 例外。
2. **`consume_death_certificate` 仍无生产调用点**：本票与 #637 一致，只完成消费方接线的方向语义；驱动入口（`RetraceLedger` 接生产驱动 + trading ladder 坐标对齐）仍归 #575 后续票。
3. **域腿解冻观测（`CenterEvent` 第三类 v2 消费者）当前只消费 `Formed`**（`fatigue_gate.rs`），`Terminated` 目前唯一消费者是 `level_operating_unit.rs` 的 T4b 加码逻辑（只关心 `Terminated{Down}`）——cert 路径补发的 `Terminated{Up}` 目前无生产消费方读取（`t4b_addons` 对 `Terminated{Up}` 走 no-op 分支），如实声明，非本票范围。
4. **cert 幂等键不含 `side`（影子评审 LOW-2）**：`consume_death_certificate` 步骤 1 的幂等判同只比较 `*frame == cert.center`（`CenterFrame` 四边，不含 `side`），故"同锚 + 同框 + 方向相反"的第二张证明会被判 `AlreadyBroken`（既不 fail-loud 也不改方向），与 #637「上游改口 fail-loud」纪律不同调。当前无实害——账本侧不变量兜底（同锚至多一个 `Confirmed`，同一身份不可能开出两张同框异 `side` 的证明）；但这是跨模块隐式依赖，`CenterBook` 侧无任何类型级/运行时保障。若日后该不变量被打破，同锚异 `side` 证明会被 `AlreadyBroken` 静默吞掉，方向读数不会更新也不会报错。如实登记，非本票范围。
5. **`events_out` 传 `None` 时 cert 杀方向事件永久丢失（影子评审 LOW-1）**：cert 侧对同一锚只登记一次死亡（`Broken` 恰一次），若该次调用传 `None`，`CenterEvent::Terminated` **永久丢失**——不同于 `ingest` 侧漏一次还能靠后续 `Formed`/`Extended` 事件补。当前无生产调用点故无实害，`center_book.rs` doc 已加警示，#575 驱动票接线时须留意传 `Some`。

## 十一、修复轮（2026-07-29 第二轮）

对首轮实施的 Standards/Spec 双轴复核，共 4 条 Standards 判断题 + Spec 轴登记，逐条处置如下。

### Standards 轴

| # | 判断题 | 裁定 | 处置 |
|---|---|---|---|
| S1 | `AlreadyBroken` 分支「补落 `dead_down`/`frozen`」的 doc 理由（"若 ingest 杀时方向信息缺失，本次补齐"）失实——`ingest` 杀路径方向从不缺失（confirmed Type3 分支 Sell 必插 `dead_down`，hard Buy 必置 `frozen`），该补写实际只做两件事：同方向重复（无害）或**方向冲突时静默翻转**（如 `ingest` 已以 Sell 杀、cert 携 Buy 后到 → `frozen` 从 `None` 静默翻 `Some`），与本模块「同锚不同框 fail-loud」的改口纪律不对称 | **CONFIRMED，已修复** | `consume_death_certificate`（`center_book.rs:441-467`）改为「先杀落位、后不覆写」：`dead_down`/`frozen` 写入与 `CenterEvent::Terminated` 补发全部收窄进 `!already_dead`（真 `Broken`）分支；`AlreadyBroken`（含步骤 1 同框早退与步骤 3 `ingest` 先杀两种情形）一律不再触碰方向位、不补发事件 |
| S2 | 该补写不推进 `version`，使 `version` 门控方（`SizeAllocator` 重算门控）感知不到 `AlreadyBroken` 路径下方向读数的静默变化 | **CONFIRMED，已修复** | 同 S1 修复后，`AlreadyBroken` 分支既不写方向也不发事件，`version` 与方向登记同一门控点重新对齐——不存在"方向变了但 version 不知道"的缺口 |
| S3 | consume doc 行号引用 `:171-173` 与 `ingest` 现行代码实际行号不符（`if hard_type3 && ev.class.side() == Side::Buy` 在 `:175`，`self.frozen[ladder] = Some(cs)` 在 `:176`） | **CONFIRMED，已修复** | 亲核 `ingest` 现行行号后订正为 `:175-176`（`center_book.rs` 方法 doc 内引用处；注：本轮修复过程中模块头段落增了 2 行，行号从中途暂定的 `:173-174` 再次前移到 `:175-176`，以最终落盘状态为准） |
| S4 | 四处文档（模块头 / `consume_death_certificate` 方法 doc / `mod.rs` 消费面登记表第 1 行 / `third_point_book.rs` #664 关系声明）重复叙述"方向已闭合"，S1 修复后四处叙述均需同步新语义，否则文档与代码行为不一致（非 Speculative Generality——是必要的四处一致性同步，非过度设计） | **CONFIRMED，已修复** | 四处全部改写为「先杀落位、后不覆写」的新语义表述（含跨通道方向冲突「先杀者为准，本票不设 fail-loud」的如实声明）——见下方逐处清单 |

四处文档同步清单：
- `rust/src/trading/center_book.rs` 模块头（`//! 方向盲区已闭合` 段）
- `rust/src/trading/center_book.rs` `consume_death_certificate` 方法 doc（`方向登记` 段，含行号订正）
- `rust/src/theta_v0/classifier/retrace_ledger/mod.rs` 消费面登记表第 1 行 ④
- `rust/src/trading/third_point_book.rs` `#664 关系声明` 段

### Spec 轴

**PASS**——票面「方向登记闭合」的产品意图（cert 杀路径不再对 `is_dead_down`/`is_frozen` 恒读错误/沉默）在本轮修复后依然成立，且修复消除了首轮实现中"跨通道方向冲突静默覆写"这一未声明的行为面；票面未要求跨通道方向冲突必须 fail-loud（遗留风险已如实登记，见「十、遗留风险 / 未裁项」第 1 条），本轮不越界处理。

### 测试同步

- `ingest_kill_then_sell_certificate_backfills_dead_down_without_reemitting_event` 更名为 `ingest_kill_then_same_side_certificate_does_not_overwrite_direction`，断言语义从"补写方向"改为"不覆写、方向读数保持 ingest 落位原值"。
- 新增负控测试 `ingest_kill_then_conflicting_side_certificate_does_not_overwrite_frozen`：`ingest` 先以 Sell3 杀（`dead_down=true`，`frozen` 从未置位），cert 后到携冲突方向 Buy → `AlreadyBroken`，断言 `dead_down` 未被覆写/清除、`frozen` 未被静默翻成 `Some`、无第二次 `Terminated`——钉死本轮修复的核心行为。
- 其余测试（`buy_certificate_end_to_end_*` / `sell_certificate_end_to_end_*` / `buy_certificate_without_hard_type3_*` / `already_broken_outcome_does_not_reemit_terminated_event` 等真 `Broken` 分支方向登记测试）逻辑不受影响，无需改动。

### 收尾指纹

| 阶段 | passed | failed | ignored |
|---|---|---|---|
| 首轮基线（`git stash` 回退到 main HEAD） | 2583 | 0 | 138 |
| 首轮终跑 | 2588 | 0 | 138 |
| 本轮终跑（含修复轮改动） | 2589 | 0 | 138 |

差值 +1 = 本轮新增负控测试 `ingest_kill_then_conflicting_side_certificate_does_not_overwrite_frozen`（更名不增删测试数）。`cargo test --lib trading::center_book`：25 passed（原 24 + 本轮新增 1）；`cargo test --lib retrace_ledger`：130 passed，1 ignored（未改动该模块判定逻辑，与首轮持平）；全量 `cargo test --lib`：2589 passed，**0 failed** 硬杠达标。

## 十二、修复轮 #2（2026-07-29 第三轮；影子评审 CONDITIONAL 归置）

对 `chanlun/review-results/issue664-shadow-20260729.md`（独立第三视角影子评审）判词 CONDITIONAL 的 3 条 MEDIUM + 5 条 LOW 逐条对账。影子评审的两条硬性放行条件（MEDIUM-1、MEDIUM-3）与其余各条本轮已全部处置；未做任何 git mutation。

### MEDIUM 逐条处置

| # | 影子判词 | 处置 | 落点 |
|---|---|---|---|
| MEDIUM-1 | 「先杀落位、后不覆写」纪律单侧成立——`ingest` 的 `frozen` 置位在 `dead` 守卫外（Python parity 既有形状），cert 先杀后 `ingest` 收到同锚 confirmed hard Buy3 仍会静默覆写 `frozen`；doc 的绝对断言失准 | **改 doc，不改代码**（parity 禁区）：把「死亡只发生一次，方向由先杀通道落位」的绝对声明改写为如实声明——`dead_down`/`Terminated` 侧先杀者为准且不覆写；`frozen` 侧因 `ingest` parity 既有形状有单侧例外，以 `ingest` 的 parity 行为为准 | `center_book.rs` 模块头、`consume_death_certificate` 方法 doc、`retrace_ledger/mod.rs` 消费面登记表④行、`third_point_book.rs` `#664 关系声明`、本报告 §二/§十 |
| MEDIUM-2 | `dead_down` 的「不覆写」无回归防护——把 `dead_down` 写移出 `!already_dead` 分支（回退首轮实现），25 条测试仍全绿 | 补镜像负控测试：`ingest` 先以 hard Buy3 杀（`dead_down` 保持 false）→ cert 携冲突方向 Sell 后到 ⟹ `AlreadyBroken` 且 `is_dead_down` 保持 false、`frozen` 不被清除、无第二次 `Terminated` | `center_book.rs` 新增测试 `ingest_kill_then_conflicting_side_certificate_does_not_overwrite_dead_down` |
| MEDIUM-3 | 实施报告 §二/§七/§十 三段仍描述首轮已推翻的行为，与落盘代码及 §十一 矛盾 | 三段逐一订正为修复轮后真实语义：§二 改写"仅在 `!already_dead` 时写、`AlreadyBroken` 零动作"；§七 第 5 条标注为"首轮语义（已推翻）"并补 6/7 两条修复轮负控；§十 第 1 条改写为"先杀者为准但 `frozen` 有单侧例外"的真实遗留风险 | 本报告 §二/§七/§十（本次编辑） |

### LOW 逐条处置

| # | 影子判词 | 处置 |
|---|---|---|
| LOW-1 | `events_out` 传 `None` 时 `Terminated` 永久丢失，doc 未警示 | `consume_death_certificate` 方法 doc 加一段警示：cert 侧对同一锚只登记一次死亡，传 `None` 会永久丢失该次事件（不同于 `ingest` 侧可靠后续事件补） |
| LOW-2 | cert 幂等键不含 `side`，同锚同框方向相反的第二张证明被 `AlreadyBroken` 静默吞 | 报告 §十遗留风险新增第 4 条：如实登记账本侧不变量兜底（当前无实害）+ 若该不变量被破的后果 |
| LOW-3 | doc 行号引用 `:147-149` 失准（实际为 `ingest` 的 `pending_departure` 清除块），S3 只订正了一处 | `consume_death_certificate` 方法 doc 判定次序步骤 3 的行号引用改为 `:188-190`（亲核 `ingest` 现行代码确认，`center_book.rs` 本轮改动使模块头净增行数，行号以本轮落盘状态为准） |
| LOW-4 | `mut events_out` + `as_deref_mut()` 在 cert 路径冗余 | 去掉签名 `mut`，`events_out.as_deref_mut()` 改为直接 `if let Some(out) = events_out`（该函数内 `events_out` 只用一次，非循环借用，不需要 `as_deref_mut`） |
| LOW-5 | 「生产消费方读数方向正确」未字面端到端（票面验收①三个生产读点 `unified_osc.rs`/`level_operating_unit.rs`/`axiom_voice.rs` 均只经 `is_dead_down` 读方向，逻辑等价而非直接触达） | 本条为验收措辞精度问题，非代码/doc 缺陷——影子已核实三处生产读点全部且仅经 `is_dead_down`，等价推论成立；如实登记于此，无需额外改动 |

### 收尾指纹（本轮，含 MEDIUM-2 新增负控测试）

| 命令 | 结果 |
|---|---|
| `cargo test --lib trading::center_book` | 26 passed（本轮基线 25 + 本轮新增 1），0 failed |
| `cargo test --lib retrace_ledger` | 130 passed，1 ignored，0 failed（未改动，与前两轮持平） |
| `cargo test --lib`（全量） | **2590 passed / 0 failed / 138 ignored** |

对账：2589（上轮终跑）+ 1（`ingest_kill_then_conflicting_side_certificate_does_not_overwrite_dead_down`）= 2590 ✓。**0 failed 硬杠达标**。全程未提交（未做任何 git mutation），`formal/` 零改动，fixture 漂移 gate 不适用。
