# #93 实装卡：事件派生 churn 修复——冻结 pair 水线（方向一，推荐）/ 增量 provider（方向二）

- 日期：2026-07-21
- 来源：issue #93（#73 A 线；parent 锚 = #75 交付报告成本实测）。本卡是纯文档起草：只读核验 + 落盘，零代码改动、零 git mutation、零 cargo build/test（另一工位独占 crate 跑 #77）。
- 前置：#78 落地后开工（票面 blocked-by：同 crate 串行；票面预期本票动 level_view.rs 签名面——方向一的签名面比票面预期小，见 §3.1/§3.3，串行纪律不变）。开工前先读 #75 交付评论（`gh issue view 75 --comments`）与 #76 实装/验收报告。
- 纪律：禁 git mutation；禁改 Cargo.toml；TDD 先行；**门关逐字节不变 + 门开决策逐字节不变**是双回归命门；090（禁简化/补丁，声明=能力）。
- 行号锚 = 本 worktree（`/tmp/kimi-nest-mainline`）当前状态。

## 1. 与 5a/5b 线形化的关系（票面硬要求）

结论一句话：**同属超线性治理、共用同一套水线证书链，但作用面互不重叠——5a/5b 治「塔重放路径的逐 pair 重算」（p123 bin），本票治「门的事件派生路径的逐 bar 全级重派生」（runner.rs NestChainGate）；互不前置，分票推进，游标语义必须同源（禁第二套水线/游标定义）。**

逐条：

- **5a 作用面**（`wave1-plan5a-implementation-card-20260719.md`，R2 已确认）：p123_fast_replay 塔重放路径——`trend_confirm_time`（level_view.rs:542-633）的 per-pair 游标驻留（`ConfirmCursorStore` 放 bin `LevelDerived` 旁），新入口 `assemble_level_view_resident`。**该入口当前未落地**（level_view.rs 无 `assemble_level_view_resident`/`ConfirmCursor` 符号，照实登记）；已落地的是其 §2.3(a) lib 暴露面：`tower_confirmed_len` 访问器（classifier/mod.rs:1003-1006、`tower0_confirmed_len` :992-994、写入点 :1563-1566）。
- **5b 作用面**（wave1-plan5b 卡，R1/R3 已确认）：p123 run 语境的 pan memo（bin `RunEntry` 旁），失效链 = e_src 水位 + ≥2 后继块门。
- **本票作用面**：`NestChainGate::derive_level_events`（runner.rs:1262-1341）经 `assemble_level_view`（旧入口，level_view.rs:1168）→ `provide_nest_candidate_events_ext`（level_view.rs:724）的事件派生——与 p123 evaluate_run 是**两条独立调用链**，状态持有者也不同（runner.rs 的 gate vs p123 bin 的 LevelDerived/RunEntry）。5a 游标即使落地也不自动覆盖本路径。
- **共享前提（已落地）**：水位证书链——L0 = parser #106 `segments_confirmed_len`（mod.rs:1570-1571 消费），L1+ = `prefix_count` sealed（mod.rs:1825，frontier pop + cascade truncate 证书，5a 卡 §3.1 同引），统一经 `tower_confirmed_len`（mod.rs:1003）读出。本票的水线语义**必须**复用此访问器，禁另造水位定义（禁第二查法）。
- **是否互相前置**：否。5a/5b 不改 gate 链路；本票（方向一）不改 p123 链路。唯一的排期耦合是「若本票方向一选择复用 5a 的 `ConfirmCursor` 类型做终假判定」——此为软依赖：5a 先落则直接复用（lib 类型、gate 持 store，与 R2「bin 持有」纪律一致）；本票先动则走最小 additive 终假暴露（§3.1 R2 条），5a 落地时归并，登记为对齐项。
- **合并还是分票**：分票。回归锁不同（5a/5b = p123 dump 对拍 + `P123_SHADOW=1`；本票 = runner 门关/门开决策 diff + 冒烟 wall）、验收读数不同、改动面不同（不同 bin + 不同入口）。R1-R3 清单附注（r1-r3-ruling-confirmation-checklist-20260720.md:26）的「同改 B3-B5 须同工位」冲突面在方向一下不出现（不动 evaluate_run/B3-B5）。

## 2. 现状机制核验（全部带锚）

### 2.1 派生链路（门开每 bar）

1. π 层主循环每 bar：`gate.sync_events(&tower_i, i)`（runner.rs:2386；出场侧同一机器 `NestCertGateExit::sync_bar` :1653-1670——**单一装配源，本票修复自动覆盖进/出两路**）。
2. `sync_events`（runner.rs:1233-1256）：对 level ∈ 1..tower.len()，指纹 = `(tower[ℓ], tower[ℓ-1])` 的 Rc `ptr_eq`（:1246-1249），任一变 ⟹ `derive_level_events` 全级重派生（:1252）。
3. `derive_level_events`（:1262-1341）：`lower_legs_from(tower[ℓ-1])` 全塔映射（:1273）→ 扫 tower[ℓ] **全部**合法 run（:1283-1339）→ 每 run `project_extended_windows_carried_only` + `decompose` + `assemble_level_view`（:1308）+ `provide_nest_candidate_events_ext`（:1320）。assemble 内 `confirm_times` 对该 view **全部 pair** 计算（level_view.rs:1225）。
4. `absorb_exts`（:1345-1371）：`n_events_seen += 1` 对**每个派生出的事件**计数（:1348，含未确认、含逐 bar 重复）；只收 `divergence_confirmed`（:1349）且 first-wins 去重（:1352-1354）。
5. 索引惰性：`has_bridge_key`（:1402-1413，by_end 增量维护 O(1)）无命中 ⟹ `sync_index` 不重建（:2387-2402、:1659-1669）——**索引侧已是惰性，churn 全在派生侧**。
6. 读数行：`NEST_GATE_INDEX events=… events_seen=… derivations=…`（:3193-3208）。

### 2.2 churn 根因链（四层，逐层可核）

- **根因①：指纹自溃（第一根因）**。gate 为防 ABA **持有** tower[ℓ]/tower[ℓ-1] 的 Rc 克隆（derived 字段 :1168-1172，更新点 :1254）⟹ classifier 端 `Rc::make_mut(&mut cache.moves_tower_l0)`（mod.rs:1598）strong_count>1 ⟹ **每 bar 写时复制新分配**（mod.rs:1573-1574 注释：strong_count==1 才原地 O(tail)）⟹ tower[0] 的 Rc 指针逐 bar 变 ⟹ :1246 的 `ptr_eq` **恒失效** ⟹ level-1 逐 bar 全级重派生。L1+ 同理（cascade truncate 走 `Rc::make_mut`，mod.rs:1821-1822）。**塔内容 99% bar 未变（下条），指纹却逐 bar 报警。**
- **根因②：COW 本身是 O(n²)**。每 bar 全 Vec<LeveledMove> 克隆 = O(prefix)/bar ⟹ 即使派生被跳过这笔也照付。门关路径无此（无跨 bar Rc 持有，strong_count==1 原地复用）——这是门开 vs 门关 wall 差的第一构成。
- **根因③：重派生是全前缀扫描**。`derive_level_events` 每触发一次 = O(prefix)（全 run × project+decompose+assemble+provider），逐 bar 触发 ⟹ O(n²)。
- **根因④：seen 计数口径放大读数**。:1348 派生即计（未确认与重复全计）⟹ 300k 冒烟 events_seen=166,226,275，而 first-wins 确认入账仅 770（出处见 §2.3）。
- **塔内容真变率的实证**：L0 塔内容变更判据（mod.rs:1583-1595，逐值比对 tail）实测真变率 0.78%（mod.rs:1578-1582）；BSP memo 注释同源实证「16K bar 中 L0 segments 仅变 120 次，~99.2% bar 全量重算是冗余」（mod.rs:2036）。**即：约 99% 的 bar 上 level-1 事件集逐字节不变，现行架构却逐 bar 全量重派生。**

### 2.3 读数出处（照实）

- 「events_seen=166M / 入账 770」：#75 交付评论（`gh issue view 75 --comments`，「照实登记」条：「门开全量 4.6M 在当前事件派生架构下不可行（派生 churn O(n²)，events_seen=166M/入账 770）」）。
- 精确值 166,226,275 与 wall_on=430s：**仅见 #93 票面**；本 worktree 全仓 grep 无 `166226275`/`430s` 命中（冒烟日志未落盘仓库，照实登记，以票面为准）。复测口径 = `nest_gate_smoke_stats`（runner.rs:7706，`NEST_GATE_SMOKE_BARS=300000`，同测试打印 wall_off/wall_on :7736-7742 + NEST_GATE_INDEX 行）。
- 「入账 770」对应 `events=` 字段（:3193-3196，`events_by_level` 总长 = first-wins 确认事件数）。

## 3. 两修复方向设计对比

两方向**机制同构**（同一水线证书 + 同一结算语义），取舍 = 切口位置、签名面、与既有裁定的对齐成本。

### 3.1 方向一：冻结 pair 水线（gate 侧切口，推荐）

**步骤 0（公共前置，指纹自溃修复，runner.rs 内）**：derived 指纹从「Rc 持有 + ptr_eq」改「水线 + 尾段值指纹」：

- per level 存 `(w_self, w_lower, tail_self: Vec<LeveledMove>, tail_lower: Vec<LeveledMove>)`——**不再持有 Rc** ⟹ classifier make_mut 恢复 strong_count==1 原地 O(tail)（mod.rs:1573），根因②的 COW 消除。
- 判定：w = `tower_confirmed_len`（mod.rs:1003；证书保 [..w] 跨 bar bit-stable，前缀免比对），tail = [w..] 逐值比对（`LeveledMove: PartialEq`，mod.rs:1592 已有同款比对先例；判据形态复刻 `forest_dirty_l0` mod.rs:1583-1595 / `popped_upper != tail_upper` mod.rs:1873-1876）。
- **跳过合法性的构造性论证**（bit-exact 命门）：塔内容不变 ⟹ ①provider 全部结构输入（legs/windows/blocks/centers/pairs）逐字节同；②confirm 扫描域 = {leg.end ≤ as_of} 的 legs 集合不变（塔不变 ⟹ 无新 leg，as_of 增长不进新元素）⟹ confirm_t 不变；③hist/dif 前缀 append-only 逐位稳定（mod.rs:861-868 同族证书）；④pan 候选枚举（level_view.rs:805 `end<=as_of` 过滤）同②。⟹ 派生事件集逐字节同，唯一差异是未确认事件的 judge_at——未确认事件在 :1349 被丢弃，对账本零贡献。已确认事件 first-wins 保首次 judge_at（:1352-1354）不后移。**跳过 = 账本逐字节不变**，这不是运行时猜值，是 :1249 注释本已声明的语义（「两级塔内容逐字节未变 ⟹ 事件集不变」）——本步骤只是让指纹兑现该语义。
- 水线回退（clear/血缘断裂）⟹ 保守全量重派生（恒正确退化）。

**步骤 1（结算冻结，dirty bar 的派生面收缩）**：dirty bar 上不再全前缀重派生，按 run 结算态分流：

- **R1 confirmed 冻结**：run 内全部 pair 已确认入账 ⟹ 该 run 结算，永不再派生（事件已 first-wins 入账，重派生产出必被 :1353 dedup——零贡献可构造性证明）。实测分布：确认 ≈ 50.9%（5a 卡 §1 引 §2.6）。
- **R2 终假冻结**：终假（TerminalFalse，proxy 只增 ⟹ 转假即终假，033:26；5a 卡 §4.4 同证书，实测 ≈43.1%）的 pair 永不确认 ⟹ 其未确认事件每次重派生都在 :1349 被丢弃——继续派生是纯 churn。**障碍**：现行 `confirm_times: Vec<Option<usize>>`（level_view.rs:1155）不区分「终假」与「扫描耗尽」。切口 = level_view.rs 最小 additive 暴露：从 `trend_confirm_time` 扫描环提取共享内核，出 `Confirmed(t*)/TerminalFalse/Scanning` 三态（禁第二查法：与 :619-621/:628-630 同一环体，形态同 5a 卡 §2.3(b) 的 `confirm_scan` 提取；若 5a 已落则直接复用其 `ConfirmCursor.terminal`，不再新造）。
- **R3 pan 出生即定型**：Consolidation 分支事件（level_view.rs:803-859+）的全部输入（locate/extreme/力度）在其 segment 落入水线 [..w_lower] 后固定 ⟹ 出生 bar（dirty bar）派生一次即终态，此后冻结。
- **活跃集**：仅「frontier run（跨水线）+ 含 Scanning pair 的 run」在 dirty bar 重派生（judge_at 首次观察纪律不变：活跃 pair 每 dirty bar 照常派生，确认当 bar 入账）。Scanning 实测 ≈6%（5a 卡 §1）——**活跃集规模是本案第一经验未知，测量先行（§4-T0）**。
- clean bar（值指纹不变）⟹ 整级跳过（步骤 0）。

**切口汇总**：runner.rs（NestChainGate derived 字段 :1168-1172、sync_events :1233-1256、derive_level_events :1262-1341 增 run 结算态）+ level_view.rs 一处 additive 终态暴露（或复用 5a 游标类型）。provider 两入口签名（:705/:724）**不动**；`assemble_level_view` 签名不动。

### 3.2 方向二：增量 provider（lib 侧切口）

新增增量派生器：`LevelDeriverState`（per-level：run 水线、pair 游标、pan 处理水线）+ `provide_nest_candidate_events_incremental(&mut state, …, w_self, w_lower)`，每 bar 只派生增量事件（新 run + frontier 增量 + Scanning 续扫）。

- 状态纪律：按 R3 裁定精神（r1-r3 清单 :22，「lib 内持状态破坏同入参同输出显式性、shadow 不可控」已否决），状态必须**显式参数 + bin 持有**——与方向一的状态持有位置相同，差异只剩「状态类型与增量逻辑定义在 lib（level_view.rs）还是 bin（runner.rs）」。
- 额外成本：须把 `assemble_level_view` 的 pair 枚举/confirm 计算拆出可增量入口（pair 级粒度），与 5a 的 resident 改造高度同构——**各自实装即两套游标语义，撞禁第二查法**；合理形态是 5a 先落、共享游标类型，本票在 lib 消费 ⟹ 排期硬耦合到 5a。
- 签名面：provider 签名面必动（票面 blocked-by #78 的本义）；与 #78 评审修复的冲突面最大。

### 3.3 对比与推荐

| 维 | 方向一（冻结 pair 水线） | 方向二（增量 provider） |
|---|---|---|
| 根因①②（指纹自溃/COW，gate 侧） | 直接修（步骤 0） | 不修，须另做——本在 runner.rs |
| lib 签名面 | 一处 additive 终态暴露（或复用 5a 类型）；provider 入口不动 | provider 签名面必动 + assemble 增量入口 |
| 与 R2/R3 裁定对齐 | 天然 bin 持有状态 | 须显式参数化规避 R3 已否决形态 |
| 与 5a 排期耦合 | 软（可先走后归并） | 硬（避第二套游标须等 5a） |
| bit-exact 论证 | 构造性（§3.1 四条 + R1/R2/R3 结算证书） | 同构，但论证面更大（lib 内部状态机） |
| 与 #78 冲突面 | 小 | 大（票面 blocked-by 本义） |

**推荐：方向一。** 理由：①第一根因（指纹自溃 + COW）本就在 gate 侧，方向一顺手根除，方向二管不到；②签名面最小，090 下论证链最短；③状态 bin 持有与 R2/R3 已定纪律零摩擦；④对 5a 是软依赖，不锁排期。**这不是补丁**：步骤 0 修的是「指纹语义与塔内容稳定性脱节」的结构性错位，步骤 1 的冻结挂在证书链（parser #106 / A3 prefix_count / 033:26 终假单调）上，声明=能力。

**决策树（照实）**：§4-T0 测量若显示「长寿 Scanning pair 钉住大量 run 活跃」（活跃 run 数随 prefix 增长），run 粒度冻结不达标 ⟹ 升 pair 粒度（复用 5a `ConfirmCursor`，方向一内升级，不转方向二）；此时若 5a 未落，先落最小终态暴露 + per-pair 水线，游标归并登记为 5a 对齐项。

## 4. 测试设计（先行）

- **T0 测量封口（先于一切实装）**：300k 冒烟打诊断（stderr，不进决策面）：per-level 派生触发率、run 结算分布（confirmed/TerminalFalse/Scanning）、活跃 run 存活期分布。产出定死：run 粒度 vs pair 粒度（§3.3 决策树）、events_seen 断言阈值（禁拍脑袋）。
- **T1 门关回归锁**：门关分支一行不动（构造保证：改动全在 gate 内 + lib additive），既有门关直通锁（runner.rs:2179 一带）、#76-T2（:7308）/#76-T3（:7384）全过。
- **T2 门开决策逐字节**：`nest_gate_smoke_stats`（runner.rs:7706）`NEST_GATE_SMOKE_BARS=300000`，实装前后各跑，决策输出 diff=0（NEST_GATE_STATS 判定字段逐字节；成本计数字段 derivations=/events_seen= 预期变，白名单逐字段列明）；三跑逐字节一致（#75 口径）。
- **T3 账本同一性单测**：合成塔序列（含 frontier 重写、延迟确认、终假、pan 出生）上，冻结派生账本 == 现状全量派生账本：`events_by_level`/`seg_c_full`/`by_end`/`seen` 逐字段相等、judge_at 不后移。关键案例：①pair 扫描耗尽 N bar 后确认 ⟹ judge_at = 确认 bar（延迟确认不丢）；②终假 pair 不再派生 ⟹ 账本同；③clean bar 跳过 ⟹ 账本同。
- **T4 值指纹单测**：内容不变 Rc 变 ⟹ 跳过；tail 单元素变 ⟹ 重派生；水线回退 ⟹ 保守全量；`clear()` 后指纹归零。
- **T5 终态暴露单测**：三态暴露与 `trend_confirm_time` 冷路径同案同果（Confirmed(t*) ⟺ Some(t*)；TerminalFalse/Scanning ⟺ None 的两分），合成序列逐案对拍（与 5a 卡 §5-1 游标单测同构——若复用 5a 类型则直接继承其测试网，禁重造）。
- **T6 events_seen 断言**：300k 冒烟 events_seen ≤ T0 定死阈值（票面目标：166M → 与入账 770 同量级，即 O(10³-10⁴)；达不到照实上报实际量级与归因）。
- **T7 性能验收**：`wall_on ≤ 5 × wall_off`（300k，同测试同机三跑取中位，票面①照实判定）；同步打 per-level derivations/冻结命中率诊断行。
- **全量底线**：`cargo test --release --lib` 全绿零变红（基线 = #78 落地后读数；#76 交付时 1807 passed，issue76-impl-20260721.md:67）。

## 5. 验收线

1. `cargo test --release --lib` 全绿零变红（基线 = #78 落地后读数）。
2. 门关全路径逐字节不变（既有回归锁过）；门开 300k 决策 diff=0（T2，三跑）。
3. 300k 冒烟 wall_on 从 430s 降至 ≤5× 门关基线（照实判定，不达标照实上报实际倍数与构成归因）。
4. events_seen 数量级下降（166,226,275 → 与入账事件同量级；实际读数与阈值论证入交付报告）。
5. NEST_GATE_INDEX 行口径不变（events/events_seen/derivations 字段语义不动），新增诊断行只进 stderr 不入决策面。
6. **不承诺项（090）**：4.6M 全量门开可行是票面标题的方向性目标，本票验收只签 300k ≤5×；4.6M 读数为外推，实装后实测照实登记，不外推承诺。

## 6. 交付报告模板

1. 改动文件 + 行号清单（含 additive 暴露的签名差异行）。
2. T0 测量读数（前置）vs 实装后对照：派生触发率、结算分布、活跃集规模、决策树走向（run/pair 粒度）及理由。
3. 决策 diff=0 证据：门关锁清单 + 门开 300k 三跑 diff 结果（白名单字段逐项）。
4. 性能对照表：wall_off / wall_on（实装前 430s 票面值 vs 实装后实测）、倍数、events_seen 前后、derivations/index_builds。
5. 测试清单（T1-T7 新增测试名 + 锚）+ cargo test 尾行。
6. 与 5a/5b 对齐项登记：游标/终态语义是否归并、归并点、遗留差异。
7. 未决项（照实）：4.6M 外推读数、Scanning 长尾、未达标项及归因。
