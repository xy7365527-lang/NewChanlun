# #465 前置事实调研：`nest_lifecycle` 对买卖点生命周期的可复用性

> 日期：2026-07-28
> 性质：只读事实调研；不裁定“抽通用账本 / 复用现有模块 / 另建第三套账本”
> 代码锚：收口时 HEAD `77d355433274545b991d12c3ec24dc468fde35ad`
> （分支 `kimi-nest-mainline-20260717`）
> 行号纪律：本报告所有 Rust 行号均指上述提交的 blob，不指工作树。调研开始时 HEAD 为
> `558104a0ffd71ae6e58cd7e593986a81f384f187`；并行 #559 线在调研中把 HEAD 推进到
> `77d35543…`，本报告已按新提交重读受影响的状态/原因码/生产消费面。新 HEAD 的
> `nest_lifecycle.rs` 为 4,228 个 `wc -l` 行；`first_retrace_replay.rs` 未随该提交改变，
> 仍为 572 个 `wc -l` 行（题面“571 行”可视为末行/换行计数差异）。

## Q1 通用性清单与结论

### 1. 先给事实结论

`nest_lifecycle` 不是一个以泛型 key 驱动的对象无关状态机。它是“可抽出的账本内核”与
“Pan/Trend 活假设业务策略”写在同一模块、同一 `advance` 中的具体实现。

按下面 §2 的 **20 个账本内核责任点**计数，而非按全文件 LOC 计数：

- **11/20（55%）对象无关**：per-key 注册、首次建项、修订追加、修订计数、倒退拒绝、
  终态吸收、钟首次写入、增量返回、迁移时保留历史、只读枚举、通用不变量骨架。
- **9/20（45%）绑死活假设域**：六元组 key、右端可动的 `bridge_identity`、三态的具体
  成立/否证条件、三类失效原因及消失成因两分、修订词汇、五钟语义、PanLive/Event 输入、MACD 力度核、
  结构完成/身份消失的业务结算。

这里的 55%/45% 只回答“状态机/修订链的责任是否对象无关”。如果把 L1 active frontier、
Pan provider、完成事件、力度证据、统计/审计载荷也纳入整个生产模块，假设域代码明显占多数；
用全文件 LOC 给复用率会被约 2,300 行测试和 provider 层扭曲，故不这样报数。

**可直接从代码确认的约束**：当前实现不能“只把 key 类型替换为买卖点身份”就复用。`LifecycleKey`
不仅出现在 `BTreeMap` 键上，还进入 `LifecycleRevision`、`NestLifecycleEntry`、完成信号、审计、
桥迁移、统计输出及 provider 输入；`advance` 又直接调用活假设的力度与结构完成逻辑。要让另一对象
使用这 55% 内核，至少需要把 key、观察、转移策略、原因/修订载荷从当前具体类型中分离。是否这样做
以及是否值得做，不在本报告裁定范围。

### 2. 20 个责任点的计数底稿

| # | 责任点 | 对象无关 / 假设域 | HEAD 代码锚 | 事实 |
|---:|---|---|---|---|
| 1 | per-key 注册表 | 对象无关 | `nest_lifecycle.rs:701-710` | `BTreeMap<K, Entry>` 形状通用，但当前 `K` 写死为 `LifecycleKey` |
| 2 | 首次观察建项 | 对象无关 | `:889-917` | 空键建 `Provisional` 并追加 `Observed` |
| 3 | append-only 修订追加 | 对象无关 | `:369-387` | `push_revision` 只追加，返回同一条 delta |
| 4 | 修订计数与历史一致 | 对象无关 | `:338-339, 1076-1082` | `revision == revisions.len()` |
| 5 | per-identity 倒退拒绝 | 对象无关 | `:861-870, 920-930` | `as_of < last_as_of` 时零修订、零状态改写 |
| 6 | 终态吸收 | 对象无关 | `:206-211, 872-875, 931-935` | 终态同 key/桥 key 后续输入均零输出 |
| 7 | 钟首次写入/不后移 | 对象无关 | `:330-367, 939-944, 988-1005` | “首次钟不改写”是通用账本纪律 |
| 8 | 每次推进返回增量 | 对象无关 | `:846-852, 1061-1066` | 返回新修订，不要求消费方重扫全账 |
| 9 | 身份迁移保留历史 | 对象无关 | `:876-887` | remove 旧键、改新键、钟/旧修订保留并追加迁移修订 |
| 10 | 只读枚举/过滤门户 | 对象无关 | `:712-748, 817-830` | `entries`、终态过滤、谱系枚举的机制通用；具体“只放 Confirmed”是域策略 |
| 11 | 不变量骨架 | 对象无关 | `:1068-1224` | 修订数、钟单调、终态互斥等可泛化 |
| 12 | 身份六元组 | 假设域 | `:108-155` | `level/side/kind/seg_a/seg_c_full/b_center_start` 全是 N 假设结构字段 |
| 13 | 白名单桥判同 | 假设域 | `:157-187` | `bridge_identity` 只允许 `seg_c_full.1` 变化；#559 又加同锚不同 C 的 `same_anchor` |
| 14 | 三态的业务含义 | 假设域 | `:193-211` | `Confirmed` 要求首次可证、C 完成、完成时仍弱；不是抽象“确认” |
| 15 | 失效原因 | 假设域 | `:213-253` | `ForceOvertake/NeverConstituted` 属背驰假设；消失又分真推翻与 provider 接缝 |
| 16 | 修订词汇 | 假设域 | `:293-315` | `FirstProvable/StructureCompleted/ForceUnavailable` 等直接编码假设域 |
| 17 | 五钟语义 | 假设域 | `:330-367` | `first_provable_at/structure_end_at` 不是任意对象都有的钟 |
| 18 | 观察与 provider 输入 | 假设域 | `:417-560, 1286-1830` | `NestCandidateEvent/PanLiveWindow`、L1 frontier、两相 feed 全是 N/Pan 结构 |
| 19 | 力度判定与证据 | 假设域 | `:255-287, 435-586, 937-987` | 直接调用 hist/dif/MACD 背驰原语并保存六个力度量 |
| 20 | 完成/消失结算策略 | 假设域 | `:988-1060` | 完成时二分；消失时按 `same_anchor` 再分 `HypothesisRefuted/ObservationSeam` |

### 3. 换成“买卖点身份”时，逐函数/逐段可复用边界

| 代码段 | 原样通用程度 | 换 key 后的事实 |
|---|---|---|
| `NestEventState::is_terminal`（`:206-211`） | 逻辑可原样 | 前提是买卖点也接受“两种终态均吸收”；状态名和进入条件仍需另定 |
| `NestLifecycleEntry::push_revision`（`:369-387`） | 算法可原样 | 函数签名和载荷类型必须泛化，当前写死 `LifecycleRevisionKind/LifecycleKey/ForceEvidence` |
| `invalidate`（`:389-410`） | 算法可原样 | 原因、证据、终态钟及 `VanishCause` 投影需要替换 |
| `NestLifecycleBook::{new,len,is_empty,get,entries}`（`:712-748`） | 算法可原样 | 容器/参数/返回值的具体类型必须替换 |
| `settlement_stats`（`:750-813`） | 遍历骨架可用 | 统计桶按 N 的原因、消失两分和 Force 寿命命名，不能原样作为买卖点统计 |
| `consumable_closed`（`:817-823`） | 过滤机制可用 | “仅 Confirmed 可消费”是 N 当前策略；买卖点何态可消费未在 `first_retrace` 定义 |
| `lineage_nodes`（`:829-830`） | 枚举机制可用 | 其禁止进入证书真值路径的红线是 N 当前接线约束 |
| `advance` 第 1 段建项（`:857-918`） | 骨架可用 | `bridge_match` 与 `Supersedes` 的含义必须替换 |
| `advance` 第 2/3 段（`:920-935`） | 可原样抽出 | 倒退守卫与终态吸收不依赖 Pan/力度 |
| `advance` 第 4-7 段（`:937-1029`） | 不可原样 | 全部直接编码力度首次可证、反超、不可验、C 完成和两种假设终局 |
| `advance` 第 8 段（`:1031-1060`） | 扫描算法可抽 | 当前还按同锚判断真推翻/接缝；买卖点是否适用，仓内没有答案 |
| `assert_invariants`（`:1068-1224`） | 前半骨架可用 | revision/钟/终态通用；Force、消失成因、完成信号等断言专属 |
| `bridge_match`（`:1267-1283`） | 搜索骨架可用 | 实际相等关系由 N 六元组与 C 右端规则决定 |
| provider/feed 层（`:1286-1830`） | 不可原样 | 输入、定位、阶段、完成信号都属 PanLive/Nest；只能借“两相输入”这一模式 |

买卖点域当前已经明确的身份只有
`RetraceIdentity { center: CoordinateWindow, departure_move_index }`
（`first_retrace_replay.rs:88-92`）。它没有给出与 `bridge_identity` 同等的判同关系、`as_of` 钟、
身份消失判据、原因/证据载荷或恢复持久化协议。因此“替 key 后剩余泛型参数是什么”在当前仓内
尚未被定义完。

### 4. #421 接线后的真实生产消费面

HEAD 全仓调用面：

- `p123_fast_replay` 创建并逐 bar 推进 `NestLifecycleBook`
  （`p123_fast_replay.rs:1040, 1412-1457`）。L1 活窗来自 parser active frontier；完成相从
  已完成 lower unit 组装 typed `PanCompletionEvent`（`:1592-1665, 1772-1817`）。
- `feed_lifecycle_bar` 读取并落审计的字段是：
  - `CompletionSignal` 的 `as_of`、完整 `LifecycleKey`、lower id、`completed_at`；
  - 每条 revision 的 `as_of`、完整 key、`kind`、`evidence`；
  - 完成时力度不可验 audit 的 key/reason；
  - 末尾 `settlement_stats` 的状态、原因和寿命分布
    （`:1820-1902, 1490-1513`）。
- `p409_pan_live_probe` 是诊断/反事实调用面；它读 entry 的状态、钟、原因、修订数等，不是
  订单/证书消费面。
- 全仓外部调用搜索显示，`consumable_closed()` 与 `lineage_nodes()` **没有生产调用点**，
  只在 `nest_lifecycle.rs` 自身测试出现。当前生产真正“消费”的是 sidecar dump、stderr 统计和
  `assert_invariants()`，不是 N 装配、证书或订单路径。

所以，“#421 已接生产”这一前提已满足的是 **生产进料、修订留档、统计与审计可见**；它并不等于
`NestLifecycleEntry` 已成为交易真值或装配输入。

### 5. 修订链在真实生产中的触发形态

按报告时间线分开，避免把反事实与生产混算：

1. **#409 反事实探针**：
   `panlive-frequency-probe-20260727.md:15, 120-127` 在 OKLO 100k 看到 213 个活假设，
   95 个曾 first-provable，88 个最终 ForceOvertake，即 **92.6%**。同报告 `:68` 明写
   `structure_completed=false`，测不出 Confirmed。这条后来被 #523 定位为
   post-completion holding counterfactual，不能当生产生命周期终局分布。
2. **#523 首见即完成根因**：
   `issue421-structure-completion-root-cause-20260728.md:21-29, 102-107` 对 BTC 100k 的
   248 个身份全量 join，得到
   `p123.observed_at = p409.observed_at = completion_as_of`，**248/248，差值 0/0**。
   旧 provider 在生产完成首见 bar 才开始“活窗”，因此旧生产链大量为同 bar
   `Observed → (FirstProvable) → StructureCompleted → terminal`。
3. **#527 L1 active provider**：
   `issue527-panlive-l1-provider-20260728.md:10-16, 69-97` 报告 248 个完成身份中
   **156 个 L1 身份在完成前可见**，提前量 min/median/max = `16/104.5/411` bar；
   生产首次出现 **35 条**
   `Observed → FirstProvable → Invalidated{ForceOvertake}`，闪现从 `248/248`
   降到 `92/284 = 32.39%`。完成信号仍为 248，零丢弃、零新增。
4. **#559 影子复核与同票修复轮**：
   `shadow-527-review-20260728.md:13-31, 77-98, 322-325` 独立复现 156、35 和
   true-flash=0，故“L1 完成前可见”成立；同时指出原报告的 46 个 L1 缺口归因里有 11 个
   未落原因码、`observed_completion_at` 在生产 248/248 恒等于 `as_of`。收口 HEAD
   `77d35543…` 已落实 #559 修复：完成钟从三分删为
   `completed_at/as_of` 两分；`IdentityVanished` 的 37 个终局分成
   `HypothesisRefuted=25` 与 `ObservationSeam=12`；entries、35 条 ForceOvertake、
   闪现/非闪现读数不变
   （`issue527-panlive-l1-provider-20260728.md:333-384, 434-477`）。

这四组事实说明修订链的 append、跨 bar 存活、ForceOvertake 终局不是只存在于单测；但它们出现
所依赖的 observation、bridge、力度和 completion 全是 N/Pan provider 语义。

## Q2 三桶分解

### 1. 571/572 行的结构分解

| HEAD 行段 | 行数 | 内容 | 生产性 |
|---|---:|---|---|
| `1-5` | 5 | 模块边界：D7 只读复核，不产信号、不改 C2 seam | 声明 |
| `7-86` | 80 | D1 seed → 唯一 CompletedMove；两 move 必须不同、正向严格相邻 | 买卖点/回试输入原语 |
| `88-104` | 17 | `RetraceIdentity/Outcome/Observation` | 买卖点域模型 |
| `106-131` | 26 | 四种 lifecycle event 与两个 replay error | 买卖点域协议 |
| `133-183` | 51 | `replay_first_retrace` 瞬态自动机 | 只读 replay |
| `185-572` | 388 | fixtures 与单测 | 测试 |

模块在 `classifier/mod.rs` 注册，但 HEAD 全仓引用只出现在本文件及其测试，**零生产调用点**。
它没有 book/map、没有持久 entry、没有 `as_of`、没有修订计数/留档、没有恢复/reopen，也没有
终态状态枚举；每次调用只从传入 observations 重新产生一个 `Vec<RetraceLifecycleEvent>`。

### 2. 严格相邻 CompletedMove pair

`strict_completed_pair`（`:31-56`）和 `unique_completed_move`（`:58-86`）执行：

1. 按 `ElementId` 在 `ExactThreeProjection.seeds` 找唯一 seed；找不到报 `MissingSource`。
2. 在 `LevelAsOfView.moves` 中找包含该 seed 且 `CompletionStatus::Completed` 的 move；
   0 个报 `NoCompletedMove`，多个报 `AmbiguousCompletedMove`。
3. leave/retest 不得落同一 move；retest 必须精确等于 `leave + 1`，否则 fail closed。

这一段没有 `nest_lifecycle` 同构物。`nest_lifecycle` 接收 provider 已组装好的 observation，不负责
D1 seed 到 C2 CompletedMove 的唯一/相邻证明。

### 3. 三个题面事件的实际语义

- `RetestReenters`：当前 `(center, departure CompletedMove)` 身份的第一次严格回试失败并重新
  进入中枢；这次观察立即把该身份的 `firstRetrace` 资格置为已消费，并允许下一次 **不同
  departure** 触发 restart（`:133-143, 161-171`）。
- `Supersede`：仅当旧身份已 `RetestReenters`、下一 observation 的 leave move 改变时发出；
  事件指向旧 `RetraceIdentity`（`:146-158`）。它不是“同一 key 的窗口右端修订”。
- `Restart`：紧跟上述 `Supersede(old)`，用相同 center、新
  `departure_move_index` 建新身份并把 `consumed=false`（`:153-160`）。
- `Success`：消费当前身份第一次严格回试并关闭 restart 许可（`:164-179`）。
- 同 departure 在 `RetestReenters` 后晚到 `Success` 会报
  `FirstRetraceConsumed`；未获得 restart 许可却换 departure，会报
  `PairDepartureMismatch`。两者都不是静默容错。

### 4. 与 `nest_lifecycle` 的逐件同构/差异

| 维度 | `first_retrace_replay` | `nest_lifecycle` | 关系 |
|---|---|---|---|
| 身份 | `(center window, departure move index)` | 六元组 N 假设 key | 都是显式复合身份；字段无同构 |
| 当前对象 | 局部变量 `active` | `BTreeMap` 多身份并存 | 单活跃自动机 vs 持久多对象账本 |
| 未决态 | `consumed=false` | `Provisional` | 只在“尚可消费”概念上相近 |
| 成功终态 | `Success` event，无存储态 | `Confirmed` entry | 均吸收当前身份；证明条件完全不同 |
| 失败终态 | `RetestReenters` 消耗 first chance | `Invalidated{reason}` | 均阻止同身份晚成功；原因域不同 |
| 后续同身份输入 | `FirstRetraceConsumed` error | 终态零输出吸收 | 一个报错，一个幂等空 |
| Supersede | 旧身份退位，另建新 departure 身份 | 同一假设桥迁移，保留钟和修订链 | 名同义不同 |
| Restart | 显式新身份事件，可再消费一次 | 无 Restart；终态禁止复活 | `nest_lifecycle` 不提供对应语义 |
| 历史 | 本次调用返回事件 Vec | entry 内 append-only revisions | 都能产事件；只有后者持久留档 |
| 时间 | 只有 move index，无 `as_of` | 五钟 + `last_as_of` | 无同构 |
| 输入完整性 | 自证 unique、Completed、adjacent | 信任 provider observation | first-retrace 独有 |
| 业务判据 | 回试是否重入/成功 | N 假设力度、完成、消失 | 空集 |

### 5. 三桶

#### (a) 与 `nest_lifecycle` 重复的部分（若存在通用内核，可由内核承载）

以下是**机制重复**，不是当前代码可直接删除的结论：

- 按显式身份持有当前对象；
- 第一次观察后消费资格改变；
- 成功/失败后同身份不得静默再成功；
- 旧身份被替代时产一条可审计事件；
- 从输入序列确定性地产生有序 delta；
- “当前状态 + 事件 → 新状态/拒绝”的小型状态机骨架。

这些对应 Q1 的 per-key 状态、终态吸收、append delta、Supersede 留痕。当前
`nest_lifecycle` 的具体 API 因 key、revision、reason、observation 全写死，不能原样接住
`first_retrace`；“重复”只成立在可抽账本协议层。

#### (b) 买卖点/首次回试特有、不能随账本骨架丢掉的部分

- D1 seed → 唯一 CompletedMove 映射及四类 fail-closed 错误；
- leave/retest 必须是不同、正向严格相邻的 CompletedMove；
- `RetraceIdentity` 的 center + departure move 身份；
- 每个身份只消费第一次严格回试；
- `RetestReenters` 后，同 departure 晚成功明确拒绝；
- 只有旧身份先重入，**不同 departure** 才可
  `Supersede(old) → Restart(new)`；
- `RetestReenters` 与 `Success` 携带实际 retest move index；
- 错 departure 与重复消费是错误，不是 `IdentityVanished` 或幂等空。

#### (c) 当前仓内说不清

- `CoordinateWindow.end` 延展时是否仍是同一个买卖点/中枢身份；`first_retrace` 没有自己的
  bridge 规则。
- replay observations 是否保证按 bar/`as_of` 单调；函数只有切片顺序，没有时间守卫。
- 跨进程/跨窗口如何恢复 `active/consumed/restart_allowed`；没有持久化协议。
- 同一 center 同时存在多个 departure 身份时，是只允许一个 active，还是多身份并存。
- observation 暂时缺席是否代表身份消失；当前函数没有“本 prefix 未见”概念。
- `RetestReenters` 应映射为通用 `Invalidated`、独立终态还是仅一个原因事件；代码只给事件，
  没给共享状态定义。
- `Success` 后若结构重算出现新 departure，是否允许新一轮；当前逻辑会因
  `restart_allowed=false` 报错。
- 买卖点身份需要哪些钟、原因码、证据载荷和消费门户；当前模块均未定义。

### 6. 三种“伪生命周期”交集

#### `recursive_tower.rs:497` 的 `RETEST_REENTERS_B`

- `CpRecallAtom::RetestReentersB` 位于 HEAD `recursive_tower.rs:473-510`；
  模块注释明确它只是稳定 `B_p/c_p` 对象的只读召回审计原子，**不改变生命周期或
  strict-chain 真值**。
- `recall_failure_atom`（`:530-553`）的几何含义与 first-retrace 相交：leave 已严格出 B，
  retest 又进入 B。
- 真实 `CpScanOwnership.lifecycle` 是 `Pending → Closed`
  （`:433-471, 1700-1853`），只在 `judge_third_cert` 成功时关闭；
  `RetestReentersB` 不参与这次状态迁移。

因此两者只有“重入 B 的几何失败标签”交集；first-retrace 把它提升为“消费该身份第一次回试”
的生命周期事件，Cp recall 没有这层语义。

#### `nest_lifecycle` 的 `Supersedes`

- 相交：都记录旧身份被后继形态替代，且事件有旧身份可查。
- first-retrace：旧身份结束，新身份更换 departure；必须先有 `RetestReenters`，随后显式
  `Restart`。
- nest：仍是同一假设的工程桥迁移，只改 key 的 C 右端，保留所有钟和 revisions；
  终态甚至不允许迁移。

所以名称相同，身份哲学相反：前者是“旧对象退位 + 新对象重启”，后者是“同对象换键留史”。

#### `fill.rs` 消费的中枢 `Superseded`

当前 HEAD **没有** `rust/src/theta_v0/classifier/center_lifecycle.rs`，也没有该 `fill.rs`
接线；它们存在于已分叉的 `main`（本调研只读跨分支对照）：

- `main:classifier/center_lifecycle.rs:265-314`：
  `CenterLifecycleEvent::Superseded` 表示新中枢链推进后，旧 `CenterId` 失去在场身份；
  是 `DeathForm::ArenaTermination`，不是三类点教义破坏。
- `main:backtest/fill.rs:892-924, 2365-2413`：
  #414 后 `Superseded` 命中挂起只记 continuation，**不终结、不清算**，一直等原中枢的三类点。

它与 first-retrace 的交集是“有身份的结构对象被新对象取代并发事件”。差异是中枢事件由链推进
产生，不要求 `RetestReenters`，也没有 first chance/restart 规则；对下游挂起甚至是继续等待，
不是 first-retrace 那种旧身份已消费。故不能把三个 `Supersede(d)` 仅凭名字视为同一转移。

## Q3 同型物清单

### 1. 计数口径

为避免“账本”一词把缓存、执行持仓和结构状态机煮在一起，本节同时给宽口径和严格口径：

- **宽口径同型物**：有稳定身份、对象状态、显式推进/拒绝规则三者即可。
- **严格修订账本**：还必须保留可回放的事件/修订历史，而不是只保留当前值。

### 2. 当前 HEAD 的宽口径同型物：7 套

| # | 类型 | 身份 | 状态/推进 | 历史 | 归类 |
|---:|---|---|---|---|---|
| 1 | `NestLifecycleBook` | `LifecycleKey` | Provisional/Confirmed/Invalidated；`advance` | entry 内 append-only revisions | **严格多态修订账本** |
| 2 | `first_retrace_replay` | `RetraceIdentity` | `active/consumed/restart_allowed` 瞬态推进 | 仅本次返回 event Vec | replay 自动机，不是持久 book |
| 3 | `CpScanOwnership` | `b_center_id + departure_move_id` | Pending→Closed；`advance_cp_lifecycles` | 仅终态证书/证据，无 revision chain | 持久二态对象账 |
| 4 | `ConfirmCursorStore` | `ConfirmKey` | Scanning→Confirmed/TerminalFalse | 无事件历史；失活 key 会 `retain` 删除 | keyed cursor FSM |
| 5 | `CompletedFreezeReducer` | `CompletedMoveId` | 未见→Frozen，冲突拒绝、同事件幂等 | JSONL + reducer 内 append sequence | **严格事件账本，但只有冻结一态** |
| 6 | `PersistentRegistry` | `ElementId` | LivePresent/LiveDetached/Closed/Invalidated 的注册语义；merge/close/invalidate | 只存当前 flags，无 revision chain | 身份注册表 |
| 7 | `trading::CenterBook` | `ladder + seg_start` | Formed/Extended/Terminated；另有 candidate departure 窗 | dead 集和当前值持久，事件只向调用方返回 | 生产中枢生命周期账，无内部 revision chain |

代码锚：

- `CpScanOwnership/CpLifecycleStatus`：
  `recursive_tower.rs:433-471, 1700-1853`。
- `ConfirmState/ConfirmKey/ConfirmCursorStore`：
  `level_view.rs:490-628`。它会删除消失 run 的 cursor，故不满足 append-only 留档。
- `CompletedMoveId/CompletedFreezeEvent/CompletedFreezeReducer`：
  `level_view_store.rs:18-69, 80-166, 168-237`。JSONL 正式边界只追加、replay 幂等、
  sequence 冲突和 freeze 变异都拒绝。
- `HeldLegState/PersistentRegistry`：
  `strategy/persistent.rs:42-73, 79-176`。类型公开四态，但 entry 实存
  `snapshot_present + invalidated`，`close()` 与 `invalidate()` 都把同一 bool 置真，
  无逐对象原因/修订史。
- `trading::CenterBook/CenterEvent`：
  `trading/center_book.rs:1-8, 27-84, 86-204` 与 `trading/types.rs:269-303`。它按
  `ladder + seg_start` 维护 last/dead/frozen/pending-departure，并在唯一 diff 点派生
  `Formed/Extended/Terminated`；`trading/runner.rs`、`spiral/signal.rs` 等有生产调用。
  dead identity 会留在集合中，但事件历史不在 book 内追加保存。

所以当前 HEAD：

- 宽口径是 **7 套**（含题面的 first-retrace prototype）；
- 满足“可回放历史”的严格账本是 **2 套**：
  `NestLifecycleBook` 与 `CompletedFreezeReducer`；
- 其中只有 **1 套**同时具备多状态终局和逐对象 append-only 修订链：
  `NestLifecycleBook`。

### 3. 分支外但仓库可见的第 8 套

`main` 另有 `CenterEventMachine/CenterLifecycleEvent`，具稳定 `CenterId`，能产
`Born/Broken/Superseded/Reset`，是生产中枢生命周期机；当前调研 HEAD 不含对应文件。
若按“仓库 refs 可见”而不是“当前 HEAD 可编译面”计数，宽口径为 **8 套**。本报告不把
`main` 的类型冒充成本分支当前能力。

### 4. 邻近但不计入 7 套

- `OscillationBook`（`strategy/oscillation.rs:525-558`）是有 `OscillationId` 的执行 lot
  账本，记录父腿/子腿和剩余单位；它属于持仓执行记账，不是结构候选/Cand 生命周期。
- `PanMemo` 是可失效/删除的计算缓存，不是对象终局账本。
- 普通 `BTreeMap<id, value>`、统计 witness、audit Vec 若没有对象状态推进规则，均未计。

### 5. 共享定义/trait

HEAD 搜索没有 `Lifecycle` trait、通用 `Revision<K,E>`、通用 terminal-state trait 或共同
event-store 接口。七套各自定义：

- 自己的 key/identity；
- 自己的 state/event/reason；
- 自己的推进函数；
- 自己的幂等、删除、终态和历史策略。

共享的只是偶然基础类型，如 `ElementId`、`CoordinateWindow`、`Side`、`usize/as_of`；
`CompletedFreezeReducer` 的 append-only store 也没有被 nest/first-retrace 作为共同 trait 使用。
这与 #257/#450 所指“三套 `Cand^δ` 无共享形式化定义”在代码形态上的同病事实是：
**相似的身份—状态—事件概念存在，但没有共同类型契约，且相同词（尤其 Supersede）承担不同语义。**
本报告只登记这个事实，不推出应建立哪一种共同定义。

## 事实边界（没查什么）

1. 未读取或改写 GitHub issue 在线正文/评论；#409/#421/#523/#527/#559 的事实来自当前仓内
   review-result 文档与提交态源码。
2. 未运行 BTC/OKLO replay、p123/p409 probe 或 Rust tests；没有重算 92.6%、248、156、35。
   本报告核对的是报告中的分母/口径和当前 HEAD 接线拓扑。
3. 调研中并行线把 HEAD 从 `558104a0…` 推进到 `77d35543…`；本报告没有拿未提交工作树
   当证据，而是在提交落地后重读了 diff、状态/原因类型、`advance`、不变量和 p123 消费面，
   最终源码行号锁在 `77d35543…`。后续若再改状态、key、provider 或消费面，仍需重新对 HEAD。
4. `main` 与调研 HEAD 已分叉；`CenterEventMachine` / `fill.rs` Superseded 只作跨分支事实
   对照，不算当前 HEAD 的可调用能力。
5. 55%/45% 是公开列出底稿的“20 个责任点”工程分类，不是 LOC、运行时覆盖率或架构裁定。
6. 未检查 #570 的 `issue570-*`、gamma/trades 输出，未碰
   `chanlun/agent-roster*`，未执行 git 写操作。
7. 本报告没有决定：买卖点是否应采用三态、`RetestReenters` 是否等于 Invalidated、
   是否需要 append-only 持久史、以及最终复用/抽取/另建方案。这些仍归编排者裁定。
