# LEE 级别事件驱动执行与既有 voice/overlay/dual_ledger 集成点调研（issue #60 resolution）

- 日期：2026-07-20（worktree `/tmp/kimi-nest-mainline`，分支 `kimi-nest-mainline-20260717`，HEAD=`640609071d`）
- 性质：**只读调研文档工位**——未改 `rust/src` 任何一行，无 git mutation，主仓只读。每个结论附代码行号/文档锚；照实否定为合格产出。
- 任务来源：GitHub issue #60（wayfinder:research）三问：(a) LEE 四支柱在现有代码的最近挂载点；(b) formation_level 是否已被 SepLeg/TypedTrade 完整携带（M1 前置核实项）；(c) LEE M1 最小实装面。
- 输入文档：`multi-level-native-execution-design-20260719.md`（下称 **LEE 设计文档**）§C/§D；`level-exec-existing-inventory-20260720.md`（下称 **盘点文档**）§⑥ 可复用项。

## 行号漂移声明

LEE 设计文档与盘点文档写于 HEAD=`640609071d`，与本次核实同一 commit，但两文档引用的部分行号已与当前文件实际位置漂移（设计文档 §F.4 已授权「以锚点周围语义注释重定位」）。本报告全部行号为本次逐文件 Read 核实的**当前行号**：

- `newly_confirmed_step`：文档引 `runner.rs:809-835` → 实际 `runner.rs:925-951`（调用点 `runner.rs:1603`）。
- `events_by_level`：设计文档引 `nest.rs:540-542` → 该处现为 `TerminalMatch` 定义；`events_by_level` 的生产函数是 `cand_delta_tower`（`classifier/mod.rs:514`），类型为 `Vec<Vec<CandDeltaEvent>>`（下标=塔级别，事件类型 `recursive_tower.rs:1151` / `level_view.rs:480 NestCandidateEvent`），nest.rs 内为消费侧形参（`nest.rs:676/:744` 等）。
- `SepLeg` 打包点 `coverage.rs:2342-2354`、`OverlayState.step` 调用点 `runner.rs:1916` 经核实与文档一致。

---

## (a) LEE 四支柱在现有代码的最近挂载点

### 支柱 1：每级独立仓位账本 Ledger_ℓ → `overlay_state.rs` 的 `OverlayState.books` / `VoiceBook`

- 最近挂载点：`rust/src/theta_v0/strategy/overlay_state.rs:104`（`books: HashMap<ElementId, VoiceBook>`）+ 账本行定义 `overlay_state.rs:59-79`（`VoiceBook{id: ElementId, side, q, role_v, parent_id, entry_bar, entry_px, pnl_v}`）。
- 键 `ElementId` 已含 `level`（`recursive_tower.rs:76-81`），**按 `id.level` 分桶即 Ledger_ℓ 镜像，零 schema 改动**（详见 (b)）。
- 事件驱动版第二挂载点：`VoiceExecBook.positions: HashMap<ElementId, VoicePosition>`（`overlay_state.rs:380-382`）——W1 声部独立执行簿，fill 只在声部开/合事件产生 + sizing 开仓冻结（`overlay_state.rs:275-290` 文件头声明），语义上比 OverlayState 更靠近 LEE 的账本形态。
- `dual_ledger.rs` 的 `DualLedger`（`dual_ledger.rs:42-48`）**不是** Ledger_ℓ 的挂载点：它是**单级平坦双腿簿**（q_long/q_short 两坐标，无 level 维度，盘点文档 §2.2-6 同判定）。它对 LEE 的价值是验收协议而非账本结构——D8 嵌入恒等逐字节对拍（`dual_ledger.rs:440-525`）是 M1「加性细化 bit-exact」的现成协议模板（`compatible_leg_orders`，`dual_ledger.rs:233`）。
- 同形先例（文档侧已点名，本次未展开核实）：`trading/level_operating_unit.rs:48-55` `RevTranche{level,…}` + 「每级别至多一个开放 tranche」（LEE 设计文档 §C.1 支柱1）。

### 支柱 2：每级独立事件时钟 clock_ℓ → `runner.rs:925-951` `newly_confirmed_step`（按 ℓ 拆分）

- 最近挂载点：`rust/src/theta_v0/backtest/runner.rs:925-951`。该函数对本 bar **所有级别**的新确认 BSP 一次性 diff（`seen.insert((lvl, p.source_index, bsp_bits_disc(&p.bits)))`，`runner.rs:944`），diff 键已含 `lvl`——把单桶 `seen` 拆成 `seen_ℓ` 即 clock_ℓ 的雏形输入（LEE 设计文档 §C.2 伪码变化部分①）。调用点 `runner.rs:1603`。
- 事件源第二路（更细粒度）：`classifier/mod.rs:514` `cand_delta_tower` 产 `Vec<Vec<CandDeltaEvent>>`（下标=塔级别，mod.rs:519 返回类型）；`classifier/mod.rs:584` `cand_delta_entry_tower` 产 P53 放宽入口事件。这是「每级候选/证书事件」的既有按级分桶结构，nest 模块消费侧同口径（`nest.rs:759` `events_by_level.get(level)`）。
- 无前视锚：E2E 五钟（`observed/first_provable/structure_end/confirmed/invalidated`，`mainline-merged-roadmap-20260717.md:75` E2E-N5）在文档层定义事件时点的因果性；**无 rust 实装**（盘点文档 §3.3-4）。
- 照实否定：`newly_confirmed_step` 只 diff BSP 确认；「BSP Invalidated / 中枢生灭 / 段完成」作为可消费事件不存在（盘点文档 §4.3：BspPoint 无 invalidated 字段，`bsp.rs:113-165`）。clock_ℓ 的最小完备事件集仍是 LEE 设计文档 §F 未决问题②。

### 支柱 3：每级独立 BSP 消费 Consume_ℓ → `StepTrace.sep_legs` 暴露处（`coverage.rs:2342-2354` → `runner.rs:1916`）

- 生产侧 `Consume_at` **全 rust/src 0 匹配**（盘点文档 §3.1 grep 判定），只在原型文档 `doc-divergence-endtoend-prototype-20260718.md:75-78`——LEE 不能挂 `Consume_at`，只能挂其下游已实装的消费链。
- rust 侧最近挂载点：消费出口链 `assemble_gamma_with_tower`（`interp.rs:455`）→ `interpret` 三桶（规则2 反向关闭同级别活动腿，`interp.rs:1198`）→ `recognize`/`recognize_nested`（`strategy/mod.rs:663`）→ `plan_orders`/`plan_orders_dual`（`mod.rs:252`/`mod.rs:888`）。其中**级别路由的天然插入点**是 `SepLeg` 打包处：`coverage.rs:2342-2354`（`coverage_step_from_buckets_sep` 把 legs 对位到 ElementId 产 `Vec<SepLeg>`，只读重打包不新计算）→ 经 `StepTrace.sep_legs`（`coverage.rs:2866`）→ runner 旁路消费点 `runner.rs:1916`（`ov.step(&step_trace.sep_legs, …)`）。按 `sep_leg.id.level` 分桶即 Consume_ℓ 的路由动作，不触碰 E2E-S* 缝合线。
- 现行决策出口仍是统一净额 p̃（`coverage.rs:2762-2763` 诚实声明「v0 净额」）——Consume_ℓ 本体（独立目标计算）是 M2 以后的事，M1 只读。

### 支柱 4：每级独立 sizing → `coverage.rs:1524/:1548` `leg_target` 权重公式（depth → level 重锚点）

- 同构机制已就位且真实接线：`voice::depth_weight`（`voice.rs:188`，w=[0.60,0.30,0.10]，越界返 0.0 未用留现金，`voice.rs:12`/`voice.rs:182-184`）→ 配置 `config.rs:137`/`:156` → sizing 消费 `coverage.rs:1524`（`leg_target`：`w = depth_weight(depth) × dir_weight × w_grade`）与 `coverage.rs:1548`（`leg_target_two_segment` 同式，注释声明保 bit-exact）。
- 最近挂载点 = 这两行权重公式：LEE 的 w_ℓ 是把 `depth_weight(depth)` 的索引从 depth（声部树深度）重锚到 level（塔级别，元素上现成：`CoverageElement.level` `coverage.rs:167` / `SepLeg.id.level`）。这是**改索引语义**不是新增机制（LEE 设计文档 §C.1 支柱4 自承「重锚」）。
- 纪律边界：sizing 参数全属 Θ_risk（`coverage.rs:2430-2434` 同纪律），LEE 不新增任何缠论声明；M4 才上线（LEE 设计文档 §D 迁移表），M1 不触碰。

### 四支柱挂载点汇总表

| LEE 支柱 | 最近挂载点（文件:行号） | 就位度 |
|---|---|---|
| 1 Ledger_ℓ | `overlay_state.rs:104`（books，键含 level）+ `overlay_state.rs:59-79`（VoiceBook 行）；事件驱动版 `overlay_state.rs:380-382` | 账本行可直接复用，分桶零改动 |
| 2 clock_ℓ | `runner.rs:925-951`（newly_confirmed_step，diff 键含 lvl）；事件源 `classifier/mod.rs:514` `cand_delta_tower` | diff 雏形在；失效事件缺 |
| 3 Consume_ℓ | `coverage.rs:2342-2354`（SepLeg 打包）→ `runner.rs:1916`（旁路消费点） | 路由点现成；Consume_at 无实装 |
| 4 sizing | `coverage.rs:1524`/`:1548`（leg_target w 公式）+ `voice.rs:188` depth_weight | 同构机制已接线，待重锚 |

---

## (b) formation_level 是否已被 SepLeg/TypedTrade 完整携带（M1 前置核实项）

**结论：级别信息已被完整携带（经 `ElementId.level` / 显式 `level` 字段），但无 `formation_level` 字面字段名；且按 roadmap:64 `BspKey` 四元组口径核，SepLeg 缺 `source_index` 与 `point_class`——M1 按级别分桶不需要它们，M2+ 若要全键路由才需补。**

逐字段核（全部经本次 Read 核实）：

### SepLeg（`coverage.rs:1388-1399`）

字段全集：`{id: ElementId, side: VoiceSide, q_units: f64, role_v: Vertical, parent_id: Option<ElementId>}`。

- `id.level` = 该腿的塔级别 = BSP 出现级别，证据链：
  1. SepLeg 由 `coverage.rs:2342-2354` 从 `CoverageElement` 打包（`id: e.id`）。
  2. 候选元素构造 `build_candidate_element`（`interp.rs:720-743`）：`level: lvl`（lvl = `classification.levels` 下标，`assemble_gamma` 同型 `interp.rs:280-296`），`id: carrier_id.unwrap_or(ElementId { level: lvl, ordinal: ci })`。
  3. carrier 命中分支：`attach_bsp_carrier_indexed`（`coverage.rs:625-638`）按 `(level_g, source_index)` 键查树端点索引，host 元素级别 == `level_g` == lvl ⟹ `host.id.level == lvl`。
  4. 树元素侧：「compose level == id.level」（`interp.rs:491` TreeKey 指纹注释）；`ElementId` 定义注释「级别 ℓ（L0=0，逐级递增）」（`recursive_tower.rs:77-78`）。
  5. `ActiveLeg.level`（显式字段，`interp.rs:115-116`）由 `element_as_leg`（`coverage.rs:1919-1928`）从 `e.level` 复制，与 `id.level` 同源一致。
- **缺口（照实列出）**：SepLeg 不携 `source_index`（ActiveLeg 有，`interp.rs:120`；SepLeg 打包时丢弃）与 `point_class`/`BspBits`（BSP 点类）。roadmap:64 的 `BspKey=(formation_level,point_class,side,source_index)` 在 SepLeg 上只能还原 `(level, side)` 两个分量。M1 按 `id.level` 分桶**不需要**后两者；若后续要用 BspKey 全键做路由/去重，需在打包点（`coverage.rs:2345-2351`）补字段——属加性只读扩展，但当前不必要。

### TypedTrade（`runner.rs:2667-2717`）——三重冗余携带

1. `voice_id: ElementId`（`runner.rs:2671`，= `ActiveLeg.id`，注释自声明）→ `voice_id.level` = 腿级别。
2. `position_node_id: PositionNodeId`（`runner.rs:2694-2699`）内含 `EntryCertificate { level: u32, source_index: usize }`（`interp.rs:155-160`）——**显式「入场信号级别 ℓ_g」+ 入场触发点**，这是四元组中最接近 roadmap:64 `formation_level` 语义的字段（还顺带补上了 SepLeg 缺的 source_index）。
3. `entry_z: MuClass`（`runner.rs:2669`）结构身份维含 level（`runner.rs:2708` 注释枚举 level/δ/i_class/…）。

### 账本行（M1 直接消费对象）

- `VoiceBook.id: ElementId`（`overlay_state.rs:64`）→ level 可得，无独立 level 字段但**不需要**（分桶键 `b.id.level`）。
- `VoicePosition.id: ElementId`（`overlay_state.rs:318-319`）同。
- `DualLedger`（`dual_ledger.rs:42-48`）：**无任何级别/声部字段**——单级双腿簿，(ℓ, voice) 二维键控是 LEE M1-M4 的事（盘点文档 §2.2-6 引 e2e-fix:19/:185-186）。

### 判定

LEE 设计文档 §F 未决问题①「SepLeg/ActiveLeg 当前是否完整携带 formation_level」→ **成立（按级别分桶口径）**：`formation_level ≡ SepLeg.id.level`，零 schema 改动即可按级分桶；字段名是 `level` 不是 `formation_level`（roadmap:60「不得重命名后偷换口径」——本报告不引入新名，只指出既有 `level` 字段语义 = 塔级别 = BSP 出现级别）。BspKey 全四元组口径下 SepLeg 缺 `source_index`/`point_class`，M1 不需要，如实登记为 M2+ 的潜在加性扩展点。

---

## (c) LEE M1（级别账本只读旁路，bit-exact 起步）的最小实装面

M1 定义（LEE 设计文档 §D 迁移表）：OverlayState 的 SepLeg 按 formation_level 分组，建只读 Ledger_ℓ 镜像；不改净额主路径，bit-exact；验收 = 各级账本净额之和恒等 ΔN。

结合 (a)/(b) 核实结果，最小实装面为**三个接触点 + 一个验收协议**，全部为加性旁路：

1. **新账本类型（唯一新文件）**：`rust/src/theta_v0/strategy/level_ledger.rs`（候选名）——`LevelLedgerMirror { books: BTreeMap<u32 /*level*/, Vec<VoiceBook 式行>> }` 或直接复用 `VoiceBook`（`overlay_state.rs:62-79`）按 `id.level` 分桶。**不需要给 SepLeg/VoiceBook 加字段**（(b) 已证 `id.level` 完整携带）。用 BTreeMap 保确定序（bit-exact 可复现，同 `settle_forced_virtual` 的确定序纪律 `overlay_state.rs:598-610`）。
2. **接线点（runner 一处）**：`runner.rs:1916` 旁——`ov.step(&step_trace.sep_legs, …)` 是既有只读旁路的精确模式（`overlay=None ⟹ 净额路径 bit-exact`，`runner.rs:1271` 注释自声明）；镜像在同一决策点读同一份 `step_trace.sep_legs` 按级分桶，与 OverlayState 并列旁挂。备选挂点是 `pi_theta_fill_loop` 的 `overlay: Option<&mut OverlayState>` 形参位（`runner.rs:1285`）同款 `Option<&mut LevelLedgerMirror>`。
3. **不变量断言（验收）**：LEE-Net 恒等 `Σ_ℓ net_ℓ ≡ ΔN`（设计文档 §C.2）——逐 bar 断言 `Σ_ℓ (Σ_{v∈ℓ} σ_v q_v) == overlay.net()`（后者已有 ΔN 守恒断言锚 `overlay_state.rs:17-18`）。这是纯加性细化（线性代数，认识论 L1），订单流零变化。
4. **验收协议模板（现成，不需新造）**：`dual_ledger.rs:440-525` D8 逐字节对拍协议（cash/equity/realized/fee/trade_pnls `.to_bits()` 逐项断言）+ `overlay_state.rs:22-27` Σpnl_v=N·ΔP 对账先例（e2e-fix:150 已指明三者同一协议）。M1 的「既有测试零变化 + 新镜像自对账」直接套此模板。

**明确排除（M1 不做）**：不动 `pi_theta_step`/净额主路径；不动 `newly_confirmed_step` 的全级别混合 diff（clock_ℓ 拆分是 M3 事件门控的事，M1 仍每 bar 重估）；不动 `leg_target` 权重公式（M4）；不引入 `formation_level` 新字段名；不碰 nautilus 生产路径（OverlayState 当前也只在回测臂接线，`runner.rs:765-776`）。

**风险点（照实）**：① `SepLeg.id.level` 对 registry restore 注入的祖先腿同样成立（restore 行来自持久化 ActiveLeg，其 id 入场时定型），但 restore 路径罕触发（`coverage.rs:182-183` L2 诊断 sd_parent_held=0），M1 首跑应在含 restore 的样本上抽验分桶归属；② 幽灵腿（gross_zeroed）`q_units` 已零化（`coverage.rs:2341`），分桶镜像与净额口径一致地贡献 0，不产生分桶偏差。

---

## 锚点索引

- **代码**：`overlay_state.rs:59-79/:104/:380-382`（账本行/簿）；`dual_ledger.rs:42-48/:233/:440-525`（DualLedger/兼容腿/D8 协议）；`runner.rs:925-951/:1285/:1603/:1916`（diff/旁路形参/调用点/overlay.step）；`coverage.rs:1388-1399/:2342-2354/:1524/:1548/:625-638/:1919-1928`（SepLeg/打包/sizing/attach/element_as_leg）；`interp.rs:114-139/:155-160/:280-296/:455/:720-743/:1198`（ActiveLeg/EntryCertificate/候选构造/消费链）；`recursive_tower.rs:76-81`（ElementId）；`classifier/mod.rs:514/:584`（cand_delta_tower 事件源）；`strategy/mod.rs:252/:663/:888`（plan_orders/recognize_nested/plan_orders_dual）；`voice.rs:5-12/:188`（根=L*/depth_weight）；`runner.rs:2667-2717`（TypedTrade）。
- **文档**：LEE 设计文档 §C.1（四支柱）/§C.2（不变量）/§D（M0-M4）/§F（未决问题）；盘点文档 §3.1（grep 判定）/§4.3（失效机制）/§⑥（可复用项）；`mainline-merged-roadmap-20260717.md:60-79`（E2E 横切⑪）；`doc-divergence-endtoend-prototype-20260718.md:75-78`（Consume_at 原型）。
- **教义**：039:30（段事件机械程式）；032:26/:30（按级别操作）；044:16（跨级形态）——经 LEE 设计文档 §A.1 核对，本报告不新增教义引用。
