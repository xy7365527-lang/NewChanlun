# #637 实施报告：中枢账消费死亡证明登记 Broken（#575 消费方接线 1/3）

日期：2026-07-29　worktree：`/private/tmp/wt-637`（分支 `ticket-637`，基 = main HEAD `039cf86974`）
模型档：常规（sonnet）　票面：GitHub #637（parent #575；上游 #622/#620）

**本报告已按修复轮全面修订**——首轮内容（§一 ~ §八）保留作历史记录，但其中与影子评审冲突、
被编排者裁定推翻的表述（§二判同映射、§三 `KilledByOtherCause` 形状、§四迟到折算、§六指纹表、
§七 rustfmt 声明）已被 **§九「修复轮」** 的对应节更新覆盖，阅读时以 §九 为准。§九是本次交付的
权威落点。

---

## §九 修复轮（2026-07-29，按编排者四项裁定 1A/2A/3A/4A 修正语义）

### 九.1 四项裁定逐字登记 + 落位

| 裁定 | 内容摘要 | 本轮落位 |
|---|---|---|
| **1A（迟到/已死）** | 迟到证明照登记（不问迟到——054:60、ADR-0001 补充十一「Superseded 非死」、#583「迟到但都到」）；被更替的锚 ≠ 无此中枢；两条三类点杀路径撞车 = 同一教义事件两次观测，幂等确认，非分歧；`KilledByOtherCause` 是伪概念，删除该变体；「无此中枢」限缩为「从未见过」 | 新增 `known` 字段（"见过"锚集，与 `dead` 分离）；`DeathCertificateError::KilledByOtherCause` 删除；判定次序步骤 2 改判 `known` 而非 `last`/`is_dead` |
| **2A（判同基座）** | 只做锚判同（`start_index as i64 ↔ seg_start`，名义映射，doc 声明两引擎段序列无对齐依据）；禁跨量纲数值比较——删除 `Tick as f64` 对 `LiveCenter.zd/zg` 的相等判断；证明的 `CenterFrame` 全四条边原样存档，同锚不同框 = 上游改口，fail-loud | `consume_death_certificate` 判同分支只比较 `anchor`，不再读 `self.last[ladder]` 的价格边；`broken_by_certificate` 类型从 `HashSet<i64>` 改 `HashMap<i64, CenterFrame>`，存档整框供"证明 vs 证明"全四边比较 |
| **3A（登记表口径）** | 票面「生产调用点 ≥1」按字面结（`CenterDeathCertificate` 出现于生产代码即达标）；登记表措辞收紧为「已接线（#637）：消费入口已立；驱动入口的上游生产链归 #575 后续票」；1/3 计数注明 = 入口计数口径 | `retrace_ledger/mod.rs` 消费方登记表 #1 行 + 计数行措辞按裁定原文改写 |
| **4A（方向缺口）** | cert 无 `side` ⟹ 不进 `dead_down`、不置 `frozen`、不发 `CenterEvent::Terminated`——本票不修，已上浮 #664（已立票，blocked_by #637）；本票只需代码 doc + 报告登记该盲区并指向 #664 | 模块头新增「方向盲区」大字声明段；`consume_death_certificate` 方法 doc 末段同样声明；本节 §九.5 遗留风险条目更新指向 #664 |

### 九.2 影子评审 13 条发现逐条处置对照

| # | 严重度 | 发现摘要 | 处置 | 去向 |
|---|---|---|---|---|
| HIGH-1 | HIGH | 迟到证明被拒，与票面/ADR-0001 反向 | **已修** | 裁定 1A：`known` 判同不再要求锚在 `last` 在场，迟到/被更替锚照登记 Broken；旧测试 6 断言方向已推翻（新测试 `superseded_anchor_certificate_still_registers_broken`） |
| HIGH-2 | HIGH | `KilledByOtherCause` 是伪概念 | **已修** | 裁定 1A：变体删除；ingest 先杀 + 证明后到 ⟹ `Ok(AlreadyBroken)`（新测试 `ingest_kill_then_certificate_is_idempotent_and_archives_frame`） |
| HIGH-3 | HIGH | 判同价格边量纲错配，真实数据恒不命中 | **已修** | 裁定 2A：删除 `zd`/`zg` 对 `LiveCenter` 的比较，只做锚判同 |
| HIGH-4 | HIGH | 票面「生产调用点 ≥1」未满足；登记表不实 | **按裁定字面结**，不修代码 | 裁定 3A：票面按字面结（`CenterDeathCertificate` 出现于生产代码即达标）；登记表措辞收紧为「消费入口已立；驱动链归 #575」，不再用可能引起误读的「已接线」裸词 |
| HIGH-5 | HIGH | cert 杀路径不清 `pending_departure`，H2 力度历史可被污染；`dead_down`/`frozen` 缺失 | **部分修 + 部分上浮** | `pending_departure` 清除已补齐（新测试 `cert_kill_clears_pending_departure_window_and_negate_is_noop` 固化时序）；`dead_down`/`frozen`/`CenterEvent` 三项方向相关缺口按裁定 4A 上浮 #664，模块头 + 方法 doc 已大字声明 |
| MEDIUM-1 | MEDIUM | 幂等键是锚而非证明，矛盾证明被静默吞 | **已修** | `broken_by_certificate` 存整框，同锚不同框 ⟹ `Err(ConflictingCertificate)`（新变体替代 `KilledByOtherCause`；新测试 `conflicting_certificate_same_anchor_different_frame_fails_loud`） |
| MEDIUM-2 | MEDIUM | 端到端测试不端到端，手搓值掩盖 HIGH-3 | **已修** | 新增 `real_death_certificate` 辅助函数，经 `RetraceLedger::observe` 真实判胜落锤取真实证明；`death_certificate_registers_broken_end_to_end` 改用该路径 |
| MEDIUM-3 | MEDIUM | 「迟到证明」负控是空壳，`issued_as_of` 零引用 | **如实声明不修** | `issued_as_of` 本轮仍不参与判定（`CenterBook` 无可比对时钟坐标，未变）；"迟到"语义由裁定 1A「照登记」的行为本身表达，非另设时钟分支——已在方法 doc 明确声明这一设计选择；新测试 `late_issued_as_of_does_not_block_broken_when_anchor_still_present` 正面钉死该行为（小 `issued_as_of` + 锚仍在场 ⟹ Broken），不再是名不副实的「迟到测试」 |
| MEDIUM-4 | MEDIUM | 证明无级别信息，`ladder` 全凭调用方传，无校验 | **如实声明不修** | 本票范围未变——`ladder` 仍无 level↔ladder 映射校验，本轮未新增该校验（超出裁定 1A/2A/3A/4A 范围）；已在 §九.5 遗留风险重新登记，未被首轮报告漏记的问题延续 |
| LOW-1 | LOW | 旧 `KilledByOtherCause` 判定不校验价格边 | **随 HIGH-2 一并消解** | 变体已删除，判定次序改为「存档框命中→比较框；否则查 known」，不存在"已死分支缺边校验"这一形状 |
| LOW-2 | LOW | cert 杀不可观测：无查询接口、不产 `CenterEvent` | **未修，维持如实声明** | 不产 `CenterEvent` 与首轮一致（`CenterDeathCertificate` 无 side，构造不出 `Terminated{direction}`，见 #664）；本轮未新增查询接口，非本票裁定范围 |
| LOW-3 | LOW | `ladder >= MAX_LADDER` 裸索引 panic | **未修，维持如实声明** | 与 `ingest` 等既有代码同惯例，非本票新引入，本轮未改 |
| LOW-4 | LOW | 报告称 rustfmt 差异「全部落在本票改动之外」不实 | **已订正** | 见 §九.4：本轮独立复核，`rustfmt --check` 确认新增代码行同样被 stock rustfmt 标记差异（如 `known` 字段插入行），订正首轮的不实声明；结论仍是「本仓不以 stock rustfmt 为 gate，未强行重排版」 |

### 九.3 修正后的语义（实现摘要）

`consume_death_certificate(ladder, cert)`，`anchor = cert.center.start_index as i64`：

- **状态**：新增 `known: [Option<HashSet<i64>>; MAX_LADDER]`（`ingest` 凡见到带 `cs` 的事件即记录，kill 分支与 last 更新分支都记，插入点在两分支之前统一处理）；`broken_by_certificate` 类型从 `HashSet<i64>` 改为 `HashMap<i64, CenterFrame>`（anchor → 存档框）。
- **判定次序**：
  1. `broken_by_certificate` 命中 `anchor`：存档框全四边等于本证明 ⟹ `Ok(AlreadyBroken)` 零动作；不等 ⟹ `Err(ConflictingCertificate)`。
  2. `known` 不含 `anchor` ⟹ `Err(NoMatchingCenter)`（从未见过）。
  3. 否则登记：`dead` 插 `anchor`；`broken_by_certificate` 存档本证明的框；若 `pending_departure[ladder]` 的锚 == `anchor` 则清除该窗口。`is_dead(ladder, anchor)` 此前已为真（ingest 先杀）⟹ `Ok(AlreadyBroken)`（`version` 不再前进）；否则 ⟹ `Ok(Broken)`，`version += 1`。
- **错误枚举**：`NoMatchingCenter { ladder, anchor }`（从未见过）、`ConflictingCertificate { ladder, anchor }`（同锚不同框）。`KilledByOtherCause` 已删除。
- **`issued_as_of` 仍不消费**：`CenterBook` 无可比对时钟坐标，doc 如实声明；迟到语义由「不问迟到 = 被更替/迟到锚照登记」表达。

### 九.4 测试清单更新（`rust/src/trading/center_book.rs::tests`，本票新增 8 条，替换首轮 6 条）

| # | 测试名 | 覆盖点 |
|---|---|---|
| 1 | `death_certificate_registers_broken_end_to_end` | **真端到端**：`RetraceLedger::observe` 真实判胜落锤 → `death_certificate` 取真实证明 → 消费 → Broken（修 MEDIUM-2） |
| 2 | `no_matching_center_never_seen_fails_loud` | 从未见过的锚 ⟹ `NoMatchingCenter` |
| 3 | `superseded_anchor_certificate_still_registers_broken` | 被更替锚的证明 ⟹ `Broken`，新中枢不受影响（推翻首轮测试 6，修 HIGH-1） |
| 4 | `ingest_kill_then_certificate_is_idempotent_and_archives_frame` | ingest 先杀、证明后到 ⟹ `AlreadyBroken` + 框已存档 + version 不变（推翻首轮测试 4，修 HIGH-2） |
| 5 | `repeated_consumption_is_idempotent_zero_action` | 同一证明重复消费 ⟹ `AlreadyBroken` 零动作（首轮已有，行为不变） |
| 6 | `conflicting_certificate_same_anchor_different_frame_fails_loud` | 同锚不同框 ⟹ `ConflictingCertificate`（修 MEDIUM-1） |
| 7 | `cert_kill_clears_pending_departure_window_and_negate_is_noop` | candidate 窗口置位 → cert 杀 ⟹ `has_pending_departure` 转 false；后续 `negate_pending_departure` 不推 `up_strength`（修 HIGH-5 的时序演算） |
| 8 | `late_issued_as_of_does_not_block_broken_when_anchor_still_present` | 迟到证明（小 `issued_as_of`）+ 锚仍在场 ⟹ `Broken`（不问迟到正面钉死，修 MEDIUM-3） |

`rustfmt --check` 独立复核确认新增代码行（如 `known` 字段插入行、`consume_death_certificate` 体内多处）与既有代码行一样被 stock rustfmt 标记差异——订正首轮 §七「全部落在本票改动之外」的不实声明；结论不变：仓内无 `rustfmt.toml`，本仓不以 stock rustfmt 为 gate。

### 九.5 指纹对照更新

| | passed | failed | ignored |
|---|---|---|---|
| 首轮终跑（本轮开工前） | 2495 | 0 | 138 |
| **本轮终跑**（`cargo test --lib`） | **2497** | **0** | **138** |

- 增量 = +2：首轮 6 条测试 → 本轮 8 条（净增 2），逐一对应，无非预期红/绿变化。
- `cargo test --lib trading::center_book`：19 passed（首轮 11 条既有 + 本轮 8 条新增 = 19，逐一对应）。
- 0 failed、138 ignored 不动，与票面基线要求一致。

### 九.6 遗留风险更新（覆盖首轮 §八，按影子评审补全）

1. **level↔ladder 无映射**（影子 MEDIUM-4，本轮未修）：`CenterDeathCertificate` 无级别字段，`consume_death_certificate` 的 `ladder` 完全由调用方决定，函数内无校验；同一证明可被喂给两个不同 ladder 各自登记一次 Broken，无守卫。超出本轮四项裁定范围，留待后续票。
2. **cert vs ingest 框不可比**：`broken_by_certificate` 存档的 `CenterFrame`（Tick 量纲）与 `LiveCenter` 的价格边（原始 f64）之间仍无换算——裁定 2A 明确禁止这类跨量纲比较，本轮不引入 `tick_size` 之类的换算参数，如实声明这是"本票范围内不做"，非遗漏。
3. **方向盲区 → #664（机制）**：`CenterDeathCertificate` 无 `side`，cert 杀路径不进 `dead_down`、不置 `frozen`、不发 `CenterEvent::Terminated`——已上浮至 #664（blocked_by #637，已立票），本轮只做 doc 声明，不做代码修复。**机制补登记（尾部评审 MEDIUM-1）**：这三项缺口不是「本次不置」，而是「此后由 `ingest` 通道永不补」——`ingest` 的三项收尾动作（`dead_down` 插入、`version` 前进、`CenterEvent::Terminated` 发出）全部在 `if !dead.contains(&cs)` 守卫（`:147`）之内；`consume_death_certificate` 会无条件把 `anchor` 插入 `dead`，之后若 `ingest` 再收到同一锚的 confirmed Type3，整块 kill 逻辑都被该守卫跳过，不会补发。**若 #664 的实施方选择「cert 路径不动、让 `ingest` 事后补方向」这种修法，会被该守卫静默吞掉且无任何测试拦截**——#664 的方案必须绕开或移除该守卫，不能依赖 `ingest` 事后补齐。
4. **跨引擎数值判同——风险方向已翻转（恒不命中 → 可误命中 ⟹ 误杀在场中枢）**：判同现在只是锚（`start_index`）的名义映射，`start_index`/`seg_start` 分属两套引擎各自切分的段序列，无对齐依据。删除跨量纲价格边比较（裁定 2A）后，两套引擎段索引的**整数相等**成为判同的全部依据——风险方向已从首轮的「恒不命中（静默失效）」翻转为「**可数值巧合命中（静默误杀在场中枢）**」：一张锚定 theta_v0 段索引的死亡证明，若撞上 legacy 引擎本层历史上曾出现过的相同整数 `cs`（且该中枢恰为当前 `last` 且未死），会被判同成功 ⟹ `dead.insert` + `version += 1` + `Ok(Broken)` ⟹ `alive()` 转 `None` ⟹ `SizeAllocator::maybe_recompute` 当场把该层仓位配额剔除。今日 `consume_death_certificate` 零生产调用点（grep 实证，仅测试内引用），故不可触发，是纯潜伏风险；若未来 #575 剩余部分把两引擎接同一数据管线、或任何生产路径开始喂 cert，需要重新核实这套锚映射在真实数据下是否成立，并按上述误杀方向（而非「恒不命中」）评估。
5. `ladder >= MAX_LADDER` 裸索引 panic（LOW-3，与既有代码同惯例，未修）；`consume_death_certificate` 不产 `CenterEvent`（`Terminated` variant 需要 `direction`，无 `side` 构造不出，属 #664 方向缺口的一部分）、且无独立查询接口区分某中枢是 cert 杀还是 ingest 杀（LOW-2 后半，与方向缺口相关但非 #664 明列范围，本轮未加、留待后续票视需要补）。

全程未提交（worktree `/private/tmp/wt-637`，未 commit/push/reset）。

## 一、CenterBook vs CenterEventMachine 核查结论

开工先核票面点名的"当前权威实现"问题：仓内同时存在两套"中枢死亡/Broken"语义的载体——

| | `trading::center_book::CenterBook` | `theta_v0::classifier::center_lifecycle::CenterEventMachine` |
|---|---|---|
| 数据源 | legacy Python-parity 信号流的 `BspEvent`（`cs`/`zd: f64`/`zg: f64`），per-ladder 自 diff 派生 `CenterEvent` | theta_v0 塔链 `levels[level].centers`（R3 单一真相源，#336 裁定），链游标即在场中枢 |
| 生产接线 | **是**——`trading/runner.rs`、`level_operating_unit.rs`、`allocator.rs` 等 18 处生产文件 `CenterBook::new()` 直接持有实例（grep 实证，非仅测试） | **是**——theta_v0 classifier 主线自身的中枢生死账，`CenterLifecycleEvent::Broken`/`Superseded` 是塔链下游消费的权威事件 |
| Broken 语义 | 本票新增前：无独立"Broken"概念，只有 `dead`/`dead_down`/`frozen` 三态标记 | 本级三类点破坏 ⟹ `CenterLifecycleEvent::Broken`（教义死亡），与"在场终结/Superseded"并列两种死亡登记 |
| 与 `retrace_ledger` 的关系 | 票面 #574 裁定一"发中枢账登记 Broken"点名的**消费目标**（登记表 #1 行原文即写明 `crate::trading::center_book::CenterBook`） | 与 `retrace_ledger` **同属 theta_v0 命名空间**但结构独立——`CenterEventMachine` 自己的 Broken 判据是"本级确认三类点+载体为在场实例"，不读 `RetraceLedger`；`retrace_ledger` 是另一套（S1/S2 票，#621/#622）独立的候选判案账本，两者当前互不调用 |

**结论**：`CenterBook` 与 `CenterEventMachine` 是**两条独立、都在生产路径上运行的中枢生死账**，服务不同的下游（交易层 vs theta_v0 分类器主线），彼此不耦合，本票不涉及、不改动 `CenterEventMachine`。接线目标遵票面与登记表原文 = `trading::CenterBook`，未偏离。`CenterBook` 的既有 `ingest()` 自诊断派生路径（doc 称"唯一 diff 点"）与本票新增的 `consume_death_certificate()` 是**两个并存的死亡登记入口**（前者诊断自派生，后者外部证明消费），非替代关系——已在登记表更新行与新方法 doc 中显式声明，避免"唯一 diff 点"表述被误读为本票违反。

## 二、判同映射设计取舍

`CenterDeathCertificate { center: CenterFrame{ zd: Tick(i64), zg: Tick(i64), start_index: usize, end_index: usize }, issued_as_of: usize }` vs `CenterBook::LiveCenter { seg_start: i64, zd: f64, zg: f64 }`。

- **grep 核实**：仓内无既有 theta_v0 中枢框 ↔ trading 中枢账换算/对照表（`crate::theta_v0` 在 `trading/*.rs` 零命中，`crate::trading` 在 `theta_v0/**/*.rs` 仅 `retrace_ledger/mod.rs` 的两行**文档注释**提及、无实际 `use`）。两引擎数值基座本不同源：`CenterBook.zd/zg: f64` 直接来自 legacy Python-parity 引擎（`lib.rs`/`spiral/engine.rs`/`fugue_v3/engine.rs` 的原始浮点信号），`CenterFrame.zd/zg: Tick` 来自 theta_v0 的 `quantize()` 后整数刻度；本票是**首次**建立两者之间的显式映射，非复用既有换算。
- **锚点映射**：`start_index (usize) ↔ seg_start (i64)`，取"同角色字段直等"（`cert.center.start_index as i64 == live.seg_start`）。依据：theta_v0 自身把 `CenterFrame::anchor() = CenterAnchor(start_index)` 定为该证明所属引擎的**唯一路由锚**（`retrace_ledger/mod.rs:146-149`），与 `CenterBook` 用 `seg_start` 做 per-ladder 中枢身份键（`dead`/`is_dead`/`last` 全部以 `seg_start` 为键）在"字段角色"上对应——都是"该中枢起点标识"。这是结构对应，不是数值同源保证。
- **价格边映射**：`zd`/`zg` 各自 `Tick as f64` 后与 `LiveCenter.zd/zg` 做**精确相等**比较（不做容差）。理由：仓内既有代码对 `f64` 中枢边界的比较（`ingest()` 里 `Some(lc.zd) != ev.zd`）本就是精确相等语义（Python parity 要求），本票延续同一惯例，不新引入容差判据（容差需要 `tick_size` 才能定义，`CenterBook` 当前不持有该参数——引入即扩大范围，票面禁"顺手重构"）。
- **四条边→三条边的结构性缺口（如实登记，非遗漏）**：`CenterFrame` 有四条边（`zd`/`zg`/`start_index`/`end_index`），`LiveCenter` 只存三个可比对字段——**无 `end_index`（临时右边/落锤右边）对应字段**。`end_index` 结构性不参与判同；已在 `consume_death_certificate` doc 注释中明确声明这一缺口，不是静默丢弃。若未来两引擎真正接同一数据管线（本票范围外），`end_index` 的落位需另案设计（`CenterBook` 若要收窄"当前存活中枢"以外的历史右边信息，需要扩展 `LiveCenter` 或另建索引，是比本票大得多的改动）。
- **由于零production调用点**：`RetraceLedger` 目前仓内**零生产调用点**（只被测试群消费，模块 doc 自述"生产接线点计数 1/3"——即本票这一条），意味着真实 `CenterDeathCertificate` 在生产磁带上尚不存在；上述映射目前只有结构意义（类型层面可编译、测试可构造），要到 #575 剩余 2/3（RetraceLedger 本身接入生产驱动 + 交易层消费 `ThirdPointPack`）落地后才会有真实跨引擎数值流过这条判同路径。本票范围内如实声明这一点，不夸大"接通"的实际效果。

## 三、fail-loud 错误形状

```rust
pub enum DeathCertificateOutcome { Broken, AlreadyBroken }

pub enum DeathCertificateError {
    NoMatchingCenter { ladder: usize, anchor: i64 },
    KilledByOtherCause { ladder: usize, anchor: i64 },
}

pub fn consume_death_certificate(
    &mut self, ladder: usize, cert: &CenterDeathCertificate,
) -> Result<DeathCertificateOutcome, DeathCertificateError>
```

- 判同失败（无此中枢/已被新中枢取代/迟到）与"该锚已死于别的路径"两类都是 `Result::Err`——不静默吞掉、不返回 `bool`/`Option` 掩盖分歧。调用方必须显式处理（Rust `#[must_use]` 由 `Result` 自带保证不会被静默丢弃）。
- 未采用 `panic!`：`CenterBook` 是多处生产文件共享的账本类型，consume 路径的失败是"证明与本账当前状态不符"这一**可能合法**的运行时状况（例如 #575 尚未把交易层接成同一数据源前，证明与账天然可能不同步），panic 会把这种可恢复分歧升级为进程崩溃，与仓内既有错误处理风格（`RetraceRejection`/`SnapshotRejection` 皆为返回值，非 panic）不一致。

## 四、幂等与迟到判据

- **幂等**：新增 `broken_by_certificate: [Option<HashSet<i64>>; MAX_LADDER]`，与既有 `dead` 集合分离维护——`dead` 记录"是否已死"（不分死因），`broken_by_certificate` 记录"是否已被**本消费路径**登记死"。判据顺序：先查 `broken_by_certificate`（命中⟹零动作 `Ok(AlreadyBroken)`，不重复插入、`version` 不再前进）；未命中再查 `is_dead`（命中⟹别的路径杀的，`Err(KilledByOtherCause)`，不允许证明"补登记"掩盖两条路径都想杀同一中枢这一分歧）；两者都未命中才做判同匹配。
- **迟到证明**：`CenterDeathCertificate` 本身不携带足以判断"本账当前态"的时钟字段可比对（`issued_as_of` 是 theta_v0 侧的知情时坐标，`CenterBook` 当前不追踪任何 bar/时钟坐标——`ingest()` 本身也不接收外部 bar index，只有内部 `version` 单调计数器）。故"迟到"没有独立的时钟比较分支，按**票面语义**折算为：迟到证明消费时点，`CenterBook` 状态可能已推进（旧锚被新中枢取代 / 已死于他因）——这两种情形都落在既有的 `NoMatchingCenter`/`KilledByOtherCause` 分支内，不新增第三种错误变体。已用专门测试（`late_certificate_after_center_superseded_fails_loud`）覆盖"证明锚定的中枢已被新中枢取代"这一叙事场景，验证行为是 fail-loud 而非误伤新中枢。**如实声明**：这是把"迟到"归约到已有判同失败机制的设计选择，不是给 `CenterBook` 新增时钟基础设施（那将是远超本票范围的改动，且 `ingest()` 调用链当前无 bar index 可传，牵动所有 18 处生产调用点）。

## 五、测试清单（`rust/src/trading/center_book.rs::tests`，全部新增，6 条）

1. `death_certificate_registers_broken_end_to_end` —— 端到端：`ingest` 产中枢 → 构造匹配证明 → `consume_death_certificate` → `Ok(Broken)` + `is_dead`=true + `alive`=None + `version` 前进。
2. `no_matching_center_absent_ladder_fails_loud` —— 负控①：该层从未见过中枢 ⟹ `Err(NoMatchingCenter)`。
3. `no_matching_center_edge_mismatch_fails_loud` —— 负控①变体：起点判同但价格边不符 ⟹ `Err(NoMatchingCenter)`。
4. `killed_by_other_cause_fails_loud_not_silent` —— 负控①变体：中枢已被 `ingest` 自身的 confirmed Type3 杀死（非本证明所杀）⟹ `Err(KilledByOtherCause)`，不静默。
5. `repeated_consumption_is_idempotent_zero_action` —— 负控②：同一证明消费两次 ⟹ 第二次 `Ok(AlreadyBroken)`，`version` 不再前进。
6. `late_certificate_after_center_superseded_fails_loud` —— 负控③：旧中枢已被新中枢取代后消费指向旧中枢的证明 ⟹ `Err(NoMatchingCenter)`，且不误伤新中枢（`alive` 仍存在）。

## 六、全量指纹对照

| | passed | failed | ignored |
|---|---|---|---|
| 文档基线（票面标称） | 2450 | 0 | 138 |
| 本次开工前实测（clean, `cargo test --lib`） | 2488 | **1**（flaky） | 138 |
| 本次开工前实测（单独重跑该失败用例） | — | 0（通过） | — |
| 终跑（本票改动后，`cargo test --lib`） | **2495** | 0 | 138 |

- 文档基线 2450/0/138 与实测不符（如实记录，未强行对齐）：main 自基线记录后经多轮 merge（#630/#631/#633/#635 等），测试总数增长符合预期，非本票引入。
- 开工前实测出现的 1 个失败 `theta_v0::classifier::tests::incremental_tower_scaling_dominates_full_synthetic` 是**已知计时类 flaky 测试**（标度比断言依赖 wall-clock，单独重跑/终跑时都通过）——与本票无关，未修改该测试或其依赖代码。
- 终跑 2495 passed = 开工前总用例数 2489（2488 passed + 1 flaky-failed）+ 本票新增 6 条测试，0 failed，138 ignored 不变——增量与本票新增测试数**逐一对应**，无非预期红/绿变化。

## 七、Standards 自查

- 改动仅 2 文件：`rust/src/trading/center_book.rs`（+168/-0，新增字段/方法/枚举/测试）、`rust/src/theta_v0/classifier/retrace_ledger/mod.rs`（消费方登记表 2 处文字更新，+4/-3）。未触碰买卖点账本侧（`retrace_ledger` 内部逻辑零改动，只改了登记表文字），符合票面"账本不进口外部状态"禁区。
- 未做无关重构；新增代码风格（中文 doc 注释、错误枚举命名、`Result` 返回、`HashSet` 惰性初始化模式）与 `center_book.rs` 既有代码逐一比对一致（`get_or_insert_with(HashSet::new)` 是文件内既有惯用法，直接复用）。
- `rustfmt --check` 对本文件报出的差异**全部落在本票改动之外的既有代码行**（如 `is_dead`/`negate_pending_departure` 等预先存在的单行 vs 折行差异）；仓内无 `rustfmt.toml`，且既有代码本就不满足 stock rustfmt 默认宽度，判定本仓不以 stock rustfmt 为 gate，未对无关代码重排版（避免扩大 diff）。
- `cargo build --lib` / `cargo test --lib` 均只产出**仓内既有的**大量 `dead_code`/`unused_imports` 类 warning（与本票改动文件无关，逐一核对确认未新增于 `center_book.rs`/`retrace_ledger/mod.rs` 之外）；本票新增代码无编译警告。
- worktree 内全程未执行任何 git mutation（未 commit/push/reset/checkout），改动以未提交形态留存，交由编排方统一处理。

## 八、遗留风险 / 未裁项

1. **跨引擎映射目前只有结构意义**：`start_index ↔ seg_start`、`Tick→f64` 精确比较的映射，在 `RetraceLedger` 仍是零生产调用点的现状下没有真实数值流验证；若 #575 剩余 2/3（RetraceLedger 生产驱动接线、交易层 `ThirdPointPack` 消费）落地后两引擎真正跑同一磁带，需要回头核实这套映射在真实数据下是否成立（尤其 `Tick` 量化精度 vs 原始 f64 精确相等比较是否会出现"本应判同但因量化舍入不等"的假阴性）——本票未观测到、也无法观测到这类假阴性（无生产数据可测）,如实标注为待验证项。
2. **`end_index` 结构性不参与判同**：若后续需要更严格的"四条边全等"判同（例如同一 `start_index`/`zd`/`zg` 但不同 `end_index` 的两次证明需要区分），当前实现无法分辨——`LiveCenter` 需要扩字段才能支持，本票判定为超出票面范围（票面只要求"对锚定中枢登记 Broken"，未要求区分同锚不同右边的证明）。
3. **`consume_death_certificate` 不产出 `CenterEvent`**：`CenterDeathCertificate` 不携带 `side`/`direction`，无法构造 `CenterEvent::Terminated{direction}`（该 variant 要求 direction 字段），故本次消费只更新内部 `dead`/`broken_by_certificate`/`version` 状态，不经 `events_out` 通道广播事件。若后续消费方（如 #575 的交易层 2/3、3/3）需要感知这次 Broken 登记，需要另设读口（如新增 `is_broken_by_certificate(ladder, seg_start) -> bool` 查询方法）——本票未添加，因票面验收未点名"需要可观测事件流"，避免过度设计；如后续票需要，属于新范围。
4. **`CenterEventMachine` 与 `CenterBook` 的双 Broken 语义并存**：本票核查确认二者独立生产运行、互不调用，但这意味着仓内现在有**两套**"Broken"概念（`CenterLifecycleEvent::Broken` vs `CenterBook.is_dead()` via `consume_death_certificate`），语义相近但载体、生产路径、驱动源完全不同。是否需要长期统一（或明确二者永久分工）不在本票范围，留给后续架构裁决。
