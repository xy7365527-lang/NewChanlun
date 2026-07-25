# V3 活假设状态机最小切片实装卡（Provisional→Confirmed/Invalidated + first_provable 钟）

- 日期：2026-07-20
- 工位：**设计文档工位**——本卡只设计不实装；未改 `rust/src` 一行，未做 git mutation，主仓 `/Users/silencehan/Projects/NewChanlun` 只读（教义原文亲读自主仓 `docs/chanlun/text/blog/`，worktree 同路径同内容）。
- 分支：`kimi-nest-mainline-20260717`（worktree `/tmp/kimi-nest-mainline`，HEAD `640609071d`）
- 输入材料：`event-lifecycle-existing-inventory-20260720.md`（Provisional/ReviseEvent 零命中、judge_at 一钟多职）、`doctrine-vs-code-nest-recursion-20260720.md`（:102/:147 活假设排除出定义域）、`doc-divergence-endtoend-prototype-20260718.md`（E2E-D5/§1 修订协议/§4.1 五钟，下称 **E2E**）、`mainline-merged-roadmap-20260717.md:75`（E2E-N5）
- 引用格式：代码锚=文件:行号（均相对 `rust/src/`，已逐处亲读核对）；文档锚=文档名 §节/:行号；教义锚=课号:行号（`0XX:n` = `docs/chanlun/text/blog/0XX-*.md` 实际行号）。

## 0. 结论先行

1. **状态不挂在 `NestCandidateEvent` 上，也不挂在 `TypedNestCertificate` 上**：前者是 `Copy` 值语义、每 prefix 由 provider 重算（level_view.rs:479），挂状态即破坏值语义与 bit-exact；后者是终态装配产物、「构造即有效」（nest.rs:842-843），在活假设阶段根本不存在。状态挂在**新 sidecar 注册表 `NestLifecycleBook: BTreeMap<LifecycleKey, NestLifecycleEntry>`** 上，范式复用 `PersistentElement/Pi`（persistent.rs:88-107）与 `YieldBook`（p92_nest_replay_postruling.rs:127-129）——bin 里已有一台 first-write-wins 的雏形账（`entry().or_insert()`，p92:751/:759）。
2. **`LifecycleKey` 不能直接复用 p92 `EventKey`**（p92:68-90）：trend 域确认时 `interval_b/turn_source` 被收束改写为 `[c_start, t*]/t*`（level_view.rs:699-702），同一活假设「确认即身份改变」，会触发假性「身份消失→Invalidated」。最小切片的身份键必须用**结构坐标**：`(level, side, kind, seg_a, seg_c_full, b_center_start)`。
3. **「反超即失效」的两台既有机器都已定位，不需新造力度判据**：trend 域复用 `trend_confirm_time` 扫描的「T5-OR 转假即终假」（level_view.rs:538-539）——反超语义已被机械化，只是现状把终假输出为 `None`（无事件、无记录）；pan 域复用 `segments_diverge_or`（divergence.rs:334-349）作用于行进中 c 窗，c 窗只延不缩 ⟹ 三通道值各自单调不减 ⟹ OR 谓词沿 as_of 单调 真→假（level_view.rs:536 注「各 proxy 只增 ⟹ 真→假单调」），首次翻假即反超时刻、且在同一身份内永久。
4. **trend 域现状的 R1 `t*`（level_view.rs:526-527）语义是 `first_provable_at`，不是 `confirmed_at`**：t* 是「全合取首个全成立时点」，c 段未必完成；而 024:24 要求「C 段的走势类型完成时」结算。状态机把二者拆开——t* 进 `first_provable_at`，`confirmed_at` 另需结构完成（`structure_end_at`）加完成时复核仍弱。这正是盘点 §2.1 认定的「终态几何 + prefix 首证钟」缝合（event-lifecycle-existing-inventory-20260720.md:106）。
5. **judge_at 拆分策略 = 一个 bit 不动**：`judge_at` 字段、provider 写入点（level_view.rs:712/:786）、p92 终态回填（p92:202-207）、CERT 主键（p124_merge.rs:30）、D3 统计（nest.rs:456-461）全部保持逐 bit 现状；新五钟只活在新 lifecycle entry 里。语义修复以**文档化别名 + 新字段**完成，不改既有消费者。
6. **本切片不改 #43 裁定**：N^δ 装配继续只消费已闭合完整 `c_p`（nest.rs:802-810/:987-989）；状态机产出的是观察记录，不进入 `d_parent_interval_snapshot/terminal` 的输入。#43 是否需要为「Provisional 父对象进谱系构建」开口，列入重审材料骨架（§8），由 escalate 裁定文书决定，本卡不越权落锤。

## 1. 亲读核对记录（任务①）

### 1.1 三范式源码

| 范式 | 锚 | 状态集 | 推进机制 | 可借 / 不可借 |
|---|---|---|---|---|
| `MoveStatus` | decompose.rs:34-39（`Active`「链尾块——尾关系/尾中枢仍可变」/`Completed`「前缀不可变」）；fold decompose.rs:56-74；resume decompose.rs:105-122 | 二态（活/完成） | 结构 fold，sealed 前缀冻结 | **借**：`Active` 尾块就是「行进中 c」的结构载体（活假设的合法宿主）；`Completed` 迁移是 `structure_end_at` 的判据源。不可借：无证伪路径 |
| `CpLifecycleStatus` | recursive_tower.rs:461-464；字段 recursive_tower.rs:446（「只能从 Pending 单调闭合为 Closed」:445）；推进 `advance_cp_lifecycles` recursive_tower.rs:1697；dirty 回退 recursive_tower.rs:1437/:1461 | 二态（Pending/Closed） | 独立推进函数作用于 book 内对象 | **借**：①生命周期字段挂 **book 内对象**（`CpScanOwnership`）而非事件——正是本卡 sidecar 范式；②单调闭合 + 显式推进函数形态。不可借：回退 Closed→Pending 是依赖维护不是证伪（inventory:126）；二态无 Invalidated |
| `HeldLegState` | persistent.rs:50-60；`registry_live` persistent.rs:62-66；`PersistentElement` persistent.rs:88-107（pid 内禀、parent 是关系非身份 :99-101） | 四态（LivePresent/LiveDetached/Closed/Invalidated） | persistent overlay 注册表 Pi | **借**：全仓唯一含证伪终态的状态机（`Invalidated` persistent.rs:59）；注册表与快照分离的 overlay 形态。**注意边界**：其 `LiveDetached`（快照找不到 ≠ 作废，persistent.rs:53-54）不可移植——nest 候选身份是结构身份，候选消失 = 结构证伪（E2E §1:81），不是重建伪影（inventory:165 同判） |

### 1.2 教义亲读（主仓原文，逐行核对）

- **061:26**（061-第61课.md:26）：「65开始的走势，由于没实际走出来，所以在和55-60比较时，都可以先假设是进入背驰段。而当走势实际走出来，一旦力度大于前者，那么就可以断定背驰段不成立……在没有证据否定背驰之前，就要观察从65开始的一段其内部结构中的背驰情况」——**活假设（Provisional）／力度反超证伪（Invalidated）／行进中递降观察**三语义同句俱在。注意比较域是「围绕一中枢的两段走势」——活假设的教义原例同时覆盖 pan 语境。
- **024:24**（024-第24课.md:24）：「而C 段的走势类型完成时对应的MACD柱子面积……比A 段对应的面积要小，这时候就构成标准的背弛」——**完成时复核力度仍弱才结算（Confirmed）**；同句含 T4 回拉 0 轴前提（「B 是这个大趋势的另一个中枢，这个中枢一般会把MACD 的黄白线回拉到0 轴附近」）。
- **024:40**（024-第24课.md:40）：「9 点35的第一个红柱，由于并没创新高，所以不构成背驰，10 点40 的第二个红柱子，由于这时候的C 段还没有形成一个中枢，根据走势必完美，这C 段肯定没完，所以继续」——**未完成不结算**（走势必完美 ⟹ C 未完则继续观察，不提前冻结）；同句后段「13 点05 分，第三个红柱子……也没有A段两个红柱子面积和大，显然背驰了」= 同色柱面积口径（divergence.rs:286-304 的教义锚 060:44/057:40/024:40 同源）。
- **024:46**（024-第24课.md:46）：「一旦出现背驰，其回跌，一定至少重新回到B 段的中枢里」——**回中枢是背驰的后果定理，不是确认前提**（E2E P5 `postcondition_at?` 仅诊断钟，E2E §4.1:153）。
- 024:28（同课 :28）「把已经出现的面积乘2，就可以当成是该段的面积」——部分面积外推是 first_provable 的**可选**合法降延迟手段（E2E §4.2:159 合法项）；最小切片不采用（保持谓词与结算同机，避免双口径），登记为后续切片选项。

### 1.3 E2E 原型亲读

- **E2E-D5**（E2E §1 表 :25）：`state: Provisional｜Confirmed｜Invalidated｜Unresolved` + 事件五钟；教义锚 061:26、024:24、024:40、064:28（结构破坏）、024:46（回中枢是后果）。
- **转移表**（E2E §1:81）：无旧事件且 NoCandidate→无输出；候选身份消失→`Invalidated`；候选存在但后一段未完成→`Provisional`；后一段已冻结完成且全部前提可证→`Confirmed`；结构或力度被证伪→`Invalidated`；无法判定→`Unresolved`。**`first_provable_at` 只是 E2E-D1–E2E-D4 在某前缀首次同时可证的时刻，不等于 `confirmed_at`**；旧修订保留，禁止用删除模拟失效。
- **修订协议**（E2E §1:83）：`observed_at/first_provable_at` 一旦首次写入即不后移；`confirmed_at/invalidated_at` 只在首次进入对应终态时写入；`Confirmed/Invalidated` 对同一 key 为终态，终态后身份/规则变更新 key，禁止复活旧 key。
- **五钟**（E2E §4.1:144）：`observed_at / first_provable_at / structure_end_at / confirmed_at / invalidated_at`；P0-P5 阶段表（E2E §4.1:146-153）。
- **EventKey 不含状态与钟**（E2E §6.1:248：「状态、钟与行进中的区间不进入事件身份键」）。
- **E2E-N5**（roadmap:75）：五钟缺，「prefix 首见钟与终态几何拼合」→「事件五钟全由同一前缀产生」；验收 = E2E-S5（`s5_clocks.jsonl`，E2E §6.3:322：逐修订相等且均不大于产生该修订的 as_of）。

## 2. 设计①：状态对象选择与身份键

### 2.1 三个候选的判定

| 候选 | 判定 | 证据 |
|---|---|---|
| 挂在 `NestCandidateEvent` 加字段 | **否决** | 事件 `#[derive(Copy)]`（level_view.rs:479），provider 每 prefix 全量重算（level_view.rs:703-718/:777-794）；值语义 + 无 revision + 快照覆盖正是现状病灶（inventory:103-106）。加状态字段 = 把生命周期塞进一次性快照，跨 prefix 无承载 |
| 挂在 `TypedNestCertificate` | **否决** | 证书「构造即有效」（nest.rs:842-843 `debug_assert!(cert.n_delta())`），只存在于三门合取满足之后——活假设阶段证书不存在，状态时序上无处安放 |
| **新 sidecar 注册表（新 EventKey）** | **采纳** | 范式：Pi 注册表（persistent.rs:88-107）、`CpScanOwnership` book（recursive_tower.rs:434-457）、`YieldBook`（p92:127-129）。事件流不动，状态机在旁路账本上 `advance`；E2E §6.1:197-209 `TowerDivergenceEvent` 的 revision/五钟字段全部落在 entry 上 |

### 2.2 `LifecycleKey`：身份稳定性分析（本卡最关键的工程判定）

p92 `EventKey`（p92:68-90）= `(level, short, kind, seg_a, interval_b, interval_a, turn_source)`。**不可直接升格为生命周期键**：

- trend 域确认分支把 `interval_b` 收束为 `(seg_c.0, t*)`、`turn_source` 改写为 `t*`（level_view.rs:699-702：「confirmed ⟹ 确认时点 t*……interval_b/turn_source 收束到 [c_start, t*]」）。同一活假设从未确认到确认，EventKey 改变 ⟹ 若直接复用，每次 trend 确认都假性触发「候选身份消失→Invalidated」（E2E §1:81）+ 新 key 复活——恰好是 E2E §1:83「禁止复活旧 key」的反面。
- pan 域身份相对稳定：`interval_b = structure.seg_c`、`turn_source = structure.source_index`（level_view.rs:781-785），且 provider 只放已完成段（`segment.end_index <= view.query.as_of`，level_view.rs:723）⟹ 段身份一旦产出即稳定；但窄锚/A′ 回退两路（level_view.rs:730-743）在后续 prefix 可改选不同结构——这是**合法的**身份消失路径（新段完成改变结构选择），不是伪影。

**设计**：`LifecycleKey = (level, side, kind, seg_a, seg_c_full, b_center_start)`——

- `seg_c_full`：c 全离开段结构坐标。trend 域现状在 confirmed 分支丢弃全段坐标（level_view.rs:699-702），**需 provider 增挂一个 payload 字段**（如 `seg_c_full: (usize, usize)`，未确认时 `== seg_c`，确认后仍保全段）——加字段不改既有字段值（影响分析见 §6）。
- `b_center_start`：B 中枢身份快照，已有「prefix 首次观察快照纪律、延伸不改写」（level_view.rs:495-501 注释；p117 §7 实测 37/37 同 start 同核——引 level_view.rs:497-500），是天然的稳定锚。
- 状态、五钟、行进中区间**不进键**（E2E §6.1:248）。

### 2.3 entry 与转移表（最小切片三态，`Unresolved` 出切片）

```text
enum NestEventState { Provisional, Confirmed, Invalidated }   // 终态：Confirmed/Invalidated

struct NestLifecycleEntry {
    key: LifecycleKey,
    state: NestEventState,
    revision: u32,                    // 业务载荷投影变化才 +1（E2E §1:83）
    observed_at: usize,               // 首见 prefix；首次写入不后移（E2E §1:83）
    first_provable_at: Option<usize>, // D1–D4 首次同时可证；首次写入不后移
    structure_end_at: Option<usize>,  // c 结构完成（§4.3）
    confirmed_at: Option<usize>,      // 首次进 Confirmed
    invalidated_at: Option<usize>,    // 首次进 Invalidated
}
```

转移（E2E §1:81 的最小子集，`Unresolved` 见 §2.4）：

| 迁移 | 触发 | 写钟 |
|---|---|---|
| ∅ → Provisional | prefix as_of 首次观察到比较对（§3 的 ObserveCandidate） | `observed_at = as_of` |
| Provisional（自环） | D1–D4 首次同真 | `first_provable_at = as_of`（只写一次） |
| Provisional → Invalidated | **力度反超**（§4.2）或**身份消失**（上一 prefix 有、本 prefix provider 不再产出该 key） | `invalidated_at = as_of` |
| Provisional → Confirmed | `first_provable_at` 已写 ∧ c 结构完成（`structure_end_at = as_of`）∧ 完成时复核力度仍弱（024:24：同一谓词在完成窗上重判为真） | `structure_end_at`、`confirmed_at = as_of` |
| 终态吸收 | Confirmed/Invalidated 后同 key 任何后续观察零输出 | —（禁复活，E2E §1:83） |

完成时力度复核**必须重算**而非沿用 first_provable 旧值——「用『曾经弱过』替代完成时复核」是 E2E P3 明令禁止项（E2E §4.1:151）。

### 2.4 `Unresolved` 出切片的诚实声明

`Unresolved` 要求区分「无候选」与「有对象但证据不足判」（E2E §1:81 末支），后者需要 proxy 前提四态（`Present/Absent/Undecidable/NotApplicable`，E2E §2.2:116）与 provider 失败原因通道；v0 provider 当前对映射失败静默置 `false`（inventory:104），无此通道。本切片不含 Unresolved——**声明=能力**：切片交付的是「活假设→反超失效/完成确认」三态链，不是 E2E-D5 全四态。`DeferOrphan` 的 defer 重判（turn_class.rs:87-89 自认遗留）与 Unresolved→后续前缀继续修订同源，一并列入后续切片（inventory:164）。

## 3. 设计②：first_provable 怎么算（D1–D4 首次同时可证）

定义（E2E §1:81）：`first_provable_at = min { as_of | D1(as_of) ∧ D2(as_of) ∧ D3(as_of) ∧ D4(as_of) }`，所有窗口截到 `as_of`，禁前视。逐要素到既有机器的映射：

| 要素 | trend 域映射 | pan 域映射 | 锚 |
|---|---|---|---|
| D1 定义域/种类 | provider pair 产出（a+A+b+B+c 投影） | 中枢 + Consolidation 块定位 | level_view.rs:660-719；level_view.rs:721-729 |
| D2 比较对 | `seg_a`(b)/`seg_c`(c) 身份 | `structure.seg_a/seg_c` 身份 | level_view.rs:707-708；level_view.rs:732-743 |
| D3 结构前提 | T3 三买 ∧ T2 破极值（包络只扩 ⟹ 假→真单调） | `pan_div_structure_extreme`（044:234 新极值） | level_view.rs:531-534；level_view.rs:735/:740 |
| D4 力度/代理 | T4 回拉 0 轴（proxy 前提）∧ T5 力度或关系 | `segments_diverge_or` 三通道 OR | level_view.rs:530/:535-536；divergence.rs:334-349 |

**关键发现（trend 域）**：`trend_confirm_time` 的 R1 扫描（「从 t3 起逐段端点推进，首个 T2∧T5-OR 同真的段端点即 t*」，level_view.rs:538-539）**就是 D3∧D4 的 prefix 因果首证机**，其输出 t* 应重释为 `first_provable_at`——现状把 t* 当确认时点并把事件坐标收束到 t*（level_view.rs:691-701），是把 first_provable 误作 confirmed 的缝合点（inventory:106）。状态机不重算 trend 力度，**复用 `view.confirm_times` 既有结果**（level_view.rs:695-698 已做纯函数复用）只改语义归属。

**关键发现（pan 域）**：pan 候选只在 c 段**完成后**才产出（level_view.rs:723），故 pan 域现状**没有行进中对象**——#43 排除活假设在候选层的同构表现。最小切片需新增一台**活对观察**：中枢 + `seg_a` 定位沿用 `locate_pan_div_structure`（level_view.rs:732），行进中 c 窗 = `[c_start_live, as_of]`，`c_start_live` = 中枢最近确认段后首个离开段起点（与完成后 `structure.seg_c.0` 同锚，实装时以段序对齐校验——若完成后的 seg_c.0 ≠ c_start_live 则该 key 走身份消失路径，诚实记 Invalidated 而非改锚）。c 窗结构载体 = `MoveStatus::Active` 链尾块（decompose.rs:36）；注意 #148 窗口内 frontier 可变（decompose.rs:97-101），Active 块外缘可改写——c 窗右端随 as_of 前进是设计内行为，左端改写则触发身份消失。

**v0 基例衔接**：`exit.rs:154-156` 预留「v0 基例的 first_provable ≡ 候选确认 bar……v1 改读最深处证书 first_provable」与 `exit.rs:180`——本切片投产后，`reverse_nest_cert_base` 的 v1 升级点即消费本 entry 的 `first_provable_at`，接口预留位不变。

## 4. 设计③：「反超即失效」触发判据

### 4.1 判据（061:26「一旦力度大于前者，那么就可以断定背驰段不成立」）

```text
反超(key, as_of) ≔ entry.first_provable_at.is_some()            // 曾可证（曾弱）
                ∧ ¬force_predicate(key, windows(as_of))          // 当前不再弱
```

- **trend 域**：`force_predicate` = T5-OR；反超机**已在** `trend_confirm_time` 扫描里——「T5-OR 转假即终假（扫描终止，返回 None）」（level_view.rs:538-539）。现状把终假丢成 `None`（无事件、无审计）；状态机改为：终假 as_of = `invalidated_at`，产出 `Invalidated` revision。**零新判据，只接住既有机器丢掉的结果**。
- **pan 域**：`force_predicate` = `segments_diverge_or(hist, dif, side, a_idx, c_idx)`（divergence.rs:334-349）作用于活窗。

### 4.2 单调性 ⟹ 反超时刻良定义且终态合法

- `same_color_area`（divergence.rs:291-304）：c 窗只延不缩，逐项 `+= h.abs()`/`+= h` 非负 ⟹ 单调不减。
- `same_dir_hist_peak`（divergence.rs:310-327）：段内峰值的 max/min-fold，窗扩 ⟹ 峰值单调不减。`segment_dif_peak` 同理（level_view.rs:536 注「各 proxy 只增 ⟹ 真→假单调」）。
- 每通道谓词 `c_val < a_val`（a 窗固定）随 c 延**只能 真→假**；三通道 OR 同样单调 真→假（divergence.rs:345-349）。
- ⟹ 同一身份内首次翻假的 as_of 唯一、且此后不会自行复真——`Invalidated` 作为终态在数学上干净（不依赖「不再复活」的工程纪律，而由单调性保证；身份不变前提下）。

### 4.3 口径注记（090：声明=能力）

- OR 的否定 = **三通道同时** `c ≥ a`——「力度反超」在本切片口径 = 「没有任何一个通道还支持 c 弱于 a」。061:26 说「力度大于前者」，代码通道谓词是严格 `<`（divergence.rs:331-333「严格 `<`，等值不算衰减」），故**等力亦失效**——这是工程口径，与 `is_divergence` 同 discipline，实装卡登记为显式口径选择，不冒充教义逐字。
- **结构证伪**（064:28 比较错对象、身份消失）走第二触发（§2.3 转移表「身份消失」）；「小级别延伸使大级别摆脱背驰」（043:30，E2E P4 :152）属跨级证伪，依赖 E2E-L 谱系，出本切片。
- **结构完成判据**（`structure_end_at` 的源）：pan = `seg_c` 所在段进入 provider 已完成段集（level_view.rs:723 同条件）；trend = c 所在块 `MoveStatus::Active→Completed` 迁移（decompose.rs:34-39）或段终 `SegmentTermination::{FirstKindConfirmed,SecondKindConfirmed}`（segment.rs:200/:205）——两者皆既有判据，钟字段新建。

## 5. 设计④：judge_at 一钟多职的拆分（不破坏既有消费）

### 5.1 现状三职（逐项带锚）

1. **provider 写入**：`judge_at = view.query.as_of`（level_view.rs:712/:786），注释自称「由 prefix 首次观察写入，不读取 CompletedFreezeEvent.created_at 或挂钟」（level_view.rs:477-478）——但 provider 在**每个** prefix 都写当前 as_of，「首次」语义实际由下游 book 的 `or_insert` 兑现。
2. **bin 终态回填**：终态装配时 `event.judge_at = book.divergences.get(&key).or_else(|| book.candidates.get(&key)).copied().unwrap_or(max_bars - 1)`（p92:202-207）——**已确认事件回填「首证」as_of，未确认事件回填「首见」as_of，两个钟挤进一个字段**；`unwrap_or(max_bars-1)` 是兜底后移（E2E §4.2:160「以未来完成回填早钟」的温和形态）。book 本身 first-write-wins（p92:751/:759 `entry().or_insert(as_of)`；prefix pass 同型 p92:505/:508）。
3. **下游消费**：CERT 行主键 = judge_at 向量（p124_merge.rs:30）；D3 逐边单调统计（nest.rs:456-461，只计数不作硬门 nest.rs:617）；`FrozenCompletedMove.entry_bar = judge_at + 1`（level_view_store.rs:395）；离线 dump 解析（p95_carriedonly_ab.rs:591 等）。

### 5.2 拆分策略：**不动 judge_at，新钟进新对象**

- `judge_at` 字段、写入点、回填、下游消费**逐 bit 保持**（§6 影响分析）。语义上以文档冻结：`judge_at` ≡ 兼容别名，新代码一律读 lifecycle entry。
- 新五钟只存在于 `NestLifecycleEntry`（§2.3），由 prefix 事件流喂给 `NestLifecycleBook::advance(book, events, as_of)`。
- **迁移对应关系（pan 域零重算）**：`book.candidates` 首见 ≈ `observed_at`；`book.divergences` 首证 ≈ `first_provable_at`（pan 域 divergence_confirmed 首真即 D4 首真，D1–D3 已由候选产出承载）。p92 回填逻辑把二者择优挤入 judge_at（p92:202-207），恰恰是一钟多职的实证——新 book 把两张表升格为两个字段，回填语义被解释而非修改。

### 5.3 首次写入不后移的断言（实装时落 `debug_assert` / 单测）

1. `observed_at`、`first_provable_at` 一旦 `Some` 不再改写（`or_insert` 同构；E2E §1:83）。
2. 钟序不变量：`observed_at ≤ first_provable_at ≤ structure_end_at ≤ confirmed_at`（存在偏序的项之间）；`invalidated_at ≥ first_provable_at`（反超定义要求曾可证，§4.1）；身份消失路径允许 `invalidated_at` 独立于 `first_provable_at`。
3. 一切钟 ≤ 产生该 revision 的 `as_of`（E2E-S5 判据，E2E §6.3:322）。
4. 幂等：同 `as_of` 重复 advance ⟹ 零新 revision（E2E §1:83 `as_of = state_as_of` 条款的最小形态）。
5. 终态吸收：`Confirmed/Invalidated` 后同 key advance 零输出（禁复活）。

## 6. 前/后伪码 + bit-exact 影响分析

### 6.1 前（现状，逐行有锚）

```text
# provider（每个 prefix as_of 全量重算；level_view.rs:660-804）
for as_of in prefixes:
    events = provider(view(as_of))           # judge_at = as_of（:712/:786）
    for ev in events:
        book.candidates.entry(Key(ev)).or_insert(as_of)        # p92:751
        if ev.divergence_confirmed:
            book.divergences.entry(Key(ev)).or_insert(as_of)   # p92:759
# 终态装配（p92:199-208）
for ev in terminal_events:
    ev.judge_at = book.divergences.get(key)                    # 首证（已确认）
                  .or_else(book.candidates.get(key))           # 否则首见
                  .unwrap_or(max_bars - 1)                     # 兜底后移
# 结果：divergence_confirmed 快照覆盖（inventory:105）；反超被丢成 None（level_view.rs:539）
```

### 6.2 后（最小切片）

```text
# rust/src/theta_v0/classifier/ 新增 lifecycle.rs（新文件；既有行零改动）
book: NestLifecycleBook = Genesis
for as_of in prefixes:
    events = provider(view(as_of))           # 逐 bit 不变
    delta = book.advance(events, as_of)      # §2.3 转移表；§5.3 断言 1-5
    #   ObserveCandidate: D1–D3（trend=pair 产出；pan=locate+活窗 c_start_live）
    #   ReviseEvent:      D4（trend=confirm_times 复用 :695-698；pan=segments_diverge_or 活窗）
    #   反超:             trend=T5-OR 终假 :538-539；pan=OR 翻假（§4.2 单调）
    #   结算:             structure_end（§4.3）∧ 完成窗重判仍弱（024:24）
    emit(delta)                              # 新 dump（新增产物，不改既有行）
# 终态装配回填、p92 dump、CERT 主键、D3 统计：逐 bit 不变
```

### 6.3 bit-exact 影响分析

| 面 | 影响 | 依据 |
|---|---|---|
| `NestCandidateEvent` 事件流 | **零**（不加字段、不改值） | 切片只消费事件流；`seg_c_full` 身份坐标问题（§2.2）的解法若是 provider 加 payload 字段，则属**后续切片**——本切片 trend 域身份键暂用 `(level,side,kind,seg_a,seg_c_as_emitted,b_center_start)` 并显式登记：trend 确认时 key 变化的假性失效由「t* 收束白名单」豁免（同 key 除 interval_b/turn_source 外全等且新 interval_b ⊆ 旧 ⟹ 视为同身份迁移，谱系记 `supersedes` 不记 Invalidated）。**诚实标注**：该豁免是工程桥，不是 E2E WireV1 EventKey（E2E §6.1:248），全量正解待 provider 暴露 seg_c_full |
| CERT dump / p124 合并主键 / D3 统计 | **零** | judge_at 向量与回填不动（§5.2） |
| `divergence_confirmed` 语义 | **零** | t* 重释为 first_provable 只发生在 lifecycle book 语义层；bool 本身不动——若未来改 bool 口径将触发 p92 兜底对账（p92:507-510）与 p95 翻转计数变化，**出本切片** |
| N^δ 装配 / #43 | **零** | `d_parent_interval_snapshot`（nest.rs:802-810）、装配拒绝（nest.rs:987-989）、基例 `terminal.confirm_side`（nest.rs:642）不读 lifecycle book |
| 新产物 | lifecycle dump（新文件/新列，纯增量） | E2E-S5 `s5_clocks.jsonl` 的最小前身 |
| v3 硬禁令合规 | 无概率/统计推断、无回测验证、无 EMH 假设 | 全部判据为确定性结构/力度谓词；测试用合成序列（§7） |

## 7. 单测设计（「反超即失效可触发」为首要验收）

全部为确定性合成序列（hist/dif/结构桩），无随机、无统计断言：

- **T1 trend 反超可触发**：构造 T3 确立后 T5-OR 在 `t_k` 为真（c 同色面积 < a）、于 `t_{k+m}` 因 c 窗面积累积反超而转假的序列。断言：`observed_at=t_0`、`first_provable_at=t_k`、`invalidated_at=t_{k+m}`、状态 Provisional→Invalidated；`t_{k+m+1}` 再 advance 零输出（终态吸收）。判据源 = level_view.rs:538-539 终假路径——**本测试同时证明「现状把反超丢成 None」被接住**。
- **T2 pan 活窗反超可触发**：a 窗固定、c 活窗逐 bar 延展，`same_color_area(c) < same_color_area(a)` 于 as_of1 成立、as_of2 三通道全不成立。断言 first_provable_at=as_of1、invalidated_at=as_of2。**等力边界用例**：c 面积 == a 面积（严格 `<`，divergence.rs:331-333）⟹ 该通道不成立——登记 §4.3 口径。
- **T3 单调不复活**：T1/T2 失效后继续投喂 c 窗再变弱的序列（理论上单调性禁止，§4.2），断言无新 revision——把单调性论证转成可执行证伪器。
- **T4 钟不变量**：§5.3 断言 1-4 全列（写入不后移、钟序、≤ as_of、幂等零 delta）。
- **T5 bit-exact 护栏**：同输入下挂/不挂 lifecycle book 的 provider 事件流逐字节相等（序列化对比）——防「sidecar 反流改生产」。
- **T6 身份消失**：pan 窄锚→A′ 回退切换（level_view.rs:730-743 两路）致上一 prefix 的 key 消失，断言 Invalidated（E2E §1:81），且与 T2 的反超失效可区分（失效原因码入 revision 载荷，诊断用）。
- **T7 完成时复核**：c 完成窗上力度仍弱 ⟹ Confirmed（structure_end_at = confirmed_at = as_of）；完成窗上已反超 ⟹ Invalidated 而非 Confirmed——024:24「完成时……面积要小」的机械表达，防「曾经弱过」冒充（E2E §4.1:151 禁项）。
- **T8 trend 身份迁移豁免**：confirmed 分支 interval_b 收束（level_view.rs:699-702）触发 key 变化，断言按 §6.3 白名单记 `supersedes` 迁移而非 Invalidated。

## 8. #43 裁定重审材料骨架（只备料，不落锤）

1. **被审裁定**：`dparent-leftend-p0-review-20260711.md:1`（P0 定义复议裁决 #43）；核心条款：规范父区间只消费已闭合完整 `c_p`，§3.2(5) 禁止映射（:81——不得把 episode 左端/趋势起点/confirm_src/until_start/产量锚代填 `c_start_full`）；代码落点 nest.rs:802-810、nest.rs:987-989、strict_nest_check.rs:725；能力预检 `p43-replay-cfull-20260712.md`（BLOCKED-CAPABILITY，未产新候选/边/证书）。
2. **裁定当时的正确性（重审不否认）**：拒绝未来函数、声明=能力（doctrine-vs-code-nest-recursion-20260720.md:147 前半「工程上是诚实拒绝未来函数」）。
3. **冲突陈述**：同文 :147 后半——「语义上等于把『活假设』从定义域中删除：一个尚未完成的父背驰段在系统中不是任何对象」，与 061:26「在没有证据否定背驰之前，就要观察……逐次下去」直接冲突；E2E-D5 `Provisional` 零载体（inventory:21-27）。
4. **本切片不请求变更的部分**：N^δ 装配消费口径（仍只闭合 c_p）；`d_parent_interval_*` 三函数语义；基例门。
5. **提请裁定的问题清单**：(a) Provisional 父对象是否允许进入 E2E-L 谱系**构建**（P2 递降并行，E2E §4.1:150；`Consume_at` 仍只接 Closed，E2E §1:85）——即「构建放开、消费不放开」是否裁定合法；(b) Invalidated 事件作为审计证据保留（禁删除模拟失效，E2E §1:81）是否进入 #43 的诚实域；(c) 身份键口径（§2.2 结构坐标 vs E2E §6.1:248 WireV1 EventKey）何时并轨；(d) `seg_c_full` 暴露是否授权为 provider 正式字段。
6. **重审证据包**：本卡 §7 T1–T8 单测；E2E-S5 clocks（E2E §6.3:322）与 S1 predicates（§6.3:318）最小集；bit-exact 护栏 T5 结果；doctrine-vs-code :102/:147 与本卡 §1.2 教义行号。
7. **程序**：按 escalate 惯例出裁定文书，SUPERSEDE 条款显式列明被替代条款编号（参照 recover-trigger-nesting-ruling-20260718.md:208 复议触发条件格式）；重审前 #43 现行有效，本切片所有设计均不以其被推翻为前提。

## 9. 边界与未决登记

- 本卡**不实装**：无 rust/src 改动、无 cargo、无重放、无 git mutation；后续实装任务应以本卡 §2-§7 为规格、§8 为裁定前置。
- 出切片项（诚实登记，非简化）：`Unresolved` 四态（§2.4）、WireV1 全量 EventKey/StateKey/修订链（E2E §6.1）、谱系两钟 opened/closed（E2E-N5 的 lineage 半部）、跨级证伪（043:30）、024:28 面积乘 2 外推、postcondition 诊断钟、`DeferOrphan` 重判（turn_class.rs:87-89）、Lean 侧 `ActiveTail↔OpenTailSystem` 桥（Parse.lean:337，inventory:166）。
- trend 域身份键白名单豁免（§6.3）是工程桥，并轨 WireV1 前不得进入任何证书真值路径——该限制本身需写入实装任务验收。
- 后续实装若触碰 `divergence_confirmed` 布尔口径或 judge_at 写入/回填，必须重过 p92 兜底对账（p92:507-510）与 p95 翻转基线（p95_carriedonly_ab.rs:473）——本卡不授权此类改动。
